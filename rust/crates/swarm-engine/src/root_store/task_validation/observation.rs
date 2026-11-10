//! Authoritative submission observation shared by ordinary controls and the
//! explicit managed candidate stage. No Git or control process runs here.
use super::{source_authority, submission, Request, ReviewSource, E};
use serde::Serialize;
/// Internal routing purpose, never deserializable or an execution permit.
pub(super) enum RepositoryPurpose {
    Ordinary,
    Candidate,
}
/// Actual SQL/settings/report observations; caller JSON cannot construct this
/// executor input. Serialization supports interval freshness checks only.
#[derive(Serialize)]
pub(super) struct Observation {
    pub(super) work: crate::Work,
    pub(super) task: crate::Task,
    pub(super) producer: crate::Agent,
    pub(super) submitted: crate::TaskReportRequest,
    pub(super) policy: crate::ValidationPolicy,
    pub(super) normalized: crate::ValidationPolicy,
    pub(super) scope_generation: Option<crate::SettingCount>,
    pub(super) authority: Option<source_authority::Authority>,
}
/// Read actual completed producer, canonical submission, active authority, saved
/// per-task policy and report hash under the caller's snapshot. Candidate purpose
/// admits only managed work; RecordedOpinion may observe the exact blocked
/// changes-requested task without any retry admission. Ordinary/reviewer purpose keeps its original managed
/// refusal. No Git, command, grant, mutation or candidate path is touched. Errors
/// retain their existing typed causes; observations are not future authority.
pub(super) fn observe_submission(
    db: &rusqlite::Connection,
    root: &std::path::Path,
    settings: &crate::EngineSettingsDocument,
    work: &str,
    r: &Request,
    review_source: ReviewSource<'_>,
    repository: RepositoryPurpose,
) -> Result<Observation, E> {
    super::super::require_planning_mission_active(db, work)
        .map_err(|e| E::caused("task-validation-mission-inactive", e))?;
    let w =
        crate::read_recorded_work(db, work).map_err(|e| E::caused("task-validation-work", e))?;
    if w.revision != r.expected_revision {
        return Err(E::Refused("task-validation-revision-conflict"));
    }
    if let Some(planning) = w.planning.as_deref() {
        if planning.repository.is_some() && matches!(repository, RepositoryPurpose::Ordinary) {
            return Err(E::Refused("task-validation-managed-repository-unported"));
        }
        if planning.paused {
            return Err(E::Refused("task-validation-planning-paused"));
        }
        if !planning.failure.is_empty() {
            return Err(E::Refused("task-validation-planning-stopped"));
        }
        crate::validate_mission_organization(&w)
            .map_err(|e| E::caused("task-validation-organization", e))?;
    }
    if matches!(repository, RepositoryPurpose::Candidate)
        && !w
            .planning
            .as_deref()
            .is_some_and(|p| p.repository.is_some())
    {
        return Err(E::Refused("task-validation-managed-repository-required"));
    }
    let mut matches = w.tasks.iter().flatten().filter(|t| t.id == r.task_id);
    let task = matches
        .next()
        .ok_or(E::Refused("task-validation-task-missing"))?;
    if matches.next().is_some() {
        return Err(E::Refused("task-validation-task-duplicate"));
    }
    // A recorded adverse opinion has already blocked this same submitted
    // producer. Only readonly source observation may inspect that exact record;
    // fresh controls, new reviewer claims and retry authority stay separate.
    let adverse_observation = task.status == "blocked"
        && matches!(&review_source, ReviewSource::RecordedOpinion(r) if r.state == "changes_requested");
    if task.status != "submitted" && !adverse_observation {
        return Err(E::Refused("task-validation-submitted-required"));
    }
    if task.independent_review.as_deref() != review_source.record()
        || !review_source.applicable()
        || task.corrective_recovery.is_some()
        || task.recovered_result.is_some()
        || task.review_coordination.is_some()
        || task.batch_review_resume.is_some()
    {
        return Err(E::Refused("task-validation-review-recovery-unported"));
    }
    let (scope_generation, authority) = if let Some(planning) = w.planning.as_deref() {
        let scope = planning
            .scopes
            .iter()
            .flatten()
            .find(|v| v.id == task.scope_id)
            .ok_or(E::Refused("task-validation-scope-inactive"))?;
        if scope.state == "closed" {
            return Err(E::Refused("task-validation-scope-inactive"));
        }
        (
            Some(crate::SettingCount(scope.generation)),
            Some(source_authority::read(db, &w)?),
        )
    } else {
        if !task.scope_id.is_empty() || !task.plan_role.is_empty() {
            return Err(E::Refused(
                "task-validation-ordinary-task-scope-inconsistent",
            ));
        }
        (None, None)
    };
    let submitted = submission(db, &w, &task.id)?;
    let producer = crate::task_report::completed_producer(db, &w.id, task, &submitted.agent_id)?;
    let policy = task
        .validation_policy
        .as_deref()
        .ok_or(E::Refused("task-validation-policy-required"))?;
    if policy.authorized_at.is_empty() || policy.actor.is_empty() {
        return Err(E::Refused("task-validation-policy-unauthorized"));
    }
    let limits = settings
        .values
        .mutation_limits()
        .map_err(|e| E::caused("task-validation-policy-settings", e))?;
    let normalized = crate::normalize_validation_policy(policy.clone(), &limits.policy)
        .map_err(|e| E::caused("task-validation-policy-invalid", e))?;
    crate::validation_policy_covers_task(&normalized, task)
        .map_err(|e| E::caused("task-validation-coverage", e))?;
    normalized
        .controls
        .as_deref()
        .filter(|c| !c.is_empty())
        .ok_or(E::Refused("task-validation-controls-required"))?;
    let context = settings
        .values
        .context_policy_limits()
        .map_err(|e| E::caused("task-validation-context-policy", e))?;
    let report = crate::read_review_delivery(root, &submitted.report_path, &context)
        .map_err(|e| E::caused("task-validation-report", e))?;
    if report.trim().is_empty()
        || crate::provider_configuration::configuration_digest(report.as_bytes())
            != submitted.report_sha256
    {
        return Err(E::Refused("task-validation-report-changed"));
    }
    Ok(Observation {
        task: task.clone(),
        policy: policy.clone(),
        normalized,
        submitted,
        producer,
        scope_generation,
        authority,
        work: w,
    })
}
