//! Independently pinned operator package and deterministic service-only handoff.
//! Shared authority kernel: no engine/CLI dependency, new keys/proof, account,
//! grant, journal, installation or listener. Atomic temp filenames use entropy.
//! UID match is data in the current kernel view, never human/namespace admission.
use super::{self as material, repository as package, service_artifact};
use serde::Serialize;
use std::{error::Error as StdError, fmt, path::Path};
use swarm_enrollment_config::{
    self as config,
    store::{
        self,
        documents::{
            Documents, DurableReadError, Publication, ReadDurability, ReadValidation, WriteError,
        },
    },
};
/// Closed protocol vocabulary, not a business permission or execution policy.
#[derive(Clone, Copy)]
pub enum Action {
    Check,
    Export,
    Reconcile,
}
impl Action {
    /// Parse an exact public operation label without IO, defaults or aliases.
    /// A recognized label is vocabulary only, never authority to use material.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "operator-check" => Some(Self::Check),
            "service-export" => Some(Self::Export),
            "service-export-reconcile" => Some(Self::Reconcile),
            _ => None,
        }
    }
    /// Public exact operation label; no failed create falls back to reconciliation.
    pub fn name(self) -> &'static str {
        match self {
            Self::Check => "operator-check",
            Self::Export => "service-export",
            Self::Reconcile => "service-export-reconcile",
        }
    }
}
/// Actual package read/ack observations, without authority or usable-key claims.
#[derive(Clone, Copy)]
struct InputProgress {
    read: package::ReadObservation,
    durability: ReadDurability,
}
impl Default for InputProgress {
    /// No read or acknowledgement inferred before IO begins.
    fn default() -> Self {
        Self {
            read: package::ReadObservation::NotAttempted,
            durability: ReadDurability::NotAcknowledged,
        }
    }
}
/// Private input branches preserve the original primary and independent close.
enum InputCause {
    Path,
    Role,
    Storage(config::Error),
    Package(package::Error),
    Read(package::ReadError),
    Artifact(service_artifact::Error),
    Joined {
        cause: Box<InputCause>,
        cleanup: package::Error,
    },
}
impl InputCause {
    /// Static safe category, never path/pin/key/subject or nested IO text.
    fn code(&self) -> &'static str {
        match self {
            Self::Path => "operator-input-path",
            Self::Role => "operator-controller-uid",
            Self::Storage(e) => e.code(),
            Self::Package(e) => e.code(),
            Self::Read(e) => e.cause.code(),
            Self::Artifact(e) => e.code(),
            Self::Joined { cause, .. } => cause.code(),
        }
    }
}
impl fmt::Display for InputCause {
    /// Omit private values and raw IO messages from ordinary diagnostics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for InputCause {
    /// Same private-data redaction as Display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for InputCause {
    /// Primary source only; error_links also exposes independent close branches.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Storage(e) => Some(e),
            Self::Package(e) => Some(e),
            Self::Read(e) => Some(e),
            Self::Artifact(e) => Some(e),
            Self::Joined { cause, .. } => Some(cause.as_ref()),
            _ => None,
        }
    }
}
/// Failed source read/close/role check never exposes decoded material to dispatch.
pub struct InputFailure {
    progress: InputProgress,
    cause: InputCause,
}
impl fmt::Display for InputFailure {
    /// Technical observations, not private contents or a successful overall check.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}; read={:?}, durability={:?}",
            self.cause, self.progress.read, self.progress.durability
        )
    }
}
impl fmt::Debug for InputFailure {
    /// Same safe rendering as Display, including fixture assertion failures.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for InputFailure {
    /// Preserve typed source and all nested original causes.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.cause)
    }
}
/// Join real package-store close with the original result. A late close refuses
/// and drops a successfully decoded material value, with no automatic retry.
fn input_finish<T>(
    r: Result<T, InputCause>,
    close: Result<(), package::Error>,
) -> Result<T, InputCause> {
    match (r, close) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(InputCause::Package(e)),
        (Err(cause), Err(cleanup)) => Err(InputCause::Joined {
            cause: Box::new(cause),
            cleanup,
        }),
    }
}
/// Read/fsync independently pinned package under owned descriptors; compare its
/// declared controller UID to the actual captured operator UID, then close before
/// returning usable material. Private decoding reconstructs existing proposals,
/// not current authority. Any late refusal drops keys, retains read/ack phases,
/// and never returns a source-derived artifact. No entropy or live Root is used.
fn load_owned(
    store: package::PackageStore,
    pin: &package::Pin,
    uid: u32,
) -> Result<material::Prepared, InputFailure> {
    let mut progress = InputProgress::default();
    let ctx = store::Context::new();
    let result = match store.load_durable(&ctx, pin) {
        Ok(prepared) => {
            progress.read = package::ReadObservation::Decoded;
            progress.durability = ReadDurability::DirectoryAcknowledged;
            if prepared.request().controller_uid != uid {
                Err(InputCause::Role)
            } else {
                Ok(prepared)
            }
        }
        Err(e) => {
            progress.read = e.observation;
            progress.durability = e.durability;
            Err(InputCause::Read(e))
        }
    };
    input_finish(result, store.close()).map_err(|cause| InputFailure { progress, cause })
}
/// Canonical pin and absolute private operator directory before IO. Documents
/// binds actual current-UID owned0700 directory and immutable singleton0600 file;
/// no request-selected UID or source pathname is adopted as a storage identity.
/// Opening may create the private lock; reading acknowledges the local FS only.
fn load(path: &Path, pin: &str, uid: u32) -> Result<material::Prepared, InputFailure> {
    let failed = |cause| InputFailure {
        progress: InputProgress::default(),
        cause,
    };
    let pin = package::Pin::parse(pin).map_err(|e| failed(InputCause::Package(e)))?;
    if !path.is_absolute() {
        return Err(failed(InputCause::Path));
    }
    let documents = Documents::open(path, uid).map_err(|e| failed(InputCause::Storage(e)))?;
    load_owned(package::PackageStore::new(documents), &pin, uid)
}
/// Output observations distinguish publication from subsequent read/acknowledgement.
#[derive(Clone, Copy, Debug)]
struct OutputProgress {
    publication: Publication,
    validation: ReadValidation,
    durability: ReadDurability,
}
impl Default for OutputProgress {
    /// No output IO, publication or acknowledgement inferred initially.
    fn default() -> Self {
        Self {
            publication: Publication::NotAttempted,
            validation: ReadValidation::NotAttempted,
            durability: ReadDurability::NotAcknowledged,
        }
    }
}
/// Private target errors keep publication and every independent cleanup cause.
pub enum TargetCause {
    Path,
    Storage(config::Error),
    Write(WriteError),
    Read(DurableReadError),
    Joined {
        cause: Box<TargetCause>,
        cleanup: config::Error,
    },
}
impl TargetCause {
    /// Safe categories only, no private filename, pin, key or raw IO diagnostic.
    fn code(&self) -> &'static str {
        match self {
            Self::Path => "operator-output-path",
            Self::Storage(e) => e.code(),
            Self::Write(e) => e.cause.code(),
            Self::Read(e) => e.cause.code(),
            Self::Joined { cause, .. } => cause.code(),
        }
    }
}
impl fmt::Display for TargetCause {
    /// Redact nested input/OS data in ordinary diagnostics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for TargetCause {
    /// Same redaction as Display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for TargetCause {
    /// Primary source only; error_links retains independent cleanup as well.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Storage(e) => Some(e),
            Self::Write(e) => Some(e),
            Self::Read(e) => Some(e),
            Self::Joined { cause, .. } => Some(cause.as_ref()),
            _ => None,
        }
    }
}
/// Join release/close without discarding original output effects or failures.
fn target_finish<T>(
    r: Result<T, TargetCause>,
    close: Result<(), config::Error>,
) -> Result<T, TargetCause> {
    match (r, close) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(TargetCause::Storage(e)),
        (Err(cause), Err(cleanup)) => Err(TargetCause::Joined {
            cause: Box::new(cause),
            cleanup,
        }),
    }
}
/// Run one exact create or explicit read reconciliation on retained target handles.
/// NOREPLACE never adopts an existing file. Reconciliation verifies the expected
/// pin derived from the separately protected package before file/directory fsync;
/// it does not learn a pin, create a file, initialize journals or generate entropy.
/// Preserve progress on write/read/unlock/close failure; raw seed buffers remain
/// caller-guarded and never enter reports or diagnostics. Parent lock/atime are
/// allowed side effects. Caller owns protected handoff to the real service UID.
fn target_owned(
    docs: Documents,
    name: &str,
    action: Action,
    raw: &[u8],
    pin: &str,
) -> Result<OutputProgress, (OutputProgress, TargetCause)> {
    let mut progress = OutputProgress::default();
    let ctx = store::Context::new();
    let result = (|| {
        let reservation = docs.reserve(&ctx).map_err(TargetCause::Storage)?;
        let result = match action {
            Action::Export => match reservation.create(name, raw) {
                Ok(phase) => {
                    progress.publication = phase;
                    Ok(())
                }
                Err(e) => {
                    progress.publication = e.publication;
                    Err(TargetCause::Write(e))
                }
            },
            Action::Reconcile => match reservation.read_private_durable(name, pin) {
                Ok(bytes) => {
                    progress.validation = ReadValidation::PinMatched;
                    progress.durability = ReadDurability::DirectoryAcknowledged;
                    drop(bytes);
                    Ok(())
                }
                Err(e) => {
                    progress.validation = e.validation;
                    progress.durability = e.durability;
                    Err(TargetCause::Read(e))
                }
            },
            Action::Check => Err(TargetCause::Path),
        };
        target_finish(result, reservation.finish())
    })();
    target_finish(result, docs.close())
        .map(|()| progress)
        .map_err(|cause| (progress, cause))
}
/// Absolute private file target; no implicit directory creation/chmod/adoption.
/// Basename grammar/reserved lock and actual current-UID/private/singlelink rules
/// come from the same Documents producer. A create writes only service artifact
/// bytes, never the source package. Same-directory output does not copy other
/// files; protected service-UID handoff must transfer only the selected artifact.
fn target(
    path: &Path,
    action: Action,
    raw: &[u8],
    pin: &str,
    uid: u32,
) -> Result<OutputProgress, (OutputProgress, TargetCause)> {
    let failed = |cause| (OutputProgress::default(), cause);
    if !path.is_absolute() {
        return Err(failed(TargetCause::Path));
    }
    let parent = path.parent().ok_or_else(|| failed(TargetCause::Path))?;
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| failed(TargetCause::Path))?;
    let docs = Documents::open(parent, uid).map_err(|e| failed(TargetCause::Storage(e)))?;
    target_owned(docs, name, action, raw, pin)
}
/// Fixed diagnostic projection of actual publication, not operator policy.
fn publication(value: Publication) -> &'static str {
    match value {
        Publication::NotAttempted => "not_attempted",
        Publication::Unknown => "unknown",
        Publication::Visible => "visible",
        Publication::Durable => "durable",
    }
}
/// Fixed names for actual private-read validation, without an authority verdict.
fn validation(value: ReadValidation) -> &'static str {
    match value {
        ReadValidation::NotAttempted => "not_attempted",
        ReadValidation::ReadUnknown => "read_unknown",
        ReadValidation::BytesRead => "bytes_read",
        ReadValidation::PinMatched => "pin_matched",
    }
}
/// Fixed local FS acknowledgement labels, never freshness/hardware survival.
fn durability(value: ReadDurability) -> &'static str {
    match value {
        ReadDurability::NotAcknowledged => "not_acknowledged",
        ReadDurability::FileAcknowledged => "file_acknowledged",
        ReadDurability::DirectoryAcknowledged => "directory_acknowledged",
    }
}
/// Closed operator metadata after source custody is complete. Pins are explicit
/// private caller output, not ordinary diagnostic text. No seed/proof/path/grant
/// is serialized. UID equality is data matching, never account/human authority.
#[derive(Clone, Serialize)]
pub struct Report {
    schema_version: i64,
    protocol: &'static str,
    operation: &'static str,
    operation_result: &'static str,
    operator_package_pin: String,
    service_artifact_pin: String,
    controller_uid_matches_operator: bool,
    source_read: &'static str,
    source_durability: &'static str,
    publication: &'static str,
    output_validation: &'static str,
    output_durability: &'static str,
    human_authentication_verified: bool,
    current_authority_verified: bool,
    service_installed: bool,
    service_started: bool,
    separate_os_uid_verified: bool,
    confinement_verified: bool,
}
impl Report {
    /// Safe current semantic/FS facts, without pins, paths, identity or key data.
    /// Observation does not acknowledge transport or confer current authority.
    pub fn observation(&self) -> String {
        format!(
            "result:{}; publication:{}; output-validation:{}; output-durability:{}",
            self.operation_result, self.publication, self.output_validation, self.output_durability
        )
    }
    /// Static diagnostic labels for every known operation/output phase. They are
    /// data observations only; failed/uncertain effects must not be discarded.
    pub fn safe_labels(&self) -> Vec<String> {
        vec![
            format!("operator-reported-result:{}", self.operation_result),
            format!("operator-output-publication:{}", self.publication),
            format!("operator-output-validation:{}", self.output_validation),
            format!("operator-output-durability:{}", self.output_durability),
        ]
    }
    /// Successful source read/role/close only. Output progress starts unattempted;
    /// a later target outcome separately changes its result and observations.
    fn new(action: Action, package_pin: &str, artifact_pin: &str) -> Self {
        Self {
            schema_version: 1,
            protocol: "swarm-operator-package-cli-v1",
            operation: action.name(),
            operation_result: "completed",
            operator_package_pin: package_pin.into(),
            service_artifact_pin: artifact_pin.into(),
            controller_uid_matches_operator: true,
            source_read: "decoded",
            source_durability: "directory_acknowledged",
            publication: "not_attempted",
            output_validation: "not_attempted",
            output_durability: "not_acknowledged",
            human_authentication_verified: false,
            current_authority_verified: false,
            service_installed: false,
            service_started: false,
            separate_os_uid_verified: false,
            confinement_verified: false,
        }
    }
    /// Record actual target phases on either success or failure, never infer rollback.
    fn target(&mut self, progress: OutputProgress, success: bool) {
        self.operation_result = if success { "completed" } else { "failed" };
        self.publication = publication(progress.publication);
        self.output_validation = validation(progress.validation);
        self.output_durability = durability(progress.durability);
    }
}
impl fmt::Debug for Report {
    /// Ordinary Debug excludes private pins and other caller data.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OperatorPackageReport { <private> }")
    }
}

