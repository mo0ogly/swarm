//! Real served approver-consent page qualification on a REAL bound approval:
//! the same owned two-phase control protocol as the preview-bound browser
//! session (paea_git_call.rs). The Rust test freezes one exact approve command
//! through the real `prepare_approval` producer, signs both permissions into
//! the operator's enrollment, provisions REAL reader-frontier authorization
//! for the guarded presentation/preview reads (blp-policy-apply then
//! blp-object-attribute binding the EXACT canonical digest of the prepared
//! action, mirroring the approval-form fixture), binds the real native server
//! with the approval attached via `bind_git_call_with_approval`, publishes
//! server.json (including the fixture reader credential the harness types
//! into the page's ephemeral input) and drives phase markers; the Node
//! harness opens the real served /paea-consent.html in a real Chrome, types
//! the reader credential into the page's own control, asserts the exact
//! frozen-command digest through the guarded presentation, the exact prepared
//! action and the consent form (phase 1), then a wrong-bytes submission
//! visibly refuses and a correct-bytes/wrong-password submission visibly
//! refuses before any retry is possible (phase 2, real route calls made by
//! the PAGE, not by the harness fetches). No approval effect, grant, Git call
//! or acceptance is claimed; auth simulated on isolated fixtures.
#![cfg(target_os = "linux")]
// The shared fixture declares helpers consumed only by other including tests;
// the warnings it generates here match the pattern of the parallel lots.
#[path = "../../swarm-enrollment-authority/tests/common/authenticated_fixture.rs"]
#[allow(dead_code, unused_imports)]
mod fixture;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde_json::{json, Value};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};
use swarm_engine::{RootStore, SystemEnvironment};
use swarm_identity_contract::Scope;
use swarm_protection_contract::{Action, AdmissionContext, Proposal, SourceRef};
use swarm_web::PaeaGitCallSource;

const PIN: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

