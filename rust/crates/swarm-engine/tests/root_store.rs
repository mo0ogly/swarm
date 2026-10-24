#![cfg(unix)]
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use swarm_engine::{EngineSettingsChange, RootStore, SettingCount, SystemEnvironment};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let p =
            std::env::temp_dir().join(swarm_engine::generate_identifier("swarm-root-").unwrap());
        fs::create_dir(&p).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        Self(p)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
}
fn request(event: &str, revision: i64, fields: serde_json::Value) -> Vec<u8> {
    let mut v = fields;
    v["schema_version"] = 1.into();
    v["event_id"] = event.into();
    v["expected_revision"] = revision.into();
    serde_json::to_vec(&v).unwrap()
}
#[test]
fn native_initialization_schema_and_work_writes_need_no_go_runtime() {
    let f = Fixture::new();
    let mut env = SystemEnvironment;
    let mut store = RootStore::initialize(&f.0, &mut env).unwrap();
    assert_eq!(store.settings().unwrap().revision, SettingCount(1));
    assert!(store.list_works().unwrap().is_empty());
    let raw = request(
        "create",
        0,
        serde_json::json!({"title":"Title","objective":"Goal","scope":"Scope","criteria":["Proof"]}),
    );
    let result = store
        .execute_external("", "work.create", &raw, &mut env)
        .unwrap();
    let id = result.result.work.id.clone();
    assert_eq!(result.settings_revision, SettingCount(1));
    assert_eq!(result.storage_attempts, 1);
    store.close().unwrap();
    let store = RootStore::open(&f.0, false).unwrap();
    assert_eq!(store.read_work(&id).unwrap().revision, 1);
    assert_eq!(store.events(&id).unwrap().len(), 1);
    store.close().unwrap();
    let store = RootStore::initialize(&f.0, &mut env).unwrap();
    assert_eq!(store.settings().unwrap().history.len(), 1);
    assert_eq!(store.list_works().unwrap().len(), 1);
    store.close().unwrap();
}
#[test]
fn settings_preview_persistence_reopen_and_actual_consumer_effect() {
    let f = Fixture::new();
    let mut env = SystemEnvironment;
    let mut store = RootStore::initialize(&f.0, &mut env).unwrap();
    let created=store.execute_external("","work.create",&request("create",0,serde_json::json!({"title":"Title","objective":"Goal","scope":"Scope","criteria":["Proof"]})),&mut env).unwrap();
    let id = created.result.work.id;
    store.execute_external(&id,"task.add",&request("add",1,serde_json::json!({"id":"task","title":"Task","deliverable":"report","criteria":["Proof"]})),&mut env).unwrap();
    let edit = request("edit", 2, serde_json::json!({"id":"task","max_attempts":4}));
    assert!(store
        .execute_external(&id, "task.update", &edit, &mut env)
        .is_err());
    let before = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
    let doc = store.settings().unwrap();
    let mut values = doc.values.clone();
    values.task_max_attempts = Some(SettingCount(5));
    let mut change = EngineSettingsChange {
        schema_version: doc.schema_version,
        event_id: "configuration".into(),
        expected_revision: doc.revision,
        values,
        reason: "Operator raises the definition cap".into(),
        preview_token: String::new(),
    };
    change.preview_token = store.preview_settings(&change).unwrap().preview_token;
    assert_eq!(
        fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
        before
    );
    store.apply_settings(&change, &mut env).unwrap();
    store.close().unwrap();
    let mut store = RootStore::open(&f.0, true).unwrap();
    assert_eq!(store.settings().unwrap().revision, SettingCount(2));
    let changed = store
        .execute_external(&id, "task.update", &edit, &mut env)
        .unwrap();
    assert_eq!(changed.settings_revision, SettingCount(2));
    assert_eq!(
        changed.result.work.tasks.as_ref().unwrap()[0].plan_max_attempts,
        4
    );
    let before = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
    assert!(store.apply_settings(&change, &mut env).unwrap().1);
    assert_eq!(
        fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
        before
    );
    store.close().unwrap();
}
#[test]
fn old_unknown_and_nonempty_uninitialized_storage_are_refused_without_data_reset() {
    for epoch in [0, 1, 36, 38] {
        let f = Fixture::new();
        let mut env = SystemEnvironment;
        let store = RootStore::initialize(&f.0, &mut env).unwrap();
        store.close().unwrap();
        let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
        db.pragma_update(None, "user_version", epoch).unwrap();
        db.close().unwrap();
        let before = fs::read(f.0.join(".swarm/state.db")).unwrap();
        assert!(RootStore::open(&f.0, true).is_err());
        assert!(RootStore::initialize(&f.0, &mut env).is_err());
        assert_eq!(fs::read(f.0.join(".swarm/state.db")).unwrap(), before);
    }
}
#[test]
fn redirected_database_or_settings_and_corrupt_history_refuse_without_mutating_originals() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let mut env = SystemEnvironment;
    let store = RootStore::initialize(&f.0, &mut env).unwrap();
    store.close().unwrap();
    let path = f.0.join(".swarm/engine-settings.json");
    let original = fs::read(&path).unwrap();
    fs::rename(&path, f.0.join("original-settings.json")).unwrap();
    symlink(f.0.join("original-settings.json"), &path).unwrap();
    assert!(RootStore::open(&f.0, true).is_err());
    assert_eq!(
        fs::read(f.0.join("original-settings.json")).unwrap(),
        original
    );
    fs::remove_file(&path).unwrap();
    fs::rename(f.0.join("original-settings.json"), &path).unwrap();
    let mut doc: serde_json::Value = serde_json::from_slice(&original).unwrap();
    doc["history"][0]["actor"] = "changed".into();
    fs::write(&path, serde_json::to_vec(&doc).unwrap()).unwrap();
    assert!(RootStore::open(&f.0, true).is_err());
    fs::write(&path, &original).unwrap();
    let db = f.0.join(".swarm/state.db");
    fs::rename(&db, f.0.join("original.db")).unwrap();
    symlink(f.0.join("original.db"), &db).unwrap();
    assert!(RootStore::open(&f.0, true).is_err());
}

