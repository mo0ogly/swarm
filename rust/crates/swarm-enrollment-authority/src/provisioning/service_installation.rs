//! Private initial-root publisher. Independent pinned service material only;
//! no operator business key/proof, account, OS identity, listener or confinement.
use super::service_artifact::{self, ServiceArtifact};
use crate::{authenticated_control, authenticated_startup, personal, startup};
use sha2::{Digest, Sha256};
use std::{
    error::Error as StdError,
    fmt,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use subtle::ConstantTimeEq;
use swarm_enrollment_bootstrap as bootstrap;
use swarm_enrollment_config::{
    self as config,
    store::{
        self,
        documents::{Documents, DurableReadError, Publication, WriteError},
    },
};
use swarm_enrollment_state::{self as state, Head, Journal};
use zeroize::Zeroizing;
mod filesystem;
/// Explicit caller-selected recovery, never a fallback from a failed creation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Create,
    ResumeStage,
    RecoverPublished,
}
/// Actual journal attempt/acknowledgement of the exact independently pinned event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JournalProgress {
    NotAttempted,
    Unknown,
    Acknowledged,
    Observed,
}
/// Observations, not authority. Error/cancel never implies rollback. NotAttempted
/// is no publication attempt by this call, not proof no prior stage exists.
#[derive(Clone, Debug)]
pub struct Progress {
    pub stage: Publication,
    pub intent: Publication,
    pub business: JournalProgress,
    pub personal: JournalProgress,
    pub published: Publication,
}
impl Default for Progress {
    /// No IO acknowledgement invented before the corresponding operation.
    fn default() -> Self {
        Self {
            stage: Publication::NotAttempted,
            intent: Publication::NotAttempted,
            business: JournalProgress::NotAttempted,
            personal: JournalProgress::NotAttempted,
            published: Publication::NotAttempted,
        }
    }
}
/// Caller-owned cancellation across all kernels. No timer/retry/deadline is added.
#[derive(Clone, Default)]
pub struct Context {
    cancelled: Arc<AtomicBool>,
    files: store::Context,
    journal: state::Context,
    lifecycle: authenticated_control::Context,
}
impl Context {
    /// New uncanceled operation, no background work.
    pub fn new() -> Self {
        Self::default()
    }
    /// Mark every consumer canceled; already known filesystem effects remain known.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.files.cancel();
        self.journal.cancel();
        self.lifecycle.cancel()
    }
    /// Local gate between filesystem operations; cannot preempt an in-flight syscall.
    fn check(&self) -> Result<(), Error> {
        if self.cancelled.load(Ordering::SeqCst) {
            Err(Error::Canceled)
        } else {
            Ok(())
        }
    }
}
/// Technical syscall stage only, no path or private OS diagnostic in formatting.
#[derive(Clone, Copy, Debug)]
pub enum IoPhase {
    Open,
    Stat,
    Create,
    Sync,
    Close,
    Rename,
    ReadDirectory,
}
/// Closed private document role, retained on failure without exposing its bytes.
#[derive(Clone, Copy, Debug)]
pub enum PrivateDocument {
    Intent,
    Declaration,
    Control,
    StagingDeclaration,
    StagingControl,
    PersonalSeed,
}
impl PrivateDocument {
    /// Internal versioned protocol name, never an arbitrary caller path.
    fn name(self) -> &'static str {
        match self {
            Self::Intent => "service-installation-v1.intent",
            Self::Declaration => "bootstrap-v3.json",
            Self::Control => "bootstrap-control-v2.json",
            Self::StagingDeclaration => "staging-v3.json",
            Self::StagingControl => "staging-control-v2.json",
            Self::PersonalSeed => "personal-authority.seed",
        }
    }
}
/// Typed primary plus every independent cleanup cause. No payload/path/key/pin
/// text is formatted; trusted traversal must inspect Joined.cleanup as well.
pub enum Error {
    Binding,
    Frame(serde_json::Error),
    Encoding(base64::DecodeError),
    Project(std::num::ParseIntError),
    Document {
        document: PrivateDocument,
        cause: Box<Error>,
    },
    Exists,
    Conflict,
    Canceled,
    #[cfg(feature = "test-fixtures")]
    Interrupted,
    Artifact(service_artifact::Error),
    Storage(config::Error),
    Write(WriteError),
    DurableRead(DurableReadError),
    Bootstrap(bootstrap::Error),
    State(state::Error),
    Startup(startup::Error),
    Lifecycle(authenticated_startup::Error),
    Personal(personal::coordinator::Error),
    LifecycleClose(authenticated_control::CloseError),
    Io {
        phase: IoPhase,
        cause: std::io::Error,
    },
    Joined {
        cause: Box<Error>,
        cleanup: Box<Error>,
    },
}
impl Error {
    /// Bounded static category, keeping the primary category through cleanup.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Binding => "binding",
            Self::Frame(_) => "intent-frame",
            Self::Encoding(_) => "key-encoding",
            Self::Project(_) => "project-encoding",
            Self::Document { cause, .. } => cause.code(),
            Self::Exists => "exists",
            Self::Conflict => "conflict",
            Self::Canceled => "canceled",
            #[cfg(feature = "test-fixtures")]
            Self::Interrupted => "fixture-interrupted",
            Self::Artifact(e) => e.code(),
            Self::Storage(e) => e.code(),
            Self::Write(e) => e.cause.code(),
            Self::DurableRead(e) => e.cause.code(),
            Self::Bootstrap(e) => e.code(),
            Self::State(e) => e.code(),
            Self::Startup(e) => e.code(),
            Self::Lifecycle(e) => e.code(),
            Self::Personal(e) => e.code(),
            Self::LifecycleClose(_) => "lifecycle-close",
            Self::Io { phase, .. } => match phase {
                IoPhase::Open => "io-open",
                IoPhase::Stat => "io-stat",
                IoPhase::Create => "io-create",
                IoPhase::Sync => "io-sync",
                IoPhase::Close => "io-close",
                IoPhase::Rename => "io-rename",
                IoPhase::ReadDirectory => "io-read-directory",
            },
            Self::Joined { cause, .. } => cause.code(),
        }
    }
}
impl fmt::Display for Error {
    /// Ordinary diagnostics omit all nested/private input and OS text.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "service installation: {}", self.code())
    }
}
impl fmt::Debug for Error {
    /// Same safe category as Display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Error {
    /// Primary original cause; Joined.cleanup is an independent mandatory branch.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Frame(e) => Some(e),
            Self::Encoding(e) => Some(e),
            Self::Project(e) => Some(e),
            Self::Document { cause, .. } => Some(cause.as_ref()),
            Self::Artifact(e) => Some(e),
            Self::Storage(e) => Some(e),
            Self::Write(e) => Some(e),
            Self::DurableRead(e) => Some(e),
            Self::Bootstrap(e) => Some(e),
            Self::State(e) => Some(e),
            Self::Startup(e) => Some(e),
            Self::Lifecycle(e) => Some(e),
            Self::Personal(e) => Some(e),
            Self::LifecycleClose(e) => Some(e),
            Self::Io { cause, .. } => Some(cause),
            Self::Joined { cause, .. } => Some(cause.as_ref()),
            _ => None,
        }
    }
}
/// Observed progress survives any later failure, including cleanup after rename.
pub struct Failure {
    pub progress: Progress,
    pub cause: Error,
}
impl fmt::Display for Failure {
    /// Safe technical phases only; inspect cause tree for authorized diagnosis.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}; {:?}", self.cause, self.progress)
    }
}
impl fmt::Debug for Failure {
    /// No path/key/pin/declaration is exposed.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Failure {
    /// Original primary/cleanup tree, distinct from progress.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.cause)
    }
}
/// Private startup locations and independently derived declaration pin. This
/// result establishes file publication, not human/OS/service readiness or a grant.
pub struct Installed {
    pub bootstrap_path: String,
    pub bootstrap_pin: String,
    pub personal_seed_path: String,
    pub progress: Progress,
}
impl fmt::Debug for Installed {
    /// Do not leak private paths or pins through routine diagnostics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ServiceInstallation { <private> }")
    }
}
/// Combine failures without discarding any secondary close/release cause.
fn combined<T>(r: Result<T, Error>, c: Result<(), Error>) -> Result<T, Error> {
    match (r, c) {
        (r, Ok(())) => r,
        (Ok(_), Err(e)) => Err(e),
        (Err(cause), Err(cleanup)) => Err(Error::Joined {
            cause: Box::new(cause),
            cleanup: Box::new(cleanup),
        }),
    }
}
/// Retain original syscall cause without formatting it.
fn io(phase: IoPhase, cause: impl Into<std::io::Error>) -> Error {
    Error::Io {
        phase,
        cause: cause.into(),
    }
}
/// Exact byte digest only; never learn an expected pin from untrusted storage.
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
/// Fixed technical checkpoints. Only test-fixtures builds expose hook injection;
/// no production retry/wait/policy hook is installed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixturePhase {
    IntentDurable,
    BusinessApplied,
    PersonalApplied,
    ReadyToPublish,
    RootRenamed,
    RootDurable,
    TreeClosed,
}
#[derive(Default)]
struct Hooks {
    #[cfg(feature = "test-fixtures")]
    hook: Option<Arc<dyn Fn(FixturePhase) -> Result<(), Error> + Send + Sync>>,
}
impl Hooks {
    /// Optional isolated test interruption, absent from normal-profile execution.
    fn hit(&self, phase: FixturePhase) -> Result<(), Error> {
        #[cfg(feature = "test-fixtures")]
        if let Some(h) = &self.hook {
            h(phase)?;
        }
        let _ = phase;
        Ok(())
    }
}
/// Publish only after real signed journals/current grant/bootstrap checks. An
/// independently protected expected artifact pin and actual receiving euid are
/// prerequisites; no actor/UID/head/key is learned from disk. Existing final roots
/// never reset or overwrite. Explicit resume only reuses exact pinned intent.
/// Errors retain all progress/close causes; partial files/stages are preserved.
/// Creates no personal checkpoint/account/listener or OS separation. Caller must
/// keep this trusted installer outside the agent domain and retain receipt pins.
pub fn install(
    ctx: &Context,
    artifact: &ServiceArtifact,
    expected_pin: &str,
    mode: Mode,
) -> Result<Installed, Failure> {
    run(ctx, artifact, expected_pin, mode, Hooks::default())
}
/// Isolated fixture interruption through the identical real filesystem/journals.
/// Never use on live roots; no hook or setter exists in the normal profile.
#[cfg(feature = "test-fixtures")]
pub fn install_with_hook(
    ctx: &Context,
    artifact: &ServiceArtifact,
    expected_pin: &str,
    mode: Mode,
    hook: Arc<dyn Fn(FixturePhase) -> Result<(), Error> + Send + Sync>,
) -> Result<Installed, Failure> {
    run(
        ctx,
        artifact,
        expected_pin,
        mode,
        Hooks { hook: Some(hook) },
    )
}

