# MeshPlex

**A Rust-native control plane for choosing paths across independent overlay networks.**

MeshPlex observes independent networks, associates their endpoints with application-level service identities, and selects a fresh route according to an explicit policy.

The first targets are **CJDNS** and **Yggdrasil**. Their protocol implementations, cryptographic identities, addressing schemes, and routing algorithms remain independent.

> Status: early development. Service identity, measurement freshness, a read-only CJDNS adapter, and a protocol-neutral Rust core are implemented. The Yggdrasil adapter and system-level routing integration are planned.

## Current implementation

- **Service identity:** a stable SHA-256 identifier derived from a canonical service public key, independent of overlay node keys.
- **Service advertisements:** map one service identity to explicitly advertised endpoints on one or more overlays.
- **Measurement freshness:** selector rejects missing, future-dated, and expired observations. The default maximum age is 30 seconds.
- **CJDNS adapter:** reads the paginated CJDNS NodeStore routing table through the local Admin UDP API.
- **Route selection:** deterministic policies for latency, throughput, packet loss, and preferred overlay.
- **CI:** formatting, Clippy, and unit tests.

The CJDNS adapter is read-only. It reports a route-table entry as Routed, not Reachable: the presence of a route is not an end-to-end application probe. Its path-quality value is CJDNS's unitless link score, not latency or throughput.

## What is not implemented

MeshPlex currently does **not**:

- Start, configure, or supervise overlay daemons.
- Implement CJDNS, CryptoAuth, Ironwood, or Yggdrasil protocols.
- Provide a signed service-advertisement or trust-distribution protocol.
- Implement the Yggdrasil adapter yet.
- Forward, bridge, translate, or encapsulate packets between overlays.
- Make an endpoint on one overlay reachable through another overlay.
- Change host routing tables or transparently migrate established TCP connections.
- Provide anonymity, an exit-node network, VPN functionality, or an Internet replacement.
- Claim a performance improvement before reproducible benchmarks exist.

The selector chooses only among endpoints explicitly associated with the requested service identity and backed by fresh overlay observations. It cannot infer that two unrelated addresses identify the same service.

## Workspace

- `crates/meshplex-core`: service identity, advertisements, route models, freshness checks, selection policies, and the adapter contract.
- `crates/meshplex-cjdns`: read-only CJDNS Admin UDP adapter.

## Build and test

Requires Rust 1.85 or newer.

~~~sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
~~~

## CJDNS adapter

The adapter defaults to `127.0.0.1:11234/UDP` and calls the read-only `NodeStore_dumpTable` Admin API function. It follows the API's page indicator and applies a configurable timeout and page limit.

~~~rust
use meshplex_cjdns::{CjdnsAdapter, CjdnsConfig};
use meshplex_core::OverlayAdapter;

let adapter = CjdnsAdapter::new(CjdnsConfig::default());
let snapshot = adapter.snapshot()?;

println!("Known CJDNS routes: {}", snapshot.known_routes);
~~~

The CJDNS Admin interface can expose state-changing operations. Keep it bound to loopback or another tightly controlled local endpoint. MeshPlex sends only the read-only route-table query; it does not authenticate or invoke state-changing functions.

CJDNS route-table entries expose a link-quality score and route label. MeshPlex currently retains the quality score and collection timestamp. It does not claim an RTT measurement or count route-table entries as active peers.

## Yggdrasil adapter plan

The first Yggdrasil adapter will talk to the official **yggdrasil-go** Admin JSON socket, using its documented newline-delimited JSON protocol and read-only requests such as `getPeers` and `getSelf`.

The adapter boundary is intentionally independent of the daemon implementation. A later native Rust backend can be evaluated using `petrified` and `yggdrasil-mesh` without changing service identity or route-selection policy.

## Design principles

- **Do not merge protocols.** CJDNS and Yggdrasil retain their own identities, cryptography, address spaces, and routing semantics.
- **Separate service identity from overlay identity.** A service can advertise multiple endpoints without pretending its overlay nodes share one key.
- **Prefer observation before control.** The adapters are read-only. Route changes require a separately designed and explicitly enabled integration.
- **Make freshness visible.** Missing timestamps and stale observations are ineligible, not silently treated as current.
- **Do not invent reachability.** A route-table entry is not a successful end-to-end probe.
- **Measure before optimizing.** No performance advantage is claimed until reproducible benchmarks exist.
- **Keep protocol code reusable.** Adapters consume documented interfaces rather than forking protocol implementations.

## Security and privacy

A service identifier is a public-key digest, not proof that an endpoint advertisement is authentic. Signed advertisements, key rotation, revocation, and trust policy remain future work.

MeshPlex is not an anonymity system. It does not hide a user's source address from overlay peers or create trust between unrelated networks. Metrics and service advertisements can reveal network topology and activity.

No license has been selected for this repository yet. Do not copy code from CJDNS, Yggdrasil, or their Rust implementations without reviewing the source project's license and notices.
