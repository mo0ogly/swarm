//! Guarded protected consumption of the qualified crate-owned transfer job:
//! real signed current authority, actual provisioned declaration and native
//! preparation on owned fixtures. Fictitious effects only; no protected daemon,
//! ref effect, UID separation, human presence or accepted service is claimed.
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
    PaeaGitTransferCallError, RootStore, RootStoreError, SystemEnvironment,
};
use swarm_enrollment_authority::{
    authenticated_control::Context as UseContext, authenticated_startup,
};
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
/// Explicit actual first-publication project, writable: real managed publication
/// and work are required by the job's native source observation. This is the
/// same newly owned SWARM_PAEA_SOURCE_FIXTURE that other source recipes
/// durably advance; the native review receipts bind the real .swarm device and
/// inode, so no copy can substitute for the actual fixture root. Trust
/// configuration with an explicit effect role is applied through the public
/// operator path on this owned fixture.
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
/// Actual project Trust configuration with an explicit effect role; tokens only
/// for local fixture identities, never enrolled ones. Mirrors the settlement
/// fixture so policy refusals exercise the same real configuration surface.
/// Read the current Trust revision from the fixture's own bounded state file
/// so the applied configuration expects the actual next-revision predecessor
/// and the signed grant binds the actual post-configuration revision.
fn current_trust_revision(root: &Path) -> i64 {
    let state: Value =
        serde_json::from_slice(&fs::read(root.join(".swarm/trust-actions.json")).unwrap())
            .unwrap();
    state["revision"].as_i64().expect("fixture Trust revision")
}
fn effect_config() -> Vec<u8> {
    serde_json::to_vec(&json!({
        "schema_version":1,
        "event_id":"effect-config",
        "expected_revision":0,
        "reason":"Explicit fixture effect role for Git transfer",
        "policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":false},
        "identities":[
            {"subject":"enrolled-effect","integrity":3,"roles":["admin","effect"],"enrollment_principal":{"principal_id":"operator","principal_kind":"human","issuer":"https://fixture.invalid","subject":"stable-person"}},
            {"subject":"requester","token":"fixture-requester-token","integrity":1,"roles":["requester"]},
            {"subject":"approver","token":"fixture-approver-token","integrity":2,"roles":["approver"]}
        ]
    }))
    .unwrap()
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
/// Valid Git proposal for the service-assigned repository and actual candidate.
fn proposal(id: &str, candidate: &str) -> Proposal {
    Proposal {
        schema_version: 1,
        request_id: "transfer-fixture".into(),
        action: Action::Git {
            repository_id: id.into(),
            target_ref: "refs/heads/release".into(),
            expected_old_oid: "1".repeat(40),
            candidate_oid: candidate.into(),
        },
    }
}
/// Declared context: policy fields are declarations rechecked under held
/// authority; one declared source, never an admission evidence. The policy
/// revision binds the actual current fixture Trust revision.
fn context(trust_revision: i64) -> AdmissionContext {
    AdmissionContext {
        project_id: "fixture-project".into(),
        principal_id: "operator".into(),
        instance_id: "fixture-instance".into(),
        mandate_id: "fixture-mandate".into(),
        mandate_revision: 1,
        policy_revision: trust_revision,
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
/// Complete owned fixture set: the actual native project (writable, Trust
/// applied, plan scope read), a fresh service (journal + provisioned
/// declaration), and the exact prepared call. All owners must be explicitly
/// closed by the caller; the native project directory is an owned test fixture
/// and is never deleted here.
struct Owned {
    #[allow(dead_code)]
    project_path: PathBuf,
    project: RootStore,
    _service_dir: tempfile::TempDir,
    service: RootStore,
    prepared: swarm_engine::PaeaGitTransferCall,
    scope: Scope,
    trust_revision: i64,
}
fn owned() -> Owned {
    let (project_path, project, work, candidate) = project();
    project
        .trust("config-apply", &effect_config(), "")
        .unwrap_or_else(|e| panic!("explicit fixture config refused: {}", e.code()));
    // The applied configuration advanced the Trust revision; the signed grant
    // and attributed context must both bind the actual post-configuration one.
    let trust_revision = current_trust_revision(&project_path);
    let scope = scope(&project_path, &work);
    let (service_dir, service) = service();
    service
        .paea_initialize_git_intents("fixture-project", PIN)
        .unwrap();
    bare(service_dir.path(), "bare");
    let binding = service
        .paea_provision_git_repository(&project, &work, "bare", PIN)
        .unwrap();
    let proposal = proposal(binding.repository_id(), &candidate);
    let prepared = service
        .paea_git_transfer_call(
            &project,
            &proposal,
            &context(trust_revision),
            &work,
            "bare",
            PIN,
            "transfer-pack",
        )
        .unwrap_or_else(|e| panic!("explicit preparation refused: {:?}", e));
    Owned {
        project_path,
        project,
        _service_dir: service_dir,
        service,
        prepared,
        scope,
        trust_revision,
    }
}
/// Mint a separately signed fixture permission for this exact retained request,
/// open actual personal/business kernels, consume once and close both.
fn invoke(
    owned: &Owned,
) -> Result<swarm_engine::PaeaGitTransferReceipt, RootStoreError> {
    let f = fixture::Fixture::build_for_trust(
        owned.prepared.request().permission(),
        owned.scope.clone(),
        owned.trust_revision,
    );
    let (app, head) =
        authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin).unwrap();
    let result = owned
        .service
        .call_paea_git_transfer(
            &owned.project,
            &owned.prepared,
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
/// Real authority consumes one exact reconciliation: the native preparation
/// completes on owned fixtures and returns its frozen receipt. No ref effect
/// occurs and the exact permission spelling is the GitPromotion reconcile one.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn transfer_call_current_authority_prepares_once_and_reports_receipt() {
    let owned = owned();
    let receipt = invoke(&owned).unwrap();
    assert_eq!(receipt.candidate.len(), 40);
    assert!(receipt.pack_bytes > 0);
    assert_eq!(receipt.pack_sha256.len(), 64);
    let permission = owned.prepared.request().permission();
    assert_eq!(permission.operation, "effect.execute");
    assert!(permission.resource.starts_with("paea-action-v1/"));
    assert!(permission.resource.contains("/repository.promote/reconcile/"));
    owned.prepared.close().unwrap();
    owned.service.close().unwrap();
    owned.project.close().unwrap();
}
/// A wrong-root permission cannot consume this prepared call: the signed grant
/// binds the physical witness digest of the preparing pair.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn transfer_call_refuses_other_service_root_without_preparation() {
    let owned = owned();
    let f = fixture::Fixture::build_for_trust(
        owned.prepared.request().permission(),
        owned.scope.clone(),
        owned.trust_revision,
    );
    let (app, head) =
        authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin).unwrap();
    let (_wrong_dir, wrong) = service();
    let error = wrong
        .call_paea_git_transfer(
            &owned.project,
            &owned.prepared,
            &app,
            &UseContext::new(),
            &head,
            b"STARTUP_FIXTURE_PASSWORD",
        )
        .map_err(|e| match e {
            PaeaGitTransferCallError::Action(error) => error,
            other => panic!("unexpected authority refusal: {}", other.code()),
        })
        .unwrap_err();
    assert!(matches!(
        error,
        RootStoreError::Refused("paea-git-transfer-root-binding")
    ));
    wrong.close().unwrap();
    app.close().unwrap();
    f.close();
    owned.prepared.close().unwrap();
    owned.service.close().unwrap();
    owned.project.close().unwrap();
}