fn configure(
    store: &RootStore,
    env: &mut SystemEnvironment,
    event: &str,
    update: impl FnOnce(&mut swarm_engine::EngineSettingsValues),
) -> EngineSettingsChange {
    let doc = store.settings().unwrap();
    let mut values = doc.values;
    update(&mut values);
    let mut change = EngineSettingsChange {
        schema_version: doc.schema_version,
        event_id: event.into(),
        expected_revision: doc.revision,
        values,
        reason: "Explicit test policy".into(),
        preview_token: String::new(),
    };
    change.preview_token = store.preview_settings(&change).unwrap().preview_token;
    store.apply_settings(&change, env).unwrap();
    change
}
#[test]
fn sqlite_busy_retries_use_persisted_count_leave_no_partial_work_and_recover() {
    let f = Fixture::new();
    let mut env = SystemEnvironment;
    let mut store = RootStore::initialize(&f.0, &mut env).unwrap();
    configure(&store, &mut env, "retry-policy", |v| {
        v.storage_busy_timeout_ms = SettingCount(0);
        v.storage_busy_retries = Some(SettingCount(2));
        v.storage_busy_retry_delay_ms = SettingCount(0);
    });
    // Valid control with the same configured root before actual contention.
    let raw = request(
        "create-control",
        0,
        serde_json::json!({"title":"Title","objective":"Goal","scope":"Scope","criteria":["Proof"]}),
    );
    assert_eq!(
        store
            .execute_external("", "work.create", &raw, &mut env)
            .unwrap()
            .storage_attempts,
        1
    );
    let raw = request(
        "create-contended",
        0,
        serde_json::json!({"title":"Title","objective":"Goal","scope":"Scope","criteria":["Proof"]}),
    );
    let blocker = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    blocker.execute_batch("BEGIN EXCLUSIVE").unwrap();
    let error = store
        .execute_external("", "work.create", &raw, &mut env)
        .err()
        .unwrap();
    match error {
        swarm_engine::RootStoreError::Busy {
            attempts,
            settings_revision,
            ..
        } => {
            assert_eq!(attempts, 3);
            assert_eq!(settings_revision, SettingCount(2));
        }
        other => panic!("unexpected {other:?}"),
    }
    blocker.execute_batch("ROLLBACK").unwrap();
    blocker.close().unwrap();
    assert_eq!(store.list_works().unwrap().len(), 1);
    let result = store
        .execute_external("", "work.create", &raw, &mut env)
        .unwrap();
    assert_eq!(result.storage_attempts, 1);
    assert_eq!(store.events(&result.result.work.id).unwrap().len(), 1);
    assert_eq!(store.list_works().unwrap().len(), 2);
    store.close().unwrap();
}
#[test]
fn native_initialization_imports_legacy_retry_once_and_preserves_override_and_layout() {
    let f = Fixture::new();
    let mut env = SystemEnvironment;
    fs::create_dir(f.0.join(".swarm")).unwrap();
    fs::set_permissions(f.0.join(".swarm"), fs::Permissions::from_mode(0o700)).unwrap();
    let old = f.0.join(".swarm/storage-retry.json");
    fs::write(
        &old,
        br#"{"schema_version":1,"busy_retries":7,"busy_retry_delay_ms":99}"#,
    )
    .unwrap();
    fs::set_permissions(&old, fs::Permissions::from_mode(0o644)).unwrap();
    let store = RootStore::initialize(&f.0, &mut env).unwrap();
    assert_eq!(
        store.settings().unwrap().values.storage_busy_retries,
        Some(SettingCount(7))
    );
    assert_eq!(
        store.settings().unwrap().values.storage_busy_retry_delay_ms,
        SettingCount(99)
    );
    assert!(f.0.join(".swarm/views").is_dir());
    assert_eq!(fs::read(f.0.join(".swarm/.gitignore")).unwrap(), b"*\n");
    configure(&store, &mut env, "changed", |v| {
        v.storage_busy_retries = Some(SettingCount(4))
    });
    store.close().unwrap();
    fs::write(f.0.join(".swarm/.gitignore"), b"user content\n").unwrap();
    fs::write(&old, b"invalid legacy file now superseded").unwrap();
    let store = RootStore::initialize(&f.0, &mut env).unwrap();
    assert_eq!(
        store.settings().unwrap().values.storage_busy_retries,
        Some(SettingCount(4))
    );
    assert_eq!(store.settings().unwrap().history.len(), 2);
    assert_eq!(
        fs::read(f.0.join(".swarm/.gitignore")).unwrap(),
        b"user content\n"
    );
    store.close().unwrap();
}
struct NoObservations;
impl swarm_engine::WorkEnvironment for NoObservations {
    fn identifier(&mut self, _: &str) -> Result<String, swarm_engine::RuntimeError> {
        panic!("unexpected identifier")
    }
    fn timestamp(&mut self) -> Result<String, swarm_engine::RuntimeError> {
        panic!("unexpected timestamp")
    }
    fn operator(&mut self) -> Result<String, swarm_engine::RuntimeError> {
        panic!("unexpected operator")
    }
    fn git(
        &mut self,
        _: &std::path::Path,
    ) -> Result<swarm_engine::GitObservation, swarm_engine::RuntimeError> {
        panic!("unexpected git")
    }
}
#[test]
fn rejected_settings_and_exact_replay_do_not_call_environment_or_mutate() {
    let f = Fixture::new();
    let mut env = SystemEnvironment;
    let store = RootStore::initialize(&f.0, &mut env).unwrap();
    let change = configure(&store, &mut env, "change", |v| {
        v.task_max_attempts = Some(SettingCount(5))
    });
    let before = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
    let mut denied = NoObservations;
    assert!(store.apply_settings(&change, &mut denied).unwrap().1);
    let mut conflict = change.clone();
    conflict.reason = "different".into();
    assert!(store.apply_settings(&conflict, &mut denied).is_err());
    conflict.event_id = "new".into();
    assert!(store.apply_settings(&conflict, &mut denied).is_err());
    conflict.expected_revision = SettingCount(2);
    assert!(store.apply_settings(&conflict, &mut denied).is_err());
    assert_eq!(
        fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
        before
    );
    store.close().unwrap();
}

