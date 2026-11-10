//! Public-data observations for the admitted private service channel. No peer,
//! freshness, grant or human evidence. Original typed Reply is retained across
//! encoding/send errors; no response status can replace current authorization.
use crate::{
    control,
    personal::coordinator::{self, JournalOutcome},
    personal_service::{OutcomeData, Reply},
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use std::{
    error::Error as StdError,
    fmt,
    io::{self, Write},
};
use swarm_enrollment_config::{self as config, store::documents::Publication};

/// Protocol publication observation, no rollback or hardware durability claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilePhase {
    NotAttempted,
    Unknown,
    Visible,
    Durable,
}
impl From<Publication> for FilePhase {
    /// Exact existing phase mapping, never infer a later acknowledgement.
    fn from(p: Publication) -> Self {
        match p {
            Publication::NotAttempted => Self::NotAttempted,
            Publication::Unknown => Self::Unknown,
            Publication::Visible => Self::Visible,
            Publication::Durable => Self::Durable,
        }
    }
}
/// CurrentVerified is recovery success without claiming it newly applied data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalPhase {
    NotAttempted,
    Unknown,
    Acknowledged,
    Observed,
    CurrentVerified,
}
/// Closed failure layers; original detailed typed causes remain in Reply locally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureLayer {
    Personal,
    Business,
    PersonalAfterUse,
    BusinessAfterStorage,
}
/// Untrusted wire observation, not authority. Deserialization alone is insufficient;
/// use consumer::decode to obtain a request-bound, semantically checked wrapper.
/// Known results may coexist with failures/uncertainty after commitment.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    schema_version: i64,
    protocol: String,
    operation: u8,
    request_digest: String,
    challenge: String,
    completed: bool,
    failure_layers: Vec<FailureLayer>,
    intent: Option<FilePhase>,
    journal: Option<JournalPhase>,
    checkpoint: Option<FilePhase>,
    sequence: Option<i64>,
    configuration_effect_unknown: bool,
    view: Option<config::View>,
    projection: Option<config::Projection>,
}
impl fmt::Debug for Observation {
    /// No configuration, public identity, private paths or request binding logged.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PersonalServiceObservation { <private> }")
    }
}
impl Observation {
    /// True only when the underlying operation itself succeeded; not send status.
    pub fn completed(&self) -> bool {
        self.completed
    }
    /// Preserved configured storage result, even after a later authority failure.
    pub fn view(&self) -> Option<&config::View> {
        self.view.as_ref()
    }
    /// Preserved nonmutating projection; never interpret this as saved state.
    pub fn projection(&self) -> Option<&config::Projection> {
        self.projection.as_ref()
    }
    /// Safe ordered failure layers; detailed causes stay in the retained Reply.
    pub fn failure_layers(&self) -> &[FailureLayer] {
        &self.failure_layers
    }
    /// Conservative existing configuration uncertainty, not absent-effect proof.
    pub fn configuration_effect_unknown(&self) -> bool {
        self.configuration_effect_unknown
    }
    /// Safe status metadata for output-loss diagnostics. Excludes configuration,
    /// identity, request digest/challenge, private paths and credential/head pins.
    /// Completed describes the operation only, never transport or service close.
    pub fn diagnostic_summary(&self) -> String {
        format!("operation={}, completed={}, journal={:?}, sequence={:?}, checkpoint={:?}, configuration_effect_unknown={}",self.operation,self.completed,self.journal,self.sequence,self.checkpoint,self.configuration_effect_unknown)
    }
    /// Preserve known view/projection without changing a prior failure verdict.
    fn response(&mut self, response: &control::Response) {
        match response {
            control::Response::View(v) => self.view = Some(v.clone()),
            control::Response::Preview(p) => self.projection = Some(p.clone()),
        }
    }
    /// Follow the actual business error, retaining known AfterStorage response
    /// and conservative uncertainty. Refusal does not become an empty success.
    fn business_failure(&mut self, error: &control::Error) {
        self.completed = false;
        self.configuration_effect_unknown |= error.effect_unknown();
        match error {
            control::Error::AfterStorage { storage, .. } => {
                self.failure_layers.push(FailureLayer::BusinessAfterStorage);
                if let Ok(response) = storage.as_ref() {
                    self.response(response)
                }
            }
            control::Error::AuthorityRefusal { refusal, .. } => self.business_failure(refusal),
            _ => self.failure_layers.push(FailureLayer::Business),
        }
    }
    /// Retain business Result unchanged in the public observation after outer
    /// personal completion fails; never let a known result clear that failure.
    fn business_result(&mut self, result: &Result<control::Response, control::Error>) {
        match result {
            Ok(response) => self.response(response),
            Err(error) => self.business_failure(error),
        }
    }
    /// Exact personal failure progress; no credential or private head digest.
    fn personal_failure(&mut self, error: &coordinator::CommitError) {
        self.completed = false;
        self.failure_layers.push(FailureLayer::Personal);
        self.intent = Some(error.progress.intent.into());
        self.checkpoint = Some(error.progress.checkpoint.into());
        let (journal, sequence) = match &error.progress.journal {
            JournalOutcome::NotAttempted => (JournalPhase::NotAttempted, None),
            JournalOutcome::Unknown => (JournalPhase::Unknown, None),
            JournalOutcome::Acknowledged(h) => (JournalPhase::Acknowledged, Some(h.sequence)),
            JournalOutcome::Observed(h) => (JournalPhase::Observed, Some(h.sequence)),
        };
        self.journal = Some(journal);
        self.sequence = sequence;
    }
    /// Project actual typed Reply without consuming its original causes. Closed
    /// opcode came from validated dispatch. Successful recovery does not reveal
    /// whether pending bytes were replayed. Configuration data is for the admitted
    /// controller only; this is no public unauthenticated publication endpoint.
    pub fn from_reply(reply: &Reply) -> Self {
        let mut out = Self {
            schema_version: 1,
            protocol: "swarm-personal-service-response-v1".into(),
            operation: reply.operation(),
            request_digest: reply.request_digest().to_owned(),
            challenge: URL_SAFE_NO_PAD.encode(reply.challenge),
            completed: true,
            failure_layers: vec![],
            intent: None,
            journal: None,
            checkpoint: None,
            sequence: None,
            configuration_effect_unknown: false,
            view: None,
            projection: None,
        };
        match &reply.outcome {
            OutcomeData::Initialized(Ok(())) => {
                out.intent = Some(FilePhase::NotAttempted);
                out.journal = Some(JournalPhase::NotAttempted);
                out.checkpoint = Some(FilePhase::Durable)
            }
            OutcomeData::Initialized(Err(e)) | OutcomeData::Account(Err(e)) => {
                out.personal_failure(e)
            }
            OutcomeData::Account(Ok(a)) => {
                out.sequence = Some(a.head().sequence);
                out.checkpoint = Some(FilePhase::Durable);
                if reply.operation() == 2 {
                    out.intent = Some(FilePhase::Durable);
                    out.journal = Some(JournalPhase::Acknowledged)
                } else {
                    out.journal = Some(JournalPhase::CurrentVerified)
                }
            }
            OutcomeData::Control(Ok(result)) => out.business_result(result),
            OutcomeData::Control(Err(coordinator::UseError::Before(_))) => {
                out.completed = false;
                out.failure_layers.push(FailureLayer::Personal)
            }
            OutcomeData::Control(Err(coordinator::UseError::AfterUse { outcome, .. })) => {
                out.completed = false;
                out.failure_layers.push(FailureLayer::PersonalAfterUse);
                out.business_result(outcome)
            }
        };
        out
    }
}
/// Canonical request-bound consumer checks; sender admission remains external.
pub mod consumer;
/// Technical output phase only. WriteUnknown may include a partial/full write;
/// WrittenFlushFailed retains write_all success, never receipt by the peer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputPhase {
    Encode,
    WriteUnknown,
    WrittenFlushFailed,
}
/// Complete original Reply remains inspectable after output, whether its business
/// operation succeeded or failed. Sending successfully is not business acceptance.
pub struct Sent {
    pub reply: Reply,
    pub observation: Observation,
}
impl fmt::Debug for Sent {
    /// No retained operation or configuration data formatted.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PersonalServiceSent { <private> }")
    }
}
/// Original output cause, no private text in formatting.
pub enum OutputCause {
    Encode(serde_json::Error),
    Io(io::Error),
}
impl fmt::Display for OutputCause {
    /// Fixed category without nested OS/custom error strings.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Encode(_) => "service-response-encoding",
            Self::Io(_) => "service-response-io",
        })
    }
}
impl fmt::Debug for OutputCause {
    /// Same safe diagnostic as Display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for OutputCause {
    /// Original typed IO/serde cause for trusted traversal only.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(match self {
            Self::Encode(e) => e,
            Self::Io(e) => e,
        })
    }
}
/// Own every original operation/cleanup branch plus the output phase/cause.
/// Consumers must inspect reply.outcome independently of source().
pub struct SendError {
    pub phase: OutputPhase,
    pub cause: OutputCause,
    pub reply: Reply,
    pub observation: Observation,
}
impl fmt::Display for SendError {
    /// Phase only; no IO text, paths, credentials or retained response payload.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "personal service output: {:?}", self.phase)
    }
}
impl fmt::Debug for SendError {
    /// Same safe formatting; typed causes/known results remain owned.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for SendError {
    /// Output cause only; reply.outcome is an independent mandatory branch.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.cause)
    }
}
/// Encode canonical public-data JSON, then write_all/flush exactly once, retaining
/// the complete Reply and observation in either outcome. No automatic business
/// retry, rollback, peer acknowledgement or close. Writer/transport ownership and
/// actual packet bounds belong to its caller. No newline framing is appended.
pub fn send(writer: &mut dyn Write, reply: Reply) -> Result<Sent, SendError> {
    let observation = Observation::from_reply(&reply);
    let raw = match swarm_wire_json::encode(&observation) {
        Ok(raw) => raw,
        Err(cause) => {
            return Err(SendError {
                phase: OutputPhase::Encode,
                cause: OutputCause::Encode(cause),
                reply,
                observation,
            })
        }
    };
    if let Err(cause) = writer.write_all(&raw) {
        return Err(SendError {
            phase: OutputPhase::WriteUnknown,
            cause: OutputCause::Io(cause),
            reply,
            observation,
        });
    }
    if let Err(cause) = writer.flush() {
        return Err(SendError {
            phase: OutputPhase::WrittenFlushFailed,
            cause: OutputCause::Io(cause),
            reply,
            observation,
        });
    }
    Ok(Sent { reply, observation })
}

