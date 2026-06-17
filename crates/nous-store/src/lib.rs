//! `nous-store` — content-addressed object store for the Nous workspace.
//!
//! Objects are keyed by their BLAKE3 digest ([`ObjectId`]).  All writes are
//! atomic (write to `tmp/`, then rename).  Every `get` verifies the stored
//! bytes against the id before returning them.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use nous_core::{Commit, Error, File, Manifest, Meta, Object, ObjectId, Result, Tree};

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

/// A content-addressed object store rooted at a single directory.
///
/// Layout:
/// ```text
/// <root>/
///   config.toml           {version = 1, algo = "b3"}
///   objects/ab/cd/<64hex> object bytes
///   meta/<64hex>.toml     serialised Meta
///   caps/                 (reserved, empty)
///   logs/                 (reserved, empty)
///   tmp/                  staging area for atomic writes
/// ```
#[derive(Debug, Clone)]
pub struct Store {
    root: PathBuf,
}

impl Store {
    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    /// Create the `.nous` skeleton at `root` (root **is** the `.nous` dir).
    ///
    /// Creates `objects/`, `meta/`, `caps/`, `logs/`, `tmp/`, and writes
    /// `config.toml`.  Idempotent — calling `init` on an existing store is
    /// safe.
    pub fn init(root: &Path) -> Result<Store> {
        for subdir in &["objects", "meta", "caps", "logs", "tmp"] {
            std::fs::create_dir_all(root.join(subdir))?;
        }
        let cfg = root.join("config.toml");
        if !cfg.exists() {
            std::fs::write(&cfg, "version = 1\nalgo = \"b3\"\n")?;
        }
        Ok(Store { root: root.to_path_buf() })
    }

    /// Open an existing store.
    ///
    /// Returns [`Error::NotFound`] if `root/config.toml` is absent.
    pub fn open(root: &Path) -> Result<Store> {
        let cfg = root.join("config.toml");
        if !cfg.exists() {
            return Err(Error::NotFound(format!(
                "config.toml not found in {}",
                root.display()
            )));
        }
        Ok(Store { root: root.to_path_buf() })
    }

    // -----------------------------------------------------------------------
    // Accessors
    // -----------------------------------------------------------------------

    /// Return the root directory of this store.
    pub fn root(&self) -> &Path {
        &self.root
    }

    // -----------------------------------------------------------------------
    // Write
    // -----------------------------------------------------------------------

    /// Hash `data`, write the object and its sidecar metadata, return the id.
    ///
    /// Idempotent — re-putting identical bytes returns the same id without
    /// error and without redundant disk writes.
    pub fn put(&self, data: &[u8], content_type: Option<String>) -> Result<ObjectId> {
        let id = ObjectId::of_bytes(data);

        // Fast-path: already stored.
        if self.has(&id) {
            return Ok(id);
        }

        // --- object bytes (atomic) ------------------------------------------
        let obj_path = self.object_path(&id);
        std::fs::create_dir_all(obj_path.parent().unwrap())?;
        let tmp_obj = self.unique_tmp("obj");
        std::fs::write(&tmp_obj, data)?;
        std::fs::rename(&tmp_obj, &obj_path)?;

        // --- meta sidecar (atomic) ------------------------------------------
        let meta = Meta {
            id: id.to_string(),
            algo: id.algo.name().to_owned(),
            size: data.len() as u64,
            created: current_timestamp(),
            content_type,
        };
        let toml_str = toml::to_string(&meta)
            .map_err(|e| Error::Other(format!("toml serialise: {e}")))?;
        let meta_path = self.meta_path(&id);
        let tmp_meta = self.unique_tmp("meta");
        std::fs::write(&tmp_meta, toml_str)?;
        std::fs::rename(&tmp_meta, &meta_path)?;

        Ok(id)
    }

    /// Read a file from disk and store it.  `content_type` is left `None`.
    pub fn put_path(&self, path: &Path) -> Result<ObjectId> {
        let data = std::fs::read(path)?;
        self.put(&data, None)
    }

    // -----------------------------------------------------------------------
    // Read
    // -----------------------------------------------------------------------

