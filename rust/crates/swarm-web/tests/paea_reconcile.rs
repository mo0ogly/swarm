#![cfg(target_os = "linux")]
//! Protected read-only PA-09 reconciliation route against real distinct Roots:
//! explicit uninitialized report without mutation, shared-root refusal, saved
//! settings consumption and strict transport refusals. No effect, authority,
//! ledger initialization or acceptance is claimed by any covered path.
use http::{
    header::{COOKIE, HOST, ORIGIN},
    Request, StatusCode,
};
use std::{fs, net::TcpListener, os::unix::fs::PermissionsExt, path::PathBuf};
use swarm_engine::{EngineSettingsChange, RootStore, SettingCount, SystemEnvironment};
use swarm_web::{HttpReply, LocalWebAuthority, NativeHttpService};

struct Fixture {
    root: PathBuf,
    ledger: PathBuf,
    listener: TcpListener,
    key: String,
}
impl Fixture {
    /// Real owned service Root plus a physically distinct uninitialized ledger
    /// Root; all test data stays under these 0700 temp directories.
    fn new() -> Self {
        let mut make = |prefix: &str| {
            let dir = std::env::temp_dir()
                .join(swarm_engine::generate_identifier(prefix).unwrap());
            fs::create_dir(&dir).unwrap();
            fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
            let store = RootStore::initialize(&dir, &mut SystemEnvironment).unwrap();
            store.close().unwrap();
            dir
        };
        let root = make("paea-reconcile-http94-");
        let ledger = make("paea-reconcile-ledger94-");
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let r = RootStore::open(&root, true).unwrap();
        let key = r.web_session(&listener).unwrap().secret().to_owned();
        r.close().unwrap();
        Self {
            root,
            ledger,
            listener,
            key,
        }
    }
    /// Save the ledger path through the real settings contract (preview+apply),
    /// returning the actual applied revision; no direct file substitution.
    fn configure_ledger(&self, service: &NativeHttpService, path_value: &str) -> SettingCount {
        let request = |method: &str, path: &str, raw: Vec<u8>| {
            let a = LocalWebAuthority::from_listener(&self.listener, &self.key).unwrap();
            Request::builder()
                .method(method)
                .uri(path)
                .header(HOST, a.host())
                .header(COOKIE, format!("{}={}", a.cookie_name(), self.key))
                .header(ORIGIN, format!("http://{}", a.host()))
                .header("X-Swarm-CSRF", &self.key)
                .body(raw)
                .unwrap()
        };
        let document: serde_json::Value = serde_json::from_slice(
            service
                .handle(request("GET", "/api/v1/native-settings", vec![]))
                .unwrap()
                .response
                .body(),
        )
        .unwrap();
        let mut values = document["values"].clone();
        values["paea_settlement_ledger_root"] = path_value.into();
        let mut change = EngineSettingsChange {
            schema_version: document["schema_version"].as_i64().unwrap(),
            event_id: "configure-ledger".into(),
            expected_revision: serde_json::from_value(document["revision"].clone()).unwrap(),
            values: serde_json::from_value(values).unwrap(),
            reason: "Real saved ledger path".into(),
            preview_token: String::new(),
        };
        let preview: serde_json::Value = serde_json::from_slice(
            service
                .handle(request(
                    "POST",
                    "/api/v1/native-settings/preview",
                    serde_json::to_vec(&change).unwrap(),
                ))
                .unwrap()
                .response
                .body(),
        )
        .unwrap();
        change.preview_token = preview["preview_token"].as_str().unwrap().into();
        let applied: serde_json::Value = serde_json::from_slice(
            service
                .handle(request(
                    "POST",
                    "/api/v1/native-settings/apply",
                    serde_json::to_vec(&change).unwrap(),
                ))
                .unwrap()
                .response
                .body(),
        )
        .unwrap();
        serde_json::from_value(applied["document"]["revision"].clone()).unwrap()
    }
}
impl Drop for Fixture {
    /// Preserve failed fixtures; remove only these owned successful roots.
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.root);
            let _ = fs::remove_dir_all(&self.ledger);
        }
    }
}
/// Authorized GET with a real Host/session/CSRF; other verbs stay available.
fn get(f: &Fixture, s: &NativeHttpService, path: &str) -> HttpReply {
    let a = LocalWebAuthority::from_listener(&f.listener, &f.key).unwrap();
    s.handle(
        Request::builder()
            .method("GET")
            .uri(path)
            .header(HOST, a.host())
            .header(COOKIE, format!("{}={}", a.cookie_name(), f.key))
            .body(vec![])
            .unwrap(),
    )
    .unwrap()
}
/// The route reads the saved distinct ledger path and returns the explicit
/// uninitialized v1 report for both domains; DB bytes stay identical.
#[test]
fn reconcile_effects_reports_explicit_uninitialized_from_saved_ledger_path() {
    let f = Fixture::new();
    let s = NativeHttpService::open_local(&f.root, &f.listener).unwrap();
    f.configure_ledger(&s, f.ledger.to_str().unwrap());
    let before_service = fs::read(f.root.join(".swarm/state.db")).unwrap();
    let before_ledger = fs::read(f.ledger.join(".swarm/state.db")).unwrap();
    let reply = get(&f, &s, "/api/v1/paea/reconcile-effects");
    assert_eq!(reply.response.status(), StatusCode::OK, "{:?}", reply.failure);
    let report: serde_json::Value =
        serde_json::from_slice(reply.response.body()).unwrap();
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["settlement"]["state"], "uninitialized");
    assert_eq!(report["git"]["state"], "uninitialized");
    assert_eq!(fs::read(f.root.join(".swarm/state.db")).unwrap(), before_service);
    assert_eq!(fs::read(f.ledger.join(".swarm/state.db")).unwrap(), before_ledger);
    // HEAD mirrors status without a body; the paired close stays observable.
    let head = get(&f, &s, "/api/v1/paea/reconcile-effects");
    assert_eq!(head.response.status(), StatusCode::OK);
    s.close().unwrap();
}
/// The engine's separation guard surfaces through the route: a saved ledger
/// path equal to the service root refuses with the exact code, mutating
/// nothing, and the route recovers for later authorized calls.
#[test]
fn reconcile_effects_refuses_shared_saved_root_and_mutation_never_occurs() {
    let f = Fixture::new();
    let s = NativeHttpService::open_local(&f.root, &f.listener).unwrap();
    // Point the saved setting at the service root itself.
    f.configure_ledger(&s, f.root.to_str().unwrap());
    let reply = get(&f, &s, "/api/v1/paea/reconcile-effects");
    assert_eq!(reply.response.status(), StatusCode::CONFLICT);
    let failure = reply.failure.expect("typed refusal");
    // The engine's exact separation code surfaces through the HTTP error body.
    assert_eq!(failure.code(), "paea-effect-reconcile-separate-ledger-required");
    s.close().unwrap();
}
/// Without a session the route refuses before any open; with a session but a
/// body or query it refuses without reading settings or opening the ledger.
#[test]
fn reconcile_effects_refuses_unauthenticated_and_malformed_requests_without_io() {
    let f = Fixture::new();
    let s = NativeHttpService::open_local(&f.root, &f.listener).unwrap();
    let a = LocalWebAuthority::from_listener(&f.listener, &f.key).unwrap();
    let no_cookie = Request::builder()
        .method("GET")
        .uri("/api/v1/paea/reconcile-effects")
        .header(HOST, a.host())
        .body(vec![])
        .unwrap();
    let reply = s.handle(no_cookie).unwrap();
    assert_eq!(reply.response.status(), StatusCode::FORBIDDEN);
    for (method, uri, body) in [
        ("POST", "/api/v1/paea/reconcile-effects", vec![b'x']),
        ("GET", "/api/v1/paea/reconcile-effects?q=1", vec![]),
    ] {
        let bad = Request::builder()
            .method(method)
            .uri(uri)
            .header(HOST, a.host())
            .header(COOKIE, format!("{}={}", a.cookie_name(), f.key))
            .body(body)
            .unwrap();
        let reply = s.handle(bad).unwrap();
        assert!(!reply.response.status().is_success(), "{method} {uri}");
        assert!(reply.failure.is_some());
    }
    s.close().unwrap();
}

