# Research Continuation Prompt

Continue researching Nous as a modular computing substrate. Focus on implementation decisions, not vague futurism.

Research and compare:

1. seL4, L4, QNX, Redox, Fuchsia, Plan 9, Inferno, Oberon, TempleOS, SerenityOS.
2. RISC-V vector extensions, CHERI capabilities, ARM big.LITTLE, NUMA, GPU/NPU scheduling.
3. IPFS, Git, ZFS, btrfs, Nix store, Guix, Bazel, Buildroot, Yocto.
4. Erlang/OTP supervision, actor systems, crash-only software.
5. llama.cpp, GGUF, Ollama, IREE, MLIR, ONNX Runtime, CPU-only inference optimizations.
6. QUIC, WireGuard, Noise Protocol, Tor onion services, libp2p, delay-tolerant networking.
7. Post-quantum cryptography: ML-KEM/Kyber-class, Dilithium/ML-DSA-class, Falcon-class.
8. Long-term software preservation, reproducible builds, formal specifications, multiple implementation strategy.

For each area, produce:

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
