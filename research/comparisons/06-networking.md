# 6. Networking & Transport

Compared: QUIC, WireGuard, Noise Protocol, Tor onion services, libp2p, delay-tolerant networking (DTN/Bundle Protocol).

## What to copy

- **QUIC** — UDP-based, multiplexed streams without head-of-line blocking, integrated TLS 1.3, 0-RTT/1-RTT handshakes, connection migration across IP changes. Copy QUIC as NousNet's default transport — mobility + multiplexing fit an identity-first overlay.
- **Noise Protocol Framework** — composable handshake patterns (IK, XX, NK), mutual auth from static keys, forward secrecy, small and auditable, no PKI/CA required. Copy Noise (IK pattern) as NousNet's identity-first handshake — peers authenticate by *key*, not certificate.
- **WireGuard** — cryptokey routing: identity *is* the public key; tiny attack surface; fast. Copy the key-as-identity model (aligns with NousCaps, identity-first).
- **libp2p** — `multiaddr` (self-describing addresses), `PeerId` (hash of public key = identity), transport-agnostic + protocol multiplexing + multiple discovery (mDNS, DHT, bootstrap). Copy multiaddr + PeerId + transport abstraction as NousNet's addressing/identity layer.
- **Tor onion services** — location-hidden, self-authenticating addresses (`.onion` = hash of key), rendezvous without revealing IP. Copy the *optional* self-authenticating-address pattern for privacy-sensitive use — not mandatory.
- **DTN / Bundle Protocol (RFC 9171)** — store-and-forward, custody transfer, tolerance of intermittent/long-delay links. Copy the store-and-forward model for offline/intermittent operation ("reliable under scarcity," principle #7).

## What to avoid

- Mandatory global Tor — explicitly forbidden by the seed spec; high latency, operational fragility; keep optional.
- Rolling custom crypto/transport — use vetted Noise + QUIC libraries; do not invent handshakes.
- libp2p's full stack weight as a hard early dependency — adopt its *concepts* (multiaddr/PeerId) first; pull the full stack only when real P2P is needed.
- TCP+TLS as the only transport — head-of-line blocking and no migration; QUIC supersedes for the overlay.
- DHT-by-default for discovery — unbounded latency and privacy leakage; make discovery pluggable, local-first (mDNS/bootstrap) before global DHT.
- Assuming always-on connectivity — design store-and-forward from the start so offline degrades gracefully.

## Performance implications

- QUIC removes HOL blocking and resumes fast (0-RTT), but UDP + userspace crypto costs CPU vs kernel TCP; connection migration avoids reconnect stalls on mobile.
- Noise handshakes are cheap (few asymmetric ops) and add forward secrecy with minimal overhead.
- DHT lookups add unbounded hops/latency — keep off latency-critical paths.
- Tor adds large latency (multi-hop) — opt-in only.
- DTN trades latency for delivery guarantees under disruption — correct trade for intermittent links.

## Security implications

- Key-as-identity (WireGuard/libp2p/Noise) removes CA/PKI trust roots and aligns network identity with NousCaps holders — a peer's identity is cryptographic, not administrative.
- Noise gives mutual auth + forward secrecy by default; QUIC's TLS 1.3 floor is strong.
- Self-authenticating addresses (onion/PeerId) prevent address spoofing and enable optional location privacy.
- Crypto agility is mandatory (area 7): the handshake must negotiate algorithms so PQC can drop in.
- Store-and-forward introduces at-rest message exposure — bundles must be encrypted to the recipient key.

## Implementation complexity

- QUIC via an existing library (e.g. quinn/quiche): low–moderate.
- Noise via an existing library (snow/noise-c): low–moderate.
- multiaddr/PeerId addressing layer: low–moderate.
- Full libp2p / DHT / NAT traversal: moderate–high.
- DTN store-and-forward + custody: moderate.
- Tor integration: moderate (optional).

## How it maps to Nous modules

- **NousNet** ⇐ QUIC transport + Noise IK handshake + WireGuard/libp2p key-as-identity + multiaddr/PeerId addressing + optional Tor + DTN store-and-forward.
- **NousCaps** ⇐ network identity = capability holder (key-based).
- **NousBridge** ⇐ adapts NousNet to HTTP/current web for interop.
- **NousFS** ⇐ content-addressed objects shipped over NousNet (CID request/response), DTN-friendly because objects are immutable.

## Recommended MVP decision

Phase 1 networking is just the local HTTP gateway (`GET/POST /object`, `/meta`, `/health`) — no overlay. Keep it plain HTTP/1.1 or HTTP/2 over TLS, capability-gated via `Authorization: Bearer <cap>`. Do **not** build NousNet, DHT, or Tor yet. Reserve the design: when NousNet starts, default to QUIC + Noise IK with libp2p-style PeerId identity. Record peer identity as a public key from day one even in the HTTP cap tokens, so the identity model is forward-compatible.

## Recommended long-term decision

NousNet = QUIC transport + Noise IK handshake + key-as-identity (PeerId/multiaddr), transport-agnostic with pluggable discovery (local mDNS/bootstrap before global DHT), optional Tor-style self-authenticating addresses for privacy, and DTN store-and-forward for intermittent links. All crypto is agile/PQC-ready (area 7). Network identity is unified with NousCaps so authorization and addressing share one cryptographic identity (identity-first + capability security).
