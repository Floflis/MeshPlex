# Architecture

## Goal

MeshPlex coordinates service endpoint choice across independent overlay networks. It is not a replacement routing protocol and does not combine overlay cryptographic identities or internal routing algorithms.

The first integration targets are CJDNS and Yggdrasil. Other overlays may be added behind the same protocol-neutral boundary.

## Service identity

A service identity is derived from a canonical application public key:

`SHA-256("meshplex/service-id/v1\\0" || u64be(key_length) || public_key)`

The resulting 32-byte ServiceId is stable across overlays. It is deliberately not derived from a CJDNS public key, Yggdrasil public key, overlay IPv6 address, or peering session.

A ServiceAdvertisement maps one ServiceId to explicitly advertised OverlayEndpoints. The same service can therefore publish a CJDNS endpoint and a Yggdrasil endpoint without implying that the two overlay nodes have the same identity.

The current advertisement is an in-memory mapping, not a signed wire format. A matching ServiceId does not prove endpoint ownership. Signed advertisements, key rotation, revocation, and trust/discovery mechanisms remain future work.

## Control plane and data plane

The project separates decisions from packet transport:

- **Overlay adapters** collect route observations from independent daemons.
- **Service advertisements** associate one application identity with explicitly published overlay endpoints.
- **The join step** matches an advertised endpoint to a route observation on the same overlay and IP address.
- **The selector** chooses among fresh candidates for the requested ServiceId.
- **System integration** is a later, optional component that may apply a decision through a local proxy or operating-system routing.

The current implementation is a library-level control plane. It does not manipulate the host routing table or carry user traffic.

## Measurement freshness

Each RouteMetrics sample has an observed_at wall-clock timestamp set when the adapter collects it.

Selection rejects a candidate when:
- The timestamp is missing.
- The timestamp is in the future relative to the selector.
- The sample is older than the configured maximum age.

The default maximum age is 30 seconds. Callers may use select_at with an explicit time and age limit for deterministic tests or a different policy.

This timestamp describes when the adapter observed the data, not necessarily when the underlying overlay last measured it. Adapters must not relabel old daemon metrics as newly measured values. Future versions may add per-field timestamps, monotonic durations, and stale-data policies.

## Route state semantics

- **Reachable:** an active end-to-end probe succeeded.
- **Routed:** the overlay currently has a route to the destination; no end-to-end probe is implied.
- **Degraded:** the adapter has evidence that the path is usable but degraded.
- **Unreachable:** a check found the path unavailable.
- **Unknown:** the adapter cannot establish the current state.

Reachable ranks ahead of Routed, and Routed ahead of Degraded. Unknown and Unreachable are ineligible.

## CJDNS adapter

The CJDNS adapter talks to the CJDNS Admin interface over UDP. It sends the read-only NodeStore_dumpTable request, decodes the bencoded response, and follows the paginated more indicator.

For each routing-table entry it records:
- The CJDNS IPv6 destination.
- RouteState::Routed.
- The NodeStore link value as path_quality.
- The local collection time as observed_at.

The NodeStore link value is a unitless route-quality score, not RTT, bandwidth, or packet loss. NodeStore_dumpTable does not provide a reliable connected-peer count, so connected_peers is None rather than a fabricated zero.

The adapter does not call RouterModule_pingNode, which is an authenticated operation. It does not change peers, interfaces, routes, or daemon settings. The default Admin endpoint is loopback UDP 127.0.0.1:11234; administrators should keep the privileged Admin interface tightly scoped.

## Yggdrasil adapter plan

The first adapter will target the official yggdrasil-go Admin JSON socket. The documented socket accepts newline-delimited JSON requests over local TCP or Unix-domain socket transports. Initial read-only requests will include getPeers and getSelf.

The adapter API will remain protocol-neutral. A later native Rust backend can be evaluated using petrified and yggdrasil-mesh, while retaining the same MeshPlex service identity and selection model.

## Failover and connection continuity

Selecting a different endpoint for a new connection is distinct from migrating an existing connection.

The selector can return a new preferred endpoint when fresh observations change. It does not preserve a TCP session, replay application data, or transparently migrate an established flow. Those capabilities require a transport-aware proxy or a separately designed connection layer.

Adaptive routing, hysteresis, failover, and load balancing remain future work. They must use fresh measurements and avoid route flapping.

## Security boundaries

- Keep overlay keys, identities, and trust relationships separate.
- Treat service advertisements as untrusted until a signature/trust mechanism exists.
- Do not claim anonymity or concealment.
- Do not enable forwarding between overlays by default.
- Do not modify system routes until a dedicated, audited integration exists.
- Treat service advertisements and telemetry as potentially sensitive.
- Make privileged operations explicit and minimize privileges.

## Performance claims

No performance improvement is claimed by the current core. It ranks collected candidates and the CJDNS adapter performs a local UDP Admin query; neither creates a multiplexed data path.

Benchmarks must compare one overlay against multiple independently running overlays and then against any future multiplexed data path. Measure CPU, resident memory, throughput, latency distributions, packet loss, convergence time, and energy use on constrained devices.
