# Nous

Nous is an experimental project based on a frustration: modern computing is great, but so much of it does not feel elegant. A lot of today’s stack feels layered, patched, wrapped, abstracted, and repaired over decades rather than designed as one coherent system.

This project is not an attempt to replace everything overnight. It is an experiment in asking a smaller question first:

> If we rebuilt parts of the computing stack carefully, could we make them more modular, faster, safer, more failure-tolerant, and easier to understand?

Nous is intended to be a modular, high-performance, capability-secure computing substrate. **NousOS is only one possible host.** The first goal is not to build a full operating system immediately, but to create components that are useful on existing systems first.

Every major component should be usable independently before a native NousOS exists.

---

## Core thesis

Modern computing works, but it carries a lot of historical weight:

- legacy protocols
- unsafe defaults
- bloated runtimes
- fragile drivers
- global authority models
- dependency sprawl
- ad hoc security layers
- AI added on top rather than designed into the system

Nous is an attempt to explore whether a cleaner computing substrate can be built from first principles while still remaining compatible with current technology.

The goal is not purity for its own sake. The goal is practical elegance: systems that are small enough to reason about, fast under constraint, explicit about permissions, resilient to failure, and useful without requiring a complete migration.

---

## Non-negotiable principles

1. **Modularity first**
   No component should require the full OS unless there is no realistic alternative.

2. **Performance under constraint**
   Every module should declare its CPU, memory, latency, wakeup, dependency, and failure budgets.

3. **Capability security**
   No global root authority by default. Rights should be explicit, delegable, revocable handles.

4. **Failure containment**
   Apps, drivers, agents, UI shells, network services, and storage layers should fail locally and recover cleanly.

5. **Source and IR outlive binaries**
   Raw binaries are disposable cache. Source, specifications, intermediate representation, tests, and build recipes are the canonical artifacts.

6. **Local-first and offline-first**
   The system should remain useful without internet access.

7. **AI-native, but not AI-trusting**
   LLMs may assist, generate, and propose. Verification, policy, tests, and explicit user authority decide what executes.

8. **Crypto-agile and post-quantum ready**
   Cryptographic algorithms must be replaceable over time.

9. **Bridge the current world**
   HTTP, Git, FUSE, POSIX, WebSocket, QUIC, SSH, and existing operating systems should interoperate with Nous components.

10. **Designed for long life**
    Critical modules should carry specifications, reference implementations, examples, tests, migration paths, and enough context for future maintainers to reconstruct them.

---

## First MVP

Nous should begin as tools, not as a full OS.

The first target is a small but useful vertical slice:

```text
NousFS   -> content-addressed object store
NousCLI  -> command-line interface
NousHTTP -> HTTP gateway for Nous objects and services
NousCaps -> capability manifest and token model
```

Target commands:

```bash
nous init
nous put ./file.txt
nous get <cid>
nous serve --http 8080
nous grant read <cid> --ttl 10m
```

This MVP should prove that a file can enter Nous, become content-addressed, receive explicit access rights, be served through current web protocols, and be retrieved safely.

---

## Technical direction

Nous is expected to evolve in layers:

```text
NousFS      content-addressed storage
NousCaps    explicit permission and capability model
NousHTTP    compatibility bridge to current web systems
NousNet     identity-first peer networking
NousVM      sandboxed execution runtime
NousLang    future systems language and IR
NousOS      optional native operating system host
```

The early implementation should prioritize correctness, small surface area, testability, and measurable resource usage over feature count.

---

## Repository map

```text
docs/        architecture, principles, threat model, roadmap
modules/     module specifications
research/    study notes and open questions
roadmap/     milestones and execution plan
prompts/     continuation prompts for AI/code agents
prototypes/  starter implementation folders
```

---

## Current status

Nous is currently an early research and prototype project.

The immediate focus is to turn the idea into executable pieces:

1. define the object format
2. implement `nous put` and `nous get`
3. add capability manifests
4. expose objects through an HTTP gateway
5. measure performance and resource usage
6. keep the design small enough to understand

This project may eventually lead to a native operating system, but the first milestone is simpler:

> Build useful, modular computing components that can run alongside today’s systems.
