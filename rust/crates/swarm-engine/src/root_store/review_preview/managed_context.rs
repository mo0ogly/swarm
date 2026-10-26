//! Canonical readonly cohort protocol from already checked Root/candidate inputs.
use super::{inputs::ReviewInputSource, PreparedReview, E};
use crate::{ManagedReviewContext, ManagedReviewTaskContext, ManagedTaskReview, ReviewSource};

/// Build complete typed quote context from actual current tasks, canonical
/// submissions and successful candidate-job observations. The caller owns source
/// verification, candidate custody and final freshness/close. No historical
/// accepted verdict is inferred from a task status or report. Missing/cohort-
/// mismatched attempts, observations and reports refuse; codec/settings causes
/// survive. All retained material is supplied as attributed data; there is no
/// file IO, provider invocation, reservation, mutation or acceptance here.
pub(super) fn build(
    prepared: &PreparedReview,
    candidate: &crate::root_store::task_validation::ReviewCandidate,
    reports: &[crate::TaskReportRequest],
    materials: &[ReviewInputSource],
) -> Result<crate::managed_review_protocol::ManagedReviewProtocol, E> {
    let checkpoint = candidate.candidate();
    let plan = checkpoint.control_plan.as_ref().ok_or(E::Refused("review-managed-plan-required"))?;
    if plan.policies.len() != reports.len() {
        return Err(E::Refused("review-managed-reports-incomplete"));
    }
    let observations = candidate.metadata().get("control_observations")
        .and_then(serde_json::Value::as_array).ok_or(E::Refused("review-managed-control-observations-required"))?;
    let mut tasks = Vec::new();
    for (policy, report) in plan.policies.iter().zip(reports) {
        if report.id != policy.task_id { return Err(E::Refused("review-managed-report-task-changed")); }
        let task = prepared.work.tasks.iter().flatten().find(|task| task.id == policy.task_id)
            .ok_or(E::Refused("review-task-missing"))?;
        let attempt = task.attempts.iter().flatten().last().ok_or(E::Refused("review-managed-attempt-required"))?;
        if !swarm_identity_contract::name(attempt.id.as_bytes()) || !swarm_identity_contract::name(report.agent_id.as_bytes()) {
            return Err(E::Refused("review-managed-attempt-invalid"));
        }
        if task.id == prepared.preview.task_id && (attempt.id != prepared.preview.attempt_id
            || report.agent_id != prepared.preview.producer_agent_id) {
            return Err(E::Refused("review-managed-producer-changed"));
        }
        let name = format!("cohort/{}/{}", task.id, report.report_path);
        let source = materials.iter().find(|source| source.name == name && source.kind == "report")
            .ok_or(E::Refused("review-managed-report-required"))?;
        if source.sha256 != report.report_sha256 { return Err(E::Refused("review-report-changed")); }
        let mut controls = Vec::new();
        for observation in observations.iter().filter(|observation| observation.get("task_id").and_then(serde_json::Value::as_str) == Some(task.id.as_str())) {
            let raw = observation.get("result").filter(|value| !value.is_null())
                .ok_or(E::Refused("review-managed-control-result-required"))?;
            let result: crate::ValidationControlResult = serde_json::from_value(raw.clone())
                .map_err(|e| E::Launch(Box::new(crate::AgentLaunchError::caused("review-managed-control-result-decode",e))))?;
            controls.push(result);
        }
        if controls.len() != plan.slots.iter().filter(|slot| slot.task_id == task.id).count() || controls.is_empty() {
            return Err(E::Refused("review-managed-task-controls-incomplete"));
        }
        let policy_digest = crate::provider_configuration::configuration_digest(
            &crate::encode_model(&policy.original).map_err(E::Encode)?);
        tasks.push(ManagedReviewTaskContext {
            task: task.id.clone(), title: task.title.clone(), deliverable: task.deliverable.clone(),
            criteria: task.criteria.clone(), report: source.content.clone(), controls: Some(controls),
            binding: ManagedTaskReview {
                task: task.id.clone(), attempt: attempt.id.clone(), producer: report.agent_id.clone(),
                contract: crate::task_review_contract(task).map_err(E::Encode)?, policy: policy_digest,
                report: report.report_path.clone(), report_sha256: report.report_sha256.clone(),
                ..Default::default()
            },
            // No delivery or historical verdict is invented from this observation.
            delivery: None,
        });
    }
    let sources = source_materials(materials)?;
    let context = ManagedReviewContext {
        citation_contract_version: 2, inspection_scope_version: 1, inspection_only: true,
        accepted_baseline: None, sources: Some(sources), candidate_commit: checkpoint.commit.clone(),
        previous_candidate: checkpoint.previous.clone(), diff: String::new(),
        receipt: crate::model_wire::persisted_raw(candidate.metadata()).map_err(E::Encode)?, tasks: Some(tasks),
    };
    let limits = prepared.settings.values.review_reply_limits().map_err(E::Settings)?;
    crate::managed_review_protocol::build(context, limits)
        .map_err(|e| E::Launch(Box::new(crate::AgentLaunchError::caused("review-managed-protocol",e))))
}

