# NousLang and NousIR

## Purpose

NousLang is a systems language for writing the OS, services, agents, drivers, and applications. It should be memory-safe, capability-aware, fast, and simple enough to self-host.

## Requirements

```text
Memory safe by default
AOT compiled
Zero-cost abstractions where possible
Async/actor model native
Capability-aware types
Deterministic build support
No hidden global authority
Excellent FFI boundaries
Embeddable formal specs
Fast compile mode and optimizing compile mode
```

## Compilation model

```text
NousSource
  -> NousIR
  -> target backend: x86_64 / ARM64 / RISC-V / WASM / future ISA
```

Raw binary is not canonical. Store all three:

```text
source.nous
program.nir
program.<target>
```

## Binary translation stance

NousLang -> binary can be efficient.

Binary -> NousLang is fundamentally lossy unless the binary embeds metadata, source maps, IR, types, capability manifests, and build recipes.

Therefore, Nous artifacts must be self-describing from the start.

## Example syntax sketch

```nous
service Photos {
    grant fs.read("/photos")
    grant net.connect("backup-node")

    async fn index(image: Blob) -> IndexResult {
        let embedding = ai.embed(image)
        store(embedding)
    }
}
```
