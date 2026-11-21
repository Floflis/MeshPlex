# Peer Routing Economy: Study and Design Notes

**Status:** research/design document. This is not an implementation specification and does not claim that any economic feature is implemented.

**Repository:** [Floflis/MeshPlex](https://github.com/Floflis/MeshPlex)  
**Last updated:** 2026-11-21

## 1. Executive summary

This study explores how MeshPlex could support an optional economic coordination layer for networks using independent overlays such as CJDNS and Yggdrasil. The target is broader than a conventional ISP billing system:

- **Backhaul-to-network:** a router, gateway, community operator, upstream provider, or other capacity supplier sells transit/connectivity to downstream peers.
- **Router-to-router (R2R):** intermediate peers are paid for forwarding traffic for neighboring peers, hop by hop.
- **User-to-user (U2U):** end-user devices or home routers can buy, sell, relay, or exchange connectivity directly, without every transaction passing through a central ISP/operator.
- **Gateway and exit services:** peers can sell Internet transit, remote egress, or other explicitly described services.
- **Return traffic:** the model accounts for both directions. A peer may pay to send a request while another peer is paid to carry the response.

The central Althea idea worth studying is **pay-for-forward**: nodes pay the next-hop neighbors that actually forward their packets. A route's total forwarding cost is composed from the prices charged by its forwarding hops. Althea combines this with price-aware routing and machine-to-machine micropayments. MeshPlex can study these ideas without becoming Rita, copying Althea's deployment model, or requiring Althea's blockchain.

The essential architectural distinction is:

> MeshPlex currently observes independent overlays, binds explicit endpoints to service identities, and selects paths using fresh measurements. It does not merge overlay protocols, forward packets, or mutate host routes. A system that pays for forwarding requires a separate data-plane/metering integration. Route-selection observations alone cannot prove that traffic was forwarded.

This document is exploratory. The existing MeshPlex README and architecture document remain authoritative for implemented behavior.

## 2. Existing MeshPlex boundary

The repository currently describes MeshPlex as a Rust-native control plane for path choice across independent overlay networks.

Current documented capabilities:
- Service identity derived from canonical public-key bytes.
- Signed service advertisements with Ed25519 signatures and validity intervals.
- Freshness filtering and route-selection policies.
- Read-only CJDNS NodeStore observation.
- Read-only Yggdrasil Admin adapters for Go Yggdrasil, yggdrasil-rs, and Yggdrasil-ng.

Current documented non-goals / unimplemented areas:
- Overlay protocol implementation or protocol merging.
- Daemon supervision and system integration.
- Forwarding or gateway operation.
- Host-route mutation and connection migration.
- Discovery, trust lifecycle, key rotation, and revocation.
- A stable external serialization format.

The economic layer discussed here must respect those boundaries. It should be an optional subsystem or a separately versioned companion component. It must not silently turn read-only adapters into packet forwarders.

### 2.1 Responsibility map

| Concern | Candidate responsibility |
|---|---|
| Observe CJDNS/Yggdrasil state | Existing MeshPlex overlay adapters |
| Bind service identities to advertised endpoints | MeshPlex identity/advertisement layer |
| Choose among candidate paths | MeshPlex selector |
| Discover and establish a forwarding relationship | Future peer-session / integration component |
| Forward packets | Overlay, tunnel, OS networking, or dedicated data plane, not the selector |
| Measure bytes attributable to a peer | Metering integration at a trusted forwarding boundary |
| Calculate amounts owed | Economic accounting component |
| Negotiate/hold payment collateral | Payment backend or bilateral channel |
| Settle balances | Pluggable settlement backend |
| Present offers, balances, receipts, and controls | Optional user/operator interface |

These are conceptual boundaries. Names and crate boundaries are not prescribed.

## 3. Althea and Rita: reusable concepts

Althea is a useful reference because it treats connectivity as a market among network devices rather than only as a retail subscription sold by a central ISP.

The Rita repository describes Rita as coordination, management, routing integration, and billing software for the Althea Telecom Stack. Its documented approach includes:

1. Discover neighboring routers on physical interfaces.
2. Establish authenticated WireGuard tunnels between peers.
3. Track traffic on those tunnels to calculate what each neighbor owes or is owed.
4. Run Babel over authenticated tunnels, with a price-aware routing extension.
5. Interact with exit services and a blockchain payment backend.
6. Manage wallets, sign/publish transactions, and monitor settlement.

Althea's price-aware routing model separates monetary price from conventional path-quality metrics such as packet loss and latency. A router can choose a path using both quality and price, according to a configured preference.

Althea's whitepaper describes the key economic rule as paying neighbors for forwarding packets. A route can therefore be priced as the sum of the forwarding prices of its hops. Althea also describes payment channels and mechanisms for paying for response traffic.

### 3.1 What should not be copied blindly

Althea's production stack is designed around a particular telecom workflow: user routers, relays, gateways, Internet exits/backhaul, managed firmware, and operator tooling. That is a valuable deployment model, but not the only possible network economy.

MeshPlex should not assume:
- Every network has a single ISP-like operator.
- Every peer is a customer or relay in a fixed hierarchy.
- Every route ultimately terminates at an Internet gateway.
- Every peer uses WireGuard as its transport.
- Every network runs Babel.
- Every peer has a blockchain wallet or can afford on-chain transactions per accounting event.
- CJDNS and Yggdrasil identities are natively interchangeable.
- A signed route advertisement proves that traffic was actually forwarded.
- An advertised price is necessarily honest, available, or competitive.

Reusable concepts include pay-for-forward, neighbor-level accounting, price-aware path selection, bidirectional accounting, and machine-to-machine settlement. Protocol and deployment assumptions must be evaluated independently.

## 4. Economic roles: avoid a rigid ISP hierarchy

A peer may occupy several roles at once, and its role may change over time.

| Role | What it offers | What it may consume |
|---|---|---|
| End-user peer | Local services, optional transit/relay capacity | Transit, forwarding, gateway/exit |
| Relay peer | Forwarding to one or more next hops | Upstream forwarding, backhaul |
| Backhaul provider | High-capacity upstream connectivity | Local aggregation, peering, maintenance |
| Gateway peer | A route between an overlay/community network and another network | Upstream transit, relay capacity |
| Exit peer | Remote Internet egress or another external service | Gateway/backhaul and intermediate forwarding |
| Service provider | A named application endpoint or resource | Transit and hosting resources |

A residential router could be both a user and a relay. A community operator could sell backhaul to several independent mesh clusters. Two end users could exchange transit directly. A device could buy one service and sell another.

The accounting model should represent **relationships and services**, not hard-code a single chain of customer → ISP → backbone.

## 5. Traffic and payment model

### 5.1 Pay the actual forwarding neighbor

For a path:

**source → peer A → peer B → peer C → destination**

the source's traffic may incur forwarding charges at A, B, and C. Each hop accounts for the traffic it actually forwarded for its adjacent customer/peer, according to a negotiated tariff and accounting policy.

This differs from charging only the source for an abstract end-to-end route. Hop-by-hop accounting lets each relay earn revenue for its contribution and pay its upstream neighbor.

Conceptually:

**total route cost = sum(per-hop forwarding charges) + explicitly priced service charges**

Do not count the same byte twice at the same accounting boundary. Define whether a meter counts payload bytes, tunnel bytes, IP bytes, or link-layer bytes, and use the same definition in the bilateral agreement.

### 5.2 Backhaul payments

Backhaul is not merely another user subscription. It may be a wholesale capacity relationship with:
- A price per byte/GB, per time interval, per committed capacity unit, or a hybrid tariff.
- Capacity and rate limits.
- Availability/SLA terms, if offered.
- Burst and congestion rules.
- Direction-specific rates.
- Minimum commitments, prepaid credit, or postpaid settlement.
- A clearly identified service boundary and upstream provider.

A backhaul provider may serve an entire community network or several peers independently. A mesh may have multiple upstream providers and choose between them using cost, quality, capacity, policy, and trust.

A capacity reservation is not the same as actual forwarded traffic. Distinguish reserved capacity, measured usage, and service availability.

### 5.3 User-to-user payments

A user-to-user relationship should be a first-class case, not an exception hidden inside an ISP account.

Example: **User A → User B → User C**.

If B forwards traffic for A, B can earn a forwarding fee. If B also consumes upstream forwarding to reach C, B may owe C. A and B can have a direct bilateral relationship even when neither is an operator.

The model should support:
- A peer acting as buyer and seller simultaneously.
- Direct neighbor-to-neighbor agreements.
- Dynamic peers and changing connectivity.
- Small balances and low-value transactions.
- Prepaid, credit-limited, or postpaid relationships.
- Per-service and per-direction tariffs.
- A peer leaving the network without invalidating historical receipts.

### 5.4 Bidirectional traffic and response payments

A common mistake is to count only traffic from the requester toward a destination. Real applications exchange traffic in both directions.

The accounting model must distinguish:
- Bytes forwarded from A to B.
- Bytes forwarded from B to A.
- Which peer is the buyer for each direction.
- Which service or flow the traffic belongs to.
- Whether the return path is symmetric or uses different peers.
- Whether the receiver/exit is also paying upstream peers to deliver response traffic.

A request and its response may traverse different paths. Do not assume that the reverse direction uses the same relays or has the same price. Account for each forwarding relationship independently.

Althea's published material discusses paying for response traffic through its exit-node model. MeshPlex's study should generalize the principle: **every direction has its own forwarding path, service boundary, and accounting evidence**.

## 6. Price-aware path selection

A price-aware selector should compare more than a single price number.

Candidate inputs:
- Monetary price per defined unit (for example, per MiB or GB).
- Latency and jitter, when measured.
- Packet loss, when measured.
- Throughput/capacity, when measured.
- Peer and service availability.
- Freshness of the measurement.
- Trust and policy constraints.
- Maximum spend, prepaid balance, or credit limit.
- User/operator preference between cost and quality.

A possible conceptual objective is:

**score(path) = f(price, latency, loss, capacity, freshness, trust, policy)**

This is deliberately not a prescribed formula. The selector must not fabricate measurements or compare incompatible units. Price should be normalized to a declared tariff unit and direction before comparison.

A low price must not override a hard security or trust policy. A route that is cheap but stale, unreachable, or outside the user's policy should be excluded or explicitly marked degraded/unknown.

### 6.1 Price advertisement versus contract

An advertised price is an offer, not proof of a contract. Before traffic is charged, peers need a mutually understood agreement containing at least:
- Buyer and seller identities.
- Service/forwarding scope.
- Direction and accounting unit.
- Price and currency/asset identifier.
- Validity period and version.
- Limits, caps, and optional minimums.
- Payment method and settlement conditions.
- Dispute/expiry behavior.
- Signatures or equivalent authenticated acceptance.

A route selector may use an offer as an input, but should not imply that a payment obligation exists until the relevant agreement has been accepted.

## 7. Metering: the hard part

A cryptographic signature proves that a key signed a record and that the record was not modified. It does **not** prove that the bytes in the record were actually forwarded.

Similarly:
- A route advertisement does not prove reachability.
- A peer-reported byte counter is not automatically trustworthy.
- A tunnel counter may include protocol overhead or traffic unrelated to the billable service.
- A source transmit counter and a receiver receive counter may differ because of loss, retransmission, buffering, and measurement boundaries.
- A malicious peer can over-report, under-report, replay, or selectively omit records unless the protocol prevents or detects it.

A production-grade economic layer needs a defined metering boundary and threat model.

### 7.1 Metering boundary

Possible integration points include:
- Per-peer authenticated tunnel counters.
- A forwarding daemon or data-plane hook.
- OS/kernel traffic accounting with stable peer attribution.
- A service proxy that can attribute bytes to a session.
- A dedicated gateway/exit accounting boundary.

The correct point depends on transport and forwarding architecture. MeshPlex's current read-only CJDNS/Yggdrasil adapters do not expose enough evidence to bill forwarded traffic by themselves.

### 7.2 Bilateral accounting

For each neighbor relationship, maintain separate directional counters and records. A useful conceptual record includes:
- Protocol/schema version and domain separator.
- Session/agreement identifier.
- Buyer and seller peer identifiers.
- Overlay and endpoint context, without assuming overlay identities are interchangeable.
- Direction.
- Monotonic sequence number.
- Accounting interval or cumulative counter checkpoint.
- Counter unit and metering-point definition.
- Byte count and tariff version.
- Timestamp/validity interval.
- Previous-record hash or checkpoint linkage.
- Signature/authentication material.

The record format must be canonical and versioned before interoperability is promised. Avoid putting private user activity, destination browsing history, or unnecessary flow metadata into public records.

### 7.3 Reconciliation and disputes

A robust design should compare evidence from both sides where possible:
- Forwarder's transmit/egress count.
- Neighbor's receive/ingress count.
- Session identifiers and monotonic checkpoints.
- A bounded tolerance for known protocol overhead, loss, or counter timing.
- Sampling or challenge mechanisms for suspicious discrepancies.
- A dispute window before irreversible settlement, when appropriate.

No single measurement method eliminates collusion or compromised endpoints. The goal is to make fraud detectable, limit exposure, and define what evidence is accepted under each deployment's trust model.

### 7.4 Replay, double counting, and resets

The accounting protocol must address:
- Replayed receipts.
- Duplicate settlement submissions.
- Counter wraparound and reset after reboot.
- Device replacement and key rotation.
- Clock drift and stale timestamps.
- Interrupted sessions.
- Peers disappearing before final reconciliation.
- Traffic crossing multiple tunnels or interfaces.
- Route changes while a flow is active.
- A byte being observed at more than one accounting layer.

Use monotonic sequence numbers and session epochs. Treat counters as scoped to an identified session and metering point. Do not assume wall-clock time alone is sufficient for ordering.

## 8. Payments and settlement

The economic protocol should be independent of any one chain or payment network. A pluggable backend makes it possible to use different settlement mechanisms according to hardware, network, transaction size, jurisdiction, and risk tolerance.

### 8.1 Candidate models

| Model | Strengths | Trade-offs / questions |
|---|---|---|
| Prepaid balance | Simple exposure control; easy to understand | Requires top-ups and a balance service or local wallet |
| Bilateral credit | Can operate with very low overhead | Requires limits, reputation/trust, and default handling |
| Payment channels | Efficient repeated micropayments between peers after setup | Channel liquidity, opening/closing, routing and recovery complexity |
| On-chain micropayments | Transparent settlement and composability | Fees, confirmation latency, chain availability, privacy |
| Periodic net settlement | Batches many tiny usage records into fewer payments | Requires reconciliation and counterparty credit exposure |
| Operator clearing/ledger | Practical for managed deployments | Introduces an operator/trust domain; must not be confused with permissionless settlement |

A sensible design may combine local metering and signed receipts with periodic settlement. A payment channel or other off-chain mechanism can handle frequent small updates, while an on-chain backend settles channel state or net balances.

### 8.2 Althea as one backend/reference, not a mandatory dependency

Rita documents support for Althea L1 or EVM-compatible blockchain payment backends. Current Althea documentation describes Althea L1 as a Cosmos SDK / hybrid EVM chain intended for machine-to-machine infrastructure payments.

This makes Althea a useful integration target and reference implementation to study. It does not mean MeshPlex must depend on Althea L1, use its token, or embed Rita.

The study should compare:
- Althea L1 / EVM-compatible settlement.
- Generic EVM networks and stablecoin settlement.
- Payment-channel designs.
- Other interoperable payment/clearing mechanisms.
- Offline/prepaid or local-credit modes for peers without reliable chain access.

Do not select a backend until fees, finality, liquidity, wallet UX, recovery, privacy, and operational requirements have been evaluated.

### 8.3 Settlement is not metering

Keep these stages distinct:

1. Observe/measure forwarding.
2. Produce authenticated accounting evidence.
3. Reconcile bilateral records.
4. Calculate a balance under the accepted tariff.
5. Authorize or dispute the balance.
6. Settle through a backend.
7. Record settlement status and prevent duplicate payment.

A successful blockchain transaction does not prove that the underlying network service was delivered. Likewise, a valid usage receipt does not mean the payment has settled.

## 9. CJDNS and Yggdrasil considerations

CJDNS and Yggdrasil are independent overlay networks. MeshPlex should continue to treat them as independent domains.

- A CJDNS peer identity/address is not automatically a Yggdrasil identity.
- An endpoint observed on one overlay must not be silently reinterpreted as an endpoint on another.
- A peer may expose the same service through more than one overlay, but the binding between those endpoints needs an explicit, authenticated service identity and policy.
- Routing or accounting across overlays requires an explicit bridge, gateway, tunnel, or other transport relationship. It is not native protocol interoperability.
- A payment relationship should identify the actual authenticated neighbor/session and service boundary, not rely only on an IP address.

The current MeshPlex adapters are read-only observers. They can inform candidate-path discovery and freshness, but they are not forwarding meters. Any future adapter that supports paid forwarding needs a separately reviewed integration with the component that actually carries the traffic.

## 10. Candidate MeshPlex architecture

The following is a conceptual decomposition for study. It does not assert that these crates or APIs exist.

### 10.1 Peer Economy / R2R layer

Responsibilities could include:
- Peer offers and tariffs.
- Bilateral service agreements.
- Peer-level accounts and exposure limits.
- Directional usage records.
- Receipt signing, verification, and reconciliation.
- Balance calculation.
- Payment backend abstraction.
- Settlement status and audit trail.

The layer should be optional. MeshPlex path selection must remain useful when no economic backend is configured.

### 10.2 Metering adapters

A metering adapter would connect a real forwarding boundary to the accounting model. Each adapter must document:
- What bytes it measures.
- Which peer/session those bytes belong to.
- Whether counters are directional.
- How resets and restarts are detected.
- What overhead is included.
- What trust assumptions apply.
- Whether the adapter can be independently cross-checked.

Do not label a read-only overlay observation as a traffic meter.

### 10.3 Tariff and offer model

An offer should be machine-readable, signed, versioned, and explicit about units. Avoid a bare price field without a currency/asset, direction, unit, and validity period.

Potential tariff dimensions:
- Price per byte/MB/GB.
- Price per time or reserved capacity.
- Direction-specific pricing.
- Service class or priority.
- Congestion or burst policy.
- Minimum charge, cap, and expiration.

### 10.4 Settlement backend interface

A backend abstraction should support capabilities rather than assume all backends behave alike. Candidate operations:
- Discover supported asset/network and fee information.
- Check balance and spendable liquidity.
- Open/close or fund a channel, if supported.
- Authorize a payment or credit reservation.
- Submit a settlement.
- Query transaction status.
- Recover or reconcile pending operations.
- Report unsupported capabilities explicitly.

Backend errors, pending states, reorgs, and timeouts must be visible. A timeout must not trigger a blind duplicate payment.

### 10.5 User and operator experience

A peer should be able to understand:
- Who is charging it and for which service.
- The tariff unit and current price.
- How much traffic was counted in each direction.
- What is pending, settled, disputed, or overdue.
- Its spending cap and remaining prepaid/credit allowance.
- Why a route was selected when price influenced selection.
- How to pause buying or selling connectivity.

Operator views may aggregate revenue, upstream costs, relay payments, capacity commitments, and service health. Such aggregation should not erase the underlying bilateral records.

## 11. Security, privacy, and abuse cases

At minimum, study and test:
- Fake or inflated usage reports.
- Colluding peers and fabricated forwarding.
- Sybil identities and repeated unpaid accounts.
- Replay of signed receipts.
- Double settlement.
- Counter reset or rollback.
- Stolen or rotated signing keys.
- Malicious price advertisements.
- Denial of service through expensive route/settlement churn.
- Payment backend outage or chain congestion.
- Peer departure with unsettled balance.
- Route flapping that creates excessive accounting sessions.
- Traffic attribution errors across shared tunnels.
- Privacy leakage through public payment metadata.

Mitigations should be layered: authenticated sessions, bounded credit, spending caps, replay protection, canonical receipts, reconciliation, rate limits, key lifecycle, local privacy controls, and dispute handling.

Avoid making payment metadata a global traffic-surveillance system. Public ledgers are especially unsuitable for publishing detailed user destinations, per-flow histories, or stable mappings between personal identities and network activity.

## 12. Failure and offline behavior

The system must define what happens when:
- The payment chain is unavailable.
- A payment backend is slow or returns an ambiguous result.
- A peer loses connectivity after forwarding traffic.
- A metering process restarts.
- A peer runs out of prepaid balance.
- A neighbor refuses to accept a new tariff.
- A route changes mid-session.
- An overlay is reachable but the payment service is not.
- Accounting evidence is disputed.

Possible policies include prepaid-only service, bounded offline credit, temporary grace periods, local queues for settlement, or fail-closed behavior. The choice should be explicit and configurable. Never silently create unlimited debt.

The routing system should distinguish network reachability from economic eligibility. A peer can be reachable but unwilling/unable to sell service, or economically eligible while currently unreachable.

## 13. Research and implementation roadmap

These are proposed study tasks, not completed work. Each item should remain open until supported by a design decision, implementation, and evidence.

### Phase A — Requirements and reference study

- [ ] Map relevant Rita components: peer discovery, WireGuard session setup, Babel price propagation, tunnel metering, billing, wallet, and exit behavior.
- [ ] Trace which Rita measurements are used for each neighbor balance and how directionality is represented.
- [ ] Study the Althea whitepaper's pay-for-forward, response-traffic, metering, and settlement sections.
- [ ] Compare Althea's assumptions with CJDNS and Yggdrasil peer/session models.
- [ ] Define target deployments: community backhaul, independent relay mesh, direct user-to-user sharing, and multi-overlay service access.
- [ ] Write explicit non-goals and threat assumptions for each deployment.

### Phase B — Economic model

- [ ] Define peer roles as composable capabilities rather than fixed network tiers.
- [ ] Define buyer/seller relationships and bilateral service agreements.
- [ ] Specify tariff units, direction, currency/asset, validity, limits, and service scope.
- [ ] Specify how route price is composed from per-hop charges.
- [ ] Define backhaul capacity contracts separately from measured forwarding usage.
- [ ] Define return-traffic and asymmetric-route accounting.
- [ ] Define prepaid, credit, payment-channel, and periodic net-settlement policies.
- [ ] Define default, dispute, expiry, and peer-departure behavior.

### Phase C — Metering and evidence

- [ ] Select candidate metering boundaries for each supported forwarding technology.
- [ ] Define a canonical, versioned directional usage receipt.
- [ ] Define session epochs, monotonic sequence numbers, and counter-reset behavior.
- [ ] Define bilateral reconciliation and acceptable measurement tolerances.
- [ ] Threat-model forged, replayed, omitted, duplicated, and collusive records.
- [ ] Prove that the chosen meter can attribute bytes to the correct neighbor and direction.
- [ ] Document what the meter cannot prove.

### Phase D — Payment backend research

- [ ] Compare Althea L1, generic EVM settlement, payment channels, and local credit.
- [ ] Compare fees, finality, liquidity, offline behavior, wallet UX, privacy, and recovery.
- [ ] Define a backend capability model without hard-coding one chain.
- [ ] Specify idempotency and recovery for pending/ambiguous settlement.
- [ ] Define balance exposure and maximum unpaid usage.

### Phase E — MeshPlex integration design

- [ ] Decide whether the economy is a MeshPlex optional crate, companion daemon, or separate project.
- [ ] Keep route selection independent of payment backend availability.
- [ ] Define how accepted offers and fresh measurements influence path selection.
- [ ] Define explicit transport/session bindings for CJDNS and Yggdrasil.
- [ ] Define how a real forwarding component supplies metering events.
- [ ] Do not claim paid forwarding until the forwarding and metering path is implemented and tested.
- [ ] Document all new trust boundaries and permissions.

### Phase F — Validation

- [ ] Test direct two-peer accounting in both directions.
- [ ] Test a multi-hop path with at least two paid relays.
- [ ] Test backhaul provider → gateway/relay → end-user traffic.
- [ ] Test user A → user B → user C with B earning and paying independently.
- [ ] Test asymmetric forward and return routes.
- [ ] Test route changes while traffic is active.
- [ ] Test peer reboot, counter reset, key rotation, and peer departure.
- [ ] Test payment backend outage, duplicate submission, delayed finality, and recovery.
- [ ] Test malicious and inconsistent metering reports.
- [ ] Test spending caps, credit exhaustion, and dispute handling.
- [ ] Verify no CJDNS/Yggdrasil protocol merging or identity conflation occurs.
- [ ] Publish reproducible tests and measured results before making performance or security claims.

## 14. Open design questions

1. Should the economic layer live inside MeshPlex as an optional module, or beside it as a companion service?
2. Which forwarding components are expected to provide the first trustworthy metering integration?
3. Is the first deployment a managed community network, a permissionless peer market, or both?
4. Should the initial payment model be prepaid, bounded bilateral credit, payment channels, or periodic settlement?
5. What is the minimum viable receipt that both sides can independently verify?
6. How should a peer advertise a price without implying guaranteed capacity or reachability?
7. How should a user express a cost/quality preference without allowing price to bypass security policy?
8. Which data must remain local to protect user privacy?
9. How are taxes, refunds, consumer protection, and operator obligations handled in deployments that sell connectivity commercially?
10. What is the recovery policy when a peer disappears with an unsettled balance?

## 15. References

### MeshPlex
- [MeshPlex README](https://github.com/Floflis/MeshPlex)
- [MeshPlex architecture](https://github.com/Floflis/MeshPlex/blob/main/docs/architecture.md)

### Althea / Rita
- [Althea Rita repository](https://github.com/althea-net/rita) — coordination, routing integration, neighbor traffic accounting, billing, and payment backend overview.
- [Althea Whitepaper v2.0](https://updates.althea.net/Althea-Whitepaper-v2.0.pdf) — pay-for-forward, metering, route pricing, exit nodes, and settlement.
- [Althea: How it works](https://staging.altheamesh.com/how-it-works) — payment channels, price-aware routing, route verification, and response traffic.
- [Althea Platform](https://blog.althea.net/althea-platform/) — platform overview and pay-per-forward model.
- [Althea L1 introduction](https://docs.althea.net/pages/L1/althea-l1-intro.html) — Althea L1 purpose and architecture.
- [Althea L1 token economics overview](https://docs.althea.net/pages/L1/althea-l1-token-economics-overview.html) — machine-to-machine payments and infrastructure economics.
- [Althea 2025 in Review](https://blog.althea.net/althea-2025-in-review/) — recent ecosystem and firmware direction.

---

## Document status and interpretation

This document records a research direction for MeshPlex. It does not change the current implementation contract:

- Existing CJDNS and Yggdrasil adapters remain read-only observers unless a separately reviewed change says otherwise.
- MeshPlex does not merge CJDNS and Yggdrasil.
- MeshPlex does not become an overlay implementation merely by studying payments.
- Route selection is not packet forwarding.
- Signed advertisements are not proof of delivered traffic.
- Metering is not settlement, and settlement is not proof of service delivery.
- No Althea/Rita code, chain, token, or payment protocol is adopted by this document.

Any implementation should begin only after the relevant requirements, trust model, metering boundary, and payment semantics have been reviewed.
