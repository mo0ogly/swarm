//! Persisted structured-provider waiting and scope-lease maintenance policy.
use crate::{EngineSettingsError as E, PlanningActionPolicy, SettingCount};
use serde::Serialize;
use std::time::Duration;

/// Explicit settings27, with no historical fallback. Zero disables silence,
/// maximum duration or renewal individually. The default lease is coordination
/// ownership, not a provider-duration bound; a live owner can renew it. Timers
/// consume transport activity and suspend-aware elapsed observations, not proof
/// that a model is making useful progress or that remote billing has stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct PlanningWaitPolicy {
    pub silence_seconds: SettingCount,
    pub max_duration_seconds: SettingCount,
    pub default_lease_seconds: SettingCount,
    pub renew_after_percent: SettingCount,
}

/// Policy observation only; signalling an actual owned process and handling
/// unresolved children/remote requests remain separate controller obligations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanningWaitStop {
    MaximumDuration,
    Silence,
}

impl PlanningWaitPolicy {
    /// Check intrinsic integer domains without IO or clamping. Renewal0 disables
    /// maintenance;1..99 schedules strictly before lease expiry.100 cannot renew
    /// a still-live lease and is invalid. Seconds are nonnegative signed64 input
    /// values; no legacy Go nanosecond ceiling is imposed on Rust Duration.
    fn intrinsic(&self) -> Result<(), E> {
        if self.silence_seconds.0 < 0 || self.max_duration_seconds.0 < 0 {
            return Err(E::Refused("settings-planning-wait-duration-invalid"));
        }
        if !(0..100).contains(&self.renew_after_percent.0) {
            return Err(E::Refused("settings-planning-wait-renewal-invalid"));
        }
        if self.default_lease_seconds.0 < 1 {
            return Err(E::Refused("settings-planning-wait-lease-invalid"));
        }
        Ok(())
    }

    /// Validate the complete saved policy against current authoritative lease
    /// bounds. Contradictory edits refuse, never silently adjust a default or
    /// an already owned lease. Caller owns document/history validation.
    pub fn validate(&self, action: &PlanningActionPolicy) -> Result<(), E> {
        self.intrinsic()?;
        action.check_lease(self.default_lease_seconds.0)
    }

    /// Derive renewal scheduling from the ACTUAL positive claimed lease and the
    /// explicit saved percentage. Zero returnsNone; no half-lease constant or
    /// provider timeout is inferred. Nanosecond arithmetic and conversions are
    /// checked; callers still validate current min/max, actual identity/fences,
    /// timestamp representability and recheck before renewing. Scheduling jitter
    /// may lose a lease; this result cannot resurrect expired ownership.
    pub fn renewal_delay(&self, lease_seconds: i64) -> Result<Option<Duration>, E> {
        self.intrinsic()?;
        let seconds = u64::try_from(lease_seconds)
            .ok()
            .filter(|n| *n > 0)
            .ok_or(E::Refused("planning-wait-lease-invalid"))?;
        if self.renew_after_percent.0 == 0 {
            return Ok(None);
        }
        let nanos = Duration::from_secs(seconds)
            .as_nanos()
            .checked_mul(self.renew_after_percent.0 as u128)
            .ok_or(E::Refused("planning-wait-renewal-representation"))?
            / 100;
        let seconds = u64::try_from(nanos / 1_000_000_000)
            .map_err(|_| E::Refused("planning-wait-renewal-representation"))?;
        let subsecond = u32::try_from(nanos % 1_000_000_000)
            .map_err(|_| E::Refused("planning-wait-renewal-representation"))?;
        Ok(Some(Duration::new(seconds, subsecond)))
    }

    /// Evaluate explicitly enabled timers at an observed controller wake. The
    /// caller supplies suspend-aware elapsed/idle durations from a monotonic
    /// source and resets idle only on actual provider bytes. Zero disables only
    /// its timer; a default lease never becomes an elapsed-duration limit. Both
    /// expired timers report maximum first deterministically. Observation is not
    /// a hard realtime/remote cancellation guarantee or process-exit evidence.
    pub fn stop_reason(
        &self,
        elapsed: Duration,
        idle: Duration,
    ) -> Result<Option<PlanningWaitStop>, E> {
        self.intrinsic()?;
        if self.max_duration_seconds.0 > 0
            && elapsed >= Duration::from_secs(self.max_duration_seconds.0 as u64)
        {
            return Ok(Some(PlanningWaitStop::MaximumDuration));
        }
        if self.silence_seconds.0 > 0 && idle >= Duration::from_secs(self.silence_seconds.0 as u64)
        {
            return Ok(Some(PlanningWaitStop::Silence));
        }
        Ok(None)
    }
}
