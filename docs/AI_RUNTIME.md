# NousAI Runtime

## Purpose

Make local AI and LLM execution first-class, efficient, offline-capable, and safe.

## Design goals

```text
Run local LLMs without fragile dependency stacks
Prefer CPU-heavy workloads when required
Use GPU/NPU/DSP when available and more efficient
Manage model residency
Reduce model load latency
Support quantized models
Keep data local and permissioned
Prevent agents from escaping capabilities
```

## Runtime features

```text
Model registry with signatures and provenance
Memory-mapped model loading
Huge-page support
KV-cache manager
Quantized tensor formats
Backend abstraction: CPU/GPU/NPU/DSP
Thermal and battery-aware scheduling
Prompt privacy boundaries
Agent quotas
Local RAG using NousFS
```

## Scheduling examples

```text
Token sampling         -> CPU
Matrix multiplication  -> GPU/NPU if available
Voice/audio AI         -> DSP/NPU
Background indexing    -> efficiency cores or charging-only
Critical user request  -> performance cores
```

## Avoid

```text
Always-on giant AI daemon
Cloud-only AI dependency
Unbounded autonomous agents
LLM-generated kernel/security changes without verification
```
