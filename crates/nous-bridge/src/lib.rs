//! `nous-bridge` — adapters between NousFS and the host world.
//!
//! Phase 4 implements the POSIX/Git-style bridge: snapshot a directory tree
//! into content-addressed [`Tree`]/[`Commit`] objects, restore a commit back
//! to the filesystem, and walk commit history.
//!
//! Files larger than [`CHUNK_THRESHOLD`] are stored as chunked [`File`]
//! objects (cross-version dedup); smaller files are stored as plain blobs.
//!
//! A FUSE mount adapter is intentionally not built here: it requires a
//! platform FUSE library (macFUSE / libfuse) and cannot be verified in a
//! headless environment. It belongs behind an optional cargo feature in a
//! later phase.

use std::path::Path;

use nous_core::{Commit, Object, ObjectId, ObjectKind, Result, Tree, TreeEntry};
use nous_store::Store;

/// Files at or above this size are stored chunked; smaller files are blobs.
pub const CHUNK_THRESHOLD: usize = 64 * 1024;

/// The directory name that holds the store itself; never snapshotted.
const STORE_DIR: &str = ".nous";

// ---------------------------------------------------------------------------
// Snapshot (filesystem -> NousFS)
// ---------------------------------------------------------------------------

/// Recursively snapshot `dir` into a [`Tree`] object, returning its id.
///
/// Entries are sorted by name (via [`Tree::new`]) so the tree id is a
/// deterministic function of directory contents. Symlinks and the `.nous`
/// store directory are skipped.
pub fn snapshot_tree(store: &Store, dir: &Path) -> Result<ObjectId> {
    let mut entries = Vec::new();

    let mut names: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .collect();
    names.sort_by_key(|e| e.file_name());

    for entry in names {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == STORE_DIR {
            continue;
        }
        let path = entry.path();
        let ft = entry.file_type()?;
        if ft.is_symlink() {
            continue; // not represented in v0
        } else if ft.is_dir() {
            let sub = snapshot_tree(store, &path)?;
            entries.push(TreeEntry { name, id: sub, kind: ObjectKind::Tree });
        } else if ft.is_file() {
            let data = std::fs::read(&path)?;
            let (id, kind) = if data.len() >= CHUNK_THRESHOLD {
                (store.put_file(&data)?, ObjectKind::File)
            } else {
                (store.put(&data, None)?, ObjectKind::Blob)
            };
            entries.push(TreeEntry { name, id, kind });
        }
    }

    store.put_object(&Object::Tree(Tree::new(entries)))
}

/// Snapshot `dir` and wrap it in a [`Commit`], returning the commit id.
///
/// `timestamp` is supplied by the caller (Unix seconds) so the function has no
/// hidden clock dependency and is testable/deterministic.
pub fn snapshot(
    store: &Store,
    dir: &Path,
    author: &str,
    message: &str,
    parent: Option<ObjectId>,
    timestamp: i64,
) -> Result<ObjectId> {
    let tree = snapshot_tree(store, dir)?;
    let commit = Commit {
        tree,
        parents: parent.into_iter().collect(),
        author: author.to_string(),
        message: message.to_string(),
        timestamp,
    };
    store.put_object(&Object::Commit(commit))
}

// ---------------------------------------------------------------------------
// Restore (NousFS -> filesystem)
// ---------------------------------------------------------------------------

/// Restore a [`Tree`] object to `dest`, creating directories and files.
pub fn restore_tree(store: &Store, tree_id: &ObjectId, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    let tree = store.get_tree(tree_id)?;
    for e in tree.entries {
        let target = dest.join(&e.name);
        match e.kind {
            ObjectKind::Tree => restore_tree(store, &e.id, &target)?,
            ObjectKind::Blob => {
                let data = store.get(&e.id)?;
                std::fs::write(&target, data)?;
            }
            ObjectKind::File => {
                let data = store.get_file(&e.id)?;
                std::fs::write(&target, data)?;
            }
            other => {
                return Err(nous_core::Error::Other(format!(
                    "unexpected tree entry kind {other:?} for {}",
                    e.name
                )))
            }
        }
    }
    Ok(())
}

