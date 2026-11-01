//! Daemon-served administration API (phase B, slice B8).
//!
//! Served by the daemon itself under [`crate::policy::AUTHORITY_PREFIX`]
//! (never relayed: accounts, sessions and the daemon policy live under the
//! daemon UID, not in the engine). JSON in and out; refusals are
//! `{"error": CODE}` with a stable code and never echo a password.
//!
//! - `GET  …/me` — caller's account, session id and `administer` capability
//!   (self-service: allowed to any signed-in account, not policy-checked).
//! - `POST …/me/password {current, new}` — change one's own password; the
//!   current password is re-verified; a fresh session cookie replaces the
//!   ended one.
//! - `GET  …/accounts`, `POST …/accounts {subject, display_name, roles,
//!   integrity, attributes, password}`, `POST …/accounts/S/profile {...}`,
//!   `POST …/accounts/S/password {password}`, `POST …/accounts/S/disable`.
//! - `GET  …/sessions`, `POST …/sessions/ID/revoke`.
//! - `GET  …/policy`, `POST …/policy/preview {policy, method, path, roles,
//!   integrity, attributes}`, `POST …/policy {policy, revision}`.
//! - `GET  …/config`, `POST …/config/preview {config}`,
//!   `POST …/config {config, revision}`.
//! - `GET  …/audit` — every audit line ([`crate::audit`]) and `chain_valid`;
//!   `GET …/audit/sessions?from&to&subject` (connections per session),
//!   `GET …/audit/sessions/CREDENTIAL` (lines of one session or key),
//!   `GET …/audit/dashboard?from&to` (per day and account), `POST
//!   …/audit/purge {older_than_days}` (≥ 1, no default; chain kept). All
//!   need the `audit.read` permission.
//! - B9 keys ([`crate::keys`]): `GET …/me/keys`, `POST …/me/keys {name,
//!   scope, expires_in_seconds}` (secret returned once), `POST
//!   …/me/keys/ID/revoke` (own keys, self-service); `GET …/keys`, `POST
//!   …/keys/ID/revoke` (all accounts, `administer`). Keys themselves are
//!   refused on this whole API (`authority-key-scope`, in the server).
//!
//! Everything but `me` is `administer` (checked by the caller in
//! [`crate::server`] before dispatch). Safety rules enforced here, not by
//! policy: an administrator cannot disable their own account, cannot change
//! their own profile into one the current policy would deny administration
//! to, and cannot apply a policy that would deny themselves administration
//! (`authority-self-lockout`). Policy and configuration writes require the
//! revision the editor read (`authority-revision-stale`, 409) and replace
//! `policy.json` / `authority.json` atomically (temporary file in the same
//! directory, fsync, rename, directory fsync) before swapping the live copy.
use crate::accounts::{AccountView, Profile};
use crate::config::AuthorityConfig;
use crate::policy::{self, Action, Policy, Subject};
use crate::server::{full, now, read_body, Body, Front, COOKIE};
use http::{header, HeaderValue, Method, StatusCode};
use hyper::body::{Bytes, Incoming};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path, sync::Arc};
use swarm_enrollment_state::Context;

/// Self-service subtree, allowed to every signed-in account.
pub const ME: &str = "/api/v1/authority/me";

fn reply(status: StatusCode, v: Value) -> http::Response<Body> {
    full(status, "application/json; charset=utf-8", v.to_string())
}
fn refuse(status: StatusCode, code: &str) -> http::Response<Body> {
    reply(status, json!({"error": code}))
}
fn ok(v: Value) -> http::Response<Body> {
    reply(StatusCode::OK, v)
}

/// Revision of a persisted document: SHA-256 of its canonical JSON.
pub fn revision<T: serde::Serialize>(doc: &T) -> String {
    format!("{:x}", Sha256::digest(serde_json::to_vec(doc).unwrap_or_default()))
}

/// Replace `dir/name` atomically with `bytes` (0600). On any failure the
/// previous file is intact and the temporary file removed; error
/// `authority-persist-io`.
pub fn persist(dir: &Path, name: &str, bytes: &[u8]) -> Result<(), &'static str> {
    use std::io::Write as _;
    use std::os::unix::fs::OpenOptionsExt as _;
    let mut nonce = [0u8; 8];
    getrandom::fill(&mut nonce).map_err(|_| "authority-entropy")?;
    let tmp = dir.join(format!(".{name}.{}.tmp", nonce.iter().map(|b| format!("{b:02x}")).collect::<String>()));
    let result = (|| {
        let mut f = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        std::fs::rename(&tmp, dir.join(name))?;
        std::fs::File::open(dir)?.sync_all()
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
        return Err("authority-persist-io");
    }
    Ok(())
}

