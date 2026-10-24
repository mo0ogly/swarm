#![cfg(unix)]
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
use swarm_engine::{
    GitObservation, RootStore, RuntimeError, SystemEnvironment, Work, WorkEnvironment,
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let p = std::env::temp_dir()
            .join(swarm_engine::generate_identifier("root-lifecycle-").unwrap());
        fs::create_dir(&p).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        Self(p)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap()
        }
    }
}
struct NoObservations;
impl WorkEnvironment for NoObservations {
    fn identifier(&mut self, _: &str) -> Result<String, RuntimeError> {
        panic!("unexpected identifier")
    }
    fn timestamp(&mut self) -> Result<String, RuntimeError> {
        panic!("unexpected clock")
    }
    fn operator(&mut self) -> Result<String, RuntimeError> {
        panic!("unexpected operator")
    }
    fn git(&mut self, _: &Path) -> Result<GitObservation, RuntimeError> {
        panic!("unexpected Git")
    }
}
fn request(event: &str, revision: i64, mut fields: serde_json::Value) -> Vec<u8> {
    fields["schema_version"] = 1.into();
    fields["event_id"] = event.into();
    fields["expected_revision"] = revision.into();
    serde_json::to_vec(&fields).unwrap()
}
fn apply(
    store: &mut RootStore,
    w: &Work,
    event: &str,
    kind: &str,
    fields: serde_json::Value,
) -> Work {
    store
        .execute_external(
            &w.id,
            kind,
            &request(event, w.revision, fields),
            &mut SystemEnvironment,
        )
        .unwrap()
        .result
        .work
}
/// Exercise actual Root manual transitions with a complete declared gate.
/// Persistence, stale-dependency refusal and attempt preservation are observed
/// on an isolated store; declared checks do not authenticate a reviewer.
#[test]
fn root_manual_lifecycle_persists_acceptance_and_blocks_stale_dependency_without_new_attempt() {
    let f = Fixture::new();
    let mut store = RootStore::initialize(&f.0, &mut SystemEnvironment).unwrap();
    fs::write(f.0.join("proof.txt"), b"proof").unwrap();
    let mut w=store.execute_external("","work.create",&request("create",0,serde_json::json!({"title":"Lifecycle","objective":"Actual Root transitions","scope":"Fixture","criteria":["Proof"]})),&mut SystemEnvironment).unwrap().result.work;
    w = apply(
        &mut store,
        &w,
        "first",
        "task.add",
        serde_json::json!({"id":"first","title":"First","deliverable":"Report","criteria":["Proof"]}),
    );
    w = apply(
        &mut store,
        &w,
        "second",
        "task.add",
        serde_json::json!({"id":"second","title":"Second","deliverable":"Report","criteria":["Proof"],"depends":["first"]}),
    );
    let before = w.clone();
    let count = store.events(&w.id).unwrap().len();
    assert!(store
        .execute_external(
            &w.id,
            "task.update",
            &request(
                "premature",
                w.revision,
                serde_json::json!({"id":"second","status":"running"})
            ),
            &mut NoObservations
        )
        .is_err());
    assert_eq!(store.read_work(&w.id).unwrap(), before);
    assert_eq!(store.events(&w.id).unwrap().len(), count);
    w = apply(
        &mut store,
        &w,
        "first-running",
        "task.update",
        serde_json::json!({"id":"first","status":"running"}),
    );
    assert_eq!(
        w.tasks.as_ref().unwrap()[0]
            .attempts
            .as_ref()
            .unwrap()
            .len(),
        1
    );
    assert!(w.tasks.as_ref().unwrap()[0].attempts.as_ref().unwrap()[0]
        .id
        .starts_with("a-"));
    w = apply(
        &mut store,
        &w,
        "first-submitted",
        "task.update",
        serde_json::json!({"id":"first","status":"submitted","outcome":"completed"}),
    );
    // Declared gate fixture follows the actual created task's required IDs;
    // these PASS observations are not executable control receipts or a provider run.
    let checks:Vec<_>=w.tasks.as_ref().unwrap()[0].plan_checks.as_ref().unwrap().iter().map(|(id,phase)|serde_json::json!({"id":id,"gate":phase,"domain":"quality","mandatory":true,"severity":"minor","penalty":1,"max_penalty":100})).collect();
    let results:Vec<_>=checks.iter().map(|c|serde_json::json!({"id":c["id"],"status":"PASS","count":0,"evidence":["proof.txt"]})).collect();
    let gate = serde_json::json!({"method_version":"3","scope_id":"first","artifacts":{"proof.txt":format!("{:x}",Sha256::digest(b"proof"))},"domains":{"quality":1},"checks":checks,"results":results});
    w = apply(
        &mut store,
        &w,
        "gate",
        "gate",
        serde_json::json!({"task_id":"first","phase":"delivery","name":"Current proof","document":gate}),
    );
    w = apply(
        &mut store,
        &w,
        "accepted",
        "task.update",
        serde_json::json!({"id":"first","status":"accepted"}),
    );
    assert_eq!(w.tasks.as_ref().unwrap()[0].status, "accepted");
    store.inspect_gate_evidence(&w.id, "first").unwrap();
    let before = w.clone();
    let count = store.events(&w.id).unwrap().len();
    fs::write(f.0.join("proof.txt"), b"changed").unwrap();
    assert!(store
        .execute_external(
            &w.id,
            "task.update",
            &request(
                "stale-dependent",
                w.revision,
                serde_json::json!({"id":"second","status":"running"})
            ),
            &mut NoObservations
        )
        .is_err());
    assert_eq!(store.read_work(&w.id).unwrap(), before);
    assert_eq!(store.events(&w.id).unwrap().len(), count);
    fs::write(f.0.join("proof.txt"), b"proof").unwrap();
    let raw = request(
        "second-running",
        w.revision,
        serde_json::json!({"id":"second","status":"running"}),
    );
    let result = store
        .execute_external(&w.id, "task.update", &raw, &mut SystemEnvironment)
        .unwrap();
    assert!(!result.result.replayed);
    w = result.result.work;
    let count = store.events(&w.id).unwrap().len();
    let replay = store
        .execute_external(&w.id, "task.update", &raw, &mut NoObservations)
        .unwrap();
    assert!(replay.result.replayed);
    assert_eq!(replay.result.work, w);
    assert_eq!(store.events(&w.id).unwrap().len(), count);
    store.close().unwrap();
    let store = RootStore::open(&f.0, false).unwrap();
    assert_eq!(store.read_work(&w.id).unwrap(), w);
    assert_eq!(
        w.tasks.as_ref().unwrap()[1]
            .attempts
            .as_ref()
            .unwrap()
            .len(),
        1
    );
    store.close().unwrap();
}
