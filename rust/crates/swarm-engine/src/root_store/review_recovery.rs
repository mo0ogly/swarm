//! Original reviewer-recovery observations; no provider, mutation or permit.
use super::{RootStore, RootStoreError as E, task_validation};
use crate::{AgentLaunchError as A, IndependentReview, Work, EngineSettingsDocument};
use rusqlite::Transaction;

/// Attributed instructions included in the real reviewer prompt. Hashes identify
/// canonical bytes, not semantic actor identity, quality or permission to exec.
/// Failure category/owner come only from the same checked private original.
#[derive(Clone, serde::Serialize)]
pub(super) struct Directive {
    pub decision: String,
    pub decision_request_sha256: String,
    pub source_event: String,
    pub review: String,
    pub original_body_sha256: String,
    /// Category and responsible role from the verified original, not free-form
    /// error text, caller input, actor authentication or an execution permit.
    pub failure_code: String,
    pub failure_owner: String,
    pub task: String,
    pub attempt: String,
    pub producer: String,
    pub report_sha256: String,
    pub instruction: String,
}
/// Read-only provenance constructed only by the Root observer below. Keeping
/// controls separate from the new admission source avoids reusing an old grant.
/// Clone copies observations only; every gate still reobserves all originals.
#[derive(Clone)]
pub(super) struct Recovery {
    pub previous: IndependentReview,
    pub validation: task_validation::NativeValidationEvidence,
    pub directive: Directive,
}
/// Keep the static safe boundary and original typed cause, never private text.
fn caused(code: &'static str, error: impl std::error::Error + Send + Sync + 'static) -> E {
    E::Launch(Box::new(A::caused(code, error)))
}
impl RootStore {
    /// Observe a current failed review for a new call, or the immediate archived
    /// same-attempt error behind an already checked running job. Requires actual
    /// stored Work/SQL snapshot, typed original failure, original control chain,
    /// unchanged report/contract/project and exact canonical correction/input ack.
    /// None denotes an ordinary first-review path, never a relaxed error fallback.
    /// Reads create no claim, debit, file, signal or acceptance; IO/codec causes
    /// remain failures. Caller independently enforces fresh funding/host/source.
    pub(super) fn review_recovery(&self, tx: &Transaction<'_>, work: &Work, task_id: &str, claimed: Option<&IndependentReview>, settings: &EngineSettingsDocument) -> Result<Option<Recovery>, E> {
        if tx.is_autocommit() { return Err(E::Refused("review-recovery-transaction-required")); }
        let stored = crate::read_recorded_work(tx, &work.id).map_err(E::Work)?;
        if &stored != work { return Err(E::Refused("review-recovery-source-changed")); }
        let mut tasks = work.tasks.iter().flatten().filter(|t| t.id == task_id);
        let task = tasks.next().ok_or(E::Refused("review-recovery-task-required"))?;
        if tasks.next().is_some() { return Err(E::Refused("review-recovery-task-ambiguous")); }
        let previous = match claimed {
            None => match task.independent_review.as_deref() {
                Some(r) if r.state == "error" && crate::review_evidence::native_receipt_reference(&r.receipt) => r,
                _ => return Ok(None),
            },
            Some(r) => {
                if r.state != "running" || task.independent_review.as_deref() != Some(r) { return Err(E::Refused("review-recovery-current-job-changed")); }
                match task.previous_reviews.iter().flatten().last() {
                    Some(old) if old.state == "error" && old.attempt == r.attempt && crate::review_evidence::native_receipt_reference(&old.receipt) => old,
                    _ => return Ok(None),
                }
            }
        };
        let planning = work.planning.as_deref().ok_or(E::Refused("review-recovery-planning-required"))?;
        if planning.repository.is_some() || task.plan_role != "worker" || task.status != "submitted" || task.launch_held || task.planning_retry
            || task.attempts.iter().flatten().last().is_none_or(|a| a.id != previous.attempt || a.status != "completed") {
            return Err(E::Refused("review-recovery-attempt-changed"));
        }
        if claimed.is_none() && task.previous_reviews.iter().flatten().any(|r| r.id == previous.id) { return Err(E::Refused("review-recovery-history-conflict")); }
        let observed = self.recorded_native_review(tx, &work.id, &previous.id)?;
        let failure = observed.retry_failure.ok_or(E::Refused("review-recovery-original-failure-required"))?;
        if observed.task != task_id || &observed.review != previous || observed.revision > work.revision {
            return Err(E::Refused("review-recovery-original-protocol-required"));
        }
        let decision = super::native_review_context::verify_review_retry_decision_for_input(tx, work, task, &previous.id, &observed.original_receipt_sha256, failure)?;
        let mut inputs = planning.inbox.iter().flatten().filter(|e| e.id == previous.id && e.task == task_id && e.scope == task.scope_id && e.attempt == previous.attempt && e.kind == "independent_review");
        let input = inputs.next().ok_or(E::Refused("review-recovery-input-required"))?;
        if inputs.next().is_some() || input.decision.is_empty() { return Err(E::Refused("review-recovery-input-ambiguous")); }
        if crate::review_contract(task).map_err(E::Encode)? != previous.contract { return Err(E::Refused("review-recovery-contract-changed")); }
        let limits = settings.values.context_policy_limits().map_err(E::Settings)?;
        crate::check_project_context_freshness(&self.root, previous.workflow.as_deref().and_then(|w| w.project.as_deref()), "reviewer", &limits)
            .map_err(|e| caused("review-recovery-project-changed", e))?;
        for (name, expected) in previous.report_artifacts.iter().flatten().chain(std::iter::once((&previous.report, &previous.sha256))) {
            let (_, actual) = crate::gate_evaluation::artifact_hash(&self.root, name).map_err(|e| caused("review-recovery-artifact-read", e))?;
            if &actual != expected { return Err(E::Refused("review-recovery-artifact-changed")); }
        }
        let validation = task_validation::recorded_evidence_for_review_result(self, work, task_id, &observed.validation_receipt_sha256)?;
        let raw: Vec<u8> = crate::agent_launch::query(tx, "SELECT request FROM events WHERE id=? AND work_id=? AND kind='planning.decide'", rusqlite::params![decision,work.id], |rows| {
            let bytes = rows.next()?.ok_or(A::Refused("review-recovery-decision-required"))?.get(0)?;
            if rows.next()?.is_some() { return Err(A::Refused("review-recovery-decision-ambiguous")); }
            Ok(bytes)
        }).map_err(|e| E::Launch(Box::new(e)))?;
        let policy = settings.values.planning_decision_policy().map_err(|e| caused("review-recovery-instruction-policy", e))?;
        crate::PlanningDecisionPolicy::check(policy.max_instruction_bytes, task.next.len(), "review-recovery-instruction-current-policy")
            .map_err(|e| E::Launch(Box::new(e)))?;
        Ok(Some(Recovery {
            previous: previous.clone(), validation,
            directive: Directive { decision, decision_request_sha256: crate::provider_configuration::configuration_digest(&raw),
                source_event: input.id.clone(), review: previous.id.clone(), original_body_sha256: observed.original_receipt_sha256,
                failure_code: failure.code().into(), failure_owner: failure.owner().into(),
                task: task.id.clone(), attempt: previous.attempt.clone(), producer: previous.producer.clone(), report_sha256: previous.sha256.clone(), instruction: task.next.clone() },
        }))
    }
}