/// Restore the tree referenced by `commit_id` to `dest`.
pub fn restore(store: &Store, commit_id: &ObjectId, dest: &Path) -> Result<()> {
    let commit = store.get_commit(commit_id)?;
    restore_tree(store, &commit.tree, dest)
}

// ---------------------------------------------------------------------------
// History
// ---------------------------------------------------------------------------

/// Walk the first-parent commit chain starting at `commit_id`, newest first.
pub fn log(store: &Store, commit_id: &ObjectId) -> Result<Vec<(ObjectId, Commit)>> {
    let mut out = Vec::new();
    let mut cur = Some(*commit_id);
    while let Some(id) = cur {
        let commit = store.get_commit(&id)?;
        let next = commit.parents.first().copied();
        out.push((id, commit));
        cur = next;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("nous-bridge-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn write(p: &Path, rel: &str, bytes: &[u8]) {
        let f = p.join(rel);
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(f, bytes).unwrap();
    }

    #[test]
    fn snapshot_restore_round_trip() {
        let base = tmp("rt");
        let src = base.join("src");
        let store_dir = base.join("store");
        let dest = base.join("dest");
        std::fs::create_dir_all(&src).unwrap();
        let store = Store::init(&store_dir).unwrap();

        write(&src, "a.txt", b"alpha");
        write(&src, "sub/b.txt", b"beta");
        write(&src, "sub/deep/c.bin", &vec![7u8; 200_000]); // chunked

        let commit = snapshot(&store, &src, "tester", "init", None, 1000).unwrap();
        restore(&store, &commit, &dest).unwrap();

        assert_eq!(std::fs::read(dest.join("a.txt")).unwrap(), b"alpha");
        assert_eq!(std::fs::read(dest.join("sub/b.txt")).unwrap(), b"beta");
        assert_eq!(std::fs::read(dest.join("sub/deep/c.bin")).unwrap(), vec![7u8; 200_000]);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn snapshot_tree_is_deterministic() {
        let base = tmp("det");
        let src = base.join("src");
        let store_dir = base.join("store");
        std::fs::create_dir_all(&src).unwrap();
        let store = Store::init(&store_dir).unwrap();
        write(&src, "x", b"1");
        write(&src, "y", b"2");

        let t1 = snapshot_tree(&store, &src).unwrap();
        let t2 = snapshot_tree(&store, &src).unwrap();
        assert_eq!(t1, t2, "same dir contents -> same tree id");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn log_walks_parents() {
        let base = tmp("log");
        let src = base.join("src");
        let store_dir = base.join("store");
        std::fs::create_dir_all(&src).unwrap();
        let store = Store::init(&store_dir).unwrap();

        write(&src, "f", b"v1");
        let c1 = snapshot(&store, &src, "a", "first", None, 1).unwrap();
        write(&src, "f", b"v2");
        let c2 = snapshot(&store, &src, "a", "second", Some(c1), 2).unwrap();

        let history = log(&store, &c2).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].0, c2);
        assert_eq!(history[0].1.message, "second");
        assert_eq!(history[1].0, c1);
        assert_eq!(history[1].1.message, "first");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn store_dir_is_skipped() {
        let base = tmp("skip");
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        // simulate an in-place store: src/.nous should be ignored
        let store = Store::init(&src.join(".nous")).unwrap();
        write(&src, "real.txt", b"data");

        let tree_id = snapshot_tree(&store, &src).unwrap();
        let tree = store.get_tree(&tree_id).unwrap();
        let names: Vec<_> = tree.entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"real.txt"));
        assert!(!names.contains(&".nous"));
        let _ = std::fs::remove_dir_all(&base);
    }
}
