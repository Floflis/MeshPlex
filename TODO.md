# TODO

## Completed in the initial bootstrap

- [x] Create a Rust workspace.
- [x] Define protocol-neutral overlay, endpoint, reachability, and metric types.
- [x] Define a read-only adapter contract.
- [x] Implement deterministic selection policies.
- [x] Add unit tests for route eligibility and selection.
- [x] Document current non-goals and security boundaries.
- [x] Add CI checks for formatting, Clippy, and tests.

## Next: foundations

- [ ] Decide and document the repository license before distributing releases.
- [ ] Define a stable service identifier and endpoint advertisement format.
- [ ] Add measurement timestamps and freshness limits to route observations.
- [ ] Add jitter-aware and cost-aware policies.
- [ ] Add policy hysteresis to avoid route flapping.
- [ ] Define an adapter conformance test suite.

## Overlay integrations

- [ ] Audit supported CJDNS admin/API interfaces and their authentication requirements.
- [ ] Audit supported Yggdrasil admin API and its authentication requirements.
- [ ] Implement read-only CJDNS snapshot adapter.
- [ ] Implement read-only Yggdrasil snapshot adapter.
- [ ] Test adapters against real daemon instances and document versions tested.
- [ ] Keep protocol-specific crates and licenses independent; do not copy code without review.

## System integration

- [ ] Design an opt-in local proxy integration.
- [ ] Design policy-routing integration for supported operating systems.
- [ ] Define safe rollback and recovery behavior before changing host routes.
- [ ] Evaluate gateway mode separately from endpoint selection.
- [ ] Document why this project is not an anonymity or exit-node system.

## Validation and performance

- [ ] Build reproducible single-overlay and dual-overlay benchmark scenarios.
- [ ] Measure CPU, memory, throughput, RTT, jitter, packet loss, convergence, and energy.
- [ ] Test peer churn, stale measurements, daemon restarts, and partial network failures.
- [ ] Test behavior on constrained hardware.
