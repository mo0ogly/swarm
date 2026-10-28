//! Native operator lifecycle commands, not a listener, human presence or grant.
//! Startup paths/pin must be independently protected. Secrets arrive only on the
//! enrollment byte stream; no RootStore, provider, new key or bootstrap is created.
use super::{json_bytes, personal_secret_frame as frame, Arguments, CliError};
use serde::Serialize;
use std::{
    error::Error as StdError,
    fmt,
    io::{self, IsTerminal, Read, Write},
    path::Path,
};
use swarm_enrollment_authority::{
    authenticated_control::CloseError,
    authenticated_startup as startup,
    personal::{
        self,
        coordinator::{self, CommitError, JournalOutcome, Progress},
    },
    protected_signer,
};
use swarm_enrollment_config::store::documents::Publication;
/// Closed command vocabulary, not permissions or an operator execution policy.
#[derive(Clone, Copy)]
enum Action {
    Initialize,
    Enroll,
    Recover,
}
impl Action {
    /// Public protocol operation name, never copied from arbitrary input.
    fn name(self) -> &'static str {
        match self {
            Self::Initialize => "personal-init",
            Self::Enroll => "personal-enroll",
            Self::Recover => "personal-recover",
        }
    }
}
/// Public observation only, never accepted as authority or proof of a human.
/// No credential digest, password, installation proof, key or private head digest.
/// Private fields prevent construction of arbitrary diagnostic labels by callers.
#[derive(Clone, Debug, Serialize)]
pub struct Report {
    schema_version: i64,
    protocol: &'static str,
    operation: &'static str,
    operation_result: &'static str,
    intent_publication: &'static str,
    journal: &'static str,
    checkpoint_publication: &'static str,
    sequence: Option<i64>,
    close: &'static str,
    human_presence_verified: bool,
    service_started: bool,
    confinement_verified: bool,
}
/// Exact publication vocabulary, not a retry/rollback decision or hardware proof.
fn publication(p: Publication) -> &'static str {
    match p {
        Publication::NotAttempted => "not_attempted",
        Publication::Unknown => "unknown",
        Publication::Visible => "visible",
        Publication::Durable => "durable",
    }
}
/// Keep read acknowledgement distinct from new apply and unknown publication.
/// Only the sequence is public; the private digest is never projected.
fn journal(j: &JournalOutcome) -> (&'static str, Option<i64>) {
    match j {
        JournalOutcome::NotAttempted => ("not_attempted", None),
        JournalOutcome::Unknown => ("unknown", None),
        JournalOutcome::Acknowledged(h) => ("acknowledged", Some(h.sequence)),
        JournalOutcome::Observed(h) => ("observed", Some(h.sequence)),
    }
}
impl Report {
    /// Incomplete metadata before close/output; no action or authority performed.
    fn base(action: Action) -> Self {
        Self {
            schema_version: 1,
            protocol: "swarm-personal-lifecycle-cli-v1",
            operation: action.name(),
            operation_result: "not_reported",
            intent_publication: "not_reported",
            journal: "not_reported",
            checkpoint_publication: "not_reported",
            sequence: None,
            close: "not_attempted",
            human_presence_verified: false,
            service_started: false,
            confinement_verified: false,
        }
    }
    /// Project only outcomes established by the corresponding successful API.
    /// Recover may apply pending bytes or observe a committed account; the API
    /// does not expose which on success, so current_verified never infers apply.
    fn completed(action: Action, sequence: Option<i64>) -> Self {
        let mut r = Self::base(action);
        r.operation_result = "completed";
        r.sequence = sequence;
        r.checkpoint_publication = "durable";
        match action {
            Action::Initialize => {
                r.intent_publication = "not_attempted";
                r.journal = "not_attempted"
            }
            Action::Enroll => {
                r.intent_publication = "durable";
                r.journal = "acknowledged"
            }
            Action::Recover => r.journal = "current_verified",
        };
        r
    }
    /// Preserve all reported phases even after an error; failed never means that
    /// the journal or filesystem is unchanged. No digest or private cause text.
    fn failed(action: Action, p: &Progress) -> Self {
        let mut r = Self::base(action);
        r.operation_result = "failed";
        r.intent_publication = publication(p.intent);
        r.checkpoint_publication = publication(p.checkpoint);
        let (j, s) = journal(&p.journal);
        r.journal = j;
        r.sequence = s;
        r
    }
    /// Safe state summary for stdout-loss/late-close diagnostics. This is metadata,
    /// never a grant, an instruction to retry or a statement of absent effects.
    fn summary(&self) -> String {
        format!(
            "operation={}, result={}, journal={}, checkpoint={}, close={}",
            self.operation,
            self.operation_result,
            self.journal,
            self.checkpoint_publication,
            self.close
        )
    }
}
/// Closed output phase; no caller-provided label can enter a diagnostic.
#[derive(Clone, Copy, Debug)]
pub enum OutputPhase {
    Encode,
    Write,
    Flush,
}
impl OutputPhase {
    /// Stable protocol phase name, not operator policy or private IO text.
    fn name(self) -> &'static str {
        match self {
            Self::Encode => "encode",
            Self::Write => "write",
            Self::Flush => "flush",
        }
    }
}
/// Original refusals plus every independent late close/output branch. Ordinary
/// formatting is redacted; report retains known phases even when stdout failed.
pub enum Error {
    Frame(frame::Error),
    Startup(startup::Error),
    State,
    InputStreamRequired,
    Operation {
        cause: Box<CommitError>,
        close: Option<CloseError>,
        report: Report,
    },
    Close {
        cause: CloseError,
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
    /// Safe category only, including original frame/startup/operation classification.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Frame(e) => e.code(),
            Self::Startup(e) => e.code(),
            Self::State => "personal-lifecycle-state",
            Self::InputStreamRequired => "personal-input-stream-required",
            Self::Operation { cause, .. } => cause.cause.code(),
            Self::Close { .. } => "personal-lifecycle-close",
            Self::Output { .. } => "personal-lifecycle-output",
        }
    }
    /// Retained known operation/publication/close observations. None means the
    /// lifecycle operation was not reached, not absence of startup lock/sidecars.
    pub fn report(&self) -> Option<&Report> {
        match self {
            Self::Operation { report, .. }
            | Self::Close { report, .. }
            | Self::Output { report, .. } => Some(report),
            _ => None,
        }
    }
    /// Public-safe summary without private IO strings, paths, hashes or input.
    pub fn observation(&self) -> String {
        self.report().map(Report::summary).unwrap_or_else(|| {
            "lifecycle operation not started; startup effects not inferred".into()
        })
    }
}
impl fmt::Display for Error {
    /// Never format private nested causes; structured originals remain accessible.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "personal lifecycle: {}", self.code())
    }
}
impl fmt::Debug for Error {
    /// Same redaction as Display, including custom IO and checkpoint payloads.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Error {
    /// Primary cause only; error_links also retains close and prior operation
    /// failures after output loss. Source traversal alone is not sufficient.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Frame(e) => Some(e),
            Self::Startup(e) => Some(e),
            Self::Operation { cause, .. } => Some(cause.as_ref()),
            Self::Close { cause, .. } => Some(cause),
            Self::Output { cause, .. } => Some(cause),
            _ => None,
        }
    }
}
/// Reject wrong shape/default/project fields before any input or source access.
/// The shared parser has already enforced option presence/uniqueness for identity.
fn validated_action(a: &Arguments) -> Result<Action, CliError> {
    if a.pos.len() != 4
        || a.input.is_empty()
        || a.input == "-"
        || a.root != "."
        || a.server
        || !a.output.is_empty()
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    match a.pos[1].as_str() {
        "personal-init" => Ok(Action::Initialize),
        "personal-enroll" => Ok(Action::Enroll),
        "personal-recover" => Ok(Action::Recover),
        _ => Err(CliError::Usage),
    }
}
/// Correlate validated action and borrowed secret data before source acquisition.
/// This carries knowledge input only; current bootstrap verifies the actual proof.
enum Input<'a> {
    Initialize,
    Enroll(&'a [u8; 32], &'a [u8]),
    Recover,
}
/// Finish stdout and flush after both storage closes. Every encode/write/flush
/// refusal retains the complete known report and any earlier failure/close tree.
fn emit(report: Report, failure: Option<Error>, out: &mut dyn Write) -> Result<u8, Error> {
    let bytes = match json_bytes(&report) {
        Ok(v) => v,
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
/// Validate frame/operation correlation before startup, use only the startup-owned
/// signer/heads, explicitly close on every operation outcome and drop secret input
/// before closing/output. No first-caller provisioning, RootStore or automatic retry.
/// Borrowed input FD remains caller-owned; internal byte buffers zeroize on drop.
fn run_action(
    a: &Arguments,
    action: Action,
    reader: &mut dyn Read,
    out: &mut dyn Write,
) -> Result<u8, Error> {
    let pin = a.pos.get(2).ok_or(Error::State)?;
    let key = a.pos.get(3).ok_or(Error::State)?;
    let frame = if matches!(action, Action::Enroll) {
        Some(frame::Frame::read(reader).map_err(Error::Frame)?)
    } else {
        None
    };
    let parts = frame
        .as_ref()
        .map(frame::Frame::parts)
        .transpose()
        .map_err(Error::Frame)?;
    let input = match (action, parts) {
        (Action::Initialize, None) => Input::Initialize,
        (Action::Enroll, Some((s, p))) => Input::Enroll(s, p),
        (Action::Recover, None) => Input::Recover,
        _ => return Err(Error::State),
    };
    let app = startup::open_lifecycle(
        &swarm_enrollment_state::Context::new(),
        &a.input,
        pin,
        Path::new(key),
    )
    .map_err(Error::Startup)?;
    let ctx = swarm_enrollment_authority::authenticated_control::Context::new();
    let outcome = match input {
        Input::Initialize => app.initialize(&ctx).map(|()| None),
        Input::Enroll(s, p) => app.enroll(&ctx, s, p).map(|a| Some(a.head().sequence)),
        Input::Recover => app.recover(&ctx).map(|a| Some(a.head().sequence)),
    };
    let mut report = match &outcome {
        Ok(sequence) => Report::completed(action, *sequence),
        Err(e) => Report::failed(action, &e.progress),
    };
    drop(frame);
    let close = app.close().err();
    report.close = if close.is_some() { "failed" } else { "closed" };
    let failure = match (outcome, close) {
        (Err(cause), close) => Some(Error::Operation {
            cause: Box::new(cause),
            close,
            report: report.clone(),
        }),
        (Ok(_), Some(cause)) => Some(Error::Close {
            cause,
            report: report.clone(),
        }),
        (Ok(_), None) => None,
    };
    emit(report, failure, out)
}
/// Public native command adapter. Binary enrollment rejects terminal stdin to
/// avoid echoing a typed secret; no interactive prompt is implemented. This is
/// transport grammar, not an execution policy. Init/recover never read/lock stdin; enrollment
/// borrows its versioned byte stream only after pure command validation. Paths/pin
/// are protected operator startup inputs, not a remotely supplied actor/context.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    let action = validated_action(a)?;
    let result = match action {
        Action::Enroll if io::stdin().is_terminal() => Err(Error::InputStreamRequired),
        Action::Enroll => run_action(a, action, &mut io::stdin().lock(), out),
        _ => run_action(a, action, &mut io::empty(), out),
    };
    result.map_err(|cause| CliError::PersonalLifecycle {
        cause: Box::new(cause),
        english: a.language == "en",
    })
}
/// Retain exact progress labels, never the credential/head digest or failure text.
fn progress_labels(p: &Progress, out: &mut Vec<String>) {
    out.push(format!(
        "personal-progress-intent:{}",
        publication(p.intent)
    ));
    out.push(format!(
        "personal-progress-checkpoint:{}",
        publication(p.checkpoint)
    ));
    let (j, s) = journal(&p.journal);
    out.push(format!("personal-progress-journal:{j}"));
    if let Some(s) = s {
        out.push(format!("personal-progress-sequence:{s}"))
    }
}
/// Traverse every new personal/startup/key/close branch in addition to source(),
/// including errors wrapped inside custom IO. Pure observation, no recovery/grant.
/// Existing control/config/state/IO families remain in their established matrices.
pub(super) fn error_links<'a>(
    e: &'a (dyn StdError + 'static),
) -> (Vec<String>, Vec<&'a (dyn StdError + 'static)>) {
    let mut labels = vec![];
    let mut children: Vec<&'a (dyn StdError + 'static)> = vec![];
    if let Some(e) = e.downcast_ref::<Error>() {
        labels.push(format!("personal-cli:{}", e.code()));
        if let Some(r) = e.report() {
            labels.push(format!("personal-reported-result:{}", r.operation_result));
            labels.push(format!("personal-reported-journal:{}", r.journal));
            labels.push(format!(
                "personal-reported-checkpoint:{}",
                r.checkpoint_publication
            ));
            labels.push(format!("personal-reported-close:{}", r.close));
            if let Some(s) = r.sequence {
                labels.push(format!("personal-reported-sequence:{s}"))
            }
        }
        match e {
            Error::Operation { close: Some(c), .. } => children.push(c),
            Error::Output { failure, phase, .. } => {
                labels.push(format!("personal-output-phase:{}", phase.name()));
                if let Some(e) = failure {
                    children.push(e.as_ref())
                }
            }
            _ => {}
        }
    }
    if let Some(e) = e.downcast_ref::<startup::Error>() {
        labels.push(format!("authenticated-startup:{}", e.code()));
        if let Some(c) = e.source_cleanup() {
            children.push(c)
        }
        if let Some(c) = e.coordinator_cleanup() {
            children.push(c)
        }
    }
    if let Some(e) = e.downcast_ref::<protected_signer::Error>() {
        labels.push(format!("protected-signer:{}", e.code()));
        if let protected_signer::Error::Joined { cleanup, .. } = e {
            children.push(cleanup.as_ref())
        }
    }
    if let Some(e) = e.downcast_ref::<coordinator::Error>() {
        labels.push(format!("personal-coordinator:{}", e.code()));
        if let Some(c) = e.cleanup_error() {
            children.push(c)
        }
    }
    if let Some(e) = e.downcast_ref::<CommitError>() {
        progress_labels(&e.progress, &mut labels)
    }
    if let Some(e) = e.downcast_ref::<personal::Error>() {
        labels.push(format!("personal-source:{}", e.code()));
        if let Some(c) = e.authority_error() {
            children.push(c)
        }
    }
    if let Some(e) = e.downcast_ref::<CloseError>() {
        labels.push("personal-lifecycle:close".into());
        if let Some(c) = &e.personal {
            children.push(c)
        }
        if let Some(c) = &e.control {
            children.push(c)
        }
    }
    if let Some(e) = e.downcast_ref::<frame::Error>() {
        labels.push(format!("personal-frame:{}", e.code()))
    }
    (labels, children)
}

#[cfg(test)]
mod tests;
