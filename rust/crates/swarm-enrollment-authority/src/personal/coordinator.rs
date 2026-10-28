//! Durable personal enrollment intent/checkpoint coordinator. Protected startup
//! supplies Source and a dedicated private witness directory. No first-caller
//! adoption, service endpoint, human presence, session or OS confinement.
use super::{credentials, Prepared, Source, ACCOUNT_ID, BOOTSTRAP_ID};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use std::{error::Error as StdError, fmt};
use swarm_enrollment_config::{
    self as config,
    store::{
        self,
        documents::{Documents, Publication, WriteError},
    },
};
use swarm_enrollment_state::{self as state, Head};
use swarm_identity_contract::{digest, Principal};
use swarm_wire_json as wire;
mod commit;
const FILE: &str = "personal-checkpoint.json";
/// Caller-owned paired cancellation, no timer/deadline/default. cancel marks both
/// existing journal/file contexts; it does not preempt IO or undo external effects.
#[derive(Clone, Default)]
pub struct Context {
    journal: state::Context,
    files: store::Context,
}
impl Context {
    /// Fresh uncanceled operation context; starts no background work or watchdog.
    pub fn new() -> Self {
        Self::default()
    }
    /// Cancel both consumers. In-flight effects require the returned observations.
    pub fn cancel(&self) {
        self.journal.cancel();
        self.files.cancel()
    }
}
/// Data from a protected checkpoint, never a grant, session or proof of a human.
#[derive(Clone, PartialEq, Eq)]
pub struct Account {
    head: Head,
    credential_digest: String,
}
impl Account {
    /// Exact recorded head. Current journal authority must still be checked.
    pub fn head(&self) -> &Head {
        &self.head
    }
    /// Private verifier pin; never export as a bearer token or infer authority.
    pub fn credential_digest(&self) -> &str {
        &self.credential_digest
    }
}
impl fmt::Debug for Account {
    /// Ordinary diagnostics omit private identity/checkpoint/verifier material.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PersonalAccount { <private checkpoint> }")
    }
}
/// Actual journal attempt/acknowledgement, independent of checkpoint durability.
pub enum JournalOutcome {
    NotAttempted,
    Unknown,
    Acknowledged(Head),
    Observed(Head),
}
impl fmt::Debug for JournalOutcome {
    /// Bounded phase only; no private head or digest is formatted.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotAttempted => "NotAttempted",
            Self::Unknown => "Unknown",
            Self::Acknowledged(_) => "Acknowledged",
            Self::Observed(_) => "Observed",
        })
    }
}
/// Separate observations survive every error. A Durable intent does not imply
/// journal consumption, and journal acknowledgement does not imply confirmation.
#[derive(Debug)]
pub struct Progress {
    pub intent: Publication,
    pub journal: JournalOutcome,
    pub checkpoint: Publication,
}
impl Default for Progress {
    /// No IO/journal attempt inferred before the operation actually starts.
    fn default() -> Self {
        Self {
            intent: Publication::NotAttempted,
            journal: JournalOutcome::NotAttempted,
            checkpoint: Publication::NotAttempted,
        }
    }
}
/// Safe refusal with original structured causes. Joined retains secondary release
/// or journal-completion failure; ordinary diagnostics contain no private data.
pub enum Error {
    Checkpoint,
    Pending,
    NoPending,
    Missing,
    Exists,
    Interrupted,
    Internal,
    Storage(config::Error),
    Write(WriteError),
    Personal(super::Error),
    State(state::Error),
    Frame(serde_json::Error),
    Encoding(base64::DecodeError),
    Joined {
        cause: Box<Error>,
        cleanup: Box<Error>,
    },
}
impl Error {
    /// Stable category, never raw checkpoint, path, password or callback payload.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Checkpoint => "personal-checkpoint",
            Self::Pending => "personal-pending",
            Self::NoPending => "personal-no-pending",
            Self::Missing => "personal-checkpoint-missing",
            Self::Exists => "personal-account-exists",
            Self::Interrupted => "personal-interrupted",
            Self::Internal => "personal-coordinator-state",
            Self::Storage(e) => e.code(),
            Self::Write(e) => e.cause.code(),
            Self::Personal(e) => e.code(),
            Self::State(e) => e.code(),
            Self::Frame(_) => "personal-checkpoint-json",
            Self::Encoding(_) => "personal-checkpoint-encoding",
            Self::Joined { cause, .. } => cause.code(),
        }
    }
    /// Independent secondary release/authority cause; also traverse source().
    pub fn cleanup_error(&self) -> Option<&Error> {
        match self {
            Self::Joined { cleanup, .. } => Some(cleanup),
            _ => None,
        }
    }
}
impl fmt::Display for Error {
    /// Fixed redacted category; typed originals remain available to trusted code.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "personal coordinator: {}", self.code())
    }
}
impl fmt::Debug for Error {
    /// No nested private framing/storage/callback data in Debug.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Error {
    /// Preserve typed primary cause; cleanup_error is an additional required branch.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Storage(e) => Some(e),
            Self::Write(e) => Some(e),
            Self::Personal(e) => Some(e),
            Self::State(e) => Some(e),
            Self::Frame(e) => Some(e),
            Self::Encoding(e) => Some(e),
            Self::Joined { cause, .. } => Some(cause.as_ref()),
            _ => None,
        }
    }
}
/// Failed enrollment/initialization with actual publication observations. Err is
/// never proof of an absent account or absent checkpoint. Keep exact intent for
/// explicit recovery; never retry with a new credential after unknown application.
pub struct CommitError {
    pub cause: Error,
    pub progress: Progress,
}
impl fmt::Display for CommitError {
    /// Safe phase/category only; no private head, password or verifier is printed.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}; {:?}", self.cause, self.progress)
    }
}
impl fmt::Debug for CommitError {
    /// Same redacted representation as Display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for CommitError {
    /// Primary refusal/cleanup tree; progress is separate mandatory observation.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.cause)
    }
}
/// Authentication refusal or authority failure after use. AfterUse retains the
/// entire successful/failed callback result and never claims its effect rolled back.
pub enum UseError<T, E> {
    Before(Error),
    AfterUse {
        cause: state::Error,
        outcome: Result<T, E>,
    },
}
impl<T, E> fmt::Display for UseError<T, E> {
    /// Never format private business outcomes, including errors returned by caller.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Before(e) => fmt::Display::fmt(e, f),
            Self::AfterUse { .. } => {
                f.write_str("personal coordinator: authority failed after use; effect UNKNOWN")
            }
        }
    }
}
impl<T, E> fmt::Debug for UseError<T, E> {
    /// No T/E Debug bounds or payload exposure.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl<T, E> StdError for UseError<T, E> {
    /// Initial/authority cause, with AfterUse.outcome retained as a separate branch.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Before(e) => Some(e),
            Self::AfterUse { cause, .. } => Some(cause),
        }
    }
}
/// Deterministic interruption points, absent from normal production APIs.
#[cfg(feature = "test-fixtures")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixturePhase {
    IntentDurable,
    JournalApplied,
    CheckpointDurable,
}
/// Trusted disposable-fixture hook only. It runs under the private-file
/// reservation and must not reenter/close the coordinator. Never a live operator hook.
#[cfg(feature = "test-fixtures")]
pub type FixtureHook = std::sync::Arc<dyn Fn(FixturePhase) -> Result<(), Error> + Send + Sync>;
/// Strict independent witness. Canonical optional fields are explicit nulls;
/// pending contains base64url exact signed proposal bytes, never a password.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Checkpoint {
    schema_version: i64,
    installation_digest: String,
    principal: Principal,
    generation: i64,
    authority_public_key: String,
    project_digest: String,
    head: Head,
    credential_digest: Option<String>,
    pending: Option<String>,
}
struct Loaded {
    checkpoint: Checkpoint,
    raw: Vec<u8>,
    prepared: Option<Prepared>,
}
/// Owns the protected journal and private witness. new never creates/adopts a
/// checkpoint or derives identity from storage. Root provisioning and independent
/// witness protection remain the trusted startup's responsibility.
pub struct Coordinator {
    source: Source,
    documents: Documents,
    #[cfg(feature = "test-fixtures")]
    hook: std::sync::Mutex<Option<FixtureHook>>,
}
/// Retain primary and secondary failure without discarding a successful outcome
/// externally: callers keep Progress before invoking this cleanup combination.
fn combined<T>(result: Result<T, Error>, cleanup: Result<(), Error>) -> Result<T, Error> {
    match (result, cleanup) {
        (r, Ok(())) => r,
        (Ok(_), Err(e)) => Err(e),
        (Err(cause), Err(cleanup)) => Err(Error::Joined {
            cause: Box::new(cause),
            cleanup: Box::new(cleanup),
        }),
    }
}
/// Canonical lowercase hexadecimal representation of an independently held key
/// domain; no hashing, root discovery, private data logging or policy default.
fn project(raw: &[u8; 32]) -> String {
    raw.iter().map(|b| format!("{b:02x}")).collect()
}
impl Coordinator {
    /// Bind already opened sources. Does no IO, provisioning, migration, service
    /// launch or implicit initialization. A missing/unsafe checkpoint still refuses.
    pub fn new(source: Source, documents: Documents) -> Self {
        Self {
            source,
            documents,
            #[cfg(feature = "test-fixtures")]
            hook: std::sync::Mutex::new(None),
        }
    }
    /// Construct only from independently supplied startup context, not file fields.
    fn bootstrap_checkpoint(&self, head: &Head) -> Checkpoint {
        Checkpoint {
            schema_version: 1,
            installation_digest: self.source.expected.installation_digest.clone(),
            principal: self.source.expected.principal.clone(),
            generation: self.source.expected.generation,
            authority_public_key: URL_SAFE_NO_PAD.encode(self.source.key),
            project_digest: project(&self.source.project),
            head: head.clone(),
            credential_digest: None,
            pending: None,
        }
    }
    /// Strict canonical closed checkpoint; validate independent context and signed
    /// pending shape without SQLite/current-head adoption. Missing data never resets.
    fn load(&self, ctx: &Context, r: &store::documents::Reservation<'_>) -> Result<Loaded, Error> {
        let raw = r.read(FILE).map_err(|e| {
            if matches!(e, config::Error::NotConfigured) {
                Error::Missing
            } else {
                Error::Storage(e)
            }
        })?;
        let c: Checkpoint = wire::decode(&raw).map_err(Error::Frame)?;
        if wire::encode(&c).map_err(Error::Frame)? != raw
            || c.schema_version != 1
            || c.installation_digest != self.source.expected.installation_digest
            || c.principal != self.source.expected.principal
            || c.generation != self.source.expected.generation
            || c.authority_public_key != URL_SAFE_NO_PAD.encode(self.source.key)
            || c.project_digest != project(&self.source.project)
            || !c.head.valid()
            || c.head.sequence <= 0
        {
            return Err(Error::Checkpoint);
        }
        if c.credential_digest
            .as_ref()
            .is_some_and(|p| !digest(p.as_bytes()))
            || (c.credential_digest.is_some() && c.pending.is_some())
        {
            return Err(Error::Checkpoint);
        }
        let prepared = match &c.pending {
            None => None,
            Some(encoded) => {
                let bytes = URL_SAFE_NO_PAD.decode(encoded).map_err(Error::Encoding)?;
                if URL_SAFE_NO_PAD.encode(&bytes) != *encoded {
                    return Err(Error::Checkpoint);
                }
                let p = self
                    .source
                    .restore_prepared(&ctx.journal, &bytes)
                    .map_err(Error::Personal)?;
                if p.base != c.head {
                    return Err(Error::Checkpoint);
                }
                Some(p)
            }
        };
        Ok(Loaded {
            checkpoint: c,
            raw,
            prepared,
        })
    }
    /// Extract data from an already validated committed witness. This is not a
    /// current authority observation; callers still check the real signed journal.
    fn account(c: &Checkpoint) -> Result<Account, Error> {
        if c.pending.is_some() {
            return Err(Error::Pending);
        }
        Ok(Account {
            head: c.head.clone(),
            credential_digest: c.credential_digest.clone().ok_or(Error::Checkpoint)?,
        })
    }
    /// Validate a current signed resource while retaining journal completion/rollback
    /// failures in addition to a private schema/binding refusal. No external effect.
    fn observe<T>(
        &self,
        ctx: &Context,
        head: &Head,
        id: &str,
        f: impl FnOnce(&state::Record) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let mut out = None;
        let mut f = Some(f);
        let result = self
            .source
            .journal
            .with_active(&ctx.journal, head, id, |record| {
                out = Some(match f.take() {
                    Some(f) => f(&record),
                    None => Err(Error::Internal),
                });
                Ok(())
            });
        match (result, out) {
            (Ok(()), Some(out)) => out,
            (Ok(()), None) => Err(Error::Internal),
            (Err(e), Some(Err(cause))) => combined(Err(cause), Err(Error::State(e))),
            (Err(e), _) => Err(Error::State(e)),
        }
    }
    /// Require the exact signed bootstrap at an independently supplied head. This
    /// validates context only, never authenticates a human or validates its secret.
    pub(crate) fn bootstrap_current(&self, ctx: &Context, head: &Head) -> Result<(), Error> {
        self.observe(ctx, head, BOOTSTRAP_ID, |r| {
            let b = super::decode_bootstrap(&r.body).map_err(Error::Personal)?;
            if b.installation_digest != self.source.expected.installation_digest
                || b.principal != self.source.expected.principal
                || b.generation != self.source.expected.generation
            {
                return Err(Error::Checkpoint);
            }
            Ok(())
        })
    }
    /// Require the exact active account/pin on the retained signed journal for
    /// recovery acknowledgement only. It establishes no password knowledge/session.
    fn account_current(&self, ctx: &Context, a: &Account) -> Result<(), Error> {
        self.observe(ctx, &a.head, ACCOUNT_ID, |r| {
            let record = credentials::decode(&r.body)
                .map_err(|e| Error::Personal(super::Error::Credential(e)))?;
            if record.installation_digest != self.source.expected.installation_digest
                || record.principal != self.source.expected.principal
                || record.generation != self.source.expected.generation
                || credentials::fingerprint(&record)
                    .map_err(|e| Error::Personal(super::Error::Credential(e)))?
                    != a.credential_digest
            {
                return Err(Error::Checkpoint);
            }
            Ok(())
        })
    }
    /// Read/validate and fsync committed witness, then release/check file cleanup
    /// BEFORE current password authority/callback. Pending/missing/refusals never
    /// invoke f. No file lock crosses a business effect. AfterUse preserves either
    /// callback outcome if later authority completion fails; no implicit retry.
    pub fn with_authenticated<T, E>(
        &self,
        ctx: &Context,
        password: &[u8],
        f: impl for<'r> FnOnce(&credentials::Proof<'r>) -> Result<T, E>,
    ) -> Result<Result<T, E>, UseError<T, E>> {
        let r = self
            .documents
            .reserve(&ctx.files)
            .map_err(|e| UseError::Before(Error::Storage(e)))?;
        let loaded = self.load(ctx, &r).and_then(|l| {
            let account = Self::account(&l.checkpoint)?;
            r.sync().map_err(Error::Storage)?;
            Ok(account)
        });
        let a = combined(loaded, r.finish().map_err(Error::Storage)).map_err(UseError::Before)?;
        self.source
            .with_authenticated(&ctx.journal, &a.head, &a.credential_digest, password, f)
            .map_err(|e| match e {
                super::UseError::Before(e) => UseError::Before(Error::Personal(e)),
                super::UseError::AfterUse { cause, outcome } => {
                    UseError::AfterUse { cause, outcome }
                }
            })
    }
    /// Explicitly close both owned sources and preserve both failures. A failure
    /// can leave one closed while another refuses Busy; retry close after the
    /// active operation ends, never infer restored data authority or reset history.
    pub fn close(&self) -> Result<(), Error> {
        let files = self.documents.close().map_err(Error::Storage);
        let journal = self.source.close().map_err(Error::Personal);
        combined(files, journal)
    }
    /// Set a disposable deterministic phase hook; absent from production profile.
    /// The caller must not change it during an operation or reenter from the hook.
    #[cfg(feature = "test-fixtures")]
    pub fn fixture_hook(&self, hook: Option<FixtureHook>) {
        *self
            .hook
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = hook
    }
    /// Invoke one test-only hook without keeping its mutex across callback; no
    /// production path, deadline, retry or interpretation of business decisions.
    #[cfg(feature = "test-fixtures")]
    fn phase(&self, p: FixturePhase) -> Result<(), Error> {
        let hook = self.hook.lock().map_err(|_| Error::Internal)?.clone();
        match hook {
            Some(f) => f(p),
            None => Ok(()),
        }
    }
}
