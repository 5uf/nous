use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{Error, Result};

// ---------------------------------------------------------------------------
// HashAlgo
// ---------------------------------------------------------------------------

/// The hashing algorithm used to produce an [`ObjectId`].
///
/// Carrying the algorithm tag in the ID itself means the on-disk format
/// remains compatible when a new algorithm is added later (hash agility).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum HashAlgo {
    Blake3,
}

impl HashAlgo {
    /// Multihash function code for the algorithm.
    ///
    /// `0x1e` is the registered code for BLAKE3 in the multihash table.
    pub fn code(&self) -> u8 {
        match self {
            HashAlgo::Blake3 => 0x1e,
        }
    }

    /// Reconstruct an algorithm from its multihash code.
    pub fn from_code(c: u8) -> Option<HashAlgo> {
        match c {
            0x1e => Some(HashAlgo::Blake3),
            _ => None,
        }
    }

    /// Short human-readable tag used in serialised IDs.
    pub fn name(&self) -> &'static str {
        match self {
            HashAlgo::Blake3 => "b3",
        }
    }
}

// ---------------------------------------------------------------------------
// ObjectId
// ---------------------------------------------------------------------------

/// A content-addressed identifier for any object stored by Nous.
///
/// The 32-byte digest is always BLAKE3 in this release, but the `algo` field
/// ensures that future algorithms can be introduced without breaking existing
/// stored data.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ObjectId {
    pub algo: HashAlgo,
    pub digest: [u8; 32],
}

impl ObjectId {
    /// Hash `data` with BLAKE3 and return the resulting [`ObjectId`].
    pub fn of_bytes(data: &[u8]) -> ObjectId {
        let digest = *blake3::hash(data).as_bytes();
        ObjectId { algo: HashAlgo::Blake3, digest }
    }

    /// Return the 64-character lowercase hex representation of the digest.
    pub fn hex(&self) -> String {
        self.digest.iter().fold(String::with_capacity(64), |mut s, b| {
            use std::fmt::Write;
            write!(s, "{b:02x}").unwrap();
            s
        })
    }

    /// Return a two-level shard key derived from the first two digest bytes.
    ///
    /// Example: digest starting `abcd…` → `("ab", "cd")`.
    /// Used by `nous-store` to distribute objects across subdirectories and
    /// avoid filesystem directory-entry limits.
    pub fn shard(&self) -> (String, String) {
        let first  = format!("{:02x}", self.digest[0]);
        let second = format!("{:02x}", self.digest[1]);
        (first, second)
    }
}

// ---------------------------------------------------------------------------
// Display / FromStr
// ---------------------------------------------------------------------------

impl fmt::Display for ObjectId {
    /// Serialises to `"b3:<64hex>"`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.algo.name(), self.hex())
    }
}

impl FromStr for ObjectId {
    type Err = Error;

    /// Parses `"b3:<64hex>"`.  Returns [`Error::InvalidId`] for any
    /// malformed input.
    fn from_str(s: &str) -> Result<Self> {
        let (prefix, hex) = s
            .split_once(':')
            .ok_or_else(|| Error::InvalidId(format!("missing ':' in {s:?}")))?;

        let algo = match prefix {
            "b3" => HashAlgo::Blake3,
            other => {
                return Err(Error::InvalidId(format!("unknown algorithm prefix {other:?}")))
            }
        };

        if hex.len() != 64 {
            return Err(Error::InvalidId(format!(
                "expected 64 hex chars, got {} in {s:?}",
                hex.len()
            )));
        }

        let mut digest = [0u8; 32];
        for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
            let hi = hex_nibble(chunk[0], s)?;
            let lo = hex_nibble(chunk[1], s)?;
            digest[i] = (hi << 4) | lo;
        }

        Ok(ObjectId { algo, digest })
    }
}

/// Decode a single ASCII hex digit.
fn hex_nibble(b: u8, src: &str) -> Result<u8> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(Error::InvalidId(format!("non-hex character in {src:?}"))),
    }
}

// ---------------------------------------------------------------------------
// Serde — serialize as the Display string, deserialize via FromStr
// ---------------------------------------------------------------------------

impl Serialize for ObjectId {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ObjectId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        raw.parse::<ObjectId>().map_err(serde::de::Error::custom)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// BLAKE3 hash of the empty byte slice, precomputed for cross-checking.
    const EMPTY_B3_HEX: &str =
        "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262";

    #[test]
    fn of_bytes_deterministic() {
        let a = ObjectId::of_bytes(b"hello nous");
        let b = ObjectId::of_bytes(b"hello nous");
        assert_eq!(a, b);
    }

    #[test]
    fn of_bytes_empty_known_hash() {
        let id = ObjectId::of_bytes(b"");
        assert_eq!(id.hex(), EMPTY_B3_HEX);
    }

    #[test]
    fn display_round_trip() {
        let id = ObjectId::of_bytes(b"round-trip test");
        let s = id.to_string();
        assert!(s.starts_with("b3:"), "display must start with 'b3:'");
        assert_eq!(s.len(), 3 + 64, "display must be 'b3:' + 64 hex chars");
        let parsed: ObjectId = s.parse().expect("round-trip parse failed");
        assert_eq!(id, parsed);
    }

    #[test]
    fn from_str_rejects_wrong_prefix() {
        let err = "sha256:af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
            .parse::<ObjectId>();
        assert!(err.is_err(), "wrong prefix must be rejected");
    }

    #[test]
    fn from_str_rejects_short_hex() {
        let err = "b3:deadbeef".parse::<ObjectId>();
        assert!(err.is_err(), "short hex must be rejected");
    }

    #[test]
    fn from_str_rejects_non_hex() {
        // 64 chars but contains 'z'
        let bad = format!("b3:{}z{}", "a".repeat(32), "b".repeat(31));
        assert!(bad.parse::<ObjectId>().is_err());
    }

    #[test]
    fn from_str_rejects_missing_colon() {
        assert!("b3af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
            .parse::<ObjectId>()
            .is_err());
    }

    #[test]
    fn shard_returns_first_two_bytes() {
        let id = ObjectId::of_bytes(b"");
        // EMPTY_B3_HEX starts with "af13..."
        let (a, b) = id.shard();
        assert_eq!(a, "af");
        assert_eq!(b, "13");
    }

    #[test]
    fn serde_json_round_trip() {
        let id = ObjectId::of_bytes(b"serde test");
        let json = serde_json::to_string(&id).expect("serialize");
        let back: ObjectId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(id, back);
    }
}
