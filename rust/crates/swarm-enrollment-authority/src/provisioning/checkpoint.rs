//! Immutable private pin anchor, separate from received package bytes.
//! Caller protects directory/name as the independent trust locator before use.
//! No package/key generation, authority, rollback or human/UID separation proof.
use super::repository;
use serde::{Deserialize, Serialize};
use std::{error::Error as StdError, fmt};
use swarm_enrollment_config::{
    self as config,
    store::{
        documents::{
            Documents, DurableReadError, Publication, ReadDurability, ReadValidation, Reservation,
            WriteError,
        },
        Context,
    },
};
use swarm_wire_json as wire;
/// Closed canonical metadata; private fields prevent unchecked edits. Digests
/// bind data only, not authorization. Debug never discloses either private pin.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {
    schema_version: i64,
    protocol: String,
    request_pin: String,
    package_pin: String,
}
impl fmt::Debug for Checkpoint {
    /// Pins are explicit private projections only, never ordinary log values.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OperatorCheckpoint { <private> }")
    }
}
impl Checkpoint {
    /// Bind a canonical request digest to the seal pin known BEFORE publication.
    /// Pure validation/allocation only, no IO/entropy or claim that a caller kept it.
    pub fn new(request_pin: &str, package_pin: &repository::Pin) -> Result<Self, Error> {
        repository::Pin::parse(request_pin).map_err(Error::Pin)?;
        Ok(Self {
            schema_version: 1,
            protocol: "swarm-operator-anchor-v1".into(),
            request_pin: request_pin.into(),
            package_pin: package_pin.as_str().into(),
        })
    }
    /// Private caller output for matching the separately pinned canonical Request.
    /// Possession/equality is data only, never human/current business authority.
    pub fn request_pin(&self) -> &str {
        &self.request_pin
    }
    /// Expected package digest from this already PROTECTED anchor locator. Never
    /// construct/adopt an anchor from an untrusted package's observed digest.
    pub fn package_pin(&self) -> &str {
        &self.package_pin
    }
    /// Validate closed protocol and both canonical pins; no IO or auth inference.
    fn validate(&self) -> Result<(), Error> {
        if self.schema_version != 1 || self.protocol != "swarm-operator-anchor-v1" {
            return Err(Error::Document);
        }
        repository::Pin::parse(&self.request_pin).map_err(Error::Pin)?;
        repository::Pin::parse(&self.package_pin).map_err(Error::Pin)?;
        Ok(())
    }
    /// Exact versioned JSON bytes with fixed member order/no whitespace suffix.
    /// Contains private metadata only, no key/proof. Caller owns storage protection.
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        self.validate()?;
        wire::encode(self).map_err(Error::Json)
    }
    /// Reject missing/unknown/duplicate/wrong fields and any noncanonical bytes.
    /// A successful parse alone establishes no protected anchor provenance.
    pub fn decode(raw: &[u8]) -> Result<Self, Error> {
        let value: Self = wire::decode(raw).map_err(Error::Json)?;
        value.validate()?;
        if value.encode()?.as_slice() != raw {
            return Err(Error::Document);
        }
        Ok(value)
    }
}
/// Typed primary failure plus independent cleanup. Formatting redacts all pins,
/// paths, private bytes and raw IO/JSON diagnostics; source retains original.
pub enum Error {
    Document,
    RequestBinding,
    Pin(repository::Error),
    Json(serde_json::Error),
    Store(config::Error),
    Write(WriteError),
    Read(DurableReadError),
    Joined {
        cause: Box<Error>,
        cleanup: config::Error,
    },
}
impl Error {
    /// Stable safe classification only; do not classify by display/error wording.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Document => "operator-anchor-document",
            Self::RequestBinding => "operator-anchor-request",
            Self::Pin(e) => e.code(),
            Self::Json(_) => "operator-anchor-json",
            Self::Store(e) => e.code(),
            Self::Write(e) => e.cause.code(),
            Self::Read(e) => e.cause.code(),
            Self::Joined { cause, .. } => cause.code(),
        }
    }
}
impl fmt::Display for Error {
    /// Safe category only, even when the original cause contains private input.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for Error {
    /// Same redaction as Display, including assertion/fixture failures.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Error {
    /// Primary typed source; error_links additionally exposes cleanup siblings.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Pin(e) => Some(e),
            Self::Json(e) => Some(e),
            Self::Store(e) => Some(e),
            Self::Write(e) => Some(e),
            Self::Read(e) => Some(e),
            Self::Joined { cause, .. } => Some(cause.as_ref()),
            _ => None,
        }
    }
}
/// Real create publication survives write/release/close refusal; no implicit retry.
pub struct CommitError {
    pub publication: Publication,
    pub cause: Error,
}
impl fmt::Display for CommitError {
    /// Safe physical stage only, never anchor pins/paths or nested diagnostic text.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "operator anchor {:?}: {}",
            self.publication,
            self.cause.code()
        )
    }
}
impl fmt::Debug for CommitError {
    /// Same redaction as Display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for CommitError {
    /// Preserve original typed primary cause plus traversal cleanup siblings.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.cause)
    }
}
/// Source bytes/ack/decode facts survive refusal; no usable Checkpoint is returned.
pub struct ReadError {
    pub validation: ReadValidation,
    pub durability: ReadDurability,
    pub decoded: bool,
    pub cause: Error,
}
impl fmt::Display for ReadError {
    /// No private metadata or nested JSON/IO values in ordinary diagnostics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "operator anchor {:?} {:?} decoded={}: {}",
            self.validation,
            self.durability,
            self.decoded,
            self.cause.code()
        )
    }
}
impl fmt::Debug for ReadError {
    /// Same redaction as Display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for ReadError {
    /// Original typed primary cause, including any independent close branches.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.cause)
    }
}
/// Explicit cleanup consumes success on late failure and retains both errors.
/// This helper performs no IO/retry; owners invoke it after real release/close.
fn finish<T>(result: Result<T, Error>, cleanup: Result<(), config::Error>) -> Result<T, Error> {
    match (result, cleanup) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(Error::Store(e)),
        (Err(cause), Err(cleanup)) => Err(Error::Joined {
            cause: Box::new(cause),
            cleanup,
        }),
    }
}
/// Publish absent canonical anchor with NOREPLACE, then finish and close ALL
/// supplied Documents handles before returning. Caller selected/protects locator,
/// retains seal and must not publish its package until this returns Durable.
/// A refusal may leave an anchor visible/durable; preserve it and never regenerate
/// implicitly. Existing anchors (even equal) refuse without overwrite. No package,
/// Root, journal, key/proof, account or service is created by this operation.
pub fn create(
    docs: Documents,
    name: &str,
    checkpoint: &Checkpoint,
    ctx: &Context,
) -> Result<Publication, CommitError> {
    let mut publication = Publication::NotAttempted;
    let result = (|| {
        let raw = checkpoint.encode()?;
        let r = docs.reserve(ctx).map_err(Error::Store)?;
        let result = create_raw_reserved(&r, name, &raw);
        publication = match &result {
            Ok(p) => *p,
            Err(e) => e.publication,
        };
        let result = result.map_err(|e| e.cause);
        finish(result, r.finish())
    })();
    finish(result, docs.close()).map_err(|cause| CommitError { publication, cause })
}
/// Read/fsync an independently protected anchor locator, validate canonical record
/// and expected request pin, then finish/close before returning usable metadata.
/// No expected package digest is learned from package bytes: no package is read.
/// The protected locator is the caller's trust precondition; owned descriptors,
/// exact bytes and local FS acknowledgement do NOT prove human/current authority,
/// freshness/rollback resistance, real UID separation or hardware survival.
/// Every late failure withholds the Checkpoint and retains decode/ack/all causes.
pub fn load(
    docs: Documents,
    name: &str,
    expected_request_pin: &str,
    ctx: &Context,
) -> Result<Checkpoint, ReadError> {
    let mut validation = ReadValidation::NotAttempted;
    let mut durability = ReadDurability::NotAcknowledged;
    let mut decoded = false;
    let result = (|| {
        repository::Pin::parse(expected_request_pin).map_err(Error::Pin)?;
        let r = docs.reserve(ctx).map_err(Error::Store)?;
        let result = load_reserved(&r, name, expected_request_pin);
        (validation, durability, decoded) = match &result {
            Ok(_) => (
                ReadValidation::BytesRead,
                ReadDurability::DirectoryAcknowledged,
                true,
            ),
            Err(e) => (e.validation, e.durability, e.decoded),
        };
        finish(result.map_err(|e| e.cause), r.finish())
    })();
    finish(result, docs.close()).map_err(|cause| ReadError {
        validation,
        durability,
        decoded,
        cause,
    })
}
/// Canonical record publication under an ALREADY HELD reservation. No finish,
/// close or key generation: caller owns independent locator and complete cleanup.
/// Result keeps every actual publication phase; existing records never adopted.
pub(super) fn create_reserved(
    r: &Reservation<'_>,
    name: &str,
    checkpoint: &Checkpoint,
) -> Result<Publication, CommitError> {
    let raw = checkpoint.encode().map_err(|cause| CommitError {
        publication: Publication::NotAttempted,
        cause,
    })?;
    create_raw_reserved(r, name, &raw)
}
/// Write already canonical metadata shared by standalone and coordinated paths.
/// Internal callers alone supply encoded record bytes; no release/close here.
fn create_raw_reserved(
    r: &Reservation<'_>,
    name: &str,
    raw: &[u8],
) -> Result<Publication, CommitError> {
    r.create(name, raw).map_err(|e| CommitError {
        publication: e.publication,
        cause: Error::Write(e),
    })
}
/// Any preexisting anchor refuses before generation, without adopting its data.
/// Only authoritative missing-file refusal permits absence; borrowed reservation
/// and its retained directory/lock ownership remain the caller's responsibility.
pub(super) fn require_absent(r: &Reservation<'_>, name: &str) -> Result<(), Error> {
    match r.read_private(name) {
        Err(config::Error::NotConfigured) => Ok(()),
        Ok(_) => Err(Error::Store(config::Error::Exists)),
        Err(e) => Err(Error::Store(e)),
    }
}
/// Read/ack/decode an already protected anchor on a borrowed reservation, with
/// expected request binding. No package/key generation or new locator trust.
/// Caller must finish/close and withhold returned data on any cleanup failure.
pub(super) fn load_reserved(
    r: &Reservation<'_>,
    name: &str,
    expected_request_pin: &str,
) -> Result<Checkpoint, ReadError> {
    let mut validation = ReadValidation::NotAttempted;
    let mut durability = ReadDurability::NotAcknowledged;
    let mut decoded = false;
    let result = (|| {
        repository::Pin::parse(expected_request_pin).map_err(Error::Pin)?;
        let raw = match r.read_trusted_private_durable(name) {
            Ok(raw) => {
                validation = ReadValidation::BytesRead;
                durability = ReadDurability::DirectoryAcknowledged;
                raw
            }
            Err(e) => {
                validation = e.validation;
                durability = e.durability;
                return Err(Error::Read(e));
            }
        };
        let checkpoint = Checkpoint::decode(raw.as_bytes())?;
        drop(raw);
        decoded = true;
        if checkpoint.request_pin != expected_request_pin {
            return Err(Error::RequestBinding);
        }
        Ok(checkpoint)
    })();
    result.map_err(|cause| ReadError {
        validation,
        durability,
        decoded,
        cause,
    })
}
/// Add this anchor's independent cleanup to the established error traversal.
/// Caller supplies ordinary source traversal, wide identity cycle detection and
/// the repository/config/IO wrapper matrix (operator_handoff::error_links).
/// This adds observations only; it performs no IO, retry or state change.
pub fn error_links<'a>(
    e: &'a (dyn StdError + 'static),
) -> (Vec<String>, Vec<&'a (dyn StdError + 'static)>) {
    let (mut labels, mut children) = super::operator_handoff::error_links(e);
    if let Some(e) = e.downcast_ref::<Error>() {
        labels.push(format!("operator-anchor:{}", e.code()));
        if let Error::Joined { cleanup, .. } = e {
            children.push(cleanup);
        }
    }
    if let Some(e) = e.downcast_ref::<CommitError>() {
        labels.push(format!("operator-anchor-publication:{:?}", e.publication));
    }
    if let Some(e) = e.downcast_ref::<ReadError>() {
        labels.push(format!("operator-anchor-validation:{:?}", e.validation));
        labels.push(format!("operator-anchor-durability:{:?}", e.durability));
        labels.push(format!("operator-anchor-decoded:{}", e.decoded));
    }
    (labels, children)
}
