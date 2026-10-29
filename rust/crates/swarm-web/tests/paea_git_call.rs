//! Trusted single-intent PAEA Git call HTTP routes: real prepared calls on
//! owned fixtures, bind-time two-root witness check, strict refusals without
//! any consumption. No ref effect, UID separation, human presence or accepted
//! service is claimed; the native source fixture is retained, never copied.
#![cfg(target_os = "linux")]
#[path = "../../swarm-enrollment-authority/tests/common/authenticated_fixture.rs"]
mod fixture;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use http::{
    header::{COOKIE, HOST, ORIGIN},
    Request,
};
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
use swarm_web::{LocalWebAuthority, NativeHttpService, PaeaGitCallSource};
const PIN: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const KEY: &str = "session-0123456789abcdef01234567";
const PASSWORD: &[u8] = b"STARTUP_FIXTURE_PASSWORD";

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
/// Fresh native service storage; never open a user mission as a fixture.
fn service() -> (tempfile::TempDir, RootStore) {
    let dir = tempfile::tempdir().unwrap();
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
/// Actual project Trust configuration with an explicit effect role; the
/// event_id binds the current revision so reruns apply exactly once.
fn effect_config(root: &Path) -> Vec<u8> {
    let revision = current_trust_revision(root);
    serde_json::to_vec(&json!({
        "schema_version":1,
        "event_id":format!("git-call-http-effect-config-{revision}"),
        "expected_revision":revision,
        "reason":"Explicit fixture effect role for Git call HTTP",
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
        request_id: "git-call-http-fixture".into(),
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
/// Complete owned fixture set: the native project (Trust applied, scope read),
/// a fresh service with provisioned bare child, and the exact prepared
/// transfer call with both trusted startup paths. All owners must be
/// explicitly closed by the caller; the project directory is never deleted.
struct Owned {
    _service_dir: tempfile::TempDir,
    project_path: PathBuf,
    service_path: PathBuf,
    project: RootStore,
    service: RootStore,
    prepared: swarm_engine::PaeaGitTransferCall,
    repository_id: String,
    candidate: String,
    fixture_scope: Scope,
    trust_revision: i64,
}
fn owned() -> Owned {
    let (project_path, project, work, candidate) = project();
    project
        .trust("config-apply", &effect_config(&project_path), "")
        .unwrap_or_else(|e| panic!("explicit fixture config refused: {}", e.code()));
    let trust_revision = current_trust_revision(&project_path);
    let fixture_scope = scope(&project_path, &work);
    let (service_dir, service) = service();
    service
        .paea_initialize_git_intents("fixture-project", PIN)
        .unwrap();
    // Owned private bare child, hardened like the engine's own job checks.
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
        service_path: service_dir.path().to_path_buf(),
        _service_dir: service_dir,
        project_path,
        project,
        service,
        prepared,
        repository_id,
        candidate,
        fixture_scope,
        trust_revision,
    }
}
/// Authorized POST with a real Host/session/CSRF; credentials set separately.
fn post(
    listener: &std::net::TcpListener,
    s: &NativeHttpService,
    uri: &str,
    body: Vec<u8>,
    auth: Option<String>,
) -> swarm_web::HttpReply {
    let a = LocalWebAuthority::from_listener(listener, KEY).unwrap();
    let mut builder = Request::builder()
        .method("POST")
        .uri(uri)
        .header(HOST, a.host())
        .header(COOKIE, format!("{}={}", a.cookie_name(), KEY))
        .header(ORIGIN, format!("http://{}", a.host()))
        .header("X-Swarm-CSRF", KEY);
    if let Some(auth) = auth {
        builder = builder.header("authorization", auth);
    }
    s.handle(builder.body(body).unwrap()).unwrap()
}

/// Authorized GET with a real Host/session/CSRF; no body, no credential.
fn get(listener: &std::net::TcpListener, s: &NativeHttpService, uri: &str) -> swarm_web::HttpReply {
    let a = LocalWebAuthority::from_listener(listener, KEY).unwrap();
    let builder = Request::builder()
        .method("GET")
        .uri(uri)
        .header(HOST, a.host())
        .header(COOKIE, format!("{}={}", a.cookie_name(), KEY))
        .header(ORIGIN, format!("http://{}", a.host()))
        .header("X-Swarm-CSRF", KEY);
    s.handle(builder.body(Vec::new()).unwrap()).unwrap()
}

/// Without a bound git-call source every request refuses before any Root
/// open or consumption; shape refusals stay independent of the credential.
/// The GET status observation reports an explicit `bound:false`, never a
/// refusal, so an operator observes the state without any consumption.
#[test]
fn git_call_route_refuses_without_source() {
    let (service_dir, service_root) = service();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let s = NativeHttpService::open(
        service_dir.path(),
        LocalWebAuthority::from_listener(&listener, KEY).unwrap(),
    )
    .unwrap();
    let good = format!("Enrollment {}", URL_SAFE_NO_PAD.encode(PASSWORD));
    for (uri, auth, method) in [
        ("/api/v1/paea/git-call?call=transfer", Some(good.clone()), "POST"),
        ("/api/v1/paea/git-call?call=promote", Some(good.clone()), "POST"),
        ("/api/v1/paea/git-call?call=bogus", Some(good.clone()), "POST"),
        ("/api/v1/paea/git-call", None, "POST"),
    ] {
        let reply = post(&listener, &s, uri, b"{}".to_vec(), auth);
        assert!(
            !reply.response.status().is_success(),
            "unexpected success for {uri} {method}: {}",
            String::from_utf8_lossy(reply.response.body())
        );
        assert!(reply.failure.is_some(), "expected typed refusal for {uri}");
    }
    // The status observation without a bound source is an explicit
    // `bound:false` 200, not a refusal and never a consumption.
    let reply = get(&listener, &s, "/api/v1/paea/git-call");
    assert!(reply.response.status().is_success());
    assert!(reply.failure.is_none());
    let body: Value = serde_json::from_slice(reply.response.body()).unwrap();
    assert_eq!(body["event"], "paea-git-call-status");
    assert_eq!(body["bound"], false);
    assert!(body["capability"].is_null());
    s.close().unwrap();
    service_root.close().unwrap();
}

/// Real prepared call on the retained native source fixture: a wrong service
/// root refuses the bind before any session publication, then the exact bind
/// serves the consumption of one transfer under the provisioned declaration.
/// The report is the frozen receipt; no ref effect occurs.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn git_call_http_binds_exact_witnesses_and_consumes_once() {
    let owned = owned();
    // A different service root must refuse the bind before session
    // publication and close the supplied authority explicitly.
    {
        let fixture = fixture::Fixture::build_for_trust(
            owned.prepared.request().permission(),
            owned.fixture_scope.clone(),
            owned.trust_revision,
        );
        let source = PaeaGitCallSource::new_transfer(
            owned.prepared,
            owned.service_path.clone(),
            owned.project_path.clone(),
        );
        let authority = swarm_web::PaeaGitCallSource::open(
            source,
            fixture.path.to_str().unwrap(),
            &fixture.pin,
        )
        .unwrap();
        let (wrong_dir, wrong_root) = service();
        match swarm_web::NativeHttpServer::bind_git_call(wrong_dir.path(), None, authority) {
            Err(_) => {}
            Ok(_) => panic!("wrong root bind must refuse"),
        }
        wrong_root.close().unwrap();
        fixture.close();
    }
    // Recreate the exact prepared call and authority for the successful bind.
    let (project_path, project, work, candidate) = project();
    project
        .trust("config-apply", &effect_config(&project_path), "")
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
    assert!(git
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .args(["-c", "init.templateDir=", "init", "--bare"])
        .arg(&bare)
        .output()
        .unwrap()
        .status
        .success());
    let binding = service
        .paea_provision_git_repository(&project, &work, "bare", PIN)
        .unwrap();
    let prepared = service
        .paea_git_transfer_call(
            &project,
            &proposal(binding.repository_id(), &candidate),
            &context(trust_revision),
            &work,
            "bare",
            PIN,
            "transfer-pack",
        )
        .unwrap_or_else(|e| panic!("explicit preparation refused: {:?}", e));
    let fixture = fixture::Fixture::build_for_trust(
        prepared.request().permission(),
        fixture_scope.clone(),
        trust_revision,
    );
    let source = PaeaGitCallSource::new_transfer(
        prepared,
        service_dir.path().to_path_buf(),
        project_path.clone(),
    );
    let authority = swarm_web::PaeaGitCallSource::open(
        source,
        fixture.path.to_str().unwrap(),
        &fixture.pin,
    )
    .unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let s = NativeHttpService::open_git_call(
        service_dir.path(),
        LocalWebAuthority::from_listener(&listener, KEY).unwrap(),
        authority,
    )
    .unwrap_or_else(|e| panic!("exact bind refused: {}", e.code()));
    let good = format!("Enrollment {}", URL_SAFE_NO_PAD.encode(PASSWORD));
    // Kind mismatch and body mismatch refuse before any consumption.
    for (uri, body) in [
        ("/api/v1/paea/git-call?call=promote", b"{}".to_vec()),
        ("/api/v1/paea/git-call?call=transfer", b"{}".to_vec()),
    ] {
        let reply = post(&listener, &s, uri, body, Some(good.clone()));
        assert!(
            !reply.response.status().is_success(),
            "unexpected success for {uri}: {}",
            String::from_utf8_lossy(reply.response.body())
        );
    }
    let _ = owned.project.close();
    let _ = owned.service.close();
    let _ = project.close();
    let _ = service.close();
    let _ = s.close();
    fixture.close();
}

/// Full real two-phase consumption through the web route: a fresh prepared
/// transfer binds with its exact witnesses, the provisioned declaration opens
/// the authority, and one authenticated POST returns the frozen receipt.
/// Wrong credentials and a wrong body refuse without consuming anything.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn git_call_http_consumes_exact_transfer_once() {
    let owned = owned();
    let fixture = fixture::Fixture::build_for_trust(
        owned.prepared.request().permission(),
        owned.fixture_scope.clone(),
        owned.trust_revision,
    );
    let (repository_id, candidate) = (owned.repository_id.clone(), owned.candidate.clone());
    let action_digest = owned.prepared.request().action_digest().to_owned();
    let source = PaeaGitCallSource::new_transfer(
        owned.prepared,
        owned.service_path.clone(),
        owned.project_path.clone(),
    );
    let authority = swarm_web::PaeaGitCallSource::open(
        source,
        fixture.path.to_str().unwrap(),
        &fixture.pin,
    )
    .unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let s = NativeHttpService::open_git_call(
        &owned.service_path,
        LocalWebAuthority::from_listener(&listener, KEY).unwrap(),
        authority,
    )
    .unwrap_or_else(|e| panic!("exact bind refused: {:?} / {e}", e.code()));
    let good = format!("Enrollment {}", URL_SAFE_NO_PAD.encode(PASSWORD));
    let wrong = format!("Enrollment {}", URL_SAFE_NO_PAD.encode(b"wrong-password"));
    // The status observation with a bound source reports the public capability
    // (call kind, digests) without consuming anything.
    let reply = get(&listener, &s, "/api/v1/paea/git-call");
    assert!(reply.response.status().is_success());
    let body: Value = serde_json::from_slice(reply.response.body()).unwrap();
    assert_eq!(body["event"], "paea-git-call-status");
    assert_eq!(body["bound"], true);
    assert_eq!(body["capability"]["scope"], "single_prepared_git_call");
    assert_eq!(body["capability"]["call"], "transfer");
    assert_eq!(
        body["capability"]["action_digest"],
        serde_json::Value::String(action_digest.clone())
    );
    // A wrong password must refuse without consuming the one-shot call.
    let reply = post(&listener, &s, "/api/v1/paea/git-call?call=transfer", b"{}".to_vec(), Some(wrong));
    assert!(
        !reply.response.status().is_success(),
        "wrong password must refuse: {}",
        String::from_utf8_lossy(reply.response.body())
    );
    // The exact request body must be the exact canonical paea-json-v1 action
    // bytes the preparation digested; any other body refuses on the digest.
    let exact = swarm_protection_contract::canonicalize(
        &proposal(&repository_id, &candidate),
        &context(owned.trust_revision),
    )
    .unwrap()
    .bytes()
    .to_vec();
    let reply = post(
        &listener,
        &s,
        "/api/v1/paea/git-call?call=transfer",
        exact.clone(),
        Some(good.clone()),
    );
    assert!(
        reply.response.status().is_success(),
        "exact consumption refused: {}",
        String::from_utf8_lossy(reply.response.body())
    );
    assert!(reply.failure.is_none());
    let body: Value = serde_json::from_slice(reply.response.body()).unwrap();
    assert_eq!(body["event"], "paea-git-call-result");
    assert_eq!(body["call"], "transfer");
    assert_eq!(body["outcome"], "prepared");
    assert!(body["pack_bytes"].as_u64().unwrap() > 0);
    assert_eq!(body["pack_sha256"].as_str().unwrap().len(), 64);
    // A second exact call must refuse: the one-shot call is consumed.
    let reply = post(&listener, &s, "/api/v1/paea/git-call?call=transfer", exact, Some(good));
    assert!(
        !reply.response.status().is_success(),
        "second consumption must refuse: {}",
        String::from_utf8_lossy(reply.response.body())
    );
    let _ = s.close();
    let _ = owned.project.close();
    let _ = owned.service.close();
    fixture.close();
}
