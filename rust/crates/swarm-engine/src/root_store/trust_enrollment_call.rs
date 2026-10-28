//! Shared current-identity invocation for transports; no startup or grant issuance.
use super::{RootStore, RootStoreError, TrustEnrollmentRequest};
use crate::TrustReply;
use std::{convert::Infallible, error::Error, fmt};
use swarm_enrollment_authority::{
    authenticated_control::{AuthenticatedControl, Context},
    personal::coordinator::UseError,
    trust_action::Use,
};
use swarm_enrollment_state::Head;

/// Full invocation failure, including a known action after authority completion
/// failed. No variant means rollback. Transports must inspect publication/reply
/// and all branches; ordinary formatting deliberately excludes private payloads.
pub enum TrustEnrollmentCallError {
    Personal(UseError<Use<TrustReply, RootStoreError>, Infallible>),
    Authority(Use<TrustReply, RootStoreError>),
    Action(RootStoreError),
    MissingAction,
}
impl TrustEnrollmentCallError {
    /// Complete action result, when invocation reached the business callback.
    /// Some(Ok) can coexist with failed current authority; it is never success.
    fn action(&self) -> Option<&Result<TrustReply, RootStoreError>> {
        match self {
            Self::Personal(UseError::AfterUse {
                outcome: Ok(used), ..
            })
            | Self::Authority(used) => used.action.as_ref(),
            _ => None,
        }
    }
    /// Borrow the actual known business reply after a late authority failure.
    /// Do not automatically replay: reply availability does not restore authority.
    pub fn known_reply(&self) -> Option<&TrustReply> {
        self.action().and_then(|a| a.as_ref().ok())
    }
    /// Known publication from the reply or action error; None means unknown,
    /// never proof of no mutation. No private response bytes are formatted.
    pub fn publication(&self) -> Option<&'static str> {
        match self {
            Self::Action(error) => error.trust_publication(),
            _ => self.action().and_then(|a| match a {
                Ok(reply) => reply.publication,
                Err(error) => error.trust_publication(),
            }),
        }
    }
    /// Every independent cause beyond source(), including an inner action error
    /// beside business/personal completion failure. Nested Root wrappers remain
    /// subject to the engine's existing traversal matrix.
    pub fn branches(&self) -> Vec<&(dyn Error + 'static)> {
        let mut out: Vec<&(dyn Error + 'static)> = Vec::new();
        match self {
            Self::Personal(error) => {
                out.push(error);
                if let UseError::AfterUse {
                    outcome: Ok(used), ..
                } = error
                {
                    if let Err(error) = &used.authority {
                        out.push(error);
                    }
                }
            }
            Self::Authority(used) => {
                if let Err(error) = &used.authority {
                    out.push(error);
                }
            }
            Self::Action(error) => out.push(error),
            Self::MissingAction => (),
        }
        if let Some(Err(error)) = self.action() {
            out.push(error);
        }
        out
    }
    /// Safe class only; credentials and nested private diagnostics stay retained.
    pub fn code(&self) -> &'static str {
        match self {
            // These typed failures precede business use. Storage, stale heads,
            // and every after-use failure remain server/authority failures.
            Self::Personal(UseError::Before(
                swarm_enrollment_authority::personal::coordinator::Error::Personal(
                    swarm_enrollment_authority::personal::Error::Credential(
                        swarm_enrollment_authority::credentials::Error::Password
                        | swarm_enrollment_authority::credentials::Error::Empty,
                    ),
                ),
            )) => "authorization_required",
            Self::Authority(Use { authority: Err(swarm_enrollment_authority::control::Error::Permission), action: None }) => "authorization_required",
            Self::Personal(_) => "trust-enrollment-personal",
            Self::Authority(_) => "trust-enrollment-authority",
            Self::Action(error) => error.code(),
            Self::MissingAction => "trust-enrollment-action-missing",
        }
    }
}
impl fmt::Display for TrustEnrollmentCallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for TrustEnrollmentCallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for TrustEnrollmentCallError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.branches().into_iter().next()
    }
}
impl RootStore {
    /// Invoke one prepared exact intent through independently opened current
    /// personal/business authority. Retains the complete outcome when either
    /// completion fails after callback. No source, head, role or grant is inferred
    /// from raw bytes; caller owes protected startup and checked owner closes.
    pub fn call_trust_as_enrolled(
        &self,
        prepared: &TrustEnrollmentRequest,
        authority: &AuthenticatedControl,
        context: &Context,
        head: &Head,
        password: &[u8],
        raw: &[u8],
    ) -> Result<TrustReply, TrustEnrollmentCallError> {
        let used = authority
            .with_trust_action(context, head, password, prepared.request(), |identity| {
                self.trust_as_enrolled(prepared, identity, raw)
            })
            .map_err(TrustEnrollmentCallError::Personal)?;
        if used.authority.is_err() {
            return Err(TrustEnrollmentCallError::Authority(used));
        }
        used.action
            .ok_or(TrustEnrollmentCallError::MissingAction)?
            .map_err(TrustEnrollmentCallError::Action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use swarm_enrollment_authority::control;

    /// Only typed pre-use credential/permission refusals classify as forbidden;
    /// a late failure with a known reply must retain its server-failure class.
    #[test]
    fn refusal_classification_never_hides_late_effect() {
        let denied=TrustEnrollmentCallError::Authority(Use{authority:Err(control::Error::Permission),action:None});
        assert_eq!(denied.code(),"authorization_required");
        let late=TrustEnrollmentCallError::Authority(Use{authority:Err(control::Error::Permission),action:Some(Ok(TrustReply{bytes:vec![],publication:Some("recorded")}))});
        assert_eq!(late.code(),"trust-enrollment-authority");
        assert_eq!(late.publication(),Some("recorded"));
    }

    /// A callback reply survives authority refusal through direct, boxed and
    /// custom-IO Root wrappers; neither formatting nor labels leak reply bytes.
    #[test]
    fn late_authority_failure_retains_reply_and_publication() {
        let late = || {
            TrustEnrollmentCallError::Authority(Use {
                authority: Err(control::Error::Binding),
                action: Some(Ok(TrustReply {
                    bytes: b"private-reply".to_vec(),
                    publication: Some("recorded"),
                })),
            })
        };
        let error = late();
        assert_eq!(error.known_reply().unwrap().bytes, b"private-reply");
        assert_eq!(error.publication(), Some("recorded"));
        assert!(!format!("{error:?}").contains("private-reply"));
        let root = RootStoreError::TrustEnrollment(Box::new(late()));
        assert_eq!(root.trust_publication(), Some("recorded"));
        let io = std::io::Error::other(root);
        let root = RootStoreError::SqlTransaction(Box::new(io));
        assert_eq!(root.trust_publication(), Some("recorded"));
        let error: Box<dyn Error> = Box::new(Box::new(late()));
        let links = crate::native_error_links(error.as_ref());
        assert_eq!(links.children.len(), 1);
        assert_eq!(
            links.children[0]
                .1
                .downcast_ref::<TrustEnrollmentCallError>()
                .unwrap()
                .publication(),
            Some("recorded")
        );
    }

    /// Personal completion can fail after business completion. Both business
    /// failure and publication survive beside that independent outer cause.
    #[test]
    fn personal_after_use_retains_action_and_both_authority_causes() {
        let error = TrustEnrollmentCallError::Personal(UseError::AfterUse {
            cause: swarm_enrollment_state::Error::Head,
            outcome: Ok(Use {
                authority: Err(swarm_enrollment_authority::control::Error::Binding),
                action: Some(Err(RootStoreError::TrustPublished {
                    primary: Box::new(RootStoreError::Refused("fixture-late-write")),
                    publication: "recorded",
                })),
            }),
        });
        assert_eq!(error.branches().len(), 3);
        assert_eq!(error.publication(), Some("recorded"));
        let root = RootStoreError::TrustEnrollment(Box::new(error));
        assert_eq!(root.trust_publication(), Some("recorded"));
        assert!(!format!("{root:?}").contains("fixture-late-write"));
    }

    /// Inner action refusal remains a distinct cause beside authority failure;
    /// absent action never invents a reply or a publication.
    #[test]
    fn independent_action_cause_and_missing_result_are_not_success() {
        let error = TrustEnrollmentCallError::Authority(Use {
            authority: Err(control::Error::Binding),
            action: Some(Err(RootStoreError::Refused("fixture-action-refusal"))),
        });
        assert_eq!(error.branches().len(), 2);
        assert!(error.known_reply().is_none());
        assert!(error.publication().is_none());
        let missing = TrustEnrollmentCallError::MissingAction;
        assert!(missing.branches().is_empty());
        assert!(missing.known_reply().is_none());
        assert!(missing.publication().is_none());
    }
}
