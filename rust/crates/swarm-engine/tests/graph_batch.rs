#![cfg(target_os = "linux")]
//! Actual Root transaction tests; only private fresh fixtures are SQL-injected
//! for rights/activity/failed-write controls. No live mission or provider.
use serde_json::{json, Value};
use std::{fs, path::PathBuf};
use swarm_engine::{GraphTaskDefinition, RootStore, SystemEnvironment, Work};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let p =
            std::env::temp_dir().join(swarm_engine::generate_identifier("graph-batch-").unwrap());
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn db(&self) -> rusqlite::Connection {
        rusqlite::Connection::open(self.0.join(".swarm/state.db")).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap()
        }
    }
}
fn raw(v: &Value) -> Vec<u8> {
    serde_json::to_vec(v).unwrap()
}
fn definition(id: &str, depends: &[&str]) -> Value {
    json!({"id":id,"title":format!("Task {id}"),"deliverable":format!("docs/{id}.md"),"criteria":["Verified"],"depends":depends})
}
fn batch(revision: i64, changes: Value) -> Value {
    json!({"schema_version":1,"event_id":"batch","expected_revision":revision,"changes":changes})
}
fn create() -> (Fixture, RootStore, Work) {
    let f = Fixture::new();
    let mut store = RootStore::initialize(&f.0, &mut SystemEnvironment).unwrap();
    let w=store.execute_external("","work.create",&raw(&json!({"schema_version":1,"event_id":"create","expected_revision":0,"title":"Graph","objective":"Atomic edits","scope":"Private fixture","criteria":["Observed"]})),&mut SystemEnvironment).unwrap().result.work;
    (f, store, w)
}
fn preview_apply(store: &mut RootStore, w: &Work, r: &mut Value) -> Work {
    let p = store
        .preview_graph_batch(&w.id, &raw(r), &mut SystemEnvironment)
        .unwrap();
    r["preview_digest"] = p.preview_digest.into();
    store
        .execute_external(&w.id, "graph.batch", &raw(r), &mut SystemEnvironment)
        .unwrap()
        .result
        .work
}

