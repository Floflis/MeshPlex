# MeshPlex Roadmap

> Status: early development. This roadmap distinguishes implemented read-only foundations from planned system integration. A checkbox must not be closed based on design intent alone.

## Direction

Build a Rust-native control plane for choosing paths across independent overlay networks. MeshPlex observes overlays and selects routes for application service identities; it does not merge overlay protocols or claim to implement their transports.

## 1. Identity and signed service records
- [x] Derive ServiceId from canonical public-key bytes.
- [x] Define signed service advertisements with deterministic endpoint encoding and validity timestamps.
- [x] Filter stale advertisements and expose route-selection policies.
- [ ] Define and test key rotation, revocation, and identity migration.
- [ ] Specify trust policy separately from signature validity and endpoint reachability.

## 2. Overlay adapters
- [x] Read CJDNS NodeStore data.
- [x] Read Yggdrasil administration data through the documented Go, yggdrasil-rs, and Yggdrasil-ng adapters.
- [ ] Validate adapter behavior against supported upstream versions and malformed/stale input.
- [ ] Add capability/version detection and clear diagnostics for unavailable or incompatible adapters.
- [ ] Preserve read-only boundaries until write operations have explicit safety contracts.

## 3. Dynamic topology and route lifecycle
- [ ] Detect interface and address changes without requiring a daemon restart.
- [ ] Refresh route candidates when overlay peers, interfaces, or endpoint state changes.
- [ ] Re-evaluate listening capability and endpoint reachability after topology changes.
- [ ] Re-advertise updated service endpoints to configured discovery/trackers.
- [ ] Define expiry, withdrawal, and recovery behavior when an endpoint disappears.
- [ ] Test rapid churn, partial failures, duplicate events, and stale advertisements.

## 4. Route selection and system integration
- [ ] Define a platform abstraction for observing routes and applying host-route changes.
- [ ] Implement dry-run and read-only diagnostics before enabling route mutation.
- [ ] Add guarded route application, rollback, and recovery behavior.
- [ ] Define connection migration and interruption semantics before claiming seamless failover.
- [ ] Keep forwarding, gateway, VPN, and anonymity features out of scope unless separately specified.

## 5. Althea-compatible routing payments
- [ ] Specify the accounting model for paid routing and the trust boundary between measurement, settlement, and forwarding.
- [ ] Design support for both backhaul/provider routing and user-to-user routing.
- [ ] Define metering units, session identity, quote/acceptance flow, settlement lifecycle, dispute handling, and failure behavior.
- [ ] Prototype payment integration behind an optional interface; routing must remain understandable and testable without payment services.
- [ ] Add simulations and tests before enabling real-value settlement.

## 6. Portability and release targets
- [ ] Establish a target matrix for Linux, Windows, Android/Termux, ARM variants, and RISC-V.
- [ ] Investigate Redox on RISC-V as a distinct OS/architecture target; do not infer support from Linux RISC-V or other Redox builds.
- [ ] Track compile, link, test, and runtime evidence separately for every target.
- [ ] Add reproducible packaging and artifact checksums for supported releases.

## 7. Operations and observability
- [ ] Define daemon supervision, configuration format, and graceful shutdown.
- [ ] Add structured logs and diagnostics for adapters, topology events, route decisions, and advertisements.
- [ ] Provide a status command that explains why a candidate route was accepted or rejected.
- [ ] Document permissions and safety implications of any system-level integration.

## Completion policy

Each completed task must link to its PR, commit, and evidence of tests or platform validation. Keep implemented, experimental, and planned capabilities visibly separate. Do not close a checkbox merely because an adapter compiles or a design document exists.
