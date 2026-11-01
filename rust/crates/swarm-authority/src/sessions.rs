//! Daemon-held sign-in sessions (phase B, slice B4).
//!
//! A session token is 32 random bytes (base64url, prefix `as1-`) returned
//! once at sign-in; only its SHA-256 is kept, in memory (no session secret on
//! disk, so a daemon restart signs everyone out). Each resolution re-reads
//! the account: a disabled account or a new password generation ends the
//! session at the next request (revocation point = that read, EP-05).
//! Lifetimes come from [`crate::config::AuthorityConfig`] (absolute and idle).
use crate::accounts::{AccountView, Accounts, Error};
use crate::config::AuthorityConfig;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::{Mutex, RwLock}};
use swarm_enrollment_state::Context;

const PREFIX: &str = "as1-";

struct Session {
    subject: String,
    generation: i64,
    created: i64,
    last_seen: i64,
}

/// In-memory session table. All methods take `now` (Unix seconds) explicitly
/// so expiry is testable and no clock policy hides inside.
pub struct Sessions {
    config: RwLock<AuthorityConfig>,
    table: Mutex<HashMap<[u8; 32], Session>>,
}

/// Displayable session entry (no secret: the id is a hash prefix).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub subject: String,
    pub created: i64,
    pub last_seen: i64,
}

/// Public id of a session: first 8 bytes of the token hash, hex.
fn public_id(k: &[u8; 32]) -> String {
    k[..8].iter().map(|b| format!("{b:02x}")).collect()
}

/// Public id of a presented token (for marking the caller's own session).
pub fn token_id(token: &str) -> String {
    public_id(&key(token))
}

