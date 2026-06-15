# Module: NousAI

Local AI runtime and model manager.

## MVP

Do not implement inference engine from scratch first. Wrap existing local runners through a stable NousAI API.

Initial adapters:

```text
llama.cpp
Ollama-compatible HTTP adapter
ONNX Runtime later
IREE/MLIR later
```

## Native target

Eventually own:

```text
model registry
mmap model loader
KV cache manager
backend scheduler
quant format manager
capability-limited agent runtime
```