// --- Reader frontier (BLP-RET-05 / PA-09): reproduction and contract tests ---

#[path = "../../swarm-enrollment-authority/tests/common/authenticated_fixture.rs"]
mod fixture;

use serde_json::{json, Value};
use swarm_engine::{PaeaEffectReconcileJournal, PaeaSettlementOutcome};
use swarm_enrollment_authority::{
    authenticated_control::Context as UseContext, authenticated_startup,
};
use swarm_enrollment_state::Context;
use swarm_identity_contract::Scope;

const RECONCILE_REPORT: &str = "/api/v1/paea/reconcile-effects";

/// Request helper honoring an explicit method (the older helper hardcodes GET,
/// which made the "HEAD mirrors" assertion a second GET — corrected here).
fn request(
    f: &Fixture,
    s: &NativeHttpService,
    method: &str,
    path: &str,
    authorization: Option<&str>,
) -> HttpReply {
    let a = LocalWebAuthority::from_listener(&f.listener, &f.key).unwrap();
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header(HOST, a.host())
        .header(COOKIE, format!("{}={}", a.cookie_name(), f.key));
    if let Some(token) = authorization {
        builder = builder.header(http::header::AUTHORIZATION, token);
    }
    s.handle(builder.body(vec![]).unwrap()).unwrap()
}

