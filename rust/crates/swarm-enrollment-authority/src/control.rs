//! Local production library core, without listener, session provider or OS
//! sandbox. Only independently protected startup may select the authority pins
//! and human binding. Matching caller data, an UID or a cockpit cookie does not
//! establish that prerequisite. No raw recover/reset operation is exposed.
/// Exact human-authorized Trust action callback, distinct from configuration.
pub mod trust_action;
mod held_action;
/// Exact PAEA effect capability; separate from Trust command permissions.
pub mod paea_action;
use crate::{decode, Error as DocumentError};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use std::{
    fmt,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard, TryLockError},
};
use swarm_enrollment_config::{
    self as config,
    store::{self, Coordinator},
    Change, Pins,
};
use swarm_enrollment_state::{self as state, BoundJournal, Head};
use swarm_identity_contract::{digest, text, Permission, Principal, Scope};
use time::OffsetDateTime;

/// Expected human enrollment selected by protected startup, never by the
/// operation's request. Structural validity does not authenticate a human.
#[derive(Clone, Debug)]
pub struct Binding {
    pub enrollment_id: String,
    pub generation: i64,
    pub principal: Principal,
    pub scope: Scope,
    pub trust_revision: i64,
}
/// Independently provisioned service inputs. The journal must already exist;
/// configuration and witness directories must satisfy Coordinator protection.
/// This type neither provisions accounts nor authenticates its producer.
pub struct Startup {
    pub journal: PathBuf,
    pub authority_key: [u8; 32],
    pub project: [u8; 32],
    pub configuration_directory: PathBuf,
    pub witness_directory: PathBuf,
    pub owner: u32,
    pub pins: Pins,
    pub binding: Binding,
}
/// Closed operation set. Changes carry only target configuration, event ID and
/// expected configuration head; there is no actor, grant or authority override.
#[derive(Clone, Debug)]
pub enum Request {
    Read,
    Preview(Change),
    Create(Change),
    Apply(Change),
}
impl Request {
    /// Protocol permission discriminator, not a configurable execution policy.
    pub fn operation(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Preview(_) => "preview",
            Self::Create(_) => "create",
            Self::Apply(_) => "apply",
        }
    }
    fn change(&self) -> Option<&Change> {
        match self {
            Self::Read => None,
            Self::Preview(c) | Self::Create(c) | Self::Apply(c) => Some(c),
        }
    }
    fn mutating(&self) -> bool {
        matches!(self, Self::Create(_) | Self::Apply(_))
    }
}
/// Exact existing effect.execute permission for one installation and operation.
/// Grants are persisted in the signed enrollment, not inferred from defaults.
/// Invalid installation digests refuse before constructing a resource.
pub fn permission(installation: &str, request: &Request) -> Result<Permission, Error> {
    if !digest(installation.as_bytes()) {
        return Err(Error::Binding);
    }
    Ok(Permission {
        operation: "effect.execute".into(),
        resource: format!("enrollment-config/{installation}/{}", request.operation()),
    })
}
#[derive(Debug)]
pub enum Response {
    View(config::View),
    Preview(config::Projection),
}
/// A storage error after a mutating invocation conservatively leaves its effect
/// unknown. Authority failure after storage retains that storage result, including
/// any cleanup cause. An Err never asserts that no configuration was published.
pub enum Error {
    Binding,
    PersonalBinding,
    Permission,
    Busy,
    State,
    Document(DocumentError),
    Authority(state::Error),
    Storage {
        cause: config::Error,
        effect_unknown: bool,
    },
    AuthorityRefusal {
        cause: state::Error,
        refusal: Box<Error>,
    },
    AfterStorage {
        cause: state::Error,
        storage: Box<Result<Response, config::Error>>,
        effect_unknown: bool,
    },
    Close {
        authority: Option<state::Error>,
        storage: Option<config::Error>,
    },
}
impl Error {
    /// Whether configuration or its witness may have changed despite this error.
    /// False is not a global claim about other calls or processes.
    pub fn effect_unknown(&self) -> bool {
        matches!(
            self,
            Self::Storage {
                effect_unknown: true,
                ..
            } | Self::AfterStorage {
                effect_unknown: true,
                ..
            }
        )
    }
    /// Stable, safe diagnostic category. Inspect typed causes for local recovery;
    /// Display does not print private filesystem paths or signed payload bodies.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Binding => "binding",
            Self::PersonalBinding => "personal-binding",
            Self::Permission => "permission",
            Self::Busy => "busy",
            Self::State => "state",
            Self::Document(e) => e.code(),
            Self::Authority(e) => e.code(),
            Self::Storage { cause, .. } => cause.code(),
            Self::AfterStorage { .. } => "authority-after-storage",
            Self::AuthorityRefusal { .. } => "authority-with-refusal",
            Self::Close { .. } => "close",
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "enrollment control: {}; configuration effect {}",
            self.code(),
            if self.effect_unknown() {
                "UNKNOWN"
            } else {
                "not established by this diagnostic"
            }
        )
    }
}
impl fmt::Debug for Error {
    /// Same safe category/effect observation as Display. Private retained Response,
    /// storage configuration, document inputs and nested payloads are not formatted;
    /// all original structured variants and causes remain available to trusted code.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Document(e) => Some(e),
            Self::Authority(e) => Some(e),
            Self::Storage { cause, .. } => Some(cause),
            Self::AfterStorage { cause, .. } | Self::AuthorityRefusal { cause, .. } => Some(cause),
            Self::Close {
                authority: Some(e), ..
            } => Some(e),
            Self::Close {
                storage: Some(e), ..
            } => Some(e),
            _ => None,
        }
    }
}
struct Permit {
    change: Change,
    principal: Principal,
}
/// Private permit lifetime cannot escape a held authority operation. Poisoned
/// mutexes remain poisoned (future access refuses); cleanup still removes data
/// during unwinding. No callback or permit is exposed to the request producer.
struct PermitGuard(Arc<Mutex<Option<Permit>>>);
impl Drop for PermitGuard {
    fn drop(&mut self) {
        match self.0.lock() {
            Ok(mut p) => {
                p.take();
            }
            Err(poisoned) => {
                poisoned.into_inner().take();
            }
        }
    }
}
/// Composes current signed enrollment and two-directory durable configuration.
/// Its mutex is fail-fast; no implicit queue, timeout or retry is introduced.
/// It is not yet a transport-authenticated service or an engine action permit.
pub struct Control {
    authority: BoundJournal,
    coordinator: Coordinator,
    binding: Binding,
    project: String,
    installation: String,
    permit: Arc<Mutex<Option<Permit>>>,
    operation: Mutex<()>,
}
impl Control {
    /// Validate trusted startup consistency, open existing authority, then open
    /// protected config/witness storage. No human/session lookup is performed.
    /// An open failure explicitly closes authority and retains a close failure.
    pub fn open(ctx: &state::Context, startup: Startup) -> Result<Self, Error> {
        let b = &startup.binding;
        if !text(b.enrollment_id.as_bytes())
            || b.generation <= 0
            || b.trust_revision <= 0
            || !b.principal.valid()
            || b.principal.kind != "human"
            || !b.scope.valid()
            || startup.pins.principal != b.principal
            || startup.pins.public_key != URL_SAFE_NO_PAD.encode(startup.authority_key)
        {
            return Err(Error::Binding);
        }
        startup.pins.validate().map_err(|cause| Error::Storage {
            cause,
            effect_unknown: false,
        })?;
        let authority = BoundJournal::open(
            ctx,
            &startup.journal,
            &startup.authority_key,
            startup.project,
        )
        .map_err(Error::Authority)?;
        let permit: Arc<Mutex<Option<Permit>>> = Arc::new(Mutex::new(None));
        let callback = permit.clone();
        let authorize: store::Authorizer = Arc::new(move |_, _, change| {
            let p = callback
                .try_lock()
                .map_err(|_| config::Error::Unauthorized)?;
            let p = p.as_ref().ok_or(config::Error::Unauthorized)?;
            if p.change != change {
                return Err(config::Error::Unauthorized);
            }
            Ok(p.principal.clone())
        });
        let coordinator = match Coordinator::open(
            &startup.configuration_directory,
            &startup.witness_directory,
            startup.owner,
            startup.pins.clone(),
            Some(authorize),
        ) {
            Ok(c) => c,
            Err(cause) => {
                return match authority.close() {
                    Ok(()) => Err(Error::Storage {
                        cause,
                        effect_unknown: false,
                    }),
                    Err(e) => Err(Error::Close {
                        authority: Some(e),
                        storage: Some(cause),
                    }),
                }
            }
        };
        Ok(Self {
            authority,
            coordinator,
            binding: startup.binding,
            project: startup.project.iter().map(|b| format!("{b:02x}")).collect(),
            installation: startup.pins.installation_digest,
            permit,
            operation: Mutex::new(()),
        })
    }
    /// Acquire operation ownership without waiting or hiding poisoning.
    fn lock(&self) -> Result<MutexGuard<'_, ()>, Error> {
        self.operation.try_lock().map_err(|e| match e {
            TryLockError::WouldBlock => Error::Busy,
            TryLockError::Poisoned(_) => Error::State,
        })
    }
    /// Trusted crate composition only: caller must hold its configured protected
    /// current account through this call and completion. Credential knowledge by
    /// itself is not such authority. Bind exact principal and installation before
    /// any business/storage operation; project/rights remain the signed grant's
    /// responsibility. This helper is not a public proof/actor admission endpoint.
    pub(crate) fn execute_from_current_personal(
        &self,
        proof: &crate::credentials::Proof<'_>,
        authority_ctx: &state::Context,
        storage_ctx: &store::Context,
        expected_authority_head: &Head,
        request: Request,
    ) -> Result<Response, Error> {
        if proof.principal() != &self.binding.principal
            || proof.installation_digest() != self.installation
        {
            return Err(Error::PersonalBinding);
        }
        self.execute(authority_ctx, storage_ctx, expected_authority_head, request)
    }
    /// Recheck actual signed current state and exact permission at service time,
    /// holding its reservation through storage return and authority COMMIT.
    /// expected_authority_head is a CAS expectation, never a grant. Cancellation
    /// tokens are caller-controlled, with no implicit timer. A mutation's error
    /// is UNKNOWN until authorized readback/exact-event recovery establishes it.
    /// Expiry is checked before storage, not atomically with filesystem writes.
    pub fn execute(
        &self,
        authority_ctx: &state::Context,
        storage_ctx: &store::Context,
        expected_authority_head: &Head,
        request: Request,
    ) -> Result<Response, Error> {
        let _operation = self.lock()?;
        let required = permission(&self.installation, &request)?;
        let mut storage = None;
        let mut refusal = None;
        let result = self.authority.with_active(
            authority_ctx,
            expected_authority_head,
            &self.binding.enrollment_id,
            |record| {
                let checked = (|| {
                    let d = validate_record(&record, &self.binding, &self.project)?;
                    let at = OffsetDateTime::now_utc();
                    validate_time(&d, at)?;
                    if !d.permissions.contains(&required) {
                        return Err(Error::Permission);
                    }
                    let _permit = PermitGuard(self.permit.clone());
                    if let Some(c) = request.change() {
                        *self.permit.try_lock().map_err(|_| Error::State)? = Some(Permit {
                            change: c.clone(),
                            principal: d.principal,
                        });
                    }
                    storage = Some(match &request {
                        Request::Read => self.coordinator.read(storage_ctx).map(Response::View),
                        Request::Preview(c) => self
                            .coordinator
                            .preview(storage_ctx, c, at)
                            .map(Response::Preview),
                        Request::Create(c) => self
                            .coordinator
                            .create(storage_ctx, c, at)
                            .map(Response::View),
                        Request::Apply(c) => self
                            .coordinator
                            .apply(storage_ctx, c, at)
                            .map(Response::View),
                    });
                    Ok(())
                })();
                // Retain the business refusal outside the journal, without erasing
                // a later transaction/rollback failure from the authority layer.
                refusal = checked.err();
                Ok(())
            },
        );
        match (result, storage, refusal) {
            (Err(cause), Some(storage), _) => Err(Error::AfterStorage {
                cause,
                storage: Box::new(storage),
                effect_unknown: request.mutating(),
            }),
            (Err(cause), None, Some(refusal)) => Err(Error::AuthorityRefusal {
                cause,
                refusal: Box::new(refusal),
            }),
            (Err(e), None, None) => Err(Error::Authority(e)),
            (Ok(()), _, Some(e)) => Err(e),
            (Ok(()), Some(Ok(response)), None) => Ok(response),
            (Ok(()), Some(Err(cause)), None) => Err(Error::Storage {
                cause,
                effect_unknown: request.mutating(),
            }),
            (Ok(()), None, None) => Err(Error::State),
        }
    }
    /// Close authority and both stores after obtaining exclusive operation
    /// ownership. Both cleanup causes are retained; Busy makes no close attempt.
    /// Drop alone does not establish successful durable cleanup.
    pub fn close(&self) -> Result<(), Error> {
        let _operation = self.lock()?;
        let authority = self.authority.close().err();
        let storage = self.coordinator.close().err();
        if authority.is_some() || storage.is_some() {
            Err(Error::Close { authority, storage })
        } else {
            Ok(())
        }
    }
}
/// Validate canonical signed-record payload against independently selected
/// enrollment/context. This checks data only; callers must hold current signed
/// journal authority, then check time and the exact operation before any effect.
pub(crate) fn validate_record(
    record: &state::Record,
    binding: &Binding,
    project: &str,
) -> Result<crate::Document, Error> {
    let d = decode(&record.body).map_err(Error::Document)?;
    if record.id != binding.enrollment_id
        || record.enrolled_sequence != binding.generation
        || d.enrollment_id != record.id
        || d.generation != record.enrolled_sequence
        || d.project_digest != project
        || d.principal != binding.principal
        || d.scope != binding.scope
        || d.trust_revision != binding.trust_revision
    {
        return Err(Error::Binding);
    }
    Ok(d)
}
/// Check service-observed validity without issuing a grant or changing storage.
/// Expiry is a pre-use check, not atomic with a later filesystem/network effect.
pub(crate) fn validate_time(d: &crate::Document, at: OffsetDateTime) -> Result<(), Error> {
    if swarm_wire_json::timestamp::zero(at) || at < d.not_before || at >= d.expires_at {
        return Err(Error::Document(DocumentError::Inactive));
    }
    Ok(())
}
#[cfg(test)]
mod tests;
