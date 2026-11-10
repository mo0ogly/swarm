//! Exact PAEA service permissions, independent of historical Trust commands.
//! Requests are data; only current held account/business authority yields Identity.
/// Independent action and authority outcomes, identical retention to Trust.
pub use super::trust_action::Use;
use super::{Control, Error};
use crate::{credentials, Document};
use swarm_enrollment_state::{Context, Head};
use swarm_identity_contract::{digest, Permission, Principal, Scope};
use time::OffsetDateTime;

/// Closed effect service vocabulary. Neither variant selects a path or endpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Service {
    FictitiousPayment,
    GitPromotion,
}
impl Service {
    /// Stable protocol spelling, not an operator-selectable execution policy.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FictitiousPayment => "payment.settle",
            Self::GitPromotion => "repository.promote",
        }
    }
}
/// Separate permissions for first commitment and observation/reconciliation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Commit,
    Reconcile,
}
impl Operation {
    /// Stable protocol spelling; reconciliation never grants a new commitment.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Commit => "commit",
            Self::Reconcile => "reconcile",
        }
    }
}
/// Exact data for a separately signed permission. No permission is issued here.
/// The trusted consumer must derive root from a retained datastore witness and
/// action from canonical PAEA bytes, then compare both again under its own locks.
#[derive(Clone, Debug)]
pub struct Request {
    root: String,
    service: Service,
    operation: Operation,
    action: String,
}
impl Request {
    /// Check canonical SHA256 representations without IO or admission. Reject
    /// malformed digests; never normalize them or infer project/OS protection.
    pub fn new(
        root: &str,
        service: Service,
        operation: Operation,
        action: &str,
    ) -> Result<Self, Error> {
        if !digest(root.as_bytes()) || !digest(action.as_bytes()) {
            return Err(Error::Binding);
        }
        Ok(Self {
            root: root.into(),
            service,
            operation,
            action: action.into(),
        })
    }
    /// Exact effect.execute permission in the paea-action-v1 namespace. A Trust
    /// grant, another service/operation, wildcard or nearby digest cannot match.
    pub fn permission(&self) -> Permission {
        Permission {
            operation: "effect.execute".into(),
            resource: format!(
                "paea-action-v1/{}/{}/{}/{}",
                self.root,
                self.service.as_str(),
                self.operation.as_str(),
                self.action
            ),
        }
    }
    /// Storage witness digest, requiring comparison to the consumer's held root.
    pub fn root_digest(&self) -> &str {
        &self.root
    }
    /// Canonical PAEA action digest; not a caller credential or transport key.
    pub fn action_digest(&self) -> &str {
        &self.action
    }
    /// Service bound by the exact signed permission, not a route to execute.
    pub fn service(&self) -> Service {
        self.service
    }
    /// Operation bound by the grant; Commit and Reconcile are never substitutes.
    pub fn operation(&self) -> Operation {
        self.operation
    }
}
/// Borrowed capability available only while current personal and business
/// reservations are held. Not constructible, serializable or cloneable by callers.
/// Cloned Principal data alone is not this capability or a later-use permission.
/// JSON can never manufacture this held capability:
/// ```compile_fail
/// use swarm_enrollment_authority::paea_action::Identity;
/// let _: Identity<'_> = serde_json::from_str("{}").unwrap();
/// ```
/// Trust and PAEA capabilities cannot substitute each other:
/// ```compile_fail
/// use swarm_enrollment_authority::{paea_action, trust_action};
/// fn use_trust(_: &trust_action::Identity<'_>) {}
/// fn wrong_namespace(held: &paea_action::Identity<'_>) { use_trust(held); }
/// ```
/// The borrowed capability cannot escape its held callback:
/// ```compile_fail
/// use swarm_enrollment_authority::{authenticated_control::{AuthenticatedControl, Context}, paea_action::Request};
/// use swarm_enrollment_state::Head;
/// fn escape(app: &AuthenticatedControl, ctx: &Context, head: &Head, request: &Request) {
///     let _ = app.with_paea_action(ctx, head, b"not-a-credential", request, |held| Ok::<_, ()>(held));
/// }
/// ```
pub struct Identity<'a> {
    document: &'a Document,
    request: &'a Request,
    head: &'a Head,
}
impl Identity<'_> {
    /// Actual held journal head, for evidence linkage, never a standalone permit.
    pub fn authority_head(&self) -> &Head {
        self.head
    }
    /// Exact current enrollment record selected by protected startup.
    pub fn enrollment_id(&self) -> &str {
        &self.document.enrollment_id
    }
    /// Signed validity boundary checked before and after callback use.
    pub fn expires_at(&self) -> OffsetDateTime {
        self.document.expires_at
    }
    /// Actual principal from the current account and signed matching record.
    pub fn principal(&self) -> &Principal {
        &self.document.principal
    }
    /// Grant's exact plan scope; a consumer must validate its own current plan.
    pub fn scope(&self) -> &Scope {
        &self.document.scope
    }
    /// Signed Trust revision; the consumer must recheck its effective revision.
    pub fn trust_revision(&self) -> i64 {
        self.document.trust_revision
    }
    /// Exact PAEA action binding, still requiring the consumer's own object comparison.
    pub fn request(&self) -> &Request {
        self.request
    }
    /// Signed project context, not an operating-system isolation claim.
    pub fn project_digest(&self) -> &str {
        &self.document.project_digest
    }
}

impl Control {
    /// Current account proof and business reservation span the callback. PAEA
    /// identity is minted only here; the consumer still validates its storage,
    /// canonical action, current plan/revision, policy and actual effect service.
    pub(crate) fn paea_action<T, E>(
        &self,
        proof: &credentials::Proof<'_>,
        ctx: &Context,
        head: &Head,
        request: &Request,
        f: impl for<'a> FnOnce(&Identity<'a>) -> Result<T, E>,
    ) -> Use<T, E> {
        self.exact_action(proof, ctx, head, &request.permission(), |document| {
            f(&Identity {
                document,
                request,
                head,
            })
        })
    }
}
