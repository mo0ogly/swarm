//! Compose held personal-account knowledge and held current business authority.
//! Trusted protected startup must supply both kernels and distinct physical
//! journals. No listener, session, human-presence claim or OS confinement here.
use crate::{
    control::{self, Control, Request, Response},
    personal::coordinator as personal,
};
use std::{error::Error as StdError, fmt};
use swarm_enrollment_config::store;
use swarm_enrollment_state::{self as state, Head};
/// Caller-owned cancellation across personal, business and configuration layers.
/// No timer, default duration, retry or preemptive external rollback is created.
#[derive(Clone, Default)]
pub struct Context {
    pub(crate) personal: personal::Context,
    business: state::Context,
    configuration: store::Context,
}
impl Context {
    /// Fresh uncanceled contexts for one operation; starts no worker/watchdog.
    pub fn new() -> Self {
        Self::default()
    }
    /// Mark all consumers canceled. Returned observations still determine known
    /// effects; cancellation does not roll back an already published configuration.
    pub fn cancel(&self) {
        self.personal.cancel();
        self.business.cancel();
        self.configuration.cancel()
    }
}
/// Both business result and personal after-use result remain explicit. A business
/// Err may contain known storage response/uncertainty; an outer AfterUse retains
/// that complete inner Result. Consumers must inspect both, never flatten into
/// empty success or infer no effect from a failure. No raw secret is returned.
pub type Outcome =
    Result<Result<Response, control::Error>, personal::UseError<Response, control::Error>>;
/// Own both preopened protected kernels. Public execute accepts password only,
/// never caller proof/actor. Current account remains held through business grant,
/// storage and both authority completions. This composition is not a transport
/// gate for legacy routes until protected startup and routing are integrated.
pub struct AuthenticatedControl {
    pub(crate) personal: personal::Coordinator,
    control: Control,
}
impl AuthenticatedControl {
    /// Take ownership of trusted independently provisioned sources. No IO,
    /// authentication or implicit first-caller adoption. Protected startup must
    /// verify root custody and separate lexical/physical journal domains.
    pub fn new(personal: personal::Coordinator, control: Control) -> Self {
        Self { personal, control }
    }
    /// Validate/fsync/release personal witness, verify actual current password
    /// account, bind principal/install to this control, then execute under current
    /// signed business authority. expected_business_head is CAS only, not a grant.
    /// Account and business generations are distinct. No actor, clock, grant or
    /// credential proof is accepted from the request. Preserve every inner/outer
    /// known result and failure; no automatic replay/recovery is authorized here.
    pub fn execute(
        &self,
        ctx: &Context,
        expected_business_head: &Head,
        password: &[u8],
        request: Request,
    ) -> Outcome {
        self.personal
            .with_authenticated(&ctx.personal, password, |proof| {
                self.control.execute_from_current_personal(
                    proof,
                    &ctx.business,
                    &ctx.configuration,
                    expected_business_head,
                    request,
                )
            })
    }
    /// Authenticate the configured current personal account, then hold the signed
    /// exact action grant through callback and authority completion. A RootStore
    /// consumer must compare root/request/plan/revision before use. This creates
    /// no identity, role or transport; after-use errors retain the complete result.
    pub fn with_trust_action<T, E>(
        &self,
        ctx: &Context,
        head: &Head,
        password: &[u8],
        request: &crate::trust_action::Request,
        f: impl for<'a> FnOnce(&crate::trust_action::Identity<'a>) -> Result<T, E>,
    ) -> Result<
        crate::trust_action::Use<T, E>,
        personal::UseError<crate::trust_action::Use<T, E>, std::convert::Infallible>,
    > {
        match self
            .personal
            .with_authenticated(&ctx.personal, password, |proof| {
                Ok::<_, std::convert::Infallible>(self.control.trust_action(
                    proof,
                    &ctx.business,
                    head,
                    request,
                    f,
                ))
            }) {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(never)) => match never {},
            Err(error) => Err(error),
        }
    }
    /// Authenticate the configured current personal account, then hold the signed
    /// exact action grant through callback and authority completion. A PAEA service
    /// must compare held storage/action/service/plan/revision before use. This creates
    /// a borrowed capability, no human-presence or OS isolation claim; after-use errors retain the complete result.
    pub fn with_paea_action<T, E>(
        &self,
        ctx: &Context,
        head: &Head,
        password: &[u8],
        request: &crate::paea_action::Request,
        f: impl for<'a> FnOnce(&crate::paea_action::Identity<'a>) -> Result<T, E>,
    ) -> Result<
        crate::paea_action::Use<T, E>,
        personal::UseError<crate::paea_action::Use<T, E>, std::convert::Infallible>,
    > {
        match self
            .personal
            .with_authenticated(&ctx.personal, password, |proof| {
                Ok::<_, std::convert::Infallible>(self.control.paea_action(
                    proof,
                    &ctx.business,
                    head,
                    request,
                    f,
                ))
            }) {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(never)) => match never {},
            Err(error) => Err(error),
        }
    }
    /// Attempt both owned closes, preserving each failure. One side can be
    /// partially closed while another refuses Busy; retry explicit close only
    /// after the active owner releases it, never reset history or infer rollback.
    pub fn close(&self) -> Result<(), CloseError> {
        let personal = self.personal.close().err();
        let control = self.control.close().err();
        if personal.is_some() || control.is_some() {
            Err(CloseError { personal, control })
        } else {
            Ok(())
        }
    }
}
/// Explicit independent close branches; both must be inspected. Error presence
/// alone establishes neither complete shutdown nor absence of an earlier effect.
pub struct CloseError {
    pub personal: Option<personal::Error>,
    pub control: Option<control::Error>,
}
impl fmt::Display for CloseError {
    /// Fixed categories only; no private nested storage/response data formatted.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "authenticated control close: personal={}, business={}",
            self.personal.as_ref().map(|e| e.code()).unwrap_or("none"),
            self.control.as_ref().map(|e| e.code()).unwrap_or("none")
        )
    }
}
impl fmt::Debug for CloseError {
    /// Same redacted representation, while structured branches remain intact.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for CloseError {
    /// First present cause; control is a separate required branch when both fail.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        if let Some(e) = &self.personal {
            Some(e)
        } else {
            self.control.as_ref().map(|e| e as &dyn StdError)
        }
    }
}
