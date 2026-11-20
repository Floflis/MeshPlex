//! Read-only adapter for the CJDNS Admin UDP interface.
//!
//! This adapter reads the paginated NodeStore routing table. It does not authenticate,
//! alter peers, add or remove routes, or issue active remote-node pings.

use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};
use std::time::{Duration, SystemTime};

use meshplex_core::{
    AdapterError, OverlayAdapter, OverlayEndpoint, OverlayId, OverlayRoute, OverlaySnapshot,
    RouteMetrics, RouteState,
};

const MAX_UDP_PAYLOAD: usize = 65_535;
const MAX_BENCODE_DEPTH: usize = 32;

/// Connection and pagination settings for the local CJDNS Admin API.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CjdnsConfig {
    /// CJDNS Admin UDP address. The upstream default is 127.0.0.1:11234.
    pub admin_addr: SocketAddr,
    /// Timeout for each individual Admin API request.
    pub timeout: Duration,
    /// Safety cap for NodeStore_dumpTable pagination.
    pub max_pages: usize,
}

impl Default for CjdnsConfig {
    fn default() -> Self {
        Self {
            admin_addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 11_234)),
            timeout: Duration::from_secs(2),
            max_pages: 256,
        }
    }
}

/// A read-only CJDNS adapter using NodeStore_dumpTable.
#[derive(Clone, Debug, Default)]
pub struct CjdnsAdapter {
    config: CjdnsConfig,
}

impl CjdnsAdapter {
    pub fn new(config: CjdnsConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &CjdnsConfig {
        &self.config
    }

    fn request_page(&self, page: usize) -> Result<BValue, AdapterError> {
        let request = encode_dump_table_request(page);
        let bind_addr = match self.config.admin_addr {
            SocketAddr::V4(_) => SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)),
            SocketAddr::V6(_) => SocketAddr::from((Ipv6Addr::UNSPECIFIED, 0)),
        };

        let socket = UdpSocket::bind(bind_addr)
            .map_err(|error| AdapterError::new(format!("bind CJDNS Admin UDP socket: {error}")))?;
        socket
            .set_read_timeout(Some(self.config.timeout))
            .map_err(|error| AdapterError::new(format!("set CJDNS read timeout: {error}")))?;
        socket
            .set_write_timeout(Some(self.config.timeout))
            .map_err(|error| AdapterError::new(format!("set CJDNS write timeout: {error}")))?;
        socket
            .connect(self.config.admin_addr)
            .map_err(|error| AdapterError::new(format!("connect to CJDNS Admin API: {error}")))?;
        socket
            .send(&request)
            .map_err(|error| AdapterError::new(format!("send CJDNS Admin request: {error}")))?;

        let mut response = vec![0_u8; MAX_UDP_PAYLOAD];
        let length = socket.recv(&mut response).map_err(|error| {
            AdapterError::new(format!("receive CJDNS Admin response: {error}"))
        })?;
        response.truncate(length);

        decode_bencode(&response).map_err(|error| {
            AdapterError::new(format!("decode CJDNS Admin bencode response: {error}"))
        })
    }

    fn collect_routes(&self) -> Result<Vec<OverlayRoute>, AdapterError> {
        if self.config.max_pages == 0 {
            return Err(AdapterError::new("CJDNS max_pages must be greater than zero"));
        }

        let mut routes = Vec::new();

        for page in 0..self.config.max_pages {
            let response = self.request_page(page)?;
            check_admin_error(&response)?;
            routes.extend(parse_route_page(&response)?);

            let more = response
                .get(b"more")
                .and_then(BValue::as_integer)
                .unwrap_or(0);
            if more == 0 {
                return Ok(routes);
            }
        }

        Err(AdapterError::new(format!(
            "CJDNS routing table exceeded the configured {} page limit",
            self.config.max_pages
        )))
    }
}

impl OverlayAdapter for CjdnsAdapter {
    fn snapshot(&self) -> Result<OverlaySnapshot, AdapterError> {
        let routes = self.collect_routes()?;
        Ok(OverlaySnapshot {
            network: OverlayId::Cjdns,
            // NodeStore_dumpTable exposes routes, not a reliable active-peer count.
            connected_peers: None,
            known_routes: routes.len(),
            routes,
        })
    }
}

fn encode_dump_table_request(page: usize) -> Vec<u8> {
    // Bencode dictionary keys are ordered lexicographically: args comes before q.
    format!("d4:argsd4:pagei{page}ee1:q19:NodeStore_dumpTablee").into_bytes()
}

fn check_admin_error(response: &BValue) -> Result<(), AdapterError> {
    if let Some(error) = response.get(b"error").and_then(BValue::as_bytes) {
        if error != b"none" {
            return Err(AdapterError::new(format!(
                "CJDNS Admin API returned error: {}",
                String::from_utf8_lossy(error)
            )));
        }
    }
    Ok(())
}

