# MeshPlex

**Rust-native control plane for path choice across independent overlay networks.**

MeshPlex observes CJDNS and Yggdrasil, binds explicit endpoints to application service identities, and selects routes using fresh measurements. It does not merge overlay protocols.

> Early development: identity, signed advertisements, freshness, read-only CJDNS and three read-only Yggdrasil adapters implemented. System integration remains planned.

## Capabilities
- ServiceId derived from canonical public key bytes.
- SignedServiceAdvertisement: Ed25519 signature, deterministic encoding, timestamps, bounded binary export/import v1.
- Freshness filtering (30-second default) and route-selection policies.
- Read-only CJDNS NodeStore and Yggdrasil Admin adapters (Go, yggdrasil-rs, Yggdrasil-ng).

Signatures prove key possession and record integrity, not trust or endpoint reachability. Discovery, trust, key rotation, and revocation are not implemented.

## Not implemented
Daemon supervision, overlay protocol implementation, discovery/trust lifecycle, forwarding/gateway, host-route mutation, connection migration, anonymity/VPN, or unbenchmarked performance claims.

## Crates
- `meshplex-core`: identity, signed records, metrics, selector.
- `meshplex-cjdns`: read-only CJDNS adapter.
- `meshplex-yggdrasil`: read-only adapters for three implementations.

## Planned release targets

- Windows builds/exports.
- Termux on Android, with the Galaxy S20 FE as a target device.
- ARMv7, armhf, and RISC-V.
- RedoxOS and BSD family: FreeBSD, NetBSD, OpenBSD, DragonFly BSD.

CI cross-target checks do not imply native runtime validation or release packaging.

Build/packaging and runtime validation will be tracked separately for each target.

## Checks
Rust 1.85+: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace`.
See `docs/architecture.md` for boundaries and limitations.
