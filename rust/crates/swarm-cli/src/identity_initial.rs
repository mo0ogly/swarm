//! Explicit dormant operator material creation/reconciliation over private inputs.
//! No Root/account/service/current authority/human/separate-UID proof is inferred.
use super::{identity_provisioning, json_bytes, Arguments, CliError};
use serde::Serialize;
use std::{
    error::Error as StdError,
    fmt,
    io::{self, Write},
    path::Path,
};
use swarm_enrollment_authority::provisioning::{self as material, initial_creation as initial};
use swarm_enrollment_config::store::{
    documents::{Publication, ReadDurability, ReadValidation},
    Context,
};
use zeroize::Zeroizing;
/// Exact operation selector; no create failure falls back to generation or read.
#[derive(Clone, Copy)]
enum Action {
    Create,
    Reconcile,
}
impl Action {
    /// Public fixed labels, protocol vocabulary only, never current authority.
    fn name(self) -> &'static str {
        match self {
            Self::Create => "initial-create",
            Self::Reconcile => "initial-reconcile",
        }
    }
}
/// Closed private metadata only after all input/target owners finished/closed.
/// Pins are explicit private output; no seeds/proof/path/grant serialized. Every
/// unproved authority, locator provenance, actual role separation remains false.
#[derive(Serialize)]
pub struct Report {
    schema_version: i64,
    protocol: &'static str,
    operation: &'static str,
    core_result: &'static str,
    request_pin: String,
    package_pin: String,
    source_request_read: &'static str,
    source_request_durability: &'static str,
    intent_publication: &'static str,
    anchor_publication: &'static str,
    package_publication: &'static str,
    intent_read_validation: &'static str,
    intent_read_durability: &'static str,
    intent_decoded: bool,
    anchor_read_validation: &'static str,
    anchor_read_durability: &'static str,
    anchor_decoded: bool,
    package_read: &'static str,
    package_read_durability: &'static str,
    material_generation_attempted: bool,
    material_generation_completed: bool,
    request_pin_matched: bool,
    package_request_matched: bool,
    locator_trust_verified: bool,
    current_authority_verified: bool,
    human_authentication_verified: bool,
    separate_os_uid_verified: bool,
    confinement_verified: bool,
    service_installed: bool,
    service_started: bool,
}
impl Report {
    /// Core receipt is closed/usable metadata, not live current authority. Source
    /// private reader also closed after canonical decode and local FS ack.
    fn checked(action: Action, receipt: &initial::Receipt) -> Self {
        let p = &receipt.progress;
        Self {
            schema_version: 1,
            protocol: "swarm-initial-creation-cli-v1",
            operation: action.name(),
            core_result: "completed",
            request_pin: receipt.checkpoint.request_pin().into(),
            package_pin: receipt.checkpoint.package_pin().into(),
            source_request_read: "decoded",
            source_request_durability: "directory_acknowledged",
            intent_publication: publication(p.intent_publication),
            anchor_publication: publication(p.anchor_publication),
            package_publication: publication(p.package_publication),
            intent_read_validation: validation(p.intent_read),
            intent_read_durability: durability(p.intent_durability),
            intent_decoded: p.intent_decoded,
            anchor_read_validation: validation(p.anchor_read),
            anchor_read_durability: durability(p.anchor_durability),
            anchor_decoded: p.anchor_decoded,
            package_read: package_read(p.package_read),
            package_read_durability: durability(p.package_durability),
            material_generation_attempted: p.material_generation_attempted,
            material_generation_completed: p.material_generation_completed,
            request_pin_matched: p.request_pin_matched,
            package_request_matched: p.package_request_matched,
            locator_trust_verified: false,
            current_authority_verified: false,
            human_authentication_verified: false,
            separate_os_uid_verified: false,
            confinement_verified: false,
            service_installed: false,
            service_started: false,
        }
    }
}
impl fmt::Debug for Report {
    /// Ordinary diagnostics exclude both private pins and all private source data.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("InitialCreationReport { <private> }")
    }
}
/// Known local publication phase, never implicit rollback or hardware guarantee.
fn publication(p: Publication) -> &'static str {
    match p {
        Publication::NotAttempted => "not_attempted",
        Publication::Unknown => "unknown",
        Publication::Visible => "visible",
        Publication::Durable => "durable",
    }
}
/// Explicit file validation only; trusted-locator reads never become PinMatched.
fn validation(v: ReadValidation) -> &'static str {
    match v {
        ReadValidation::NotAttempted => "not_attempted",
        ReadValidation::ReadUnknown => "read_unknown",
        ReadValidation::BytesRead => "bytes_read",
        ReadValidation::PinMatched => "pin_matched",
    }
}
/// Local FS ack labels, not freshness/current authority/power-loss resistance.
fn durability(v: ReadDurability) -> &'static str {
    match v {
        ReadDurability::NotAcknowledged => "not_acknowledged",
        ReadDurability::FileAcknowledged => "file_acknowledged",
        ReadDurability::DirectoryAcknowledged => "directory_acknowledged",
    }
}
/// Pinned package decode facts, distinct from new material or generation claims.
fn package_read(v: material::repository::ReadObservation) -> &'static str {
    use material::repository::ReadObservation as O;
    match v {
        O::NotAttempted => "not_attempted",
        O::ReadUnknown => "read_unknown",
        O::BytesRead => "bytes_read",
        O::PinMatched => "pin_matched",
        O::Decoded => "decoded",
    }
}
/// Output progress is separate from already completed core publication/close.
#[derive(Clone, Copy)]
pub enum OutputPhase {
    Encode,
    Write,
    Flush,
}
impl OutputPhase {
    /// Protocol facts only, no retry/duration or receiver-ack inference.
    fn name(self) -> &'static str {
        match self {
            Self::Encode => "encode",
            Self::Write => "write-unknown",
            Self::Flush => "written-flush-failed",
        }
    }
}
/// Original private input/core/encoding/output failure, with known durable facts.
pub enum Error {
    Input(identity_provisioning::Error),
    Request(material::Error),
    Core(initial::Failure),
    Output {
        report: Report,
        phase: OutputPhase,
        cause: io::Error,
    },
}
impl Error {
    /// Safe static category, not raw private IO/JSON/Request values.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Input(e) => e.code(),
            Self::Request(e) => e.code(),
            Self::Core(e) => e.cause.code(),
            Self::Output { .. } => "initial-creation-output",
        }
    }
    /// Known safe state for user recovery; errors never imply absent effects.
    pub fn observation(&self) -> String {
        match self{Self::Input(e)=>e.observation(),Self::Request(_)=>"source-read:closed; material-generation:not-attempted".into(),Self::Core(e)=>format!("{:?}",e.progress),Self::Output{report,..}=>format!("initial-intent-publication:{}; initial-anchor-publication:{}; initial-package-publication:{}; initial-material-attempted:{}",report.intent_publication,report.anchor_publication,report.package_publication,report.material_generation_attempted)}
    }
}
impl fmt::Display for Error {
    /// Redact all pins, source paths, raw private causes and generated material.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}; {}", self.code(), self.observation())
    }
}
impl fmt::Debug for Error {
    /// Same redaction as Display, including failure assertions.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Error {
    /// Preserve original typed source; core/CLI matrix exposes independent cleanup.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Input(e) => Some(e),
            Self::Request(e) => Some(e),
            Self::Core(e) => Some(e),
            Self::Output { cause, .. } => Some(cause),
        }
    }
}

