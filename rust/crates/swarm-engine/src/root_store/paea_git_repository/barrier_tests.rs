//! Actual Root writer/format SDK. No effect authority or positive SQL seeding.
use super::*;
use crate::SystemEnvironment;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};
/// Create a disposable native Root with private storage, not a user mission.
fn fresh() -> (tempfile::TempDir, RootStore) {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let root = RootStore::initialize(dir.path(), &mut SystemEnvironment).unwrap();
    (dir, root)
}
/// Explicit actual first-publication result and its retained birth are required.
/// This73 fixture is newly generated and may take read-only SQL transactions;
/// no accepted state, review or control receipt is inserted by the SDK.
fn fixture() -> (PathBuf, RootStore, String, String) {
    let proof = PathBuf::from(
        std::env::var_os("SWARM_PAEA_WRITER_FIXTURE").expect("explicit newly owned fixture73"),
    );
    let result: serde_json::Value =
        serde_json::from_slice(&fs::read(proof.join("result.json")).unwrap()).unwrap();
    assert_eq!(result["status"], "PASS");
    assert_eq!(result["task_accepted"], true);
    assert_eq!(result["paid_provider"], false);
    let path = PathBuf::from(result["root"].as_str().unwrap());
    assert!(born(&proof, path.to_str().unwrap()));
    let root = RootStore::open(&path, true).unwrap();
    let work = result["work"].as_str().unwrap().to_string();
    let current = root.current_managed_git_publication(&work).unwrap();
    (path, root, work, current.candidate_commit)
}
/// Recurse only real proof directories to match the actual fixture birth.
fn born(path: &Path, root: &str) -> bool {
    for entry in fs::read_dir(path).unwrap() {
        let e = entry.unwrap();
        let kind = e.file_type().unwrap();
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() && born(&e.path(), root) {
            return true;
        }
        if kind.is_file()
            && e.file_name().to_string_lossy().starts_with("owned-root-")
            && e.path().extension().is_some_and(|x| x == "json")
        {
            let v: serde_json::Value =
                serde_json::from_slice(&fs::read(e.path()).unwrap()).unwrap();
            if v["root"] == root {
                return true;
            }
        }
    }
    false
}
/// Ordinary Work demonstrates the gate rejects no/read transaction, while a real
/// IMMEDIATE reaches the underlying managed-repository refusal without nesting.
/// A separate fixture observer uses zero busy wait solely to observe the actual
/// lock refusal immediately; it is not the product's saved contention policy.
#[test]
fn writer_requires_actual_write_and_recovers() {
    let (dir, mut root) = fresh();
    let work=root.execute_external("","work.create",br#"{"schema_version":1,"event_id":"create","expected_revision":0,"title":"Writer boundary","objective":"No fake fence","scope":"Fixture","criteria":["Actual SQLite Write"]}"#,&mut SystemEnvironment).unwrap().result.work;
    assert!(matches!(
        root.current_managed_git_publication_for_effect(&work.id),
        Err(E::Refused("managed-current-publication-writer-required"))
    ));
    let tx =
        rusqlite::Transaction::new_unchecked(&root.db, rusqlite::TransactionBehavior::Deferred)
            .unwrap();
    let _: i64 = tx
        .query_row("SELECT count(*) FROM works", [], |r| r.get(0))
        .unwrap();
    assert!(matches!(
        root.require_native_git_writer(),
        Err(E::Refused("managed-current-publication-writer-required"))
    ));
    tx.rollback().unwrap();
    let tx =
        rusqlite::Transaction::new_unchecked(&root.db, rusqlite::TransactionBehavior::Immediate)
            .unwrap();
    root.require_native_git_writer().unwrap();
    assert!(matches!(
        root.current_managed_git_publication_for_effect(&work.id),
        Err(E::Refused(
            "managed-current-publication-repository-required"
        ))
    ));
    let peer = rusqlite::Connection::open(dir.path().join(".swarm/state.db")).unwrap();
    peer.busy_timeout(std::time::Duration::ZERO).unwrap();
    let error = peer.execute_batch("BEGIN IMMEDIATE").unwrap_err();
    assert_eq!(
        error.sqlite_error_code(),
        Some(rusqlite::ErrorCode::DatabaseBusy)
    );
    tx.rollback().unwrap();
    peer.execute_batch("BEGIN IMMEDIATE;ROLLBACK;").unwrap();
    peer.close().unwrap();
    assert!(root.db.is_autocommit());
    assert_eq!(root.read_work(&work.id).unwrap(), work);
    root.close().unwrap();
}
/// Fixed real Git fixture setup, no inherited global/system config or hooks.
fn bare(path: &Path, child: &str, format: &str) {
    let directory = path.join(".swarm").join(child);
    fs::create_dir(&directory).unwrap();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
    let mut command = Command::new("git");
    for (key, _) in std::env::vars_os() {
        if key.as_encoded_bytes().starts_with(b"GIT_") {
            command.env_remove(key);
        }
    }
    let r = command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "init.templateDir=",
            "init",
            "--bare",
        ])
        .arg(format!("--object-format={format}"))
        .arg(&directory)
        .output()
        .unwrap();
    assert!(r.status.success());
}
/// Valid data for actual immutable candidate and service-generated resource ID.
fn proposal(binding: &PaeaGitRepositoryBinding, oid: &str) -> Proposal {
    Proposal {
        schema_version: 1,
        request_id: "writer-fixture".into(),
        action: Action::Git {
            repository_id: binding.repository_id().into(),
            target_ref: "refs/heads/release".into(),
            expected_old_oid: "1".repeat(oid.len()),
            candidate_oid: oid.into(),
        },
    }
}
/// Invoke actual native candidate reader and retained binding under the SAME
/// writable connection/transaction; the ordinary observer must refuse nesting.
#[test]
#[ignore = "requires newly owned actual native fixture73"]
fn writer_native_candidate_and_binding_share_transaction() {
    let (_path, root, work, oid) = fixture();
    let before = root.read_work(&work).unwrap();
    let expected = root.current_managed_git_publication(&work).unwrap();
    let (dir, service) = fresh();
    let pin = "a".repeat(64);
    service
        .paea_initialize_git_intents("fixture-project", &pin)
        .unwrap();
    bare(dir.path(), "bare", "sha1");
    let binding = service
        .paea_provision_git_repository(&root, &work, "bare", &pin)
        .unwrap();
    let held = service
        .paea_open_git_repository(&root, &work, "bare", &pin)
        .unwrap();
    let p = proposal(&binding, &oid);
    assert!(matches!(
        held.inspect_for_effect(&service, &root, &p),
        Err(E::Refused("managed-current-publication-writer-required"))
    ));
    let tx =
        rusqlite::Transaction::new_unchecked(&root.db, rusqlite::TransactionBehavior::Immediate)
            .unwrap();
    assert_eq!(
        held.inspect_for_effect(&service, &root, &p).unwrap(),
        expected
    );
    assert!(held.inspect(&service, &root, &p).is_err());
    root.require_native_git_writer().unwrap();
    assert_eq!(
        held.inspect_for_effect(&service, &root, &p).unwrap(),
        expected
    );
    tx.rollback().unwrap();
    assert_eq!(held.inspect(&service, &root, &p).unwrap(), expected);
    assert_eq!(root.read_work(&work).unwrap(), before);
    held.close().unwrap();
    service.close().unwrap();
    root.close().unwrap();
}
/// Compatible real SHA1 storage passes; SHA256 private storage with an actual
/// SHA1 reviewed candidate refuses specifically before any intent or ref effect.
#[test]
#[ignore = "requires newly owned actual native fixture73"]
fn object_format_compatibility_refuses_before_intent() {
    let (_path, root, work, oid) = fixture();
    assert_eq!(oid.len(), 40);
    let (dir, service) = fresh();
    let pin = "a".repeat(64);
    service
        .paea_initialize_git_intents("fixture-project", &pin)
        .unwrap();
    bare(dir.path(), "compatible", "sha1");
    let b = service
        .paea_provision_git_repository(&root, &work, "compatible", &pin)
        .unwrap();
    let held = service
        .paea_open_git_repository(&root, &work, "compatible", &pin)
        .unwrap();
    let p = proposal(&b, &oid);
    assert_eq!(
        held.inspect_object_format(&service, &root, &p).unwrap(),
        PaeaGitObjectFormat::Sha1
    );
    bare(dir.path(), "incompatible", "sha256");
    let b2 = service
        .paea_provision_git_repository(&root, &work, "incompatible", &pin)
        .unwrap();
    let h2 = service
        .paea_open_git_repository(&root, &work, "incompatible", &pin)
        .unwrap();
    let p2 = proposal(&b2, &oid);
    assert!(matches!(
        h2.inspect_object_format(&service, &root, &p2),
        Err(E::Refused("paea-git-object-format-mismatch"))
    ));
    assert_eq!(
        held.inspect_object_format(&service, &root, &p).unwrap(),
        PaeaGitObjectFormat::Sha1
    );
    let count: i64 = service
        .db
        .query_row("SELECT count(*) FROM paea_git_intents", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
    h2.close().unwrap();
    held.close().unwrap();
    service.close().unwrap();
    root.close().unwrap();
}
