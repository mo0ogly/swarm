//! Nonmanaged current review observations for the historical aggregate.
//! Go result_presentation.go currentReviewPresentation ordering. Recorded review
//! phase is not reviewer process liveness, identity authentication or acceptance.
use super::{
    mission_status_evidence::{semantic_artifact, semantic_gate},
    RootStoreError as E,
};
use crate::gate_evaluation::artifact_hash;
use crate::mission_status_tasks::{ReviewObservation, ReviewPresentation};
use crate::{Agent, EngineSettingsDocument, IndependentReview, Task, Work};
use std::path::Path;

/// Require the actual latest attempt and producer, without normalization or a
/// chronological guess from opaque identifiers. Passed deliberately falls
/// through as Go; this pure predicate verifies no evidence or authority.
pub(super) fn applies(
    planning: bool,
    latest_attempt: Option<&str>,
    agent_id: &str,
    agent_attempt: &str,
    review: &IndependentReview,
) -> bool {
    planning
        && review.state != "passed"
        && latest_attempt == Some(review.attempt.as_str())
        && review.attempt == agent_attempt
        && review.producer == agent_id
}

/// Match the historical presentation vocabulary; unknown is not a passed opinion.
fn state_label(state: &str) -> &'static str {
    match state {
        "running" => "Examen en cours",
        "passed" => "Avis favorable",
        "changes_requested" => "Corrections ou preuves demandées",
        "error" => "Vérification interrompue",
        "stale" => "Avis périmé",
        _ => "Avis indisponible",
    }
}

/// Construct only review causal fields, before checking report and contract.
/// Caller must not publish the base alone; no accepted/process field is set.
fn base(review: &IndependentReview) -> ReviewPresentation {
    ReviewPresentation {state:"review_blocked".into(),label:state_label(&review.state).into(),validation_state:"review_unvalidated".into(),reason:format!("Revue indépendante : {}",review.reason),next_step:"Examiner le rapport et le motif de la revue avant de choisir une correction ou une reprise autorisée.".into(),report_id:review.report.clone(),receipt_id:review.receipt.clone(),report_state:"submitted".into()}
}

/// Mark an observed missing/changed report. This changes presentation only;
/// caller separately retains a typed original semantic failure where one exists.
fn stale_report(p: &mut ReviewPresentation) {
    p.report_state = "stale".into();
    p.validation_state = "failed_or_stale".into();
    p.label = "Preuves à renouveler".into();
    p.reason="Le rapport transmis au vérificateur est indisponible ou modifié ; ses preuves doivent être réexaminées.".into();
}

/// Check the contained report bytes against their recorded digest. Known stale
/// errors are retained in the sidecar and return false; unknown IO/cleanup errors
/// refuse the whole read. No unreadable file becomes absence or an empty success.
fn report_current(
    root: &Path,
    review: &IndependentReview,
    p: &mut ReviewPresentation,
    causes: &mut Vec<E>,
) -> Result<bool, E> {
    match artifact_hash(root, &review.report) {
        Ok((_, hash)) if hash == review.sha256 => Ok(true),
        Ok(_) => {
            causes.push(E::Refused("mission-status-review-report-stale"));
            stale_report(p);
            Ok(false)
        }
        Err(error) if semantic_artifact(&error) => {
            causes.push(E::Gate(crate::GateContractError::Evaluation(error)));
            stale_report(p);
            Ok(false)
        }
        Err(error) => Err(E::Gate(crate::GateContractError::Evaluation(error))),
    }
}

