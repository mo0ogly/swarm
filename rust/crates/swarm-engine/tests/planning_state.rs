use std::collections::BTreeMap;
use swarm_engine::{
    apply_planning_validation_signals, decode_work, planning_validation_event_id,
    validate_judge_state, validate_planning_state, validate_requirement_prerequisites,
    PlanningRefusal, PlanningStateError, Work,
};
fn fixture() -> Work {
    decode_work(br#"{"schema_version":1,"id":"work","revision":1,"criteria":["Global"],"tasks":[{"id":"task","scope_id":"child","plan_role":"worker","requirements":["req-1"],"status":"accepted","attempts":[{"id":"last"}]}],"planning":{"version":1,"max_tasks":1,"max_decisions":1,"max_activations":1,"scopes":[{"id":"root","revision":1,"state":"closed"},{"id":"child","parent":"root","revision":1,"requirements":["req-1"],"state":"closed"}]}}"#).unwrap()
}
#[test]
fn legacy_work_without_planning_or_prerequisites_remains_valid() {
    let w = decode_work(br#"{"schema_version":1,"id":"work","revision":1,"tasks":[]}"#).unwrap();
    validate_planning_state(&w).unwrap();
}

#[test]
fn planning_guards_are_discriminating_and_read_only() {
    validate_planning_state(&fixture()).unwrap();
    for case in 0..18 {
        let mut w = fixture();
        let p = w.planning.as_mut().unwrap();
        let expected = match case {
            0 => {
                p.version = 0;
                PlanningRefusal::Policy
            }
            1 => {
                p.max_scopes = 1;
                PlanningRefusal::Policy
            }
            2 => {
                p.activations = -1;
                PlanningRefusal::Policy
            }
            3 => {
                p.decisions = 2;
                PlanningRefusal::Policy
            }
            4 => {
                p.max_depth = 1;
                PlanningRefusal::Depth
            }
            5 => {
                p.scopes.as_mut().unwrap()[1].revision = 0;
                PlanningRefusal::Scope
            }
            6 => {
                p.scopes.as_mut().unwrap()[1].id = "root".into();
                PlanningRefusal::Scope
            }
            7 => {
                p.scopes.as_mut().unwrap()[0].requirements = Some(vec!["req-1".into()]);
                PlanningRefusal::RequirementOwner
            }
            8 => {
                p.scopes.as_mut().unwrap()[0].parent = "child".into();
                PlanningRefusal::Root
            }
            9 => {
                p.scopes.as_mut().unwrap()[1].parent = "absent".into();
                PlanningRefusal::Parent
            }
            10 => {
                p.scopes.as_mut().unwrap()[1].parent = "child".into();
                PlanningRefusal::Cycle
            }
            11 => {
                w.criteria = Some(vec![]);
                PlanningRefusal::RequirementCoverage
            }
            12 => {
                w.tasks.as_mut().unwrap()[0].scope_id = "absent".into();
                PlanningRefusal::TaskScope
            }
            13 => {
                w.tasks.as_mut().unwrap()[0].plan_role = "planner".into();
                PlanningRefusal::TaskRole
            }
            14 => {
                w.tasks.as_mut().unwrap()[0].requirements = Some(vec![]);
                PlanningRefusal::TaskRequirements
            }
            15 => {
                w.tasks.as_mut().unwrap()[0].requirements = Some(vec!["req-2".into()]);
                PlanningRefusal::TaskOwnership
            }
            16 => {
                p.inbox = Some(vec![swarm_engine::PlanningEvent {
                    id: "../bad".into(),
                    scope: "root".into(),
                    ..Default::default()
                }]);
                PlanningRefusal::Event
            }
            _ => {
                p.reviewer = Some(Box::new(swarm_engine::ReviewerConfig {
                    input_context_tokens: -1,
                    ..Default::default()
                }));
                PlanningRefusal::ReviewerContext
            }
        };
        let original = w.clone();
        let e = validate_planning_state(&w).unwrap_err();
        assert!(
            matches!(e,PlanningStateError::Refused(r)if r==expected),
            "case {case}: {e}"
        );
        assert_eq!(w, original);
    }
}

#[test]
fn prerequisites_walk_iteratively_and_refuse_cycles_unknowns_and_duplicates() {
    let mut w = Work {
        criteria: Some(vec!["Criterion".into(); 2048]),
        ..Default::default()
    };
    let mut edges = BTreeMap::new();
    for i in 1..2048 {
        edges.insert(format!("req-{i}"), Some(vec![format!("req-{}", i + 1)]));
    }
    w.requirement_prerequisites = Some(edges);
    validate_requirement_prerequisites(&w).unwrap();
    for (last, reason) in [
        (vec!["req-1".into()], PlanningRefusal::PrerequisiteCycle),
        (vec!["missing".into()], PlanningRefusal::PrerequisiteUnknown),
        (
            vec!["req-1".into(), "req-1".into()],
            PlanningRefusal::PrerequisiteDuplicate,
        ),
    ] {
        let mut invalid = w.clone();
        invalid
            .requirement_prerequisites
            .as_mut()
            .unwrap()
            .insert("req-2048".into(), Some(last));
        assert!(
            matches!(validate_requirement_prerequisites(&invalid),Err(PlanningStateError::Refused(r))if r==reason)
        );
        validate_requirement_prerequisites(&w).unwrap();
    }
}

#[test]
fn deep_tree_uses_configured_depth_without_call_stack_or_hidden_cutoff() {
    let mut w = fixture();
    let p = w.planning.as_mut().unwrap();
    p.max_depth = 0;
    for i in 0..20000 {
        p.scopes
            .as_mut()
            .unwrap()
            .push(swarm_engine::PlanningScope {
                id: format!("scope-{i}"),
                parent: if i == 0 {
                    "root".into()
                } else {
                    format!("scope-{}", i - 1)
                },
                revision: 1,
                ..Default::default()
            });
    }
    validate_planning_state(&w).unwrap();
    w.planning.as_mut().unwrap().max_depth = 20001;
    validate_planning_state(&w).unwrap();
    w.planning.as_mut().unwrap().max_depth = 20000;
    assert!(matches!(
        validate_planning_state(&w),
        Err(PlanningStateError::Refused(PlanningRefusal::Depth))
    ));
}

#[test]
fn judge_history_checks_verdict_renewal_dates_and_reasons_without_mutation() {
    let raw=br#"{"schema_version":1,"planning":{"judge":{"cycle":2,"max_cycles":0,"history":[{"cycle":1,"verdict":"continue","reason":"Renew","at":"2026-10-18T1:00:00,123Z","renewed_at":"2026-10-18T02:00:00+24:60"}]}}}"#;
    let w = decode_work(raw).unwrap();
    let j = w.planning.as_ref().unwrap().judge.as_deref().unwrap();
    validate_judge_state(Some(j)).unwrap();
    validate_judge_state(None).unwrap();
    for case in 0..7 {
        let mut bad = j.clone();
        match case {
            0 => bad.cycle = 0,
            1 => bad.max_cycles = -1,
            2 => bad.closed = true,
            3 => bad.history.as_mut().unwrap()[0].renewed_at.clear(),
            4 => bad.history.as_mut().unwrap()[0].at = "invalid".into(),
            5 => bad.history.as_mut().unwrap()[0].reason = " ".into(),
            _ => bad.history.as_mut().unwrap()[0].verdict = "end".into(),
        };
        assert!(validate_judge_state(Some(&bad)).is_err());
        validate_judge_state(Some(j)).unwrap();
    }
}

#[test]
fn accepted_changes_emit_ordered_signals_and_reopen_owner_and_ancestors() {
    let mut w = fixture();
    let before = BTreeMap::from([("task".into(), "running".into())]);
    apply_planning_validation_signals(&mut w, &before, "accepted-change", "clock-fixture").unwrap();
    let p = w.planning.as_ref().unwrap();
    assert!(p
        .scopes
        .as_ref()
        .unwrap()
        .iter()
        .all(|s| s.state == "ready"));
    let e = &p.inbox.as_ref().unwrap()[0];
    assert_eq!(
        e.id,
        planning_validation_event_id("accepted-change", "task").unwrap()
    );
    assert_eq!(
        (
            &*e.scope,
            &*e.kind,
            &*e.task,
            &*e.attempt,
            &*e.message,
            &*e.at
        ),
        (
            "child",
            "validation_changed",
            "task",
            "last",
            "accepted",
            "clock-fixture"
        )
    );
    assert_eq!(w.revision, 1);
    let prior = w.clone();
    let unchanged = BTreeMap::from([("task".into(), "accepted".into())]);
    apply_planning_validation_signals(&mut w, &unchanged, "new-event", "clock-fixture").unwrap();
    assert_eq!(w, prior);
}

#[test]
fn bad_ancestry_is_refused_before_signals_and_recovers_without_partial_changes() {
    let before = BTreeMap::from([("task".into(), "running".into())]);
    for parent in ["missing", "child"] {
        let mut w = fixture();
        w.planning.as_mut().unwrap().scopes.as_mut().unwrap()[1].parent = parent.into();
        let original = w.clone();
        assert!(apply_planning_validation_signals(&mut w, &before, "transition", "clock").is_err());
        assert_eq!(w, original);
        w.planning.as_mut().unwrap().scopes.as_mut().unwrap()[1].parent = "root".into();
        apply_planning_validation_signals(&mut w, &before, "transition", "clock").unwrap();
        assert_eq!(
            w.planning.as_ref().unwrap().inbox.as_ref().unwrap().len(),
            1
        );
    }
}

#[test]
fn duplicate_signal_rolls_back_the_whole_in_memory_plan_and_retry_with_new_event_recovers() {
    let mut w = fixture();
    let before = BTreeMap::from([("task".into(), "running".into())]);
    apply_planning_validation_signals(&mut w, &before, "first", "clock").unwrap();
    let original = w.clone();
    assert!(matches!(
        apply_planning_validation_signals(&mut w, &before, "first", "clock"),
        Err(PlanningStateError::Refused(PlanningRefusal::Event))
    ));
    assert_eq!(w, original);
    apply_planning_validation_signals(&mut w, &before, "second", "clock").unwrap();
    assert_eq!(
        w.planning.as_ref().unwrap().inbox.as_ref().unwrap().len(),
        2
    );
}

/// Negative scope consumption/limits refuse only after a valid inherited-limit
/// witness. Zero inheritance and positive consumed history are preserved, not reset.
#[test]
fn planning_scope_budget_negative_fields_refuse_without_rewriting_history() {
    for field in ["activations", "max_tasks", "max_activations"] {
        validate_planning_state(&fixture()).unwrap();
        let mut w = fixture();
        let scope = &mut w.planning.as_mut().unwrap().scopes.as_mut().unwrap()[1];
        match field {
            "activations" => scope.activations = -1,
            "max_tasks" => scope.max_tasks = -1,
            _ => scope.max_activations = -1,
        }
        let before = swarm_engine::encode_model(&w).unwrap();
        assert!(
            validate_planning_state(&w).is_err(),
            "negative scope budget field {field} accepted"
        );
        assert_eq!(swarm_engine::encode_model(&w).unwrap(), before);
    }
    let mut w = fixture();
    let p = w.planning.as_mut().unwrap();
    p.activations = 4;
    p.max_activations = 5;
    let scopes = p.scopes.as_mut().unwrap();
    scopes[0].activations = 4;
    scopes[1].activations = 4;
    scopes[1].max_activations = 2;
    validate_planning_state(&w).unwrap();
}

/// Stored history is data, not today's admission policy. A healthy short record
/// precedes the historical byte-bound counterexample; no Root or grant exists.
#[test]
fn judge_history_keeps_long_reasons_without_a_hidden_admission_policy() {
    let mut w = decode_work(br#"{"schema_version":1,"planning":{"judge":{"cycle":1,"verdict":"continue","reason":"Control","at":"2026-10-22T12:00:00Z","history":[{"cycle":1,"verdict":"continue","reason":"Control","at":"2026-10-22T12:00:00Z"}]}}}"#).unwrap();
    let judge = w.planning.as_mut().unwrap().judge.as_mut().unwrap();
    validate_judge_state(Some(judge)).unwrap();
    let reason = "é".repeat(2500); // Explicit wire input:5000 UTF8 bytes, no product policy.
    judge.reason = reason.clone(); judge.history.as_mut().unwrap()[0].reason = reason;
    let before = judge.clone();
    validate_judge_state(Some(judge)).expect("Stored history must not impose a second4000byte admission policy");
    assert!(judge.as_ref() == before.as_ref());
}
