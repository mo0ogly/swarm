//! Read-only declared-event replay. Provider declarations are not execution proof.
use crate::{Agent, AgentGuardError, AgentGuardObservation, AgentGuardVisibility, AgentToolGuard};
use serde::{Deserialize, Serialize};
/// Strict v1 diagnostic input. Every timestamp is caller-declared monotonic
/// milliseconds, not an authenticated clock. No event count policy is invented;
/// public transport size/depth limits remain applicable. No events execute tools.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentGuardTrace {
    pub schema_version: i64,
    pub events: Vec<AgentGuardEvent>,
    pub final_at_ms: i64,
}
/// One externally tagged declaration. Required per-kind fields prevent missing
/// result flags or timestamps becoming successful zero/default observations.
/// Freeform result messages are accepted for replay only and never waive errors.
/// Raw call inputs stay opaque JSON, avoiding recursive Value construction/drop.
#[derive(Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum AgentGuardEvent {
    Output {
        at_ms: i64,
    },
    Call {
        at_ms: i64,
        id: String,
        name: String,
        input: crate::GoRaw,
    },
    Result {
        at_ms: i64,
        id: String,
        failed: bool,
        message: Option<String>,
    },
    Visibility {
        at_ms: i64,
        cause: AgentGuardVisibility,
    },
}
/// Safe final reconstruction over stored frozen values. Raw arguments/messages
/// never enter the report. Preview cannot stop a process, mutate history, reset
/// budgets or establish the authenticity of caller events or stored identities.
#[derive(Serialize)]
pub struct AgentGuardReplay {
    pub schema_version: i64,
    pub preview_only: bool,
    pub process_started: bool,
    pub event_source: &'static str,
    pub settings_revision: String,
    pub lookback_multiplier: crate::SettingCap,
    pub frozen_limits: crate::RunLimits,
    pub observation: AgentGuardObservation,
}
/// Decode strict native JSON without Go zero/default projection. Missing fields,
/// duplicate known keys, unknown keys, types and syntax refuse with typed causes;
/// schema/monotonic/attribution checks run when consumed. No IO or mutation.
pub fn decode_agent_guard_trace(raw: &[u8]) -> Result<AgentGuardTrace, AgentGuardError> {
    swarm_wire_json::decode(raw)
        .map_err(|e| AgentGuardError::caused("agent-guard-trace-invalid", e))
}
/// Reconstruct declared events with this retained Agent's frozen limits/policy.
/// Legacy absence refuses; current Admin values are never a fallback. The caller
/// must establish its storage snapshot/ownership consistency separately. Failure
/// drops only private replay state, preserving typed causes and all stored data.
pub fn replay_agent_guard(
    agent: &Agent,
    trace: &AgentGuardTrace,
) -> Result<AgentGuardReplay, AgentGuardError> {
    if trace.schema_version != 1 {
        return Err(AgentGuardError::refused(
            "agent-guard-trace-schema-unsupported",
        ));
    }
    let snapshot = agent
        .native_tool_guard_policy
        .as_deref()
        .ok_or_else(|| AgentGuardError::refused("agent-guard-snapshot-missing"))?;
    let mut guard = AgentToolGuard::new(agent.limits.clone(), snapshot)?;
    for event in &trace.events {
        match event {
            AgentGuardEvent::Output { at_ms } => guard.output(*at_ms)?,
            AgentGuardEvent::Call {
                at_ms,
                id,
                name,
                input,
            } => guard.call(*at_ms, id, name, input.0.get().as_bytes())?,
            AgentGuardEvent::Result {
                at_ms,
                id,
                failed,
                message: _,
            } => guard.result(*at_ms, id, *failed)?,
            AgentGuardEvent::Visibility { at_ms, cause } => {
                guard.lose_visibility(*at_ms, *cause)?
            }
        }
    }
    let observation = guard.observation(trace.final_at_ms)?;
    Ok(AgentGuardReplay {
        schema_version: 1,
        preview_only: true,
        process_started: false,
        event_source: "caller_declared_unverified",
        settings_revision: snapshot.settings_revision.clone(),
        lookback_multiplier: snapshot.lookback_multiplier,
        frozen_limits: agent.limits.clone(),
        observation,
    })
}
