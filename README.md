# Nous

Nous is a modular, high-performance, failure-tolerant, capability-secure computing substrate. NousOS is only one possible host. Every major component must be usable independently on existing systems before a native OS exists.

## Core thesis

Modern computing is powerful but inelegant: layers of legacy protocols, unsafe languages, bloated runtimes, global authority models, fragile drivers, ad hoc security, and AI bolted on after the fact. Nous is an experiment in rebuilding the stack from first principles while staying compatible with current technology.

## Non-negotiable principles

1. Modularity first. No component may require the full OS unless there is no alternative.
2. Performance under constraint. Every module declares CPU, memory, latency, wakeup, dependency, and failure budgets.
3. Capability security. No global root authority. Rights are explicit, delegable, revocable handles.
4. Failure containment. Apps, drivers, agents, UI shells, network services, and storage layers must fail locally and recover.
5. Source and IR outlive binaries. Raw binary is disposable cache, not the canonical artifact.
6. Local-first and offline-first. The system must remain useful without internet.
7. AI-native but not AI-trusting. LLMs may generate proposals; verification, tests, and policy decide execution.
8. Crypto-agile and post-quantum ready. Algorithms must be replaceable.
9. Bridge the current world. HTTP, Git, FUSE, POSIX, WebSocket, QUIC, SSH, and existing OSes must interoperate.
10. Reconstructable over centuries. Critical modules carry specs, reference implementations, examples, tests, and migration paths.

## First MVP

Build Nous as tools first:

```text
NousFS   -> content-addressed object store
NousCLI  -> command-line interface
NousHTTP -> HTTP gateway to expose Nous objects/services
NousCaps -> capability manifest and token model
```

Target first commands:

```bash
nous init
nous put ./file.txt
nous get <cid>
nous serve --http 8080
nous grant read <cid> --ttl 10m
```

## Repository map

```text
docs/       architecture, principles, threat model, roadmap
modules/    module specifications
research/   study notes and open questions
roadmap/    milestones and execution plan
prompts/    continuation prompts for AI/code agents
prototypes/ starter implementation folders
```
