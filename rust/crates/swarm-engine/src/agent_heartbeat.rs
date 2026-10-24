//! Frozen supervisor heartbeat observation. No termination or provider inference.
use crate::{Agent, AgentLaunchError, SettingCount};
/// Current clock versus retained supervisor heartbeat. Millisecond age is a
/// decimal string to preserve precision in API consumers. A heartbeat is not
/// authenticated actor identity, process liveness or provider progress.
#[derive(serde::Serialize, Debug)]
pub struct AgentHeartbeatObservation {
    pub source: &'static str,
    pub state: &'static str,
    pub age_ms: Option<String>,
    pub stale_after_ms: SettingCount,
}
/// Observe only agents with complete native supervision metadata. Legacy absence
/// returns None, never current defaults. Supplied observation time is not an
/// authenticated clock. Malformed snapshots/timestamps refuse with typed causes;
/// future heartbeat is explicit, not silently clamped to age zero. No IO, stop,
/// lease release, budget change or task acceptance occurs. The threshold is frozen
/// per execution; a newly saved Admin threshold applies to future executions.
pub fn observe_agent_heartbeat(
    agent: &Agent,
    now: &str,
) -> Result<Option<AgentHeartbeatObservation>, AgentLaunchError> {
    let Some(raw) = &agent.native_execution else {
        return Ok(None);
    };
    let snapshot = crate::decode_agent_execution_snapshot(raw.0.get().as_bytes())
        .map_err(|e| AgentLaunchError::caused(e.code(), e))?;
    let now = swarm_wire_json::go_rfc3339::parse(now)
        .map_err(|e| AgentLaunchError::caused("agent-heartbeat-clock-invalid", e))?;
    let heartbeat = swarm_wire_json::go_rfc3339::parse(&agent.heartbeat)
        .map_err(|e| AgentLaunchError::caused("agent-heartbeat-clock-invalid", e))?;
    let age = now.unix_timestamp_nanos() - heartbeat.unix_timestamp_nanos();
    let state = if !agent.is_active() {
        "ended"
    } else if age < 0 {
        "future"
    } else if age >= i128::from(snapshot.policy.heartbeat_stale_ms.0) * 1_000_000 {
        "stale"
    } else {
        "recent"
    };
    Ok(Some(AgentHeartbeatObservation {
        source: "retained_supervisor_heartbeat",
        state,
        age_ms: if age < 0 {
            None
        } else {
            Some((age / 1_000_000).to_string())
        },
        stale_after_ms: snapshot.policy.heartbeat_stale_ms,
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    /// Explicit frozen threshold after a complete metadata witness; no current
    /// settings fallback for historical absence, no clamping of future clocks.
    #[test]
    fn heartbeat_threshold_future_legacy_and_corruption_are_distinct() {
        let mut policy = crate::EngineSettingsValues::installation_defaults()
            .unwrap()
            .agent_execution_policy()
            .unwrap();
        policy.heartbeat_stale_ms = SettingCount(1000);
        let snapshot = crate::AgentExecutionSnapshot {
            schema_version: 1,
            settings_revision: SettingCount(1),
            work_revision: 1,
            context_sha256: "0".repeat(64),
            phase: crate::AgentExecutionPhase::PermissionRecorded,
            policy,
        };
        let mut agent = Agent {
            status: "running".into(),
            heartbeat: "2026-10-19T00:00:00Z".into(),
            native_execution: Some(crate::GoRaw(
                serde_json::value::to_raw_value(&snapshot).unwrap(),
            )),
            ..Agent::default()
        };
        assert_eq!(
            observe_agent_heartbeat(&agent, "2026-10-19T00:00:00.999Z")
                .unwrap()
                .unwrap()
                .state,
            "recent"
        );
        let stale = observe_agent_heartbeat(&agent, "2026-10-19T00:00:01Z")
            .unwrap()
            .unwrap();
        assert_eq!(stale.state, "stale");
        assert_eq!(stale.age_ms.as_deref(), Some("1000"));
        let future = observe_agent_heartbeat(&agent, "2026-10-18T23:59:59Z")
            .unwrap()
            .unwrap();
        assert_eq!(future.state, "future");
        assert_eq!(future.age_ms, None);
        agent.heartbeat = "invalid".into();
        assert!(observe_agent_heartbeat(&agent, "2026-10-19T00:00:00Z").is_err());
        agent.native_execution = None;
        assert!(observe_agent_heartbeat(&agent, "invalid")
            .unwrap()
            .is_none());
    }
}