#[test]
fn invalid_legacy_retry_refuses_without_default_fallback_then_recovers() {
    for invalid in [
        br#"{"schema_version":2,"busy_retries":3,"busy_retry_delay_ms":25}"#.as_slice(),
        br#"{"schema_version":1,"busy_retries":11,"busy_retry_delay_ms":25}"#,
        br#"{"schema_version":1,"busy_retries":3,"busy_retry_delay_ms":1001}"#,
        br#"{"schema_version":1,"busy_retries":3,"busy_retry_delay_ms":25,"unknown":true}"#,
        b"not json",
    ] {
        let f = Fixture::new();
        let mut env = SystemEnvironment;
        // Valid control on this exact root before fault introduction.
        let store = RootStore::initialize(&f.0, &mut env).unwrap();
        store.close().unwrap();
        let settings = f.0.join(".swarm/engine-settings.json");
        let original = fs::read(&settings).unwrap();
        fs::remove_file(&settings).unwrap();
        let retry = f.0.join(".swarm/storage-retry.json");
        fs::write(&retry, invalid).unwrap();
        let database = fs::read(f.0.join(".swarm/state.db")).unwrap();
        assert!(RootStore::initialize(&f.0, &mut env).is_err());
        assert!(!settings.exists());
        assert_eq!(fs::read(&retry).unwrap(), invalid);
        assert_eq!(fs::read(f.0.join(".swarm/state.db")).unwrap(), database);
        fs::remove_file(&retry).unwrap();
        fs::write(&settings, &original).unwrap();
        fs::set_permissions(&settings, fs::Permissions::from_mode(0o600)).unwrap();
        let store = RootStore::initialize(&f.0, &mut env).unwrap();
        assert_eq!(store.settings().unwrap().revision, SettingCount(1));
        store.close().unwrap();
    }
}
#[test]
fn symlinked_layout_refuses_before_database_or_settings_mutation_and_recovers() {
    use std::os::unix::fs::symlink;
    for name in ["views", "imports"] {
        let f = Fixture::new();
        let mut env = SystemEnvironment;
        let store = RootStore::initialize(&f.0, &mut env).unwrap();
        store.close().unwrap();
        let directory = f.0.join(".swarm").join(name);
        if directory.exists() {
            fs::remove_dir(&directory).unwrap();
        }
        let external = f.0.join("outside");
        fs::create_dir(&external).unwrap();
        symlink(&external, &directory).unwrap();
        let db = fs::read(f.0.join(".swarm/state.db")).unwrap();
        let settings = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
        assert!(RootStore::open(&f.0, true).is_err());
        assert!(RootStore::initialize(&f.0, &mut env).is_err());
        assert_eq!(fs::read(f.0.join(".swarm/state.db")).unwrap(), db);
        assert_eq!(
            fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
            settings
        );
        assert_eq!(fs::read_dir(&external).unwrap().count(), 0);
        fs::remove_file(&directory).unwrap();
        let store = RootStore::initialize(&f.0, &mut env).unwrap();
        store.close().unwrap();
    }
}

