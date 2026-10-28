//! Explicit scope-activation policy. No lease, execution or authority is created.
use crate::{EngineSettingsError, SettingCount};

/// Saved scope-lease bounds and projection caps, separate from supervisor leases.
/// None removes an operator ceiling; positive leases and timestamp/usize
/// representation remain required. Zero projection caps forbid nonempty output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlanningActionPolicy {
    pub lease_min_seconds: SettingCount,
    pub lease_max_seconds: Option<SettingCount>,
    pub context_max_events: Option<usize>,
    pub context_max_bytes: Option<usize>,
}
impl PlanningActionPolicy {
    /// Reject contradictory bounds without IO, mutation, default adoption or
    /// clamping. A finite maximum must admit at least the positive minimum.
    pub fn validate(&self) -> Result<(), EngineSettingsError> {
        if self.lease_min_seconds.0 < 1
            || self
                .lease_max_seconds
                .is_some_and(|v| v.0 < self.lease_min_seconds.0)
        {
            return Err(EngineSettingsError::Refused(
                "settings-planning-lease-bounds-invalid",
            ));
        }
        Ok(())
    }
    /// Validate one explicitly requested duration under current saved policy.
    /// No fallback/default lease is selected; existing deadlines are unchanged.
    /// Timestamp addition/formatting must also succeed at the actual consumer.
    pub fn check_lease(&self, seconds: i64) -> Result<(), EngineSettingsError> {
        self.validate()?;
        if seconds < self.lease_min_seconds.0
            || self.lease_max_seconds.is_some_and(|v| seconds > v.0)
        {
            return Err(EngineSettingsError::Refused(
                "planning-lease-outside-policy",
            ));
        }
        Ok(())
    }
}

/// Persisted decision request/operation limits, separate from transport limits.
/// Null removes a cap; zero forbids nonempty data. Positive worker attempts are
/// copied only to new tasks and never reset attempts or imply execution rights.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlanningDecisionPolicy {
    pub max_inputs: Option<usize>,
    pub max_operations: Option<usize>,
    pub max_reason_bytes: Option<usize>,
    pub max_title_characters: Option<usize>,
    pub max_instruction_bytes: Option<usize>,
    pub max_dependencies: Option<usize>,
    pub max_criterion_bytes: Option<usize>,
    pub worker_default_attempts: i64,
}
impl PlanningDecisionPolicy {
    /// Check one observed count/length against an explicit cap; overflow is
    /// represented by the caller's checked computation, never silently clamped.
    pub(crate) fn check(
        cap: Option<usize>,
        value: usize,
        reason: &'static str,
    ) -> Result<(), crate::AgentLaunchError> {
        if cap.is_some_and(|n| value > n) {
            return Err(crate::AgentLaunchError::Refused(reason));
        }
        Ok(())
    }
}
