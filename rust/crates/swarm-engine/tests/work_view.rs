#![cfg(target_os = "linux")]
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
struct Fixture(PathBuf);
impl Fixture {
    fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
}
/// Allocate an owned isolated root; failed checks retain it for diagnosis.
fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(swarm_engine::generate_identifier("work-view-").unwrap());
    fs::create_dir(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    Fixture(path)
}
use swarm_engine::{
    EngineSettingsChange, EngineSettingsValues, RootStore, SettingCap, SettingCount,
    SystemEnvironment, WorkViewLanguage,
};
/// Change through public preview/CAS with history retained, never by journal edit.
fn change(store: &RootStore, event: &str, edit: impl FnOnce(&mut EngineSettingsValues)) {
    let doc = store.settings().unwrap();
    let mut values = doc.values;
    edit(&mut values);
    let mut request = EngineSettingsChange {
        schema_version: values.schema_version(),
        event_id: event.into(),
        expected_revision: doc.revision,
        values,
        reason: "Actual view policy".into(),
        preview_token: String::new(),
    };
    request.preview_token = store.preview_settings(&request).unwrap().preview_token;
    store
        .apply_settings(&request, &mut SystemEnvironment)
        .unwrap();
}
/// Persist a versioned external request through the actual transaction owner.
fn mutate(
    store: &mut RootStore,
    id: &str,
    kind: &str,
    event: &str,
    revision: i64,
    mut fields: serde_json::Value,
) -> swarm_engine::Work {
    fields["schema_version"] = 1.into();
    fields["event_id"] = event.into();
    fields["expected_revision"] = revision.into();
    store
        .execute_external(
            id,
            kind,
            &serde_json::to_vec(&fields).unwrap(),
            &mut SystemEnvironment,
        )
        .unwrap()
        .result
        .work
}
/// The actual persisted settings reach rendering after reopen. Selected malformed
/// decisions refuse before publication, preserve original errors/state/files and
/// recover after the private fixture's original payload is restored.
#[test]
fn root_render_uses_saved_wrap_and_decision_limits_and_refuses_malformed_selected_event() {
    let root = fixture();
    let path = root.path();
    let mut store = RootStore::initialize(path, &mut SystemEnvironment).unwrap();
    let mut work = mutate(
        &mut store,
        "",
        "work.create",
        "create",
        0,
        serde_json::json!({"title":"Fixture","objective":"Proof","scope":"Private","criteria":["Observed"]}),
    );
    for event in ["first", "second"] {
        work = mutate(
            &mut store,
            &work.id,
            "ooda",
            event,
            work.revision,
            serde_json::json!({"observation":event,"orientation":"Context","decision":"Proceed","owner":"human","next":"Implement"}),
        );
    }
    let view = store
        .render_work_view(&work.id, None, WorkViewLanguage::French)
        .unwrap();
    assert!(view.resume_markdown.contains("observation=first"));
    assert!(view.resume_markdown.contains("observation=second"));
    change(&store, "one", |v| {
        v.work_view_recent_decisions = Some(SettingCap(Some(SettingCount(1))));
        v.work_view_wrap_characters = Some(SettingCap(None));
    });
    store.close().unwrap();
    let store = RootStore::open(path, true).unwrap();
    let one = store
        .render_work_view(&work.id, None, WorkViewLanguage::English)
        .unwrap();
    assert!(!one.resume_markdown.contains("observation=first"));
    assert!(one.resume_markdown.contains("observation=second"));
    change(&store, "zero", |v| {
        v.work_view_recent_decisions = Some(SettingCap(Some(SettingCount(0))));
        v.work_view_wrap_characters = Some(SettingCap(Some(SettingCount(16))));
    });
    store.close().unwrap();
    let store = RootStore::open(path, true).unwrap();
    let zero = store
        .render_work_view(&work.id, None, WorkViewLanguage::French)
        .unwrap();
    assert!(!zero.resume_markdown.contains("observation="));
    assert!(zero
        .resume_markdown
        .lines()
        .all(|l| l.chars().count() <= 16));
    change(&store, "all", |v| {
        v.work_view_recent_decisions = Some(SettingCap(None));
        v.work_view_wrap_characters = Some(SettingCap(None));
    });
    let original = store
        .render_work_view(&work.id, None, WorkViewLanguage::French)
        .unwrap();
    store.close().unwrap();
    let viewfile = path.join(format!(".swarm/views/{}.md", work.id));
    let saved_view = fs::read(&viewfile).unwrap();
    let database = path.join(".swarm/state.db");
    let db = rusqlite::Connection::open(&database).unwrap();
    let old: Vec<u8> = db
        .query_row("SELECT payload FROM events WHERE id='second'", [], |r| {
            r.get(0)
        })
        .unwrap();
    db.execute(
        "UPDATE events SET payload=? WHERE id='second'",
        [b"{private-malformed".as_slice()],
    )
    .unwrap();
    db.close().unwrap();
    let damaged = fs::read(&database).unwrap();
    let store = RootStore::open(path, true).unwrap();
    let error = store
        .render_work_view(&work.id, None, WorkViewLanguage::French)
        .unwrap_err();
    assert_eq!(error.code(), "work-view");
    assert!(!format!("{error:?}").contains("private-malformed"));
    assert!(std::error::Error::source(&error).is_some());
    assert_eq!(fs::read(&viewfile).unwrap(), saved_view);
    assert_eq!(store.read_work(&work.id).unwrap(), work);
    store.close().unwrap();
    assert_eq!(fs::read(&database).unwrap(), damaged);
    let db = rusqlite::Connection::open(&database).unwrap();
    db.execute("UPDATE events SET payload=? WHERE id='second'", [old])
        .unwrap();
    db.close().unwrap();
    let store = RootStore::open(path, true).unwrap();
    let recovery = store
        .render_work_view(&work.id, None, WorkViewLanguage::French)
        .unwrap();
    assert_eq!(recovery.resume_markdown, original.resume_markdown);
    store.close().unwrap();
}

