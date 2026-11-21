# MeshPlex

**A Rust-native control plane for choosing paths across independent overlay networks.**

MeshPlex observes independent networks, associates their endpoints with application-level service identities, and selects a fresh route according to an explicit policy.

The first targets are **CJDNS** and **Yggdrasil**. Their protocol implementations, cryptographic identities, addressing schemes, and routing algorithms remain independent.

> Status: early development. Service identity, measurement freshness, read-only CJDNS integration, and read-only Yggdrasil adapters for the Go and two Rust node implementations are available. System-level routing integration remains planned.

## Current implementation

- **Service identity:** a stable SHA-256 identifier derived from a canonical service public key, independent of overlay node keys.
- **Service advertisements:** map one service identity to explicitly advertised endpoints on one or more overlays.
- **Measurement freshness:** selector rejects missing, future-dated, and expired observations. The default maximum age is 30 seconds.
- **CJDNS adapter:** reads the paginated CJDNS NodeStore routing table through the local Admin UDP API.
- **Yggdrasil adapters:** read active peers from the shared Admin JSON protocol for official yggdrasil-go, yggdrasil-rs, and Yggdrasil-ng.
- **Route selection:** deterministic policies for latency, throughput, packet loss, and preferred overlay.
- **CI:** formatting, Clippy, and unit tests.

The CJDNS adapter is read-only. It reports a routing-table entry as Routed, not Reachable: the presence of a route is not an end-to-end application probe. Its path-quality value is CJDNS's unitless link score, not latency or throughput.

The Yggdrasil adapters also report direct active peers as Routed, not Reachable. Peer RTT is retained only when the Admin response supplies it. Missing packet loss and throughput remain unknown.

## What is not implemented

MeshPlex currently does **not**:

- Start, configure, or supervise overlay daemons.
- Implement CJDNS, CryptoAuth, Ironwood, or Yggdrasil protocols.
- Provide a signed service-advertisement or trust-distribution protocol.
- Forward, bridge, translate, or encapsulate packets between overlays.
- Make an endpoint on one overlay reachable through another overlay.
- Change host routing tables or transparently migrate established TCP connections.
- Provide anonymity, an exit-node network, VPN functionality, or an Internet replacement.
- Claim a performance improvement before reproducible benchmarks exist.

The selector chooses only among endpoints explicitly associated with the requested service identity and backed by fresh overlay observations. It cannot infer that two unrelated addresses identify the same service.

## Workspace

- crates/meshplex-core: service identity, advertisements, route models, freshness checks, selection policies, and adapter contract.
- crates/meshplex-cjdns: read-only CJDNS Admin UDP adapter.
- crates/meshplex-yggdrasil: read-only adapters for yggdrasil-go, yggdrasil-rs, and Yggdrasil-ng.

## Build and test

Requires Rust 1.85 or newer.

~~~sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
~~~

## CJDNS adapter

The adapter defaults to 127.0.0.1:11234/UDP and calls the read-only NodeStore_dumpTable Admin API function. It follows the API's page indicator and applies a configurable timeout and page limit.

~~~rust
use meshplex_cjdns::{CjdnsAdapter, CjdnsConfig};
use meshplex_core::OverlayAdapter;

let adapter = CjdnsAdapter::new(CjdnsConfig::default());
let snapshot = adapter.snapshot()?;

println!("Known CJDNS routes: {}", snapshot.known_routes);
~~~

The CJDNS Admin interface can expose state-changing operations. Keep it bound to loopback or another tightly controlled local endpoint. MeshPlex sends only the read-only route-table query; it does not authenticate or invoke state-changing functions.

CJDNS route-table entries expose a link-quality score and route label. MeshPlex currently retains the quality score and collection timestamp. It does not claim an RTT measurement or count route-table entries as active peers.

## Yggdrasil adapters

All three node implementations expose a compatible newline-delimited JSON Admin protocol. MeshPlex uses only the read-only getSelf and getPeers requests.

