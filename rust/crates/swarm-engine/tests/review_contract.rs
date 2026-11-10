use swarm_engine::{decode_task, review_contract};
#[test]
fn review_contract_binds_go_base_fields_nil_empty_order_and_memory_without_other_task_state() {
    let base = decode_task(br#"{}"#).unwrap();
    let digest = review_contract(&base).unwrap();
    assert_eq!(digest.len(), 64);
    for raw in [
        br#"{"title":"Changed"}"#.as_slice(),
        br#"{"deliverable":"report.md"}"#,
        br#"{"criteria":[]}"#,
        br#"{"depends":[]}"#,
        br#"{"plan_brief_hash":"new"}"#,
        br#"{"memory_selections":[{"work":"w","id":"m","version":1,"sha256":"a"}]}"#,
    ] {
        assert_ne!(review_contract(&decode_task(raw).unwrap()).unwrap(), digest);
    }
    for raw in [
        br#"{"id":"other","status":"accepted","owner":"operator"}"#.as_slice(),
        br#"{"memory_selections":[]}"#,
        br#"{"gate":{"document":{"n":1e999}}}"#,
    ] {
        assert_eq!(review_contract(&decode_task(raw).unwrap()).unwrap(), digest);
    }
    let a=decode_task(br#"{"criteria":["a","b"],"memory_selections":[{"work":"w","id":"m","version":1,"sha256":"a"}]}"#).unwrap();
    let mut b = a.clone();
    b.criteria.as_mut().unwrap().reverse();
    assert_ne!(review_contract(&a).unwrap(), review_contract(&b).unwrap());
    b = a.clone();
    b.memory_selections.as_mut().unwrap()[0].version = 2;
    assert_ne!(review_contract(&a).unwrap(), review_contract(&b).unwrap());
}

/// Verify cohort membership, ordering, latest attempts and policy bindings.
#[test]
fn managed_contract_binds_selected_and_accepted_tasks_in_order_with_latest_attempt_and_policy() {
    use swarm_engine::{decode_work, managed_review_contract, Attempt, Task};
    let w=decode_work(br#"{"tasks":[{"id":"selected","title":"First","attempts":[{"id":"old"},{"id":"latest"}]},{"id":"prior","status":"accepted","title":"Prior"},{"id":"ignored","status":"todo"}]}"#).unwrap();
    let digest = managed_review_contract(&w, "selected").unwrap();
    let mut changed = w.clone();
    changed.tasks.as_mut().unwrap()[2].title = "Changed ignored".into();
    assert_eq!(
        managed_review_contract(&changed, "selected").unwrap(),
        digest
    );
    changed = w.clone();
    changed.tasks.as_mut().unwrap()[0]
        .attempts
        .as_mut()
        .unwrap()[0]
        .id = "different old attempt".into();
    assert_eq!(
        managed_review_contract(&changed, "selected").unwrap(),
        digest
    );
    for fault in [
        "title",
        "latest",
        "accepted",
        "policy",
        "order",
        "new-accepted",
    ] {
        let mut changed = w.clone();
        let tasks = changed.tasks.as_mut().unwrap();
        match fault {
            "title" => tasks[0].title = "Changed".into(),
            "latest" => tasks[0].attempts.as_mut().unwrap().push(Attempt {
                id: "next".into(),
                ..Default::default()
            }),
            "accepted" => tasks[1].title = "Changed prior".into(),
            "policy" => tasks[1].validation_policy = Some(Box::default()),
            "order" => tasks.swap(0, 1),
            "new-accepted" => tasks.push(Task {
                id: "new".into(),
                status: "accepted".into(),
                ..Default::default()
            }),
            _ => unreachable!(),
        }
        assert_ne!(
            managed_review_contract(&changed, "selected").unwrap(),
            digest,
            "{fault}"
        );
    }
}
/// Verify Go selection semantics without implying Git, identity or authority.
#[test]
fn managed_contract_is_data_only_and_preserves_go_empty_and_selection_semantics() {
    use swarm_engine::{decode_work, managed_review_contract};
    let empty = managed_review_contract(&decode_work(b"{}").unwrap(), "missing").unwrap();
    assert_eq!(
        empty,
        managed_review_contract(&decode_work(br#"{"tasks":[]}"#).unwrap(), "missing").unwrap()
    );
    let w = decode_work(br#"{"tasks":[{"id":"t","status":"ACCEPTED"}]}"#).unwrap();
    assert_eq!(empty, managed_review_contract(&w, "missing").unwrap());
    let one = managed_review_contract(&w, "t").unwrap();
    assert_ne!(empty, one);
    let changed=decode_work(br#"{"id":"another-work","planning":{"repository":{"candidate_commit":"other"}},"tasks":[{"id":"t","status":"blocked","owner":"someone","attempts":[] }]}"#).unwrap();
    assert_eq!(one, managed_review_contract(&changed, "t").unwrap());
    let criteria = decode_work(br#"{"tasks":[{"id":"t","criteria":[]}]}"#).unwrap();
    assert_ne!(one, managed_review_contract(&criteria, "t").unwrap());
}