fn parse_route_page(response: &BValue) -> Result<Vec<OverlayRoute>, AdapterError> {
    let entries = response
        .get(b"routingTable")
        .and_then(BValue::as_list)
        .ok_or_else(|| AdapterError::new("CJDNS response has no routingTable list"))?;

    let observed_at = SystemTime::now();
    let mut routes = Vec::with_capacity(entries.len());

    for entry in entries {
        let ip = entry
            .get(b"ip")
            .and_then(BValue::as_bytes)
            .ok_or_else(|| AdapterError::new("CJDNS route entry has no string ip"))?;
        let ip = std::str::from_utf8(ip)
            .map_err(|_| AdapterError::new("CJDNS route IP is not valid UTF-8"))?
            .parse::<IpAddr>()
            .map_err(|error| AdapterError::new(format!("invalid CJDNS route IP: {error}")))?;

        // NodeStore_dumpTable contains CJDNS IPv6 destinations. Ignore any
        // non-IPv6 values rather than presenting them as CJDNS overlay endpoints.
        let IpAddr::V6(ipv6) = ip else {
            continue;
        };

        let link = entry
            .get(b"link")
            .and_then(BValue::as_integer)
            .ok_or_else(|| AdapterError::new("CJDNS route entry has no integer link score"))?;
        let path_quality = u32::try_from(link)
            .map_err(|_| AdapterError::new("CJDNS link score is outside the u32 range"))?;

        routes.push(OverlayRoute {
            endpoint: OverlayEndpoint {
                network: OverlayId::Cjdns,
                address: IpAddr::V6(ipv6),
                port: None,
            },
            // A routing-table entry means the overlay has a route. It is not
            // evidence of an application-level or end-to-end probe success.
            state: RouteState::Routed,
            metrics: RouteMetrics {
                path_quality: Some(path_quality),
                observed_at: Some(observed_at),
                ..RouteMetrics::default()
            },
        });
    }

    Ok(routes)
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum BValue {
    Bytes(Vec<u8>),
    Integer(i64),
    List(Vec<BValue>),
    Dict(BTreeMap<Vec<u8>, BValue>),
}

impl BValue {
    fn get(&self, key: &[u8]) -> Option<&Self> {
        match self {
            Self::Dict(entries) => entries.get(key),
            _ => None,
        }
    }

    fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes(bytes) => Some(bytes),
            _ => None,
        }
    }

    fn as_integer(&self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(*value),
            _ => None,
        }
    }

    fn as_list(&self) -> Option<&[BValue]> {
        match self {
            Self::List(values) => Some(values),
            _ => None,
        }
    }
}

fn decode_bencode(input: &[u8]) -> Result<BValue, String> {
    let mut decoder = BDecoder {
        input,
        cursor: 0,
    };
    let value = decoder.parse_value(0)?;
    if decoder.cursor != input.len() {
        return Err("trailing bytes after bencode value".to_string());
    }
    Ok(value)
}

struct BDecoder<'a> {
    input: &'a [u8],
    cursor: usize,
}

