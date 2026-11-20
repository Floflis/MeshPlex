# Architecture

## Goal

MeshPlex coordinates endpoint choice across independent overlay networks. It is not a replacement routing protocol and does not combine their cryptographic identities or internal routing algorithms.

The initial integration targets are CJDNS and Yggdrasil. Other overlays may be added behind the same protocol-neutral boundary.

## Control plane and data plane

The project separates decisions from packet transport:

- **Adapters** collect observations from an overlay implementation.
- **Service discovery** maps a logical service identifier to the endpoints that the service explicitly advertises on each overlay.
- **The selector** ranks eligible endpoints according to operator policy.
- **System integration** is a later, optional component that applies a chosen endpoint through a local proxy or operating-system routing.

The first release is a library-level control-plane core. It does not manipulate the host routing table or carry user traffic.

## Important distinction: two endpoints are not one route

A service may publish one endpoint on CJDNS and another on Yggdrasil. MeshPlex can compare those paths only after it has evidence that both endpoints refer to the same service.

The presence of an address on one overlay does not imply reachability through another overlay. Cross-overlay access requires the service to be present on both networks, or an explicitly configured gateway or proxy.

## Adapter boundary

The initial OverlayAdapter contract is read-only and returns an OverlaySnapshot containing:

- The overlay identifier.
- Connected and known peer counts.
- Route candidates with endpoint, reachability state, and measurements.

Adapters must report the source and freshness of measurements in their future implementations. A missing measurement is unknown, not zero.

The first adapters should use documented administrative or local interfaces. They should not parse unstable internal state or modify daemon configuration.

## Selection rules

The initial policies are:

- Lowest latency.
- Highest throughput.
- Lowest packet loss.
- Prefer a configured overlay.

Only Reachable and Degraded candidates are eligible. Reachable candidates rank before Degraded candidates. Unknown and Unreachable candidates are excluded.

Missing measurements rank behind known measurements. Ties use deterministic secondary metrics and endpoint ordering so tests and operators can reproduce decisions.

These rules are a starting point, not a claim that one metric captures application quality. Future policies may include jitter, cost, administrative constraints, service type, measurement age, and hysteresis.

## Failover and connection continuity

Selecting a different endpoint for a new connection is distinct from migrating an existing connection.

The initial selector can return a new preferred endpoint when observations change. It does not preserve a TCP session, replay application data, or transparently migrate an established flow. Those capabilities would require a transport-aware proxy or a separately designed connection layer.

Failover must also use fresh reachability data. A stale successful probe must not keep a dead route eligible indefinitely.

## Security boundaries

- Keep overlay keys, identities, and trust relationships separate.
- Do not claim anonymity or concealment.
- Do not enable forwarding between overlays by default.
- Do not modify system routes until a dedicated, audited integration exists.
- Treat service advertisements and telemetry as potentially sensitive.
- Make privileged operations explicit and minimize privileges.

## Performance claims

No performance improvement is claimed by the initial core. It only ranks already collected candidates and does not send packets.

Benchmarks must compare one overlay against multiple independently running overlays and then against any future multiplexed data path. Measurements should include CPU, resident memory, throughput, latency distributions, packet loss, convergence time, and energy use on constrained devices.
