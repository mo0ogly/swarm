//! Immutable initial operator material, before protected publication/recovery.
//! Core material functions create no files/journals/OS accounts/service/human
//! authority. The repository submodule separately publishes operator packages
//! through owned private Documents handles; it installs no service files.
//! The explicit business grant is an operator prerequisite, never inferred from
//! possession of this package. Keep its business key outside the personal service.
use crate::{credentials, personal, Document};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::TryReserveError, error::Error as StdError, fmt, io::Write};
use swarm_enrollment_bootstrap::{
    self as bootstrap, AuthenticatedDocument, ControlDocument, PersonalDeclaration,
};
use swarm_enrollment_config::Pins;
use swarm_enrollment_state::{self as state, Head};
use swarm_identity_contract::{digest, text};
use swarm_wire_json as wire;
use zeroize::Zeroizing;
/// Immutable operator package storage under an independently protected pin.
pub mod repository;
/// Immutable separately protected pin anchors; no package or authority created.
pub mod checkpoint;
/// Durable intent/anchor before dormant package publication; no service authority.
pub mod initial_creation;
/// Kernel for pinned operator observation and deterministic service-only handoff.
pub mod operator_handoff;
/// Service-only handoff data; operator business key/proof never exported there.
pub mod service_artifact;
/// Private initial-root publication; no account, listener or OS identity created.
pub mod service_installation;

/// Private package framing: two Ed25519 seeds and one installation proof, each
/// raw32 bytes, followed by the exact canonical request through EOF. These are
/// protocol sizes, not an operator password, execution or retention policy.
const HEADER: &[u8] = b"swarm-initial-provisioning-v1\n";
const SECRET_BYTES: usize = 3 * 32;

/// Explicit operator decision with no default KDF, grant, duration, UID or path.
/// Logical clean roots and binding only: neither this data nor its canonical
/// encoding establishes actual filesystem ownership, approval or human presence.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema_version: i64,
    pub installation_digest: String,
    pub root_directory: String,
    pub service_uid: u32,
    pub controller_uid: u32,
    pub grant: Document,
    pub personal_project_context_digest: String,
    pub personal_generation: i64,
    pub parameters: credentials::Parameters,
}
impl fmt::Debug for Request {
    /// Keep private operator paths, subject and rights out of ordinary diagnostics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("InitialProvisioningRequest { <private> }")
    }
}
/// Typed refusals; ordinary formatting never includes request, key, proof, path
/// or nested private IO text. Output errors never establish absence of writes.
pub enum Error {
    Request,
    Frame(serde_json::Error),
    Authority(crate::Error),
    Personal(personal::Error),
    Credentials(credentials::Error),
    Declaration(bootstrap::Error),
    Entropy(getrandom::Error),
    Allocation(TryReserveError),
    Output {
        phase: OutputPhase,
        cause: std::io::Error,
    },
}
/// Known private sink phase, without claiming filesystem durability or receipt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputPhase {
    /// write_all refused; zero, some or all bytes may already be delivered.
    WriteUnknown,
    /// All32 bytes were acknowledged before flush refused; close/fsync/receiver
    /// processing are still unknown and remain the sink owner's responsibility.
    WrittenFlushFailed,
}
impl Error {
    /// Stable safe classification; typed source retains the original cause.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Request => "provisioning-request",
            Self::Frame(_) => "provisioning-frame",
            Self::Authority(_) => "provisioning-grant",
            Self::Personal(_) => "provisioning-personal",
            Self::Credentials(_) => "provisioning-parameters",
            Self::Declaration(_) => "provisioning-declaration",
            Self::Entropy(_) => "provisioning-entropy",
            Self::Allocation(_) => "provisioning-allocation",
            Self::Output { .. } => "provisioning-output",
        }
    }
    /// Retain the actually known delivery phase after a sink failure. None means
    /// this error is not a sink error, never a claim about some earlier caller IO.
    pub fn output_phase(&self) -> Option<OutputPhase> {
        match self {
            Self::Output { phase, .. } => Some(*phase),
            _ => None,
        }
    }
}
impl fmt::Display for Error {
    /// No private input or underlying IO diagnostic is formatted.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "initial provisioning: {}", self.code())
    }
}
impl fmt::Debug for Error {
    /// Same redaction as Display, even for partial private output failures.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Error {
    /// Actual primary cause; no cleanup branch because this module opens no FD
    /// and borrows sinks. Caller owns sink close, durability and publication state.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Frame(e) => Some(e),
            Self::Authority(e) => Some(e),
            Self::Personal(e) => Some(e),
            Self::Credentials(e) => Some(e),
            Self::Declaration(e) => Some(e),
            Self::Entropy(e) => Some(e),
            Self::Allocation(e) => Some(e),
            Self::Output { cause, .. } => Some(cause),
            Self::Request => None,
        }
    }
}

