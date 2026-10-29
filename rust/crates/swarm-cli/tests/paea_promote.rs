//! Two-phase real CLI fixture for the guarded PAEA Git promote call: real
//! signed current authority, actual provisioned declaration, qualified transfer
//! through the delivered public CLI `paea transfer-call` (importing the
//! candidate) and one OLD-fenced reference commit or explicit reconciliation
//! on the retained owned source fixture. Fictitious effects
//! only; no protected daemon, UID separation, human presence or accepted
//! service is claimed. The service/child/pin inputs mirror the engine promote
//! harness exactly. The chain tests exercise the same candidate through the
//! guarded CLI promotions, replay and public read-only reconciliation, either
//! through the cargo-launched binary (`CARGO_BIN_EXE_swarm-rust`, default) or —
//! in an explicit test environment only — through one exact externally
//! designated delivered binary (`SWARM_PAEA_DELIVERED_BIN` bound to its
//! expected SHA-256, see `delivered_binary`); no product setting is added.
#![cfg(target_os = "linux")]
#[path = "../../swarm-enrollment-authority/tests/common/authenticated_fixture.rs"]
mod fixture;
use serde_json::{json, Value};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use swarm_engine::{RootStore, SystemEnvironment};
use swarm_identity_contract::Scope;

const PIN: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

/// Exclusive in-process guard over the shared retained source fixture. The
/// actual `SWARM_PAEA_SOURCE_FIXTURE` root is a single physical directory (its
/// inode-bound copy is invalid), so every test of this harness serializes its
/// whole ownership of the project root — from `owned()` acquisition through the
/// final witnesses — instead of running against the other tests' Trust
/// `config-apply` windows and revision reads. The guard is harness-local only:
/// it adds no product lock, no `--test-threads` requirement and does not
/// serialize tests of other binaries or other workspaces.
fn shared_project_root_guard() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::{Mutex, OnceLock};
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    let mutex = GUARD.get_or_init(|| Mutex::new(()));
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Sensitivity guard of the exclusion itself, run with the other tests:
/// concurrent acquisitions of the shared-root guard must never both proceed.
/// Without the guard this test is the reproduced race witness — two threads
/// mutating the shared fixture Trust state inside each other's windows — and
/// with it, exactly one holder advances at a time. It needs no fixture root
/// and stays cheap, so it runs in the plain (non-ignored) default lot.
/// Failure mode: a panicked holder poisons the mutex, explicitly reported as
/// `shared-project-root-guard-poisoned` instead of silently proceeding.
#[test]
fn shared_project_root_guard_admits_one_holder_at_a_time() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;
    let in_critical = Arc::new(AtomicUsize::new(0));
    let overlapping = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();
    for _ in 0..4 {
        let in_critical = in_critical.clone();
        let overlapping = overlapping.clone();
        handles.push(std::thread::spawn(move || {
            for _ in 0..25 {
                let _guard = shared_project_root_guard();
                if in_critical.fetch_add(1, Ordering::SeqCst) != 0 {
                    overlapping.fetch_add(1, Ordering::SeqCst);
                }
                std::thread::sleep(Duration::from_micros(200));
                in_critical.fetch_sub(1, Ordering::SeqCst);
            }
        }));
    }
    for handle in handles {
        handle.join().unwrap_or_else(|_| {
            panic!("shared-project-root-guard-poisoned")
        });
    }
    assert_eq!(
        overlapping.load(Ordering::SeqCst),
        0,
        "two threads held the shared project root guard at once"
    );
}

/// A designated delivered binary whose actual SHA-256 differs from the
/// mandatory witness refuses the chain before any spawn: the explicit
/// test-environment selection is bound to its declared bytes, so a silently
/// swapped or rebuilt file can never pass. Cheap, fixture-free guard over the
/// refusal branch itself (run with the plain tests, no fixture root): it
/// creates a one-byte scratch file, sets `SWARM_PAEA_DELIVERED_BIN` to it with
/// a mismatching `SWARM_PAEA_DELIVERED_BIN_SHA256`, and expects the panic
/// `designated delivered binary hash mismatch`. Process environment restored
/// afterwards; nothing else is touched.
#[test]
fn delivered_binary_refuses_wrong_sha256_witness() {
    use std::sync::Mutex;
    static ENV_LOCK: Mutex<()> = Mutex::new(());
    let _env = ENV_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("swarm-mismatch");
    fs::write(&file, b"not-the-delivered-elf").unwrap();
    // SAFETY of the env twins: tests of this binary run single-threaded per
    // test binary under the recipe's `--exact` filter; the local mutex
    // additionally serializes any concurrent plain-lot execution within this
    // harness, and the previous value is always restored below.
    let previous = std::env::var_os("SWARM_PAEA_DELIVERED_BIN");
    let previous_sha = std::env::var_os("SWARM_PAEA_DELIVERED_BIN_SHA256");
    std::env::set_var("SWARM_PAEA_DELIVERED_BIN", &file);
    std::env::set_var("SWARM_PAEA_DELIVERED_BIN_SHA256", "00".repeat(32));
    let panic = std::panic::catch_unwind(|| delivered_binary());
    match previous {
        Some(value) => std::env::set_var("SWARM_PAEA_DELIVERED_BIN", value),
        None => std::env::remove_var("SWARM_PAEA_DELIVERED_BIN"),
    }
    match previous_sha {
        Some(value) => std::env::set_var("SWARM_PAEA_DELIVERED_BIN_SHA256", value),
        None => std::env::remove_var("SWARM_PAEA_DELIVERED_BIN_SHA256"),
    }
    let message = panic
        .err()
        .map(|value| {
            value
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| value.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default()
        })
        .expect("mismatching designated binary must refuse");
    assert!(
        message.contains("designated delivered binary hash mismatch"),
        "unexpected panic message: {message}"
    );
}

