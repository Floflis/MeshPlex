# TODO

## Completed

- [x] Create a Rust workspace.
- [x] Define protocol-neutral overlay endpoints and route observations.
- [x] Derive a stable ServiceId from a canonical public identity key.
- [x] Map one service identity to endpoints on multiple independent overlays.
- [x] Join service advertisements to matching overlay/address observations.
- [x] Reject missing, future-dated, and expired measurements during selection.
- [x] Add deterministic selection policies for latency, throughput, packet loss, and preferred overlay.
- [x] Implement a read-only CJDNS Admin UDP adapter using paginated NodeStore_dumpTable.
- [x] Keep CJDNS route-table quality separate from RTT and peer-count metrics.
- [x] Add unit tests and CI checks for formatting, Clippy, and workspace tests.

## Service identity and discovery

- [ ] Define a signed service-advertisement format.
- [ ] Define trust, key rotation, revocation, and service-key ownership rules.
- [ ] Define advertisement distribution/discovery without coupling it to one overlay.
- [ ] Decide whether service identity keys should support multiple algorithms and explicit key-type identifiers.

## Measurement and policy

- [ ] Add per-metric timestamps if adapters expose measurements with different collection times.
- [ ] Evaluate monotonic-age tracking alongside wall-clock timestamps.
- [ ] Add hysteresis and route-flap protection.
- [ ] Add policy options for jitter, cost, administrative preference, and service type.
- [ ] Add adapter conformance tests and stale-data integration tests.
- [ ] Add optional authenticated CJDNS active probes; keep them disabled unless explicitly configured.

## Overlay integrations

- [ ] Implement a read-only Yggdrasil adapter using the official yggdrasil-go Admin JSON socket first.
- [ ] Test the Go adapter against documented getPeers and getSelf response formats.
- [ ] Later evaluate a native Rust backend using petrified and yggdrasil-mesh without changing the adapter contract.
- [ ] Document daemon versions tested for every adapter.
- [ ] Keep protocol-specific crates and licenses independent; do not copy code without review.

## System integration and adaptive routing

- [ ] Design an opt-in local proxy integration for new connections.
- [ ] Design policy-routing integration for supported operating systems.
- [ ] Implement adaptive path selection and failover for new connections.
- [ ] Evaluate connection migration separately from endpoint failover.
- [ ] Define safe rollback and recovery before changing host routes.
- [ ] Evaluate gateway mode and cross-overlay forwarding separately.

## Validation and performance

- [ ] Build reproducible single-overlay and dual-overlay benchmark scenarios.
- [ ] Measure CPU, memory, throughput, RTT, jitter, packet loss, convergence, and energy.
- [ ] Test peer churn, stale measurements, daemon restarts, and partial network failures.
- [ ] Test behavior on constrained hardware.

## Project housekeeping

- [ ] Decide and document the repository license before distributing releases.