/// Check every declared review artifact. A digest mismatch or known semantic
/// failure returns false with its typed sidecar; unknown IO/close errors refuse.
/// Missing map is exactly the stored absence, not a fabricated evidence list.
fn artifacts_current(
    root: &Path,
    review: &IndependentReview,
    causes: &mut Vec<E>,
) -> Result<bool, E> {
    for (name, expected) in review.report_artifacts.iter().flatten() {
        match artifact_hash(root, name) {
            Ok((_, actual)) if &actual == expected => {}
            Ok(_) => {
                causes.push(E::Refused("mission-status-review-artifact-stale"));
                return Ok(false);
            }
            Err(error) if semantic_artifact(&error) => {
                causes.push(E::Gate(crate::GateContractError::Evaluation(error)));
                return Ok(false);
            }
            Err(error) => return Err(E::Gate(crate::GateContractError::Evaluation(error))),
        }
    }
    Ok(true)
}

/// Finalize only after report/contract/artifact checks. The stored running phase
/// describes review workflow, never an observed live reviewer process. No state
/// can turn this opinion into accepted/validated or grant another call.
fn apply_state(p: &mut ReviewPresentation, state: &str) {
    match state {
        "changes_requested" => p.validation_state = "review_changes_requested".into(),
        "stale" => p.validation_state = "failed_or_stale".into(),
        "running" => {
            p.state = "review_in_progress".into();
            p.validation_state = "pending_review".into();
            p.reason =
                "Le rapport est conservé ; le vérificateur indépendant examine le candidat testé."
                    .into();
            p.next_step =
                "Attendre l’avis indépendant ; le résultat n’est pas encore accepté.".into()
        }
        _ => {}
    }
}

/// Distinguish stale control evidence from unknown IO/configuration/codec errors.
/// Quality policy absence is a missing precondition, never an unfavorable verdict.
fn semantic_validation(error: &crate::ValidationEvidenceError) -> bool {
    match error {
        crate::ValidationEvidenceError::Refused(code) => matches!(
            *code,
            "current-validation-required"
                | "successful-controls-required"
                | "complete-receipt-required"
                | "control-not-demonstrated"
                | "control-output-not-authorized"
                | "control-output-incoherent"
                | "validation-artifact-stale"
        ),
        crate::ValidationEvidenceError::Gate(error) => semantic_gate(error),
        crate::ValidationEvidenceError::Artifact(error) => semantic_artifact(error),
        crate::ValidationEvidenceError::Encode(_) => false,
    }
}

/// Match the known Go control-refusal vocabulary for the static receipt cases.
/// Other typed causes retain their safe Rust diagnostic; this formatting neither
/// suppresses the original cause nor establishes verbatim parity for all errors.
fn validation_reason(error:&crate::ValidationEvidenceError)->String {
    match error {
        crate::ValidationEvidenceError::Refused("current-validation-required")=>"contrôles moteur courants requis avant la revue".into(),
        crate::ValidationEvidenceError::Refused("successful-controls-required")=>"contrôles non réussis".into(),
        crate::ValidationEvidenceError::Refused("complete-receipt-required")=>"reçu moteur incomplet".into(),
        _=>error.to_string(),
    }
}

