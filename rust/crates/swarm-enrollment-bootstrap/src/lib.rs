//! Immutable installation declaration and read-only Linux storage observation.
//! A human label, public key or distinct UID is never authenticated authority.
#![forbid(unsafe_code)]
mod authenticated;
mod control;
pub use authenticated::{
    decode_authenticated, encode_authenticated, AuthenticatedDocument, PersonalDeclaration,
};
pub use control::{decode_control, encode_control, ControlDocument};
use serde::{Deserialize, Serialize};
use std::fmt;
use swarm_enrollment_config::Pins;
use swarm_wire_json as wire;
#[cfg(target_os = "linux")]
mod loader;
#[cfg(target_os = "linux")]
pub use loader::{load, load_authenticated, load_control};
#[cfg(all(target_os = "linux", feature = "test-fixtures"))]
pub use loader::{
    load_authenticated_with_faults, load_control_with_faults, load_with_faults, FixtureFaults,
};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub schema_version: i64,
    pub pins: Pins,
    pub service_uid: u32,
    pub controller_uid: u32,
    pub configuration_directory: String,
    pub witness_directory: String,
}
#[derive(Clone, Copy, Debug)]
pub enum IoOperation {
    Open,
    Stat,
    Read,
    Close,
}
pub enum Error {
    Document,
    Binding(swarm_enrollment_config::Error),
    Frame(serde_json::Error),
    Pin,
    Path,
    Protected(swarm_enrollment_config::Error),
    Io {
        operation: IoOperation,
        cause: std::io::Error,
    },
    Joined {
        cause: Box<Error>,
        cleanup: Box<Error>,
    },
}
impl Error {
    /// Safe validation/storage category; typed Binding preserves shared Pins refusal.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Document | Self::Frame(_) | Self::Binding(_) => "document",
            Self::Pin => "pin",
            Self::Path | Self::Protected(_) => "path",
            Self::Io {
                operation: IoOperation::Open | IoOperation::Stat,
                ..
            } => "path",
            Self::Io { .. } => "storage",
            Self::Joined { cause, .. } => cause.code(),
        }
    }
    pub fn contains_code(&self, code: &str) -> bool {
        self.code() == code
            || match self {
                Self::Binding(cause) => cause.contains_code(code),
                Self::Joined { cause, cleanup } => {
                    cause.contains_code(code) || cleanup.contains_code(code)
                }
                _ => false,
            }
    }
    pub fn cleanup_error(&self) -> Option<&Self> {
        match self {
            Self::Joined { cleanup, .. } => Some(cleanup),
            _ => None,
        }
    }
}
impl fmt::Display for Error {
    /// Safe categories and metadata only; never echo declaration/public-key input.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Binding(cause) => write!(f, "enrollment bootstrap: binding ({})", cause.code()),
            Self::Frame(e) => write!(
                f,
                "enrollment bootstrap: document ({:?}, line {}, column {})",
                e.classify(),
                e.line(),
                e.column()
            ),
            Self::Protected(e) => write!(f, "enrollment bootstrap: protected storage: {e}"),
            Self::Io { operation, cause } => write!(
                f,
                "enrollment bootstrap: {operation:?} ({:?}, errno {:?})",
                cause.kind(),
                cause.raw_os_error()
            ),
            Self::Joined { cause, cleanup } => write!(f, "{cause}; cleanup: {cleanup}"),
            e => write!(f, "enrollment bootstrap: {}", e.code()),
        }
    }
}
impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BootstrapError")
            .field("code", &self.code())
            .field("message", &self.to_string())
            .finish()
    }
}
impl std::error::Error for Error {
    /// Original typed primary cause; cleanup_error is an additional Joined branch.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Binding(cause) => Some(cause),
            Self::Frame(e) => Some(e),
            Self::Protected(e) => Some(e),
            Self::Io { cause, .. } => Some(cause),
            Self::Joined { cause, .. } => Some(cause.as_ref()),
            _ => None,
        }
    }
}
fn clean_path(path: &str) -> bool {
    swarm_identity_contract::text(path.as_bytes())
        && path.starts_with('/')
        && path != "/"
        && path[1..]
            .split('/')
            .all(|p| !p.is_empty() && p != "." && p != "..")
}
fn overlaps(a: &str, b: &str) -> bool {
    a == b
        || a.strip_prefix(b).is_some_and(|tail| tail.starts_with('/'))
        || b.strip_prefix(a).is_some_and(|tail| tail.starts_with('/'))
}
/// Encode validated canonical Linux installation data, without IO or authority.
/// Both UID roles require assignable numeric representation and must differ;
/// UID0 is allowed as data, never proof of an existing account or permission.
/// Invalid schema/pins/UID/path returns a typed refusal before serialization.
pub fn encode(d: &Document) -> Result<Vec<u8>, Error> {
    if d.schema_version != 1
        || d.pins.validate().is_err()
        || !swarm_identity_contract::linux_uid(d.service_uid)
        || !swarm_identity_contract::linux_uid(d.controller_uid)
        || d.service_uid == d.controller_uid
        || !clean_path(&d.configuration_directory)
        || !clean_path(&d.witness_directory)
        || overlaps(&d.configuration_directory, &d.witness_directory)
    {
        return Err(Error::Document);
    }
    wire::encode(d).map_err(Error::Frame)
}
/// Decode exact canonical data and apply the same UID/pins/path validation.
/// Reserved UID, missing/unknown/ambiguous fields and noncanonical bytes refuse;
/// no file access, automatic migration, identity adoption or authority is issued.
pub fn decode(raw: &[u8]) -> Result<Document, Error> {
    let d: Document = wire::decode(raw).map_err(Error::Frame)?;
    if encode(&d)? != raw {
        return Err(Error::Document);
    }
    Ok(d)
}
