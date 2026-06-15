# 8. Longevity, Reproducibility & Formal Methods

Compared: long-term software preservation, reproducible builds, formal specifications, multiple-implementation strategy.

## What to copy

- **Reproducible Builds** — bit-for-bit identical outputs from identical sources: `SOURCE_DATE_EPOCH`, sorted/normalized inputs, stripped non-determinism (timestamps, paths, locale, thread ordering), pinned toolchains, recorded build environment. Copy the full reproducible-builds discipline from commit one.
- **Content addressing for provenance** (area 3) — Merkle DAGs / CIDs make every input and output verifiable and immutable; the substrate of "source + IR + specs outlive binaries" (principle #6).
- **Bootstrappability** (Guix `bootstrap`, live-bootstrap, GNU Mes) — a path from a tiny auditable seed to the full toolchain; defends against trusting-trust attacks and toolchain loss over decades. Copy the reduced-bootstrap-seed goal for NousBuild.
- **Formal specification** — machine-checkable specs of interfaces and invariants. Tiers worth copying: lightweight (TLA+/Alloy for protocols, property-based testing for code), heavyweight (seL4-style proofs for the eventual NousOS core only). Copy: spec the wire/object/cap *formats* and protocol state machines formally; property-test the implementations.
- **Multiple implementation strategy** — independent implementations of one spec catch spec ambiguity and impl bugs (TLS, C compilers, Ethereum clients). Copy: a normative spec + ≥2 conforming implementations as the longevity guarantee (principle #10).
- **Self-describing formats** — files/objects that carry enough metadata (algorithm tags, schema version, format docs) to be decoded centuries later without external context. Copy for every Nous format.
- **Archival practice** — emulation over migration where possible; store the spec + reference impl + IR alongside data; multiple geographic/media copies (LOCKSS "lots of copies keep stuff safe").

## What to avoid

- Non-deterministic builds (embedded timestamps, absolute paths, unsorted dirs, parallel-order-dependent output) — silently break reproducibility and provenance.
- Binary-only artifacts with no recoverable source/IR — violates principle #6; binaries rot, specs endure.
- Over-formalization early — full proofs of the whole system freeze interfaces and cost person-years; reserve heavyweight proof for the eventual minimal NousOS core.
- Single-implementation lock-in — one impl == the spec is whatever that code does; ambiguity goes undetected.
- Format dependence on living infrastructure (a specific DB, cloud service, or proprietary codec) — anything not self-describing and openly specified is a longevity risk.
- "We'll document it later" — the spec *is* the deliverable for longevity; lagging docs decay (principle #10).

## Performance implications

- Reproducibility is largely free at runtime; cost is build-time discipline (normalization, pinning) and some lost nondeterministic optimizations.
- Bootstrappable builds are slow to build from seed but rarely run — acceptable.
- Formal verification has zero runtime cost (compile/proof-time only) but large human cost.
- Multiple implementations multiply maintenance but not runtime cost; pick the fastest conforming impl per target.

## Security implications

- Reproducible + bootstrappable builds defeat trusting-trust and supply-chain tampering: anyone can rebuild and compare hashes (ties to areas 3, 7 signing).
- Content-addressed, signed artifacts give end-to-end provenance from source to binary.
- Formal specs of cap/crypto/protocol state machines catch security-critical logic errors before shipping.
- Multiple implementations reduce monoculture risk (one impl bug ≠ total compromise).
- Self-describing + openly specified formats prevent "lost key/lost tool" denial of access over decades.

## Implementation complexity

- Reproducible build hygiene (SOURCE_DATE_EPOCH, pinned toolchain, normalized inputs): **low** — start now.
- Recording build inputs as content-addressed graph: low–moderate (reuse NousFS).
- Bootstrappable toolchain from seed: high — long-term NousBuild.
- Lightweight formal specs (TLA+/Alloy + property tests): moderate — feasible per-protocol now.
- Heavyweight proofs (seL4-class): very high — NousOS core only, long-term.
- Second conforming implementation: moderate–high per module — phase in for stable specs.

## How it maps to Nous modules

- **NousBuild** ⇐ reproducible builds + bootstrappability + recorded content-addressed input graph.
- **NousFS** ⇐ content addressing + self-describing formats = the preservation substrate.
- **All modules** ⇐ normative spec + property tests; ≥2 implementations for the most critical (NousCaps, NousFS, crypto, wire format) over time.
- **NousOS** ⇐ heavyweight formal proof of the minimal core (long-term, area 1).
- **NousLang** ⇐ formally specified semantics so source/IR is unambiguous across centuries (principle #6).

## Recommended MVP decision

From commit one: build reproducibly (set `SOURCE_DATE_EPOCH`, pin the Rust toolchain via `rust-toolchain.toml`, `--locked` dependencies, normalized/sorted outputs), and **write a short normative spec** for the only formats that exist in Phase 1 — the object ID/multihash format, the `meta/<hash>.toml` schema, and the capability token fields. Property-test the store (round-trip put/get, corruption detection, atomic-write crash safety). Do *not* attempt formal proofs or a second implementation yet. Keep every format self-describing (algorithm + version tags) so Phase 1 artifacts remain decodable forever.

## Recommended long-term decision

NousBuild delivers fully reproducible, bootstrappable (reduced-seed) builds with a content-addressed input graph; every artifact is signed (area 7) and reconstructable from pinned source + IR + spec (principle #6). Critical modules (NousCaps, NousFS, crypto, NousNet wire format, NousLang semantics) get formal specifications (TLA+/Alloy + machine-checked invariants) and **at least two independent conforming implementations**. The eventual NousOS core gets seL4-class proof. Specs, reference implementations, and IR are stored *alongside* data in NousFS, in self-describing formats, in multiple copies — so the system is reconstructable over centuries (principle #10).
