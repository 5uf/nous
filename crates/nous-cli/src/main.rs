//! `nous` — command-line interface for the Nous content-addressed store.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Parser, Subcommand};

use nous_caps::{Capability, IssuerKey};
use nous_core::{Error, ObjectId, Result};
use nous_store::Store;

// ---------------------------------------------------------------------------
// TTL parser
// ---------------------------------------------------------------------------

/// Parse a human-readable duration string into a number of seconds.
///
/// Accepted formats:
/// - Bare integer: treated as seconds (e.g. `"45"` → 45)
/// - Integer + suffix: `s` (seconds), `m` (minutes), `h` (hours), `d` (days)
///
/// Returns an error for unknown suffixes or non-integer input.
pub(crate) fn parse_ttl(s: &str) -> Result<i64> {
    let s = s.trim();

    // Try bare integer first.
    if let Ok(n) = s.parse::<i64>() {
        return Ok(n);
    }

    // Split at the last non-digit character.
    let split_pos = s
        .find(|c: char| !c.is_ascii_digit())
        .ok_or_else(|| nous_core::Error::Other(format!("invalid ttl {s:?}: no suffix found")))?;

    let (num_str, suffix) = s.split_at(split_pos);
    let n: i64 = num_str.parse().map_err(|_| {
        nous_core::Error::Other(format!("invalid ttl {s:?}: not a valid integer"))
    })?;

    let multiplier: i64 = match suffix {
        "s" => 1,
        "m" => 60,
        "h" => 3600,
        "d" => 86400,
        other => {
            return Err(nous_core::Error::Other(format!(
                "invalid ttl {s:?}: unknown suffix {other:?} (use s, m, h, or d)"
            )))
        }
    };

    Ok(n * multiplier)
}

// ---------------------------------------------------------------------------
// CLI structure (clap derive)
// ---------------------------------------------------------------------------

/// Nous — content-addressed object store.
#[derive(Parser)]
#[command(name = "nous", version, about = "Content-addressed object store")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialise a new store in the current directory.
    Init,

    /// Store a file and print its content ID.
    Put {
        /// Path to the file to store.
        file: PathBuf,
        /// Split into content-defined chunks (dedup large files); stores a
        /// File object referencing the chunks.
        #[arg(long)]
        chunked: bool,
    },

    /// Retrieve a stored object and write it to a file.
    Get {
        /// Content ID of the object to retrieve.
        cid: String,
        /// Path to write the retrieved bytes to.
        #[arg(long)]
        out: PathBuf,
    },

    /// List all stored content IDs.
    Ls,

    /// Inspect the metadata of a stored object.
    Inspect {
        /// Content ID of the object to inspect.
        cid: String,
    },

    /// Verify the integrity of a stored object.
    Verify {
        /// Content ID of the object to verify.
        cid: String,
    },

    /// Serve the store over HTTP.
    Serve {
        /// TCP port to listen on (binds to 127.0.0.1).
        #[arg(long = "http")]
        port: u16,
        /// Require a valid capability token on every request.
        #[arg(long)]
        enforce_caps: bool,
    },

    /// Grant a capability token for an object.
    Grant {
        #[command(subcommand)]
        kind: GrantKind,
    },

    /// Snapshot a directory into a content-addressed commit.
    Snapshot {
        /// Directory to snapshot.
        dir: PathBuf,
        /// Commit message.
        #[arg(short, long)]
        message: String,
        /// Optional parent commit id.
        #[arg(long)]
        parent: Option<String>,
        /// Author name (default: "nous").
        #[arg(long, default_value = "nous")]
        author: String,
    },

    /// Restore a commit's tree to a directory.
    Restore {
        /// Commit id to restore.
        commit: String,
        /// Destination directory.
        #[arg(long)]
        out: PathBuf,
    },

    /// Show commit history (first-parent chain).
    Log {
        /// Commit id to start from.
        commit: String,
    },

    /// Wrap a content root in a named, versioned package (Manifest).
    Package {
        /// Root content id (usually a tree or commit).
        root: String,
        /// Package name.
        #[arg(long)]
        name: String,
        /// Package version.
        #[arg(long)]
        version: String,
    },

    /// Export the object DAG reachable from a root into a portable archive.
    Export {
        /// Root content id.
        root: String,
        /// Output archive path.
        #[arg(long)]
        out: PathBuf,
    },

    /// Import a portable archive into the store (verifies every object).
    Import {
        /// Archive path.
        file: PathBuf,
    },

    /// Generate an Ed25519 issuer key for signing capabilities.
    Keygen,

    /// Verify and inspect a capability token.
    VerifyCap {
        /// The bearer token to verify.
        token: String,
    },
}

