//! Read-only native control provenance and reviewer configuration observations.
//! Complete text inputs are observations; no invocation grant or publication.
pub(super) mod inputs;
mod managed_context;
mod check;
pub use check::TaskReviewCheck;
pub use inputs::{TaskReviewInputs, ReviewInputSource};
use super::{finished, task_validation, RootStore, RootStoreError as E};
use serde::Serialize;

/// One current Root snapshot with original native control evidence. Provider and
/// workflow digests describe configured data, not identity or a freshness lease.
/// False execution flags are observation facts, never operator policy defaults.
#[derive(Serialize)]
pub struct TaskReviewPreview {
    pub schema_version: i64,
    pub work_id: String,
    pub work_revision: i64,
    pub task_id: String,
    pub settings_revision: crate::SettingCount,
    pub producer_agent_id: String,
    pub attempt_id: String,
    pub validation: task_validation::NativeValidationEvidence,
    pub review_provider: String,
    pub review_provider_digest: String,
    pub review_contract_sha256: String,
    pub workflow_sha256: String,
    pub workflow_prompt_sha256: String,
    pub criterion_count: usize,
    pub remaining_review_calls: i64,
    pub native_control_evidence_verified: bool,
    pub invocation_context_materialized: bool,
    pub invocation_authorized: bool,
    pub provider_started: bool,
    pub task_accepted: bool,
}
/// Inputs retained under the same SQL/settings read snapshot. Private task text
/// has no Debug representation; this value grants no future freshness lease.
pub(super) struct PreparedReview {
    pub(super) work: crate::Work,
    pub(super) preview: TaskReviewPreview,
    pub(super) workflow: crate::AgentWorkflowObservation,
    pub(super) controls: crate::ValidationEvidenceObservation,
    pub(super) settings: crate::EngineSettingsDocument,
    /// Fresh current source/authority, separate from original control coordinates.
    pub(super) admission_source_sha256: String,
    pub(super) recovery: Option<super::review_recovery::Recovery>,
}
impl RootStore {
    /// Observe submitted native producer/control evidence and current configured
    /// reviewer in one shared settings/SQL snapshot. The request uses the bounded
    /// native task envelope; no provider, command, actor or authority is accepted.
    /// Rechecks canonical source, registered controller/slots, original journals,
    /// publication event and current public receipt, then current gate evidence.
    /// Missing, failed, stale, uncertain or unported task state refuses without
    /// mutation. Owner/recovery permissions are never transferred for this read;
    /// an already published controller may remain alive. Actual budget reservation,
    /// provider/model availability, full context/images and native reviewer job
    /// permission are not established. Typed IO/SQL/codec and cleanup causes remain.
    pub fn preview_task_review(&self, work: &str, raw: &[u8]) -> Result<TaskReviewPreview, E> {
        self.with_prepared_review(work, raw, |_, prepared, _| Ok(prepared.preview))
    }

    /// Materialize complete original text and current task framing, or the full
    /// retained managed candidate/cohort inspection projection, without
    /// invoking a provider, reserving a call or accepting a task. Saved Work and
    /// global context limits are required and enforced without truncation.
    /// Missing/stale originals, unsupported image/client-window inputs or limits
    /// refuse without mutation; typed read/codec and cleanup causes are retained.
    /// Returned text is private input data, never authority or an execution lease.
    pub fn materialize_task_review(&self, work: &str, raw: &[u8]) -> Result<TaskReviewInputs, E> {
        self.with_prepared_review(work, raw, |tx, prepared, candidate| {
            self.assemble_review_inputs(tx, prepared, candidate)
        })
    }