    /// Read object bytes, verifying integrity on every read.
    ///
    /// Returns [`Error::NotFound`] if absent, [`Error::Corrupt`] if the
    /// stored bytes do not match the id.
    pub fn get(&self, id: &ObjectId) -> Result<Vec<u8>> {
        let path = self.object_path(id);
        if !path.exists() {
            return Err(Error::NotFound(id.to_string()));
        }
        let data = std::fs::read(&path)?;
        let actual = ObjectId::of_bytes(&data);
        if actual != *id {
            return Err(Error::Corrupt {
                expected: id.to_string(),
                actual: actual.to_string(),
            });
        }
        Ok(data)
    }

    /// Read the metadata sidecar for an object.
    pub fn get_meta(&self, id: &ObjectId) -> Result<Meta> {
        let path = self.meta_path(id);
        if !path.exists() {
            return Err(Error::NotFound(id.to_string()));
        }
        let raw = std::fs::read_to_string(&path)?;
        toml::from_str::<Meta>(&raw).map_err(|e| Error::Corrupt {
            expected: "valid Meta toml".into(),
            actual: e.to_string(),
        })
    }

    /// Return `true` if the object is present in the store.
    pub fn has(&self, id: &ObjectId) -> bool {
        self.object_path(id).exists()
    }

    /// List all stored object ids by scanning the `meta/` directory.
    pub fn list(&self) -> Result<Vec<ObjectId>> {
        let meta_dir = self.root.join("meta");
        let mut ids = Vec::new();
        for entry in std::fs::read_dir(&meta_dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            // Each file is named "<64hex>.toml"
            if let Some(hex) = name.strip_suffix(".toml") {
                if let Ok(id) = format!("b3:{hex}").parse::<ObjectId>() {
                    ids.push(id);
                }
            }
        }
        Ok(ids)
    }

    /// Recompute the hash of the stored bytes and compare to `id`.
    ///
    /// Returns `true` if the object is intact.  Returns [`Error::NotFound`]
    /// if the object does not exist.
    pub fn verify(&self, id: &ObjectId) -> Result<bool> {
        let path = self.object_path(id);
        if !path.exists() {
            return Err(Error::NotFound(id.to_string()));
        }
        let data = std::fs::read(&path)?;
        Ok(ObjectId::of_bytes(&data) == *id)
    }

    // -----------------------------------------------------------------------
    // Typed objects (Phase 2)
    // -----------------------------------------------------------------------

    /// Store a structured [`Object`] (encoded canonically) and return its id.
    /// The object is stored as an ordinary blob; its content_type records the
    /// kind (`application/nous-<kind>`).
    pub fn put_object(&self, obj: &Object) -> Result<ObjectId> {
        let kind = format!("application/nous-{:?}", obj.kind()).to_lowercase();
        let id = self.put(&obj.encode(), Some(kind))?;
        debug_assert_eq!(id, obj.id());
        Ok(id)
    }

    /// Load and decode a structured object (verifies bytes on read via `get`).
    pub fn get_object(&self, id: &ObjectId) -> Result<Object> {
        Object::decode(&self.get(id)?)
    }

    /// Load an object expected to be a [`Tree`].
    pub fn get_tree(&self, id: &ObjectId) -> Result<Tree> {
        match self.get_object(id)? {
            Object::Tree(t) => Ok(t),
            other => Err(Error::Other(format!("expected tree, got {:?}", other.kind()))),
        }
    }

    /// Load an object expected to be a [`Commit`].
    pub fn get_commit(&self, id: &ObjectId) -> Result<Commit> {
        match self.get_object(id)? {
            Object::Commit(c) => Ok(c),
            other => Err(Error::Other(format!("expected commit, got {:?}", other.kind()))),
        }
    }

    /// Load an object expected to be a [`Manifest`].
    pub fn get_manifest(&self, id: &ObjectId) -> Result<Manifest> {
        match self.get_object(id)? {
            Object::Manifest(m) => Ok(m),
            other => Err(Error::Other(format!("expected manifest, got {:?}", other.kind()))),
        }
    }

    // -----------------------------------------------------------------------
    // Large files via content-defined chunking (Phase 3)
    // -----------------------------------------------------------------------

    /// Store `data` as a content-defined-chunked [`File`]: each chunk is stored
    /// as a blob (deduplicated by content), then a `File` object listing the
    /// chunk ids in order is stored. Returns the `File` object id.
    pub fn put_file(&self, data: &[u8]) -> Result<ObjectId> {
        let mut chunk_ids = Vec::new();
        for ch in nous_core::chunk::chunks(data) {
            chunk_ids.push(self.put(ch, Some("application/nous-chunk".to_string()))?);
        }
        let file = Object::File(File {
            size: data.len() as u64,
            chunks: chunk_ids,
        });
        self.put_object(&file)
    }

