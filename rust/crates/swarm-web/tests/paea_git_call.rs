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

/// The preview observation without a bound source stays an explicit
/// `available:false` 200 — never a refusal and never a success claiming a
/// bound preparation. Refusals keep their typed codes for shape violations.
#[test]
fn preview_route_reports_explicit_absence_without_source() {
    let (service_dir, service_root) = service();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let s = NativeHttpService::open(
        service_dir.path(),
        LocalWebAuthority::from_listener(&listener, KEY).unwrap(),
    )
    .unwrap();
    // Explicit absence, not an error and not a bound preview.
    let reply = get(&listener, &s, "/api/v1/paea/git-call/preview");
    assert!(reply.response.status().is_success());
    assert!(reply.failure.is_none());
    let body: Value = serde_json::from_slice(reply.response.body()).unwrap();
    assert_eq!(body["event"], "paea-git-call-preview");
    assert_eq!(body["available"], false);
    assert_eq!(
        body["reason"],
        "paea-git-call-preview-source-unavailable"
    );
    // Shape violations keep typed refusals; no consumption or authority.
    for (uri, method, body) in [
        ("/api/v1/paea/git-call/preview", "POST", Vec::new()),
        ("/api/v1/paea/git-call/preview", "GET", b"x".to_vec()),
        (
            "/api/v1/paea/git-call/preview?x=1",
            "GET",
            Vec::new(),
        ),
    ] {
        let a = LocalWebAuthority::from_listener(&listener, KEY).unwrap();
        let mut builder = Request::builder()
            .method(method)
            .uri(uri)
            .header(HOST, a.host())
            .header(COOKIE, format!("{}={}", a.cookie_name(), KEY))
            .header(ORIGIN, format!("http://{}", a.host()))
            .header("X-Swarm-CSRF", KEY);
        if method != "POST" {
            // GET/HEAD must not require Origin/CSRF for admission, but the
            // route itself still refuses the shape; headers stay harmless.
            builder = Request::builder()
                .method(method)
                .uri(uri)
                .header(HOST, a.host())
                .header(COOKIE, format!("{}={}", a.cookie_name(), KEY));
        }
        let reply = s.handle(builder.body(body).unwrap()).unwrap();
        assert!(
            !reply.response.status().is_success(),
            "unexpected success for {method} {uri}"
        );
        assert!(reply.failure.is_some(), "expected typed refusal for {uri}");
    }
    s.close().unwrap();
    service_root.close().unwrap();
}

