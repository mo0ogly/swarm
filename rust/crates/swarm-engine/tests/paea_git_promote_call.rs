//! Guarded protected Git reference promotion under real signed current
//! authority: durable intent, exact OLD fencing and recorded effect on owned
//! fixtures. Fictitious effects only; no protected daemon, UID separation,
//! human presence or accepted service is claimed.
#![cfg(target_os = "linux")]
#[path = "../../swarm-enrollment-authority/tests/common/authenticated_fixture.rs"]
mod fixture;
use serde_json::{json, Value};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};
use swarm_engine::{
    PaeaGitPromoteCallError, PaeaGitPromoteOutcome, PaeaGitTransferCallError, RootStore,
    RootStoreError, SystemEnvironment,
};
use swarm_enrollment_authority::{authenticated_control::Context as UseContext, authenticated_startup};
use swarm_enrollment_state::Context;
use swarm_identity_contract::Scope;
use swarm_protection_contract::{Action, AdmissionContext, Proposal, SourceRef};
const PIN: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
/// Resolve/pin actual installed Git; shared fixture pattern from protocol69.
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
            let value: Value =
                serde_json::from_slice(&fs::read(entry.path()).unwrap()).unwrap();
            if value["root"] == root {
                return true;
            }
        }
    }
    false
}
/// Fresh native service storage; never open a user mission as a fixture.
fn service() -> (tempfile::TempDir, RootStore) {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let root = RootStore::initialize(dir.path(), &mut SystemEnvironment).unwrap();
    (dir, root)
}
/// Explicit actual first-publication project from the retained source fixture;
/// the native review receipts bind the real .swarm device and inode, so no
/// copy can substitute for the actual fixture root.
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
    assert!(born(&proof, path.to_str().unwrap()));
    let root = RootStore::open(&path, true).unwrap();
    let work = value["work"].as_str().unwrap().to_owned();
    let current = root.current_managed_git_publication(&work).unwrap();
    (path, root, work, current.candidate_commit)
}
/// Prepare an owned private Git child; no inherited Git config or hooks.
fn bare(root: &Path, child: &str) {
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
/// Read the current Trust revision from the fixture's own bounded state file
/// so the applied configuration expects the actual next-revision predecessor.
fn current_trust_revision(root: &Path) -> i64 {
    let state: Value =
        serde_json::from_slice(&fs::read(root.join(".swarm/trust-actions.json")).unwrap())
            .unwrap();
    state["revision"].as_i64().expect("fixture Trust revision")
}
/// Actual project Trust configuration with an explicit effect role; tokens only
/// for local fixture identities, never enrolled ones.
fn effect_config(root: &Path) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "schema_version":1,
        "event_id":format!("promote-effect-config-{}", current_trust_revision(root)),
        "expected_revision":current_trust_revision(root),
        "reason":"Explicit fixture effect role for Git promotion",
        "policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":false},
        "identities":[
            {"subject":"enrolled-effect","integrity":3,"roles":["admin","effect"],"enrollment_principal":{"principal_id":"operator","principal_kind":"human","issuer":"https://fixture.invalid","subject":"stable-person"}},
            {"subject":"requester","token":"fixture-requester-token","integrity":1,"roles":["requester"]},
            {"subject":"approver","token":"fixture-approver-token","integrity":2,"roles":["approver"]}
        ]
    }))
    .unwrap()
}
/// Valid Git proposal for the service-assigned repository and actual candidate.
/// The v1 grammar refuses the zero-OID marker, so OLD must be a real existing
/// reference in the destination: the fixture seeds it before the commit call.
fn proposal(id: &str, candidate: &str, old: &str) -> Proposal {
    Proposal {
        schema_version: 1,
        request_id: "promote-fixture".into(),
        action: Action::Git {
            repository_id: id.into(),
            target_ref: "refs/heads/release".into(),
            expected_old_oid: old.into(),
            candidate_oid: candidate.into(),
        },
    }
}
/// Seed the destination's promotion target with a real OLD object: one commit
/// directly on the bare child using the hardened environment; the actual
/// candidate object is imported by the object-format/bare checks through the
/// managed source in the real flow, so the OLD commit is created in place.
fn seed_old(root: &Path, child: &str, ref_name: &str) -> String {
    let path = root.join(".swarm").join(child);
    let mut git = Command::new(git_executable());
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
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut c| {
            use std::io::Write;
            c.stdin
                .as_mut()
                .expect("piped stdin")
                .write_all(b"fixture old\n")?;
            c.wait_with_output()
        })
        .unwrap();
    assert!(commit.status.success());
    let oid = String::from_utf8(commit.stdout).unwrap().trim().to_owned();
    let update = Command::new(git_executable())
        .args([
            "--git-dir",
            path.to_str().unwrap(),
            "update-ref",
            ref_name,
            &oid,
        ])
        .output()
        .unwrap();
    assert!(update.status.success());
    oid
}
/// Declared context: policy fields are declarations rechecked under held
/// authority; one declared source, never an admission evidence. The revision
/// is the actual post-configuration Trust revision of this fixture.
fn context(revision: i64) -> AdmissionContext {
    AdmissionContext {
        project_id: "fixture-project".into(),
        principal_id: "operator".into(),
        instance_id: "fixture-instance".into(),
        mandate_id: "fixture-mandate".into(),
        mandate_revision: 1,
        policy_revision: revision,
        policy_digest: "aa".repeat(32),
        tool_manifest_digest: "bb".repeat(32),
        sources: vec![SourceRef {
            source_id: "fixture-source".into(),
            version: "1".into(),
            digest: "cc".repeat(32),
            integrity: 2,
            confidentiality_labels: vec![],
        }],
    }
}
/// Complete owned fixture set for one promotion call: the actual native project
/// (writable, Trust applied, plan scope read), a fresh service (intents +
/// promotions journals + provisioned declaration + bare child holding the OLD
/// reference and the imported candidate), the exact qualified transfer receipt
/// and both exact prepared calls (Commit and Reconcile). All owners must be
/// explicitly closed by the caller; the native project directory is an owned
/// test fixture and is never deleted here.
struct Owned {
    project_path: PathBuf,
    project: RootStore,
    _service_dir: tempfile::TempDir,
    service: RootStore,
    work: String,
    candidate: String,
    trust_revision: i64,
    transfer_receipt: swarm_engine::PaeaGitTransferReceipt,
    commit_prepared: swarm_engine::PaeaGitPromoteCall,
    reconcile_prepared: swarm_engine::PaeaGitPromoteCall,
    scope: Scope,
}
fn owned() -> Owned {
    let (project_path, project, work, candidate) = project();
    project
        .trust("config-apply", &effect_config(&project_path), "")
        .unwrap_or_else(|e| panic!("explicit fixture config refused: {}", e.code()));
    // The applied configuration advanced the Trust revision; the signed grant
    // and attributed context must both bind the actual post-configuration one.
    let trust_revision = current_trust_revision(&project_path);
    let scope = scope(&project_path, &work);
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
    let old = seed_old(service_dir.path(), "bare", "refs/heads/release");
    let proposal = proposal(binding.repository_id(), &candidate, &old);
    // Real business order: the qualified transfer (exact candidate pack import,
    // no ref effect) precedes the promotion, or Git refuses the ref write with
    // a nonexistent object. The transfer uses the same signed-current-authority
    // pattern as the promotion call below.
    let transfer_prepared = service
        .paea_git_transfer_call(
            &project,
            &proposal,
            &context(trust_revision),
            &work,
            "bare",
            PIN,
            "transfer-pack",
        )
        .unwrap_or_else(|e| panic!("explicit transfer preparation refused: {:?}", e));
    let transfer_receipt = invoke_transfer(
        &project,
        &service,
        &transfer_prepared,
        &scope,
        trust_revision,
    )
    .unwrap();
    assert_eq!(transfer_receipt.candidate, candidate);
    transfer_prepared.close().unwrap();
    // Commit policy fields are storage-attributed; Reconcile keeps historical
    // context (settlement contract). Both canonical digests must therefore be
    // identical: the Reconcile preparation reuses the attributed context.
    let commit_prepared = service
        .paea_git_promote_call(
            &project,
            &proposal,
            &context(trust_revision),
            swarm_enrollment_authority::paea_action::Operation::Commit,
            &work,
            "bare",
            PIN,
        )
        .unwrap_or_else(|e| panic!("explicit commit preparation refused: {:?}", e));
    let reconcile_prepared = service
        .paea_git_promote_call(
            &project,
            &proposal,
            commit_prepared.context(),
            swarm_enrollment_authority::paea_action::Operation::Reconcile,
            &work,
            "bare",
            PIN,
        )
        .unwrap_or_else(|e| panic!("explicit reconcile preparation refused: {:?}", e));
    Owned {
        project_path,
        project,
        _service_dir: service_dir,
        service,
        work,
        candidate,
        trust_revision,
        transfer_receipt,
        commit_prepared,
        reconcile_prepared,
        scope,
    }
}
/// Mint a separately signed fixture permission for the exact retained transfer
/// request, open actual personal/business kernels, consume the one-shot native
/// pack import once and close both. Mirrors the promotion `invoke` pattern.
fn invoke_transfer(
    project: &RootStore,
    service: &RootStore,
    prepared: &swarm_engine::PaeaGitTransferCall,
    scope: &Scope,
    trust_revision: i64,
) -> Result<swarm_engine::PaeaGitTransferReceipt, RootStoreError> {
    let f = fixture::Fixture::build_for_trust(
        prepared.request().permission(),
        scope.clone(),
        trust_revision,
    );
    let (app, head) =
        authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin).unwrap();
    let result = service
        .call_paea_git_transfer(
            project,
            prepared,
            &app,
            &UseContext::new(),
            &head,
            b"STARTUP_FIXTURE_PASSWORD",
        )
        .map_err(|e| match e {
            PaeaGitTransferCallError::Action(error) => error,
            other => panic!("unexpected authority refusal: {}", other.code()),
        });
    app.close().unwrap();
    f.close();
    result
}
/// Mint a separately signed fixture permission for this exact retained request,
/// open actual personal/business kernels, consume once and close both.
fn invoke(
    owned: &Owned,
    prepared: &swarm_engine::PaeaGitPromoteCall,
) -> Result<PaeaGitPromoteOutcome, RootStoreError> {
    let f = fixture::Fixture::build_for_trust(
        prepared.request().permission(),
        owned.scope.clone(),
        owned.trust_revision,
    );
    let (app, head) =
        authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin).unwrap();
    let result = owned
        .service
        .call_paea_git_promote(
            &owned.project,
            prepared,
            &app,
            &UseContext::new(),
            &head,
            b"STARTUP_FIXTURE_PASSWORD",
        )
        .map_err(|e| match e {
            PaeaGitPromoteCallError::Action(error) => error,
            other => panic!("unexpected authority refusal: {}", other.code()),
        });
    app.close().unwrap();
    f.close();
    result
}
/// Real authority consumes one exact commit: the durable intent is reserved,
/// the reference is created at the candidate under OLD fencing and the effect
/// row is recorded. The exact permission spelling is the GitPromotion commit one.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn promote_call_current_authority_commits_once_and_records_effect() {
    let owned = owned();
    let outcome = invoke(&owned, &owned.commit_prepared).unwrap();
    let PaeaGitPromoteOutcome::Recorded {
        intent_id,
        action_digest,
        target_ref,
        new_oid,
        replayed,
    } = &outcome
    else {
        panic!("expected recorded outcome, got {outcome:?}");
    };
    assert!(!replayed);
    assert_eq!(intent_id.len(), 64);
    assert_eq!(action_digest, owned.commit_prepared.canonical().digest());
    assert_eq!(target_ref, "refs/heads/release");
    assert_eq!(new_oid, &owned.candidate);
    let permission = owned.commit_prepared.request().permission();
    assert_eq!(permission.operation, "effect.execute");
    assert!(permission.resource.starts_with("paea-action-v1/"));
    assert!(permission.resource.contains("/repository.promote/commit/"));
    // The actual bare reference now names the candidate.
    let refs = Command::new(git_executable())
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
    assert_eq!(
        String::from_utf8(refs.stdout).unwrap().trim(),
        &owned.candidate
    );
    owned.commit_prepared.close().unwrap();
    owned.reconcile_prepared.close().unwrap();
    owned.service.close().unwrap();
    owned.project.close().unwrap();
}
/// A second identical commit replays the recorded effect without any new
/// publication; the reconciliation path returns the same durable observation.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn promote_call_replays_recorded_effect_and_reconciles() {
    let owned = owned();
    let first = invoke(&owned, &owned.commit_prepared).unwrap();
    assert!(matches!(first, PaeaGitPromoteOutcome::Recorded { replayed: false, .. }));
    let replay = invoke(&owned, &owned.commit_prepared).unwrap();
    assert!(matches!(replay, PaeaGitPromoteOutcome::Recorded { replayed: true, .. }));
    let reconciled = invoke(&owned, &owned.reconcile_prepared).unwrap();
    assert!(matches!(reconciled, PaeaGitPromoteOutcome::Recorded { replayed: true, .. }));
    owned.commit_prepared.close().unwrap();
    owned.reconcile_prepared.close().unwrap();
    owned.service.close().unwrap();
    owned.project.close().unwrap();
}
/// Reconciliation before any commit observes no durable effect: an explicit
/// negative result, never an empty success or a Git process.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn promote_call_reconcile_before_commit_reports_not_committed() {
    let owned = owned();
    let outcome = invoke(&owned, &owned.reconcile_prepared).unwrap();
    assert_eq!(
        outcome,
        PaeaGitPromoteOutcome::ReconciledNotCommitted {
            intent_id: owned.reconcile_prepared.canonical().intent_id().to_owned(),
            target_ref: "refs/heads/release".into(),
        }
    );
    owned.commit_prepared.close().unwrap();
    owned.reconcile_prepared.close().unwrap();
    owned.service.close().unwrap();
    owned.project.close().unwrap();
}
/// A wrong-root permission cannot consume this prepared call: the signed grant
/// binds the physical witness digest of the preparing pair.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn promote_call_refuses_other_service_root_without_effect() {
    let owned = owned();
    let f = fixture::Fixture::build_for_trust(
        owned.commit_prepared.request().permission(),
        owned.scope.clone(),
        1,
    );
    let (app, head) =
        authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin).unwrap();
    let (_wrong_dir, wrong) = service();
    wrong
        .paea_initialize_git_intents("fixture-project", PIN)
        .unwrap();
    wrong
        .paea_initialize_git_promotions("fixture-project", PIN)
        .unwrap();
    let error = wrong
        .call_paea_git_promote(
            &owned.project,
            &owned.commit_prepared,
            &app,
            &UseContext::new(),
            &head,
            b"STARTUP_FIXTURE_PASSWORD",
        )
        .map_err(|e| match e {
            PaeaGitPromoteCallError::Action(error) => error,
            other => panic!("unexpected authority refusal: {}", other.code()),
        })
        .unwrap_err();
    assert!(matches!(
        error,
        RootStoreError::Refused("paea-git-promote-root-binding")
    ));
    wrong.close().unwrap();
    app.close().unwrap();
    f.close();
    owned.commit_prepared.close().unwrap();
    owned.reconcile_prepared.close().unwrap();
    owned.service.close().unwrap();
    owned.project.close().unwrap();
}
