//! Safe preparation failures retaining original and cleanup causes.
use std::{error::Error, fmt};
/// No failure variant grants a process, reservation, identity or acceptance.
/// Commit uncertainty requires rereading the event/agent before replay.
pub enum AgentLaunchError {
    Refused(&'static str),
    /// Saved SQLite retry policy exhausted. Count includes the first SQL attempt;
    /// revision identifies the policy refusing another attempt, not agent rights.
    Busy {
        cause: Box<Self>,
        attempts: u64,
        settings_revision: crate::SettingCount,
    },
    Cause {
        code: &'static str,
        cause: Box<dyn Error + Send + Sync>,
    },
    Cleanup {
        primary: Box<Self>,
        cleanup: Box<Self>,
    },
    Commit {
        cause: Box<Self>,
        rollback_confirmed: bool,
        cleanup: Option<Box<Self>>,
    },
}
impl AgentLaunchError {
    /// Keep the original typed cause while rendering only a static public code.
    pub(crate) fn caused(code: &'static str, cause: impl Error + Send + Sync + 'static) -> Self {
        Self::Cause {
            code,
            cause: Box::new(cause),
        }
    }
    /// Identify only direct SQLite BUSY storage failures. Compound cleanup,
    /// nested business/read layers and all commit outcomes stay non-retryable.
    /// Caller must also verify autocommit after checked rollback and release its
    /// settings reservation before retrying; this classifier alone proves neither.
    pub(crate) fn retryable_busy(&self) -> bool {
        match self {
            Self::Cause {
                code: "agent-launch-storage",
                cause,
            } => cause
                .downcast_ref::<rusqlite::Error>()
                .and_then(rusqlite::Error::sqlite_error)
                .is_some_and(|e| e.code == rusqlite::ErrorCode::DatabaseBusy),
            _ => false,
        }
    }
    /// Stable diagnostic contains no paths, prompt, credentials or raw SQL.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Refused(code) | Self::Cause { code, .. } => code,
            Self::Busy { .. } => "agent-launch-storage-busy",
            Self::Cleanup { .. } => "agent-launch-cleanup",
            Self::Commit {
                rollback_confirmed: true,
                ..
            } => "agent-launch-commit-refused",
            Self::Commit { .. } => "agent-launch-commit-uncertain",
        }
    }
}
impl fmt::Display for AgentLaunchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Self::Busy {
            attempts,
            settings_revision,
            ..
        } = self
        {
            write!(
                f,
                "{} (storage_attempts={attempts}; settings_revision={})",
                self.code(),
                settings_revision.0
            )
        } else {
            f.write_str(self.code())
        }
    }
}
impl fmt::Debug for AgentLaunchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for AgentLaunchError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Refused(_) => None,
            Self::Cause { cause, .. } => Some(cause.as_ref()),
            Self::Cleanup { primary, .. } => Some(primary.as_ref()),
            Self::Commit { cause, .. } | Self::Busy { cause, .. } => Some(cause.as_ref()),
        }
    }
}
/// Join an operation and mandatory cleanup without discarding either failure.
/// A successful operation becomes refusal if its cleanup fails; no retry occurs.
pub(crate) fn finish<T>(
    result: Result<T, AgentLaunchError>,
    cleanup: Result<(), AgentLaunchError>,
) -> Result<T, AgentLaunchError> {
    match (result, cleanup) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), Ok(())) | (Ok(_), Err(e)) => Err(e),
        (Err(primary), Err(cleanup)) => Err(AgentLaunchError::Cleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    }
}
