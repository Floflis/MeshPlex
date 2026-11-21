# OS and multi-architecture matrix

**Status:** platform engineering roadmap. MeshPlex's Rust-native control plane is not yet declared portable to every target in this document.

## 1. Evidence model

Keep these states distinct:

- **OS port:** the operating system supports the CPU architecture.
- **Rust target:** rustc defines a target triple.
- **Toolchain:** the target standard library/toolchain can be installed or built.
- **Compile:** MeshPlex's actual workspace and features compile.
- **Run:** tests execute on native hardware or a documented emulator/VM.
- **Release:** a clean, distributable artifact is smoke-tested.

A target triple alone proves none of the later states. In particular, Tier 3 means experimental target support and may require a custom toolchain or `-Z build-std`.

## 2. Advanced target inventory

| OS | Architecture | Rust target | Initial lane |
|---|---|---|---|
| Linux | x86-64 | `x86_64-unknown-linux-gnu` | Native hosted CI |
| Linux | AArch64 | `aarch64-unknown-linux-gnu` | Native ARM64 / cross-check |
| Linux | ARMv7 hard-float | `armv7-unknown-linux-gnueabihf` | Cross-check, then QEMU |
| Linux | RISC-V 64GC | `riscv64gc-unknown-linux-gnu` | Cross-check, then QEMU/native |
| Linux musl | x86-64, AArch64, ARMv7, RISC-V 64 | Corresponding `*-unknown-linux-musl` targets where available | Static-link research |
| Android | AArch64 | `aarch64-linux-android` | NDK + Termux device test |
| Android | ARMv7 | `armv7-linux-androideabi` | NDK + device test |
| macOS | Intel x86-64 | `x86_64-apple-darwin` | Native Intel runner |
| macOS | Apple Silicon | `aarch64-apple-darwin` | Native ARM64 runner |
| Windows | x86-64 MSVC | `x86_64-pc-windows-msvc` | Native runner |
| Windows | ARM64 MSVC | `aarch64-pc-windows-msvc` | Native ARM64 runner |
| FreeBSD | amd64 | `x86_64-unknown-freebsd` | VM/self-hosted |
| FreeBSD | AArch64 | `aarch64-unknown-freebsd` | Experimental VM/native |
| FreeBSD | ARMv7 / RISC-V 64 | `armv7-unknown-freebsd`, `riscv64gc-unknown-freebsd` | Experimental; verify toolchain |
| OpenBSD | amd64 | `x86_64-unknown-openbsd` | VM |
| OpenBSD | AArch64 / RISC-V 64 | `aarch64-unknown-openbsd`, `riscv64gc-unknown-openbsd` | Tier 3 research |
| NetBSD | amd64 | `x86_64-unknown-netbsd` | VM |
| NetBSD | ARM64 / RISC-V 64 | `aarch64-unknown-netbsd`, `riscv64gc-unknown-netbsd` | Experimental; verify target tier |
| DragonFly BSD | x86-64 | `x86_64-unknown-dragonfly` | VM; no other CPU port assumed |
| RedoxOS | x86-64 | `x86_64-unknown-redox` | redoxer + QEMU |
| RedoxOS | AArch64 | `aarch64-unknown-redox` | Tier 3/custom toolchain research |
| RedoxOS | i586 | `i586-unknown-redox` | Tier 3/custom toolchain research |
| RedoxOS | RISC-V 64GC | `riscv64gc-unknown-redox` | Tier 3/custom toolchain + QEMU/native |

The Redox RISC-V target is explicitly included. It must remain marked experimental until the toolchain, dependencies, VM image, and actual runtime tests are available.

## 3. MeshPlex-specific compatibility gates

MeshPlex currently consists of a workspace with core, CJDNS and Yggdrasil crates. Its platform matrix must validate each crate and adapter, not just an empty Rust binary.

Review:
- Filesystem and process APIs used by adapters and configuration.
- TCP/UDP socket behavior, address families and interface discovery.
- CJDNS NodeStore observation and Yggdrasil Admin API connectivity.
- TLS, certificate and cryptographic dependency support.
- OS permissions, service supervision and data-directory behavior.
- Endianness and pointer-width assumptions; do not equate RISC-V 64GC with generic RISC-V.
- Runtime behavior of the external CJDNS/Yggdrasil daemons. A MeshPlex binary compiling on an OS does not prove those daemons are available there.

MeshPlex is a control plane and observer. Do not claim that adding an OS target makes it a packet forwarder, changes host routes, or implements overlay protocol support.

## 4. CI lanes

### Hosted native lane

Keep the existing formatting, Clippy and workspace tests. Add native architecture lanes when GitHub-hosted runners are available, especially Linux ARM64, macOS ARM64 and Windows ARM64.

### Stable cross-check lane

Use Linux cross-checks for stable Rust targets where the standard library is distributed. A `cargo check` result is not a linked executable or runtime pass.

### BSD lane

Use reproducible FreeBSD/OpenBSD/NetBSD/DragonFly VMs or self-hosted runners. Record OS release, architecture, Rust version and image checksum. Do not label an Ubuntu cross-compile as a BSD test.

### Redox lane

Start with x86-64 through `redoxer` and QEMU. Then evaluate AArch64, i586 and RISC-V 64GC separately. For Tier 3 targets, pin the Rust nightly/toolchain and Redox sysroot, and record any `build-std` or custom linker configuration. Do not make the RISC-V lane silently fall back to x86-64.

### Android lane

Use the Android NDK for cross-builds and a real Termux device for runtime validation. Termux is an Android userspace, not a separate Rust target.

## 5. Required CI summary fields

Every target job should report:

- OS and release/version.
- CPU architecture and exact Rust target triple.
- Rust toolchain and target std source (prebuilt or custom).
- Cargo package/features checked.
- Build stage: target install, cargo check, link, unit test, integration test, runtime smoke test.
- Runner type: hosted native, self-hosted native, QEMU, or other VM.
- Result: PASS, FAIL, BLOCKED, or NOT TESTED.

Do not turn missing runners into false green checks. A skipped Tier 3 target is visible as NOT TESTED.

## 6. Implementation order

1. Preserve current Linux x86-64 CI.
2. Add Linux ARM64 and Linux RISC-V cross-checks.
3. Validate Android ARM64/Termux and ARMv7.
4. Add macOS Intel/Apple Silicon and Windows x64/ARM64 native builds.
5. Add FreeBSD and NetBSD x64 VM coverage.
6. Add OpenBSD and DragonFly VM experiments.
7. Boot Redox x86-64 using redoxer/QEMU.
8. Bring up Redox RISC-V 64GC explicitly, then AArch64 and i586.
9. Promote targets only with native/emulated runtime evidence and a repeatable release artifact.

## 7. Follow-up checklist

- [ ] Capture the current workspace dependency and feature graph per target.
- [ ] Establish Linux ARM64 and RISC-V cross-checks.
- [ ] Add a reproducible Redox x86-64 VM recipe.
- [ ] Investigate the Redox RISC-V 64GC sysroot, linker and QEMU machine configuration.
- [ ] Validate CJDNS and Yggdrasil adapter behavior separately on each OS.
- [ ] Add BSD VM images and record exact release/architecture combinations.
- [ ] Keep unsupported targets explicitly experimental rather than dropping them from the matrix.
