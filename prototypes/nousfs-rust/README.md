# nousfs-rust prototype

Starter prototype for Phase 1.

Suggested stack:

```text
Rust
clap for CLI
blake3 for hashing
axum or tiny-http for HTTP gateway
serde for manifest files only, not hot paths
```

Initial commands:

```bash
cargo run -- init
cargo run -- put ./file.txt
cargo run -- get <cid> --out file.txt
cargo run -- serve --port 8080
```
