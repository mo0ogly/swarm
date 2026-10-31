//! Single-use cockpit login tickets stored in the Root's private `.swarm`
//! directory, so `swarm open` can sign the browser in without the operator
//! copying the persisted owner key.
//!
//! A ticket stands only for the existing owner key of one listener address: it
//! adds no account, role or authority, and lives under the same operator UID
//! as that key (no new separation claim, see EP-07). Each ticket is one file
//! `web-ticket-<address hash>-<hash of owner key and token>` whose content is
//! its expiry in Unix seconds; neither token nor key is stored, and rotating
//! the owner key orphans every unconsumed ticket. Consumption unlinks the file first,
//! so concurrent or repeated use has exactly one winner; an expired ticket is
//! still consumed and then refused.
#![cfg(target_os = "linux")]
use crate::{
    local_storage::{AtomicStorageFault, LocalDirectory},
    LocalStorageError, LocalStorageOperation, PublicationState, WebSessionError,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};
use std::{error::Error, net::SocketAddr};

/// Wire prefix of a ticket token (protocol representation, not a policy).
const PREFIX: &str = "ticket-";
/// Random bytes per token; 32 bytes base64url-encode to 43 characters.
const TOKEN_BYTES: usize = 32;
/// Largest expiry representation accepted on read (i64 decimal digits plus sign).
const EXPIRY_MAX_BYTES: usize = 20;

/// Freshly issued ticket. Debug is redacted; the token must only be handed to
/// the operator's own browser opener and never logged.
pub struct WebLoginTicket {
    token: String,
    expires_at: i64,
}
impl WebLoginTicket {
    /// Sensitive single-use token to place in the `/login?ticket=` URL.
    pub fn token(&self) -> &str {
        &self.token
    }
    /// Expiry in Unix seconds, derived from the saved v50 lifetime.
    pub fn expires_at(&self) -> i64 {
        self.expires_at
    }
}
impl std::fmt::Debug for WebLoginTicket {
    /// Show only the expiry; the token never appears in diagnostics.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebLoginTicket")
            .field("expires_at", &self.expires_at)
            .finish_non_exhaustive()
    }
}

/// Outcome of presenting a ticket. Every non-`Accepted` value is a refusal;
/// callers must not distinguish them to the browser (no oracle).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebTicketUse {
    /// The ticket existed, was consumed by this call and had not expired.
    Accepted,
    /// Malformed token, unknown ticket, or already consumed by another use.
    Unknown,
    /// The ticket existed and is now consumed, but its expiry had passed.
    Expired,
}

/// Valid token representation: prefix plus 43 base64url characters.
/// Format only; it proves neither issuance nor freshness.
pub fn valid_web_login_ticket(value: &str) -> bool {
    value.len() == PREFIX.len() + 43
        && value.starts_with(PREFIX)
        && value[PREFIX.len()..]
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// File name binding a token to one listener address and to the owner key it
/// stands for. Hash prefixes keep the name short and free of token/key
/// material; 128 bits of hash avoid collisions. Rotating the key changes every
/// name, so unconsumed tickets of the old key become unknown.
fn file_name(address: SocketAddr, owner_key: &str, token: &str) -> String {
    let a = format!("{:x}", Sha256::digest(address.to_string().as_bytes()));
    let mut h = Sha256::new();
    h.update(b"swarm-web-login-ticket-v2\0");
    h.update(owner_key.as_bytes());
    h.update(b"\0");
    h.update(token.as_bytes());
    let t = format!("{:x}", h.finalize());
    format!("web-ticket-{}-{}", &a[..16], &t[..32])
}

/// Issue one ticket for `address`, valid `lifetime_seconds` after `now`.
/// Publishes the expiry with NOREPLACE (a collision refuses, never overwrites).
/// Errors: `web-ticket-lifetime-invalid` for a non-positive or overflowing
/// lifetime, entropy and typed storage causes otherwise. No cookie or
/// authority is created; the caller owns delivering the token privately.
pub(crate) fn issue(
    directory: &LocalDirectory,
    address: SocketAddr,
    owner_key: &str,
    lifetime_seconds: i64,
    now: i64,
) -> Result<WebLoginTicket, WebSessionError> {
    let expires_at = now
        .checked_add(lifetime_seconds)
        .filter(|_| lifetime_seconds > 0)
        .ok_or(WebSessionError::Refused("web-ticket-lifetime-invalid"))?;
    let mut bytes = [0u8; TOKEN_BYTES];
    getrandom::getrandom(&mut bytes).map_err(|e| {
        WebSessionError::Entropy(crate::RuntimeError::new(
            crate::RuntimeOperation::Entropy,
            e,
        ))
    })?;
    let token = format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(bytes));
    directory
        .publish_new_with_fault(
            &file_name(address, owner_key, &token),
            expires_at.to_string().as_bytes(),
            AtomicStorageFault::None,
        )
        .map_err(WebSessionError::Storage)?;
    Ok(WebLoginTicket { token, expires_at })
}

/// True when `error` is exactly a not-found at `operation`, before any
/// publication or cleanup: the ordinary "no such ticket" condition.
fn missing(error: &LocalStorageError, operation: LocalStorageOperation) -> bool {
    error.operation == operation
        && error.publication == PublicationState::NotPublished
        && error.cleanup.is_none()
        && error
            .source()
            .and_then(|e| e.downcast_ref::<rustix::io::Errno>())
            .is_some_and(|e| *e == rustix::io::Errno::NOENT)
}

