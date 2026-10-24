use std::collections::BTreeMap;
use swarm_engine::{
    apply_ooda, apply_task_addition, apply_work_creation, apply_work_definition, decode_work,
    validate_work_request, GitState, Request, Work,
};
fn work() -> Work {
    decode_work(br#"{"schema_version":1,"id":"work","revision":2,"title":"old","objective":"objective","scope":"scope","criteria":["first"],"tasks":[]}"#).unwrap()
}
fn request() -> Request {
    Request {
        schema_version: 1,
        ..Default::default()
    }
}
#[test]
fn command_schema_precedes_kind_and_does_not_authorize_any_operation() {
    assert_eq!(
        validate_work_request("unknown", &Request::default())
            .unwrap_err()
            .code(),
        "invalid_schema"
    );
    assert_eq!(
        validate_work_request("unknown", &request())
            .unwrap_err()
            .code(),
        "unknown_command"
    );
    for kind in [
        "work.create",
        "work.update",
        "task.add",
        "task.update",
        "checkpoint",
        "ooda",
    ] {
        validate_work_request(kind, &request()).unwrap();
    }
}
#[test]
fn creation_keeps_caller_envelope_and_valid_criteria_without_mutating_on_refusal() {
    let mut w = work();
    let r = Request {
        title: "new".into(),
        objective: "goal".into(),
        scope: "scope".into(),
        criteria: Some(vec!["Proof".into()]),
        next: "next".into(),
        ..request()
    };
    let git = GitState {
        head: "head".into(),
        branch: "branch".into(),
        changes: " M proof".into(),
    };
    apply_work_creation(&mut w, &r, git.clone()).unwrap();
    assert_eq!((&*w.id, w.revision), ("work", 2));
    assert_eq!(w.title, "new");
    assert_eq!(w.criteria, Some(vec!["Proof".into()]));
    assert_eq!(w.git, git);
    for r in [
        Request {
            title: " ".into(),
            ..r.clone()
        },
        Request {
            criteria: Some(vec![]),
            ..r.clone()
        },
        Request {
            requirement_prerequisites: Some(BTreeMap::from([("missing".into(), None)])),
            ..r.clone()
        },
    ] {
        let original = w.clone();
        assert!(apply_work_creation(&mut w, &r, GitState::default()).is_err());
        assert_eq!(w, original);
    }
}
#[test]
fn work_reframing_preserves_tasks_history_reviews_and_automatic_checks_but_invalidates_gates() {
    let mut w=decode_work(br#"{"schema_version":1,"id":"work","revision":2,"criteria":["first"],"tasks":[{"id":"existing","status":"blocked","attempts":[{"id":"attempt"}],"gate":{},"automatic_validation":{},"manual_override":{},"revalidation":{},"independent_review":{"state":"passed"}}]}"#).unwrap();
    let before = w.clone();
    let r = Request {
        title: "reframed".into(),
        criteria: Some(vec!["changed".into()]),
        ..request()
    };
    apply_work_definition(&mut w, &r).unwrap();
    let t = &w.tasks.as_ref().unwrap()[0];
    let old = &before.tasks.as_ref().unwrap()[0];
    assert_eq!(t.id, old.id);
    assert_eq!(t.attempts, old.attempts);
    assert_eq!(t.independent_review, old.independent_review);
    assert_eq!(t.automatic_validation, old.automatic_validation);
    assert!(t.gate.is_none() && t.manual_override.is_none() && t.revalidation.is_none());
    assert_eq!(w.revision, 2);
}
#[test]
fn update_rejects_task_fields_active_tasks_and_bad_prerequisites_without_partial_changes() {
    let mut w = work();
    let valid = Request {
        title: "next".into(),
        ..request()
    };
    let invalid = [
        Request {
            depends: Some(vec![]),
            ..valid.clone()
        },
        Request {
            criteria: Some(vec![]),
            ..valid.clone()
        },
        Request {
            scope: "\u{0085}\u{3000}".into(),
            ..valid.clone()
        },
        Request {
            requirement_prerequisites: Some(BTreeMap::from([(
                "req-1".into(),
                Some(vec!["missing".into()]),
            )])),
            ..valid.clone()
        },
    ];
    for r in invalid {
        let original = w.clone();
        let e = apply_work_definition(&mut w, &r).unwrap_err();
        assert_eq!(w, original);
        assert!(!format!("{e:?}").contains("missing"));
    }
    apply_work_definition(&mut w, &valid).unwrap();
    w.tasks = Some(vec![swarm_engine::Task {
        status: "running".into(),
        ..Default::default()
    }]);
    let original = w.clone();
    assert!(apply_work_definition(&mut w, &valid).is_err());
    assert_eq!(w, original);
    w.tasks.as_mut().unwrap()[0].status = "todo".into();
    assert!(apply_work_definition(
        &mut w,
        &Request {
            requirement_prerequisites: Some(BTreeMap::new()),
            ..valid.clone()
        }
    )
    .is_err());
    apply_work_definition(&mut w, &valid).unwrap();
}
/// Preserve nil/empty dependency encoding and reject missing targets on a
/// validated pure task addition. The fixture supplies nonblank criteria; this
/// constructor test performs no execution, persistence or operator authorization.
#[test]
fn task_add_preserves_nil_empty_dependencies_and_requires_existing_targets() {
    let mut w = work();
    let r = Request {
        id: "first".into(),
        title: "task".into(),
        deliverable: "proof".into(),
        criteria: Some(vec!["Proof".into()]),
        ..request()
    };
    apply_task_addition(&mut w, &r).unwrap();
    let task = &w.tasks.as_ref().unwrap()[0];
    assert_eq!(task.status, "todo");
    assert_eq!(task.attempts, Some(vec![]));
    assert_eq!(task.depends, None);
    let original = w.clone();
    assert!(apply_task_addition(&mut w, &r).is_err());
    assert_eq!(w, original);
    let second = Request {
        id: "second".into(),
        depends: Some(vec!["missing".into()]),
        ..r.clone()
    };
    assert!(apply_task_addition(&mut w, &second).is_err());
    assert_eq!(w, original);
    apply_task_addition(
        &mut w,
        &Request {
            depends: Some(vec!["first".into(), "first".into()]),
            ..second
        },
    )
    .unwrap();
    assert_eq!(
        w.tasks.as_ref().unwrap()[1].depends,
        Some(vec!["first".into(), "first".into()])
    );
}
#[test]
fn ooda_requires_all_fields_and_keeps_summary_when_result_absent() {
    let mut w = work();
    w.summary = "prior".into();
    let original = w.clone();
    assert!(apply_ooda(&mut w, &request()).is_err());
    assert_eq!(w, original);
    let r = Request {
        observation: "observe".into(),
        orientation: "orient".into(),
        decision: "decide".into(),
        owner: "operator".into(),
        next: "next".into(),
        ..request()
    };
    apply_ooda(&mut w, &r).unwrap();
    assert_eq!(w.summary, "prior");
    assert_eq!(w.next, "next");
    apply_ooda(
        &mut w,
        &Request {
            result: "outcome".into(),
            ..r
        },
    )
    .unwrap();
    assert_eq!(w.summary, "outcome");
}

#[test]
fn mutation_intent_preserves_explicit_empty_collections_while_legacy_codec_stays_compatible() {
    use swarm_engine::{encode_request, encode_work_mutation_intent};
    let baseline = Request {
        title: "changed".into(),
        ..request()
    };
    for r in [
        Request {
            criteria: Some(vec![]),
            ..baseline.clone()
        },
        Request {
            memory: Some(vec![]),
            ..baseline.clone()
        },
        Request {
            requirement_prerequisites: Some(BTreeMap::new()),
            ..baseline.clone()
        },
    ] {
        assert_eq!(
            encode_request(&r).unwrap(),
            encode_request(&baseline).unwrap()
        );
        assert_ne!(
            encode_work_mutation_intent(&r).unwrap(),
            encode_work_mutation_intent(&baseline).unwrap()
        );
        assert_eq!(
            swarm_engine::decode_request(&encode_work_mutation_intent(&r).unwrap()).unwrap(),
            r
        );
    }
    let r = Request {
        criteria: Some(vec![]),
        depends: Some(vec![]),
        memory: Some(vec![]),
        requirement_prerequisites: Some(BTreeMap::new()),
        ..baseline.clone()
    };
    let raw = encode_work_mutation_intent(&r).unwrap();
    assert_eq!(swarm_engine::decode_request(&raw).unwrap(), r);
    assert_eq!(
        encode_work_mutation_intent(&baseline).unwrap(),
        encode_request(&baseline).unwrap()
    );
}

#[test]
fn reserved_event_refuses_absent_versus_clear_intent_then_new_revision_recovers() {
    use swarm_engine::{
        encode_work_mutation_intent, reserve_recorded_command, stage_recorded_change,
        RecordedChange, RecordedChangeOutcome,
    };
    let mut db = rusqlite::Connection::open_in_memory().unwrap();
    db.execute_batch("PRAGMA user_version=37;CREATE TABLE works(id TEXT PRIMARY KEY,revision INTEGER,body BLOB);CREATE TABLE events(id TEXT PRIMARY KEY,work_id TEXT,revision INTEGER,kind TEXT,at TEXT,payload BLOB,request BLOB)").unwrap();
    let mut w = work();
    w.criteria = Some(vec!["A".into(), "B".into()]);
    w.requirement_prerequisites = Some(BTreeMap::from([(
        "req-1".into(),
        Some(vec!["req-2".into()]),
    )]));
    db.execute(
        "INSERT INTO works VALUES('work',2,?)",
        [swarm_engine::encode_model(&w).unwrap()],
    )
    .unwrap();
    let r = Request {
        event_id: "intent".into(),
        expected_revision: 2,
        title: "changed".into(),
        ..request()
    };
    let bytes = encode_work_mutation_intent(&r).unwrap();
    apply_work_definition(&mut w, &r).unwrap();
    w.revision = 3;
    let mut tx = db.transaction().unwrap();
    assert!(matches!(
        stage_recorded_change(
            &mut tx,
            RecordedChange {
                work: &w,
                expected_revision: 2,
                event: "intent",
                kind: "work.update",
                request: &bytes
            }
        )
        .unwrap(),
        RecordedChangeOutcome::Staged
    ));
    tx.commit().unwrap();
    let clear = Request {
        requirement_prerequisites: Some(BTreeMap::new()),
        ..r
    };
    let clear_bytes = encode_work_mutation_intent(&clear).unwrap();
    let tx = db.transaction().unwrap();
    assert!(
        reserve_recorded_command(&tx, "work", "work.update", "intent", &bytes)
            .unwrap()
            .is_some()
    );
    assert_eq!(
        reserve_recorded_command(&tx, "work", "work.update", "intent", &clear_bytes)
            .unwrap_err()
            .code(),
        "event_conflict"
    );
    tx.rollback().unwrap();
    assert_eq!(swarm_engine::read_recorded_work(&db, "work").unwrap(), w);
    let clear = Request {
        event_id: "clear".into(),
        expected_revision: 3,
        ..clear
    };
    let clear_bytes = encode_work_mutation_intent(&clear).unwrap();
    apply_work_definition(&mut w, &clear).unwrap();
    w.revision = 4;
    let mut tx = db.transaction().unwrap();
    assert!(matches!(
        stage_recorded_change(
            &mut tx,
            RecordedChange {
                work: &w,
                expected_revision: 3,
                event: "clear",
                kind: "work.update",
                request: &clear_bytes
            }
        )
        .unwrap(),
        RecordedChangeOutcome::Staged
    ));
    tx.commit().unwrap();
    assert!(swarm_engine::read_recorded_work(&db, "work")
        .unwrap()
        .requirement_prerequisites
        .is_none());
    assert_eq!(
        db.query_row("SELECT count(*) FROM events", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
}

/// New task contracts immediately declare every criterion's mandatory check;
/// invalid blank criteria preserve the input Work and corrected data recovers.
#[test]
fn new_task_criteria_are_bound_immediately_and_blank_items_refuse_without_mutation() {
    let valid = Request {
        id: "task".into(),
        title: "Task".into(),
        deliverable: "Report".into(),
        criteria: Some(vec!["Criterion A".into(), "Criterion B".into()]),
        ..request()
    };
    let mut w = work();
    apply_task_addition(&mut w, &valid).unwrap();
    let checks = w.tasks.as_ref().unwrap()[0]
        .plan_checks
        .as_ref()
        .expect("criteria mapping at creation");
    assert_eq!(checks.len(), 5);
    assert_eq!(checks["plan-criterion-2"], "validation");
    for criteria in [
        None,
        Some(vec![]),
        Some(vec!["".into()]),
        Some(vec![" \t\n".into()]),
        Some(vec!["\u{0085}\u{3000}".into()]),
        Some(vec!["Proof".into(), " ".into()]),
    ] {
        let mut original = work();
        let before = original.clone();
        let bad = Request {
            criteria,
            ..valid.clone()
        };
        assert!(apply_task_addition(&mut original, &bad).is_err());
        assert_eq!(original, before);
        apply_task_addition(&mut original, &valid).unwrap();
    }
}

/// A matching valid constructor is checked before each blank/mixed criterion.
/// Rejection must preserve the entire candidate; corrected content recovers.
#[test]
fn creation_refuses_blank_criteria_before_mutation_and_recovers() {
    for criteria in [vec![""], vec![" \t\n"], vec!["Valid", " "], vec!["\u{85}"]] {
        let r = Request {
            title: "Work".into(),
            objective: "Goal".into(),
            scope: "Fixture".into(),
            criteria: Some(vec!["Proof".into()]),
            ..request()
        };
        let mut control = work();
        apply_work_creation(&mut control, &r, GitState::default()).unwrap();
        let mut candidate = work();
        let before = candidate.clone();
        let mut bad = r;
        bad.criteria = Some(criteria.into_iter().map(String::from).collect());
        let e = apply_work_creation(&mut candidate, &bad, GitState::default()).unwrap_err();
        assert_eq!(e.code(), "format");
        assert_eq!(candidate, before);
        bad.criteria = Some(vec!["Proof repaired".into()]);
        apply_work_creation(&mut candidate, &bad, GitState::default()).unwrap();
        assert_eq!(candidate.criteria, bad.criteria);
    }
}