/// Fresh native service storage; never open a user mission as a fixture.
fn service() -> (tempfile::TempDir, RootStore) {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let root = RootStore::initialize(dir.path(), &mut SystemEnvironment).unwrap();
    (dir, root)
}

/// CLI binary exercised by the composed chain test. By default every spawn
/// still targets the cargo-launched `CARGO_BIN_EXE_swarm-rust`; the default is
/// never inferred from a search path or replaced by any ambient build. When an
/// explicit test environment designates a delivered production binary via
/// `SWARM_PAEA_DELIVERED_BIN`, that exact file is used instead and its actual
/// SHA-256 must equal the mandatory `SWARM_PAEA_DELIVERED_BIN_SHA256` witness
/// at every spawn: a silently swapped or rebuilt binary refuses the chain.
/// Test-environment selection only: no product setting, no build-profile
/// qualification and no claim that the designated file was produced by a
/// particular compilation or delivery are made here — declared-input recipes
/// and the separately retained profile evidence prove that linkage.
fn delivered_binary() -> Command {
    match std::env::var_os("SWARM_PAEA_DELIVERED_BIN") {
        None => Command::new(env!("CARGO_BIN_EXE_swarm-rust")),
        Some(path) => {
            use sha2::Digest;
            let expected = std::env::var("SWARM_PAEA_DELIVERED_BIN_SHA256").expect(
                "SWARM_PAEA_DELIVERED_BIN_SHA256 must accompany SWARM_PAEA_DELIVERED_BIN",
            );
            let raw = fs::read(&path).expect("designated delivered binary readable");
            let actual = format!("{:x}", sha2::Sha256::digest(&raw));
            assert_eq!(
                actual, expected,
                "designated delivered binary hash mismatch: {path:?}"
            );
            println!(
                "delivered-binary-witness path={} sha256={actual}",
                path.to_string_lossy()
            );
            Command::new(path)
        }
    }
}

/// Explicit actual first-publication project, writable: the same retained
/// SWARM_PAEA_SOURCE_FIXTURE the engine harnesses use, never a copy.
fn project() -> (PathBuf, RootStore, String, String) {
    let proof = PathBuf::from(
        std::env::var_os("SWARM_PAEA_SOURCE_FIXTURE").expect("explicit new source fixture"),
    );
    let value: Value =
        serde_json::from_slice(&fs::read(proof.join("result.json")).unwrap()).unwrap();
    assert_eq!(value["status"], "PASS");
    assert_eq!(value["task_accepted"], true);
    assert_eq!(value["paid_provider"], false);
    let path = PathBuf::from(value["root"].as_str().unwrap());
    let root = RootStore::open(&path, true).unwrap();
    let work = value["work"].as_str().unwrap().to_owned();
    let current = root.current_managed_git_publication(&work).unwrap();
    (path, root, work, current.candidate_commit)
}

