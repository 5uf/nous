//! `nous-caps` — capability tokens for the Nous workspace.
//!
//! A [`Capability`] is an attenuable, self-describing bearer token.  v0 tokens
//! are **unsigned** (`alg = "none"`) but the `alg` field provides algorithm
//! agility so that Ed25519 or ML-DSA signatures can be added later without a
//! format break.
//!
//! Wire format: `base64url_nopad(json(Capability))`.

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use nous_core::{ObjectId, Right};

// ---------------------------------------------------------------------------
// Capability
// ---------------------------------------------------------------------------

/// A transferable, attenuable capability token.
///
/// Tokens are serialised as `base64url_nopad(JSON)` so they are opaque to
/// callers but self-describing to any party that decodes them.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Capability {
    /// Unique identifier for this capability (random hex).
    pub cap_id: String,
    /// Issuer identity — `"local"` in v0; a public key fingerprint later.
    pub issuer: String,
    /// Holder identity — `None` makes this a bearer token.
    pub holder: Option<String>,
    /// The resource this capability grants access to (ObjectId Display string).
    pub resource: String,
    /// Set of rights granted.
    pub rights: Vec<Right>,
    /// Expiry as Unix seconds; `0` means the token never expires.
    pub expiry: i64,
    /// Arbitrary key/value constraints (e.g. `max_bytes`, `ip_range`).
    pub constraints: BTreeMap<String, String>,
    /// Algorithm agility header — `"none"` in v0.
    pub alg: String,
    /// Detached signature bytes (base64url); `None` in v0.
    pub signature: Option<String>,
}

// ---------------------------------------------------------------------------
// Entropy helper
// ---------------------------------------------------------------------------

/// Generate a 16-byte pseudo-random hex string without an external RNG crate.
///
/// Combines `SystemTime` nanoseconds and the OS process-id, then hashes with
/// a simple mix to spread bits.  Suitable for v0 unsigned local tokens; not
/// cryptographically strong on its own.
fn random_hex_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    let pid = std::process::id() as u64;

    // Simple mixing: combine fields and run a few rounds of xorshift.
    let mut x = nanos ^ (pid << 32) ^ (counter.wrapping_mul(0x9e37_79b9_7f4a_7c15));
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^= x >> 31;

    // Use two 64-bit values for 16 bytes of entropy.
    let y = x
        .wrapping_add(counter)
        .wrapping_mul(0x517c_c1b7_2722_0a95);

    format!("{x:016x}{y:016x}")
}

// ---------------------------------------------------------------------------
// Unix-seconds "now" helper (respects SOURCE_DATE_EPOCH for reproducibility)
// ---------------------------------------------------------------------------

