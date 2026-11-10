//! Handoff submission binds current native agent/attempt and report bytes.
//! Stored consistency is not actor/model authentication or independent acceptance.
use crate::{
    model_wire::{Fields, Kind},
    AgentLaunchError as E, Work,
};
use serde::{Deserialize, Serialize};
/// Native version1 submission request. The command derives submitted status;
/// no caller origin/actor/conductor grant or arbitrary Task patch is accepted.
/// Private fields keep consumers on the checked decoder and canonical intent.
#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
pub struct TaskReportRequest {
    pub(crate) schema_version: i64,
    pub(crate) event_id: String,
    pub(crate) expected_revision: i64,
    pub(crate) id: String,
    pub(crate) agent_id: String,
    pub(crate) report_path: String,
    pub(crate) report_sha256: String,
    pub(crate) status: String,
}
/// Command fields use the common scalar/null/case/merge projection; this is a
/// native envelope around Go's report operation, not a serialized actor claim.
fn shape() -> Fields {
    &[
        ("schema_version", Kind::Int64),
        ("event_id", Kind::String),
        ("expected_revision", Kind::Int64),
        ("id", Kind::String),
        ("agent_id", Kind::String),
        ("report_path", Kind::String),
        ("report_sha256", Kind::String),
        ("status", Kind::String),
    ]
}
/// Strictly decode one report request. Require valid IDs/revision/hash, a relative
/// nonempty terminal-safe path and only empty/submitted status. Unknown fields,
/// types/multiple JSON and origin/actor assertions refuse with retained causes.
/// Normalizing status supplies the existing task.submit active-agent guard;
/// it grants no execution/review identity. No IO or state changes occur.
pub fn decode_task_report_request(raw: &[u8]) -> Result<TaskReportRequest, E> {
    let mut r: TaskReportRequest = crate::work::decode_mode(raw, shape(), true)
        .map_err(|e| E::caused("task-report-request-invalid", e))?;
    if r.schema_version != 1
        || r.expected_revision < 1
        || [&r.event_id, &r.id, &r.agent_id]
            .iter()
            .any(|v| !swarm_identity_contract::name(v.as_bytes()))
        || !crate::provider_configuration::configuration_digest_valid(&r.report_sha256)
        || r.report_path.trim().is_empty()
        || std::path::Path::new(&r.report_path).is_absolute()
        || crate::agent_terminal_text::terminal_text(&r.report_path) != r.report_path
        || !matches!(r.status.as_str(), "" | "submitted")
    {
        return Err(E::Refused("task-report-request-invalid"));
    }
    r.status = "submitted".into();
    Ok(r)
}
/// Re-read the actual worker and normalized ownership for this task's latest
/// completed attempt. Shared by submission and later validation preparation;
/// callers own task status, authorization, transaction and report-byte guards.
/// Returns the stored producer without mutating it or granting execution/review
/// identity. Missing, stale, non-native or incomplete records refuse with typed
/// causes; absence of process evidence never becomes a successful completion.
pub(crate) fn completed_producer(
    db: &rusqlite::Connection,
    work_id: &str,
    task: &crate::Task,
    agent_id: &str,
) -> Result<crate::Agent, E> {
    let agent =
        crate::read_agent(db, agent_id).map_err(|e| E::caused("task-report-agent-invalid", e))?;
    crate::agent_store::verify_agent_ownership(db, &agent)
        .map_err(|e| E::caused("task-report-agent-ownership", e))?;
    let attempt = task
        .attempts
        .as_ref()
        .and_then(|a| a.last())
        .ok_or(E::Refused("task-report-completed-attempt-required"))?;
    if agent.work_id != work_id
        || agent.task_id != task.id
        || agent.role != "worker"
        || agent.status != "completed"
        || !agent.desired.is_empty()
        || agent.exit_code.as_deref() != Some(&0)
        || agent.ended.is_empty()
        || agent.attempt_id != attempt.id
        || attempt.status != "completed"
        || attempt.ended.is_empty()
    {
        return Err(E::Refused("task-report-current-completed-agent-required"));
    }
    let snapshot = agent
        .native_execution
        .as_ref()
        .ok_or(E::Refused("task-report-native-agent-required"))?;
    let snapshot = crate::decode_agent_execution_snapshot(snapshot.0.get().as_bytes())
        .map_err(|e| E::caused("task-report-native-snapshot-invalid", e))?;
    if snapshot.phase != crate::AgentExecutionPhase::PermissionRecorded
        || snapshot.context_sha256
            != crate::provider_configuration::configuration_digest(agent.prompt.as_bytes())
        || agent.native_execution_result.is_none()
    {
        return Err(E::Refused("task-report-native-completion-inconsistent"));
    }
    Ok(agent)
}
/// Stage a manual or scoped operator handoff on a private Work candidate after replay/revision and
/// active-agent guards. Require one current completed native worker attempt and
/// consistent persisted ownership/snapshot; no synthetic attempt or budget edit.
/// Read the real report under saved review-delivery policy, verify exact bytes
/// and revalidation freshness. Scoped submission signals its owning supervisor;
/// managed repository/review/recovery branches, empty/unsafe/
/// changed report and malformed/stale records refuse; original causes survive.
/// This observes report bytes, never authenticates its author/model or accepts
/// quality. Caller owns transaction, event, rollback and later operator/gate choice.
pub(crate) fn apply<Ev: crate::WorkEnvironment>(
    db: &rusqlite::Connection,
    root: &std::path::Path,
    w: &mut Work,
    r: &TaskReportRequest,
    settings: &crate::EngineSettingsDocument,
    environment: &mut Ev,
) -> Result<(), E> {
    if w.planning.is_some() {
        crate::validate_planning_state(w).map_err(|e| E::caused("task-report-scope-invalid", e))?;
    }
    let mut indices = w
        .tasks
        .iter()
        .flatten()
        .enumerate()
        .filter(|(_, t)| t.id == r.id)
        .map(|(i, _)| i);
    let i = indices
        .next()
        .ok_or(E::Refused("task-report-task-missing"))?;
    if indices.next().is_some() {
        return Err(E::Refused("task-report-task-duplicate"));
    }
    let task = &w
        .tasks
        .as_ref()
        .ok_or(E::Refused("task-report-task-missing"))?[i];
    if !matches!(task.status.as_str(), "blocked" | "todo") {
        return Err(E::Refused("task-report-status-refused"));
    }
    if task.independent_review.is_some()
        || task.corrective_recovery.is_some()
        || task.recovered_result.is_some()
        || task.review_coordination.is_some()
        || task.batch_review_resume.is_some()
    {
        return Err(E::Refused("task-report-review-recovery-unported"));
    }
    let agent = completed_producer(db, &w.id, task, &r.agent_id)?;
    if w.planning.as_deref().is_some_and(|p|p.repository.is_some()) {
        crate::managed_repository::verify_worker(root, db, w, &agent, settings, crate::managed_repository::WorkerCheck::Observe)?;
        let retained = crate::managed_repository::verify_result(root, w, &agent, settings)?;
        if retained.report_path != r.report_path || retained.report_sha256 != r.report_sha256 {
            return Err(E::Refused("task-report-managed-result-mismatch"));
        }
    }
    let limits = settings
        .values
        .context_policy_limits()
        .map_err(|e| E::caused(e.reason(), e))?;
    let report = crate::read_review_delivery(root, &r.report_path, &limits)
        .map_err(|e| E::caused("task-report-file-invalid", e))?;
    if report.trim().is_empty() {
        return Err(E::Refused("task-report-empty"));
    }
    let actual = crate::provider_configuration::configuration_digest(report.as_bytes());
    if actual != r.report_sha256 {
        return Err(E::Refused("task-report-digest-changed"));
    }
    if task
        .revalidation
        .as_ref()
        .and_then(|v| v.previous_artifacts.as_ref())
        .is_some_and(|a| a.values().any(|h| h == &actual))
    {
        return Err(E::Refused("task-report-revalidation-old-content"));
    }
    let task = &mut w
        .tasks
        .as_mut()
        .ok_or(E::Refused("task-report-task-missing"))?[i];
    if let Some(revalidation) = &mut task.revalidation {
        revalidation.report = r.report_path.clone();
        revalidation.report_hash = actual;
    }
    task.status = "submitted".into();
    task.blocker.clear();
    task.next = format!(
        "Évaluer les preuves et la gate delivery ; handoff : {}",
        r.report_path
    );
    if w.planning.is_some() {
        let at = environment
            .timestamp()
            .map_err(|e| E::caused("task-report-clock", e))?;
        crate::agent_scope_signals::handoff(
            w,
            &r.id,
            &agent.attempt_id,
            crate::ExchangeArtifact {
                path: r.report_path.clone(),
                sha256: r.report_sha256.clone(),
            },
            &at,
        )?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    /// A native explicit report command; content/ownership is checked at Root.
    fn request() -> serde_json::Value {
        serde_json::json!({"schema_version":1,"event_id":"submit","expected_revision":4,"id":"task","agent_id":"session","report_path":"handoff.md","report_sha256":"a".repeat(64)})
    }
    /// Canonical status cannot be used to skip the existing active-agent guard;
    /// scalar null/case behavior follows the shared command projection.
    #[test]
    fn task_report_request_is_strict_and_status_is_derived() {
        let raw = serde_json::to_vec(&request()).unwrap();
        assert_eq!(
            decode_task_report_request(&raw).unwrap().status,
            "submitted"
        );
        let raw = String::from_utf8(raw)
            .unwrap()
            .replace("\"schema_version\"", "\"SCHEMA_VERSION\"");
        assert!(decode_task_report_request(raw.as_bytes()).is_ok());
        for patch in [
            serde_json::json!({"origin":"conductor"}),
            serde_json::json!({"actor":"claimed"}),
            serde_json::json!({"status":"accepted"}),
            serde_json::json!({"report_path":"/outside.md"}),
            serde_json::json!({"report_path":"bad\npath"}),
            serde_json::json!({"report_sha256":"bad"}),
            serde_json::json!({"expected_revision":0}),
        ] {
            let mut r = request();
            r.as_object_mut()
                .unwrap()
                .extend(patch.as_object().unwrap().clone());
            assert!(decode_task_report_request(&serde_json::to_vec(&r).unwrap()).is_err());
        }
    }
}