/// Real retained native source fixture (never a copy): the same harness the
/// engine reconciliation tests use, via the permanent recipe environment.
fn source_project() -> (PathBuf, RootStore, Scope, String) {
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
    let scope = scope_of(&path, &work);
    (path, root, scope, work)
}

fn scope_of(dir: &std::path::Path, work: &str) -> Scope {
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

/// Identity configuration for a FRESH project root: the same labeled
/// identities as the engine harness (blp-principal check passes; auth
/// simulated, acknowledged — the real human enrollment stays an open
/// precondition of live operation). The retained fixture is already
/// configured (its own revision), so this apply-once guard never re-applies.
fn identity_config() -> Value {
    json!({
        "schema_version":1,
        "event_id":"reconcile-reader-config",
        "expected_revision":0,
        "reason":"Explicit fixture effect role for the reconcile reader frontier",
        "policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":false},
        "identities":[
            {"subject":"enrolled-effect","integrity":3,"roles":["admin","effect"],"enrollment_principal":{"principal_id":"operator","principal_kind":"human","issuer":"https://fixture.invalid","subject":"stable-person"}},
            {"subject":"requester","token":"fixture-requester-token","integrity":1,"roles":["requester"]},
            {"subject":"approver","token":"fixture-approver-token","integrity":2,"roles":["approver"]},
            {"subject":"agreement-admin","token":"fixture-admin-token","integrity":3,"roles":["admin"]}
        ]
    })
}

/// Apply the identity configuration exactly once per root: a fresh root
/// applies at revision 0; the retained fixture (already configured with the
/// same labeled identities) is left untouched.
fn ensure_identities(project: &RootStore, project_path: &std::path::Path) {
    if !project_path.join(".swarm/trust-actions.json").exists() {
        project
            .trust("config-apply", &serde_json::to_vec(&identity_config()).unwrap(), "")
            .unwrap_or_else(|e| panic!("explicit fixture config refused: {}", e.code()));
    }
}

