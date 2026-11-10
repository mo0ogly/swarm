//! Trusted single-intent HTTP source, never constructed from request JSON.
use std::{error::Error, fmt};
use swarm_engine::{RootStore, RootStoreError, TrustEnrollmentRequest, TrustReply};
use swarm_enrollment_authority::{
    authenticated_control::{AuthenticatedControl, CloseError, Context},
    authenticated_startup,
};
use swarm_enrollment_state::Head;

/// Own one explicitly prepared exact intent and its independently pinned source.
/// This is a trusted startup input, not a general identity provider or queue.
/// The caller provisions its exact permission separately before opening it.
pub struct TrustEnrollmentSource {
    prepared: TrustEnrollmentRequest,
    authority: AuthenticatedControl,
    head: Head,
}
/// Startup/close causes are independent; close failure is never authentication.
/// No path, pin, password or private nested diagnostic is formatted.
pub struct TrustEnrollmentSourceError {
    pub startup: Option<authenticated_startup::Error>,
    pub authority_close: Option<CloseError>,
    pub request_close: Option<RootStoreError>,
}
impl TrustEnrollmentSourceError {
    /// Complete retained causes; source() alone cannot enumerate cleanup errors.
    pub fn branches(&self) -> Vec<&(dyn Error + 'static)> {
        let mut causes: Vec<&(dyn Error + 'static)> = Vec::new();
        if let Some(e) = &self.startup {
            causes.push(e);
        }
        if let Some(e) = &self.authority_close {
            causes.push(e);
        }
        if let Some(e) = &self.request_close {
            causes.push(e);
        }
        causes
    }
}
impl fmt::Display for TrustEnrollmentSourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("trust-enrollment-source-failed")
    }
}
impl fmt::Debug for TrustEnrollmentSourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for TrustEnrollmentSourceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.branches().into_iter().next()
    }
}
impl TrustEnrollmentSource {
    /// Compare the prepared witness against the very Root being bound, before
    /// web-session publication. Caller keeps that opened Root through bootstrap.
    pub(super) fn check_root(&self, root: &RootStore) -> Result<(), RootStoreError> {
        self.prepared.check_root(root)
    }
    /// Public startup observation only; this does not authenticate the next call
    /// or promise that its account/grant remains current. Contains no body/secret.
    pub(super) fn capability(&self) -> serde_json::Value {
        serde_json::json!({"root_api_available":true,"http_available":true,"scope":"single_prepared_intent","reason":"trusted-single-intent-source-bound","operation":self.prepared.request().operation().as_str(),"request_digest":self.prepared.request().request_digest()})
    }
    /// Open only an independently approved protected declaration/pin, after the
    /// owner has provisioned the prepared permission. No enrollment/account is
    /// created or current head adopted. On refusal close the owned request too.
    pub fn open(
        prepared: TrustEnrollmentRequest,
        declaration: &str,
        pin: &str,
    ) -> Result<Self, TrustEnrollmentSourceError> {
        match authenticated_startup::open(&swarm_enrollment_state::Context::new(), declaration, pin)
        {
            Ok((authority, head)) => Ok(Self {
                prepared,
                authority,
                head,
            }),
            Err(startup) => Err(TrustEnrollmentSourceError {
                startup: Some(startup),
                authority_close: None,
                request_close: prepared.close().err(),
            }),
        }
    }
    /// Run only the configured exact operation. Root validates original bytes,
    /// physical binding, current plan/revision and mapped role under held guards.
    /// Authentication failure never falls back to a local bearer. Caller retains
    /// HTTP header/body ownership and owes cancellation/transport policy handling.
    pub fn call(
        &self,
        root: &RootStore,
        operation: &str,
        raw: &[u8],
        password: &[u8],
    ) -> Result<TrustReply, RootStoreError> {
        use sha2::{Digest, Sha256};
        let digest = Sha256::digest(raw)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        if operation != self.prepared.request().operation().as_str()
            || digest != self.prepared.request().request_digest()
        {
            return Err(RootStoreError::Refused("trust-enrollment-operation"));
        }
        root.call_trust_as_enrolled(
            &self.prepared,
            &self.authority,
            &Context::new(),
            &self.head,
            password,
            raw,
        )
        .map_err(|e| RootStoreError::TrustEnrollment(Box::new(e)))
    }
    /// Close both owners after all requests drain; retain both independent causes.
    /// This neither revokes the signed grant nor removes persisted account data.
    pub fn close(self) -> Result<(), TrustEnrollmentSourceError> {
        let authority_close = self.authority.close().err();
        let request_close = self.prepared.close().err();
        if authority_close.is_some() || request_close.is_some() {
            Err(TrustEnrollmentSourceError {
                startup: None,
                authority_close,
                request_close,
            })
        } else {
            Ok(())
        }
    }
}