/// Consume a presented ticket for `address` at `now`. Order: validate format,
/// read expiry, unlink (the single-use boundary), then compare expiry. Losing
/// an unlink race yields `Unknown`. Corrupt content is consumed and refused as
/// `web-ticket-content-invalid`. Storage failures keep their typed cause; a
/// directory-sync failure after unlink is still reported (state uncertain).
pub(crate) fn consume(
    directory: &LocalDirectory,
    address: SocketAddr,
    owner_key: &str,
    token: &str,
    now: i64,
) -> Result<WebTicketUse, WebSessionError> {
    if !valid_web_login_ticket(token) {
        return Ok(WebTicketUse::Unknown);
    }
    let name = file_name(address, owner_key, token);
    let raw = match directory.read_bounded(&name, EXPIRY_MAX_BYTES) {
        Ok(raw) => raw,
        Err(e) if missing(&e, LocalStorageOperation::OpenFile) => return Ok(WebTicketUse::Unknown),
        Err(e) => return Err(WebSessionError::Storage(e)),
    };
    match directory.remove(&name) {
        Ok(()) => {}
        Err(e) if missing(&e, LocalStorageOperation::Remove) => return Ok(WebTicketUse::Unknown),
        Err(e) => return Err(WebSessionError::Storage(e)),
    }
    let expires_at = std::str::from_utf8(&raw)
        .map_err(WebSessionError::Encoding)?
        .parse::<i64>()
        .map_err(|_| WebSessionError::Refused("web-ticket-content-invalid"))?;
    Ok(if now < expires_at {
        WebTicketUse::Accepted
    } else {
        WebTicketUse::Expired
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::DirBuilderExt};
    /// Private fixture directory under the repository's `.swarm-fixtures/`
    /// (never /tmp); retained after the run for inspection.
    fn fixture() -> std::path::PathBuf {
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.swarm-fixtures");
        fs::DirBuilder::new().recursive(true).mode(0o700).create(&base).unwrap();
        let path = base.join(crate::generate_identifier("web-ticket-").unwrap());
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        path
    }
    const KEY: &str = "session-0123456789abcdef01234567";
    fn address(port: u16) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], port))
    }

    /// Valid witness first; then replay, wrong address, expiry and malformed
    /// tokens all refuse, and no file keeps the raw token.
    #[test]
    fn ticket_is_single_use_address_bound_and_expires() {
        let path = fixture();
        let d = LocalDirectory::open(&path).unwrap();
        let t = issue(&d, address(40001), KEY, 60, 1_000).unwrap();
        assert!(valid_web_login_ticket(t.token()));
        assert_eq!(t.expires_at(), 1_060);
        assert!(!format!("{t:?}").contains(t.token()));
        for entry in fs::read_dir(&path).unwrap() {
            let entry = entry.unwrap();
            assert!(!entry.file_name().to_string_lossy().contains(&t.token()[PREFIX.len()..]));
            assert!(!fs::read_to_string(entry.path()).unwrap().contains(t.token()));
        }
        assert_eq!(consume(&d, address(40002), KEY, t.token(), 1_001).unwrap(), WebTicketUse::Unknown);
        assert_eq!(consume(&d, address(40001), KEY, t.token(), 1_001).unwrap(), WebTicketUse::Accepted);
        assert_eq!(consume(&d, address(40001), KEY, t.token(), 1_002).unwrap(), WebTicketUse::Unknown);
        let late = issue(&d, address(40001), KEY, 60, 1_000).unwrap();
        assert_eq!(consume(&d, address(40001), KEY, late.token(), 1_060).unwrap(), WebTicketUse::Expired);
        assert_eq!(consume(&d, address(40001), KEY, late.token(), 1_000).unwrap(), WebTicketUse::Unknown);
        for bad in ["", "ticket-", "ticket-../../etc/passwd-padding-padding-padd", "session-0123456789abcdef01234567"] {
            assert_eq!(consume(&d, address(40001), KEY, bad, 1_000).unwrap(), WebTicketUse::Unknown);
        }
        assert_eq!(fs::read_dir(&path).unwrap().count(), 0);
    }

    /// A ticket issued under one owner key is unknown under a rotated key
    /// (witness: the same ticket is accepted with its own key afterwards).
    #[test]
    fn rotated_owner_key_orphans_unconsumed_ticket() {
        let path = fixture();
        let d = LocalDirectory::open(&path).unwrap();
        let t = issue(&d, address(40004), KEY, 60, 1_000).unwrap();
        let rotated = "session-ffffffffffffffffffffffff";
        assert_eq!(consume(&d, address(40004), rotated, t.token(), 1_001).unwrap(), WebTicketUse::Unknown);
        assert_eq!(consume(&d, address(40004), KEY, t.token(), 1_001).unwrap(), WebTicketUse::Accepted);
    }

    /// Zero, negative and overflowing lifetimes refuse without publication.
    #[test]
    fn invalid_lifetime_refuses_without_file() {
        let path = fixture();
        let d = LocalDirectory::open(&path).unwrap();
        issue(&d, address(40003), KEY, 1, 0).unwrap();
        for (lifetime, now) in [(0, 0), (-5, 0), (1, i64::MAX)] {
            assert_eq!(issue(&d, address(40003), KEY, lifetime, now).unwrap_err().code(), "web-ticket-lifetime-invalid");
        }
        assert_eq!(fs::read_dir(&path).unwrap().count(), 1);
    }
}
