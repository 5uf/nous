# Nous Architecture

## One architecture, many targets

Nous is not one binary for every device. It is one coherent system model with target-specialized builds.

```text
Nous Spec
  -> NousBuild
  -> Target Profile
  -> Generated Device Image / Runtime Package
```

## Target profiles

```text
Nous-Micro   -> microcontrollers, sensors, robots
Nous-Core    -> minimal runtime for Linux/macOS/Windows/Android
Nous-AI      -> local LLM and heavy compute profile
Nous-Net     -> routers, relays, mesh nodes
Nous-OS      -> full desktop/mobile OS
Nous-Cluster -> distributed compute and model-serving cluster
```

## Native OS architecture

```text
Hardware
  -> Bootloader
  -> Tiny capability kernel
  -> user-space drivers
  -> supervised services
  -> NousRuntime
  -> NousFS / NousNet / NousAI / NousUI
```

Kernel should contain only:

```text
scheduler
memory management
capability enforcement
IPC
interrupts/timers
minimal hardware abstraction
emergency recovery path
```

Everything else should be user-space or optional.

## Compatibility architecture

```text
Existing world
HTTP / Git / POSIX / FUSE / WebSocket / QUIC / SSH
  -> NousBridge
  -> NousCaps
  -> NousFS / NousNet / NousRuntime
```

The old world should not need to understand Nous. Nous must understand the old world.

## Hot path rules

- zero-copy IPC
- memory-mapped models and objects
- static/AOT compilation by default
- capability checks at open/spawn/grant boundaries
- compact binary protocols for hot paths
- typed schemas for service boundaries
- no dynamic reflection in critical loops

## Failure model

```text
Kernel fault          -> minimal panic, crash dump, last-known-good boot
Driver fault          -> restart driver service
Filesystem fault      -> snapshot rollback / repair
App fault             -> kill app, preserve state
UI shell fault        -> restart shell
Network fault         -> reconnect / degrade offline
AI agent fault        -> revoke caps, preserve audit log
Update fault          -> atomic rollback
Hardware fault        -> isolate device / reduced mode
```

## Inspirations

- seL4: small capability microkernel and verification mindset
- Plan 9: namespace and distributed resource ideas
- Erlang/OTP: supervision and crash containment
- CHERI: hardware capabilities
- RISC-V: open and extensible ISA strategy
- Nix/Guix: reproducible builds and rollbacks
- IPFS/Git/ZFS: content addressing, versioning, integrity
- TempleOS/Oberon: coherence and understandability