    /// Share complete ordinary/managed assembly within the caller's current
    /// snapshot and external candidate custody. Typed source/codec/limit causes
    /// propagate; no process, reservation, write or future permission is created.
    fn assemble_review_inputs(
        &self, tx: &rusqlite::Transaction<'_>, prepared: PreparedReview,
        candidate: Option<&task_validation::ReviewCandidate>,
    ) -> Result<TaskReviewInputs,E> {
            self.assemble_review_inputs_using(tx,prepared,candidate,false)
    }
    /// Share exact source assembly with engine-owned native invocation framing;
    /// the flag conveys a supported transport fact, never a grant or policy.
    /// When a managed Recovery is consumed through actual held candidate
    /// custody, its original artifact map is verified against the actually
    /// assembled observations before any input reaches the caller, Material or
    /// a staged claim; missing original map or mismatch refuses typed-safe.
    /// Ordinary assembly and its errors are unchanged.
    fn assemble_review_inputs_using(
        &self, tx:&rusqlite::Transaction<'_>,prepared:PreparedReview,
        candidate:Option<&task_validation::ReviewCandidate>,invocation:bool,
    )->Result<TaskReviewInputs,E>{
            let submitted = task_validation::submission(tx, &prepared.work, &prepared.preview.task_id)
                .map_err(|e| E::Launch(Box::new(e)))?;
            // Borrow-then-clone only the few fields the managed recovery artifact
            // check needs; `prepared` is consumed by the assembler below. A
            // candidate is Some only for managed Work, whose recovery always
            // comes from the managed-only observation entry.
            let managed_original_artifacts = match (candidate, prepared.recovery.as_ref()) {
                (Some(_), Some(recovery)) => Some(recovery.previous.report_artifacts.clone()),
                _ => None,
            };
            let controls_artifacts = prepared.controls.artifacts.clone();
            let inputs = match candidate {
                Some(candidate) => {
                    let plan=candidate.candidate().control_plan.as_ref().ok_or(E::Refused("review-managed-plan-required"))?;
                    let reports=plan.policies.iter().map(|policy|task_validation::submission(tx,&prepared.work,&policy.task_id)
                        .map_err(|e|E::Launch(Box::new(e)))).collect::<Result<Vec<_>,E>>()?;
                    inputs::assemble_managed_using(&self.root,prepared,candidate,&reports,invocation)
                }
                None => inputs::assemble(&self.root, prepared, &submitted),
            }?;
            // Consuming obligation of every managed Recovery: the original
            // recorded alias map (data names, never passed to Root
            // artifact_hash here) must equal the map rebuilt from actual
            // assembled observations. Ordinary recovery re-read real Root
            // artifacts inside review_recovery and is never checked here.
            if let Some(original) = managed_original_artifacts {
                verify_managed_recovery_artifacts(original, &controls_artifacts, &inputs.materials)?;
            }
            Ok(inputs)
    }

