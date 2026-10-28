//! Authentication sources chosen by trusted callers, never by command JSON.
use super::{proof_types::*, state::State, TrustError as E};
use crate::trust_types::{credential_valid, token_hash, TrustIdentity};
use subtle::ConstantTimeEq;
#[derive(Clone, Copy)]
pub(crate) enum Source<'a> {
    Local(&'a str),
    #[cfg(target_os = "linux")]
    Enrolled(&'a swarm_enrollment_authority::trust_action::Identity<'a>),
}
impl<'a> From<&'a str> for Source<'a> {
    fn from(value: &'a str) -> Self {
        Self::Local(value)
    }
}
/// Runtime actor owns observations, not future authority. Only Source under held
/// enrollment guards can create its nonlocal context; persisted mappings cannot.
pub(crate) struct Actor {
    identity: TrustIdentity,
    pub enrollment: Option<EnrollmentContext>,
    pub expires_at: Option<String>,
}
impl std::ops::Deref for Actor {
    type Target = TrustIdentity;
    fn deref(&self) -> &Self::Target {
        &self.identity
    }
}
/// Resolve a current stored role mapping with exactly one authentication source.
/// Enrolled mappings reject every bearer. No source fallback or mutation occurs.
pub(crate) fn authenticated<'a>(st: &State, source: impl Into<Source<'a>>) -> Result<Actor, E> {
    match source.into() {
        Source::Local(token) => {
            if !credential_valid(token) {
                return Err(E::Authorization);
            }
            let digest = token_hash(token);
            let identity = st
                .identities
                .iter()
                .find(|id| {
                    id.enrollment_principal.is_none()
                        && !id.disabled
                        && bool::from(id.token_hash.as_bytes().ct_eq(digest.as_bytes()))
                })
                .cloned()
                .ok_or(E::Authorization)?;
            Ok(Actor {
                identity,
                enrollment: None,
                expires_at: None,
            })
        }
        #[cfg(target_os = "linux")]
        Source::Enrolled(held) => {
            if held.trust_revision() != st.revision
                || time::OffsetDateTime::now_utc() >= held.expires_at()
            {
                return Err(E::Authorization);
            }
            let identity = st
                .identities
                .iter()
                .find(|id| {
                    !id.disabled && id.enrollment_principal.as_ref() == Some(held.principal())
                })
                .cloned()
                .ok_or(E::Authorization)?;
            let scope = held.scope();
            let enrollment = EnrollmentContext {
                enrollment_id: held.enrollment_id().into(),
                head_sequence: held.authority_head().sequence,
                head_digest: held.authority_head().digest.clone(),
                project_digest: held.project_digest().into(),
                work_id: scope.work_id.clone(),
                plan_version: scope.plan_version,
                plan_digest: scope.plan_digest.clone(),
            };
            Ok(Actor {
                identity,
                enrollment: Some(enrollment),
                expires_at: Some(super::operations::stamp(held.expires_at())?),
            })
        }
    }
}
