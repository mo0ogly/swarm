//! Immutable private operator package only, before service file publication.
//! A pin must be protected independently before writes. This store proves no
//! human approval, freshness, OS separation, journal effect or installation.
use super::{decode_private, Prepared};
use sha2::{Digest, Sha256};
use std::{error::Error as StdError, fmt};
use subtle::ConstantTimeEq;
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
use zeroize::Zeroizing;
/// Singleton immutable package layout, not operator retention or execution policy.
const NAME: &str = "initial-provisioning-v1.bin";
/// Exact-byte expected digest only. Caller supplies its independent protection;
/// parsing/cloning this data never creates authority or an authenticated pin.
#[derive(Clone, PartialEq, Eq)]
pub struct Pin(String);
impl Pin {
    /// Require canonical lowercase SHA256, without reading/adopting a package.
    pub fn parse(raw: &str) -> Result<Self, Error> {
        if !swarm_identity_contract::digest(raw.as_bytes()) {
            return Err(Error::Pin);
        }
        Ok(Self(raw.into()))
    }
    /// Explicit data projection for an independently protected caller checkpoint.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Debug for Pin {
    /// Do not include private installation identity/digest in ordinary logs.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OperatorPackagePin { <private> }")
    }
}
/// Guarded exact private candidate, not Clone/Serialize. Keep it and its pin
/// across failed/uncertain create; never regenerate secrets for an exact retry.
pub struct Seal {
    raw: Zeroizing<Vec<u8>>,
    pin: Pin,
}
impl Seal {
    /// Data known before IO; caller must retain it independently before effects.
    pub fn pin(&self) -> &Pin {
        &self.pin
    }
}
impl fmt::Debug for Seal {
    /// No key, proof, request, size, path or pin is printed.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OperatorPackageSeal { <private> }")
    }
}
/// Consume generated material into its exact guarded package. The original keys
/// drop/zeroize after encoding; no file/FD, grant or service is created. This
/// package contains the business key: keep it outside the personal service.
pub fn seal(prepared: Prepared) -> Result<Seal, Error> {
    let raw = prepared.encode_private().map_err(Error::Material)?;
    let pin = Pin(format!("{:x}", Sha256::digest(&*raw)));
    Ok(Seal { raw, pin })
}
/// Typed primary and independent release/cleanup causes. Ordinary formatting
/// excludes private IO text, package bytes, pins and decoded key material.
pub enum Error {
    Pin,
    Material(super::Error),
    Store(config::Error),
    Write(WriteError),
    DurableRead(DurableReadError),
    Joined {
        cause: Box<Error>,
        cleanup: config::Error,
    },
}
impl Error {
    /// Stable safe category; preserve concrete storage/material refusal classes.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Pin => "operator-package-pin",
            Self::Material(e) => e.code(),
            Self::Store(e) => e.code(),
            Self::Write(e) => e.cause.code(),
            Self::DurableRead(e) => {
                if primary_pin_refusal(&e.cause) {
                    "operator-package-pin"
                } else {
                    e.cause.code()
                }
            }
            Self::Joined { cause, .. } => cause.code(),
        }
    }
    /// Additional release failure, alongside the primary source(). Existing
    /// config/WriteError sources can themselves contain their own Joined tree.
    pub fn cleanup_error(&self) -> Option<&config::Error> {
        match self {
            Self::Joined { cleanup, .. } => Some(cleanup),
            _ => None,
        }
    }
}
impl fmt::Display for Error {
    /// No nested private diagnostic or pin is formatted.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "operator package: {}", self.code())
    }
}
impl fmt::Debug for Error {
    /// Same redaction as Display, including secondary private IO errors.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Error {
    /// Actual primary cause; consumers must also traverse cleanup_error.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Pin => None,
            Self::Material(e) => Some(e),
            Self::Store(e) => Some(e),
            Self::Write(e) => Some(e),
            Self::DurableRead(e) => Some(e),
            Self::Joined { cause, .. } => Some(cause.as_ref()),
        }
    }
}
/// Failed create keeps the last actual publication acknowledgement. Err never
/// means absent package, and release failure cannot erase Durable/Visible state.
pub struct CommitError {
    pub publication: Publication,
    pub cause: Error,
}
impl fmt::Display for CommitError {
    /// Safe physical phase/category only, never secrets or caller pin.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "operator package create: {:?} ({})",
            self.publication,
            self.cause.code()
        )
    }
}
impl fmt::Debug for CommitError {
    /// Preserve redaction on unwrap_err/assertion failures.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for CommitError {
    /// Typed cause tree, including independent release and nested write cleanup.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.cause)
    }
}
/// Read observations only, never publication durability/freshness/authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadObservation {
    /// Package read not attempted; earlier directory/lock operations may exist.
    NotAttempted,
    /// Read attempted, no complete verified byte buffer returned; may be partial.
    ReadUnknown,
    BytesRead,
    PinMatched,
    Decoded,
}
/// Failed load retains known read stages; decoded keys are dropped on any
/// release failure rather than returned as usable material after that refusal.
pub struct ReadError {
    pub observation: ReadObservation,
    /// Actual local FS acknowledgements, independent from decode/read outcome.
    /// Plain load always leaves this NotAcknowledged; no implicit sync added.
    pub durability: ReadDurability,
    pub cause: Error,
}
impl fmt::Display for ReadError {
    /// Static observation/category only, not private content or pin.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "operator package read: {:?} {:?} ({})",
            self.observation,
            self.durability,
            self.cause.code()
        )
    }
}
impl fmt::Debug for ReadError {
    /// Same safe diagnostic as Display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for ReadError {
    /// Primary typed tree, with Error::cleanup_error as an independent branch.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.cause)
    }
}
/// Retain primary and release errors without discarding known success silently.
/// Successful T is dropped on release failure; caller separately retains phases.
fn finish<T>(result: Result<T, Error>, release: Result<(), config::Error>) -> Result<T, Error> {
    match (result, release) {
        (r, Ok(())) => r,
        (Ok(_), Err(e)) => Err(Error::Store(e)),
        (Err(cause), Err(cleanup)) => Err(Error::Joined {
            cause: Box::new(cause),
            cleanup,
        }),
    }
}
/// Own an already descriptor-bound private Documents store. This is a kernel,
/// not an authenticated endpoint or proof that operator/service roots differ.
pub struct PackageStore {
    documents: Documents,
}
impl PackageStore {
    /// Take existing owned handles without IO, path adoption or approval. Trusted
    /// caller owns opening/protecting a dedicated operator directory; do not hand
    /// Documents clones to untrusted code. Physical service separation is future
    /// installer's responsibility before copying any file/journal material.
    pub fn new(documents: Documents) -> Self {
        Self { documents }
    }
    /// Create only absent singleton bytes with NOREPLACE. Never auto-adopt an
    /// existing file, even with equal bytes. Caller knows pin before the write
    /// and retains Seal across uncertainty. Always finish explicit reservation;
    /// publication phase survives write/unlock errors. No journal/service effect.
    pub fn create(&self, ctx: &Context, seal: &Seal) -> Result<Publication, CommitError> {
        let r = self.documents.reserve(ctx).map_err(|e| CommitError {
            publication: Publication::NotAttempted,
            cause: Error::Store(e),
        })?;
        let result = create_reserved(&r, seal);
        let publication = match &result {
            Ok(p) => *p,
            Err(e) => e.publication,
        };
        finish(result.map_err(|e| e.cause), r.finish())
            .map_err(|cause| CommitError { publication, cause })
    }
    /// Read guarded bytes under reservation and compare the independently given
    /// exact pin before decoding. No entropy, checkpoint/head discovery or reset.
    /// Always finish; any late failure withholds/drops decoded keys. Success is
    /// a read observation only: no implicit file/directory fsync/durability or
    /// freshness/human approval claim. Borrowed/OS/compiler copies not erased.
    pub fn load(&self, ctx: &Context, pin: &Pin) -> Result<Prepared, ReadError> {
        self.load_inner(ctx, pin, false)
    }
    /// Explicit recovery acknowledgement: independently pinned exact bytes are
    /// file+directory synced on held descriptors before decoding/return. This
    /// establishes local FS acknowledgement only, not human authority/freshness
    /// or hardware survival. All known acknowledgements survive decode/finish
    /// errors; any failure withholds/drops keys. Plain load remains observational.
    pub fn load_durable(&self, ctx: &Context, pin: &Pin) -> Result<Prepared, ReadError> {
        self.load_inner(ctx, pin, true)
    }
    /// Mechanical operation selector, not operator policy/default. Share exact
    /// decoding/release; preserve read-validation and durability as distinct facts.
    fn load_inner(&self, ctx: &Context, pin: &Pin, durable: bool) -> Result<Prepared, ReadError> {
        let r = self.documents.reserve(ctx).map_err(|cause| ReadError {
            observation: ReadObservation::NotAttempted,
            durability: ReadDurability::NotAcknowledged,
            cause: Error::Store(cause),
        })?;
        let result = load_reserved(&r, pin, durable);
        let (observation, durability) = match &result {
            Ok(_) => (
                ReadObservation::Decoded,
                if durable {
                    ReadDurability::DirectoryAcknowledged
                } else {
                    ReadDurability::NotAcknowledged
                },
            ),
            Err(e) => (e.observation, e.durability),
        };
        finish(result.map_err(|e| e.cause), r.finish()).map_err(|cause| ReadError {
            observation,
            durability,
            cause,
        })
    }
    /// Consume owner and explicitly close its Documents handles. Busy/poison/
    /// cleanup errors remain typed; an error never resets or deletes the package.
    pub fn close(self) -> Result<(), Error> {
        self.documents.close().map_err(Error::Store)
    }
}

