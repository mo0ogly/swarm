//! Multi-account registry on the signed enrollment journal (phase B, slice B2.2).
//!
//! Each account generation is one journal record `account/<subject>/<nonce>`
//! whose body is an [`AccountBody`](crate::accounts::AccountBody) holding attributes (roles, integrity,
//! free attributes) and an Argon2id verifier (`credentials::Record`); the
//! password is never stored. A password change enrolls a new generation and
//! then revokes the old one (a revoked id never comes back). Reads pick the
//! highest active generation, so an interruption between the two writes
//! leaves the new password effective; the stale generation is revoked by the
//! next write on that subject.
//!
//! Limits: no rollback witness (the journal head is not compared with an
//! independent copy — EP-06 stays open). Journal calls of this process are
//! serialized (the journal has no busy wait by contract). Sessions live in
//! [`crate::sessions`], API keys in [`crate::keys`] (same journal, `apikey/`
//! records). Argon2 costs come from [`crate::config::AuthorityConfig`].
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use swarm_enrollment_authority::credentials::{self, Parameters, Record};
use swarm_enrollment_state::{sign_change, Context, Head, Journal};
use swarm_identity_contract::Principal;

/// Issuer of locally held human principals (distinct from `urn:swarm:local`).
pub const ISSUER: &str = "urn:swarm:authority:local";

/// Persisted body of one account generation. `deny_unknown_fields`: a body
/// written by a newer schema refuses instead of being half-read.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccountBody {
    pub schema_version: i64,
    pub subject: String,
    pub display_name: String,
    pub roles: Vec<String>,
    pub integrity: i64,
    pub attributes: BTreeMap<String, String>,
    pub generation: i64,
    pub credential: Record,
}

/// Account attributes without verifier material; safe to display.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AccountView {
    pub subject: String,
    pub display_name: String,
    pub roles: Vec<String>,
    pub integrity: i64,
    pub attributes: BTreeMap<String, String>,
    pub generation: i64,
}

/// Attributes for a new account or a profile change.
#[derive(Clone, Debug)]
pub struct Profile {
    pub display_name: String,
    pub roles: Vec<String>,
    pub integrity: i64,
    pub attributes: BTreeMap<String, String>,
}

/// Typed refusal; `code()` is stable and never contains a password or subject.
#[derive(Debug)]
pub enum Error {
    Journal(swarm_enrollment_state::Error),
    Credential(credentials::Error),
    Body(serde_json::Error),
    Entropy,
    Refused(&'static str),
}
impl Error {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Journal(e) => e.code(),
            Self::Credential(_) => "account-credential",
            Self::Body(_) => "account-body-invalid",
            Self::Entropy => "account-entropy",
            Self::Refused(code) => code,
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}

/// Registry bound to one journal, its signing key and installation identity.
pub struct Accounts {
    pub(crate) journal: Journal,
    pub(crate) key: SigningKey,
    pub(crate) project: [u8; 32],
    installation: String,
    /// Serializes this process's journal calls: the journal takes a write
    /// reservation per call and has no busy wait by contract, so concurrent
    /// browser requests would otherwise refuse with `sqlite` busy.
    serial: std::sync::Mutex<()>,
}

fn principal(subject: &str) -> Principal {
    Principal { id: subject.into(), kind: "human".into(), issuer: ISSUER.into(), subject: subject.into() }
}

/// Structural checks shared by create/update: subject is a contract `name`,
/// display name non-empty without control characters, roles non-empty,
/// unique contract names (their existence in the access policy is checked
/// by the callers that know it), integrity 1..=3, attribute keys contract names.
fn validate(subject: &str, p: &Profile) -> Result<(), Error> {
    if !swarm_identity_contract::name(subject.as_bytes()) {
        return Err(Error::Refused("account-subject-invalid"));
    }
    if p.display_name.trim().is_empty() || p.display_name.chars().any(char::is_control) {
        return Err(Error::Refused("account-display-name-invalid"));
    }
    let mut seen = std::collections::BTreeSet::new();
    if p.roles.is_empty() || p.roles.iter().any(|r| !swarm_identity_contract::name(r.as_bytes()) || !seen.insert(r)) {
        return Err(Error::Refused("account-roles-invalid"));
    }
    if !(1..=3).contains(&p.integrity) {
        return Err(Error::Refused("account-integrity-invalid"));
    }
    if p.attributes.iter().any(|(k, v)| !swarm_identity_contract::name(k.as_bytes()) || v.chars().any(char::is_control)) {
        return Err(Error::Refused("account-attributes-invalid"));
    }
    Ok(())
}

impl Accounts {
    fn serial(&self) -> Result<std::sync::MutexGuard<'_, ()>, Error> {
        self.serial.lock().map_err(|_| Error::Refused("account-state"))
    }
    pub(crate) fn serial_snapshot(&self, ctx: &Context) -> Result<(Head, Vec<String>), swarm_enrollment_state::Error> {
        let _g = self.serial().map_err(|_| swarm_enrollment_state::Error::State)?;
        self.journal.snapshot(ctx)
    }
    pub(crate) fn serial_active<T>(&self, ctx: &Context, head: &Head, id: &str, f: impl FnOnce(swarm_enrollment_state::Record) -> Result<T, swarm_enrollment_state::Error>) -> Result<T, swarm_enrollment_state::Error> {
        let _g = self.serial().map_err(|_| swarm_enrollment_state::Error::State)?;
        self.journal.with_active(ctx, head, id, f)
    }
    pub(crate) fn serial_apply(&self, ctx: &Context, head: &Head, raw: &[u8]) -> Result<Head, swarm_enrollment_state::Error> {
        let _g = self.serial().map_err(|_| swarm_enrollment_state::Error::State)?;
        self.journal.apply(ctx, head, raw)
    }