#[test]
fn forward_references_preview_pure_single_revision_replay_and_reopen() {
    let (f, mut store, w) = create();
    let mut r = batch(
        w.revision,
        json!([{"before":null,"after":definition("b",&["a"])},{"before":null,"after":definition("a",&[])}]),
    );
    let before = fs::read(f.0.join(".swarm/state.db")).unwrap();
    let p = store
        .preview_graph_batch(&w.id, &raw(&r), &mut SystemEnvironment)
        .unwrap();
    assert_eq!(p.changes.len(), 2);
    assert_eq!(p.revision, 1);
    assert!(p.no_implicit_launch);
    assert_eq!(fs::read(f.0.join(".swarm/state.db")).unwrap(), before);
    assert_eq!(store.events(&w.id).unwrap().len(), 1);
    r["preview_digest"] = p.preview_digest.into();
    let next = store
        .execute_external(&w.id, "graph.batch", &raw(&r), &mut SystemEnvironment)
        .unwrap();
    assert!(!next.result.replayed);
    assert_eq!(next.result.work.revision, 2);
    assert_eq!(next.result.work.tasks.as_ref().unwrap().len(), 2);
    let replay = store
        .execute_external(&w.id, "graph.batch", &raw(&r), &mut SystemEnvironment)
        .unwrap();
    assert!(replay.result.replayed);
    assert_eq!(store.events(&w.id).unwrap().len(), 2);
    store.close().unwrap();
    let store = RootStore::open(&f.0, false).unwrap();
    assert_eq!(
        store.read_work(&w.id).unwrap().tasks.unwrap()[0]
            .depends
            .as_ref()
            .unwrap(),
        &vec!["a".to_string()]
    );
    store.close().unwrap();
}
#[test]
fn invalid_second_task_cycle_duplicate_and_noop_never_publish() {
    let (_f, mut store, w) = create();
    let control = batch(
        w.revision,
        json!([{"before":null,"after":definition("a",&[])}]),
    );
    assert!(store
        .preview_graph_batch(&w.id, &raw(&control), &mut SystemEnvironment)
        .is_ok());
    for changes in [
        json!([{"before":null,"after":definition("a",&["b"])},{"before":null,"after":definition("b",&["a"])}]),
        json!([{"before":null,"after":definition("a",&[])},{"before":null,"after":definition("a",&[])}]),
        json!([{"before":null,"after":definition("a",&[])},{"before":null,"after":{"id":"b","title":"","deliverable":"docs/b","criteria":["Valid"],"depends":[]}}]),
    ] {
        let mut r = batch(w.revision, changes);
        assert!(store
            .preview_graph_batch(&w.id, &raw(&r), &mut SystemEnvironment)
            .is_err());
        r["preview_digest"] = "0".repeat(64).into();
        assert!(store
            .execute_external(&w.id, "graph.batch", &raw(&r), &mut SystemEnvironment)
            .is_err());
        assert_eq!(store.read_work(&w.id).unwrap().revision, 1);
        assert_eq!(store.events(&w.id).unwrap().len(), 1)
    }
    let mut good = control;
    let next = preview_apply(&mut store, &w, &mut good);
    let a = GraphTaskDefinition::observed(&next.tasks.as_ref().unwrap()[0]);
    let r = batch(next.revision, json!([{"before":a,"after":a}]));
    assert!(store
        .preview_graph_batch(&w.id, &raw(&r), &mut SystemEnvironment)
        .is_err());
    store.close().unwrap();
}
#[test]
fn digest_edit_or_revision_change_refuses_then_new_preview_recovers() {
    let (_f, mut store, w) = create();
    let mut r = batch(
        w.revision,
        json!([{"before":null,"after":definition("a",&[])}]),
    );
    let p = store
        .preview_graph_batch(&w.id, &raw(&r), &mut SystemEnvironment)
        .unwrap();
    r["preview_digest"] = p.preview_digest.into();
    r["changes"][0]["after"]["title"] = "Edited after preview".into();
    assert_eq!(
        store
            .execute_external(&w.id, "graph.batch", &raw(&r), &mut SystemEnvironment)
            .err()
            .unwrap()
            .code(),
        "graph-preview-stale"
    );
    let next = preview_apply(&mut store, &w, &mut r);
    let changed = batch(
        w.revision,
        json!([{"before":null,"after":definition("b",&[])}]),
    );
    assert!(store
        .preview_graph_batch(&w.id, &raw(&changed), &mut SystemEnvironment)
        .is_err());
    assert_eq!(next.revision, 2);
    store.close().unwrap();
}
#[test]
fn undo_is_derived_last_batch_atomic_and_never_accepts_arbitrary_delete() {
    let (_f, mut store, w) = create();
    let mut r = batch(
        w.revision,
        json!([{"before":null,"after":definition("a",&[])},{"before":null,"after":definition("b",&["a"])}]),
    );
    let next = preview_apply(&mut store, &w, &mut r);
    let a = GraphTaskDefinition::observed(&next.tasks.as_ref().unwrap()[0]);
    let deletion = batch(next.revision, json!([{"before":a,"after":null}]));
    assert!(store
        .preview_graph_batch(&w.id, &raw(&deletion), &mut SystemEnvironment)
        .is_err());
    let mut undo = json!({"schema_version":1,"event_id":"undo","expected_revision":next.revision,"changes":[],"undo_of":"batch"});
    let p = store
        .preview_graph_batch(&w.id, &raw(&undo), &mut SystemEnvironment)
        .unwrap();
    assert!(p.changes.iter().all(|c| c.after.is_none()));
    undo["preview_digest"] = p.preview_digest.into();
    let reverted = store
        .execute_external(&w.id, "graph.batch", &raw(&undo), &mut SystemEnvironment)
        .unwrap()
        .result
        .work;
    assert_eq!(reverted.revision, 3);
    assert!(reverted.tasks.unwrap().is_empty());
    assert_eq!(store.events(&w.id).unwrap().len(), 3);
    assert!(
        store
            .execute_external(&w.id, "graph.batch", &raw(&undo), &mut SystemEnvironment)
            .unwrap()
            .result
            .replayed
    );
    undo["event_id"] = "undo-again".into();
    undo["expected_revision"] = 3.into();
    assert!(store
        .preview_graph_batch(&w.id, &raw(&undo), &mut SystemEnvironment)
        .is_err());
    store.close().unwrap();
}
#[test]
fn definition_compensation_preserves_history_and_later_event_blocks_undo() {
    let (_f, mut store, w) = create();
    let mut add = batch(
        w.revision,
        json!([{"before":null,"after":definition("a",&[])}]),
    );
    let next = preview_apply(&mut store, &w, &mut add);
    let before = GraphTaskDefinition::observed(&next.tasks.as_ref().unwrap()[0]);
    let mut after = before.clone();
    after.title = "Changed".into();
    let mut edit = batch(next.revision, json!([{"before":before,"after":after}]));
    edit["event_id"] = "edit".into();
    let changed = preview_apply(&mut store, &next, &mut edit);
    let mut undo = json!({"schema_version":1,"event_id":"undo-edit","expected_revision":changed.revision,"changes":[],"undo_of":"edit"});
    let restored = preview_apply(&mut store, &changed, &mut undo);
    assert_eq!(
        GraphTaskDefinition::observed(&restored.tasks.as_ref().unwrap()[0]),
        before
    );
    assert_eq!(store.events(&w.id).unwrap().len(), 4);
    let late = json!({"schema_version":1,"event_id":"old-undo","expected_revision":restored.revision,"changes":[],"undo_of":"batch"});
    assert_eq!(
        store
            .preview_graph_batch(&w.id, &raw(&late), &mut SystemEnvironment)
            .err()
            .unwrap()
            .code(),
        "graph-undo-not-last-batch"
    );
    store.close().unwrap();
}
#[test]
fn durable_rights_revocation_scope_and_replay_are_enforced() {
    let (f, mut store, w) = create();
    let mut r = batch(
        w.revision,
        json!([{"before":null,"after":definition("a",&[])}]),
    );
    let p = store
        .preview_graph_batch(&w.id, &raw(&r), &mut SystemEnvironment)
        .unwrap();
    r["preview_digest"] = p.preview_digest.into();
    let actor = swarm_engine::WorkEnvironment::operator(&mut SystemEnvironment).unwrap();
    let db = f.db();
    db.execute("INSERT INTO graph_draft_authorizations(work_id,actor,revision,prepare,apply,scopes,updated) VALUES(?,?,1,1,0,?,?)",rusqlite::params![w.id,actor,b"null","fixture"]).unwrap();
    assert_eq!(
        store
            .execute_external(&w.id, "graph.batch", &raw(&r), &mut SystemEnvironment)
            .err()
            .unwrap()
            .code(),
        "graph-authorization-required"
    );
    db.execute(
        "UPDATE graph_draft_authorizations SET revision=2,apply=1,scopes=?",
        [b"[\"outside\"]".as_slice()],
    )
    .unwrap();
    assert_eq!(
        store
            .preview_graph_batch(&w.id, &raw(&r), &mut SystemEnvironment)
            .err()
            .unwrap()
            .code(),
        "graph-authorization-scope"
    );
    db.execute(
        "UPDATE graph_draft_authorizations SET revision=3,scopes=?",
        [b"null".as_slice()],
    )
    .unwrap();
    let next = preview_apply(&mut store, &w, &mut r);
    assert_eq!(next.revision, 2);
    db.execute(
        "UPDATE graph_draft_authorizations SET revision=4,apply=0",
        [],
    )
    .unwrap();
    assert_eq!(
        store
            .execute_external(&w.id, "graph.batch", &raw(&r), &mut SystemEnvironment)
            .err()
            .unwrap()
            .code(),
        "graph-authorization-required"
    );
    db.close().unwrap();
    store.close().unwrap();
}
#[test]
fn atomic_storage_failure_retains_original_and_exact_request_can_recover() {
    let (f, mut store, w) = create();
    let mut r = batch(
        w.revision,
        json!([{"before":null,"after":definition("a",&[])},{"before":null,"after":definition("b",&["a"])}]),
    );
    let p = store
        .preview_graph_batch(&w.id, &raw(&r), &mut SystemEnvironment)
        .unwrap();
    r["preview_digest"] = p.preview_digest.into();
    let db = f.db();
    db.execute_batch("CREATE TRIGGER reject_graph BEFORE INSERT ON events WHEN NEW.kind='graph.batch' BEGIN SELECT RAISE(ABORT,'fixture rejected event after Work write'); END;").unwrap();
    assert!(store
        .execute_external(&w.id, "graph.batch", &raw(&r), &mut SystemEnvironment)
        .is_err());
    assert_eq!(store.read_work(&w.id).unwrap().revision, 1);
    assert_eq!(store.events(&w.id).unwrap().len(), 1);
    db.execute_batch("DROP TRIGGER reject_graph;").unwrap();
    let next = store
        .execute_external(&w.id, "graph.batch", &raw(&r), &mut SystemEnvironment)
        .unwrap()
        .result
        .work;
    assert_eq!(next.revision, 2);
    assert_eq!(next.tasks.unwrap().len(), 2);
    db.close().unwrap();
    store.close().unwrap();
}
#[test]
fn strict_input_and_duplicate_fields_refuse_without_mutation() {
    let (_f, store, w) = create();
    for value in [
        json!({"schema_version":1,"event_id":"bad","expected_revision":1,"changes":[]}),
        json!({"schema_version":1,"event_id":"bad","expected_revision":1,"changes":[],"undo_of":"batch","origin":"engine"}),
        json!({"schema_version":1,"event_id":"bad","expected_revision":1,"changes":[{"before":null,"after":{"id":"bad/id","title":"T","deliverable":"D","criteria":["C"],"depends":[]}}]}),
    ] {
        assert!(store
            .preview_graph_batch(&w.id, &raw(&value), &mut SystemEnvironment)
            .is_err())
    }
    assert!(swarm_engine::decode_graph_batch(br#"{"schema_version":1,"schema_version":1,"event_id":"x","expected_revision":1,"changes":[]}"#).is_err());
    assert_eq!(store.events(&w.id).unwrap().len(), 1);
    store.close().unwrap();
}

/// Inject only controlled private metadata after a valid preview. This proves
/// the actual guards, not the existence of a native running agent or mission.
#[test]
fn active_or_hierarchical_snapshot_refuses_before_publication() {
    for hierarchical in [false, true] {
        let (f, mut store, w) = create();
        let mut add = batch(
            w.revision,
            json!([{"before":null,"after":definition("a",&[])},{"before":null,"after":definition("b",&[])}]),
        );
        let current = preview_apply(&mut store, &w, &mut add);
        let before = GraphTaskDefinition::observed(&current.tasks.as_ref().unwrap()[1]);
        let mut after = before.clone();
        after.title = "Proposed".into();
        let mut r = batch(current.revision, json!([{"before":before,"after":after}]));
        r["event_id"] = "edit".into();
        let preview = store
            .preview_graph_batch(&w.id, &raw(&r), &mut SystemEnvironment)
            .unwrap();
        r["preview_digest"] = preview.preview_digest.into();
        let mut injected = current.clone();
        if hierarchical {
            injected.planning = Some(Box::default())
        } else {
            injected.tasks.as_mut().unwrap()[0].status = "running".into()
        }
        let db = f.db();
        swarm_engine::register_identity_plan_projection(&db).unwrap();
        db.execute(
            "UPDATE works SET body=? WHERE id=?",
            rusqlite::params![swarm_engine::encode_model(&injected).unwrap(), w.id],
        )
        .unwrap();
        let expected = if hierarchical {
            "graph-batch-hierarchical-work"
        } else {
            "graph-batch-active-work"
        };
        assert_eq!(
            store
                .preview_graph_batch(&w.id, &raw(&r), &mut SystemEnvironment)
                .err()
                .unwrap()
                .code(),
            expected
        );
        assert_eq!(
            store
                .execute_external(&w.id, "graph.batch", &raw(&r), &mut SystemEnvironment)
                .err()
                .unwrap()
                .code(),
            expected
        );
        assert_eq!(store.events(&w.id).unwrap().len(), 2);
        assert_eq!(store.read_work(&w.id).unwrap().revision, 2);
        db.close().unwrap();
        store.close().unwrap();
    }
}

/// Full graph definitions must never inherit patch semantics for empty strings.
/// Valid controls precede refusals; preview/apply preserve bytes/events, then a
/// complete correction and its derived compensation must match their previews.
#[test]
fn complete_definitions_refuse_blank_fields_and_match_recorded_compensation() {
    let (f, mut store, w) = create();
    let mut add = batch(
        w.revision,
        json!([{"before":null,"after":definition("a",&[])}]),
    );
    let current = preview_apply(&mut store, &w, &mut add);
    let before = GraphTaskDefinition::observed(&current.tasks.as_ref().unwrap()[0]);
    let original = fs::read(f.0.join(".swarm/state.db")).unwrap();
    for field in ["title", "deliverable"] {
        for value in ["", " \t\n"] {
            let mut after = serde_json::to_value(&before).unwrap();
            after[field] = value.into();
            let mut request = batch(current.revision, json!([{"before":before,"after":after}]));
            request["event_id"] = "invalid-full-definition".into();
            assert!(
                store
                    .preview_graph_batch(&w.id, &raw(&request), &mut SystemEnvironment)
                    .is_err(),
                "blank full definition accepted: {field}"
            );
            request["preview_digest"] = "0".repeat(64).into();
            assert!(store
                .execute_external(&w.id, "graph.batch", &raw(&request), &mut SystemEnvironment)
                .is_err());
            assert_eq!(store.read_work(&w.id).unwrap(), current);
            assert_eq!(store.events(&w.id).unwrap().len(), 2);
            assert_eq!(fs::read(f.0.join(".swarm/state.db")).unwrap(), original);
        }
    }
    let mut after = before.clone();
    after.title = "Corrected title".into();
    after.deliverable = "docs/corrected.md".into();
    let mut edit = batch(current.revision, json!([{"before":before,"after":after}]));
    edit["event_id"] = "complete-edit".into();
    let preview = store
        .preview_graph_batch(&w.id, &raw(&edit), &mut SystemEnvironment)
        .unwrap();
    assert_eq!(preview.changes[0].after.as_ref().unwrap(), &after);
    let edited = preview_apply(&mut store, &current, &mut edit);
    assert_eq!(
        GraphTaskDefinition::observed(&edited.tasks.as_ref().unwrap()[0]),
        after
    );
    let mut undo = batch(edited.revision, json!([]));
    undo["event_id"] = "complete-undo".into();
    undo["undo_of"] = "complete-edit".into();
    let preview = store
        .preview_graph_batch(&w.id, &raw(&undo), &mut SystemEnvironment)
        .unwrap();
    assert_eq!(preview.changes[0].after.as_ref().unwrap(), &before);
    let restored = preview_apply(&mut store, &edited, &mut undo);
    assert_eq!(
        GraphTaskDefinition::observed(&restored.tasks.as_ref().unwrap()[0]),
        before
    );
    assert_eq!(store.events(&w.id).unwrap().len(), 4);
    store.close().unwrap();
}
