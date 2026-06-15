# Roadmap

## Phase 0: Constitution and MVP spec

Deliverables:

```text
README
principles
module specs
resource budget template
threat model
MVP implementation plan
```

## Phase 1: NousFS + CLI + HTTP

Goal: useful on current OSes.

Deliverables:

```text
Rust CLI
local object store
BLAKE3 content IDs
put/get/list
basic HTTP gateway
basic capability token
unit tests
benchmarks
```

## Phase 2: NousCaps + NousBridge

Deliverables:

```text
capability schema
grant/revoke commands
HTTP auth bridge
FUSE mount prototype
JS/Python/Rust SDKs
```

## Phase 3: NousNet

Deliverables:

```text
identity model
peer discovery
encrypted peer sync
relay mode
Tor adapter experiment
```

## Phase 4: NousAI

Deliverables:

```text
local model registry
llama.cpp adapter
Ollama-compatible adapter
capability-limited agent runner
RAG over NousFS
```

## Phase 5: NousVM / NousIR

Deliverables:

```text
sandbox execution
stable ABI sketch
NousIR spec draft
simple language or DSL experiments
```

## Phase 6: Kernel experiment

Target:

```text
RISC-V QEMU first
serial console
memory allocator
capability table
IPC prototype
supervised service demo
```

## Phase 7: NousOS image

Target:

```text
bootable image
NousFS root snapshot
shell
service supervisor
two QEMU nodes sync over NousNet
```
