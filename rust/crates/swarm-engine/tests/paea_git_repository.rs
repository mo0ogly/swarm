//! Explicit private Git association on disposable service roots and retained
//! actual native project71, read-only. No grant, ref promotion or UID claim.
#![cfg(target_os = "linux")]
use serde_json::Value;
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    process::Command,
};
use swarm_engine::{RootStore, RootStoreError, SystemEnvironment};
use swarm_protection_contract::{Action, Proposal};
const PIN: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
/// Resolve/pin actual installed Git, shared fixture pattern from protocol69.
fn git_executable() -> &'static std::path::Path {
    use sha2::{Digest, Sha256};
    static TOOL: std::sync::OnceLock<(std::path::PathBuf, String)> = std::sync::OnceLock::new();
    let (path, hash) = TOOL.get_or_init(|| {
        let path = std::env::split_paths(&std::env::var_os("PATH").expect("fixture PATH required"))
            .map(|p| p.join("git"))
            .find(|p| p.is_file() && fs::metadata(p).unwrap().permissions().mode() & 0o111 != 0)
            .expect("real Git required")
            .canonicalize()
            .unwrap();
        let hash = format!("{:x}", Sha256::digest(fs::read(&path).unwrap()));
        let version = Command::new(&path).arg("--version").output().unwrap();
        assert!(version.status.success());
        println!(
            "GIT_REFERENCE_TOOL {} SHA256 {} {}",
            path.display(),
            hash,
            String::from_utf8(version.stdout).unwrap().trim()
        );
        (path, hash)
    });
    assert_eq!(
        &format!("{:x}", Sha256::digest(fs::read(path).unwrap())),
        hash,
        "fixture Git changed"
    );
    path
}

/// Check actual fixture birth without following symlinks; no production authority.
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

