//! Read-only adapters for the Yggdrasil Admin JSON socket.
//!
//! The official Go daemon, yggdrasil-rs (published as yggdrasil-mesh and using
//! petrified for Ironwood), and Yggdrasil-ng expose compatible newline-delimited
//! JSON admin protocols. Their peer-stat fields are not identical, so each
//! backend has an explicit adapter type and latency-unit conversion.
//!
//! Only active direct peers are exported. A peer session proves that the
//! overlay currently has a route to that peer; it does not prove that an
//! application on the peer is reachable.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{Ipv6Addr, TcpStream, ToSocketAddrs};
use std::time::{Duration, SystemTime};

#[cfg(unix)]
use std::path::PathBuf;

use meshplex_core::{
    AdapterError, OverlayAdapter, OverlayEndpoint, OverlayId, OverlayRoute, OverlaySnapshot,
    RouteMetrics, RouteState,
};
use serde_json::{Value, json};

const DEFAULT_ADMIN_ADDR: &str = "127.0.0.1:9001";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(3);
const DEFAULT_MAX_RESPONSE_BYTES: usize = 1024 * 1024;

/// How MeshPlex connects to the Yggdrasil Admin socket.
///
/// Go and yggdrasil-rs support TCP and, on Unix systems, Unix-domain sockets.
/// Yggdrasil-ng currently documents a TCP admin listener. Use the endpoint
/// configured by the running daemon rather than assuming every platform has
/// the same default.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdminEndpoint {
    /// TCP endpoint in host:port form.
    Tcp(String),
    /// Unix-domain socket path. Available on Unix platforms.
    #[cfg(unix)]
    Unix(PathBuf),
}

impl AdminEndpoint {
    /// Construct a TCP endpoint such as 127.0.0.1:9001.
    pub fn tcp(address: impl Into<String>) -> Self {
        Self::Tcp(address.into())
    }

    /// Parse a tcp://host:port or, on Unix, unix:///path endpoint.
    pub fn parse(value: &str) -> Result<Self, AdapterError> {
        if let Some(address) = value.strip_prefix("tcp://") {
            if address.is_empty() {
                return Err(AdapterError::new("Yggdrasil TCP admin address is empty"));
            }
            return Ok(Self::Tcp(address.to_string()));
        }

        if let Some(path) = value.strip_prefix("unix://") {
            #[cfg(unix)]
            {
                if path.is_empty() {
                    return Err(AdapterError::new("Yggdrasil Unix admin socket path is empty"));
                }
                return Ok(Self::Unix(PathBuf::from(path)));
            }
            #[cfg(not(unix))]
            {
                let _ = path;
                return Err(AdapterError::new(
                    "Unix-domain Yggdrasil admin sockets are not supported on this platform",
                ));
            }
        }

        Err(AdapterError::new(
            "unsupported Yggdrasil admin endpoint; use tcp://host:port or unix:///path",
        ))
    }
}

impl Default for AdminEndpoint {
    fn default() -> Self {
        Self::Tcp(DEFAULT_ADMIN_ADDR.to_string())
    }
}

/// Connection limits for a local Yggdrasil Admin socket.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct YggdrasilConfig {
    /// The daemon's configured Admin socket.
    pub endpoint: AdminEndpoint,
    /// Timeout for connect, read, and write operations.
    pub timeout: Duration,
    /// Maximum response size accepted from one Admin request.
    pub max_response_bytes: usize,
}

