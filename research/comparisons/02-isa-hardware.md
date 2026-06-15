# 2. ISA & Hardware

Compared: RISC-V vector extension (RVV 1.0), CHERI capabilities, ARM big.LITTLE / DynamIQ, NUMA, GPU/NPU/DSP scheduling.

## What to copy

- **RISC-V RVV 1.0** — vector-length-agnostic (VLA) model: code written once runs across `VLEN` 128…1024+ bits without recompilation. This is the right portability model for NousAI compute kernels and any NousLang SIMD story.
- **CHERI** — hardware capabilities as 128-bit fat pointers carrying bounds + permissions + a tag bit; spatial memory safety enforced in hardware; compartmentalization at near-zero IPC cost. Hardware realization of NousCaps. Morello (ARM) and CHERI-RISC-V are the prototypes.
- **ARM big.LITTLE / DynamIQ** — heterogeneous cores + energy-aware scheduling (EAS); core-type placement, race-to-idle. Copy the scheduler-hint model: tasks declare latency-vs-efficiency intent.
- **NUMA** — first-touch allocation, node-local memory, explicit memory-policy APIs (`mbind`, `numactl`). Copy: model weights and large objects should be node-pinned and loaded with locality awareness.
- **GPU/NPU scheduling** — queue/submit model with explicit command buffers (Vulkan/Metal/CUDA streams); accelerators are async coprocessors, not synchronous calls. Copy the submit-and-fence model for NousAI offload.

## What to avoid

- Hand-written fixed-width SIMD intrinsics (AVX-512 / NEON specific) in portable code — defeats RVV's VLA advantage and rots across targets. Keep behind a runtime-dispatch boundary only.
- Assuming CHERI availability — not shipping in mainstream silicon; treat as a future target, not a dependency.
- Vendor-locked accelerator APIs (CUDA-only) in the substrate core — push behind an abstraction (see IREE/MLIR, area 5).
- NUMA-oblivious "just malloc" for multi-GB model loads — cross-node traffic and latency cliffs.
- Pinning Nous semantics to one core topology — heterogeneity (P/E cores, NPUs) is now the norm even on laptops/phones.

## Performance implications

- RVV VLA: one kernel, near-peak across widths — but compiler/runtime must handle tail loops; mispredicted vector config (`vsetvl`) costs cycles.
- CHERI: ~0–10% overhead depending on pointer density; compartment switch far cheaper than process IPC — enables fine-grained sandboxing the OS path can't afford.
- big.LITTLE: wrong placement (latency task on E-core) can be 2–4× slower; correct EAS saves large energy with little latency cost.
- NUMA: remote access ~1.5–2× local latency and lower bandwidth; matters for inference token throughput on many-core / dual-socket machines.
- Accelerator offload: only wins above a transfer-cost threshold; small ops are faster on CPU (relevant to area 5).

## Security implications

- CHERI gives spatial safety + compartmentalization in hardware — directly enforces NousCaps principle #4 at finer granularity and lower cost than process isolation.
- Heterogeneous cores and shared accelerators add side-channel surface (shared caches/NPUs); cross-tenant inference needs isolation policy.
- NUMA/huge-page sharing can leak across boundaries if not partitioned.

## Implementation complexity

- Consume RVV/NEON/AVX via compiler autovec + runtime dispatch: moderate.
- CHERI targeting: high and hardware-gated; realistic only as a compile target flag for NousLang/NousVM later.
- NUMA-aware loading + core-affinity hints: low–moderate (OS APIs).
- Accelerator scheduling abstraction: moderate–high (deferred to NousAI via existing runtimes).

## How it maps to Nous modules

- **NousCaps** ⇐ CHERI (hardware capability backend, long-term).
- **NousAI** ⇐ RVV VLA kernels, NUMA-aware model loading, GPU/NPU submit/fence offload.
- **NousVM / NousLang** ⇐ RVV as the portable vector model; CHERI as a hardening compile target.
- **NousOS scheduler** ⇐ big.LITTLE EAS + NUMA placement policy.

## Recommended MVP decision

Hardware-agnostic. Phase 1 has no vector/accelerator needs — let the Rust compiler autovectorize BLAKE3/IO and stop there. Do not write SIMD, do not target CHERI, do not add NUMA logic. Only record a `target`/capability descriptor in object & build metadata so future hardware-specific artifacts are addressable.

## Recommended long-term decision

NousCaps gets a CHERI backend where silicon allows (capabilities enforced in hardware, cheap compartments). NousAI uses RVV VLA kernels + NUMA-aware loading + an accelerator submit/fence abstraction. NousOS scheduler is heterogeneity-first (per-task latency/efficiency intent → P/E/NPU placement). All hardware specialization stays behind runtime dispatch so a single source/IR stays portable (principle #6).
