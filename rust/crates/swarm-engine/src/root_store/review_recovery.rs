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
/// Internal purpose of one recovery observation. It selects only which
/// repository rule applies and whether the original artifact set is re-read
/// from real Root paths inside this observation or deferred to the consuming
/// assembled-observation equality check in review_preview; it is never a
/// public boolean, operator policy, execution mode or admission source.
#[derive(Clone, Copy)]
enum Observation {
    /// Ordinary Work: planning.repository must be absent, and every original
    /// artifact is re-read from real Root paths inside this observation.
    Ordinary,
    /// Managed-only observation: planning.repository must be present, and the
    /// recorded artifact names are Material data aliases (cohort/...,
    /// candidate/...), so the filesystem re-read is deferred to the consuming
    /// equality check against actually assembled observations.
    ManagedObservation,
}
impl RootStore {
    /// Observe a current failed review for a new call, or the immediate archived
    /// same-attempt error behind an already checked running job. Requires actual
    /// stored Work/SQL snapshot, typed original failure, original control chain,
    /// unchanged report/contract/project and exact canonical correction/input ack.
    /// Ordinary entry: requires planning.repository to be absent and re-reads
    /// every original artifact from real Root paths inside this observation.
    /// None denotes an ordinary first-review path, never a relaxed error fallback.
    /// Reads create no claim, debit, file, signal or acceptance; IO/codec causes
    /// remain failures. Caller independently enforces fresh funding/host/source.
    pub(super) fn review_recovery(&self, tx: &Transaction<'_>, work: &Work, task_id: &str, claimed: Option<&IndependentReview>, settings: &EngineSettingsDocument) -> Result<Option<Recovery>, E> {
        self.review_recovery_observation(tx, work, task_id, claimed, settings, Observation::Ordinary)
    }
    /// Managed-only recovery observation for the held-candidate retry route.
    /// Takes the same current Work/task/claimed/settings as the ordinary entry
    /// and runs the same shared core, but requires planning.repository Some
    /// instead of the ordinary exclusion and defers exactly one obligation:
    /// the per-report_artifacts filesystem re-read, because those recorded
    /// names are Material-produced data aliases (cohort/..., candidate/...),
    /// never real Root paths. Everything else is observed here as read-only
    /// data in the caller's current SQL/settings snapshot, acquiring no
    /// custody or authority: stored-Work equality, task uniqueness, the
    /// current qualified error (claimed=None) or the immediate same-attempt
    /// archived error behind an already checked running job (claimed=Some,
    /// never claimed=None only), attempt/status/launch-hold/planning-retry
    /// guards, history-conflict refusal, typed recorded_native_review failure
    /// with original error provenance, original Root job/certificate protocol,
    /// canonical planning.decide review-retry verification with the actual
    /// input acknowledgement, exact inbox decision, review_contract equality,
    /// project context freshness, current instruction-policy bound and
    /// recorded_evidence_for_review_result over the original native control
    /// receipt (legitimate Root metadata from the supplied implementation,
    /// kept unchanged; not a managed repository source). Returns cloneable
    /// Recovery DATA only, never a permission, candidate substitute, execution
    /// grant or admission source; reads create no claim, debit, file, signal
    /// or acceptance and IO/SQL/codec causes remain failures. Consuming
    /// obligations NOT discharged here: original managed artifact freshness,
    /// which review_preview::assemble_review_inputs_using must check by
    /// rebuilding the full Material map from actually assembled observations
    /// and requiring exact equality with previous.report_artifacts before
    /// returning inputs or staging any claim, in addition to the held
    /// candidate's full Captured/bound_source preview checks and the
    /// assembler's actual source reads and SHA comparisons; and every shared
    /// decision/history check that a later private gate, process or
    /// publication must re-observe. None denotes no qualified original, never
    /// a relaxed fallback. This entry alone unlocks no public early admission.
    pub(super) fn managed_review_recovery(&self, tx: &Transaction<'_>, work: &Work, task_id: &str, claimed: Option<&IndependentReview>, settings: &EngineSettingsDocument) -> Result<Option<Recovery>, E> {
        self.review_recovery_observation(tx, work, task_id, claimed, settings, Observation::ManagedObservation)
    }
    /// Shared observation core behind both entries; each named entry fixes one
    /// internal purpose and no caller selects a purpose. Every original guard
    /// is retained for both purposes: actual stored Work and task uniqueness,
    /// same producer/attempt/status/history, typed recorded_native_review
    /// qualified failure, original Root job/certificate, canonical
    /// planning.decide review-retry with actual input acknowledgement, exact
    /// inbox decision, review_contract, project/workflow freshness, current
    /// instruction-policy bound, recorded_evidence_for_review_result and typed
    /// IO/SQL/codec causes, inside the caller's non-autocommit transaction
    /// whose rollback/close remain the caller's paired obligations. The purpose
    /// selects only the repository rule and whether the original artifact set
    /// is re-read from real Root paths here (ordinary) or deferred to the
    /// consuming assembled-observation equality check (managed observation).
    fn review_recovery_observation(&self, tx: &Transaction<'_>, work: &Work, task_id: &str, claimed: Option<&IndependentReview>, settings: &EngineSettingsDocument, observation: Observation) -> Result<Option<Recovery>, E> {
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
        match observation {
            Observation::Ordinary if planning.repository.is_some() => return Err(E::Refused("review-recovery-attempt-changed")),
            Observation::ManagedObservation if planning.repository.is_none() => return Err(E::Refused("review-recovery-managed-repository-required")),
            _ => {}
        }
        if task.plan_role != "worker" || task.status != "submitted" || task.launch_held || task.planning_retry
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
        // Managed observation defers exactly this filesystem loop: the recorded
        // names are Material data aliases, not real Root paths; the consuming
        // equality check lives in review_preview::assemble_review_inputs_using.
        if matches!(observation, Observation::Ordinary) {
            for (name, expected) in previous.report_artifacts.iter().flatten().chain(std::iter::once((&previous.report, &previous.sha256))) {
                let (_, actual) = crate::gate_evaluation::artifact_hash(&self.root, name).map_err(|e| caused("review-recovery-artifact-read", e))?;
                if &actual != expected { return Err(E::Refused("review-recovery-artifact-changed")); }
            }
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