/// Prepare only an owned private Git child; no inherited Git config or hooks.
fn bare(root: &Path, child: &str) -> PathBuf {
    let path = root.join(".swarm").join(child);
    fs::create_dir(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    let mut git = Command::new(git_executable());
    for (key, _) in std::env::vars_os() {
        if key.as_encoded_bytes().starts_with(b"GIT_") {
            git.env_remove(key);
        }
    }
    let result = git
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .args(["-c", "init.templateDir=", "init", "--bare"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(result.status.success());
    path
}
/// Fresh native service storage; never open a user mission as a fixture.
fn service() -> (tempfile::TempDir, RootStore) {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let root = RootStore::initialize(dir.path(), &mut SystemEnvironment).unwrap();
    (dir, root)
}
/// Read only a genuinely born actual cumulative project. Actual source is retained
/// by71; tests never change its SQLite, original reviews or managed Git objects.
fn project() -> (PathBuf, RootStore, String, String) {
    let proof = PathBuf::from(
        std::env::var_os("SWARM_PAEA_CURRENT_PUBLICATION_FIXTURE")
            .expect("explicit actual native proof"),
    );
    let value: Value =
        serde_json::from_slice(&fs::read(proof.join("result.json")).unwrap()).unwrap();
    assert_eq!(value["status"], "PASS");
    assert_eq!(value["cumulative_publication"], true);
    assert_eq!(value["paid_provider"], false);
    let path = PathBuf::from(value["root"].as_str().unwrap());
    assert!(born(&proof, path.to_str().unwrap()));
    let root = RootStore::open(&path, false).unwrap();
    (
        path,
        root,
        value["work"].as_str().unwrap().into(),
        value["candidate_commit"].as_str().unwrap().into(),
    )
}
/// Valid data for the service-assigned ID and actual reviewed candidate; does not
/// assert target OLD/direct type, grant or transfer into the private bare repo.
fn proposal(id: &str, candidate: &str) -> Proposal {
    Proposal {
        schema_version: 1,
        request_id: "fixture-transport".into(),
        action: Action::Git {
            repository_id: id.into(),
            target_ref: "refs/heads/release".into(),
            expected_old_oid: "1".repeat(40),
            candidate_oid: candidate.into(),
        },
    }
}
/// Fast refusal for absent trusted journal provision; no first-call adoption.
#[test]
fn git_repository_no_journal_never_adopts() {
    let (_sd, service) = service();
    let (_pd, project) = self::service();
    bare(_sd.path(), "bare");
    let failure = service
        .paea_provision_git_repository(&project, "work", "bare", PIN)
        .unwrap_err();
    assert!(failure.known_binding().is_none());
    assert!(format!("{failure:?}").contains("paea-git-repository"));
    let labels = swarm_engine::native_error_links(&failure).labels;
    assert!(labels.iter().any(|s| s == "paea-git-binding-not-observed"));
    service.close().unwrap();
    project.close().unwrap();
}
/// Real declaration, exact source/candidate observation, NOREPLACE and reopen.
#[test]
#[ignore = "requires explicit retained inert native project proof"]
fn git_repository_provision_inspect_and_reopen() {
    let (path, project, work, candidate) = project();
    let db_before = fs::read(path.join(".swarm/state.db")).unwrap();
    let (dir, service) = service();
    service
        .paea_initialize_git_intents("fixture-project", PIN)
        .unwrap();
    bare(dir.path(), "bare");
    let binding = service
        .paea_provision_git_repository(&project, &work, "bare", PIN)
        .unwrap();
    assert_eq!(binding.work_id(), work);
    assert_eq!(binding.project_id(), "fixture-project");
    assert_eq!(binding.project_digest(), PIN);
    assert_eq!(binding.repository_id().len(), 64);
    let file = dir.path().join(".swarm").join(format!(
        "paea-git-repository-{}.json",
        binding.repository_id()
    ));
    let original = fs::read(&file).unwrap();
    let held = service
        .paea_open_git_repository(&project, &work, "bare", PIN)
        .unwrap();
    assert_eq!(held.binding(), &binding);
    let p = proposal(binding.repository_id(), &candidate);
    let observation = held.inspect(&service, &project, &p).unwrap();
    assert_eq!(observation.candidate_commit, candidate);
    assert_eq!(
        observation.repository_request_digest,
        binding.repository_request_digest()
    );
    held.close().unwrap();
    assert!(service
        .paea_provision_git_repository(&project, &work, "bare", PIN)
        .is_err());
    assert_eq!(fs::read(&file).unwrap(), original);
    service.close().unwrap();
    let service = RootStore::open(dir.path(), true).unwrap();
    let held = service
        .paea_open_git_repository(&project, &work, "bare", PIN)
        .unwrap();
    assert_eq!(held.binding(), &binding);
    held.inspect(&service, &project, &p).unwrap();
    held.close().unwrap();
    service.close().unwrap();
    project.close().unwrap();
    assert_eq!(fs::read(path.join(".swarm/state.db")).unwrap(), db_before);
}
/// Valid control, then wrong resource/pin/candidate/Root and absence all refuse.
#[test]
#[ignore = "requires explicit retained inert native project proof"]
fn git_repository_exact_resource_candidate_and_roots_refuse() {
    let (_path, project, work, candidate) = project();
    let (dir, service) = service();
    service
        .paea_initialize_git_intents("fixture-project", PIN)
        .unwrap();
    bare(dir.path(), "bare");
    let binding = service
        .paea_provision_git_repository(&project, &work, "bare", PIN)
        .unwrap();
    let held = service
        .paea_open_git_repository(&project, &work, "bare", PIN)
        .unwrap();
    let p = proposal(binding.repository_id(), &candidate);
    held.inspect(&service, &project, &p).unwrap();
    let mut malformed = p.clone();
    if let Action::Git { target_ref, .. } = &mut malformed.action {
        *target_ref = "refs/heads/release\ncommit".into();
    }
    let error = held.inspect(&service, &project, &malformed).unwrap_err();
    let mut pending: Vec<&(dyn std::error::Error + 'static)> = vec![&error];
    let mut proposal_phase = false;
    let mut sql_phase = false;
    while let Some(error) = pending.pop() {
        let links = swarm_engine::native_error_links(error);
        proposal_phase |= links.labels.iter().any(|s| s == "paea-git-proposal");
        sql_phase |= links.labels.iter().any(|s| s.starts_with("sql-phase:"));
        pending.extend(links.children.into_iter().map(|(_, e)| e));
    }
    assert!(proposal_phase && !sql_phase);
    held.inspect(&service, &project, &p).unwrap();
    let mut wrong = p.clone();
    if let Action::Git { repository_id, .. } = &mut wrong.action {
        *repository_id = "another-repository".into();
    }
    assert!(matches!(
        held.inspect(&service, &project, &wrong),
        Err(RootStoreError::Refused("paea-git-proposal-repository"))
    ));
    wrong = p.clone();
    if let Action::Git { candidate_oid, .. } = &mut wrong.action {
        *candidate_oid = "3".repeat(40);
    }
    assert!(matches!(
        held.inspect(&service, &project, &wrong),
        Err(RootStoreError::Refused("paea-git-current-candidate"))
    ));
    assert!(service
        .paea_open_git_repository(&project, &work, "bare", &"b".repeat(64))
        .is_err());
    bare(dir.path(), "undeclared");
    assert!(service
        .paea_open_git_repository(&project, &work, "undeclared", PIN)
        .is_err());
    assert!(service
        .paea_open_git_repository(&project, &work, "../bare", PIN)
        .is_err());
    let (_otherdir, other) = self::service();
    assert!(matches!(
        held.inspect(&other, &project, &p),
        Err(RootStoreError::Refused("paea-git-root-binding"))
    ));
    assert!(matches!(
        held.inspect(&service, &other, &p),
        Err(RootStoreError::Refused("paea-git-root-binding"))
    ));
    other.close().unwrap();
    held.inspect(&service, &project, &p).unwrap();
    held.close().unwrap();
    service.close().unwrap();
    project.close().unwrap();
}
/// Hold old inode through rename/replacement, refuse named-child rebinding and
/// symlinks, then restore before asserting actual recovery. No ref is changed.
#[test]
#[ignore = "requires explicit retained inert native project proof"]
fn git_repository_rebinding_and_symlink_refuse_then_recover() {
    let (_path, project, work, candidate) = project();
    let (dir, service) = service();
    service
        .paea_initialize_git_intents("fixture-project", PIN)
        .unwrap();
    let original = bare(dir.path(), "bare");
    let binding = service
        .paea_provision_git_repository(&project, &work, "bare", PIN)
        .unwrap();
    let held = service
        .paea_open_git_repository(&project, &work, "bare", PIN)
        .unwrap();
    let p = proposal(binding.repository_id(), &candidate);
    held.inspect(&service, &project, &p).unwrap();
    let inode = fs::metadata(&original).unwrap().ino();
    let saved = original.with_file_name("held-original");
    fs::rename(&original, &saved).unwrap();
    let replacement = bare(dir.path(), "bare");
    assert_ne!(fs::metadata(&replacement).unwrap().ino(), inode);
    let denied = held.inspect(&service, &project, &p);
    let unopened = service
        .paea_open_git_repository(&project, &work, "bare", PIN)
        .is_err();
    let spare = original.with_file_name("owned-replacement");
    fs::rename(&replacement, &spare).unwrap();
    fs::rename(&saved, &original).unwrap();
    assert!(denied.is_err() && unopened);
    assert_eq!(fs::metadata(&original).unwrap().ino(), inode);
    held.inspect(&service, &project, &p).unwrap();
    fs::rename(&original, &saved).unwrap();
    std::os::unix::fs::symlink(&saved, &original).unwrap();
    let denied = held.inspect(&service, &project, &p);
    fs::remove_file(&original).unwrap();
    fs::rename(&saved, &original).unwrap();
    assert!(denied.is_err());
    held.inspect(&service, &project, &p).unwrap();
    held.close().unwrap();
    service.close().unwrap();
    project.close().unwrap();
}
/// Extra canonical bytes and same-length content mutation both refuse, preserving
/// the declaration. Only this owned service fixture atomically restores bytes.
#[test]
#[ignore = "requires explicit retained inert native project proof"]
fn git_repository_declaration_corruption_refuses_then_recovers() {
    use std::io::Write;
    let (_path, project, work, candidate) = project();
    let (dir, service) = service();
    service
        .paea_initialize_git_intents("fixture-project", PIN)
        .unwrap();
    bare(dir.path(), "bare");
    let binding = service
        .paea_provision_git_repository(&project, &work, "bare", PIN)
        .unwrap();
    let held = service
        .paea_open_git_repository(&project, &work, "bare", PIN)
        .unwrap();
    let p = proposal(binding.repository_id(), &candidate);
    held.inspect(&service, &project, &p).unwrap();
    let file = dir.path().join(".swarm").join(format!(
        "paea-git-repository-{}.json",
        binding.repository_id()
    ));
    let raw = fs::read(&file).unwrap();
    let publish = |bytes: &[u8]| {
        let mut temporary = tempfile::NamedTempFile::new_in(file.parent().unwrap()).unwrap();
        temporary.write_all(bytes).unwrap();
        temporary.as_file().sync_all().unwrap();
        temporary.persist(&file).unwrap();
        fs::File::open(file.parent().unwrap())
            .unwrap()
            .sync_all()
            .unwrap();
    };
    let mut bad = raw.clone();
    bad.push(b' ');
    publish(&bad);
    let denied = held.inspect(&service, &project, &p);
    publish(&raw);
    assert!(denied.is_err());
    held.inspect(&service, &project, &p).unwrap();
    let mut bad = raw.clone();
    let index = bad.iter().position(|b| *b == b'a').unwrap();
    bad[index] = b'b';
    publish(&bad);
    let denied = held.inspect(&service, &project, &p);
    publish(&raw);
    assert!(denied.is_err());
    held.inspect(&service, &project, &p).unwrap();
    assert_eq!(fs::read(&file).unwrap(), raw);
    held.close().unwrap();
    service.close().unwrap();
    project.close().unwrap();
}

/// Integration calls the normal (cfg(test)=false) library. Real SHA1/SHA256 bare
/// storage is compared to the genuine reviewed SHA1 candidate before any intent.
#[test]
#[ignore = "requires explicit retained inert native project proof"]
fn git_repository_normal_library_object_format_compatibility() {
    let (_path, project, work, candidate) = project();
    assert_eq!(candidate.len(), 40);
    let (dir, service) = service();
    service
        .paea_initialize_git_intents("fixture-project", PIN)
        .unwrap();
    bare(dir.path(), "compatible");
    let binding = service
        .paea_provision_git_repository(&project, &work, "compatible", PIN)
        .unwrap();
    let held = service
        .paea_open_git_repository(&project, &work, "compatible", PIN)
        .unwrap();
    let p = proposal(binding.repository_id(), &candidate);
    assert_eq!(
        held.inspect_object_format(&service, &project, &p).unwrap(),
        swarm_engine::PaeaGitObjectFormat::Sha1
    );
    let incompatible = dir.path().join(".swarm/incompatible");
    fs::create_dir(&incompatible).unwrap();
    fs::set_permissions(&incompatible, fs::Permissions::from_mode(0o700)).unwrap();
    let mut c = Command::new(git_executable());
    for (key, _) in std::env::vars_os() {
        if key.as_encoded_bytes().starts_with(b"GIT_") {
            c.env_remove(key);
        }
    }
    let out = c
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .args([
            "-c",
            "init.templateDir=",
            "init",
            "--bare",
            "--object-format=sha256",
        ])
        .arg(&incompatible)
        .output()
        .unwrap();
    assert!(out.status.success());
    let binding2 = service
        .paea_provision_git_repository(&project, &work, "incompatible", PIN)
        .unwrap();
    let h2 = service
        .paea_open_git_repository(&project, &work, "incompatible", PIN)
        .unwrap();
    let p2 = proposal(binding2.repository_id(), &candidate);
    assert!(matches!(
        h2.inspect_object_format(&service, &project, &p2),
        Err(RootStoreError::Refused("paea-git-object-format-mismatch"))
    ));
    assert_eq!(
        held.inspect_object_format(&service, &project, &p).unwrap(),
        swarm_engine::PaeaGitObjectFormat::Sha1
    );
    let db = rusqlite::Connection::open_with_flags(
        dir.path().join(".swarm/state.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let count: i64 = db
        .query_row("SELECT count(*) FROM paea_git_intents", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
    db.close().unwrap();
    h2.close().unwrap();
    held.close().unwrap();
    service.close().unwrap();
    project.close().unwrap();
}