/// Resolve/pin actual installed Git; shared fixture pattern from the engine tests.
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
/// Fresh native service storage under the governed repository fixture area;
/// never open a user mission as a fixture.
fn service() -> (tempfile::TempDir, RootStore) {
    let dir = tempfile::tempdir_in(swarm_engine::fixture_area()).unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let root = RootStore::initialize(dir.path(), &mut SystemEnvironment).unwrap();
    (dir, root)
}
/// Explicit actual first-publication project, writable: the same retained
/// SWARM_PAEA_SOURCE_FIXTURE the engine harness uses, never a copy.
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
/// Read the current Trust revision from the fixture's own bounded state file.
fn current_trust_revision(root: &Path) -> i64 {
    let state: Value =
        serde_json::from_slice(&fs::read(root.join(".swarm/trust-actions.json")).unwrap())
            .unwrap();
    state["revision"].as_i64().expect("fixture Trust revision")
}
/// Actual project Trust configuration: enrolled identity WITH the approver
/// role plus the agreement admin; the real mandate/request producers run.
fn effect_config_agreement(root: &Path) -> Vec<u8> {
    let fresh = !root.join(".swarm/trust-actions.json").exists();
    let revision = if fresh { 0 } else { current_trust_revision(root) };
    serde_json::to_vec(&json!({
        "schema_version":1,
        "event_id":format!("consent-browser-agreement-config-{revision}"),
        "expected_revision":revision,
        "reason":"Explicit fixture agreement config for the consent browser qualification",
        "policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":false},
        "identities":[
            json!({"subject":"enrolled-effect","integrity":3,"roles":["admin","effect","approver"],"enrollment_principal":{"principal_id":"operator","principal_kind":"human","issuer":"https://fixture.invalid","subject":"stable-person"}}),
            json!({"subject":"requester","token":"fixture-requester-token","integrity":1,"roles":["requester"]}),
            json!({"subject":"approver","token":"fixture-approver-token","integrity":2,"roles":["approver"]}),
            json!({"subject":"agreement-admin","token":"fixture-admin-token","integrity":3,"roles":["admin"]}),
        ],
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
        request_id: "consent-browser-fixture".into(),
        action: Action::Git {
            repository_id: id.into(),
            target_ref: "refs/heads/release".into(),
            expected_old_oid: "1".repeat(40),
            candidate_oid: candidate.into(),
        },
    }
}
/// Declared context rechecked under held authority; one declared source.
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
struct Owned {
    project_path: PathBuf,
    service_path: PathBuf,
    project: RootStore,
    service: RootStore,
    // Keeps the service tempdir alive for the whole qualification: the bind
    // below re-opens `service_path` by canonicalized path, which fails with a
    // "path" error once TempDir::drop has removed the tree.
    _service_dir: tempfile::TempDir,
    prepared: swarm_engine::PaeaGitTransferCall,
    prepared_request_permission: swarm_identity_contract::Permission,
    repository_id: String,
    candidate: String,
    fixture_scope: Scope,
    trust_revision: i64,
}
fn owned_agreement() -> Owned {
    let (project_path, project, work, candidate) = project();
    project
        .trust("config-apply", &effect_config_agreement(&project_path), "")
        .unwrap_or_else(|e| panic!("explicit fixture config refused: {}", e.code()));
    let trust_revision = current_trust_revision(&project_path);
    let fixture_scope = scope(&project_path, &work);
    let (service_dir, service) = service();
    service
        .paea_initialize_git_intents("fixture-project", PIN)
        .unwrap();
    let bare = service_dir.path().join(".swarm/bare");
    fs::create_dir(&bare).unwrap();
    fs::set_permissions(&bare, fs::Permissions::from_mode(0o700)).unwrap();
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
        .arg(&bare)
        .output()
        .unwrap();
    assert!(result.status.success());
    let binding = service
        .paea_provision_git_repository(&project, &work, "bare", PIN)
        .unwrap();
    let repository_id = binding.repository_id().to_owned();
    let prepared = service
        .paea_git_transfer_call(
            &project,
            &proposal(&repository_id, &candidate),
            &context(trust_revision),
            &work,
            "bare",
            PIN,
            "transfer-pack",
        )
        .unwrap_or_else(|e| panic!("explicit preparation refused: {:?}", e));
    Owned {
        prepared_request_permission: prepared.request().permission(),
        service_path: service_dir.path().to_path_buf(),
        project_path,
        project,
        service,
        _service_dir: service_dir,
        prepared,
        repository_id,
        candidate,
        fixture_scope,
        trust_revision,
    }
}
/// Wait for one harness marker containing our exact nonce; bounded sleep loop
/// owned by this test (no global watchdog), fails with the observed directory
/// state after 600 seconds so a dead harness never hangs the run forever.
fn wait_marker(control_dir: &Path, nonce: &str, marker: &str) {
    let target = control_dir.join(marker);
    for _ in 0..6000 {
        if let Ok(bytes) = fs::read_to_string(&target) {
            assert!(
                bytes.contains(nonce),
                "marker {marker} present but not ours"
            );
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    panic!(
        "harness marker {marker} never appeared in {}",
        control_dir.display()
    );
}
/// Real served consent page against a REAL bound approval. Phase 1: the page
/// presents the bound capability, the exact prepared action and the consent
/// form. Phase 2: the page itself submits a drifted file (visible typed
/// refusal, no state change) and a correct file with a wrong password (typed
/// refusal); neither consumes the bound approval nor records anything. The
/// browser performs every POST through the real page; the Rust test never
/// drives the route itself, so no approval, agreement or ref effect occurs
/// and no acceptance is claimed.
#[test]
#[ignore = "requires explicit retained native source fixture and browser harness"]
fn consent_page_serves_real_bound_approval_to_real_browser() {
    let owned = owned_agreement();
    let (repository_id, candidate) = (owned.repository_id.clone(), owned.candidate.clone());
    let terms = owned
        .project
        .paea_git_agreement_terms(
            &swarm_protection_contract::canonicalize(
                &proposal(&repository_id, &candidate),
                &context(owned.trust_revision),
            )
            .unwrap()
            .bytes()
            .to_vec(),
        )
        .unwrap()
        .expect("recipe terms derivable for this canonical Git action");
    let resource = terms.resource.clone();
    let trust = |op: &str, value: Value, token: &str| -> Value {
        serde_json::from_slice(
            &owned
                .project
                .trust(op, &serde_json::to_vec(&value).unwrap(), token)
                .unwrap_or_else(|e| panic!("{op} refused: {}", e.code()))
                .bytes,
        )
        .unwrap()
    };
    // Reader frontier (BLP-RET-05): the guarded presentation and preview
    // refuse a bare session, so the browser fixture must produce REAL
    // confidentiality authorization through the supported control operations
    // BEFORE any guarded read — a BLP policy over the real fixture identities
    // (blp-policy-apply) then an attribution binding the EXACT canonical
    // digest of the prepared action to this project root
    // (blp-object-attribute). Event ids are bound to the live trust revision
    // (apply-once per revision): reruns on the retained fixture advance
    // through the real apply path and never revive a stale recorded entry.
    let blp_revision = trust("blp-show", json!({}), "fixture-admin-token")["record"]
        ["history"]
        .as_array()
        .and_then(|h| h.last())
        .and_then(|e| e["revision"].as_u64())
        .unwrap_or(0);
    trust(
        "blp-policy-apply",
        json!({
            "schema_version":1,
            "event_id":format!("consent-browser-policy-{}", owned.trust_revision),
            "expected_revision":blp_revision,
            "expected_trust_revision":owned.trust_revision,
            "reason":"Reader-frontier policy for the consent browser fixture",
            "policy":{
                "schema_version":1,
                "levels":[
                    {"id":"public","name_fr":"Public","name_en":"Public"},
                    {"id":"restricted","name_fr":"Diffusion restreinte","name_en":"Restricted"},
                    {"id":"classified","name_fr":"Diffusion contrôlée","name_en":"Classified"},
                ],
                "compartments":[],
                "clearances":{
                    "enrolled-effect":{"level":"classified","compartments":[]},
                    "requester":{"level":"restricted","compartments":[]},
                    "approver":{"level":"public","compartments":[]},
                    "agreement-admin":{"level":"classified","compartments":[]},
                },
            },
        }),
        "fixture-admin-token",
    );
    let attribute_revision = trust("blp-show", json!({}), "fixture-admin-token")["record"]
        ["history"]
        .as_array()
        .and_then(|h| h.last())
        .and_then(|e| e["revision"].as_u64())
        .unwrap_or(0);
    let attribute_reply = trust(
        "blp-object-attribute",
        json!({
            "schema_version":1,
            "event_id":format!("consent-browser-attribute-{}", owned.trust_revision),
            "expected_revision":attribute_revision,
            "expected_trust_revision":owned.trust_revision,
            "canonical_digest":swarm_protection_contract::canonicalize(
                &proposal(&repository_id, &candidate),
                &context(owned.trust_revision),
            )
            .unwrap()
            .digest(),
            "project_path":owned.project_path.to_str().unwrap(),
            "classification":{"level":"classified","compartments":[]},
            "reason":"Reader-frontier attribution for the exact prepared action of the consent browser fixture",
        }),
        "fixture-admin-token",
    );
    assert_eq!(
        attribute_reply["attributed"], true,
        "supported producer must attribute the exact prepared action"
    );
    // Real producers on the project root: mandate-create (admin bearer),
    // request (requester bearer) — the pending action exists before the page.
    let mandate_id = format!("consent-browser-mandate-{}", std::process::id());
    trust(
        "mandate-create",
        json!({
            "schema_version":1,
            "event_id":format!("consent-browser-mandate-{mandate_id}"),
            "mandate_id":mandate_id,
            "subject":"requester",
            "actions":["settle"],
            "resource":resource,
            "max_amount_minor":terms.amount_minor,
            "currency":terms.currency,
            "beneficiary":terms.beneficiary,
        }),
        "fixture-admin-token",
    );
    let action: Value = trust(
        "request",
        json!({
            "schema_version":1,
            "event_id":format!("consent-browser-request-{mandate_id}"),
            "mandate_id":mandate_id,
            "resource":resource,
            "amount_minor":terms.amount_minor,
            "currency":terms.currency,
            "beneficiary":terms.beneficiary,
            "idempotency_key":format!("consent-browser-key-{mandate_id}"),
        }),
        "fixture-requester-token",
    );
    assert_eq!(action["state"], "pending_approval");
    // The frozen approve command: exact terms of the pending action.
    let approve = serde_json::to_vec(&json!({
        "schema_version":1,
        "event_id":format!("consent-browser-approve-{mandate_id}"),
        "action_id":action["id"],
        "mandate_id":mandate_id,
        "content_digest":action["content_digest"],
        "resource":resource,
        "amount_minor":terms.amount_minor,
        "currency":terms.currency,
        "beneficiary":terms.beneficiary,
    }))
    .unwrap();
    // A visibly different file for the page's wrong-bytes submission: same
    // shape, one drifted term — digest comparison must refuse it typed.
    let drifted = serde_json::to_vec(&json!({
        "schema_version":1,
        "event_id":format!("consent-browser-drift-{mandate_id}"),
        "action_id":action["id"],
        "mandate_id":mandate_id,
        "content_digest":action["content_digest"],
        "resource":resource,
        "amount_minor":terms.amount_minor + 1,
        "currency":terms.currency,
        "beneficiary":terms.beneficiary,
    }))
    .unwrap();
    // Freeze the approval against the source's own project root, sign BOTH
    // permissions into one enrollment (one enroll per enrollment id), bind.
    let source = PaeaGitCallSource::new_transfer(
        owned.prepared,
        owned.service_path.clone(),
        owned.project_path.clone(),
    );
    let approval_source = source
        .prepare_approval(
            swarm_enrollment_authority::trust_action::Operation::Approve,
            &approve,
        )
        .unwrap_or_else(|e| panic!("approval preparation refused: {:?}", e.code()));
    let request_digest = approval_source.request().request_digest().to_owned();
    let fixture = fixture::Fixture::build_for_trust_actions(
        vec![
            owned.prepared_request_permission.clone(),
            approval_source.request().permission(),
        ],
        owned.fixture_scope.clone(),
        owned.trust_revision,
    );
    let authority = swarm_web::PaeaGitCallSource::open(
        source,
        fixture.path.to_str().unwrap(),
        &fixture.pin,
    )
    .unwrap_or_else(|e| panic!("provisioned declaration refused: {:?}", e.branches()));
    // Real bound listener with the approval attached; serve_until owns the
    // accept loop in its own thread until the owned stop marker.
    let server = swarm_web::NativeHttpServer::bind_git_call_with_approval(
        &owned.service_path,
        None,
        authority,
        Some(approval_source),
    )
    .unwrap_or_else(|e| panic!("approval bind refused: {:?} / {:?}", e.code(), e));
    let address = server.local_addr().unwrap();
    let control_dir = std::path::PathBuf::from(
        std::env::var_os("CONSENT_BROWSER_DIR").expect("harness control directory"),
    );
    let stop_path = std::sync::Arc::new(control_dir.join("server.stop"));
    let stop_wait = std::sync::Arc::clone(&stop_path);
    let server_thread = std::thread::spawn(move || {
        server
            .serve_until(
                async move {
                    loop {
                        if stop_wait.exists() {
                            break;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                },
                &mut |_| Ok(()),
            )
            .is_ok()
    });
    // Ordinary persisted session published by the bind; the browser reads the
    // same file through the ordinary cookie name for this exact address.
    let session_name = fs::read_dir(owned.service_path.join(".swarm"))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .find(|name| name.starts_with("web-session-"))
        .expect("bind must persist the ordinary web session credential");
    let nonce = format!("consent-bound-browser-{}", std::process::id());
    let startup = serde_json::json!({
        "schema_version":1,
        "event":"consent-bound-browser-ready",
        "nonce":nonce,
        "address":address.to_string(),
        "service_root":owned.service_path,
        "project_root":owned.project_path,
        "request_digest":request_digest,
        "trust_revision":owned.trust_revision,
        "cookie_name":format!(
            "swarm_session_{}",
            &{
                use sha2::{Digest, Sha256};
                let hash = Sha256::digest(address.to_string().as_bytes());
                hash.iter().map(|b| format!("{b:02x}")).collect::<String>()
            }[..12]
        ),
        "session_file":session_name,
        // Fixture reader credential the harness types into the page's
        // EPHEMERAL reader input (request-time Bearer, cleared on every exit).
        "reader_token":"fixture-admin-token",
        "frozen_bytes_b64":URL_SAFE_NO_PAD.encode(&approve),
        "drifted_bytes_b64":URL_SAFE_NO_PAD.encode(&drifted),
    });
    fs::write(control_dir.join("server.json"), serde_json::to_vec(&startup).unwrap())
        .unwrap();
    // Phase 1: fresh bound state — capability, exact action, consent form.
    wait_marker(&control_dir, &nonce, "phase1.done");
    // Phase 2 handoff: nothing changes server-side between the phases (no
    // revision change, unlike the staleness qualifications); the marker only
    // certifies phase 1 is observed before the harness drives its own visible
    // refusal submissions through the page.
    fs::write(
        control_dir.join("phase2.ready"),
        serde_json::to_vec(&serde_json::json!({"nonce": nonce, "trust_revision": owned.trust_revision})).unwrap(),
    )
    .unwrap();
    // The page submits wrong bytes then wrong password; both must be visible
    // typed refusals with the bound approval still unconsumed.
    wait_marker(&control_dir, &nonce, "phase2.done");
    // Explicit owned shutdown: the serving thread observes the stop marker and
    // serve_until drains and checked-closes listener/binding there.
    fs::write(stop_path.as_path(), b"stop").unwrap();
    let served = server_thread.join().expect("serve thread must not panic");
    assert!(served, "serve_until must report a clean owned shutdown");
    // No agreement may exist: the page never produced an approval.
    let exact = swarm_protection_contract::canonicalize(
        &proposal(&repository_id, &candidate),
        &context(owned.trust_revision),
    )
    .unwrap()
    .bytes()
    .to_vec();
    assert!(owned
        .project
        .paea_git_agreement_match(&exact)
        .unwrap()
        .is_none());
    let _ = owned.project.close();
    let _ = owned.service.close();
    fixture.close();
}
