//! PA-09 read-only reconciliation of both real effect journals: the settlement
//! ledger and the service Git promote journal, joined to their durable intent
//! reservation and re-derived by independent recomputation. Fictitious effects
//! on owned fixtures; no protected daemon, UID separation, human presence or
//! accepted service is claimed.
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
    PaeaEffectReconcileJournal, PaeaGitPromoteCallError,
    PaeaGitPromoteOutcome, PaeaGitTransferCallError, RootStore, RootStoreError,
    SystemEnvironment,
};
use swarm_enrollment_authority::{authenticated_control::Context as UseContext, authenticated_startup};
use swarm_enrollment_state::Context;
use swarm_identity_contract::Scope;
use swarm_protection_contract::{Action, AdmissionContext, Proposal, SourceRef};

const PIN: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

// --- Settlement harness (autonomous; mirrors tests/paea_settlement.rs) ---

fn root() -> (tempfile::TempDir, RootStore) {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let store = RootStore::initialize(dir.path(), &mut SystemEnvironment).unwrap();
    (dir, store)
}
fn project() -> (tempfile::TempDir, RootStore, Scope) {
    let (dir, mut store) = root();
    let work = store
        .execute_external(
            "",
            "work.create",
            br#"{"schema_version":1,"event_id":"create","expected_revision":0,"title":"Reconcile plan","objective":"Exact fictitious effect","scope":"Fixture","criteria":["No duplicate invoice"]}"#,
            &mut SystemEnvironment,
        )
        .unwrap()
        .result
        .work
        .id;
    let config = json!({
        "schema_version":1,
        "event_id":"effect-config",
        "expected_revision":0,
        "reason":"Explicit fixture effect role",
        "policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":false},
        "identities":[
            {"subject":"enrolled-effect","integrity":3,"roles":["admin","effect"],"enrollment_principal":{"principal_id":"operator","principal_kind":"human","issuer":"https://fixture.invalid","subject":"stable-person"}},
            {"subject":"requester","token":"fixture-requester-token","integrity":1,"roles":["requester"]},
            {"subject":"approver","token":"fixture-approver-token","integrity":2,"roles":["approver"]}
        ]
    });
    store
        .trust("config-apply", &serde_json::to_vec(&config).unwrap(), "")
        .unwrap();
    let scope = scope_of(dir.path(), &work);
    (dir, store, scope)
}
fn scope_of(dir: &Path, work: &str) -> Scope {
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
fn action() -> (Proposal, AdmissionContext) {
    let doc = swarm_protection_contract::decode_document(include_bytes!(
        "../../../../web/paea-examples/v1/payment-document.json"
    ))
    .unwrap();
    (doc.proposal, doc.context)
}
fn invoke_settlement(
    project: &RootStore,
    ledger: &RootStore,
    request: &swarm_engine::PaeaSettlementRequest,
    scope: &Scope,
) -> Result<swarm_engine::PaeaSettlementOutcome, swarm_engine::PaeaSettlementError> {
    let f = fixture::Fixture::build_for_trust(
        request.request().permission(),
        scope.clone(),
        1,
    );
    let (app, head) =
        authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin).unwrap();
    let result = ledger
        .call_paea_settlement(
            project,
            request,
            &app,
            &UseContext::new(),
            &head,
            b"STARTUP_FIXTURE_PASSWORD",
        )
        .map_err(|e| match e {
            swarm_engine::PaeaSettlementCallError::Action(error) => error,
            other => panic!("unexpected current authority refusal: {}", other.code()),
        });
    app.close().unwrap();
    f.close();
    result
}

