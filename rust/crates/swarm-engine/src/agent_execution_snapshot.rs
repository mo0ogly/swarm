//! Native execution metadata. Storage consistency is not agent authentication.
use crate::{AgentExecutionPolicy, EngineSettingsError, SettingCount};
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};
/// Durable gate phase, not evidence that the provider actually consumed input.
/// PermissionRecorded conservatively means it may have started; a crash between
/// recording and gate release must never manufacture a refund or task acceptance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentExecutionPhase {
    AwaitingPermission,
    PermissionRecorded,
}
/// Strict native v1 metadata bound to a Work revision and complete context hash.
/// Agent stores the complete raw JSON opaquely, including future constraints;
/// consumers use this strict decoder and refuse unsupported data without fallback.
/// These fields prove neither actor identity, kernel ownership nor execution.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentExecutionSnapshot {
    pub schema_version: i64,
    pub settings_revision: SettingCount,
    pub work_revision: i64,
    pub context_sha256: String,
    pub phase: AgentExecutionPhase,
    pub policy: AgentExecutionPolicy,
}
impl fmt::Debug for AgentExecutionSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AgentExecutionSnapshot { data: redacted }")
    }
}
/// Original parse/policy causes with safe families; raw context/JSON never appears
/// in Display/Debug. A refusal creates no lifecycle, process or authority effect.
pub enum AgentExecutionSnapshotError {
    Format(serde_json::Error),
    Policy(EngineSettingsError),
    Invalid,
}
impl AgentExecutionSnapshotError {
    /// Stable safe family; callers still owe ownership, freshness and gate checks.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Format(_) | Self::Invalid => "agent-execution-snapshot-invalid",
            Self::Policy(_) => "agent-execution-policy-invalid",
        }
    }
}
impl fmt::Display for AgentExecutionSnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for AgentExecutionSnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for AgentExecutionSnapshotError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Format(e) => Some(e),
            Self::Policy(e) => Some(e),
            Self::Invalid => None,
        }
    }
}
impl AgentExecutionSnapshot {
    /// Validate schema, positive revisions, canonical SHA256 and complete policy.
    /// Pure data checks do not read current Admin defaults, update a historical
    /// intention, verify a stored identity or permit any process to start.
    pub fn validate(&self) -> Result<(), AgentExecutionSnapshotError> {
        if self.schema_version != 1
            || self.settings_revision.0 < 1
            || self.work_revision < 1
            || self.context_sha256.len() != 64
            || !self
                .context_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(AgentExecutionSnapshotError::Invalid);
        }
        self.policy
            .validate()
            .map_err(AgentExecutionSnapshotError::Policy)
    }
}
/// Decode complete strict native metadata, without Go defaults/merging. Missing,
/// duplicate and unknown fields/phases, null required values and invalid policy
/// refuse with original typed causes. This neither authenticates metadata nor
/// proves its association with a current Agent row or a real kernel process.
pub fn decode_agent_execution_snapshot(
    raw: &[u8],
) -> Result<AgentExecutionSnapshot, AgentExecutionSnapshotError> {
    let snapshot: AgentExecutionSnapshot =
        swarm_wire_json::decode(raw).map_err(AgentExecutionSnapshotError::Format)?;
    snapshot.validate()?;
    Ok(snapshot)
}
