# Migration Strategy

Nous must be usable side-by-side with current systems.

## Staircase adoption

```text
1. Install NousCLI and NousFS daemon
2. Use HTTP gateway
3. Mount NousFS with FUSE
4. Use NousNet for identity sync
5. Use NousAI for local model management
6. Target NousVM for apps
7. Boot NousOS only when useful
```

## Required bridges

```text
NousHTTP      -> expose Nous services over HTTP/HTTPS
NousFUSE      -> mount NousFS on Linux/macOS
NousCLI       -> shell interface
NousSDK       -> JS/Python/Rust clients
NousGateway   -> bridge HTTP/WebSocket/QUIC to NousNet
NousDNS       -> map DNS names to Nous identities
NousGitBridge -> sync Git repos into NousFS
NousWASM      -> run limited Nous apps in browsers
NousPOSIX     -> compatibility layer for selected legacy apps
```

## HTTP example

```text
GET https://example.com/nous/object/<cid>
  -> NousHTTP Gateway
  -> capability check
  -> NousFS object read
```

## Native example

```text
nous://device.sufi/photos/2026/car.jpg
```

## Rule

Current systems do not need to understand Nous. Nous must understand current systems.
