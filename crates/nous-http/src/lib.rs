//! `nous-http` — tiny_http-based HTTP server for the Nous content store.
//!
//! # Endpoints
//! - `GET  /health`        → 200 `"ok"`
//! - `GET  /object/<cid>`  → 200 raw bytes  (`application/octet-stream`)
//! - `GET  /meta/<cid>`    → 200 JSON of [`nous_core::Meta`]
//! - `POST /object`        → 201 `<cid-string>`
//!
//! Cap-gating (when `enforce_caps = true`):
//!   - Missing / unparseable token → 401
//!   - Valid token but not allowed for this cid/right → 403

use std::net::SocketAddr;
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tiny_http::{Header, Method, Request, Response, Server};

use nous_caps::Capability;
use nous_core::{Error, ObjectId, Result, Right};
use nous_store::Store;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Configuration for capability enforcement.
#[derive(Clone, Default)]
pub struct CapPolicy {
    /// When true, reads require a valid signed capability and writes are denied.
    pub enforce: bool,
    /// Base64url public keys of issuers whose signed capabilities are trusted.
    pub trusted_issuers: Vec<String>,
}

/// Blocking server loop (tiny_http). Returns `Err` only on a fatal bind error.
///
/// When `policy.enforce` is true:
/// - `GET /object/<cid>` and `GET /meta/<cid>` require an
///   `Authorization: Bearer <captoken>` whose signature verifies against a
///   trusted issuer and which grants `Right::Read` for the cid (see
///   [`Capability::verify_from`]). Unsigned tokens are rejected.
/// - `POST /object` (writes) is denied (403) until write-capabilities are
///   designed in a later phase.
pub fn serve(store: Store, addr: SocketAddr, policy: CapPolicy) -> Result<()> {
    let server = Server::http(addr.to_string())
        .map_err(|e| Error::Http(format!("bind failed on {addr}: {e}")))?;

    eprintln!(
        "[nous-http] listening on {addr} (enforce_caps={}, trusted_issuers={})",
        policy.enforce,
        policy.trusted_issuers.len()
    );

    for request in server.incoming_requests() {
        handle_request(&store, request, &policy);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Request dispatch
// ---------------------------------------------------------------------------

fn handle_request(store: &Store, request: Request, policy: &CapPolicy) {
    let method = request.method().clone();
    let path = request.url().to_owned();
    let status = dispatch(store, request, policy);
    eprintln!("[nous-http] {} {} -> {}", method, path, status);
}

/// Dispatch one request, returning the HTTP status code that was sent.
fn dispatch(store: &Store, request: Request, policy: &CapPolicy) -> u16 {
    let method = request.method().clone();
    let url = request.url().to_owned();

    match (method, url.as_str()) {
        (Method::Get, "/health") => {
            send(request, 200, "text/plain", b"ok".to_vec());
            200
        }

        (Method::Get, path) if path.starts_with("/object/") => {
            let cid_str = path["/object/".len()..].to_owned();
            handle_get_object(store, request, &cid_str, policy)
        }

        (Method::Get, path) if path.starts_with("/meta/") => {
            let cid_str = path["/meta/".len()..].to_owned();
            handle_get_meta(store, request, &cid_str, policy)
        }

        (Method::Post, "/object") => handle_post_object(store, request, policy),

        _ => {
            send(request, 404, "text/plain", b"not found".to_vec());
            404
        }
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

fn handle_get_object(
    store: &Store,
    request: Request,
    cid_str: &str,
    policy: &CapPolicy,
) -> u16 {
    let cid = match parse_cid(cid_str) {
        Ok(id) => id,
        Err(status) => {
            send(request, status, "text/plain", b"invalid object id".to_vec());
            return status;
        }
    };

    if policy.enforce {
        if let Some(status) = check_cap(&request, Right::Read, &cid, policy) {
            send(request, status, "text/plain", cap_error_body(status));
            return status;
        }
    }

    match store.get(&cid) {
        Ok(bytes) => {
            send(request, 200, "application/octet-stream", bytes);
            200
        }
        Err(Error::NotFound(_)) => {
            send(request, 404, "text/plain", b"not found".to_vec());
            404
        }
        Err(Error::Corrupt { .. }) => {
            send(request, 500, "text/plain", b"corrupt object".to_vec());
            500
        }
        Err(e) => {
            send(request, 500, "text/plain", format!("error: {e}").into_bytes());
            500
        }
    }
}

fn handle_get_meta(
    store: &Store,
    request: Request,
    cid_str: &str,
    policy: &CapPolicy,
) -> u16 {
    let cid = match parse_cid(cid_str) {
        Ok(id) => id,
        Err(status) => {
            send(request, status, "text/plain", b"invalid object id".to_vec());
            return status;
        }
    };

    if policy.enforce {
        if let Some(status) = check_cap(&request, Right::Read, &cid, policy) {
            send(request, status, "text/plain", cap_error_body(status));
            return status;
        }
    }

    match store.get_meta(&cid) {
        Ok(meta) => match serde_json::to_vec(&meta) {
            Ok(json) => {
                send(request, 200, "application/json", json);
                200
            }
            Err(e) => {
                send(
                    request,
                    500,
                    "text/plain",
                    format!("serialization error: {e}").into_bytes(),
                );
                500
            }
        },
        Err(Error::NotFound(_)) => {
            send(request, 404, "text/plain", b"not found".to_vec());
            404
        }
        Err(e) => {
            send(request, 500, "text/plain", format!("error: {e}").into_bytes());
            500
        }
    }
}

fn handle_post_object(store: &Store, mut request: Request, policy: &CapPolicy) -> u16 {
    // Writes are not yet cap-gated by a Right::Write capability. Until that is
    // designed, deny all writes when enforcement is on (fail closed) rather
    // than silently accepting unauthenticated writes.
    if policy.enforce {
        send(request, 403, "text/plain", b"writes are not permitted when caps are enforced".to_vec());
        return 403;
    }

    let content_type = header_value(&request, "content-type").map(|s| s.to_owned());

    let mut body = Vec::new();
    if let Err(e) = request.as_reader().read_to_end(&mut body) {
        send(request, 500, "text/plain", format!("read error: {e}").into_bytes());
        return 500;
    }

    match store.put(&body, content_type) {
        Ok(cid) => {
            let cid_str = cid.to_string().into_bytes();
            send(request, 201, "text/plain", cid_str);
            201
        }
        Err(e) => {
            send(request, 500, "text/plain", format!("store error: {e}").into_bytes());
            500
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers: parsing, cap-check, header access
// ---------------------------------------------------------------------------

/// Parse a CID string from a URL path segment.
/// Returns `Err(400)` on any parse failure.
pub fn parse_cid(s: &str) -> std::result::Result<ObjectId, u16> {
    ObjectId::from_str(s).map_err(|_| 400u16)
}

/// Check the `Authorization` header for a valid signed capability.
///
/// Returns `None` if the request is permitted, or `Some(status)` where:
/// - `401` = missing or malformed/undecodable token
/// - `403` = token decoded but the signature does not verify against a trusted
///   issuer, or it is expired / does not grant `right` for this cid
///
/// Enforcement requires a *signed* capability ([`Capability::verify_from`]):
/// unsigned (`alg = "none"`) tokens never pass.
pub fn check_cap(request: &Request, right: Right, cid: &ObjectId, policy: &CapPolicy) -> Option<u16> {
    let now = unix_now_secs();

    let auth = match header_value(request, "authorization") {
        Some(v) => v.to_owned(),
        None => return Some(401),
    };

    // Expect "Bearer <token>"
    let token = match auth.strip_prefix("Bearer ") {
        Some(t) => t.trim().to_owned(),
        None => return Some(401),
    };

    let cap = match Capability::decode(&token) {
        Ok(c) => c,
        Err(_) => return Some(401),
    };

    if cap.verify_from(&policy.trusted_issuers, right, cid, now) {
        None
    } else {
        Some(403)
    }
}

/// Find the first header matching `name` (case-insensitive), returning its value.
pub fn header_value<'a>(request: &'a Request, name: &str) -> Option<&'a str> {
    let name_lower = name.to_lowercase();
    request
        .headers()
        .iter()
        .find(|h| h.field.as_str().as_str().to_ascii_lowercase() == name_lower)
        .map(|h| h.value.as_str())
}

/// Current Unix timestamp in seconds.
///
/// Deliberately uses the real clock only: capability expiry is a security
/// boundary, so it must not be influenced by `SOURCE_DATE_EPOCH` or any other
/// attacker-influenceable environment variable.
pub fn unix_now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs() as i64
}

fn cap_error_body(status: u16) -> Vec<u8> {
    match status {
        401 => b"missing or invalid capability token".to_vec(),
        403 => b"capability does not permit this operation".to_vec(),
        _ => b"unauthorized".to_vec(),
    }
}

/// Send a response with the given status, content-type, and body.
fn send(request: Request, status: u16, content_type: &str, body: Vec<u8>) {
    let ct_header = Header::from_bytes(b"Content-Type", content_type.as_bytes())
        .expect("static header value is always valid");
    let response = Response::from_data(body)
        .with_status_code(status)
        .with_header(ct_header);
    // Ignore send errors — client may have disconnected
    let _ = request.respond(response);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Unit tests for parse_cid — no network, no store required
    // -----------------------------------------------------------------------

    #[test]
    fn parse_cid_valid() {
        let id = ObjectId::of_bytes(b"test data");
        let s = id.to_string();
        assert_eq!(parse_cid(&s).unwrap(), id);
    }

    #[test]
    fn parse_cid_bad_prefix_returns_400() {
        assert_eq!(parse_cid("sha256:deadbeef"), Err(400));
    }

    #[test]
    fn parse_cid_short_hex_returns_400() {
        assert_eq!(parse_cid("b3:deadbeef"), Err(400));
    }

    #[test]
    fn parse_cid_empty_returns_400() {
        assert_eq!(parse_cid(""), Err(400));
    }

    #[test]
    fn parse_cid_non_hex_returns_400() {
        // 64 chars but contains 'z'
        let bad = format!("b3:{}z{}", "a".repeat(32), "b".repeat(31));
        assert_eq!(parse_cid(&bad), Err(400));
    }

    // -----------------------------------------------------------------------
    // unix_now_secs must ignore SOURCE_DATE_EPOCH (security: expiry boundary)
    // -----------------------------------------------------------------------

    #[test]
    fn unix_now_secs_ignores_source_date_epoch() {
        std::env::set_var("SOURCE_DATE_EPOCH", "1700000000");
        let t = unix_now_secs();
        std::env::remove_var("SOURCE_DATE_EPOCH");
        // Real clock is well past 2023-11; must NOT be frozen to the epoch.
        assert!(t > 1_700_000_000_i64, "expiry clock must use real time, got {t}");
    }

    // -----------------------------------------------------------------------
    // Enforcement: unsigned/forged tokens are rejected by check_cap path
    // -----------------------------------------------------------------------

    #[test]
    fn unsigned_cap_rejected_by_verify_from() {
        use nous_caps::Capability;
        let cid = ObjectId::of_bytes(b"resource");
        // An attacker-crafted unsigned cap that "allows" read.
        let forged = Capability::new_read(&cid, 0);
        let trusted: Vec<String> = vec![];
        // verify_from must reject: no valid signature, no trusted issuer.
        assert!(!forged.verify_from(&trusted, Right::Read, &cid, 0));
    }

    // -----------------------------------------------------------------------
    // Integration-style test: real tiny_http server + real TCP connection.
    //
    // Port: 38080 by default; override with NOUS_HTTP_TEST_PORT env var.
    //
    // We skip gracefully if nous-store cannot open the temp directory (e.g.
    // sandboxed CI without filesystem write access).  Core HTTP logic is
    // covered by the unit tests above even when this test is skipped.
    // -----------------------------------------------------------------------

    #[test]
    fn integration_health_and_get_object() {
        use std::io::{Read as _, Write as _};
        use std::net::TcpStream;

        let port: u16 = std::env::var("NOUS_HTTP_TEST_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(38080);

        let dir = std::env::temp_dir().join(format!("nous_http_test_{port}"));
        std::fs::create_dir_all(&dir).expect("create temp dir");

        let store = match nous_store::Store::open(&dir) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("SKIP integration test: store::open failed: {e}");
                return;
            }
        };

        let payload = b"hello nous-http";
        let cid = store.put(payload, None).expect("put object");
        let cid_str = cid.to_string();

        // Reopen store for the server thread (Store is not Clone)
        let store2 = match nous_store::Store::open(&dir) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("SKIP integration test: store reopen failed: {e}");
                return;
            }
        };

        let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
        std::thread::spawn(move || {
            let _ = serve(store2, addr, CapPolicy::default());
        });

        // Give tiny_http a moment to bind
        std::thread::sleep(std::time::Duration::from_millis(150));

        // --- GET /health ---
        {
            let mut stream = TcpStream::connect(addr).expect("connect for /health");
            write!(stream, "GET /health HTTP/1.0\r\nHost: localhost\r\n\r\n").unwrap();
            let mut resp = String::new();
            stream.read_to_string(&mut resp).unwrap();
            assert!(
                resp.contains("ok"),
                "/health response must contain 'ok', got: {resp}"
            );
        }

        // --- GET /object/<cid> ---
        {
            let mut stream = TcpStream::connect(addr).expect("connect for /object");
            write!(
                stream,
                "GET /object/{cid_str} HTTP/1.0\r\nHost: localhost\r\n\r\n"
            )
            .unwrap();
            let mut resp = Vec::new();
            stream.read_to_end(&mut resp).unwrap();
            let resp_str = String::from_utf8_lossy(&resp);
            let status_line = &resp_str[..resp_str.find('\r').unwrap_or(resp_str.len())];
            assert!(
                resp_str.starts_with("HTTP/1.1 200"),
                "expected 200, got: {status_line}"
            );
            // Body is everything after the blank header/body separator
            let sep = b"\r\n\r\n";
            let body_start = resp
                .windows(4)
                .position(|w| w == sep)
                .map(|i| i + 4)
                .unwrap_or(resp.len());
            let body = &resp[body_start..];
            assert_eq!(body, payload, "response body must match stored payload");
        }

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }
}
