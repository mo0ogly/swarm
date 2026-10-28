use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
use swarm_engine::{
    transition_manual_task, Attempt, EngineSettingsDocument, EngineSettingsValues, Request,
    SystemEnvironment, Task, TaskLifecycleContext, Work,
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let p = std::env::temp_dir().join(swarm_engine::generate_identifier("lifecycle-").unwrap());
        fs::create_dir(&p).unwrap();
        fs::write(p.join("proof.txt"), b"proof").unwrap();
        Self(p)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap()
        }
    }
}
fn work() -> Work {
    Work {
        schema_version: 1,
        id: "work".into(),
        tasks: Some(vec![Task {
            id: "task".into(),
            title: "Task".into(),
            deliverable: "Report".into(),
            criteria: Some(vec!["Proof".into()]),
            status: "todo".into(),
            attempts: Some(vec![]),
            ..Default::default()
        }]),
        ..Default::default()
    }
}
fn request(status: &str) -> Request {
    Request {
        schema_version: 1,
        event_id: "event".into(),
        expected_revision: 1,
        id: "task".into(),
        status: status.into(),
        ..Default::default()
    }
}
#[test]
fn manual_lifecycle_records_real_stamps_and_requires_gate_before_acceptance() {
    let f = Fixture::new();
    let doc = EngineSettingsDocument::initial(
        EngineSettingsValues::installation_defaults().unwrap(),
        "operator".into(),
        "2026-10-18T00:00:00Z".into(),
    )
    .unwrap();
    let profile = doc.quality_policy_snapshot().unwrap();
    let limits = doc.values.mutation_limits().unwrap();
    let context = TaskLifecycleContext {
        root: f.path(),
        quality_policy: Some(&profile),
        task_limits: &limits.task,
        validation_limits: &limits.policy,
    };
    let mut env = SystemEnvironment;
    let mut w = work();
    let original = swarm_engine::encode_model(&w).unwrap();
    let started = transition_manual_task(&w, &request("running"), &context, &mut env).unwrap();
    assert_eq!(swarm_engine::encode_model(&w).unwrap(), original);
    assert_eq!(started.attempts.as_ref().unwrap().len(), 1);
    assert_eq!(started.attempts.as_ref().unwrap()[0].status, "recorded");
    assert!(!started.attempts.as_ref().unwrap()[0].started.is_empty());
    w.tasks.as_mut().unwrap()[0] = started;
    let mut submitted = request("submitted");
    submitted.outcome = "completed".into();
    let finished = transition_manual_task(&w, &submitted, &context, &mut env).unwrap();
    assert_eq!(finished.attempts.as_ref().unwrap()[0].status, "completed");
    assert!(!finished.attempts.as_ref().unwrap()[0].ended.is_empty());
    w.tasks.as_mut().unwrap()[0] = finished;
    assert!(transition_manual_task(&w, &request("accepted"), &context, &mut env).is_err());
    let raw=serde_json::to_vec(&serde_json::json!({"method_version":"3","scope_id":"task","artifacts":{"proof.txt":format!("{:x}",Sha256::digest(b"proof"))},"domains":{"quality":1},"checks":[{"id":"check","domain":"quality","mandatory":true,"gate":"delivery","severity":"minor","penalty":1,"max_penalty":100}],"results":[{"id":"check","status":"PASS","count":0,"evidence":["proof.txt"]}]})).unwrap();
    let saved = swarm_engine::prepare_gate_record(
        &w.tasks.as_ref().unwrap()[0],
        &swarm_engine::GateSubmission {
            name: "Proof",
            phase: "delivery",
            document: &raw,
            at: "2026-10-18T00:00:00Z",
        },
        f.path(),
        &profile,
    )
    .unwrap();
    w.tasks.as_mut().unwrap()[0] = saved;
    let accepted = transition_manual_task(&w, &request("accepted"), &context, &mut env).unwrap();
    assert_eq!(accepted.status, "accepted");
    w.tasks.as_mut().unwrap()[0] = accepted;
    fs::write(f.path().join("proof.txt"), b"changed").unwrap();
    assert!(transition_manual_task(&w, &request("accepted"), &context, &mut env).is_err());
    let reopened = transition_manual_task(&w, &request("todo"), &context, &mut env).unwrap();
    assert!(reopened.gate.is_none());
    assert!(reopened.revalidation.is_some());
    assert_eq!(reopened.attempts.as_ref().unwrap().len(), 1);
}
#[test]
fn manual_lifecycle_refuses_missing_attempt_and_illegal_status_without_mutation() {
    let f = Fixture::new();
    let values = EngineSettingsValues::installation_defaults().unwrap();
    let limits = values.mutation_limits().unwrap();
    let context = TaskLifecycleContext {
        root: f.path(),
        quality_policy: None,
        task_limits: &limits.task,
        validation_limits: &limits.policy,
    };
    let mut env = SystemEnvironment;
    for status in ["todo", "submitted", "blocked", "abandoned"] {
        let mut w = work();
        w.tasks.as_mut().unwrap()[0].status = "running".into();
        let mut r = request(status);
        r.outcome = "completed".into();
        r.blocker = "Known issue".into();
        let mut good = w.clone();
        good.tasks.as_mut().unwrap()[0].attempts = Some(vec![Attempt {
            id: "a-control".into(),
            status: "recorded".into(),
            started: "2026-10-18T00:00:00Z".into(),
            ..Default::default()
        }]);
        transition_manual_task(&good, &r, &context, &mut env).unwrap();
        let before = swarm_engine::encode_model(&w).unwrap();
        let error = transition_manual_task(&w, &r, &context, &mut env)
            .expect_err("incoherent running state must refuse");
        assert_eq!(error.code(), "attempt_state_invalid");
        assert_eq!(swarm_engine::encode_model(&w).unwrap(), before);
    }
    for status in ["accepted", "waived", "UNKNOWN", "running blocked"] {
        let w = work();
        let before = swarm_engine::encode_model(&w).unwrap();
        assert!(transition_manual_task(&w, &request(status), &context, &mut env).is_err());
        assert_eq!(swarm_engine::encode_model(&w).unwrap(), before);
    }
}