/// With a real bound preparation on the retained native source fixture, the
/// preview presents the exact held action to an authorized Bearer reader:
/// declared proposal/context verbatim, recomputed canonical identity matching
/// the bound digest, current witness state and current Trust revision. No
/// consent, grant or consumption occurs. The BLP reader frontier (BLP-RET-05)
/// requires the policy and the attribution of the exact canonical bytes to be
/// produced through the supported control producers before the read, exactly
/// like the reader-guard test; a bare session gets no classified marker.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn preview_route_presents_exact_bound_action_with_current_state() {
    let owned = owned_agreement();
    let action_digest = owned.prepared.request().action_digest().to_owned();
    let trust_revision = owned.trust_revision;
    let (repository_id, candidate) = match &owned.prepared.proposal().action {
        Action::Git {
            repository_id,
            candidate_oid,
            ..
        } => (repository_id.clone(), candidate_oid.clone()),
        _ => panic!("git proposal expected"),
    };
    // BLP reader frontier: produce the current policy and the current
    // attribution of the exact canonical bytes through the same supported
    // producers as the reader-guard test, before any read occurs.
    let blp_revision = {
        let show = owned
            .project
            .trust("blp-show", b"{}".as_slice(), "fixture-admin-token")
            .unwrap_or_else(|e| panic!("blp-show refused: {}", e.code()));
        serde_json::from_slice::<Value>(&show.bytes).unwrap()["record"]["history"]
            .as_array()
            .and_then(|h| h.last())
            .and_then(|e| e["revision"].as_u64())
            .unwrap_or(0)
    };
    let current_revision = current_trust_revision(&owned.project_path);
    owned
        .project
        .trust(
            "blp-policy-apply",
            &serde_json::to_vec(&json!({
                "schema_version":1,
                "event_id":format!("preview-baseline-policy-{current_revision}"),
                "expected_revision":blp_revision,
                "expected_trust_revision":current_revision,
                "reason":"Reader-guard policy for the preview baseline",
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
            }))
            .unwrap(),
            "fixture-admin-token",
        )
        .unwrap_or_else(|e| panic!("blp-policy-apply refused: {}", e.code()));
    owned
        .project
        .trust(
            "blp-object-attribute",
            &serde_json::to_vec(&json!({
                "schema_version":1,
                "event_id":format!("preview-baseline-attribute-{current_revision}"),
                "expected_revision":blp_revision + 1,
                "expected_trust_revision":current_revision,
                "canonical_digest":swarm_protection_contract::canonicalize(
                    &proposal(&repository_id, &candidate),
                    &context(trust_revision),
                ).unwrap().digest(),
                "project_path":owned.project_path.to_str().unwrap(),
                "classification":{"level":"classified","compartments":[]},
                "reason":"Reader-guard attribution for the preview baseline",
            }))
            .unwrap(),
            "fixture-admin-token",
        )
        .unwrap_or_else(|e| panic!("blp-object-attribute refused: {}", e.code()));
    const READER: &str = "Bearer fixture-admin-token";
    let fixture = fixture::Fixture::build_for_trust(
        owned.prepared.request().permission(),
        owned.fixture_scope.clone(),
        trust_revision,
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
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let s = NativeHttpService::open_git_call(
        &owned.service_path,
        LocalWebAuthority::from_listener(&listener, KEY).unwrap(),
        authority,
    )
    .unwrap_or_else(|e| panic!("exact bind refused: {}", e.code()));
    let reply = get_with(&listener, &s, "/api/v1/paea/git-call/preview", Some(READER));
    assert!(reply.response.status().is_success());
    assert!(reply.failure.is_none());
    let body: Value = serde_json::from_slice(reply.response.body()).unwrap();
    assert_eq!(body["event"], "paea-git-call-preview");
    assert_eq!(body["available"], true);
    assert_eq!(body["capability"]["action_digest"], action_digest);
    // The declared proposal and context are presented verbatim.
    assert_eq!(body["proposal"]["schema_version"], 1);
    assert_eq!(
        body["proposal"]["action"]["target_ref"],
        "refs/heads/release"
    );
    assert_eq!(body["context"]["policy_revision"], trust_revision);
    // The canonical identity is recomputed and matches the bound digest.
    assert_eq!(body["canonical"]["digest_matches"], true);
    assert_eq!(body["canonical"]["action_digest"], action_digest);
    assert!(body["canonical"]["intent_id"].as_str().unwrap().len() == 64);
    // Current state is explicit and, on this fresh fixture, coherent.
    assert_eq!(body["current"]["roots_bound"], true);
    assert_eq!(body["current"]["trust_revision"], trust_revision);
    assert_eq!(
        body["current"]["declared_trust_revision"],
        trust_revision
    );
    assert_eq!(body["current"]["trust_revision_current"], true);
    // The engine-derived approval terms are presented: the fixed recipe the
    // captured-agreement guard matches on, at the current Trust revision —
    // nothing browser-computed, and the preview itself still captures no
    // consent (the terms are a presentation, not an approval).
    assert_eq!(body["agreement_terms"]["binding"], "recipe");
    assert_eq!(
        body["agreement_terms"]["resource"],
        format!("fixture-project/{repository_id}/refs/heads/release/{candidate}")
    );
    assert_eq!(body["agreement_terms"]["amount_minor"], 1);
    assert_eq!(body["agreement_terms"]["currency"], "OID");
    assert_eq!(body["agreement_terms"]["beneficiary"], "1".repeat(40));
    assert_eq!(body["agreement_terms"]["revision"], trust_revision);
    assert_eq!(body["agreement_terms"]["action_id"], "");
    // The preview captures no consent and performs no consumption.
    assert_eq!(body["limits"]["consent"], "none-captured");
    assert_eq!(body["limits"]["consumption"], "none");
    assert_eq!(body["limits"]["human_presence"], "not-established");
    // The one-shot call stays consumable afterwards: the preview consumed
    // nothing (proven by the status observation still reporting bound).
    let status = get(&listener, &s, "/api/v1/paea/git-call");
    let status_body: Value = serde_json::from_slice(status.response.body()).unwrap();
    assert_eq!(status_body["bound"], true);
    let _ = s.close();
    let _ = owned.project.close();
    let _ = owned.service.close();
    fixture.close();
}
/// Real prepared call bound to a real native HTTP server with its ordinary
/// persisted session, held alive while a real browser qualification drives the
/// served page through two public states: the exact fresh bound preparation,
/// then the explicitly stale state after a real public Trust revision change
/// on the project root (an actual `config-apply` through the engine — never a
/// journal edit). Phase coordination with the browser harness uses owned
/// marker files; every file the harness writes must carry the exact startup
/// event nonce, so no other process can drive this server. The server closes
/// checked on every path; no consent, grant, consumption or ref effect occurs
/// (the preview route itself consumes nothing) and no acceptance is claimed.
#[test]
#[ignore = "requires explicit retained native source fixture and browser harness"]
fn preview_bound_serves_real_browser_session() {
    let owned = owned();
    let action_digest = owned.prepared.request().action_digest().to_owned();
    let trust_revision = owned.trust_revision;
    let fixture = fixture::Fixture::build_for_trust(
        owned.prepared.request().permission(),
        owned.fixture_scope.clone(),
        trust_revision,
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
    .unwrap_or_else(|e| panic!("provisioned declaration refused: {:?}", e.branches()));
    // Real bound listener with the ordinary served transport: serve_until owns
    // the accept loop in its own thread until this test explicitly stops it
    // through the owned stop marker (the shutdown future polls that file).
    // Binding alone never accepts connections; without serving, the browser
    // would hang exactly as an unstarted production server would.
    let server = swarm_web::NativeHttpServer::bind_git_call(
        &owned.service_path,
        None,
        authority,
    )
    .unwrap_or_else(|e| panic!("exact bind refused: {}", e.code()));
    let address = server.local_addr().unwrap();
    let stop_path = std::sync::Arc::new(
        std::env::var_os("PREVIEW_BOUND_BROWSER_DIR")
            .map(std::path::PathBuf::from)
            .expect("harness control directory")
            .join("server.stop"),
    );
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
    // Ordinary persisted session: bind_source opened the service Root writable
    // and from_open_store published the ordinary credential through the
    // engine's own web_session writer for this exact listener address — the
    // browser reads that same file; no parallel authority path exists and no
    // secret enters the startup event.
    let session_name = fs::read_dir(owned.service_path.join(".swarm"))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .find(|name| name.starts_with("web-session-"))
        .expect("bind must persist the ordinary web session credential");
    let session_name = fs::read_dir(owned.service_path.join(".swarm"))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .find(|name| name.starts_with("web-session-"))
        .expect("web_session must persist the ordinary web session credential");
    let nonce = format!(
        "preview-bound-browser-{}",
        std::process::id()
    );
    let startup = serde_json::json!({
        "schema_version":1,
        "event":"preview-bound-browser-ready",
        "nonce":nonce,
        "address":address.to_string(),
        "service_root":owned.service_path,
        "project_root":owned.project_path,
        "action_digest":action_digest,
        "trust_revision":trust_revision,
        // Same address-scoped protocol namespace as LocalWebAuthority (the
        // first 12 hex bytes of the SHA-256 of the observed host string);
        // derived here from the same session file stem, no second authority.
        "cookie_name":format!(
            "swarm_session_{}",
            &{
                use sha2::{Digest, Sha256};
                let hash = Sha256::digest(address.to_string().as_bytes());
                hash.iter().map(|b| format!("{b:02x}")).collect::<String>()
            }[..12]
        ),
        "session_file":session_name,
    });
    let control_dir = std::path::PathBuf::from(
        std::env::var_os("PREVIEW_BOUND_BROWSER_DIR").expect("harness control directory"),
    );
    fs::write(control_dir.join("server.json"), serde_json::to_vec(&startup).unwrap())
        .unwrap();
    // Phase 1: fresh state. The harness drives the real page, asserts the
    // exact presented identity/declared terms and writes the phase-1 done
    // marker with our nonce before we mutate anything.
    wait_marker(&control_dir, &nonce, "phase1.done");
    // Real public Trust revision change on the project root: an actual
    // config-apply through the engine under the same local operator recovery
    // path the engine tests use, exactly once, on the owned fixture. No
    // journal file is edited and no state is fabricated. The public `show`
    // view omits raw tokens, so the two token identities get their exact
    // fixture tokens back and the enrolled identity keeps its principal
    // mapping (same reconstruction the settlement tests apply).
    let view: Value =
        serde_json::from_slice(&owned.project.trust("show", b"{}", "").unwrap().bytes).unwrap();
    let mut identities = view["identities"].clone();
    for identity in identities.as_array_mut().unwrap() {
        identity["disabled"] = false.into();
        identity["token"] = match identity["subject"].as_str() {
            Some("requester") => "fixture-requester-token".into(),
            Some("approver") => "fixture-approver-token".into(),
            _ => Value::String(String::new()),
        };
    }
    let change = serde_json::json!({
        "schema_version":1,
        "event_id":format!("preview-bound-browser-stale-{}", std::process::id()),
        "expected_revision":trust_revision,
        "reason":"Explicit fixture Trust revision change for browser staleness qualification",
        "policy":view["policy"],
        "identities":identities,
    });
    owned
        .project
        .trust("config-apply", &serde_json::to_vec(&change).unwrap(), "")
        .unwrap_or_else(|e| panic!("stale revision change refused: {}", e.code()));
    let stale_revision = current_trust_revision(&owned.project_path);
    assert_eq!(stale_revision, trust_revision + 1);
    fs::write(control_dir.join("phase2.ready"), serde_json::to_vec(&serde_json::json!({
        "nonce":nonce,
        "trust_revision":stale_revision,
    })).unwrap()).unwrap();
    // Phase 2: stale state. The harness re-observes the page through the same
    // session and asserts the visible stale refusal without any consent.
    wait_marker(&control_dir, &nonce, "phase2.done");
    // Explicit owned shutdown: the serving thread observes the stop marker and
    // serve_until drains and checked-closes listener/binding there; the thread's
    // success bit keeps that close outcome visible, never dropped.
    fs::write(stop_path.as_path(), b"stop").unwrap();
    let served = server_thread.join().expect("serve thread must not panic");
    assert!(served, "serve_until must report a clean owned shutdown");
    let _ = owned.project.close();
    let _ = owned.service.close();
    fixture.close();
}

/// Wait for one harness marker containing our exact nonce; bounded sleep loop
/// owned by this test (no global watchdog), fails with the observed directory
/// state after 600 seconds so a dead harness never hangs the run forever.
fn wait_marker(control_dir: &Path, nonce: &str, marker: &str) {
    let target = control_dir.join(marker);
    for _ in 0..6000 {
        if let Ok(bytes) = fs::read_to_string(&target) {
            // The harness writes an error marker when its browser fails: the
            // FIRST browser error is the real cause and must reach the test
            // failure verbatim — never masked by a later assertion panic.
            if let Ok(value) = serde_json::from_str::<Value>(&bytes) {
                if value.get("nonce").and_then(|n| n.as_str()) == Some(nonce) {
                    if let Some(error) = value.get("error").and_then(|e| e.as_str()) {
                        panic!("browser failed before {marker}: {error}");
                    }
                }
            }
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
/// Actual project Trust configuration with an explicit effect role; the
/// event_id binds the current revision so reruns apply exactly once.
fn effect_config(root: &Path) -> Vec<u8> {
    effect_config_named(root, "git-call-http-effect-config", false)
}
/// Agreement variant adds one bearer I3 administrator on the isolated
/// fixture (labeled auth simulated) so the real mandate-create producer can
/// run; the enrolled identity also holds the approver role so the same held
/// enrollment can approve through the protected approval channel (the single
/// enrollment-principal mapping constraint keeps one identity per root).
fn effect_config_agreement(root: &Path) -> Vec<u8> {
    effect_config_named(root, "git-call-http-agreement-config", true)
}
/// Wrong-role variant: the same agreement fixture but the enrolled identity
/// deliberately WITHOUT the approver role (and the mandate/request producers
/// keep their simulator admin/requester tokens). The route's frozen approve
/// must reach the existing approve guard and be visibly denied — never
/// approved — and no separate role mapping may be mintable by the request.
fn effect_config_agreement_no_approver(root: &Path) -> Vec<u8> {
    let fresh = !root.join(".swarm/trust-actions.json").exists();
    let revision = if fresh { 0 } else { current_trust_revision(root) };
    serde_json::to_vec(&json!({
        "schema_version":1,
        "event_id":format!("git-call-http-agreement-no-approver-{revision}"),
        "expected_revision":revision,
        "reason":"Explicit fixture effect role without the approver role for Git call HTTP",
        "policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":false},
        "identities":[
            json!({"subject":"enrolled-effect","integrity":3,"roles":["admin","effect"],"enrollment_principal":{"principal_id":"operator","principal_kind":"human","issuer":"https://fixture.invalid","subject":"stable-person"}}),
            json!({"subject":"requester","token":"fixture-requester-token","integrity":1,"roles":["requester"]}),
            json!({"subject":"approver","token":"fixture-approver-token","integrity":2,"roles":["approver"]}),
            json!({"subject":"agreement-admin","token":"fixture-admin-token","integrity":3,"roles":["admin"]}),
        ],
    }))
    .unwrap()
}
fn effect_config_named(root: &Path, event: &str, agreement_admin: bool) -> Vec<u8> {
    // Idempotent on both regimes: a fresh root (no state file yet) applies
    // at expected revision 0 with a revision-stable event id, so repeated
    // or parallel setup applies exactly once; a retained root keeps the
    // original revision-bound event id at its actual current revision.
    let fresh = !root.join(".swarm/trust-actions.json").exists();
    let revision = if fresh { 0 } else { current_trust_revision(root) };
    let mut identities = vec![
        json!({"subject":"enrolled-effect","integrity":3,"roles":["admin","effect","approver"],"enrollment_principal":{"principal_id":"operator","principal_kind":"human","issuer":"https://fixture.invalid","subject":"stable-person"}}),
        json!({"subject":"requester","token":"fixture-requester-token","integrity":1,"roles":["requester"]}),
        json!({"subject":"approver","token":"fixture-approver-token","integrity":2,"roles":["approver"]}),
    ];
    if agreement_admin {
        identities.push(json!({"subject":"agreement-admin","token":"fixture-admin-token","integrity":3,"roles":["admin"]}));
    }
    serde_json::to_vec(&json!({
        "schema_version":1,
        "event_id":format!("{event}-{revision}"),
        "expected_revision":revision,
        "reason":"Explicit fixture effect role for Git call HTTP",
        "policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":false},
        "identities":identities,
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
    /// Exact paea-action permission of the prepared transfer call, retained so
    /// a fixture account can be signed with it TOGETHER with the frozen
    /// approval's trust-action permission (one enroll per enrollment id).
    prepared_request_permission: swarm_identity_contract::Permission,
    repository_id: String,
    candidate: String,
    /// The fixture's actual first-publication work id, needed by any second
    /// preparation against the same project (wrong-holder bind test).
    work: String,
    fixture_scope: Scope,
    trust_revision: i64,
}
fn owned() -> Owned {
    owned_with(effect_config)
}
/// Agreement variant: same fixture with the extra simulated bearer
/// administrator so the mandate/request/approve producers run.
fn owned_agreement() -> Owned {
    owned_with(effect_config_agreement)
}
/// Wrong-role variant: the enrolled principal maps to an identity holding
/// admin+effect but NOT approver, so the frozen approve must be denied by
/// the existing guard even through the fully legitimate enrolled channel.
fn owned_agreement_no_approver() -> Owned {
    owned_with(effect_config_agreement_no_approver)
}
fn owned_with(config: fn(&Path) -> Vec<u8>) -> Owned {
    let (project_path, project, work, candidate) = project();
    project
        .trust("config-apply", &config(&project_path), "")
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
        prepared_request_permission: prepared.request().permission(),
        service_path: service_dir.path().to_path_buf(),
        _service_dir: service_dir,
        project_path,
        project,
        service,
        prepared,
        repository_id,
        candidate,
        work,
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
    get_with(listener, s, uri, None)
}

/// Authorized HEAD with a real Host/session/CSRF; no body, no credential.
fn head(
    listener: &std::net::TcpListener,
    s: &NativeHttpService,
    uri: &str,
) -> swarm_web::HttpReply {
    let a = LocalWebAuthority::from_listener(listener, KEY).unwrap();
    let builder = Request::builder()
        .method("HEAD")
        .uri(uri)
        .header(HOST, a.host())
        .header(COOKIE, format!("{}={}", a.cookie_name(), KEY))
        .header(ORIGIN, format!("http://{}", a.host()))
        .header("X-Swarm-CSRF", KEY);
    s.handle(builder.body(Vec::new()).unwrap()).unwrap()
}

/// GET variant with an optional `Authorization` header value verbatim (e.g.
/// `Bearer <token>`), mirroring `post`'s credential parameter. Nothing else
/// differs: real Host/session/CSRF, empty body, no query.
fn get_with(
    listener: &std::net::TcpListener,
    s: &NativeHttpService,
    uri: &str,
    auth: Option<&str>,
) -> swarm_web::HttpReply {
    let a = LocalWebAuthority::from_listener(listener, KEY).unwrap();
    let mut builder = Request::builder()
        .method("GET")
        .uri(uri)
        .header(HOST, a.host())
        .header(COOKIE, format!("{}={}", a.cookie_name(), KEY))
        .header(ORIGIN, format!("http://{}", a.host()))
        .header("X-Swarm-CSRF", KEY);
    if let Some(auth) = auth {
        builder = builder.header("authorization", auth);
    }
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
    let owned = owned_agreement();
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
    // The status observation with a bound source reports ONLY the `bound`
    // boolean: nothing derived from the selected prepared request passes —
    // not the digests, and not the selected call/service/operation either
    // (a closed vocabulary does not make the selected value a constant,
    // BLP-RET-05). The full capability stays behind the guarded preview route.
    let reply = get(&listener, &s, "/api/v1/paea/git-call");
    assert!(reply.response.status().is_success());
    let body: Value = serde_json::from_slice(reply.response.body()).unwrap();
    assert_eq!(body["event"], "paea-git-call-status");
    assert_eq!(body["bound"], true);
    assert_eq!(
        body["capability"], serde_json::Value::Null,
        "status must serve no request-derived capability: {}",
        body
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
    // The captured-agreement guard now precedes consumption: the exact
    // durable Trust approval must bind these bytes first, through the real
    // mandate/request/approve producers (auth simulated, labeled fixture).
    capture_agreement(&owned.project, &repository_id, &candidate);
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

/// Capture the exact durable agreement for the prepared transfer action
/// through the real Trust operations (mandate, request, exact approve) with
/// the fixed recipe terms binding the full canonical Git-intent identity
/// (project/repository/ref/candidate, beneficiary = old OID, amount 1 OID).
/// Bearer tokens on the isolated fixture are labeled auth simulated — never
/// human enrollment evidence. Returns nothing; panics on any producer refusal.
fn capture_agreement(
    project: &RootStore,
    repository_id: &str,
    candidate: &str,
) {
    let resource = format!("fixture-project/{repository_id}/refs/heads/release/{candidate}");
    let mandate_id = format!("agreement-mandate-{}", std::process::id());
    let trust = |op: &str, value: Value, token: &str| {
        project
            .trust(op, &serde_json::to_vec(&value).unwrap(), token)
            .unwrap_or_else(|e| panic!("{op} refused: {}", e.code()))
    };
    trust(
        "mandate-create",
        json!({
            "schema_version":1,
            "event_id":format!("agreement-mandate-{mandate_id}"),
            "mandate_id":mandate_id,
            "subject":"requester",
            "actions":["settle"],
            "resource":resource,
            "max_amount_minor":1,
            "currency":"OID",
            "beneficiary":"1".repeat(40),
        }),
        "fixture-admin-token",
    );
    let action: Value = serde_json::from_slice(&trust(
        "request",
        json!({
            "schema_version":1,
            "event_id":format!("agreement-request-{mandate_id}"),
            "mandate_id":mandate_id,
            "resource":resource,
            "amount_minor":1,
            "currency":"OID",
            "beneficiary":"1".repeat(40),
            "idempotency_key":format!("agreement-key-{mandate_id}"),
        }),
        "fixture-requester-token",
    )
    .bytes)
    .unwrap();
    assert_eq!(action["state"], "pending_approval");
    let approved: Value = serde_json::from_slice(&trust(
        "approve",
        json!({
            "schema_version":1,
            "event_id":format!("agreement-approve-{mandate_id}"),
            "action_id":action["id"],
            "mandate_id":mandate_id,
            "content_digest":action["content_digest"],
            "resource":resource,
            "amount_minor":1,
            "currency":"OID",
            "beneficiary":"1".repeat(40),
        }),
        "fixture-approver-token",
    )
    .bytes)
    .unwrap();
    assert_eq!(approved["state"], "approved");
    assert_eq!(approved["approver"], "approver");
}

/// The captured-agreement guard on the consumption route: a prepared transfer
/// with NO durable Trust approval bound to its exact canonical action bytes
/// refuses with the typed `paea-git-call-agreement` code before any root
/// opens or consumption — the browser provides neither authority nor a
/// replacement action. Auth simulated on isolated fixtures (labeled, never a
/// live mission database); no human presence or acceptance is claimed.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn git_call_http_refuses_without_captured_agreement() {
    let owned = owned();
    let fixture = fixture::Fixture::build_for_trust(
        owned.prepared.request().permission(),
        owned.fixture_scope.clone(),
        owned.trust_revision,
    );
    let (repository_id, candidate) = (owned.repository_id.clone(), owned.candidate.clone());
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
    .unwrap_or_else(|e| panic!("exact bind refused: {}", e.code()));
    let good = format!("Enrollment {}", URL_SAFE_NO_PAD.encode(PASSWORD));
    let exact = swarm_protection_contract::canonicalize(
        &proposal(&repository_id, &candidate),
        &context(owned.trust_revision),
    )
    .unwrap()
    .bytes()
    .to_vec();
    // Exact digest, exact witnesses, held authority — but no captured
    // agreement: the typed refusal, not a kind/digest/credential refusal.
    let reply = post(
        &listener,
        &s,
        "/api/v1/paea/git-call?call=transfer",
        exact,
        Some(good),
    );
    assert!(
        !reply.response.status().is_success(),
        "consumption without agreement must refuse: {}",
        String::from_utf8_lossy(reply.response.body())
    );
    let refusal = reply.failure.expect("typed refusal expected");
    assert_eq!(refusal.code(), "paea-git-call-agreement");
    let _ = s.close();
    let _ = owned.project.close();
    let _ = owned.service.close();
    fixture.close();
}

/// Fresh compliant chain through the real web route: mandate, request and
/// exact approval captured on the project root under the fixed agreement
/// recipe (identity = project/repository/ref/candidate, beneficiary = old
/// OID, amount 1 OID), then ONE authenticated consumption succeeds. The
/// approval is captured through the real Trust operations (bearer tokens on
/// the isolated fixture — labeled auth simulated, not human enrollment
/// evidence); the engine authority/witness/plan checks still run after.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn git_call_http_consumes_with_captured_agreement() {
    let owned = owned_agreement();
    let fixture = fixture::Fixture::build_for_trust(
        owned.prepared.request().permission(),
        owned.fixture_scope.clone(),
        owned.trust_revision,
    );
    let (repository_id, candidate) = (owned.repository_id.clone(), owned.candidate.clone());
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
    .unwrap_or_else(|e| panic!("exact bind refused: {}", e.code()));
    let good = format!("Enrollment {}", URL_SAFE_NO_PAD.encode(PASSWORD));
    let exact = swarm_protection_contract::canonicalize(
        &proposal(&repository_id, &candidate),
        &context(owned.trust_revision),
    )
    .unwrap()
    .bytes()
    .to_vec();
    // Before the agreement: typed refusal, no consumption.
    let reply = post(
        &listener,
        &s,
        "/api/v1/paea/git-call?call=transfer",
        exact.clone(),
        Some(good.clone()),
    );
    assert_eq!(
        reply.failure.expect("typed refusal expected").code(),
        "paea-git-call-agreement"
    );
    // Wrong role refuses before any capture: the requester cannot approve.
    {
        let resource = format!("fixture-project/{repository_id}/refs/heads/release/{candidate}");
        let mandate_id = format!("role-mandate-{}", std::process::id());
        let trust = |op: &str, value: Value, token: &str| {
            owned
                .project
                .trust(op, &serde_json::to_vec(&value).unwrap(), token)
                .unwrap_or_else(|e| panic!("{op} refused: {}", e.code()))
        };
        trust(
            "mandate-create",
            json!({
                "schema_version":1,
                "event_id":format!("agreement-mandate-{mandate_id}"),
                "mandate_id":mandate_id,
                "subject":"requester",
                "actions":["settle"],
                "resource":resource,
                "max_amount_minor":1,
                "currency":"OID",
                "beneficiary":"1".repeat(40),
            }),
            "fixture-admin-token",
        );
        let action: Value = serde_json::from_slice(&trust(
            "request",
            json!({
                "schema_version":1,
                "event_id":format!("agreement-request-{mandate_id}"),
                "mandate_id":mandate_id,
                "resource":resource,
                "amount_minor":1,
                "currency":"OID",
                "beneficiary":"1".repeat(40),
                "idempotency_key":format!("agreement-key-{mandate_id}"),
            }),
            "fixture-requester-token",
        )
        .bytes)
        .unwrap();
        let wrong_role = json!({
            "schema_version":1,
            "event_id":format!("agreement-wrong-role-{mandate_id}"),
            "action_id":action["id"],
            "mandate_id":mandate_id,
            "content_digest":action["content_digest"],
            "resource":resource,
            "amount_minor":1,
            "currency":"OID",
            "beneficiary":"1".repeat(40),
        });
        assert_eq!(
            serde_json::from_slice::<Value>(
                &trust("approve", wrong_role, "fixture-requester-token").bytes
            )
            .unwrap()["state"],
            "denied"
        );
        // Wrong action (non-exact terms) refuses — no agreement.
        let wrong_terms = json!({
            "schema_version":1,
            "event_id":format!("agreement-wrong-terms-{mandate_id}"),
            "action_id":action["id"],
            "mandate_id":mandate_id,
            "content_digest":action["content_digest"],
            "resource":resource,
            "amount_minor":2,
            "currency":"OID",
            "beneficiary":"1".repeat(40),
        });
        assert_eq!(
            serde_json::from_slice::<Value>(
                &trust("approve", wrong_terms, "fixture-approver-token").bytes
            )
            .unwrap()["state"],
            "denied"
        );
    }
    // Capture the exact agreement through the real Trust producers.
    capture_agreement(&owned.project, &repository_id, &candidate);
    // Replaying the same producer events (same deterministic ids) is retained:
    // the Trust state must not change and no second agreement is created.
    let state_before = fs::read(owned.project_path.join(".swarm/trust-actions.json")).unwrap();
    capture_agreement(&owned.project, &repository_id, &candidate);
    assert_eq!(
        fs::read(owned.project_path.join(".swarm/trust-actions.json")).unwrap(),
        state_before
    );
    // Fresh compliant consumption: exactly one success.
    let reply = post(
        &listener,
        &s,
        "/api/v1/paea/git-call?call=transfer",
        exact.clone(),
        Some(good.clone()),
    );
    assert!(
        reply.response.status().is_success(),
        "fresh compliant consumption refused: {}",
        String::from_utf8_lossy(reply.response.body())
    );
    assert!(reply.failure.is_none());
    let body: Value = serde_json::from_slice(reply.response.body()).unwrap();
    assert_eq!(body["event"], "paea-git-call-result");
    assert_eq!(body["outcome"], "prepared");
    assert!(body["pack_bytes"].as_u64().unwrap() > 0);
    // The one-shot call is consumed; a second identical request refuses.
    let reply = post(
        &listener,
        &s,
        "/api/v1/paea/git-call?call=transfer",
        exact,
        Some(good),
    );
    assert!(
        !reply.response.status().is_success(),
        "second consumption must refuse"
    );
    let _ = s.close();
    let _ = owned.project.close();
    let _ = owned.service.close();
    fixture.close();
}

/// A Trust configuration change on the project root after the approval voids
/// the captured agreement at last use: the same typed refusal, no consumption,
/// no fallback — recovery requires a fresh approval under the new revision.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn git_call_http_agreement_void_after_revision_change() {
    let owned = owned_agreement();
    let fixture = fixture::Fixture::build_for_trust(
        owned.prepared.request().permission(),
        owned.fixture_scope.clone(),
        owned.trust_revision,
    );
    let (repository_id, candidate) = (owned.repository_id.clone(), owned.candidate.clone());
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
    .unwrap_or_else(|e| panic!("exact bind refused: {}", e.code()));
    let good = format!("Enrollment {}", URL_SAFE_NO_PAD.encode(PASSWORD));
    let exact = swarm_protection_contract::canonicalize(
        &proposal(&repository_id, &candidate),
        &context(owned.trust_revision),
    )
    .unwrap()
    .bytes()
    .to_vec();
    // Capture the exact agreement first.
    let resource = format!("fixture-project/{repository_id}/refs/heads/release/{candidate}");
    let mandate_id = format!("void-mandate-{}", std::process::id());
    let trust = |op: &str, value: Value, token: &str| {
        owned
            .project
            .trust(op, &serde_json::to_vec(&value).unwrap(), token)
            .unwrap_or_else(|e| panic!("{op} refused: {}", e.code()))
    };
    trust(
        "mandate-create",
        json!({
            "schema_version":1,
            "event_id":format!("void-mandate-{}", std::process::id()),
            "mandate_id":mandate_id,
            "subject":"requester",
            "actions":["settle"],
            "resource":resource,
            "max_amount_minor":1,
            "currency":"OID",
            "beneficiary":"1".repeat(40),
        }),
        "fixture-admin-token",
    );
    let action: Value = serde_json::from_slice(&trust(
        "request",
        json!({
            "schema_version":1,
            "event_id":format!("void-request-{}", std::process::id()),
            "mandate_id":mandate_id,
            "resource":resource,
            "amount_minor":1,
            "currency":"OID",
            "beneficiary":"1".repeat(40),
            "idempotency_key":format!("void-key-{mandate_id}"),
        }),
        "fixture-requester-token",
    )
    .bytes)
    .unwrap();
    let approved: Value = serde_json::from_slice(&trust(
        "approve",
        json!({
            "schema_version":1,
            "event_id":format!("void-approve-{}", std::process::id()),
            "action_id":action["id"],
            "mandate_id":mandate_id,
            "content_digest":action["content_digest"],
            "resource":resource,
            "amount_minor":1,
            "currency":"OID",
            "beneficiary":"1".repeat(40),
        }),
        "fixture-approver-token",
    )
    .bytes)
    .unwrap();
    assert_eq!(approved["state"], "approved");
    // The agreement is captured but a consumption now would still need the
    // engine chain; first prove the guard is satisfied by asserting the
    // engine observation directly, then apply a real public revision change.
    let guard_bytes = swarm_protection_contract::canonicalize(
        &proposal(&repository_id, &candidate),
        &context(owned.trust_revision),
    )
    .unwrap()
    .bytes()
    .to_vec();
    assert!(owned
        .project
        .paea_git_agreement_match(&guard_bytes)
        .unwrap()
        .is_some());
    // Real public configuration change on the project root (same
    // reconstruction the browser staleness phase uses): the captured
    // agreement becomes stale at last use.
    let view: Value =
        serde_json::from_slice(&owned.project.trust("show", b"{}", "").unwrap().bytes).unwrap();
    let mut identities = view["identities"].clone();
    for identity in identities.as_array_mut().unwrap() {
        identity["disabled"] = false.into();
        // The enrolled identity keeps its principal mapping and gets no token;
        // bearer identities get their exact fixture tokens back (the public
        // view omits raw tokens). Unknown subjects are dropped rather than
        // invented — an invalid empty token would refuse the whole change.
        if identity["subject"] == "requester" {
            identity["token"] = "fixture-requester-token".into();
        } else if identity["subject"] == "approver" {
            identity["token"] = "fixture-approver-token".into();
        } else if identity["subject"] == "agreement-admin" {
            identity["token"] = "fixture-admin-token".into();
        } else if identity.get("enrollment_principal").is_none() {
            identity["token"] = "".into();
        }
    }
    let identities: Vec<Value> = identities
        .as_array()
        .unwrap()
        .iter()
        .filter(|identity| {
            identity.get("enrollment_principal").is_some()
                || identity["token"].as_str().is_some_and(|t| t.len() >= 16)
        })
        .cloned()
        .collect();
    let change = serde_json::json!({
        "schema_version":1,
        "event_id":format!("agreement-void-stale-{}", std::process::id()),
        "expected_revision":owned.trust_revision,
        "reason":"Explicit fixture revision change voiding the captured agreement",
        "policy":view["policy"],
        "identities":identities,
    });
    owned
        .project
        .trust("config-apply", &serde_json::to_vec(&change).unwrap(), "")
        .unwrap_or_else(|e| panic!("stale revision change refused: {}", e.code()));
    assert_eq!(
        current_trust_revision(&owned.project_path),
        owned.trust_revision + 1
    );
    // The stale agreement no longer matches; the route refuses with the same
    // typed code and nothing is consumed.
    assert!(owned
        .project
        .paea_git_agreement_match(&guard_bytes)
        .unwrap()
        .is_none());
    let reply = post(
        &listener,
        &s,
        "/api/v1/paea/git-call?call=transfer",
        exact,
        Some(good),
    );
    assert!(
        !reply.response.status().is_success(),
        "stale agreement must refuse: {}",
        String::from_utf8_lossy(reply.response.body())
    );
    assert_eq!(
        reply.failure.expect("typed refusal expected").code(),
        "paea-git-call-agreement"
    );
    let _ = s.close();
    let _ = owned.project.close();
    let _ = owned.service.close();
    fixture.close();
}

/// The approver journey for the PA08 terms exposure: an approver reads the
/// engine-derived exact terms from the preview route, then captures the
/// agreement through the EXISTING Trust producers of the PROJECT root (the
/// only root where the business identities are configured — the HTTP
/// trust-actions route serves the service root and refuses a business bearer
/// with `authorization_required`, observed below as an explicit boundary, not
/// silently assumed). The captured agreement is exactly the one the
/// consumption guard recognizes. Negative branches stay visible refusals with
/// no Git effect: wrong role (denied by the existing approve guard), stale
/// terms (rejected as non-exact), wrong candidate (no match). The preview
/// itself never captures consent; consumption is never attempted here. Auth
/// simulated on isolated fixtures (labeled bearer tokens); no human presence
/// or acceptance is claimed.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn approver_ux_preview_terms_then_existing_trust_channel_captures_agreement() {
    let owned = owned_agreement();
    let fixture = fixture::Fixture::build_for_trust(
        owned.prepared.request().permission(),
        owned.fixture_scope.clone(),
        owned.trust_revision,
    );
    let (repository_id, candidate) = (owned.repository_id.clone(), owned.candidate.clone());
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
    .unwrap_or_else(|e| panic!("exact bind refused: {}", e.code()));
    let bearer = |token: &str| format!("Bearer {token}");

    // BLP reader frontier (BLP-RET-05): the preview is served to an authorized
    // Bearer reader only. The policy and the attribution of the exact
    // canonical bytes are produced through the same supported producers as the
    // reader-guard test, before the approver's read below.
    let blp_revision = {
        let show = owned
            .project
            .trust("blp-show", b"{}".as_slice(), "fixture-admin-token")
            .unwrap_or_else(|e| panic!("blp-show refused: {}", e.code()));
        serde_json::from_slice::<Value>(&show.bytes).unwrap()["record"]["history"]
            .as_array()
            .and_then(|h| h.last())
            .and_then(|e| e["revision"].as_u64())
            .unwrap_or(0)
    };
    let current_revision = current_trust_revision(&owned.project_path);
    owned
        .project
        .trust(
            "blp-policy-apply",
            &serde_json::to_vec(&json!({
                "schema_version":1,
                "event_id":format!("ux-preview-policy-{current_revision}"),
                "expected_revision":blp_revision,
                "expected_trust_revision":current_revision,
                "reason":"Reader-guard policy for the approver journey preview",
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
            }))
            .unwrap(),
            "fixture-admin-token",
        )
        .unwrap_or_else(|e| panic!("blp-policy-apply refused: {}", e.code()));
    owned
        .project
        .trust(
            "blp-object-attribute",
            &serde_json::to_vec(&json!({
                "schema_version":1,
                "event_id":format!("ux-preview-attribute-{current_revision}"),
                "expected_revision":blp_revision + 1,
                "expected_trust_revision":current_revision,
                "canonical_digest":swarm_protection_contract::canonicalize(
                    &proposal(&repository_id, &candidate),
                    &context(owned.trust_revision),
                ).unwrap().digest(),
                "project_path":owned.project_path.to_str().unwrap(),
                "classification":{"level":"classified","compartments":[]},
                "reason":"Reader-guard attribution for the approver journey preview",
            }))
            .unwrap(),
            "fixture-admin-token",
        )
        .unwrap_or_else(|e| panic!("blp-object-attribute refused: {}", e.code()));

    // Step 1: the approver opens the preview and reads the engine-derived
    // terms (no consent captured, no consumption).
    let reply = get_with(
        &listener,
        &s,
        "/api/v1/paea/git-call/preview",
        Some(bearer("fixture-admin-token").as_str()),
    );
    assert!(reply.response.status().is_success());
    let preview: Value = serde_json::from_slice(reply.response.body()).unwrap();
    assert_eq!(preview["available"], true);
    let terms = preview["agreement_terms"].clone();
    assert_eq!(terms["binding"], "recipe");
    let resource = terms["resource"].as_str().unwrap().to_owned();
    let amount = terms["amount_minor"].as_i64().unwrap();
    let currency = terms["currency"].as_str().unwrap().to_owned();
    let beneficiary = terms["beneficiary"].as_str().unwrap().to_owned();
    assert_eq!(
        resource,
        format!("fixture-project/{repository_id}/refs/heads/release/{candidate}")
    );

    // Step 2: the requester creates the mandate and the action through the
    // same existing Trust channel (the preview does not do it for anyone).
    let trust = |op: &str, value: Value, token: &str| {
        owned
            .project
            .trust(op, &serde_json::to_vec(&value).unwrap(), token)
            .unwrap_or_else(|e| panic!("{op} refused: {}", e.code()))
    };
    let mandate_id = format!("ux-mandate-{}", std::process::id());
    trust(
        "mandate-create",
        json!({
            "schema_version":1,
            "event_id":format!("ux-mandate-create-{mandate_id}"),
            "mandate_id":mandate_id,
            "subject":"requester",
            "actions":["settle"],
            "resource":resource,
            "max_amount_minor":1,
            "currency":currency,
            "beneficiary":beneficiary,
        }),
        "fixture-admin-token",
    );
    let action: Value = serde_json::from_slice(
        &trust(
            "request",
            json!({
                "schema_version":1,
                "event_id":format!("ux-request-{mandate_id}"),
                "mandate_id":mandate_id,
                "resource":resource,
                "amount_minor":amount,
                "currency":currency,
                "beneficiary":beneficiary,
                "idempotency_key":format!("ux-key-{mandate_id}"),
            }),
            "fixture-requester-token",
        )
        .bytes,
    )
    .unwrap();
    assert_eq!(action["state"], "pending_approval");

    // Step 3 (boundary): the HTTP trust-actions route serves the SERVICE root,
    // not the prepared project root — a business bearer is refused before any
    // business identity lookup. This explicit boundary is observed here and
    // routed for the NEXT tranche (consent integration); no silent fallback.
    let http_refused = post(
        &listener,
        &s,
        "/api/v1/trust-actions?action=approve",
        serde_json::to_vec(&json!({
            "schema_version":1,
            "event_id":format!("ux-wrong-role-{mandate_id}"),
            "action_id":action["id"],
            "mandate_id":mandate_id,
            "content_digest":action["content_digest"],
            "resource":resource,
            "amount_minor":amount,
            "currency":currency,
            "beneficiary":beneficiary,
        }))
        .unwrap(),
        Some(bearer("fixture-requester-token")),
    );
    assert!(
        !http_refused.response.status().is_success(),
        "service-root approve must not accept a business bearer"
    );
    let refused: Value = serde_json::from_slice(http_refused.response.body()).unwrap();
    assert_eq!(refused["error"], "authorization_required");

    // Step 4 (negative, real producer): a wrong role through the existing
    // approve guard on the project root is a visible denial — no agreement.
    let denied: Value = serde_json::from_slice(
        &trust(
            "approve",
            json!({
                "schema_version":1,
                "event_id":format!("ux-wrong-role-root-{mandate_id}"),
                "action_id":action["id"],
                "mandate_id":mandate_id,
                "content_digest":action["content_digest"],
                "resource":resource,
                "amount_minor":amount,
                "currency":currency,
                "beneficiary":beneficiary,
            }),
            "fixture-requester-token",
        )
        .bytes,
    )
    .unwrap();
    assert_eq!(denied["state"], "denied");

    // Step 5 (negative, real producer): stale terms (amount drifted from the
    // preview) are rejected by the existing exact-terms guard on the project
    // root — still no agreement.
    let drifted: Value = serde_json::from_slice(
        &trust(
            "approve",
            json!({
                "schema_version":1,
                "event_id":format!("ux-drift-root-{mandate_id}"),
                "action_id":action["id"],
                "mandate_id":mandate_id,
                "content_digest":action["content_digest"],
                "resource":resource,
                "amount_minor":amount + 1,
                "currency":currency,
                "beneficiary":beneficiary,
            }),
            "fixture-approver-token",
        )
        .bytes,
    )
    .unwrap();
    assert_eq!(drifted["state"], "denied");
    // No agreement yet: the consumption would refuse (never attempted here —
    // the engine observation is the guard's own lookup).
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

    // Step 6: the approver captures the agreement through the existing Trust
    // producers of the project root, with the preview-derived terms verbatim.
    let approved: Value = serde_json::from_slice(
        &trust(
            "approve",
            json!({
                "schema_version":1,
                "event_id":format!("ux-approve-{mandate_id}"),
                "action_id":action["id"],
                "mandate_id":mandate_id,
                "content_digest":action["content_digest"],
                "resource":resource,
                "amount_minor":amount,
                "currency":currency,
                "beneficiary":beneficiary,
            }),
            "fixture-approver-token",
        )
        .bytes,
    )
    .unwrap();
    assert_eq!(approved["state"], "approved");
    assert_eq!(approved["approver"], "approver");

    // Step 6: the captured agreement is EXACTLY the one the consumption guard
    // recognizes for this action/revision — one source of truth.
    assert!(owned
        .project
        .paea_git_agreement_match(&exact)
        .unwrap()
        .is_some());
    // Wrong candidate never matches.
    let other = swarm_protection_contract::canonicalize(
        &proposal(&repository_id, &"e".repeat(40)),
        &context(owned.trust_revision),
    )
    .unwrap()
    .bytes()
    .to_vec();
    assert!(owned
        .project
        .paea_git_agreement_match(&other)
        .unwrap()
        .is_none());

    let _ = s.close();
    let _ = owned.project.close();
    let _ = owned.service.close();
    fixture.close();
}

/// PA08 approval channel end-to-end on the retained native source fixture.
/// One exact business `approve` command is frozen at startup through the real
/// `prepare_approval` producer (fresh project-root physical witness), signed
/// into the operator's enrollment TOGETHER WITH the one-shot Git-call
/// permission, bound via `open_git_call_with_approval`, and executed once
/// through `POST /api/v1/paea/git-call/approval` under the held enrolled
/// authority. The project guard then recognizes the exact agreement; the
/// SERVICE root trust state stays byte-identical throughout. Refusals stay
/// typed with no effect on either store: wrong credential, any non-empty body
/// (the command is held server-side, so every body is refused before any Root
/// open), a stale revision after a real public `config-apply` on the project
/// root. The exact replay of the same approved command is the engine's
/// retained reply of the same original, never a second effect. Auth simulated
/// on isolated fixtures (labeled enrollment password); no human presence,
/// acceptance or Git effect is claimed anywhere in this test.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn approval_route_captures_project_agreement_and_refuses_wrong_body_revision() {
    let owned = owned_agreement();
    let (repository_id, candidate) = (owned.repository_id.clone(), owned.candidate.clone());
    // The exact canonical action bytes the preparation digested; the frozen
    // approve command carries the engine-derived recipe terms for them.
    let exact = swarm_protection_contract::canonicalize(
        &proposal(&repository_id, &candidate),
        &context(owned.trust_revision),
    )
    .unwrap()
    .bytes()
    .to_vec();
    let terms = owned
        .project
        .paea_git_agreement_terms(&exact)
        .unwrap()
        .expect("recipe terms derivable for this canonical Git action");
    assert_eq!(terms.binding, "recipe");
    assert_eq!(terms.revision, owned.trust_revision);
    let resource = terms.resource.clone();
    // Frozen command chain through the real producers: mandate-create (admin
    // bearer), request (requester bearer) — then approve only via the route.
    let mandate_id = format!("approval-mandate-{}", std::process::id());
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
    trust(
        "mandate-create",
        json!({
            "schema_version":1,
            "event_id":format!("approval-mandate-{mandate_id}"),
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
            "event_id":format!("approval-request-{mandate_id}"),
            "mandate_id":mandate_id,
            "resource":resource,
            "amount_minor":terms.amount_minor,
            "currency":terms.currency,
            "beneficiary":terms.beneficiary,
            "idempotency_key":format!("approval-key-{mandate_id}"),
        }),
        "fixture-requester-token",
    );
    assert_eq!(action["state"], "pending_approval");
    // The frozen approve command: exact terms of the pending action.
    let approve = serde_json::to_vec(&json!({
        "schema_version":1,
        "event_id":format!("approval-approve-{mandate_id}"),
        "action_id":action["id"],
        "mandate_id":mandate_id,
        "content_digest":action["content_digest"],
        "resource":resource,
        "amount_minor":terms.amount_minor,
        "currency":terms.currency,
        "beneficiary":terms.beneficiary,
    }))
    .unwrap();
    // Freeze the approval against the source's own project root BEFORE open:
    // the exact trust-action permission derived from these bytes is what the
    // operator provisions into the same signed enrollment as the Git-call
    // permission — no second account, no second agreement system.
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
    // One held account signed with BOTH permissions at bootstrap (the journal
    // accepts one enroll per enrollment id, so nothing can be appended later);
    // the same enrolled principal maps to the single identity holding
    // approver+effect roles on this root.
    let mut fixture = fixture::Fixture::build_for_trust_actions(
        vec![
            owned.prepared_request_permission.clone(),
            approval_source.request().permission(),
        ],
        owned.fixture_scope.clone(),
        owned.trust_revision,
    );
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let authority = swarm_web::PaeaGitCallSource::open(
        source,
        fixture.path.to_str().unwrap(),
        &fixture.pin,
    )
    .unwrap();
    let s = swarm_web::NativeHttpService::open_git_call_with_approval(
        &owned.service_path,
        LocalWebAuthority::from_listener(&listener, KEY).unwrap(),
        authority,
        Some(approval_source),
    )
    .unwrap_or_else(|e| panic!("approval bind refused: {:?}", e.code()));
    let good = format!("Enrollment {}", URL_SAFE_NO_PAD.encode(PASSWORD));
    let wrong = format!("Enrollment {}", URL_SAFE_NO_PAD.encode(b"wrong-password"));
    // Startup observation: the approval status serves ONLY the `bound`
    // boolean — the capability stays Null, because the selected operation
    // (even inside a closed vocabulary) is derived from the frozen classified
    // action and no declassification right is created (BLP-RET-05). The exact
    // operation and command digest stay behind the guarded presentation route.
    let status = get(&listener, &s, "/api/v1/paea/git-call/approval");
    assert!(status.response.status().is_success());
    assert!(status.failure.is_none());
    let status_body: Value = serde_json::from_slice(status.response.body()).unwrap();
    assert_eq!(status_body["event"], "paea-git-call-approval-status");
    assert_eq!(status_body["bound"], true);
    assert_eq!(
        status_body["capability"], serde_json::Value::Null,
        "approval status must serve no request-derived capability: {status_body}"
    );
    // The service root trust state before any approval: observed, not assumed.
    let service_before = service_trust_state(&owned.service_path);
    // Wrong credential refuses typed before any business effect: the decoded
    // password is non-empty so the shape gate passes, and the held personal
    // authority itself refuses with the engine's typed pre-use code (403),
    // consuming nothing.
    let refused = post(
        &listener,
        &s,
        "/api/v1/paea/git-call/approval",
        Vec::new(),
        Some(wrong),
    );
    assert!(!refused.response.status().is_success());
    assert_eq!(
        refused.failure.expect("typed refusal").code(),
        "authorization_required"
    );
    // A non-empty body refuses typed before any Root open: the command is
    // held server-side, so no transportable bytes can ever match it. A
    // drifted-terms body is exactly such a refusal case.
    let drifted = serde_json::to_vec(&json!({
        "schema_version":1,
        "event_id":format!("approval-drift-{mandate_id}"),
        "action_id":action["id"],
        "mandate_id":mandate_id,
        "content_digest":action["content_digest"],
        "resource":resource,
        "amount_minor":terms.amount_minor + 1,
        "currency":terms.currency,
        "beneficiary":terms.beneficiary,
    }))
    .unwrap();
    let wrong_body = post(
        &listener,
        &s,
        "/api/v1/paea/git-call/approval",
        drifted,
        Some(good.clone()),
    );
    assert!(!wrong_body.response.status().is_success());
    assert_eq!(
        wrong_body.failure.expect("typed refusal").code(),
        "paea-git-call-approval-request"
    );
    // No approval exists yet: the guard sees nothing for these exact bytes.
    assert!(owned
        .project
        .paea_git_agreement_match(&exact)
        .unwrap()
        .is_none());
    // Positive witness: the empty-body consumption under the held enrolled
    // authority consumes the server-held approve command and captures the
    // agreement on the project root.
    let ok = post(
        &listener,
        &s,
        "/api/v1/paea/git-call/approval",
        Vec::new(),
        Some(good.clone()),
    );
    assert!(
        ok.response.status().is_success(),
        "approval must succeed: {}",
        String::from_utf8_lossy(ok.response.body())
    );
    assert!(ok.failure.is_none());
    let published: Value = serde_json::from_slice(ok.response.body()).unwrap();
    // The enrolled approver is I3 and the action object is I2: Biba read-up is
    // refused, so the public reply is the engine's status acknowledgement
    // (state + object_id), not the action's fields. The durable approver is
    // observed through the project guard below, never through this body.
    assert_eq!(published["state"], "approved");
    assert_eq!(published["kind"], "trust-status-ack-v1");
    assert_eq!(published["content_redacted"], true);
    // The project guard now recognizes the exact agreement: one source of truth.
    let matched = owned
        .project
        .paea_git_agreement_match(&exact)
        .unwrap()
        .expect("exact agreement recognized by the project guard");
    assert_eq!(matched.action_id, action["id"].as_str().unwrap());
    assert_eq!(matched.approver, "enrolled-effect");
    // The service root trust state is byte-identical: this channel never
    // touched the service root's trust journal.
    assert_eq!(service_before, service_trust_state(&owned.service_path));
    // Legitimate replay: the same empty-body consumption replays the retained
    // original — success, and the durable agreement state stays identical.
    let replay = post(
        &listener,
        &s,
        "/api/v1/paea/git-call/approval",
        Vec::new(),
        Some(good.clone()),
    );
    assert!(replay.response.status().is_success());
    assert_eq!(service_before, service_trust_state(&owned.service_path));
    // Refusal: stale revision. A real public config change on the project root
    // (the same reconstruction the staleness tests apply) voids the captured
    // agreement; the same frozen command then refuses without any new approval
    // or Git effect.
    let view: Value =
        serde_json::from_slice(&owned.project.trust("show", b"{}", "").unwrap().bytes).unwrap();
    let mut identities = view["identities"].clone();
    for identity in identities.as_array_mut().unwrap() {
        identity["disabled"] = false.into();
        identity["token"] = match identity["subject"].as_str() {
            Some("requester") => "fixture-requester-token".into(),
            Some("approver") => "fixture-approver-token".into(),
            Some("agreement-admin") => "fixture-admin-token".into(),
            _ => Value::String(String::new()),
        };
    }
    let change = serde_json::json!({
        "schema_version":1,
        "event_id":format!("approval-stale-{}", std::process::id()),
        "expected_revision":owned.trust_revision,
        "reason":"Explicit fixture Trust revision change voiding the captured approval agreement",
        "policy":view["policy"],
        "identities":identities,
    });
    owned
        .project
        .trust("config-apply", &serde_json::to_vec(&change).unwrap(), "")
        .unwrap_or_else(|e| panic!("stale revision change refused: {}", e.code()));
    assert!(owned
        .project
        .paea_git_agreement_match(&exact)
        .unwrap()
        .is_none());
    let stale = post(
        &listener,
        &s,
        "/api/v1/paea/git-call/approval",
        Vec::new(),
        Some(good.clone()),
    );
    assert!(
        !stale.response.status().is_success(),
        "stale revision must refuse: {}",
        String::from_utf8_lossy(stale.response.body())
    );
    assert_eq!(service_before, service_trust_state(&owned.service_path));
    // Legitimate recovery at the new revision: fresh recipe terms derive for
    // the same canonical bytes, so a fresh preparation could re-bind and
    // re-approve. The channel imposes no hidden one-strike lock.
    let fresh_terms = owned
        .project
        .paea_git_agreement_terms(&exact)
        .unwrap()
        .expect("terms re-derivable at the new revision");
    assert_eq!(fresh_terms.revision, owned.trust_revision + 1);
    let _ = s.close();
    let _ = owned.project.close();
    let _ = owned.service.close();
    fixture.close();
}

/// The approval presentation route serves only the authorized projection
/// behind the ordinary session gate: the public operation, the request digest,
/// the canonical identity of the underlying Git action, the engine-derived
/// agreement terms and the current observed revision/witness state. The frozen
/// command bytes and their private fields (action_id, mandate_id,
/// content_digest) are NEVER served — the enrolled approver (I3) holds no Biba
/// read right over the I2 object, the same redaction the engine already
/// applies to its reply — so the browser never transports command bytes: the
/// POST route consumes the command held server-side with an empty body. After
/// a real approval through the POST route, the presentation stays the same
/// read-only observation — never a second approval system. Without a bound
/// approval it stays an explicit `available:false` 200. The held command then
/// really consumes once through the POST route: the ack is the engine's, the
/// agreement is recognized by the project guard, and a non-empty body still
/// refuses typed before any Root open. A Biba read-object discriminant with
/// the fixture's real roles proves the disclosure asymmetry the projection
/// relies on: I2 reads its object, I3 is refused, and the served projection
/// stays within the public preview class.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn approval_presentation_serves_frozen_command_and_terms() {
    let owned = owned_agreement();
    let (repository_id, candidate) = (owned.repository_id.clone(), owned.candidate.clone());
    let exact = swarm_protection_contract::canonicalize(
        &proposal(&repository_id, &candidate),
        &context(owned.trust_revision),
    )
    .unwrap()
    .bytes()
    .to_vec();
    let terms = owned
        .project
        .paea_git_agreement_terms(&exact)
        .unwrap()
        .expect("recipe terms derivable for this canonical Git action");
    let resource = terms.resource.clone();
    let mandate_id = format!("approval-presentation-mandate-{}", std::process::id());
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
    // BLP reader frontier (BLP-RET-05): the presentation projection is now
    // served ONLY to a Bearer reader under a CURRENT, control-produced
    // attribution of the exact canonical bytes. The baseline produces the
    // policy and attribution through the same supported producers as the
    // reader-guard test, then reads with the authorized credential; the
    // approval POST path below stays untouched (bare Enrollment credential).
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
            "event_id":format!("approval-presentation-policy-{}", current_trust_revision(&owned.project_path)),
            "expected_revision":blp_revision,
            "expected_trust_revision":current_trust_revision(&owned.project_path),
            "reason":"Reader-guard policy for the approval presentation baseline",
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
    trust(
        "blp-object-attribute",
        json!({
            "schema_version":1,
            "event_id":format!("approval-presentation-attribute-{}", current_trust_revision(&owned.project_path)),
            "expected_revision":blp_revision + 1,
            "expected_trust_revision":current_trust_revision(&owned.project_path),
            "canonical_digest":swarm_protection_contract::canonicalize(
                &proposal(&repository_id, &candidate),
                &context(owned.trust_revision),
            ).unwrap().digest(),
            "project_path":owned.project_path.to_str().unwrap(),
            "classification":{"level":"classified","compartments":[]},
            "reason":"Reader-guard attribution for the approval presentation baseline",
        }),
        "fixture-admin-token",
    );
    const READER: &str = "Bearer fixture-admin-token";
    trust(
        "mandate-create",
        json!({
            "schema_version":1,
            "event_id":format!("approval-presentation-mandate-{mandate_id}"),
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
            "event_id":format!("approval-presentation-request-{mandate_id}"),
            "mandate_id":mandate_id,
            "resource":resource,
            "amount_minor":terms.amount_minor,
            "currency":terms.currency,
            "beneficiary":terms.beneficiary,
            "idempotency_key":format!("approval-presentation-key-{mandate_id}"),
        }),
        "fixture-requester-token",
    );
    assert_eq!(action["state"], "pending_approval");
    let approve = serde_json::to_vec(&json!({
        "schema_version":1,
        "event_id":format!("approval-presentation-approve-{mandate_id}"),
        "action_id":action["id"],
        "mandate_id":mandate_id,
        "content_digest":action["content_digest"],
        "resource":resource,
        "amount_minor":terms.amount_minor,
        "currency":terms.currency,
        "beneficiary":terms.beneficiary,
    }))
    .unwrap();
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
    let mut fixture = fixture::Fixture::build_for_trust_actions(
        vec![
            owned.prepared_request_permission.clone(),
            approval_source.request().permission(),
        ],
        owned.fixture_scope.clone(),
        owned.trust_revision,
    );
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let authority = swarm_web::PaeaGitCallSource::open(
        source,
        fixture.path.to_str().unwrap(),
        &fixture.pin,
    )
    .unwrap();
    let s = swarm_web::NativeHttpService::open_git_call_with_approval(
        &owned.service_path,
        LocalWebAuthority::from_listener(&listener, KEY).unwrap(),
        authority,
        Some(approval_source),
    )
    .unwrap_or_else(|e| panic!("approval bind refused: {:?}", e.code()));
    let good = format!("Enrollment {}", URL_SAFE_NO_PAD.encode(PASSWORD));
    // Shape refusals first: POST and non-empty query refuse typed before any
    // Root open, exactly like the preview route.
    let wrong_method = post(
        &listener,
        &s,
        "/api/v1/paea/git-call/approval/presentation",
        Vec::new(),
        None,
    );
    assert!(!wrong_method.response.status().is_success());
    assert_eq!(
        wrong_method.failure.expect("typed refusal").code(),
        "web-method-refused"
    );
    let wrong_query = {
        let a = LocalWebAuthority::from_listener(&listener, KEY).unwrap();
        let builder = Request::builder()
            .method("GET")
            .uri("/api/v1/paea/git-call/approval/presentation?x=1")
            .header(HOST, a.host())
            .header(COOKIE, format!("{}={}", a.cookie_name(), KEY))
            .header(ORIGIN, format!("http://{}", a.host()))
            .header("X-Swarm-CSRF", KEY);
        s.handle(builder.body(Vec::new()).unwrap()).unwrap()
    };
    assert!(!wrong_query.response.status().is_success());
    assert_eq!(
        wrong_query.failure.expect("typed refusal").code(),
        "paea-git-call-approval-presentation-query-invalid"
    );
    // Bound presentation: the authorized projection only — the operation, the
    // digest identity, the canonical identity and the engine-derived terms.
    // The reader credential is required by the BLP reader frontier.
    let presentation = get_with(
        &listener,
        &s,
        "/api/v1/paea/git-call/approval/presentation",
        Some(READER),
    );
    assert!(
        presentation.response.status().is_success(),
        "presentation must serve: {}",
        String::from_utf8_lossy(presentation.response.body())
    );
    assert!(presentation.failure.is_none());
    let body: Value = serde_json::from_slice(presentation.response.body()).unwrap();
    assert_eq!(body["event"], "paea-git-call-approval-presentation");
    assert_eq!(body["available"], true);
    assert_eq!(body["operation"], "approve");
    assert_eq!(body["request_digest"].as_str().unwrap().len(), 64);
    // The private command identity is NEVER served: no bytes, no parsed
    // fields — the same disclosure class the engine's reply redaction gives
    // the I3 approver.
    assert!(body.get("command_bytes_b64url").is_none());
    assert!(body.get("command").is_none());
    assert!(body
        .to_string()
        .contains(action["id"].as_str().unwrap()) == false);
    assert!(body.to_string().contains(&mandate_id) == false);
    assert!(body.to_string().contains(action["content_digest"].as_str().unwrap()) == false);
    assert_eq!(body["canonical"]["digest_matches"], true);
    assert_eq!(body["canonical"]["action_digest"].as_str().unwrap().len(), 64);
    // The derived terms are the engine's own, for the underlying action, and
    // derive from the public canonical proposal bytes the preview serves.
    assert_eq!(body["agreement_terms"]["binding"], "recipe");
    assert_eq!(body["agreement_terms"]["resource"], resource.as_str());
    assert_eq!(body["agreement_terms"]["amount_minor"], terms.amount_minor);
    assert_eq!(body["agreement_terms"]["currency"], terms.currency);
    assert_eq!(body["agreement_terms"]["beneficiary"], terms.beneficiary);
    assert_eq!(body["agreement_terms"]["revision"], owned.trust_revision);
    assert_eq!(body["current"]["roots_bound"], true);
    assert_eq!(body["current"]["declared_trust_revision"], owned.trust_revision);
    assert_eq!(body["current"]["trust_revision_current"], true);
    assert_eq!(body["limits"]["consent"], "none-captured");
    assert_eq!(body["limits"]["consumption"], "none");
    assert_eq!(body["limits"]["human_presence"], "not-established");
    // No agreement exists yet: presentation captures nothing.
    assert!(owned
        .project
        .paea_git_agreement_match(&exact)
        .unwrap()
        .is_none());
    // Real consumption through the POST route with an EMPTY body: the command
    // is held server-side (the bound approval), so the browser transports
    // only the credential — nothing served is re-POSTed.
    let ok = post(&listener, &s, "/api/v1/paea/git-call/approval", Vec::new(), Some(good.clone()));
    assert!(
        ok.response.status().is_success(),
        "approval must succeed: {}",
        String::from_utf8_lossy(ok.response.body())
    );
    let published: Value = serde_json::from_slice(ok.response.body()).unwrap();
    assert_eq!(published["state"], "approved");
    assert_eq!(published["kind"], "trust-status-ack-v1");
    assert_eq!(published["content_redacted"], true);
    let matched = owned
        .project
        .paea_git_agreement_match(&exact)
        .unwrap()
        .expect("exact agreement recognized by the project guard");
    assert_eq!(matched.action_id, action["id"].as_str().unwrap());
    assert_eq!(matched.approver, "enrolled-effect");
    // After consumption the presentation stays the same read-only observation:
    // still the same digest identity, still no second approval system. The
    // reader credential stays required after the real approval effect.
    let after = get_with(
        &listener,
        &s,
        "/api/v1/paea/git-call/approval/presentation",
        Some(READER),
    );
    assert!(after.response.status().is_success());
    let after_body: Value = serde_json::from_slice(after.response.body()).unwrap();
    assert_eq!(after_body["available"], true);
    assert_eq!(
        after_body["request_digest"], body["request_digest"],
        "same frozen command after consumption"
    );
    // A non-empty body still refuses typed after the presentation existed:
    // the channel never accepts request data again.
    let wrong_body = post(
        &listener,
        &s,
        "/api/v1/paea/git-call/approval",
        serde_json::to_vec(&json!({"schema_version":1,"resource":resource})).unwrap(),
        Some(good),
    );
    assert!(!wrong_body.response.status().is_success());
    assert_eq!(
        wrong_body.failure.expect("typed refusal").code(),
        "paea-git-call-approval-request"
    );
    // Biba read-object discriminant with the fixture's REAL roles on the same
    // project root: the I2 approver token reads the pending action object,
    // the I3 enrolled approver (projection consumer) is refused — proving the
    // projection is not a new read-down channel. No event is journaled by
    // these reads, so the stale config-apply below stays exactly aligned.
    let project_state = service_trust_state(&owned.project_path);
    let project = RootStore::open(&owned.project_path, false).unwrap();
    let read_cmd = json!({
        "schema_version":1,
        "event_id":format!("approval-presentation-read-{}", std::process::id()),
        "action_id":action["id"],
    });
    let visible = project
        .trust(
            "read-object",
            &serde_json::to_vec(&read_cmd).unwrap(),
            "fixture-approver-token",
        )
        .expect("I2 approver reads its own object");
    let seen: Value = serde_json::from_slice(&visible.bytes).unwrap();
    assert_eq!(seen["amount_minor"], terms.amount_minor);
    assert_eq!(
        project
            .trust(
                "read-object",
                &serde_json::to_vec(&read_cmd).unwrap(),
                "fixture-admin-token",
            )
            .err()
            .expect("I3 read-down must refuse")
            .code(),
        "authorization_required"
    );
    drop(project);
    assert_eq!(
        project_state,
        service_trust_state(&owned.project_path),
        "read-object must not journal anything"
    );
    let _ = s.close();
    let _ = owned.project.close();
    let _ = owned.service.close();
    fixture.close();
}

/// The presentation route without a bound approval is an explicit
/// `available:false` 200 with a typed reason, and without any bound source it
/// never refuses: absence is observable without any consumption. Shape
/// refusals (POST, query) still refuse typed independent of the credential.
#[test]
fn approval_presentation_reports_explicit_absence_without_source() {
    let (service_dir, service_root) = service();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let s = NativeHttpService::open(
        service_dir.path(),
        LocalWebAuthority::from_listener(&listener, KEY).unwrap(),
    )
    .unwrap();
    let reply = get(&listener, &s, "/api/v1/paea/git-call/approval/presentation");
    assert!(reply.response.status().is_success());
    assert!(reply.failure.is_none());
    let body: Value = serde_json::from_slice(reply.response.body()).unwrap();
    assert_eq!(body["event"], "paea-git-call-approval-presentation");
    assert_eq!(body["available"], false);
    assert_eq!(
        body["reason"], "paea-git-call-approval-source-unavailable",
        "no source bound: the reason names the missing source"
    );
    s.close().unwrap();
    service_root.close().unwrap();
}

/// BLP reader frontier (BLP-RET-05) on a REAL bound approval: GET preview and
/// GET approval/presentation both serve the classified projection to an
/// authorized Bearer reader under a CURRENT, control-produced object
/// attribution bound to the exact canonical bytes, the project and the policy
/// revision — re-read at every consumption. The approval is the real
/// mandate/request/prepare_approval rig (same as
/// `approval_presentation_serves_frozen_command_and_terms`), so the
/// presentation route reaches its reader guard instead of answering the
/// no-source vacuity code; an `available:true` witness on BOTH routes with
/// the produced attribution is the POSITIVE result of this test, and no
/// `paea-git-call-approval-unavailable` reply counts as success anywhere.
/// Refusal segments: a bare session and an enrolled-but-uncleared reader
/// receive NO classified marker (exact source label, digest or request id)
/// with the stable guard codes on BOTH routes; a stale policy revision
/// refuses like an unknown one. Session alone is never authorization. The
/// complementary exact refusal matrix (unknown credential, supported
/// disablement with pre-observed staleness, absent/wrong-digest/wrong-project
/// attribution, no-mutation brackets) is coded as
/// `reader_refusal_matrix_on_bound_approval_both_routes`; the delivery
/// qualification (production ELF/assets, FR/EN notes, installation) remains
/// PENDING for the independent verifier.
#[test]
fn preview_reader_guard_enforces_produced_attribution() {
    let owned = owned_agreement();
    let (repository_id, candidate) = (owned.repository_id.clone(), owned.candidate.clone());
    let trust_revision = owned.trust_revision;
    // Policy produced by the supported producer BEFORE any guarded read; every
    // clearance principal is a real fixture identity (blp-principal check).
    // The current BLP revision and fixture Trust revision are both read live
    // through supported operations (blp-show / the journal's own revision), so
    // a retained fixture with an earlier recorded policy advances through the
    // real apply path (a stale recorded entry is never revived) and the
    // apply-once event id survives reruns.
    let policy = json!({
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
    });
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
    let trust_revision_of = |path: &Path| current_trust_revision(path);
    let blp_revision = trust("blp-show", json!({}), "fixture-admin-token")["record"]
        ["history"]
        .as_array()
        .and_then(|h| h.last())
        .and_then(|e| e["revision"].as_u64())
        .unwrap_or(0);
    let project_path = owned.project_path.clone();
    trust(
        "blp-policy-apply",
        json!({
            "schema_version":1,
            "event_id":format!("blp-reader-guard-policy-{}", trust_revision_of(&project_path)),
            "expected_revision":blp_revision,
            "expected_trust_revision":trust_revision_of(&project_path),
            "reason":"Reader-guard policy for the PAEA preview frontier fixture",
            "policy":policy,
        }),
        "fixture-admin-token",
    );
    // Distinctive classified marker carried by the declared source, so any
    // served proposal/context leak is observable client-side by exact string.
    const PRIVATE_MARKER: &str = "fixture-classified-marker-blp-ret-05";
    let mut classified_context = context(trust_revision);
    classified_context.sources[0].confidentiality_labels = vec![PRIVATE_MARKER.to_owned()];
    let exact = swarm_protection_contract::canonicalize(
        &proposal(&repository_id, &candidate),
        &classified_context,
    )
    .unwrap()
    .bytes()
    .to_vec();
    // The attribution is produced by the supported control operation (I3
    // admin, same gate as blp-policy-apply) bound to the EXACT canonical
    // bytes, the actual project root and the current policy revision; it is
    // pushed on the current entry so its policy revision is the applied one.
    let attribute = trust("blp-show", json!({}), "fixture-admin-token")["record"]
        ["history"]
        .as_array()
        .and_then(|h| h.last())
        .and_then(|e| e["revision"].as_u64())
        .unwrap_or(0);
    let attribute_reply = trust(
        "blp-object-attribute",
        json!({
            "schema_version":1,
            "event_id":format!("blp-reader-guard-attribute-{}", trust_revision_of(&project_path)),
            "expected_revision":attribute,
            "expected_trust_revision":trust_revision_of(&project_path),
            "canonical_digest":swarm_protection_contract::canonicalize(
                &proposal(&repository_id, &candidate),
                &classified_context,
            ).unwrap().digest(),
            "project_path":owned.project_path.to_str().unwrap(),
            "classification":{"level":"classified","compartments":[]},
            "reason":"Reader-guard attribution for the exact prepared action fixture",
        }),
        "fixture-admin-token",
    );
    assert_eq!(attribute_reply["attributed"], true, "supported producer must attribute");
    assert_eq!(attribute_reply["runtime_enforced"], false, "seam is explicit: no runtime enforcement beyond the served surface is claimed");
    // REAL approval rig (the same producers as
    // `approval_presentation_serves_frozen_command_and_terms`): the engine
    // derives the recipe terms from the EXACT classified canonical bytes, a
    // real mandate is created, the requester really files the action and the
    // frozen approve command is bound server-side via `prepare_approval`, so
    // BOTH guarded routes reach their reader guard with a real approval
    // instead of the presentation route's no-source vacuity reply.
    let terms = owned
        .project
        .paea_git_agreement_terms(&exact)
        .unwrap()
        .expect("recipe terms derivable for this canonical Git action");
    let resource = terms.resource.clone();
    let mandate_id = format!("approval-reader-guard-mandate-{}", std::process::id());
    trust(
        "mandate-create",
        json!({
            "schema_version":1,
            "event_id":format!("approval-reader-guard-mandate-{mandate_id}"),
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
            "event_id":format!("approval-reader-guard-request-{mandate_id}"),
            "mandate_id":mandate_id,
            "resource":resource,
            "amount_minor":terms.amount_minor,
            "currency":terms.currency,
            "beneficiary":terms.beneficiary,
            "idempotency_key":format!("approval-reader-guard-key-{mandate_id}"),
        }),
        "fixture-requester-token",
    );
    assert_eq!(action["state"], "pending_approval");
    let approve = serde_json::to_vec(&json!({
        "schema_version":1,
        "event_id":format!("approval-reader-guard-approve-{mandate_id}"),
        "action_id":action["id"],
        "mandate_id":mandate_id,
        "content_digest":action["content_digest"],
        "resource":resource,
        "amount_minor":terms.amount_minor,
        "currency":terms.currency,
        "beneficiary":terms.beneficiary,
    }))
    .unwrap();
    drop(trust);
    drop(trust_revision_of);
    let mut owned = owned;
    owned.prepared = owned
        .service
        .paea_git_transfer_call(
            &owned.project,
            &proposal(&repository_id, &candidate),
            &classified_context,
            &owned.work,
            "bare",
            PIN,
            "transfer-pack-reader-guard",
        )
        .unwrap_or_else(|e| panic!("classified preparation refused: {:?}", e));
    owned.prepared_request_permission = owned.prepared.request().permission();
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
    // BOTH permissions are signed at fixture construction time (one enroll per
    // enrollment id): the prepared transfer request AND the bound approval
    // request, exactly like the reference approval rig.
    let mut fixture = fixture::Fixture::build_for_trust_actions(
        vec![
            owned.prepared_request_permission.clone(),
            approval_source.request().permission(),
        ],
        owned.fixture_scope.clone(),
        trust_revision,
    );
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let authority = swarm_web::PaeaGitCallSource::open(
        source,
        fixture.path.to_str().unwrap(),
        &fixture.pin,
    )
    .unwrap();
    let s = swarm_web::NativeHttpService::open_git_call_with_approval(
        &owned.service_path,
        LocalWebAuthority::from_listener(&listener, KEY).unwrap(),
        authority,
        Some(approval_source),
    )
    .unwrap_or_else(|e| panic!("approval bind refused: {:?}", e.code()));
    // Bare session (no credential): the reader guard refuses BOTH routes with
    // the stable no-credential code and NO private marker. With the approval
    // now really bound, the presentation route reaches its guard instead of
    // the no-source vacuity reply — a real positive witness must never accept
    // `paea-git-call-approval-unavailable` as success.
    for uri in [
        "/api/v1/paea/git-call/preview",
        "/api/v1/paea/git-call/approval/presentation",
    ] {
        let body: Value = serde_json::from_slice(get(&listener, &s, uri).response.body()).unwrap();
        let raw = serde_json::to_string(&body).unwrap();
        assert!(!raw.contains(PRIVATE_MARKER), "{uri} served the marker to a bare session after attribution");
        assert_eq!(
            body["available"], false,
            "{uri} bare session must stay refused after attribution"
        );
        assert_eq!(
            body["reason"], "paea-reader-authentication-required",
            "{uri} bare session refuses with the stable no-credential code"
        );
    }
    // Enrolled but NOT cleared for the attributed classification: refusal
    // without any private marker, digest or identifier (payload, error, id)
    // on BOTH routes — the guard is reached before any projection field.
    for uri in [
        "/api/v1/paea/git-call/preview",
        "/api/v1/paea/git-call/approval/presentation",
    ] {
        let reply = get_with(
            &listener,
            &s,
            uri,
            Some("Bearer fixture-requester-token"),
        );
        let raw = String::from_utf8(reply.response.body().to_vec()).unwrap();
        assert!(!raw.contains(PRIVATE_MARKER), "{uri} served the marker to an uncleared reader");
        assert!(!raw.contains("action_digest"), "{uri} disclosed the classified digest to an uncleared reader");
        let body: Value = serde_json::from_slice(reply.response.body()).unwrap();
        assert_eq!(body["available"], false);
        assert_eq!(
            body["reason"], "paea-reader-access-denied",
            "{uri} uncleared enrolled reader refuses with the access-denied code"
        );
    }
    // POSITIVE witness on the preview route: the authorized reader
    // (classified clearance) with the Bearer credential reads the EXACT
    // permitted projection — full proposal/context with the marker, digest
    // match and a success state. Provenance from the produced attribution,
    // never from labels or cookies.
    {
        let uri = "/api/v1/paea/git-call/preview";
        let reply = get_with(&listener, &s, uri, Some("Bearer fixture-admin-token"));
        let raw = String::from_utf8(reply.response.body().to_vec()).unwrap();
        assert!(raw.contains(PRIVATE_MARKER), "{uri} authorized reader must receive the exact permitted projection");
        let body: Value = serde_json::from_slice(reply.response.body()).unwrap();
        assert_eq!(body["available"], true, "{uri} authorized reader reads the projection");
        assert_eq!(body["canonical"]["digest_matches"], true);
    }
    // POSITIVE witness on the approval/presentation route with the REAL bound
    // approval: the authorized reader reads the exact permitted projection —
    // the operation, the digest identity, the canonical identity and the
    // engine-derived terms — with NO private command field (no bytes, no
    // parsed fields, no action/mandate/digest identifier): the same
    // disclosure class the engine's reply redaction gives the I3 approver.
    {
        let uri = "/api/v1/paea/git-call/approval/presentation";
        let presentation = get_with(&listener, &s, uri, Some("Bearer fixture-admin-token"));
        assert!(
            presentation.response.status().is_success(),
            "presentation must serve: {}",
            String::from_utf8_lossy(presentation.response.body())
        );
        assert!(presentation.failure.is_none());
        let body: Value = serde_json::from_slice(presentation.response.body()).unwrap();
        assert_eq!(body["event"], "paea-git-call-approval-presentation");
        assert_eq!(body["available"], true, "{uri} the real bound approval is presented under the produced attribution");
        assert_eq!(body["operation"], "approve");
        assert_eq!(body["request_digest"].as_str().unwrap().len(), 64);
        assert!(body.get("command_bytes_b64url").is_none());
        assert!(body.get("command").is_none());
        assert!(body.to_string().contains(action["id"].as_str().unwrap()) == false);
        assert!(body.to_string().contains(&mandate_id) == false);
        assert!(body
            .to_string()
            .contains(action["content_digest"].as_str().unwrap()) == false);
        assert_eq!(body["canonical"]["digest_matches"], true);
        assert_eq!(body["canonical"]["action_digest"].as_str().unwrap().len(), 64);
        assert_eq!(body["agreement_terms"]["binding"], "recipe");
        assert_eq!(body["agreement_terms"]["resource"], resource.as_str());
        assert_eq!(body["agreement_terms"]["amount_minor"], terms.amount_minor);
        assert_eq!(body["agreement_terms"]["currency"], terms.currency);
        assert_eq!(body["agreement_terms"]["beneficiary"], terms.beneficiary);
        assert_eq!(body["agreement_terms"]["revision"], trust_revision);
        assert_eq!(body["current"]["roots_bound"], true);
        assert_eq!(body["current"]["declared_trust_revision"], trust_revision);
        assert_eq!(body["current"]["trust_revision_current"], true);
        assert_eq!(body["limits"]["consent"], "none-captured");
        assert_eq!(body["limits"]["consumption"], "none");
        assert_eq!(body["limits"]["human_presence"], "not-established");
        assert!(String::from_utf8_lossy(presentation.response.body()).contains(PRIVATE_MARKER) == false,
            "{uri} presentation never discloses the classified proposal/context marker");
    }
    // A REAL Trust configuration change (config-apply with a NEW event) moves
    // the recorded policy out of currency: the attribution rides the old
    // entry, so the guard refuses the same authorized reader on BOTH routes
    // without any marker — stale provenance is never revived and no label is
    // re-minted.
    owned
        .project
        .trust(
            "config-apply",
            &effect_config_named(
                &owned.project_path,
                "git-call-http-reader-guard-stale",
                true,
            ),
            "",
        )
        .unwrap_or_else(|e| panic!("stale config refused: {}", e.code()));
    for uri in [
        "/api/v1/paea/git-call/preview",
        "/api/v1/paea/git-call/approval/presentation",
    ] {
        let raw = String::from_utf8(
            get_with(&listener, &s, uri, Some("Bearer fixture-admin-token"))
                .response
                .body()
                .to_vec(),
        )
        .unwrap();
        assert!(!raw.contains(PRIVATE_MARKER), "{uri} served the marker after the policy lost currency");
        let body: Value = serde_json::from_slice(raw.as_bytes()).unwrap();
        assert_eq!(body["available"], false);
        assert_eq!(
            body["reason"], "paea-reader-policy-stale",
            "{uri} stale policy refuses with the stable stale code"
        );
    }
    let _ = s.close();
    let _ = owned.project.close();
    let _ = owned.service.close();
    fixture.close();
}

/// Complementary reader-refusal matrix (BLP-RET-05 lot): BOTH guarded GET
/// routes (`/api/v1/paea/git-call/preview` and
/// `/api/v1/paea/git-call/approval/presentation`) are exercised against a REAL
/// bound approval — the same rig as
/// `preview_reader_guard_enforces_produced_attribution` (real policy, real
/// `blp-object-attribute` attribution, real mandate/request/prepare_approval
/// chain) — and every refusal variant is preceded, on the SAME preconditions,
/// by a positive authorized witness (`available:true`, exact digest match), so
/// no refusal can be explained by a missing approval, a missing source or the
/// presentation route's no-source vacuity reply.
///
/// Refusal variants, each asserted with its exact observable code and with NO
/// classified marker, digest or private identifier (action id, mandate id,
/// content digest) anywhere in the reply, and never
/// `paea-git-call-approval-unavailable`:
/// - an unknown bearer credential refuses with the typed Err-path code
///   `authorization_required` (non-success status; the body carries no
///   projection field) on both routes;
/// - the approver identity is DISABLED through a supported Trust operation
///   (real `config-apply` reconstruction setting `disabled` on that identity),
///   after which the BLP policy is renewed at the new revision WITHOUT the
///   disabled principal's clearance and the object is re-attributed by the
///   real producer — the pre-renewal `paea-reader-policy-stale` state is
///   observed first, proving the renewal was necessary, so the disabled
///   identity is then seen DIRECTLY as `authorization_required` and never
///   masked by staleness;
/// - attribution ABSENT at the current policy revision (a further real policy
///   renewal without re-attribution): even the maximally-cleared reader is
///   refused `available:false` with `paea-reader-attribution-required`;
/// - attribution bound to a WRONG canonical digest (real producer, real digest
///   of the unclassified canonical bytes) and then to a WRONG project root
///   (the service root path): still `paea-reader-attribution-required` — the
///   served contract is asserted as it exists (the defined-but-unused
///   `paea-reader-attribution-mismatch` code is deliberately not expected);
/// - recovery: the correct re-attribution at the current revision restores the
///   positive witness on both routes, proving the refusals were
///   attribution-state-driven and the channel intact.
///
/// No mutation: every refused batch is bracketed by byte equality of BOTH
/// roots' trust journals (`service_trust_state`), the managed Git refs are
/// byte-identical across the whole matrix, and refused reads journal nothing.
///
/// Preconditions: `SWARM_PAEA_SOURCE_FIXTURE` (via `owned_agreement`) and the
/// permanent recipe environment; the same labeled fixture identities as the
/// reference rig. Errors: panics on any unexpected reply — retained as the
/// failing reproducer. Effects: none beyond the test's own fixture roots and
/// the supported producer operations it drives; no guard or route code is
/// exercised beyond the public HTTP surface. Limits: asserts the CURRENT
/// public contract of the guard and both routes; introduces no declassification
/// policy, fake identity, enrollment or confinement beyond the labeled
/// fixture. Does not close BLP-RET-05 or PAEA.
#[test]
fn reader_refusal_matrix_on_bound_approval_both_routes() {
    // ---- Same rig as `preview_reader_guard_enforces_produced_attribution`:
    // real policy, real attribution, real bound approval, both routes open.
    let owned = owned_agreement();
    let (repository_id, candidate) = (owned.repository_id.clone(), owned.candidate.clone());
    let trust_revision = owned.trust_revision;
    let policy = json!({
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
    });
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
    let trust_revision_of = |path: &Path| current_trust_revision(path);
    let blp_revision = trust("blp-show", json!({}), "fixture-admin-token")["record"]
        ["history"]
        .as_array()
        .and_then(|h| h.last())
        .and_then(|e| e["revision"].as_u64())
        .unwrap_or(0);
    let project_path = owned.project_path.clone();
    trust(
        "blp-policy-apply",
        json!({
            "schema_version":1,
            "event_id":format!("reader-matrix-policy-{}", trust_revision_of(&project_path)),
            "expected_revision":blp_revision,
            "expected_trust_revision":trust_revision_of(&project_path),
            "reason":"Reader-refusal-matrix policy for the PAEA both-route frontier fixture",
            "policy":policy.clone(),
        }),
        "fixture-admin-token",
    );
    const PRIVATE_MARKER: &str = "fixture-classified-marker-blp-ret-05";
    let mut classified_context = context(trust_revision);
    classified_context.sources[0].confidentiality_labels = vec![PRIVATE_MARKER.to_owned()];
    let exact = swarm_protection_contract::canonicalize(
        &proposal(&repository_id, &candidate),
        &classified_context,
    )
    .unwrap()
    .bytes()
    .to_vec();
    let classified_digest = swarm_protection_contract::canonicalize(
        &proposal(&repository_id, &candidate),
        &classified_context,
    )
    .unwrap()
    .digest()
    .to_string();
    let attribute = trust("blp-show", json!({}), "fixture-admin-token")["record"]
        ["history"]
        .as_array()
        .and_then(|h| h.last())
        .and_then(|e| e["revision"].as_u64())
        .unwrap_or(0);
    let attribute_reply = trust(
        "blp-object-attribute",
        json!({
            "schema_version":1,
            "event_id":format!("reader-matrix-attribute-{}", trust_revision_of(&project_path)),
            "expected_revision":attribute,
            "expected_trust_revision":trust_revision_of(&project_path),
            "canonical_digest":classified_digest,
            "project_path":owned.project_path.to_str().unwrap(),
            "classification":{"level":"classified","compartments":[]},
            "reason":"Reader-refusal-matrix attribution for the exact prepared action fixture",
        }),
        "fixture-admin-token",
    );
    assert_eq!(attribute_reply["attributed"], true, "supported producer must attribute");
    assert_eq!(attribute_reply["runtime_enforced"], false, "seam is explicit");
    // Real approval rig: engine-derived terms, real mandate, real pending
    // request, frozen approve command bound server-side.
    let terms = owned
        .project
        .paea_git_agreement_terms(&exact)
        .unwrap()
        .expect("recipe terms derivable for this canonical Git action");
    let resource = terms.resource.clone();
    let mandate_id = format!("reader-matrix-mandate-{}", std::process::id());
    trust(
        "mandate-create",
        json!({
            "schema_version":1,
            "event_id":format!("reader-matrix-mandate-{mandate_id}"),
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
            "event_id":format!("reader-matrix-request-{mandate_id}"),
            "mandate_id":mandate_id,
            "resource":resource,
            "amount_minor":terms.amount_minor,
            "currency":terms.currency,
            "beneficiary":terms.beneficiary,
            "idempotency_key":format!("reader-matrix-key-{mandate_id}"),
        }),
        "fixture-requester-token",
    );
    assert_eq!(action["state"], "pending_approval");
    let approve = serde_json::to_vec(&json!({
        "schema_version":1,
        "event_id":format!("reader-matrix-approve-{mandate_id}"),
        "action_id":action["id"],
        "mandate_id":mandate_id,
        "content_digest":action["content_digest"],
        "resource":resource,
        "amount_minor":terms.amount_minor,
        "currency":terms.currency,
        "beneficiary":terms.beneficiary,
    }))
    .unwrap();
    drop(trust);
    drop(trust_revision_of);
    let mut owned = owned;
    owned.prepared = owned
        .service
        .paea_git_transfer_call(
            &owned.project,
            &proposal(&repository_id, &candidate),
            &classified_context,
            &owned.work,
            "bare",
            PIN,
            "transfer-pack-reader-matrix",
        )
        .unwrap_or_else(|e| panic!("classified preparation refused: {:?}", e));
    owned.prepared_request_permission = owned.prepared.request().permission();
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
    let mut fixture = fixture::Fixture::build_for_trust_actions(
        vec![
            owned.prepared_request_permission.clone(),
            approval_source.request().permission(),
        ],
        owned.fixture_scope.clone(),
        trust_revision,
    );
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let authority = swarm_web::PaeaGitCallSource::open(
        source,
        fixture.path.to_str().unwrap(),
        &fixture.pin,
    )
    .unwrap();
    let s = swarm_web::NativeHttpService::open_git_call_with_approval(
        &owned.service_path,
        LocalWebAuthority::from_listener(&listener, KEY).unwrap(),
        authority,
        Some(approval_source),
    )
    .unwrap_or_else(|e| panic!("approval bind refused: {:?}", e.code()));
    // Post-open producer operations (the trust closure borrows the project
    // root again now that the partial moves are done).
    let trust_post = |op: &str, value: Value, token: &str| -> Value {
        serde_json::from_slice(
            &owned
                .project
                .trust(op, &serde_json::to_vec(&value).unwrap(), token)
                .unwrap_or_else(|e| panic!("{op} refused: {}", e.code()))
                .bytes,
        )
        .unwrap()
    };
    let blp_last = || {
        trust_post("blp-show", json!({}), "fixture-admin-token")["record"]["history"]
            .as_array()
            .and_then(|h| h.last())
            .and_then(|e| e["revision"].as_u64())
            .unwrap_or(0)
    };
    const PREVIEW_URI: &str = "/api/v1/paea/git-call/preview";
    const PRESENTATION_URI: &str = "/api/v1/paea/git-call/approval/presentation";
    let action_id = action["id"].as_str().unwrap().to_string();
    let content_digest = action["content_digest"].as_str().unwrap().to_string();
    // Typed Err-path refusal (guard returns Err → HTTP refusal with the exact
    // engine code): non-success, `authorization_required` in the typed failure
    // AND the body, and zero classified byte or private identifier.
    let assert_err_refusal = |uri: &str, token: &str, why: &str| {
        let reply = get_with(&listener, &s, uri, Some(token));
        assert!(
            !reply.response.status().is_success(),
            "{uri} {why} must refuse: {}",
            String::from_utf8_lossy(reply.response.body())
        );
        assert_eq!(
            reply
                .failure
                .as_ref()
                .map(|f| f.code().to_string())
                .unwrap_or_default(),
            "authorization_required",
            "{uri} {why}: exact typed code expected"
        );
        let raw = String::from_utf8(reply.response.body().to_vec()).unwrap();
        let body: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(body["error"], "authorization_required", "{uri} {why}: exact body error");
        assert!(!raw.contains(PRIVATE_MARKER), "{uri} {why} leaked the marker");
        assert!(!raw.contains("action_digest"), "{uri} {why} leaked a digest field");
        assert!(!raw.contains(&action_id), "{uri} {why} leaked the action id");
        assert!(!raw.contains(&mandate_id), "{uri} {why} leaked the mandate id");
        assert!(!raw.contains(&content_digest), "{uri} {why} leaked the content digest");
        assert!(
            !raw.contains("paea-git-call-approval-unavailable"),
            "{uri} {why}: approval-unavailable must never pose as a guard refusal"
        );
    };
    // Refused-verdict refusal (guard refuses → 200 available:false with the
    // exact reason): same zero-leak contract.
    let assert_refused_verdict = |uri: &str, token: &str, expected: &str, why: &str| {
        let reply = get_with(&listener, &s, uri, Some(token));
        assert!(
            reply.response.status().is_success(),
            "{uri} {why} must be an explicit verdict: {}",
            String::from_utf8_lossy(reply.response.body())
        );
        assert!(reply.failure.is_none(), "{uri} {why}: verdict, not a typed refusal");
        let raw = String::from_utf8(reply.response.body().to_vec()).unwrap();
        let body: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(body["available"], false, "{uri} {why} must stay refused");
        assert_eq!(body["reason"], expected, "{uri} {why}: exact guard code expected");
        assert!(!raw.contains(PRIVATE_MARKER), "{uri} {why} leaked the marker");
        assert!(!raw.contains("action_digest"), "{uri} {why} leaked a digest field");
        assert!(!raw.contains(&action_id), "{uri} {why} leaked the action id");
        assert!(!raw.contains(&mandate_id), "{uri} {why} leaked the mandate id");
        assert!(!raw.contains(&content_digest), "{uri} {why} leaked the content digest");
        assert!(
            !raw.contains("paea-git-call-approval-unavailable"),
            "{uri} {why}: approval-unavailable must never pose as a guard refusal"
        );
    };
    // Positive witness on the SAME preconditions as the refusal variants.
    let assert_positive = |uri: &str, token: &str, marker_expected: bool, why: &str| {
        let reply = get_with(&listener, &s, uri, Some(token));
        assert!(
            reply.response.status().is_success(),
            "{uri} {why} must serve: {}",
            String::from_utf8_lossy(reply.response.body())
        );
        assert!(reply.failure.is_none(), "{uri} {why}: unexpected typed refusal");
        let raw = String::from_utf8(reply.response.body().to_vec()).unwrap();
        let body: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(body["available"], true, "{uri} {why} must read the projection");
        assert_eq!(body["canonical"]["digest_matches"], true, "{uri} {why}: exact digest identity");
        assert_eq!(
            raw.contains(PRIVATE_MARKER),
            marker_expected,
            "{uri} {why}: marker disclosure must match the route contract"
        );
        assert!(
            !raw.contains("paea-git-call-approval-unavailable"),
            "{uri} {why}: approval-unavailable must never pose as a witness"
        );
    };
    // Both roots' trust journals, for the no-mutation brackets.
    let roots_state = || {
        (
            service_trust_state(&owned.project_path),
            service_trust_state(&owned.service_path),
        )
    };
    let refs_before = managed_git_refs(&owned.project_path, &owned.work);
    // POSITIVE witnesses FIRST, before any refusal variant, on the same
    // preconditions: full field contract on the presentation route (frozen
    // terms at the construction revision) and the marker only on preview.
    {
        let uri = PREVIEW_URI;
        let reply = get_with(&listener, &s, uri, Some("Bearer fixture-admin-token"));
        let raw = String::from_utf8(reply.response.body().to_vec()).unwrap();
        assert!(raw.contains(PRIVATE_MARKER), "{uri} authorized reader must receive the exact permitted projection");
        let body: Value = serde_json::from_slice(reply.response.body()).unwrap();
        assert_eq!(body["available"], true);
        assert_eq!(body["canonical"]["digest_matches"], true);
    }
    {
        let uri = PRESENTATION_URI;
        let presentation = get_with(&listener, &s, uri, Some("Bearer fixture-admin-token"));
        assert!(
            presentation.response.status().is_success(),
            "presentation must serve: {}",
            String::from_utf8_lossy(presentation.response.body())
        );
        assert!(presentation.failure.is_none());
        let body: Value = serde_json::from_slice(presentation.response.body()).unwrap();
        assert_eq!(body["event"], "paea-git-call-approval-presentation");
        assert_eq!(body["available"], true);
        assert_eq!(body["operation"], "approve");
        assert_eq!(body["request_digest"].as_str().unwrap().len(), 64);
        assert!(body.get("command_bytes_b64url").is_none());
        assert!(body.get("command").is_none());
        assert!(body.to_string().contains(action_id.as_str()) == false);
        assert!(body.to_string().contains(&mandate_id) == false);
        assert!(body.to_string().contains(content_digest.as_str()) == false);
        assert_eq!(body["canonical"]["digest_matches"], true);
        assert_eq!(body["agreement_terms"]["binding"], "recipe");
        assert_eq!(body["agreement_terms"]["resource"], resource.as_str());
        assert_eq!(body["agreement_terms"]["amount_minor"], terms.amount_minor);
        assert_eq!(body["agreement_terms"]["currency"], terms.currency);
        assert_eq!(body["agreement_terms"]["beneficiary"], terms.beneficiary);
        assert_eq!(body["agreement_terms"]["revision"], trust_revision);
        assert_eq!(body["current"]["roots_bound"], true);
        assert_eq!(body["current"]["declared_trust_revision"], trust_revision);
        assert_eq!(body["current"]["trust_revision_current"], true);
        assert_eq!(body["limits"]["consent"], "none-captured");
        assert_eq!(body["limits"]["consumption"], "none");
        assert_eq!(body["limits"]["human_presence"], "not-established");
        assert!(String::from_utf8_lossy(presentation.response.body()).contains(PRIVATE_MARKER) == false);
    }
    // VARIANT 1 — unknown bearer credential: typed Err-path refusal on both
    // routes, no mutation of either root.
    {
        let before = roots_state();
        assert_err_refusal(PREVIEW_URI, "Bearer fixture-no-such-reader-token", "unknown credential");
        assert_err_refusal(PRESENTATION_URI, "Bearer fixture-no-such-reader-token", "unknown credential");
        assert_eq!(before, roots_state(), "unknown-credential refusals must not mutate either root");
    }
    // VARIANT 2 — identity DISABLED via a supported Trust operation. First a
    // real config-apply that disables the approver identity only; the
    // attribution rides the old entry, so EVERY reader is policy-stale (the
    // staleness that must never mask the disabled identity is observed here).
    let view: Value =
        serde_json::from_slice(&owned.project.trust("show", b"{}", "").unwrap().bytes).unwrap();
    let mut identities = view["identities"].clone();
    for identity in identities.as_array_mut().unwrap() {
        identity["token"] = match identity["subject"].as_str() {
            Some("requester") => "fixture-requester-token".into(),
            Some("approver") => "fixture-approver-token".into(),
            Some("agreement-admin") => "fixture-admin-token".into(),
            _ => Value::String(String::new()),
        };
        identity["disabled"] = (identity["subject"].as_str() == Some("approver")).into();
    }
    owned
        .project
        .trust(
            "config-apply",
            &serde_json::to_vec(&json!({
                "schema_version":1,
                "event_id":format!("reader-matrix-disable-{}", current_trust_revision(&owned.project_path)),
                "expected_revision":current_trust_revision(&owned.project_path),
                "reason":"Reader matrix: supported disablement of the approver fixture identity",
                "policy":view["policy"],
                "identities":identities,
            }))
            .unwrap(),
            "",
        )
        .unwrap_or_else(|e| panic!("disable config refused: {}", e.code()));
    {
        let before = roots_state();
        assert_refused_verdict(
            PREVIEW_URI,
            "Bearer fixture-admin-token",
            "paea-reader-policy-stale",
            "post-config attribution rides the old entry",
        );
        assert_refused_verdict(
            PRESENTATION_URI,
            "Bearer fixture-admin-token",
            "paea-reader-policy-stale",
            "post-config attribution rides the old entry",
        );
        assert_eq!(before, roots_state(), "stale verdicts must not mutate either root");
    }
    // Renew the BLP policy at the new revision WITHOUT the disabled
    // principal's clearance, then re-attribute the exact classified bytes with
    // the real producer: currency is restored, the identity stays disabled.
    let mut renewal_policy = policy.clone();
    renewal_policy["clearances"]
        .as_object_mut()
        .expect("policy clearances object")
        .remove("approver");
    trust_post(
        "blp-policy-apply",
        json!({
            "schema_version":1,
            "event_id":format!("reader-matrix-renewal-{}", current_trust_revision(&owned.project_path)),
            "expected_revision":blp_last(),
            "expected_trust_revision":current_trust_revision(&owned.project_path),
            "reason":"Reader matrix: policy renewal without the disabled approver clearance",
            "policy":renewal_policy.clone(),
        }),
        "fixture-admin-token",
    );
    let renewed_attribute = trust_post(
        "blp-object-attribute",
        json!({
            "schema_version":1,
            "event_id":format!("reader-matrix-attribute-renewed-{}", current_trust_revision(&owned.project_path)),
            "expected_revision":blp_last(),
            "expected_trust_revision":current_trust_revision(&owned.project_path),
            "canonical_digest":classified_digest,
            "project_path":owned.project_path.to_str().unwrap(),
            "classification":{"level":"classified","compartments":[]},
            "reason":"Reader matrix: re-attribution at the renewed revision",
        }),
        "fixture-admin-token",
    );
    assert_eq!(
        renewed_attribute["attributed"], true,
        "re-attribution at the renewed revision must be producer-accepted"
    );
    // Same preconditions restored: the authorized reader reads again, then the
    // DISABLED identity is refused DIRECTLY — authentication precedes the
    // currency check, so staleness cannot mask it.
    assert_positive(
        PREVIEW_URI,
        "Bearer fixture-admin-token",
        true,
        "authorized reader after the disablement renewal",
    );
    assert_positive(
        PRESENTATION_URI,
        "Bearer fixture-admin-token",
        false,
        "authorized reader after the disablement renewal",
    );
    {
        let before = roots_state();
        assert_err_refusal(PREVIEW_URI, "Bearer fixture-approver-token", "disabled identity");
        assert_err_refusal(PRESENTATION_URI, "Bearer fixture-approver-token", "disabled identity");
        assert_eq!(before, roots_state(), "disabled-identity refusals must not mutate either root");
    }
    // VARIANT 3 — attribution ABSENT at the current policy revision: a further
    // real renewal WITHOUT re-attribution; even the maximally-cleared reader
    // is refused with the attribution code, never a projection byte.
    trust_post(
        "blp-policy-apply",
        json!({
            "schema_version":1,
            "event_id":format!("reader-matrix-renewal-unattributed-{}", current_trust_revision(&owned.project_path)),
            "expected_revision":blp_last(),
            "expected_trust_revision":current_trust_revision(&owned.project_path),
            "reason":"Reader matrix: renewal WITHOUT re-attribution to observe the attribution-absent refusal",
            "policy":renewal_policy.clone(),
        }),
        "fixture-admin-token",
    );
    {
        let before = roots_state();
        assert_refused_verdict(
            PREVIEW_URI,
            "Bearer fixture-admin-token",
            "paea-reader-attribution-required",
            "attribution absent for the maximally-cleared reader",
        );
        assert_refused_verdict(
            PRESENTATION_URI,
            "Bearer fixture-admin-token",
            "paea-reader-attribution-required",
            "attribution absent for the maximally-cleared reader",
        );
        assert_eq!(before, roots_state(), "attribution-absent refusals must not mutate either root");
    }
    // VARIANT 4 — attribution bound to a WRONG canonical digest: the REAL
    // producer accepts a distinct (policy revision, digest, project) key — the
    // digest of the same action under the UNCLASSIFIED context.
    let wrong_digest = swarm_protection_contract::canonicalize(
        &proposal(&repository_id, &candidate),
        &context(trust_revision),
    )
    .unwrap()
    .digest()
    .to_string();
    assert_ne!(wrong_digest, classified_digest, "the wrong-digest variant must differ from the classified digest");
    let wrong_digest_attribute = trust_post(
        "blp-object-attribute",
        json!({
            "schema_version":1,
            "event_id":format!("reader-matrix-attribute-wrong-digest-{}", current_trust_revision(&owned.project_path)),
            "expected_revision":blp_last(),
            "expected_trust_revision":current_trust_revision(&owned.project_path),
            "canonical_digest":wrong_digest,
            "project_path":owned.project_path.to_str().unwrap(),
            "classification":{"level":"classified","compartments":[]},
            "reason":"Reader matrix: producer-real attribution bound to a DIFFERENT canonical digest",
        }),
        "fixture-admin-token",
    );
    assert_eq!(
        wrong_digest_attribute["attributed"], true,
        "the wrong-digest attribution must be producer-accepted (real producer, real key)"
    );
    {
        let before = roots_state();
        assert_refused_verdict(
            PREVIEW_URI,
            "Bearer fixture-admin-token",
            "paea-reader-attribution-required",
            "attribution bound to the wrong digest",
        );
        assert_refused_verdict(
            PRESENTATION_URI,
            "Bearer fixture-admin-token",
            "paea-reader-attribution-required",
            "attribution bound to the wrong digest",
        );
        assert_eq!(before, roots_state(), "wrong-digest refusals must not mutate either root");
    }
    // VARIANT 5 — attribution bound to a WRONG project root (the service root
    // path): same served contract.
    assert_ne!(
        owned.service_path, owned.project_path,
        "the wrong-project variant must target a different root"
    );
    let wrong_project_attribute = trust_post(
        "blp-object-attribute",
        json!({
            "schema_version":1,
            "event_id":format!("reader-matrix-attribute-wrong-project-{}", current_trust_revision(&owned.project_path)),
            "expected_revision":blp_last(),
            "expected_trust_revision":current_trust_revision(&owned.project_path),
            "canonical_digest":classified_digest,
            "project_path":owned.service_path.to_str().unwrap(),
            "classification":{"level":"classified","compartments":[]},
            "reason":"Reader matrix: producer-real attribution bound to a DIFFERENT project root",
        }),
        "fixture-admin-token",
    );
    assert_eq!(
        wrong_project_attribute["attributed"], true,
        "the wrong-project attribution must be producer-accepted (real producer, real key)"
    );
    {
        let before = roots_state();
        assert_refused_verdict(
            PREVIEW_URI,
            "Bearer fixture-admin-token",
            "paea-reader-attribution-required",
            "attribution bound to the wrong project root",
        );
        assert_refused_verdict(
            PRESENTATION_URI,
            "Bearer fixture-admin-token",
            "paea-reader-attribution-required",
            "attribution bound to the wrong project root",
        );
        assert_eq!(before, roots_state(), "wrong-project refusals must not mutate either root");
    }
    // RECOVERY — the correct re-attribution at the current revision restores
    // the positive witness on both routes: the refusals above were
    // attribution-state-driven and the channel is intact.
    let restored_attribute = trust_post(
        "blp-object-attribute",
        json!({
            "schema_version":1,
            "event_id":format!("reader-matrix-attribute-restored-{}", current_trust_revision(&owned.project_path)),
            "expected_revision":blp_last(),
            "expected_trust_revision":current_trust_revision(&owned.project_path),
            "canonical_digest":classified_digest,
            "project_path":owned.project_path.to_str().unwrap(),
            "classification":{"level":"classified","compartments":[]},
            "reason":"Reader matrix: correct re-attribution restores the positive witness",
        }),
        "fixture-admin-token",
    );
    assert_eq!(
        restored_attribute["attributed"], true,
        "the restoring attribution must be producer-accepted"
    );
    assert_positive(
        PREVIEW_URI,
        "Bearer fixture-admin-token",
        true,
        "authorized reader after the restoring attribution",
    );
    assert_positive(
        PRESENTATION_URI,
        "Bearer fixture-admin-token",
        false,
        "authorized reader after the restoring attribution",
    );
    assert_eq!(
        refs_before,
        managed_git_refs(&owned.project_path, &owned.work),
        "the whole matrix must leave the managed Git refs untouched"
    );
    let _ = s.close();
    let _ = owned.project.close();
    let _ = owned.service.close();
    fixture.close();
}

/// BLP-RET-05 remaining frontier (status siblings): the two cookie-only GET
/// status routes — `GET /api/v1/paea/git-call` and
/// `GET /api/v1/paea/git-call/approval` — must NOT serve ANY value derived
/// from the selected prepared request to a session alone, while the guarded
/// preview and presentation routes refuse that same bare session with the
/// stable no-credential code. A closed vocabulary does NOT make the selected
/// value a constant: the selected `call`/`service`/`operation` and every
/// exact digest are request-derived classified selections, and no
/// declassification right is created. The counterexample runs on the SAME rig
/// as `preview_reader_guard_enforces_produced_attribution` (real BLP policy,
/// real `blp-object-attribute` attribution bound to the exact classified
/// canonical digest, real mandate/request/approve chain, `prepare_approval`,
/// bound approval): a positive witness first (the guarded routes DO refuse a
/// bare session on these exact preconditions, so the rig is real and no
/// refusal can be vacuity — `paea-git-call-approval-unavailable` never
/// counts), then the minimal status contract: each status serves `bound:true`
/// with a NULL capability object — no root/action/request digest, no selected
/// call/service/operation value or field name, no action/mandate identifier,
/// no content digest, no classified marker — on GET and HEAD; the exact
/// metadata stays readable only through the guarded routes already covered by
/// the reference rig and the refusal matrix
/// (`reader_refusal_matrix_on_bound_approval_both_routes`), and the
/// approval status stays a real refusal code for consumption under a wrong
/// credential.
///
/// Preconditions: `SWARM_PAEA_SOURCE_FIXTURE` (via `owned_agreement`) and the
/// permanent recipe environment. Errors: panics on any unexpected reply — the
/// failure output carries the served body as the reproducer evidence. Effects:
/// the test's own fixture roots and supported producer operations only; the
/// status routes are read-only observations. Limits: asserts the public
/// contract of the two status siblings under a bare session; unknown-token,
/// disabled-identity and missing/misbound-attribution variants keep their
/// coverage in the refusal matrix; no global confidentiality or OS protection
/// is claimed; does not close BLP-RET-05 or PAEA.
#[test]
fn status_siblings_serve_no_classified_digest_to_bare_session() {
    // ---- Same rig as `preview_reader_guard_enforces_produced_attribution`:
    // real policy, real attribution, real bound approval, all routes open.
    let owned = owned_agreement();
    let (repository_id, candidate) = (owned.repository_id.clone(), owned.candidate.clone());
    let trust_revision = owned.trust_revision;
    let policy = json!({
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
    });
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
    let trust_revision_of = |path: &Path| current_trust_revision(path);
    let blp_revision = trust("blp-show", json!({}), "fixture-admin-token")["record"]
        ["history"]
        .as_array()
        .and_then(|h| h.last())
        .and_then(|e| e["revision"].as_u64())
        .unwrap_or(0);
    let project_path = owned.project_path.clone();
    trust(
        "blp-policy-apply",
        json!({
            "schema_version":1,
            "event_id":format!("status-siblings-policy-{}", trust_revision_of(&project_path)),
            "expected_revision":blp_revision,
            "expected_trust_revision":trust_revision_of(&project_path),
            "reason":"Status-siblings counterexample policy for the PAEA both-status frontier",
            "policy":policy.clone(),
        }),
        "fixture-admin-token",
    );
    const PRIVATE_MARKER: &str = "fixture-classified-marker-blp-ret-05";
    let mut classified_context = context(trust_revision);
    classified_context.sources[0].confidentiality_labels = vec![PRIVATE_MARKER.to_owned()];
    let exact = swarm_protection_contract::canonicalize(
        &proposal(&repository_id, &candidate),
        &classified_context,
    )
    .unwrap()
    .bytes()
    .to_vec();
    let classified_digest = swarm_protection_contract::canonicalize(
        &proposal(&repository_id, &candidate),
        &classified_context,
    )
    .unwrap()
    .digest()
    .to_string();
    let attribute = trust("blp-show", json!({}), "fixture-admin-token")["record"]
        ["history"]
        .as_array()
        .and_then(|h| h.last())
        .and_then(|e| e["revision"].as_u64())
        .unwrap_or(0);
    let attribute_reply = trust(
        "blp-object-attribute",
        json!({
            "schema_version":1,
            "event_id":format!("status-siblings-attribute-{}", trust_revision_of(&project_path)),
            "expected_revision":attribute,
            "expected_trust_revision":trust_revision_of(&project_path),
            "canonical_digest":classified_digest,
            "project_path":owned.project_path.to_str().unwrap(),
            "classification":{"level":"classified","compartments":[]},
            "reason":"Status-siblings attribution for the exact prepared action fixture",
        }),
        "fixture-admin-token",
    );
    assert_eq!(attribute_reply["attributed"], true, "supported producer must attribute");
    assert_eq!(attribute_reply["runtime_enforced"], false, "seam is explicit: no runtime enforcement beyond the served surface is claimed");
    // Real approval rig: engine-derived terms, real mandate, real pending
    // request, frozen approve command bound server-side.
    let terms = owned
        .project
        .paea_git_agreement_terms(&exact)
        .unwrap()
        .expect("recipe terms derivable for this canonical Git action");
    let resource = terms.resource.clone();
    let mandate_id = format!("status-siblings-mandate-{}", std::process::id());
    trust(
        "mandate-create",
        json!({
            "schema_version":1,
            "event_id":format!("status-siblings-mandate-{mandate_id}"),
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
            "event_id":format!("status-siblings-request-{mandate_id}"),
            "mandate_id":mandate_id,
            "resource":resource,
            "amount_minor":terms.amount_minor,
            "currency":terms.currency,
            "beneficiary":terms.beneficiary,
            "idempotency_key":format!("status-siblings-key-{mandate_id}"),
        }),
        "fixture-requester-token",
    );
    assert_eq!(action["state"], "pending_approval");
    let action_id = action["id"].as_str().unwrap().to_string();
    let content_digest = action["content_digest"].as_str().unwrap().to_string();
    let approve = serde_json::to_vec(&json!({
        "schema_version":1,
        "event_id":format!("status-siblings-approve-{mandate_id}"),
        "action_id":action["id"],
        "mandate_id":mandate_id,
        "content_digest":action["content_digest"],
        "resource":resource,
        "amount_minor":terms.amount_minor,
        "currency":terms.currency,
        "beneficiary":terms.beneficiary,
    }))
    .unwrap();
    drop(trust);
    drop(trust_revision_of);
    let mut owned = owned;
    owned.prepared = owned
        .service
        .paea_git_transfer_call(
            &owned.project,
            &proposal(&repository_id, &candidate),
            &classified_context,
            &owned.work,
            "bare",
            PIN,
            "transfer-pack-status-siblings",
        )
        .unwrap_or_else(|e| panic!("classified preparation refused: {:?}", e));
    owned.prepared_request_permission = owned.prepared.request().permission();
    // Exact digests held by the preparation and the bound approval, captured
    // before the moves: the counterexample asserts their ABSENCE from the
    // cookie-only siblings.
    let root_digest = owned.prepared.request().root_digest().to_owned();
    let action_digest = owned.prepared.request().action_digest().to_owned();
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
    let approval_request_digest = approval_source.request().request_digest().to_owned();
    let mut fixture = fixture::Fixture::build_for_trust_actions(
        vec![
            owned.prepared_request_permission.clone(),
            approval_source.request().permission(),
        ],
        owned.fixture_scope.clone(),
        trust_revision,
    );
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let authority = swarm_web::PaeaGitCallSource::open(
        source,
        fixture.path.to_str().unwrap(),
        &fixture.pin,
    )
    .unwrap();
    let s = swarm_web::NativeHttpService::open_git_call_with_approval(
        &owned.service_path,
        LocalWebAuthority::from_listener(&listener, KEY).unwrap(),
        authority,
        Some(approval_source),
    )
    .unwrap_or_else(|e| panic!("approval bind refused: {:?}", e.code()));
    // TÉMOIN POSITIF D'ABORD: on these exact preconditions (real attribution,
    // real bound approval) the guarded routes refuse a bare session with the
    // stable no-credential code — the rig is real, so any later status-leak
    // finding cannot be explained by an empty rig, and
    // `paea-git-call-approval-unavailable` never counts as a guard refusal.
    for uri in [
        "/api/v1/paea/git-call/preview",
        "/api/v1/paea/git-call/approval/presentation",
    ] {
        let body: Value = serde_json::from_slice(get(&listener, &s, uri).response.body()).unwrap();
        let raw = serde_json::to_string(&body).unwrap();
        assert!(!raw.contains(PRIVATE_MARKER), "{uri} served the marker to a bare session");
        assert_eq!(body["available"], false, "{uri} bare session must stay refused");
        assert_eq!(
            body["reason"], "paea-reader-authentication-required",
            "{uri} bare session refuses with the stable no-credential code"
        );
        assert!(
            !raw.contains("paea-git-call-approval-unavailable"),
            "{uri}: approval-unavailable must never pose as a guard refusal"
        );
    }
    // CONTRE-EXEMPLE DES SIBLINGS STATUS: a bare session — the SAME ordinary
    // cookie the guarded routes just refused — must read ONLY the observation
    // boolean and literal protocol constants. NO request-derived value of any
    // kind passes: digests AND the selected call/service/operation (a closed
    // vocabulary does not make the selected value a constant — no
    // declassification right is created). Asserted on GET and HEAD, against
    // the exact known values and field names.
    for method in ["GET", "HEAD"] {
        for (uri, forbidden) in [
            (
                "/api/v1/paea/git-call",
                vec![
                    ("root_digest", root_digest.as_str()),
                    ("action_digest", action_digest.as_str()),
                    ("call", "transfer"),
                    ("service", "repository.promote"),
                    ("operation", "reconcile"),
                ],
            ),
            (
                "/api/v1/paea/git-call/approval",
                vec![
                    ("request_digest", approval_request_digest.as_str()),
                    ("operation", "approve"),
                ],
            ),
        ] {
            let reply = if method == "GET" {
                get(&listener, &s, uri)
            } else {
                head(&listener, &s, uri)
            };
            let raw = String::from_utf8_lossy(reply.response.body()).to_string();
            // HEAD contract: success, no typed refusal, and NO body bytes —
            // the digest/metadata non-disclosure on HEAD follows from the
            // empty body itself, so nothing is parsed there.
            if method == "HEAD" {
                assert!(
                    reply.response.status().is_success(),
                    "HEAD {uri} must succeed: {}",
                    String::from_utf8_lossy(reply.response.body())
                );
                assert!(reply.failure.is_none(), "HEAD {uri}: unexpected typed refusal");
                assert!(
                    reply.response.body().is_empty(),
                    "HEAD {uri} must serve an empty body, got: {raw}"
                );
                continue;
            }
            let body: Value = serde_json::from_slice(reply.response.body()).unwrap();
            assert_eq!(body["bound"], true, "{method} {uri} bound must stay observable");
            assert_eq!(
                body["capability"], serde_json::Value::Null,
                "{method} {uri} must serve NO capability object under a bare session: {raw}"
            );
            for (field, value) in &forbidden {
                assert!(
                    raw.as_bytes().windows(value.len()).any(|w| w == value.as_bytes()) == false,
                    "{method} {uri} served the request-derived {field} value to a bare session"
                );
            }
            // Structural complement: the capability key carries NO derived
            // field name at all — it is the JSON null, never an object
            // (field-name presence on the raw body is not asserted: the
            // event name itself legitimately contains "call").
            assert!(
                body["capability"].as_object().is_none(),
                "{method} {uri} served a capability object under a bare session"
            );
            for leak in [
                classified_digest.as_str(),
                &action_id,
                &mandate_id,
                &content_digest,
                PRIVATE_MARKER,
            ] {
                assert!(
                    !raw.contains(leak),
                    "{method} {uri} leaked a private identifier or marker"
                );
            }
            assert!(
                !raw.contains("paea-git-call-approval-unavailable"),
                "{method} {uri}: approval-unavailable must never pose as a status reply"
            );
        }
    }
    // Positive status contract AFTER correction (kept from the first lot): on
    // a bound source each status serves `bound:true` with a NULL capability
    // for a bare session — the observation boolean is the whole cookie-only
    // contract; the null-capability agreement route (the form's first read)
    // keeps its explicit refusal code for consumption.
    {
        let approval_status: Value = serde_json::from_slice(
            get(&listener, &s, "/api/v1/paea/git-call/approval").response.body(),
        )
        .unwrap();
        assert_eq!(approval_status["bound"], true);
        assert_eq!(approval_status["capability"], serde_json::Value::Null);
        let refus = post(
            &listener,
            &s,
            "/api/v1/paea/git-call/approval",
            Vec::new(),
            Some(format!("Enrollment {}", URL_SAFE_NO_PAD.encode(b"wrong-password"))),
        );
        assert!(
            !refus.response.status().is_success(),
            "approval consumption must stay refused under a wrong credential"
        );
    }
    let _ = s.close();
    let _ = owned.project.close();
    let _ = owned.service.close();
    fixture.close();
}

/// Counter-evidence for the selection channel (BLP-RET-05): TWO different
/// prepared intentions, bound to two separate real services, must yield
/// BYTE-IDENTICAL status replies on both cookie-only routes. If any
/// request-derived value (call kind, digests, repository/candidate identity,
/// operation) leaked into the status projection, the two replies would differ
/// and this equality would fail. Only presence (`bound`) may discriminate a
/// bound service from an unbound one; two DISTINCT bound preparations may not.
///
/// Preconditions: `SWARM_PAEA_SOURCE_FIXTURE` (via `owned_agreement`). Errors:
/// panics with both raw bodies on any discriminating byte. Effects: the test's
/// own fixture roots only; status routes are read-only observations and the
/// prepared calls are never consumed. Limits: proves non-discrimination of the
/// status projection between two transfer preparations on the same fixture
/// family; it does not close BLP-RET-05 and claims no global confidentiality.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn status_does_not_discriminate_two_prepared_intentions() {
    // Intention A: the ordinary owned agreement rig on the retained fixture.
    let owned_a = owned_agreement();
    // Intention B: a second prepared transfer from the SAME retained fixture
    // but with DISTINCT proposal bytes (other repository id, other candidate
    // oid) bound to its own fresh service root — a different classified
    // intention, different action digest, same fixture family.
    let (service_dir_b, service_b) = service();
    service_b
        .paea_initialize_git_intents("fixture-project", PIN)
        .unwrap();
    let bare = service_dir_b.path().join(".swarm/bare");
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
    let binding_b = service_b
        .paea_provision_git_repository(&owned_a.project, &owned_a.work, "bare", PIN)
        .unwrap();
    let prepared_b = service_b
        .paea_git_transfer_call(
            &owned_a.project,
            &proposal(binding_b.repository_id(), &"d".repeat(40)),
            &context(owned_a.trust_revision),
            &owned_a.work,
            "bare",
            PIN,
            "transfer-pack-b",
        )
        .unwrap_or_else(|e| panic!("intention B preparation refused: {:?}", e));
    // The two intentions really differ on the bytes the frontier protects.
    assert_ne!(
        owned_a.prepared.request().action_digest(),
        prepared_b.request().action_digest(),
        "the two prepared intentions must carry different action digests"
    );
    let source_b = PaeaGitCallSource::new_transfer(
        prepared_b,
        service_dir_b.path().to_path_buf(),
        owned_a.project_path.clone(),
    );
    let fixture_b = fixture::Fixture::build_for_trust(
        source_b.request().permission(),
        owned_a.fixture_scope.clone(),
        owned_a.trust_revision,
    );
    let authority_b = swarm_web::PaeaGitCallSource::open(
        source_b,
        fixture_b.path.to_str().unwrap(),
        &fixture_b.pin,
    )
    .unwrap_or_else(|e| panic!("intention B authority refused: {:?}", e));
    let listener_b = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let s_b = NativeHttpService::open_git_call(
        service_dir_b.path(),
        LocalWebAuthority::from_listener(&listener_b, KEY).unwrap(),
        authority_b,
    )
    .unwrap_or_else(|e| panic!("intention B bind refused: {:?}", e.code()));
    // Service A (bound to the owned rig's transfer authority, approval-less).
    let source_a = PaeaGitCallSource::new_transfer(
        owned_a.prepared,
        owned_a.service_path.clone(),
        owned_a.project_path.clone(),
    );
    let fixture_a = fixture::Fixture::build_for_trust(
        owned_a.prepared_request_permission.clone(),
        owned_a.fixture_scope.clone(),
        owned_a.trust_revision,
    );
    let authority_a = swarm_web::PaeaGitCallSource::open(
        source_a,
        fixture_a.path.to_str().unwrap(),
        &fixture_a.pin,
    )
    .unwrap_or_else(|e| panic!("intention A authority refused: {:?}", e));
    let listener_a = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let s_a = NativeHttpService::open_git_call(
        &owned_a.service_path,
        LocalWebAuthority::from_listener(&listener_a, KEY).unwrap(),
        authority_a,
    )
    .unwrap_or_else(|e| panic!("intention A bind refused: {:?}", e.code()));
    // Both status replies, byte-exact, per route. The transfer route must be
    // bound:true on both services (each holds its own prepared transfer); the
    // approval route has no prepared approval on either service, so its
    // observed `bound` value must match between the two services (whatever it
    // is) — the discrimination proof is the byte equality itself.
    for uri in ["/api/v1/paea/git-call", "/api/v1/paea/git-call/approval"] {
        let reply_a = get(&listener_a, &s_a, uri);
        let reply_b = get(&listener_b, &s_b, uri);
        let raw_a = reply_a.response.body().to_vec();
        let raw_b = reply_b.response.body().to_vec();
        assert!(
            raw_a == raw_b,
            "{uri} discriminated the two prepared intentions:\nA: {}\nB: {}",
            String::from_utf8_lossy(&raw_a),
            String::from_utf8_lossy(&raw_b)
        );
        let body: Value = serde_json::from_slice(&raw_a).unwrap();
        if uri == "/api/v1/paea/git-call" {
            assert_eq!(body["bound"], true, "{uri} must report bound:true");
        }
        assert_eq!(
            body["capability"], serde_json::Value::Null,
            "{uri} served a capability object: {}",
            String::from_utf8_lossy(&raw_a)
        );
    }
    let _ = s_a.close();
    let _ = s_b.close();
    let _ = service_b.close();
    let _ = owned_a.project.close();
    let _ = owned_a.service.close();
    fixture_a.close();
    fixture_b.close();
}

