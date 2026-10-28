//! Native current-publication reads on a genuinely born, inert cumulative fixture.
#![cfg(target_os = "linux")]
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};
use swarm_engine::{RootStore, RootStoreError, SystemEnvironment};
/// Find the retained birth record without traversing symlinks or adopting SQL.
/// This is a fixture ownership guard, not product authority or confinement.
fn born(dir: &Path, root: &str) -> bool {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let kind = entry.file_type().unwrap();
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() && born(&entry.path(), root) {
            return true;
        }
        if kind.is_file()
            && entry
                .file_name()
                .to_string_lossy()
                .starts_with("owned-root-")
            && entry.path().extension().is_some_and(|x| x == "json")
        {
            let value: Value = serde_json::from_slice(&fs::read(entry.path()).unwrap()).unwrap();
            if value["root"] == root {
                return true;
            }
        }
    }
    false
}
/// No managed repository or publication is inferred from an ordinary Work.
#[test]
fn current_managed_git_publication_refuses_nonmanaged_work() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let mut root = RootStore::initialize(dir.path(), &mut SystemEnvironment).unwrap();
    let work=root.execute_external("","work.create",br#"{"schema_version":1,"event_id":"create","expected_revision":0,"title":"No Git","objective":"No inferred candidate","scope":"Fixture","criteria":["No fabricated proof"]}"#,&mut SystemEnvironment).unwrap().result.work;
    assert!(matches!(
        root.current_managed_git_publication(&work.id),
        Err(RootStoreError::Refused(
            "managed-current-publication-repository-required"
        ))
    ));
    assert!(root.current_managed_git_publication("absent").is_err());
    root.close().unwrap();
}
/// This ignored SDK test requires a freshly retained real fixture. It never
/// generates accepted SQL/reviews: all positive states come from the public CLI.
/// Mutation faults restore exact original bytes BEFORE asserting refusal.
#[test]
#[ignore = "requires explicit newly owned inert cumulative publication fixture"]
fn current_managed_git_publication_latest_originals_and_recovery() {
    let proof = PathBuf::from(
        std::env::var_os("SWARM_PAEA_CURRENT_PUBLICATION_FIXTURE")
            .expect("fresh owned fixture proof"),
    );
    let result: Value =
        serde_json::from_slice(&fs::read(proof.join("result.json")).unwrap()).unwrap();
    assert_eq!(result["status"], "PASS");
    assert_eq!(result["paid_provider"], false);
    assert_eq!(result["cumulative_publication"], true);
    let path = PathBuf::from(result["root"].as_str().unwrap());
    let work_id = result["work"].as_str().unwrap();
    assert!(born(&proof, path.to_str().unwrap()));
    let root = RootStore::open(&path, false).unwrap();
    let before = root.read_work(work_id).unwrap();
    let repo = before
        .planning
        .as_ref()
        .unwrap()
        .repository
        .as_ref()
        .unwrap();
    let first = before
        .tasks
        .iter()
        .flatten()
        .find(|t| t.id == "task")
        .unwrap();
    let second = before
        .tasks
        .iter()
        .flatten()
        .find(|t| t.id == "task-2")
        .unwrap();
    assert_eq!(first.status, "accepted");
    assert_eq!(second.status, "accepted");
    assert_ne!(
        first.independent_review.as_ref().unwrap().candidate_commit,
        second.independent_review.as_ref().unwrap().candidate_commit
    );
    let valid = || root.current_managed_git_publication(work_id).unwrap();
    let original = valid();
    assert_eq!(original.candidate_commit, repo.candidate_commit);
    assert_eq!(original.candidate_commit, result["candidate_commit"]);
    assert_eq!(original.publication_event, result["publication_event"]);
    assert_eq!(original.task_id, "task-2");
    assert_eq!(original.work_revision, before.revision);
    assert_eq!(original.repository_request_digest, repo.request_digest);
    assert_eq!(
        original.review_id,
        second.independent_review.as_ref().unwrap().id
    );
    let encoded = serde_json::to_string(&original).unwrap();
    assert!(!encoded.contains(path.to_str().unwrap()));
    println!("CURRENT_MANAGED_GIT latest-cumulative-not-old-task-and-no-private-path PASS");
    // Read-only observer must not launch the actual inert provider again.
    let marker = path.join("managed-reviewer-invoked.json");
    let invocation = fs::read(&marker).unwrap();
    let db = rusqlite::Connection::open(path.join(".swarm/state.db")).unwrap();
    let event_count = || {
        db.query_row("SELECT count(*) FROM events", [], |r| r.get::<_, i64>(0))
            .unwrap()
    };
    let events = event_count();
    // Typed original/canonical errors must refuse, not yield empty observations.
    let (request, payload): (Vec<u8>, Vec<u8>) = db
        .query_row(
            "SELECT request,payload FROM events WHERE id=?",
            [&original.publication_event],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(valid(), original);
    let mut changed = request.clone();
    changed.push(b' ');
    db.execute(
        "UPDATE events SET request=?,payload=? WHERE id=?",
        rusqlite::params![changed, changed, original.publication_event],
    )
    .unwrap();
    let denied = root.current_managed_git_publication(work_id);
    db.execute(
        "UPDATE events SET request=?,payload=? WHERE id=?",
        rusqlite::params![request, payload, original.publication_event],
    )
    .unwrap();
    assert!(denied.is_err());
    assert_eq!(valid(), original);
    println!("CURRENT_MANAGED_GIT noncanonical-publication-refuses-and-restores PASS");
    for (label, task) in [
        ("older-cohort-original", first),
        ("latest-original", second),
    ] {
        let review = task.independent_review.as_ref().unwrap();
        let file = path.join(&review.receipt);
        let raw = fs::read(&file).unwrap();
        assert_eq!(valid(), original);
        let mut bad = raw.clone();
        bad.push(b' ');
        let mutation = replace(&file, &bad);
        let denied = mutation
            .as_ref()
            .ok()
            .map(|_| root.current_managed_git_publication(work_id));
        let restored = replace(&file, &raw);
        assert!(
            mutation.is_ok() && restored.is_ok(),
            "mutation={mutation:?}; restoration={restored:?}"
        );
        let denied = denied.expect("mutation succeeded");
        assert!(denied.is_err(), "{label}");
        assert_eq!(valid(), original);
        println!("CURRENT_MANAGED_GIT {label}-refuses-and-restores PASS");
    }
    // Remove only this fixture's immutable candidate object, retaining its bytes.
    let storage = PathBuf::from(&repo.storage);
    let storage = if storage.is_absolute() {
        storage
    } else {
        path.join(storage)
    };
    let oid = &original.candidate_commit;
    let object = storage
        .join("repository.git/objects")
        .join(&oid[..2])
        .join(&oid[2..]);
    let held = object.with_extension("paea71-owned-held");
    assert!(object.is_file() && !held.exists());
    assert_eq!(valid(), original);
    fs::rename(&object, &held).unwrap();
    let denied = root.current_managed_git_publication(work_id);
    fs::rename(&held, &object).unwrap();
    assert!(denied.is_err());
    assert_eq!(valid(), original);
    println!("CURRENT_MANAGED_GIT actual-candidate-object-missing-refuses-and-restores PASS");
    assert_eq!(root.read_work(work_id).unwrap(), before);
    assert_eq!(event_count(), events);
    assert_eq!(fs::read(&marker).unwrap(), invocation);
    assert!(root.current_managed_git_publication("absent").is_err());
    assert_eq!(valid(), original);
    println!("CURRENT_MANAGED_GIT no-new-event-provider-or-work-and-missing-work-refuses PASS");
    db.close().unwrap();
    root.close().unwrap();
}

/// Stage fixture bytes beside the original, sync, then atomically replace.
/// Pre-publication write failures preserve the original; a later directory sync
/// failure may mean publication and must still trigger explicit restoration.
fn replace(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    replace_with(path, |file| file.write_all(bytes))
}
/// Fixture writer injection only; no product timeout or fault option is exposed.
/// Partial-write errors drop only the temporary file, never truncate the original.
fn replace_with(
    path: &Path,
    write: impl FnOnce(&mut fs::File) -> std::io::Result<()>,
) -> std::io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("fixture parent missing"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary
        .as_file()
        .set_permissions(fs::metadata(path)?.permissions())?;
    write(temporary.as_file_mut())?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    fs::File::open(parent)?.sync_all()
}
/// Valid replacement precedes an injected ENOSPC after partial temporary write.
/// Refusal preserves the original exactly; a fresh atomic write then recovers.
#[test]
fn current_managed_git_fixture_atomic_write_refuses_and_recovers() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("original");
    fs::write(&path, b"initial").unwrap();
    replace(&path, b"valid control").unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"valid control");
    let failure = replace_with(&path, |file| {
        file.write_all(b"partial")?;
        Err(std::io::Error::from_raw_os_error(28))
    })
    .unwrap_err();
    assert_eq!(failure.raw_os_error(), Some(28));
    assert_eq!(fs::read(&path).unwrap(), b"valid control");
    replace(&path, b"recovered").unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"recovered");
}