#[test]
fn manual_dependencies_require_fresh_evidence_and_reject_cycles_or_missing_nodes() {
    let f = Fixture::new();
    let doc = EngineSettingsDocument::initial(
        EngineSettingsValues::installation_defaults().unwrap(),
        "operator".into(),
        "2026-10-18T00:00:00Z".into(),
    )
    .unwrap();
    let profile = doc.quality_policy_snapshot().unwrap();
    let limits = doc.values.mutation_limits().unwrap();
    let context = TaskLifecycleContext {
        root: f.path(),
        quality_policy: Some(&profile),
        task_limits: &limits.task,
        validation_limits: &limits.policy,
    };
    let mut env = SystemEnvironment;
    let mut w = work();
    let raw=serde_json::to_vec(&serde_json::json!({"method_version":"3","scope_id":"task","artifacts":{"proof.txt":format!("{:x}",Sha256::digest(b"proof"))},"domains":{"quality":1},"checks":[{"id":"check","domain":"quality","mandatory":true,"gate":"delivery","severity":"minor","penalty":1,"max_penalty":100}],"results":[{"id":"check","status":"PASS","count":0,"evidence":["proof.txt"]}]})).unwrap();
    let first = swarm_engine::prepare_gate_record(
        &w.tasks.as_ref().unwrap()[0],
        &swarm_engine::GateSubmission {
            name: "Dependency proof",
            phase: "delivery",
            document: &raw,
            at: "2026-10-18T00:00:00Z",
        },
        f.path(),
        &profile,
    )
    .unwrap();
    w.tasks.as_mut().unwrap()[0] = first;
    w.tasks.as_mut().unwrap()[0].status = "accepted".into();
    let mut next = work().tasks.unwrap().remove(0);
    next.id = "next".into();
    next.depends = Some(vec!["task".into()]);
    w.tasks.as_mut().unwrap().push(next);
    let mut r = request("running");
    r.id = "next".into();
    transition_manual_task(&w, &r, &context, &mut env).unwrap();
    fs::write(f.path().join("proof.txt"), b"changed").unwrap();
    assert!(transition_manual_task(&w, &r, &context, &mut env).is_err());
    fs::write(f.path().join("proof.txt"), b"proof").unwrap();
    transition_manual_task(&w, &r, &context, &mut env).unwrap();
    let mut missing = w.clone();
    missing.tasks.as_mut().unwrap()[1].depends = Some(vec!["missing".into()]);
    assert!(transition_manual_task(&missing, &r, &context, &mut env).is_err());
    let mut cyclic = w.clone();
    cyclic.tasks.as_mut().unwrap()[0].depends = Some(vec!["next".into()]);
    cyclic.tasks.as_mut().unwrap()[1].status = "waived".into();
    cyclic.tasks.as_mut().unwrap()[1].manual_override =
        Some(Box::new(swarm_engine::ManualOverride {
            reason: "Existing operator exception".into(),
            ..Default::default()
        }));
    assert!(swarm_engine::check_manual_task_freshness(&cyclic, "task", &context).is_err());
    let mut waived = w.clone();
    waived.tasks.as_mut().unwrap()[0].status = "waived".into();
    waived.tasks.as_mut().unwrap()[0].manual_override =
        Some(Box::new(swarm_engine::ManualOverride {
            reason: "Existing operator exception".into(),
            ..Default::default()
        }));
    transition_manual_task(&waived, &r, &context, &mut env).unwrap();
    waived.tasks.as_mut().unwrap()[0]
        .manual_override
        .as_mut()
        .unwrap()
        .reason = " ".into();
    assert!(transition_manual_task(&waived, &r, &context, &mut env).is_err());
    let mut hierarchical = w.clone();
    hierarchical.planning = Some(Default::default());
    hierarchical.tasks.as_mut().unwrap()[0].independent_review =
        Some(Box::new(swarm_engine::IndependentReview {
            state: "passed".into(),
            ..Default::default()
        }));
    assert_eq!(
        transition_manual_task(&hierarchical, &r, &context, &mut env)
            .err()
            .unwrap()
            .code(),
        "command_not_ported"
    );
}