#[test]
fn root_external_frame_limit_rejects_before_environment_and_preserves_state_then_recovers() {
    let f = Fixture::new();
    let mut env = SystemEnvironment;
    let mut store = RootStore::initialize(&f.0, &mut env).unwrap();
    let control=store.execute_external("","work.create",&request("frame-control",0,serde_json::json!({"title":"Title","objective":"Goal","scope":"Scope","criteria":["Proof"]})),&mut env).unwrap();
    let id = control.result.work.id;
    let database = fs::read(f.0.join(".swarm/state.db")).unwrap();
    let settings = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
    let oversized = vec![b' '; swarm_engine::COMMAND_INPUT_MAX_BYTES + 1];
    let error = store
        .execute_external(&id, "work.update", &oversized, &mut NoObservations)
        .err()
        .unwrap();
    assert_eq!(error.code(), "input-document-too-large");
    assert_eq!(fs::read(f.0.join(".swarm/state.db")).unwrap(), database);
    assert_eq!(
        fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
        settings
    );
    let result = store
        .execute_external(
            &id,
            "work.update",
            &request(
                "after-oversize",
                1,
                serde_json::json!({"title":"Recovered"}),
            ),
            &mut env,
        )
        .unwrap();
    assert_eq!(result.result.work.revision, 2);
    assert_eq!(store.events(&id).unwrap().len(), 2);
    store.close().unwrap();
}

