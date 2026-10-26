//! Current native cohort publication observation, never an acceptance permit.
use super::{finished, material_path, task_validation, RootStore, RootStoreError as E};
use crate::{AgentLaunchError as A, IndependentReview, ManagedReviewContext, Work};
use serde::Serialize;

/// Exact current candidate and original-opinion coordinates. Verification means
/// the named observations passed during this read, not criterion truth, identity,
/// a future lease or permission to accept. Missing publication/baseline consumers
/// remain explicit blockers even when the original opinion says passed.
#[derive(Serialize)]
pub struct ManagedPublicationPreview {
    pub schema_version: u32,
    pub work_id: String,
    pub work_revision: i64,
    pub settings_revision: crate::SettingCount,
    pub task_id: String,
    pub review_id: String,
    pub opinion_state: String,
    pub candidate_commit: String,
    pub previous_candidate: String,
    pub candidate_binding_sha256: String,
    pub context_sha256: String,
    pub cohort_tasks: Vec<String>,
    pub validation: task_validation::NativeValidationEvidence,
    pub native_original_verified: bool,
    pub current_candidate_verified: bool,
    pub publication_supported: bool,
    pub publication_ready: bool,
    pub blockers: Vec<&'static str>,
    pub provider_started: bool,
    pub task_accepted: bool,
}

/// Keep actual canonical state and original context private between snapshots.
/// No descriptor is owned here; the caller holds candidate custody separately.
struct Observed {
    work: Work,
    review: IndependentReview,
    context: ManagedReviewContext,
    validation: task_validation::NativeValidationEvidence,
    settings_revision: crate::SettingCount,
    settings_sha256: String,
}

/// Preserve typed source/codec failures behind a static safe diagnostic family.
fn caused(code: &'static str, error: impl std::error::Error + Send + Sync + 'static) -> E {
    E::Launch(Box::new(A::caused(code, error)))
}

/// Read one exact current terminal managed opinion and original controls under a
/// short material-bound snapshot. Optional held custody rechecks the authoritative
/// source without Git inside SQL. No event, receipt, job or Work write occurs.
fn observe(root: &RootStore, work: &str, request: &task_validation::Request,
    candidate: Option<&task_validation::ReviewCandidate>) -> Result<Observed, E> {
    material_path::with_material_read(root, || {
        let settings = root.read_settings()?;
        let tx = root.db.unchecked_transaction().map_err(E::Sql)?;
        let result = (|| {
            let current = crate::read_recorded_work(&tx, work).map_err(E::Work)?;
            if current.revision != request.expected_revision {
                return Err(E::Refused("publication-preview-revision-conflict"));
            }
            if !current.planning.as_deref().is_some_and(|p| p.repository.is_some()) {
                return Err(E::Refused("publication-preview-managed-repository-required"));
            }
            let mut tasks = current.tasks.iter().flatten().filter(|t| t.id == request.task_id);
            let task = tasks.next().ok_or(E::Refused("publication-preview-task-required"))?;
            if tasks.next().is_some() { return Err(E::Refused("publication-preview-task-ambiguous")); }
            let review = task.independent_review.as_deref()
                .ok_or(E::Refused("publication-preview-original-required"))?;
            let original = root.recorded_native_review(&tx, work, &review.id)?;
            if original.task != task.id || !original.verdict_applicable || original.review != *review
                || original.revision > current.revision {
                return Err(E::Refused("publication-preview-current-opinion-required"));
            }
            let context = original.managed_context.ok_or(E::Refused("publication-preview-managed-original-required"))?;
            let validation = task_validation::recorded_evidence_for_review_result(
                root, &current, &task.id, &original.validation_receipt_sha256)?;
            if let Some(candidate) = candidate {
                candidate.preview(root, &tx, &settings, work, request, Some(review))
                    .map_err(|e| E::Launch(Box::new(e)))?;
                verify_cohort(root, &tx, &current, review, &context, candidate)?;
            }
            Ok(Observed { review: review.clone(), context, validation,
                settings_revision: settings.revision,
                settings_sha256: crate::provider_configuration::configuration_digest(
                    &crate::encode_model(&settings).map_err(E::Encode)?), work: current })
        })();
        finished(result, tx.rollback().map_err(E::Sql))
    })
}

