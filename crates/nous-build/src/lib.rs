//! `nous-build` — reproducible packaging and portable object bundles.
//!
//! Two capabilities:
//!
//! 1. [`package`]: wrap a content root (usually a [`Tree`]) in a [`Manifest`]
//!    with a name, version, and pinned metadata — a reproducible package
//!    descriptor (its id is a pure function of its inputs).
//!
//! 2. [`export`] / [`import`]: serialize the entire object DAG reachable from a
//!    root into a single self-describing, content-addressed archive, and load
//!    it back into another store with per-object integrity verification.
//!
//! The archive is deterministic (objects are emitted in sorted id order), so
//! the same DAG always produces byte-identical bytes — reproducible
//! distribution and a precursor to networked sync.
//!
//! ## Archive format (`NOUSAR01`)
//! ```text
//! magic   : 8 bytes  = b"NOUSAR01"
//! count   : u32 LE   = number of objects
//! object* :
//!   algo      : u8        (hash algorithm code; 0x1e = BLAKE3)
//!   digest    : 32 bytes
//!   ct_len    : u16 LE    (content-type length)
//!   ct        : ct_len bytes (UTF-8; empty = none)
//!   data_len  : u64 LE
//!   data      : data_len bytes
//! ```

use std::collections::{BTreeMap, HashSet};

use nous_core::{Error, HashAlgo, Manifest, Object, ObjectId, ObjectKind, Result};
use nous_store::Store;

const MAGIC: &[u8; 8] = b"NOUSAR01";

// ---------------------------------------------------------------------------
// Packaging
// ---------------------------------------------------------------------------

/// Build a [`Manifest`] over `root` and store it, returning the manifest id.
pub fn package(
    store: &Store,
    root: ObjectId,
    name: &str,
    version: &str,
    meta: BTreeMap<String, String>,
) -> Result<ObjectId> {
    let manifest = Manifest {
        name: name.to_string(),
        version: version.to_string(),
        root,
        meta,
    };
    store.put_object(&Object::Manifest(manifest))
}

// ---------------------------------------------------------------------------
// DAG traversal
// ---------------------------------------------------------------------------

/// Determine an object's kind from its stored metadata content-type.
/// Anything without a recognised `application/nous-*` type is treated as a blob.
pub fn kind_of(store: &Store, id: &ObjectId) -> Result<ObjectKind> {
    let ct = store.get_meta(id)?.content_type;
    Ok(match ct.as_deref() {
        Some("application/nous-tree") => ObjectKind::Tree,
        Some("application/nous-commit") => ObjectKind::Commit,
        Some("application/nous-manifest") => ObjectKind::Manifest,
        Some("application/nous-file") => ObjectKind::File,
        _ => ObjectKind::Blob,
    })
}

/// Collect every object id reachable from `root` (inclusive).
///
/// Tree entries carry their own kind, so traversal is exact; the root's kind
/// and a manifest's root kind are resolved via [`kind_of`].
pub fn reachable(store: &Store, root: ObjectId) -> Result<HashSet<ObjectId>> {
    let mut seen = HashSet::new();
    let mut stack = vec![(root, kind_of(store, &root)?)];

    while let Some((id, kind)) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        match kind {
            ObjectKind::Blob => {}
            ObjectKind::File => {
                if let Object::File(f) = store.get_object(&id)? {
                    for c in f.chunks {
                        stack.push((c, ObjectKind::Blob));
                    }
                }
            }
            ObjectKind::Tree => {
                for e in store.get_tree(&id)?.entries {
                    stack.push((e.id, e.kind));
                }
            }
            ObjectKind::Commit => {
                let c = store.get_commit(&id)?;
                stack.push((c.tree, ObjectKind::Tree));
                for p in c.parents {
                    stack.push((p, ObjectKind::Commit));
                }
            }
            ObjectKind::Manifest => {
                let m = store.get_manifest(&id)?;
                let k = kind_of(store, &m.root)?;
                stack.push((m.root, k));
            }
        }
    }
    Ok(seen)
}

// ---------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------

/// Serialize the full object DAG reachable from `root` into a portable archive.
/// Objects are emitted in sorted id order, so output is deterministic.
pub fn export(store: &Store, root: ObjectId) -> Result<Vec<u8>> {
    let mut ids: Vec<ObjectId> = reachable(store, root)?.into_iter().collect();
    ids.sort_by(|a, b| (a.algo.code(), a.digest).cmp(&(b.algo.code(), b.digest)));

    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(ids.len() as u32).to_le_bytes());

    for id in &ids {
        let data = store.get(id)?; // verifies on read
        let ct = store.get_meta(id)?.content_type.unwrap_or_default();

        out.push(id.algo.code());
        out.extend_from_slice(&id.digest);
        out.extend_from_slice(&(ct.len() as u16).to_le_bytes());
        out.extend_from_slice(ct.as_bytes());
        out.extend_from_slice(&(data.len() as u64).to_le_bytes());
        out.extend_from_slice(&data);
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Import
// ---------------------------------------------------------------------------

/// A cursor over the archive byte stream with bounds-checked reads.
struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or_else(|| Error::Other("archive length overflow".into()))?;
        if end > self.buf.len() {
            return Err(Error::Other("archive truncated".into()));
        }
        let s = &self.buf[self.pos..end];
        self.pos = end;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }
    fn u32(&mut self) -> Result<u32> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn u64(&mut self) -> Result<u64> {
        let b = self.take(8)?;
        let mut a = [0u8; 8];
        a.copy_from_slice(b);
        Ok(u64::from_le_bytes(a))
    }
}

