#![cfg(target_os = "linux")]
//! Actual public binary and private Root, no provider or database edits.
use serde_json::{json, Value};
use std::{fs, path::Path, process::Command};
fn call(root: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .args(args)
        .args(["--root", root.to_str().unwrap(), "--json"])
        .output()
        .unwrap()
}
fn parse(out: std::process::Output) -> Value {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}
#[test]
fn public_preview_apply_replay_and_undo_are_native_and_persistent() {
    let root = tempfile::tempdir_in(swarm_engine::fixture_area()).unwrap();
    parse(call(root.path(), &["init"]));
    let file = root.path().join("request.json");
    fs::write(&file,serde_json::to_vec(&json!({"schema_version":1,"event_id":"create","expected_revision":0,"title":"CLI graph","objective":"Atomic batches","scope":"Private","criteria":["Verified"]})).unwrap()).unwrap();
    let created = parse(call(
        root.path(),
        &["work", "create", "--input", file.to_str().unwrap()],
    ));
    let id = created["work"]["id"].as_str().unwrap();
    let mut request = json!({"schema_version":1,"event_id":"batch","expected_revision":1,"changes":[{"before":null,"after":{"id":"a","title":"A","deliverable":"docs/a","criteria":["Proof"],"depends":[]}}]});
    fs::write(&file, serde_json::to_vec(&request).unwrap()).unwrap();
    let preview = parse(call(
        root.path(),
        &[
            "task",
            "graph-preview",
            id,
            "--input",
            file.to_str().unwrap(),
        ],
    ));
    assert_eq!(preview["revision"], 1);
    assert_eq!(
        parse(call(root.path(), &["work", "show", id]))["work"]["revision"],
        1
    );
    request["preview_digest"] = preview["preview_digest"].clone();
    fs::write(&file, serde_json::to_vec(&request).unwrap()).unwrap();
    let applied = parse(call(
        root.path(),
        &["task", "graph-apply", id, "--input", file.to_str().unwrap()],
    ));
    assert_eq!(applied["work"]["revision"], 2);
    assert_eq!(applied["applied_revision"], 2);
    assert_eq!(applied["event_id"], "batch");
    assert_eq!(applied["replayed"], false);
    assert_eq!(
        parse(call(
            root.path(),
            &["task", "graph-apply", id, "--input", file.to_str().unwrap()]
        ))["replayed"],
        true
    );
    let mut undo = json!({"schema_version":1,"event_id":"undo","expected_revision":2,"changes":[],"undo_of":"batch"});
    fs::write(&file, serde_json::to_vec(&undo).unwrap()).unwrap();
    let p = parse(call(
        root.path(),
        &[
            "task",
            "graph-preview",
            id,
            "--input",
            file.to_str().unwrap(),
        ],
    ));
    undo["preview_digest"] = p["preview_digest"].clone();
    fs::write(&file, serde_json::to_vec(&undo).unwrap()).unwrap();
    let applied = parse(call(
        root.path(),
        &["task", "graph-apply", id, "--input", file.to_str().unwrap()],
    ));
    assert_eq!(applied["work"]["revision"], 3);
    assert!(applied["work"]["tasks"].as_array().unwrap().is_empty());
    assert_eq!(
        parse(call(root.path(), &["work", "show", id]))["work"]["revision"],
        3
    );
}