/// Real frozen approval command bound to a real native HTTP server with its
/// ordinary persisted session, held alive while a real browser qualification
/// drives the served approval form through two public states: the fresh
/// presentation with the authorized projection (operation, request digest,
/// engine-derived terms — no private command fields) and a REAL authenticated
/// enrollment consumption through the form with an EMPTY body (labeled fixture
/// password, ack `state:"approved"` shown, password erased), then the
/// explicitly stale state after a real public Trust `config-apply` (never a
/// journal edit), re-observed with no confirmation solicited and no success
/// shown. The guarded presentation sits behind the BLP reader frontier
/// (BLP-RET-05): a bare session refuses `paea-reader-authentication-required`
/// before any projection, so the fixture produces REAL confidentiality
/// authorization first — `blp-policy-apply` over the fixture identities and
/// `blp-object-attribute` binding the EXACT canonical digest of the prepared
/// action to this project root — and the harness types the fixture reader
/// credential (server.json `reader_token`) into the page's EPHEMERAL reader
/// input; the page carries it as a request-time Bearer on the guarded read
/// only and clears it right after (no storage, URL or log; enrollment
/// authority never reinterpreted as clearance). After the stale config-apply
/// the recorded policy rides the old revision, so the fixture renews the
/// policy and re-attributes through the same real producers at the NEW
/// revision: the browser then observes the form's real stale comparison
/// (`trust_revision_current:false`). Phase coordination with the browser
/// harness uses owned marker files; every harness marker must carry the exact
/// startup nonce, so no other process can drive this server, and an error
/// marker fails the test with the FIRST browser error before any later
/// assertion can mask it. The server closes checked on every path; no Git
/// effect, ref change, second approval system or acceptance is claimed — the
/// browser consumes the server-held command once, under the fixture
/// enrollment.
#[test]
#[ignore = "requires explicit retained native source fixture and browser harness"]
fn approval_form_serves_real_browser_session() {
    let owned = owned_agreement();
    let (repository_id, candidate) = (owned.repository_id.clone(), owned.candidate.clone());
    // The exact canonical action bytes the preparation digested; the frozen
    // approve command carries the engine-derived recipe terms for them.
    let exact = swarm_protection_contract::canonicalize(
        &proposal(&repository_id, &candidate),
        &context(owned.trust_revision),
    )
    .unwrap()
    .bytes()
    .to_vec();
    let terms = owned
        .project
        .paea_git_agreement_terms(&exact)
        .unwrap()
        .expect("recipe terms derivable for this canonical Git action");
    let resource = terms.resource.clone();
    let mandate_id = format!("approval-form-mandate-{}", std::process::id());
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
    // Reader frontier (BLP-RET-05): the guarded presentation refuses a bare
    // session, so the browser fixture must produce REAL confidentiality
    // authorization through the supported control operations BEFORE any
    // guarded read — a BLP policy over the real fixture identities
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
            "event_id":format!("approval-form-browser-policy-{}", owned.trust_revision),
            "expected_revision":blp_revision,
            "expected_trust_revision":owned.trust_revision,
            "reason":"Reader-frontier policy for the approval form browser fixture",
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
            "event_id":format!("approval-form-browser-attribute-{}", owned.trust_revision),
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
            "reason":"Reader-frontier attribution for the exact prepared action of the browser fixture",
        }),
        "fixture-admin-token",
    );
    assert_eq!(
        attribute_reply["attributed"], true,
        "supported producer must attribute the exact prepared action"
    );
    trust(
        "mandate-create",
        json!({
            "schema_version":1,
            "event_id":format!("approval-form-mandate-{mandate_id}"),
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
            "event_id":format!("approval-form-request-{mandate_id}"),
            "mandate_id":mandate_id,
            "resource":resource,
            "amount_minor":terms.amount_minor,
            "currency":terms.currency,
            "beneficiary":terms.beneficiary,
            "idempotency_key":format!("approval-form-key-{mandate_id}"),
        }),
        "fixture-requester-token",
    );
    assert_eq!(action["state"], "pending_approval");
    let approve = serde_json::to_vec(&json!({
        "schema_version":1,
        "event_id":format!("approval-form-approve-{mandate_id}"),
        "action_id":action["id"],
        "mandate_id":mandate_id,
        "content_digest":action["content_digest"],
        "resource":resource,
        "amount_minor":terms.amount_minor,
        "currency":terms.currency,
        "beneficiary":terms.beneficiary,
    }))
    .unwrap();
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
    let mut fixture = fixture::Fixture::build_for_trust_actions(
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
    // Real bound listener with the ordinary served transport; the approval is
    // attached at bind time under the same two-root witness check, so the
    // served startup is exactly the production approval-capable startup.
    let server = swarm_web::NativeHttpServer::bind_git_call_with_approval(
        &owned.service_path,
        None,
        authority,
        Some(approval_source),
    )
    .unwrap_or_else(|e| panic!("exact approval bind refused: {:?}", e.code()));
    let address = server.local_addr().unwrap();
    let stop_path = std::sync::Arc::new(
        std::env::var_os("APPROVAL_FORM_BROWSER_DIR")
            .map(std::path::PathBuf::from)
            .expect("harness control directory")
            .join("server.stop"),
    );
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
    // Ordinary persisted session (bind_source opened the service Root writable
    // and published the ordinary credential for this exact listener address);
    // the browser reads that same file — no parallel authority path.
    let session_name = fs::read_dir(owned.service_path.join(".swarm"))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .find(|name| name.starts_with("web-session-"))
        .expect("bind must persist the ordinary web session credential");
    let nonce = format!("approval-form-browser-{}", std::process::id());
    let startup = serde_json::json!({
        "schema_version":1,
        "event":"approval-form-browser-ready",
        "nonce":nonce,
        "address":address.to_string(),
        "service_root":owned.service_path,
        "project_root":owned.project_path,
        "request_digest":request_digest,
        "mandate_id":mandate_id,
        "resource":resource,
        "amount_minor":terms.amount_minor,
        "currency":terms.currency,
        "beneficiary":terms.beneficiary,
        "action_id":action["id"],
        "content_digest":action["content_digest"],
        "trust_revision":owned.trust_revision,
        // Same address-scoped protocol namespace as LocalWebAuthority (the
        // first 12 hex bytes of the SHA-256 of the observed host string);
        // derived here from the same session file stem, no second authority.
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
        // EPHEMERAL reader input (agreement-admin, classified clearance): the
        // guarded presentation read carries it as a request-time Bearer only —
        // never stored, logged or transported anywhere else by the page.
        "reader_token":"fixture-admin-token",
    });
    let control_dir = std::path::PathBuf::from(
        std::env::var_os("APPROVAL_FORM_BROWSER_DIR").expect("harness control directory"),
    );
    fs::write(control_dir.join("server.json"), serde_json::to_vec(&startup).unwrap())
        .unwrap();
    // Phase 1: fresh state. The harness drives the real form, asserts the
    // authorized projection (no private command fields), performs the explicit
    // confirmation + empty-body enrollment consumption (fixture password,
    // labeled simulated), observes the real approved ack and the erased
    // password, then writes the phase-1 done marker with our nonce before we
    // mutate anything. An error marker fails here with the browser's first
    // error, before any agreement assertion can mask it.
    wait_marker(&control_dir, &nonce, "phase1.done");
    // The real consumption already happened through the form: the exact
    // agreement must now be recognized by the project guard — proven here
    // against the engine, not against any page-internal copy.
    let matched = owned
        .project
        .paea_git_agreement_match(&exact)
        .unwrap()
        .expect("form consumption must capture the exact agreement");
    assert_eq!(matched.action_id, action["id"].as_str().unwrap());
    assert_eq!(matched.approver, "enrolled-effect");
    // Real public Trust revision change on the project root: an actual
    // config-apply through the engine under the same local operator recovery
    // path the engine tests use, exactly once, on the owned fixture. No
    // journal file is edited and no state is fabricated. The public `show`
    // view omits raw tokens, so the three bearer identities (requester,
    // approver, agreement-admin) get their exact fixture tokens back and the
    // enrolled identity keeps its principal
    // mapping (same reconstruction the settlement tests apply).
    let view: Value =
        serde_json::from_slice(&owned.project.trust("show", b"{}", "").unwrap().bytes).unwrap();
    let mut identities = view["identities"].clone();
    for identity in identities.as_array_mut().unwrap() {
        identity["disabled"] = false.into();
        identity["token"] = match identity["subject"].as_str() {
            Some("requester") => "fixture-requester-token".into(),
            Some("approver") => "fixture-approver-token".into(),
            Some("agreement-admin") => "fixture-admin-token".into(),
            _ => Value::String(String::new()),
        };
    }
    let change = serde_json::json!({
        "schema_version":1,
        "event_id":format!("approval-form-browser-stale-{}", std::process::id()),
        "expected_revision":owned.trust_revision,
        "reason":"Explicit fixture Trust revision change for approval form staleness qualification",
        "policy":view["policy"],
        "identities":identities,
    });
    owned
        .project
        .trust("config-apply", &serde_json::to_vec(&change).unwrap(), "")
        .unwrap_or_else(|e| panic!("stale revision change refused: {}", e.code()));
    let stale_revision = current_trust_revision(&owned.project_path);
    assert_eq!(stale_revision, owned.trust_revision + 1);
    // The config-apply moved the trust revision: the recorded BLP policy lost
    // currency (its entry rides the old revision), so the guarded presentation
    // would refuse paea-reader-policy-stale BEFORE the projection exposes the
    // real stale comparison. The browser phase-2 assertions observe the
    // form's explicit stale state through that comparison, so the fixture
    // renews the policy through the real producer at the NEW revision (same
    // re-sealing the reader-refusal matrix applies) and re-attributes the
    // exact prepared action: the reader then reaches the projection and the
    // stale state is observed as `trust_revision_current:false` — provenance
    // stays producer-real, and a real stale-policy refusal is the phase-1
    // behavior for a not-yet-resealed state (no bypass, no label minted).
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
            "event_id":format!("approval-form-browser-policy-renewal-{stale_revision}"),
            "expected_revision":blp_revision,
            "expected_trust_revision":stale_revision,
            "reason":"Reader-frontier policy renewal after the stale Trust revision change",
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
            "event_id":format!("approval-form-browser-attribute-renewal-{stale_revision}"),
            "expected_revision":attribute_revision,
            "expected_trust_revision":stale_revision,
            "canonical_digest":swarm_protection_contract::canonicalize(
                &proposal(&repository_id, &candidate),
                &context(owned.trust_revision),
            )
            .unwrap()
            .digest(),
            "project_path":owned.project_path.to_str().unwrap(),
            "classification":{"level":"classified","compartments":[]},
            "reason":"Reader-frontier re-attribution after the stale Trust revision change",
        }),
        "fixture-admin-token",
    );
    assert_eq!(
        attribute_reply["attributed"], true,
        "re-attribution at the stale revision must be producer-accepted"
    );
    fs::write(control_dir.join("phase2.ready"), serde_json::to_vec(&serde_json::json!({
        "nonce":nonce,
        "trust_revision":stale_revision,
    })).unwrap()).unwrap();
    // Phase 2: stale state. The harness re-observes the form through the same
    // session and asserts the visible stale refusal with no confirmation
    // solicited and the send control disabled — never a success on a stale
    // state, and never a second consumption.
    wait_marker(&control_dir, &nonce, "phase2.done");
    // Explicit owned shutdown: the serving thread observes the stop marker and
    // serve_until drains and checked-closes listener/binding there; the thread's
    // success bit keeps that close outcome visible, never dropped.
    fs::write(stop_path.as_path(), b"stop").unwrap();
    let served = server_thread.join().expect("serve thread must not panic");
    assert!(served, "serve_until must report a clean owned shutdown");
    let _ = owned.project.close();
    let _ = owned.service.close();
    fixture.close();
}