impl BDecoder<'_> {
    fn parse_value(&mut self, depth: usize) -> Result<BValue, String> {
        if depth > MAX_BENCODE_DEPTH {
            return Err("bencode nesting limit exceeded".to_string());
        }

        match self.peek()? {
            b'd' => self.parse_dict(depth + 1),
            b'l' => self.parse_list(depth + 1),
            b'i' => self.parse_integer(),
            b'0'..=b'9' => self.parse_bytes().map(BValue::Bytes),
            other => Err(format!("unexpected bencode token: {other}")),
        }
    }

    fn parse_dict(&mut self, depth: usize) -> Result<BValue, String> {
        self.cursor += 1;
        let mut entries = BTreeMap::new();

        while self.peek()? != b'e' {
            let key = self.parse_bytes()?;
            let value = self.parse_value(depth)?;
            if entries.insert(key, value).is_some() {
                return Err("duplicate key in bencode dictionary".to_string());
            }
        }

        self.cursor += 1;
        Ok(BValue::Dict(entries))
    }

    fn parse_list(&mut self, depth: usize) -> Result<BValue, String> {
        self.cursor += 1;
        let mut values = Vec::new();

        while self.peek()? != b'e' {
            values.push(self.parse_value(depth)?);
        }

        self.cursor += 1;
        Ok(BValue::List(values))
    }

    fn parse_integer(&mut self) -> Result<BValue, String> {
        self.cursor += 1;
        let start = self.cursor;
        let end = self
            .input
            .get(start..)
            .and_then(|remaining| remaining.iter().position(|byte| *byte == b'e'))
            .map(|offset| start + offset)
            .ok_or_else(|| "unterminated bencode integer".to_string())?;

        let encoded = std::str::from_utf8(&self.input[start..end])
            .map_err(|_| "bencode integer is not ASCII".to_string())?;
        if encoded.is_empty() || encoded == "-0" || encoded.starts_with('+') {
            return Err("invalid bencode integer".to_string());
        }
        if encoded.len() > 1 && encoded.starts_with('0') {
            return Err("bencode integer has a leading zero".to_string());
        }
        if encoded.len() > 2 && encoded.starts_with("-0") {
            return Err("bencode integer has a leading zero".to_string());
        }

        let value = encoded
            .parse::<i64>()
            .map_err(|_| "bencode integer is out of range".to_string())?;
        self.cursor = end + 1;
        Ok(BValue::Integer(value))
    }

    fn parse_bytes(&mut self) -> Result<Vec<u8>, String> {
        let length_start = self.cursor;
        let colon = self
            .input
            .get(length_start..)
            .and_then(|remaining| remaining.iter().position(|byte| *byte == b':'))
            .map(|offset| length_start + offset)
            .ok_or_else(|| "bencode byte string has no length separator".to_string())?;

        let encoded_length = std::str::from_utf8(&self.input[length_start..colon])
            .map_err(|_| "bencode byte string length is not ASCII".to_string())?;
        if encoded_length.is_empty()
            || (encoded_length.len() > 1 && encoded_length.starts_with('0'))
        {
            return Err("invalid bencode byte string length".to_string());
        }

        let length = encoded_length
            .parse::<usize>()
            .map_err(|_| "bencode byte string length is out of range".to_string())?;
        let start = colon + 1;
        let end = start
            .checked_add(length)
            .ok_or_else(|| "bencode byte string length overflow".to_string())?;
        let value = self
            .input
            .get(start..end)
            .ok_or_else(|| "truncated bencode byte string".to_string())?
            .to_vec();

        self.cursor = end;
        Ok(value)
    }

    fn peek(&self) -> Result<u8, String> {
        self.input
            .get(self.cursor)
            .copied()
            .ok_or_else(|| "unexpected end of bencode input".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BValue, CjdnsAdapter, CjdnsConfig, decode_bencode, encode_dump_table_request,
        parse_route_page,
    };
    use std::net::{IpAddr, Ipv6Addr, SocketAddr};
    use std::time::Duration;

    #[test]
    fn dump_table_request_is_valid_bencode_with_expected_fields() {
        let request = encode_dump_table_request(3);
        let decoded = decode_bencode(&request).unwrap();

        assert_eq!(
            decoded.get(b"q").and_then(BValue::as_bytes),
            Some(&b"NodeStore_dumpTable"[..])
        );
        assert_eq!(
            decoded
                .get(b"args")
                .and_then(|args| args.get(b"page"))
                .and_then(BValue::as_integer),
            Some(3)
        );
    }

    #[test]
    fn parses_cjdns_routes_as_routed_not_end_to_end_reachable() {
        let ip = "fc00::1234";
        let path = "0000.0000.0000.0001";
        let entry = format!(
            "d2:ip{}:{}4:linki4294967295e4:path{}:{}e",
            ip.len(),
            ip,
            path.len(),
            path
        );
        let response = format!("d4:morei0e12:routingTablel{entry}ee");
        let decoded = decode_bencode(response.as_bytes()).unwrap();
        let routes = parse_route_page(&decoded).unwrap();

        assert_eq!(routes.len(), 1);
        assert_eq!(
            routes[0].endpoint.address,
            IpAddr::V6("fc00::1234".parse::<Ipv6Addr>().unwrap())
        );
        assert_eq!(routes[0].state, meshplex_core::RouteState::Routed);
        assert_eq!(routes[0].metrics.path_quality, Some(u32::MAX));
        assert!(routes[0].metrics.observed_at.is_some());
    }

    #[test]
    fn ignores_non_ipv6_route_entries() {
        let ip = "192.0.2.1";
        let path = "0000.0000.0000.0001";
        let entry = format!(
            "d2:ip{}:{}4:linki1e4:path{}:{}e",
            ip.len(),
            ip,
            path.len(),
            path
        );
        let response = format!("d12:routingTablel{entry}e4:morei0ee");
        let decoded = decode_bencode(response.as_bytes()).unwrap();

        assert!(parse_route_page(&decoded).unwrap().is_empty());
    }

    #[test]
    fn rejects_duplicate_dictionary_keys_and_trailing_data() {
        assert!(decode_bencode(b"d1:ai1e1:ai2ee").is_err());
        assert!(decode_bencode(b"i1ee").is_err());
    }

    #[test]
    fn rejects_invalid_route_quality_values() {
        let ip = "fc00::1";
        let path = "0000.0000.0000.0001";
        let entry = format!(
            "d2:ip{}:{}4:linki-1e4:path{}:{}e",
            ip.len(),
            ip,
            path.len(),
            path
        );
        let response = format!("d12:routingTablel{entry}e4:morei0ee");
        let decoded = decode_bencode(response.as_bytes()).unwrap();

        assert!(parse_route_page(&decoded).is_err());
    }

    #[test]
    fn config_defaults_to_local_cjdns_admin_udp_socket() {
        let config = CjdnsConfig::default();
        assert_eq!(
            config.admin_addr,
            SocketAddr::from(([127, 0, 0, 1], 11_234))
        );
        assert_eq!(config.timeout, Duration::from_secs(2));
        assert_eq!(config.max_pages, 256);

        let adapter = CjdnsAdapter::default();
        assert_eq!(adapter.config(), &config);
    }
}
