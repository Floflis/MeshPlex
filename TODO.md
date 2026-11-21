# TODO

## Completed
- [x] Rust workspace, protocol-neutral route models, service identity, and cross-overlay endpoint mapping.
- [x] Freshness checks and deterministic route-selection policies.
- [x] Read-only CJDNS adapter (route-quality kept distinct from RTT).
- [x] Read-only Yggdrasil adapters for Go, yggdrasil-rs, and Yggdrasil-ng.
- [x] Normalize peers as Routed and preserve RTT only when available.
- [x] CI format, Clippy, workspace tests.
- [x] Ed25519 signed advertisements with deterministic canonical encoding and validity windows.

## Service identity/discovery
- [ ] Trust policy, key rotation, revocation, ownership semantics.
- [ ] Overlay-neutral advertisement distribution/discovery.
- [ ] Multi-algorithm key types and stable external serialization/versioning.

## Measurements and policy
- [ ] Per-metric timestamps, monotonic freshness, hysteresis, route-flap protection.
- [ ] Policy options for jitter/cost/admin preference/service type.
- [ ] Adapter conformance and stale-data integration tests.
- [ ] Optional authenticated probes only after safety/auth design.

## Overlay integration
- [ ] Live daemon integration tests for Yggdrasil implementations.
- [ ] Comparable route-table observations and tested version records.

## Routing/system integration
- [ ] Opt-in local proxy; OS policy routing.
- [ ] Adaptive selection and failover for new flows.
- [ ] Connection migration evaluated separately.
- [ ] Rollback/recovery before host-route modifications.
- [ ] Gateway and cross-overlay forwarding design.

## Validation
- [ ] Reproducible single/dual-overlay benchmarks and CPU/memory/throughput/latency/jitter/loss/convergence/energy metrics.
- [ ] Peer churn, stale data, daemon restart, partial failures, constrained devices.
- [ ] Select repository license before release.

## Release targets and portability
- [ ] Windows builds/exports (native Windows targets).
- [ ] Termux on Android, specifically Galaxy S20 FE (Exynos 4G SM-G780F).
- [ ] ARMv7 targets.
- [ ] armhf (32-bit ARM hard-float).
- [ ] RISC-V targets.
- [ ] Establish CI build matrix, packaging, and smoke tests for each target; distinguish build support from tested runtime support.
