use std::error::Error;
use swarm_engine::{
    decode_work, encode_model, planning_rejection_state, planning_rejection_unchanged,
    PlanningRejectionWait, Work,
};
/// Valid two-branch durable fixture; includes HTML/UTF8 and explicit nullable data.
fn fixture() -> Work {
    decode_work(br#"{"schema_version":1,"id":"work","revision":1,"criteria":["Child <&>","Other"],"tasks":[{"id":"task-child","scope_id":"child","requirements":["req-1"],"status":"accepted","next":"A <&>"},{"id":"task-other","scope_id":"other","requirements":["req-2"],"status":"failed","next":"B"}],"planning":{"version":1,"max_tasks":10,"max_decisions":10,"max_activations":20,"scopes":[{"id":"root","revision":1},{"id":"parent","parent":"root","revision":1,"max_tasks":2,"max_activations":2},{"id":"child","parent":"parent","revision":1,"objective":"Child <&>","requirements":["req-1"]},{"id":"other","parent":"root","revision":1,"requirements":["req-2"]}],"inbox":[{"id":"source-child","scope":"child","kind":"operator_resume","message":"Input <&>"}]}}"#).unwrap()
}
/// The fingerprint is immutable deterministic data; absent wait is false, never authority.
#[test]
fn planning_rejection_fingerprint_is_readonly_and_wait_is_explicit() {
    let mut w = fixture();
    let before = encode_model(&w).unwrap();
    let state = planning_rejection_state(&w, "child").unwrap();
    assert_eq!(state.len(), 64);
    assert_eq!(state, planning_rejection_state(&w, "child").unwrap());
    assert_eq!(before, encode_model(&w).unwrap());
    assert!(!planning_rejection_unchanged(&w, "child").unwrap());
    w.planning.as_mut().unwrap().scopes.as_mut().unwrap()[2].rejection_wait =
        Some(Box::new(PlanningRejectionWait {
            event_id: "diagnostic".into(),
            state_sha256: state,
        }));
    assert!(planning_rejection_unchanged(&w, "child").unwrap());
}
/// Leases/counters/diagnostic and unrelated task content do not fake changed preconditions.
#[test]
fn planning_rejection_ignores_counters_leases_and_unrelated_activity() {
    let mut w = fixture();
    let before = planning_rejection_state(&w, "child").unwrap();
    w.revision += 1;
    let p = w.planning.as_mut().unwrap();
    p.activations += 1;
    p.decisions += 1;
    let scope = &mut p.scopes.as_mut().unwrap()[2];
    scope.revision += 1;
    scope.generation += 1;
    scope.holder = "holder".into();
    scope.lease_until = "2026-10-19T12:00:00Z".into();
    scope.rejection_wait = Some(Box::new(PlanningRejectionWait {
        event_id: "diagnostic".into(),
        state_sha256: before.clone(),
    }));
    p.inbox.as_mut().unwrap().push(swarm_engine::PlanningEvent {
        id: "diagnostic".into(),
        scope: "child".into(),
        kind: "planning_rejected".into(),
        ..Default::default()
    });
    p.inbox.as_mut().unwrap().push(swarm_engine::PlanningEvent {
        id: "other-event".into(),
        scope: "other".into(),
        kind: "operator_resume".into(),
        ..Default::default()
    });
    w.tasks.as_mut().unwrap()[1].next = "unrelated".into();
    assert_eq!(before, planning_rejection_state(&w, "child").unwrap());
    assert!(planning_rejection_unchanged(&w, "child").unwrap());
}
/// Own input/task instructions, global IDs, criteria and explicit policies reopen a wait.
#[test]
fn planning_rejection_recognizes_changed_owned_inputs_and_policy() {
    let base = fixture();
    let before = planning_rejection_state(&base, "child").unwrap();
    for change in 0..7 {
        let mut w = base.clone();
        match change {
            0 => w.tasks.as_mut().unwrap()[0].next = "correction".into(),
            1 => w.criteria.as_mut().unwrap()[0] = "Changed".into(),
            2 => w.planning.as_mut().unwrap().provider = "new-provider".into(),
            3 => w.planning.as_mut().unwrap().max_tasks += 1,
            4 => w.planning.as_mut().unwrap().scopes.as_mut().unwrap()[2].max_tasks = 1,
            5 => {
                w.planning.as_mut().unwrap().inbox.as_mut().unwrap()[0].message =
                    "new public input".into()
            }
            _ => w.tasks.as_mut().unwrap()[1].id = "new-global-id".into(),
        }
        assert_ne!(
            before,
            planning_rejection_state(&w, "child").unwrap(),
            "change {change}"
        );
    }
}
/// Invalid state/unknown scope preserve causes and never return a hash/false success.
#[test]
fn planning_rejection_invalid_state_refuses_then_recovers() {
    let base = fixture();
    planning_rejection_state(&base, "child").unwrap();
    for scope in ["", "PRIVATE_UNKNOWN"] {
        let e = planning_rejection_state(&base, scope).unwrap_err();
        assert!(e.source().is_some());
        assert!(!format!("{e:?}").contains("PRIVATE_"));
        assert!(planning_rejection_unchanged(&base, scope).is_err());
    }
    let mut invalid = base.clone();
    invalid.planning.as_mut().unwrap().scopes.as_mut().unwrap()[1].parent = "child".into();
    assert!(planning_rejection_state(&invalid, "child").is_err());
    assert!(planning_rejection_state(&Work::default(), "child").is_err());
    assert!(planning_rejection_state(&base, "child").is_ok());
}
/// Reviewer call consumption is ignored, while policy/null-empty/child/descendant
/// content remains significant; no history is mutated to obtain the fingerprint.
#[test]
fn planning_rejection_preserves_nullable_shapes_and_policy_changes() {
    let mut w = fixture();
    w.planning.as_mut().unwrap().reviewer = Some(Box::new(swarm_engine::ReviewerConfig {
        provider: "reviewer".into(),
        max_calls: 20,
        calls: 3,
        ..Default::default()
    }));
    let before = planning_rejection_state(&w, "parent").unwrap();
    w.planning
        .as_mut()
        .unwrap()
        .reviewer
        .as_mut()
        .unwrap()
        .calls += 1;
    assert_eq!(before, planning_rejection_state(&w, "parent").unwrap());
    let base = w.clone();
    for change in 0..4 {
        let mut w = base.clone();
        match change {
            0 => {
                w.planning
                    .as_mut()
                    .unwrap()
                    .reviewer
                    .as_mut()
                    .unwrap()
                    .max_calls += 1
            }
            1 => {
                w.planning.as_mut().unwrap().scopes.as_mut().unwrap()[1].requirements = Some(vec![])
            }
            2 => w.planning.as_mut().unwrap().scopes.as_mut().unwrap()[2].state = "closed".into(),
            _ => w.tasks.as_mut().unwrap()[0].next = "descendant correction".into(),
        }
        assert_ne!(
            before,
            planning_rejection_state(&w, "parent").unwrap(),
            "change {change}"
        );
    }
}
