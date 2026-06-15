# Module: NousVM

Portable execution substrate.

## Initial strategy

Start with WASM/component-model-style sandboxing or a Rust-native plugin model. Do not build a custom VM too early.

## Requirements

```text
capability handles
resource quotas
zero-copy buffers
async tasks
stable ABI
AOT path
native fast path
```
