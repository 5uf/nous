# Comparative Analysis

Implementation-decision research for Nous. Each file covers one area and applies a fixed template:

```text
What to copy
What to avoid
Performance implications
Security implications
Implementation complexity
How it maps to Nous modules
Recommended MVP decision
Recommended long-term decision
```

Scope rule: every recommendation must be actionable for a module that ships on Linux/macOS/Windows/Android before NousOS exists. No futurism. MVP decisions bias toward boring, fast, reuse-existing. Long-term decisions record where Nous eventually diverges and why the divergence pays for itself.

| # | Area | File |
|---|------|------|
| 1 | OS architectures & kernels | [01-os-architectures.md](01-os-architectures.md) |
| 2 | ISA & hardware | [02-isa-hardware.md](02-isa-hardware.md) |
| 3 | Storage & build/content addressing | [03-storage-build.md](03-storage-build.md) |
| 4 | Failure tolerance & supervision | [04-failure-tolerance.md](04-failure-tolerance.md) |
| 5 | Local AI runtime | [05-ai-runtime.md](05-ai-runtime.md) |
| 6 | Networking & transport | [06-networking.md](06-networking.md) |
| 7 | Post-quantum cryptography | [07-post-quantum-crypto.md](07-post-quantum-crypto.md) |
| 8 | Longevity, reproducibility, formal methods | [08-longevity-reproducibility.md](08-longevity-reproducibility.md) |

## Cross-area MVP summary

Decisions that hold across all eight areas for Phase 1 (`NousFS + NousCLI + NousHTTP + basic NousCaps`):

- **Storage**: BLAKE3 content addressing, multihash-style self-describing IDs, atomic temp+rename writes, ZFS-style read-time verification. (Areas 1, 3, 8)
- **Caps**: macaroon-style attenuable tokens, enforced on host via Landlock/pledge/seccomp/Job objects, not ambient root. (Areas 1, 2, 7)
- **Network**: QUIC + Noise IK as the wire, libp2p multiaddr/peer-ID identity model, no global Tor requirement. (Area 6)
- **Crypto**: hybrid X25519+ML-KEM-768 KEM, Ed25519 signatures now with an agility header so ML-DSA-65 drops in later. (Area 7)
- **AI**: vendor llama.cpp + GGUF behind a thin NousAI process boundary; do not write inference kernels. (Area 5)
- **Reproducibility**: SOURCE_DATE_EPOCH + pinned toolchain + content-addressed inputs from commit one; full Nix/Guix-grade hermeticity is long-term. (Areas 3, 8)
- **Failure**: crash-only modules, supervision tree in NousCLI/host process, every module a separate restartable process. (Area 4)
