//! Service-only installation data: personal seed, pinned declaration and opaque
//! signed initial events. Never contains the operator business seed or proof.
//! Parsing/binding is not signature/current-authority/human/OS verification;
//! trusted publication must use the real journals before exposing a final root.
use super::{private_output, Prepared};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};
use std::{collections::TryReserveError, error::Error as StdError, fmt, io::Write, path::Path};
use subtle::ConstantTimeEq;
use swarm_enrollment_bootstrap::{self as bootstrap, AuthenticatedDocument};
use zeroize::Zeroizing;
/// Fixed framing/version and seed width are protocol, never operator policy.
const HEADER: &[u8] = b"swarm-service-installation-v1\n";
/// Typed safe refusal; ordinary formatting never prints input/path/pin/key data.
pub enum Error {
    Pin,
    Frame,
    Binding,
    Material(super::Error),
    Declaration(bootstrap::Error),
    Allocation(TryReserveError),
}
impl Error {
    /// Static category only. Original material/output phases remain in source.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Pin => "service-artifact-pin",
            Self::Frame => "service-artifact-frame",
            Self::Binding => "service-artifact-binding",
            Self::Material(e) => e.code(),
            Self::Declaration(_) => "service-artifact-document",
            Self::Allocation(_) => "service-artifact-allocation",
        }
    }
}
impl fmt::Display for Error {
    /// No original private diagnostics or decoded contents are included.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "service artifact: {}", self.code())
    }
}
impl fmt::Debug for Error {
    /// Same safe diagnostic as Display, including underlying allocation/IO errors.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Error {
    /// Preserve typed causes; material output can retain known write/flush phase.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Material(e) => Some(e),
            Self::Declaration(e) => Some(e),
            Self::Allocation(e) => Some(e),
            _ => None,
        }
    }
}
/// Non-Clone/non-Serialize service material. Seed and encoded private result
/// zeroize on drop; caller/compiler/OS copies are outside that erasure claim.
pub struct ServiceArtifact {
    seed: Zeroizing<[u8; 32]>,
    document: AuthenticatedDocument,
    business: Vec<u8>,
    personal: Vec<u8>,
    root: String,
}
impl fmt::Debug for ServiceArtifact {
    /// No seed, verifier, path, identity, size or pin is exposed.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ServiceInstallationArtifact { <private> }")
    }
}
/// Extract only existing personal seed/declaration/signed proposals. No fresh
/// entropy, KDF, signature, account, grant or IO. Operator business seed and proof
/// are neither copied nor placed into this type. Protect the result's pin before
/// handoff; canonical data does not authenticate its operator or receiving UID.
pub fn from_prepared(material: &Prepared) -> Result<ServiceArtifact, Error> {
    let seed = Zeroizing::new(material.personal_key.to_bytes());
    let document = material.declaration().map_err(Error::Material)?;
    let business = material.business_initial().to_vec();
    let personal = material.personal_initial().to_vec();
    let root = binding(&seed, &document, &business, &personal)?;
    Ok(ServiceArtifact {
        seed,
        document,
        business,
        personal,
        root,
    })
}
/// Derive the fixed initial layout after the shared closed v3 validator. Reject
/// root/multi-root layouts so later atomic service-root publication cannot write
/// elsewhere. This is lexical binding, not filesystem/UID/approval observation.
fn root(document: &AuthenticatedDocument) -> Result<String, Error> {
    bootstrap::encode_authenticated(document).map_err(Error::Declaration)?;
    let d = &document.control;
    let installation = &d.installation;
    let root = Path::new(&installation.configuration_directory)
        .parent()
        .and_then(Path::to_str)
        .filter(|p| !p.is_empty() && *p != "/")
        .ok_or(Error::Binding)?;
    let child = |n: &str| format!("{root}/{n}");
    if installation.configuration_directory != child("configuration")
        || installation.witness_directory != child("configuration-witness")
        || d.journal_path != child("business/journal.sqlite")
        || document.personal.journal_path != child("personal/journal.sqlite")
        || document.personal.checkpoint_directory != child("personal-witness")
    {
        return Err(Error::Binding);
    }
    Ok(root.into())
}
/// Bind personal key and exact event-byte expected heads. Opaque event bodies,
/// signatures, business/personal semantics and current authority are deliberately
/// verified later by the authoritative journals/startup, not duplicated here.
fn binding(
    seed: &[u8; 32],
    document: &AuthenticatedDocument,
    business: &[u8],
    personal: &[u8],
) -> Result<String, Error> {
    let root = root(document)?;
    let key = SigningKey::from_bytes(seed);
    if URL_SAFE_NO_PAD.encode(key.verifying_key().to_bytes())
        != document.personal.authority_public_key
        || document.control.generation != 1
        || document.control.authority_head.sequence != 1
        || document.personal.bootstrap_head.sequence != 1
        || business.is_empty()
        || personal.is_empty()
        || document.control.authority_head.digest != hash(business)
        || document.personal.bootstrap_head.digest != hash(personal)
    {
        return Err(Error::Binding);
    }
    Ok(root)
}
/// Exact bytes digest only, no authenticity/freshness or approval inferred.
fn hash(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}
/// Borrow one checked frame range without allocation or usize wrap/truncation.
fn take<'a>(raw: &'a [u8], offset: &mut usize, len: usize) -> Result<&'a [u8], Error> {
    let end = offset.checked_add(len).ok_or(Error::Frame)?;
    let value = raw.get(*offset..end).ok_or(Error::Frame)?;
    *offset = end;
    Ok(value)
}
/// Read a protocol u64BE-length byte frame. Representation errors refuse; no
/// product volume cutoff/default is hidden in this memory-only codec.
fn piece<'a>(raw: &'a [u8], offset: &mut usize) -> Result<&'a [u8], Error> {
    let length = u64::from_be_bytes(take(raw, offset, 8)?.try_into().map_err(|_| Error::Frame)?);
    take(
        raw,
        offset,
        usize::try_from(length).map_err(|_| Error::Frame)?,
    )
}
/// Decode only under independently supplied canonical SHA256 before parsing.
/// Header/raw32seed/threeu64BEframes/exactEOF, canonicalv3/personalkey/layout/head
/// binding. No IO, entropy, fallback, signature/admission/current-state claim.
/// Borrowed raw bytes remain caller-owned; protect the pin outside the request.
pub fn decode(raw: &[u8], expected_pin: &str) -> Result<ServiceArtifact, Error> {
    if !swarm_identity_contract::digest(expected_pin.as_bytes())
        || !bool::from(hash(raw).as_bytes().ct_eq(expected_pin.as_bytes()))
    {
        return Err(Error::Pin);
    }
    if !raw.starts_with(HEADER) {
        return Err(Error::Frame);
    }
    let mut offset = HEADER.len();
    let mut seed = Zeroizing::new([0; 32]);
    seed.copy_from_slice(take(raw, &mut offset, 32)?);
    let document =
        bootstrap::decode_authenticated(piece(raw, &mut offset)?).map_err(Error::Declaration)?;
    let business = piece(raw, &mut offset)?;
    let personal = piece(raw, &mut offset)?;
    if offset != raw.len() {
        return Err(Error::Frame);
    }
    let root = binding(&seed, &document, business, personal)?;
    Ok(ServiceArtifact {
        seed,
        document,
        business: business.to_vec(),
        personal: personal.to_vec(),
        root,
    })
}
impl ServiceArtifact {
    /// Expected canonical declaration data, not loaded storage or current grant.
    pub fn document(&self) -> &AuthenticatedDocument {
        &self.document
    }
    /// Clean fixed-layout final root label; later publisher validates real opened
    /// parent/root/UID and operator-vault separation before file effects.
    pub fn root_directory(&self) -> &str {
        &self.root
    }
    /// Exact opaque business proposal. Authoritative journal verifies signature,
    /// context/CAS/body before commit; this borrow never authorizes its use.
    pub fn business_initial(&self) -> &[u8] {
        &self.business
    }
    /// Exact opaque personal bootstrap proposal, with commitment but no proof.
    pub fn personal_initial(&self) -> &[u8] {
        &self.personal
    }
    /// Binary private handoff, one reservation before copying personal seed.
    /// Business key/initialproof absent by structure. Lengths are checked u64BE
    /// representation; no secret-bearing growth or operator limit. Caller must
    /// protect the guarded result/independent pin; this does not encrypt bytes.
    pub fn encode_private(&self) -> Result<Zeroizing<Vec<u8>>, Error> {
        binding(&self.seed, &self.document, &self.business, &self.personal)?;
        let document =
            bootstrap::encode_authenticated(&self.document).map_err(Error::Declaration)?;
        let parts = [
            document.as_slice(),
            self.business.as_slice(),
            self.personal.as_slice(),
        ];
        let mut capacity = HEADER.len().checked_add(32).ok_or(Error::Frame)?;
        for part in parts {
            u64::try_from(part.len()).map_err(|_| Error::Frame)?;
            capacity = capacity
                .checked_add(8)
                .and_then(|n| n.checked_add(part.len()))
                .ok_or(Error::Frame)?;
        }
        let mut raw = Zeroizing::new(Vec::new());
        raw.try_reserve_exact(capacity).map_err(Error::Allocation)?;
        raw.extend_from_slice(HEADER);
        raw.extend_from_slice(&*self.seed);
        for part in parts {
            let len = u64::try_from(part.len()).map_err(|_| Error::Frame)?;
            raw.extend_from_slice(&len.to_be_bytes());
            raw.extend_from_slice(part)
        }
        Ok(raw)
    }
    /// Expected exact private handoff digest for independent retention before IO;
    /// never use a digest learned from received bytes as authentication.
    pub fn pin(&self) -> Result<String, Error> {
        Ok(hash(&self.encode_private()?))
    }
    /// Deliver only personalraw32seed to a trusted private sink. Preserve actual
    /// unknownwrite versus allbyteswritten/flushfailed via the original error.
    /// Caller owns close/fsync/publication; no borrowed/OS/compiler-copy wipe claim.
    pub fn write_personal_seed(&self, out: &mut dyn Write) -> Result<(), Error> {
        private_output(&self.seed, out).map_err(Error::Material)
    }
}
