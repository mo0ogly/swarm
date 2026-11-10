//! Current human authority for exact Trust actions, not enrollment configuration.
//! Protected startup remains mandatory; request data is never an identity source.
use super::{Control, Error};
use crate::{credentials, Document};
use swarm_enrollment_state::{Context, Head};
use swarm_identity_contract::{digest, Permission, Principal, Scope};
use time::OffsetDateTime;

/// Closed Trust operation vocabulary. These names bind signed permissions;
/// they neither grant a role nor select a process, provider or network endpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    MandateCreate,
    MandateRevoke,
    Request,
    Approve,
    Commit,
    Reconcile,
    ConfigPreview,
    ConfigApply,
    Audit,
    KeyPrepare,
    KeyActivate,
    KeyRevoke,
    PublicAnchor,
}
impl Operation {
    /// Decode only the signed Trust vocabulary. Unknown aliases, whitespace and
    /// read-only legacy operations refuse; parsing grants no authority or role.
    pub fn parse(value: &str) -> Result<Self, Error> {
        [Self::MandateCreate, Self::MandateRevoke, Self::Request, Self::Approve,
         Self::Commit, Self::Reconcile, Self::ConfigPreview, Self::ConfigApply,
         Self::Audit, Self::KeyPrepare, Self::KeyActivate, Self::KeyRevoke,
         Self::PublicAnchor].into_iter().find(|op| op.as_str() == value)
            .ok_or(Error::Binding)
    }
    /// Canonical operation name, identical to the existing Trust protocol.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MandateCreate => "mandate-create",
            Self::MandateRevoke => "mandate-revoke",
            Self::Request => "request",
            Self::Approve => "approve",
            Self::Commit => "commit",
            Self::Reconcile => "reconcile",
            Self::ConfigPreview => "config-preview",
            Self::ConfigApply => "config-apply",
            Self::Audit => "audit-transaction",
            Self::KeyPrepare => "evidence-key-prepare",
            Self::KeyActivate => "evidence-key-activate",
            Self::KeyRevoke => "evidence-key-revoke",
            Self::PublicAnchor => "public-anchor",
        }
    }
}
/// Exact action data. Only the retained authority callback below produces an
/// Identity. The consumer must derive root/request digests from its own objects.
#[derive(Clone, Debug)]
pub struct Request {
    root: String,
    operation: Operation,
    request: String,
}
impl Request {
    /// Validate SHA256 representations without reading a Root or granting rights.
    pub fn new(root: &str, operation: Operation, request: &str) -> Result<Self, Error> {
        if !digest(root.as_bytes()) || !digest(request.as_bytes()) {
            return Err(Error::Binding);
        }
        Ok(Self {
            root: root.into(),
            operation,
            request: request.into(),
        })
    }
    /// Exact signed permission; no wildcard or enrollment-config permission matches.
    pub fn permission(&self) -> Permission {
        Permission {
            operation: "effect.execute".into(),
            resource: format!(
                "trust-action/{}/{}/{}",
                self.root,
                self.operation.as_str(),
                self.request
            ),
        }
    }
    /// Root witness data; consumers must compare with their retained datastore.
    pub fn root_digest(&self) -> &str {
        &self.root
    }
    /// Exact request bytes fingerprint, not a caller-selected canonical action.
    pub fn request_digest(&self) -> &str {
        &self.request
    }
    /// Closed operation bound by the signed permission.
    pub fn operation(&self) -> Operation {
        self.operation
    }
}
/// Borrowed capability available only while current personal and business
/// reservations are held. Not constructible, serializable or cloneable by callers.
/// Cloned Principal data alone is not this capability or a later-use permission.
/// JSON can never manufacture this held capability:
/// ```compile_fail
/// use swarm_enrollment_authority::trust_action::Identity;
/// let _: Identity<'_> = serde_json::from_str("{}").unwrap();
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
    /// Exact action binding, still requiring the consumer's own object comparison.
    pub fn request(&self) -> &Request {
        self.request
    }
    /// Signed project context, not an operating-system isolation claim.
    pub fn project_digest(&self) -> &str {
        &self.document.project_digest
    }
}
/// Independent outcomes: authority can fail after the callback changed business
/// state. Never discard action on an authority error or infer automatic rollback.
#[must_use = "inspect both authority and action, including after-use failures"]
pub struct Use<T, E> {
    /// Current journal validation/completion, possibly failing after action use.
    pub authority: Result<(), Error>,
    /// None means the callback was not invoked; Some retains its complete result.
    pub action: Option<Result<T, E>>,
}

#[cfg(test)]
mod tests {
    use super::Operation;

    /// Every transport spelling round-trips through the same signed vocabulary;
    /// no read/export, alternate case or caller-selected operation is adopted.
    #[test]
    fn transport_operation_is_closed_and_canonical() {
        for name in ["mandate-create", "mandate-revoke", "request", "approve",
            "commit", "reconcile", "config-preview", "config-apply",
            "audit-transaction", "evidence-key-prepare", "evidence-key-activate",
            "evidence-key-revoke", "public-anchor"] {
            assert_eq!(Operation::parse(name).unwrap().as_str(), name);
        }
        for name in ["", "show", "export", "anchor", "verify", "direct-write",
            "elevate", "Approve", " approve", "approve ", "*", "../commit"] {
            assert!(Operation::parse(name).is_err(), "{name}");
        }
    }
}

impl Control {
    /// Crate-only composition: caller must hold the configured current personal
    /// source throughout this call. Exact permission/time are rechecked before
    /// callback and expiry afterwards; authority completion errors retain action.
    pub(crate) fn trust_action<T, E>(
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