impl Default for YggdrasilConfig {
    fn default() -> Self {
        Self {
            endpoint: AdminEndpoint::default(),
            timeout: DEFAULT_TIMEOUT,
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Backend {
    Go,
    RustMesh,
    RustNg,
}

#[derive(Clone, Debug)]
struct YggdrasilClient {
    config: YggdrasilConfig,
    backend: Backend,
}

impl YggdrasilClient {
    fn new(config: YggdrasilConfig, backend: Backend) -> Self {
        Self { config, backend }
    }

    fn snapshot(&self) -> Result<OverlaySnapshot, AdapterError> {
        if self.config.timeout.is_zero() {
            return Err(AdapterError::new(
                "Yggdrasil Admin timeout must be greater than zero",
            ));
        }
        if self.config.max_response_bytes == 0 {
            return Err(AdapterError::new(
                "Yggdrasil maximum response size must be greater than zero",
            ));
        }

        // Query self first so the local node is not exported as a remote route.
        let self_info = self.request("getSelf", json!({}))?;
        let peers_info = self.request("getPeers", json!({"sort": ""}))?;
        build_snapshot(&self_info, &peers_info, self.backend)
    }

    fn request(&self, name: &str, arguments: Value) -> Result<Value, AdapterError> {
        let request = json!({
            "request": name,
            "arguments": arguments,
        });
        let mut bytes = serde_json::to_vec(&request).map_err(|error| {
            AdapterError::new(format!("encode Yggdrasil Admin request: {error}"))
        })?;
        bytes.push(b'\n');

        let response = match &self.config.endpoint {
            AdminEndpoint::Tcp(address) => {
                let mut stream = connect_tcp(address, self.config.timeout)?;
                stream
                    .set_read_timeout(Some(self.config.timeout))
                    .map_err(|error| {
                        AdapterError::new(format!("set Yggdrasil Admin read timeout: {error}"))
                    })?;
                stream
                    .set_write_timeout(Some(self.config.timeout))
                    .map_err(|error| {
                        AdapterError::new(format!("set Yggdrasil Admin write timeout: {error}"))
                    })?;
                exchange(stream, &bytes, self.config.max_response_bytes)?
            }
            #[cfg(unix)]
            AdminEndpoint::Unix(path) => {
                let mut stream = std::os::unix::net::UnixStream::connect(path).map_err(|error| {
                    AdapterError::new(format!(
                        "connect to Yggdrasil Unix Admin socket {}: {error}",
                        path.display()
                    ))
                })?;
                stream
                    .set_read_timeout(Some(self.config.timeout))
                    .map_err(|error| {
                        AdapterError::new(format!("set Yggdrasil Admin read timeout: {error}"))
                    })?;
                stream
                    .set_write_timeout(Some(self.config.timeout))
                    .map_err(|error| {
                        AdapterError::new(format!("set Yggdrasil Admin write timeout: {error}"))
                    })?;
                exchange(stream, &bytes, self.config.max_response_bytes)?
            }
        };

        let envelope: Value = serde_json::from_slice(&response).map_err(|error| {
            AdapterError::new(format!("decode Yggdrasil Admin JSON response: {error}"))
        })?;
        response_payload(&envelope, name)
    }
}

fn connect_tcp(address: &str, timeout: Duration) -> Result<TcpStream, AdapterError> {
    let addresses = address.to_socket_addrs().map_err(|error| {
        AdapterError::new(format!("resolve Yggdrasil Admin address {address}: {error}"))
    })?;

    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => return Ok(stream),
            Err(error) => last_error = Some(error),
        }
    }

    let error = last_error
        .map(|error| error.to_string())
        .unwrap_or_else(|| "address resolved to no socket addresses".to_string());
    Err(AdapterError::new(format!(
        "connect to Yggdrasil Admin TCP socket {address}: {error}"
    )))
}

fn exchange<S: Read + Write>(
    mut stream: S,
    request: &[u8],
    max_response_bytes: usize,
) -> Result<Vec<u8>, AdapterError> {
    stream
        .write_all(request)
        .and_then(|()| stream.flush())
        .map_err(|error| AdapterError::new(format!("write Yggdrasil Admin request: {error}")))?;

    // The request omits keepalive, so the daemon closes the connection after
    // its response. Read to EOF because the response is pretty-printed JSON
    // and can contain internal newlines.
    let mut response = Vec::new();
    stream
        .take(max_response_bytes as u64 + 1)
        .read_to_end(&mut response)
        .map_err(|error| AdapterError::new(format!("read Yggdrasil Admin response: {error}")))?;

    if response.len() > max_response_bytes {
        return Err(AdapterError::new(format!(
            "Yggdrasil Admin response exceeds the configured {} byte limit",
            max_response_bytes
        )));
    }
    if response.is_empty() {
        return Err(AdapterError::new(
            "Yggdrasil Admin socket closed without a response",
        ));
    }

    Ok(response)
}

fn response_payload(envelope: &Value, request_name: &str) -> Result<Value, AdapterError> {
    match envelope.get("status").and_then(Value::as_str) {
        Some("success") => {}
        Some("error") => {
            let message = envelope
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("unspecified Admin API error");
            return Err(AdapterError::new(format!(
                "Yggdrasil Admin {request_name} failed: {message}"
            )));
        }
        Some(status) => {
            return Err(AdapterError::new(format!(
                "Yggdrasil Admin {request_name} returned unknown status {status:?}"
            )));
        }
        None => {
            return Err(AdapterError::new(format!(
                "Yggdrasil Admin {request_name} response has no status"
            )));
        }
    }

    envelope
        .get("response")
        .cloned()
        .filter(Value::is_object)
        .ok_or_else(|| {
            AdapterError::new(format!(
                "Yggdrasil Admin {request_name} response payload is not an object"
            ))
        })
}

fn peer_round_trip_time(peer: &Value, backend: Backend) -> Option<Duration> {
    let latency = peer.get("latency").or_else(|| peer.get("latency_ms"))?;

    match backend {
        // Go's time.Duration JSON encoding and yggdrasil-rs use nanoseconds.
        Backend::Go | Backend::RustMesh => {
            let nanos = latency
                .as_u64()
                .or_else(|| latency.as_i64().and_then(|value| u64::try_from(value).ok()))?;
            (nanos > 0).then(|| Duration::from_nanos(nanos))
        }
        // Yggdrasil-ng serializes the peer's latency_ms value as milliseconds.
        Backend::RustNg => {
            let millis = latency
                .as_u64()
                .map(|value| value as f64)
                .or_else(|| latency.as_f64())?;
            if millis <= 0.0 || !millis.is_finite() {
                return None;
            }
            let nanos = millis * 1_000_000.0;
            if !nanos.is_finite() || nanos > u64::MAX as f64 {
                return None;
            }
            Some(Duration::from_nanos(nanos.round() as u64))
        }
    }
}

macro_rules! define_adapter {
    ($adapter:ident, $backend:expr, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug)]
        pub struct $adapter {
            client: YggdrasilClient,
        }

