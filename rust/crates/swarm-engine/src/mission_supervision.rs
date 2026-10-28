//! Mission lease clock policy. Presence is not process liveness or agent identity.
use crate::{EngineSettingsError, SettingCount};

/// Explicit v21 settings, frozen at claim. Positive cadence/expiry and nonnegative
/// future tolerance are operator choices. No hidden fallback or unlimited value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissionSupervisionPolicy {
    pub poll_interval_ms: SettingCount,
    pub stale_after_ms: SettingCount,
    pub future_tolerance_ms: SettingCount,
}
/// Clock classification only; stopped/ownership/business eligibility are separate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissionLeaseClock {
    Present,
    Stale,
    Future,
}
impl MissionSupervisionPolicy {
    /// Validate supplied settings without IO. Representation accepts the full i64
    /// millisecond range; comparisons use i128 nanoseconds without truncation.
    /// Positive expiry is inclusive, and zero future tolerance is meaningful.
    pub fn validate(&self) -> Result<(), EngineSettingsError> {
        if self.poll_interval_ms.0 < 1
            || self.stale_after_ms.0 < 1
            || self.future_tolerance_ms.0 < 0
        {
            return Err(EngineSettingsError::Refused(
                "settings-mission-supervision-bounds",
            ));
        }
        Ok(())
    }
    /// Compare strict Go-compatible RFC3339 timestamps with frozen policy. Invalid
    /// input retains the parser cause, never becomes absence. Slight future stamps
    /// within tolerance remain present; exact expiry remains present as in Go.
    /// No IO, sleep, process signal, clock authentication or ownership is implied.
    pub fn observe(
        &self,
        heartbeat: &str,
        now: &str,
    ) -> Result<MissionLeaseClock, MissionSupervisionError> {
        self.validate().map_err(MissionSupervisionError::Settings)?;
        let heartbeat = parse(heartbeat)?;
        let now = parse(now)?;
        let age = now.unix_timestamp_nanos() - heartbeat.unix_timestamp_nanos();
        Ok(
            if -age > i128::from(self.future_tolerance_ms.0) * 1_000_000 {
                MissionLeaseClock::Future
            } else if age > i128::from(self.stale_after_ms.0) * 1_000_000 {
                MissionLeaseClock::Stale
            } else {
                MissionLeaseClock::Present
            },
        )
    }
}
/// Safe classification; typed storage/clock/settings causes and both cleanup
/// failures survive. Commit uncertainty is not rollback or permission to retry.
pub enum MissionSupervisionError {
    Refused(&'static str),
    Settings(EngineSettingsError),
    Root(Box<crate::RootStoreError>),
    Sql(rusqlite::Error),
    Cause {
        code: &'static str,
        cause: Box<dyn std::error::Error + Send + Sync>,
    },
    Cleanup {
        primary: Box<Self>,
        cleanup: Box<Self>,
    },
    Commit {
        cause: Box<rusqlite::Error>,
        rollback_confirmed: bool,
        cleanup: Option<Box<rusqlite::Error>>,
    },
    /// The lease mutation committed before an owned descriptor/release failed.
    /// Re-read; do not blindly repeat acquisition or imply business completion.
    Recorded {
        primary: Box<Self>,
    },
}
impl MissionSupervisionError {
    /// Stable safe code excludes private source labels, host tuples and SQL text.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Refused(code) | Self::Cause { code, .. } => code,
            Self::Settings(_) => "mission-settings-invalid",
            Self::Root(_) => "mission-root-unavailable",
            Self::Sql(_) => "mission-storage-failed",
            Self::Cleanup { .. } => "mission-cleanup-failed",
            Self::Commit {
                rollback_confirmed: true,
                ..
            } => "mission-commit-refused",
            Self::Commit { .. } => "mission-commit-uncertain",
            Self::Recorded { .. } => "mission-mutation-recorded-cleanup-failed",
        }
    }
    /// Cleanup, commit and known-recorded uncertainty must not become an automatic
    /// Root storage retry just because a nested SQL cause is BUSY. This pure
    /// classifier grants no recovery action or business acceptance.
    pub fn requires_reconciliation(&self) -> bool {
        matches!(
            self,
            Self::Cleanup { .. } | Self::Commit { .. } | Self::Recorded { .. }
        )
    }
    /// Independently retain cleanup failure when Error::source follows primary.
    pub fn cleanup(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Cleanup { cleanup, .. } => Some(cleanup.as_ref()),
            Self::Commit {
                cleanup: Some(error),
                ..
            } => Some(error.as_ref()),
            Self::Recorded { primary } => primary.cleanup(),
            _ => None,
        }
    }
}
impl std::fmt::Display for MissionSupervisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::fmt::Debug for MissionSupervisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
impl std::error::Error for MissionSupervisionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Settings(e) => Some(e),
            Self::Root(e) => Some(e.as_ref()),
            Self::Sql(e) => Some(e),
            Self::Cause { cause, .. } => Some(cause.as_ref()),
            Self::Cleanup { primary, .. } | Self::Recorded { primary } => Some(primary.as_ref()),
            Self::Commit { cause, .. } => Some(cause.as_ref()),
            Self::Refused(_) => None,
        }
    }
}
/// Parse observation without lossy timezone/nanosecond normalization or fallback.
/// The clock is supplied by the trusted local caller, not authenticated by parsing.
pub(crate) fn parse(raw: &str) -> Result<time::OffsetDateTime, MissionSupervisionError> {
    swarm_wire_json::go_rfc3339::parse(raw).map_err(|cause| MissionSupervisionError::Cause {
        code: "mission-clock-invalid",
        cause: Box::new(cause),
    })
}
/// Join mandatory cleanup without swallowing either cause or retrying.
pub(crate) fn finish<T>(
    result: Result<T, MissionSupervisionError>,
    cleanup: Result<(), MissionSupervisionError>,
) -> Result<T, MissionSupervisionError> {
    match (result, cleanup) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), Ok(())) | (Ok(_), Err(e)) => Err(e),
        (Err(primary), Err(cleanup)) => Err(MissionSupervisionError::Cleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    }
}