/// Read only a fixture plan, finalizing the independent connection before use.
fn scope(dir: &Path, work: &str) -> Scope {
    let db = rusqlite::Connection::open_with_flags(
        dir.join(".swarm/state.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let version: i64 = db
        .query_row(
            "SELECT projection_version FROM agent_identity_plan_heads WHERE work_id=?",
            [work],
            |r| r.get(0),
        )
        .unwrap();
    let projection = match version {
        1 => swarm_engine::IdentityPlanProjection::V1,
        2 => swarm_engine::IdentityPlanProjection::V2,
        _ => panic!("unknown fixture projection"),
    };
    let stamp = swarm_engine::read_identity_plan_stamp(&db, work, projection).unwrap();
    db.close().unwrap();
    Scope {
        work_id: stamp.work_id,
        plan_version: stamp.generation,
        plan_digest: stamp.digest,
    }
}

/// Read the current Trust revision from the fixture's own bounded state file.
fn current_trust_revision(root: &Path) -> i64 {
    let state: Value =
        serde_json::from_slice(&fs::read(root.join(".swarm/trust-actions.json")).unwrap())
            .unwrap();
    state["revision"].as_i64().expect("fixture Trust revision")
}

/// Actual project Trust configuration with an explicit effect role. The fixed
/// `expected_revision:0` shares the first-application idempotency of the
/// engine transfer-call harness: on an unconfigured fixture root (no
/// `trust-actions.json` yet) the apply creates the state; on a retained
/// root already at revision N the exact same event replay is accepted without
/// advancing further, so reruns never mutate the fixture Trust revision.
fn effect_config() -> Vec<u8> {
    serde_json::to_vec(&json!({
        "schema_version":1,
        "event_id":"promote-cli-effect-config",
        "expected_revision":0,
        "reason":"Explicit fixture effect role for Git promotion CLI",
        "policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":false},
        "identities":[
            {"subject":"enrolled-effect","integrity":3,"roles":["admin","effect"],"enrollment_principal":{"principal_id":"operator","principal_kind":"human","issuer":"https://fixture.invalid","subject":"stable-person"}},
            {"subject":"requester","token":"fixture-requester-token","integrity":1,"roles":["requester"]},
            {"subject":"approver","token":"fixture-approver-token","integrity":2,"roles":["approver"]}
        ]
    }))
    .unwrap()
}

/// Owned private bare child, hardened like the engine's own job checks: the
/// directory must exist, be owned and mode 0700 before `git init` so the
/// custody metadata checks see a private directory, not a shared one.
fn bare(root: &Path, child: &str) {
    let path = root.join(".swarm").join(child);
    fs::create_dir(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    let mut git = Command::new("git");
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
}

/// Seed the destination's promotion target with a real OLD object so the v1
/// proposal grammar accepts the fixture commit call.
fn seed_old(root: &Path, child: &str, ref_name: &str) -> String {
    let path = root.join(".swarm").join(child);
    let mut git = Command::new("git");
    for (key, _) in std::env::vars_os() {
        if key.as_encoded_bytes().starts_with(b"GIT_") {
            git.env_remove(key);
        }
    }
    git.env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@invalid")
        .env("GIT_COMMITTER_NAME", "fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@invalid");
    let commit = git
        .args(["--git-dir", path.to_str().unwrap(), "commit-tree"])
        .arg("4b825dc642cb6eb9a060e54bf8d69288fbee4904")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .and_then(|mut c| {
            c.stdin
                .as_mut()
                .expect("piped stdin")
                .write_all(b"fixture old\n")?;
            c.wait_with_output()
        })
        .unwrap();
    assert!(commit.status.success());
    let oid = String::from_utf8(commit.stdout).unwrap().trim().to_owned();
    let update = Command::new("git")
        .args(["--git-dir", path.to_str().unwrap(), "update-ref", ref_name, &oid])
        .output()
        .unwrap();
    assert!(update.status.success());
    oid
}

/// Full owned fixture set for one CLI promotion run: the actual native project
/// (writable, Trust applied, plan scope read), a fresh service (intents +
/// promotions journals + provisioned declaration + bare child holding the OLD
/// reference), the qualified transfer already consumed (candidate imported,
/// no ref effect) and the JSON document for the requested operation. All
/// owners must be explicitly closed by the caller.
struct Owned {
    project_path: PathBuf,
    project: RootStore,
    _service_dir: tempfile::TempDir,
    service: RootStore,
    work: String,
    candidate: String,
    trust_revision: i64,
    scope: Scope,
    document: Vec<u8>,
}

fn owned(operation: &str) -> Owned {
    let (project_path, project, work, candidate) = project();
    project
        .trust("config-apply", &effect_config(), "")
        .unwrap_or_else(|e| panic!("explicit fixture config refused: {}", e.code()));
    // The applied configuration advanced the Trust revision; the signed grant
    // and attributed context must both bind the actual post-configuration one.
    let trust_revision = current_trust_revision(&project_path);
    let fixture_scope = scope(&project_path, &work);
    let (service_dir, service) = service();
    service
        .paea_initialize_git_intents("fixture-project", PIN)
        .unwrap();
    service
        .paea_initialize_git_promotions("fixture-project", PIN)
        .unwrap();
    bare(service_dir.path(), "bare");
    let binding = service
        .paea_provision_git_repository(&project, &work, "bare", PIN)
        .unwrap();
    let repository_id = binding.repository_id().to_owned();
    drop(binding);
    let old = seed_old(service_dir.path(), "bare", "refs/heads/release");
    let proposal = serde_json::to_vec(&json!({
        "proposal":{"schema_version":1,"request_id":"promote-cli-fixture","action":{"kind":"repository.promote","repository_id":repository_id,"target_ref":"refs/heads/release","expected_old_oid":old,"candidate_oid":candidate}},
        "context":{"project_id":"fixture-project","principal_id":"operator","instance_id":"fixture-instance","mandate_id":"fixture-mandate","mandate_revision":1,"policy_revision":trust_revision,"policy_digest":"aa".repeat(32),"tool_manifest_digest":"bb".repeat(32),"sources":[{"source_id":"fixture-source","version":"1","digest":"cc".repeat(32),"integrity":2,"confidentiality_labels":[]}]}
    }))
    .unwrap();
    // Real business order: the qualified transfer imports the exact candidate
    // pack before any promotion, or Git refuses the ref write with a
    // nonexistent object. The transfer runs through the delivered public CLI
    // two-phase `paea transfer-call` operation on the one designated binary
    // (see `delivered_binary`, re-hashed at each spawn) with the exact
    // permission-provisioning and fixture-enrollment pattern of the transfer
    // harness (`tests/paea_transfer.rs`), never through a direct engine
    // library call. The receipt names the exact imported candidate.
    let receipt = invoke_transfer_via_delivered_cli(
        &project_path,
        service_dir.path(),
        &work,
        &fixture_scope,
        trust_revision,
        &proposal,
    );
    assert_eq!(receipt["candidate"], candidate);
    assert!(receipt["pack_bytes"].as_u64().unwrap() > 0);
    assert_eq!(receipt["pack_sha256"].as_str().unwrap().len(), 64);
    Owned {
        project_path,
        project,
        _service_dir: service_dir,
        service,
        work,
        candidate,
        trust_revision,
        scope: fixture_scope,
        document: proposal,
    }
}

/// Full two-phase invocation with real out-of-band provisioning: the prepared
/// event names the exact permission, the fixture mints it into a declaration,
/// then the frame answers with the fixture pin. `bad_frame` replaces the frame
/// with a wrong header. The provisioned grant binds the preparation-time Trust
/// revision and nothing mutates the current state inside the window.
fn invoke_with_provisioning(
    owned: &Owned,
    operation: &str,
    bad_frame: bool,
) -> (std::process::ExitStatus, String, String) {
    invoke_between(owned, operation, bad_frame, owned.trust_revision, &|| {})
}

/// Same real two-phase invocation with the provisioning window explicitly
/// open: `between` runs after the public prepared event and before the private
/// frame, while the CLI process holds both prepared physical witnesses and no
/// reservation, mirroring an out-of-band change of the current authority. The
/// provisioned grant binds the explicitly supplied current Trust revision, not
/// the preparation-time one; no other parameter changes.
fn invoke_between(
    owned: &Owned,
    operation: &str,
    bad_frame: bool,
    revision: i64,
    between: &dyn Fn(),
) -> (std::process::ExitStatus, String, String) {
    invoke_with(owned, operation, bad_frame, revision, between, delivered_binary())
}

/// `delivered_chain` variant: the two-phase invocation runs on an explicit CLI
/// binary selected by `delivered_binary()`, so transfer preparation frames and
/// both guarded promotion calls exercise one externally designated executable
/// instead of the cargo-launched one. Re-verification of the binary hash at
/// each preparation spawn keeps the whole window bound to the same bytes.
///
/// `delivered_transfer` variant: the same two-phase consumption shape, run
/// against the transfer operation's prepared event and grammar instead of the
/// promotion one (event name, operation label and stdio shape differ); used by
/// `owned()` so the composed chain's transfer step exercises the exact public
/// CLI `paea transfer-call` contract on the designated binary.
fn invoke_with(
    owned: &Owned,
    operation: &str,
    bad_frame: bool,
    revision: i64,
    between: &dyn Fn(),
    mut command: Command,
) -> (std::process::ExitStatus, String, String) {
    let service_dir = owned._service_dir.path();
    let input = service_dir.join("promote-request.json");
    fs::write(&input, &owned.document).unwrap();
    let declaration_dir = service_dir.join("operator-declaration");
    fs::create_dir_all(&declaration_dir).unwrap();
    fs::set_permissions(&declaration_dir, fs::Permissions::from_mode(0o700)).unwrap();
    let declaration = declaration_dir.join("declaration.json");
    let mut child = command
        .args([
            "--root",
            service_dir.to_str().unwrap(),
            "--json",
            "paea",
            "promote-call",
            operation,
            service_dir.to_str().unwrap(),
            owned.project_path.to_str().unwrap(),
            &owned.work,
            "bare",
            PIN,
            declaration.to_str().unwrap(),
            "--input",
            input.to_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut first = String::new();
    let read = stdout.read_line(&mut first).unwrap();
    if read == 0 {
        let output = child.wait_with_output().unwrap();
        return (
            output.status,
            String::new(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        );
    }
    let prepared: Value = serde_json::from_str(&first).unwrap();
    assert_eq!(prepared["event"], "paea-promote-prepared");
    assert_eq!(prepared["authority"], false);
    assert_eq!(prepared["restart_reusable"], false);
    assert_eq!(prepared["operation"], operation);
    let permission: swarm_identity_contract::Permission =
        serde_json::from_value(prepared["permission"].clone()).unwrap();
    between();
    let f = fixture::Fixture::build_for_trust(permission, owned.scope.clone(), revision);
    if !bad_frame {
        fs::copy(&f.path, &declaration).unwrap();
        fs::set_permissions(&declaration, fs::Permissions::from_mode(0o600)).unwrap();
    }
    let packet = if bad_frame {
        b"wrong-header\n".to_vec()
    } else {
        let mut packet = format!("swarm-trust-source-v1\n{}\n", f.pin).into_bytes();
        packet.extend_from_slice(b"STARTUP_FIXTURE_PASSWORD");
        packet
    };
    child.stdin.take().unwrap().write_all(&packet).unwrap();
    let mut rest = String::new();
    stdout.read_to_string(&mut rest).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(!rest.contains("STARTUP_FIXTURE_PASSWORD"));
    assert!(!String::from_utf8_lossy(&output.stderr)
        .contains("STARTUP_FIXTURE_PASSWORD"));
    f.close();
    (
        output.status,
        rest,
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

/// Run the qualified transfer of the composed chain through the delivered
/// public CLI `paea transfer-call` two-phase operation on the one designated
/// binary (`delivered_binary()`, whose SHA-256 is re-proven at this spawn),
/// reusing the exact permission-provisioning and fixture-enrollment pattern of
/// the transfer harness (`tests/paea_transfer.rs::invoke_with_provisioning`):
/// the prepared `paea-transfer-prepared` event names the exact permission, the
/// fixture mints it into a declaration, then the frame answers with the
/// fixture pin and password. Inputs: the owned project path (already opened
/// writable and Trust-configured by `owned()`), the fresh service root path
/// (already initialized and provisioned), the retained fixture work id, the
/// fixture plan scope and the current Trust revision to bind the grant to, and
/// the exact promotion request document whose `{proposal, context}` form feeds
/// the transfer's strict input grammar. Output: the parsed
/// `paea-transfer-result` receipt (candidate, pack_bytes, pack_sha256). No
/// library transfer call happens here; every refusal propagates as a panic
/// with the CLI stderr so a blocked chain cannot silently fall back.
///
/// Returns the parsed receipt; panics on any CLI refusal (checked exits and
/// event/field assertions), so callers see the exact stderr cause.
fn invoke_transfer_via_delivered_cli(
    project_path: &Path,
    service_dir: &Path,
    work: &str,
    fixture_scope: &Scope,
    trust_revision: i64,
    document: &[u8],
) -> Value {
    let input = service_dir.join("transfer-request.json");
    fs::write(&input, document).unwrap();
    let declaration_dir = service_dir.join("operator-declaration");
    fs::create_dir_all(&declaration_dir).unwrap();
    fs::set_permissions(&declaration_dir, fs::Permissions::from_mode(0o700)).unwrap();
    let declaration = declaration_dir.join("declaration.json");
    let mut child = delivered_binary()
        .args([
            "--root",
            service_dir.to_str().unwrap(),
            "--json",
            "paea",
            "transfer-call",
            service_dir.to_str().unwrap(),
            project_path.to_str().unwrap(),
            work,
            "bare",
            PIN,
            "transfer-pack",
            declaration.to_str().unwrap(),
            "--input",
            input.to_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut first = String::new();
    let read = stdout.read_line(&mut first).unwrap();
    if read == 0 {
        let output = child.wait_with_output().unwrap();
        panic!(
            "delivered CLI transfer preparation produced no event: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let prepared: Value = serde_json::from_str(&first).unwrap();
    assert_eq!(prepared["event"], "paea-transfer-prepared");
    assert_eq!(prepared["authority"], false);
    assert_eq!(prepared["restart_reusable"], false);
    assert_eq!(prepared["operation"], "reconcile");
    let permission: swarm_identity_contract::Permission =
        serde_json::from_value(prepared["permission"].clone()).unwrap();
    let f = fixture::Fixture::build_for_trust(permission, fixture_scope.clone(), trust_revision);
    fs::copy(&f.path, &declaration).unwrap();
    fs::set_permissions(&declaration, fs::Permissions::from_mode(0o600)).unwrap();
    let mut packet = format!("swarm-trust-source-v1\n{}\n", f.pin).into_bytes();
    packet.extend_from_slice(b"STARTUP_FIXTURE_PASSWORD");
    child.stdin.take().unwrap().write_all(&packet).unwrap();
    let mut rest = String::new();
    stdout.read_to_string(&mut rest).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(!rest.contains("STARTUP_FIXTURE_PASSWORD"));
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(!stderr.contains("STARTUP_FIXTURE_PASSWORD"));
    f.close();
    assert!(
        output.status.success(),
        "delivered CLI transfer refused: {stderr}"
    );
    serde_json::from_str(rest.trim()).unwrap()
}

/// Real authority consumes one exact commit through the CLI: the durable
/// intent is reserved, the reference is created at the candidate under OLD
/// fencing and the effect row is recorded. No frame secret ever escapes.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn promote_call_cli_commits_once_and_records_effect() {
    let _root = shared_project_root_guard();
    let owned = owned("commit");
    let (status, rest, stderr) = invoke_with_provisioning(&owned, "commit", false);
    assert!(status.success(), "stderr: {stderr}");
    let reply: Value = serde_json::from_str(rest.trim()).unwrap();
    assert_eq!(reply["event"], "paea-promote-result");
    assert_eq!(reply["outcome"], "recorded");
    assert_eq!(reply["replayed"], false);
    assert_eq!(reply["target_ref"], "refs/heads/release");
    assert_eq!(reply["new_oid"], owned.candidate);
    // The actual bare reference now names the candidate.
    let refs = Command::new("git")
        .args([
            "--git-dir",
            owned._service_dir.path().join(".swarm/bare").to_str().unwrap(),
            "rev-parse",
            "--verify",
            "refs/heads/release",
        ])
        .output()
        .unwrap();
    assert!(refs.status.success());
    assert_eq!(
        String::from_utf8(refs.stdout).unwrap().trim(),
        owned.candidate
    );
    owned.service.close().unwrap();
    owned.project.close().unwrap();
}

/// Reconciliation before any commit reports the explicit negative outcome
/// through the CLI with no Git process and no ref change.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn promote_call_cli_reconciles_before_commit_reports_not_committed() {
    let _root = shared_project_root_guard();
    let owned = owned("reconcile");
    let (status, rest, stderr) = invoke_with_provisioning(&owned, "reconcile", false);
    assert!(status.success(), "stderr: {stderr}");
    let reply: Value = serde_json::from_str(rest.trim()).unwrap();
    assert_eq!(reply["event"], "paea-promote-result");
    assert_eq!(reply["outcome"], "reconciled_not_committed");
    assert!(reply["intent_id"].as_str().unwrap().len() == 64);
    // The bare reference still names the seeded OLD object, never the candidate.
    let refs = Command::new("git")
        .args([
            "--git-dir",
            owned._service_dir.path().join(".swarm/bare").to_str().unwrap(),
            "rev-parse",
            "--verify",
            "refs/heads/release",
        ])
        .output()
        .unwrap();
    assert!(refs.status.success());
    assert_ne!(
        String::from_utf8(refs.stdout).unwrap().trim(),
        owned.candidate
    );
    owned.service.close().unwrap();
    owned.project.close().unwrap();
}

/// One composed real chain on the same owned roots and candidate: the
/// qualified transfer imports the exact candidate, the guarded CLI promotion
/// commits it under OLD fencing, an identical CLI promotion call replays the
/// same durable intent without a second publication, and the public read-only
/// `reconcile-effects` CLI on a separate fresh ledger reports exactly one
/// verified Git effect bound to that intent. Test-environment composition
/// only: the agent identity is the existing explicitly simulated fixture, no
/// UID separation, human presence or accepted service is proven.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn promote_call_cli_chain_transfer_promote_replay_reconcile_once() {
    let _root = shared_project_root_guard();
    let owned = owned("commit");
    // --- Step 1: guarded CLI promotion commits the transferred candidate. ---
    let (status, rest, stderr) = invoke_with_provisioning(&owned, "commit", false);
    assert!(status.success(), "stderr: {stderr}");
    let first: Value = serde_json::from_str(rest.trim()).unwrap();
    assert_eq!(first["event"], "paea-promote-result");
    assert_eq!(first["outcome"], "recorded");
    assert_eq!(first["replayed"], false);
    assert_eq!(first["target_ref"], "refs/heads/release");
    assert_eq!(first["new_oid"], owned.candidate);
    let first_intent = first["intent_id"].as_str().unwrap().to_owned();
    assert_eq!(first_intent.len(), 64);
    let first_digest = first["action_digest"].as_str().unwrap().to_owned();
    // The actual bare reference now names the exact transferred candidate.
    let observed = |dir: &Path| {
        let refs = Command::new("git")
            .args([
                "--git-dir",
                dir.join(".swarm/bare").to_str().unwrap(),
                "rev-parse",
                "--verify",
                "refs/heads/release",
            ])
            .output()
            .unwrap();
        assert!(refs.status.success());
        String::from_utf8(refs.stdout).unwrap().trim().to_owned()
    };
    assert_eq!(observed(owned._service_dir.path()), owned.candidate);
    // --- Step 2: identical CLI promotion replays one durable intent. ---
    let (status, rest, stderr) = invoke_with_provisioning(&owned, "commit", false);
    assert!(status.success(), "stderr: {stderr}");
    let replay: Value = serde_json::from_str(rest.trim()).unwrap();
    assert_eq!(replay["event"], "paea-promote-result");
    assert_eq!(replay["outcome"], "recorded");
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["intent_id"], first["intent_id"]);
    assert_eq!(replay["action_digest"], first["action_digest"]);
    assert_eq!(replay["new_oid"], owned.candidate);
    // The reference never moved: no second publication occurred.
    assert_eq!(observed(owned._service_dir.path()), owned.candidate);
    // --- Step 3: public read-only CLI reconciliation on a separate ledger. ---
    let ledger_dir = tempfile::tempdir().unwrap();
    fs::set_permissions(ledger_dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    RootStore::initialize(ledger_dir.path(), &mut SystemEnvironment)
        .unwrap()
        .close()
        .unwrap();
    let report_raw = Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .args([
            "paea",
            "reconcile-effects",
            ledger_dir.path().to_str().unwrap(),
            "--json",
            "--root",
            owned._service_dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        report_raw.status.success(),
        "{}",
        String::from_utf8_lossy(&report_raw.stderr)
    );
    assert!(report_raw.stderr.is_empty());
    let report: Value = serde_json::from_slice(&report_raw.stdout).unwrap();
    assert_eq!(report["schema_version"], 1);
    // The fresh separate ledger has no settlement journal: explicit state.
    assert_eq!(report["settlement"]["state"], "uninitialized");
    // The Git journal holds exactly one effect, the verified chain effect.
    assert_eq!(report["git"]["state"], "consistent");
    assert_eq!(report["git"]["effectCount"], 1);
    assert_eq!(report["git"]["findings"][0]["intent_id"], first_intent);
    assert_eq!(report["git"]["findings"][0]["action_digest"], first_digest);
    assert_eq!(
        report["git"]["findings"][0]["verified"],
        true,
        "unexpected refusals: {:?}",
        report["git"]["findings"][0]["refusals"]
    );
    owned.service.close().unwrap();
    owned.project.close().unwrap();
}

/// One composed real chain through one externally designated delivered binary
/// (see `delivered_binary`): the qualified transfer importing the exact
/// candidate through the public CLI `paea transfer-call` two-phase operation
/// (see `invoke_transfer_via_delivered_cli`), the guarded CLI
/// promotion committing it under OLD fencing, an identical promotion call
/// replaying the same durable intent without a second publication, and the
/// public read-only `reconcile-effects` CLI on a separate fresh ledger
/// reporting exactly one verified Git effect. Every process spawn of this
/// chain — the transfer, both promotion
/// preparations and the reconciliation — targets the one designated binary,
/// whose SHA-256 is re-proven at each spawn. Test-environment composition
/// only: the agent identity is the existing explicitly simulated fixture, no
/// UID separation, human presence, accepted service or delivery provenance is
/// proven here; without `SWARM_PAEA_DELIVERED_BIN` the chain runs on the
/// cargo-launched binary exactly as before.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn delivered_chain_transfer_promote_replay_reconcile_once() {
    let _root = shared_project_root_guard();
    let owned = owned("commit");
    // --- Step 1: guarded CLI promotion commits the transferred candidate. ---
    let (status, rest, stderr) = invoke_with_provisioning(&owned, "commit", false);
    assert!(status.success(), "stderr: {stderr}");
    let first: Value = serde_json::from_str(rest.trim()).unwrap();
    assert_eq!(first["event"], "paea-promote-result");
    assert_eq!(first["outcome"], "recorded");
    assert_eq!(first["replayed"], false);
    assert_eq!(first["target_ref"], "refs/heads/release");
    assert_eq!(first["new_oid"], owned.candidate);
    let first_intent = first["intent_id"].as_str().unwrap().to_owned();
    assert_eq!(first_intent.len(), 64);
    let first_digest = first["action_digest"].as_str().unwrap().to_owned();
    let observed = |dir: &Path| {
        let refs = Command::new("git")
            .args([
                "--git-dir",
                dir.join(".swarm/bare").to_str().unwrap(),
                "rev-parse",
                "--verify",
                "refs/heads/release",
            ])
            .output()
            .unwrap();
        assert!(refs.status.success());
        String::from_utf8(refs.stdout).unwrap().trim().to_owned()
    };
    assert_eq!(observed(owned._service_dir.path()), owned.candidate);
    // --- Step 2: identical CLI promotion replays one durable intent. ---
    let (status, rest, stderr) = invoke_with_provisioning(&owned, "commit", false);
    assert!(status.success(), "stderr: {stderr}");
    let replay: Value = serde_json::from_str(rest.trim()).unwrap();
    assert_eq!(replay["event"], "paea-promote-result");
    assert_eq!(replay["outcome"], "recorded");
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["intent_id"], first["intent_id"]);
    assert_eq!(replay["action_digest"], first["action_digest"]);
    assert_eq!(replay["new_oid"], owned.candidate);
    assert_eq!(observed(owned._service_dir.path()), owned.candidate);
    // --- Step 3: public read-only CLI reconciliation on a separate ledger. ---
    let ledger_dir = tempfile::tempdir().unwrap();
    fs::set_permissions(ledger_dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    RootStore::initialize(ledger_dir.path(), &mut SystemEnvironment)
        .unwrap()
        .close()
        .unwrap();
    let mut command = delivered_binary();
    let report_raw = command
        .args([
            "paea",
            "reconcile-effects",
            ledger_dir.path().to_str().unwrap(),
            "--json",
            "--root",
            owned._service_dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        report_raw.status.success(),
        "{}",
        String::from_utf8_lossy(&report_raw.stderr)
    );
    assert!(report_raw.stderr.is_empty());
    let report: Value = serde_json::from_slice(&report_raw.stdout).unwrap();
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["settlement"]["state"], "uninitialized");
    assert_eq!(report["git"]["state"], "consistent");
    assert_eq!(report["git"]["effectCount"], 1);
    assert_eq!(report["git"]["findings"][0]["intent_id"], first_intent);
    assert_eq!(report["git"]["findings"][0]["action_digest"], first_digest);
    assert_eq!(
        report["git"]["findings"][0]["verified"],
        true,
        "unexpected refusals: {:?}",
        report["git"]["findings"][0]["refusals"]
    );
    owned.service.close().unwrap();
    owned.project.close().unwrap();
}

/// A wrong frame refuses without any preparation consumption and without any
/// business mutation; the roots stay independently closed and Trust state is
/// unchanged.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn promote_call_cli_refuses_bad_frame_without_consumption() {
    let _root = shared_project_root_guard();
    let owned = owned("commit");
    let before = fs::read(owned.project_path.join(".swarm/trust-actions.json")).unwrap();
    let (status, rest, stderr) = invoke_with_provisioning(&owned, "commit", true);
    assert!(!status.success());
    assert!(rest.is_empty(), "unexpected stdout: {rest}");
    assert!(
        stderr.contains("paea-promote-call-frame"),
        "stderr: {stderr}"
    );
    let after = fs::read(owned.project_path.join(".swarm/trust-actions.json")).unwrap();
    assert_eq!(before, after);
    owned.service.close().unwrap();
    owned.project.close().unwrap();
}

/// One real revocation window between the guarded CLI preparation and its
/// consumption: after the public prepared event (witnesses held, no
/// reservation), the operator performs the supported public Trust
/// configuration change `config-apply` on the project through the library,
/// disabling the effect identity; this advances the current Trust revision.
/// The still-signed grant then refuses twice under current storage state —
/// `paea-git-promote-trust-revision` for the preparation-time revision and,
/// with a revision-matched grant, `paea-git-promote-effect-role` for the
/// disabled effect identity — with no Git process, no reference change and no
/// durable effect line. After the public restore configuration the explicitly
/// re-provisioned current grant consumes the same prepared call exactly once:
/// `recorded`, replayed=false, the reference at the candidate and exactly one
/// durable effect. Authority stays the explicit simulated fixture; no UID
/// separation, human presence or accepted service is proven.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn promote_call_cli_revoked_current_authority_refuses_without_effect_then_recovers() {
    let _root = shared_project_root_guard();
    let owned = owned("commit");
    // Evidence witnesses taken before any revocation window.
    let trust_state = |root: &Path| {
        let value: Value =
            serde_json::from_slice(&fs::read(root.join(".swarm/trust-actions.json")).unwrap())
                .unwrap();
        value["revision"].as_i64().unwrap()
    };
    let read_ref = |owned: &Owned| -> String {
        let refs = Command::new("git")
            .args([
                "--git-dir",
                owned
                    ._service_dir
                    .path()
                    .join(".swarm/bare")
                    .to_str()
                    .unwrap(),
                "rev-parse",
                "--verify",
                "refs/heads/release",
            ])
            .output()
            .unwrap();
        assert!(refs.status.success());
        String::from_utf8(refs.stdout).unwrap().trim().to_owned()
    };
    // Before promotion the reference holds the seeded OLD object, not the
    // candidate; every refusal witness compares against this observed start.
    let old_ref = read_ref(&owned);
    assert_ne!(old_ref, owned.candidate);
    let assert_ref = |owned: &Owned, expected: &str| assert_eq!(read_ref(owned), expected);
    let effects = |service_dir: &Path| -> i64 {
        let db = rusqlite::Connection::open_with_flags(
            service_dir.join(".swarm/state.db"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let n = db
            .query_row(
                "SELECT count(*) FROM paea_git_promote_effects",
                [],
                |r| r.get(0),
            )
            .unwrap();
        db.close().unwrap();
        n
    };
    let effects_before = effects(&owned._service_dir.path());
    assert_eq!(effects_before, 0);
    // Snapshot the configured identity set so the revocation disables the
    // effect mapping and the restore applies the identical set enabled again,
    // both through the same supported public configuration operation.
    let view: Value = serde_json::from_slice(
        &owned.project.trust("show", b"{}", "").unwrap().bytes,
    )
    .unwrap();
    let mut identities = view["identities"].clone();
    for id in identities.as_array_mut().unwrap() {
        if id["enrollment_principal"].is_null() {
            id["token"] = format!("fixture-{}-token", id["subject"].as_str().unwrap()).into();
        }
    }
    let restored = identities.clone();
    for id in identities.as_array_mut().unwrap() {
        if id["subject"] == "enrolled-effect" {
            id["disabled"] = true.into();
        }
    }
    let revision_before = trust_state(&owned.project_path);
    // The event id binds the revision it applies to, matching the established
    // transfer-harness pattern: each run on the retained root creates fresh
    // events, never a conflicting replay of an older retained fingerprint.
    let change = |event: &str, revision: i64, ids: &Value| {
        serde_json::to_vec(&json!({
            "schema_version":1,
            "event_id":format!("{event}-{revision}"),
            "expected_revision":revision,
            "reason":"Explicit fixture current-authority change",
            "policy":view["policy"],
            "identities":ids
        }))
        .unwrap()
    };
    // --- Window 1: revocation between preparation and consumption. ---
    let (status, rest, stderr) = invoke_between(&owned, "commit", false, owned.trust_revision, &|| {
        let applied: Value = serde_json::from_slice(
            &owned
                .project
                .trust(
                    "config-apply",
                    &change("promote-cli-revoke-effect", revision_before, &identities),
                    "",
                )
                .unwrap()
                .bytes,
        )
        .unwrap();
        assert_eq!(applied["revision"], revision_before + 1);
    });
    assert!(!status.success());
    assert!(rest.is_empty(), "unexpected stdout: {rest}");
    let error: Value = serde_json::from_str(stderr.trim()).unwrap();
    assert_eq!(error["failure"]["code"], "paea-git-promote-action");
    assert!(
        error["failure"]["causes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|cause| cause == "paea-git-promote-trust-revision"),
        "unexpected causes: {:?}",
        error["failure"]["causes"]
    );
    // Refusal left no current change beyond the explicit revocation itself.
    assert_eq!(trust_state(&owned.project_path), revision_before + 1);
    assert_eq!(read_ref(&owned), old_ref);
    assert_eq!(effects(&owned._service_dir.path()), effects_before);
    // --- Window 2: same revoked current state with a revision-matched grant. ---
    let revoked_revision = revision_before + 1;
    let (status, rest, stderr) =
        invoke_between(&owned, "commit", false, revoked_revision, &|| {});
    assert!(!status.success());
    assert!(rest.is_empty(), "unexpected stdout: {rest}");
    let error: Value = serde_json::from_str(stderr.trim()).unwrap();
    assert_eq!(error["failure"]["code"], "paea-git-promote-action");
    assert!(
        error["failure"]["causes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|cause| cause == "paea-git-promote-effect-role"),
        "unexpected causes: {:?}",
        error["failure"]["causes"]
    );
    assert_eq!(trust_state(&owned.project_path), revoked_revision);
    assert_eq!(read_ref(&owned), old_ref);
    assert_eq!(effects(&owned._service_dir.path()), effects_before);
    // --- Legitimate recovery: public restore, then one exact consumption. ---
    let applied: Value = serde_json::from_slice(
        &owned
            .project
            .trust(
                "config-apply",
                &change("promote-cli-restore-effect", revoked_revision, &restored),
                "",
            )
            .unwrap()
            .bytes,
    )
    .unwrap();
    assert_eq!(applied["revision"], revoked_revision + 1);
    let restored_revision = trust_state(&owned.project_path);
    let (status, rest, stderr) =
        invoke_between(&owned, "commit", false, restored_revision, &|| {});
    assert!(status.success(), "stderr: {stderr}");
    let reply: Value = serde_json::from_str(rest.trim()).unwrap();
    assert_eq!(reply["event"], "paea-promote-result");
    assert_eq!(reply["outcome"], "recorded");
    assert_eq!(reply["replayed"], false);
    assert_eq!(reply["target_ref"], "refs/heads/release");
    assert_eq!(reply["new_oid"], owned.candidate);
    assert_eq!(reply["intent_id"].as_str().unwrap().len(), 64);
    assert_eq!(trust_state(&owned.project_path), restored_revision);
    assert_eq!(effects(&owned._service_dir.path()), effects_before + 1);
    assert_eq!(read_ref(&owned), owned.candidate);
    owned.service.close().unwrap();
    owned.project.close().unwrap();
}
