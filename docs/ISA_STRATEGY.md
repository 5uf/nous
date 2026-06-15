# ISA Strategy

## Decision

Do not start with a custom ISA. Start with existing hardware:

```text
x86_64  -> development and desktop/server compatibility
ARM64   -> laptops, phones, SBCs
RISC-V  -> open future target and custom extension path
```

## Long-term direction

RISC-V is the best long-term base because it is open and extensible. Nous should define optional extensions, not a full replacement ISA at the beginning.

## Candidate Nous extensions

```text
Capability registers / CHERI-like pointers
Fast capability transfer instructions
Vector/tensor operations for LLM inference
Crypto and post-quantum acceleration hooks
Message-passing primitives
Persistent memory hints
Energy and thermal hint instructions
```

## Principle

Performance should first come from removing inefficiencies in software architecture:

- less copying
- fewer layers
- fewer wakeups
- better memory residency
- better scheduling
- better locality
- less dynamic dependency overhead

Custom hardware is Phase 3+, not Phase 1.
