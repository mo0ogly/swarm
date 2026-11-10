//! Read-only private anchor CLI over explicit trusted-locator core semantics.
//! Caller-supplied locator trust is not verified; no package, key or Root read.
use super::{json_bytes, Arguments, CliError};
use serde::Serialize;
use std::{
    error::Error as StdError,
    fmt,
    io::{self, Write},
    path::Path,
};
use swarm_enrollment_authority::provisioning::{checkpoint, repository};
use swarm_enrollment_config::{
    self as config,
    store::{documents::Documents, Context},
};
/// Original input/schema/close cause; ordinary diagnostics contain no private pins.
pub enum InputCause {
    Path,
    Pin(repository::Error),
    Store(config::Error),
    Read(checkpoint::ReadError),
}
impl InputCause {
    /// Static category only, never nested IO/JSON values or caller paths.
    fn code(&self) -> &'static str {
        match self {
            Self::Path => "anchor-input-path",
            Self::Pin(e) => e.code(),
            Self::Store(e) => e.code(),
            Self::Read(e) => e.cause.code(),
        }
    }
}
impl fmt::Display for InputCause {
    /// Redact private data while preserving original typed source separately.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for InputCause {
    /// Same redaction as Display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for InputCause {
    /// Preserve original schema/storage/read and independent nested cleanup causes.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Path => None,
            Self::Pin(e) => Some(e),
            Self::Store(e) => Some(e),
            Self::Read(e) => Some(e),
        }
    }
}
/// Closed private data projection after read/decode/binding/release/close succeed.
/// Actual local acknowledgement is distinct from caller locator trust/current
/// authority: every unproved authority or package/material effect stays false.
#[derive(Serialize)]
pub struct Report {
    schema_version: i64,
    protocol: &'static str,
    operation: &'static str,
    request_pin: String,
    package_pin: String,
    read_validation: &'static str,
    read_durability: &'static str,
    anchor_decoded: bool,
    locator_trust_verified: bool,
    current_authority_verified: bool,
    human_authentication_verified: bool,
    separate_os_uid_verified: bool,
    operator_package_read: bool,
    material_generated: bool,
    service_installed: bool,
}
impl Report {
    /// Called only after core load has acknowledged and closed all owners. This is
    /// data matching, not proof that the caller selected a trustworthy anchor.
    fn checked(value: &checkpoint::Checkpoint) -> Self {
        Self {
            schema_version: 1,
            protocol: "swarm-operator-anchor-cli-v1",
            operation: "checkpoint-check",
            request_pin: value.request_pin().into(),
            package_pin: value.package_pin().into(),
            read_validation: "bytes_read",
            read_durability: "directory_acknowledged",
            anchor_decoded: true,
            locator_trust_verified: false,
            current_authority_verified: false,
            human_authentication_verified: false,
            separate_os_uid_verified: false,
            operator_package_read: false,
            material_generated: false,
            service_installed: false,
        }
    }
}
impl fmt::Debug for Report {
    /// Explicit private pins never enter ordinary debug/error output.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AnchorCheckReport { <private> }")
    }
}
/// Known output transport phase, never receiver/persistence acknowledgement.
#[derive(Clone, Copy)]
pub enum OutputPhase {
    Encode,
    Write,
    Flush,
}
impl OutputPhase {
    /// Fixed labels are protocol observations, not time/retry/operator policy.
    fn name(self) -> &'static str {
        match self {
            Self::Encode => "encode",
            Self::Write => "write-unknown",
            Self::Flush => "written-flush-failed",
        }
    }
}
/// Input refusal or completed read facts preserved after output failure.
pub enum Error {
    Input(InputCause),
    Output {
        report: Report,
        phase: OutputPhase,
        cause: io::Error,
    },
}
impl Error {
    /// Stable safe category, independent of private input or raw error wording.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Input(e) => e.code(),
            Self::Output { .. } => "operator-anchor-output",
        }
    }
    /// Safe read/ack/decode facts only; no pin/path/current-authority inference.
    pub fn observation(&self) -> String {
        match self{Self::Input(InputCause::Read(e))=>format!("anchor-read-validation:{:?}; anchor-read-durability:{:?}; anchor-decoded:{}",e.validation,e.durability,e.decoded),Self::Input(_)=>"anchor-read:not-completed".into(),Self::Output{..}=>"anchor-read-validation:bytes_read; anchor-read-durability:directory_acknowledged; anchor-decoded:true".into()}
    }
}
impl fmt::Display for Error {
    /// Never interpolate nested private IO/JSON messages or report pins.
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
    /// Primary original source; shared traversal adds independent cleanup/IO inner.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Input(e) => Some(e),
            Self::Output { cause, .. } => Some(cause),
        }
    }
}
/// Private path and request digest before IO; actual euid selects ownership.
/// Core load acknowledges, validates and closes every owner before usable return.
/// The caller-protected locator remains a precondition, never inferred from pins.
fn load(path: &Path, pin: &str) -> Result<checkpoint::Checkpoint, InputCause> {
    repository::Pin::parse(pin).map_err(InputCause::Pin)?;
    if !path.is_absolute() {
        return Err(InputCause::Path);
    }
    let parent = path.parent().ok_or(InputCause::Path)?;
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or(InputCause::Path)?;
    let docs =
        Documents::open(parent, rustix::process::geteuid().as_raw()).map_err(InputCause::Store)?;
    checkpoint::load(docs, name, pin, &Context::new()).map_err(InputCause::Read)
}
/// Serialize/write/flush after input closure. Every late failure retains known
/// read/ack report, without claiming receiver receipt or retrying private IO.
fn emit(report: Report, out: &mut dyn Write) -> Result<u8, Error> {
    let bytes = match json_bytes(&report) {
        Ok(v) => v,
        Err(cause) => {
            return Err(Error::Output {
                report,
                phase: OutputPhase::Encode,
                cause,
            })
        }
    };
    if let Err(cause) = out.write_all(&bytes) {
        return Err(Error::Output {
            report,
            phase: OutputPhase::Write,
            cause,
        });
    }
    if let Err(cause) = out.flush() {
        return Err(Error::Output {
            report,
            phase: OutputPhase::Flush,
            cause,
        });
    }
    Ok(0)
}
/// Exact closed read-only grammar; no stdin, Root, UID, policy or output override.
/// Global identity parser also rejects duplicate/default-valued forbidden flags.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 3
        || a.pos[1] != "checkpoint-check"
        || a.input.is_empty()
        || !Path::new(&a.input).is_absolute()
        || a.root != "."
        || a.server
        || !a.output.is_empty()
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let result = load(Path::new(&a.input), &a.pos[2])
        .map_err(Error::Input)
        .and_then(|record| emit(Report::checked(&record), out));
    result.map_err(|cause| CliError::CheckpointCheck {
        cause: Box::new(cause),
        english: a.language == "en",
    })
}
/// Add safe completed-read/output facts and every core cleanup/custom IO branch.
/// Caller supplies ordinary source traversal and wide-identity cycle detection.
pub(super) fn error_links<'a>(
    e: &'a (dyn StdError + 'static),
) -> (Vec<String>, Vec<&'a (dyn StdError + 'static)>) {
    let (mut labels, children) = checkpoint::error_links(e);
    if let Some(e) = e.downcast_ref::<Error>() {
        labels.push(format!("operator-anchor-cli:{}", e.code()));
        if let Error::Output { report, phase, .. } = e {
            labels.push(format!("anchor-decoded:{}", report.anchor_decoded));
            labels.push(format!("anchor-read-durability:{}", report.read_durability));
            labels.push(format!("anchor-transport-phase:{}", phase.name()));
        }
    }
    (labels, children)
}
