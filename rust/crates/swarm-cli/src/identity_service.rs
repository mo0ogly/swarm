//! Trusted operator installation adapter, not an agent-facing authority API.
//! Private service-only artifact input; no business key, proof, RootStore/provider,
//! human, OS account or listener. Diagnostic and stdout custody remain distinct.
use super::{json_bytes, Arguments, CliError};
use serde::Serialize;
use std::{
    error::Error as StdError,
    fmt,
    io::{self, Write},
    path::Path,
};
use swarm_enrollment_authority::provisioning::{
    service_artifact::{self, ServiceArtifact},
    service_installation as core,
};
use swarm_enrollment_config::{
    self as config,
    store::{
        self,
        documents::{Documents, DurableReadError, Publication, ReadDurability, ReadValidation},
    },
};
/// Closed command vocabulary, never an execution permission or policy.
#[derive(Clone, Copy)]
enum Action {
    Install,
    Resume,
    Reconcile,
}
impl Action {
    /// Explicit operator-selected initial protocol operation.
    fn name(self) -> &'static str {
        match self {
            Self::Install => "service-install",
            Self::Resume => "service-resume",
            Self::Reconcile => "service-reconcile",
        }
    }
    /// No fallback from failed creation to resume or reconciliation.
    fn mode(self) -> core::Mode {
        match self {
            Self::Install => core::Mode::Create,
            Self::Resume => core::Mode::ResumeStage,
            Self::Reconcile => core::Mode::RecoverPublished,
        }
    }
}
/// Independent observations survive late input decode/release/close failure.
#[derive(Clone, Copy)]
pub struct InputProgress {
    pub validation: ReadValidation,
    pub durability: ReadDurability,
    pub decoded: bool,
}
impl Default for InputProgress {
    /// No IO or successful decode inferred before the actual operation.
    fn default() -> Self {
        Self {
            validation: ReadValidation::NotAttempted,
            durability: ReadDurability::NotAcknowledged,
            decoded: false,
        }
    }
}
/// Private input cause tree; source() selects primary, Joined.cleanup independent.
pub enum InputCause {
    Path,
    Pin,
    Storage(config::Error),
    Read(DurableReadError),
    Artifact(service_artifact::Error),
    Joined {
        cause: Box<InputCause>,
        cleanup: Box<InputCause>,
    },
}
impl InputCause {
    /// Bounded safe category only, never file/pin/key or nested IO text.
    fn code(&self) -> &'static str {
        match self {
            Self::Path => "service-input-path",
            Self::Pin => "service-input-pin",
            Self::Storage(e) => e.code(),
            Self::Read(e) => e.cause.code(),
            Self::Artifact(e) => e.code(),
            Self::Joined { cause, .. } => cause.code(),
        }
    }
}
impl fmt::Display for InputCause {
    /// Do not include any private storage/decoder diagnostic contents.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "service input: {}", self.code())
    }
}
impl fmt::Debug for InputCause {
    /// Same safe category as Display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for InputCause {
    /// Original primary cause; inspect Joined.cleanup as well.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Storage(e) => Some(e),
            Self::Read(e) => Some(e),
            Self::Artifact(e) => Some(e),
            Self::Joined { cause, .. } => Some(cause.as_ref()),
            _ => None,
        }
    }
}
/// Failed input never yields a key, even if decoding succeeded before close.
pub struct InputFailure {
    pub progress: InputProgress,
    pub cause: InputCause,
}
impl fmt::Display for InputFailure {
    /// Technical observations only. No file/pin/data or nested IO text.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}; validation={:?}, durability={:?}, decoded={}",
            self.cause, self.progress.validation, self.progress.durability, self.progress.decoded
        )
    }
}
impl fmt::Debug for InputFailure {
    /// Same redacted diagnostic as Display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for InputFailure {
    /// Full original primary/cleanup input tree, with progress retained separately.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.cause)
    }
}
/// Withhold a successful owned key if cleanup fails; preserve both failures.
fn input_combined<T>(r: Result<T, InputCause>, c: Result<(), InputCause>) -> Result<T, InputCause> {
    match (r, c) {
        (r, Ok(())) => r,
        (Ok(_), Err(e)) => Err(e),
        (Err(cause), Err(cleanup)) => Err(InputCause::Joined {
            cause: Box::new(cause),
            cleanup: Box::new(cleanup),
        }),
    }
}
/// Read exact private bytes/pin before fsync, decode42 then explicitly finish and
/// close the owned input store. Any late refusal drops/zeroizes decoded material.
/// Only independent expected pins are accepted; no data/head/key is adopted.
fn load_owned(docs: Documents, name: &str, pin: &str) -> Result<ServiceArtifact, InputFailure> {
    let mut progress = InputProgress::default();
    let ctx = store::Context::new();
    let result = (|| {
        let r = docs.reserve(&ctx).map_err(InputCause::Storage)?;
        let result = (|| {
            let raw = match r.read_private_durable(name, pin) {
                Ok(raw) => {
                    progress.validation = ReadValidation::PinMatched;
                    progress.durability = ReadDurability::DirectoryAcknowledged;
                    raw
                }
                Err(e) => {
                    progress.validation = e.validation;
                    progress.durability = e.durability;
                    return Err(InputCause::Read(e));
                }
            };
            let artifact =
                service_artifact::decode(raw.as_bytes(), pin).map_err(InputCause::Artifact)?;
            progress.decoded = true;
            drop(raw);
            Ok(artifact)
        })();
        input_combined(result, r.finish().map_err(InputCause::Storage))
    })();
    input_combined(result, docs.close().map_err(InputCause::Storage))
        .map_err(|cause| InputFailure { progress, cause })
}
/// Absolute private input path only; canonical pin checked before parent IO.
/// Parent0700/current euid and file0600/singlelink/name/nofollow checks come from
/// shared Documents. Basename uses its ASCII protocol grammar. Opening may create
/// a private lockfile, never change input bytes or open/create the target root.
fn load(path: &Path, pin: &str) -> Result<ServiceArtifact, InputFailure> {
    let failure = |cause| InputFailure {
        progress: InputProgress::default(),
        cause,
    };
    if !swarm_identity_contract::digest(pin.as_bytes()) {
        return Err(failure(InputCause::Pin));
    }
    if !path.is_absolute() {
        return Err(failure(InputCause::Path));
    }
    let parent = path.parent().ok_or_else(|| failure(InputCause::Path))?;
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| failure(InputCause::Path))?;
    let docs = Documents::open(parent, rustix::process::geteuid().as_raw())
        .map_err(|e| failure(InputCause::Storage(e)))?;
    load_owned(docs, name, pin)
}

