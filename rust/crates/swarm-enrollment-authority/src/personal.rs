//! Transactional personal-account bootstrap and held current secret knowledge.
//! Independent protected provisioning, key custody and checkpoints are caller
//! prerequisites. This library starts no service and proves no human presence.
use crate::credentials::{self, Parameters};
/// Durable personal intent/checkpoint coordination; protected startup required.
pub mod coordinator;
mod proposal;
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error as StdError, fmt, path::Path};
use subtle::ConstantTimeEq;
use swarm_enrollment_state::{self as state, sign_change, BoundJournal, Context, Head};
use swarm_identity_contract::{digest, Principal};
use swarm_wire_json as wire;

/// Fixed resource names inside the independently pinned personal-journal domain;
/// not operator execution policy and never a mission-specific exception.
pub const BOOTSTRAP_ID: &str = "personal-bootstrap-v1";
/// Immutable account resource; revoked records cannot be enrolled under this ID again.
pub const ACCOUNT_ID: &str = "personal-account-v1";
/// Expected identity supplied by protected installation, never an actor field.
/// Matching these labels is structural binding only, not human authentication.
#[derive(Clone, PartialEq, Eq)]
pub struct Expected {
    pub installation_digest: String,
    pub principal: Principal,
    pub generation: i64,
}
impl Expected {
    /// Validate canonical context and a positive stable human generation.
    fn validate(&self) -> Result<(), Error> {
        if !digest(self.installation_digest.as_bytes())
            || !self.principal.valid()
            || self.principal.kind != "human"
            || self.generation <= 0
        {
            return Err(Error::Binding);
        }
        Ok(())
    }
}
/// Closed canonical bootstrap data in the signed journal. The commitment must
/// come from a fresh random32byte installation secret delivered independently,
/// outside the agent channel. There is no shared/preinstalled password or first
/// caller takeover. KDF costs are explicit, not chosen from hidden defaults.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bootstrap {
    pub schema_version: i64,
    pub installation_digest: String,
    pub principal: Principal,
    pub generation: i64,
    pub parameters: Parameters,
    pub proof_commitment: String,
}
impl fmt::Debug for Bootstrap {
    /// Never reveal installation verifier/identity data through debug formatting.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PersonalBootstrap { <private> }")
    }
}
/// Refusals preserve original credential/journal/framing causes. Apply errors
/// conservatively leave enrollment publication UNKNOWN until exact replay/readback.
pub enum Error {
    Binding,
    Bootstrap,
    Proposal,
    Secret,
    Internal,
    Credential(credentials::Error),
    Frame(serde_json::Error),
    Encoding(base64::DecodeError),
    State(state::Error),
    Apply(state::Error),
    Joined {
        cause: Box<Error>,
        authority: state::Error,
    },
}
impl Error {
    /// Safe category without private paths, verifier material or typed payloads.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Binding => "personal-binding",
            Self::Bootstrap => "personal-bootstrap",
            Self::Proposal => "personal-proposal",
            Self::Secret => "installation-proof",
            Self::Internal => "personal-state",
            Self::Credential(e) => e.code(),
            Self::Frame(_) => "personal-json",
            Self::Encoding(_) => "personal-encoding",
            Self::State(e) => e.code(),
            Self::Apply(_) => "enrollment-UNKNOWN",
            Self::Joined { cause, .. } => cause.code(),
        }
    }
    /// Independent transaction/rollback failure after an earlier bootstrap refusal.
    /// Consumers must traverse this branch in addition to the primary source.
    pub fn authority_error(&self) -> Option<&state::Error> {
        match self {
            Self::Joined { authority, .. } => Some(authority),
            _ => None,
        }
    }
}
impl fmt::Display for Error {
    /// Redact all private original input; callers retain typed causes separately.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "personal account: {}", self.code())
    }
}
impl fmt::Debug for Error {
    /// Use safe category even when nested inside another Debug diagnostic.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Error {
    /// Primary concrete cause; also inspect authority_error for Joined cleanup.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Credential(e) => Some(e),
            Self::Frame(e) => Some(e),
            Self::Encoding(e) => Some(e),
            Self::State(e) | Self::Apply(e) => Some(e),
            Self::Joined { cause, .. } => Some(cause.as_ref()),
            _ => None,
        }
    }
}
/// Callback result is retained if authority COMMIT/cancellation fails after use.
/// No diagnostic infers that a callback effect was absent or rolled back.
pub enum UseError<T, E> {
    Before(Error),
    AfterUse {
        cause: state::Error,
        outcome: Result<T, E>,
    },
}
impl<T, E> fmt::Display for UseError<T, E> {
    /// Never format callback values/errors, which may contain private payloads.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Before(e) => fmt::Display::fmt(e, f),
            Self::AfterUse { .. } => {
                f.write_str("personal account: authority failed after use; effect UNKNOWN")
            }
        }
    }
}
impl<T, E> fmt::Debug for UseError<T, E> {
    /// Debug shares the redacted Display contract without T/E formatting bounds.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl<T, E> StdError for UseError<T, E> {
    /// Authority/initial refusal cause; AfterUse.outcome is a separate mandatory
    /// consumer branch, retaining both successful and failed business outcomes.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Before(e) => Some(e),
            Self::AfterUse { cause, .. } => Some(cause),
        }
    }
}
/// Hash a protocol-sized random installation secret with a domain and exact
/// canonical installation. It never generates/delivers that secret or authenticates
/// its source; caller must protect both delivery and the independently pinned root.
pub fn commitment(installation: &str, secret: &[u8; 32]) -> Result<String, Error> {
    if !digest(installation.as_bytes()) {
        return Err(Error::Binding);
    }
    let mut h = Sha256::new();
    h.update(b"swarm-personal-bootstrap-v1\0");
    h.update(installation.as_bytes());
    h.update(secret);
    Ok(format!("{:x}", h.finalize()))
}
/// Encode strict bootstrap data only. No journal/account creation or bootstrap
/// consumption; invalid context, commitment or explicit KDF parameters refuse.
pub fn encode_bootstrap(b: &Bootstrap) -> Result<Vec<u8>, Error> {
    Expected {
        installation_digest: b.installation_digest.clone(),
        principal: b.principal.clone(),
        generation: b.generation,
    }
    .validate()?;
    if b.schema_version != 1 || !digest(b.proof_commitment.as_bytes()) {
        return Err(Error::Bootstrap);
    }
    b.parameters.validate().map_err(Error::Credential)?;
    wire::encode(b).map_err(Error::Frame)
}
/// Require exact closed canonical JSON. No implicit versions/defaults, unknown
/// fields, ambiguous keys, null coercion or automatic bootstrap reinitialization.
pub fn decode_bootstrap(raw: &[u8]) -> Result<Bootstrap, Error> {
    let b: Bootstrap = wire::decode(raw).map_err(Error::Frame)?;
    if encode_bootstrap(&b)? != raw {
        return Err(Error::Bootstrap);
    }
    Ok(b)
}
/// Privately constructed exact signed two-event proposal after installation
/// secret verification. Retain it for uncertain exact replay; it contains verifier
/// material, not plaintext passwords, and must stay in protected service storage.
/// No public constructor, grant or process authority is provided. Use Source's
/// exact verified codec for protected persistence; ordinary serde is unavailable.
pub struct Prepared {
    base: Head,
    next: Head,
    record_digest: String,
    expected: Expected,
    key: [u8; 32],
    project: [u8; 32],
    raws: [Vec<u8>; 2],
}
impl fmt::Debug for Prepared {
    /// Keep signed verifier bodies and expected human data out of diagnostics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PersonalEnrollmentProposal { <private> }")
    }
}
impl Prepared {
    /// Independently selected starting checkpoint, preserved for exact replay.
    pub fn base_head(&self) -> &Head {
        &self.base
    }
    /// Predicted terminal checkpoint; this getter never proves storage commit.
    pub fn next_head(&self) -> &Head {
        &self.next
    }
    /// Proposed credential digest; consumers must retain/approve it with the
    /// actual committed checkpoint rather than infer publication from this value.
    pub fn credential_digest(&self) -> &str {
        &self.record_digest
    }
}
/// Personal signed journal on one retained existing connection. Keep its storage,
/// keys and checkpoints inaccessible to agents. This API does not provision an
/// OS account or validate path ancestors; protected startup must do that first.
/// Use a journal distinct from business grants; lock order is personal → grants
/// → configuration. No callback may reenter this source or wait on its writers.
pub struct Source {
    journal: BoundJournal,
    expected: Expected,
    key: [u8; 32],
    project: [u8; 32],
}
impl Source {
    /// Open an existing signed WAL source with independent public pins/context.
    /// Never initializes a missing journal, chooses a principal from a request,
    /// or relearns a checkpoint. The retained connection pins the opened object;
    /// it is not OS confinement or proof of trusted bootstrap delivery.
    pub fn open(
        ctx: &Context,
        path: impl AsRef<Path>,
        key: &[u8; 32],
        project: [u8; 32],
        expected: Expected,
    ) -> Result<Self, Error> {
        expected.validate()?;
        let journal = BoundJournal::open(ctx, path, key, project).map_err(Error::State)?;
        Ok(Self {
            journal,
            expected,
            key: *key,
            project,
        })
    }
    /// Check the real active bootstrap at the independent expected head and
    /// verify the installation secret before password work/signing. Preparation
    /// derives private verifier data only; it consumes nothing and grants nothing.
    /// Exact signed revoke+enroll bytes allow replay without generating a new salt.
    /// The signing key must be the protected source's own key. Caller owns/wipes
    /// input secrets; actual human delivery, KDF policy/settings and protected
    /// durable proposal/checkpoint storage remain caller obligations.
    pub fn prepare(
        &self,
        ctx: &Context,
        head: &Head,
        key: &SigningKey,
        secret: &[u8; 32],
        password: &[u8],
    ) -> Result<Prepared, Error> {
        if key.verifying_key().to_bytes() != self.key {
            return Err(Error::Binding);
        }
        let mut outcome = None;
        let observed = self.journal.with_active(ctx, head, BOOTSTRAP_ID, |r| {
            outcome = Some((|| {
                let b = decode_bootstrap(&r.body)?;
                if b.installation_digest != self.expected.installation_digest
                    || b.principal != self.expected.principal
                    || b.generation != self.expected.generation
                {
                    return Err(Error::Binding);
                }
                let actual = commitment(&b.installation_digest, secret)?;
                if !bool::from(actual.as_bytes().ct_eq(b.proof_commitment.as_bytes())) {
                    return Err(Error::Secret);
                }
                let record = credentials::create(
                    &b.installation_digest,
                    &b.principal,
                    b.generation,
                    b.parameters,
                    password,
                )
                .map_err(Error::Credential)?;
                let body = credentials::encode(&record).map_err(Error::Credential)?;
                let record_digest = credentials::fingerprint(&record).map_err(Error::Credential)?;
                let first = sign_change(key, self.project, head, "revoke", BOOTSTRAP_ID, b"")
                    .map_err(Error::State)?;
                let middle = Head {
                    sequence: head.sequence.checked_add(1).ok_or(Error::Binding)?,
                    digest: format!("{:x}", Sha256::digest(&first)),
                };
                let second = sign_change(key, self.project, &middle, "enroll", ACCOUNT_ID, &body)
                    .map_err(Error::State)?;
                let next = Head {
                    sequence: middle.sequence.checked_add(1).ok_or(Error::Binding)?,
                    digest: format!("{:x}", Sha256::digest(&second)),
                };
                Ok(Prepared {
                    base: head.clone(),
                    next,
                    record_digest,
                    expected: self.expected.clone(),
                    key: self.key,
                    project: self.project,
                    raws: [first, second],
                })
            })());
            Ok(())
        });
        match observed {
            Ok(()) => outcome.ok_or(Error::Internal)?,
            Err(authority) => match outcome {
                Some(Err(cause)) => Err(Error::Joined {
                    cause: Box::new(cause),
                    authority,
                }),
                _ => Err(Error::State(authority)),
            },
        }
    }
    /// Consume bootstrap and create the credential in one signed atomic batch.
    /// Another proposal/revocation winning CAS refuses; exact terminal replay
    /// never duplicates events. No automatic head adoption or re-enrollment.
    /// An Apply error leaves publication UNKNOWN until authorized exact replay/
    /// readback; predicted next_head is not a witness or acknowledgement.
    pub fn enroll(&self, ctx: &Context, p: &Prepared) -> Result<Head, Error> {
        if p.expected != self.expected || p.key != self.key || p.project != self.project {
            return Err(Error::Binding);
        }
        self.journal
            .apply_batch(ctx, &p.base, &[&p.raws[0], &p.raws[1]])
            .map_err(Error::Apply)
    }
    /// Check signed active account/context and independent credential pin, verify
    /// its password, then hold the same journal reservation through callback and
    /// authority completion. No session/caller identity or grant is inferred.
    /// The opaque borrow cannot escape. Business errors remain Result::Err inside
    /// a successful authority outcome; authority failure after callback retains
    /// that entire outcome as AfterUse with effect uncertainty. No automatic
    /// retry or external rollback. Callbacks must obey the documented lock order.
    pub fn with_authenticated<T, E>(
        &self,
        ctx: &Context,
        head: &Head,
        record_digest: &str,
        password: &[u8],
        f: impl for<'r> FnOnce(&credentials::Proof<'r>) -> Result<T, E>,
    ) -> Result<Result<T, E>, UseError<T, E>> {
        let mut outcome = None;
        let mut refusal = None;
        let mut callback = Some(f);
        let result = self.journal.with_active(ctx, head, ACCOUNT_ID, |r| {
            let checked = (|| {
                let record = credentials::decode(&r.body).map_err(Error::Credential)?;
                let proof = credentials::verify(
                    &self.expected.installation_digest,
                    &self.expected.principal,
                    self.expected.generation,
                    record_digest,
                    &record,
                    password,
                )
                .map_err(Error::Credential)?;
                outcome = Some(callback.take().ok_or(Error::Internal)?(&proof));
                Ok(())
            })();
            refusal = checked.err();
            Ok(())
        });
        match (result, outcome, refusal) {
            (Ok(()), Some(outcome), None) => Ok(outcome),
            (Err(cause), Some(outcome), _) => Err(UseError::AfterUse { cause, outcome }),
            (Ok(()), None, Some(cause)) => Err(UseError::Before(cause)),
            (Err(authority), None, Some(cause)) => Err(UseError::Before(Error::Joined {
                cause: Box::new(cause),
                authority,
            })),
            (Err(cause), None, None) => Err(UseError::Before(Error::State(cause))),
            _ => Err(UseError::Before(Error::Internal)),
        }
    }
    /// Explicitly close the retained source; contention refuses without closing,
    /// subsequent calls refuse Closed. No bootstrap or history reset on close.
    pub fn close(&self) -> Result<(), Error> {
        self.journal.close().map_err(Error::State)
    }
}
