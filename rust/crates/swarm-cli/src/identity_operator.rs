//! Exact operator CLI grammar and observable output over the shared handoff kernel.
//! No private filesystem logic or authority policy is duplicated here.
use super::{json_bytes, Arguments, CliError};
use std::{
    error::Error as StdError,
    fmt,
    io::{self, Write},
    path::Path,
};
use swarm_enrollment_authority::provisioning::operator_handoff::{self as core, Action, Report};
/// Known transport phase remains distinct from target publication/read state.
#[derive(Clone, Copy)]
pub enum OutputPhase {
    Encode,
    Write,
    Flush,
}
impl OutputPhase {
    /// Fixed safe labels; no retry/deadline policy or receiver acknowledgement.
    fn name(self) -> &'static str {
        match self {
            Self::Encode => "encode",
            Self::Write => "write-unknown",
            Self::Flush => "written-flush-failed",
        }
    }
}
/// Original failure, known report and independent output cause remain available.
pub enum Error {
    Operation(core::Error),
    Output {
        report: Report,
        phase: OutputPhase,
        cause: io::Error,
        failure: Option<Box<core::Error>>,
    },
}
impl Error {
    /// Safe static classification, never original private IO text or pins.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Operation(e) => e.code(),
            Self::Output { .. } => "operator-package-output",
        }
    }
    /// Known report survives late serialization/write/flush failure; Input has none.
    fn report(&self) -> Option<&Report> {
        match self {
            Self::Operation(e) => e.report(),
            Self::Output { report, .. } => Some(report),
        }
    }
    /// Safe technical state only; any error still forbids a successful overall result.
    pub fn observation(&self) -> String {
        match self {
            Self::Operation(e) => e.observation(),
            Self::Output { report, .. } => report.observation(),
        }
    }
}
impl fmt::Display for Error {
    /// Exclude every pin/path/key/subject and nested IO message.
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
    /// Primary typed cause; output's earlier failure and secondary cleanup are
    /// additional branches available through error_links, not source alone.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Operation(e) => Some(e),
            Self::Output { cause, .. } => Some(cause),
        }
    }
}
/// Emit only after source/target owners and private buffers are gone. A target
/// failure remains failure even if its report writes successfully; an output
/// failure preserves both known report and earlier cause, with no implicit retry.
fn emit(report: Report, failure: Option<core::Error>, out: &mut dyn Write) -> Result<u8, Error> {
    let bytes = match json_bytes(&report) {
        Ok(v) => v,
        Err(cause) => {
            return Err(Error::Output {
                report,
                phase: OutputPhase::Encode,
                cause,
                failure: failure.map(Box::new),
            })
        }
    };
    if let Err(cause) = out.write_all(&bytes) {
        return Err(Error::Output {
            report,
            phase: OutputPhase::Write,
            cause,
            failure: failure.map(Box::new),
        });
    }
    if let Err(cause) = out.flush() {
        return Err(Error::Output {
            report,
            phase: OutputPhase::Flush,
            cause,
            failure: failure.map(Box::new),
        });
    }
    match failure {
        Some(e) => Err(Error::Operation(e)),
        None => Ok(0),
    }
}
/// Pure exact grammar before IO. Shared parser forbids default-valued aliases
/// and duplicate flags. Only absolute input directory and, for exports, absolute
/// file output are accepted. No Root/project/clock/UID/key overrides or stdin.
fn action(a: &Arguments) -> Result<Action, CliError> {
    let kind = a
        .pos
        .get(1)
        .and_then(|s| Action::parse(s))
        .ok_or(CliError::Usage)?;
    let count = if matches!(kind, Action::Check) { 3 } else { 4 };
    if a.pos.len() != count
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
    if count == 4
        && (!Path::new(&a.pos[3]).is_absolute() || Path::new(&a.pos[3]).file_name().is_none())
    {
        return Err(CliError::Usage);
    }
    Ok(kind)
}
/// Validate exact argv then invoke the shared pinned handoff kernel. Source and
/// target custodians are gone before report output. A late write/flush failure
/// retains known effects and any prior core failure; it never triggers retry.
/// The kernel establishes neither human/current authority nor service admission.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    let action = action(a)?;
    let output = if matches!(action, Action::Check) {
        None
    } else {
        Some(Path::new(&a.pos[3]))
    };
    let result = core::run(action, Path::new(&a.input), &a.pos[2], output);
    let result = match result {
        Ok(report) => emit(report, None, out),
        Err(cause) => match cause.report().cloned() {
            Some(report) => emit(report, Some(cause), out),
            None => Err(Error::Operation(cause)),
        },
    };
    result.map_err(|cause| CliError::OperatorPackage {
        cause: Box::new(cause),
        english: a.language == "en",
    })
}
/// Extend shared kernel traversal with transport facts and earlier failure.
/// Caller supplies cycle detection/source traversal; labels redact private data.
pub(super) fn error_links<'a>(
    e: &'a (dyn StdError + 'static),
) -> (Vec<String>, Vec<&'a (dyn StdError + 'static)>) {
    let (mut labels, mut children) = core::error_links(e);
    if let Some(e) = e.downcast_ref::<Error>() {
        labels.push(format!("operator-cli:{}", e.code()));
        if let Some(report) = e.report() {
            labels.extend(report.safe_labels());
        }
        if let Error::Output { phase, failure, .. } = e {
            labels.push(format!("operator-transport-phase:{}", phase.name()));
            if let Some(cause) = failure {
                children.push(cause.as_ref());
            }
        }
    }
    (labels, children)
}
#[cfg(test)]
mod tests;
