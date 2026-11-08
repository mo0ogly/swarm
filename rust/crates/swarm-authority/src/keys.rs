//! Personal API keys (phase B, slice B9) for scripts and tools.
//!
//! A key belongs to one account and acts **as that account**, re-read at
//! every request (a disabled account ends all its keys), capped by the key's
//! `scope`: the highest action family it may perform (`read` < `write` <
//! `approve` < `administer`); the access policy and Biba floors still apply
//! on top. Keys never reach the daemon administration API
//! (`/api/v1/authority/*`): accounts, sessions, keys, policy and settings are
//! managed only from a signed-in browser session.
//!
//! Secret format `sak1-<id>.<secret>`: `id` is 16 hex chars (public, shown
//! in lists and the audit), `secret` 32 random bytes base64url, returned
//! once at creation. Only SHA-256 of the secret is stored, in a journal
//! record `apikey/<id>` signed like the accounts (survives restarts;
//! revocation is a journal revoke and cannot be undone). Every key has an
//! explicit expiry chosen by its creator; no default lifetime exists.
use crate::accounts::{AccountView, Accounts, Error};
use crate::policy::Action;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use swarm_enrollment_state::{sign_change, Context};

const PREFIX: &str = "sak1-";

/// Persisted body of one key (no secret, only its digest).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct KeyBody {
    schema_version: i64,
    id: String,
    subject: String,
    name: String,
    scope: Action,
    created: i64,
    expires: i64,
    digest: String,
}

/// Displayable key (never the secret or its digest).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct KeyView {
    pub id: String,
    pub subject: String,
    pub name: String,
    pub scope: Action,
    pub created: i64,
    pub expires: i64,
}

fn view(b: &KeyBody) -> KeyView {
    KeyView { id: b.id.clone(), subject: b.subject.clone(), name: b.name.clone(), scope: b.scope, created: b.created, expires: b.expires }
}

fn digest(secret: &str) -> String {
    format!("{:x}", Sha256::digest(secret.as_bytes()))
}

/// Constant-time equality of two equal-length hex digests.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

impl Accounts {
    fn key_bodies(&self, ctx: &Context) -> Result<Vec<KeyBody>, Error> {
        let (head, ids) = self.serial_snapshot(ctx).map_err(Error::Journal)?;
        let mut out = Vec::new();
        for id in ids.into_iter().filter(|id| id.starts_with("apikey/")) {
            let body = self.serial_active(ctx, &head, &id, |r| Ok(r.body)).map_err(Error::Journal)?;
            let parsed: KeyBody = serde_json::from_slice(&body).map_err(Error::Body)?;
            if format!("apikey/{}", parsed.id) != id || parsed.schema_version != 1 {
                return Err(Error::Refused("key-body-mismatch"));
            }
            out.push(parsed);
        }
        out.sort_by(|a, b| (&a.subject, a.created, &a.id).cmp(&(&b.subject, b.created, &b.id)));
        Ok(out)
    }

    /// Create a key for active account `subject`. `name`: non-empty, no
    /// control characters; `expires` must be later than `now`. Returns the
    /// secret (shown once) and the view. Errors: `account-missing`,
    /// `key-name-invalid`, `key-expiry-invalid`, entropy/journal errors.
    pub fn create_key(&self, ctx: &Context, subject: &str, name: &str, scope: Action, now: i64, expires: i64) -> Result<(String, KeyView), Error> {
        use base64::Engine as _;
        if self.current(ctx, subject)?.is_none() {
            return Err(Error::Refused("account-missing"));
        }
        if name.trim().is_empty() || name.chars().any(char::is_control) {
            return Err(Error::Refused("key-name-invalid"));
        }
        if expires <= now {
            return Err(Error::Refused("key-expiry-invalid"));
        }
        let (mut id_raw, mut secret_raw) = ([0u8; 8], [0u8; 32]);
        getrandom::fill(&mut id_raw).map_err(|_| Error::Entropy)?;
        getrandom::fill(&mut secret_raw).map_err(|_| Error::Entropy)?;
        let id: String = id_raw.iter().map(|b| format!("{b:02x}")).collect();
        let secret = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(secret_raw);
        let body = KeyBody { schema_version: 1, id: id.clone(), subject: subject.into(), name: name.into(), scope, created: now, expires, digest: digest(&secret) };
        let raw_body = serde_json::to_vec(&body).map_err(Error::Body)?;
        let (head, _) = self.serial_snapshot(ctx).map_err(Error::Journal)?;
        let raw = sign_change(&self.key, self.project, &head, "enroll", &format!("apikey/{id}"), &raw_body).map_err(Error::Journal)?;
        self.serial_apply(ctx, &head, &raw).map_err(Error::Journal)?;
        Ok((format!("{PREFIX}{id}.{secret}"), view(&body)))
    }

    /// Active (not revoked) keys, expired ones included; `subject` filters.
    pub fn keys(&self, ctx: &Context, subject: Option<&str>) -> Result<Vec<KeyView>, Error> {
        Ok(self.key_bodies(ctx)?.iter().filter(|b| subject.is_none_or(|s| b.subject == s)).map(view).collect())
    }

    /// Revoke key `id` (permanent). When `owner` is given the key must
    /// belong to it. Errors: `key-missing` (unknown, revoked or not owned).
    pub fn revoke_key(&self, ctx: &Context, id: &str, owner: Option<&str>) -> Result<(), Error> {
        let found = self.key_bodies(ctx)?.into_iter().any(|b| b.id == id && owner.is_none_or(|o| b.subject == o));
        if !found {
            return Err(Error::Refused("key-missing"));
        }
        let (head, _) = self.serial_snapshot(ctx).map_err(Error::Journal)?;
        let raw = sign_change(&self.key, self.project, &head, "revoke", &format!("apikey/{id}"), b"").map_err(Error::Journal)?;
        self.serial_apply(ctx, &head, &raw).map_err(Error::Journal)?;
        Ok(())
    }

    /// Resolve a presented `sak1-…` key at `now`: the account's current view,
    /// the key scope and its id. Malformed, unknown, revoked, expired keys
    /// and disabled accounts all refuse with `key-refused` (no oracle).
    pub fn resolve_key(&self, ctx: &Context, presented: &str, now: i64) -> Result<(AccountView, Action, String), Error> {
        let refused = || Error::Refused("key-refused");
        let (id, secret) = presented.strip_prefix(PREFIX).and_then(|r| r.split_once('.')).ok_or_else(refused)?;
        if id.len() != 16 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(refused());
        }
        let body = self.key_bodies(ctx)?.into_iter().find(|b| b.id == id).ok_or_else(refused)?;
        if !same(&body.digest, &digest(secret)) || now >= body.expires {
            return Err(refused());
        }
        let account = self.current(ctx, &body.subject)?.ok_or_else(refused)?;
        Ok((account, body.scope, body.id))
    }
}
