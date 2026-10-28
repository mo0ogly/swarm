//! Trusted local operator adapter, not an admitted Unix peer or personal daemon.
//! Startup inputs must be independently protected; no RootStore is opened.
use super::{personal_secret_frame as input, Arguments, CliError};
use std::{
    error::Error as StdError,
    fmt,
    io::{self, IsTerminal, Read, Write},
    path::Path,
};
use swarm_enrollment_authority::{
    authenticated_control::{CloseError, Context},
    control,
    personal::coordinator::UseError,
    personal_service::{self as core, OutcomeData, Reply},
    service_request as request,
    service_response::{self as response, Observation, OutputPhase, Sent},
};

/// Complete known operation/close/output branches. No formatted payload or raw
/// private OS/parser text; presence of Err never means absent earlier effects.
pub enum Error {
    Terminal,
    Input(input::Error),
    Request(request::Error),
    Target,
    Startup(core::Error),
    Dispatch {
        cause: core::Error,
        close: Option<CloseError>,
    },
    Operation {
        sent: Sent,
        close: Option<CloseError>,
    },
    Output {
        cause: response::SendError,
        close: Option<CloseError>,
    },
}
impl Error {
    /// Safe static code; inspect known observation and every independent cause.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Terminal => "service-call-stream-required",
            Self::Input(e) => e.code(),
            Self::Request(e) => e.code(),
            Self::Target => "service-call-target",
            Self::Startup(e) => e.code(),
            Self::Dispatch { cause, .. } => cause.code(),
            Self::Operation { sent, .. } if !sent.observation.completed() => {
                "service-call-operation"
            }
            Self::Operation { .. } => "service-call-close",
            Self::Output { cause, .. } => match cause.phase {
                OutputPhase::Encode => "service-call-output-encode",
                OutputPhase::WriteUnknown => "service-call-output-write",
                OutputPhase::WrittenFlushFailed => "service-call-output-flush",
            },
        }
    }
    /// Borrow actual retained operation observation after operation/output error.
    /// No fabricated report for startup/input refusal before dispatch.
    fn known(&self) -> Option<&Observation> {
        match self {
            Self::Operation { sent, .. } => Some(&sent.observation),
            Self::Output { cause, .. } => Some(&cause.observation),
            _ => None,
        }
    }
    /// Safe operation/close summary for stdout loss. Known completed operation may
    /// coexist with failed close/output; never infer rollback or automatic retry.
    pub fn observation(&self) -> String {
        let close = match self {
            Self::Dispatch { close, .. }
            | Self::Operation { close, .. }
            | Self::Output { close, .. } => {
                if close.is_some() {
                    "failed"
                } else {
                    "closed"
                }
            }
            _ => "not_attempted",
        };
        format!(
            "{}; close={close}",
            self.known()
                .map(Observation::diagnostic_summary)
                .unwrap_or_else(|| "operation=not_reported".into())
        )
    }
}
impl fmt::Display for Error {
    /// Static category/status only, never nested cause text or input data.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "personal service call: {}; {}",
            self.code(),
            self.observation()
        )
    }
}
impl fmt::Debug for Error {
    /// Same redaction as Display while all typed branches remain retained.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Error {
    /// Primary output/startup/input/close cause. Operation Reply and independent
    /// close remain mandatory branches beyond source(), exposed in error_links.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Input(e) => Some(e),
            Self::Request(e) => Some(e),
            Self::Startup(e) => Some(e),
            Self::Dispatch { cause, .. } => Some(cause),
            Self::Output { cause, .. } => Some(cause),
            Self::Operation { close: Some(e), .. } => Some(e),
            _ => None,
        }
    }
}
/// Existing parser has checked explicit option presence/uniqueness. Pure command
/// shape/canonical pin/absolute startup paths refuse before stdin or filesystem.
fn validate(a: &Arguments) -> Result<(), CliError> {
    if a.pos.len() != 4
        || a.pos[1] != "service-call"
        || a.input.is_empty()
        || a.input == "-"
        || !Path::new(&a.input).is_absolute()
        || !Path::new(&a.pos[3]).is_absolute()
        || !swarm_identity_contract::digest(a.pos[2].as_bytes())
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
/// Dispatch exactly once from guarded input, after target comparison before
/// startup IO. Drop input before consuming close and output. A successful JSON
/// send preserves a failed business operation as nonzero; close failure remains
/// independent even with completed=true. No RootStore/listener/session or retry.
fn run(a: &Arguments, reader: &mut dyn Read, out: &mut dyn Write) -> Result<u8, Error> {
    let packet = input::read_packet(reader).map_err(Error::Input)?;
    let decoded = request::decode(&packet).map_err(Error::Request)?;
    if decoded.target_pin() != a.pos[2] {
        return Err(Error::Target);
    }
    let mut service = core::Service::open(
        &swarm_enrollment_state::Context::new(),
        &a.input,
        &a.pos[2],
        Path::new(&a.pos[3]),
    )
    .map_err(Error::Startup)?;
    let result = service.dispatch(&Context::new(), decoded);
    drop(packet);
    let close = service.close().err();
    let reply = match result {
        Ok(reply) => reply,
        Err(cause) => return Err(Error::Dispatch { cause, close }),
    };
    finish(reply, close, out)
}
/// Retain complete original operation and both storage closes through write/flush.
/// Completed reports only the operation; output success does not clear failures.
fn finish(reply: Reply, close: Option<CloseError>, out: &mut dyn Write) -> Result<u8, Error> {
    let sent = match response::send(out, reply) {
        Ok(sent) => sent,
        Err(cause) => return Err(Error::Output { cause, close }),
    };
    if !sent.observation.completed() || close.is_some() {
        Err(Error::Operation { sent, close })
    } else {
        Ok(0)
    }
}
/// Public one-shot local operator command. Terminal stdin refuses before locking/
/// reading to avoid echo; binary complete packet only, no secret argv/env/file.
/// stdout is canonical47Observation (operation status); exit/stderr also report
/// close/output failure. This does not start a service or authenticate a human.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    validate(a)?;
    let result = if io::stdin().is_terminal() {
        Err(Error::Terminal)
    } else {
        run(a, &mut io::stdin().lock(), out)
    };
    result.map_err(|cause| CliError::ServiceCall {
        cause: Box::new(cause),
        english: a.language == "en",
    })
}
/// Original operation errors inside a retained Reply, including both AfterUse
/// authority and inner business failure. No formatting or mutation is performed.
fn reply_errors(reply: &Reply) -> Vec<&(dyn StdError + 'static)> {
    let mut children: Vec<&(dyn StdError + 'static)> = vec![];
    match &reply.outcome {
        OutcomeData::Initialized(Err(e)) | OutcomeData::Account(Err(e)) => children.push(e),
        OutcomeData::Control(Ok(Err(e))) => children.push(e),
        OutcomeData::Control(Err(e)) => children.push(e),
        _ => {}
    };
    children
}
/// Extend the established primary/customIO/independent-close traversal. Original
/// Reply causes are separate from output source(), and AfterUse.outcome must be
/// followed even when personal authority already failed. No raw text is emitted.
pub(super) fn error_links<'a>(
    e: &'a (dyn StdError + 'static),
) -> (Vec<String>, Vec<&'a (dyn StdError + 'static)>) {
    let mut labels = vec![];
    let mut children: Vec<&(dyn StdError + 'static)> = vec![];
    if let Some(e) = e.downcast_ref::<Error>() {
        labels.push(format!("personal-service-call:{}", e.code()));
        labels.push(format!(
            "personal-service-call-observation:{}",
            e.observation()
        ));
        match e {
            Error::Operation { sent, close } => {
                children.extend(reply_errors(&sent.reply));
                if let Some(c) = close {
                    children.push(c)
                }
            }
            Error::Output { close, .. } | Error::Dispatch { close, .. } => {
                if let Some(c) = close {
                    children.push(c)
                }
            }
            _ => {}
        }
    }
    if let Some(e) = e.downcast_ref::<response::SendError>() {
        children.extend(reply_errors(&e.reply));
        labels.push(format!("personal-service-output:{:?}", e.phase));
    }
    if let Some(UseError::AfterUse {
        outcome: Err(inner),
        ..
    }) = e.downcast_ref::<UseError<control::Response, control::Error>>()
    {
        children.push(inner);
    }
    if let Some(e) = e.downcast_ref::<request::Error>() {
        labels.push(format!("personal-service-request:{}", e.code()));
    }
    if let Some(e) = e.downcast_ref::<core::Error>() {
        labels.push(format!("personal-service-core:{}", e.code()));
    }
    if let Some(e) = e.downcast_ref::<response::consumer::Error>() {
        labels.push(format!("personal-service-response:{}", e.code()));
    }
    (labels, children)
}
#[cfg(test)]
mod tests;