/// Wrong physical root refuses at bind, typed, with both stores untouched.
/// One valid approval preparation binds for projectA (its own witness); a
/// second preparation of the same family — same frozen approve bytes, same
/// enrollment fixture — is presented against projectB, a physically distinct
/// project root. `bind_approval` must refuse (the engine's physical-witness
/// check compares the actually opened project root against the retained
/// binding) BEFORE any session publication, close the supplied authority and
/// witness on the refusal path. Observed no-effect assertions, no more and no
/// less: the byte-identical Trust states of projectA, serviceA, the REAL
/// projectB directory and serviceB; the byte-identical managed refs listing
/// of projectA with the promotion target ref explicitly absent before AND
/// after (through real Git); the unchanged durable effect journals (Git
/// intents, promotion effects) of both service roots read through independent
/// read-only connections; and the agreement still unbound. Bind prepares a
/// witness only; it never writes Trust state.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn approval_route_wrong_physical_project_root_refuses_without_effects() {
    // projectA: the valid bind target (agreement variant for the producers).
    let owned = owned_agreement();
    // projectB: physically distinct project root on the same retained fixture
    // family — a second explicit root on the same fixture path is refused by
    // the store's own binding, so this uses a separate fresh service root,
    // never a copy of projectA's state.
    let (service_dir_b, service_b) = service();
    // The exact canonical action bytes and the frozen approve command, built
    // for projectA exactly like the positive test.
    let exact = swarm_protection_contract::canonicalize(
        &proposal(&owned.repository_id, &owned.candidate),
        &context(owned.trust_revision),
    )
    .unwrap()
    .bytes()
    .to_vec();
    let terms = owned
        .project
        .paea_git_agreement_terms(&exact)
        .unwrap()
        .expect("recipe terms derivable for this canonical Git action");
    let resource = terms.resource.clone();
    let mandate_id = format!("wrong-root-mandate-{}", std::process::id());
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
    trust(
        "mandate-create",
        json!({
            "schema_version":1,
            "event_id":format!("wrong-root-mandate-{mandate_id}"),
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
            "event_id":format!("wrong-root-request-{mandate_id}"),
            "mandate_id":mandate_id,
            "resource":resource,
            "amount_minor":terms.amount_minor,
            "currency":terms.currency,
            "beneficiary":terms.beneficiary,
            "idempotency_key":format!("wrong-root-key-{mandate_id}"),
        }),
        "fixture-requester-token",
    );
    assert_eq!(action["state"], "pending_approval");
    let approve = serde_json::to_vec(&json!({
        "schema_version":1,
        "event_id":format!("wrong-root-approve-{mandate_id}"),
        "action_id":action["id"],
        "mandate_id":mandate_id,
        "content_digest":action["content_digest"],
        "resource":resource,
        "amount_minor":terms.amount_minor,
        "currency":terms.currency,
        "beneficiary":terms.beneficiary,
    }))
    .unwrap();
    // Physical witnesses read before any bind: both Trust stores (projectA,
    // serviceA), the projectA managed refs (raw Git listing), the explicit
    // absence observation of the promotion target ref and the actual effect
    // journals (intents/promotions, read-only) of BOTH service roots. These
    // are the comparison basis for "no mutation" — nothing is claimed beyond
    // what is actually observed here.
    let project_before = service_trust_state(&owned.project_path);
    let service_before = service_trust_state(&owned.service_path);
    let refs_before = managed_git_refs(&owned.project_path, &owned.work);
    let target_ref_before = managed_git_target_ref(&owned.project_path, &owned.work, "refs/heads/release");
    let intents_a_before = sqlite_effect_journal_state(&owned.service_path.join(".swarm/state.db"));
    assert!(owned
        .project
        .paea_git_agreement_match(&exact)
        .unwrap()
        .is_none());
    // Valid bind: same frozen bytes against projectA's own source — accepted.
    let source_a = PaeaGitCallSource::new_transfer(
        owned.prepared,
        owned.service_path.clone(),
        owned.project_path.clone(),
    );
    let approval_a = source_a
        .prepare_approval(
            swarm_enrollment_authority::trust_action::Operation::Approve,
            &approve,
        )
        .unwrap_or_else(|e| panic!("projectA approval preparation refused: {:?}", e.code()));
    let fixture_a = fixture::Fixture::build_for_trust_actions(
        vec![
            owned.prepared_request_permission.clone(),
            approval_a.request().permission(),
        ],
        owned.fixture_scope.clone(),
        owned.trust_revision,
    );
    let authority_a = swarm_web::PaeaGitCallSource::open(
        source_a,
        fixture_a.path.to_str().unwrap(),
        &fixture_a.pin,
    )
    .unwrap();
    let listener_a = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let service_a = NativeHttpService::open_git_call_with_approval(
        &owned.service_path,
        LocalWebAuthority::from_listener(&listener_a, KEY).unwrap(),
        authority_a,
        Some(approval_a),
    )
    .unwrap_or_else(|e| panic!("projectA approval bind refused: {:?}", e.code()));
    // Bind state observed through the public status route, not assumed:
    // capability bound:true, digest-only.
    let status_a = get(&listener_a, &service_a, "/api/v1/paea/git-call/approval");
    assert!(status_a.response.status().is_success());
    let body_a: Value = serde_json::from_slice(status_a.response.body()).unwrap();
    assert_eq!(body_a["bound"], true);
    // Wrong physical root: a same-family approval preparation whose witness
    // targets a DIFFERENT physically distinct project root. projectB is a
    // second initialized project directory (never a copy of projectA's
    // state); service_b is its separate service root. prepare_approval
    // prepares its fresh witness against projectB. The held transfer
    // authority stays the projectA source: at bind, bind_approval opens
    // projectA (the held source's project path) and check_root compares the
    // projectB witness — the exact physical mismatch under test. It must
    // refuse typed.
    let project_b_dir = tempfile::tempdir_in(swarm_engine::fixture_area()).unwrap();
    fs::set_permissions(project_b_dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let project_b = RootStore::initialize(project_b_dir.path(), &mut SystemEnvironment).unwrap();
    // The REAL projectB's Trust snapshot is taken right after its actual
    // initialization and BEFORE any preparation/bind: this is the basis the
    // final no-mutation check compares (the service B snapshot below covers
    // serviceB only, not projectB).
    let project_b_before = service_trust_state(project_b_dir.path());
    // serviceB's own Trust snapshot and its journals, same basis rule.
    let service_b_state_before = service_trust_state(service_dir_b.path());
    let intents_b_before =
        sqlite_effect_journal_state(&service_dir_b.path().join(".swarm/state.db"));
    let disposable_b = PaeaGitCallSource::new_transfer(
        service_b
            .paea_git_transfer_call(
                &project_b,
                // Distinct valid oid bytes: same canonical family, different
                // action identity from projectA's actual candidate.
                &proposal("fixture-project-wrong-root", &"d".repeat(40)),
                &context(owned.trust_revision),
                "fixture-project-wrong-root",
                "bare",
                PIN,
                "transfer-pack",
            )
            .unwrap_or_else(|e| panic!("projectB preparation refused: {:?}", e)),
        service_dir_b.path().to_path_buf(),
        // The approval witness targets projectB's path (this source's own
        // project path); the transfer witness binds serviceB/projectB. Only
        // the approval witness of this source reaches the held projectA bind.
        project_b_dir.path().to_path_buf(),
    );
    project_b.close().unwrap_or_else(|e| {
        panic!(
            "projectB explicit close refused: {:?}",
            e.code()
        )
    });
    let approval_wrong = disposable_b
        .prepare_approval(
            swarm_enrollment_authority::trust_action::Operation::Approve,
            &approve,
        )
        .unwrap_or_else(|e| panic!("projectB approval preparation refused: {:?}", e.code()));
    // Same enrollment fixture family: the provisioned permission is valid for
    // the approval command bytes; only the PHYSICAL root differs. The wrong
    // approval binds onto a held projectA transfer authority — a fresh
    // authority from the SAME provisioned declaration, since the first one is
    // already bound inside the live projectA service above.
    let fixture_wrong = fixture::Fixture::build_for_trust_actions(
        vec![approval_wrong.request().permission()],
        owned.fixture_scope.clone(),
        owned.trust_revision,
    );
    // The disposable projectB transfer source never binds a service; its held
    // transfer witnesses are released through the established public path:
    // open the provisioned authority, then close it explicitly (closing the
    // authority closes the source's prepared witnesses and retains causes).
    let disposable_authority = disposable_b
        .open(fixture_wrong.path.to_str().unwrap(), &fixture_wrong.pin)
        .unwrap_or_else(|e| panic!("projectB disposable authority refused: {:?}", e));
    disposable_authority
        .close()
        .unwrap_or_else(|e| panic!("projectB disposable authority close refused: {:?}", e));
    let source_a_wrong = PaeaGitCallSource::new_transfer(
        service_b
            .paea_git_transfer_call(
                &owned.project,
                &proposal(&owned.repository_id, &owned.candidate),
                &context(owned.trust_revision),
                &owned.work,
                "bare",
                PIN,
                "transfer-pack",
            )
            .unwrap_or_else(|e| panic!("projectA wrong-holder preparation refused: {:?}", e)),
        service_dir_b.path().to_path_buf(),
        owned.project_path.clone(),
    );
    let wrong_authority = swarm_web::PaeaGitCallSource::open(
        source_a_wrong,
        fixture_wrong.path.to_str().unwrap(),
        &fixture_wrong.pin,
    )
    .unwrap();
    let listener_wrong = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let wrong_bind = NativeHttpService::open_git_call_with_approval(
        service_dir_b.path(),
        LocalWebAuthority::from_listener(&listener_wrong, KEY).unwrap(),
        wrong_authority,
        Some(approval_wrong),
    );
    // Wrong physical root refuses typed: the engine's physical-witness check
    // finds the retained projectB approval witness beside the actually opened
    // projectA root and refuses "trust-enrollment-root" through the Root
    // wrapper. The refused bind consumed the supplied authority (approval
    // witness included), so nothing further is opened or closed for it.
    let bind_error = match wrong_bind {
        Err(error) => error,
        Ok(_) => panic!("wrong physical root bind must refuse"),
    };
    assert_eq!(
        bind_error.code(),
        "trust-enrollment-root",
        "typed wrong-root refusal, got: {}",
        bind_error.code()
    );
    // No mutation from either bind or refusal: all three Trust stores byte-
    // identical (projectA, serviceA, the REAL projectB directory), the
    // projectA managed refs listing and the explicit promotion-target-ref
    // observation unchanged (absence stays absence — no ref was created), the
    // durable effect journals of both service roots unchanged, and no
    // approval was captured anywhere.
    assert_eq!(
        project_before,
        service_trust_state(&owned.project_path),
        "projectA trust state must stay byte-identical through valid and wrong binds"
    );
    assert_eq!(
        service_before,
        service_trust_state(&owned.service_path),
        "serviceA trust state must stay byte-identical"
    );
    assert_eq!(
        refs_before,
        managed_git_refs(&owned.project_path, &owned.work),
        "projectA managed refs listing must stay byte-identical through both binds"
    );
    assert_eq!(
        target_ref_before,
        managed_git_target_ref(&owned.project_path, &owned.work, "refs/heads/release"),
        "the promotion target ref must stay explicitly absent from projectA"
    );
    assert_eq!(
        intents_a_before,
        sqlite_effect_journal_state(&owned.service_path.join(".swarm/state.db")),
        "serviceA durable effect journals must stay unchanged through both binds"
    );
    assert_eq!(
        project_b_before,
        service_trust_state(project_b_dir.path()),
        "the REAL projectB trust state must stay byte-identical through the wrong bind"
    );
    assert_eq!(
        service_b_state_before,
        service_trust_state(service_dir_b.path()),
        "serviceB trust state must stay byte-identical through the wrong bind"
    );
    assert_eq!(
        intents_b_before,
        sqlite_effect_journal_state(&service_dir_b.path().join(".swarm/state.db")),
        "serviceB durable effect journals must stay unchanged through the wrong bind"
    );
    assert!(owned
        .project
        .paea_git_agreement_match(&exact)
        .unwrap()
        .is_none());
    let _ = service_a.close();
    let _ = service_b.close();
    let _ = owned.project.close();
    let _ = owned.service.close();
    let _ = fixture_a.close();
    let _ = fixture_wrong.close();
}

