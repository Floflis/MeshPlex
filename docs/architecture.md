# Architecture

## Goal

MeshPlex coordinates service endpoint choice across independent overlay networks. It is not a replacement routing protocol and does not combine overlay cryptographic identities or internal routing algorithms.

The first integration targets are CJDNS and Yggdrasil. Other overlays may be added behind the same protocol-neutral boundary.

## Service identity

A service identity is derived from a canonical application public key:

SHA-256("meshplex/service-id/v1\\0" || u64be(key_length) || public_key)

The resulting 32-byte ServiceId is stable across overlays. It is deliberately not derived from a CJDNS public key, Yggdrasil public key, overlay IPv6 address, or peering session.

A ServiceAdvertisement maps one ServiceId to explicitly advertised OverlayEndpoints. The same service can therefore publish a CJDNS endpoint and a Yggdrasil endpoint without implying that the two overlay nodes share the same identity.

The current advertisement is an in-memory mapping, not a signed wire format. A matching ServiceId does not prove endpoint ownership. Signed advertisements, key rotation, revocation, and trust/discovery mechanisms remain future work.

## Control plane and data plane

- **Overlay adapters** collect route observations from independent daemons.
- **Service advertisements** associate one application identity with its separately published overlay endpoints.
- **The join step** matches an advertised endpoint to a route observation on the same overlay and IP address.
- **The selector** chooses among fresh candidates for the requested ServiceId.
- **System integration** is a later, optional component that may apply a decision through a local proxy or operating-system routing.

The current implementation is a library-level control plane. It does not manipulate the host routing table or carry user traffic.

## Measurement freshness

Each RouteMetrics sample has an observed_at wall-clock timestamp set when the adapter collects it.

Selection rejects a candidate when the timestamp is missing, is in the future, or is older than the configured maximum age. The default maximum age is 30 seconds.

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

For each routing-table entry it records the CJDNS IPv6 destination, RouteState::Routed, the NodeStore link value as path_quality, and the local collection time as observed_at.

The NodeStore link value is a unitless route-quality score, not RTT, bandwidth, or packet loss. NodeStore_dumpTable does not provide a reliable connected-peer count, so connected_peers is None rather than a fabricated zero.

The adapter does not call RouterModule_pingNode, which is an authenticated operation. It does not change peers, interfaces, routes, or daemon settings. The default Admin endpoint is loopback UDP 127.0.0.1:11234; administrators should keep the privileged Admin interface tightly scoped.

## Yggdrasil adapters

The official yggdrasil-go daemon, yggdrasil-rs, and Yggdrasil-ng expose compatible newline-delimited JSON Admin request/response envelopes. The MeshPlex crate meshplex-yggdrasil uses read-only getSelf and getPeers requests for all three.

The Rust implementation yggdrasil-rs is published as the yggdrasil-mesh crate and uses petrified, a Rust port of the Ironwood routing protocol. petrified is not itself a complete Yggdrasil node or an Admin socket; the adapter talks to the complete yggdrasil-rs daemon.

Yggdrasil peer records have backend-specific details:
- Go encodes its time.Duration latency field as an integer number of nanoseconds.
- yggdrasil-rs reports peer latency in nanoseconds.
- Yggdrasil-ng currently reports its latency field in milliseconds and uses some different peer-field names.

The adapter converts those values to Duration without treating missing or zero latency as a measurement. It exports only up=true peers with valid IPv6 addresses and excludes the local node's own address.

Each exported peer is classified as Routed. The presence of a direct peer session does not establish that an application service is listening or reachable at the same address. The adapters do not infer packet loss, end-to-end throughput, or a full DHT/spanning-tree route table from peer statistics.

Admin access is privileged. The protocol does not provide authentication by itself, so the adapter defaults to loopback TCP and should be pointed only at a tightly scoped local socket. The Yggdrasil-ng implementation currently documents a TCP Admin listener; Go and yggdrasil-rs can also support Unix-domain sockets when enabled by their configuration/build.

The adapter does not send addPeer, removePeer, route mutations, remote debug queries, or other state-changing/admin requests.

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

No performance improvement is claimed by the current core. It ranks collected candidates and the adapters collect read-only local daemon observations; neither creates a multiplexed data path.

Benchmarks must compare one overlay against multiple independently running overlays and then against any future multiplexed data path. Measure CPU, resident memory, throughput, latency distributions, packet loss, convergence time, and energy use on constrained devices.
