//! Pure branch controls only. Explicit supplied observations are test data, not
//! a real gate/review/report/Root verification or permission to execute anything.
use super::*;

/// Supplied inert baseline. No observation is loaded, inferred or authenticated.
fn result_facts() -> ResultObservation {
    ResultObservation {
        freshness: FreshnessObservation::NotCurrent { blockers: vec![] },
        gate: GateObservation::Absent,
        reports: ReportListing::Observed(vec![]),
        attribution: ReportAttribution::Unavailable {
            reason: "aucun rapport attribuable".into(),
        },
        incomplete_report: IncompleteReport::Observed(None),
        review: ReviewObservation::NotApplicable,
    }
}
/// Complete test-only observation with a known missing profile; no grant follows.
fn facts(id: &str) -> TaskObservation {
    TaskObservation {
        task_id: id.into(),
        result: result_facts(),
        dependencies: vec![],
        readiness: ReadinessObservation::Observed {
            disposition: DispatchDisposition::Intervention,
            reason: "fixture-profile-missing".into(),
            profile: ProfileObservation::Missing,
            assistance: AssistObservation::NotObserved,
            workspace: WorkspaceObservation::NotObserved,
        },
        prepared_launch: PreparedLaunchObservation::Absent,
        execution: ExecutionObservation::NotPresent,
    }
}
/// Stored model fixture only. Default fields are intentional test data, never a
/// production observation fallback or proof that an actual artifact was checked.
fn task(id: &str, status: &str) -> Task {
    Task {
        id: id.into(),
        title: id.into(),
        status: status.into(),
        plan_max_attempts: 3,
        ..Default::default()
    }
}
/// Fully supplied empty diagnostic, not an implicit fallback for read failure.
fn diagnostic() -> AttemptDiagnostic {
    AttemptDiagnostic {
        agent_id: "fixture-agent".into(),
        attempt_id: "fixture-attempt".into(),
        observed_errors: 0,
        consecutive_error_limit: 3,
        limit_reached: false,
        consecutive_error_limit_reached: false,
        limit_reason: String::new(),
        summary: String::new(),
        items: Some(vec![]),
    }
}

/// A normal process completion cannot become task acceptance. An attributable
/// report advances only to review; only accepted+current evidence validates.
#[test]
fn mission_status_task_completion_report_and_acceptance_are_separate() {
    let mut task = task("t", "todo");
    let agent = Agent {
        id: "fixture-agent".into(),
        task_id: "t".into(),
        attempt_id: "fixture-attempt".into(),
        status: "completed".into(),
        ..Default::default()
    };
    let diagnostic = diagnostic();
    let observed = AgentObservation {
        agent: &agent,
        observed_state: "completed",
        fallback_diagnostic: &diagnostic,
        environment_recovery: false,
    };
    let mut facts = result_facts();
    let completed = result_presentation(&task, Some(&observed), &facts).unwrap();
    assert_eq!(completed.process_state, "completed");
    assert_eq!(completed.state, "completed_unproven");
    assert_eq!(completed.validation_state, "not_started");
    facts.attribution = ReportAttribution::Attributed("fixture-report.md".into());
    let report = result_presentation(&task, Some(&observed), &facts).unwrap();
    assert_eq!(report.state, "result_to_review");
    assert_eq!(report.report_state, "attributable_unsubmitted");
    task.status = "accepted".into();
    task.gate = Some(Default::default());
    facts.freshness = FreshnessObservation::Current;
    facts.gate = GateObservation::Current;
    let accepted = result_presentation(&task, Some(&observed), &facts).unwrap();
    assert_eq!(accepted.state, "validated");
    assert_eq!(accepted.validation_state, "fresh");
    assert_eq!(accepted.process_state, "completed");
}