/// The wrong ROLE through the same legitimate enrolled channel is a visible
/// denial by the existing approve guard, never an approval: the project is
/// configured so the enrolled principal maps to an identity WITHOUT the
/// approver role (the single enrollment-principal mapping constraint means
/// the request cannot mint another identity or role). The signed permission
/// injected here — the approve trust-action permission of the frozen
/// command — is deliberately VALID: the refusal under test is the project
/// Trust ROLE guard (`approver`), a business denial, never a signed
/// permission refusal and never `authorization_required`. The frozen approve
/// command travels the same protected route under the same held authority —
/// every gate before the guard passes (bound source, exact digest bytes,
/// real personal password, physical witness, current revision) — and the
/// engine's approve guard itself denies with no action mutation and no
/// captured agreement. Observed effects, no more and no less: the public
/// reply is the guard's own denial (HTTP 200 with `denied`, since the
/// project-side refusal is a business reply, not a transport failure); the
/// denial is RECORDED in the project's Trust journal (the trust state bytes
/// change — evidence of the visible refusal, not a hidden success); the
/// action stays `pending_approval`; no agreement matches the canonical
/// bytes; the service root's Trust state, the projectA managed refs listing
/// and the service root's durable effect journals stay byte-identical; and
/// a legitimate replay of the same frozen bytes retains the same denial
/// without a second journal entry. No Git execution happens on this route.
///
/// A missing Trust role is then recovered ONLY through the supported public
/// producers — no minted grant, no JSON claims, no lowered guard: a real
/// `config-apply` grants the `approver` role to the SAME enrolled identity
/// (the single enrollment-principal mapping forbids any second mapping) and
/// bumps the Trust revision. The bump deactivates the old mandate and stales
/// the old signed enrollment grant (enrolled authentication checks revision
/// equality; one enroll per enrollment id forbids appending), so the
/// recovery re-runs the whole supported chain at the CURRENT revision: fresh
/// recipe terms for the same canonical bytes, a NEW mandate and request
/// through the real producers, a fresh transfer preparation, a new approval
/// preparation, a new enrollment signed at the current revision and a REAL
/// HTTP approval. The project guard then recognizes the exact same canonical
/// bytes as a NEW agreement (approver `enrolled-effect`, new action id,
/// current revision); a replay retains it with no second journal entry and
/// an identical match; the denied action stays `pending_approval` forever;
/// and the service Trust state, the managed refs, the target-ref absence and
/// the durable journals stay byte-identical through the whole recovery.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn approval_route_wrong_role_denies_without_approval_or_service_effects() {
    // ProjectA uses the no-approver agreement config: the enrolled identity
    // holds admin+effect but not approver; the separate simulator approver
    // bearer exists but the route never accepts a bearer (its credential is
    // the held personal password, and the enrolled mapping is the one used).
    let owned = owned_agreement_no_approver();
    // The exact canonical action bytes and the frozen approve command, built
    // exactly like the positive test on this project's recipe terms.
    let exact = swarm_protection_contract::canonicalize(
        &proposal(&owned.repository_id, &owned.candidate),
        &context(owned.trust_revision),
    )
    .unwrap()
    .bytes()
    .to_vec();
    let terms = owned
        .project
        .paea_git_agreement_terms(&exact)
        .unwrap()
        .expect("recipe terms derivable for this canonical Git action");
    let resource = terms.resource.clone();
    let mandate_id = format!("wrong-role-mandate-{}", std::process::id());
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
    // Real producers on the project root: mandate-create needs the simulator
    // admin bearer, request needs the requester bearer — neither needs the
    // approver role, so the pending action exists before the route runs.
    trust(
        "mandate-create",
        json!({
            "schema_version":1,
            "event_id":format!("wrong-role-mandate-{mandate_id}"),
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
            "event_id":format!("wrong-role-request-{mandate_id}"),
            "mandate_id":mandate_id,
            "resource":resource,
            "amount_minor":terms.amount_minor,
            "currency":terms.currency,
            "beneficiary":terms.beneficiary,
            "idempotency_key":format!("wrong-role-key-{mandate_id}"),
        }),
        "fixture-requester-token",
    );
    assert_eq!(action["state"], "pending_approval");
    let approve = serde_json::to_vec(&json!({
        "schema_version":1,
        "event_id":format!("wrong-role-approve-{mandate_id}"),
        "action_id":action["id"],
        "mandate_id":mandate_id,
        "content_digest":action["content_digest"],
        "resource":resource,
        "amount_minor":terms.amount_minor,
        "currency":terms.currency,
        "beneficiary":terms.beneficiary,
    }))
    .unwrap();
    // Physical witnesses read before any bind: the project Trust state, the
    // service Trust state, the managed refs listing, the explicit absence
    // observation of the promotion target ref, and the durable effect
    // journals (intents/promotions) through independent read-only
    // connections. The denial below must change ONLY the project's own
    // Trust journal (the denial itself is recorded there), nothing else.
    let project_before = service_trust_state(&owned.project_path);
    let service_before = service_trust_state(&owned.service_path);
    let refs_before = managed_git_refs(&owned.project_path, &owned.work);
    let target_ref_before =
        managed_git_target_ref(&owned.project_path, &owned.work, "refs/heads/release");
    let intents_before =
        sqlite_effect_journal_state(&owned.service_path.join(".swarm/state.db"));
    // Valid bind first: the same frozen bytes against projectA's own source.
    // The bind checks the physical witness only; the role refusal comes
    // later, from the engine's approve guard inside the business call.
    let source = PaeaGitCallSource::new_transfer(
        owned.prepared,
        owned.service_path.clone(),
        owned.project_path.clone(),
    );
    let approval = source
        .prepare_approval(
            swarm_enrollment_authority::trust_action::Operation::Approve,
            &approve,
        )
        .unwrap_or_else(|e| panic!("approval preparation refused: {:?}", e.code()));
    let fixture = fixture::Fixture::build_for_trust_actions(
        vec![
            owned.prepared_request_permission.clone(),
            approval.request().permission(),
        ],
        owned.fixture_scope.clone(),
        owned.trust_revision,
    );
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let authority = swarm_web::PaeaGitCallSource::open(
        source,
        fixture.path.to_str().unwrap(),
        &fixture.pin,
    )
    .unwrap();
    let s = swarm_web::NativeHttpService::open_git_call_with_approval(
        &owned.service_path,
        LocalWebAuthority::from_listener(&listener, KEY).unwrap(),
        authority,
        Some(approval),
    )
    .unwrap_or_else(|e| panic!("approval bind refused: {:?}", e.code()));
    let good = format!("Enrollment {}", URL_SAFE_NO_PAD.encode(PASSWORD));
    // Startup observation: the approval status is the minimal projection —
    // the `bound` boolean only, no capability object (a closed vocabulary
    // does not make the selected operation a constant, BLP-RET-05). The
    // wrong role is therefore invisible at the startup layer entirely; the
    // role denial is proven by the actual guarded consumption below.
    let status = get(&listener, &s, "/api/v1/paea/git-call/approval");
    assert!(status.response.status().is_success());
    let status_body: Value = serde_json::from_slice(status.response.body()).unwrap();
    assert_eq!(status_body["bound"], true);
    assert_eq!(status_body["capability"], serde_json::Value::Null,
        "approval status must serve no request-derived capability: {status_body}");
    // The empty-body consumption under the correct held password reaches the
    // engine's approve guard with the server-held frozen command: the enrolled
    // identity lacks the approver role, so the guard denies. The route returns
    // HTTP 200 — a business denial reply from the project root is the engine's
    // typed outcome, not a transport error — with the guard's own denial state
    // and no redaction bypass.
    let denied = post(
        &listener,
        &s,
        "/api/v1/paea/git-call/approval",
        Vec::new(),
        Some(good.clone()),
    );
    assert!(
        denied.response.status().is_success(),
        "denial reply must be visible: {}",
        String::from_utf8_lossy(denied.response.body())
    );
    assert!(denied.failure.is_none());
    let denial: Value = serde_json::from_slice(denied.response.body()).unwrap();
    assert_eq!(denial["state"], "denied");
    assert_eq!(denial["business_mutated"], false);
    assert_eq!(
        denial["reason"],
        "séparation demandeur/approbateur ou intégrité insuffisante"
    );
    // The denial is recorded in the project's OWN Trust journal: the project
    // Trust state changed (visible refusal, not a hidden success). Every
    // other witness is byte-identical: the service root's Trust state, the
    // managed refs, the explicitly absent target ref and both journals.
    let project_after = service_trust_state(&owned.project_path);
    assert_ne!(
        project_before, project_after,
        "the visible denial must be recorded in the project Trust journal"
    );
    assert_eq!(
        service_before,
        service_trust_state(&owned.service_path),
        "serviceA trust state must stay byte-identical through the denial"
    );
    assert_eq!(
        refs_before,
        managed_git_refs(&owned.project_path, &owned.work),
        "projectA managed refs listing must stay byte-identical through the denial"
    );
    assert_eq!(
        target_ref_before,
        managed_git_target_ref(&owned.project_path, &owned.work, "refs/heads/release"),
        "the promotion target ref must stay explicitly absent through the denial"
    );
    assert_eq!(
        intents_before,
        sqlite_effect_journal_state(&owned.service_path.join(".swarm/state.db")),
        "serviceA durable effect journals must stay unchanged through the denial"
    );
    // The action stays pending: a role denial never mutates the action.
    let view: Value =
        serde_json::from_slice(&owned.project.trust("show", b"{}", "").unwrap().bytes).unwrap();
    let shown = view["actions"][action["id"].as_str().unwrap()]
        .clone();
    assert!(
        shown.is_object(),
        "requested action present in the project Trust view"
    );
    assert_eq!(shown["state"], "pending_approval");
    // No agreement matches the exact canonical bytes: a denial captures none.
    assert!(owned
        .project
        .paea_git_agreement_match(&exact)
        .unwrap()
        .is_none());
    // Legitimate replay: the same empty-body consumption retains the SAME
    // denial reply (the engine's replay path), with no further journal growth
    // beyond the first denial's change. The project state stays at the
    // post-denial bytes; no second denial entry appears.
    let replay = post(
        &listener,
        &s,
        "/api/v1/paea/git-call/approval",
        Vec::new(),
        Some(good),
    );
    assert!(replay.response.status().is_success());
    let replay_denial: Value = serde_json::from_slice(replay.response.body()).unwrap();
    assert_eq!(replay_denial["state"], "denied");
    assert_eq!(replay_denial["reason"], denial["reason"]);
    assert_eq!(
        project_after,
        service_trust_state(&owned.project_path),
        "the replay must retain the recorded denial without a new journal entry"
    );
    let _ = s.close();

    // ---- Legitimate recovery at the current revision ----
    // The denial proved the missing Trust role. Recovery uses only supported
    // public producers: a real config-apply grants the `approver` role to the
    // SAME enrolled identity (no second mapping is mintable), which bumps the
    // revision. That bump deactivates the old mandate and stales the old
    // signed grant, so the recovery re-runs the whole supported chain at the
    // new revision: fresh terms, new mandate + request, fresh transfer and
    // approval preparations, a new enrollment grant signed at the current
    // revision, a fresh bind and a REAL HTTP approval.
    let view: Value =
        serde_json::from_slice(&owned.project.trust("show", b"{}", "").unwrap().bytes).unwrap();
    assert_eq!(view["revision"], owned.trust_revision);
    let mut identities = view["identities"].clone();
    for identity in identities.as_array_mut().unwrap() {
        identity["disabled"] = false.into();
        identity["token"] = match identity["subject"].as_str() {
            Some("requester") => "fixture-requester-token".into(),
            Some("approver") => "fixture-approver-token".into(),
            Some("agreement-admin") => "fixture-admin-token".into(),
            _ => Value::String(String::new()),
        };
    }
    // The role grant touches ONLY the enrolled identity's role list; the
    // enrollment principal mapping is preserved exactly as the engine's show
    // view returned it (the mapping cannot be duplicated or reassigned).
    let enrolled = identities
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|i| i["subject"] == "enrolled-effect")
        .expect("enrolled identity present in the project Trust view");
    let roles = enrolled["roles"].as_array().unwrap().clone();
    assert!(
        !roles.iter().any(|r| r == "approver"),
        "precondition: the enrolled identity still lacks the approver role"
    );
    let mut granted = roles;
    granted.push(json!("approver"));
    enrolled["roles"] = Value::Array(granted);
    let grant = serde_json::json!({
        "schema_version":1,
        "event_id":format!("wrong-role-recovery-{}", std::process::id()),
        "expected_revision":owned.trust_revision,
        "reason":"Public recovery: grant the approver role to the enrolled identity after a recorded role denial",
        "policy":view["policy"],
        "identities":identities,
    });
    owned
        .project
        .trust("config-apply", &serde_json::to_vec(&grant).unwrap(), "")
        .unwrap_or_else(|e| panic!("recovery config-apply refused: {}", e.code()));
    let new_revision = current_trust_revision(&owned.project_path);
    assert_eq!(new_revision, owned.trust_revision + 1);
    // The old mandate is deactivated by the revision bump; the old pending
    // action can never be approved at the new revision. A NEW mandate and
    // request through the real producers produce the fresh pending action.
    let fresh_terms = owned
        .project
        .paea_git_agreement_terms(&exact)
        .unwrap()
        .expect("fresh recipe terms derivable for the same canonical bytes");
    assert_eq!(fresh_terms.revision, new_revision);
    let recovery_mandate_id = format!("wrong-role-recovery-mandate-{}", std::process::id());
    trust(
        "mandate-create",
        json!({
            "schema_version":1,
            "event_id":format!("wrong-role-recovery-mandate-{recovery_mandate_id}"),
            "mandate_id":recovery_mandate_id,
            "subject":"requester",
            "actions":["settle"],
            "resource":resource,
            "max_amount_minor":terms.amount_minor,
            "currency":terms.currency,
            "beneficiary":terms.beneficiary,
        }),
        "fixture-admin-token",
    );
    let fresh_action: Value = trust(
        "request",
        json!({
            "schema_version":1,
            "event_id":format!("wrong-role-recovery-request-{recovery_mandate_id}"),
            "mandate_id":recovery_mandate_id,
            "resource":resource,
            "amount_minor":terms.amount_minor,
            "currency":terms.currency,
            "beneficiary":terms.beneficiary,
            "idempotency_key":format!("wrong-role-recovery-key-{recovery_mandate_id}"),
        }),
        "fixture-requester-token",
    );
    assert_eq!(fresh_action["state"], "pending_approval");
    assert_ne!(
        fresh_action["id"], action["id"],
        "the recovery must be a NEW action, never the denied one"
    );
    let fresh_approve = serde_json::to_vec(&json!({
        "schema_version":1,
        "event_id":format!("wrong-role-recovery-approve-{recovery_mandate_id}"),
        "action_id":fresh_action["id"],
        "mandate_id":recovery_mandate_id,
        "content_digest":fresh_action["content_digest"],
        "resource":resource,
        "amount_minor":terms.amount_minor,
        "currency":terms.currency,
        "beneficiary":terms.beneficiary,
    }))
    .unwrap();
    // Fresh transfer preparation at the current revision: same canonical
    // bytes, current admission context. The old preparation's witnesses stay
    // frozen; this new preparation rebinds under the current revision.
    let fresh_prepared = owned
        .service
        .paea_git_transfer_call(
            &owned.project,
            &proposal(&owned.repository_id, &owned.candidate),
            &context(new_revision),
            &owned.work,
            "bare",
            PIN,
            "transfer-pack",
        )
        .unwrap_or_else(|e| panic!("recovery transfer preparation refused: {:?}", e));
    let fresh_prepared_permission = fresh_prepared.request().permission();
    let fresh_source = PaeaGitCallSource::new_transfer(
        fresh_prepared,
        owned.service_path.clone(),
        owned.project_path.clone(),
    );
    let fresh_approval = fresh_source
        .prepare_approval(
            swarm_enrollment_authority::trust_action::Operation::Approve,
            &fresh_approve,
        )
        .unwrap_or_else(|e| panic!("recovery approval preparation refused: {:?}", e.code()));
    // New enrollment at the CURRENT revision: the old grant cannot
    // authenticate (enrolled authentication checks revision equality) and the
    // journal accepts one enroll per enrollment id, so nothing can be
    // appended to the old fixture.
    let fresh_fixture = fixture::Fixture::build_for_trust_actions(
        vec![
            fresh_prepared_permission,
            fresh_approval.request().permission(),
        ],
        owned.fixture_scope.clone(),
        new_revision,
    );
    let fresh_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let fresh_authority = swarm_web::PaeaGitCallSource::open(
        fresh_source,
        fresh_fixture.path.to_str().unwrap(),
        &fresh_fixture.pin,
    )
    .unwrap();
    let fresh_service = swarm_web::NativeHttpService::open_git_call_with_approval(
        &owned.service_path,
        LocalWebAuthority::from_listener(&fresh_listener, KEY).unwrap(),
        fresh_authority,
        Some(fresh_approval),
    )
    .unwrap_or_else(|e| panic!("recovery approval bind refused: {:?}", e.code()));
    let fresh_good = format!("Enrollment {}", URL_SAFE_NO_PAD.encode(PASSWORD));
    // The recovered approval is bound again: minimal status only (the
    // `bound` boolean, no capability object — BLP-RET-05).
    let fresh_status = get(&fresh_listener, &fresh_service, "/api/v1/paea/git-call/approval");
    assert!(fresh_status.response.status().is_success());
    let fresh_status_body: Value = serde_json::from_slice(fresh_status.response.body()).unwrap();
    assert_eq!(fresh_status_body["bound"], true);
    assert_eq!(fresh_status_body["capability"], serde_json::Value::Null,
        "recovered approval status must serve no request-derived capability: {fresh_status_body}");
    // REAL HTTP approval under the same held password: an empty-body
    // consumption of the freshly server-held command. The enrolled identity
    // now holds the approver role at the current revision, so the engine's
    // approve guard admits it and captures the agreement.
    let approved = post(
        &fresh_listener,
        &fresh_service,
        "/api/v1/paea/git-call/approval",
        Vec::new(),
        Some(fresh_good),
    );
    assert!(
        approved.response.status().is_success(),
        "recovery approval must succeed: {}",
        String::from_utf8_lossy(approved.response.body())
    );
    assert!(approved.failure.is_none());
    let published: Value = serde_json::from_slice(approved.response.body()).unwrap();
    assert_eq!(published["state"], "approved");
    assert_eq!(published["kind"], "trust-status-ack-v1");
    assert_eq!(published["content_redacted"], true);
    // The project guard now recognizes the EXACT same canonical bytes as a
    // NEW agreement captured at the CURRENT revision by the enrolled identity.
    let matched = owned
        .project
        .paea_git_agreement_match(&exact)
        .unwrap()
        .expect("recovered exact agreement recognized by the project guard");
    assert_eq!(matched.action_id, fresh_action["id"].as_str().unwrap());
    assert_eq!(matched.approver, "enrolled-effect");
    assert_eq!(matched.revision, new_revision);
    // The recovery's effects, no more and no less: the project Trust state
    // changed (the grant + the new mandate/request/approval chain are all
    // recorded there); every other witness stays byte-identical.
    assert_ne!(
        project_after,
        service_trust_state(&owned.project_path),
        "the recovery must be recorded in the project Trust journal"
    );
    let project_recovered = service_trust_state(&owned.project_path);
    assert_eq!(
        service_before,
        service_trust_state(&owned.service_path),
        "serviceA trust state must stay byte-identical through the recovery"
    );
    assert_eq!(
        refs_before,
        managed_git_refs(&owned.project_path, &owned.work),
        "projectA managed refs listing must stay byte-identical through the recovery"
    );
    assert_eq!(
        target_ref_before,
        managed_git_target_ref(&owned.project_path, &owned.work, "refs/heads/release"),
        "the promotion target ref must stay explicitly absent through the recovery"
    );
    assert_eq!(
        intents_before,
        sqlite_effect_journal_state(&owned.service_path.join(".swarm/state.db")),
        "serviceA durable effect journals must stay unchanged through the recovery"
    );
    // The denied action stays pending forever; only the NEW action is
    // approved. A role recovery never retroactively approves a denial.
    let final_view: Value =
        serde_json::from_slice(&owned.project.trust("show", b"{}", "").unwrap().bytes).unwrap();
    let denied_still = final_view["actions"][action["id"].as_str().unwrap()].clone();
    assert_eq!(denied_still["state"], "pending_approval");
    let recovered = final_view["actions"][fresh_action["id"].as_str().unwrap()].clone();
    assert_eq!(recovered["state"], "approved");
    assert_eq!(recovered["approver"], "enrolled-effect");
    // Replay the same empty-body recovery consumption: legitimate replay
    // retains the same approval with no second journal entry and no second
    // agreement.
    let recovery_replay = post(
        &fresh_listener,
        &fresh_service,
        "/api/v1/paea/git-call/approval",
        Vec::new(),
        Some(format!("Enrollment {}", URL_SAFE_NO_PAD.encode(PASSWORD))),
    );
    assert!(recovery_replay.response.status().is_success());
    let replay_published: Value = serde_json::from_slice(recovery_replay.response.body()).unwrap();
    assert_eq!(replay_published["state"], "approved");
    assert_eq!(replay_published["kind"], "trust-status-ack-v1");
    let matched_again = owned
        .project
        .paea_git_agreement_match(&exact)
        .unwrap()
        .expect("replayed agreement still recognized");
    assert_eq!(matched_again, matched);
    assert_eq!(
        project_recovered,
        service_trust_state(&owned.project_path),
        "the replay must retain the approval without a new journal entry"
    );
    let _ = fresh_service.close();
    let _ = owned.project.close();
    let _ = owned.service.close();
    fixture.close();
    fresh_fixture.close();
}