/// Publish exact guarded Seal under the caller's ALREADY HELD reservation.
/// No reservation/finish/close is performed here: caller must retain exclusion,
/// independent pin and Seal across uncertainty, then explicitly release/close.
/// No key/proof copy or generation; known publication and original error survive.
pub(super) fn create_reserved(
    r: &Reservation<'_>,
    seal: &Seal,
) -> Result<Publication, CommitError> {
    r.create(NAME, &seal.raw).map_err(|e| CommitError {
        publication: e.publication,
        cause: Error::Write(e),
    })
}
/// Refuse every existing package before generation, including empty/equal bytes.
/// Guarded read verifies actual metadata and drops private bytes; only authoritative
/// NotConfigured means absence. Does not decode/adopt or release borrowed owner.
pub(super) fn require_absent(r: &Reservation<'_>) -> Result<(), Error> {
    match r.read_private(NAME) {
        Err(config::Error::NotConfigured) => Ok(()),
        Ok(_) => Err(Error::Store(config::Error::Exists)),
        Err(e) => Err(Error::Store(e)),
    }
}
/// Pinned package read on a borrowed existing reservation. Explicit durable is
/// the existing mechanical operation selector, not policy/default. Verify pin
/// before decode/fsync, retain all ack/observation on failure; never generate.
/// Caller must finish/close before letting returned Prepared leave its custody.
pub(super) fn load_reserved(
    r: &Reservation<'_>,
    pin: &Pin,
    durable: bool,
) -> Result<Prepared, ReadError> {
    let mut observation = ReadObservation::ReadUnknown;
    let mut durability = ReadDurability::NotAcknowledged;
    let result = (|| {
        let raw = if durable {
            match r.read_private_durable(NAME, pin.as_str()) {
                Ok(raw) => {
                    durability = ReadDurability::DirectoryAcknowledged;
                    raw
                }
                Err(e) => {
                    observation = match e.validation {
                        ReadValidation::NotAttempted => ReadObservation::NotAttempted,
                        ReadValidation::ReadUnknown => ReadObservation::ReadUnknown,
                        ReadValidation::BytesRead => ReadObservation::BytesRead,
                        ReadValidation::PinMatched => ReadObservation::PinMatched,
                    };
                    durability = e.durability;
                    return Err(Error::DurableRead(e));
                }
            }
        } else {
            r.read_private(NAME).map_err(Error::Store)?
        };
        if !durable {
            observation = ReadObservation::BytesRead;
            let actual = format!("{:x}", Sha256::digest(raw.as_bytes()));
            if !bool::from(actual.as_bytes().ct_eq(pin.as_str().as_bytes())) {
                return Err(Error::Pin);
            }
        }
        observation = ReadObservation::PinMatched;
        let prepared = decode_private(raw.as_bytes()).map_err(Error::Material)?;
        observation = ReadObservation::Decoded;
        Ok(prepared)
    })();
    result.map_err(|cause| ReadError {
        observation,
        durability,
        cause,
    })
}

/// Recognize authoritative pin rejection through primary cleanup wrappers;
/// never classify a secondary failure or arbitrary error text as the main refusal.
fn primary_pin_refusal(e: &config::Error) -> bool {
    match e {
        config::Error::Pins => true,
        config::Error::Joined { cause, .. } => primary_pin_refusal(cause),
        _ => false,
    }
}
