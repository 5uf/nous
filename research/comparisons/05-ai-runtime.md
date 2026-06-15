# 5. Local AI Runtime

Compared: llama.cpp, GGUF, Ollama, IREE, MLIR, ONNX Runtime, CPU-only inference optimizations.

## What to copy

- **llama.cpp** — the reference for CPU-first, dependency-light local inference: aggressive quantization (Q4_K_M, Q5_K, Q8, IQ-series), memory-mapped weights, NUMA-aware threading, `ggml` backend abstraction (CPU/CUDA/Metal/Vulkan), prompt-cache/KV-cache reuse. Copy its execution model and vendor it; do not reimplement.
- **GGUF** — single-file, self-describing model container: metadata (arch, hyperparams, tokenizer) + quantized tensors, mmap-friendly, versioned. Copy GGUF as NousAI's model object format — already content-addressable-friendly (hash the file → CID).
- **Ollama** — UX/ops layer: model registry, pull-by-name, `Modelfile`, lifecycle/keep-alive, simple HTTP API. Copy the model-manager UX and registry concept (maps to NousAI + NousFS-addressed models).
- **MLIR** — multi-level IR with progressive lowering and dialects; the right substrate for a portable compute IR. Copy the dialect/progressive-lowering idea for any NousLang→hardware path (ties to area 2 RVV/CHERI).
- **IREE** — MLIR-based, ahead-of-time compiled, deployable model artifacts, multiple HAL backends, small runtime. Copy the AOT-compile-to-portable-artifact model for long-term NousAI.
- **ONNX Runtime** — stable cross-framework model interchange + execution-provider plugin model. Copy ONNX as an *interchange/import* format and the execution-provider abstraction.
- **CPU-only opts** — quantization, KV-cache, weight mmap, thread pinning, AVX/NEON/RVV kernels, speculative decoding, batching, flash-attention-style memory layout. Copy the whole bag — Nous must run on commodity/constrained hardware (principle #7).

## What to avoid

- Reimplementing inference kernels from scratch — explicitly forbidden for Phase 1 and a multi-year trap; stand on llama.cpp/ggml.
- Python/PyTorch runtime dependency in the substrate — heavy, slow startup, poor for constrained targets; keep training/experimentation out of the runtime path.
- MLIR/IREE build-system weight (LLVM dependency) in early phases — enormous; defer.
- Ollama's Go daemon as a hard dependency — copy the concept, integrate via NousAI directly.
- Vendor-locked accelerator backends in the core — keep behind ggml/HAL-style abstraction (area 2).
- Unbounded model memory use — must respect idle-RAM budgets and degrade gracefully on constrained devices.

## Performance implications

- Quantization (Q4/Q5) cuts memory ~4–8× and is the single biggest enabler of local inference; small quality loss, large reach gain.
- mmap + KV-cache reuse dominates latency/throughput; cold-load vs warm-load differs by seconds.
- NUMA placement + thread pinning (area 2) materially affect tokens/sec on many-core CPUs.
- AOT compilation (IREE) reduces per-run overhead and startup vs interpreted graphs, at build-time cost.
- Accelerator offload only wins above a size threshold (area 2) — CPU path must stay first-class.

## Security implications

- Models are untrusted data: GGUF/ONNX parsers are an attack surface (malicious tensors/metadata) — must run inside NousCaps confinement + crash-only worker (area 4).
- Model provenance: content-address models (hash GGUF → CID) so a model's identity is verifiable and pinnable (principle #6).
- Inference workers must be capability-confined: no ambient filesystem/network; only granted resources.
- Prompt/data isolation between tenants if NousAI is shared.

## Implementation complexity

- Vendor llama.cpp + GGUF behind a thin NousAI process/API: **low–moderate**.
- Ollama-style model manager over NousFS-addressed GGUF blobs: moderate.
- ONNX import path: moderate.
- MLIR/IREE AOT compile pipeline: high (LLVM-scale) — long-term.

## How it maps to Nous modules

- **NousAI** ⇐ llama.cpp/ggml execution + GGUF model format + Ollama-style manager + ONNX import + (long-term) IREE/MLIR AOT.
- **NousFS** ⇐ content-addresses GGUF/ONNX model blobs (provenance + dedup).
- **NousCaps** ⇐ confines inference workers (untrusted-model containment).
- **NousVM / NousLang** ⇐ MLIR dialects as a compute-IR precedent (long-term, area 2).

## Recommended MVP decision

Out of Phase 1 scope by the seed spec ("do not implement LLM inference from scratch in Phase 1"). When NousAI starts: vendor llama.cpp, adopt GGUF as the model object format stored *in NousFS by content hash*, expose a thin local HTTP/CLI surface, run inference in a separate capability-confined crash-only worker process. No MLIR/IREE, no Python, no kernel-writing. Reuse the existing CPU-only optimization stack as-is.

## Recommended long-term decision

NousAI keeps the llama.cpp/ggml-style CPU-first path as the floor, adds an IREE/MLIR AOT pipeline for portable, hardware-specialized model artifacts (RVV/CHERI/NPU targets from area 2), and treats every model as a content-addressed, capability-gated object. ONNX remains the import/interchange boundary via NousBridge. Inference workers stay supervised, isolated, and reconstructable from pinned source + model CID (principles #6, #7, #10).