/// Compare the verified original cohort against the held current checkpoint and
/// actual canonical submissions/completed producers. Metadata binds every original
/// control, policy and full candidate digest; per-task checks retain all attempts,
/// contracts, report digests and original policies. No status implies old baseline
/// authority. Codec/SQL/IO errors refuse, without a partial cohort or fallback.
fn verify_cohort(root: &RootStore, db: &rusqlite::Connection, work: &Work,
    review: &IndependentReview, context: &ManagedReviewContext,
    candidate: &task_validation::ReviewCandidate) -> Result<(), E> {
    let held = candidate.candidate();
    if review.candidate_commit != held.commit || context.candidate_commit != held.commit
        || review.previous_candidate != held.previous || context.previous_candidate != held.previous {
        return Err(E::Refused("publication-preview-candidate-changed"));
    }
    let metadata: serde_json::Value = serde_json::from_str(context.receipt.0.get())
        .map_err(|e| caused("publication-preview-control-context", e))?;
    if crate::encode_model(&metadata).map_err(E::Encode)?
        != crate::encode_model(candidate.metadata()).map_err(E::Encode)? {
        return Err(E::Refused("publication-preview-control-context-changed"));
    }
    let plan = held.control_plan.as_ref().ok_or(E::Refused("publication-preview-plan-required"))?;
    let tasks = context.tasks.as_deref().ok_or(E::Refused("publication-preview-cohort-required"))?;
    if tasks.len() != plan.policies.len() { return Err(E::Refused("publication-preview-cohort-changed")); }
    for (entry, policy) in tasks.iter().zip(&plan.policies) {
        let task = work.tasks.iter().flatten().find(|t| t.id == policy.task_id)
            .ok_or(E::Refused("publication-preview-cohort-task-required"))?;
        let report = task_validation::submission(db, work, &task.id).map_err(|e| E::Launch(Box::new(e)))?;
        let producer = crate::task_report::completed_producer(db, &work.id, task, &report.agent_id)
            .map_err(|e| E::Launch(Box::new(e)))?;
        let binding = &entry.binding;
        if entry.task != task.id || binding.task != task.id || binding.attempt != producer.attempt_id
            || binding.producer != producer.id || binding.contract != crate::task_review_contract(task).map_err(E::Encode)?
            || binding.policy != crate::provider_configuration::configuration_digest(
                &crate::encode_model(&policy.original).map_err(E::Encode)?)
            || binding.report != report.report_path || binding.report_sha256 != report.report_sha256
            || entry.criteria != task.criteria || entry.title != task.title || entry.deliverable != task.deliverable {
            return Err(E::Refused("publication-preview-task-binding-changed"));
        }
        let (_, hash) = crate::gate_evaluation::artifact_hash(&root.root, &report.report_path)
            .map_err(|e| caused("publication-preview-report", e))?;
        if hash != binding.report_sha256 || crate::provider_configuration::configuration_digest(entry.report.as_bytes()) != hash {
            return Err(E::Refused("publication-preview-report-changed"));
        }
    }
    Ok(())
}

impl RootStore {
    /// Observe whether original native whole-cohort evidence still matches the
    /// current managed candidate. Takes the strict native task/revision envelope;
    /// the event ID is input syntax only and is never persisted. Source pin/refresh
    /// runs outside Root locks, with descriptors retained across both snapshots
    /// and explicitly closed on every result. Current saved policies are rechecked.
    /// Returns verified observations and explicit unsupported-publication blockers,
    /// never a ready/accepted result. Stale/incomplete/ordinary/non-verdict originals,
    /// task/report/candidate drift, malformed inputs and cleanup errors refuse with
    /// causes. No provider, debit, new checkpoint, Work or Git ref mutation occurs.
    pub fn preview_managed_publication(&self, work: &str, raw: &[u8]) -> Result<ManagedPublicationPreview, E> {
        let request = task_validation::decode(raw).map_err(|e| E::Launch(Box::new(e)))?;
        let before = observe(self, work, &request, None)?;
        let candidate = task_validation::acquire_claimed_review_candidate(
            self, work, &request, Some(&before.review))?
            .ok_or(E::Refused("publication-preview-candidate-required"))?;
        let result = (|| {
            candidate.refresh(self)?;
            let checked = observe(self, work, &request, Some(&candidate))?;
            if checked.settings_sha256 != before.settings_sha256 || checked.work != before.work {
                return Err(E::Refused("publication-preview-inputs-changed"));
            }
            candidate.refresh(self)?;
            // Final canonical/material read after outside-lock Git observations;
            // no matching digest is a future source lease or permission.
            let after = observe(self, work, &request, Some(&candidate))?;
            if after.work != checked.work || after.settings_sha256 != checked.settings_sha256 {
                return Err(E::Refused("publication-preview-inputs-changed"));
            }
            let mut blockers = vec!["managed-protected-publication-unported", "managed-accepted-baseline-authority-unported"];
            if checked.review.state != "passed" { blockers.push("managed-passed-opinion-required"); }
            Ok(ManagedPublicationPreview { schema_version: 1, work_id: work.into(),
                work_revision: checked.work.revision, settings_revision: checked.settings_revision,
                task_id: request.task_id, review_id: checked.review.id,
                opinion_state: checked.review.state, candidate_commit: candidate.candidate().commit.clone(),
                previous_candidate: candidate.candidate().previous.clone(),
                candidate_binding_sha256: candidate.candidate().binding_digest().map_err(|e| E::Launch(Box::new(e)))?,
                context_sha256: checked.review.context_sha256,
                cohort_tasks: checked.context.tasks.unwrap_or_default().into_iter().map(|t| t.task).collect(),
                validation: checked.validation, native_original_verified: true, current_candidate_verified: true,
                publication_supported: false, publication_ready: false, blockers,
                provider_started: false, task_accepted: false })
        })();
        finished(result, candidate.close())
    }
}