    /// Keep source admission and caller assembly inside one settings/SQL read
    /// snapshot. Managed pin/final refresh run outside those locks; an external
    /// owner retains descriptors through every read. Rollback/release failures
    /// retain both causes; no writer, provider reservation or process permit.
    fn with_prepared_review<T>(
        &self,
        work: &str,
        raw: &[u8],
        consume: impl FnOnce(&rusqlite::Transaction<'_>, PreparedReview, Option<&task_validation::ReviewCandidate>) -> Result<T, E>,
    ) -> Result<T, E> {
        let launch = |e| E::Launch(Box::new(e));
        let request = task_validation::decode(raw).map_err(launch)?;
        // Custody stays outside every snapshot/result wrapper, including failures
        // during reservation/rollback/assembly and the final outside-lock refresh.
        let candidate=task_validation::acquire_review_candidate(self,work,&request)?;
        let result = (|| {
            let reservation = self.directory.reserve(false, false).map_err(E::Storage)?;
            let result=(||{
            let settings = self.read_settings()?;
            let tx = self.db.unchecked_transaction().map_err(E::Sql)?;
            let result = self.prepare_review_snapshot_using(&tx, settings, work, &request,None,candidate.as_ref())
                .and_then(|prepared| consume(&tx, prepared,candidate.as_ref()));
            finished(result, tx.rollback().map_err(E::Sql))
            })();
            finished(result,reservation.close().map_err(E::Storage))
        })();
        let result=result.and_then(|value|{
            if let Some(candidate)=&candidate{candidate.refresh(self)?;}
            Ok(value)
        });
        finished(result,candidate.map_or(Ok(()),|candidate|candidate.close()))
    }

    /// Reuse the exact native reviewer admission inside a caller-owned snapshot.
    /// Caller owns settings/SQL reservation, transaction completion and purpose;
    /// this only reads current originals/metadata and grants no process or mutation.
    /// Parsed request still undergoes source/revision/authority checks; failures
    /// retain the same typed provenance, policy and read causes as public previews.
    #[cfg(test)]
    pub(super) fn prepare_review_snapshot(
        &self,
        tx: &rusqlite::Transaction<'_>,
        settings: crate::EngineSettingsDocument,
        work: &str,
        request: &task_validation::Request,
    ) -> Result<PreparedReview, E> {
        self.prepare_review_snapshot_for_job(tx, settings, work, request, None)
    }
    /// Internal reobservation for one exact claimed review, after normalized job
    /// checks by the caller, including its intact financial reservation. None
    /// retains new-call admission; Some rechecks already-counted capacity without
    /// a second debit. This never grants execution or mutates the existing review;
    /// all source/control checks remain. A canonical protocol correction admits
    /// the actual current error as a fresh call (not reserved funding); original
    /// controls stay separately bound while fresh source authority is recomputed.
    /// An already claimed retry reobserves its immediate archived original.
    #[cfg(test)]
    pub(super) fn prepare_review_snapshot_for_job(
        &self,
        tx: &rusqlite::Transaction<'_>,
        settings: crate::EngineSettingsDocument,
        work: &str,
        request: &task_validation::Request,
        review: Option<&crate::IndependentReview>,
    ) -> Result<PreparedReview, E> {
        self.prepare_review_snapshot_using(tx,settings,work,request,review,None)
    }
    /// Build observations for a readonly managed inspector with actual external
    /// candidate custody. Native invocation supplies the same borrowed holder;
    /// missing custody retains its refusal. This parameter is never a JSON
    /// permission or a relaxed provider admission.
    /// Managed Work with held custody selects the managed-only recovery
    /// observation and, for review=None with a Recovery, the explicit held
    /// ReviewerRetry re-preview with that exact Recovery data; review=Some
    /// keeps the existing held-record preview and fresh/no-recovery keeps the
    /// existing preview. Recovery data are observations: the original managed
    /// artifact freshness is a consuming obligation discharged later in
    /// assemble_review_inputs_using against actually assembled observations.
    /// This internal route does not wire public acquisition to acquire_for_retry
    /// and grants no execution. Ordinary Work keeps the ordinary recovery entry
    /// and preview selection unchanged.
    /// Caller owns snapshot/settings, final source refresh and explicit custody.
    fn prepare_review_snapshot_using(
        &self, tx:&rusqlite::Transaction<'_>,settings:crate::EngineSettingsDocument,
        work:&str,request:&task_validation::Request,review:Option<&crate::IndependentReview>,
        candidate:Option<&task_validation::ReviewCandidate>,
    )->Result<PreparedReview,E>{
        let launch = |e| E::Launch(Box::new(e));
        let current = crate::read_recorded_work(tx, work).map_err(E::Work)?;
        // Managed review requires the external complete candidate holder. A
        // new control receipt alone never unlocks an ordinary target-only review.
        let managed = current.planning.as_deref().is_some_and(|p| p.repository.is_some());
        if managed && candidate.is_none() {
            return Err(E::Refused("review-managed-candidate-unported"));
        }
        // Managed-only observation entry only for managed Work with actual held
        // candidate custody; every other shape keeps the ordinary entry.
        let recovery = match (managed, candidate.is_some()) {
            (true, true) => self.managed_review_recovery(tx, &current, &request.task_id, review, &settings)?,
            _ => self.review_recovery(tx, &current, &request.task_id, review, &settings)?,
        };
        let source = if let Some(candidate)=candidate {
            match (review, recovery.as_ref()) {
                // Retry-only explicit purpose over the exact Root-verified Recovery.
                (None, Some(recovery)) =>
                    candidate.preview_for_retry(self,tx,&settings,work,request,recovery),
                // Existing held-record preview for the exact running review or fresh.
                (review, _) => candidate.preview(self,tx,&settings,work,request,review),
            }
        }else{match (review, recovery.as_ref()) {
            (None, Some(recovery)) => task_validation::prepare_reviewer_retry_preview(
                tx, &self.root, &settings, work, request, recovery),
            (Some(review), _) => task_validation::prepare_claimed_review_preview(
                tx, &self.root, &settings, work, request, review),
            (None, None) => task_validation::prepare_preview(tx, &self.root, &settings, work, request),
        }}.map_err(launch)?;
        task_validation::require_validation_idle(tx, work, &source.task_id)
            .map_err(launch)?;
        let planning = current
            .planning
            .as_deref()
            .ok_or(E::Refused("review-planning-configuration-required"))?;
        let reviewer = planning
            .reviewer
            .as_deref()
            .ok_or(E::Refused("reviewer_required"))?;
        if reviewer.authorized.is_empty() || !reviewer.failure.is_empty() {
            return Err(E::Refused("reviewer-configuration-not-active"));
        }
        let launch_policy = settings.values.agent_launch_policy().map_err(E::Settings)?;
        match review {
            Some(_) => crate::agent_reviewer_preflight::check_reserved(
                &self.root, &self.directory, planning, &launch_policy),
            None => crate::agent_reviewer_preflight::check(
                &self.root, &self.directory, Some(planning), &launch_policy),
        }.map_err(launch)?;
        let task = current
            .tasks
            .iter()
            .flatten()
            .find(|t| t.id == source.task_id)
            .ok_or(E::Refused("review-task-missing"))?;
        let validation = match &recovery {
            Some(recovery) => recovery.validation.clone(),
            None => task_validation::recorded_evidence(self, &current, &source)?,
        };
        let output = settings
            .values
            .validation_output_limit()
            .map_err(E::Settings)?;
        let quality = settings.quality_policy_snapshot().map_err(E::Settings)?;
        let controls = crate::check_validation_evidence(&self.root, task, Some(&quality), output)
            .map_err(E::ValidationEvidence)?
            .ok_or(E::Refused("review-current-controls-required"))?;
        let limits = settings
            .values
            .context_policy_limits()
            .map_err(E::Settings)?;
        let cap = settings
            .values
            .agent_workflow_body_limit()
            .map_err(E::Settings)?;
        let workflow = crate::read_agent_workflow(&self.root, "reviewer", cap, &limits)
            .map_err(E::Workflow)?;
        let remaining = reviewer
            .max_calls
            .checked_sub(reviewer.calls)
            .ok_or(E::Refused("reviewer-call-count-overflow"))?;
        let preview = TaskReviewPreview {
            schema_version: 1,
            work_id: current.id.clone(),
            work_revision: current.revision,
            task_id: task.id.clone(),
            settings_revision: settings.revision,
            producer_agent_id: source.producer_agent_id,
            attempt_id: source.attempt_id,
            validation,
            review_provider: reviewer.provider.clone(),
            review_provider_digest: reviewer.provider_digest.clone(),
            review_contract_sha256: crate::task_review_contract(task).map_err(E::Encode)?,
            workflow_sha256: workflow.workflow.sha256.clone(),
            workflow_prompt_sha256: workflow.prompt_sha256.clone(),
            criterion_count: task.criteria.as_ref().map_or(0, Vec::len),
            remaining_review_calls: remaining,
            native_control_evidence_verified: true,
            invocation_context_materialized: false,
            invocation_authorized: false,
            provider_started: false,
            task_accepted: false,
        };
        Ok(PreparedReview { work: current, preview, workflow, controls, settings,
            admission_source_sha256: source.source_sha256, recovery })
    }
}

/// Verify one managed Recovery's original artifact map against the actually
/// assembled observations before any inputs reach a caller, Material or a
/// staged claim. `original` is `previous.report_artifacts` cloned from the
/// Root-verified failed managed review; its keys are Material-produced data
/// aliases (cohort/..., candidate/...), never real Root paths, so they are
/// compared only as data names and are never passed to Root artifact_hash.
/// `controls_artifacts` is the current checked control artifact map and
/// `materials` are the complete assembled sources. This rebuilds the exact
/// full map Material records: current control artifacts plus every
/// report/document material, refusing a matching name with a different hash,
/// then requires exact equality with the original map. Freshness is never
/// satisfied by hashes or DTOs alone: the held candidate's full Captured/
/// bound_source preview checks and the assembler's actual source reads and
/// SHA comparisons remain mandatory and untouched. A missing original map or
/// any mismatch refuses typed-safe with no success-empty fallback. Pure
/// in-memory comparison: no filesystem, SQL or provider access, no mutation,
/// no new path or IO, and no authority, grant or acceptance is created.
fn verify_managed_recovery_artifacts(
    original: Option<std::collections::BTreeMap<String, String>>,
    controls_artifacts: &std::collections::BTreeMap<String, String>,
    materials: &[ReviewInputSource],
) -> Result<(), E> {
    let recorded = original.ok_or(E::Refused("review-recovery-original-artifacts-required"))?;
    let mut actual = controls_artifacts.clone();
    for source in materials.iter().filter(|s| matches!(s.kind.as_str(), "report" | "document")) {
        if actual.get(&source.name).is_some_and(|hash| hash != &source.sha256) {
            return Err(E::Refused("review-input-artifact-changed-since-controls"));
        }
        actual.insert(source.name.clone(), source.sha256.clone());
    }
    if recorded != actual {
        return Err(E::Refused("review-recovery-managed-artifacts-changed"));
    }
    Ok(())
}

// The historical internal module name preserves existing replay filters. Normal
// Root operations below use the same controller; SDK helpers remain cfg(test).
#[path = "reviewer_native/mod.rs"]
mod reviewer_candidate;
pub use reviewer_candidate::{execute_review_gate, TaskReviewOutcome};