    /// Open (or create) the journal at `path`, verified with the public half
    /// of `key`. `installation` is the 64-hex installation digest bound into
    /// every verifier. No account is created. Journal errors stay typed.
    pub fn open(
        ctx: &Context,
        path: &std::path::Path,
        key: SigningKey,
        project: [u8; 32],
        installation: &str,
    ) -> Result<Self, Error> {
        let journal = Journal::open(ctx, path, &key.verifying_key().to_bytes(), project).map_err(Error::Journal)?;
        Ok(Self { journal, key, project, installation: installation.into(), serial: std::sync::Mutex::new(()) })
    }

    /// Active generations of `subject`, highest generation first, with the head.
    fn generations(&self, ctx: &Context, subject: &str) -> Result<(Head, Vec<(String, AccountBody)>), Error> {
        let (head, ids) = self.serial_snapshot(ctx).map_err(Error::Journal)?;
        let prefix = format!("account/{subject}/");
        let mut out = Vec::new();
        for id in ids.into_iter().filter(|id| id.starts_with(&prefix)) {
            let body = self.serial_active(ctx, &head, &id, |r| Ok(r.body)).map_err(Error::Journal)?;
            let parsed: AccountBody = serde_json::from_slice(&body).map_err(Error::Body)?;
            if parsed.subject != subject || parsed.schema_version != 1 {
                return Err(Error::Refused("account-body-mismatch"));
            }
            out.push((id, parsed));
        }
        out.sort_by(|a, b| b.1.generation.cmp(&a.1.generation));
        Ok((head, out))
    }

    /// Enroll one generation, then revoke every older active one (stale
    /// generations from an interrupted change included). Returns the new head.
    fn write(&self, ctx: &Context, subject: &str, profile: &Profile, credential: Record, generation: i64, mut head: Head, stale: Vec<String>) -> Result<Head, Error> {
        let mut nonce = [0u8; 8];
        getrandom::fill(&mut nonce).map_err(|_| Error::Entropy)?;
        let id = format!("account/{subject}/{}", nonce.iter().map(|b| format!("{b:02x}")).collect::<String>());
        let body = serde_json::to_vec(&AccountBody {
            schema_version: 1,
            subject: subject.into(),
            display_name: profile.display_name.clone(),
            roles: profile.roles.clone(),
            integrity: profile.integrity,
            attributes: profile.attributes.clone(),
            generation,
            credential,
        })
        .map_err(Error::Body)?;
        let raw = sign_change(&self.key, self.project, &head, "enroll", &id, &body).map_err(Error::Journal)?;
        head = self.serial_apply(ctx, &head, &raw).map_err(Error::Journal)?;
        for old in stale {
            let raw = sign_change(&self.key, self.project, &head, "revoke", &old, b"").map_err(Error::Journal)?;
            head = self.serial_apply(ctx, &head, &raw).map_err(Error::Journal)?;
        }
        Ok(head)
    }

    /// Create `subject` with `password` (Argon2id with `params`). Refuses an
    /// already active subject (`account-exists`), invalid profile/password.
    pub fn create(&self, ctx: &Context, subject: &str, profile: &Profile, password: &[u8], params: Parameters) -> Result<Head, Error> {
        validate(subject, profile)?;
        let (head, current) = self.generations(ctx, subject)?;
        if !current.is_empty() {
            return Err(Error::Refused("account-exists"));
        }
        let credential = credentials::create(&self.installation, &principal(subject), 1, params, password).map_err(Error::Credential)?;
        self.write(ctx, subject, profile, credential, 1, head, Vec::new())
    }

