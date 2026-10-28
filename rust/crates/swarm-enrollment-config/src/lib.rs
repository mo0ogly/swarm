//! Installation configuration data and protected local storage. Neither format
//! validation nor a declared principal authenticates a human or effective service.
#![forbid(unsafe_code)]
mod go_clock;
#[cfg(target_os = "linux")]
pub mod store;
mod wire_input;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, fmt};
use swarm_identity_contract::{digest, text, Principal};
use swarm_wire_json::{self as wire, timestamp};
use time::{OffsetDateTime, UtcOffset};
// Linux sockaddr_un.sun_path has 108 bytes, including its terminating NUL.
// Representation limit only; no execution policy or waiting bound.
pub const LOCAL_SOCKET_PATH_MAX_BYTES: usize = 107;
#[derive(Clone, Copy, Debug)]
pub enum IoClass {
    Path,
    Storage,
    Indeterminate,
}
impl IoClass {
    fn code(self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::Storage => "storage",
            Self::Indeterminate => "indeterminate",
        }
    }
}
pub enum Error {
    Format,
    Version,
    Mode,
    Pins,
    Head,
    Frame(serde_json::Error),
    Projection(wire::projection::Error),
    Busy,
    Conflict,
    Event,
    Unauthorized,
    NotConfigured,
    Exists,
    Path,
    Closed,
    Canceled,
    DeadlineExceeded,
    State,
    WitnessMissing,
    Witness,
    Pending,
    WitnessFrame(Box<Error>),
    PendingCause(Box<Error>),
    At {
        operation: Operation,
        cause: Box<Error>,
    },
    Io {
        class: IoClass,
        cause: std::io::Error,
    },
    Authorization(Box<Error>),
    Joined {
        cause: Box<Error>,
        cleanup: Box<Error>,
    },
}
/// Fixed diagnostic labels contain no caller data or private paths.
#[derive(Clone, Copy, Debug)]
pub enum Operation {
    CheckConfiguration,
    CheckWitness,
    SyncConfiguration,
    SyncWitness,
    PrepareWitness,
    SyncPreparedWitness,
    WriteConfiguration,
    SyncRecoveredConfiguration,
    ConfirmWitness,
}
impl Operation {
    fn label(self) -> &'static str {
        match self {
            Self::CheckConfiguration => "check configuration directory",
            Self::CheckWitness => "check witness directory",
            Self::SyncConfiguration => "sync configuration directory",
            Self::SyncWitness => "sync witness directory",
            Self::PrepareWitness => "prepare witness",
            Self::SyncPreparedWitness => "sync prepared witness",
            Self::WriteConfiguration => "write configuration",
            Self::SyncRecoveredConfiguration => "sync recovered configuration",
            Self::ConfirmWitness => "confirm witness",
        }
    }
}
impl Error {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Format | Self::Frame(_) | Self::Projection(_) => "format",
            Self::Version => "version",
            Self::Mode => "mode",
            Self::Pins => "pins",
            Self::Head => "head",
            Self::Busy => "busy",
            Self::Conflict => "conflict",
            Self::Event => "event",
            Self::Unauthorized | Self::Authorization(_) => "unauthorized",
            Self::NotConfigured => "not_configured",
            Self::Exists => "exists",
            Self::Path => "path",
            Self::Closed => "closed",
            Self::Canceled => "canceled",
            Self::DeadlineExceeded => "deadline",
            Self::State => "state",
            Self::WitnessMissing => "witness_missing",
            Self::Witness | Self::WitnessFrame(_) => "witness",
            Self::Pending | Self::PendingCause(_) => "pending",
            Self::At { cause, .. } => cause.code(),
            Self::Io { class, .. } => class.code(),
            Self::Joined { cause, .. } => cause.code(),
        }
    }
    pub fn contains_code(&self, code: &str) -> bool {
        self.code() == code
            || match self {
                Self::Authorization(e)
                | Self::WitnessFrame(e)
                | Self::PendingCause(e)
                | Self::At { cause: e, .. } => e.contains_code(code),
                Self::Joined { cause, cleanup } => {
                    cause.contains_code(code) || cleanup.contains_code(code)
                }
                _ => false,
            }
    }
    pub fn cleanup_error(&self) -> Option<&Self> {
        match self {
            Self::At { cause, .. } => cause.cleanup_error(),
            Self::Joined { cleanup, .. } => Some(cleanup),
            _ => None,
        }
    }
    pub(crate) fn io(class: IoClass, cause: impl Into<std::io::Error>) -> Self {
        Self::Io {
            class,
            cause: cause.into(),
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WitnessFrame(e) => write!(f, "enrollment config: invalid witness: {e}"),
            Self::PendingCause(e) => write!(f, "enrollment config: pending intent: {e}"),
            Self::At { operation, cause } => write!(f, "{}: {cause}", operation.label()),
            Self::Projection(e) => write!(f, "enrollment config: {e}"),
            Self::Frame(e) => write!(
                f,
                "enrollment config: invalid canonical configuration ({:?}, line {}, column {})",
                e.classify(),
                e.line(),
                e.column()
            ),
            Self::Io { class, cause } => write!(
                f,
                "enrollment config: {} ({:?}, errno {:?})",
                class.code(),
                cause.kind(),
                cause.raw_os_error()
            ),
            Self::Authorization(e) => write!(
                f,
                "enrollment config: authenticated bootstrap authority required: {e}"
            ),
            Self::Joined { cause, cleanup } => write!(f, "{cause}; cleanup: {cleanup}"),
            e => write!(f, "enrollment config: {}", e.code()),
        }
    }
}
// Ordinary diagnostics never echo raw serde/callback values. Structured source
// and cleanup branches remain available to an authorized diagnostic caller.
impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EnrollmentConfigError")
            .field("code", &self.code())
            .field("message", &self.to_string())
            .finish()
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Frame(e) => Some(e),
            Self::Projection(e) => Some(e),
            Self::Io { cause, .. } => Some(cause),
            Self::Authorization(e) => Some(e.as_ref()),
            Self::WitnessFrame(e) | Self::PendingCause(e) | Self::At { cause: e, .. } => {
                Some(e.as_ref())
            }
            Self::Joined { cause, .. } => Some(cause.as_ref()),
            _ => None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pins {
    pub installation_digest: String,
    #[serde(rename = "authority_public_key")]
    pub public_key: String,
    #[serde(rename = "bootstrap_principal")]
    pub principal: Principal,
}
impl Pins {
    pub fn validate(&self) -> Result<(), Error> {
        let key = URL_SAFE_NO_PAD
            .decode(&self.public_key)
            .map_err(|_| Error::Pins)?;
        if key.len() != 32
            || URL_SAFE_NO_PAD.encode(&key) != self.public_key
            || !digest(self.installation_digest.as_bytes())
            || !self.principal.valid()
            || self.principal.kind != "human"
        {
            return Err(Error::Pins);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Values {
    pub mode: String,
    #[serde(rename = "local_socket_path")]
    pub socket_path: String,
}
fn clean_absolute(path: &str) -> bool {
    path.starts_with('/')
        && path != "/"
        && path[1..]
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}
impl Values {
    fn validate(&self) -> Result<(), Error> {
        if self.mode == "remote" {
            return Err(Error::Mode);
        }
        if self.mode != "local"
            || !text(self.socket_path.as_bytes())
            || !clean_absolute(&self.socket_path)
            || self.socket_path.len() > LOCAL_SOCKET_PATH_MAX_BYTES
        {
            return Err(Error::Format);
        }
        Ok(())
    }
}
mod revision {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(n: &u64, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&n.to_string())
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        let raw = String::deserialize(d)?;
        if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
            return Err(serde::de::Error::custom("unsigned decimal string required"));
        }
        raw.parse().map_err(serde::de::Error::custom)
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Head {
    #[serde(with = "revision")]
    pub revision: u64,
    pub digest: String,
}
impl Head {
    fn valid(&self) -> bool {
        if self.revision == 0 {
            self.digest.is_empty()
        } else {
            digest(self.digest.as_bytes())
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    pub schema_version: i64,
    pub event_id: String,
    pub expected_head: Head,
    pub values: Values,
    pub reason: String,
}
impl Change {
    fn validate(&self) -> Result<(), Error> {
        if self.schema_version != 1 {
            return Err(Error::Version);
        }
        if !text(self.event_id.as_bytes())
            || !text(self.reason.as_bytes())
            || !self.expected_head.valid()
        {
            return Err(Error::Format);
        }
        self.values.validate()
    }
}
pub fn encode_change(change: &Change) -> Result<Vec<u8>, Error> {
    change.validate()?;
    wire::encode(change).map_err(Error::Frame)
}
pub fn decode_change(raw: &[u8]) -> Result<Change, Error> {
    let c: Change = wire_input::change(raw)?;
    if encode_change(&c)? != raw {
        return Err(Error::Format);
    }
    Ok(c)
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub change: Change,
    pub actor: Principal,
    #[serde(
        serialize_with = "timestamp::serialize_preserving_offset",
        deserialize_with = "go_clock::deserialize"
    )]
    pub at: OffsetDateTime,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub schema_version: i64,
    pub pins: Pins,
    pub history: Vec<Entry>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct View {
    pub snapshot: Snapshot,
    pub head: Head,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Projection {
    pub base: Head,
    pub proposed: View,
    pub current_event_replay: bool,
}
fn entry_head(pins: &Pins, entry: &Entry) -> Result<Head, Error> {
    #[derive(Serialize)]
    struct Frame<'a> {
        pins: &'a Pins,
        entry: &'a Entry,
    }
    let raw = wire::encode(&Frame { pins, entry }).map_err(Error::Frame)?;
    let mut preimage = b"swarm-installation-config-v1\0entry\0".to_vec();
    preimage.extend(raw);
    Ok(Head {
        revision: entry
            .change
            .expected_head
            .revision
            .checked_add(1)
            .ok_or(Error::Format)?,
        digest: format!("{:x}", Sha256::digest(preimage)),
    })
}
pub fn encode(snapshot: &Snapshot) -> Result<Vec<u8>, Error> {
    if snapshot.schema_version != 1 {
        return Err(Error::Version);
    }
    if snapshot.pins.validate().is_err() || snapshot.history.is_empty() {
        return Err(Error::Format);
    }
    let mut canonical = snapshot.clone();
    let mut base = Head::default();
    let mut events = HashSet::new();
    for entry in &mut canonical.history {
        entry.change.validate()?;
        if entry.change.expected_head != base
            || !events.insert(&entry.change.event_id)
            || entry.actor != canonical.pins.principal
            || timestamp::zero(entry.at)
            || base.revision == u64::MAX
        {
            return Err(Error::Format);
        }
        entry.at = entry
            .at
            .checked_to_offset(UtcOffset::UTC)
            .ok_or(Error::Format)?;
        base = entry_head(&canonical.pins, entry)?;
    }
    wire::encode(&canonical).map_err(Error::Frame)
}
pub fn decode(raw: &[u8]) -> Result<Snapshot, Error> {
    let s: Snapshot = wire_input::snapshot(raw)?;
    if encode(&s)? != raw {
        return Err(Error::Format);
    }
    Ok(s)
}
/// Return owned validated canonical data; storage/authentication are separate.
pub fn snapshot_view(snapshot: &Snapshot) -> Result<View, Error> {
    let snapshot = decode(&encode(snapshot)?)?;
    let head = entry_head(
        &snapshot.pins,
        snapshot.history.last().ok_or(Error::Format)?,
    )?;
    Ok(View { snapshot, head })
}
pub fn validate_view(view: &View) -> Result<(), Error> {
    // Go first marshals the supplied view without normalizing its timestamps.
    // Decode then rejects a non-UTC representation, even when the instant matches.
    let raw = wire::encode(&view.snapshot).map_err(Error::Frame)?;
    let s = decode(&raw)?;
    let expected = entry_head(&s.pins, s.history.last().ok_or(Error::Format)?)?;
    if expected != view.head {
        return Err(Error::Head);
    }
    Ok(())
}
pub fn validate_projection(projection: &Projection) -> Result<(), Error> {
    validate_view(&projection.proposed)?;
    if !projection.base.valid() {
        return Err(Error::Head);
    }
    let expected = if projection.current_event_replay {
        &projection.proposed.head
    } else {
        &projection
            .proposed
            .snapshot
            .history
            .last()
            .ok_or(Error::Format)?
            .change
            .expected_head
    };
    if &projection.base != expected {
        return Err(Error::Head);
    }
    Ok(())
}
