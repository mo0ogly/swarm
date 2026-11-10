//! Versioned human-authority payloads above the signed journal. These are data,
//! never an engine capability. Callers must independently resolve expectations
//! and serialize final use with the current protected authority source.
#![forbid(unsafe_code)]
/// Personal-secret derivation and exact-record knowledge checking; no sessions or grants.
pub mod credentials;
/// One-time personal bootstrap and held current credential use; protected startup required.
#[cfg(target_os = "linux")]
pub mod personal;
/// Linux configuration-control core; transport and protected bootstrap are not
/// supplied by this module. Current authority is retained through storage use.
#[cfg(target_os = "linux")]
pub mod control;
/// Held personal account and current business authority composition; protected startup required.
#[cfg(target_os = "linux")]
pub mod authenticated_control;
/// Exact Trust action bindings and borrowed current-human identity.
#[cfg(target_os = "linux")]
pub use control::trust_action;
/// Exact PAEA effect permissions and held current-account capability.
#[cfg(target_os = "linux")]
pub use control::paea_action;
/// Pinned v3two-source opening; no service, account initialization or human session.
#[cfg(target_os = "linux")]
pub mod authenticated_startup;
/// Private pinned signing-key snapshot loader; no service or human authority.
#[cfg(target_os = "linux")]
pub mod protected_signer;
/// Owned startup-bound personal enrollment/recovery; no transport or human inference.
#[cfg(target_os = "linux")]
pub mod personal_lifecycle;
/// Guarded personal-service request data; no peer admission or authority.
#[cfg(target_os = "linux")]
pub mod service_request;
/// Pinned personal-service core; authenticated transport and OS custody required.
#[cfg(target_os = "linux")]
pub mod personal_service;
/// Service observations and output-error retention; not peer authentication.
#[cfg(target_os = "linux")]
pub mod service_response;
/// Initial operator-private material only; publication/service/approval remain separate.
#[cfg(target_os = "linux")]
pub mod provisioning;

pub mod publication;
/// Independently pinned Linux startup declaration and current-source preflight.
#[cfg(target_os = "linux")]
pub mod startup;