/// Observe the applicable ordinary review using current contained artifacts and
/// contract, with Go's passed/no-planning/stale-attempt fallthrough. Managed
/// presentation remains refused. Before first required review, current controls
/// use saved validation-output/quality settings; semantic failures are explicit
/// ControlsStale with original causes, unknown failures remain Err. Caller
/// supplies a real canonical Work/settings snapshot under material association,
/// preserves the sidecar even on later errors, and rechecks its outer snapshot.
/// No launch, SQL mutation, retry, new policy, authentication or acceptance occurs.
pub(super) fn observe(
    root: &Path,
    work: &Work,
    task: &Task,
    agent: &Agent,
    settings: &EngineSettingsDocument,
    causes: &mut Vec<E>,
) -> Result<ReviewObservation, E> {
    let Some(planning) = work.planning.as_deref() else {
        return Ok(ReviewObservation::NotApplicable);
    };
    if planning.repository.is_some() {
        return Err(E::Refused("mission-status-managed-review-unported"));
    }
    let latest = task
        .attempts
        .as_ref()
        .and_then(|v| v.last())
        .map(|a| a.id.as_str());
    if task.independent_review.is_none()
        && planning.reviewer_required
        && task.status == "submitted"
        && agent.status == "completed"
        && latest == Some(agent.attempt_id.as_str())
    {
        let output = settings
            .values
            .validation_output_limit()
            .map_err(E::Settings)?;
        let quality = if task.validation_policy.as_deref().is_some_and(|p| {
            p.mode == "automatic" && p.controls.as_ref().is_some_and(|v| !v.is_empty())
        }) {
            Some(settings.quality_policy_snapshot().map_err(E::Settings)?)
        } else {
            None
        };
        match crate::check_validation_evidence(root, task, quality.as_ref(), output) {
            Ok(_) => {}
            Err(error) if semantic_validation(&error) => {
                let reason = validation_reason(&error);
                causes.push(E::ValidationEvidence(error));
                return Ok(ReviewObservation::ControlsStale { reason });
            }
            Err(error) => return Err(E::ValidationEvidence(error)),
        }
    }
    let Some(review) = task.independent_review.as_deref() else {
        return Ok(ReviewObservation::NotApplicable);
    };
    if !applies(true, latest, &agent.id, &agent.attempt_id, review) {
        return Ok(ReviewObservation::NotApplicable);
    }
    let mut presentation = base(review);
    if !report_current(root, review, &mut presentation, causes)? {
        return Ok(ReviewObservation::Presented(presentation));
    }
    let contract = crate::review_contract(task).map_err(E::Encode)?;
    if review.contract != contract || !artifacts_current(root, review, causes)? {
        if review.contract != contract {
            causes.push(E::Refused("mission-status-review-contract-stale"))
        }
        presentation.validation_state = "failed_or_stale".into();
        presentation.label = "Avis périmé".into();
        presentation.reason =
            "Le contrat ou la révision de base a changé depuis cette revue.".into();
        return Ok(ReviewObservation::Presented(presentation));
    }
    apply_state(&mut presentation, &review.state);
    Ok(ReviewObservation::Presented(presentation))
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Pure identity matching is not liveness, artifact or acceptance evidence.
    #[test]
    fn current_review_matches_latest_attempt_and_producer_only() {
        let mut review = IndependentReview {
            attempt: "a3".into(),
            producer: "agent".into(),
            state: "changes_requested".into(),
            ..Default::default()
        };
        assert!(applies(true, Some("a3"), "agent", "a3", &review));
        for (planning, latest, id, attempt) in [
            (false, Some("a3"), "agent", "a3"),
            (true, None, "agent", "a3"),
            (true, Some("a4"), "agent", "a3"),
            (true, Some("a3"), "other", "a3"),
            (true, Some("a3"), "agent", "a4"),
        ] {
            assert!(!applies(planning, latest, id, attempt, &review))
        }
        review.state = "passed".into();
        assert!(!applies(true, Some("a3"), "agent", "a3", &review));
    }
    /// Final/current metadata never manufactures validation, including unknown
    /// phases; exact historical labels remain distinct from process liveness.
    #[test]
    fn current_review_states_are_not_acceptance() {
        for (state, label, validation, result) in [
            (
                "changes_requested",
                "Corrections ou preuves demandées",
                "review_changes_requested",
                "review_blocked",
            ),
            (
                "error",
                "Vérification interrompue",
                "review_unvalidated",
                "review_blocked",
            ),
            ("stale", "Avis périmé", "failed_or_stale", "review_blocked"),
            (
                "running",
                "Examen en cours",
                "pending_review",
                "review_in_progress",
            ),
            (
                "other",
                "Avis indisponible",
                "review_unvalidated",
                "review_blocked",
            ),
        ] {
            let review = IndependentReview {
                state: state.into(),
                ..Default::default()
            };
            let mut p = base(&review);
            apply_state(&mut p, state);
            assert_eq!(p.label, label);
            assert_eq!(p.validation_state, validation);
            assert_eq!(p.state, result);
        }
    }
    /// Before first required review, a missing real control receipt blocks the
    /// result without inventing a reviewer call. Pure supplied Work/settings,
    /// no filesystem/SQL writes or current native acceptance proof.
    #[test]
    fn current_review_missing_controls_are_observed_not_ignored() {
        let settings = EngineSettingsDocument::initial(
            crate::EngineSettingsValues::installation_defaults().unwrap(),
            "fixture".into(),
            "2026-10-23T12:00:00Z".into(),
        )
        .unwrap();
        let task:Task=serde_json::from_value(serde_json::json!({"id":"t","status":"submitted","attempts":[{"id":"a3"}],"validation_policy":{"mode":"human","controls":[{"id":"fixture-control","command":["python3","-c","pass"]}]}})).unwrap();
        let mut work = Work {
            planning: Some(Box::new(crate::PlanningState {
                reviewer_required: false,
                ..Default::default()
            })),
            ..Default::default()
        };
        let agent = Agent {
            id: "agent".into(),
            task_id: "t".into(),
            attempt_id: "a3".into(),
            status: "completed".into(),
            ..Default::default()
        };
        let mut causes = vec![];
        assert!(matches!(
            observe(
                Path::new("/unused-read-free-fixture"),
                &work,
                &task,
                &agent,
                &settings,
                &mut causes
            )
            .unwrap(),
            ReviewObservation::NotApplicable
        ));
        assert!(causes.is_empty());
        work.planning.as_mut().unwrap().reviewer_required = true;
        assert!(matches!(
            observe(
                Path::new("/unused-read-free-fixture"),
                &work,
                &task,
                &agent,
                &settings,
                &mut causes
            )
            .unwrap(),
            ReviewObservation::ControlsStale { reason } if reason=="contrôles moteur courants requis avant la revue"
        ));
        assert_eq!(causes.len(), 1);
        assert!(matches!(&causes[0], E::ValidationEvidence(_)));
        assert!(!semantic_validation(
            &crate::ValidationEvidenceError::Refused("unknown-future-error")
        ));
    }
    /// A real current report then changed/missing bytes and restoration exercise
    /// semantic sidecars. The unique owned fixture is retained for diagnostics.
    #[test]
    fn current_review_report_and_artifact_staleness_recover() {
        let root = std::env::temp_dir().join(format!(
            "swarm-status-review-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        eprintln!("OWNED_REVIEW_FIXTURE {}", root.display());
        std::fs::write(root.join("report.md"), b"original").unwrap();
        let hash = artifact_hash(&root, "report.md").unwrap().1;
        let review = IndependentReview {
            report: "report.md".into(),
            sha256: hash.clone(),
            report_artifacts: Some(std::collections::BTreeMap::from([(
                "report.md".into(),
                hash,
            )])),
            ..Default::default()
        };
        let mut causes = vec![];
        let mut p = base(&review);
        assert!(report_current(&root, &review, &mut p, &mut causes).unwrap());
        assert!(artifacts_current(&root, &review, &mut causes).unwrap());
        assert!(causes.is_empty());
        std::fs::write(root.join("report.md"), b"changed").unwrap();
        assert!(!report_current(&root, &review, &mut p, &mut causes).unwrap());
        assert_eq!(p.report_state, "stale");
        assert!(!artifacts_current(&root, &review, &mut causes).unwrap());
        assert_eq!(causes.len(), 2);
        std::fs::write(root.join("report.md"), b"original").unwrap();
        assert!(report_current(&root, &review, &mut base(&review), &mut vec![]).unwrap());
        std::fs::remove_file(root.join("report.md")).unwrap();
        let mut missing = vec![];
        assert!(!report_current(&root, &review, &mut p, &mut missing).unwrap());
        assert_eq!(missing.len(), 1);
        assert!(matches!(&missing[0], E::Gate(_)));
    }
}
