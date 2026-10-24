#![cfg(unix)]
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
use swarm_engine::{
    EngineSettingsChange, GitObservation, LocalStorageOperation, RootStore, RootStoreError,
    RuntimeError, SettingCount, SystemEnvironment, WorkEnvironment,
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(swarm_engine::generate_identifier("root-gate-").unwrap());
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap()
        }
    }
}
fn request(event: &str, revision: i64, mut fields: serde_json::Value) -> Vec<u8> {
    fields["schema_version"] = 1.into();
    fields["event_id"] = event.into();
    fields["expected_revision"] = revision.into();
    serde_json::to_vec(&fields).unwrap()
}
struct NoObservations;
impl WorkEnvironment for NoObservations {
    fn identifier(&mut self, _: &str) -> Result<String, RuntimeError> {
        panic!("unexpected identifier")
    }
    fn timestamp(&mut self) -> Result<String, RuntimeError> {
        panic!("unexpected timestamp")
    }
    fn operator(&mut self) -> Result<String, RuntimeError> {
        panic!("unexpected operator")
    }
    fn git(&mut self, _: &Path) -> Result<GitObservation, RuntimeError> {
        panic!("unexpected Git")
    }
}
fn seed(store: &mut RootStore) -> swarm_engine::Work {
    let mut env = SystemEnvironment;
    let created=store.execute_external("","work.create",&request("create",0,serde_json::json!({"title":"Gate proof","objective":"Native transaction","scope":"Fixture","criteria":["Proof"]})),&mut env).unwrap().result.work;
    let added=store.execute_external(&created.id,"task.add",&request("add",created.revision,serde_json::json!({"id":"task","title":"Task","deliverable":"Report","criteria":["Initial"]})),&mut env).unwrap().result.work;
    store
        .execute_external(
            &added.id,
            "task.update",
            &request(
                "criteria",
                added.revision,
                serde_json::json!({"id":"task","criteria":["Amended"]}),
            ),
            &mut env,
        )
        .unwrap()
        .result
        .work
}
fn gate_document(task: &swarm_engine::Task) -> serde_json::Value {
    let mut checks:Vec<_>=task.plan_checks.as_ref().unwrap().iter().map(|(id,phase)|serde_json::json!({"id":id,"gate":phase,"mandatory":true,"severity":"minor","penalty":1,"max_penalty":100})).collect();
    let mut results:Vec<_>=checks.iter().map(|c|serde_json::json!({"id":c["id"],"status":"PASS","count":0,"evidence":["proof.txt"]})).collect();
    checks.push(serde_json::json!({"id":"quality","domain":"quality","gate":"delivery","mandatory":false,"severity":"minor","penalty":20,"max_penalty":100}));
    results.push(
        serde_json::json!({"id":"quality","status":"FAIL","count":1,"evidence":["proof.txt"]}),
    );
    serde_json::json!({"method_version":"3","scope_id":"task","artifacts":{"proof.txt":format!("{:x}",Sha256::digest(b"proof"))},"domains":{"quality":1},"checks":checks,"results":results})
}
fn policy_change(store: &RootStore, event: &str, value: i64) -> EngineSettingsChange {
    let doc = store.settings().unwrap();
    let mut change = EngineSettingsChange {
        schema_version: doc.schema_version,
        event_id: event.into(),
        expected_revision: doc.revision,
        values: doc.values,
        reason: "Persisted quality bar for actual gate".into(),
        preview_token: String::new(),
    };
    change.values.quality_delivery_min_percent = Some(SettingCount(value));
    change.preview_token = store.preview_settings(&change).unwrap().preview_token;
    change
}
struct ChangeDuringStamp {
    root: PathBuf,
    change: EngineSettingsChange,
    blocked: bool,
}
impl WorkEnvironment for ChangeDuringStamp {
    fn identifier(&mut self, p: &str) -> Result<String, RuntimeError> {
        SystemEnvironment.identifier(p)
    }
    fn timestamp(&mut self) -> Result<String, RuntimeError> {
        if !self.blocked {
            let store = RootStore::open(&self.root, true).unwrap();
            let error = store
                .apply_settings(&self.change, &mut SystemEnvironment)
                .unwrap_err();
            assert!(
                matches!(error,RootStoreError::Storage(ref e)if e.operation==LocalStorageOperation::Lock)
            );
            store.close().unwrap();
            self.blocked = true;
        }
        SystemEnvironment.timestamp()
    }
    fn operator(&mut self) -> Result<String, RuntimeError> {
        SystemEnvironment.operator()
    }
    fn git(&mut self, p: &Path) -> Result<GitObservation, RuntimeError> {
        SystemEnvironment.git(p)
    }
}
#[test]
fn root_gate_holds_policy_through_commit_and_does_not_revive_receipts_after_revert() {
    let f = Fixture::new();
    let mut store = RootStore::initialize(&f.0, &mut SystemEnvironment).unwrap();
    fs::write(f.0.join("proof.txt"), b"proof").unwrap();
    let w = seed(&mut store);
    let doc = gate_document(&w.tasks.as_ref().unwrap()[0]);
    let control = policy_change(&store, "lock-free-control", 70);
    store
        .apply_settings(&control, &mut SystemEnvironment)
        .unwrap();
    let baseline_revision = store.settings().unwrap().revision;
    let change = policy_change(&store, "raise", 75);
    let mut env = ChangeDuringStamp {
        root: f.0.clone(),
        change: change.clone(),
        blocked: false,
    };
    let raw = request(
        "gate",
        w.revision,
        serde_json::json!({"task_id":"task","phase":"delivery","name":"Delivery proof","document":doc}),
    );
    let saved = store
        .execute_external(&w.id, "gate", &raw, &mut env)
        .unwrap();
    assert!(env.blocked);
    assert_eq!(saved.settings_revision, baseline_revision);
    assert_eq!(saved.result.work.revision, w.revision + 1);
    assert_eq!(store.settings().unwrap().revision, baseline_revision);
    let observed = store.inspect_gate_evidence(&w.id, "task").unwrap();
    assert_eq!(observed.evaluation.quality.unwrap().0, 80.0);
    assert!(observed.evaluation.ship_allowed);
    assert_eq!(observed.work_revision, saved.result.work.revision);
    store.close().unwrap();
    let mut store = RootStore::open(&f.0, true).unwrap();
    assert_eq!(
        store
            .inspect_gate_evidence(&w.id, "task")
            .unwrap()
            .policy
            .generation,
        SettingCount(0)
    );
    let events = store.events(&w.id).unwrap().len();
    let replay = store
        .execute_external(&w.id, "gate", &raw, &mut NoObservations)
        .unwrap();
    assert!(replay.result.replayed);
    assert_eq!(store.events(&w.id).unwrap().len(), events);
    store
        .apply_settings(&change, &mut SystemEnvironment)
        .unwrap();
    assert!(store.inspect_gate_evidence(&w.id, "task").is_err());
    let revert = policy_change(&store, "revert", 70);
    store
        .apply_settings(&revert, &mut SystemEnvironment)
        .unwrap();
    assert_eq!(
        store
            .settings()
            .unwrap()
            .quality_policy_snapshot()
            .unwrap()
            .generation,
        SettingCount(2)
    );
    assert!(store.inspect_gate_evidence(&w.id, "task").is_err());
    let current = store.read_work(&w.id).unwrap();
    assert_eq!(current, saved.result.work);
    let fresh = request(
        "new-gate-without-reorientation",
        current.revision,
        serde_json::json!({"task_id":"task","phase":"delivery","name":"Changed policy","document":doc}),
    );
    assert!(store
        .execute_external(&w.id, "gate", &fresh, &mut NoObservations)
        .is_err());
    assert_eq!(store.read_work(&w.id).unwrap(), current);
    store.close().unwrap();
}
fn refusal_preserves_rows_events_and_required_checks(defect: &str) {
    let f = Fixture::new();
    let mut store = RootStore::initialize(&f.0, &mut SystemEnvironment).unwrap();
    fs::write(f.0.join("proof.txt"), b"proof").unwrap();
    let w = seed(&mut store);
    let doc = gate_document(&w.tasks.as_ref().unwrap()[0]);
    let raw = request(
        "valid",
        w.revision,
        serde_json::json!({"task_id":"task","phase":"delivery","name":"Proof","document":doc}),
    );
    let saved = store
        .execute_external(&w.id, "gate", &raw, &mut SystemEnvironment)
        .unwrap()
        .result
        .work;
    let count = store.events(&w.id).unwrap().len();
    store.inspect_gate_evidence(&w.id, "task").unwrap();
    let mut frame =
        serde_json::json!({"task_id":"task","phase":"delivery","name":"Proof","document":doc});
    match defect {
        "alias" => {
            let required = frame["document"]["checks"].clone();
            frame["document"]["CHECKS"] = required;
            frame["document"]["checks"][0]["mandatory"] = false.into();
        }
        "scope" => frame["document"]["scope_id"] = "other".into(),
        "name" => frame["name"] = " ".into(),
        "actor-policy" => frame["policy"] = serde_json::json!({"generation":0}),
        "missing-task" => frame["task_id"] = "missing".into(),
        "stale-artifact" => {
            fs::write(f.0.join("proof.txt"), b"changed").unwrap();
        }
        _ => panic!("unknown refusal fixture"),
    }
    let bad = request(defect, saved.revision, frame);
    let error = store
        .execute_external(&w.id, "gate", &bad, &mut NoObservations)
        .err()
        .expect("invalid gate must fail");
    assert_eq!(
        error.code(),
        if defect == "actor-policy" {
            "format"
        } else {
            "gate-evidence-invalid"
        },
        "{defect}"
    );
    assert_eq!(store.read_work(&w.id).unwrap(), saved);
    assert_eq!(store.events(&w.id).unwrap().len(), count);
    fs::write(f.0.join("proof.txt"), b"proof").unwrap();
    store.inspect_gate_evidence(&w.id, "task").unwrap();
    store.close().unwrap();
}

#[test]
fn root_gate_rejects_required_alias_without_mutation() {
    refusal_preserves_rows_events_and_required_checks("alias")
}
#[test]
fn root_gate_rejects_scope_mismatch_without_mutation() {
    refusal_preserves_rows_events_and_required_checks("scope")
}
#[test]
fn root_gate_rejects_empty_name_without_mutation() {
    refusal_preserves_rows_events_and_required_checks("name")
}
#[test]
fn root_gate_rejects_caller_policy_without_mutation() {
    refusal_preserves_rows_events_and_required_checks("actor-policy")
}
#[test]
fn root_gate_missing_task_has_specific_error_without_mutation() {
    refusal_preserves_rows_events_and_required_checks("missing-task")
}
#[test]
fn root_gate_rejects_stale_artifact_without_mutation() {
    refusal_preserves_rows_events_and_required_checks("stale-artifact")
}
