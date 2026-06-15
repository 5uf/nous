# Module: NousBridge

Compatibility adapters for current systems.

## Required adapters

```text
HTTP gateway
FUSE mount
Git bridge
CLI
SDKs: Rust, JS, Python
WASM/browser bridge
```

## HTTP mapping example

```http
GET /object/<cid>
Authorization: Bearer <capability>
```

maps to:

```text
NousCaps validate -> NousFS read -> HTTP response
```