/// Valid current evidence precedes stale and missing variants; stored acceptance
/// survives as history while stale evidence never counts as currently validated.
#[test]
fn mission_status_task_stale_and_unobserved_evidence_do_not_validate() {
    let mut task = task("t", "accepted");
    task.gate = Some(Default::default());
    let mut facts = result_facts();
    facts.freshness = FreshnessObservation::Current;
    facts.gate = GateObservation::Current;
    assert_eq!(
        result_presentation(&task, None, &facts).unwrap().state,
        "validated"
    );
    facts.freshness = FreshnessObservation::NotCurrent {
        blockers: vec!["fixture-preuve-modifiée".into()],
    };
    facts.gate = GateObservation::NotCurrent;
    let stale = result_presentation(&task, None, &facts).unwrap();
    assert_eq!(stale.state, "validation_stale");
    assert_eq!(stale.reason, "fixture-preuve-modifiée");
    facts.freshness = FreshnessObservation::Current;
    facts.gate = GateObservation::Current;
    assert_eq!(
        result_presentation(&task, None, &facts).unwrap().state,
        "validated"
    );
    facts.freshness = FreshnessObservation::NotObserved;
    assert!(matches!(
        result_presentation(&task, None, &facts),
        Err(E::Refused("mission-status-freshness-unobserved"))
    ));
}

/// The valid completed witness precedes absent attribution/review. Missing input
/// refuses instead of inventing empty reports, no applicable review or success.
#[test]
fn mission_status_task_missing_causal_observations_refuse() {
    let task = task("t", "todo");
    let agent = Agent {
        id: "fixture-agent".into(),
        task_id: "t".into(),
        status: "completed".into(),
        ..Default::default()
    };
    let diagnostic = diagnostic();
    let observed = AgentObservation {
        agent: &agent,
        observed_state: "completed",
        fallback_diagnostic: &diagnostic,
        environment_recovery: false,
    };
    let mut facts = result_facts();
    assert_eq!(
        result_presentation(&task, Some(&observed), &facts)
            .unwrap()
            .state,
        "completed_unproven"
    );
    facts.attribution = ReportAttribution::NotObserved;
    assert!(matches!(
        result_presentation(&task, Some(&observed), &facts),
        Err(E::Refused("mission-status-report-attribution-unobserved"))
    ));
    facts.attribution = ReportAttribution::Unavailable {
        reason: "fixture".into(),
    };
    assert_eq!(
        result_presentation(&task, Some(&observed), &facts)
            .unwrap()
            .state,
        "completed_unproven"
    );
    facts.review = ReviewObservation::NotObserved;
    assert!(matches!(
        result_presentation(&task, Some(&observed), &facts),
        Err(E::Refused("mission-status-review-unobserved"))
    ));
}

/// Applicable review preserves failed process identity; an injected validated
/// result is rejected rather than letting review bypass the acceptance branch.
#[test]
fn mission_status_task_review_cannot_rewrite_process_or_acceptance() {
    let task = task("t", "blocked");
    let agent = Agent {
        id: "a".into(),
        task_id: "t".into(),
        status: "failed".into(),
        ..Default::default()
    };
    let diagnostic = diagnostic();
    let observed = AgentObservation {
        agent: &agent,
        observed_state: "failed",
        fallback_diagnostic: &diagnostic,
        environment_recovery: false,
    };
    let mut facts = result_facts();
    facts.review = ReviewObservation::Presented(ReviewPresentation {
        state: "review_blocked".into(),
        label: "fixture".into(),
        validation_state: "review_unvalidated".into(),
        reason: "fixture-refus".into(),
        next_step: "fixture".into(),
        report_id: "r".into(),
        receipt_id: "receipt".into(),
        report_state: "submitted".into(),
    });
    let review = result_presentation(&task, Some(&observed), &facts).unwrap();
    assert_eq!(review.state, "review_blocked");
    assert_eq!(review.process_state, "failed");
    assert_eq!(review.reason, "fixture-refus");
    if let ReviewObservation::Presented(review) = &mut facts.review {
        review.state = "validated".into();
    }
    assert!(matches!(
        result_presentation(&task, Some(&observed), &facts),
        Err(E::Refused("mission-status-review-result-invalid"))
    ));
}