/// Trusted operator startup data, derived from independently pinned material,
/// never read back/learned from installed storage. Protect stdout/retained copy.
#[derive(Clone, Serialize)]
struct Startup {
    bootstrap_path: String,
    bootstrap_pin: String,
    personal_seed_path: String,
}
/// Closed output protocol; never an input grant or a human/OS assertion. Startup
/// paths/pin are private operator output; Debug/diagnostics remain redacted.
#[derive(Clone, Serialize)]
pub struct Report {
    schema_version: i64,
    protocol: &'static str,
    operation: &'static str,
    operation_result: &'static str,
    stage: &'static str,
    intent: &'static str,
    business_journal: &'static str,
    personal_journal: &'static str,
    publication: &'static str,
    startup: Option<Startup>,
    human_presence_verified: bool,
    service_started: bool,
    confinement_verified: bool,
}
impl fmt::Debug for Report {
    /// No private path, pin or nested report data in routine Debug.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ServiceInstallationReport { <private> }")
    }
}
/// Existing exact publication labels, not retry policy or absence-of-effect proof.
fn publication(p: Publication) -> &'static str {
    match p {
        Publication::NotAttempted => "not_attempted",
        Publication::Unknown => "unknown",
        Publication::Visible => "visible",
        Publication::Durable => "durable",
    }
}
/// Keep observation distinct from applying an event and from an unknown attempt.
fn journal(j: core::JournalProgress) -> &'static str {
    match j {
        core::JournalProgress::NotAttempted => "not_attempted",
        core::JournalProgress::Unknown => "unknown",
        core::JournalProgress::Acknowledged => "acknowledged",
        core::JournalProgress::Observed => "observed",
    }
}
impl Report {
    /// Project only supplied actual kernel progress. Failed never implies no effect;
    /// no key, proof, credential or private digest is placed in the error summary.
    fn failed(action: Action, p: &core::Progress) -> Self {
        Self {
            schema_version: 1,
            protocol: "swarm-service-installation-cli-v1",
            operation: action.name(),
            operation_result: "failed",
            stage: publication(p.stage),
            intent: publication(p.intent),
            business_journal: journal(p.business),
            personal_journal: journal(p.personal),
            publication: publication(p.published),
            startup: None,
            human_presence_verified: false,
            service_started: false,
            confinement_verified: false,
        }
    }
    /// Successful kernel close and durable publication; startup data derives only
    /// from expected material. This grants no account, session or business authority.
    fn completed(action: Action, installed: core::Installed) -> Self {
        let mut r = Self::failed(action, &installed.progress);
        r.operation_result = "completed";
        r.startup = Some(Startup {
            bootstrap_path: installed.bootstrap_path,
            bootstrap_pin: installed.bootstrap_pin,
            personal_seed_path: installed.personal_seed_path,
        });
        r
    }
    /// Bounded public-safe summary; excludes private startup paths/pin and IO text.
    fn summary(&self) -> String {
        format!("operation={}, result={}, stage={}, intent={}, business={}, personal={}, publication={}",self.operation,self.operation_result,self.stage,self.intent,self.business_journal,self.personal_journal,self.publication)
    }
}
/// Transport phase distinct from kernel filesystem/journal outcome.
#[derive(Clone, Copy, Debug)]
pub enum OutputPhase {
    Encode,
    Write,
    Flush,
}
impl OutputPhase {
    /// Stable safe label, no caller/private text.
    fn name(self) -> &'static str {
        match self {
            Self::Encode => "encode",
            Self::Write => "write",
            Self::Flush => "flush",
        }
    }
}
/// Every input/kernel/output failure retains its original cause and observations.
/// Ordinary diagnostics omit all private startup data and nested error contents.
pub enum Error {
    Input(InputFailure),
    Operation {
        cause: Box<core::Failure>,
        report: Report,
    },
    Output {
        cause: io::Error,
        phase: OutputPhase,
        failure: Option<Box<Error>>,
        report: Report,
    },
}
impl Error {
    /// Static safe category only, preserving actual kernel refusal on operation error.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Input(e) => e.cause.code(),
            Self::Operation { cause, .. } => cause.cause.code(),
            Self::Output { .. } => "service-cli-output",
        }
    }
    /// Original known kernel report even when stdout write/flush subsequently failed.
    pub fn report(&self) -> Option<&Report> {
        match self {
            Self::Operation { report, .. } | Self::Output { report, .. } => Some(report),
            _ => None,
        }
    }
    /// Safe observations distinguish unstarted installer/input effects from known
    /// root publication. Never an instruction to retry or absence-of-change claim.
    pub fn observation(&self) -> String {
        match self {
            Self::Input(e) => format!(
                "installer not started; input validation={:?}, durability={:?}, decoded={}",
                e.progress.validation, e.progress.durability, e.progress.decoded
            ),
            Self::Operation { report, .. } | Self::Output { report, .. } => report.summary(),
        }
    }
}
impl fmt::Display for Error {
    /// Category only; use typed causes for authorized diagnosis.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "service CLI: {}", self.code())
    }
}
impl fmt::Debug for Error {
    /// Same redaction as Display, never nested report/IO formatting.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Error {
    /// Primary cause; output.failure and core/input Joined cleanup remain mandatory
    /// independent branches, supplied by error_links to both shared CLI traversals.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Input(e) => Some(e),
            Self::Operation { cause, .. } => Some(cause.as_ref()),
            Self::Output { cause, .. } => Some(cause),
        }
    }
}
/// Grammar/default checks before any IO. Shared parser already enforces each
/// identity input/json/lang option once, including default-valued forbidden flags.
fn action(a: &Arguments) -> Result<Action, CliError> {
    if a.pos.len() != 3
        || a.input.is_empty()
        || a.input == "-"
        || !Path::new(&a.input).is_absolute()
        || a.root != "."
        || a.server
        || !a.output.is_empty()
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    match a.pos[1].as_str() {
        "service-install" => Ok(Action::Install),
        "service-resume" => Ok(Action::Resume),
        "service-reconcile" => Ok(Action::Reconcile),
        _ => Err(CliError::Usage),
    }
}
/// Emit/flush after all key/storage owners are gone. Transport failures retain
/// the exact known report and any earlier input/kernel/cleanup failure tree.
fn emit(report: Report, failure: Option<Error>, out: &mut dyn Write) -> Result<u8, Error> {
    let bytes = match json_bytes(&report) {
        Ok(bytes) => bytes,
        Err(cause) => {
            return Err(Error::Output {
                cause,
                phase: OutputPhase::Encode,
                failure: failure.map(Box::new),
                report,
            })
        }
    };
    if let Err(cause) = out.write_all(&bytes) {
        return Err(Error::Output {
            cause,
            phase: OutputPhase::Write,
            failure: failure.map(Box::new),
            report,
        });
    }
    if let Err(cause) = out.flush() {
        return Err(Error::Output {
            cause,
            phase: OutputPhase::Flush,
            failure: failure.map(Box::new),
            report,
        });
    }
    match failure {
        Some(e) => Err(e),
        None => Ok(0),
    }
}
/// Load only private42material, run exactly one explicitly selected43operation,
/// drop the artifact before output, preserve all known phases and close branches.
/// No stdin, RootStore/provider, new key, OS account, service or human is used.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    let action = action(a)?;
    let run = (|| {
        let artifact = load(Path::new(&a.input), &a.pos[2]).map_err(Error::Input)?;
        let result = core::install(&core::Context::new(), &artifact, &a.pos[2], action.mode());
        drop(artifact);
        match result {
            Ok(installed) => emit(Report::completed(action, installed), None, out),
            Err(cause) => {
                let report = Report::failed(action, &cause.progress);
                emit(
                    report.clone(),
                    Some(Error::Operation {
                        cause: Box::new(cause),
                        report,
                    }),
                    out,
                )
            }
        }
    })();
    run.map_err(|cause| CliError::ServiceInstallation {
        cause: Box::new(cause),
        english: a.language == "en",
    })
}
/// Add every independent branch to existing source/custom-IO traversal. Labels
/// contain static categories/technical enums only, never startup paths/pins/data.
pub(super) fn error_links<'a>(
    e: &'a (dyn StdError + 'static),
) -> (Vec<String>, Vec<&'a (dyn StdError + 'static)>) {
    let mut labels = vec![];
    let mut children: Vec<&'a (dyn StdError + 'static)> = vec![];
    if let Some(e) = e.downcast_ref::<Error>() {
        labels.push(format!("service-cli:{}", e.code()));
        if let Some(r) = e.report() {
            labels.push(format!("service-reported-result:{}", r.operation_result));
            labels.push(format!("service-reported-publication:{}", r.publication));
            labels.push(format!("service-reported-stage:{}", r.stage));
            labels.push(format!("service-reported-intent:{}", r.intent));
            labels.push(format!("service-reported-business:{}", r.business_journal));
            labels.push(format!("service-reported-personal:{}", r.personal_journal));
        }
        if let Error::Output { phase, failure, .. } = e {
            labels.push(format!("service-output-phase:{}", phase.name()));
            if let Some(f) = failure {
                children.push(f.as_ref());
            }
        }
    }
    if let Some(e) = e.downcast_ref::<InputFailure>() {
        labels.push(format!(
            "service-input-validation:{:?}",
            e.progress.validation
        ));
        labels.push(format!(
            "service-input-durability:{:?}",
            e.progress.durability
        ));
        labels.push(format!("service-input-decoded:{}", e.progress.decoded));
    }
    if let Some(e) = e.downcast_ref::<InputCause>() {
        labels.push(format!("service-input:{}", e.code()));
        if let InputCause::Joined { cleanup, .. } = e {
            children.push(cleanup.as_ref());
        }
    }
    if let Some(e) = e.downcast_ref::<core::Failure>() {
        labels.push(format!(
            "service-progress-stage:{}",
            publication(e.progress.stage)
        ));
        labels.push(format!(
            "service-progress-intent:{}",
            publication(e.progress.intent)
        ));
        labels.push(format!(
            "service-progress-business:{}",
            journal(e.progress.business)
        ));
        labels.push(format!(
            "service-progress-personal:{}",
            journal(e.progress.personal)
        ));
        labels.push(format!(
            "service-progress-publication:{}",
            publication(e.progress.published)
        ));
    }
    if let Some(e) = e.downcast_ref::<core::Error>() {
        labels.push(format!("service-installation:{}", e.code()));
        if let core::Error::Joined { cleanup, .. } = e {
            children.push(cleanup.as_ref());
        }
        if let core::Error::Document { document, .. } = e {
            labels.push(format!("service-document:{document:?}"));
        }
    }
    if let Some(e) = e.downcast_ref::<DurableReadError>() {
        labels.push(format!("private-read-validation:{:?}", e.validation));
        labels.push(format!("private-read-durability:{:?}", e.durability));
    }
    if let Some(e) = e.downcast_ref::<service_artifact::Error>() {
        labels.push(format!("service-artifact:{}", e.code()));
    }
    (labels, children)
}
#[cfg(test)]
mod tests;