// --- Git harness (mirrors tests/paea_git_promote_call.rs) ---

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
        (path, hash)
    });
    assert_eq!(
        &format!("{:x}", Sha256::digest(fs::read(path).unwrap())),
        hash,
        "fixture Git changed"
    );
    path
}
/// Actual first-publication project from the retained source fixture.
fn git_project() -> (PathBuf, RootStore, String, String) {
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
fn current_trust_revision(root: &Path) -> i64 {
    let state: Value =
        serde_json::from_slice(&fs::read(root.join(".swarm/trust-actions.json")).unwrap())
            .unwrap();
    state["revision"].as_i64().expect("fixture Trust revision")
}
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
/// Complete owned Git fixture set: actual native project, fresh service with
/// intents/promotions journals, bare child with the OLD ref and the imported
/// candidate, real qualified transfer first, both prepared calls. Mirrors the
/// promote harness exactly.
struct Owned {
    project: RootStore,
    _service_dir: tempfile::TempDir,
    service: RootStore,
    candidate: String,
    trust_revision: i64,
    commit_prepared: swarm_engine::PaeaGitPromoteCall,
    scope: Scope,
}
fn owned() -> Owned {
    let (project_path, project, work, candidate) = git_project();
    project
        .trust("config-apply", &effect_config(&project_path), "")
        .unwrap_or_else(|e| panic!("explicit fixture config refused: {}", e.code()));
    let trust_revision = current_trust_revision(&project_path);
    let scope = scope_of(&project_path, &work);
    let (service_dir, service) = root();
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
    Owned {
        project,
        _service_dir: service_dir,
        service,
        candidate,
        trust_revision,
        commit_prepared,
        scope,
    }
}
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
fn invoke_promote(
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
/// A fresh writable root with no initialized journal: its settlement report
/// must be the explicit Uninitialized state, never an empty success.
fn fresh_ledger() -> (tempfile::TempDir, RootStore) {
    root()
}
/// Corrupt one effect digest in the durable journal through an independent
/// read-write connection. Owned test fixture storage only, never a live base.
fn corrupt_effect_digest(dir: &Path, table: &str, column: &str) {
    let db = rusqlite::Connection::open(dir.join(".swarm/state.db")).unwrap();
    db.execute(
        &format!("UPDATE main.{table} SET {column} = {column} || '0'"),
        [],
    )
    .unwrap();
    db.close().unwrap();
}

/// Both real journals reconcile consistently: settlement effect bound to its
/// re-derived canonical payment and provisioned debt; Git effect bound to its
/// durable intent bytes and exact receipt columns. The uninitialized state is
/// explicit for absent journals.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn reconcile_reports_consistent_journals_and_explicit_uninitialized() {
    // --- Settlement domain (autonomous) ---
    let (_pd, project, scope) = project();
    let (_ld, ledger) = root();
    let (_sd, service) = root();
    // Before initialization the report is explicit, not an empty success.
    // The verifier itself never acts as its own ledger: two distinct roots.
    let report = service.paea_reconcile_effect_journals(&ledger).unwrap();
    assert_eq!(report.settlement, PaeaEffectReconcileJournal::Uninitialized);
    assert_eq!(report.git, PaeaEffectReconcileJournal::Uninitialized);
    let (p, c) = action();
    ledger
        .paea_initialize_settlement(&c.project_id, &"0b".repeat(32), std::slice::from_ref(&p))
        .unwrap();
    let request = project
        .paea_settlement_request(&ledger, &p, &c, swarm_enrollment_authority::paea_action::Operation::Commit)
        .unwrap();
    let first = match invoke_settlement(&project, &ledger, &request, &scope).unwrap() {
        swarm_engine::PaeaSettlementOutcome::Recorded { receipt, replayed } => {
            assert!(!replayed);
            receipt
        }
        other => panic!("expected recorded settlement, got {other:?}"),
    };
    let report = service.paea_reconcile_effect_journals(&ledger).unwrap();
    let PaeaEffectReconcileJournal::Consistent { effect_count: 1, findings } = &report.settlement
    else {
        panic!("expected consistent settlement journal, got {:?}", report.settlement);
    };
    assert_eq!(findings[0].intent_id, first.intent_id);
    assert_eq!(findings[0].action_digest, first.action_digest);
    assert!(findings[0].verified, "unexpected refusals: {:?}", findings[0].refusals);
    request.close().unwrap();

    // --- Git domain (real native fixture) ---
    let owned = owned();
    let outcome = invoke_promote(&owned, &owned.commit_prepared).unwrap();
    let PaeaGitPromoteOutcome::Recorded { intent_id, action_digest, .. } = &outcome else {
        panic!("expected recorded promotion, got {outcome:?}");
    };
    let (_fl, fresh_ledger) = fresh_ledger();
    let report = owned
        .service
        .paea_reconcile_effect_journals(&fresh_ledger)
        .unwrap();
    let PaeaEffectReconcileJournal::Consistent { effect_count: 1, findings } = &report.git else {
        panic!("expected consistent git journal, got {:?}", report.git);
    };
    assert_eq!(findings[0].intent_id, *intent_id);
    assert_eq!(findings[0].action_digest, *action_digest);
    assert!(findings[0].verified, "unexpected refusals: {:?}", findings[0].refusals);
    owned.commit_prepared.close().unwrap();
    owned.service.close().unwrap();
    owned.project.close().unwrap();
    ledger.close().unwrap();
    service.close().unwrap();
    project.close().unwrap();
}

/// The verifier never acts as its own ledger: passing the same physical root
/// as both the service and the settlement ledger is refused, because one
/// compromised store could otherwise fabricate both sides of the cross-check.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn reconcile_refuses_self_reconciliation_of_one_root() {
    let (_ld, ledger) = root();
    let err = ledger
        .paea_reconcile_effect_journals(&ledger)
        .unwrap_err();
    match err {
        RootStoreError::Refused(code) => {
            assert_eq!(code, "paea-effect-reconcile-separate-ledger-required");
        }
        other => panic!("expected separation refusal, got {other:?}"),
    }
    ledger.close().unwrap();
}