/// All task observations are supplied before counting. Completion remains an
/// intervention; submitted result contributes to review, never validated/running.
#[test]
fn mission_status_task_projection_counts_results_and_requires_complete_inputs() {
    let work = Work {
        id: "w".into(),
        tasks: Some(vec![task("t", "submitted")]),
        ..Default::default()
    };
    let context = crate::mission_status_contract::tests::status();
    let facts = facts("t");
    let projected = project_tasks(&work, &context, std::slice::from_ref(&facts), &[]).unwrap();
    assert_eq!(
        (projected.validated, projected.review, projected.running),
        (0, 1, 0)
    );
    assert_eq!(projected.tasks[0].state, "review");
    assert_eq!(projected.tasks[0].result.validation_state, "missing_gate");
    assert!(matches!(
        project_tasks(&work, &context, &[], &[]),
        Err(E::Refused("mission-status-task-observations-mismatch"))
    ));
    let mut changed = facts;
    changed.prepared_launch = PreparedLaunchObservation::NotObserved;
    assert!(matches!(
        project_tasks(&work, &context, &[changed], &[]),
        Err(E::Refused("mission-status-prepared-launch-unobserved"))
    ));
}

/// A direct prerequisite uses its observed freshness and original ID. Unknown
/// dependency/readiness inputs are not converted into waiting, ready or zero.
#[test]
fn mission_status_task_dependency_and_readiness_observations_are_mandatory() {
    let mut child = task("t", "todo");
    child.depends = Some(vec!["p".into()]);
    let work = Work {
        id: "w".into(),
        tasks: Some(vec![task("p", "waived"), child]),
        ..Default::default()
    };
    let context = crate::mission_status_contract::tests::status();
    let parent = facts("p");
    let mut child = facts("t");
    child.dependencies = vec![DependencyObservation {
        task_id: "p".into(),
        freshness: DependencyFreshness::NotCurrent,
    }];
    let projected = project_tasks(&work, &context, &[parent.clone(), child.clone()], &[]).unwrap();
    assert_eq!(projected.tasks[1].state, "waiting");
    assert_eq!(projected.tasks[1].target, "p");
    child.dependencies[0].freshness = DependencyFreshness::NotObserved;
    assert!(matches!(
        project_tasks(&work, &context, &[parent.clone(), child.clone()], &[]),
        Err(E::Refused("mission-status-dependency-unobserved"))
    ));
    child.dependencies[0].freshness = DependencyFreshness::Current;
    let valid = project_tasks(&work, &context, &[parent.clone(), child.clone()], &[]).unwrap();
    assert_eq!(valid.tasks[1].state, "configure");
    child.readiness = ReadinessObservation::NotObserved;
    assert!(matches!(
        project_tasks(&work, &context, &[parent, child], &[]),
        Err(E::Refused("mission-status-readiness-unobserved"))
    ));
}

/// A controls-stale observation changes review causality only; the previously
/// attributed report and receipt survive and no task becomes accepted.
#[test]
fn mission_status_controls_stale_keeps_report_and_receipt() {
    let mut task = task("t", "submitted");
    task.next = "handoff : report.md".into();
    task.automatic_validation = Some(Box::new(crate::AutomaticValidation {
        receipt: "receipt.json".into(),
        ..Default::default()
    }));
    let agent = Agent {
        id: "fixture-agent".into(),
        task_id: "t".into(),
        attempt_id: "fixture-attempt".into(),
        status: "completed".into(),
        ..Default::default()
    };
    let diagnostic = diagnostic();
    let observed = AgentObservation {
        agent: &agent,
        observed_state: "completed",
        fallback_diagnostic: &diagnostic,
        environment_recovery: false,
    };
    let mut facts = result_facts();
    facts.review = ReviewObservation::ControlsStale {
        reason: "current-validation-required".into(),
    };
    let result = result_presentation(&task, Some(&observed), &facts).unwrap();
    assert_eq!(result.state, "review_blocked");
    assert_eq!(result.report_id, "report.md");
    assert_eq!(result.receipt_id, "receipt.json");
    assert_eq!(result.validation_state, "failed_or_stale");
}
