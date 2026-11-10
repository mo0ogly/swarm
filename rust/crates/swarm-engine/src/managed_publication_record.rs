//! Canonical native publication SQL receipts; historical attribution is no grant.
use crate::{AgentLaunchError as E, Work};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
mod history;
pub(crate) use history::chain;
/// Exact immutable predecessor publication, not a caller-provided acceptance.
/// The canonical event body is rehashed when traversing the SQL chain.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PreviousPublication {
    pub event_id: String,
    pub sha256: String,
}
/// Engine-generated intent and original evidence coordinates. Strict decoding and
/// canonical bytes exclude unknown fields; hashes bind data, not actor identity.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Record {
    pub schema_version: i64,
    pub event_id: String,
    pub expected_revision: i64,
    pub work_id: String,
    pub task_id: String,
    pub producer: String,
    pub attempt: String,
    pub result_commit: String,
    pub candidate_commit: String,
    pub previous_candidate: String,
    /// Native v2 separates the worker's original base from the merge parent.
    /// None is retained byte-for-byte only for initial native v1 publications.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_base: Option<String>,
    /// Required for v2 cumulative publications; absent in initial v1 receipts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_publication: Option<PreviousPublication>,
    pub review_id: String,
    pub original_sha256: String,
    pub context_sha256: String,
    pub validation_sha256: String,
    pub binding_sha256: String,
    pub cohort_sha256: String,
    pub accepted_task_sha256: String,
    pub settings_sha256: String,
    pub accepted_work: Work,
}
impl Record {
    /// Borrow the original worker base under the already checked wire version.
    /// Initial v1 has equal base/parent; v2 must store its distinct base. This
    /// performs no IO and never infers missing cumulative provenance.
    pub(crate) fn worker_base(&self) -> Result<&str, E> {
        match (self.schema_version, &self.worker_base, &self.previous_publication) {
            (1, None, None) => Ok(&self.previous_candidate),
            (2, Some(base), Some(_)) if !base.is_empty() => Ok(base),
            _ => Err(E::Refused("managed-publication-record-version")),
        }
    }
}
/// Hash canonical model bytes, preserving codec causes. No IO or authority.
pub(crate) fn digest(value: &impl Serialize) -> Result<String, E> {
    Ok(crate::provider_configuration::configuration_digest(&crate::encode_model(value)
        .map_err(|e| E::caused("managed-publication-encode", e))?))
}
/// Read one actual publication and verify canonical request plus its recorded
/// accepted Work payload. Optional absence supports first invocation only;
/// malformed or colliding rows refuse. This proves recorded state, not freshness.
pub(crate) fn read(db: &Connection, event: &str) -> Result<Option<Record>, E> {
    crate::agent_launch::query(db,
        "SELECT work_id,revision,kind,request,payload FROM main.events WHERE id=?", [event], |rows| {
        let Some(row) = rows.next()? else { return Ok(None); };
        let work_id: String = row.get(0)?; let revision: i64 = row.get(1)?;
        let kind: String = row.get(2)?; let raw: Vec<u8> = row.get(3)?; let payload: Vec<u8> = row.get(4)?;
        if rows.next()?.is_some() || kind != "managed.publication" { return Err(E::Refused("managed-publication-event-conflict")); }
        let record: Record = serde_json::from_slice(&raw).map_err(|e| E::caused("managed-publication-record-format",e))?;
        record.worker_base()?;
        if crate::encode_model(&record).map_err(|e| E::caused("managed-publication-record-encode",e))? != raw
            || record.event_id != event || record.work_id != work_id
            || record.expected_revision.checked_add(1) != Some(revision) {
            return Err(E::Refused("managed-publication-record-binding"));
        }
        // RecordedChange intentionally stores the same canonical command in
        // request and payload; the publication command owns its Work preimage.
        if payload != raw || record.accepted_work.id != work_id || record.accepted_work.revision != revision {
            return Err(E::Refused("managed-publication-record-payload"));
        }
        matches_current(&record, &record.accepted_work)?;
        Ok(Some(record))
    })
}
/// Match a receipt to its own recorded publication snapshot. V1 remains initial;
/// v2's preceding chain is verified separately. No current-source authority.
pub(crate) fn matches_current(record: &Record, work: &Work) -> Result<(), E> {
    matches_task(record, work)?;
    let repo = work.planning.as_deref().and_then(|p| p.repository.as_deref())
        .ok_or(E::Refused("managed-publication-repository-required"))?;
    if repo.candidate_commit != record.candidate_commit
        || (record.schema_version == 1 && repo.base_commit != record.previous_candidate) {
        return Err(E::Refused("managed-publication-current-binding"));
    }
    Ok(())
}
/// Preserve one exact accepted task and original review across later publications.
/// This deliberately does not assert the historical candidate is current: callers
/// must verify the complete canonical chain and latest whole-cohort originals.
fn matches_task(record: &Record, work: &Work) -> Result<(), E> {
    let mut tasks = work.tasks.iter().flatten().filter(|t| t.id == record.task_id);
    let task = tasks.next().ok_or(E::Refused("managed-publication-task-required"))?;
    let review = task.independent_review.as_deref().ok_or(E::Refused("managed-publication-review-required"))?;
    let attempt = task.attempts.iter().flatten().last().ok_or(E::Refused("managed-publication-attempt-required"))?;
    let repo = work.planning.as_deref().and_then(|p| p.repository.as_deref())
        .ok_or(E::Refused("managed-publication-repository-required"))?;
    if tasks.next().is_some() || work.id != record.work_id || work.revision <= record.expected_revision
        || task.status != "accepted" || digest(task)? != record.accepted_task_sha256
        || record.accepted_work.planning.as_deref().and_then(|p|p.repository.as_deref())
            .is_none_or(|original|original.base_commit != repo.base_commit)
        || review.id != record.review_id || review.state != "passed" || review.producer != record.producer
        || review.candidate_commit != record.candidate_commit || review.previous_candidate != record.previous_candidate
        || review.context_sha256 != record.context_sha256 || attempt.id != record.attempt || attempt.status != "completed" {
        return Err(E::Refused("managed-publication-current-binding"));
    }
    Ok(())
}
/// Find the canonical receipt of an integrated worker row. This read never grants
/// execution: physical worker admission still requires its original ready state.
/// Row attribution, payload and current accepted state must all agree.
pub(crate) fn integrated(db: &Connection, work: &Work, agent: &str, task: &str, base: &str, result: &str, event: &str) -> Result<Record, E> {
    let record = chain(db,work)?.into_iter().find(|r|r.event_id == event)
        .ok_or(E::Refused("managed-publication-record-required"))?;
    matches_task(&record,work)?;
    if record.producer != agent || record.task_id != task || record.worker_base()? != base || record.result_commit != result {
        return Err(E::Refused("managed-publication-worker-binding"));
    }
    // Require the normalized row as well; a payload alone never establishes settlement.
    let exact: bool = crate::agent_launch::query(db,
        "SELECT 1 FROM main.managed_attempts WHERE agent_id=? AND work_id=? AND task_id=? AND base_commit=? AND state='integrated' AND result_commit=? AND detail=?",
        params![agent,work.id,task,base,result,event], |rows| Ok(rows.next()?.is_some()))?;
    if !exact { return Err(E::Refused("managed-publication-worker-row")); }
    Ok(record)
}