/// Corruption is detected, not repaired or hidden: an altered effect digest
/// yields an explicit unverified finding with the exact refusal codes, in both
/// domains, while verified rows in the same journal stay verified.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn reconcile_detects_altered_effect_digest_as_unverified_finding() {
    // Settlement: one recorded effect, then the durable digest is altered.
    let (_pd, project, scope) = project();
    let (ld, ledger) = root();
    let (_sd, service) = root();
    let (p, c) = action();
    ledger
        .paea_initialize_settlement(&c.project_id, &"0b".repeat(32), std::slice::from_ref(&p))
        .unwrap();
    let request = project
        .paea_settlement_request(&ledger, &p, &c, swarm_enrollment_authority::paea_action::Operation::Commit)
        .unwrap();
    let _ = invoke_settlement(&project, &ledger, &request, &scope).unwrap();
    request.close().unwrap();
    ledger.close().unwrap();
    let ledger = RootStore::open(ld.path(), true).unwrap();
    corrupt_effect_digest(ld.path(), "paea_settlement_effects", "action_digest");
    let report = service.paea_reconcile_effect_journals(&ledger).unwrap();
    let PaeaEffectReconcileJournal::Consistent { effect_count: 1, findings } = &report.settlement
    else {
        panic!("expected consistent settlement journal, got {:?}", report.settlement);
    };
    assert!(!findings[0].verified);
    assert!(findings[0]
        .refusals
        .contains(&"paea-effect-reconcile-effect-digest"));
    ledger.close().unwrap();
    service.close().unwrap();
    project.close().unwrap();

    // Git: one recorded promotion, then the durable digest is altered.
    let owned = owned();
    let _ = invoke_promote(&owned, &owned.commit_prepared).unwrap();
    owned.commit_prepared.close().unwrap();
    corrupt_effect_digest(
        owned._service_dir.path(),
        "paea_git_promote_effects",
        "action_digest",
    );
    let (_fl, fresh_ledger) = fresh_ledger();
    let report = owned
        .service
        .paea_reconcile_effect_journals(&fresh_ledger)
        .unwrap();
    let PaeaEffectReconcileJournal::Consistent { effect_count: 1, findings } = &report.git else {
        panic!("expected consistent git journal, got {:?}", report.git);
    };
    assert!(!findings[0].verified);
    assert!(findings[0]
        .refusals
        .contains(&"paea-effect-reconcile-effect-digest"));
    owned.service.close().unwrap();
    owned.project.close().unwrap();
}

/// A missing durable intent (effect row without reservation bytes) is an
/// explicit unverified finding, never an empty success.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn reconcile_reports_missing_git_intent_as_unverified() {
    let owned = owned();
    let _ = invoke_promote(&owned, &owned.commit_prepared).unwrap();
    owned.commit_prepared.close().unwrap();
    let db = rusqlite::Connection::open(owned._service_dir.path().join(".swarm/state.db")).unwrap();
    db.execute("DELETE FROM main.paea_git_intents", []).unwrap();
    db.close().unwrap();
    let (_fl, fresh_ledger) = fresh_ledger();
    let report = owned
        .service
        .paea_reconcile_effect_journals(&fresh_ledger)
        .unwrap();
    let PaeaEffectReconcileJournal::Consistent { effect_count: 1, findings } = &report.git else {
        panic!("expected consistent git journal, got {:?}", report.git);
    };
    assert!(!findings[0].verified);
    assert!(findings[0]
        .refusals
        .contains(&"paea-effect-reconcile-intent-missing"));
    owned.service.close().unwrap();
    owned.project.close().unwrap();
}
