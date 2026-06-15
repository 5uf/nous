# Continuation Prompt for Building Nous

You are continuing the Nous project: a modular, high-performance, capability-secure, failure-tolerant computing substrate. Treat this repository as the canonical seed spec.

## Mission

Build Nous as a usable modular stack first, not as a full OS first. NousOS is optional and later. Every module must be independently useful on Linux/macOS/Windows/Android before native NousOS exists.

## Core vision

Nous aims to remove modern computing inefficiencies while remaining compatible with current systems. It should be elegant, tight, resource-conscious, and reliable under constraints, inspired by Apollo-style engineering, seL4, Plan 9, Erlang/OTP, CHERI, RISC-V, Nix, IPFS/Git/ZFS, TempleOS, Oberon, and high-performance computing systems.

Nous is not one giant tool. It is a coherent substrate made of interoperable modules:

```text
NousFS      content-addressed storage
NousCaps    capability security
NousNet     identity-first network
NousBridge  adapters to HTTP/FUSE/Git/POSIX/current web
NousAI      local AI/runtime/model manager
NousVM      sandboxed portable runtime
NousBuild   reproducible builds and target images
NousLang    future systems language
NousOS      optional native operating system
```

## Non-negotiable principles

1. No unnecessary abstraction.
2. Every module must be usable without NousOS.
3. Current tech compatibility is mandatory.
4. Capability security replaces global root/admin.
5. Failure containment is mandatory.
6. Source + IR + specs outlive binaries.
7. Performance under constraints is more important than feature count.
8. AI helps generate proposals but must not bypass verification.
9. Crypto must be agile and post-quantum ready.
10. The system must be self-describing and reconstructable over centuries.

## Immediate implementation target

Implement the Phase 1 MVP:

```text
NousFS + NousCLI + NousHTTP + basic NousCaps
```

Target commands:

```bash
nous init
nous put ./file.txt
nous get <cid> --out ./file.txt
nous ls
nous inspect <cid>
nous verify <cid>
nous serve --http 8080
nous grant read <cid> --ttl 10m
```

## Preferred implementation language

Use Rust for Phase 1.

Rationale:

```text
memory safety
good CLI ecosystem
fast native binaries
cross-platform support
suitable for systems prototypes
```

## Starter architecture for MVP

```text
crates/
  nous-core     -> object IDs, manifests, errors
  nous-store    -> local content-addressed store
  nous-caps     -> capability token MVP
  nous-http     -> HTTP gateway
  nous-cli      -> command-line app
```

## MVP storage layout

```text
.nous/
  config.toml
  objects/
    ab/
      cd/
        <hash>
  meta/
    <hash>.toml
  caps/
  logs/
```

## Object model MVP

Start with blobs only.

Later add:

```text
Tree
Commit
Manifest
CapGrant
SignedObject
```

## Hashing

Use BLAKE3 initially. The ID should include algorithm metadata eventually, but do not overbuild the first version.

## Capability MVP

A simple signed or local token is enough initially.

Fields:

```text
cap_id
issuer
holder optional
resource cid
rights
expiry
constraints
signature optional in v0
```

HTTP gateway should accept:

```http
Authorization: Bearer <capability-token>
```

## HTTP MVP

Endpoints:

```http
GET /object/<cid>
GET /meta/<cid>
POST /object
GET /health
```

If capability enforcement is enabled, object reads require a valid read cap.

## Performance requirements

Add benchmark hooks from the beginning:

```text
put throughput
get throughput
hash throughput
HTTP read latency
startup time
binary size
idle RAM
```

Do not add heavy dependencies unless justified.

## Failure tolerance requirements

- Corrupt object read must fail safely.
- Partial writes must not corrupt store.
- Use temp files and atomic rename.
- Keep audit logs.
- HTTP server failure must not corrupt data.

## What NOT to do yet

Do not build the kernel first.
Do not design a full custom ISA yet.
Do not implement a full programming language yet.
Do not build a blockchain.
Do not require Tor globally.
Do not implement LLM inference from scratch in Phase 1.
Do not overdesign distributed sync before local store is correct.

## Deliverables expected from next agent

1. Inspect repository docs.
2. Create a Rust workspace.
3. Implement NousFS local object store.
4. Implement NousCLI commands: init, put, get, ls, inspect, verify.
5. Implement basic HTTP gateway.
6. Implement minimal capability token format.
7. Add tests and benchmarks.
8. Keep code small, explicit, and well-documented.
9. Update docs when design changes.

## Quality bar

The result should be boring, fast, reliable, and understandable. Prefer a small correct implementation over a grand incomplete architecture.

## Design slogan

Nous: elegant under abundance, reliable under scarcity.