/// Read-only historical data fixture, not an authenticated waiver or execution.
/// The actual persisted row passes real FK/UDF constraints. List counts fresh
/// waivers while validated remains the stricter accepted-with-gate count.
#[test]
fn root_list_counts_declared_historical_waiver_separately_from_validated() {
    let root = fixture();
    let path = root.path();
    let mut store = RootStore::initialize(path, &mut SystemEnvironment).unwrap();
    let work = mutate(
        &mut store,
        "",
        "work.create",
        "create",
        0,
        serde_json::json!({"title":"Historical fixture","objective":"Data","scope":"Private","criteria":["Proof"]}),
    );
    let mut work = mutate(
        &mut store,
        &work.id,
        "task.add",
        "add",
        work.revision,
        serde_json::json!({"id":"task","title":"Task","deliverable":"Report","criteria":["Proof"]}),
    );
    store.close().unwrap();
    let task = &mut work.tasks.as_mut().unwrap()[0];
    task.status = "waived".into();
    task.manual_override = Some(Box::new(swarm_engine::ManualOverride {
        reason: "Historical fixture data only".into(),
        actor: "fixture".into(),
        at: "2026-10-18T00:00:00Z".into(),
        ..Default::default()
    }));
    let db = rusqlite::Connection::open(path.join(".swarm/state.db")).unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    swarm_engine::register_identity_plan_projection(&db).unwrap();
    db.execute(
        "UPDATE works SET body=? WHERE id=?",
        rusqlite::params![
            String::from_utf8(swarm_engine::encode_model(&work).unwrap()).unwrap(),
            work.id
        ],
    )
    .unwrap();
    db.close().unwrap();
    let store = RootStore::open(path, false).unwrap();
    let observed = store
        .inspect_work_validation(&work.id, WorkViewLanguage::French)
        .unwrap();
    assert_eq!(observed.validation.validated, 0);
    assert!(observed.validation.tasks["task"].fresh);
    let text = store
        .inspect_work_list_text(WorkViewLanguage::French)
        .unwrap();
    assert!(text.contains("1/1"), "{text}");
    store.close().unwrap();
}