#[test]
fn schema_cross_kind_name_collision_cannot_hide_an_extra_core_trigger() {
    let f = Fixture::new();
    let mut env = SystemEnvironment;
    RootStore::initialize(&f.0, &mut env)
        .unwrap()
        .close()
        .unwrap();
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    db.execute_batch("CREATE TRIGGER WORKS AFTER INSERT ON works BEGIN SELECT 1;END;")
        .unwrap();
    db.close().unwrap();
    match RootStore::open(&f.0, true) {
        Ok(store) => {
            store.close().unwrap();
            panic!("extra trigger hidden by table name collision")
        }
        Err(error) => assert_eq!(error.code(), "root-schema-mismatch"),
    }
}

fn restore_fixture_database(path: &std::path::Path, raw: &[u8]) {
    use std::{io::Write, os::unix::fs::OpenOptionsExt};
    let temporary = path.with_extension("fixture-restore");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)
        .unwrap();
    file.write_all(raw).unwrap();
    file.sync_all().unwrap();
    nix::unistd::close(file).unwrap();
    fs::rename(&temporary, path).unwrap();
    let directory = fs::File::open(path.parent().unwrap()).unwrap();
    directory.sync_all().unwrap();
    nix::unistd::close(directory).unwrap();
}
#[test]
fn root_rejects_missing_or_modified_sql_contract_without_repairing_data_and_recovers() {
    let variants=[
        "DROP TABLE works;",
        "DROP INDEX agent_log_cursor;",
        "ALTER TABLE works ADD COLUMN surprise TEXT;",
        "DROP TRIGGER agent_identity_plan_update;CREATE TRIGGER agent_identity_plan_update AFTER UPDATE OF body ON works BEGIN SELECT 1;END;",
        "CREATE TRIGGER sqliteXextra AFTER INSERT ON works BEGIN SELECT 1;END;",
        "DROP TABLE works;CREATE VIEW works AS SELECT 'fake' AS id,0 AS revision,'{}' AS body;",
    ];
    for sql in variants {
        let f = Fixture::new();
        let mut env = SystemEnvironment;
        let mut store = RootStore::initialize(&f.0, &mut env).unwrap();
        let created=store.execute_external("","work.create",&request("schema-control",0,serde_json::json!({"title":"Preserved","objective":"Goal","scope":"Scope","criteria":["Proof"]})),&mut env).unwrap();
        let id = created.result.work.id;
        store.close().unwrap();
        let path = f.0.join(".swarm/state.db");
        let original = fs::read(&path).unwrap();
        fs::write(f.0.join("original.db"), &original).unwrap();
        let settings = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
        let db = rusqlite::Connection::open(&path).unwrap();
        db.pragma_update(None, "foreign_keys", false).unwrap();
        db.execute_batch(sql).unwrap();
        assert_eq!(
            db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            37
        );
        db.close().unwrap();
        let altered = fs::read(&path).unwrap();
        for writable in [false, true] {
            let error = RootStore::open(&f.0, writable)
                .err()
                .expect("invalid schema accepted");
            assert_eq!(error.code(), "root-schema-mismatch", "{sql}");
        }
        let error = RootStore::initialize(&f.0, &mut NoObservations)
            .err()
            .expect("invalid root declared initialized");
        assert_eq!(error.code(), "root-schema-mismatch");
        assert_eq!(fs::read(&path).unwrap(), altered);
        assert_eq!(
            fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
            settings
        );
        restore_fixture_database(&path, &original);
        let store = RootStore::initialize(&f.0, &mut NoObservations).unwrap();
        assert_eq!(store.read_work(&id).unwrap().title, "Preserved");
        assert_eq!(store.events(&id).unwrap().len(), 1);
        store.close().unwrap();
    }
}
#[test]
fn root_schema_accepts_unrelated_objects_and_existing_projection_v2_contract() {
    let f = Fixture::new();
    let mut env = SystemEnvironment;
    let mut store = RootStore::initialize(&f.0, &mut env).unwrap();
    let created=store.execute_external("","work.create",&request("v2-control",0,serde_json::json!({"title":"Preserved","objective":"Goal","scope":"Scope","criteria":["Proof"]})),&mut env).unwrap();
    let id = created.result.work.id;
    store.close().unwrap();
    let mut db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    swarm_engine::register_identity_plan_projection(&db).unwrap();
    db.execute_batch("CREATE TABLE caller_notes(note TEXT);CREATE TRIGGER caller_notes AFTER INSERT ON caller_notes BEGIN SELECT 1;END;INSERT INTO caller_notes VALUES('preserve');").unwrap();
    let mut tx = db.transaction().unwrap();
    swarm_engine::upgrade_identity_plan_projection(&mut tx).unwrap();
    tx.commit().unwrap();
    db.close().unwrap();
    let mut store = RootStore::open(&f.0, true).unwrap();
    assert_eq!(store.read_work(&id).unwrap().revision, 1);
    let changed = store
        .execute_external(
            &id,
            "work.update",
            &request("v2-update", 1, serde_json::json!({"title":"Still valid"})),
            &mut env,
        )
        .unwrap();
    assert_eq!(changed.result.work.revision, 2);
    store.close().unwrap();
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    assert_eq!(
        db.query_row("SELECT note FROM caller_notes", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "preserve"
    );
    assert_eq!(
        db.query_row(
            "SELECT projection_version FROM agent_identity_plan_heads",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        2
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*)FROM agent_identity_plan_projection_history",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    db.close().unwrap();
}

#[test]
fn persisted_v1_upgrade_preserves_replay_and_changes_output_retention_after_reopen() {
    use std::io::Write;
    use swarm_engine::{EngineSettingsDocument, SettingCap, ValidationOutput};
    let f = Fixture::new();
    let mut env = SystemEnvironment;
    RootStore::initialize(&f.0, &mut env)
        .unwrap()
        .close()
        .unwrap();
    let file = f.0.join(".swarm/engine-settings.json");
    let original = include_bytes!("fixtures/legacy-v1-document.json");
    // Only our private fixture; public API intentionally does not replace history.
    fs::write(&file, original).unwrap();
    let store = RootStore::open(&f.0, true).unwrap();
    let old = store.settings().unwrap();
    assert_eq!(old.schema_version, 1);
    assert!(ValidationOutput::from_settings(&old.values).is_err());
    let mut change = EngineSettingsChange {
        schema_version: 2,
        event_id: "explicit-v2".into(),
        expected_revision: old.revision,
        values: old.values.clone(),
        reason: "Adopt explicit output cap".into(),
        preview_token: String::new(),
    };
    change.values.validation_max_output_bytes = Some(SettingCap(Some(SettingCount(5))));
    change.preview_token = store.preview_settings(&change).unwrap().preview_token;
    assert_eq!(fs::read(&file).unwrap(), original);
    store.apply_settings(&change, &mut env).unwrap();
    store.close().unwrap();
    let store = RootStore::open(&f.0, true).unwrap();
    let new = store.settings().unwrap();
    assert_eq!(new.schema_version, 2);
    assert_eq!(&new.history[..old.history.len()], &old.history);
    let mut output = ValidationOutput::from_settings(&new.values).unwrap();
    std::io::copy(&mut &b"abcdefghijk"[..], &mut output).unwrap();
    output.write_all(b"lm").unwrap();
    assert_eq!(output.retained(), b"abcde");
    assert_eq!(output.total(), 13);
    assert!(output.truncated());
    let replay: EngineSettingsChange =
        serde_json::from_slice(include_bytes!("fixtures/legacy-v1-request.json")).unwrap();
    let bytes = fs::read(&file).unwrap();
    let (observed, replayed) = store.apply_settings(&replay, &mut NoObservations).unwrap();
    assert!(replayed);
    assert_eq!(observed, new);
    assert_eq!(fs::read(&file).unwrap(), bytes);
    assert_eq!(EngineSettingsDocument::decode(&bytes).unwrap(), new);
    store.close().unwrap();
}

/// Creation consumes the same saved criterion-count policy as definition edits.
/// A valid control precedes rejection; a saved increase recovers the unused event
/// after reopen without changing Work/history during the refused transaction.
#[test]
fn task_creation_obeys_saved_criterion_count_after_reopen_and_recovers() {
    let f = Fixture::new();
    let mut env = SystemEnvironment;
    let mut store = RootStore::initialize(&f.0, &mut env).unwrap();
    let created=store.execute_external("","work.create",&request("create",0,serde_json::json!({"title":"Task policy","objective":"Actual consumer","scope":"Private","criteria":["Proof"]})),&mut env).unwrap().result.work;
    configure(&store, &mut env, "criteria-two", |v| {
        v.task_max_criteria = Some(SettingCount(2))
    });
    store.close().unwrap();
    let mut store = RootStore::open(&f.0, true).unwrap();
    let control=store.execute_external(&created.id,"task.add",&request("control",created.revision,serde_json::json!({"id":"control","title":"Task","deliverable":"Report","criteria":["A","B"]})),&mut env).unwrap().result.work;
    let raw = request(
        "three",
        control.revision,
        serde_json::json!({"id":"three","title":"Task","deliverable":"Report","criteria":["A","B","C"]}),
    );
    let before = store.events(&created.id).unwrap();
    assert!(
        store
            .execute_external(&created.id, "task.add", &raw, &mut env)
            .is_err(),
        "creation ignored saved criteria limit"
    );
    assert_eq!(store.read_work(&created.id).unwrap(), control);
    assert_eq!(store.events(&created.id).unwrap(), before);
    configure(&store, &mut env, "criteria-three", |v| {
        v.task_max_criteria = Some(SettingCount(3))
    });
    store.close().unwrap();
    let mut store = RootStore::open(&f.0, true).unwrap();
    let recovered = store
        .execute_external(&created.id, "task.add", &raw, &mut env)
        .unwrap();
    assert_eq!(recovered.result.work.revision, control.revision + 1);
    assert!(!recovered.result.replayed);
    assert!(
        store
            .execute_external(&created.id, "task.add", &raw, &mut env)
            .unwrap()
            .result
            .replayed
    );
    let current = store.read_work(&created.id).unwrap();
    let many = request(
        "unlimited-thirteen",
        current.revision,
        serde_json::json!({"id":"unlimited","title":"Task","deliverable":"Report","criteria":(1..=13).map(|i|format!("Criterion {i}")).collect::<Vec<_>>()}),
    );
    assert!(store
        .execute_external(&created.id, "task.add", &many, &mut env)
        .is_err());
    configure(&store, &mut env, "criteria-unlimited", |v| {
        v.task_max_criteria = None
    });
    store.close().unwrap();
    let mut store = RootStore::open(&f.0, true).unwrap();
    let unlimited = store
        .execute_external(&created.id, "task.add", &many, &mut env)
        .unwrap()
        .result
        .work;
    assert_eq!(
        unlimited
            .tasks
            .as_ref()
            .unwrap()
            .last()
            .unwrap()
            .plan_checks
            .as_ref()
            .unwrap()
            .len(),
        16
    );
    store.close().unwrap();
}