/// Read the service root's bounded trust state bytes; empty ONLY when the file
/// is genuinely absent (an unconfigured root), never as a fallback for other
/// read errors. The route must never touch the service root: byte-equality
/// before/after the approval is the direct physical witness of that separation.
fn service_trust_state(root: &Path) -> Vec<u8> {
    match fs::read(root.join(".swarm/trust-actions.json")) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => panic!(
            "trust state read failed on {}: {e} (an actual read error is not an absence)",
            root.display()
        ),
    }
}

/// Actual Git ref lines of the work's managed bare repository, through the
/// real Git binary with the hardened environment (same pattern as the
/// engine's transfer/promote tests). Output is the raw `for-each-ref
/// --format` listing, hashed for a stable comparison basis.
fn managed_git_refs(root: &Path, work: &str) -> String {
    let git_dir = root
        .join(".swarm/managed")
        .join(work)
        .join("repository.git");
    let mut git = Command::new(git_executable());
    for (key, _) in std::env::vars_os() {
        if key.as_encoded_bytes().starts_with(b"GIT_") {
            git.env_remove(key);
        }
    }
    let out = git
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .args([
            "--git-dir",
            git_dir.to_str().expect("managed git dir encoding"),
            "for-each-ref",
            "--format=%(refname) %(objectname)",
        ])
        .output()
        .expect("git for-each-ref must run");
    assert!(
        out.status.success(),
        "git for-each-ref failed on {}: {}",
        git_dir.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("git ref listing utf-8")
}

/// Explicit observation of the promotion target ref through real Git:
/// Some(oid) when the ref exists, None when rev-parse --verify fails. A
/// successful exit with unexpected output still refuses here (typed panic),
/// so absence is never confused with a silent failure.
fn managed_git_target_ref(root: &Path, work: &str, ref_name: &str) -> Option<String> {
    let git_dir = root
        .join(".swarm/managed")
        .join(work)
        .join("repository.git");
    let mut git = Command::new(git_executable());
    for (key, _) in std::env::vars_os() {
        if key.as_encoded_bytes().starts_with(b"GIT_") {
            git.env_remove(key);
        }
    }
    let out = git
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .args([
            "--git-dir",
            git_dir.to_str().expect("managed git dir encoding"),
            "rev-parse",
            "--verify",
            "--quiet",
            ref_name,
        ])
        .output()
        .expect("git rev-parse must run");
    if out.status.success() {
        let oid = String::from_utf8(out.stdout).expect("git oid utf-8");
        assert_eq!(oid.trim().len(), 40, "canonical oid shape");
        Some(oid.trim().to_owned())
    } else {
        assert!(
            out.stdout.is_empty(),
            "failing rev-parse must not emit an oid: {}",
            String::from_utf8_lossy(&out.stdout)
        );
        None
    }
}

/// Count rows of the actual durable effect journals (Git intents, promotion
/// effects) in a service root's state.db through an independent read-only
/// connection — the same reader pattern the settlement tests use. Table
/// absence is an explicit observation (count 0 plus an explicit marker),
/// never an error and never fabricated state; real read errors refuse.
fn sqlite_effect_journal_state(db_path: &Path) -> String {
    let db = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap_or_else(|e| panic!("read-only state.db open failed on {db_path:?}: {e}"));
    let mut report = String::new();
    for table in ["paea_git_intent_meta", "paea_git_intents", "paea_git_promote_meta", "paea_git_promote_effects"] {
        let exists: i64 = db
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name=?",
                [table],
                |r| r.get(0),
            )
            .unwrap_or_else(|e| panic!("sqlite_master read failed for {table}: {e}"));
        let rows: i64 = if exists == 1 {
            db.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap_or_else(|e| panic!("count read failed for {table}: {e}"))
        } else {
            0
        };
        report.push_str(&format!("{table}:{exists}:{rows};"));
    }
    db.close()
        .unwrap_or_else(|e| panic!("read-only state.db close failed: {e:?}"));
    report
}

