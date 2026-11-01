//! Signed per-request assertion relayed from the authority daemon to the
//! engine (phase B, slice B5.1). Pure functions: no IO, no clock, no policy.
//!
//! Wire: header `X-Swarm-Assertion: v2.<b64url(json claims)>.<b64url(sig)>`,
//! Ed25519 over `swarm-authority-assertion-v2\0` + the exact claims bytes.
//! v2 adds the daemon's [`Witness`](crate::assertion::Witness) (heads of its account journal and audit)
//! so the engine, under another UID, can detect a restored older copy
//! (EP-06); v1 assertions are refused.
//! The claims bind the authenticated subject and its ABAC attributes to one
//! request (method, path with query, SHA-256 of the body) at one instant with
//! a random nonce. The verifier checks signature, binding, freshness window
//! and nonce reuse; window and replay memory are supplied by the caller (an
//! operator setting on the engine side, never a constant here).
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64, Engine};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// HTTP header carrying the assertion.
pub const HEADER: &str = "x-swarm-assertion";
const DOMAIN: &[u8] = b"swarm-authority-assertion-v2\0";

/// Heads of the daemon's persistent state at signing time: account journal
/// (`journal_seq`, `journal_digest`) and access audit (`audit_seq`,
/// `audit_digest` = SHA-256 of its last line). Sequences only grow; the
/// engine keeps the highest it has seen (cross-UID rollback witness).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Witness {
    pub journal_seq: i64,
    pub journal_digest: String,
    pub audit_seq: u64,
    pub audit_digest: String,
}

/// Claims about the authenticated human and the exact request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claims {
    pub v: i64,
    pub subject: String,
    pub roles: Vec<String>,
    pub integrity: i64,
    pub method: String,
    pub path: String,
    pub body_sha256: String,
    pub issued_at: i64,
    pub nonce: String,
    pub witness: Witness,
}

/// SHA-256 hex of a request body (empty body included).
pub fn body_digest(body: &[u8]) -> String {
    format!("{:x}", Sha256::digest(body))
}

/// Sign `claims` with the daemon key and return the header value.
/// Errors: `assertion-encode` if claims cannot serialize.
pub fn sign(key: &SigningKey, claims: &Claims) -> Result<String, &'static str> {
    let raw = serde_json::to_vec(claims).map_err(|_| "assertion-encode")?;
    let mut msg = DOMAIN.to_vec();
    msg.extend_from_slice(&raw);
    let sig = key.sign(&msg);
    Ok(format!("v2.{}.{}", B64.encode(&raw), B64.encode(sig.to_bytes())))
}

/// Replay memory supplied by the verifier owner: returns `true` when the
/// nonce is new (and records it), `false` when already seen in the window.
pub trait NonceMemory {
    fn first_use(&mut self, nonce: &str, issued_at: i64) -> bool;
}

/// Verify a header value against the request actually received.
///
/// Refusal codes: `assertion-format`, `assertion-signature`,
/// `assertion-request-mismatch` (method/path/body differ),
/// `assertion-stale` (|now - issued_at| > `max_age_seconds`),
/// `assertion-replayed`. `max_age_seconds` must be positive (else
/// `assertion-window-invalid`). Success returns the claims for ABAC.
pub fn verify(
    key: &VerifyingKey,
    header: &str,
    method: &str,
    path: &str,
    body: &[u8],
    now: i64,
    max_age_seconds: i64,
    nonces: &mut dyn NonceMemory,
) -> Result<Claims, &'static str> {
    if max_age_seconds < 1 {
        return Err("assertion-window-invalid");
    }
    let mut parts = header.split('.');
    let (Some("v2"), Some(c), Some(s), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
        return Err("assertion-format");
    };
    let raw = B64.decode(c).map_err(|_| "assertion-format")?;
    let sig: [u8; 64] = B64.decode(s).map_err(|_| "assertion-format")?.try_into().map_err(|_| "assertion-format")?;
    let mut msg = DOMAIN.to_vec();
    msg.extend_from_slice(&raw);
    key.verify(&msg, &Signature::from_bytes(&sig)).map_err(|_| "assertion-signature")?;
    let claims: Claims = serde_json::from_slice(&raw).map_err(|_| "assertion-format")?;
    if claims.v != 2 || claims.nonce.len() < 16 {
        return Err("assertion-format");
    }
    if claims.method != method || claims.path != path || claims.body_sha256 != body_digest(body) {
        return Err("assertion-request-mismatch");
    }
    if now.abs_diff(claims.issued_at) > max_age_seconds.unsigned_abs() {
        return Err("assertion-stale");
    }
    if !nonces.first_use(&claims.nonce, claims.issued_at) {
        return Err("assertion-replayed");
    }
    Ok(claims)
}

/// Simple bounded-by-window nonce memory: forgets nonces older than the
/// window it is pruned with. Suitable for one engine process.
#[derive(Default)]
pub struct WindowNonces {
    seen: std::collections::HashMap<String, i64>,
}
impl WindowNonces {
    /// Drop nonces issued before `now - max_age_seconds`; they can no longer
    /// pass the freshness check, so forgetting them cannot enable replay.
    pub fn prune(&mut self, now: i64, max_age_seconds: i64) {
        self.seen.retain(|_, at| now - *at <= max_age_seconds);
    }
}
impl NonceMemory for WindowNonces {
    fn first_use(&mut self, nonce: &str, issued_at: i64) -> bool {
        self.seen.insert(nonce.to_owned(), issued_at).is_none()
    }
}
