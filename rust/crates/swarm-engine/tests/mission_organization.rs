//! Pure organization contracts; declared metadata is not authenticated authority.
use swarm_engine::{
    decode_task, decode_work, validate_mission_organization, validate_mission_reviewer_capacity,
    Work,
};

/// Canonical shape from Go TestOrganizationChecksRolesAndValidation, with one
/// requirement, configured reviewer and an actual control description; no tool runs.
fn organization() -> Work {
    decode_work(br#"{"schema_version":1,"id":"work","revision":1,"criteria":["test"],"planning":{"version":1,"max_tasks":10,"max_decisions":10,"max_activations":10,"provider":"fixture","scopes":[{"id":"root","revision":1,"requirements":["req-1"]}],"checks":{"req-1":[{"id":"objective-check","command":["git","diff","--exit-code"],"criteria":[1],"justification":"Fixture control","timeout_seconds":10}]},"reviewer":{"provider":"fixture-reviewer","authorized":"2026-10-20T00:00:00Z","max_calls":10}}}"#).unwrap()
}

/// An actual configured witness precedes omitted/partial reviewer cases. Declared
/// human review never silently substitutes for configured independent review.
#[test]
fn organization_requires_independent_reviewer_even_with_declared_human_review() {
    let valid = organization();
    validate_mission_organization(&valid).unwrap();
    let mut absent = valid.clone();
    absent.planning = None;
    assert_eq!(
        validate_mission_organization(&absent).unwrap_err().reason(),
        "organization-planning-missing"
    );
    let mut human = valid.clone();
    human.planning.as_mut().unwrap().human_review_authorized = "declared".into();
    human.planning.as_mut().unwrap().reviewer = None;
    assert_eq!(
        validate_mission_organization(&human).unwrap_err().reason(),
        "organization-reviewer-missing"
    );
    for field in ["provider", "authorized", "max_calls"] {
        let mut broken = valid.clone();
        let r = broken.planning.as_mut().unwrap().reviewer.as_mut().unwrap();
        match field {
            "provider" => r.provider.clear(),
            "authorized" => r.authorized.clear(),
            _ => r.max_calls = 0,
        }
        let error = validate_mission_organization(&broken).unwrap_err();
        assert_eq!(error.code(), "organization_incomplete");
        assert_eq!(error.reason(), "organization-reviewer-missing");
    }
    validate_mission_organization(&valid).unwrap();
}

/// Hierarchy/requirement/task validation failures retain a cause or explicit
/// boundary. Human requirement review does not remove task validation ownership.
#[test]
fn organization_checks_hierarchy_controls_and_task_validation() {
    use std::error::Error;
    let valid = organization();
    validate_mission_organization(&valid).unwrap();
    let mut no_planner = valid.clone();
    no_planner.planning.as_mut().unwrap().provider.clear();
    assert_eq!(
        validate_mission_organization(&no_planner)
            .unwrap_err()
            .reason(),
        "organization-planner-missing"
    );
    let mut no_checks = valid.clone();
    no_checks.planning.as_mut().unwrap().checks = None;
    assert_eq!(
        validate_mission_organization(&no_checks)
            .unwrap_err()
            .reason(),
        "organization-requirement-controls-missing"
    );
    no_checks.planning.as_mut().unwrap().human_review_authorized = "declared".into();
    validate_mission_organization(&no_checks).unwrap();
    let task = decode_task(
        br#"{"id":"task","scope_id":"root","requirements":["req-1"],"criteria":["test"]}"#,
    )
    .unwrap();
    let mut with_task = valid.clone();
    with_task.tasks = Some(vec![task]);
    assert_eq!(
        validate_mission_organization(&with_task)
            .unwrap_err()
            .reason(),
        "organization-task-validation-unauthorized"
    );
    with_task.tasks=Some(vec![decode_task(br#"{"id":"task","scope_id":"root","requirements":["req-1"],"criteria":["test"],"validation_policy":{"mode":"automatic","authorized_at":"declared","controls":[{"id":"objective-check","command":["git","diff","--exit-code"],"criteria":[1],"justification":"Fixture control","timeout_seconds":10}]}}"#).unwrap()]);
    validate_mission_organization(&with_task).unwrap();
    let mut missing_scope = with_task.clone();
    missing_scope.tasks.as_mut().unwrap()[0].scope_id = "missing".into();
    let error = validate_mission_organization(&missing_scope).unwrap_err();
    assert_eq!(error.code(), "organization_incomplete");
    assert!(error.source().is_some());
    let mut uncovered = with_task.clone();
    uncovered.tasks.as_mut().unwrap()[0].criteria = Some(vec!["first".into(), "second".into()]);
    let error = validate_mission_organization(&uncovered).unwrap_err();
    assert_eq!(error.reason(), "organization-task-validation-incomplete");
    assert!(error.source().is_some());
    validate_mission_organization(&with_task).unwrap();
}

/// Structural readiness differs from capacity; exact budget exhaustion and
/// invalid consumption refuse without resetting counters. Private failure text
/// remains available to the trusted caller but never appears in Debug/Display.
#[test]
fn organization_separates_reviewer_capacity_failure_and_consumption() {
    let valid = organization();
    let p = valid.planning.as_ref().unwrap();
    validate_mission_organization(&valid).unwrap();
    validate_mission_reviewer_capacity(p).unwrap();
    let mut boundary = valid.clone();
    let p = boundary.planning.as_mut().unwrap();
    p.reviewer.as_mut().unwrap().calls = 9;
    validate_mission_reviewer_capacity(p).unwrap();
    p.reviewer.as_mut().unwrap().calls = 10;
    assert_eq!(
        validate_mission_reviewer_capacity(p).unwrap_err().reason(),
        "reviewer-budget-exhausted"
    );
    assert_eq!(p.reviewer.as_ref().unwrap().calls, 10);
    validate_mission_organization(&boundary).unwrap();
    let mut invalid = valid.clone();
    let p = invalid.planning.as_mut().unwrap();
    p.reviewer.as_mut().unwrap().calls = -1;
    assert_eq!(
        validate_mission_reviewer_capacity(p).unwrap_err().reason(),
        "reviewer-counter-invalid"
    );
    assert_eq!(p.reviewer.as_ref().unwrap().calls, -1);
    let mut failed = valid.clone();
    let p = failed.planning.as_mut().unwrap();
    p.reviewer.as_mut().unwrap().failure = "PRIVATE original cause".into();
    let error = validate_mission_reviewer_capacity(p).unwrap_err();
    assert_eq!(error.code(), "reviewer_unavailable");
    assert_eq!(error.recorded_failure(), Some("PRIVATE original cause"));
    assert!(!format!("{error:?} {error}").contains("PRIVATE"));
    validate_mission_organization(&failed).unwrap();
    validate_mission_reviewer_capacity(valid.planning.as_ref().unwrap()).unwrap();
}
