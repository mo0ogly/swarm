//! Retained-Root fault injection on owned inert installations. No provider or
//! child is executed. Synthetic process registration tests gates, not authority.
#![cfg(target_os = "linux")]
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
    path::PathBuf,
};
use swarm_engine::{
    EngineSettingsChange, EngineSettingsValues, Preparation, PreparationTurn, RootStore,
    SettingCount, SupervisorIdentity, SystemEnvironment,
};
struct Fixture {
    root: RootStore,
    path: PathBuf,
    db: Connection,
    preparation: Preparation,
}
impl Fixture {
    /// Publish fixture attribution before initialization and use only public
    /// settings/preparation writers. Private JSON fixtures carry synthetic keys.
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "swarm-preparation-call-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .unwrap();
        let mut marker = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path.join("fixture-birth.json"))
            .unwrap();
        marker.write_all(serde_json::to_string(&serde_json::json!({"root":path,"owner":"preparation-call-integration","fixture":true,"live":false})).unwrap().as_bytes()).unwrap();
        marker.sync_all().unwrap();
        drop(marker);
        let root = RootStore::initialize(&path, &mut SystemEnvironment).unwrap();
        settings(&root, |s| {
            s.preparation_write_enabled = Some(SettingCount(1));
            s.preparation_call_enabled = Some(SettingCount(1));
        });
        std::fs::write(
            path.join(".swarm/providers.json"),
            br#"{"schema_version":1,"providers":{}}"#,
        )
        .unwrap();
        std::fs::write(path.join(".swarm/ai-connections.json"),br#"{"local":{"id":"local","label":"Owned fixture","base_url":"https://example.invalid/v1","model":"fixed-model","key":"PRIVATE_FIXTURE_KEY","disabled":false}}"#).unwrap();
        for file in ["providers.json", "ai-connections.json"] {
            std::fs::set_permissions(
                path.join(".swarm").join(file),
                std::fs::Permissions::from_mode(0o600),
            )
            .unwrap();
        }
        let p=root.preparation_command(br#"{"version":1,"action":"create","event_id":"create","expected_revision":0,"text":"Fixture need"}"#,&mut SystemEnvironment).unwrap().value;
        let preparation=root.preparation_command(&serde_json::to_vec(&serde_json::json!({"version":1,"action":"budget","event_id":"budget","preparation_id":p.id,"expected_revision":p.revision,"budget":{"limit_usd":10,"reserve_per_launch_usd":0.5,"estimate_source":"Owned fixture","reference_date":"2026-10-27"}})).unwrap(),&mut SystemEnvironment).unwrap().value;
        let db = Connection::open(path.join(".swarm/state.db")).unwrap();
        println!("PREPARATION_CALL_FAULT_ROOT {}", path.display());
        Self {
            root,
            path,
            db,
            preparation,
        }
    }
    /// Bind a fresh explicit send intent to the current saved capability.
    fn request(&self, event: &str) -> Vec<u8> {
        let cap = self.root.preparation_call_capabilities().unwrap();
        let provider = cap
            .providers
            .iter()
            .find(|p| p.name == "api-local")
            .unwrap();
        serde_json::to_vec(&serde_json::json!({"version":1,"preparation_id":self.preparation.id,"expected_revision":self.preparation.revision,"event_id":event,"provider":"api-local","capability_hash":provider.digest,"message":"Owned fixture"})).unwrap()
    }
    /// Admit without dispatch. This library operation never contacts a provider.
    fn admit(&self, event: &str) -> PreparationTurn {
        self.root
            .preparation_send(&self.request(event), &mut SystemEnvironment)
            .unwrap()
    }
    /// Exercise pending cancellation through the same public stop operation.
    fn stop(&self, id: &str) {
        assert_eq!(
            self.root
                .preparation_stop(&self.preparation.id, id, &mut SystemEnvironment)
                .unwrap()
                .status,
            "interrupted"
        );
    }
    /// Register this test's actual kernel identity in synthetic private data.
    /// This is an explicit fixture injection, not production enrollment evidence.
    fn register(&self, id: &str) -> String {
        let raw: Vec<u8> = self
            .db
            .query_row(
                "SELECT CAST(body AS BLOB) FROM preparation_turns WHERE id=?",
                [id],
                |r| r.get(0),
            )
            .unwrap();
        let mut v: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        let own = SupervisorIdentity::current().unwrap();
        v["supervisor_pid"] = own.pid().into();
        v["supervisor_stamp"] = own.process_stamp().into();
        v["host"] = own.host().into();
        let token = format!(
            "{:x}",
            Sha256::digest(v["_native"]["nonce"].as_str().unwrap().as_bytes())
        );
        self.db
            .execute(
                "UPDATE preparation_turns SET body=? WHERE id=?",
                params![serde_json::to_vec(&v).unwrap(), id],
            )
            .unwrap();
        token
    }
    /// Finish an intentionally uncontacted claim without inventing zero cost.
    fn finish(&self, id: &str) {
        self.root
            .preparation_worker_finish(id, None, None, "fixture-no-contact", &mut SystemEnvironment)
            .unwrap();
    }
    /// Close both actual SQLite handles while retaining the attributed fixture.
    fn close(self) {
        self.db.close().unwrap();
        self.root.close().unwrap();
    }
}
/// Apply a complete explicit settings change through preview and CAS.
fn settings(root: &RootStore, change: impl FnOnce(&mut EngineSettingsValues)) {
    let doc = root.settings().unwrap();
    let mut values = doc.values;
    change(&mut values);
    let mut r = EngineSettingsChange {
        schema_version: values.schema_version(),
        event_id: format!("settings-{}", doc.revision.0),
        expected_revision: doc.revision,
        values,
        reason: "Owned inert preparation fixture".into(),
        preview_token: String::new(),
    };
    r.preview_token = root.preview_settings(&r).unwrap().preview_token;
    root.apply_settings(&r, &mut SystemEnvironment).unwrap();
}