/// Dedicated packet-output cause. A local accepted send is not a peer receipt;
/// the channel branch preserves native invocation/known accepted-count evidence.
pub enum PacketOutputCause {
    Encode(serde_json::Error),
    Send(swarm_linux_identity_channel::SendFailure),
}
impl fmt::Display for PacketOutputCause {
    /// Safe static phase only, without packet bytes, paths or nested private IO.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Encode(_) => "service-response-encoding",
            Self::Send(_) => "service-response-packet-send",
        })
    }
}
impl fmt::Debug for PacketOutputCause {
    /// Same redaction as Display; typed original remains available to the caller.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for PacketOutputCause {
    /// Original encoder/channel error; inspect independent Reply branches too.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(match self {
            Self::Encode(e) => e,
            Self::Send(e) => e,
        })
    }
}
/// Complete local business result plus packet-output error. Even pre-send encode
/// refusal does not roll back the earlier operation. All Reply outcome/after-use
/// branches remain mandatory inspection independently of this output cause.
pub struct PacketSendError {
    pub cause: PacketOutputCause,
    pub reply: Reply,
    pub observation: Observation,
}
impl fmt::Display for PacketSendError {
    /// No retained configuration, request identifiers, FD or private error text.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "personal service packet output: {}", self.cause)
    }
}
impl fmt::Debug for PacketSendError {
    /// Same private-data redaction as Display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for PacketSendError {
    /// Output cause only; original Reply is an independently retained branch.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.cause)
    }
}
/// Encode the canonical public observation then send exactly one nonempty record.
/// Own the original Reply/Observation on every encode/send refusal, without retry,
/// fragmentation, flush, close or business rollback. Channel preflight and actual
/// native invocation evidence remain in the typed send cause. Success is local
/// full-byte queue acceptance, not peer receipt/admission, freshness or business
/// acceptance. Caller owns socket flags, packet capacity, protected peer/domain
/// admission and cancellation. Legacy Write/flush output remains a separate API.
pub fn send_packet(
    channel: &mut swarm_linux_identity_channel::Channel<'_>,
    reply: Reply,
) -> Result<Sent, PacketSendError> {
    let observation = Observation::from_reply(&reply);
    let raw = match swarm_wire_json::encode(&observation) {
        Ok(raw) => raw,
        Err(cause) => {
            return Err(PacketSendError {
                cause: PacketOutputCause::Encode(cause),
                reply,
                observation,
            })
        }
    };
    if let Err(cause) = channel.send_packet(&raw) {
        return Err(PacketSendError {
            cause: PacketOutputCause::Send(cause),
            reply,
            observation,
        });
    }
    Ok(Sent { reply, observation })
}