#[derive(Subcommand)]
enum GrantKind {
    /// Grant read access to an object.
    Read {
        /// Content ID of the object to grant access to.
        cid: String,
        /// Duration the token is valid for (e.g. 10m, 1h, 30s, 2d).
        #[arg(long)]
        ttl: String,
        /// Sign the token with the store's issuer key (run `nous keygen` first).
        #[arg(long)]
        sign: bool,
    },
    /// Grant write access to the store (POST /object).
    Write {
        /// Duration the token is valid for (e.g. 10m, 1h, 30s, 2d).
        #[arg(long)]
        ttl: String,
        /// Sign the token with the store's issuer key (run `nous keygen` first).
        #[arg(long)]
        sign: bool,
    },
}

// ---------------------------------------------------------------------------
// Store path helper
// ---------------------------------------------------------------------------

fn nous_dir() -> nous_core::Result<PathBuf> {
    let cwd = std::env::current_dir()
        .map_err(|e| nous_core::Error::Io(e))?;
    Ok(cwd.join(".nous"))
}

fn issuer_key_path() -> Result<PathBuf> {
    Ok(nous_dir()?.join("keys").join("issuer.key"))
}

/// Write secret bytes to `path` with owner-only permissions (0600 on unix).
fn write_secret(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(Error::Io)?;
    }
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .map_err(Error::Io)?;
        f.write_all(bytes).map_err(Error::Io)?;
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, bytes).map_err(Error::Io)?;
    }
    Ok(())
}

/// Load the issuer key seed (32 bytes) from the store.
fn load_issuer_key() -> Result<IssuerKey> {
    let p = issuer_key_path()?;
    let bytes = std::fs::read(&p).map_err(|_| {
        Error::Cap(format!(
            "no issuer key at {} — run `nous keygen` first",
            p.display()
        ))
    })?;
    let seed: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| Error::Cap("issuer key file is not 32 bytes".into()))?;
    Ok(IssuerKey::from_seed(&seed))
}

/// Current Unix seconds from the real clock.
///
/// Does NOT honour `SOURCE_DATE_EPOCH`: this drives capability expiry checks,
/// a security boundary that must not be influenced by environment variables.
fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

