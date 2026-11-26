#![cfg(target_os = "linux")]

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};
use swarm_engine::{RootStore, SystemEnvironment};

/// Invoke the public CLI against an isolated initialized Root. Returns a
/// document for success and failure so tests can assert the actual cause.
fn call(root: &Path, verb: &str, request: Option<&Value>) -> (bool, Value) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_swarm-rust"));
    command.args(["role-model", verb, "work", "--root"]).arg(root).arg("--json");
    if let Some(request) = request {
        let input = root.join("reviewer-model-request.json");
        fs::write(&input, serde_json::to_vec(request).unwrap()).unwrap();
        command.arg("--input").arg(input);
    }
    let result = command.output().unwrap();
    let output = if result.status.success() { result.stdout } else { result.stderr };
    let doc = serde_json::from_slice(&output)
        .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&output)));
    (result.status.success(), doc)
}

/// The proposal is pure, requires a matching provider snapshot and publishes
/// exactly one event. Reopening and replay preserve prior evidence and spend.
#[test]
fn reviewer_route_preview_apply_and_replay_preserve_spend() {
    let root = tempfile::tempdir_in(swarm_engine::fixture_area()).unwrap();
    let executable = root.path().join("claude");
    fs::write(&executable, "#!/bin/sh\nexit 90\n").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let policy = json!({"version":1,"billing":"external","work_level":"standard","page_level":"simple",
        "levels":{"simple":{"model":"haiku","effort":""},"standard":{"model":"sonnet","effort":""},
                  "exigeant":{"model":"opus","effort":""}}});
    let policy_typed: swarm_engine::ModelPolicy = serde_json::from_value(policy.clone()).unwrap();
    let hash = format!("{:x}", Sha256::digest(swarm_engine::encode_model(&policy_typed).unwrap()));
    let providers: swarm_engine::Providers = serde_json::from_value(json!({"schema_version":1,
        "providers":{"claude":{"command":executable,"args":["--model","sonnet"],"model_policy":policy}}})).unwrap();
    let store = RootStore::initialize(root.path(), &mut SystemEnvironment).unwrap();
    store.initialize_provider_configuration(&providers).unwrap();
    store.close().unwrap();
    let db = rusqlite::Connection::open(root.path().join(".swarm/state.db")).unwrap();
    swarm_engine::register_identity_plan_projection(&db).unwrap();
    let body = json!({"schema_version":1,"id":"work","revision":1,"criteria":["Proof"],"tasks":[],
        "planning":{"version":1,"paused":true,"max_tasks":10,"max_decisions":10,"max_activations":15,
        "reviewer":{"provider":"codex","provider_digest":"a".repeat(64),"max_calls":6,"calls":6,
            "authorized":"2026-01-01T00:00:00Z","timeout_seconds":120},
        "scopes":[{"id":"root","objective":"Root","requirements":["req-1"],"revision":1,"generation":2,"state":"waiting"}]}});
    db.execute("INSERT INTO works(id,revision,body) VALUES('work',1,?1)", [body.to_string()]).unwrap();
    db.close().unwrap();

    let (ok, before) = call(root.path(), "show", None);
    assert!(ok, "{before}");
    assert_eq!(before["planning"]["reviewer"]["provider"], "codex");
    let mut change = json!({"schema_version":1,"event_id":"model-change","expected_revision":1,
        "reviewer":true,"provider":"claude","level":"standard","model_policy_hash":hash});
    let (ok, preview) = call(root.path(), "preview", Some(&change));
    assert!(ok, "{preview}");
    assert_eq!(preview["revision"], 1);
    let reviewer = &preview["planning"]["reviewer"];
    assert_eq!(reviewer["provider"], "claude");
    assert_eq!(reviewer["model_route"]["model"], "sonnet");
    assert_eq!(reviewer["calls"], 6);
    assert_eq!(reviewer["max_calls"], 6);
    assert_eq!(reviewer["timeout_seconds"], 120);
    assert_eq!(call(root.path(), "show", None), (true, before.clone()));
    let (ok, error) = call(root.path(), "apply", Some(&change));
    assert!(!ok);
    assert_eq!(error["failure"]["code"], "reviewer-model-preview-required");

    change["provider_digest"] = json!("b".repeat(64));
    let (ok, error) = call(root.path(), "apply", Some(&change));
    assert!(!ok);
    assert_eq!(error["failure"]["code"], "reviewer-model-provider-changed");
    assert_eq!(call(root.path(), "show", None), (true, before));

    change["provider_digest"] = reviewer["provider_digest"].clone();
    let (ok, applied) = call(root.path(), "apply", Some(&change));
    assert!(ok, "{applied}");
    assert_eq!(applied["revision"], 2);
    assert_eq!(applied["planning"]["reviewer"]["calls"], 6);
    assert_eq!(call(root.path(), "apply", Some(&change)), (true, applied.clone()));
    assert_eq!(call(root.path(), "show", None), (true, applied));
}