fn now_unix_secs() -> i64 {
    if let Ok(val) = std::env::var("SOURCE_DATE_EPOCH") {
        if let Ok(secs) = val.trim().parse::<i64>() {
            return secs;
        }
    }
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

// ---------------------------------------------------------------------------
// impl Capability
// ---------------------------------------------------------------------------

impl Capability {
    /// Create a read-only capability for `resource`.
    ///
    /// If `ttl_secs > 0` the token expires `ttl_secs` seconds from now.
    /// `ttl_secs <= 0` produces a non-expiring token (`expiry = 0`).
    pub fn new_read(resource: &ObjectId, ttl_secs: i64) -> Capability {
        Self::grant(resource, vec![Right::Read], ttl_secs)
    }

    /// Create a capability granting `rights` on `resource`.
    ///
    /// If `ttl_secs > 0` the token expires `ttl_secs` seconds from now.
    /// `ttl_secs <= 0` produces a non-expiring token (`expiry = 0`).
    pub fn grant(resource: &ObjectId, rights: Vec<Right>, ttl_secs: i64) -> Capability {
        let expiry = if ttl_secs > 0 {
            now_unix_secs().saturating_add(ttl_secs)
        } else {
            0
        };

        Capability {
            cap_id: random_hex_id(),
            issuer: "local".to_string(),
            holder: None,
            resource: resource.to_string(),
            rights,
            expiry,
            constraints: BTreeMap::new(),
            alg: "none".to_string(),
            signature: None,
        }
    }

    /// Encode this capability as a bearer token.
    ///
    /// Format: `base64url_nopad(utf8(json(self)))` — opaque to callers,
    /// self-describing to any party that decodes it.
    pub fn encode(&self) -> String {
        let json = serde_json::to_string(self).expect("Capability serialization is infallible");
        URL_SAFE_NO_PAD.encode(json.as_bytes())
    }

    /// Decode a bearer token produced by [`Capability::encode`].
    ///
    /// Returns [`nous_core::Error::Cap`] on bad base64 or invalid JSON.
    pub fn decode(token: &str) -> nous_core::Result<Capability> {
        let bytes = URL_SAFE_NO_PAD
            .decode(token)
            .map_err(|e| nous_core::Error::Cap(format!("base64 decode failed: {e}")))?;

        let cap: Capability = serde_json::from_slice(&bytes)
            .map_err(|e| nous_core::Error::Cap(format!("json decode failed: {e}")))?;

        Ok(cap)
    }

    /// Return `true` if the token is not expired at `now` (Unix seconds).
    ///
    /// A token with `expiry == 0` never expires.
    pub fn is_valid(&self, now: i64) -> bool {
        self.expiry == 0 || now < self.expiry
    }

    /// Return `true` iff the token is valid at `now`, grants `right`, and
    /// the resource matches `resource`'s Display representation.
    pub fn allows(&self, right: Right, resource: &ObjectId, now: i64) -> bool {
        self.is_valid(now)
            && self.resource == resource.to_string()
            && self.rights.contains(&right)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// A fixed ObjectId for tests — hash of the byte string `"test-resource"`.
    fn test_resource() -> ObjectId {
        ObjectId::of_bytes(b"test-resource")
    }

    /// A different ObjectId.
    fn other_resource() -> ObjectId {
        ObjectId::of_bytes(b"other-resource")
    }

    // -----------------------------------------------------------------------
    // new_read
    // -----------------------------------------------------------------------

    #[test]
    fn new_read_grants_read_right() {
        let res = test_resource();
        let cap = Capability::new_read(&res, 3600);
        assert_eq!(cap.rights, vec![Right::Read]);
        assert_eq!(cap.resource, res.to_string());
        assert_eq!(cap.issuer, "local");
        assert_eq!(cap.alg, "none");
        assert!(cap.signature.is_none());
    }

    #[test]
    fn new_read_no_expiry_when_ttl_zero() {
        let cap = Capability::new_read(&test_resource(), 0);
        assert_eq!(cap.expiry, 0);
    }

    #[test]
    fn new_read_no_expiry_when_ttl_negative() {
        let cap = Capability::new_read(&test_resource(), -1);
        assert_eq!(cap.expiry, 0);
    }

    // -----------------------------------------------------------------------
    // encode / decode round-trip
    // -----------------------------------------------------------------------

    #[test]
    fn encode_decode_round_trip() {
        let res = test_resource();
        let original = Capability::grant(&res, vec![Right::Read, Right::Write], 60);
        let token = original.encode();
        let decoded = Capability::decode(&token).expect("decode must succeed");

        assert_eq!(decoded.cap_id,    original.cap_id);
        assert_eq!(decoded.issuer,    original.issuer);
        assert_eq!(decoded.holder,    original.holder);
        assert_eq!(decoded.resource,  original.resource);
        assert_eq!(decoded.rights,    original.rights);
        assert_eq!(decoded.expiry,    original.expiry);
        assert_eq!(decoded.alg,       original.alg);
        assert_eq!(decoded.signature, original.signature);
    }

    // -----------------------------------------------------------------------
    // is_valid
    // -----------------------------------------------------------------------

    #[test]
    fn is_valid_future_expiry_is_true() {
        let mut cap = Capability::new_read(&test_resource(), 3600);
        cap.expiry = 9_999_999_999; // far future
        assert!(cap.is_valid(1_000_000_000));
    }

    #[test]
    fn is_valid_past_expiry_is_false() {
        let mut cap = Capability::new_read(&test_resource(), 3600);
        cap.expiry = 1_000; // already expired
        assert!(!cap.is_valid(2_000));
    }

    #[test]
    fn is_valid_zero_expiry_always_true() {
        let mut cap = Capability::new_read(&test_resource(), 0);
        cap.expiry = 0;
        // Valid at any point in time.
        assert!(cap.is_valid(0));
        assert!(cap.is_valid(i64::MAX));
    }

    // -----------------------------------------------------------------------
    // allows
    // -----------------------------------------------------------------------

    #[test]
    fn allows_read_on_correct_resource() {
        let res = test_resource();
        let cap = Capability::new_read(&res, 3600);
        // Use a "now" well before the expiry.
        assert!(cap.allows(Right::Read, &res, 0));
    }

    #[test]
    fn allows_denies_write_on_read_cap() {
        let res = test_resource();
        let cap = Capability::new_read(&res, 3600);
        assert!(!cap.allows(Right::Write, &res, 0));
    }

    #[test]
    fn allows_denies_different_resource() {
        let res = test_resource();
        let cap = Capability::new_read(&res, 3600);
        let other = other_resource();
        assert!(!cap.allows(Right::Read, &other, 0));
    }

    #[test]
    fn allows_denies_when_expired() {
        let res = test_resource();
        let mut cap = Capability::new_read(&res, 3600);
        cap.expiry = 500; // expired
        assert!(!cap.allows(Right::Read, &res, 1000));
    }

    // -----------------------------------------------------------------------
    // decode garbage
    // -----------------------------------------------------------------------

    #[test]
    fn decode_garbage_returns_cap_error() {
        let err = Capability::decode("not-valid-base64!!!");
        assert!(
            matches!(err, Err(nous_core::Error::Cap(_))),
            "expected Error::Cap, got {err:?}"
        );
    }

    #[test]
    fn decode_valid_base64_but_bad_json_returns_cap_error() {
        // base64url of `{broken json`
        let token = URL_SAFE_NO_PAD.encode(b"{broken json");
        let err = Capability::decode(&token);
        assert!(
            matches!(err, Err(nous_core::Error::Cap(_))),
            "expected Error::Cap, got {err:?}"
        );
    }
}