/// Load an archive produced by [`export`] into `store`.
///
/// Every object's bytes are re-hashed and checked against its recorded id;
/// a mismatch aborts with [`Error::Corrupt`]. Returns the number of objects
/// imported.
pub fn import(store: &Store, bytes: &[u8]) -> Result<usize> {
    let mut c = Cursor { buf: bytes, pos: 0 };

    if c.take(8)? != MAGIC {
        return Err(Error::Other("not a NOUSAR01 archive".into()));
    }
    let count = c.u32()? as usize;

    for _ in 0..count {
        let algo_code = c.u8()?;
        let algo = HashAlgo::from_code(algo_code)
            .ok_or_else(|| Error::Other(format!("unknown hash algo code {algo_code}")))?;
        let digest_slice = c.take(32)?;
        let mut digest = [0u8; 32];
        digest.copy_from_slice(digest_slice);
        let expected = ObjectId { algo, digest };

        let ct_len = c.u16()? as usize;
        let ct_bytes = c.take(ct_len)?;
        let ct = if ct_len == 0 {
            None
        } else {
            Some(
                std::str::from_utf8(ct_bytes)
                    .map_err(|_| Error::Other("invalid content-type utf8".into()))?
                    .to_string(),
            )
        };

        let data_len = c.u64()? as usize;
        let data = c.take(data_len)?;

        // Integrity: bytes must hash to the claimed id.
        if ObjectId::of_bytes(data) != expected {
            return Err(Error::Corrupt {
                expected: expected.to_string(),
                actual: ObjectId::of_bytes(data).to_string(),
            });
        }

        let stored = store.put(data, ct)?;
        if stored != expected {
            return Err(Error::Corrupt {
                expected: expected.to_string(),
                actual: stored.to_string(),
            });
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nous_core::{Tree, TreeEntry};

    fn tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("nous-build-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    /// Build a small DAG in `store`: a tree with a blob and a chunked file,
    /// wrapped in a manifest. Returns the manifest id.
    fn build_dag(store: &Store) -> ObjectId {
        let blob = store.put(b"small file", None).unwrap();
        let big = store.put_file(&vec![9u8; 200_000]).unwrap();
        let tree = store
            .put_object(&Object::Tree(Tree::new(vec![
                TreeEntry { name: "a".into(), id: blob, kind: ObjectKind::Blob },
                TreeEntry { name: "big".into(), id: big, kind: ObjectKind::File },
            ])))
            .unwrap();
        package(store, tree, "demo", "1.0.0", BTreeMap::new()).unwrap()
    }

    #[test]
    fn export_is_deterministic() {
        let dir = tmp("det");
        let store = Store::init(&dir).unwrap();
        let root = build_dag(&store);
        let a = export(&store, root).unwrap();
        let b = export(&store, root).unwrap();
        assert_eq!(a, b, "export must be byte-deterministic");
        assert!(a.starts_with(MAGIC));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn export_import_round_trip_into_fresh_store() {
        let dir_a = tmp("rt-a");
        let dir_b = tmp("rt-b");
        let a = Store::init(&dir_a).unwrap();
        let b = Store::init(&dir_b).unwrap();

        let root = build_dag(&a);
        let archive = export(&a, root).unwrap();
        let n = import(&b, &archive).unwrap();

        // Same set of objects now present in b.
        let ra = reachable(&a, root).unwrap();
        let rb = reachable(&b, root).unwrap();
        assert_eq!(ra, rb);
        assert_eq!(n, ra.len());

        // Manifest -> tree -> chunked file reassembles correctly in b.
        let m = b.get_manifest(&root).unwrap();
        let tree = b.get_tree(&m.root).unwrap();
        let big_entry = tree.entries.iter().find(|e| e.name == "big").unwrap();
        assert_eq!(b.get_file(&big_entry.id).unwrap(), vec![9u8; 200_000]);

        let _ = std::fs::remove_dir_all(&dir_a);
        let _ = std::fs::remove_dir_all(&dir_b);
    }

    #[test]
    fn import_rejects_corrupted_archive() {
        let dir_a = tmp("corrupt-a");
        let dir_b = tmp("corrupt-b");
        let a = Store::init(&dir_a).unwrap();
        let b = Store::init(&dir_b).unwrap();
        let root = build_dag(&a);
        let mut archive = export(&a, root).unwrap();

        // Flip a byte in the last object's data region.
        let last = archive.len() - 1;
        archive[last] ^= 0xFF;
        assert!(matches!(import(&b, &archive), Err(Error::Corrupt { .. })));

        let _ = std::fs::remove_dir_all(&dir_a);
        let _ = std::fs::remove_dir_all(&dir_b);
    }

    #[test]
    fn import_rejects_bad_magic() {
        let dir = tmp("magic");
        let store = Store::init(&dir).unwrap();
        assert!(import(&store, b"GARBAGE!").is_err());
        assert!(import(&store, b"").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reachable_counts_all_objects() {
        let dir = tmp("reach");
        let store = Store::init(&dir).unwrap();
        let root = build_dag(&store);
        let ids = reachable(&store, root).unwrap();
        // manifest + tree + blob + file-object + (>=1 chunk) = at least 5
        assert!(ids.len() >= 5, "expected >=5 objects, got {}", ids.len());
        assert!(ids.contains(&root));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
