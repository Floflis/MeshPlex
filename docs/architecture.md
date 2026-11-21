# Architecture

## Purpose
MeshPlex selects among explicit service endpoints on independent overlays. It does not merge overlay identities, carry traffic, or change system routes.

## Identity and signed advertisements
ServiceId is SHA-256 over domain-separated public-key bytes. SignedServiceAdvertisement adds Ed25519 signature and validity interval. Canonical payload contains domain separator, public key, derived ID, issued/expiry Unix seconds, and sorted endpoints. Duplicate endpoints rejected. Verification checks signature, key-ID binding, validity window, future skew, and expiry (exclusive at expiry).

Signature proves key possession and integrity only. Trust, revocation, rotation, discovery, and reachability are not established. No stable external serialization format is specified yet.

## Observation
Read-only adapters collect overlay observations. Service records associate identity with endpoint addresses. Join requires same overlay and IP. Selector requires fresh data (30-second default).

## States
Reachable: successful end-to-end probe. Routed: overlay route/session only. Degraded: usable but impaired evidence. Unreachable: checked unavailable. Unknown: not established.

## Adapters
CJDNS reads paginated NodeStore_dumpTable via local UDP and stores destination, Routed, link score, and collection timestamp. No invented RTT/peer count.
Yggdrasil Go, yggdrasil-rs (complete node using petrified), and Yggdrasil-ng adapters use read-only getSelf/getPeers. Direct peers are Routed; RTT retained only when supplied. No fabricated loss, throughput, or application reachability. Admin sockets must remain local/restricted.

## Portability and release targets
Planned: Windows builds/exports; Termux on Android (Galaxy S20 FE); ARMv7; armhf; and RISC-V. The roadmap distinguishes compile/build availability from packaging and runtime testing on actual devices.

## Future work
Trust/key lifecycle, overlay-neutral discovery, serialization/versioning, live-daemon tests, richer measurements, adaptive routing/failover, proxy/OS integration, connection migration, gateway forwarding, and benchmarks.