/// Validate the operator request before consuming entropy or touching storage.
/// Grant generation1 is the actual initial journal sequence, not a default or
/// retry count. Personal generation is an independent explicit positive label.
/// UID roles must be representable Linux UIDs, including possible UID0; rejecting
/// the reserved sentinel happens before entropy/material, not by a policy default.
fn validate(r: &Request) -> Result<(), Error> {
    crate::encode(&r.grant).map_err(Error::Authority)?;
    r.parameters.validate().map_err(Error::Credentials)?;
    let path = &r.root_directory;
    if r.schema_version != 1
        || !digest(r.installation_digest.as_bytes())
        || !text(path.as_bytes())
        || !path.starts_with('/')
        || path == "/"
        || path[1..]
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
        || !swarm_identity_contract::linux_uid(r.service_uid)
        || !swarm_identity_contract::linux_uid(r.controller_uid)
        || r.service_uid == r.controller_uid
        || r.grant.principal.kind != "human"
        || r.grant.generation != 1
        || r.personal_generation <= 0
        || !digest(r.personal_project_context_digest.as_bytes())
        || r.personal_project_context_digest == r.grant.project_digest
    {
        return Err(Error::Request);
    }
    Ok(())
}
/// Canonical closed request data; no key, entropy, IO, adoption or authority.
pub fn encode_request(r: &Request) -> Result<Vec<u8>, Error> {
    validate(r)?;
    wire::encode(r).map_err(Error::Frame)
}
/// Refuse unknown fields, nulls, duplicates, alternate encodings and trailing
/// bytes. No fallback/migration/default and no entropy generated after refusal.
pub fn decode_request(raw: &[u8]) -> Result<Request, Error> {
    let r: Request = wire::decode(raw).map_err(Error::Frame)?;
    if encode_request(&r)? != raw {
        return Err(Error::Request);
    }
    Ok(r)
}
/// Convert an already canonical digest without normalization or filesystem IO.
fn project(raw: &str) -> Result<[u8; 32], Error> {
    if !digest(raw.as_bytes()) {
        return Err(Error::Request);
    }
    let mut p = [0; 32];
    for (index, byte) in p.iter_mut().enumerate() {
        *byte =
            u8::from_str_radix(&raw[index * 2..index * 2 + 2], 16).map_err(|_| Error::Request)?
    }
    Ok(p)
}
/// Own initial private material without Clone/Serialize or ordinary secret
/// formatting. Dalek keys explicitly zeroize on drop; proof/codec owned buffers
/// zeroize. Caller, compiler/runtime/OS copies and abrupt exit are not wiped here.
pub struct Prepared {
    request: Request,
    business_key: SigningKey,
    personal_key: SigningKey,
    proof: Zeroizing<[u8; 32]>,
    business_initial: Vec<u8>,
    personal_initial: Vec<u8>,
}
impl fmt::Debug for Prepared {
    /// Never print seeds, proof, verifiers, identities or the private request.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("InitialProvisioningMaterial { <private> }")
    }
}
/// Validate first, obtain one kernel-random96byte draw, then derive two distinct
/// keys and proof. Entropy failure may have written a prefix; guarded bytes drop
/// on every issue. No retry or deterministic fallback and no file is created.
pub fn prepare(request: &Request) -> Result<Prepared, Error> {
    prepare_with_entropy(request, |bytes| {
        getrandom::fill(bytes).map_err(Error::Entropy)
    })
}
/// Internal injectable byte source solely for in-module refusal witnesses, not
/// a public deterministic-key option, feature, environment setting or policy.
fn prepare_with_entropy(
    request: &Request,
    entropy: impl FnOnce(&mut [u8]) -> Result<(), Error>,
) -> Result<Prepared, Error> {
    validate(request)?;
    let mut raw = Zeroizing::new([0; SECRET_BYTES]);
    entropy(raw.as_mut_slice())?;
    from_material(request, &raw)
}
/// Reconstruct exact deterministic Ed25519 proposals from protected package
/// material. No journal/head discovery, entropy, account/password creation or IO.
fn from_material(request: &Request, raw: &[u8; SECRET_BYTES]) -> Result<Prepared, Error> {
    validate(request)?;
    let business_key = SigningKey::from_bytes((&raw[..32]).try_into().map_err(|_| Error::Request)?);
    let personal_key =
        SigningKey::from_bytes((&raw[32..64]).try_into().map_err(|_| Error::Request)?);
    if business_key.verifying_key() == personal_key.verifying_key() {
        return Err(Error::Request);
    }
    let mut proof = Zeroizing::new([0; 32]);
    proof.copy_from_slice(&raw[64..]);
    let business_initial = crate::sign_enrollment(
        &business_key,
        project(&request.grant.project_digest)?,
        &Head::default(),
        &request.grant,
    )
    .map_err(Error::Authority)?;
    let bootstrap = personal::Bootstrap {
        schema_version: 1,
        installation_digest: request.installation_digest.clone(),
        principal: request.grant.principal.clone(),
        generation: request.personal_generation,
        parameters: request.parameters,
        proof_commitment: personal::commitment(&request.installation_digest, &proof)
            .map_err(Error::Personal)?,
    };
    let personal_initial = state::sign_change(
        &personal_key,
        project(&request.personal_project_context_digest)?,
        &Head::default(),
        "enroll",
        personal::BOOTSTRAP_ID,
        &personal::encode_bootstrap(&bootstrap).map_err(Error::Personal)?,
    )
    .map_err(|e| Error::Personal(personal::Error::State(e)))?;
    let prepared = Prepared {
        request: request.clone(),
        business_key,
        personal_key,
        proof,
        business_initial,
        personal_initial,
    };
    prepared.declaration()?;
    Ok(prepared)
}
/// Decode operator-private package with exact framing/canonical request. The
/// caller must independently protect/bind these bytes; this codec has no MAC,
/// storage pin or authentication claim. Borrowed raw bytes remain caller-owned.
pub fn decode_private(raw: &[u8]) -> Result<Prepared, Error> {
    let offset = HEADER.len() + SECRET_BYTES;
    if raw.len() <= offset || !raw.starts_with(HEADER) {
        return Err(Error::Request);
    }
    let request = decode_request(&raw[offset..])?;
    let mut secrets = Zeroizing::new([0; SECRET_BYTES]);
    secrets.copy_from_slice(&raw[HEADER.len()..offset]);
    from_material(&request, &secrets)
}
impl Prepared {
    /// Borrow immutable canonical operator request data without IO, entropy,
    /// key copies or mutation. Values describe the prepared package only: they
    /// authenticate neither a current caller/account/human nor live authority.
    /// No private seed/proof is returned; caller must still protect private
    /// identity/path/grant data and independently verify the current consumer.
    pub fn request(&self) -> &Request {
        &self.request
    }
    /// Exact initial signed business event, not its commitment/permission to
    /// apply. Consumer must durably retain intent before actual journal apply.
    pub fn business_initial(&self) -> &[u8] {
        &self.business_initial
    }
    /// Exact initial personal bootstrap event; contains private verifier data,
    /// not the installation proof. Same intent/publication requirements apply.
    pub fn personal_initial(&self) -> &[u8] {
        &self.personal_initial
    }
    /// Build v3 expected startup only from this material's exact proposals.
    /// Journal protocol heads are sequence1/SHA256(canonical signed bytes), as
    /// Journal::apply defines. Never reads/adopts database heads. Fixed child
    /// names are installation layout, not operator execution policy. No file is
    /// created and no real UID, protected root or grant approval is established.
    pub fn declaration(&self) -> Result<AuthenticatedDocument, Error> {
        let r = &self.request;
        let path = |p: &str| format!("{}/{}", r.root_directory, p);
        let head = |raw: &[u8]| Head {
            sequence: 1,
            digest: format!("{:x}", Sha256::digest(raw)),
        };
        let d = AuthenticatedDocument {
            schema_version: 3,
            control: ControlDocument {
                schema_version: 2,
                installation: bootstrap::Document {
                    schema_version: 1,
                    pins: Pins {
                        installation_digest: r.installation_digest.clone(),
                        public_key: URL_SAFE_NO_PAD
                            .encode(self.business_key.verifying_key().to_bytes()),
                        principal: r.grant.principal.clone(),
                    },
                    service_uid: r.service_uid,
                    controller_uid: r.controller_uid,
                    configuration_directory: path("configuration"),
                    witness_directory: path("configuration-witness"),
                },
                journal_path: path("business/journal.sqlite"),
                project_context_digest: r.grant.project_digest.clone(),
                authority_head: head(&self.business_initial),
                enrollment_id: r.grant.enrollment_id.clone(),
                generation: r.grant.generation,
                scope: r.grant.scope.clone(),
                trust_configuration_revision: r.grant.trust_revision,
            },
            personal: PersonalDeclaration {
                authority_public_key: URL_SAFE_NO_PAD
                    .encode(self.personal_key.verifying_key().to_bytes()),
                project_context_digest: r.personal_project_context_digest.clone(),
                journal_path: path("personal/journal.sqlite"),
                checkpoint_directory: path("personal-witness"),
                generation: r.personal_generation,
                bootstrap_head: head(&self.personal_initial),
            },
        };
        bootstrap::encode_authenticated(&d).map_err(Error::Declaration)?;
        Ok(d)
    }
    /// Encode the operator-private package, including business key. Never place
    /// this package in agent or personal-service storage. One exact allocation
    /// before copying secrets; no secret-bearing growth. Guarded temporary seed
    /// copies and result zeroize; borrowed/OS/compiler copies are caller limits.
    /// This is explicit private serialization, not a public-key/authority export.
    pub fn encode_private(&self) -> Result<Zeroizing<Vec<u8>>, Error> {
        let request = encode_request(&self.request)?;
        let size = HEADER
            .len()
            .checked_add(SECRET_BYTES)
            .and_then(|n| n.checked_add(request.len()))
            .ok_or(Error::Request)?;
        let mut raw = Zeroizing::new(Vec::new());
        raw.try_reserve_exact(size).map_err(Error::Allocation)?;
        raw.extend_from_slice(HEADER);
        let business = Zeroizing::new(self.business_key.to_bytes());
        let personal = Zeroizing::new(self.personal_key.to_bytes());
        raw.extend_from_slice(&*business);
        raw.extend_from_slice(&*personal);
        raw.extend_from_slice(&*self.proof);
        raw.extend_from_slice(&request);
        Ok(raw)
    }
    /// Deliver only the personal32byte seed to a trusted private installer sink.
    /// Borrowed sink owns publication/fsync/close and may have partial writes on
    /// error; no no-effect claim. Business key is never passed to this sink.
    pub fn write_personal_seed(&self, out: &mut dyn Write) -> Result<(), Error> {
        let raw = Zeroizing::new(self.personal_key.to_bytes());
        private_output(&raw, out)
    }
    /// Independently deliver raw32 installation proof to a trusted operator sink,
    /// never an agent channel. Borrowed sink may retain partial bytes on refusal;
    /// caller owns close/fsync/confidentiality. This does not prove human receipt.
    pub fn write_installation_proof(&self, out: &mut dyn Write) -> Result<(), Error> {
        private_output(&self.proof, out)
    }
}

/// Borrow a protocol-sized secret and sink, retaining acknowledged write versus
/// unknown partial write when flush/write refuses. No FD ownership or retry.
fn private_output(raw: &[u8; 32], out: &mut dyn Write) -> Result<(), Error> {
    out.write_all(raw).map_err(|cause| Error::Output {
        phase: OutputPhase::WriteUnknown,
        cause,
    })?;
    out.flush().map_err(|cause| Error::Output {
        phase: OutputPhase::WrittenFlushFailed,
        cause,
    })
}

#[cfg(test)]
mod tests;
