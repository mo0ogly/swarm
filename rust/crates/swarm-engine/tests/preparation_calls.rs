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
        // Operator interdiction (2026-10-31): project fixtures never live under
        // the system temp directory. The recipe pins TMPDIR to the repository
        // fixture area; a bare `cargo test` run must fail here rather than
        // leak fixture roots into `/tmp` (tmpfs, wiped at reboot).
        let area = std::env::var_os("TMPDIR").expect("project-local TMPDIR required");
        let area = std::path::PathBuf::from(area).canonicalize().unwrap();
        assert!(!area.starts_with("/tmp"));
        let path = area.join(format!(
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
        let p=root.preparation_command(br#"{"version":1,"action":"create","event_id":"create","expected_revision":0,"text":"Fixture need"}"#, &mut SystemEnvironment, None).unwrap().value;
        let preparation=root.preparation_command(&serde_json::to_vec(&serde_json::json!({"version":1,"action":"budget","event_id":"budget","preparation_id":p.id,"expected_revision":p.revision,"budget":{"limit_usd":10,"reserve_per_launch_usd":0.5,"estimate_source":"Owned fixture","reference_date":"2026-10-27"}})).unwrap(), &mut SystemEnvironment, None).unwrap().value;
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
            .preparation_send(&self.request(event), &mut SystemEnvironment, None)
            .unwrap()
    }
    /// Exercise pending cancellation through the same public stop operation.
    fn stop(&self, id: &str) {
        assert_eq!(
            self.root
                .preparation_stop(&self.preparation.id, id, &mut SystemEnvironment, None)
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
    /// Remove the attributed fixture root immediately. The still-open SQLite
    /// handles stay valid (POSIX allows unlinking open files) and are closed
    /// by their own `Drop`s when `self` falls out of scope; the unlinked data
    /// is reclaimed then. A failed test retains the root instead, with its
    /// birth record, for fault diagnosis (the [`Drop`] impl keeps it on panic
    /// only).
    fn close(self) {
        std::fs::remove_dir_all(&self.path).unwrap();
    }
}
/// Retain the owned fixture root when the test panics: the birth record and
/// engine state stay for diagnosis. Otherwise remove the tree; after an
/// explicit `close(self)` this is already gone and the second call is a
/// tolerated no-op.
impl Drop for Fixture {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!("PREPARATION_CALL_FAULT_ROOT retained {}", self.path.display());
            return;
        }
        let _ = std::fs::remove_dir_all(&self.path);
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
        .preparation_send(&request, &mut SystemEnvironment, None)
        .err()
        .expect("deferred commit must refuse");
    assert_eq!(error.code(), "preparation-call-commit-uncertain");
    f.db.execute_batch("DROP TRIGGER preparation_commit_fault")
        .unwrap();
    let recovered = f
        .root
        .preparation_send(&request, &mut SystemEnvironment, None)
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
            .preparation_send(&request, &mut SystemEnvironment, None)
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
            .preparation_send(&request, &mut SystemEnvironment, None)
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
        .preparation_send(&raw, &mut SystemEnvironment, None)
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
            None,
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
            None,
        )
        .err()
        .expect("expected explicit refusal");
    assert!(format!("{unsupported:?}").contains("preparation-send-invalid"));
}

/// Exact active historical replay is an observation, never process custody.
/// The real v304 baseline returns the pending DTO; dispatch and worker claim
/// still refuse adoption, preserving both row and reserved budget.
#[test]
fn legacy_active_replay_is_readonly_without_native_authority() {
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
    let conflict = f.root.preparation_send(&swarm_engine::encode_model(&changed).unwrap(), &mut SystemEnvironment, None).err().expect("event conflict required");
    assert!(format!("{conflict:?}").contains("preparation-event-conflict"), "{conflict:?}");
    let observed = f.root.preparation_send(&f.request("legacy"), &mut SystemEnvironment, None).unwrap();
    assert_eq!(observed.id,t.id);assert_eq!(observed.status,"pending");
    assert!(!f.root.preparation_dispatch_if_native(&t.id).unwrap());
    let claim=f.root.preparation_worker_claim(&t.id,"not-a-worker-grant").err().expect("legacy row has no native custody");
    assert!(format!("{claim:?}").contains("preparation-call-private-invalid"));
    assert_eq!(f.db.query_row("SELECT state FROM preparation_reservations WHERE turn_id=?1",[&t.id],|r|r.get::<_,String>(0)).unwrap(),"reserved");
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

/// Reuse the production source builder under explicit synthetic authority,
/// then admit a fixed context for a non-admin principal without any executable
/// turn/reservation. Exercises immutable replay, every-source/clearance checks,
/// private framing corruption/recovery and current-policy revocation on reopen.
#[test]
fn confidential_admission_is_fixed_authenticated_and_never_dispatchable() {
    use serde_json::{json,Value};
    let f=Fixture::new("blp-domain");
    let call=|op:&str,body:Value,token:&str|f.root.trust_as_principal(op,&serde_json::to_vec(&body).unwrap(),token)
        .map(|r|serde_json::from_slice::<Value>(&r.bytes).unwrap());
    let codes=|error:swarm_engine::RootStoreError| {
        let mut out=error.to_string();let mut source=std::error::Error::source(&error);
        while let Some(e)=source {out.push_str(&format!(":{e}"));source=e.source();}out
    };
    call("config-apply",json!({"schema_version":1,"event_id":"init","expected_revision":0,"reason":"owned fixture",
        "policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":true},
        "identities":[{"subject":"admin","integrity":3,"roles":["admin"],"token":"fixture-admin-token"},
        {"subject":"sender","integrity":1,"roles":["requester"],"token":"fixture-sender-token"},
        {"subject":"low","integrity":1,"roles":["requester"],"token":"fixture-low-token"}]}),"").unwrap();
    let policy=json!({"schema_version":1,"levels":[{"id":"public","name_fr":"Public","name_en":"Public"},{"id":"secret","name_fr":"Secret","name_en":"Secret"}],
        "compartments":["a","b"],"clearances":{"admin":{"level":"secret","compartments":["a","b"]},"sender":{"level":"secret","compartments":["a","b"]},"low":{"level":"secret","compartments":["a"]}},"preparation_quarantine":true});
    call("blp-policy-apply",json!({"schema_version":1,"event_id":"policy","expected_revision":0,"expected_trust_revision":1,"reason":"explicit fixture policy","policy":policy}),"fixture-admin-token").unwrap();
    let intent:Value=serde_json::from_slice(&f.request("intent")).unwrap();
    let admission=json!({"schema_version":1,"event_id":"admission","expected_revision":1,"expected_trust_revision":1,"intent":intent});
    assert!(codes(call("blp-preparation-admit",admission.clone(),"").unwrap_err()).contains("authorization_required"));
    assert!(codes(call("blp-preparation-admit",admission.clone(),"fixture-sender-token").unwrap_err()).contains("blp-classification-required"));
    let attribute=|index:usize|json!({"schema_version":1,"event_id":format!("input-{index}"),"expected_revision":1,"expected_trust_revision":1,
        "intent":intent,"source_index":index,"classification":{"level":"secret","compartments":["a","b"]},"reason":"explicit source decision"});
    assert!(codes(call("blp-preparation-input-attribute",attribute(0),"fixture-sender-token").unwrap_err()).contains("authorization_required"));
    let first=call("blp-preparation-input-attribute",attribute(0),"fixture-admin-token").unwrap();
    let count=first["source_count"].as_u64().unwrap() as usize;assert!(count>1);
    assert!(!first.to_string().contains("Owned fixture"));
    for index in 1..count {
        if let Err(e)=call("blp-preparation-input-attribute",attribute(index),"fixture-admin-token") {
            assert!(codes(e).contains("blp-attribution-conflict"),"only exact duplicate source objects may conflict");
        }
    }
    assert!(codes(call("blp-preparation-admit",admission.clone(),"fixture-low-token").unwrap_err()).contains("blp-access-denied"));
    let result=call("blp-preparation-admit",admission.clone(),"fixture-sender-token").unwrap();
    assert_eq!(result["principal"],"sender");assert_eq!(result["source_count"],count);
    assert_eq!(result["output_classification"],json!({"level":"secret","compartments":["a","b"]}));
    assert_eq!(result["runtime_authorized"],false);
    assert_eq!(call("blp-preparation-admit",admission.clone(),"fixture-sender-token").unwrap(),result);
    let show=json!({"schema_version":1,"admission_id":result["admission_id"],"expected_revision":1,"expected_trust_revision":1});
    assert_eq!(call("blp-preparation-admission-show",show.clone(),"fixture-sender-token").unwrap(),result);
    assert!(codes(call("blp-preparation-admission-show",show.clone(),"fixture-admin-token").unwrap_err()).contains("blp-admission-not-found"));
    let mut changed=admission.clone();changed["intent"]["message"]="different source".into();
    assert!(codes(call("blp-preparation-admit",changed.clone(),"fixture-sender-token").unwrap_err()).contains("blp-admission-event-conflict"));
    changed["event_id"]="new-admission".into();
    assert!(codes(call("blp-preparation-admit",changed,"fixture-sender-token").unwrap_err()).contains("blp-classification-required"));
    let id=result["admission_id"].as_str().unwrap();let receipt=f.path.join(".swarm").join(format!("blp-admission-{}.receipt",&id[3..]));
    let original=std::fs::read(&receipt).unwrap();
    std::fs::write(&receipt,b"invalid owned receipt").unwrap();
    assert!(call("blp-preparation-admission-show",show.clone(),"fixture-sender-token").is_err());
    std::fs::write(&receipt,&original).unwrap();
    assert_eq!(call("blp-preparation-admission-show",show.clone(),"fixture-sender-token").unwrap(),result);
    assert_eq!(f.db.query_row("SELECT count(*) FROM preparation_turns",[],|r|r.get::<_,i64>(0)).unwrap(),0);
    assert_eq!(f.db.query_row("SELECT count(*) FROM preparation_reservations",[],|r|r.get::<_,i64>(0)).unwrap(),0);
    let reopened=RootStore::open(&f.path,false).unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&reopened.trust_as_principal("blp-preparation-admission-show",&serde_json::to_vec(&show).unwrap(),"fixture-sender-token").unwrap().bytes).unwrap(),result);reopened.close().unwrap();
    call("blp-policy-apply",json!({"schema_version":1,"event_id":"renewal","expected_revision":1,"expected_trust_revision":1,"reason":"explicit policy revision","policy":policy}),"fixture-admin-token").unwrap();
    let mut new_show=show;new_show["expected_revision"]=2.into();
    assert!(codes(call("blp-preparation-admission-show",new_show,"fixture-sender-token").unwrap_err()).contains("blp-admission-policy-changed"));
    assert_eq!(std::fs::read(&receipt).unwrap(),original);
    f.close();
}
