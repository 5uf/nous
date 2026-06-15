# Module: NousCaps

Explicit authority model for all resources.

## Capability schema draft

```yaml
capability:
  id: cap_...
  issuer: identity:...
  holder: identity:...
  resource: nousfs:cid:...
  rights: [read, write]
  expiry: 2026-06-16T00:00:00Z
  constraints:
    max_bytes: 1048576
    network: false
  signature: ...
```

## MVP approach

For Phase 1, implement local signed or unsigned capability tokens with expiration. Do not overbuild distributed identity at first.

## Hot path rule

Validate capability at boundary. Convert to short-lived internal handle. Avoid repeated expensive crypto checks in hot loops.
