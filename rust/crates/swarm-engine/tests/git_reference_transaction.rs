//! Native owned-Git protocol fixtures. No PAEA authority or live repository used.
#![cfg(target_os = "linux")]
use std::{
    error::Error,
    fs,
    io::{self, Write},
    os::{
        fd::AsFd,
        unix::{fs::PermissionsExt, net::UnixStream},
    },
    path::Path,
    process::{Command, Stdio},
    time::Instant,
};
use swarm_engine::{
    run_git_reference_transaction as run, GitReferenceDecision as Decision, GitReferenceError,
    GitReferenceObservation as Observation, RootStoreError,
};
use swarm_protection_contract::{Action, Proposal};
type Failure = Box<dyn Error + Send + Sync>;
struct Fixture {
    dir: tempfile::TempDir,
    old: String,
    new: String,
    other: String,
}
impl Fixture {
    /// Build a private bare repository and three real synthetic commit objects.
    /// No project commit, identity, provider or mission database is created.
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();
        output(path, &["init", "--bare", "."]);
        let input = path.join("fixture-input");
        fs::write(&input, []).unwrap();
        let tree = object(path, "tree", &input);
        let old = commit(path, &tree, None, "old");
        let new = commit(path, &tree, Some(&old), "new");
        let other = commit(path, &tree, Some(&old), "other");
        output(path, &["update-ref", "refs/heads/target", &old]);
        Self {
            dir,
            old,
            new,
            other,
        }
    }
    /// Exact v1 representation only, never an admitted PAEA request.
    fn proposal(&self) -> Proposal {
        Proposal {
            schema_version: 1,
            request_id: "fixture-transport".into(),
            action: Action::Git {
                repository_id: "fixture-repository".into(),
                target_ref: "refs/heads/target".into(),
                expected_old_oid: self.old.clone(),
                candidate_oid: self.new.clone(),
            },
        }
    }
    /// Observe exact resolved fixture ref; failures never count as absence.
    fn value(&self, reference: &str) -> String {
        output(self.dir.path(), &["rev-parse", "--verify", reference])
    }
    /// Trusted decision checks direct type AND exact current OID under prepare.
    fn direct(&self) -> Result<Decision, Failure> {
        let p = command(self.dir.path())
            .args([
                "symbolic-ref",
                "--quiet",
                "--no-recurse",
                "refs/heads/target",
            ])
            .output()?;
        if p.status.code() != Some(1) {
            return Ok(Decision::Abort);
        }
        if self.value("refs/heads/target") != self.old {
            return Err(io::Error::other("fixture-old-changed").into());
        }
        Ok(Decision::Commit)
    }
    /// Valid owned native update and exact reset before injecting any fault.
    fn control(&self) {
        let done = run(
            command(self.dir.path()),
            &self.proposal(),
            None,
            None,
            || self.direct(),
        )
        .unwrap();
        assert_eq!(done.observation, Observation::CommitAcknowledged);
        assert!(done.exit_status.success());
        assert_eq!(self.value("refs/heads/target"), self.new);
        output(
            self.dir.path(),
            &["update-ref", "refs/heads/target", &self.old, &self.new],
        );
    }
}
/// Resolve the actual fixture Git once, retain its version/hash and refuse any
/// changed executable bytes before subsequent commands. No runtime path policy.
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

/// Preconfigure real Git with no inherited Git config/environment or hooks.
/// This fixture constructor is not a product execution-policy source.
fn command(path: &Path) -> Command {
    let mut c = Command::new(git_executable());
    c.current_dir(path).args([
        "--no-pager",
        "--no-optional-locks",
        "-c",
        "core.hooksPath=/dev/null",
        "-c",
        "core.fsmonitor=false",
        "-c",
        "core.fsync=all",
    ]);
    for (k, _) in std::env::vars_os() {
        if k.as_encoded_bytes().starts_with(b"GIT_") {
            c.env_remove(k);
        }
    }
    c.env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_TERMINAL_PROMPT", "0");
    c
}
/// Run fixed synchronous fixture setup/observation, checking actual status.
fn output(path: &Path, args: &[&str]) -> String {
    let p = command(path).args(args).output().unwrap();
    assert!(
        p.status.success(),
        "fixture Git setup/observation failed: {:?}",
        p.status
    );
    String::from_utf8(p.stdout).unwrap().trim().into()
}
/// Create one immutable Git object from an owned temporary input file.
fn object(path: &Path, kind: &str, input: &Path) -> String {
    let p = command(path)
        .args(["hash-object", "-t", kind, "-w", "--stdin"])
        .stdin(Stdio::from(fs::File::open(input).unwrap()))
        .output()
        .unwrap();
    assert!(p.status.success());
    String::from_utf8(p.stdout).unwrap().trim().into()
}
/// Explicitly fictional author/time object bytes; no human attribution claimed.
fn commit(path: &Path, tree: &str, parent: Option<&str>, message: &str) -> String {
    let body=format!("tree {tree}\n{}author Fixture <fixture@invalid.example> 1 +0000\ncommitter Fixture <fixture@invalid.example> 1 +0000\n\n{message}\n",parent.map(|p|format!("parent {p}\n")).unwrap_or_default());
    let input = path.join("fixture-input");
    fs::write(&input, body).unwrap();
    object(path, "commit", &input)
}

