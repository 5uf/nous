# MVP Task Breakdown

## Sprint 1

- Create Rust workspace
- Implement local store directory `.nous/objects/aa/bb/hash`
- Implement BLAKE3 hashing
- `nous init`
- `nous put <path>`
- `nous get <cid> --out <path>`
- tests for hash integrity

## Sprint 2

- `nous ls`
- object metadata
- tree object draft
- HTTP server: `GET /object/<cid>`
- resource budget measurement script

## Sprint 3

- basic capability token
- `nous grant read <cid> --ttl 10m`
- HTTP bearer token validation
- audit log

## Sprint 4

- sync design doc
- peer protocol prototype
- SDK skeleton
- benchmarks