    /// Reassemble a chunked [`File`] by concatenating its chunks in order.
    /// Each chunk is verified on read; the total length is checked against the
    /// recorded size (mismatch → [`Error::Corrupt`]).
    pub fn get_file(&self, id: &ObjectId) -> Result<Vec<u8>> {
        let file = match self.get_object(id)? {
            Object::File(f) => f,
            other => {
                return Err(Error::Other(format!(
                    "expected file, got {:?}",
                    other.kind()
                )))
            }
        };
        let mut out = Vec::with_capacity(file.size as usize);
        for cid in &file.chunks {
            out.extend_from_slice(&self.get(cid)?);
        }
        if out.len() as u64 != file.size {
            return Err(Error::Corrupt {
                expected: format!("{} bytes", file.size),
                actual: format!("{} bytes", out.len()),
            });
        }
        Ok(out)
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    fn object_path(&self, id: &ObjectId) -> PathBuf {
        let (a, b) = id.shard();
        self.root.join("objects").join(a).join(b).join(id.hex())
    }

    fn meta_path(&self, id: &ObjectId) -> PathBuf {
        self.root.join("meta").join(format!("{}.toml", id.hex()))
    }

    fn unique_tmp(&self, tag: &str) -> PathBuf {
        static CTR: AtomicU64 = AtomicU64::new(0);
        let n = CTR.fetch_add(1, Ordering::Relaxed);
        self.root
            .join("tmp")
            .join(format!("{tag}-{}-{n}", std::process::id()))
    }
}

// ---------------------------------------------------------------------------
// Timestamp helper
// ---------------------------------------------------------------------------

/// Return the current Unix timestamp in seconds.
///
/// Honours `SOURCE_DATE_EPOCH` for reproducible builds; falls back to the
/// system clock.
fn current_timestamp() -> i64 {
    if let Ok(val) = std::env::var("SOURCE_DATE_EPOCH") {
        if let Ok(secs) = val.trim().parse::<i64>() {
            return secs;
        }
    }
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp_root(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join(format!("nous-store-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn init_creates_skeleton() {
        let root = tmp_root("init");
        Store::init(&root).expect("init");

        assert!(root.join("config.toml").exists(), "config.toml missing");
        for sub in &["objects", "meta", "caps", "logs", "tmp"] {
            assert!(root.join(sub).is_dir(), "{sub} dir missing");
        }

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn init_is_idempotent() {
        let root = tmp_root("init-idem");
        Store::init(&root).expect("first init");
        Store::init(&root).expect("second init should not error");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn put_get_round_trip() {
        let root = tmp_root("put-get");
        let store = Store::init(&root).expect("init");

        let data = b"hello, nous store!";
        let id = store.put(data, None).expect("put");

        assert_eq!(id, ObjectId::of_bytes(data));

        let back = store.get(&id).expect("get");
        assert_eq!(back.as_slice(), data);

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn put_is_idempotent() {
        let root = tmp_root("put-idem");
        let store = Store::init(&root).expect("init");

        let data = b"idempotent data";
        let id1 = store.put(data, None).expect("first put");
        let id2 = store.put(data, None).expect("second put");
        assert_eq!(id1, id2);

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn get_meta_correct_size() {
        let root = tmp_root("meta-size");
        let store = Store::init(&root).expect("init");

        let data = b"size check data";
        let id = store.put(data, Some("text/plain".into())).expect("put");
        let meta = store.get_meta(&id).expect("get_meta");

        assert_eq!(meta.size, data.len() as u64);
        assert_eq!(meta.content_type.as_deref(), Some("text/plain"));

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn verify_good_object() {
        let root = tmp_root("verify-good");
        let store = Store::init(&root).expect("init");

        let id = store.put(b"verify me", None).expect("put");
        assert!(store.verify(&id).expect("verify"));

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn corrupt_object_detected() {
        let root = tmp_root("corrupt");
        let store = Store::init(&root).expect("init");

        let data = b"original content";
        let id = store.put(data, None).expect("put");

        // Overwrite the object file with garbage.
        let (a, b) = id.shard();
        let obj_path = root.join("objects").join(a).join(b).join(id.hex());
        fs::write(&obj_path, b"CORRUPTED DATA").expect("corrupt write");

        // get() must return Corrupt.
        match store.get(&id) {
            Err(Error::Corrupt { .. }) => {}
            other => panic!("expected Corrupt, got {other:?}"),
        }

        // verify() must return false.
        assert!(!store.verify(&id).expect("verify call"));

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn list_returns_put_ids() {
        let root = tmp_root("list");
        let store = Store::init(&root).expect("init");

        let id1 = store.put(b"object one", None).expect("put 1");
        let id2 = store.put(b"object two", None).expect("put 2");

        let mut listed = store.list().expect("list");
        listed.sort_by_key(|id| id.hex());

        let mut expected = vec![id1, id2];
        expected.sort_by_key(|id| id.hex());

        assert_eq!(listed, expected);

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn get_unknown_id_not_found() {
        let root = tmp_root("not-found");
        let store = Store::init(&root).expect("init");

        let id = ObjectId::of_bytes(b"this is never stored");
        match store.get(&id) {
            Err(Error::NotFound(_)) => {}
            other => panic!("expected NotFound, got {other:?}"),
        }

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn open_fails_without_config() {
        let root = tmp_root("open-fail");
        fs::create_dir_all(&root).ok();
        match Store::open(&root) {
            Err(Error::NotFound(_)) => {}
            other => panic!("expected NotFound, got {other:?}"),
        }
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn source_date_epoch_respected() {
        std::env::set_var("SOURCE_DATE_EPOCH", "1000000");
        let ts = current_timestamp();
        std::env::remove_var("SOURCE_DATE_EPOCH");
        assert_eq!(ts, 1_000_000);
    }

    // -----------------------------------------------------------------------
    // Typed objects (Phase 2)
    // -----------------------------------------------------------------------

    #[test]
    fn put_get_tree_round_trip() {
        use nous_core::{ObjectKind, Tree, TreeEntry};
        let root = tmp_root("tree");
        let store = Store::init(&root).unwrap();

        let blob_id = store.put(b"hello", None).unwrap();
        let tree = Tree::new(vec![TreeEntry {
            name: "hello.txt".into(),
            id: blob_id,
            kind: ObjectKind::Blob,
        }]);
        let obj = Object::Tree(tree.clone());
        let id = store.put_object(&obj).unwrap();

        assert_eq!(id, obj.id());
        assert_eq!(store.get_tree(&id).unwrap(), tree);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn put_get_file_round_trip_large() {
        let root = tmp_root("file-rt");
        let store = Store::init(&root).unwrap();
        let data: Vec<u8> = (0..500_000u32).map(|i| (i ^ (i >> 3)) as u8).collect();

        let id = store.put_file(&data).unwrap();
        let got = store.get_file(&id).unwrap();
        assert_eq!(got, data);
        // multi-chunk: the File object lists more than one chunk
        match store.get_object(&id).unwrap() {
            Object::File(f) => {
                assert_eq!(f.size, data.len() as u64);
                assert!(f.chunks.len() > 1, "large file should be multi-chunk");
            }
            _ => panic!("expected file"),
        }
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn put_file_is_deduplicating_and_idempotent() {
        let root = tmp_root("file-dedup");
        let store = Store::init(&root).unwrap();
        let data: Vec<u8> = (0..300_000u32).map(|i| (i * 7) as u8).collect();

        let id1 = store.put_file(&data).unwrap();
        let id2 = store.put_file(&data).unwrap();
        assert_eq!(id1, id2, "same content -> same File id");

        // All chunks are already present after the first put.
        if let Object::File(f) = store.get_object(&id1).unwrap() {
            for cid in &f.chunks {
                assert!(store.has(cid), "chunk should be stored");
            }
        }
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn empty_file_round_trip() {
        let root = tmp_root("file-empty");
        let store = Store::init(&root).unwrap();
        let id = store.put_file(b"").unwrap();
        assert_eq!(store.get_file(&id).unwrap(), Vec::<u8>::new());
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn get_tree_on_wrong_kind_errors() {
        use nous_core::Commit;
        let root = tmp_root("wrongkind");
        let store = Store::init(&root).unwrap();
        let commit = Object::Commit(Commit {
            tree: store.put(b"x", None).unwrap(),
            parents: vec![],
            author: "a".into(),
            message: "m".into(),
            timestamp: 1,
        });
        let id = store.put_object(&commit).unwrap();
        assert!(store.get_tree(&id).is_err());
        assert!(store.get_commit(&id).is_ok());
        fs::remove_dir_all(&root).ok();
    }
}
