//! Native agent wire observations; supplied metadata is never execution authority.
use crate::{agent_models, Agent, Launch, Providers, WorkError};

/// Decode a retained Go agent body, tolerating unknown historical fields.
/// Preserves JSON type/range/UTF8 failures. Performs no IO or identity checks.
pub fn decode_agent(raw: &[u8]) -> Result<Agent, WorkError> {
    crate::work::decode_mode(raw, agent_models::shape_agent(), false)
}

/// Decode a strict public launch intent. Unknown fields and malformed types
/// refuse; schema, current permissions, revision and admission still need checks.
/// No provider is invoked or reservation created by decoding.
pub fn decode_launch(raw: &[u8]) -> Result<Launch, WorkError> {
    crate::work::decode_mode(raw, agent_models::shape_launch(), true)
}

/// Decode the strict existing provider-file representation without opening it.
/// This preserves explicit nil/empty collections and does not resolve executable,
/// authentication, model route, credentials or account availability.
pub fn decode_providers(raw: &[u8]) -> Result<Providers, WorkError> {
    crate::work::decode_mode(raw, agent_models::shape_providers(), true)
}

/// Observe authoritative SQL status/desired over a retained body, matching Go.
/// A legacy unregistered claim is exposed separately and never inferred to be a
/// live supervisor. Malformed body refuses instead of returning an empty agent.
pub fn decode_agent_row(raw: &[u8], status: &str, desired: &str) -> Result<Agent, WorkError> {
    let mut agent = decode_agent(raw)?;
    agent.unregistered_claim = status == "starting"
        && agent.status == "queued"
        && agent.supervisor_pid == 0
        && agent.child_pid == 0
        && agent.heartbeat.is_empty();
    agent.status = status.into();
    agent.desired = desired.into();
    Ok(agent)
}

impl Agent {
    /// Storage lifecycle membership only, not proof of a live process or lease.
    pub fn is_active(&self) -> bool {
        matches!(
            self.status.as_str(),
            "queued" | "starting" | "running" | "stopping"
        )
    }

    /// True only for the legacy SQL-starting/body-queued registration gap.
    /// Does not authorize signalling, releasing a workspace or reconciling a PID.
    pub fn has_unregistered_claim(&self) -> bool {
        self.unregistered_claim
    }
}
