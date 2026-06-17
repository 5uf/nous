//! Phase 2 object model: typed, content-addressed objects.
//!
//! Objects use a *deterministic canonical encoding* (compact JSON) so the same
//! logical object always produces identical bytes and therefore the same
//! [`ObjectId`].  This is required for content addressing and reproducibility:
//!
//! - struct fields serialize in declaration order,
//! - [`std::collections::BTreeMap`] fields serialize with sorted keys,
//! - [`Tree`] entries are kept sorted by name,
//! - no floating-point values appear anywhere.
//!
//! The [`Object`] enum is internally tagged with a `"kind"` field, so encoded
//! bytes are self-describing.

use crate::{Error, ObjectId, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Kind of a stored object. `Blob` is raw bytes (not part of [`Object`]); the
/// rest are structured objects.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ObjectKind {
    Blob,
    Tree,
    Commit,
    Manifest,
    File,
}

/// One entry in a [`Tree`]: a name bound to the id of a blob or subtree.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeEntry {
    pub name: String,
    pub id: ObjectId,
    pub kind: ObjectKind,
}

/// A directory-like mapping of names to objects. Entries are kept sorted by
/// name so the encoding is canonical regardless of insertion order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tree {
    pub entries: Vec<TreeEntry>,
}

/// A point-in-time snapshot pointing at a root [`Tree`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commit {
    pub tree: ObjectId,
    /// Parent commits; order is semantic and preserved.
    pub parents: Vec<ObjectId>,
    pub author: String,
    pub message: String,
    /// Unix seconds.
    pub timestamp: i64,
}

/// A large file split into content-defined chunks. The chunk ids reference
/// blobs (stored separately); concatenating them in order reproduces the file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct File {
    /// Total byte length of the reassembled file.
    pub size: u64,
    /// Ordered chunk blob ids.
    pub chunks: Vec<ObjectId>,
}

/// A named, versioned pointer to a root object plus arbitrary metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub root: ObjectId,
    /// Sorted (BTreeMap) for canonical encoding.
    pub meta: BTreeMap<String, String>,
}

/// A structured, content-addressed object. Internally tagged with `kind` so
/// the encoded bytes are self-describing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Object {
    Tree(Tree),
    Commit(Commit),
    Manifest(Manifest),
    File(File),
}

impl Tree {
    /// Build a tree, sorting entries by name so the result is canonical and
    /// independent of input order.
    pub fn new(mut entries: Vec<TreeEntry>) -> Tree {
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Tree { entries }
    }
}

impl Object {
    /// Deterministic canonical bytes (compact JSON).
    pub fn encode(&self) -> Vec<u8> {
        // serde_json with our field/ordering discipline is deterministic.
        // Serialization of these types cannot fail, so unwrap is safe.
        serde_json::to_vec(self).expect("object serialization is infallible")
    }

    /// Parse canonical bytes back into an object (dispatches on `kind`).
    pub fn decode(bytes: &[u8]) -> Result<Object> {
        serde_json::from_slice(bytes)
            .map_err(|e| Error::Other(format!("object decode failed: {e}")))
    }

    /// Content id of this object = BLAKE3 of its canonical encoding.
    pub fn id(&self) -> ObjectId {
        ObjectId::of_bytes(&self.encode())
    }

    pub fn kind(&self) -> ObjectKind {
        match self {
            Object::Tree(_) => ObjectKind::Tree,
            Object::Commit(_) => ObjectKind::Commit,
            Object::Manifest(_) => ObjectKind::Manifest,
            Object::File(_) => ObjectKind::File,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tree() -> Tree {
        Tree::new(vec![
            TreeEntry { name: "zeta.txt".into(), id: ObjectId::of_bytes(b"z"), kind: ObjectKind::Blob },
            TreeEntry { name: "alpha.txt".into(), id: ObjectId::of_bytes(b"a"), kind: ObjectKind::Blob },
        ])
    }

    #[test]
    fn encode_is_deterministic() {
        let a = Object::Tree(sample_tree());
        let b = Object::Tree(sample_tree());
        assert_eq!(a.encode(), b.encode());
        assert_eq!(a.id(), b.id());
    }

    #[test]
    fn tree_new_sorts_and_id_is_order_independent() {
        let t1 = Tree::new(vec![
            TreeEntry { name: "b".into(), id: ObjectId::of_bytes(b"1"), kind: ObjectKind::Blob },
            TreeEntry { name: "a".into(), id: ObjectId::of_bytes(b"2"), kind: ObjectKind::Blob },
        ]);
        assert_eq!(t1.entries[0].name, "a");
        let t2 = Tree::new(vec![
            TreeEntry { name: "a".into(), id: ObjectId::of_bytes(b"2"), kind: ObjectKind::Blob },
            TreeEntry { name: "b".into(), id: ObjectId::of_bytes(b"1"), kind: ObjectKind::Blob },
        ]);
        assert_eq!(Object::Tree(t1).id(), Object::Tree(t2).id());
    }

    #[test]
    fn round_trip_tree_commit_manifest() {
        let tree = Object::Tree(sample_tree());
        let dec = Object::decode(&tree.encode()).unwrap();
        assert_eq!(tree, dec);
        assert_eq!(tree.id(), dec.id());

        let commit = Object::Commit(Commit {
            tree: ObjectId::of_bytes(b"tree"),
            parents: vec![ObjectId::of_bytes(b"p1"), ObjectId::of_bytes(b"p2")],
            author: "alice".into(),
            message: "init".into(),
            timestamp: 1_000_000,
        });
        let dec = Object::decode(&commit.encode()).unwrap();
        assert_eq!(commit, dec);

        let mut meta = BTreeMap::new();
        meta.insert("license".to_string(), "MIT".to_string());
        let manifest = Object::Manifest(Manifest {
            name: "demo".into(),
            version: "0.1.0".into(),
            root: ObjectId::of_bytes(b"root"),
            meta,
        });
        let dec = Object::decode(&manifest.encode()).unwrap();
        assert_eq!(manifest, dec);
        assert_eq!(manifest.id(), dec.id());
    }

    #[test]
    fn encoded_bytes_carry_kind_discriminator() {
        let s = String::from_utf8(Object::Tree(sample_tree()).encode()).unwrap();
        assert!(s.contains("\"kind\""));
        assert!(s.contains("\"tree\""));
    }

    #[test]
    fn commit_parent_order_preserved() {
        let p1 = ObjectId::of_bytes(b"first");
        let p2 = ObjectId::of_bytes(b"second");
        let commit = Object::Commit(Commit {
            tree: ObjectId::of_bytes(b"t"),
            parents: vec![p1, p2],
            author: "a".into(),
            message: "m".into(),
            timestamp: 1,
        });
        if let Object::Commit(c) = Object::decode(&commit.encode()).unwrap() {
            assert_eq!(c.parents, vec![p1, p2]);
        } else {
            panic!("expected commit");
        }
    }
}