/// Exact grammar before any private IO. No stdin/Root/UID/clock/key/output override.
/// Global identity parser additionally rejects duplicate/default forbidden flags.
fn action(a: &Arguments) -> Result<Action, CliError> {
    if a.pos.len() != 5
        || a.input.is_empty()
        || !Path::new(&a.input).is_absolute()
        || !Path::new(&a.pos[3]).is_absolute()
        || !Path::new(&a.pos[4]).is_absolute()
        || Path::new(&a.pos[4]).file_name().is_none()
        || a.root != "."
        || a.server
        || !a.output.is_empty()
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    match a.pos[1].as_str() {
        "initial-create" => Ok(Action::Create),
        "initial-reconcile" => Ok(Action::Reconcile),
        _ => Err(CliError::Usage),
    }
}
/// Only metadata out after every input/target owner closed. Core can have already
/// changed private files; write/flush failure keeps those known effects and never
/// retries creation or reads automatically. Receiver durability is not inferred.
fn emit(report: Report, out: &mut dyn Write) -> Result<u8, Error> {
    let raw = match json_bytes(&report) {
        Ok(v) => v,
        Err(cause) => {
            return Err(Error::Output {
                report,
                phase: OutputPhase::Encode,
                cause,
            })
        }
    };
    if let Err(cause) = out.write_all(&raw) {
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
/// Read existing independently pinned private Request with shared canonical loader
/// and all owners closed before target IO. Its reencoding is byte-identical by
/// decode_request's exact canonical check; a guarded copy survives only until
/// core returns. No grant/current authority or caller locator trust is inferred.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    let selected = action(a)?;
    let result = (|| {
        let request = identity_provisioning::load(Path::new(&a.input), &a.pos[2])
            .map_err(|cause| Error::Input(identity_provisioning::Error::Input(cause)))?;
        let raw = Zeroizing::new(material::encode_request(&request).map_err(Error::Request)?);
        drop(request);
        let ctx = Context::new();
        let result = match selected {
            Action::Create => initial::create(
                &raw,
                &a.pos[2],
                Path::new(&a.pos[3]),
                Path::new(&a.pos[4]),
                &ctx,
            ),
            Action::Reconcile => initial::reconcile(
                &raw,
                &a.pos[2],
                Path::new(&a.pos[3]),
                Path::new(&a.pos[4]),
                &ctx,
            ),
        };
        drop(raw);
        let receipt = result.map_err(Error::Core)?;
        emit(Report::checked(selected, &receipt), out)
    })();
    result.map_err(|cause| CliError::InitialCreation {
        cause: Box::new(cause),
        english: a.language == "en",
    })
}
/// Add safe output/core known phases to established source/custom-IO/cleanup
/// traversal. Caller retains wide-identity cycle checks and ordinary source links.
pub(super) fn error_links<'a>(
    e: &'a (dyn StdError + 'static),
) -> (Vec<String>, Vec<&'a (dyn StdError + 'static)>) {
    let (mut labels, children) = initial::error_links(e);
    if let Some(e) = e.downcast_ref::<Error>() {
        labels.push(format!("initial-cli:{}", e.code()));
        if let Error::Output { report, phase, .. } = e {
            labels.push(format!(
                "initial-intent-publication:{}",
                report.intent_publication
            ));
            labels.push(format!(
                "initial-anchor-publication:{}",
                report.anchor_publication
            ));
            labels.push(format!(
                "initial-package-publication:{}",
                report.package_publication
            ));
            labels.push(format!("initial-transport-phase:{}", phase.name()));
        }
    }
    (labels, children)
}
