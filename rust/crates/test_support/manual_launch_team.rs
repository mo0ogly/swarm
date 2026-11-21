//! Public setup for configured manual admission in owned integration fixtures.
use std::{fs, os::unix::fs::PermissionsExt, path::Path};
use swarm_engine::{RootStore, SystemEnvironment};

/// Give an existing fixture task explicit authority, with an inert planner
/// and reviewer independent of the worker catalogue entry under test.
/// Returns the attached work revision. No provider is executed.
pub fn attach(root: &Path, store: &mut RootStore, work: &str) -> i64 {
    let planner = root.join("skynet_harness");
    fs::write(&planner, b"#!/bin/cat\nfixture planner is never executed\n").unwrap();
    fs::set_permissions(&planner, fs::Permissions::from_mode(0o700)).unwrap();
    let path = root.join(".swarm/providers.json");
    let mut catalogue: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let choice = serde_json::json!({"model":"fixture-planner","effort":""});
    catalogue["providers"]["team-fixture"] = serde_json::json!({
        "command":planner, "args":["--model","fixture-planner"],
        "model_policy":{"version":1,"billing":"external","work_level":"standard",
            "page_level":"simple","levels":{"simple":choice,"standard":choice,"exigeant":choice}}
    });
    fs::write(path, serde_json::to_vec(&catalogue).unwrap()).unwrap();
    let revision = store.read_work(work).unwrap().revision;
    let authorize = serde_json::json!({
        "schema_version":1,"event_id":"authorize-test-team","expected_revision":revision,
        "id":"task","validation_policy":{"mode":"human"}
    });
    let revision = store.execute_external(work,"task.update",&serde_json::to_vec(&authorize).unwrap(),&mut SystemEnvironment)
        .unwrap().result.work.revision;
    let attach = serde_json::json!({
        "schema_version":1,"event_id":"attach-test-team","expected_revision":revision,
        "provider":"team-fixture","level":"standard","max_tasks":4,"max_activations":2,
        "max_decisions":2,"max_review_calls":2,"task_scopes":{"task":"proof"},
        "checks":{"req-1":[{"id":"fixture-team-executable","inputs":["skynet_harness"],
            "command":["python3","-c","import os; assert os.access('skynet_harness', os.X_OK)"],
            "justification":"Check executable setup of the owned fixture planner"}]}
    });
    store.execute_external(work,"planning.attach",&serde_json::to_vec(&attach).unwrap(),&mut SystemEnvironment)
        .unwrap().result.work.revision
}

/// Change the worker catalogue fixture while preserving the separately frozen
/// planner and reviewer definition. Returns the underlying write failure.
pub fn replace_worker_catalogue(root: &Path, worker: impl AsRef<[u8]>) -> std::io::Result<()> {
    let path = root.join(".swarm/providers.json");
    let saved: serde_json::Value = serde_json::from_slice(&fs::read(&path)?)?;
    let mut candidate: serde_json::Value = serde_json::from_slice(worker.as_ref())?;
    candidate["providers"]["team-fixture"] = saved["providers"]["team-fixture"].clone();
    fs::write(path, serde_json::to_vec(&candidate)?)
}
