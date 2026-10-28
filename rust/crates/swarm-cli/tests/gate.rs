#![cfg(target_os = "linux")]
use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Command};
use swarm_engine::{RootStore, SystemEnvironment};
/// Execute the real CLI only against a private initialized root.
fn call(root: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .args(args)
        .arg("--root")
        .arg(root)
        .output()
        .unwrap()
}
/// Use native transactions to establish a valid task before the gate contract.
fn seed(root: &Path) -> swarm_engine::Work {
    let mut store = RootStore::initialize(root, &mut SystemEnvironment).unwrap();
    let work=store.execute_external("","work.create",br#"{"schema_version":1,"event_id":"create","expected_revision":0,"title":"Gate fixture","objective":"Contract","scope":"Private","criteria":["Proof"]}"#,&mut SystemEnvironment).unwrap().result.work;
    let raw=serde_json::to_vec(&serde_json::json!({"schema_version":1,"event_id":"add","expected_revision":work.revision,"id":"task","title":"Task","deliverable":"Report","criteria":["Proof"]})).unwrap();
    let work = store
        .execute_external(&work.id, "task.add", &raw, &mut SystemEnvironment)
        .unwrap()
        .result
        .work;
    let raw=serde_json::to_vec(&serde_json::json!({"schema_version":1,"event_id":"criteria","expected_revision":work.revision,"id":"task","criteria":["Amended proof"]})).unwrap();
    let work = store
        .execute_external(&work.id, "task.update", &raw, &mut SystemEnvironment)
        .unwrap()
        .result
        .work;
    store.close().unwrap();
    work
}
/// Build all required checks from the actual task producer; never guess IDs.
fn document(task: &swarm_engine::Task) -> serde_json::Value {
    let mut checks:Vec<_>=task.plan_checks.as_ref().unwrap().iter().map(|(id,phase)|serde_json::json!({"id":id,"gate":phase,"mandatory":true,"severity":"minor","penalty":1,"max_penalty":100})).collect();
    checks.push(serde_json::json!({"id":"fixture-blocking","domain":"quality","gate":"delivery","mandatory":true,"severity":"minor","penalty":1,"max_penalty":100}));
    let results:Vec<_>=checks.iter().map(|c|serde_json::json!({"id":c["id"],"status":"PASS","count":0,"evidence":["proof.txt"]})).collect();
    serde_json::json!({"method_version":"3","scope_id":task.id,"artifacts":{"proof.txt":format!("{:x}",Sha256::digest(b"proof"))},"domains":{"quality":1},"checks":checks,"results":results})
}
/// Gate success persists and replays; a denied but valid evaluation persists with
/// exit1. Task mismatch/unknown fields refuse without mutation, then recover.
#[test]
fn public_gate_persists_allowed_and_denied_results_without_claiming_acceptance() {
    let root = tempfile::tempdir().unwrap();
    let work = seed(root.path());
    fs::write(root.path().join("proof.txt"), b"proof").unwrap();
    let doc = document(&work.tasks.as_ref().unwrap()[0]);
    let input = root.path().join("gate.json");
    let mut request = serde_json::json!({"schema_version":1,"event_id":"gate-pass","expected_revision":work.revision,"task_id":"task","phase":"delivery","name":"Delivery gate","document":doc});
    fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
    let args = [
        "gate",
        &work.id,
        "--task",
        "task",
        "--input",
        input.to_str().unwrap(),
        "--json",
    ];
    let valid = call(root.path(), &args);
    assert!(
        valid.status.success(),
        "{}",
        String::from_utf8_lossy(&valid.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&valid.stdout).unwrap();
    assert_eq!(value["work"]["revision"], work.revision + 1);
    assert_eq!(value["work"]["tasks"][0]["status"], "todo");
    assert_eq!(value["current_gates"]["task"]["valid"], true);
    assert_eq!(
        value["current_gates"]["task"]["evaluation"]["allowed"],
        true
    );
    let replay = call(root.path(), &args);
    assert!(replay.status.success());
    assert_eq!(replay.stdout, valid.stdout);
    let db = root.path().join(".swarm/state.db");
    let before = fs::read(&db).unwrap();
    let mismatch = call(
        root.path(),
        &[
            "gate",
            &work.id,
            "--task",
            "different",
            "--input",
            input.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(mismatch.status.code(), Some(2));
    assert!(mismatch.stdout.is_empty());
    assert_eq!(fs::read(&db).unwrap(), before);
    request["event_id"] = "gate-fail".into();
    request["expected_revision"] = (work.revision + 1).into();
    let result = request["document"]["results"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["id"] == "fixture-blocking")
        .unwrap();
    result["status"] = "FAIL".into();
    result["count"] = 1.into();
    fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
    let denied = call(root.path(), &args);
    assert_eq!(
        denied.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&denied.stderr)
    );
    assert!(denied.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&denied.stdout).unwrap();
    assert_eq!(value["work"]["revision"], work.revision + 2);
    assert_eq!(value["work"]["tasks"][0]["status"], "todo");
    assert_eq!(
        value["current_gates"]["task"]["evaluation"]["allowed"],
        false
    );
    assert_eq!(call(root.path(), &args).status.code(), Some(1));
    let after = fs::read(&db).unwrap();
    request["unexpected"] = true.into();
    fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
    let invalid = call(root.path(), &args);
    assert_eq!(invalid.status.code(), Some(2));
    assert!(invalid.stdout.is_empty());
    assert_eq!(fs::read(&db).unwrap(), after);
    request.as_object_mut().unwrap().remove("unexpected");
    fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
    assert_eq!(call(root.path(), &args).status.code(), Some(1));
}

/// Adopt the existing persisted quality setting through public preview/CAS.
/// No journal reset or mutation of consumption/history is used by this fixture.
fn quality(root: &Path, event: &str, threshold: i64) {
    let store = RootStore::open(root, true).unwrap();
    let doc = store.settings().unwrap();
    let mut values = doc.values;
    values.quality_delivery_min_percent = Some(swarm_engine::SettingCount(threshold));
    let mut change = swarm_engine::EngineSettingsChange {
        schema_version: values.schema_version(),
        event_id: event.into(),
        expected_revision: doc.revision,
        values,
        reason: "Actual CLI gate quality policy".into(),
        preview_token: String::new(),
    };
    change.preview_token = store.preview_settings(&change).unwrap().preview_token;
    store
        .apply_settings(&change, &mut SystemEnvironment)
        .unwrap();
    store.close().unwrap();
}
/// Raising the saved policy reaches the real gate CLI after reopen. A supplied
/// policy is refused and preserves the stored row; the saved policy stays binding.
#[test]
fn public_gate_uses_saved_quality_and_refuses_caller_policy() {
    let root = tempfile::tempdir().unwrap();
    let work = seed(root.path());
    fs::write(root.path().join("proof.txt"), b"proof").unwrap();
    let mut doc = document(&work.tasks.as_ref().unwrap()[0]);
    doc["checks"].as_array_mut().unwrap().push(serde_json::json!({"id":"optional-quality","domain":"quality","gate":"delivery","mandatory":false,"severity":"minor","penalty":20,"max_penalty":100}));
    doc["results"].as_array_mut().unwrap().push(serde_json::json!({"id":"optional-quality","status":"FAIL","count":1,"evidence":["proof.txt"]}));
    quality(root.path(), "lower-quality", 70);
    let input = root.path().join("gate.json");
    let mut request = serde_json::json!({"schema_version":1,"event_id":"gate-policy-pass","expected_revision":work.revision,"task_id":"task","phase":"delivery","name":"Saved quality","document":doc});
    fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
    let args = [
        "gate",
        &work.id,
        "--input",
        input.to_str().unwrap(),
        "--json",
    ];
    let pass = call(root.path(), &args);
    assert!(
        pass.status.success(),
        "{}",
        String::from_utf8_lossy(&pass.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&pass.stdout).unwrap();
    assert_eq!(
        value["current_gates"]["task"]["evaluation"]["quality"],
        80.0
    );
    quality(root.path(), "raise-quality", 90);
    request["event_id"] = "gate-policy-block".into();
    request["expected_revision"] = (work.revision + 1).into();
    request["policy"] = serde_json::json!({"quality_delivery_min_percent":0});
    fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
    let db = root.path().join(".swarm/state.db");
    let before = fs::read(&db).unwrap();
    let refused = call(root.path(), &args);
    assert_eq!(refused.status.code(), Some(2));
    assert!(refused.stdout.is_empty());
    assert_eq!(fs::read(&db).unwrap(), before);
    request.as_object_mut().unwrap().remove("policy");
    fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
    let stale = call(root.path(), &args);
    assert_eq!(stale.status.code(), Some(2));
    assert_eq!(fs::read(&db).unwrap(), before);
    let current = call(root.path(), &["work", "show", &work.id, "--json"]);
    assert!(current.status.success());
    let current: serde_json::Value = serde_json::from_slice(&current.stdout).unwrap();
    assert_eq!(
        current["current_gates"]["task"]["evaluation"]["allowed"],
        false
    );
    let reorient = serde_json::json!({"schema_version":1,"event_id":"reorient-policy","expected_revision":work.revision+1,"id":"task","status":"running"});
    fs::write(&input, serde_json::to_vec(&reorient).unwrap()).unwrap();
    let reopened = call(
        root.path(),
        &[
            "task",
            "update",
            &work.id,
            "--input",
            input.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(
        reopened.status.success(),
        "{}",
        String::from_utf8_lossy(&reopened.stderr)
    );
    request["expected_revision"] = (work.revision + 2).into();
    fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
    let denied = call(root.path(), &args);
    assert_eq!(
        denied.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&denied.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&denied.stdout).unwrap();
    assert_eq!(value["work"]["revision"], work.revision + 3);
    assert_eq!(
        value["current_gates"]["task"]["evaluation"]["allowed"],
        false
    );
}