    /// Check `password` for `subject`. Unknown subject and wrong password both
    /// refuse with `account-authentication-refused` (no account oracle). The
    /// returned view proves only knowledge of the current secret at this call.
    pub fn verify(&self, ctx: &Context, subject: &str, password: &[u8]) -> Result<AccountView, Error> {
        let refused = || Error::Refused("account-authentication-refused");
        if !swarm_identity_contract::name(subject.as_bytes()) {
            return Err(refused());
        }
        let (_, current) = self.generations(ctx, subject)?;
        let Some((_, body)) = current.into_iter().next() else { return Err(refused()) };
        let pin = credentials::fingerprint(&body.credential).map_err(Error::Credential)?;
        match credentials::verify(&self.installation, &principal(subject), body.generation, &pin, &body.credential, password) {
            Ok(_) => Ok(view(&body)),
            Err(credentials::Error::Password) => Err(refused()),
            Err(e) => Err(Error::Credential(e)),
        }
    }

    /// Current active generation of `subject` without checking any secret
    /// (`None` when disabled or unknown). Used by sessions to re-check an
    /// account at every request. Journal/body errors stay typed.
    pub fn current(&self, ctx: &Context, subject: &str) -> Result<Option<AccountView>, Error> {
        if !swarm_identity_contract::name(subject.as_bytes()) {
            return Ok(None);
        }
        Ok(self.generations(ctx, subject)?.1.first().map(|(_, b)| view(b)))
    }

    /// Replace the password: new generation with the same profile, then the
    /// old generation(s) revoked. Refuses an unknown subject.
    pub fn set_password(&self, ctx: &Context, subject: &str, password: &[u8], params: Parameters) -> Result<Head, Error> {
        let (head, current) = self.generations(ctx, subject)?;
        let Some((_, body)) = current.first() else { return Err(Error::Refused("account-missing")) };
        let profile = Profile { display_name: body.display_name.clone(), roles: body.roles.clone(), integrity: body.integrity, attributes: body.attributes.clone() };
        let generation = body.generation.checked_add(1).ok_or(Error::Refused("account-generation-exhausted"))?;
        let credential = credentials::create(&self.installation, &principal(subject), generation, params, password).map_err(Error::Credential)?;
        let stale = current.into_iter().map(|(id, _)| id).collect();
        self.write(ctx, subject, &profile, credential, generation, head, stale)
    }

    /// Replace the profile (display name, roles, integrity, attributes) of
    /// `subject`, keeping its password verifier and generation: open
    /// sessions survive and see the new attributes at their next request
    /// (sessions re-read the account each time). Refuses an unknown subject
    /// (`account-missing`) or an invalid profile (same checks as `create`).
    pub fn update_profile(&self, ctx: &Context, subject: &str, profile: &Profile) -> Result<Head, Error> {
        validate(subject, profile)?;
        let (head, current) = self.generations(ctx, subject)?;
        let Some((_, body)) = current.first() else { return Err(Error::Refused("account-missing")) };
        let (credential, generation) = (body.credential.clone(), body.generation);
        let stale = current.into_iter().map(|(id, _)| id).collect();
        self.write(ctx, subject, profile, credential, generation, head, stale)
    }

    /// Disable `subject`: revoke every active generation. Its history stays
    /// in the journal; a later `create` starts a new lineage.
    pub fn disable(&self, ctx: &Context, subject: &str) -> Result<Head, Error> {
        let (mut head, current) = self.generations(ctx, subject)?;
        if current.is_empty() {
            return Err(Error::Refused("account-missing"));
        }
        for (id, _) in current {
            let raw = sign_change(&self.key, self.project, &head, "revoke", &id, b"").map_err(Error::Journal)?;
            head = self.serial_apply(ctx, &head, &raw).map_err(Error::Journal)?;
        }
        Ok(head)
    }

    /// All active accounts (current generation each), sorted by subject;
    /// verifier material is excluded.
    pub fn list(&self, ctx: &Context) -> Result<Vec<AccountView>, Error> {
        let (_, ids) = self.serial_snapshot(ctx).map_err(Error::Journal)?;
        let mut subjects: Vec<String> = ids
            .iter()
            .filter_map(|id| id.strip_prefix("account/").and_then(|r| r.split('/').next()).map(str::to_owned))
            .collect();
        subjects.dedup();
        let mut out = Vec::new();
        for subject in subjects {
            if let Some((_, body)) = self.generations(ctx, &subject)?.1.into_iter().next() {
                out.push(view(&body));
            }
        }
        Ok(out)
    }

    /// Current verified journal head (sequence, digest): the account part of
    /// the rollback witness carried by assertions (EP-06).
    pub fn head(&self, ctx: &Context) -> Result<Head, Error> {
        Ok(self.serial_snapshot(ctx).map_err(Error::Journal)?.0)
    }

    /// Close the journal; later calls refuse with the journal's `closed` code.
    pub fn close(&self) -> Result<(), Error> {
        self.journal.close().map_err(Error::Journal)
    }
}

fn view(b: &AccountBody) -> AccountView {
    AccountView {
        subject: b.subject.clone(),
        display_name: b.display_name.clone(),
        roles: b.roles.clone(),
        integrity: b.integrity,
        attributes: b.attributes.clone(),
        generation: b.generation,
    }
}