| Adapter | Upstream | Peer latency encoding |
|---|---|---|
| YggdrasilGoAdapter | [yggdrasil-go](https://github.com/yggdrasil-network/yggdrasil-go) | nanoseconds (time.Duration JSON integer) |
| YggdrasilRsAdapter | [yggdrasil-rs](https://github.com/ApexDevelopment/yggdrasil-rs), published as yggdrasil-mesh and built on [petrified](https://github.com/ApexDevelopment/petrified) | nanoseconds |
| YggdrasilNgAdapter | [Yggdrasil-ng](https://github.com/Revertron/Yggdrasil-ng) | milliseconds |

petrified is the Rust Ironwood protocol library used by yggdrasil-rs; it is not a standalone Yggdrasil daemon or Admin socket. MeshPlex therefore adapts the complete yggdrasil-rs node, not the protocol library in isolation.

The adapter defaults to tcp://127.0.0.1:9001. Configure the endpoint to match the daemon's Admin listener:

~~~rust
use meshplex_core::OverlayAdapter;
use meshplex_yggdrasil::{
    AdminEndpoint, YggdrasilConfig, YggdrasilGoAdapter, YggdrasilNgAdapter,
    YggdrasilRsAdapter,
};
use std::time::Duration;

let config = YggdrasilConfig {
    endpoint: AdminEndpoint::parse("tcp://127.0.0.1:9001")?,
    timeout: Duration::from_secs(3),
    ..YggdrasilConfig::default()
};

let go = YggdrasilGoAdapter::new(config.clone());
let rust_mesh = YggdrasilRsAdapter::new(config.clone());
let rust_ng = YggdrasilNgAdapter::new(config);

let snapshot = go.snapshot()?;
println!("Active peers: {:?}", snapshot.connected_peers);
~~~

On Unix, Go and yggdrasil-rs can also use an Admin Unix-domain socket when that transport is enabled by the build and daemon configuration:

~~~rust
let endpoint = AdminEndpoint::parse("unix:///var/run/yggdrasil.sock")?;
~~~

Yggdrasil-ng currently documents a TCP Admin listener, so use its configured TCP endpoint.

The Admin socket is privileged control-plane access and these protocols do not provide authentication on their own. Keep the listener on loopback or a tightly permissioned local Unix socket. MeshPlex never sends addPeer, removePeer, routing mutations, or remote debug requests.

The adapters expose **direct active peers**, not the entire DHT or every destination in the spanning tree. Every exported peer is classified as Routed, not Reachable. A peer's Admin latency is preserved as RTT where available; no packet-loss or application-throughput value is fabricated. The observed_at timestamp records when MeshPlex collected the response, not when the daemon last measured the metric.

## Design principles

- **Do not merge protocols.** CJDNS and Yggdrasil retain their own identities, cryptography, address spaces, and routing semantics.
- **Separate service identity from overlay identity.** A service can advertise multiple endpoints without pretending its overlay nodes share one key.
- **Prefer observation before control.** The adapters are read-only. Route changes require a separately designed and explicitly enabled integration.
- **Make freshness visible.** Missing timestamps and stale observations are ineligible, not silently treated as current.
- **Do not invent reachability.** A route-table entry or peer session is not a successful end-to-end application probe.
- **Measure before optimizing.** No performance advantage is claimed until reproducible benchmarks exist.
- **Keep protocol code reusable.** Adapters consume documented interfaces rather than forking protocol implementations.

## Security and privacy

A service identifier is a public-key digest, not proof that an endpoint advertisement is authentic. Signed advertisements, key rotation, revocation, and trust policy remain future work.

MeshPlex is not an anonymity system. It does not hide a user's source address from overlay peers or create trust between unrelated networks. Metrics and service advertisements can reveal network topology and activity.

No license has been selected for this repository yet. Do not copy code from CJDNS, Yggdrasil, or their Rust implementations without reviewing the source project's license and notices.
