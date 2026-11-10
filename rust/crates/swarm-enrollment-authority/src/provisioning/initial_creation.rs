//! Initial dormant operator package under private immediate reservations.
//! Intent precedes fresh generation; durable independent anchor precedes private
//! package publication while both reservations remain held. Late cleanup retains
//! pinned material and fails; no Root/account/service/current authority/human.
use super::{self as material, checkpoint, repository as package};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error as StdError, fmt, path::Path};
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
use swarm_wire_json as wire;
/// Known local facts only; publication belongs to this operation, not old files.
/// Read fields describe explicit validation/decode phases, not absence-only
/// preflight probes (which may read/reject an existing private file or change
/// atime). NotAttempted never asserts that no filesystem observation happened.
/// Generation attempted/completed means fresh material, never decoding existing
/// seeds. Defaults are unobserved protocol states, not execution policy/grants.
#[derive(Clone, Copy, Debug)]
pub struct Progress {
    pub intent_publication: Publication,
    pub anchor_publication: Publication,
    pub package_publication: Publication,
    pub intent_read: ReadValidation,
    pub intent_durability: ReadDurability,
    pub intent_decoded: bool,
    pub anchor_read: ReadValidation,
    pub anchor_durability: ReadDurability,
    pub anchor_decoded: bool,
    pub package_read: package::ReadObservation,
    pub package_durability: ReadDurability,
    pub material_generation_attempted: bool,
    pub material_generation_completed: bool,
    pub request_pin_matched: bool,
    pub package_request_matched: bool,
}
impl Default for Progress {
    /// No effect/read/ack/material/authority inferred before its observable step.
    fn default() -> Self {
        Self {
            intent_publication: Publication::NotAttempted,
            anchor_publication: Publication::NotAttempted,
            package_publication: Publication::NotAttempted,
            intent_read: ReadValidation::NotAttempted,
            intent_durability: ReadDurability::NotAcknowledged,
            intent_decoded: false,
            anchor_read: ReadValidation::NotAttempted,
            anchor_durability: ReadDurability::NotAcknowledged,
            anchor_decoded: false,
            package_read: package::ReadObservation::NotAttempted,
            package_durability: ReadDurability::NotAcknowledged,
            material_generation_attempted: false,
            material_generation_completed: false,
            request_pin_matched: false,
            package_request_matched: false,
        }
    }
}
/// Metadata only after all custodians finish/close. Private pins are explicit
/// projections through Checkpoint; no key/proof/Root is returned or authorized.
#[derive(Debug)]
pub struct Receipt {
    pub checkpoint: checkpoint::Checkpoint,
    pub progress: Progress,
}
/// Cleanup owner identifies a technical custodian, not an OS account/authority.
#[derive(Clone, Copy, Debug)]
pub enum Owner {
    Operator,
    Anchor,
}
/// Original primary and independent cleanup branches; safe ordinary formatting.
pub enum Cause {
    Path,
    Pin(package::Error),
    PinMismatch,
    Controller,
    Request(material::Error),
    PackageRequestMismatch,
    NotDurable,
    Operator(config::Error),
    Anchor(config::Error),
    IntentWrite(WriteError),
    IntentRead(DurableReadError),
    IntentJson(serde_json::Error),
    IntentDocument,
    IntentBinding,
    Checkpoint(checkpoint::Error),
    CheckpointWrite(checkpoint::CommitError),
    CheckpointRead(checkpoint::ReadError),
    Package(package::Error),
    PackageWrite(package::CommitError),
    PackageRead(package::ReadError),
    Joined {
        cause: Box<Cause>,
        cleanup: config::Error,
        owner: Owner,
    },
}
impl Cause {
    /// Stable safe operation/phase class; never classify by original private text.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Path => "initial-creation-path",
            Self::Pin(_) => "initial-request-pin",
            Self::PinMismatch => "initial-request-pin-mismatch",
            Self::Controller => "initial-controller-uid",
            Self::Request(e) => e.code(),
            Self::PackageRequestMismatch => "initial-package-request",
            Self::NotDurable => "initial-ack-not-durable",
            Self::Operator(e) | Self::Anchor(e) => e.code(),
            Self::IntentWrite(e) => e.cause.code(),
            Self::IntentRead(e) => e.cause.code(),
            Self::IntentJson(_) => "initial-intent-json",
            Self::IntentDocument => "initial-intent-document",
            Self::IntentBinding => "initial-intent-request",
            Self::Checkpoint(e) => e.code(),
            Self::CheckpointWrite(e) => e.cause.code(),
            Self::CheckpointRead(e) => e.cause.code(),
            Self::Package(e) => e.code(),
            Self::PackageWrite(e) => e.cause.code(),
            Self::PackageRead(e) => e.cause.code(),
            Self::Joined { cause, .. } => cause.code(),
        }
    }
}
impl fmt::Display for Cause {
    /// No pin/path/key/proof/private JSON/IO message is formatted here.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for Cause {
    /// Same redaction as Display, including fixture assertions.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Cause {
    /// Primary original cause; error_links exposes independent cleanup siblings.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Pin(e) | Self::Package(e) => Some(e),
            Self::Request(e) => Some(e),
            Self::Operator(e) | Self::Anchor(e) => Some(e),
            Self::IntentWrite(e) => Some(e),
            Self::IntentRead(e) => Some(e),
            Self::IntentJson(e) => Some(e),
            Self::Checkpoint(e) => Some(e),
            Self::CheckpointWrite(e) => Some(e),
            Self::CheckpointRead(e) => Some(e),
            Self::PackageWrite(e) => Some(e),
            Self::PackageRead(e) => Some(e),
            Self::Joined { cause, .. } => Some(cause.as_ref()),
            _ => None,
        }
    }
}
/// Failed operation keeps actual publication/read/material facts; no successful
/// receipt or usable keys leave custody. Error never establishes rollback/absence.
pub struct Failure {
    pub progress: Progress,
    pub cause: Cause,
}
impl fmt::Display for Failure {
    /// Safe technical facts only; Progress contains no pins/private source values.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}; {:?}", self.cause.code(), self.progress)
    }
}
impl fmt::Debug for Failure {
    /// Same private-data redaction as Display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Failure {
    /// Preserve original primary while traversal retains every cleanup sibling.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.cause)
    }
}
/// Immutable per-slot pre-generation marker; no key or authority in this record.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Intent {
    schema_version: i64,
    protocol: String,
    request_pin: String,
}
impl Intent {
    /// Called only with canonical independently supplied byte pin, before material.
    fn new(pin: &str) -> Self {
        Self {
            schema_version: 1,
            protocol: "swarm-initial-provisioning-intent-v1".into(),
            request_pin: pin.into(),
        }
    }
    /// Exact closed JSON, protocol framing only; caller protects selected locator.
    fn encode(&self) -> Result<Vec<u8>, Cause> {
        wire::encode(self).map_err(Cause::IntentJson)
    }
    /// Existing intent must be canonical. Caller records decode before checking
    /// expected request binding; parsing alone gives no regeneration authority.
    fn decode(raw: &[u8]) -> Result<Self, Cause> {
        let value: Self = wire::decode(raw).map_err(Cause::IntentJson)?;
        if value.schema_version != 1
            || value.protocol != "swarm-initial-provisioning-intent-v1"
            || !swarm_identity_contract::digest(value.request_pin.as_bytes())
            || value.encode()?.as_slice() != raw
        {
            return Err(Cause::IntentDocument);
        }
        Ok(value)
    }
}
/// Keep primary plus actual independent cleanup; success is withheld on error.
fn finish<T>(
    result: Result<T, Cause>,
    cleanup: Result<(), config::Error>,
    owner: Owner,
) -> Result<T, Cause> {
    match (result, cleanup) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(match owner {
            Owner::Operator => Cause::Operator(e),
            Owner::Anchor => Cause::Anchor(e),
        }),
        (Err(cause), Err(cleanup)) => Err(Cause::Joined {
            cause: Box::new(cause),
            cleanup,
            owner,
        }),
    }
}
/// Independent canonical byte digest before decode, then existing strict Request
/// validation and actual captured euid match. No IO, fresh entropy or authority.
fn validate(raw: &[u8], pin: &str, uid: u32) -> Result<material::Request, Cause> {
    package::Pin::parse(pin).map_err(Cause::Pin)?;
    let actual = format!("{:x}", Sha256::digest(raw));
    if !bool::from(actual.as_bytes().ct_eq(pin.as_bytes())) {
        return Err(Cause::PinMismatch);
    }
    let request = material::decode_request(raw).map_err(Cause::Request)?;
    if request.controller_uid != uid {
        return Err(Cause::Controller);
    }
    Ok(request)
}
/// Absolute explicit existing targets only; no mkdir, chmod/chown or Root use.
/// Documents owns actual basename/private metadata validation before generation.
fn paths<'a>(operator: &Path, anchor: &'a Path) -> Result<(&'a Path, &'a str), Cause> {
    if !operator.is_absolute() || !anchor.is_absolute() {
        return Err(Cause::Path);
    }
    let parent = anchor.parent().ok_or(Cause::Path)?;
    let name = anchor
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or(Cause::Path)?;
    Ok((parent, name))
}
/// Open current-owned private custodians only; second failure explicitly closes
/// first with both causes. No request-selected UID or newly created directory.
fn open_pair(operator: &Path, anchor: &Path, uid: u32) -> Result<(Documents, Documents), Cause> {
    let op = Documents::open(operator, uid).map_err(Cause::Operator)?;
    match Documents::open(anchor, uid) {
        Ok(a) => Ok((op, a)),
        Err(e) => finish(Err(Cause::Anchor(e)), op.close(), Owner::Operator),
    }
}
/// Intent names derive mechanically from the selected anchor basename, no business
/// retention/execution policy; OS filename errors remain authoritative IO errors.
fn intent_name(anchor: &str) -> String {
    format!("{anchor}.intent")
}
/// Only authoritative missing-file state is absence. Existing empty/invalid data
/// refuse before generation; guarded bytes drop without adoption or formatting.
fn absent(r: &Reservation<'_>, name: &str) -> Result<(), Cause> {
    match r.read_private(name) {
        Err(config::Error::NotConfigured) => Ok(()),
        Ok(_) => Err(Cause::Anchor(config::Error::Exists)),
        Err(e) => Err(Cause::Anchor(e)),
    }
}
/// Observe one exact intent publication, requiring full local ack before material.
/// Temporary filename randomness is distinct from fresh key/proof generation.
fn write_intent(
    r: &Reservation<'_>,
    name: &str,
    pin: &str,
    progress: &mut Progress,
) -> Result<(), Cause> {
    let raw = Intent::new(pin).encode()?;
    match r.create(name, &raw) {
        Ok(p) => {
            progress.intent_publication = p;
            if p != Publication::Durable {
                return Err(Cause::NotDurable);
            }
            Ok(())
        }
        Err(e) => {
            progress.intent_publication = e.publication;
            Err(Cause::IntentWrite(e))
        }
    }
}
/// Trusted locator read/ack of retained intent; no independent digest is inferred.
/// Preserve BytesRead/ack/decode before request-binding failure, never regenerate.
fn read_intent(
    r: &Reservation<'_>,
    name: &str,
    pin: &str,
    progress: &mut Progress,
) -> Result<(), Cause> {
    let raw = match r.read_trusted_private_durable(name) {
        Ok(v) => {
            progress.intent_read = ReadValidation::BytesRead;
            progress.intent_durability = ReadDurability::DirectoryAcknowledged;
            v
        }
        Err(e) => {
            progress.intent_read = e.validation;
            progress.intent_durability = e.durability;
            return Err(Cause::IntentRead(e));
        }
    };
    let intent = Intent::decode(raw.as_bytes())?;
    progress.intent_decoded = true;
    if intent.request_pin != pin {
        return Err(Cause::IntentBinding);
    }
    Ok(())
}
/// Create one dormant package in existing private operator/anchor directories.
/// Caller independently protects expected request pin and anchor locator. Validate
/// strict canonical bytes/euid before opening targets. Both immediate reservations
/// precede absence checks/material; shared physical lock/alias refuses Busy. Write
/// durable immutable intent, prepare once, then durably record the pin anchor.
/// Publish exact Seal under both reservations before releasing/closing its local
/// writer, so late cleanup cannot discard already independently pinned material.
/// No usable receipt leaves until every owner finishes/closes. Always release/
/// close; any failure keeps actual phases and withholds receipt. Never auto-retry,
/// regenerate after existing intent, adopt/reset files, install Root/account/service
/// or establish human/current authority/real UID separation/rollback resistance.
/// Cooperative cancellation follows existing FS guards; no timer/KDF interrupt.
pub fn create(
    raw_request: &[u8],
    request_pin: &str,
    operator_dir: &Path,
    anchor_file: &Path,
    ctx: &Context,
) -> Result<Receipt, Failure> {
    let mut progress = Progress::default();
    let uid = rustix::process::geteuid().as_raw();
    let request =
        validate(raw_request, request_pin, uid).map_err(|cause| Failure { progress, cause })?;
    progress.request_pin_matched = true;
    let (parent, name) =
        paths(operator_dir, anchor_file).map_err(|cause| Failure { progress, cause })?;
    let (op, anchor) =
        open_pair(operator_dir, parent, uid).map_err(|cause| Failure { progress, cause })?;
    create_owned(
        &request,
        raw_request,
        request_pin,
        op,
        anchor,
        name,
        ctx,
        &mut progress,
        &mut |r| material::prepare(r),
    )
    .map(|checkpoint| Receipt {
        checkpoint,
        progress,
    })
    .map_err(|cause| Failure { progress, cause })
}
/// Internal real custody operation; public caller validated request/euid/byte pin.
/// Private synchronous generator seam is only for tests, not user policy/runtime
/// callbacks. Supplied Documents are exclusively owned; no external clones.
fn create_owned(
    request: &material::Request,
    raw_request: &[u8],
    request_pin: &str,
    op: Documents,
    anchor: Documents,
    name: &str,
    ctx: &Context,
    progress: &mut Progress,
    prepare: &mut impl FnMut(&material::Request) -> Result<material::Prepared, material::Error>,
) -> Result<checkpoint::Checkpoint, Cause> {
    let mut anchor_close_attempted = false;
    let result = (|| {
        let op_r = op.reserve(ctx).map_err(Cause::Operator)?;
        let result = (|| {
            let stage = (|| {
                let a_r = anchor.reserve(ctx).map_err(Cause::Anchor)?;
                let stage = (|| {
                    package::require_absent(&op_r).map_err(Cause::Package)?;
                    checkpoint::require_absent(&a_r, name).map_err(Cause::Checkpoint)?;
                    absent(&a_r, &intent_name(name))?;
                    write_intent(&a_r, &intent_name(name), request_pin, progress)?;
                    package::require_absent(&op_r).map_err(Cause::Package)?;
                    progress.material_generation_attempted = true;
                    let prepared = prepare(request).map_err(Cause::Request)?;
                    progress.material_generation_completed = true;
                    if material::encode_request(prepared.request())
                        .map_err(Cause::Request)?
                        .as_slice()
                        != raw_request
                    {
                        return Err(Cause::PackageRequestMismatch);
                    }
                    progress.package_request_matched = true;
                    let seal = package::seal(prepared).map_err(Cause::Package)?;
                    let record = checkpoint::Checkpoint::new(request_pin, seal.pin())
                        .map_err(Cause::Checkpoint)?;
                    match checkpoint::create_reserved(&a_r, name, &record) {
                        Ok(p) => {
                            progress.anchor_publication = p;
                            if p != Publication::Durable {
                                return Err(Cause::NotDurable);
                            }
                        }
                        Err(e) => {
                            progress.anchor_publication = e.publication;
                            return Err(Cause::CheckpointWrite(e));
                        }
                    }
                    let result = match package::create_reserved(&op_r, &seal) {
                        Ok(p) => {
                            progress.package_publication = p;
                            if p != Publication::Durable {
                                Err(Cause::NotDurable)
                            } else {
                                Ok(())
                            }
                        }
                        Err(e) => {
                            progress.package_publication = e.publication;
                            Err(Cause::PackageWrite(e))
                        }
                    };
                    drop(seal);
                    result?;
                    Ok(record)
                })();
                finish(stage, a_r.finish(), Owner::Anchor)
            })();
            anchor_close_attempted = true;
            finish(stage, anchor.close(), Owner::Anchor)
        })();
        finish(result, op_r.finish(), Owner::Operator)
    })();
    let result = if anchor_close_attempted {
        result
    } else {
        finish(result, anchor.close(), Owner::Anchor)
    };
    finish(result, op.close(), Owner::Operator)
}
/// Explicit read-only reconciliation under same private reservations. Check exact
/// request/euid, retained intent and independently located anchor before reading
/// package under its expected pin. Close anchor before package use; compare exact
/// canonical package request and close/drop all keys before usable metadata. No
/// fresh material, file publication, reset, adoption or generation fallback. Orphan
/// intent/anchor or mismatching file refuses and preserves originals/known facts.
pub fn reconcile(
    raw_request: &[u8],
    request_pin: &str,
    operator_dir: &Path,
    anchor_file: &Path,
    ctx: &Context,
) -> Result<Receipt, Failure> {
    let mut progress = Progress::default();
    let uid = rustix::process::geteuid().as_raw();
    validate(raw_request, request_pin, uid).map_err(|cause| Failure { progress, cause })?;
    progress.request_pin_matched = true;
    let (parent, name) =
        paths(operator_dir, anchor_file).map_err(|cause| Failure { progress, cause })?;
    let (op, anchor) =
        open_pair(operator_dir, parent, uid).map_err(|cause| Failure { progress, cause })?;
    let mut anchor_close_attempted = false;
    let result = (|| {
        let op_r = op.reserve(ctx).map_err(Cause::Operator)?;
        let result = (|| {
            let stage = (|| {
                let a_r = anchor.reserve(ctx).map_err(Cause::Anchor)?;
                let stage = (|| {
                    read_intent(&a_r, &intent_name(name), request_pin, &mut progress)?;
                    match checkpoint::load_reserved(&a_r, name, request_pin) {
                        Ok(record) => {
                            progress.anchor_read = ReadValidation::BytesRead;
                            progress.anchor_durability = ReadDurability::DirectoryAcknowledged;
                            progress.anchor_decoded = true;
                            Ok(record)
                        }
                        Err(e) => {
                            progress.anchor_read = e.validation;
                            progress.anchor_durability = e.durability;
                            progress.anchor_decoded = e.decoded;
                            Err(Cause::CheckpointRead(e))
                        }
                    }
                })();
                finish(stage, a_r.finish(), Owner::Anchor)
            })();
            anchor_close_attempted = true;
            let record = finish(stage, anchor.close(), Owner::Anchor)?;
            let pin = package::Pin::parse(record.package_pin()).map_err(Cause::Package)?;
            let prepared = match package::load_reserved(&op_r, &pin, true) {
                Ok(value) => {
                    progress.package_read = package::ReadObservation::Decoded;
                    progress.package_durability = ReadDurability::DirectoryAcknowledged;
                    value
                }
                Err(e) => {
                    progress.package_read = e.observation;
                    progress.package_durability = e.durability;
                    return Err(Cause::PackageRead(e));
                }
            };
            if material::encode_request(prepared.request())
                .map_err(Cause::Request)?
                .as_slice()
                != raw_request
            {
                return Err(Cause::PackageRequestMismatch);
            }
            drop(prepared);
            progress.package_request_matched = true;
            Ok(record)
        })();
        finish(result, op_r.finish(), Owner::Operator)
    })();
    let result = if anchor_close_attempted {
        result
    } else {
        finish(result, anchor.close(), Owner::Anchor)
    };
    let result = finish(result, op.close(), Owner::Operator);
    result
        .map(|checkpoint| Receipt {
            checkpoint,
            progress,
        })
        .map_err(|cause| Failure { progress, cause })
}
/// Extend established checkpoint/package/config/custom-IO traversal with every
/// independent owned cleanup and safe progress. Caller retains source traversal
/// and wide-identity cycle detection; no private pins/source values are labels.
pub fn error_links<'a>(
    e: &'a (dyn StdError + 'static),
) -> (Vec<String>, Vec<&'a (dyn StdError + 'static)>) {
    let (mut labels, mut children) = checkpoint::error_links(e);
    if let Some(e) = e.downcast_ref::<Failure>() {
        labels.push(format!("initial-creation:{}", e.cause.code()));
        labels.push(format!(
            "initial-intent-publication:{:?}",
            e.progress.intent_publication
        ));
        labels.push(format!(
            "initial-anchor-publication:{:?}",
            e.progress.anchor_publication
        ));
        labels.push(format!(
            "initial-package-publication:{:?}",
            e.progress.package_publication
        ));
        labels.push(format!(
            "initial-material-attempted:{}",
            e.progress.material_generation_attempted
        ));
    }
    if let Some(e) = e.downcast_ref::<Cause>() {
        if let Cause::Joined { cleanup, owner, .. } = e {
            labels.push(format!("initial-cleanup-owner:{owner:?}"));
            children.push(cleanup);
        }
    }
    (labels, children)
}
#[cfg(test)]
mod tests;
