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
/// Fresh native service storage under the governed repository fixture area;
/// never open a user mission as a fixture.
fn service() -> (tempfile::TempDir, RootStore) {
    let dir = tempfile::tempdir_in(swarm_engine::fixture_area()).unwrap();
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
/// for local fixture identities, never enrolled ones. Idempotent on both
/// regimes (same contract as the CLI harness and `effect_config_agreement`):
/// a fresh root (no `trust-actions.json` yet) applies at `expected_revision:0`
/// with a stable event id; a retained root already configured applies a
/// run-unique event per harness call at its actual current revision, so reruns
/// never replay a conflicting fingerprint nor advance the fixture Trust
/// revision. The identity set is self-contained — including the I3
/// `agreement-admin` bearer `seed_agreement` requires — instead of relying on
/// residual fixture state, and replaces the whole identity set on apply
/// (`config::change` semantics).
fn effect_config(root: &Path) -> Vec<u8> {
    let fresh = !root.join(".swarm/trust-actions.json").exists();
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nonce = format!(
        "{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    );
    let (event_id, expected) = if fresh {
        ("promote-effect-config".to_owned(), 0)
    } else {
        (format!("promote-effect-config-{nonce}"), current_trust_revision(root))
    };
    serde_json::to_vec(&json!({
        "schema_version":1,
        "event_id":event_id,
        "expected_revision":expected,
        "reason":"Explicit fixture effect role for Git promotion",
        "policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":false},
        "identities":[
            {"subject":"enrolled-effect","integrity":3,"roles":["admin","effect"],"enrollment_principal":{"principal_id":"operator","principal_kind":"human","issuer":"https://fixture.invalid","subject":"stable-person"}},
            {"subject":"requester","token":"fixture-requester-token","integrity":1,"roles":["requester"]},
            {"subject":"approver","token":"fixture-approver-token","integrity":2,"roles":["approver"]},
            {"subject":"agreement-admin","token":"fixture-admin-token","integrity":3,"roles":["admin"]}
        ]
    }))
    .unwrap()
}
/// Agreement variant of the promote fixture configuration: same identities
/// plus one bearer I3 administrator on the isolated fixture (labeled auth
/// simulated) so the real mandate-create/approve producers can run for the
/// last-use freshness checks. Applied at initial configuration time, so the
/// prepared calls bind the actual (single) post-configuration revision.
fn effect_config_agreement(root: &Path) -> Vec<u8> {
    // Idempotent on both regimes: a fresh root (no state file) applies at
    // expected revision 0 with a stable event id; a retained root already
    // configured applies a run-unique event at its actual current revision.
    let fresh = !root.join(".swarm/trust-actions.json").exists();
    let (event_id, expected) = if fresh {
        ("promote-effect-config-agreement".to_owned(), 0)
    } else {
        (
            format!("promote-effect-config-agreement-{}", std::process::id()),
            current_trust_revision(root),
        )
    };
    serde_json::to_vec(&json!({
        "schema_version":1,
        "event_id":event_id,
        "expected_revision":expected,
        "reason":"Explicit fixture effect role and agreement admin for Git promotion",
        "policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":false},
        "identities":[
            {"subject":"enrolled-effect","integrity":3,"roles":["admin","effect"],"enrollment_principal":{"principal_id":"operator","principal_kind":"human","issuer":"https://fixture.invalid","subject":"stable-person"}},
            {"subject":"requester","token":"fixture-requester-token","integrity":1,"roles":["requester"]},
            {"subject":"approver","token":"fixture-approver-token","integrity":2,"roles":["approver"]},
            {"subject":"agreement-admin","token":"fixture-admin-token","integrity":3,"roles":["admin"]}
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
    /// The seeded OLD OID: the exact promotion recipe beneficiary.
    old: String,
    trust_revision: i64,
    transfer_receipt: swarm_engine::PaeaGitTransferReceipt,
    commit_prepared: swarm_engine::PaeaGitPromoteCall,
    reconcile_prepared: swarm_engine::PaeaGitPromoteCall,
    scope: Scope,
}
fn owned() -> Owned {
    owned_with(effect_config)
}
/// Owned fixture set with an explicit fixture configuration builder, so a
/// consumer test can add isolated identities (e.g. a bearer agreement admin)
/// at initial application time instead of advancing the revision afterwards.
fn owned_with(config: fn(&Path) -> Vec<u8>) -> Owned {
    let (project_path, project, work, candidate) = project();
    project
        .trust("config-apply", &config(&project_path), "")
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
        old,
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
/// The exact agreement recipe resource for this fixture's commit proposal:
/// `{project}/{repository}/{target_ref}/{candidate}`, matching the P2 recipe
/// the captured-agreement guard consults.
fn promotion_resource(owned: &Owned) -> String {
    let Action::Git {
        repository_id,
        candidate_oid,
        ..
    } = &owned.commit_prepared.proposal().action
    else {
        panic!("git proposal expected");
    };
    format!("fixture-project/{repository_id}/refs/heads/release/{candidate_oid}")
}
/// Seed one fresh compliant agreement (mandate-create → request → approve)
/// through the real Trust operations, for the exact promotion resource and the
/// seeded OLD beneficiary — the same producer chain the captured-agreement
/// guard consults. All event/mandate identifiers derive from `prefix` plus the
/// run nonce, so chains in one run and reruns on a retained root never collide
/// with prior evidence. Returns the approved action reply.
fn seed_agreement(owned: &Owned, prefix: &str, resource: &str) -> Value {
    let nonce = std::process::id();
    let event = move |name: &str| format!("{name}-{nonce}-{prefix}");
    let mandate_id = event("mandate");
    let trust_as = |op: &str, value: Value, token: &str| -> Value {
        serde_json::from_slice(
            &owned
                .project
                .trust(op, &serde_json::to_vec(&value).unwrap(), token)
                .unwrap_or_else(|e| panic!("{op} refused: {}", e.code()))
                .bytes,
        )
        .unwrap()
    };
    trust_as(
        "mandate-create",
        json!({
            "schema_version":1,
            "event_id":event("mandate-create"),
            "mandate_id":mandate_id,
            "subject":"requester",
            "actions":["settle"],
            "resource":resource,
            "max_amount_minor":1,
            "currency":"OID",
            "beneficiary":owned.old.clone(),
        }),
        "fixture-admin-token",
    );
    let action = trust_as(
        "request",
        json!({
            "schema_version":1,
            "event_id":event("request"),
            "mandate_id":mandate_id,
            "resource":resource,
            "amount_minor":1,
            "currency":"OID",
            "beneficiary":owned.old.clone(),
            "idempotency_key":event("key"),
        }),
        "fixture-requester-token",
    );
    assert_eq!(action["state"], "pending_approval");
    let approved = trust_as(
        "approve",
        json!({
            "schema_version":1,
            "event_id":event("approve"),
            "action_id":action["id"],
            "mandate_id":mandate_id,
            "content_digest":action["content_digest"],
            "resource":resource,
            "amount_minor":1,
            "currency":"OID",
            "beneficiary":owned.old.clone(),
        }),
        "fixture-approver-token",
    );
    assert_eq!(approved["state"], "approved");
    approved
}
/// Real authority consumes one exact commit: the durable intent is reserved,
/// the reference is created at the candidate under OLD fencing and the effect
/// row is recorded. The exact permission spelling is the GitPromotion commit one.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn promote_call_current_authority_commits_once_and_records_effect() {
    let owned = owned();
    // The commit path revalidates the captured agreement at last use; a valid
    // one must exist for the publication to be accepted.
    seed_agreement(&owned, "commit-once", &promotion_resource(&owned));
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
    // The commit path revalidates the captured agreement at last use; a valid
    // one must exist for the publication to be accepted.
    seed_agreement(&owned, "replay", &promotion_resource(&owned));
    let first = invoke(&owned, &owned.commit_prepared).unwrap();
    assert!(matches!(first, PaeaGitPromoteOutcome::Recorded { replayed: false, .. }));
    let replay = invoke(&owned, &owned.commit_prepared).unwrap();
    assert!(matches!(replay, PaeaGitPromoteOutcome::Recorded { replayed: true, .. }));
    let reconciled = invoke(&owned, &owned.reconcile_prepared).unwrap();
    let PaeaGitPromoteOutcome::Recorded { intent_id, action_digest, target_ref, new_oid, .. } = &first else {
        panic!("valid control must retain its recorded effect");
    };
    assert_eq!(reconciled, PaeaGitPromoteOutcome::Reconciled {
        intent_id: intent_id.clone(),
        target_ref: target_ref.clone(),
        observed_oid: Some(new_oid.clone()),
        recorded_effect: Some(swarm_engine::PaeaGitPromoteReconciledEffect {
            action_digest: action_digest.clone(),
            target_ref: target_ref.clone(),
            new_oid: new_oid.clone(),
        }),
    });
    owned.commit_prepared.close().unwrap();
    owned.reconcile_prepared.close().unwrap();
    owned.service.close().unwrap();
    owned.project.close().unwrap();
}
/// Reconciliation before commitment reads the actual seeded OLD reference and
/// no journal effect. Missing history never proves that no past effect occurred;
/// the read launches Git without publishing or reserving a new intention.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn promote_call_reconcile_before_commit_observes_ref_without_history() {
    let owned = owned();
    let outcome = invoke(&owned, &owned.reconcile_prepared).unwrap();
    assert_eq!(
        outcome,
        PaeaGitPromoteOutcome::Reconciled {
            intent_id: owned.reconcile_prepared.canonical().intent_id().to_owned(),
            target_ref: "refs/heads/release".into(),
            observed_oid: Some(owned.old.clone()),
            recorded_effect: None,
        }
    );
    owned.commit_prepared.close().unwrap();
    owned.reconcile_prepared.close().unwrap();
    owned.service.close().unwrap();
    owned.project.close().unwrap();
}
/// Last-use agreement freshness at the engine consumption point: after a
/// valid agreement is captured (the web captured-agreement guard would pass —
/// its lookup succeeds here too), a public mandate revocation run BEFORE the
/// engine consumption must refuse the commit without any Git publication or
/// durable effect, even though the Trust revision and policy digest are
/// unchanged. After re-admission (fresh mandate, request and exact approval
/// re-captured) the same action succeeds exactly once. No sleeps: the
/// revocation order completes before the commit starts, so no timing is
/// relied on; the commit itself holds the shared Trust reservation, which
/// excludes a concurrent revocation by construction. Auth simulated on the
/// isolated fixture (labeled bearers); no human presence or acceptance.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn promote_call_refuses_mandate_revoked_after_agreement_then_readmits_once() {
    let owned = owned_with(effect_config_agreement);
    let resource = promotion_resource(&owned);
    let candidate = owned.candidate.clone();
    // Run-unique event/mandate/keys: the retained fixture root keeps its Trust
    // state across runs, so reruns must never collide with prior evidence.
    let nonce = std::process::id();
    let event = move |name: &str| format!("{name}-{nonce}");
    // Fresh compliant agreement (mandate, request, exact approve) — the same
    // producer chain the web captured-agreement guard consults at lookup.
    let trust_as = |op: &str, value: Value, token: &str| -> Value {
        serde_json::from_slice(
            &owned
                .project
                .trust(op, &serde_json::to_vec(&value).unwrap(), token)
                .unwrap_or_else(|e| panic!("{op} refused: {}", e.code()))
                .bytes,
        )
        .unwrap()
    };
    seed_agreement(&owned, "last-use", &resource);
    // Valid control: the exact canonical bytes match the captured agreement
    // now (the web lookup would succeed at this point).
    assert!(
        owned
            .project
            .paea_git_agreement_match(owned.commit_prepared.canonical().bytes())
            .unwrap()
            .is_some(),
        "fresh agreement must match before revocation"
    );
    // Public revocation AFTER the successful agreement lookup, BEFORE the
    // engine consumption: no revision change, evidence immutable.
    let revision_before = current_trust_revision(&owned.project_path);
    let revoked = trust_as(
        "mandate-revoke",
        json!({"schema_version":1,"event_id":event("last-use-revoke"),"mandate_id":format!("mandate-{nonce}-last-use")}),
        "fixture-admin-token",
    );
    assert_eq!(revoked["state"], "revoked");
    assert_eq!(
        current_trust_revision(&owned.project_path),
        revision_before,
        "revocation must not change the configuration revision"
    );
    // The commit must now refuse: the agreement is void at last use.
    let error = invoke(&owned, &owned.commit_prepared).unwrap_err();
    assert!(
        matches!(error, RootStoreError::Refused("paea-git-promote-agreement-stale")),
        "revoked-mandate commit must refuse with the typed stale-agreement code, got {:?}",
        error
    );
    // No publication and no durable effect happened.
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
    assert_ne!(
        String::from_utf8(refs.stdout).unwrap().trim(),
        &candidate,
        "refused commit must not publish the candidate"
    );
    // Re-admission with a fresh agreement: the same action succeeds once.
    seed_agreement(&owned, "last-use-2", &resource);
    let outcome = invoke(&owned, &owned.commit_prepared).unwrap();
    let PaeaGitPromoteOutcome::Recorded {
        target_ref, new_oid, replayed, ..
    } = &outcome
    else {
        panic!("re-admitted commit expected recorded outcome, got {outcome:?}");
    };
    assert!(!replayed);
    assert_eq!(target_ref, "refs/heads/release");
    assert_eq!(new_oid, &candidate);
    // A second identical commit replays the same effect (exactly once).
    let replay = invoke(&owned, &owned.commit_prepared).unwrap();
    assert!(matches!(
        replay,
        PaeaGitPromoteOutcome::Recorded { replayed: true, .. }
    ));
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
/// Fixture-only barriers observe the actual public Commit/revocation paths.
/// No controller-owned shared lock may mask a missing production reservation.
#[cfg(feature = "test-fixtures")]
mod concurrent_withdrawal {
    use super::*;
    use std::{
        error::Error,
        io,
        os::unix::fs::MetadataExt,
        panic::{catch_unwind, AssertUnwindSafe},
        sync::{
            atomic::{AtomicBool, Ordering},
            mpsc, Arc, Barrier,
        },
    };
    use swarm_engine::paea_git_fixture::{with_ex_lock_hook, with_shared_section_hook, ScopeError};
    type Cause = Box<dyn Error + Send + Sync>;
    type Panic = Box<dyn std::any::Any + Send>;
    /// Controller-owned original authority fixture and every independent result.
    /// Retention is enabled before startup; both original journals close even
    /// when startup/use fails or panics. No failure implies an absent effect.
    struct Call {
        fixture: fixture::Fixture,
        result: Option<std::thread::Result<Result<PaeaGitPromoteOutcome, PaeaGitPromoteCallError>>>,
        startup_error: Option<Cause>,
        startup_panic: Option<Panic>,
        cleanup: Vec<Cause>,
    }
    impl Call {
        /// Borrow the call only after all startup/use/close outcomes are known;
        /// assertions never consume the owner or replace a retained cause.
        fn checked(&self) -> Result<&PaeaGitPromoteOutcome, &PaeaGitPromoteCallError> {
            assert!(
                self.startup_error.is_none() && self.startup_panic.is_none(),
                "authority startup failed; original retained"
            );
            assert!(
                self.cleanup.is_empty(),
                "authority cleanup failed; original retained"
            );
            self.result
                .as_ref()
                .expect("authority use absent")
                .as_ref()
                .unwrap_or_else(|_| panic!("authority body panicked; original retained"))
                .as_ref()
        }
    }
    /// One failure payload keeps the original assertion/panic and independently
    /// owned typed worker, authority and cleanup results until propagation.
    struct RetainedFailure {
        primary: Option<Panic>,
        parts: Vec<Box<dyn std::any::Any + Send>>,
    }
    /// Retain safe typed diagnostic branches without private Display text.
    /// Cyclic Error graphs are deduplicated by identity, never by a retry cap.
    fn safe_cause(error: &(dyn Error + 'static)) -> Value {
        let mut pending = vec![error];
        let mut seen: Vec<*const dyn Error> = Vec::new();
        let mut nodes = Vec::new();
        while let Some(error) = pending.pop() {
            let identity = error as *const dyn Error;
            if seen.iter().any(|prior| std::ptr::eq(*prior, identity)) {
                continue;
            }
            seen.push(identity);
            let links = swarm_engine::native_error_links(error);
            nodes.push(json!({"labels":links.labels,"io_errno":links.io_errno,"sqlite_code":links.sqlite_code}));
            pending.extend(links.children.into_iter().map(|(_, cause)| cause));
        }
        Value::Array(nodes)
    }
    /// Project safe diagnostics while the controller still owns all typed
    /// results. The projection never replaces the original failure bundle.
    fn describe_call(call: &Call) {
        let startup = call
            .startup_error
            .as_ref()
            .map(|cause| safe_cause(cause.as_ref()));
        let cleanup: Vec<_> = call
            .cleanup
            .iter()
            .map(|cause| safe_cause(cause.as_ref()))
            .collect();
        let action = match &call.result {
            Some(Ok(Err(cause))) => Some(safe_cause(cause)),
            _ => None,
        };
        println!(
            "P4_CALL_DIAGNOSTIC {}",
            json!({"startup":startup,"startup_panic":call.startup_panic.is_some(),"action":action,"cleanup":cleanup,"use_panic":matches!(call.result,Some(Err(_)))})
        );
    }
    /// Preserve every failure before raising one payload, never replacing an
    /// earlier operation/panic with a later cleanup assertion.
    fn propagate(primary: Option<Panic>, parts: Vec<Box<dyn std::any::Any + Send>>) -> ! {
        std::panic::panic_any(RetainedFailure { primary, parts })
    }

    /// Invoke the exact signed permission and retain its authority fixture.
    /// Outcome and authority-close failure remain independent, including after
    /// publication; this helper never deletes a grant or infers rollback.
    fn retained_call(owned: &Owned, prepared: &swarm_engine::PaeaGitPromoteCall) -> Call {
        let mut f = fixture::Fixture::build_for_trust(
            prepared.request().permission(),
            owned.scope.clone(),
            owned.trust_revision,
        );
        f.retain_directory();
        println!("P4_AUTHORITY_FIXTURE {}", f.path.display());
        let startup = catch_unwind(AssertUnwindSafe(|| {
            authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin)
        }));
        let mut call = Call {
            fixture: f,
            result: None,
            startup_error: None,
            startup_panic: None,
            cleanup: Vec::new(),
        };
        match startup {
            Ok(Ok((app, head))) => {
                call.result = Some(catch_unwind(AssertUnwindSafe(|| {
                    owned.service.call_paea_git_promote(
                        &owned.project,
                        prepared,
                        &app,
                        &UseContext::new(),
                        &head,
                        b"STARTUP_FIXTURE_PASSWORD",
                    )
                })));
                if let Err(error) = app.close() {
                    call.cleanup.push(Box::new(error));
                }
            }
            Ok(Err(error)) => call.startup_error = Some(Box::new(error)),
            Err(panic) => call.startup_panic = Some(panic),
        }
        if let Err(error) = call.fixture.business.close() {
            call.cleanup.push(Box::new(error));
        }
        if let Err(error) = call.fixture.personal.close() {
            call.cleanup.push(Box::new(error));
        }
        call
    }

    /// Close every prepared operation and store, retaining each independent
    /// cleanup cause. The service directory was retained before thread entry.
    fn close_owned(owned: Owned) -> Vec<Cause> {
        let mut errors = Vec::new();
        for result in [
            owned.commit_prepared.close(),
            owned.reconcile_prepared.close(),
            owned.service.close(),
            owned.project.close(),
        ] {
            if let Err(error) = result {
                errors.push(Box::new(error) as Cause);
            }
        }
        errors
    }

    /// Read the current real Git reference; never replace a command failure
    /// or malformed output with absence.
    fn current_ref(owned: &Owned) -> String {
        let out = Command::new(git_executable())
            .arg("--git-dir")
            .arg(owned._service_dir.path().join(".swarm/bare"))
            .args(["rev-parse", "--verify", "refs/heads/release"])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "Git ref observation failed: {:?}",
            out.status
        );
        String::from_utf8(out.stdout).unwrap().trim().to_owned()
    }

    /// Stable fixture diagnostic; no secrets or dynamic error text.
    fn stopped(reason: &'static str) -> io::Error {
        io::Error::new(io::ErrorKind::Other, reason)
    }

    /// Events are progress only. Typed operation/close outcomes return through
    /// joined workers; their errors are not replaced by a successful signal.
    enum Event {
        Shared,
        Probe { dev: u64, ino: u64, blocked: bool },
        CommitDone,
        RevokeDone,
    }

    /// Require the dedicated positive helper's actual successful child exit,
    /// immutable result, originals and retained observations before any test
    /// service is created. A visible PASS or publication observation alone is
    /// not qualification; no old fixture loader or receipt is silently adopted.
    fn require_qualified_source() {
        use sha2::{Digest, Sha256};
        let proof = PathBuf::from(
            std::env::var_os("SWARM_PAEA_SOURCE_FIXTURE")
                .expect("new qualified source evidence required"),
        );
        assert!(
            !proof.join("failure.json").exists()
                && !proof.join("qualification-failure.json").exists()
        );
        let read = |path: &Path| fs::read(path).unwrap();
        let hash = |raw: &[u8]| format!("{:x}", Sha256::digest(raw));
        let terminal: Value = serde_json::from_slice(&read(&proof.join("terminal.json"))).unwrap();
        assert_eq!(terminal["status"], "QUALIFIED_P4_FRESH_SOURCE");
        assert_eq!(terminal["exit"], 0);
        let process = read(&proof.join("process.json"));
        assert_eq!(terminal["process_sha256"], hash(&process));
        let process: Value = serde_json::from_slice(&process).unwrap();
        assert_eq!(process["exit"], 0);
        let raw = read(&proof.join("result.json"));
        assert_eq!(terminal["result_sha256"], hash(&raw));
        let result: Value = serde_json::from_slice(&raw).unwrap();
        assert_eq!(result["status"], "PASS");
        assert_eq!(result["task_accepted"], true);
        assert_eq!(result["paid_provider"], false);
        assert_eq!(
            result["inputs_sha256"],
            hash(&read(&proof.join("inputs.json")))
        );
        assert_eq!(
            result["observation_sha256"],
            hash(&read(&proof.join("observed-publication.json")))
        );
        for (name, digest) in result["artifacts"].as_object().unwrap() {
            let relative = Path::new(name);
            assert!(relative
                .components()
                .all(|part| matches!(part, std::path::Component::Normal(_))));
            let path = proof.join(relative);
            assert_eq!(path.canonicalize().unwrap(), path);
            assert_eq!(digest, &Value::String(hash(&read(&path))));
        }
        let root = PathBuf::from(result["root"].as_str().unwrap())
            .canonicalize()
            .unwrap();
        for (name, digest) in result["consumed_originals"].as_object().unwrap() {
            let path = Path::new(name).canonicalize().unwrap();
            assert!(path.starts_with(&root));
            assert_eq!(digest, &Value::String(hash(&read(&path))));
        }
        let marker = Path::new(result["review_marker"]["path"].as_str().unwrap())
            .canonicalize()
            .unwrap();
        assert!(marker.starts_with(&root));
        assert_eq!(result["review_marker"]["sha256"], hash(&read(&marker)));
        let physical = fs::metadata(root.join(".swarm")).unwrap();
        assert_eq!(result["physical"]["device"], physical.dev());
        assert_eq!(result["physical"]["inode"], physical.ino());
        assert_eq!(raw, read(&proof.join("result.json")));
    }

    /// Save the whole nested EX witness before examining any registration,
    /// cleanup or public-operation failure. The outer slot survives assertions
    /// and is included in RetainedFailure, including typed probe/unlock causes.
    fn retain_ex_witness<T>(
        slot: &mut Option<Result<T, ScopeError<T>>>,
        run: impl FnOnce() -> Result<T, ScopeError<T>>,
    ) -> &T {
        *slot = Some(run());
        match slot.as_ref().unwrap() {
            Ok(body) => body,
            Err(ScopeError::RegistrationRefused) => {
                panic!("EX control registration refused; result retained")
            }
            Err(ScopeError::Cleanup { .. }) => {
                panic!("EX control cleanup panicked; body and panic retained")
            }
        }
    }

    /// A successful signed Commit control precedes injection on the same
    /// composition and fresh service prerequisites. Then observe real EX
    /// contention during Commit's SH section and persistent public revocation.
    /// No sleeps, receive deadlines or global watchdog establish ordering.
    #[test]
    #[ignore = "requires newly qualified native source fixture and test-fixtures profile"]
    fn promote_call_concurrent_withdrawal_waits_for_shared_section() {
        require_qualified_source();
        let mut control = owned_with(effect_config_agreement);
        control._service_dir.disable_cleanup(true);
        println!(
            "P4_CONTROL_SERVICE {}",
            control._service_dir.path().display()
        );
        let mut control_call = None;
        let mut control_ex = None;
        let control_check = catch_unwind(AssertUnwindSafe(|| {
            seed_agreement(&control, "p4-valid-control", &promotion_resource(&control));
            control_call = Some(retained_call(&control, &control.commit_prepared));
            describe_call(control_call.as_ref().unwrap());
            let outcome = control_call.as_ref().unwrap().checked();
            assert!(
                matches!(
                    outcome,
                    Ok(PaeaGitPromoteOutcome::Recorded {
                        replayed: false,
                        ..
                    })
                ),
                "valid Commit control failed; original retained"
            );
            assert_eq!(current_ref(&control), control.candidate);
            // Positive probe control on the exact exclusive path outside a
            // shared section: acquisition and explicit unlock must succeed.
            let probed = Arc::new(AtomicBool::new(false));
            let seen = probed.clone();
            let withdrawn = retain_ex_witness(&mut control_ex, || {
                with_ex_lock_hook(
                    move |file| {
                        rustix::fs::flock(
                            file,
                            rustix::fs::FlockOperation::NonBlockingLockExclusive,
                        )
                        .map_err(io::Error::from)?;
                        rustix::fs::flock(file, rustix::fs::FlockOperation::Unlock)
                            .map_err(io::Error::from)?;
                        seen.store(true, Ordering::SeqCst);
                        Ok(())
                    },
                    || {
                        control.project.trust("mandate-revoke", &serde_json::to_vec(&json!({"schema_version":1,"event_id":format!("p4-control-revoke-{}",std::process::id()),"mandate_id":format!("mandate-{}-p4-valid-control",std::process::id())})).unwrap(), "fixture-admin-token")
                    },
                )
            });
            let withdrawn = withdrawn
                .as_ref()
                .unwrap_or_else(|_| panic!("EX public control refused; typed cause retained"));
            let withdrawn: Value = serde_json::from_slice(&withdrawn.bytes).unwrap();
            assert_eq!(withdrawn["state"], "revoked");
            assert!(probed.load(Ordering::SeqCst));
        }));
        let control_close = close_owned(control);
        if control_check.is_err() || !control_close.is_empty() {
            propagate(
                control_check.err(),
                vec![
                    Box::new(control_call),
                    Box::new(control_ex),
                    Box::new(control_close),
                ],
            );
        }
        println!("P4_VALID_COMMIT_CONTROL_PASS");

        let mut owned = owned_with(effect_config_agreement);
        owned._service_dir.disable_cleanup(true);
        println!(
            "P4_CONCURRENT_SERVICE {}",
            owned._service_dir.path().display()
        );
        let prefix = "p4-concurrent";
        seed_agreement(&owned, prefix, &promotion_resource(&owned));
        let mandate = format!("mandate-{}-{prefix}", std::process::id());
        let trust_path = owned.project_path.join(".swarm/trust-actions.json");
        let before = fs::read(&trust_path).unwrap();
        let lock_meta = fs::metadata(owned.project_path.join(".swarm/trust-actions.lock")).unwrap();
        let expected_inode = (lock_meta.dev(), lock_meta.ino());
        // Open before Commit's writer reservation, avoiding an unrelated
        // SQL preflight wait in the revocation worker.
        let revoke_root = RootStore::open(&owned.project_path, true).unwrap();
        let release_commit = Arc::new(Barrier::new(2));
        let release_probe = Arc::new(Barrier::new(2));
        let abort_commit = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        let (start_tx, start_rx) = mpsc::channel();

        let revoke_tx = tx.clone();
        let probe_gate = release_probe.clone();
        let revoker = std::thread::spawn(move || {
            let result = catch_unwind(AssertUnwindSafe(|| {
                if start_rx
                    .recv()
                    .map_err(|_| stopped("controller start lost"))?
                    == false
                {
                    return Ok(None);
                }
                let probe_tx = revoke_tx.clone();
                let hook = move |file: &std::fs::File| -> io::Result<()> {
                    let metadata = file.metadata()?;
                    let (dev, ino) = (metadata.dev(), metadata.ino());
                    match rustix::fs::flock(
                        file,
                        rustix::fs::FlockOperation::NonBlockingLockExclusive,
                    ) {
                        Err(rustix::io::Errno::AGAIN) => {
                            probe_tx
                                .send(Event::Probe {
                                    dev,
                                    ino,
                                    blocked: true,
                                })
                                .map_err(|_| stopped("controller probe lost"))?;
                            probe_gate.wait();
                            Ok(())
                        }
                        Ok(()) => {
                            // Release an unexpected acquisition before reporting
                            // the missing shared reservation; retain unlock errno.
                            rustix::fs::flock(file, rustix::fs::FlockOperation::Unlock)
                                .map_err(io::Error::from)?;
                            probe_tx
                                .send(Event::Probe {
                                    dev,
                                    ino,
                                    blocked: false,
                                })
                                .map_err(|_| stopped("controller probe lost"))?;
                            Err(stopped("exclusive probe unexpectedly acquired"))
                        }
                        Err(errno) => Err(io::Error::from(errno)),
                    }
                };
                let nested = with_ex_lock_hook(hook, || {
                    revoke_root.trust("mandate-revoke", &serde_json::to_vec(&json!({"schema_version":1,"event_id":format!("p4-revoke-{}",std::process::id()),"mandate_id":mandate})).unwrap(), "fixture-admin-token")
                });
                Ok::<_, Cause>(Some(nested))
            }));
            let closed = revoke_root.close();
            let terminal = revoke_tx.send(Event::RevokeDone);
            (result, closed, terminal)
        });
        let commit_tx = tx.clone();
        let commit_gate = release_commit.clone();
        let commit_abort = abort_commit.clone();
        let commit = std::thread::spawn(move || {
            let progress = commit_tx.clone();
            let result = catch_unwind(AssertUnwindSafe(|| {
                with_shared_section_hook(
                    move || {
                        progress
                            .send(Event::Shared)
                            .map_err(|_| stopped("controller shared point lost"))?;
                        commit_gate.wait();
                        if commit_abort.load(Ordering::SeqCst) {
                            Err(stopped("controller aborted Commit"))
                        } else {
                            Ok(())
                        }
                    },
                    || retained_call(&owned, &owned.commit_prepared),
                )
            }));
            let terminal = commit_tx.send(Event::CommitDone);
            (owned, result, terminal)
        });
        // No controller-held sender can keep a failed worker's channel alive.
        drop(tx);
        let first = rx.recv();
        let held = matches!(first, Ok(Event::Shared));
        let start = start_tx.send(held);
        drop(start_tx);
        let mut commit_done = matches!(first, Ok(Event::CommitDone));
        let mut revoke_done = false;
        let mut witness = None;
        let mut held_bytes = None;
        let mut channel_failure = first.is_err();
        if held {
            loop {
                match rx.recv() {
                    Ok(Event::Probe { dev, ino, blocked }) => {
                        witness = Some((dev, ino, blocked));
                        held_bytes = Some(fs::read(&trust_path));
                        if blocked {
                            release_probe.wait();
                        }
                        if !blocked {
                            abort_commit.store(true, Ordering::SeqCst);
                        }
                        break;
                    }
                    Ok(Event::RevokeDone) => {
                        revoke_done = true;
                        abort_commit.store(true, Ordering::SeqCst);
                        break;
                    }
                    Ok(Event::CommitDone) => {
                        commit_done = true;
                        break;
                    }
                    Ok(Event::Shared) => {
                        abort_commit.store(true, Ordering::SeqCst);
                    }
                    Err(_) => {
                        channel_failure = true;
                        abort_commit.store(true, Ordering::SeqCst);
                        break;
                    }
                }
            }
            release_commit.wait();
        }
        while !commit_done || !revoke_done {
            match rx.recv() {
                Ok(Event::CommitDone) => commit_done = true,
                Ok(Event::RevokeDone) => revoke_done = true,
                Ok(_) => channel_failure = true,
                Err(_) => {
                    channel_failure = true;
                    break;
                }
            }
        }
        // Join both before assertions, even if a prerequisite or worker failed.
        let commit_joined = commit.join();
        let revoke_joined = revoker.join();
        let (owned, commit_result, commit_terminal) = match commit_joined {
            Ok(value) => value,
            Err(panic) => propagate(Some(panic), vec![Box::new(revoke_joined)]),
        };
        let mut renewed_call = None;
        let mut reconciled_call = None;
        let verification = catch_unwind(AssertUnwindSafe(|| {
            // First decode typed results, while retaining the originals in the
            // enclosing worker structures; progression failure cannot erase them.
            let scoped = commit_result
                .as_ref()
                .unwrap_or_else(|_| panic!("Commit worker body panicked; original retained"));
            let call = match scoped {
                Ok(call) => call,
                Err(ScopeError::Cleanup { outcome, .. }) => {
                    let _ = outcome;
                    panic!("Commit hook cleanup panicked; outcome retained");
                }
                Err(ScopeError::RegistrationRefused) => panic!("Commit hook registration refused"),
            };
            describe_call(call);
            let outcome = call
                .checked()
                .unwrap_or_else(|_| panic!("Commit refused; typed cause retained"));
            let (revocation, closed, terminal) = revoke_joined
                .as_ref()
                .unwrap_or_else(|_| panic!("revoker outer panic; retained"));
            assert!(
                closed.is_ok() && terminal.is_ok(),
                "revoker close/terminal failed; retained"
            );
            let reply = match revocation {
                Ok(Ok(Some(Ok(Ok(reply))))) => reply,
                _ => panic!("revocation startup/body/scope/operation failed; original retained"),
            };
            let revoked: Value = serde_json::from_slice(&reply.bytes).unwrap();
            assert!(
                start.is_ok() && !channel_failure && held && commit_done && revoke_done,
                "progress protocol failed; operation outcomes retained"
            );
            assert!(commit_terminal.is_ok(), "Commit terminal signal lost");
            assert_eq!(revoked["state"], "revoked");
            assert_eq!(witness, Some((expected_inode.0, expected_inode.1, true)));
            assert_eq!(
                held_bytes.as_ref().unwrap().as_ref().unwrap(),
                &before,
                "Trust mutated during shared section"
            );
            let PaeaGitPromoteOutcome::Recorded {
                intent_id,
                action_digest,
                target_ref,
                new_oid,
                replayed,
            } = outcome
            else {
                panic!("Commit not recorded");
            };
            assert!(!replayed);
            assert_eq!(action_digest, owned.commit_prepared.canonical().digest());
            assert_eq!(new_oid, &owned.candidate);
            assert_eq!(current_ref(&owned), owned.candidate);
            let view = owned
                .project
                .trust("show", b"{}", "fixture-admin-token")
                .unwrap();
            let view: Value = serde_json::from_slice(&view.bytes).unwrap();
            // Public show preserves the stored mandate (revoked_at), and
            // projects effective admission separately through mandate_status.
            // The revoke command's response state is not a stored field.
            let mandate_id = format!("mandate-{}-p4-concurrent", std::process::id());
            let stored = &view["mandates"][&mandate_id];
            assert_eq!(stored["id"], mandate_id);
            let revoked_at = stored["revoked_at"]
                .as_str()
                .expect("persisted revocation timestamp required");
            time::OffsetDateTime::parse(
                revoked_at,
                &time::format_description::well_known::Rfc3339,
            )
            .expect("persisted revocation must have a valid RFC3339 timestamp");
            assert_eq!(view["mandate_status"][&mandate_id], "mandat révoqué");
            assert!(owned
                .project
                .paea_git_agreement_match(owned.commit_prepared.canonical().bytes())
                .unwrap()
                .is_none());
            renewed_call = Some(retained_call(&owned, &owned.commit_prepared));
            let renewed = renewed_call.as_ref().unwrap().checked();
            assert!(
                matches!(
                    renewed,
                    Err(PaeaGitPromoteCallError::Action(RootStoreError::Refused(
                        "paea-git-promote-agreement-stale"
                    )))
                ),
                "renewal must refuse; original retained"
            );
            assert_eq!(current_ref(&owned), owned.candidate);
            reconciled_call = Some(retained_call(&owned, &owned.reconcile_prepared));
            assert_eq!(
                reconciled_call.as_ref().unwrap().checked().unwrap(),
                &PaeaGitPromoteOutcome::Reconciled {
                    intent_id: intent_id.clone(),
                    target_ref: target_ref.clone(),
                    observed_oid: Some(new_oid.clone()),
                    recorded_effect: Some(swarm_engine::PaeaGitPromoteReconciledEffect {
                        action_digest: action_digest.clone(),
                        target_ref: target_ref.clone(),
                        new_oid: new_oid.clone()
                    })
                }
            );
            println!(
                "P4_CONTENTION_WITHDRAWAL_REFUSAL_RECONCILE_PASS inode={}:{}",
                expected_inode.0, expected_inode.1
            );
        }));
        let cleanup = close_owned(owned);
        if verification.is_err() || !cleanup.is_empty() {
            propagate(
                verification.err(),
                vec![
                    Box::new(commit_result),
                    Box::new(revoke_joined),
                    Box::new(cleanup),
                    Box::new(renewed_call),
                    Box::new(reconciled_call),
                    Box::new((
                        start,
                        channel_failure,
                        held,
                        commit_done,
                        revoke_done,
                        witness,
                        held_bytes,
                        commit_terminal,
                    )),
                ],
            );
        }
    }
    /// A valid payload survives propagation together with an independent
    /// cleanup error; no cleanup assertion replaces the original panic.
    #[test]
    fn retained_failure_keeps_primary_and_cleanup_parts() {
        let captured = catch_unwind(AssertUnwindSafe(|| {
            propagate(
                Some(Box::new("primary")),
                vec![Box::new(io::Error::from_raw_os_error(13))],
            )
        }))
        .err()
        .unwrap();
        let bundle = captured.downcast::<RetainedFailure>().ok().unwrap();
        assert_eq!(
            bundle.primary.as_ref().unwrap().downcast_ref::<&str>(),
            Some(&"primary")
        );
        assert_eq!(
            bundle.parts[0]
                .downcast_ref::<io::Error>()
                .unwrap()
                .raw_os_error(),
            Some(13)
        );
    }

    /// Normal fixture cleanup is the control; retention then keeps directory
    /// bytes after both original journal closes and owner Drop, without leaks.
    #[test]
    fn authority_fixture_retention_preserves_directory_after_closes() {
        let ordinary = fixture::Fixture::new();
        let ordinary_path = ordinary.path.clone();
        let closes = [ordinary.business.close(), ordinary.personal.close()];
        assert!(closes.iter().all(Result::is_ok));
        drop(ordinary);
        assert!(!ordinary_path.exists());
        let mut retained = fixture::Fixture::new();
        retained.retain_directory();
        let path = retained.path.clone();
        let before = fs::read(&path).unwrap();
        let closes = [retained.business.close(), retained.personal.close()];
        drop(retained);
        assert!(closes.iter().all(Result::is_ok));
        assert_eq!(fs::read(path).unwrap(), before);
    }
    /// The EX witness uses a nested Result just like the public operation.
    /// Its original errno survives the assertion that propagates failure;
    /// this component guard establishes retention, not kernel contention.
    #[test]
    fn ex_witness_error_is_retained_before_assertion() {
        let mut valid = None;
        assert!(retain_ex_witness(&mut valid, || with_ex_lock_hook(
            |_| Ok(()),
            || Ok::<_, io::Error>(())
        ))
        .is_ok());
        let mut failed = None;
        let primary = catch_unwind(AssertUnwindSafe(|| {
            let body = retain_ex_witness(&mut failed, || {
                with_ex_lock_hook(
                    |_| Ok(()),
                    || Err::<(), _>(io::Error::from_raw_os_error(13)),
                )
            });
            assert!(body.is_ok(), "EX witness failed");
        }))
        .err()
        .unwrap();
        let bundle = RetainedFailure {
            primary: Some(primary),
            parts: vec![Box::new(failed)],
        };
        type Witness = Option<Result<Result<(), io::Error>, ScopeError<Result<(), io::Error>>>>;
        let original = bundle.parts[0]
            .downcast_ref::<Witness>()
            .unwrap()
            .as_ref()
            .unwrap();
        let Ok(Err(cause)) = original else {
            panic!("nested EX cause lost");
        };
        assert_eq!(cause.raw_os_error(), Some(13));
        assert!(bundle.primary.is_some());
    }
}