        impl $adapter {
            /// Create a read-only adapter using the configured Admin socket.
            pub fn new(config: YggdrasilConfig) -> Self {
                Self {
                    client: YggdrasilClient::new(config, $backend),
                }
            }

            /// Return this adapter's connection settings.
            pub fn config(&self) -> &YggdrasilConfig {
                &self.client.config
            }
        }

        impl OverlayAdapter for $adapter {
            fn snapshot(&self) -> Result<OverlaySnapshot, AdapterError> {
                self.client.snapshot()
            }
        }
    };
}

define_adapter!(
    YggdrasilGoAdapter,
    Backend::Go,
    "Read-only adapter for the official yggdrasil-go Admin JSON socket."
);

define_adapter!(
    YggdrasilRsAdapter,
    Backend::RustMesh,
    "Read-only adapter for yggdrasil-rs (the yggdrasil-mesh crate, built on petrified)."
);

define_adapter!(
    YggdrasilNgAdapter,
    Backend::RustNg,
    "Read-only adapter for the independent Yggdrasil-ng Rust implementation."
);

/// Convert parsed Admin payloads into a MeshPlex snapshot.
///
/// Kept separate from socket I/O so response variations can be unit-tested
/// without running a Yggdrasil daemon.
fn build_snapshot(
    self_info: &Value,
    peers_info: &Value,
    backend: Backend,
) -> Result<OverlaySnapshot, AdapterError> {
    let self_address = self_info
        .get("address")
        .and_then(Value::as_str)
        .ok_or_else(|| AdapterError::new("Yggdrasil getSelf response has no address"))?
        .parse::<Ipv6Addr>()
        .map_err(|error| {
            AdapterError::new(format!(
                "Yggdrasil getSelf address is not a valid IPv6 address: {error}"
            ))
        })?;

    let peers = peers_info
        .get("peers")
        .and_then(Value::as_array)
        .ok_or_else(|| AdapterError::new("Yggdrasil getPeers response has no peers array"))?;

    let observed_at = SystemTime::now();
    let mut connected_peers = 0_usize;
    let mut routes = BTreeMap::<Ipv6Addr, RouteMetrics>::new();

    for peer in peers {
        if peer.get("up").and_then(Value::as_bool) != Some(true) {
            continue;
        }
        let Some(address) = peer.get("address").and_then(Value::as_str) else {
            continue;
        };
        let Ok(address) = address.parse::<Ipv6Addr>() else {
            // Not-yet-authenticated or disconnected peer entries may use "-".
            continue;
        };
        if address == self_address {
            continue;
        }

        connected_peers += 1;
        routes.entry(address).or_insert_with(|| RouteMetrics {
            round_trip_time: peer_round_trip_time(peer, backend),
            // This is the collection time, not the daemon's metric timestamp.
            observed_at: Some(observed_at),
            ..RouteMetrics::default()
        });
    }

    let routes = routes
        .into_iter()
        .map(|(address, metrics)| OverlayRoute {
            endpoint: OverlayEndpoint {
                network: OverlayId::Yggdrasil,
                address: address.into(),
                port: None,
            },
            state: RouteState::Routed,
            metrics,
        })
        .collect::<Vec<_>>();

    Ok(OverlaySnapshot {
        network: OverlayId::Yggdrasil,
        connected_peers: Some(connected_peers),
        known_routes: routes.len(),
        routes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn self_info(address: &str) -> Value {
        json!({
            "address": address,
            "key": "local-public-key",
            "subnet": "200::/64"
        })
    }

    #[test]
    fn default_endpoint_is_loopback_tcp() {
        assert_eq!(
            YggdrasilConfig::default().endpoint,
            AdminEndpoint::Tcp("127.0.0.1:9001".to_string())
        );
    }

    #[test]
    fn parses_tcp_endpoint() {
        assert_eq!(
            AdminEndpoint::parse("tcp://127.0.0.1:9001").unwrap(),
            AdminEndpoint::Tcp("127.0.0.1:9001".to_string())
        );
    }

    #[cfg(unix)]
    #[test]
    fn parses_unix_endpoint() {
        assert_eq!(
            AdminEndpoint::parse("unix:///run/yggdrasil.sock").unwrap(),
            AdminEndpoint::Unix(PathBuf::from("/run/yggdrasil.sock"))
        );
    }

    #[test]
    fn rejects_unknown_endpoint_schemes() {
        assert!(AdminEndpoint::parse("http://127.0.0.1:9001").is_err());
    }

    #[test]
    fn parses_go_and_yggdrasil_rs_latency_as_nanoseconds() {
        let peer = json!({"latency": 5_000_000_u64});
        assert_eq!(
            peer_round_trip_time(&peer, Backend::Go),
            Some(Duration::from_millis(5))
        );
        assert_eq!(
            peer_round_trip_time(&peer, Backend::RustMesh),
            Some(Duration::from_millis(5))
        );
    }

    #[test]
    fn parses_yggdrasil_ng_latency_as_milliseconds() {
        let peer = json!({"latency": 5_u64});
        assert_eq!(
            peer_round_trip_time(&peer, Backend::RustNg),
            Some(Duration::from_millis(5))
        );
    }

    #[test]
    fn missing_or_zero_latency_stays_unknown() {
        assert_eq!(peer_round_trip_time(&json!({}), Backend::Go), None);
        assert_eq!(
            peer_round_trip_time(&json!({"latency": 0}), Backend::RustNg),
            None
        );
    }

    #[test]
    fn exports_only_active_remote_ipv6_peers() {
        let peers = json!({
            "peers": [
                {"up": true, "address": "200:1::2", "latency": 5_000_000_u64},
                {"up": false, "address": "200:1::3", "latency": 1_000_000_u64},
                {"up": true, "address": "200:1::1", "latency": 1_000_000_u64},
                {"up": true, "address": "-", "latency": 1_000_000_u64}
            ]
        });

        let snapshot = build_snapshot(
            &self_info("200:1::1"),
            &peers,
            Backend::Go,
        )
        .unwrap();

        assert_eq!(snapshot.connected_peers, Some(1));
        assert_eq!(snapshot.known_routes, 1);
        assert_eq!(snapshot.routes.len(), 1);
        assert_eq!(
            snapshot.routes[0].endpoint.address,
            "200:1::2".parse::<Ipv6Addr>().unwrap().into()
        );
        assert_eq!(snapshot.routes[0].state, RouteState::Routed);
        assert_eq!(
            snapshot.routes[0].metrics.round_trip_time,
            Some(Duration::from_millis(5))
        );
        assert!(snapshot.routes[0].metrics.observed_at.is_some());
        assert_eq!(snapshot.routes[0].metrics.packet_loss_ppm, None);
        assert_eq!(snapshot.routes[0].metrics.throughput_bps, None);
    }

    #[test]
    fn yggdrasil_ng_uses_its_peer_latency_unit() {
        let peers = json!({
            "peers": [
                {"up": true, "uri": "tcp://peer:1234", "address": "200:1::2", "latency": 7_u64}
            ]
        });

        let snapshot = build_snapshot(
            &self_info("200:1::1"),
            &peers,
            Backend::RustNg,
        )
        .unwrap();

        assert_eq!(
            snapshot.routes[0].metrics.round_trip_time,
            Some(Duration::from_millis(7))
        );
    }

    #[test]
    fn rejects_failed_admin_requests() {
        let error = response_payload(
            &json!({
                "status": "error",
                "error": "unknown action",
                "response": null
            }),
            "getPeers",
        )
        .unwrap_err();

        assert!(error.to_string().contains("unknown action"));
    }

    #[test]
    fn rejects_missing_peer_array() {
        let error = build_snapshot(
            &self_info("200:1::1"),
            &json!({"not_peers": []}),
            Backend::Go,
        )
        .unwrap_err();

        assert!(error.to_string().contains("peers array"));
    }
}