use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fmt};
use swarm_enrollment_proof::Statement;
use swarm_enrollment_state::{decode_active_reply, sign_change, Head};
use swarm_identity_contract::{digest, text, Permission, Principal, Scope};
use swarm_wire_json as wire;
use time::OffsetDateTime;
use wire::timestamp::{deserialize as deserialize_time, serialize as serialize_time, zero};
#[derive(Debug)]
pub enum Error {
    Document,
    Binding,
    Inactive,
    State(swarm_enrollment_state::Error),
    DocumentFrame(swarm_enrollment_state::Error),
}
impl Error {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Document | Self::DocumentFrame(_) => "document",
            Self::Binding => "binding",
            Self::Inactive => "inactive",
            Self::State(e) => e.code(),
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::State(e) => write!(f, "{e}"),
            Self::DocumentFrame(e) => write!(f, "enrollment authority: invalid document: {e}"),
            e => write!(f, "enrollment authority: {}", e.code()),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::State(e) | Self::DocumentFrame(e) => Some(e),
            _ => None,
        }
    }
}
/// Immutable enrollment document, without status. The journal derives revocation
/// and enrollment sequence from its signed history rather than these fields.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub schema_version: i64,
    pub enrollment_id: String,
    pub generation: i64,
    #[serde(rename = "project_context_digest")]
    pub project_digest: String,
    pub principal: Principal,
    pub scope: Scope,
    #[serde(rename = "trust_configuration_revision")]
    pub trust_revision: i64,
    pub permissions: Vec<Permission>,
    pub delegable: Vec<Permission>,
    #[serde(
        serialize_with = "serialize_time",
        deserialize_with = "deserialize_time"
    )]
    pub not_before: OffsetDateTime,
    #[serde(
        serialize_with = "serialize_time",
        deserialize_with = "deserialize_time"
    )]
    pub expires_at: OffsetDateTime,
}
/// Trusted control must resolve these values independently of the checked payload.
#[derive(Clone, Debug)]
pub struct Expected {
    pub head: Head,
    pub enrollment_id: String,
    pub generation: i64,
    pub project_digest: String,
    pub principal: Principal,
    pub scope: Scope,
    pub trust_revision: i64,
    pub request_digest: String,
}
impl Document {
    fn valid(&self) -> bool {
        if self.schema_version != 1
            || !text(self.enrollment_id.as_bytes())
            || self.generation <= 0
            || !digest(self.project_digest.as_bytes())
            || !self.principal.valid()
            || self.principal.kind != "human"
            || !self.scope.valid()
            || self.trust_revision <= 0
            || self.permissions.is_empty()
            || zero(self.not_before)
            || self.not_before >= self.expires_at
        {
            return false;
        }
        let mut rights = HashSet::new();
        for p in &self.permissions {
            if !p.valid() || !rights.insert(p) {
                return false;
            }
        }
        let mut delegable = HashSet::new();
        for p in &self.delegable {
            if !p.valid() || !delegable.insert(p) || !rights.contains(p) {
                return false;
            }
        }
        true
    }
}
/// Validate then encode UTC instants and explicit empty delegation ceilings.
/// No issuer/subject normalization, default duration or business cap is applied.
pub fn encode(document: &Document) -> Result<Vec<u8>, Error> {
    if !document.valid() {
        return Err(Error::Document);
    }
    wire::encode(document).map_err(|_| Error::Document)
}
/// Require exact canonical bytes, including every nested structure and timestamp.
pub fn decode(raw: &[u8]) -> Result<Document, Error> {
    let document: Document = wire::decode(raw).map_err(|_| Error::Document)?;
    if encode(&document)? != raw {
        return Err(Error::Document);
    }
    Ok(document)
}
/// Sign a proposal only. Generation must equal the actual next journal sequence.
pub fn sign_enrollment(
    key: &SigningKey,
    project: [u8; 32],
    base: &Head,
    document: &Document,
) -> Result<Vec<u8>, Error> {
    let body = encode(document)?;
    let hex: String = project.iter().map(|b| format!("{b:02x}")).collect();
    if base.sequence < 0
        || base.sequence == i64::MAX
        || document.generation != base.sequence + 1
        || document.project_digest != hex
    {
        return Err(Error::Binding);
    }
    sign_change(key, project, base, "enroll", &document.enrollment_id, &body).map_err(Error::State)
}
/// Check authenticated bytes against independent context and explicit observation
/// time. This does not reread storage: an old Statement and old expected Head can
/// still pass after revocation. Final-use freshness belongs to the protected flow.
pub fn decode_active(
    statement: &Statement,
    expected: &Expected,
    at: OffsetDateTime,
) -> Result<Document, Error> {
    if expected.head.sequence <= 0
        || !digest(expected.head.digest.as_bytes())
        || !text(expected.enrollment_id.as_bytes())
        || expected.generation <= 0
        || expected.generation > expected.head.sequence
        || !digest(expected.project_digest.as_bytes())
        || !digest(expected.request_digest.as_bytes())
        || !expected.principal.valid()
        || expected.principal.kind != "human"
        || !expected.scope.valid()
        || expected.trust_revision <= 0
    {
        return Err(Error::Binding);
    }
    let r = statement.request();
    if r.project_digest != expected.project_digest || r.request_digest != expected.request_digest {
        return Err(Error::Binding);
    }
    let active = decode_active_reply(&statement.payload()).map_err(Error::DocumentFrame)?;
    if active.head != expected.head
        || active.record.id != expected.enrollment_id
        || active.record.enrolled_sequence != expected.generation
    {
        return Err(Error::Binding);
    }
    let document = decode(&active.record.body)?;
    if document.enrollment_id != active.record.id
        || document.generation != active.record.enrolled_sequence
        || document.project_digest != expected.project_digest
        || document.principal != expected.principal
        || document.scope != expected.scope
        || document.trust_revision != expected.trust_revision
    {
        return Err(Error::Binding);
    }
    if zero(at) || at < document.not_before || at >= document.expires_at {
        return Err(Error::Inactive);
    }
    Ok(document)
}
