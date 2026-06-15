# 3. Storage & Build / Content Addressing

Compared: IPFS, Git, ZFS, btrfs, Nix store, Guix, Bazel, Buildroot, Yocto.

## What to copy

- **Git** — Merkle DAG of content-addressed objects (blob/tree/commit); cheap branching; the object model NousFS extends. Copy blob→tree→commit layering and packfile delta compression later.
- **IPFS** — self-describing IDs: **multihash** (hash-algo + length prefix) + **multicodec** + CIDv1; **UnixFS** chunking + DAG for large files. Copy the self-describing-ID format (satisfies "ID should include algorithm metadata eventually") and content-defined chunking for dedup.
- **ZFS** — block checksums verified on every read; copy-on-write; atomic transactions; snapshots; self-healing from redundant copies; end-to-end integrity. Copy verify-on-read + COW + atomic transaction discipline.
- **btrfs** — COW + reflink (cheap copies), subvolumes, online checksum scrub. Copy reflink for cheap object materialization on the same FS.
- **Nix store** — `/nix/store/<hash>-<name>` content/input-addressed paths; pure, hermetic builds; closures with full dependency graph; binary substitution. Copy input-addressing + closure model for NousBuild.
- **Guix** — Nix semantics with a bootstrap story (reduced bootstrap seed) and full-source provenance. Copy the **bootstrappability** discipline (matters for area 8 longevity).
- **Bazel** — hermetic, content-hashed action graph; remote cache + remote execution keyed on input hashes; reproducible incremental builds. Copy action-graph caching keyed on content hashes.
- **Buildroot / Yocto** — reproducible target-image generation from pinned sources; Yocto's layered recipes + SBOM. Copy the pinned-source → image pipeline for NousBuild target images.

## What to avoid

- IPFS's heavy networking/DHT stack and default daemon for *local* storage — Phase 1 is local-first; reuse only the CID/multihash *format*.
- Git's SHA-1 legacy and poor large-binary handling (LFS bolt-on) — Nous starts at BLAKE3, no LFS debt.
- ZFS/btrfs as a dependency — they are filesystems, not libraries; NousFS sits *on top* of the host FS, not inside the kernel.
- Yocto's complexity cliff and Bazel's BUILD-file overhead for a tiny project — defer until there is a real multi-target build.
- Nix's steep language and global-daemon model — copy the *semantics* (input addressing, closures), not necessarily the Nix language.

## Performance implications

- BLAKE3 hashing is ~GB/s/core and SIMD/parallel — hashing-on-write is cheap, making content addressing affordable (decisive vs SHA-256).
- Content-defined chunking gives cross-file/version dedup but adds chunk-boundary CPU; tune chunk size against the put/get throughput targets.
- COW + atomic rename: near-zero-cost crash safety; reflink makes "copy" O(1).
- Action-graph caching (Bazel/Nix): large incremental-build wins; cache-key computation must be cheap and correct.
- DHT/network resolution (IPFS) adds unbounded latency — keep off the local hot path.

## Security implications

- Content addressing = integrity by construction: a CID *is* a verification. Tampering changes the ID (principle #6, area 8).
- Input-addressed builds (Nix/Bazel) give supply-chain provenance: output is a pure function of pinned inputs → reproducible, auditable.
- ZFS verify-on-read detects silent corruption/bit-rot — required for "reliable under scarcity" and century-scale storage.
- Capability-gated reads (NousCaps) must wrap the store; content addressing alone gives integrity, not confidentiality.

## Implementation complexity

- Local content-addressed blob store (BLAKE3 + sharded dir + atomic rename): **low** — this is Phase 1 core.
- Self-describing multihash IDs: low (format only).
- Merkle DAG (tree/commit objects): moderate (Phase 2).
- Content-defined chunking + dedup: moderate.
- Nix/Bazel-grade hermetic build graph: high — long-term NousBuild.

## How it maps to Nous modules

- **NousFS** ⇐ Git object model + IPFS CID/multihash + ZFS verify-on-read + btrfs reflink.
- **NousBuild** ⇐ Nix/Guix input-addressing & closures + Bazel action-graph cache + Buildroot/Yocto pinned-image pipeline.
- **NousCaps** wraps store access (confidentiality layer over integrity).

## Recommended MVP decision

Match the seed spec exactly: BLAKE3 blobs, sharded `objects/ab/cd/<hash>`, sidecar `meta/<hash>.toml`, atomic temp-file + rename writes, verify-on-read (`nous verify` recomputes BLAKE3). IDs are self-describing from day one via a short multihash-style prefix (algo + length) so future hash agility is free — but only BLAKE3 is implemented. No DHT, no daemon, no chunking yet, blobs only. NousBuild MVP = pinned toolchain + `SOURCE_DATE_EPOCH` + record input hashes (see area 8); no Nix/Bazel dependency.

## Recommended long-term decision

NousFS becomes a full Merkle DAG (blob/tree/commit/manifest, content-defined chunking, packfile deltas) with optional IPFS-compatible CID export for interop via NousBridge. NousBuild becomes a Nix/Guix-class input-addressed, bootstrappable, hermetic build system with a Bazel-style remote action cache and Yocto-style reproducible target images — all artifacts addressed by content and reconstructable from pinned source (principles #6, #10).