// ---------------------------------------------------------------------------
// Command implementations
// ---------------------------------------------------------------------------

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init => {
            let path = nous_dir()?;
            Store::init(&path)?;
            let abs = path
                .canonicalize()
                .map_err(|e| nous_core::Error::Io(e))?;
            println!("initialized {}", abs.display());
        }

        Command::Put { file, chunked } => {
            let path = nous_dir()?;
            let store = Store::open(&path)?;
            let id = if chunked {
                let data = std::fs::read(&file).map_err(Error::Io)?;
                store.put_file(&data)?
            } else {
                store.put_path(&file)?
            };
            println!("{id}");
        }

        Command::Get { cid, out } => {
            let id = ObjectId::from_str(&cid)?;
            let path = nous_dir()?;
            let store = Store::open(&path)?;
            // Transparently reassemble chunked File objects; otherwise read
            // the raw blob.
            let is_file = matches!(
                store.get_meta(&id).ok().and_then(|m| m.content_type),
                Some(ref ct) if ct == "application/nous-file"
            );
            let bytes = if is_file {
                store.get_file(&id)?
            } else {
                store.get(&id)?
            };
            std::fs::write(&out, &bytes).map_err(|e| nous_core::Error::Io(e))?;
            println!("wrote {}", out.display());
        }

        Command::Ls => {
            let path = nous_dir()?;
            let store = Store::open(&path)?;
            let ids = store.list()?;
            for id in ids {
                println!("{id}");
            }
        }

        Command::Inspect { cid } => {
            let id = ObjectId::from_str(&cid)?;
            let path = nous_dir()?;
            let store = Store::open(&path)?;
            let meta = store.get_meta(&id)?;
            println!("id:           {}", meta.id);
            println!("algo:         {}", meta.algo);
            println!("size:         {} bytes", meta.size);
            println!("created:      {}", meta.created);
            println!(
                "content_type: {}",
                meta.content_type.as_deref().unwrap_or("(none)")
            );
        }

        Command::Verify { cid } => {
            let id = ObjectId::from_str(&cid)?;
            let path = nous_dir()?;
            let store = Store::open(&path)?;
            if store.verify(&id)? {
                println!("ok");
            } else {
                eprintln!("FAILED");
                process::exit(1);
            }
        }

        Command::Serve { port, enforce_caps } => {
            let path = nous_dir()?;
            let store = Store::open(&path)?;
            let addr: SocketAddr = format!("127.0.0.1:{port}").parse().map_err(|e| {
                nous_core::Error::Other(format!("invalid address: {e}"))
            })?;
            // When enforcing, trust only the store's own issuer key. Fail
            // closed if no key exists (otherwise enforcement is meaningless).
            let policy = if enforce_caps {
                let key = load_issuer_key().map_err(|_| {
                    Error::Cap(
                        "`--enforce-caps` requires an issuer key; run `nous keygen` first"
                            .to_string(),
                    )
                })?;
                nous_http::CapPolicy {
                    enforce: true,
                    trusted_issuers: vec![key.public_b64()],
                }
            } else {
                nous_http::CapPolicy::default()
            };
            nous_http::serve(store, addr, policy)?;
        }

        Command::Grant { kind } => match kind {
            GrantKind::Read { cid, ttl, sign } => {
                let id = ObjectId::from_str(&cid)?;
                let ttl_secs = parse_ttl(&ttl)?;
                let mut cap = Capability::new_read(&id, ttl_secs);
                if sign {
                    let key = load_issuer_key()?;
                    cap.sign(&key);
                }
                println!("{}", cap.encode());
            }
            GrantKind::Write { ttl, sign } => {
                let ttl_secs = parse_ttl(&ttl)?;
                let resource = nous_caps::store_write_resource();
                let mut cap =
                    Capability::grant(&resource, vec![nous_core::Right::Write], ttl_secs);
                if sign {
                    let key = load_issuer_key()?;
                    cap.sign(&key);
                }
                println!("{}", cap.encode());
            }
        },

        Command::Snapshot { dir, message, parent, author } => {
            let path = nous_dir()?;
            let store = Store::open(&path)?;
            let parent_id = match parent {
                Some(p) => Some(ObjectId::from_str(&p)?),
                None => None,
            };
            let commit =
                nous_bridge::snapshot(&store, &dir, &author, &message, parent_id, now_secs())?;
            println!("{commit}");
        }

        Command::Restore { commit, out } => {
            let id = ObjectId::from_str(&commit)?;
            let path = nous_dir()?;
            let store = Store::open(&path)?;
            nous_bridge::restore(&store, &id, &out)?;
            println!("restored {} -> {}", commit, out.display());
        }

        Command::Log { commit } => {
            let id = ObjectId::from_str(&commit)?;
            let path = nous_dir()?;
            let store = Store::open(&path)?;
            for (cid, c) in nous_bridge::log(&store, &id)? {
                println!("commit {cid}");
                println!("    author:  {}", c.author);
                println!("    date:    {}", c.timestamp);
                println!("    tree:    {}", c.tree);
                println!("    message: {}", c.message);
                println!();
            }
        }

        Command::Package { root, name, version } => {
            let id = ObjectId::from_str(&root)?;
            let path = nous_dir()?;
            let store = Store::open(&path)?;
            let manifest =
                nous_build::package(&store, id, &name, &version, std::collections::BTreeMap::new())?;
            println!("{manifest}");
        }

        Command::Export { root, out } => {
            let id = ObjectId::from_str(&root)?;
            let path = nous_dir()?;
            let store = Store::open(&path)?;
            let archive = nous_build::export(&store, id)?;
            std::fs::write(&out, &archive).map_err(Error::Io)?;
            println!("exported {} bytes -> {}", archive.len(), out.display());
        }

        Command::Import { file } => {
            let path = nous_dir()?;
            let store = Store::open(&path)?;
            let bytes = std::fs::read(&file).map_err(Error::Io)?;
            let n = nous_build::import(&store, &bytes)?;
            println!("imported {n} objects");
        }

        Command::Keygen => {
            let path = issuer_key_path()?;
            if path.exists() {
                return Err(Error::Cap(format!(
                    "issuer key already exists at {} — refusing to overwrite",
                    path.display()
                )));
            }
            let key = IssuerKey::generate()?;
            let seed = zeroize::Zeroizing::new(key.to_seed_bytes());
            write_secret(&path, seed.as_ref())?;
            println!("issuer key created: {}", path.display());
            println!("public key:         {}", key.public_b64());
        }

        Command::VerifyCap { token } => {
            let cap = Capability::decode(&token)?;
            let now = now_secs();
            let signed = cap.alg != "none";
            let sig_ok = cap.verify_signature();
            let unexpired = cap.is_valid(now);

            println!("cap_id:    {}", cap.cap_id);
            println!("issuer:    {}", cap.issuer);
            println!("resource:  {}", cap.resource);
            println!("rights:    {:?}", cap.rights);
            println!("alg:       {}", cap.alg);
            println!("expiry:    {}", cap.expiry);
            println!(
                "signature: {}",
                if !signed {
                    "(unsigned)"
                } else if sig_ok {
                    "valid"
                } else {
                    "INVALID"
                }
            );
            println!("expired:   {}", if unexpired { "no" } else { "yes" });

            // Non-zero exit if a signed token fails its signature, or any token
            // is expired.
            if (signed && !sig_ok) || !unexpired {
                process::exit(1);
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::parse_ttl;

    #[test]
    fn ttl_seconds_suffix() {
        assert_eq!(parse_ttl("30s").unwrap(), 30);
    }

    #[test]
    fn ttl_minutes_suffix() {
        assert_eq!(parse_ttl("10m").unwrap(), 600);
    }

    #[test]
    fn ttl_hours_suffix() {
        assert_eq!(parse_ttl("1h").unwrap(), 3600);
    }

    #[test]
    fn ttl_days_suffix() {
        assert_eq!(parse_ttl("2d").unwrap(), 172800);
    }

    #[test]
    fn ttl_bare_integer() {
        assert_eq!(parse_ttl("45").unwrap(), 45);
    }

    #[test]
    fn ttl_bad_suffix_errors() {
        let err = parse_ttl("10x");
        assert!(err.is_err(), "unknown suffix must produce an error");
        let msg = err.unwrap_err().to_string();
        assert!(
            msg.contains("unknown suffix"),
            "error message should mention 'unknown suffix', got: {msg}"
        );
    }

    #[test]
    fn ttl_empty_string_errors() {
        assert!(parse_ttl("").is_err());
    }

    #[test]
    fn ttl_non_integer_errors() {
        assert!(parse_ttl("abch").is_err());
    }
}
