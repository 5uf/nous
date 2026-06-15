# 1. OS Architectures & Kernels

Compared: seL4, L4 family (Pistachio, Fiasco/L4Re, NOVA, OKL4), QNX, Redox, Fuchsia/Zircon, Plan 9, Inferno, Oberon, TempleOS, SerenityOS.

## What to copy

- **seL4** — capability-partitioned access (no ambient authority), formally verified C→binary, ~9–10k LOC kernel, fast synchronous IPC (~hundreds of cycles), MCS scheduling for time guarantees. Copy the *capability model* and the *small-verifiable-core* discipline.
- **Fuchsia/Zircon** — handle = unforgeable capability; no global filesystem namespace; component framework with explicit capability routing; **blobfs**: content-addressed (Merkle) package blob store. The blobfs + cap-routing pair is the closest existing thing to NousFS+NousCaps.
- **Plan 9** — everything-is-a-file done rigorously; **per-process namespaces**; **9P** as a uniform resource protocol; union mounts. This is the model for NousBridge (mount HTTP/Git/FUSE/POSIX into a private namespace).
- **QNX** — message-passing microkernel; user-space drivers; **resource managers** that register pathnames and answer POSIX ops — proven reliability in cars/medical. Copy the user-space-driver + path-handler reliability pattern.
- **Inferno** — portable **Dis** register VM + Styx(9P); whole environment runs hosted on any OS. Direct precedent for NousVM running hosted before NousOS.
- **SerenityOS** — `pledge()`/`unveil()` process self-sandboxing; "write it yourself, no deps" culture. Copy pledge/unveil as the host-side capability-narrowing primitive.
- **Oberon / TempleOS** — radical single-language coherence; system readable in its entirety by one person. Copy the *ethos and size budget*, not the architecture.
- **Redox** — proof that a Rust microkernel + `relibc` + URL-scheme resources is buildable; RedoxFS.

## What to avoid

- seL4's full verification cost for the whole system — it bounds the kernel to ~10k LOC and freezes interfaces; do not attempt to verify NousOS userland early (or maybe ever).
- Fuchsia's FIDL/component toolchain weight and Google-scale build system — enormous for a small team.
- TempleOS's zero security / ring-0 everything / single address space — instructive minimalism, unusable trust model.
- Plan 9 / Inferno hardware-support reality — do not target bare metal early; stay hosted.
- Redox/SerenityOS maturity trap — great references, not dependencies; do not build on their kernels.
- L4 synchronous-IPC blocking semantics as a userland API — too low-level to expose directly.

## Performance implications

- Microkernel IPC is the hot path. seL4/L4 prove it can be ~0.1–1µs with careful design; naive message passing is 10–100× worse. Any NousOS must treat IPC like seL4 treats it (register-banked fastpath) or it loses to monolithic kernels.
- Per-process namespaces (Plan 9) add a lookup indirection but enable cheap sandboxing without containers.
- Content-addressed package blobs (blobfs) give page-cache dedup and verify-on-read at the cost of hashing on write — matches NousFS's own tradeoff.

## Security implications

- Capabilities eliminate confused-deputy and ambient-root classes by construction — this is principle #4 of Nous. seL4/Zircon are the existence proofs.
- No global namespace ⇒ a compromised component sees only what was routed to it.
- pledge/unveil/Landlock give *most* of the containment benefit on existing kernels today, with no NousOS.

## Implementation complexity

- Custom microkernel: very high (person-years; verified: tens of person-years).
- Hosted capability enforcement via Landlock(Linux)/pledge(OpenBSD-style, emulate)/seccomp/Job-objects(Windows)/app-sandbox(macOS): moderate, and portable.
- Plan 9-style namespace layer in userspace (a path-router in NousBridge): low–moderate.

## How it maps to Nous modules

- **NousCaps** ⇐ seL4 + Zircon handle/capability model.
- **NousFS** ⇐ Fuchsia blobfs + Plan 9 file interface.
- **NousBridge** ⇐ Plan 9 per-process namespaces + 9P + QNX resource managers.
- **NousVM** ⇐ Inferno Dis (hosted portable runtime).
- **NousOS** ⇐ seL4-class verified microkernel + Zircon component model (long-term only).
- Coding discipline / size budget ⇐ Oberon, SerenityOS, TempleOS.

## Recommended MVP decision

Do **not** build a kernel. Enforce NousCaps on the host using the platform sandbox primitive (Landlock + seccomp on Linux, sandbox-exec/App Sandbox on macOS, Job Objects + restricted tokens on Windows). Run every module as a separate OS process. Adopt Plan 9 namespace thinking inside NousBridge as a userspace path-router. Treat 9P as the conceptual model for the bridge API, but expose plain HTTP/FUSE first.

## Recommended long-term decision

NousOS = seL4-class verified microkernel + Zircon-style component/capability routing + Plan 9 namespaces as the native userland API + Inferno-style portable VM (NousVM) as the default execution target. Keep the verified core under a strict LOC budget; everything above it is capability-confined and crash-only (see area 4).
