//! Bind a complete registry-publication descriptor. Prepared data is immutable,
//! not evidence that its principal, session, plan, roots or witness are trusted.
use crate::{Document, Expected};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, fmt, sync::Arc};
use swarm_enrollment_proof::{Challenge, Request, Statement, Verifier};
use swarm_enrollment_state::Head;
use swarm_identity_contract::{digest, name, text, Principal, Scope};
use swarm_wire_json as wire;
use time::OffsetDateTime;
pub const PUBLICATION_PURPOSE: &str = "agent_identity_registry_publication";
const PROTOCOL: &str = "swarm-enrollment-publication-request-v1";
#[derive(Debug)]
pub enum Error {
    Request,
    Binding,
    Session,
    Authority(crate::Error),
    Proof(swarm_enrollment_proof::Error),
}
impl Error {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Request => "request",
            Self::Binding => "binding",
            Self::Session => "session",
            Self::Authority(e) => e.code(),
            Self::Proof(e) => e.code(),
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Authority(e) => write!(f, "{e}"),
            Self::Proof(e) => write!(f, "{e}"),
            e => write!(f, "enrollment request: {}", e.code()),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Authority(e) => Some(e),
            Self::Proof(e) => Some(e),
            _ => None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Enrollment {
    pub id: String,
    pub generation: i64,
    pub head: Head,
    pub document_digest: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Publication {
    pub schema_version: i64,
    pub purpose: String,
    #[serde(rename = "project_context_digest")]
    pub project_digest: String,
    pub principal: Principal,
    pub session_digest: String,
    #[serde(
        serialize_with = "super::serialize_time",
        deserialize_with = "super::deserialize_time"
    )]
    pub session_verified_at: OffsetDateTime,
    #[serde(
        serialize_with = "super::serialize_time",
        deserialize_with = "super::deserialize_time"
    )]
    pub session_expires_at: OffsetDateTime,
    pub scope: Scope,
    #[serde(rename = "trust_configuration_revision")]
    pub trust_revision: i64,
    pub enrollment: Enrollment,
    pub event_id: String,
    pub snapshot_revision: i64,
    pub snapshot_digest: String,
    #[serde(
        serialize_with = "serialize_snapshot",
        deserialize_with = "deserialize_snapshot"
    )]
    pub snapshot: Vec<u8>,
    pub root_grants: Vec<String>,
    pub previous_registry_head_digest: String,
}
fn serialize_snapshot<S: Serializer>(raw: &[u8], s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&STANDARD.encode(raw))
}
fn deserialize_snapshot<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
    let s = String::deserialize(d)?;
    STANDARD.decode(s).map_err(serde::de::Error::custom)
}
fn hash(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}
// bytes.TrimSpace trims Unicode ends even if an opaque JSON string contains
// invalid UTF-8. Preserve all original bytes for the descriptor and its digest.
fn trim_go_space(raw: &[u8]) -> &[u8] {
    let start = raw
        .utf8_chunks()
        .next()
        .map_or(0, |c| c.valid().len() - c.valid().trim_start().len());
    let raw = &raw[start..];
    let end = raw.utf8_chunks().last().map_or(0, |c| {
        if c.invalid().is_empty() {
            c.valid().len() - c.valid().trim_end().len()
        } else {
            0
        }
    });
    &raw[..raw.len() - end]
}
fn valid_snapshot(raw: &[u8]) -> bool {
    let raw = trim_go_space(raw);
    // The shared decoder enforces Go's syntax depth before skipping raw values.
    raw.first() == Some(&b'{') && wire::decode::<serde::de::IgnoredAny>(raw).is_ok()
}
impl Publication {
    fn valid(&self) -> bool {
        if self.schema_version != 1
            || self.purpose != PUBLICATION_PURPOSE
            || !digest(self.project_digest.as_bytes())
            || !self.principal.valid()
            || self.principal.kind != "human"
            || !digest(self.session_digest.as_bytes())
            || super::zero(self.session_verified_at)
            || self.session_verified_at >= self.session_expires_at
            || !self.scope.valid()
            || self.trust_revision <= 0
            || !text(self.enrollment.id.as_bytes())
            || self.enrollment.generation <= 0
            || self.enrollment.head.sequence < self.enrollment.generation
            || !digest(self.enrollment.head.digest.as_bytes())
            || !digest(self.enrollment.document_digest.as_bytes())
            || !name(self.event_id.as_bytes())
            || self.snapshot_revision <= 0
            || !digest(self.snapshot_digest.as_bytes())
            || self.snapshot_digest != hash(&self.snapshot)
            || self.root_grants.is_empty()
            || (!self.previous_registry_head_digest.is_empty()
                && !digest(self.previous_registry_head_digest.as_bytes()))
            || !valid_snapshot(&self.snapshot)
        {
            return false;
        }
        let mut seen = HashSet::new();
        self.root_grants
            .iter()
            .all(|id| name(id.as_bytes()) && seen.insert(id))
    }
}
/// Owned validated descriptor bytes; no Default, Deserialize or public constructor.
/// Decode can prepare arbitrary valid data, without authenticating its source.
#[derive(Clone)]
pub struct Prepared {
    raw: Arc<[u8]>,
    digest: [u8; 32],
}
impl fmt::Debug for Prepared {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Prepared")
            .field("bytes", &self.raw.len())
            .field(
                "request_digest",
                &self
                    .digest
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>(),
            )
            .finish()
    }
}
/// No TTL, promotion, projection or root-order normalization is applied.
pub fn prepare(publication: &Publication) -> Result<Prepared, Error> {
    if !publication.valid() {
        return Err(Error::Request);
    }
    let raw = wire::encode(publication).map_err(|_| Error::Request)?;
    let mut preimage = format!("{PROTOCOL}\0request\0").into_bytes();
    preimage.extend(&raw);
    let digest = Sha256::digest(preimage).into();
    Ok(Prepared {
        raw: raw.into(),
        digest,
    })
}
pub fn decode(raw: &[u8]) -> Result<Prepared, Error> {
    let p: Publication = wire::decode(raw).map_err(|_| Error::Request)?;
    let prepared = prepare(&p)?;
    if prepared.raw.as_ref() != raw {
        return Err(Error::Request);
    }
    Ok(prepared)
}
impl Prepared {
    pub fn bytes(&self) -> Vec<u8> {
        self.raw.to_vec()
    }
    /// Always valid because an uninitialized Prepared cannot be constructed.
    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }
    pub fn publication(&self) -> Result<Publication, Error> {
        wire::decode(&self.raw).map_err(|_| Error::Request)
    }
    /// Derived declared context; trusted control must validate its source independently.
    pub fn expected(&self) -> Result<Expected, Error> {
        let p = self.publication()?;
        Ok(Expected {
            head: p.enrollment.head,
            enrollment_id: p.enrollment.id,
            generation: p.enrollment.generation,
            project_digest: p.project_digest,
            principal: p.principal,
            scope: p.scope,
            trust_revision: p.trust_revision,
            request_digest: self.digest.iter().map(|b| format!("{b:02x}")).collect(),
        })
    }
    pub fn new_challenge(&self, verifier: &Verifier) -> Result<Challenge, Error> {
        let challenge = verifier.new_challenge(self.digest).map_err(Error::Proof)?;
        self.bind(&challenge.request())?;
        Ok(challenge)
    }
    /// Bind data only; SignResponse/Verify still own envelope cryptography.
    pub fn bind(&self, request: &Request) -> Result<(), Error> {
        let e = self.expected()?;
        if request.project_digest != e.project_digest || request.request_digest != e.request_digest
        {
            return Err(Error::Binding);
        }
        Ok(())
    }
    pub fn check_active(
        &self,
        statement: &Statement,
        at: OffsetDateTime,
    ) -> Result<Document, Error> {
        let p = self.publication()?;
        if super::zero(at) || at < p.session_verified_at || at >= p.session_expires_at {
            return Err(Error::Session);
        }
        let authority =
            super::decode_active(statement, &self.expected()?, at).map_err(Error::Authority)?;
        let body = super::encode(&authority).map_err(|_| Error::Binding)?;
        if hash(&body) != p.enrollment.document_digest {
            return Err(Error::Binding);
        }
        Ok(authority)
    }
}
