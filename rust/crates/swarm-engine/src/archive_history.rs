//! Inert imported cockpit history, never a current agent or import grant.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

fn empty<T>(v: &Option<Vec<T>>) -> bool {
    v.as_ref().is_none_or(Vec::is_empty)
}

/// Historical decision data. Names, author and evidence are supplied archive
/// text, not authenticated authorship, fresh validation or a current decision.
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ArchivedDecision {
    pub id: String,
    pub task_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub agent_id: String,
    pub kind: String,
    pub summary: String,
    pub evidence: String,
    pub created: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub author: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub resolved_at: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub resolution: String,
}
/// Archived bookmark, not a current local operator consultation or identity.
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ArchivedVisit {
    pub revision: i64,
    pub at: String,
    pub operator: String,
}
/// Archived log row. Sequence and text are retained data, not live telemetry.
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ArchivedAgentLog {
    pub seq: i64,
    pub agent_id: String,
    pub at: String,
    pub kind: String,
    pub message: String,
}
/// Go cockpit_history output, preserving nil/empty collections and omitempty.
/// Recorded Agent PIDs/statuses/metadata are never observed, adopted or signalled;
/// this type is only returned inside the explicitly historical wrapper. Controls
/// and decisions are never replayed. Imported evidence is not checked here.
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ArchivedCockpitHistory {
    #[serde(skip_serializing_if = "empty")]
    pub decisions: Option<Vec<ArchivedDecision>>,
    #[serde(skip_serializing_if = "empty")]
    pub visits: Option<Vec<ArchivedVisit>>,
    pub agents: Option<Vec<crate::Agent>>,
    pub logs: Option<Vec<ArchivedAgentLog>>,
    pub controls: Option<Vec<Option<BTreeMap<String, String>>>>,
    pub limits: String,
}
/// A read-only historical projection. It cannot be deserialized as an operation;
/// history=None represents the original absent/null Cockpit pointer. No schema,
/// signature, archive blob, event-chain, model, cost or current authority proof.
#[derive(Serialize)]
pub struct ArchivedAgentHistory {
    historical_only: bool,
    pub history: Option<ArchivedCockpitHistory>,
}
#[derive(Default, Deserialize)]
#[serde(default)]
struct BundleProjection {
    cockpit_history: Option<ArchivedCockpitHistory>,
}

/// Validate the full frozen Bundle JSON type graph, then return only Cockpit.
/// Unknown fields and raw invalid UTF8 follow Go replacement/merge semantics;
/// malformed JSON and known types (even unused Work/assistant fields) refuse.
/// Native-only Agent metadata is removed by the historical graph. No admission
/// checks, logical schema/revision validation, IO, mutation or process observation
/// occur. Memory follows input size; no hidden archive/read ceiling is imposed.
pub(crate) fn decode(raw: &[u8]) -> Result<ArchivedAgentHistory, crate::WorkError> {
    let text = crate::text::go_utf8(raw);
    let b: BundleProjection = crate::work::decode(
        text.as_bytes(),
        crate::archive_history_shapes::shape_bundle(),
    )?;
    Ok(ArchivedAgentHistory {
        historical_only: true,
        history: b.cockpit_history,
    })
}
