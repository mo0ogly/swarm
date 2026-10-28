//! Read-only active Agent classification for historical status, never authority.
//! Host/PID/starttime and the frozen heartbeat policy are observations of the
//! supervisor, not provider progress, actor authentication or signal permission.
use super::RootStoreError as E;
use crate::Agent;

/// Observe each row in original order, preserving terminal states verbatim.
/// Active rows require this host, a recorded live supervisor startstamp and a
/// recent heartbeat under the execution's saved policy. Missing legacy metadata,
/// remote host, stale/future clocks or a lost supervisor stay explicitly uncertain.
/// Kernel read/close failures remain in `causes`; malformed heartbeat/snapshot
/// refuses with its original typed cause. No defaults, writes or process effects.
/// Sequential procfs reads are immediately stale and do not prove provider work.
pub(super) fn observe(agents: &[Agent], now: &str, causes: &mut Vec<E>) -> Result<Vec<String>, E> {
    let identity = if agents.iter().any(Agent::is_active) {
        Some(
            crate::SupervisorIdentity::current()
                .map_err(|e| E::Launch(Box::new(crate::AgentLaunchError::caused(e.code(), e))))?,
        )
    } else {
        None
    };
    agents
        .iter()
        .map(|agent| {
            if !agent.is_active() {
                return Ok(agent.status.clone());
            }
            let host = identity
                .as_ref()
                .ok_or(E::Refused("mission-status-host-unobserved"))?
                .host();
            if agent.host != host {
                return Ok("unknown/other-host".into());
            }
            let unconfirmed = || format!("{}/unconfirmed", agent.status);
            let Ok(pid) = u32::try_from(agent.supervisor_pid) else {
                return Ok(unconfirmed());
            };
            if pid == 0 || agent.supervisor_identity.is_empty() {
                return Ok(unconfirmed());
            }
            match crate::observe_linux_process(pid) {
                Ok(process) if process.process_stamp() != agent.supervisor_identity => {
                    return Ok("unknown/supervisor-lost".into())
                }
                Ok(_) => {}
                Err(error) => {
                    let state = if error.confirms_absence() {
                        "unknown/supervisor-lost"
                    } else {
                        "unknown/supervisor-unavailable"
                    };
                    causes.push(E::Launch(Box::new(crate::AgentLaunchError::caused(
                        error.code(),
                        error,
                    ))));
                    return Ok(state.into());
                }
            }
            if agent.heartbeat.is_empty() {
                return Ok(unconfirmed());
            }
            let heartbeat =
                crate::observe_agent_heartbeat(agent, now).map_err(|e| E::Launch(Box::new(e)))?;
            Ok(match heartbeat.as_ref().map(|value| value.state) {
                Some("recent") => agent.status.clone(),
                Some("future") => "unknown/future-heartbeat".into(),
                Some("stale") => "unknown/no-heartbeat".into(),
                None => unconfirmed(),
                _ => return Err(E::Refused("mission-status-active-heartbeat-state")),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Real current procfs is the valid control before stale, future, missing,
    /// mismatched and malformed observations; no SQL, signal or provider fixture.
    #[test]
    fn mission_status_active_identity_heartbeat_and_recovery() {
        let identity = crate::SupervisorIdentity::current().unwrap();
        let mut policy = crate::EngineSettingsValues::installation_defaults()
            .unwrap()
            .agent_execution_policy()
            .unwrap();
        policy.heartbeat_stale_ms = crate::SettingCount(1000);
        let snapshot = crate::AgentExecutionSnapshot {
            schema_version: 1,
            settings_revision: crate::SettingCount(1),
            work_revision: 1,
            context_sha256: "0".repeat(64),
            phase: crate::AgentExecutionPhase::PermissionRecorded,
            policy,
        };
        let mut agent = Agent {
            status: "running".into(),
            host: identity.host().into(),
            supervisor_pid: identity.pid(),
            supervisor_identity: identity.process_stamp().into(),
            heartbeat: "2026-10-23T00:00:00Z".into(),
            native_execution: Some(crate::GoRaw(
                serde_json::value::to_raw_value(&snapshot).unwrap(),
            )),
            ..Default::default()
        };
        let mut causes = Vec::new();
        let state = |agent: &Agent, now: &str, causes: &mut Vec<E>| {
            observe(std::slice::from_ref(agent), now, causes)
                .unwrap()
                .remove(0)
        };
        assert_eq!(
            state(&agent, "2026-10-23T00:00:00.999Z", &mut causes),
            "running"
        );
        assert!(causes.is_empty());
        assert_eq!(
            state(&agent, "2026-10-23T00:00:01Z", &mut causes),
            "unknown/no-heartbeat"
        );
        assert_eq!(
            state(&agent, "2026-10-22T23:59:59Z", &mut causes),
            "unknown/future-heartbeat"
        );
        let original = agent.clone();
        agent.host = "other-host".into();
        assert_eq!(state(&agent, "unused", &mut causes), "unknown/other-host");
        agent = original.clone();
        agent.supervisor_identity.push('0');
        assert_eq!(
            state(&agent, "unused", &mut causes),
            "unknown/supervisor-lost"
        );
        agent = original.clone();
        agent.supervisor_pid = 0;
        assert_eq!(state(&agent, "unused", &mut causes), "running/unconfirmed");
        agent = original.clone();
        agent.native_execution = None;
        assert_eq!(
            state(&agent, "2026-10-23T00:00:00Z", &mut causes),
            "running/unconfirmed"
        );
        agent = original.clone();
        agent.heartbeat.clear();
        assert_eq!(state(&agent, "unused", &mut causes), "running/unconfirmed");
        agent = original.clone();
        agent.heartbeat = "malformed".into();
        assert!(observe(
            std::slice::from_ref(&agent),
            "2026-10-23T00:00:00Z",
            &mut causes
        )
        .is_err());
        agent = original;
        assert_eq!(
            state(&agent, "2026-10-23T00:00:00.999Z", &mut causes),
            "running"
        );
        assert!(causes.is_empty());
    }
}