#[test]
fn configured_business_prerequisite_cannot_be_bypassed_by_omitting_task_dependency() {
    let f = Fixture::new();
    let doc = EngineSettingsDocument::initial(
        EngineSettingsValues::installation_defaults().unwrap(),
        "operator".into(),
        "2026-10-18T00:00:00Z".into(),
    )
    .unwrap();
    let profile = doc.quality_policy_snapshot().unwrap();
    let limits = doc.values.mutation_limits().unwrap();
    let context = TaskLifecycleContext {
        root: f.path(),
        quality_policy: Some(&profile),
        task_limits: &limits.task,
        validation_limits: &limits.policy,
    };
    let mut w = work();
    w.criteria = Some(vec![
        "First requirement".into(),
        "Second requirement".into(),
    ]);
    w.requirement_prerequisites = Some(std::collections::BTreeMap::from([(
        "req-1".into(),
        Some(vec!["req-2".into()]),
    )]));
    let raw=serde_json::to_vec(&serde_json::json!({"method_version":"3","scope_id":"task","artifacts":{"proof.txt":format!("{:x}",Sha256::digest(b"proof"))},"domains":{"quality":1},"checks":[{"id":"proof","domain":"quality","mandatory":true,"gate":"delivery","severity":"minor","penalty":1,"max_penalty":100}],"results":[{"id":"proof","status":"PASS","count":0,"evidence":["proof.txt"]}]})).unwrap();
    let mut prior = swarm_engine::prepare_gate_record(
        &w.tasks.as_ref().unwrap()[0],
        &swarm_engine::GateSubmission {
            name: "Prerequisite proof",
            phase: "delivery",
            document: &raw,
            at: "2026-10-18T00:00:00Z",
        },
        f.path(),
        &profile,
    )
    .unwrap();
    prior.status = "accepted".into();
    prior.requirements = Some(vec!["req-2".into()]);
    w.tasks.as_mut().unwrap()[0] = prior;
    let mut next = work().tasks.unwrap().remove(0);
    next.id = "next".into();
    next.requirements = Some(vec!["req-1".into()]);
    next.depends = None;
    w.tasks.as_mut().unwrap().push(next);
    let mut r = request("running");
    r.id = "next".into();
    transition_manual_task(&w, &r, &context, &mut SystemEnvironment).unwrap();
    fs::write(f.path().join("proof.txt"), b"changed").unwrap();
    let error = transition_manual_task(&w, &r, &context, &mut SystemEnvironment)
        .err()
        .unwrap();
    assert_eq!(error.code(), "requirement_prerequisite");
    assert!(std::error::Error::source(&error).is_some());
    fs::write(f.path().join("proof.txt"), b"proof").unwrap();
    transition_manual_task(&w, &r, &context, &mut SystemEnvironment).unwrap();
    w.tasks.as_mut().unwrap()[1].requirements = None;
    assert_eq!(
        transition_manual_task(&w, &r, &context, &mut SystemEnvironment)
            .err()
            .unwrap()
            .code(),
        "requirement_prerequisite"
    );
}