/// Project only report/document materials to implementation quote sources.
/// Engine control/cohort metadata stays fully available as attributed receipt
/// and public material data, without being relabeled as code/report evidence.
/// These material kinds are an evidence-role invariant, never operator policy.
/// Existing source
/// hashes/complete bytes are checked by the caller; checked length conversion
/// refuses representation overflow. No IO, mutation or provenance is inferred.
fn source_materials(materials: &[ReviewInputSource]) -> Result<Vec<ReviewSource>, E> {
    materials.iter().filter(|source| matches!(source.kind.as_str(), "report" | "document")).map(|source| {
        Ok(ReviewSource {
            path: source.name.clone(), git_blob: String::new(), sha256: source.sha256.clone(),
            bytes: i64::try_from(source.content.len()).map_err(|_| E::Refused("review-source-count-overflow"))?,
            content: source.content.clone(),
        })
    }).collect::<Result<Vec<_>,E>>()

}

#[cfg(test)]
mod tests {
    use super::*;
    /// Counterexample: engine metadata must remain data outside implementation
    /// source quotes, while a real source quote still passes the actual parser.
    #[test]
    fn engine_metadata_is_not_implementation_source_evidence() {
        let metadata = "METADATA-ONLY-UNPROVEN-IMPLEMENTATION";
        let code = "fn actual_code() { return true; }";
        let materials: Vec<_> = [("candidate/code.rs", "document", code),
            ("candidate-cohort.json", "engine_controls", metadata)].into_iter().map(|(name,kind,content)| ReviewInputSource {
                name: name.into(), kind: kind.into(), quote_kind: if kind == "document" {"source"} else {kind}.into(),
                content: content.into(), sha256: crate::provider_configuration::configuration_digest(content.as_bytes()),
            }).collect();
        let context=ManagedReviewContext {
            citation_contract_version:1, inspection_only:true, sources:Some(source_materials(&materials).unwrap()),
            candidate_commit:"a".repeat(40), tasks:Some(vec![ManagedReviewTaskContext {
                task:"task".into(), criteria:Some(vec!["behavior".into()]), report:"Actual report original".into(),
                ..Default::default()
            }]), ..Default::default()
        };
        let limits=crate::ReviewReplyLimits {min_text_bytes:8,max_text_bytes:Some(1000),min_normalized_quote_bytes:8};
        let parse=|evidence:&str| {
            let raw=crate::encode_model(&serde_json::json!({"candidate_commit":context.candidate_commit,
                "tasks":[{"task":"task","reason":"Actual source quote observed", "criteria":[{"index":1,"verdict":"pass","evidence":evidence}]}]})).unwrap();
            crate::managed_review_reply::parse_managed_review_reply(&raw,&context,None,limits,None)
        };
        assert!(parse(code).is_ok(),"valid implementation-source witness");
        assert!(parse(metadata).is_err(),"engine metadata cannot prove implementation");
    }
}
