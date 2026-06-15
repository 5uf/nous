# 7. Post-Quantum Cryptography

Compared: ML-KEM / Kyber-class (key encapsulation, FIPS 203), ML-DSA / Dilithium-class (signatures, FIPS 204), Falcon-class (FN-DSA, compact lattice signatures), with SLH-DSA/SPHINCS+ (FIPS 205) noted for contrast.

## What to copy

- **ML-KEM (Kyber)** — NIST-standardized lattice KEM; fast, moderate key/ciphertext sizes; the default for key establishment. Copy ML-KEM-768 as the PQC KEM, used in **hybrid** with X25519.
- **ML-DSA (Dilithium)** — NIST-standardized lattice signature; the general-purpose PQC signature workhorse; larger sigs but fast and conservative. Copy ML-DSA-65 as the default PQC signature for caps/objects/manifests.
- **Falcon (FN-DSA)** — lattice signature with *much smaller* signatures than ML-DSA; ideal where signature size dominates (dense Merkle/cap chains), but constant-time floating-point sign is implementation-hazardous. Copy Falcon *only* where small sigs matter and a vetted constant-time impl exists.
- **Hybrid construction** — combine classical (X25519/Ed25519) + PQC so security holds if either survives. Copy hybrid-by-default (current IETF/industry practice).
- **Crypto agility patterns** — algorithm identifiers in every key/signature/ciphertext (like multihash / JWA `alg`); negotiated suites; versioned formats. Copy agility headers everywhere (principle #9).
- **SLH-DSA (SPHINCS+)** — hash-based, conservative, stateless; large/slow signatures but minimal assumptions. Copy as a *long-term high-assurance fallback* (only hash security needed → great for century-scale, area 8).

## What to avoid

- PQC-only (no classical hybrid) before the algorithms have more deployment maturity — hybrid hedges against new lattice attacks.
- Hand-rolled lattice/Falcon implementations — side-channel and constant-time hazards are severe; use audited libraries (liboqs / pqcrypto / vetted crates).
- Hard-coding a single algorithm with no `alg` field — defeats agility; replaceability is the point (principle #9).
- Stateful hash-based signatures (XMSS/LMS) unless state management is bulletproof — key-reuse on state loss is catastrophic; prefer stateless SLH-DSA.
- Ignoring size/perf budgets — ML-DSA/SPHINCS+ signatures are large; measure against object-store and network budgets.
- Harvest-now-decrypt-later complacency — confidentiality data needs a PQC KEM *now* even though signatures can migrate more slowly.

## Performance implications

- ML-KEM: fast keygen/encaps/decaps; ciphertext ~1KB-class — acceptable for handshakes.
- ML-DSA: fast verify, larger signatures (~2–4KB-class) — fine for objects, noticeable in dense cap chains.
- Falcon: small signatures (~0.6–1.3KB-class), fast verify, *slow/hazardous* sign — good when many sigs are stored/transmitted.
- SLH-DSA: large signatures (8–50KB) and slow — reserve for high-assurance/archival, not hot paths.
- Hybrid adds the classical op cost (negligible) on top of PQC — worth it.

## Security implications

- Harvest-now-decrypt-later: any *long-lived confidential* data must use a PQC KEM today → ML-KEM hybrid in NousNet handshakes and at-rest encryption.
- Signatures protect integrity/authenticity (caps, objects, manifests); migration can be staged because forgery requires a *future* quantum computer at *verification time*, but long-validity caps should adopt PQC sooner.
- Agility is itself a security property: retiring a broken algorithm without reformatting the world (principle #9).
- Constant-time, audited implementations are non-negotiable — most PQC breaks in practice are side-channel/impl bugs, not math.

## Implementation complexity

- Hybrid KEM/signature via a vetted library (liboqs / pqcrypto crates): low–moderate.
- Agility header/format design (alg IDs in IDs, caps, handshakes): low — but must be designed *early* to avoid format churn.
- Falcon constant-time integration: high (FP hazards) — defer unless size-critical.
- Full suite negotiation + downgrade protection in NousNet: moderate.

## How it maps to Nous modules

- **NousCaps** ⇐ ML-DSA-65 (hybrid w/ Ed25519) for cap signatures; agility header in the cap format.
- **NousNet** ⇐ ML-KEM-768 hybrid (w/ X25519) in the Noise/QUIC handshake (area 6).
- **NousFS** ⇐ SignedObject uses hybrid signatures; `alg` field in the self-describing ID/manifest.
- **NousBuild** ⇐ artifact signing (ML-DSA / SLH-DSA for archival provenance, area 8).
- **NousLang/NousVM** ⇐ verify signatures on loaded code/modules.

## Recommended MVP decision

Phase 1 cap tokens may be `signature optional in v0` per the seed spec — but **design the format with an `alg` field now**. When signing turns on (v0.1), use **Ed25519** for speed/simplicity *with an algorithm identifier in every token and object ID*, so swapping to ML-DSA-65 (or hybrid) is a format-compatible change, not a rewrite. Do not implement lattice crypto in Phase 1; do not roll your own. The only hard requirement: every signed/encrypted artifact carries a self-describing algorithm tag.

## Recommended long-term decision

Hybrid-everywhere: ML-KEM-768 + X25519 for all key establishment (NousNet, at-rest); ML-DSA-65 + Ed25519 for general signatures (NousCaps, NousFS objects, NousBuild artifacts); Falcon where signature size dominates and a vetted constant-time impl exists; SLH-DSA as the conservative hash-based option for century-scale archival signing (area 8). All formats carry algorithm identifiers and support negotiated suites with downgrade protection, so any single algorithm can be retired without reformatting existing data (principle #9). Use only audited, constant-time implementations.