/// Canonical valid hierarchy before each injected fault. No provider, budget,
/// process, permission or actual independent review is manufactured by this data.
fn scoped_work() -> Work {
    let mut w = work();
    w.revision = 1;
    w.criteria = Some(vec!["Proof".into()]);
    let task = &mut w.tasks.as_mut().unwrap()[0];
    task.scope_id = "child".into();
    task.requirements = Some(vec!["req-1".into()]);
    task.plan_role = "worker".into();
    w.planning=swarm_engine::decode_work(br#"{"schema_version":1,"id":"fixture","planning":{"version":1,"max_tasks":10,"max_decisions":10,"max_activations":10,"scopes":[{"id":"root","revision":1,"state":"ready"},{"id":"child","parent":"root","revision":1,"state":"ready","requirements":["req-1"]}]}}"#).unwrap().planning;
    swarm_engine::validate_planning_state(&w).unwrap();
    w
}
/// Scoped preparation records only a Task copy; caller still owns actual process,
/// policies, reservations/events and later review. Every outcome stays blocked.
#[test]
fn scoped_worker_transition_is_pure_and_finishes_each_outcome_without_acceptance() {
    let f = Fixture::new();
    let doc = EngineSettingsDocument::initial(
        EngineSettingsValues::installation_defaults().unwrap(),
        "fixture".into(),
        "2026-10-20T00:00:00Z".into(),
    )
    .unwrap();
    let quality = doc.quality_policy_snapshot().unwrap();
    let limits = doc.values.mutation_limits().unwrap();
    let c = TaskLifecycleContext {
        root: f.path(),
        quality_policy: Some(&quality),
        task_limits: &limits.task,
        validation_limits: &limits.policy,
    };
    let w = scoped_work();
    let before = swarm_engine::encode_model(&w).unwrap();
    let started = swarm_engine::transition_scoped_worker_task(
        &w,
        &request("running"),
        &c,
        &doc,
        &mut SystemEnvironment,
    )
    .unwrap();
    assert_eq!(swarm_engine::encode_model(&w).unwrap(), before);
    assert_eq!(started.status, "running");
    assert_eq!(started.attempts.as_ref().unwrap().len(), 1);
    for outcome in ["completed", "failed", "interrupted"] {
        let mut running = w.clone();
        running.tasks = Some(vec![started.clone()]);
        let original = swarm_engine::encode_model(&running).unwrap();
        let mut r = request("blocked");
        r.outcome = outcome.into();
        r.blocker = "Fixture process outcome requires review".into();
        let ended = swarm_engine::transition_scoped_worker_task(
            &running,
            &r,
            &c,
            &doc,
            &mut SystemEnvironment,
        )
        .unwrap();
        assert_eq!(ended.status, "blocked");
        let attempt = ended.attempts.as_ref().unwrap().last().unwrap();
        assert_eq!(attempt.status, outcome);
        assert!(!attempt.ended.is_empty());
        assert_eq!(swarm_engine::encode_model(&running).unwrap(), original);
    }
    assert!(matches!(
        transition_manual_task(&w, &request("running"), &c, &mut SystemEnvironment),
        Err(swarm_engine::TaskLifecycleError::HierarchyNotPorted)
    ));
}
/// Valid witness precedes closed/foreign-scope faults; scoped preparation never
/// turns a coding worker into an accepting controller or silently edits a contract.
#[test]
fn scoped_worker_closed_ownership_and_acceptance_refuse_then_recover() {
    use std::error::Error;
    let f = Fixture::new();
    let doc = EngineSettingsDocument::initial(
        EngineSettingsValues::installation_defaults().unwrap(),
        "fixture".into(),
        "2026-10-20T00:00:00Z".into(),
    )
    .unwrap();
    let quality = doc.quality_policy_snapshot().unwrap();
    let limits = doc.values.mutation_limits().unwrap();
    let c = TaskLifecycleContext {
        root: f.path(),
        quality_policy: Some(&quality),
        task_limits: &limits.task,
        validation_limits: &limits.policy,
    };
    let valid = scoped_work();
    swarm_engine::transition_scoped_worker_task(
        &valid,
        &request("running"),
        &c,
        &doc,
        &mut SystemEnvironment,
    )
    .unwrap();
    let mut closed = valid.clone();
    closed.planning.as_mut().unwrap().scopes.as_mut().unwrap()[1].state = "closed".into();
    assert_eq!(
        swarm_engine::transition_scoped_worker_task(
            &closed,
            &request("running"),
            &c,
            &doc,
            &mut SystemEnvironment
        )
        .unwrap_err()
        .reason(),
        "task-scope-closed"
    );
    let mut foreign = valid.clone();
    foreign.tasks.as_mut().unwrap()[0].scope_id = "unknown".into();
    let original = swarm_engine::encode_model(&foreign).unwrap();
    let e = swarm_engine::transition_scoped_worker_task(
        &foreign,
        &request("running"),
        &c,
        &doc,
        &mut SystemEnvironment,
    )
    .unwrap_err();
    assert!(e
        .source()
        .unwrap()
        .downcast_ref::<swarm_engine::PlanningStateError>()
        .is_some());
    assert_eq!(swarm_engine::encode_model(&foreign).unwrap(), original);
    assert!(matches!(
        swarm_engine::transition_scoped_worker_task(
            &valid,
            &request("accepted"),
            &c,
            &doc,
            &mut SystemEnvironment
        ),
        Err(swarm_engine::TaskLifecycleError::HierarchyNotPorted)
    ));
    swarm_engine::transition_scoped_worker_task(
        &valid,
        &request("running"),
        &c,
        &doc,
        &mut SystemEnvironment,
    )
    .unwrap();
}
/// Manual SDK preserves its original validation precedence for malformed input,
/// including hierarchical Work that otherwise returns an unported refusal.
#[test]
fn manual_hierarchy_validates_request_before_unported_refusal() {
    let f = Fixture::new();
    let doc = EngineSettingsDocument::initial(
        EngineSettingsValues::installation_defaults().unwrap(),
        "fixture".into(),
        "2026-10-20T00:00:00Z".into(),
    )
    .unwrap();
    let limits = doc.values.mutation_limits().unwrap();
    let c = TaskLifecycleContext {
        root: f.path(),
        quality_policy: None,
        task_limits: &limits.task,
        validation_limits: &limits.policy,
    };
    let w = scoped_work();
    let mut invalid = request("running");
    invalid.schema_version = 0;
    assert!(matches!(
        transition_manual_task(&w, &invalid, &c, &mut SystemEnvironment),
        Err(swarm_engine::TaskLifecycleError::Definition(_))
    ));
}

/// An actual accepted dependency has fresh independent-review bytes plus a
/// separately prepared gate. Each fault follows the same valid witness; review,
/// gate and contract failures cannot mask each other or mutate the Work input.
#[test]
fn scoped_dependency_requires_current_independent_review_and_gate_then_recovers() {
    use std::collections::BTreeMap;
    let f = Fixture::new();
    fs::write(f.path().join("review.md"), b"Recorded review fixture").unwrap();
    fs::write(f.path().join("reviewed-delivery.md"), b"Observed delivery").unwrap();
    let sha = |bytes: &[u8]| format!("{:x}", Sha256::digest(bytes));
    let doc = EngineSettingsDocument::initial(
        EngineSettingsValues::installation_defaults().unwrap(),
        "fixture".into(),
        "2026-10-20T00:00:00Z".into(),
    )
    .unwrap();
    let quality = doc.quality_policy_snapshot().unwrap();
    let limits = doc.values.mutation_limits().unwrap();
    let c = TaskLifecycleContext {
        root: f.path(),
        quality_policy: Some(&quality),
        task_limits: &limits.task,
        validation_limits: &limits.policy,
    };
    let mut w = scoped_work();
    w.planning.as_mut().unwrap().reviewer = Some(Box::new(swarm_engine::ReviewerConfig {
        provider: "fixture-reviewer".into(),
        authorized: "2026-10-20T00:00:00Z".into(),
        max_calls: 10,
        ..Default::default()
    }));
    let raw=serde_json::to_vec(&serde_json::json!({"method_version":"3","scope_id":"task","artifacts":{"proof.txt":sha(b"proof")},"domains":{"quality":1},"checks":[{"id":"check","domain":"quality","mandatory":true,"gate":"delivery","severity":"minor","penalty":1,"max_penalty":100}],"results":[{"id":"check","status":"PASS","count":0,"evidence":["proof.txt"]}]})).unwrap();
    let mut prior = swarm_engine::prepare_gate_record(
        &w.tasks.as_ref().unwrap()[0],
        &swarm_engine::GateSubmission {
            name: "Dependency gate",
            phase: "delivery",
            document: &raw,
            at: "2026-10-20T00:00:00Z",
        },
        f.path(),
        &quality,
    )
    .unwrap();
    prior.status = "accepted".into();
    prior.attempts = Some(vec![Attempt {
        id: "prior-attempt".into(),
        status: "completed".into(),
        started: "2026-10-20T00:00:00Z".into(),
        ended: "2026-10-20T00:00:01Z".into(),
    }]);
    prior.independent_review = Some(Box::new(swarm_engine::IndependentReview {
        id: "review".into(),
        attempt: "prior-attempt".into(),
        contract: swarm_engine::review_contract(&prior).unwrap(),
        state: "passed".into(),
        report: "review.md".into(),
        sha256: sha(b"Recorded review fixture"),
        report_artifacts: Some(BTreeMap::from([(
            "reviewed-delivery.md".into(),
            sha(b"Observed delivery"),
        )])),
        ..Default::default()
    }));
    let mut next = scoped_work().tasks.unwrap().remove(0);
    next.id = "next".into();
    next.depends = Some(vec!["task".into()]);
    w.tasks = Some(vec![prior, next]);
    let mut r = request("running");
    r.id = "next".into();
    let check = |work: &Work| {
        swarm_engine::transition_scoped_worker_task(work, &r, &c, &doc, &mut SystemEnvironment)
    };
    let before = swarm_engine::encode_model(&w).unwrap();
    check(&w).unwrap();
    fs::write(f.path().join("review.md"), b"Changed review").unwrap();
    assert!(matches!(
        check(&w),
        Err(swarm_engine::TaskLifecycleError::Review(_))
    ));
    fs::write(f.path().join("review.md"), b"Recorded review fixture").unwrap();
    check(&w).unwrap();
    fs::write(f.path().join("proof.txt"), b"Changed proof").unwrap();
    swarm_engine::check_current_review_evidence(f.path(), &w, "task", &doc)
        .unwrap()
        .unwrap();
    assert!(matches!(
        check(&w),
        Err(swarm_engine::TaskLifecycleError::Gate(_))
    ));
    fs::write(f.path().join("proof.txt"), b"proof").unwrap();
    check(&w).unwrap();
    let mut changed = w.clone();
    changed.tasks.as_mut().unwrap()[0].title = "Changed contract".into();
    assert!(matches!(
        check(&changed),
        Err(swarm_engine::TaskLifecycleError::Review(_))
    ));
    check(&w).unwrap();
    assert_eq!(swarm_engine::encode_model(&w).unwrap(), before);
}

/// The same pure lifecycle first starts and stops a valid scoped task. Re-entry
/// must then require the internal persisted retry gate; no process/provider,
/// SQL identity or authenticated supervisor decision is invented by these data.
#[test]
fn scoped_blocked_worker_requires_retry_gate_without_mutating_history() {
    let f = Fixture::new();
    let doc = EngineSettingsDocument::initial(EngineSettingsValues::installation_defaults().unwrap(), "fixture".into(), "2026-10-22T12:00:00Z".into()).unwrap();
    let quality = doc.quality_policy_snapshot().unwrap();let limits = doc.values.mutation_limits().unwrap();
    let c = TaskLifecycleContext { root: f.path(), quality_policy: Some(&quality), task_limits: &limits.task, validation_limits: &limits.policy };
    let mut w = scoped_work();
    let started = swarm_engine::transition_scoped_worker_task(&w, &request("running"), &c, &doc, &mut SystemEnvironment).unwrap();
    w.tasks = Some(vec![started]);let mut stop = request("blocked");stop.outcome = "failed".into();stop.blocker = "Actual lifecycle stop requires supervisor correction".into();
    let ended = swarm_engine::transition_scoped_worker_task(&w, &stop, &c, &doc, &mut SystemEnvironment).unwrap();
    w.tasks = Some(vec![ended]);let original = swarm_engine::encode_model(&w).unwrap();
    let error = swarm_engine::transition_scoped_worker_task(&w, &request("running"), &c, &doc, &mut SystemEnvironment).expect_err("blocked restart must require a supervisor decision");
    assert_eq!(error.reason(), "scoped-worker-retry-decision-required");
    assert_eq!(swarm_engine::encode_model(&w).unwrap(), original);
    assert_eq!(w.tasks.as_ref().unwrap()[0].attempts.as_ref().unwrap().len(), 1);
    // A structural signal alone cannot invent a Root original/decision. Actual
    // successful replay is tested through the public native worker recipe.
    let mut signalled = w.clone();signalled.tasks.as_mut().unwrap()[0].planning_retry = true;
    signalled.tasks.as_mut().unwrap()[0].plan_max_attempts = 2;
    let error = swarm_engine::transition_scoped_worker_task(&signalled, &request("running"), &c, &doc, &mut SystemEnvironment).expect_err("synthetic signal without actual Root proof must refuse");
    assert!(matches!(error, swarm_engine::TaskLifecycleError::Review(swarm_engine::ReviewEvidenceError::Native(_))));
    assert_eq!(signalled.tasks.as_ref().unwrap()[0].attempts.as_ref().unwrap().len(), 1);
    assert_eq!(swarm_engine::encode_model(&w).unwrap(), original);
    let mut exhausted = signalled.clone();exhausted.tasks.as_mut().unwrap()[0].plan_max_attempts = 1;
    assert_eq!(swarm_engine::transition_scoped_worker_task(&exhausted, &request("running"), &c, &doc, &mut SystemEnvironment).err().unwrap().reason(), "scoped-worker-plan-attempts-exhausted");
    for state in ["error", "running", "queued"] {
        for signal in [false, true] {
            let mut protocol = signalled.clone();protocol.tasks.as_mut().unwrap()[0].planning_retry = signal;
            protocol.tasks.as_mut().unwrap()[0].independent_review = Some(Box::new(swarm_engine::IndependentReview { state: state.into(), ..Default::default() }));
            let before = swarm_engine::encode_model(&protocol).unwrap();
            assert_eq!(swarm_engine::transition_scoped_worker_task(&protocol, &request("running"), &c, &doc, &mut SystemEnvironment).err().unwrap().reason(), "scoped-worker-review-recovery-required");
            assert_eq!(swarm_engine::encode_model(&protocol).unwrap(), before);
        }
    }
}