/// Real direct CAS, explicit abort, stale OLD refusal and recovery.
#[test]
fn git_reference_direct_abort_stale_old_and_recovery() {
    let f = Fixture::new();
    f.control();
    let done = run(command(f.dir.path()), &f.proposal(), None, None, || {
        Ok(Decision::Abort)
    })
    .unwrap();
    assert_eq!(done.observation, Observation::AbortAcknowledged);
    assert_eq!(f.value("refs/heads/target"), f.old);
    let mut bad = f.proposal();
    if let Action::Git {
        expected_old_oid, ..
    } = &mut bad.action
    {
        *expected_old_oid = f.other.clone()
    }
    let mut error = run(command(f.dir.path()), &bad, None, None, || {
        panic!("stale OLD reached decision")
    })
    .unwrap_err();
    assert_eq!(error.observation, Observation::NotRequested);
    assert!(!error.exit_status.unwrap().success());
    assert!(error.process_error_mut().is_none());
    assert_eq!(f.value("refs/heads/target"), f.old);
    f.control();
}
/// Refuse symbolic refs under actual Git lock; both original refs remain intact.
#[test]
fn git_reference_symbolic_under_prepare_refuses_then_recovers() {
    let f = Fixture::new();
    f.control();
    output(f.dir.path(), &["update-ref", "refs/heads/indirect", &f.old]);
    output(
        f.dir.path(),
        &["symbolic-ref", "refs/heads/target", "refs/heads/indirect"],
    );
    let done = run(command(f.dir.path()), &f.proposal(), None, None, || {
        f.direct()
    })
    .unwrap();
    assert_eq!(done.observation, Observation::AbortAcknowledged);
    assert_eq!(
        output(f.dir.path(), &["symbolic-ref", "refs/heads/target"]),
        "refs/heads/indirect"
    );
    assert_eq!(f.value("refs/heads/indirect"), f.old);
    output(
        f.dir.path(),
        &[
            "update-ref",
            "--no-deref",
            "refs/heads/target",
            &f.old,
            &f.old,
        ],
    );
    f.control();
}
/// Both competing Git writer kinds fail while the owned transaction is prepared.
#[test]
fn git_reference_prepared_lock_excludes_competing_writers() {
    let f = Fixture::new();
    f.control();
    let done = run(command(f.dir.path()), &f.proposal(), None, None, || {
        assert!(f.dir.path().join("refs/heads/target.lock").is_file());
        for args in [
            vec![
                "update-ref",
                "refs/heads/target",
                f.other.as_str(),
                f.old.as_str(),
            ],
            vec!["symbolic-ref", "refs/heads/target", "refs/heads/elsewhere"],
        ] {
            assert!(!command(f.dir.path())
                .args(args)
                .output()
                .unwrap()
                .status
                .success());
        }
        f.direct()
    })
    .unwrap();
    assert_eq!(done.observation, Observation::CommitAcknowledged);
    assert_eq!(f.value("refs/heads/target"), f.new);
    assert!(!f.dir.path().join("refs/heads/target.lock").exists());
}
/// Preserve this actual typed error through all eight existing wrapper shapes.
fn wrappers(error: GitReferenceError, expected: Observation) {
    fn check(e: &(dyn Error + 'static), expected: Observation) {
        let typed = GitReferenceError::find(e).expect("typed Git observation survived");
        assert_eq!(typed.observation, expected);
        assert!(!format!("{typed:?}").contains("PRIVATE_FIXTURE_DIAGNOSTIC"));
    }
    check(&error, expected);
    let boxed = Box::new(error);
    check(&boxed, expected);
    let io = io::Error::other(boxed);
    check(&io, expected);
    let root = RootStoreError::SqlTransaction(Box::new(io));
    check(&root, expected);
    let boxed = Box::new(root);
    check(&boxed, expected);
    let root = RootStoreError::Cleanup {
        primary: Some(Box::new(RootStoreError::Refused("fixture-primary"))),
        cleanup: boxed,
    };
    check(&root, expected);
    let io = io::Error::other(Box::new(root));
    check(&io, expected);
    let root = RootStoreError::SqlTransaction(Box::new(io));
    check(&root, expected);
}
/// Decision failures explicitly abort, retain cause and recover without stale lock.
#[test]
fn git_reference_decision_failure_retains_abort_cause_and_wrappers() {
    let f = Fixture::new();
    f.control();
    let e = run(command(f.dir.path()), &f.proposal(), None, None, || {
        Err(io::Error::other("PRIVATE_FIXTURE_DIAGNOSTIC").into())
    })
    .unwrap_err();
    assert_eq!(e.code(), "git-reference-decision");
    assert_eq!(e.observation, Observation::AbortAcknowledged);
    assert!(e.source().unwrap().is::<io::Error>());
    assert!(e.exit_status.unwrap().success());
    assert_eq!(f.value("refs/heads/target"), f.old);
    assert!(!f.dir.path().join("refs/heads/target.lock").exists());
    wrappers(e, Observation::AbortAcknowledged);
    f.control();
}
/// Expired supplied deadline and readable cancellation refuse before spawn.
#[test]
fn git_reference_preflight_deadline_cancel_and_invalid_domain_recover() {
    let f = Fixture::new();
    f.control();
    let e = run(
        command(f.dir.path()),
        &f.proposal(),
        Some(Instant::now()),
        None,
        || panic!("expired deadline reached decision"),
    )
    .unwrap_err();
    assert_eq!(e.code(), "git-reference-preflight");
    assert_eq!(e.observation, Observation::NotRequested);
    assert!(e.exit_status.is_none());
    let (cancel, mut signal) = UnixStream::pair().unwrap();
    signal.write_all(b"cancel").unwrap();
    let e = run(
        command(f.dir.path()),
        &f.proposal(),
        None,
        Some(cancel.as_fd()),
        || panic!("cancelled reached decision"),
    )
    .unwrap_err();
    assert_eq!(e.code(), "git-reference-preflight");
    assert!(e.exit_status.is_none());
    assert_eq!(f.value("refs/heads/target"), f.old);
    let mut bad = f.proposal();
    if let Action::Git { target_ref, .. } = &mut bad.action {
        *target_ref = "refs/heads/x\ncommit".into()
    };
    let e = run(command(f.dir.path()), &bad, None, None, || {
        panic!("invalid proposal reached decision")
    })
    .unwrap_err();
    assert_eq!(e.code(), "git-reference-proposal");
    f.control();
}
/// Real Git publishes, then a fixture-only committed hook kills it before ACK.
/// Preserve CommitRequested and actual signal through the wrapper matrix; a fresh
/// direct read observes the new OID. No retry or fabricated no-effect inference.
#[test]
fn git_reference_commit_without_ack_retains_uncertainty_and_actual_ref() {
    let f = Fixture::new();
    let hooks = f.dir.path().join("fixture-hooks");
    fs::create_dir(&hooks).unwrap();
    let hook = hooks.join("reference-transaction");
    fs::write(&hook,b"#!/usr/bin/python3\nimport os,sys\nfrom pathlib import Path\nif sys.argv[1]=='committed' and Path('fixture-kill').exists():\n Path('fixture-committed').write_text('observed')\n os.kill(os.getppid(),9)\n").unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o700)).unwrap();
    let make = || {
        let mut c = command(f.dir.path());
        c.args(["-c", &format!("core.hooksPath={}", hooks.display())]);
        c
    };
    let done = run(make(), &f.proposal(), None, None, || f.direct()).unwrap();
    assert_eq!(done.observation, Observation::CommitAcknowledged);
    output(
        f.dir.path(),
        &["update-ref", "refs/heads/target", &f.old, &f.new],
    );
    fs::write(
        f.dir.path().join("fixture-kill"),
        b"fault enabled only after control",
    )
    .unwrap();
    let e = run(make(), &f.proposal(), None, None, || f.direct()).unwrap_err();
    assert_eq!(e.observation, Observation::CommitRequested);
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(e.exit_status.unwrap().signal(), Some(9));
    assert_eq!(f.value("refs/heads/target"), f.new);
    assert!(f.dir.path().join("fixture-committed").is_file());
    wrappers(e, Observation::CommitRequested);
    fs::remove_file(f.dir.path().join("fixture-kill")).unwrap();
    output(
        f.dir.path(),
        &["update-ref", "refs/heads/target", &f.old, &f.new],
    );
    f.control();
}
/// Cancellation after prepare prevents COMMIT but forced death leaves Git's lock.
/// Retain its actual inode while the child exists. Only this disposable fixture
/// removes that same lock after confirmed reap, then observes recovery. The
/// protocol API does not claim this as protected service recovery or cleanup.
#[test]
fn git_reference_prepared_cancellation_retains_owned_process_and_lock() {
    use std::{
        cell::RefCell,
        os::unix::fs::{MetadataExt, OpenOptionsExt},
    };
    let f = Fixture::new();
    f.control();
    let (cancel, mut signal) = UnixStream::pair().unwrap();
    let held = RefCell::new(None);
    let mut e = run(
        command(f.dir.path()),
        &f.proposal(),
        None,
        Some(cancel.as_fd()),
        || {
            assert_eq!(f.direct()?, Decision::Commit);
            let file = fs::OpenOptions::new()
                .read(true)
                .custom_flags(nix::libc::O_NOFOLLOW)
                .open(f.dir.path().join("refs/heads/target.lock"))?;
            held.replace(Some(file));
            signal.write_all(b"cancel after actual prepare")?;
            Ok(Decision::Commit)
        },
    )
    .unwrap_err();
    assert_eq!(e.observation, Observation::NotRequested);
    let process = e
        .process_error_mut()
        .expect("actual native process failure retained");
    assert!(process.unresolved_process_id().is_none());
    assert!(process.ownership_lost_process_id().is_none());
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(e.exit_status.unwrap().signal(), Some(9));
    assert_eq!(f.value("refs/heads/target"), f.old);
    let file = held.into_inner().unwrap();
    let pinned = file.metadata().unwrap();
    let lock = f.dir.path().join("refs/heads/target.lock");
    let current = fs::symlink_metadata(&lock).unwrap();
    assert!(current.is_file());
    assert_eq!((pinned.dev(), pinned.ino()), (current.dev(), current.ino()));
    wrappers(e, Observation::NotRequested);
    // Fixture cleanup only: no other fixture writer exists and leader is reaped.
    fs::remove_file(lock).unwrap();
    nix::unistd::close(file).unwrap();
    f.control();
}
/// Inject a one-second caller deadline, not a suite watchdog or product default.
/// A valid update under the identical supplied duration is observed first. The
/// fault callback waits only until its actual deadline to test the pre-COMMIT
/// check. Cleanup below affects solely the pinned disposable fixture lock.
#[test]
fn git_reference_deadline_during_prepared_decision_prevents_commit() {
    use std::{
        cell::RefCell,
        os::unix::fs::{MetadataExt, OpenOptionsExt},
        time::Duration,
    };
    let f = Fixture::new();
    let duration = Duration::from_secs(1);
    let start = Instant::now();
    let done = run(
        command(f.dir.path()),
        &f.proposal(),
        Some(start + duration),
        None,
        || f.direct(),
    )
    .unwrap();
    assert_eq!(done.observation, Observation::CommitAcknowledged);
    println!(
        "GIT_REFERENCE_DEADLINE_CONTROL_NS {}",
        start.elapsed().as_nanos()
    );
    output(
        f.dir.path(),
        &["update-ref", "refs/heads/target", &f.old, &f.new],
    );
    let deadline = Instant::now() + duration;
    let held = RefCell::new(None);
    let mut e = run(
        command(f.dir.path()),
        &f.proposal(),
        Some(deadline),
        None,
        || {
            assert_eq!(f.direct()?, Decision::Commit);
            held.replace(Some(
                fs::OpenOptions::new()
                    .read(true)
                    .custom_flags(nix::libc::O_NOFOLLOW)
                    .open(f.dir.path().join("refs/heads/target.lock"))?,
            ));
            while Instant::now() < deadline {
                std::thread::sleep(deadline.saturating_duration_since(Instant::now()));
            }
            Ok(Decision::Commit)
        },
    )
    .unwrap_err();
    assert_eq!(e.observation, Observation::NotRequested);
    let process = e.process_error_mut().unwrap();
    assert_eq!(
        process
            .source()
            .unwrap()
            .downcast_ref::<io::Error>()
            .unwrap()
            .kind(),
        io::ErrorKind::TimedOut
    );
    assert!(process.unresolved_process_id().is_none());
    assert!(process.ownership_lost_process_id().is_none());
    assert_eq!(f.value("refs/heads/target"), f.old);
    let file = held.into_inner().unwrap();
    let pin = file.metadata().unwrap();
    let lock = f.dir.path().join("refs/heads/target.lock");
    let actual = fs::symlink_metadata(&lock).unwrap();
    assert!(actual.is_file());
    assert_eq!((pin.dev(), pin.ino()), (actual.dev(), actual.ino()));
    fs::remove_file(lock).unwrap();
    nix::unistd::close(file).unwrap();
    f.control();
}