/// A deferred COMMIT error must roll back and leave this SAME Root reusable.
/// This reaches commit, not an earlier common refusal, after a valid control.
#[test]
fn commit_failure_rolls_back_for_same_root_reuse() {
    let f = Fixture::new("commit");
    let control = f.admit("control");
    f.stop(&control.id);
    f.db.execute_batch("CREATE TABLE preparation_commit_guard(id TEXT PRIMARY KEY,parent TEXT REFERENCES preparations(id) DEFERRABLE INITIALLY DEFERRED);CREATE TRIGGER preparation_commit_fault AFTER INSERT ON preparation_reservations BEGIN INSERT INTO preparation_commit_guard VALUES(NEW.turn_id,'absent'); END;").unwrap();
    let request = f.request("commit-fault");
    let error = f
        .root
        .preparation_send(&request, &mut SystemEnvironment)
        .err()
        .expect("deferred commit must refuse");
    assert_eq!(error.code(), "preparation-call-commit-uncertain");
    f.db.execute_batch("DROP TRIGGER preparation_commit_fault")
        .unwrap();
    let recovered = f
        .root
        .preparation_send(&request, &mut SystemEnvironment)
        .unwrap();
    f.stop(&recovered.id);
    assert_eq!(
        f.db.query_row("SELECT count(*) FROM preparation_commit_guard", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
    );
    f.close();
}
/// Recheck both turn and ledger after triggers, then recover the same event.
#[test]
fn last_write_witness_refuses_turn_and_budget_drift() {
    let f = Fixture::new("triggers");
    let control = f.admit("control");
    f.stop(&control.id);
    for (label, body) in [
        (
            "amount",
            "UPDATE preparation_reservations SET amount=999 WHERE turn_id=NEW.turn_id;",
        ),
        (
            "turn",
            "UPDATE preparation_turns SET body='{}' WHERE id=NEW.turn_id;",
        ),
    ] {
        let before: i64 =
            f.db.query_row("SELECT count(*) FROM preparation_turns", [], |r| r.get(0))
                .unwrap();
        f.db.execute_batch(&format!("CREATE TRIGGER preparation_call_fault AFTER INSERT ON preparation_reservations BEGIN {body} END;")).unwrap();
        let request = f.request(label);
        assert!(f
            .root
            .preparation_send(&request, &mut SystemEnvironment)
            .is_err());
        assert_eq!(
            f.db.query_row("SELECT count(*) FROM preparation_turns", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            before
        );
        f.db.execute_batch("DROP TRIGGER preparation_call_fault")
            .unwrap();
        let recovered = f
            .root
            .preparation_send(&request, &mut SystemEnvironment)
            .unwrap();
        f.stop(&recovered.id);
    }
    f.close();
}
/// Valid synthetic claim precedes source/policy faults. Neither refused claim
/// may mark the reservation estimated; explicit stop releases only unstarted.
#[test]
fn changed_source_and_policy_refuse_before_contact() {
    let f = Fixture::new("source");
    let control = f.admit("control");
    let token = f.register(&control.id);
    f.root
        .preparation_worker_claim(&control.id, &token)
        .unwrap();
    f.finish(&control.id);
    let source = f.path.join(".swarm/ai-connections.json");
    let original = std::fs::read(&source).unwrap();
    let t = f.admit("source-fault");
    let token = f.register(&t.id);
    let mut changed: serde_json::Value = serde_json::from_slice(&original).unwrap();
    changed["local"]["key"] = "PRIVATE_CHANGED_FIXTURE".into();
    std::fs::write(&source, serde_json::to_vec(&changed).unwrap()).unwrap();
    assert_eq!(
        f.root
            .preparation_worker_claim(&t.id, &token)
            .err()
            .unwrap()
            .code(),
        "preparation-provider-source-changed"
    );
    f.stop(&t.id);
    std::fs::write(&source, original).unwrap();
    let t = f.admit("policy-fault");
    let token = f.register(&t.id);
    settings(&f.root, |s| {
        s.preparation_call_observation_ms = Some(SettingCount(201))
    });
    assert_eq!(
        f.root
            .preparation_worker_claim(&t.id, &token)
            .err()
            .unwrap()
            .code(),
        "preparation-call-policy-changed"
    );
    f.stop(&t.id);
    assert_eq!(
        f.db.query_row(
            "SELECT count(*) FROM preparation_reservations WHERE state='estimated'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    let t = f.admit("recovery");
    let token = f.register(&t.id);
    f.root.preparation_worker_claim(&t.id, &token).unwrap();
    f.finish(&t.id);
    f.close();
}
/// A replaced physical store must not inherit the original's pending authority.
/// Restore only this fixture's directory and prove the original gate can resume.
#[test]
fn physical_root_switch_refuses_and_owned_restore_recovers() {
    let f = Fixture::new("root");
    let control = f.admit("control");
    let token = f.register(&control.id);
    f.root
        .preparation_worker_claim(&control.id, &token)
        .unwrap();
    f.finish(&control.id);
    let t = f.admit("root-fault");
    let token = f.register(&t.id);
    let (directory, retained) = (f.path.join(".swarm"), f.path.join(".swarm-retained"));
    std::fs::rename(&directory, &retained).unwrap();
    let replacement = RootStore::initialize(&f.path, &mut SystemEnvironment).unwrap();
    assert!(f.root.preparation_worker_claim(&t.id, &token).is_err());
    replacement.close().unwrap();
    std::fs::rename(&directory, f.path.join(".swarm-replacement")).unwrap();
    std::fs::rename(retained, directory).unwrap();
    f.root.preparation_worker_claim(&t.id, &token).unwrap();
    f.finish(&t.id);
    f.close();
}

/// Terminal Go-format rows have no native worker envelope. Exact recorded
/// intents remain observable, including their historical CLI/model level, but
/// replay must not rewrite history, reserve budget or manufacture worker rights.
#[test]
fn legacy_terminal_replay_without_native_authority() {
    let f = Fixture::new("legacy-terminal");
    let t = f.admit("legacy");
    let mut request = swarm_engine::decode_preparation_send(&f.request("legacy")).unwrap();
    request.level = "high".into();
    let raw = swarm_engine::encode_model(&request).unwrap();
    let mut stored: serde_json::Value = serde_json::from_slice(
        &f.db
            .query_row(
                "SELECT body FROM preparation_turns WHERE id=?1",
                [&t.id],
                |r| r.get::<_, Vec<u8>>(0),
            )
            .unwrap(),
    )
    .unwrap();
    stored.as_object_mut().unwrap().remove("_native");
    stored["request_hash"] = format!("{:x}", Sha256::digest(&raw)).into();
    stored["status"] = "failed".into();
    stored["error"] = "historical fixture failure".into();
    f.db.execute(
        "UPDATE preparation_turns SET status='failed',body=?2 WHERE id=?1",
        params![t.id, serde_json::to_vec(&stored).unwrap()],
    )
    .unwrap();
    let before: Vec<u8> =
        f.db.query_row(
            "SELECT body FROM preparation_turns WHERE id=?1",
            [&t.id],
            |r| r.get(0),
        )
        .unwrap();
    f.db.execute_batch("CREATE TRIGGER replay_is_readonly BEFORE UPDATE ON preparation_turns BEGIN SELECT RAISE(ABORT,'replay must not mutate'); END;").unwrap();
    let replay = f
        .root
        .preparation_send(&raw, &mut SystemEnvironment)
        .unwrap();
    assert_eq!(replay.id, t.id);
    assert_eq!(replay.status, "failed");
    assert_eq!(replay.error, "historical fixture failure");
    assert!(!serde_json::to_value(&replay)
        .unwrap()
        .as_object()
        .unwrap()
        .contains_key("prompt"));
    assert_eq!(
        before,
        f.db.query_row::<Vec<u8>, _, _>(
            "SELECT body FROM preparation_turns WHERE id=?1",
            [&t.id],
            |r| r.get(0)
        )
        .unwrap()
    );
    request.message = "different intent".into();
    let conflict = f
        .root
        .preparation_send(
            &swarm_engine::encode_model(&request).unwrap(),
            &mut SystemEnvironment,
        )
        .err()
        .expect("expected explicit refusal");
    assert!(format!("{conflict:?}").contains("preparation-event-conflict"));
    request.event_id = "new-high-level".into();
    let unsupported = f
        .root
        .preparation_send(
            &swarm_engine::encode_model(&request).unwrap(),
            &mut SystemEnvironment,
        )
        .err()
        .expect("expected explicit refusal");
    assert!(format!("{unsupported:?}").contains("preparation-send-invalid"));
}

/// Legacy active rows never gain native dispatch or stop authority through an
/// exact replay. The explicit unported refusal retains their row unchanged.
#[test]
fn legacy_active_replay_remains_explicitly_unported() {
    let f = Fixture::new("legacy-active");
    let t = f.admit("legacy");
    let mut stored: serde_json::Value = serde_json::from_slice(
        &f.db
            .query_row(
                "SELECT body FROM preparation_turns WHERE id=?1",
                [&t.id],
                |r| r.get::<_, Vec<u8>>(0),
            )
            .unwrap(),
    )
    .unwrap();
    stored.as_object_mut().unwrap().remove("_native");
    let before = serde_json::to_vec(&stored).unwrap();
    f.db.execute(
        "UPDATE preparation_turns SET body=?2 WHERE id=?1",
        params![t.id, before],
    )
    .unwrap();
    let mut changed = swarm_engine::decode_preparation_send(&f.request("legacy")).unwrap();
    changed.message = "different historical intent".into();
    let conflict = f.root.preparation_send(&swarm_engine::encode_model(&changed).unwrap(), &mut SystemEnvironment).err().expect("event conflict required");
    assert!(format!("{conflict:?}").contains("preparation-event-conflict"), "{conflict:?}");
    let error = f
        .root
        .preparation_send(&f.request("legacy"), &mut SystemEnvironment)
        .err()
        .expect("expected explicit refusal");
    assert!(format!("{error:?}").contains("preparation-dialogue-active-unported"));
    assert_eq!(
        before,
        f.db.query_row::<Vec<u8>, _, _>(
            "SELECT body FROM preparation_turns WHERE id=?1",
            [&t.id],
            |r| r.get(0)
        )
        .unwrap()
    );
}
