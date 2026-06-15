//! `nous` — command-line interface for the Nous content-addressed store.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process;
use std::str::FromStr;

use clap::{Parser, Subcommand};

use nous_caps::Capability;
use nous_core::{ObjectId, Result};
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

        Command::Put { file } => {
            let path = nous_dir()?;
            let store = Store::open(&path)?;
            let id = store.put_path(&file)?;
            println!("{id}");
        }

        Command::Get { cid, out } => {
            let id = ObjectId::from_str(&cid)?;
            let path = nous_dir()?;
            let store = Store::open(&path)?;
            let bytes = store.get(&id)?;
            std::fs::write(&out, &bytes)
                .map_err(|e| nous_core::Error::Io(e))?;
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
            nous_http::serve(store, addr, enforce_caps)?;
        }

        Command::Grant { kind } => match kind {
            GrantKind::Read { cid, ttl } => {
                let id = ObjectId::from_str(&cid)?;
                let ttl_secs = parse_ttl(&ttl)?;
                let cap = Capability::new_read(&id, ttl_secs);
                println!("{}", cap.encode());
            }
        },
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