/// Compose the real native runner and durable journal on disposable fixtures.
/// A valid path runs first. Lost ACK then leaves both an actually changed ref
/// and a durable pending row after reopen. This is not protected-service recovery.
#[test]
fn git_reference_durable_intent_survives_missing_ack() {
    use swarm_engine::{PaeaGitIntentReservation as Reservation, RootStore, SystemEnvironment};
    use swarm_protection_contract::{canonicalize, AdmissionContext};
    for fault in [false, true] {
        let f = Fixture::new();
        f.control();
        let journal_dir = tempfile::tempdir().unwrap();
        fs::set_permissions(journal_dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let journal = RootStore::initialize(journal_dir.path(), &mut SystemEnvironment).unwrap();
        let pin = "a".repeat(64);
        journal
            .paea_initialize_git_intents("project", &pin)
            .unwrap();
        let context = AdmissionContext {
            project_id: "project".into(),
            principal_id: "person".into(),
            instance_id: "instance".into(),
            mandate_id: "mandate".into(),
            mandate_revision: 1,
            policy_revision: 1,
            policy_digest: "a".repeat(64),
            tool_manifest_digest: "b".repeat(64),
            sources: vec![],
        };
        let mut proposal = f.proposal();
        let canonical = canonicalize(&proposal, &context).unwrap();
        let hooks = f.dir.path().join("durable-fixture-hooks");
        fs::create_dir(&hooks).unwrap();
        let hook = hooks.join("reference-transaction");
        fs::write(&hook,b"#!/usr/bin/python3\nimport os,sys\nfrom pathlib import Path\nif sys.argv[1]=='committed' and Path('durable-fixture-kill').exists(): os.kill(os.getppid(),9)\n").unwrap();
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o700)).unwrap();
        if fault {
            fs::write(
                f.dir.path().join("durable-fixture-kill"),
                b"fault after valid control",
            )
            .unwrap();
        }
        let mut cmd = command(f.dir.path());
        cmd.args(["-c", &format!("core.hooksPath={}", hooks.display())]);
        let mut decisions = 0;
        let outcome = run(cmd, &proposal, None, None, || {
            decisions += 1;
            match journal.paea_reserve_git_intent(&proposal, &context, &pin)? {
                Reservation::New(record) => {
                    // Independent connection proves the row was committed BEFORE
                    // the callback allows even one Git COMMIT byte.
                    let observer = RootStore::open(journal_dir.path(), false)?;
                    let actual = observer.paea_read_git_intent(&record.intent_id, &pin)?;
                    observer.close()?;
                    assert_eq!(actual, Some(record));
                    f.direct()
                }
                Reservation::Existing(_) => Ok(Decision::Abort),
            }
        });
        assert_eq!(decisions, 1);
        assert_eq!(f.value("refs/heads/target"), f.new);
        if fault {
            let e = outcome.unwrap_err();
            assert_eq!(e.observation, Observation::CommitRequested);
            use std::os::unix::process::ExitStatusExt;
            assert_eq!(e.exit_status.unwrap().signal(), Some(9));
        } else {
            assert_eq!(
                outcome.unwrap().observation,
                Observation::CommitAcknowledged
            );
        }
        journal.close().unwrap();
        let reopened = RootStore::open(journal_dir.path(), true).unwrap();
        proposal.request_id = "new-transport-after-restart".into();
        if let Action::Git { candidate_oid, .. } = &mut proposal.action {
            *candidate_oid = f.other.clone();
        }
        let retained = match reopened
            .paea_reserve_git_intent(&proposal, &context, &pin)
            .unwrap()
        {
            Reservation::Existing(record) => record,
            _ => panic!("must reconcile, never new commit"),
        };
        assert_eq!(retained.canonical_action, canonical.bytes());
        assert_eq!(retained.action_digest, canonical.digest());
        assert_eq!(f.value("refs/heads/target"), f.new);
        reopened.close().unwrap();
    }
}
