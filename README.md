# MeshPlex

**A Rust-native control plane for choosing paths across independent overlay networks.**

MeshPlex is an experimental multi-overlay routing coordinator. It observes independent networks, compares the paths they expose to the same service, and selects a usable endpoint according to an explicit policy.

The first targets are **CJDNS** and **Yggdrasil**. Their protocol implementations, cryptographic identities, addressing schemes, and routing algorithms remain independent.

> Status: early development. This repository currently contains a protocol-neutral core and architecture documentation. It does not yet connect to CJDNS or Yggdrasil.

## Why MeshPlex?

Running more than one overlay on the same host does not automatically provide path selection or failover. A service must advertise an endpoint on each network, and a coordinator needs trustworthy observations before it can choose one.

MeshPlex provides the intended coordination layer without replacing the routing engines inside the overlays.

## Current scope

The initial core provides:

- Protocol-neutral overlay and endpoint types.
- A read-only adapter contract for publishing network snapshots.
- Route health and metric models.
- Deterministic selection policies for latency, throughput, packet loss, and preferred networks.
- Unit tests for selection and eligibility rules.

No protocol-specific adapter is included yet.

## What is not implemented

MeshPlex currently does **not**:

- Start, configure, or supervise CJDNS or Yggdrasil daemons.
- Implement CJDNS, CryptoAuth, Ironwood, or Yggdrasil protocols.
- Forward, bridge, translate, or encapsulate packets between overlays.
- Make an endpoint on one overlay reachable through another overlay.
- Migrate an established TCP connection between paths.
- Provide anonymity, an exit-node network, VPN functionality, or an Internet replacement.
- Probe peers or paths on its own.
- Benchmark the overhead of running multiple overlays.

The selector only chooses among endpoints supplied to it. It cannot infer that two unrelated addresses identify the same service.

## Architecture

The intended architecture has four boundaries:

1. **Overlay adapters** observe existing overlay daemons and publish snapshots.
2. **Service discovery** associates one logical service with its separately advertised overlay endpoints.
3. **The policy engine** selects an eligible endpoint from current observations.
4. **An optional system integration layer** may later apply the decision through operating-system routing, a local proxy, or another explicitly configured mechanism.

See docs/architecture.md for design constraints and TODO.md for planned work.

## Workspace

The repository is a Cargo workspace containing the meshplex-core crate: protocol-neutral data types, adapter contract, and route selection.

The core currently uses only the Rust standard library.

## Build and test

Requires Rust 1.85 or newer.

~~~sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
~~~

## Design principles

- **Do not merge protocols.** CJDNS and Yggdrasil retain their own identities, cryptography, address spaces, and routing semantics.
- **Prefer observation before control.** The first adapters are read-only. Route changes require a separately designed and explicitly enabled integration.
- **Make policy visible.** Selection behavior must be deterministic and operator-configurable.
- **Do not invent reachability.** A second endpoint is useful only when the service is genuinely reachable there.
- **Measure before optimizing.** No performance advantage is claimed until reproducible benchmarks exist.
- **Keep protocol code reusable.** Adapters should consume supported APIs or stable interfaces rather than fork protocol implementations.

## Security and privacy

MeshPlex is not an anonymity system. It does not hide a user's source address from overlay peers, create trust between unrelated networks, or make traffic safe merely because it crosses an overlay.

Metrics and service advertisements can reveal network topology and activity. Future adapters must document what they collect, avoid exporting private peer information by default, and make telemetry opt-in where practical.

No license has been selected for this repository yet. Do not assume that code from CJDNS, Yggdrasil, or their Rust implementations can be copied into this project without reviewing the source project's license and notices.

## Project name

MeshPlex combines mesh and multiplexer: one coordinator, multiple independent networks.
