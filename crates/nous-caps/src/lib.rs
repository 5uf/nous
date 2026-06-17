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
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use nous_core::{Error, ObjectId, Right};

// ---------------------------------------------------------------------------
// Capability
// ---------------------------------------------------------------------------

/// A transferable, attenuable capability token.
///
/// Tokens are serialised as `base64url_nopad(JSON)` so they are opaque to
/// callers but self-describing to any party that decodes them.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
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

/// Generate a 16-byte cryptographically-random hex id from the OS CSPRNG.
fn random_hex_id() -> String {
    let mut buf = [0u8; 16];
    getrandom::getrandom(&mut buf).expect("OS CSPRNG failure");
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

// ---------------------------------------------------------------------------
// Unix-seconds "now" helper
// ---------------------------------------------------------------------------

/// Current Unix seconds from the real clock only.
///
/// Capability expiry is a security boundary, so this intentionally does NOT
/// honour `SOURCE_DATE_EPOCH` (which an attacker could set to rewind time).
fn now_unix_secs() -> i64 {
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
// Well-known resources
// ---------------------------------------------------------------------------

/// The sentinel resource a write capability must grant to permit writes to the
/// store via the HTTP gateway.
///
/// Writes are not tied to a specific object id (the id is only known *after*
/// storing the bytes), so write authorization is checked against this constant
/// resource instead of a content id.
pub fn store_write_resource() -> ObjectId {
    ObjectId::of_bytes(b"nous:store:write")
}

// ---------------------------------------------------------------------------
// Ed25519 issuer keys + signing
// ---------------------------------------------------------------------------

/// An Ed25519 issuer key used to sign capabilities.
///
/// The 32-byte seed *is* the secret; keep it confidential. The public key is
/// embedded (base64url) in a signed capability's `issuer` field.
pub struct IssuerKey {
    signing: SigningKey,
}

impl IssuerKey {
    /// Generate a fresh key from the OS CSPRNG.
    pub fn generate() -> nous_core::Result<IssuerKey> {
        let mut seed = [0u8; 32];
        getrandom::getrandom(&mut seed)
            .map_err(|e| Error::Cap(format!("rng failure: {e}")))?;
        let key = IssuerKey { signing: SigningKey::from_bytes(&seed) };
        seed.fill(0); // best-effort zeroing of the local copy
        Ok(key)
    }

    /// Reconstruct from a 32-byte secret seed.
    pub fn from_seed(seed: &[u8; 32]) -> IssuerKey {
        IssuerKey { signing: SigningKey::from_bytes(seed) }
    }

    /// The 32-byte secret seed (for persistence). Handle as a secret.
    pub fn to_seed_bytes(&self) -> [u8; 32] {
        self.signing.to_bytes()
    }

    /// Public verifying key, base64url-nopad encoded.
    pub fn public_b64(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.signing.verifying_key().to_bytes())
    }
}

impl Capability {
    /// Canonical bytes signed over: this capability with `signature` cleared.
    /// `alg` and `issuer` ARE included so the algorithm and key are bound to
    /// the signature and cannot be swapped after the fact.
    fn signing_payload(&self) -> Vec<u8> {
        let mut bare = self.clone();
        bare.signature = None;
        serde_json::to_vec(&bare).expect("Capability serialization is infallible")
    }

    /// Sign this capability with `key`. Sets `alg = "ed25519"`, `issuer` to the
    /// signer's public key (base64url), and `signature` to the detached
    /// Ed25519 signature over [`signing_payload`].
    pub fn sign(&mut self, key: &IssuerKey) {
        self.alg = "ed25519".to_string();
        self.issuer = key.public_b64();
        self.signature = None;
        let payload = self.signing_payload();
        let sig = key.signing.sign(&payload);
        self.signature = Some(URL_SAFE_NO_PAD.encode(sig.to_bytes()));
    }

    /// Verify the signature is internally consistent: `alg == "ed25519"`, the
    /// `signature` is a valid Ed25519 signature over the payload by the key in
    /// `issuer`.
    ///
    /// NOTE (v0 trust limitation): this only proves the cap was signed by
    /// whoever owns the key named in `issuer`. It does NOT establish that the
    /// issuer is trusted. Callers enforcing authorization MUST additionally
    /// check `issuer` against a trusted-issuer allowlist (see
    /// [`Capability::verify_from`]).
    pub fn verify_signature(&self) -> bool {
        if self.alg != "ed25519" {
            return false;
        }
        let Some(sig_b64) = self.signature.as_deref() else { return false };

        let Ok(pk_bytes) = URL_SAFE_NO_PAD.decode(self.issuer.as_bytes()) else { return false };
        let Ok(pk_arr): std::result::Result<[u8; 32], _> = pk_bytes.try_into() else { return false };
        let Ok(vk) = VerifyingKey::from_bytes(&pk_arr) else { return false };

        let Ok(sig_bytes) = URL_SAFE_NO_PAD.decode(sig_b64.as_bytes()) else { return false };
        let Ok(sig_arr): std::result::Result<[u8; 64], _> = sig_bytes.try_into() else { return false };
        let sig = Signature::from_bytes(&sig_arr);

        // verify_strict enforces RFC 8032 canonical encodings (rejects
        // non-canonical R/S and the small-order pitfalls), which is the
        // right choice for a non-consensus capability system.
        vk.verify_strict(&self.signing_payload(), &sig).is_ok()
    }

    /// `true` iff the signature verifies AND the token is unexpired at `now`.
    /// Does NOT check issuer trust — see [`Capability::verify_from`].
    pub fn verify(&self, now: i64) -> bool {
        self.verify_signature() && self.is_valid(now)
    }

    /// Full authorization check for a signed capability: signature verifies,
    /// the `issuer` is one of `trusted_issuers` (base64url public keys), the
    /// token is unexpired, grants `right`, and matches `resource`.
    pub fn verify_from(
        &self,
        trusted_issuers: &[String],
        right: Right,
        resource: &ObjectId,
        now: i64,
    ) -> bool {
        self.verify_signature()
            && trusted_issuers.iter().any(|k| k == &self.issuer)
            && self.allows(right, resource, now)
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

    // -----------------------------------------------------------------------
    // Ed25519 signing
    // -----------------------------------------------------------------------

    #[test]
    fn sign_then_verify_round_trip() {
        let key = IssuerKey::generate().unwrap();
        let res = test_resource();
        let mut cap = Capability::new_read(&res, 3600);
        cap.sign(&key);

        assert_eq!(cap.alg, "ed25519");
        assert_eq!(cap.issuer, key.public_b64());
        assert!(cap.signature.is_some());
        assert!(cap.verify_signature());
        assert!(cap.verify(0));
    }

    #[test]
    fn signed_cap_survives_encode_decode() {
        let key = IssuerKey::generate().unwrap();
        let mut cap = Capability::new_read(&test_resource(), 3600);
        cap.sign(&key);
        let decoded = Capability::decode(&cap.encode()).unwrap();
        assert!(decoded.verify_signature());
    }

    #[test]
    fn tampered_resource_fails_verification() {
        let key = IssuerKey::generate().unwrap();
        let mut cap = Capability::new_read(&test_resource(), 3600);
        cap.sign(&key);
        cap.resource = other_resource().to_string(); // tamper after signing
        assert!(!cap.verify_signature());
    }

    #[test]
    fn wrong_key_fails_verification() {
        let key = IssuerKey::generate().unwrap();
        let attacker = IssuerKey::generate().unwrap();
        let mut cap = Capability::new_read(&test_resource(), 3600);
        cap.sign(&key);
        cap.issuer = attacker.public_b64(); // claim a different issuer
        assert!(!cap.verify_signature());
    }

    #[test]
    fn unsigned_cap_does_not_verify() {
        let cap = Capability::new_read(&test_resource(), 3600);
        assert_eq!(cap.alg, "none");
        assert!(!cap.verify_signature());
        assert!(!cap.verify(0));
    }

    #[test]
    fn verify_rejects_expired_even_if_signature_valid() {
        let key = IssuerKey::generate().unwrap();
        let mut cap = Capability::new_read(&test_resource(), 3600);
        cap.expiry = 1_000;
        cap.sign(&key);
        assert!(cap.verify_signature()); // signature still valid
        assert!(!cap.verify(2_000)); // but expired
    }

    #[test]
    fn verify_from_enforces_trusted_issuer() {
        let key = IssuerKey::generate().unwrap();
        let res = test_resource();
        let mut cap = Capability::new_read(&res, 3600);
        cap.sign(&key);

        let trusted = vec![key.public_b64()];
        assert!(cap.verify_from(&trusted, Right::Read, &res, 0));

        // untrusted issuer list
        let untrusted = vec![IssuerKey::generate().unwrap().public_b64()];
        assert!(!cap.verify_from(&untrusted, Right::Read, &res, 0));
        // trusted but wrong right
        assert!(!cap.verify_from(&trusted, Right::Write, &res, 0));
    }

    #[test]
    fn signed_write_cap_on_sentinel_verifies() {
        let key = IssuerKey::generate().unwrap();
        let res = store_write_resource();
        let mut cap = Capability::grant(&res, vec![Right::Write], 3600);
        cap.sign(&key);
        let trusted = vec![key.public_b64()];
        assert!(cap.verify_from(&trusted, Right::Write, &res, 0));
        // a read cap on the sentinel must not satisfy a write check
        let mut rd = Capability::new_read(&res, 3600);
        rd.sign(&key);
        assert!(!rd.verify_from(&trusted, Right::Write, &res, 0));
    }

    #[test]
    fn issuer_key_seed_round_trip() {
        let key = IssuerKey::generate().unwrap();
        let seed = key.to_seed_bytes();
        let restored = IssuerKey::from_seed(&seed);
        assert_eq!(key.public_b64(), restored.public_b64());
    }
}