/// One real settlement effect recorded in the ledger through the supported
/// two-phase producers (initialize + request + real enrolled authority call),
/// mirroring the engine reconciliation harness. This fills the saved ledger so
/// the route's report carries a non-empty finding (intent_id + action_digest)
/// — the classified projection the reader frontier must guard.
fn seed_settlement_effect(project: &RootStore, ledger: &RootStore, scope: &Scope) {
    let doc = swarm_protection_contract::decode_document(include_bytes!(
        "../../../../web/paea-examples/v1/payment-document.json"
    ))
    .unwrap();
    let (proposal, context) = (doc.proposal, doc.context);
    ledger
        .paea_initialize_settlement(
            &context.project_id,
            &"0b".repeat(32),
            std::slice::from_ref(&proposal),
        )
        .unwrap();
    let request = project
        .paea_settlement_request(
            ledger,
            &proposal,
            &context,
            swarm_enrollment_authority::paea_action::Operation::Commit,
        )
        .unwrap();
    let f = fixture::Fixture::build_for_trust(
        request.request().permission(),
        scope.clone(),
        1,
    );
    let (app, head) =
        authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin).unwrap();
    let outcome = ledger
        .call_paea_settlement(
            project,
            &request,
            &app,
            &UseContext::new(),
            &head,
            b"STARTUP_FIXTURE_PASSWORD",
        )
        .map_err(|e| match e {
            swarm_engine::PaeaSettlementCallError::Action(error) => error,
            other => panic!("unexpected current authority refusal: {}", other.code()),
        })
        .unwrap();
    app.close().unwrap();
    f.close();
    match outcome {
        PaeaSettlementOutcome::Recorded { .. } => {}
        other => panic!("expected recorded settlement, got {other:?}"),
    }
}

/// REPRODUCTION (red before the fix): with a NON-EMPTY reconciliation report
/// actually obtainable, the cookie alone currently receives 200 + the full
/// report (schema, journal states, findings with intent_id/action_digest) —
/// another classified projection served with no reader identity and no
/// CURRENT BLP attribution. The classified content here is the real durable
/// effect's exact intent_id/action_digest disclosed by the report itself.
///
/// The project/ledger roots are FRESH owned roots: the settlement harness is
/// autonomous (real work.create + config-apply + initialize/request + the
/// real enrolled authority), exactly like the engine reconciliation suite;
/// the retained source fixture is NOT required for the settlement domain.
#[test]
#[ignore = "requires explicit retained native source fixture"]
fn reconcile_effects_reproduction_serves_report_to_bare_session() {
    // Fresh project root with a real work + fixture identity configuration
    // (the settlement harness needs a real plan head for the Scope).
    let (pdir, mut pstore) = {
        let dir = tempfile::tempdir().unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let store = RootStore::initialize(dir.path(), &mut SystemEnvironment).unwrap();
        (dir, store)
    };
    let work = pstore
        .execute_external(
            "",
            "work.create",
            br#"{"schema_version":1,"event_id":"create","expected_revision":0,"title":"Reconcile reader plan","objective":"Exact fictitious effect","scope":"Fixture","criteria":["No duplicate invoice"]}"#,
            &mut SystemEnvironment,
        )
        .unwrap()
        .result
        .work
        .id;
    // The settlement policy read requires a configured current Trust state on
    // the project root (same precondition as the engine harness's project()).
    pstore
        .trust("config-apply", &serde_json::to_vec(&identity_config()).unwrap(), "")
        .unwrap_or_else(|e| panic!("explicit fixture config refused: {}", e.code()));
    let scope = scope_of(pdir.path(), &work);
    let f = Fixture::new();
    let ledger = RootStore::open(&f.ledger, true).unwrap();
    seed_settlement_effect(&pstore, &ledger, &scope);
    ledger.close().unwrap();
    let s = NativeHttpService::open_local(&f.root, &f.listener).unwrap();
    f.configure_ledger(&s, f.ledger.to_str().unwrap());
    // Cookie alone (no reader credential): the defect observes 200 + report.
    let reply = request(&f, &s, "GET", RECONCILE_REPORT, None);
    let body: Value = serde_json::from_slice(reply.response.body()).unwrap();
    println!(
        "REPRODUCTION observed: status={} body={}",
        reply.response.status(),
        serde_json::to_string(&body).unwrap()
    );
    s.close().unwrap();
    pstore.close().unwrap();
}
