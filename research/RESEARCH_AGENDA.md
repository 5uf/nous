# Research Agenda

> Implementation-decision analysis for these topics lives in [`comparisons/`](comparisons/README.md) — one file per area, each with: what to copy / avoid, performance & security implications, complexity, module mapping, MVP + long-term decisions.

## OS architecture

- seL4/L4 IPC performance and verification lessons
- Redox OS architecture and Rust driver model
- Fuchsia Zircon capabilities and component framework
- Plan 9 namespaces and 9P tradeoffs
- QNX reliability and user-space driver patterns
- Erlang/OTP supervision applied to OS services

## ISA and hardware

- RISC-V vector extension for LLM workloads
- CHERI capabilities and memory safety
- ARM big.LITTLE scheduling
- NUMA and huge-page model loading
- GPU/NPU/DSP scheduling APIs
- microcontroller RTOS constraints

## Storage

- IPFS content addressing
- Git Merkle DAGs
- ZFS checksums/snapshots
- Nix store and reproducibility
- CRDTs for offline sync

## Networking

- QUIC transport
- Noise protocol
- WireGuard identity model
- Tor onion services
- libp2p and DHTs
- delay-tolerant networking

## AI runtime

- llama.cpp internals
- GGUF quant formats
- KV-cache management
- mmap loading
- IREE/MLIR
- CPU-only inference optimization

## Longevity

- software preservation
- reproducible builds
- formal specs
- multi-implementation protocol design
- post-quantum migration
