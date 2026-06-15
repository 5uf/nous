# Nous Principles

## Design law

No abstraction is allowed unless it saves more complexity than it adds.

## Apollo-style constraints

Nous should be elegant and tight like mission-critical embedded systems:

- deterministic where possible
- bounded memory usage
- bounded background activity
- no hidden daemons
- no polling loops in core services
- no unbounded dependency trees
- no JSON or text parsing in hot paths unless explicitly justified
- no Electron-style default application model
- no containers as the default isolation mechanism
- no always-online assumptions

## Performance hierarchy

1. Do not do the work.
2. If needed, do it once.
3. If repeated, cache it.
4. If moved, move references, not bytes.
5. If isolated, use capabilities, not heavy containers.
6. If verified, verify at build/load time where possible.
7. If dynamic, make it declarative and bounded.

## Resource ledger

Every module must publish:

```text
Boot cost
Idle RAM
Peak RAM
Idle wakeups/sec
CPU budget
Latency budget
Storage budget
Dependency count
Failure modes
Recovery behavior
Test coverage
```

## Adoption law

Nous must be useful before NousOS exists.