/// Public prepared-source closes complete the ownership frontier: a prepared
/// transfer source and its separately frozen approval are each released
/// exactly once through the public `close_prepared` contracts, with no
/// authority opened, no declaration, no HTTP and no journal write anywhere.
/// Observations, no more and no less: the project and service Trust states
/// byte-identical through the preparation and both closes, and a FRESH
/// preparation on the same roots succeeding afterwards (explicit release,
/// never a Drop or process-exit substitution). The approval witness is closed
/// alone first, proving the approval owner's close consumes only its own
/// witness and leaves the source closeable exactly once. Close failures stay
/// typed through the existing engine signatures; this test exercises the
/// success contract on legitimate witnesses.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn prepared_source_public_close_releases_witnesses_and_permits_fresh_preparation() {
    let owned = owned();
    let project_before = service_trust_state(&owned.project_path);
    let service_before = service_trust_state(&owned.service_path);
    // Legitimate witnesses first: one prepared transfer source plus its
    // frozen approval command against the source's own project root. The
    // command bytes are fixture-only (digested, never executed, never
    // approved); no declaration is provisioned and no authority is opened.
    let source = PaeaGitCallSource::new_transfer(
        owned.prepared,
        owned.service_path.clone(),
        owned.project_path.clone(),
    );
    let approval = source
        .prepare_approval(
            swarm_enrollment_authority::trust_action::Operation::Approve,
            b"close-contract-fixture-approve-command",
        )
        .unwrap_or_else(|e| panic!("approval preparation refused: {:?}", e.code()));
    // The approval witness closes alone through its own public contract.
    approval
        .close_prepared()
        .unwrap_or_else(|e| panic!("approval witness close refused: {:?}", e.code()));
    // The source close then releases the held transfer witness exactly once.
    source
        .close_prepared()
        .unwrap_or_else(|e| panic!("prepared source close refused: {:?}", e.code()));
    // No durable effect from preparation or closes: both Trust stores
    // byte-identical to before any preparation.
    assert_eq!(
        project_before,
        service_trust_state(&owned.project_path),
        "project trust state must stay byte-identical through preparation and closes"
    );
    assert_eq!(
        service_before,
        service_trust_state(&owned.service_path),
        "service trust state must stay byte-identical through preparation and closes"
    );
    // Fresh recovery on the same roots: the released witnesses permit a new
    // preparation and its own explicit close — the frontier stays usable
    // without any process exit or Drop fallback.
    let fresh = owned
        .service
        .paea_git_transfer_call(
            &owned.project,
            &proposal(&owned.repository_id, &owned.candidate),
            &context(owned.trust_revision),
            &owned.work,
            "bare",
            PIN,
            "transfer-pack",
        )
        .unwrap_or_else(|e| panic!("fresh preparation refused after explicit closes: {:?}", e));
    PaeaGitCallSource::new_transfer(
        fresh,
        owned.service_path.clone(),
        owned.project_path.clone(),
    )
    .close_prepared()
    .unwrap_or_else(|e| panic!("fresh source close refused: {:?}", e.code()));
    assert_eq!(
        project_before,
        service_trust_state(&owned.project_path),
        "project trust state must stay byte-identical through the fresh preparation"
    );
    let _ = owned.project.close();
    let _ = owned.service.close();
}
