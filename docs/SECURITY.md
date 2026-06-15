# Nous Security Model

## Authority model

No root. No global admin. Authority flows through explicit capabilities.

A capability may include:

```text
resource id
rights
issuer
holder
expiry
revocation handle
constraints
signature
```

## Example

```yaml
resource: nousfs:cid:bafy...
rights: [read]
holder: device:sufi-laptop
expiry: 10m
constraints:
  network: false
  copy: false
```

## Crypto agility

Every cryptographic object must specify algorithm, version, parameters, and migration path.

Initial algorithm families to support:

```text
Hashes: BLAKE3, SHA-256, SHA-3
Classical signatures: Ed25519
Post-quantum signatures: Dilithium/Falcon class
Classical key exchange: X25519
Post-quantum KEM: Kyber/ML-KEM class
```

Use hybrid modes where appropriate.

## Web3 stance

Do not make blockchain mandatory.

Keep:

```text
public-key identity
content addressing
verifiable provenance
optional timestamping / audit ledgers
```

Reject as foundation:

```text
global consensus for everything
gas fees for basic operations
token dependency
immutable mistakes without recovery
```
