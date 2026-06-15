# Module: NousFS

Content-addressed, versioned, capability-secured object storage.

## MVP

```bash
nous init
nous put file.txt
nous get <cid>
nous ls
nous serve --http 8080
```

## Core data model

```text
Blob      -> raw bytes
Tree      -> named links to objects
Commit    -> snapshot pointer + metadata
Manifest  -> app/service/system descriptor
CapGrant  -> rights to object/service
```

## Object ID

```text
cid = multicodec(hash(content))
```

Start simple with BLAKE3 hex IDs, later multihash-compatible CIDs.

## Required properties

```text
integrity checked on read
deduplication
version history
signed commits later
HTTP exposure
FUSE mount later
local-first sync later
```
