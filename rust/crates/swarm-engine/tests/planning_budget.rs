use std::error::Error;
use swarm_engine::{
    check_planning_activation_budget, check_planning_task_budget, decode_work, encode_model,
    planning_scope_ancestors, planning_scope_task_count, PlanningBudgetDimension as D,
    PlanningBudgetError as E, Work,
};
/// Valid recorded-shape data, not authority. Two branches have distinct requirements;
/// task status does not erase budget consumption. Zero local limits inherit globals.
fn fixture() -> Work {
    decode_work(br#"{"schema_version":1,"id":"work","revision":1,"criteria":["Child","Other"],"tasks":[{"id":"task-child","scope_id":"child","requirements":["req-1"],"status":"accepted"},{"id":"task-other","scope_id":"other","requirements":["req-2"],"status":"failed"}],"planning":{"version":1,"max_tasks":10,"max_decisions":10,"max_activations":20,"scopes":[{"id":"root","revision":1},{"id":"parent","parent":"root","revision":1,"max_tasks":2,"max_activations":2},{"id":"child","parent":"parent","revision":1,"requirements":["req-1"]},{"id":"other","parent":"root","revision":1,"requirements":["req-2"]}]}}"#).unwrap()
}
/// An immutable validated snapshot reports exact selected-to-root order and all
/// descendant/self tasks, including accepted and failed history, without mutation.
#[test]
fn planning_budget_ancestry_and_counts_are_complete_and_readonly() {
    let w = fixture();
    let before = encode_model(&w).unwrap();
    assert_eq!(
        planning_scope_ancestors(&w, "child")
            .unwrap()
            .iter()
            .map(|s| s.id.as_str())
            .collect::<Vec<_>>(),
        ["child", "parent", "root"]
    );
    for (id, count) in [("root", 2), ("parent", 1), ("child", 1), ("other", 1)] {
        assert_eq!(planning_scope_task_count(&w, id).unwrap(), count);
        check_planning_task_budget(&w, id).unwrap();
        check_planning_activation_budget(&w, id).unwrap();
    }
    assert_eq!(encode_model(&w).unwrap(), before);
}
/// Child zero inherits global capacity, while a tighter ancestor still refuses.
/// Independent branches remain usable; global activation and decision caps also
/// refuse on equality. A numeric pass is not a lease or permission to launch.
#[test]
fn planning_activation_budget_checks_global_decisions_and_every_ancestor() {
    let mut w = fixture();
    check_planning_activation_budget(&w, "child").unwrap();
    let p = w.planning.as_mut().unwrap();
    p.activations = 2;
    let s = p.scopes.as_mut().unwrap();
    s[0].activations = 2;
    s[1].activations = 2;
    assert!(
        matches!(check_planning_activation_budget(&w,"child"),Err(E::Exhausted{dimension:D::Activations,scope:Some(s)})if s=="parent")
    );
    check_planning_activation_budget(&w, "other").unwrap();
    let mut w = fixture();
    check_planning_activation_budget(&w, "child").unwrap();
    let p = w.planning.as_mut().unwrap();
    p.activations = p.max_activations;
    assert!(matches!(
        check_planning_activation_budget(&w, "child"),
        Err(E::Exhausted {
            dimension: D::Activations,
            scope: None
        })
    ));
    let mut w = fixture();
    check_planning_activation_budget(&w, "child").unwrap();
    let p = w.planning.as_mut().unwrap();
    p.decisions = p.max_decisions;
    assert!(matches!(
        check_planning_activation_budget(&w, "child"),
        Err(E::Exhausted {
            dimension: D::Decisions,
            scope: None
        })
    ));
    let mut w = fixture();
    let p = w.planning.as_mut().unwrap();
    p.activations = 4;
    p.scopes.as_mut().unwrap()[0].activations = 4;
    p.scopes.as_mut().unwrap()[1].activations = 4;
    swarm_engine::validate_planning_state(&w).unwrap();
    assert!(check_planning_activation_budget(&w, "child").is_err());
}
/// Ancestor counts include all descendants, never unrelated siblings outside the
/// ancestor. Global task capacity is checked independently from local inheritance.
#[test]
fn planning_task_budget_counts_history_and_ancestor_capacity_without_reset() {
    let mut w = fixture();
    check_planning_task_budget(&w, "child").unwrap();
    let mut task = w.tasks.as_ref().unwrap()[0].clone();
    task.id = "another-child".into();
    w.tasks.as_mut().unwrap().push(task);
    assert_eq!(planning_scope_task_count(&w, "parent").unwrap(), 2);
    assert_eq!(planning_scope_task_count(&w, "root").unwrap(), 3);
    let before = encode_model(&w).unwrap();
    assert!(
        matches!(check_planning_task_budget(&w,"child"),Err(E::Exhausted{dimension:D::Tasks,scope:Some(s)})if s=="parent")
    );
    check_planning_task_budget(&w, "other").unwrap();
    assert_eq!(encode_model(&w).unwrap(), before);
    let mut w = fixture();
    check_planning_task_budget(&w, "child").unwrap();
    w.planning.as_mut().unwrap().max_tasks = 2;
    assert!(matches!(
        check_planning_task_budget(&w, "child"),
        Err(E::Exhausted {
            dimension: D::Tasks,
            scope: None
        })
    ));
}
/// Invalid state and unknown scope fail before traversal or zero-count success;
/// original validation causes survive safe diagnostics. Legacy absence is not a
/// configured budget and cyclic ownership cannot consume the native call stack.
#[test]
fn planning_budget_invalid_state_unknown_scope_and_absence_refuse_safely() {
    let w = fixture();
    planning_scope_task_count(&w, "child").unwrap();
    let err = planning_scope_task_count(&w, "PRIVATE_UNKNOWN").unwrap_err();
    assert!(matches!(err, E::UnknownScope));
    assert!(!format!("{err:?}").contains("PRIVATE_"));
    assert!(planning_scope_ancestors(&w, "").is_err());
    for field in [0, 1, 2, 3] {
        let mut w = fixture();
        check_planning_activation_budget(&w, "child").unwrap();
        let s = &mut w.planning.as_mut().unwrap().scopes.as_mut().unwrap()[1];
        match field {
            0 => s.activations = -1,
            1 => s.max_tasks = -1,
            2 => s.max_activations = -1,
            _ => s.parent = "child".into(),
        };
        let before = encode_model(&w).unwrap();
        let e = check_planning_activation_budget(&w, "child").unwrap_err();
        assert!(e.source().is_some());
        assert!(check_planning_task_budget(&w, "child").is_err());
        assert_eq!(encode_model(&w).unwrap(), before);
    }
    let w = Work::default();
    assert!(matches!(
        planning_scope_task_count(&w, "root"),
        Err(E::MissingPlanning)
    ));
}
/// A deep valid chain uses iterative walkers and no hidden depth cutoff. The
/// fixture size is test data, not a policy; no duration or performance is asserted.
#[test]
fn planning_budget_deep_tree_counts_iteratively_without_hidden_cutoff() {
    let depth = 4096;
    let mut scopes = Vec::new();
    for i in 0..depth {
        scopes.push(serde_json::json!({"id":if i==0{"root".into()}else{format!("scope-{i}")},"parent":if i==0{String::new()}else if i==1{"root".into()}else{format!("scope-{}",i-1)},"revision":1,"requirements":if i==depth-1{vec!["req-1"]}else{vec![]}}));
    }
    let raw=serde_json::to_vec(&serde_json::json!({"schema_version":1,"id":"deep","revision":1,"criteria":["Proof"],"tasks":[{"id":"leaf","scope_id":format!("scope-{}",depth-1),"requirements":["req-1"]}],"planning":{"version":1,"max_tasks":2,"max_decisions":2,"max_activations":2,"scopes":scopes}})).unwrap();
    let w = decode_work(&raw).unwrap();
    assert_eq!(
        planning_scope_ancestors(&w, &format!("scope-{}", depth - 1))
            .unwrap()
            .len(),
        depth
    );
    assert_eq!(planning_scope_task_count(&w, "root").unwrap(), 1);
    check_planning_task_budget(&w, &format!("scope-{}", depth - 1)).unwrap();
    check_planning_activation_budget(&w, &format!("scope-{}", depth - 1)).unwrap();
}
