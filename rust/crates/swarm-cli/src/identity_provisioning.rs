//! Operator provisioning-request preflight only. Protected pin/read/codec checks
//! create no material, journal, grant, account, project, listener or service.
use super::{json_bytes, Arguments, CliError};
use serde::Serialize;
use std::{
    error::Error as StdError,
    fmt,
    io::{self, Write},
    path::Path,
};
use swarm_enrollment_authority::provisioning as material;
use swarm_enrollment_config::{
    self as config,
    store::{
        self,
        documents::{Documents, DurableReadError, ReadDurability, ReadValidation},
    },
};

/// Actual read/decode observations, not authority or material publication.
#[derive(Clone, Copy)]
struct Progress {
    validation: ReadValidation,
    durability: ReadDurability,
    decoded: bool,
}
impl Default for Progress {
    /// No IO, decode or acknowledgement inferred before the operation.
    fn default() -> Self {
        Self {
            validation: ReadValidation::NotAttempted,
            durability: ReadDurability::NotAcknowledged,
            decoded: false,
        }
    }
}
/// Typed private-input branches; primary and independent cleanup remain separate.
enum InputCause {
    Path,
    Pin,
    Storage(config::Error),
    Read(DurableReadError),
    Request(material::Error),
    Joined {
        cause: Box<InputCause>,
        cleanup: config::Error,
    },
}
impl InputCause {
    /// Static safe classification; nested IO/request text is never formatted.
    fn code(&self) -> &'static str {
        match self {
            Self::Path => "provisioning-input-path",
            Self::Pin => "provisioning-input-pin",
            Self::Storage(e) => e.code(),
            Self::Read(e) => e.cause.code(),
            Self::Request(e) => e.code(),
            Self::Joined { cause, .. } => cause.code(),
        }
    }
}
impl fmt::Display for InputCause {
    /// No paths, pins, principal/grant data or IO text enter ordinary diagnostics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for InputCause {
    /// Same redaction as Display even on fixture/assertion failures.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for InputCause {
    /// Primary source only; error_links also retains independent cleanup.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Storage(e) => Some(e),
            Self::Read(e) => Some(e),
            Self::Request(e) => Some(e),
            Self::Joined { cause, .. } => Some(cause.as_ref()),
            _ => None,
        }
    }
}
/// Failed private preflight retains observed phases, never returns usable input.
pub struct InputFailure {
    progress: Progress,
    cause: InputCause,
}
impl fmt::Display for InputFailure {
    /// Technical state only; decoded=true is not a permission or overall success.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}; validation={:?}, durability={:?}, decoded={}",
            self.cause, self.progress.validation, self.progress.durability, self.progress.decoded
        )
    }
}
impl fmt::Debug for InputFailure {
    /// Same private-data redaction as Display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for InputFailure {
    /// Original typed input tree, including separate release/close failures.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.cause)
    }
}
/// Join original failure and actual release/close without replacing either.
/// A late cleanup refusal drops the successfully decoded value rather than
/// returning success. This helper changes no state and performs no retry.
fn combine<T>(
    result: Result<T, InputCause>,
    cleanup: Result<(), config::Error>,
) -> Result<T, InputCause> {
    match (result, cleanup) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(InputCause::Storage(e)),
        (Err(cause), Err(cleanup)) => Err(InputCause::Joined {
            cause: Box::new(cause),
            cleanup,
        }),
    }
}
/// Read independently pinned bytes with local file+directory acknowledgement,
/// validate by the canonical producer, then finish and close all owned handles.
/// Any late refusal withholds the Request; no entropy or target is ever opened.
/// Read/decode progress survives every primary/release/close failure. OS and
/// caller copies, private lock creation and atime are outside a no-write claim.
fn load_owned(docs: Documents, name: &str, pin: &str) -> Result<material::Request, InputFailure> {
    let mut progress = Progress::default();
    let ctx = store::Context::new();
    let result = (|| {
        let reservation = docs.reserve(&ctx).map_err(InputCause::Storage)?;
        let result = (|| {
            let raw = match reservation.read_private_durable(name, pin) {
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
            let request = material::decode_request(raw.as_bytes()).map_err(InputCause::Request)?;
            progress.decoded = true;
            drop(raw);
            Ok(request)
        })();
        combine(result, reservation.finish())
    })();
    combine(result, docs.close()).map_err(|cause| InputFailure { progress, cause })
}
/// Validate canonical SHA256 and absolute input path before directory IO. Shared
/// Documents enforce current-euid private parent/file, nofollow/single-link/type
/// and retained-descriptor checks; opening can create its private lock only.
/// No request-selected service UID becomes the reader's storage identity.
/// Shared callers may reencode the returned Request: exact canonical bytes were
/// checked before successful cleanup. This data return grants no setup authority.
pub(super) fn load(path: &Path, pin: &str) -> Result<material::Request, InputFailure> {
    let failure = |cause| InputFailure {
        progress: Progress::default(),
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
/// Closed observations after full successful private input cleanup. UID values
/// are declarations, not account existence or matched operator/service identity.
/// Local FS acknowledgement is not freshness, authority or hardware survival.
#[derive(Clone, Serialize)]
pub struct Report {
    schema_version: i64,
    protocol: &'static str,
    operation_result: &'static str,
    request_digest: String,
    service_uid: u32,
    controller_uid: u32,
    request_validated: bool,
    input_durability: &'static str,
    provisioning_started: bool,
    human_authentication_verified: bool,
    current_authority_verified: bool,
    service_started: bool,
    confinement_verified: bool,
}
impl Report {
    /// Project validated explicit data without keys, paths, grant or personal data.
    /// Called only after load succeeded and its owned handles were closed.
    fn checked(request: &material::Request, pin: &str) -> Self {
        Self {
            schema_version: 1,
            protocol: "swarm-provisioning-request-cli-v1",
            operation_result: "completed",
            request_digest: pin.into(),
            service_uid: request.service_uid,
            controller_uid: request.controller_uid,
            request_validated: true,
            input_durability: "directory_acknowledged",
            provisioning_started: false,
            human_authentication_verified: false,
            current_authority_verified: false,
            service_started: false,
            confinement_verified: false,
        }
    }
}
impl fmt::Debug for Report {
    /// Do not leak even the request pin or declared UID values in ordinary logs.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ProvisioningRequestReport { <private> }")
    }
}
/// Actual output phase, distinct from successful request validation.
#[derive(Clone, Copy)]
pub enum OutputPhase {
    Encode,
    Write,
    Flush,
}
impl OutputPhase {
    /// Fixed diagnostic labels, not timeout/retry/operator policy.
    fn name(self) -> &'static str {
        match self {
            Self::Encode => "encode",
            Self::Write => "write-unknown",
            Self::Flush => "written-flush-failed",
        }
    }
}
/// Failed preflight/output never creates material or implies absent output bytes.
pub enum Error {
    Input(InputFailure),
    Output {
        report: Report,
        phase: OutputPhase,
        cause: io::Error,
    },
}
impl Error {
    /// Static classification, never private input contents or IO message.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Input(e) => e.cause.code(),
            Self::Output { .. } => "provisioning-check-output",
        }
    }
    /// Safe known-validation observation, including late input or output failure.
    pub fn observation(&self) -> String {
        match self {
            Self::Input(e) => format!(
                "request-validated:{}; input-durability:{:?}",
                e.progress.decoded, e.progress.durability
            ),
            Self::Output { .. } => "request-validated:true; no-material-created".into(),
        }
    }
}
impl fmt::Display for Error {
    /// Static category and technical facts only; no raw nested error formatting.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}; {}", self.code(), self.observation())
    }
}
impl fmt::Debug for Error {
    /// Same redaction as Display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Error {
    /// Preserve typed input tree or actual output error; custom IO remains traversable.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Input(e) => Some(e),
            Self::Output { cause, .. } => Some(cause),
        }
    }
}
/// Serialize/write/flush only after input owners are closed. Encoding/write/flush
/// refusal retains the validated report and actual cause. No success on empty
/// output, no filesystem publication, retry or authority is inferred.
fn emit(report: Report, out: &mut dyn Write) -> Result<u8, Error> {
    let raw = json_bytes(&report).map_err(|cause| Error::Output {
        report: report.clone(),
        phase: OutputPhase::Encode,
        cause,
    })?;
    out.write_all(&raw).map_err(|cause| Error::Output {
        report: report.clone(),
        phase: OutputPhase::Write,
        cause,
    })?;
    out.flush().map_err(|cause| Error::Output {
        report,
        phase: OutputPhase::Flush,
        cause,
    })?;
    Ok(0)
}
/// Pure closed grammar before private input IO; shared parser rejects duplicate
/// and forbidden flags even with default values. No stdin/project/output alias.
fn validate_arguments(a: &Arguments) -> Result<(), CliError> {
    if a.pos.len() != 3
        || a.pos[1] != "provisioning-check"
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
    Ok(())
}
/// Public operator preflight on a pinned private canonical request. It generates
/// no entropy/material, creates no grant/account/journal/Root and starts no service.
/// Input lock/fsync observations and late output/cleanup failures remain typed;
/// successful data validation never authenticates a human or current authority.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    validate_arguments(a)?;
    let english = a.language == "en";
    let request =
        load(Path::new(&a.input), &a.pos[2]).map_err(|cause| CliError::ProvisioningCheck {
            cause: Box::new(Error::Input(cause)),
            english,
        })?;
    let report = Report::checked(&request, &a.pos[2]);
    drop(request);
    emit(report, out).map_err(|cause| CliError::ProvisioningCheck {
        cause: Box::new(cause),
        english,
    })
}
/// Add secondary input cleanup and known validation/output phases to the common
/// typed/custom-IO traversal. Original IO sources remain unformatted and intact.
pub(super) fn error_links<'a>(
    e: &'a (dyn StdError + 'static),
) -> (Vec<String>, Vec<&'a (dyn StdError + 'static)>) {
    let mut labels = vec![];
    let mut children: Vec<&'a (dyn StdError + 'static)> = vec![];
    if let Some(e) = e.downcast_ref::<Error>() {
        labels.push(format!("provisioning-check:{}", e.code()));
        if let Error::Output { report, phase, .. } = e {
            labels.push(format!(
                "provisioning-request-validated:{}",
                report.request_validated
            ));
            labels.push(format!("provisioning-output-phase:{}", phase.name()));
        }
    }
    if let Some(e) = e.downcast_ref::<InputFailure>() {
        labels.push(format!(
            "provisioning-input-validation:{:?}",
            e.progress.validation
        ));
        labels.push(format!(
            "provisioning-input-durability:{:?}",
            e.progress.durability
        ));
        labels.push(format!(
            "provisioning-request-validated:{}",
            e.progress.decoded
        ));
    }
    if let Some(e) = e.downcast_ref::<InputCause>() {
        labels.push(format!("provisioning-input:{}", e.code()));
        if let InputCause::Joined { cleanup, .. } = e {
            children.push(cleanup);
        }
    }
    (labels, children)
}
#[cfg(test)]
mod tests;
