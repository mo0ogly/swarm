//! Current-account invocation with explicit retained results on late failure.
use super::{
    PaeaSettlementError, PaeaSettlementObservation, PaeaSettlementOutcome, PaeaSettlementRequest,
    RootStore,
};
use std::{convert::Infallible, error::Error, fmt};
use swarm_enrollment_authority::{
    authenticated_control::{AuthenticatedControl, Context},
    paea_action::Use,
    personal::coordinator::UseError,
};
use swarm_enrollment_state::Head;

/// All authority and backend branches, including a known effect after failure.
/// No variant implies rollback. Formatting is fixed and excludes credentials,
/// canonical action, receipts and private nested diagnostics.
pub enum PaeaSettlementCallError {
    Personal(UseError<Use<PaeaSettlementOutcome, PaeaSettlementError>, Infallible>),
    Authority(Use<PaeaSettlementOutcome, PaeaSettlementError>),
    Action(PaeaSettlementError),
    MissingAction,
}
impl PaeaSettlementCallError {
    /// Complete callback outcome when the held operation reached the backend.
    fn action(&self) -> Option<&Result<PaeaSettlementOutcome, PaeaSettlementError>> {
        match self {
            Self::Personal(UseError::AfterUse {
                outcome: Ok(used), ..
            })
            | Self::Authority(used) => used.action.as_ref(),
            _ => None,
        }
    }
    /// Borrow the backend error without discarding primary/cleanup/observation.
    pub fn backend_error(&self) -> Option<&PaeaSettlementError> {
        match self {
            Self::Action(e) => Some(e),
            _ => self.action().and_then(|a| a.as_ref().err()),
        }
    }
    /// Known recorded/replayed or negative reconciliation result despite failure.
    /// Availability never restores expired/revoked authority or grants a replay.
    pub fn known_outcome(&self) -> Option<&PaeaSettlementOutcome> {
        if let Some(Ok(known)) = self.action() {
            return Some(known);
        }
        self.backend_error().and_then(|e| match &e.observation {
            PaeaSettlementObservation::Known(known) => Some(known.as_ref()),
            _ => None,
        })
    }
    /// True only when the backend explicitly retained uncertain commitment.
    /// False does not establish absence of a historical or current effect.
    pub fn commit_uncertain(&self) -> bool {
        self.backend_error()
            .is_some_and(|e| matches!(e.observation, PaeaSettlementObservation::Uncertain))
    }
    /// Independent personal, business and backend/cleanup causes. Standard
    /// source() supplies the first only; consumers must inspect all branches.
    pub fn branches(&self) -> Vec<&(dyn Error + 'static)> {
        let mut out: Vec<&(dyn Error + 'static)> = Vec::new();
        match self {
            Self::Personal(error) => {
                out.push(error);
                if let UseError::AfterUse {
                    outcome: Ok(used), ..
                } = error
                {
                    if let Err(cause) = &used.authority {
                        out.push(cause);
                    }
                }
            }
            Self::Authority(used) => {
                if let Err(cause) = &used.authority {
                    out.push(cause);
                }
            }
            Self::Action(_) | Self::MissingAction => (),
        }
        if let Some(error) = self.backend_error() {
            out.push(error);
            for cleanup in &error.cleanup {
                out.push(cleanup);
            }
        }
        out
    }
    /// Find the retained typed envelope through the existing Box/IO/Root branch
    /// walker; no message parsing, downcast fallback or invented outcome.
    /// Returns the first envelope in branch order, without merging multiple
    /// independent calls; None establishes neither rollback nor absence of effects.
    pub fn find<'a>(error: &'a (dyn Error + 'static)) -> Option<&'a Self> {
        let mut pending = vec![error];
        let mut seen: Vec<*const dyn Error> = Vec::new();
        while let Some(error) = pending.pop() {
            let pointer = error as *const dyn Error;
            if seen.iter().any(|old| std::ptr::eq(*old, pointer)) {
                continue;
            }
            seen.push(pointer);
            if let Some(found) = error.downcast_ref::<Self>() {
                return Some(found);
            }
            pending.extend(
                crate::native_error_links(error)
                    .children
                    .into_iter()
                    .rev()
                    .map(|(_, cause)| cause),
            );
        }
        None
    }
    /// Fixed safe diagnostic, never a receipt, a password or nested SQL text.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Personal(_) => "paea-settlement-personal",
            Self::Authority(_) => "paea-settlement-authority",
            Self::Action(e) => e.code,
            Self::MissingAction => "paea-settlement-action-missing",
        }
    }
}
impl fmt::Display for PaeaSettlementCallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for PaeaSettlementCallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for PaeaSettlementCallError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.branches().into_iter().next()
    }
}
impl RootStore {
    /// Run one exact settlement/reconciliation through a configured current
    /// account and held business grant. Caller supplies only CAS head/password
    /// and retained prepared request, never a serialized capability. Preserve all
    /// backend outcomes and authority completions; no retry, provisioning, local
    /// fallback, daemon or human-presence inference. Caller closes owned sources.
    pub fn call_paea_settlement(
        &self,
        project: &RootStore,
        prepared: &PaeaSettlementRequest,
        authority: &AuthenticatedControl,
        context: &Context,
        head: &Head,
        password: &[u8],
    ) -> Result<PaeaSettlementOutcome, PaeaSettlementCallError> {
        let used = authority
            .with_paea_action(context, head, password, prepared.request(), |identity| {
                self.paea_settle(project, prepared, identity)
            })
            .map_err(PaeaSettlementCallError::Personal)?;
        if used.authority.is_err() {
            return Err(PaeaSettlementCallError::Authority(used));
        }
        used.action
            .ok_or(PaeaSettlementCallError::MissingAction)?
            .map_err(PaeaSettlementCallError::Action)
    }
}