/// Typed failure with known semantic/FS report after source custody completed.
/// Input/shape failures expose no usable material; target failure retains effects.
pub enum Error {
    Shape,
    Input(InputFailure),
    Target { report: Report, cause: TargetCause },
}
impl Error {
    /// Static safe classification only, never private values or underlying IO text.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Shape => "operator-operation-shape",
            Self::Input(e) => e.cause.code(),
            Self::Target { cause, .. } => cause.code(),
        }
    }
    /// Known target report, independent of overall failure or transport receipt.
    /// Absence means no source-derived target report was constructed, not rollback.
    pub fn report(&self) -> Option<&Report> {
        match self {
            Self::Target { report, .. } => Some(report),
            _ => None,
        }
    }
    /// Safe read/ack/result facts without pins, paths, key or private IO messages.
    pub fn observation(&self) -> String {
        match self {
            Self::Shape => "no-operation-started".into(),
            Self::Input(e) => format!(
                "source-read:{:?}; source-durability:{:?}",
                e.progress.read, e.progress.durability
            ),
            Self::Target { report, .. } => report.observation(),
        }
    }
}
impl fmt::Display for Error {
    /// Safe category and technical facts only; never format nested private causes.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}; {}", self.code(), self.observation())
    }
}
impl fmt::Debug for Error {
    /// Same redaction as Display, including fixture assertion failures.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Error {
    /// Original primary source; error_links additionally exposes cleanup siblings.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Shape => None,
            Self::Input(e) => Some(e),
            Self::Target { cause, .. } => Some(cause),
        }
    }
}
/// Observe/export/reconcile one independently pinned existing operator package.
/// Source and optional target must be absolute; Check forbids a target and the
/// two handoff operations require one. Actual euid selects descriptor-owned
/// storage and must match protected controller data before artifact derivation.
/// Source closes and business key/proof drop before target IO; only service-only
/// guarded bytes can be published. No key/proof regeneration/current authority,
/// human, namespace, real UID separation or installation is established. File
/// locks/atime/fsync and explicit target create are the described FS effects;
/// publication's random temp nonce is distinct from private material generation.
/// Every refusal preserves original/cleanup causes and known target phases.
pub fn run(
    action: Action,
    source: &Path,
    package_pin: &str,
    output: Option<&Path>,
) -> Result<Report, Error> {
    if !source.is_absolute() {
        return Err(Error::Shape);
    }
    match (action, output) {
        (Action::Check, None) => {}
        (Action::Export | Action::Reconcile, Some(path))
            if path.is_absolute() && path.file_name().is_some() => {}
        _ => return Err(Error::Shape),
    }
    let uid = rustix::process::geteuid().as_raw();
    let prepared = load(source, package_pin, uid).map_err(Error::Input)?;
    let artifact = service_artifact::from_prepared(&prepared).map_err(|cause| {
        Error::Input(InputFailure {
            progress: InputProgress {
                read: package::ReadObservation::Decoded,
                durability: ReadDurability::DirectoryAcknowledged,
            },
            cause: InputCause::Artifact(cause),
        })
    })?;
    drop(prepared);
    let pin = artifact.pin().map_err(|cause| {
        Error::Input(InputFailure {
            progress: InputProgress {
                read: package::ReadObservation::Decoded,
                durability: ReadDurability::DirectoryAcknowledged,
            },
            cause: InputCause::Artifact(cause),
        })
    })?;
    let mut report = Report::new(action, package_pin, &pin);
    if matches!(action, Action::Check) {
        drop(artifact);
        return Ok(report);
    }
    let raw = artifact.encode_private().map_err(|cause| {
        Error::Input(InputFailure {
            progress: InputProgress {
                read: package::ReadObservation::Decoded,
                durability: ReadDurability::DirectoryAcknowledged,
            },
            cause: InputCause::Artifact(cause),
        })
    })?;
    drop(artifact);
    let path = output.ok_or(Error::Shape)?;
    let outcome = target(path, action, &raw, &pin, uid);
    drop(raw);
    match outcome {
        Ok(progress) => {
            report.target(progress, true);
            Ok(report)
        }
        Err((progress, cause)) => {
            report.target(progress, false);
            Err(Error::Target { report, cause })
        }
    }
}
/// Safe labels and every additional error branch for this kernel, its package
/// repository and shared config/IO wrappers. Caller retains cycle detection and
/// ordinary source traversal; IO custom errors and cleanup must not be skipped.
/// This function observes errors only; it never retries or changes state.
pub fn error_links<'a>(
    e: &'a (dyn StdError + 'static),
) -> (Vec<String>, Vec<&'a (dyn StdError + 'static)>) {
    let mut labels = vec![];
    let mut children: Vec<&'a (dyn StdError + 'static)> = vec![];
    if let Some(e) = e.downcast_ref::<Error>() {
        labels.push(format!("operator-core:{}", e.code()));
        if let Some(report) = e.report() {
            labels.extend(report.safe_labels());
        }
    }
    if let Some(e) = e.downcast_ref::<InputFailure>() {
        labels.push(format!("operator-input-read:{:?}", e.progress.read));
        labels.push(format!(
            "operator-input-durability:{:?}",
            e.progress.durability
        ));
    }
    if let Some(e) = e.downcast_ref::<InputCause>() {
        labels.push(format!("operator-input:{}", e.code()));
        if let InputCause::Joined { cleanup, .. } = e {
            children.push(cleanup);
        }
    }
    if let Some(e) = e.downcast_ref::<TargetCause>() {
        labels.push(format!("operator-target:{}", e.code()));
        if let TargetCause::Joined { cleanup, .. } = e {
            children.push(cleanup);
        }
    }
    if let Some(e) = e.downcast_ref::<package::Error>() {
        labels.push(format!("operator-package:{}", e.code()));
        if let Some(cleanup) = e.cleanup_error() {
            children.push(cleanup);
        }
    }
    if let Some(e) = e.downcast_ref::<package::ReadError>() {
        labels.push(format!("operator-package-read:{:?}", e.observation));
        labels.push(format!("operator-package-durability:{:?}", e.durability));
    }
    if let Some(e) = e.downcast_ref::<WriteError>() {
        labels.push(format!(
            "operator-write-publication:{}",
            publication(e.publication)
        ));
    }
    if let Some(e) = e.downcast_ref::<config::Error>() {
        labels.push(format!("enrollment-config:{}", e.code()));
        if let Some(cleanup) = e.cleanup_error() {
            children.push(cleanup);
        }
    }
    if let Some(e) = e.downcast_ref::<std::io::Error>() {
        if let Some(inner) = e.get_ref() {
            children.push(inner);
        }
    }
    (labels, children)
}
#[cfg(test)]
mod tests;
