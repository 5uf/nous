# 4. Failure Tolerance & Supervision

Compared: Erlang/OTP supervision, actor systems (Akka, CAF, Pony, Orleans), crash-only software (Candea & Fox).

## What to copy

- **Erlang/OTP** — supervision trees; "let it crash"; isolated processes with no shared memory; bounded restart strategies (`one_for_one`, `one_for_all`, `rest_for_one`, max-restart-intensity); `gen_server`/`gen_statem` behaviors; hot code reload; links & monitors for failure propagation. Copy the supervision tree + restart-intensity model wholesale — canonical implementation of Nous principle #5.
- **Crash-only software** — make crash == clean shutdown and restart == recovery; no separate shutdown path to get wrong; recoverable state lives in durable, idempotent stores; fast restart instead of complex error handling. Copy as the *design contract* for every Nous module.
- **Actor systems** — message passing over shared mutable state; location transparency (local vs remote actor identical); backpressure-aware mailboxes. Copy message-passing isolation; copy Pony's *no-data-race* type discipline as an aspiration for NousLang.
- **Orleans virtual actors** — actors that exist on demand and are transparently activated/deactivated — useful for NousAI model workers and NousNet peers.

## What to avoid

- Erlang's per-message copying cost and the BEAM VM weight for *latency-critical native* paths — use OTP *patterns*, not the BEAM runtime (Nous core is native Rust).
- Akka-style untyped actors and "actor for everything" — over-actoring adds indirection and debugging pain; reserve actors for true failure/concurrency boundaries.
- Hidden shared state behind an actor facade — defeats isolation; enforce no-shared-mutable at the boundary.
- Unbounded restart loops / restart storms — always cap restart intensity and escalate to parent.
- Treating "let it crash" as an excuse to skip invariant checks — crash-only requires *durable, consistent* recoverable state.

## Performance implications

- Process/actor isolation costs context switches and message copies; the win is predictable degradation and no cascading corruption. For Nous (correctness > feature count), the trade favors isolation.
- Crash-only restart must be *fast* — startup time is already a Phase 1 benchmark; sub-second module restart keeps availability high.
- Supervision adds negligible steady-state overhead; cost is paid only on failure.
- Idempotent recoverable state (content-addressed store) makes restart cheap — NousFS immutability is a perfect fit.

## Security implications

- Failure isolation == blast-radius containment: a compromised or panicking module cannot corrupt siblings or the store (aligns with capability confinement, areas 1–2).
- Crash-only + atomic writes (area 3) means a killed process never leaves partial/corrupt state — no exploitable half-written invariants.
- Supervisor is a trust anchor; it must be minimal and itself crash-only.

## Implementation complexity

- Crash-only contract per module (atomic writes, idempotent recovery, no shutdown-only cleanup path): **low–moderate**, mostly discipline.
- Supervision tree in Rust: moderate — a supervisor task that spawns/monitors child processes (OS processes for hard isolation, or tokio tasks for soft) with restart-intensity limits.
- Full actor framework: moderate–high — avoid unless needed.
- Pony-grade data-race-free types: high — NousLang long-term only.

## How it maps to Nous modules

- **All modules** ⇐ crash-only contract (principle #5).
- **NousCLI / host supervisor** ⇐ OTP supervision tree managing module processes (NousFS, NousHTTP, NousAI workers) with bounded restarts.
- **NousNet** ⇐ actor/virtual-actor model for peers; links/monitors for connection failure.
- **NousAI** ⇐ inference workers as supervised, restartable, on-demand actors (Orleans-style).
- **NousVM / NousLang** ⇐ actor isolation + Pony-style race-free types (long-term).

## Recommended MVP decision

Bake the crash-only contract into Phase 1: atomic temp+rename writes (already required), idempotent operations, no separate shutdown cleanup path, audit logs for recovery. Run NousHTTP as a separate process so its failure cannot corrupt the store (already required). Add a minimal supervisor in NousCLI that restarts the HTTP server on crash with a capped restart rate (e.g. max 5 restarts / 10s, then fail loud). No actor framework, no BEAM — patterns only.

## Recommended long-term decision

Every Nous module is a crash-only, capability-confined process under an OTP-style supervision tree (the NousOS init/supervisor, or a host supervisor before NousOS). NousNet and NousAI use a typed actor/virtual-actor model with backpressure and location transparency. NousLang provides Pony/Erlang-style isolation and (aspirationally) data-race-free message passing as a language guarantee, so "let it crash" is safe by construction.