/// Hash a presented token; malformed tokens hash too but never match.
fn key(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

impl Sessions {
    /// Empty table using `config` lifetimes.
    pub fn new(config: AuthorityConfig) -> Self {
        Self { config: RwLock::new(config), table: Mutex::new(HashMap::new()) }
    }

    /// Replace the lifetimes used from the next resolution on (Admin edit of
    /// `authority.json`); existing sessions are judged by the new values.
    pub fn set_config(&self, config: AuthorityConfig) -> Result<(), Error> {
        *self.config.write().map_err(|_| Error::Refused("session-state"))? = config;
        Ok(())
    }

    /// Open sessions (expired ones excluded, never their token): public id
    /// (first 16 hex of the token hash), subject, sign-in and last-seen
    /// times, sorted by subject then sign-in time.
    pub fn list(&self, now: i64) -> Result<Vec<SessionInfo>, Error> {
        let config = *self.config.read().map_err(|_| Error::Refused("session-state"))?;
        let table = self.table.lock().map_err(|_| Error::Refused("session-state"))?;
        let mut out: Vec<SessionInfo> = table
            .iter()
            .filter(|(_, s)| now - s.created < config.session_absolute_seconds && now - s.last_seen < config.session_idle_seconds)
            .map(|(k, s)| SessionInfo { id: public_id(k), subject: s.subject.clone(), created: s.created, last_seen: s.last_seen })
            .collect();
        out.sort_by(|a, b| (&a.subject, a.created).cmp(&(&b.subject, b.created)));
        Ok(out)
    }

    /// End the session whose public id is `id`; `Ok(false)` when none matches.
    pub fn revoke_id(&self, id: &str) -> Result<bool, Error> {
        let mut table = self.table.lock().map_err(|_| Error::Refused("session-state"))?;
        let before = table.len();
        table.retain(|k, _| public_id(k) != id);
        Ok(table.len() != before)
    }

    /// Verify `password` for `subject` and open a session. Returns the token
    /// (to hand to the browser once, never logged) and the account view.
    /// Refusals are the registry's (`account-authentication-refused` for
    /// both unknown subject and wrong password). Entropy failure refuses.
    pub fn login(&self, ctx: &Context, accounts: &Accounts, subject: &str, password: &[u8], now: i64) -> Result<(String, AccountView), Error> {
        use base64::Engine as _;
        let view = accounts.verify(ctx, subject, password)?;
        let mut raw = [0u8; 32];
        getrandom::fill(&mut raw).map_err(|_| Error::Entropy)?;
        let token = format!("{PREFIX}{}", base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw));
        let session = Session { subject: view.subject.clone(), generation: view.generation, created: now, last_seen: now };
        self.table.lock().map_err(|_| Error::Refused("session-state"))?.insert(key(&token), session);
        Ok((token, view))
    }

    /// Resolve a presented token at `now`: unknown, expired (absolute or
    /// idle), disabled account or changed password generation all refuse with
    /// `session-refused` and drop the entry. Success refreshes `last_seen` and
    /// returns the current account view (current roles/attributes for ABAC).
    pub fn resolve(&self, ctx: &Context, accounts: &Accounts, token: &str, now: i64) -> Result<AccountView, Error> {
        let refused = || Error::Refused("session-refused");
        if !token.starts_with(PREFIX) {
            return Err(refused());
        }
        let config = *self.config.read().map_err(|_| Error::Refused("session-state"))?;
        let k = key(token);
        let (subject, generation) = {
            let mut table = self.table.lock().map_err(|_| Error::Refused("session-state"))?;
            let Some(s) = table.get(&k) else { return Err(refused()) };
            if now - s.created >= config.session_absolute_seconds || now - s.last_seen >= config.session_idle_seconds {
                table.remove(&k);
                return Err(refused());
            }
            (s.subject.clone(), s.generation)
        };
        let current = accounts.current(ctx, &subject)?;
        let mut table = self.table.lock().map_err(|_| Error::Refused("session-state"))?;
        match current {
            Some(view) if view.generation == generation => {
                if let Some(s) = table.get_mut(&k) {
                    s.last_seen = now;
                    Ok(view)
                } else {
                    Err(refused()) // logged out concurrently
                }
            }
            _ => {
                table.remove(&k);
                Err(refused())
            }
        }
    }

    /// End one session (sign-out). Unknown tokens are a no-op success
    /// (`None`); otherwise returns the subject whose session ended.
    pub fn logout(&self, token: &str) -> Result<Option<String>, Error> {
        Ok(self.table.lock().map_err(|_| Error::Refused("session-state"))?.remove(&key(token)).map(|s| s.subject))
    }

    /// End every session of `subject` (admin action); returns how many ended.
    pub fn revoke_subject(&self, subject: &str) -> Result<usize, Error> {
        let mut table = self.table.lock().map_err(|_| Error::Refused("session-state"))?;
        let before = table.len();
        table.retain(|_, s| s.subject != subject);
        Ok(before - table.len())
    }
}

/// Forgotten-password requests awaiting an administrator, in memory (a
/// restart forgets them; each one is also in the audit). A request is
/// remembered with the account generation it was made under and disappears
/// by itself once the password changed (any path: Admin, own change, the
/// sudo `set-password` command) or the account is disabled.
#[derive(Default)]
pub struct ResetRequests {
    table: Mutex<HashMap<String, (i64, i64)>>,
}

impl ResetRequests {
    /// Record a request for `subject` at `generation`; `false` when one is
    /// already pending for that generation (nothing new to record).
    pub fn request(&self, subject: &str, generation: i64, now: i64) -> bool {
        let Ok(mut t) = self.table.lock() else { return false };
        match t.get(subject) {
            Some((g, _)) if *g == generation => false,
            _ => {
                t.insert(subject.to_owned(), (generation, now));
                true
            }
        }
    }

    /// Pending requests still valid against the current `accounts` view:
    /// subject → request time. Stale entries (generation changed, account
    /// gone) are dropped.
    pub fn pending(&self, accounts: &[AccountView]) -> std::collections::BTreeMap<String, i64> {
        let Ok(mut t) = self.table.lock() else { return Default::default() };
        t.retain(|s, (g, _)| accounts.iter().any(|a| &a.subject == s && a.generation == *g));
        t.iter().map(|(s, (_, at))| (s.clone(), *at)).collect()
    }
}