fn source(dir: &Path, name: &str) -> &'static str {
    if dir.join(name).exists() { "saved" } else { "installation" }
}

fn subject_of(v: &AccountView) -> Subject<'_> {
    Subject { roles: &v.roles, integrity: v.integrity, attributes: &v.attributes }
}

/// Would `policy` let this subject administer the daemon?
fn can_administer(policy: &Policy, roles: &[String], integrity: i64, attributes: &BTreeMap<String, String>) -> bool {
    policy::decide(policy, &Subject { roles, integrity, attributes }, "POST", policy::AUTHORITY_PREFIX).allowed
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileIn {
    display_name: String,
    roles: Vec<String>,
    integrity: i64,
    #[serde(default)]
    attributes: BTreeMap<String, String>,
}
impl From<ProfileIn> for Profile {
    fn from(p: ProfileIn) -> Self {
        Profile { display_name: p.display_name, roles: p.roles, integrity: p.integrity, attributes: p.attributes }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateIn {
    subject: String,
    display_name: String,
    roles: Vec<String>,
    integrity: i64,
    #[serde(default)]
    attributes: BTreeMap<String, String>,
    password: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnPasswordIn {
    current: String,
    new: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KeyIn {
    name: String,
    scope: Action,
    /// Lifetime chosen by the creator (no default lifetime exists).
    expires_in_seconds: i64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PasswordIn {
    password: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyApplyIn {
    policy: Value,
    revision: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyPreviewIn {
    policy: Value,
    method: String,
    path: String,
    #[serde(default)]
    roles: Vec<String>,
    integrity: i64,
    #[serde(default)]
    attributes: BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigIn {
    config: Value,
    #[serde(default)]
    revision: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PurgeIn {
    /// Lines older than this many days are removed (chosen by the admin;
    /// no default retention exists).
    older_than_days: i64,
}

fn q_i64(query: &[(String, String)], key: &str) -> Option<i64> {
    query.iter().find(|(k, _)| k == key).and_then(|(_, v)| v.parse().ok())
}

#[allow(clippy::result_large_err)] // the Err is the ready HTTP refusal
fn decode<T: serde::de::DeserializeOwned>(body: &Bytes) -> Result<T, http::Response<Body>> {
    serde_json::from_slice(body).map_err(|_| refuse(StatusCode::BAD_REQUEST, "authority-request-invalid"))
}

/// Run blocking account work on the blocking pool; a join failure refuses.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    tokio::task::spawn_blocking(f).await.ok()
}

/// Every role must exist in the live policy (`account-role-unknown`).
#[allow(clippy::result_large_err)] // the Err is the ready HTTP refusal
fn known_roles(front: &Front, roles: &[String]) -> Result<(), http::Response<Body>> {
    let Ok(p) = front.policy.read() else { return Err(refuse(StatusCode::INTERNAL_SERVER_ERROR, "authority-state")) };
    if roles.iter().all(|r| p.has_role(r)) { Ok(()) } else { Err(refuse(StatusCode::BAD_REQUEST, "account-role-unknown")) }
}

fn account_refusal(code: &str) -> http::Response<Body> {
    let status = match code {
        "account-missing" | "key-missing" => StatusCode::NOT_FOUND,
        "account-role-unknown" => StatusCode::BAD_REQUEST,
        "key-name-invalid" | "key-expiry-invalid" => StatusCode::BAD_REQUEST,
        "account-exists" => StatusCode::CONFLICT,
        "account-authentication-refused" => StatusCode::FORBIDDEN,
        c if c.starts_with("account-") && c.ends_with("-invalid") => StatusCode::BAD_REQUEST,
        "account-credential" => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    refuse(status, code)
}

/// Dispatch one administration request for the signed-in `caller` holding
/// session `token`. The caller (server) has already checked Host/Origin,
/// resolved the session and, outside [`ME`], the `administer` decision.
pub async fn handle(front: Arc<Front>, req: http::Request<Incoming>, caller: AccountView, token: String) -> http::Response<Body> {
    let method = req.method().clone();
    let path = req.uri().path().to_owned();
    let rest = path.strip_prefix(policy::AUTHORITY_PREFIX).unwrap_or("").to_owned();
    let segments: Vec<&str> = rest.split('/').filter(|s| !s.is_empty()).collect();
    let query: Vec<(String, String)> = form_urlencoded::parse(req.uri().query().unwrap_or("").as_bytes()).into_owned().collect();
    let body = if method == Method::POST {
        match read_body(&front, req.into_body()).await {
            Ok(b) => b,
            Err(r) => return r,
        }
    } else {
        Bytes::new()
    };
    match (method, segments.as_slice()) {
        (Method::GET, ["me"]) => {
            let (administer, permissions) = front
                .policy
                .read()
                .map(|p| {
                    let held: Vec<&str> = policy::CATALOG.iter().map(|x| x.key).filter(|k| policy::holds(&p, &subject_of(&caller), k)).collect();
                    (held.contains(&"accounts.manage"), held)
                })
                .unwrap_or((false, Vec::new()));
            ok(json!({"account": caller, "session": crate::sessions::token_id(&token), "administer": administer, "permissions": permissions}))
        }
        (Method::POST, ["me", "password"]) => {
            let input: OwnPasswordIn = match decode(&body) { Ok(i) => i, Err(r) => return r };
            let f = Arc::clone(&front);
            let subject = caller.subject.clone();
            let outcome = blocking(move || -> Result<String, String> {
                let ctx = Context::new();
                f.accounts.verify(&ctx, &subject, input.current.as_bytes()).map_err(|e| e.code().to_owned())?;
                let params = f.config.read().map_err(|_| "authority-state".to_owned())?.argon2;
                f.accounts.set_password(&ctx, &subject, input.new.as_bytes(), params).map_err(|e| e.code().to_owned())?;
                let _ = f.sessions.revoke_subject(&subject);
                f.sessions.login(&ctx, &f.accounts, &subject, input.new.as_bytes(), now()).map(|(t, _)| t).map_err(|e| e.code().to_owned())
            })
            .await;
            match outcome {
                Some(Ok(new_token)) => {
                    let c = cookie::Cookie::build((COOKIE, new_token)).path("/").http_only(true).same_site(cookie::SameSite::Strict).build();
                    let mut r = ok(json!({"changed": true}));
                    if let Ok(v) = HeaderValue::from_str(&c.to_string()) {
                        r.headers_mut().insert(header::SET_COOKIE, v);
                    }
                    r
                }
                Some(Err(code)) => account_refusal(&code),
                None => refuse(StatusCode::INTERNAL_SERVER_ERROR, "authority-state"),
            }
        }
        (Method::GET, ["me", "keys"]) => {
            let (f, subject) = (Arc::clone(&front), caller.subject.clone());
            match blocking(move || f.accounts.keys(&Context::new(), Some(&subject)).map_err(|e| e.code())).await {
                Some(Ok(keys)) => ok(json!({"keys": keys, "now": now()})),
                Some(Err(c)) => account_refusal(c),
                None => refuse(StatusCode::INTERNAL_SERVER_ERROR, "authority-state"),
            }
        }
        (Method::POST, ["me", "keys"]) => {
            let input: KeyIn = match decode(&body) { Ok(i) => i, Err(r) => return r };
            if input.expires_in_seconds < 1 {
                return refuse(StatusCode::BAD_REQUEST, "key-expiry-invalid");
            }
            let (f, subject) = (Arc::clone(&front), caller.subject.clone());
            let at = now();
            let Some(expires) = at.checked_add(input.expires_in_seconds) else { return refuse(StatusCode::BAD_REQUEST, "key-expiry-invalid") };
            match blocking(move || f.accounts.create_key(&Context::new(), &subject, &input.name, input.scope, at, expires).map_err(|e| e.code())).await {
                Some(Ok((secret, key))) => ok(json!({"secret": secret, "key": key})),
                Some(Err(c)) => account_refusal(c),
                None => refuse(StatusCode::INTERNAL_SERVER_ERROR, "authority-state"),
            }
        }
        (Method::POST, ["me", "keys", id, "revoke"]) | (Method::POST, ["keys", id, "revoke"]) => {
            let own = segments[0] == "me";
            let (f, id, subject) = (Arc::clone(&front), id.to_string(), caller.subject.clone());
            match blocking(move || f.accounts.revoke_key(&Context::new(), &id, own.then_some(subject.as_str())).map_err(|e| e.code())).await {
                Some(Ok(())) => ok(json!({"revoked": true})),
                Some(Err(c)) => account_refusal(c),
                None => refuse(StatusCode::INTERNAL_SERVER_ERROR, "authority-state"),
            }
        }
        (Method::GET, ["keys"]) => {
            let f = Arc::clone(&front);
            match blocking(move || f.accounts.keys(&Context::new(), None).map_err(|e| e.code())).await {
                Some(Ok(keys)) => ok(json!({"keys": keys, "now": now()})),
                Some(Err(c)) => account_refusal(c),
                None => refuse(StatusCode::INTERNAL_SERVER_ERROR, "authority-state"),
            }
        }
        (Method::GET, ["accounts"]) => {
            let f = Arc::clone(&front);
            match blocking(move || f.accounts.list(&Context::new()).map_err(|e| e.code())).await {
                Some(Ok(list)) => {
                    let pending = front.reset_requests.pending(&list);
                    ok(json!({"accounts": list, "reset_requests": pending}))
                }
                Some(Err(c)) => account_refusal(c),
                None => refuse(StatusCode::INTERNAL_SERVER_ERROR, "authority-state"),
            }
        }
        (Method::POST, ["accounts"]) => {
            let input: CreateIn = match decode(&body) { Ok(i) => i, Err(r) => return r };
            if let Err(r) = known_roles(&front, &input.roles) { return r; }
            let f = Arc::clone(&front);
            let outcome = blocking(move || {
                let params = f.config.read().map_err(|_| "authority-state")?.argon2;
                let profile = Profile { display_name: input.display_name, roles: input.roles, integrity: input.integrity, attributes: input.attributes };
                f.accounts.create(&Context::new(), &input.subject, &profile, input.password.as_bytes(), params).map_err(|e| e.code())?;
                f.accounts.current(&Context::new(), &input.subject).map_err(|e| e.code())
            })
            .await;
            match outcome {
                Some(Ok(view)) => ok(json!({"account": view})),
                Some(Err(c)) => account_refusal(c),
                None => refuse(StatusCode::INTERNAL_SERVER_ERROR, "authority-state"),
            }
        }
        (Method::POST, ["accounts", subject, "profile"]) => {
            let input: ProfileIn = match decode(&body) { Ok(i) => i, Err(r) => return r };
            let profile: Profile = input.into();
            if let Err(r) = known_roles(&front, &profile.roles) { return r; }
            if *subject == caller.subject {
                let keeps = front.policy.read().map(|p| can_administer(&p, &profile.roles, profile.integrity, &profile.attributes)).unwrap_or(false);
                if !keeps {
                    return refuse(StatusCode::CONFLICT, "authority-self-lockout");
                }
            }
            let (f, s) = (Arc::clone(&front), subject.to_string());
            let outcome = blocking(move || {
                f.accounts.update_profile(&Context::new(), &s, &profile).map_err(|e| e.code())?;
                f.accounts.current(&Context::new(), &s).map_err(|e| e.code())
            })
            .await;
            match outcome {
                Some(Ok(view)) => ok(json!({"account": view})),
                Some(Err(c)) => account_refusal(c),
                None => refuse(StatusCode::INTERNAL_SERVER_ERROR, "authority-state"),
            }
        }
        (Method::POST, ["accounts", subject, "password"]) => {
            let input: PasswordIn = match decode(&body) { Ok(i) => i, Err(r) => return r };
            let (f, s) = (Arc::clone(&front), subject.to_string());
            let outcome = blocking(move || {
                let params = f.config.read().map_err(|_| "authority-state")?.argon2;
                f.accounts.set_password(&Context::new(), &s, input.password.as_bytes(), params).map_err(|e| e.code())?;
                // The generation changed: every session of `s` would refuse at
                // its next request; drop them now so the list is truthful.
                f.sessions.revoke_subject(&s).map_err(|e| e.code())
            })
            .await;
            match outcome {
                Some(Ok(ended)) => ok(json!({"changed": true, "sessions_ended": ended})),
                Some(Err(c)) => account_refusal(c),
                None => refuse(StatusCode::INTERNAL_SERVER_ERROR, "authority-state"),
            }
        }
        (Method::POST, ["accounts", subject, "disable"]) => {
            if *subject == caller.subject {
                return refuse(StatusCode::CONFLICT, "authority-self-lockout");
            }
            let (f, s) = (Arc::clone(&front), subject.to_string());
            let outcome = blocking(move || {
                f.accounts.disable(&Context::new(), &s).map_err(|e| e.code())?;
                f.sessions.revoke_subject(&s).map_err(|e| e.code())
            })
            .await;
            match outcome {
                Some(Ok(ended)) => ok(json!({"disabled": true, "sessions_ended": ended})),
                Some(Err(c)) => account_refusal(c),
                None => refuse(StatusCode::INTERNAL_SERVER_ERROR, "authority-state"),
            }
        }
        (Method::GET, ["sessions"]) => match front.sessions.list(now()) {
            Ok(list) => ok(json!({"sessions": list, "current": crate::sessions::token_id(&token)})),
            Err(e) => refuse(StatusCode::INTERNAL_SERVER_ERROR, e.code()),
        },
        (Method::POST, ["sessions", id, "revoke"]) => match front.sessions.revoke_id(id) {
            Ok(true) => ok(json!({"revoked": true, "current": *id == crate::sessions::token_id(&token)})),
            Ok(false) => refuse(StatusCode::NOT_FOUND, "authority-session-missing"),
            Err(e) => refuse(StatusCode::INTERNAL_SERVER_ERROR, e.code()),
        },
        (Method::GET, ["policy"]) => match front.policy.read() {
            Ok(p) => ok(json!({"policy": *p, "revision": revision(&*p), "source": source(&front.dir, "policy.json"),
                "catalog": policy::CATALOG, "builtin_roles": policy::BUILTIN_ROLES, "floors": {
                "read": policy::integrity_floor(Action::Read), "write": policy::integrity_floor(Action::Write),
                "approve": policy::integrity_floor(Action::Approve), "administer": policy::integrity_floor(Action::Administer)}})),
            Err(_) => refuse(StatusCode::INTERNAL_SERVER_ERROR, "authority-state"),
        },
        (Method::POST, ["policy", "preview"]) => {
            let input: PolicyPreviewIn = match decode(&body) { Ok(i) => i, Err(r) => return r };
            let candidate = match serde_json::to_vec(&input.policy).map_err(|_| "authority-policy-format").and_then(|raw| Policy::decode(&raw)) {
                Ok(p) => p,
                Err(code) => return refuse(StatusCode::BAD_REQUEST, code),
            };
            let subject = Subject { roles: &input.roles, integrity: input.integrity, attributes: &input.attributes };
            let current = match front.policy.read() { Ok(p) => policy::decide(&p, &subject, &input.method, &input.path), Err(_) => return refuse(StatusCode::INTERNAL_SERVER_ERROR, "authority-state") };
            let proposed = policy::decide(&candidate, &subject, &input.method, &input.path);
            ok(json!({"valid": true, "revision": revision(&candidate), "current": current, "proposed": proposed,
                "keeps_your_administration": can_administer(&candidate, &caller.roles, caller.integrity, &caller.attributes)}))
        }
        (Method::POST, ["policy"]) => {
            let input: PolicyApplyIn = match decode(&body) { Ok(i) => i, Err(r) => return r };
            let candidate = match serde_json::to_vec(&input.policy).map_err(|_| "authority-policy-format").and_then(|raw| Policy::decode(&raw)) {
                Ok(p) => p,
                Err(code) => return refuse(StatusCode::BAD_REQUEST, code),
            };
            if input.revision != front.policy.read().map(|p| revision(&*p)).unwrap_or_default() {
                return refuse(StatusCode::CONFLICT, "authority-revision-stale");
            }
            if !can_administer(&candidate, &caller.roles, caller.integrity, &caller.attributes) {
                return refuse(StatusCode::CONFLICT, "authority-self-lockout");
            }
            // A role still held by an account cannot disappear.
            let f = Arc::clone(&front);
            let accounts = match blocking(move || f.accounts.list(&Context::new()).map_err(|e| e.code())).await {
                Some(Ok(a)) => a,
                Some(Err(c)) => return account_refusal(c),
                None => return refuse(StatusCode::INTERNAL_SERVER_ERROR, "authority-state"),
            };
            let Ok(mut live) = front.policy.write() else { return refuse(StatusCode::INTERNAL_SERVER_ERROR, "authority-state") };
            if input.revision != revision(&*live) {
                return refuse(StatusCode::CONFLICT, "authority-revision-stale");
            }
            if let Some(a) = accounts.iter().find(|a| a.roles.iter().any(|r| !candidate.has_role(r) && live.has_role(r))) {
                return reply(StatusCode::CONFLICT, json!({"error": "authority-role-in-use", "subject": a.subject}));
            }
            let bytes = serde_json::to_vec_pretty(&candidate).unwrap_or_default();
            if let Err(code) = persist(&front.dir, "policy.json", &bytes) {
                return refuse(StatusCode::INTERNAL_SERVER_ERROR, code);
            }
            *live = candidate;
            ok(json!({"applied": true, "revision": revision(&*live)}))
        }
        (Method::GET, ["audit", "sessions"]) | (Method::GET, ["audit", "dashboard"]) => {
            let (from, to) = (q_i64(&query, "from").unwrap_or(0), q_i64(&query, "to").unwrap_or(i64::MAX));
            let subject = query.iter().find(|(k, _)| k == "subject").map(|(_, v)| v.as_str()).filter(|v| !v.is_empty());
            match front.audit.read() {
                Ok((entries, chain_valid)) if segments[1] == "sessions" => ok(json!({"sessions": crate::audit::sessions(&entries, from, to, subject), "chain_valid": chain_valid})),
                Ok((entries, chain_valid)) => {
                    let (days, accounts) = crate::audit::dashboard(&entries, from, to);
                    ok(json!({"days": days, "accounts": accounts, "chain_valid": chain_valid}))
                }
                Err(code) => refuse(StatusCode::INTERNAL_SERVER_ERROR, code),
            }
        }
        (Method::GET, ["audit", "sessions", credential]) => match front.audit.read() {
            Ok((entries, _)) => {
                let lines: Vec<_> = entries.into_iter().filter(|e| e.credential.as_deref() == Some(*credential)).collect();
                ok(json!({"credential": credential, "entries": lines}))
            }
            Err(code) => refuse(StatusCode::INTERNAL_SERVER_ERROR, code),
        },
        (Method::POST, ["audit", "purge"]) => {
            let input: PurgeIn = match decode(&body) { Ok(i) => i, Err(r) => return r };
            if input.older_than_days < 1 {
                return refuse(StatusCode::BAD_REQUEST, "authority-purge-days-invalid");
            }
            let at = now();
            let Some(before) = input.older_than_days.checked_mul(86_400).and_then(|d| at.checked_sub(d)) else {
                return refuse(StatusCode::BAD_REQUEST, "authority-purge-days-invalid");
            };
            let (f, actor) = (Arc::clone(&front), caller.subject.clone());
            match blocking(move || f.audit.purge(before, &actor, at)).await {
                Some(Ok(removed)) => ok(json!({"removed": removed, "before": before})),
                Some(Err(code @ "authority-audit-chain-broken")) => refuse(StatusCode::CONFLICT, code),
                Some(Err(code)) => refuse(StatusCode::INTERNAL_SERVER_ERROR, code),
                None => refuse(StatusCode::INTERNAL_SERVER_ERROR, "authority-state"),
            }
        }
        (Method::GET, ["audit"]) => match front.audit.read() {
            Ok((entries, chain_valid)) => ok(json!({"entries": entries, "chain_valid": chain_valid})),
            Err(code) => refuse(StatusCode::INTERNAL_SERVER_ERROR, code),
        },
        (Method::GET, ["config"]) => match front.config.read() {
            Ok(c) => ok(json!({"config": *c, "revision": revision(&*c), "source": source(&front.dir, "authority.json")})),
            Err(_) => refuse(StatusCode::INTERNAL_SERVER_ERROR, "authority-state"),
        },
        (Method::POST, ["config", "preview"]) | (Method::POST, ["config"]) => {
            let apply = segments.len() == 1;
            let input: ConfigIn = match decode(&body) { Ok(i) => i, Err(r) => return r };
            let candidate = match serde_json::to_vec(&input.config).map_err(|_| "authority-config-format").and_then(|raw| AuthorityConfig::decode(&raw)) {
                Ok(c) => c,
                Err(code) => return refuse(StatusCode::BAD_REQUEST, code),
            };
            if !apply {
                return ok(json!({"valid": true, "revision": revision(&candidate)}));
            }
            let Ok(mut live) = front.config.write() else { return refuse(StatusCode::INTERNAL_SERVER_ERROR, "authority-state") };
            if input.revision.as_deref() != Some(revision(&*live).as_str()) {
                return refuse(StatusCode::CONFLICT, "authority-revision-stale");
            }
            let bytes = serde_json::to_vec_pretty(&candidate).unwrap_or_default();
            if let Err(code) = persist(&front.dir, "authority.json", &bytes) {
                return refuse(StatusCode::INTERNAL_SERVER_ERROR, code);
            }
            if front.sessions.set_config(candidate).is_err() {
                return refuse(StatusCode::INTERNAL_SERVER_ERROR, "authority-state");
            }
            *live = candidate;
            ok(json!({"applied": true, "revision": revision(&*live)}))
        }
        _ => refuse(StatusCode::NOT_FOUND, "authority-route-unknown"),
    }
}