/// One exact private file: create absent data or validate/fsync existing bytes.
/// Published recovery never creates a missing file. Read/write phases survive in
/// typed causes; the caller supplies bytes/pins from independent material only.
fn exact(
    r: &store::documents::Reservation<'_>,
    document: PrivateDocument,
    raw: &[u8],
    allow_create: bool,
    phase: &mut Publication,
) -> Result<(), Error> {
    let name = document.name();
    let result = match r.read_private_durable(name, &hash(raw)) {
        Ok(_) => {
            *phase = Publication::Durable;
            Ok(())
        }
        Err(e) if allow_create && matches!(e.cause, config::Error::NotConfigured) => {
            match r.create(name, raw) {
                Ok(p) => {
                    *phase = p;
                    Ok(())
                }
                Err(e) => {
                    *phase = e.publication;
                    Err(Error::Write(e))
                }
            }
        }
        Err(e) => {
            if e.durability == store::documents::ReadDurability::DirectoryAcknowledged {
                *phase = Publication::Durable;
            }
            Err(Error::DurableRead(e))
        }
    };
    result.map_err(|cause| Error::Document {
        document,
        cause: Box::new(cause),
    })
}
/// Rebase only protected storage labels for prepublication verification. The
/// signed key/context/grant/head inputs remain identical to the final document.
fn stage_document(a: &ServiceArtifact, path: &Path) -> bootstrap::AuthenticatedDocument {
    let mut d = a.document().clone();
    let p = |s: &str| path.join(s).to_string_lossy().into_owned();
    d.control.installation.configuration_directory = p("configuration");
    d.control.installation.witness_directory = p("configuration-witness");
    d.control.journal_path = p("business/journal.sqlite");
    d.personal.journal_path = p("personal/journal.sqlite");
    d.personal.checkpoint_directory = p("personal-witness");
    d
}
/// Bind the top-level exact intent BEFORE creating/completing any storage child.
/// Metadata reservation finishes/closes before later personal authority. A missing
/// intent permits only an inert skeleton with no data; no existing head is adopted.
fn intent(
    ctx: &Context,
    tree: &filesystem::Tree,
    a: &ServiceArtifact,
    pin: &str,
    mode: Mode,
    progress: &mut Progress,
) -> Result<(), Error> {
    let docs = Documents::open(&tree.path, a.document().control.installation.service_uid)
        .map_err(Error::Storage)?;
    let result = (|| {
        let r = docs.reserve(&ctx.files).map_err(Error::Storage)?;
        let result = (|| {
            #[derive(serde::Serialize)]
            struct Intent<'a> {
                schema_version: i64,
                artifact_pin: &'a str,
                root_directory: &'a str,
            }
            let intent = swarm_wire_json::encode(&Intent {
                schema_version: 1,
                artifact_pin: pin,
                root_directory: a.root_directory(),
            })
            .map_err(Error::Frame)?;
            // Missing intent can authorize no existing data: only the empty stage protocol.
            let allow_intent =
                mode == Mode::Create || (mode == Mode::ResumeStage && tree.empty()?);
            exact(
                &r,
                PrivateDocument::Intent,
                &intent,
                allow_intent,
                &mut progress.intent,
            )?;
            ctx.check()?;
            if mode == Mode::RecoverPublished {
                progress.published = Publication::Visible;
            }
            Ok(())
        })();
        combined(result, r.finish().map_err(Error::Storage))
    })();
    combined(result, docs.close().map_err(Error::Storage))
}
/// Exact durable intent precedes any journal creation. Release/close the metadata
/// reservation before personal authority transactions (existing lock-order rule).
/// Empty stages can resume without adopting data, but published/mismatched state
/// refuses. All documents compare against independent expected canonical bytes.
fn identity(
    ctx: &Context,
    tree: &filesystem::Tree,
    a: &ServiceArtifact,
    mode: Mode,
) -> Result<(), Error> {
    let docs = Documents::open(
        &tree.path.join("identity"),
        a.document().control.installation.service_uid,
    )
    .map_err(Error::Storage)?;
    let result = (|| {
        let r = docs.reserve(&ctx.files).map_err(Error::Storage)?;
        let result = (|| {
            let final_raw =
                bootstrap::encode_authenticated(a.document()).map_err(Error::Bootstrap)?;
            let final_control =
                bootstrap::encode_control(&a.document().control).map_err(Error::Bootstrap)?;
            let staged = stage_document(a, &tree.original_stage());
            let stage_raw = bootstrap::encode_authenticated(&staged).map_err(Error::Bootstrap)?;
            let stage_control =
                bootstrap::encode_control(&staged.control).map_err(Error::Bootstrap)?;
            let mut seed = Zeroizing::new(Vec::with_capacity(32));
            a.write_personal_seed(&mut *seed).map_err(Error::Artifact)?;
            for (name, bytes) in [
                (PrivateDocument::Declaration, final_raw.as_slice()),
                (PrivateDocument::Control, final_control.as_slice()),
                (PrivateDocument::StagingDeclaration, stage_raw.as_slice()),
                (PrivateDocument::StagingControl, stage_control.as_slice()),
                (PrivateDocument::PersonalSeed, seed.as_slice()),
            ] {
                ctx.check()?;
                let mut phase = Publication::NotAttempted;
                exact(&r, name, bytes, mode != Mode::RecoverPublished, &mut phase)?;
            }
            Ok(())
        })();
        combined(result, r.finish().map_err(Error::Storage))
    })();
    combined(result, docs.close().map_err(Error::Storage))
}
/// Decode validated public domains without guessing/learning values from SQLite.
fn key(encoded: &str) -> Result<[u8; 32], Error> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(Error::Encoding)?
        .try_into()
        .map_err(|_| Error::Binding)
}
/// Convert only canonical declaration SHA256 to the real journal's context bytes.
fn project(digest: &str) -> Result<[u8; 32], Error> {
    let mut out = [0; 32];
    if !swarm_identity_contract::digest(digest.as_bytes()) {
        return Err(Error::Binding);
    }
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&digest[2 * i..2 * i + 2], 16).map_err(Error::Project)?;
    }
    Ok(out)
}
/// Apply exact signed initial bytes at independently fixed base zero. Existing
/// journal contract allows only exact full replay. No current head is adopted;
/// close errors preserve an already acknowledged expected head.
fn apply(
    ctx: &Context,
    path: &Path,
    key: [u8; 32],
    project: [u8; 32],
    raw: &[u8],
    expected: &Head,
    progress: &mut JournalProgress,
) -> Result<(), Error> {
    ctx.check()?;
    let journal = Journal::open(&ctx.journal, path, &key, project).map_err(Error::State)?;
    *progress = JournalProgress::Unknown;
    let result = journal
        .apply(&ctx.journal, &Head::default(), raw)
        .map_err(Error::State)
        .and_then(|h| {
            if &h == expected {
                *progress = JournalProgress::Acknowledged;
                Ok(())
            } else {
                Err(Error::Conflict)
            }
        });
    combined(result, journal.close().map_err(Error::State))
}
/// Use actual protected loader/current business source and personal bootstrap
/// semantics; no crypto copy, account, checkpoint or capability is manufactured.
fn authority(
    ctx: &Context,
    tree: &filesystem::Tree,
    a: &ServiceArtifact,
    published: bool,
) -> Result<(), Error> {
    tree.check(ctx)?;
    let prefix = if published { "bootstrap" } else { "staging" };
    let d = if published {
        a.document().clone()
    } else {
        stage_document(a, &tree.path)
    };
    let control = bootstrap::encode_control(&d.control).map_err(Error::Bootstrap)?;
    startup::check(
        &ctx.journal,
        tree.path
            .join(format!("identity/{prefix}-control-v2.json"))
            .to_str()
            .ok_or(Error::Binding)?,
        &hash(&control),
    )
    .map_err(Error::Startup)?;
    let raw = bootstrap::encode_authenticated(&d).map_err(Error::Bootstrap)?;
    let lifecycle = authenticated_startup::open_lifecycle(
        &ctx.journal,
        tree.path
            .join(format!("identity/{prefix}-v3.json"))
            .to_str()
            .ok_or(Error::Binding)?,
        &hash(&raw),
        &tree.path.join("identity/personal-authority.seed"),
    )
    .map_err(Error::Lifecycle)?;
    let result = lifecycle
        .check_bootstrap(&ctx.lifecycle)
        .map_err(Error::Personal);
    combined(result, lifecycle.close().map_err(Error::LifecycleClose))?;
    tree.check(ctx)
}
/// One bounded attempt, never implicit retry. Validate before opening the parent;
/// keep publication/journal observations outside all cleanup combinations.
fn run(
    ctx: &Context,
    a: &ServiceArtifact,
    pin: &str,
    mode: Mode,
    hooks: Hooks,
) -> Result<Installed, Failure> {
    let mut progress = Progress::default();
    let early = (|| {
        ctx.check()?;
        if !swarm_identity_contract::digest(pin.as_bytes())
            || !bool::from(
                a.pin()
                    .map_err(Error::Artifact)?
                    .as_bytes()
                    .ct_eq(pin.as_bytes()),
            )
            || a.document().control.installation.service_uid != rustix::process::geteuid().as_raw()
        {
            return Err(Error::Binding);
        }
        Ok(())
    })();
    if let Err(cause) = early {
        return Err(Failure { progress, cause });
    }
    let mut tree = filesystem::Tree::open(
        ctx,
        a.root_directory(),
        a.document().control.installation.service_uid,
        mode,
        &mut progress,
    )
    .map_err(|cause| Failure {
        progress: progress.clone(),
        cause,
    })?;
    let result = (|| {
        tree.check(ctx)?;
        tree.root_inventory()?;
        intent(ctx, &tree, a, pin, mode, &mut progress)?;
        tree.children(ctx, mode)?;
        tree.inventory()?;
        identity(ctx, &tree, a, mode)?;
        tree.acknowledge_stage(ctx, &mut progress)?;
        hooks.hit(FixturePhase::IntentDurable)?;
        ctx.check()?;
        tree.journals(ctx, mode != Mode::RecoverPublished)?;
        if mode != Mode::RecoverPublished {
            let d = a.document();
            apply(
                ctx,
                &tree.path.join("business/journal.sqlite"),
                key(&d.control.installation.pins.public_key)?,
                project(&d.control.project_context_digest)?,
                a.business_initial(),
                &d.control.authority_head,
                &mut progress.business,
            )?;
            hooks.hit(FixturePhase::BusinessApplied)?;
            ctx.check()?;
            tree.check(ctx)?;
            apply(
                ctx,
                &tree.path.join("personal/journal.sqlite"),
                key(&d.personal.authority_public_key)?,
                project(&d.personal.project_context_digest)?,
                a.personal_initial(),
                &d.personal.bootstrap_head,
                &mut progress.personal,
            )?;
            hooks.hit(FixturePhase::PersonalApplied)?;
            ctx.check()?;
            tree.check(ctx)?;
        }
        authority(ctx, &tree, a, mode == Mode::RecoverPublished)?;
        if mode == Mode::RecoverPublished {
            progress.business = JournalProgress::Observed;
            progress.personal = JournalProgress::Observed;
        }
        tree.sync(ctx)?;
        if mode != Mode::RecoverPublished {
            hooks.hit(FixturePhase::ReadyToPublish)?;
            tree.check(ctx)?;
            tree.publish(ctx, &mut progress)?;
            hooks.hit(FixturePhase::RootRenamed)?;
        }
        tree.acknowledge(ctx, &mut progress)?;
        hooks.hit(FixturePhase::RootDurable)?;
        Ok(())
    })();
    let result = combined(result, tree.close());
    let result = combined(result, hooks.hit(FixturePhase::TreeClosed));
    result.map_err(|cause| Failure {
        progress: progress.clone(),
        cause,
    })?;
    let raw = bootstrap::encode_authenticated(a.document()).map_err(|e| Failure {
        progress: progress.clone(),
        cause: Error::Bootstrap(e),
    })?;
    Ok(Installed {
        bootstrap_path: format!("{}/identity/bootstrap-v3.json", a.root_directory()),
        bootstrap_pin: hash(&raw),
        personal_seed_path: format!("{}/identity/personal-authority.seed", a.root_directory()),
        progress,
    })
}
