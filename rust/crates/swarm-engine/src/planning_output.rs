//! Planning-domain validation over the shared complete provider response stream.
//! Existing public behavior/error categories remain; no execution authority.
use crate::{
    AgentLaunchError as E, EngineSettingsValues, PlanningDecisionPolicy, PlanningProposal,
    ProviderResponseStream,
};
use std::io::{self, Write};
/// Complete decoded data plus explicitly unverified provider declarations.
/// Missing/invalid usage stays absent; it is never a zero price or invoice.
/// A valid proposal still needs live delivery/lease/permission checks and cannot
/// authorize an action, refund, process start or business acceptance.
pub struct PlanningProviderOutput {
    pub proposal: PlanningProposal,
    /// Complete validated original proposal bytes for a private durable receipt.
    /// Never authority or a public prompt; caller preserves its bound claim.
    pub(crate) raw_proposal: Vec<u8>,
    pub usage: Option<crate::Usage>,
    pub reported_model: Option<crate::ReportedModel>,
    pub provider_facts: crate::GoRaw,
}
/// Planning response adapter. Common framing/terminal/capture facts are separate
/// from persisted PlanningProposal limits; private fields prevent bypass by DTO.
pub struct PlanningOutputStream {
    stream: ProviderResponseStream,
    decision_policy: PlanningDecisionPolicy,
}
impl PlanningOutputStream {
    /// Read validated saved planning response/decision policy and common event
    /// caps and forbid known tool declarations for this tool-free role. Missing
    /// historical policy refuses; no defaults, IO, confinement or grant occurs.
    pub fn new(values: &EngineSettingsValues) -> Result<Self, E> {
        values.validate().map_err(|e| E::caused(e.reason(), e))?;
        let cap = values
            .planning_response_limit()
            .map_err(|e| E::caused(e.reason(), e))?;
        let stream = ProviderResponseStream::new_tool_free(values, cap)?;
        let decision_policy = values
            .planning_decision_policy()
            .map_err(|e| E::caused(e.reason(), e))?;
        Ok(Self {
            stream,
            decision_policy,
        })
    }
    /// Feed exact bytes through the shared parser, retaining typed causes and
    /// failure latch. This is no EOF, decision application or provider admission.
    pub fn feed(&mut self, raw: &[u8]) -> Result<(), E> {
        self.stream.feed(raw)
    }
    /// Observe unverified usage/model facts without consuming the reply. Failed
    /// or incomplete channels stay distinct; no bill or identity is inferred.
    pub(crate) fn facts_snapshot(&self) -> crate::agent_provider_facts::ProviderFactsSnapshot {
        self.stream.facts_snapshot()
    }
    /// Latch an owner-observed IO/drain failure. A later finish cannot succeed.
    pub(crate) fn incomplete_transport(&mut self) {
        self.stream.incomplete_transport()
    }
    /// Serialize current facts without IO or policy/financial mutation. Encoding
    /// failures retain causes; facts are provider declarations, not authority.
    pub fn provider_facts(&self) -> Result<crate::GoRaw, E> {
        self.stream.provider_facts()
    }
    /// Complete once after owner-observed EOF and strictly decode the original
    /// proposal before transferring bytes. Previous errors/missing terminal,
    /// invalid/duplicate fields or planning policy refusal remain errors; original
    /// bytes/facts remain in the parser on decode failure. No execution, decision
    /// application, measured cost or task acceptance occurs.
    pub fn finish(&mut self) -> Result<PlanningProviderOutput, E> {
        let policy = &self.decision_policy;
        let (raw, proposal) = self
            .stream
            .finish_decoded(|bytes| crate::decode_planning_proposal(bytes, policy))?;
        Ok(PlanningProviderOutput {
            proposal,
            raw_proposal: raw.response,
            usage: raw.usage,
            reported_model: raw.reported_model,
            provider_facts: raw.provider_facts,
        })
    }
}
impl Write for PlanningOutputStream {
    /// Preserve the exact common Write/cleanup cause and byte count.
    fn write(&mut self, raw: &[u8]) -> io::Result<usize> {
        self.stream.write(raw)
    }
    /// Memory flush is never EOF or permission and preserves a failed latch.
    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}
/// Offline parser observation, not lease/delivery/operation authorization.
/// Private response text is omitted. Explicit capability flags prevent this
/// receipt from being presented as a process start or applied decision.
#[derive(serde::Serialize)]
pub struct PlanningOutputCheck {
    pub schema_version: i64,
    pub proposal_valid: bool,
    pub input_count: usize,
    pub operation_count: usize,
    pub reason_bytes: usize,
    pub provider_facts: crate::GoRaw,
    pub process_started: bool,
    pub decision_applied: bool,
}
