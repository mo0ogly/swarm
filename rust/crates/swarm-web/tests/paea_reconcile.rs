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

/// Shorthand used by the PA-09 native witness helpers: an exclusive directory
/// creation that is 0700 FROM BIRTH (`DirBuilderExt::mode`), never a
/// 0755-then-chmod window. Inputs: the fresh directory path to create.
/// Output: `Ok(())` on exclusive creation. Errors: the raw
/// `std::io::Error` (including `AlreadyExists` for reuse/concurrent claims)
/// is returned to the caller, never masked. Effects: at most one new
/// directory at the exact path; no parent is created or touched.
fn create_dir_mode_0700(dir: &std::path::Path) -> std::io::Result<()> {
    fs::DirBuilder::new().mode(0o700).create(dir)
}

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
        let make = |prefix: &str| {
            let dir = swarm_engine::fixture_root(prefix);
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
/// Reader frontier (BLP-RET-05 / PA-09): WITHOUT a reader credential the
/// report is refused even with the saved distinct ledger configured — the
/// session alone is never authorization. GET answers the explicit
/// `available:false` projection with ONLY the stable guard code; HEAD answers
/// the typed 403 refusal with an empty body (observably a refusal, never an
/// empty-200 success). Both mutate nothing and reveal no ledger/settings IO.
#[test]
fn reconcile_effects_refuses_bare_session_with_stable_reader_code() {
    let f = Fixture::new();
    let s = NativeHttpService::open_local(&f.root, &f.listener).unwrap();
    f.configure_ledger(&s, f.ledger.to_str().unwrap());
    let before_service = fs::read(f.root.join(".swarm/state.db")).unwrap();
    let before_ledger = fs::read(f.ledger.join(".swarm/state.db")).unwrap();
    // GET, cookie alone: typed 403 refusal, the stable code in the typed
    // failure surface, no report field anywhere.
    let reply = get(&f, &s, "/api/v1/paea/reconcile-effects");
    assert_eq!(reply.response.status(), StatusCode::FORBIDDEN, "{:?}", reply.failure);
    let failure = reply.failure.as_ref().expect("typed refusal");
    assert_eq!(failure.code(), "paea-reader-authentication-required");
    let raw = String::from_utf8(reply.response.body().to_vec()).unwrap();
    assert!(!raw.contains("settlement"), "GET refusal disclosed a journal state");
    assert!(!raw.contains("findings"), "GET refusal disclosed report fields");
    // HEAD, cookie alone: typed 403 with an empty body — the refusal is
    // observable as a refusal, never an empty-200 success mirror.
    let head = request(&f, &s, "HEAD", "/api/v1/paea/reconcile-effects", None);
    assert_eq!(head.response.status(), StatusCode::FORBIDDEN);
    assert!(head.response.body().is_empty());
    assert!(head.failure.is_some(), "HEAD refusal must carry the typed failure");
    // No journal mutation and no settings/ledger IO side effect on either root.
    assert_eq!(fs::read(f.root.join(".swarm/state.db")).unwrap(), before_service);
    assert_eq!(fs::read(f.ledger.join(".swarm/state.db")).unwrap(), before_ledger);
    s.close().unwrap();
}
/// The engine's separation guard surfaces through the route for a reader whose
/// credential is RECOGNIZED against the service root's current trust state (the
/// identity precheck passed): a saved ledger path equal to the service root
/// refuses with the exact code, mutating nothing, and the route recovers for
/// later authorized calls. The fixture identities are applied on f.root itself
/// through the supported config producer (same bootstrap as the unknown-token
/// witness below): before that requirement the test observed the separation
/// 409 for an UNRECOGNIZED credential — exactly the ledger-disclosure class the
/// identity precheck now refuses at 403 `authorization_required`.
#[test]
fn reconcile_effects_refuses_shared_saved_root_and_mutation_never_occurs() {
    let f = Fixture::new();
    let s = NativeHttpService::open_local(&f.root, &f.listener).unwrap();
    // Point the saved setting at the service root itself.
    f.configure_ledger(&s, f.root.to_str().unwrap());
    // REAL fixture identities on the service root so `fixture-admin-token` is
    // a recognized credential (paired-close discipline of the received
    // bootstrap in the unknown-token witness).
    let service = RootStore::open(&f.root, true).unwrap();
    match service.trust(
        "config-apply",
        &serde_json::to_vec(&identity_config()).unwrap(),
        "",
    ) {
        Ok(_) => service.close().unwrap(),
        Err(e) => {
            let code = e.code();
            match service.close() {
                Ok(()) => panic!("config-apply refused: {}", code),
                Err(close_error) => panic!(
                    "config-apply refused: {} (close also failed: {})",
                    code,
                    close_error.code()
                ),
            }
        }
    }
    let reply = request(
        &f,
        &s,
        "GET",
        "/api/v1/paea/reconcile-effects",
        Some("Bearer fixture-admin-token"),
    );
    assert_eq!(reply.response.status(), StatusCode::CONFLICT);
    let failure = reply.failure.expect("typed refusal");
    // The engine's exact separation code surfaces through the HTTP error body.
    assert_eq!(failure.code(), "paea-effect-reconcile-separate-ledger-required");
    s.close().unwrap();
}
/// RED REGRESSION WITNESS (glm-reconcile-auth-counterproof-20261102T091806Z)
/// — reader-identity frontier BEFORE any ledger IO: a reader credential that
/// is UNKNOWN must be refused as an IDENTITY refusal (the typed
/// `authorization_required` non-success, 403 for both verbs) BEFORE any
/// settings/ledger/reconciliation IO can surface ITS errors to that caller.
/// The simplest counter-example is the shared-root configuration of the
/// separation test above, read with an unknown Bearer token: today the route
/// authenticates nobody, reaches the engine's separation guard first and
/// answers the LEDGER-side business refusal (409
/// `paea-effect-reconcile-separate-ledger-required`) — disclosing ledger-side
/// state to a caller with no recognizable identity. This test asserts the
/// REQUIRED outcome (identity refusal first); until the identity pre-check
/// exists it FAILS BY DESIGN, and the observed ledger/separation code in the
/// assert message IS the red discriminant handed to the next code task. It
/// must not be adjusted to pass without that guard.
///
/// The positive non-empty-report witness and this fixture's helpers are
/// RECEIVED evidence (glm-reconcile-fixture-compile-20261102T083208Z
/// supervisor-receipt.json: 1 PASS personally re-executed by Codex on stable
/// sources); their scope is reused unchanged — real owned Roots, the public
/// settings producers, a valid session/Host on the REAL service root, no live
/// base and no fabricated signal: the session, Host and setup are the same
/// valid preconditions as the received matrix, and the credential differs
/// only by being unknown (never transport-malformed). The service root carries
/// the fixture's REAL identities — applied on f.root itself through the
/// supported config producer with the received bootstrap — so the unknown
/// credential is refused against a configured identity state, never against an
/// absent setup. GET and HEAD share the refused branch: both must answer the
/// stable identity refusal (HEAD with the empty body) and neither may mutate
/// either Root.
#[test]
fn reconcile_effects_unknown_token_refused_identity_before_ledger_error() {
    let f = Fixture::new();
    let s = NativeHttpService::open_local(&f.root, &f.listener).unwrap();
    // Same public settings producer as the received matrix: the saved ledger
    // path designates the service Root itself (shape-valid), so the route's
    // ledger-side path reaches the separation guard — the first frontier an
    // unknown credential must never be allowed to observe.
    f.configure_ledger(&s, f.root.to_str().unwrap());
    // REAL fixture identities on the service root, through the supported
    // producer with the received bootstrap (empty token, expected_revision 0)
    // and the paired-close discipline of the received witness. This makes the
    // unknown-credential refusal an identity decision against a CONFIGURED
    // state, never an artifact of a missing setup.
    let service = RootStore::open(&f.root, true).unwrap();
    match service.trust(
        "config-apply",
        &serde_json::to_vec(&identity_config()).unwrap(),
        "",
    ) {
        Ok(_) => service.close().unwrap(),
        Err(e) => {
            let code = e.code();
            match service.close() {
                Ok(()) => panic!("config-apply refused: {}", code),
                Err(close_error) => panic!(
                    "config-apply refused: {} (close also failed: {})",
                    code,
                    close_error.code()
                ),
            }
        }
    }
    let before_service = fs::read(f.root.join(".swarm/state.db")).unwrap();
    let before_ledger = fs::read(f.ledger.join(".swarm/state.db")).unwrap();
    // GET with an UNKNOWN reader credential: the REQUIRED contract is the
    // typed identity refusal — 403 with the stable `authorization_required`
    // code in the failure surface — before ANY ledger/separation error can
    // be observed. The observed status/code are carried in the assert
    // messages so the red run names the concrete business error it surfaced.
    let reply = request(
        &f,
        &s,
        "GET",
        "/api/v1/paea/reconcile-effects",
        Some("Bearer unknown-reader-token"),
    );
    let status = reply.response.status();
    let observed = reply.failure.as_ref().map(|e| e.code().to_owned());
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "unknown reader token must receive the identity refusal before any ledger IO; \
         observed {status} with code {observed:?}"
    );
    assert_eq!(
        observed.as_deref(),
        Some("authorization_required"),
        "unknown reader token observed a ledger/separation business code instead of the \
         stable identity refusal: {observed:?}"
    );
    let raw = String::from_utf8(reply.response.body().to_vec()).unwrap();
    assert!(
        !raw.contains("settlement") && !raw.contains("findings"),
        "GET identity refusal disclosed ledger/report fields: {raw}"
    );
    // HEAD shares the refused branch: same identity refusal, empty body.
    let head = request(
        &f,
        &s,
        "HEAD",
        "/api/v1/paea/reconcile-effects",
        Some("Bearer unknown-reader-token"),
    );
    let head_status = head.response.status();
    let head_observed = head.failure.as_ref().map(|e| e.code().to_owned());
    assert_eq!(
        head_status,
        StatusCode::FORBIDDEN,
        "HEAD unknown reader token must mirror the identity refusal; \
         observed {head_status} with code {head_observed:?}"
    );
    assert_eq!(
        head_observed.as_deref(),
        Some("authorization_required"),
        "HEAD observed a ledger/separation business code instead of the identity refusal: \
         {head_observed:?}"
    );
    assert!(
        head.response.body().is_empty(),
        "HEAD refusal must keep the empty body"
    );
    // No mutation of either Root by the refused reads.
    assert_eq!(fs::read(f.root.join(".swarm/state.db")).unwrap(), before_service);
    assert_eq!(fs::read(f.ledger.join(".swarm/state.db")).unwrap(), before_ledger);
    s.close().unwrap();
}
/// DISABLED-READER STATE-TRANSITION WITNESS
/// (glm-pa09-disabled-reader-20261102T112514Z) — the gate-2 identity precheck
/// must re-read the service Root's CURRENT trust state at every consumption,
/// never serve a cached identity observation. A RECOGNIZED fixture admin
/// reader (identities applied on f.root through the supported config
/// producer, received bootstrap) first reaches the engine's separation fault
/// with the saved ledger path designating the service Root itself (409
/// `paea-effect-reconcile-separate-ledger-required` — the invalid-ledger
/// witness shape, proving the fault path is reachable for a known reader).
/// After the SUPPORTED disable operation (the received matrix producer:
/// `trust("show")` → token re-binding by subject → `config-apply` with
/// `disabled:true` on the target identity at the current persisted revision),
/// the SAME token on the SAME session/Host is refused by the precheck with
/// 403 `authorization_required` BEFORE any ledger IO. The observed code
/// discriminates: the ledger stays invalid-by-construction, so any other
/// code (notably the separation 409) would prove ledger IO preceded the
/// identity decision. GET carries the stable code; HEAD is the empty-body
/// typed refusal; neither carries a report or ledger field. After the
/// SUPPORTED re-enable (same producer, `disabled:false` everywhere) the
/// known fault path returns verbatim — the precheck re-read current bytes,
/// no identity cache. Every read is bracketed by state.db/trust-journal
/// snapshots taken after each configuration transition: those transitions
/// are the only authorized fixture mutations; refused reads mutate nothing.
///
/// Limits: same-UID fixture identities are labeled simulation — this proves
/// nothing about a human identity, OS confinement or any acceptance; no
/// report attribution or declassification is exercised (the ledger stays
/// invalid by construction); the received five-test scope of this file is
/// unchanged.
#[test]
fn reconcile_effects_known_reader_disabled_then_reenabled_before_invalid_ledger() {
    let f = Fixture::new();
    let s = NativeHttpService::open_local(&f.root, &f.listener).unwrap();
    // Invalid-ledger witness shape (the shared-root configuration of the
    // received separation test): the saved ledger path designates the
    // service Root itself.
    f.configure_ledger(&s, f.root.to_str().unwrap());
    // REAL fixture identities on the service root, through the supported
    // producer with the received bootstrap and the paired-close discipline.
    let service = RootStore::open(&f.root, true).unwrap();
    match service.trust(
        "config-apply",
        &serde_json::to_vec(&identity_config()).unwrap(),
        "",
    ) {
        Ok(_) => service.close().unwrap(),
        Err(e) => {
            let code = e.code();
            match service.close() {
                Ok(()) => panic!("config-apply refused: {}", code),
                Err(close_error) => panic!(
                    "config-apply refused: {} (close also failed: {})",
                    code,
                    close_error.code()
                ),
            }
        }
    }
    // Supported control operations on the service root: the received
    // paired-close discipline of this file's frontier contract witness.
    let trust = |op: &str, value: Value, token: &str| -> Value {
        let service = RootStore::open(&f.root, true).unwrap();
        match service.trust(op, &serde_json::to_vec(&value).unwrap(), token) {
            Ok(reply) => {
                service.close().unwrap();
                serde_json::from_slice(&reply.bytes).unwrap()
            }
            Err(e) => {
                let code = e.code();
                match service.close() {
                    Ok(()) => panic!("{op} refused: {}", code),
                    Err(close_error) => panic!(
                        "{op} refused: {} (close also failed: {})",
                        code,
                        close_error.code()
                    ),
                }
            }
        }
    };
    // The received matrix's supported disable/enable producer: read the
    // CURRENT public identity view, re-bind the local tokens by subject
    // (the public view never carries tokens), set `disabled` only on the
    // target identity, and apply at the current persisted revision. The
    // enrolled identity keeps its carried enrollment mapping with an empty
    // local token (enrollment inputs require exactly that).
    let reconfigured = |disabled_admin: bool, prefix: &str| {
        let view: Value = trust("show", json!({}), "");
        let revision = view["revision"].as_i64().expect("fixture Trust revision");
        let mut identities = view["identities"].clone();
        for identity in identities.as_array_mut().unwrap() {
            identity["token"] = match identity["subject"].as_str() {
                Some("requester") => "fixture-requester-token".into(),
                Some("approver") => "fixture-approver-token".into(),
                Some("agreement-admin") => "fixture-admin-token".into(),
                _ => Value::String(String::new()),
            };
            identity["disabled"] =
                (disabled_admin && identity["subject"].as_str() == Some("agreement-admin")).into();
        }
        trust(
            "config-apply",
            json!({
                "schema_version":1,
                "event_id":format!("{prefix}-{revision}"),
                "expected_revision":revision,
                "reason":"PA09 disabled-reader witness: supported identity state transition",
                "policy":view["policy"],
                "identities":identities,
            }),
            "",
        );
    };
    // Journal byte-witnesses: state.db on both roots plus the service trust
    // journal; snapshotted after every configuration transition, compared
    // after every refused read.
    let service_db = f.root.join(".swarm/state.db");
    let ledger_db = f.ledger.join(".swarm/state.db");
    let service_journal = f.root.join(".swarm/trust-actions.json");
    let snapshot = || -> Vec<(std::path::PathBuf, Vec<u8>)> {
        vec![
            (service_db.clone(), fs::read(&service_db).unwrap()),
            (ledger_db.clone(), fs::read(&ledger_db).unwrap()),
            (service_journal.clone(), fs::read(&service_journal).unwrap()),
        ]
    };
    let assert_unchanged = |baseline: &[(std::path::PathBuf, Vec<u8>)]| {
        for (path, bytes) in baseline {
            assert_eq!(
                &fs::read(path).unwrap(),
                bytes,
                "{} was mutated by a refused read",
                path.display()
            );
        }
    };
    // WITNESS 1 — the KNOWN reader reaches the separation fault: the fault
    // path is reachable for a recognized credential (pre-disable state).
    let baseline = snapshot();
    let reply = request(
        &f,
        &s,
        "GET",
        "/api/v1/paea/reconcile-effects",
        Some("Bearer fixture-admin-token"),
    );
    let status = reply.response.status();
    let observed = reply.failure.as_ref().map(|e| e.code().to_owned());
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "known reader must reach the separation fault on the invalid-ledger shape; \
         observed {status} with code {observed:?}"
    );
    assert_eq!(
        observed.as_deref(),
        Some("paea-effect-reconcile-separate-ledger-required"),
        "known reader observed {observed:?} instead of the separation code"
    );
    assert_unchanged(&baseline);
    // Transition 1 — SUPPORTED disable of the fixture admin reader identity.
    reconfigured(true, "reconcile-reader-disable");
    let disabled_baseline = snapshot();
    // WITNESS 2 — the SAME token is now refused by the identity precheck
    // BEFORE any ledger IO: the ledger stays invalid-by-construction, so the
    // ONLY acceptable code is the identity refusal (a separation code here
    // would prove ledger IO preceded the identity decision).
    let reply = request(
        &f,
        &s,
        "GET",
        "/api/v1/paea/reconcile-effects",
        Some("Bearer fixture-admin-token"),
    );
    let status = reply.response.status();
    let observed = reply.failure.as_ref().map(|e| e.code().to_owned());
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "disabled reader must receive the identity refusal before any ledger IO; \
         observed {status} with code {observed:?}"
    );
    assert_eq!(
        observed.as_deref(),
        Some("authorization_required"),
        "disabled reader observed {observed:?} instead of the identity refusal — \
         the precheck did not honor the CURRENT disabled state"
    );
    let raw = String::from_utf8(reply.response.body().to_vec()).unwrap();
    assert!(
        !raw.contains("settlement") && !raw.contains("findings"),
        "GET identity refusal disclosed ledger/report fields: {raw}"
    );
    assert_unchanged(&disabled_baseline);
    let head = request(
        &f,
        &s,
        "HEAD",
        "/api/v1/paea/reconcile-effects",
        Some("Bearer fixture-admin-token"),
    );
    let head_status = head.response.status();
    let head_observed = head.failure.as_ref().map(|e| e.code().to_owned());
    assert_eq!(
        head_status,
        StatusCode::FORBIDDEN,
        "HEAD disabled reader must mirror the identity refusal; \
         observed {head_status} with code {head_observed:?}"
    );
    assert_eq!(
        head_observed.as_deref(),
        Some("authorization_required"),
        "HEAD observed {head_observed:?} instead of the identity refusal"
    );
    assert!(
        head.response.body().is_empty(),
        "HEAD refusal must keep the empty body"
    );
    assert_unchanged(&disabled_baseline);
    // Transition 2 — SUPPORTED re-enable of the same identity.
    reconfigured(false, "reconcile-reader-enable");
    let enabled_baseline = snapshot();
    // WITNESS 3 — the known fault path returns verbatim: the precheck
    // re-read the CURRENT trust state (no identity cache).
    let reply = request(
        &f,
        &s,
        "GET",
        "/api/v1/paea/reconcile-effects",
        Some("Bearer fixture-admin-token"),
    );
    let status = reply.response.status();
    let observed = reply.failure.as_ref().map(|e| e.code().to_owned());
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "re-enabled reader must return to the known separation path; \
         observed {status} with code {observed:?}"
    );
    assert_eq!(
        observed.as_deref(),
        Some("paea-effect-reconcile-separate-ledger-required"),
        "re-enabled reader observed {observed:?} instead of the separation code"
    );
    assert_unchanged(&enabled_baseline);
    s.close().unwrap();
}
/// Without a session cookie the admission gate refuses before the route; with
/// a session but a body or query the route refuses without any reader IO.
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

/// Reader-frontier contract witness on a REAL non-empty report (BLP-RET-05 /
/// PA-09). The settlement effect is produced by the REAL two-phase producers
/// (fresh owned roots; autonomous settlement harness, no retained fixture
/// needed), so the report carries the exact durable effect identifiers. The
/// BLP policy and the report attribution are produced through the supported
/// control operations (`blp-policy-apply`, `blp-object-attribute`) bound to
/// the digest the route recomputes — the SAME encoding+hash domain as the
/// served bytes, verified here against `encode_model` (the test digests the
/// encoded model independently and the authorized read must succeed on that
/// binding, never on a browser hash or a clearance assumption).
///
/// Asserted sequence:
/// - every Trust/BLP producer of this sequence targets the REAL service root
///   (`f.root`) through `trust_on_service`: the explicit identity
///   configuration (an authenticated `blp-show` administrator is required
///   before any control read), the BLP policy, every attribution and the
///   staleness configuration change — never the settlement project root;
/// - WITHOUT attribution: even the maximally-cleared reader is refused with
///   the exact `paea-reader-attribution-required` typed 403 code (GET in the
///   failure surface; HEAD empty body), disclosing no finding, identifier or
///   journal state;
/// - the supported producer attributes the report digest at the CURRENT
///   policy revision for the real service root; the authorized reader then
///   receives the EXACT report (schema, both domains, effectCount=1, the
///   real finding verified with its intent_id/action_digest) and HEAD
///   mirrors it as an empty 200;
/// - an UNKNOWN credential refuses with the typed authorization refusal
///   (non-success), an INSUFFICIENT reader refuses
///   `paea-reader-access-denied` — both with no report byte;
/// - a Trust configuration change moves the recorded policy out of currency:
///   the same authorized reader is then refused `paea-reader-policy-stale`;
/// - recovery through public operations (re-apply the policy at the new
///   revision, re-attribute) restores the authorized read — the refusals
///   were attribution-state-driven and the channel intact;
/// - across every refused read BOTH journals stay byte-identical (no
///   mutation, no replay, no partial report) and the failed `Refused` cause
///   remains typed for the transport without leaking any report field.
#[test]
fn reconcile_effects_reader_frontier_contract_with_real_report() {
    // Fresh project root with a real work + fixture identity configuration
    // (the settlement policy read requires a configured current Trust state,
    // same precondition as the engine reconciliation harness).
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
    // BLP policy over the REAL identities of this fresh project root, applied
    // through the supported producer. The policy is re-read at every verdict;
    // the entry rides the current Trust revision (just configured).
    // The reader guard reads the CURRENT BLP record on the SERVICE root
    // (self.root — the root whose journals the report reads); the policy, the
    // real fixture identities and every attribution are produced THERE through
    // the supported control operations, not on the settlement project root.
    // The helper consumes its opened `RootStore` exactly once per path:
    // `unwrap_or_else` moved `service` into the failure closure while the
    // success path closed it again (E0382). The `match` keeps the trust cause
    // AND the checked close paired on every path — a double failure panics
    // with both codes, never an unchecked close.
    let trust_on_service = |op: &str, value: Value, token: &str| -> Value {
        let service = RootStore::open(&f.root, true).unwrap();
        match service.trust(op, &serde_json::to_vec(&value).unwrap(), token) {
            Ok(reply) => {
                service.close().unwrap();
                serde_json::from_slice(&reply.bytes).unwrap()
            }
            Err(e) => {
                let code = e.code();
                match service.close() {
                    Ok(()) => panic!("{op} refused: {}", code),
                    Err(close_error) => panic!(
                        "{op} refused: {} (close also failed: {})",
                        code,
                        close_error.code()
                    ),
                }
            }
        }
    };
    let trust = trust_on_service;
    // The service root starts with NO Trust state: `blp-show` and every
    // control operation require an authenticated administrator, so the
    // explicit identity configuration boots this root first (event_id
    // `reconcile-reader-config`, `expected_revision:0`, local operator on a
    // fresh root). This makes the fixture's identities on f.root REAL for the
    // reader verdicts — configuration on the settlement project root never
    // reaches the reader guard.
    trust(
        "config-apply",
        identity_config(),
        "",
    );
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
            "event_id":"reconcile-reader-policy",
            "expected_revision":blp_revision,
            "expected_trust_revision":1,
            "reason":"Reader-frontier policy for the PA-09 reconcile report fixture",
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
    // Journal byte-witnesses before any guarded read: the reconciliation is a
    // read-only consumer of both roots.
    let service_db = f.root.join(".swarm/state.db");
    let ledger_db = f.ledger.join(".swarm/state.db");
    let before_service = fs::read(&service_db).unwrap();
    let before_ledger = fs::read(&ledger_db).unwrap();
    // Gate 2 without attribution: the maximally-cleared reader (I3 admin,
    // classified clearance under the applied policy) is refused because no
    // attribution binds the report digest yet — clearance alone invents no
    // provenance. GET: explicit projection with the exact code; HEAD: typed
    // 403 with an empty body.
    {
        let reply = request(&f, &s, "GET", RECONCILE_REPORT, Some("Bearer fixture-admin-token"));
        assert_eq!(reply.response.status(), StatusCode::FORBIDDEN, "{:?}", reply.failure);
        assert_eq!(reply.failure.as_ref().expect("typed refusal").code(), "paea-reader-attribution-required");
        let raw = String::from_utf8(reply.response.body().to_vec()).unwrap();
        assert!(!raw.contains("settlement") && !raw.contains("findings"),
            "GET refusal disclosed report fields");
        let head = request(&f, &s, "HEAD", RECONCILE_REPORT, Some("Bearer fixture-admin-token"));
        assert_eq!(head.response.status(), StatusCode::FORBIDDEN);
        assert!(head.response.body().is_empty());
        assert!(head.failure.is_some());
    }
    // Compute the EXACT attribution input the route uses: the digest of the
    // report's canonical encoding. The test recomputes it from the engine's
    // own encoding domain (encode_model → the same canonical bytes the route
    // serves) so the digest producer is verified against the transmitted
    // bytes, never assumed from a docstring.
    let report = {
        let service = RootStore::open(&f.root, false).unwrap();
        let ledger = RootStore::open(&f.ledger, false).unwrap();
        let report = service.paea_reconcile_effect_journals(&ledger).unwrap();
        ledger.close().unwrap();
        service.close().unwrap();
        report
    };
    let PaeaEffectReconcileJournal::Consistent { effect_count: 1, findings } = &report.settlement
    else {
        panic!("expected a non-empty settlement journal, got {:?}", report.settlement);
    };
    let real_intent = findings[0].intent_id.clone();
    let real_digest = findings[0].action_digest.clone();
    assert!(findings[0].verified, "unexpected refusals: {:?}", findings[0].refusals);
    let encoded = swarm_engine::encode_model(&report).unwrap();
    let report_digest = {
        use sha2::{Digest as _, Sha256};
        format!("{:x}", Sha256::digest(&encoded))
    };
    // The attribution binds the recomputed digest, the ACTUAL service root
    // (the root whose Git journals the report reads) and the CURRENT policy
    // revision, through the supported control operation.
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
            "event_id":"reconcile-reader-attribute",
            "expected_revision":attribute_revision,
            "expected_trust_revision":1,
            "canonical_digest":report_digest,
            "project_path":f.root.to_str().unwrap(),
            "classification":{"level":"classified","compartments":[]},
            "reason":"Reader-frontier attribution for the exact reconcile report",
        }),
        "fixture-admin-token",
    );
    assert_eq!(attribute_reply["attributed"], true, "supported producer must attribute");
    // POSITIVE witness: the authorized reader receives the EXACT served bytes.
    {
        let reply = request(&f, &s, "GET", RECONCILE_REPORT, Some("Bearer fixture-admin-token"));
        assert!(reply.response.status().is_success(), "{:?}", reply.failure);
        assert_eq!(reply.response.body(), &encoded, "served bytes must equal the attributed encoding exactly");
        let body: Value = serde_json::from_slice(reply.response.body()).unwrap();
        assert_eq!(body["schema_version"], 1);
        let settlement = &body["settlement"];
        assert_eq!(settlement["state"], "consistent");
        assert_eq!(settlement["effectCount"], 1);
        assert_eq!(settlement["findings"][0]["intent_id"], real_intent);
        assert_eq!(settlement["findings"][0]["action_digest"], real_digest);
        assert_eq!(settlement["findings"][0]["verified"], true);
        // HEAD mirrors the allowed read: empty 200.
        let head = request(&f, &s, "HEAD", RECONCILE_REPORT, Some("Bearer fixture-admin-token"));
        assert_eq!(head.response.status(), StatusCode::OK);
        assert!(head.response.body().is_empty());
    }
    // UNKNOWN credential: typed authorization refusal, non-success, no report.
    {
        let reply = request(&f, &s, "GET", RECONCILE_REPORT, Some("Bearer unknown-reader-token"));
        assert!(!reply.response.status().is_success(), "{:?}", reply.failure);
        let raw = String::from_utf8(reply.response.body().to_vec()).unwrap();
        assert!(!raw.contains(&real_intent) && !raw.contains(&real_digest),
            "unknown credential saw a report identifier");
    }
    // INSUFFICIENT reader: enrolled but not cleared for the attributed
    // classification; refusal with the exact code, no report byte.
    {
        let reply = request(&f, &s, "GET", RECONCILE_REPORT, Some("Bearer fixture-requester-token"));
        assert_eq!(reply.response.status(), StatusCode::FORBIDDEN);
        assert_eq!(reply.failure.as_ref().expect("typed refusal").code(), "paea-reader-access-denied");
        let raw = String::from_utf8(reply.response.body().to_vec()).unwrap();
        assert!(!raw.contains(&real_intent) && !raw.contains(&real_digest),
            "insufficient reader saw a report identifier");
    }
    // STALE policy: a real Trust configuration change moves the recorded
    // policy out of currency; the same authorized reader is then refused
    // with the stable stale code and no report byte. The change targets the
    // SERVICE root (the root whose Trust state the reader verdict reads) —
    // mutating the settlement project root's configuration could not move
    // the served verdict. `expected_revision:1` is the f.root configuration
    // revision after the explicit bootstrap above.
    trust(
        "config-apply",
        json!({
            "schema_version":1,
            "event_id":"reconcile-reader-stale-config",
            "expected_revision":1,
            "reason":"Reader-frontier staleness witness for the reconcile report",
            "policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":false},
            "identities":[
                {"subject":"enrolled-effect","integrity":3,"roles":["admin","effect"],"enrollment_principal":{"principal_id":"operator","principal_kind":"human","issuer":"https://fixture.invalid","subject":"stable-person"}},
                {"subject":"requester","token":"fixture-requester-token","integrity":1,"roles":["requester"]},
                {"subject":"approver","token":"fixture-approver-token","integrity":2,"roles":["approver"]},
                {"subject":"agreement-admin","token":"fixture-admin-token","integrity":3,"roles":["admin"]}
            ]
        }),
        "",
    );
    {
        let reply = request(&f, &s, "GET", RECONCILE_REPORT, Some("Bearer fixture-admin-token"));
        assert_eq!(reply.response.status(), StatusCode::FORBIDDEN);
        assert_eq!(reply.failure.as_ref().expect("typed refusal").code(), "paea-reader-policy-stale");
        let raw = String::from_utf8(reply.response.body().to_vec()).unwrap();
        assert!(!raw.contains(&real_intent), "stale refusal disclosed the intent identifier");
    }
    // RECOVERY through public operations: renew the policy at the new Trust
    // revision and re-attribute the SAME report digest; the authorized read
    // is restored — the refusals were attribution-state-driven.
    let renewed = trust("blp-show", json!({}), "fixture-admin-token")["record"]["history"]
        .as_array()
        .and_then(|h| h.last())
        .and_then(|e| e["revision"].as_u64())
        .unwrap_or(0);
    trust(
        "blp-policy-apply",
        json!({
            "schema_version":1,
            "event_id":"reconcile-reader-policy-renewed",
            "expected_revision":renewed,
            "expected_trust_revision":2,
            "reason":"Reader-frontier policy renewal after the staleness witness",
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
    let attribute_revision2 = trust("blp-show", json!({}), "fixture-admin-token")["record"]
        ["history"]
        .as_array()
        .and_then(|h| h.last())
        .and_then(|e| e["revision"].as_u64())
        .unwrap_or(0);
    trust(
        "blp-object-attribute",
        json!({
            "schema_version":1,
            "event_id":"reconcile-reader-attribute-renewed",
            "expected_revision":attribute_revision2,
            "expected_trust_revision":2,
            "canonical_digest":report_digest,
            "project_path":f.root.to_str().unwrap(),
            "classification":{"level":"classified","compartments":[]},
            "reason":"Reader-frontier attribution renewal for the same report bytes",
        }),
        "fixture-admin-token",
    );
    {
        let reply = request(&f, &s, "GET", RECONCILE_REPORT, Some("Bearer fixture-admin-token"));
        assert!(reply.response.status().is_success(), "recovery read refused: {:?}", reply.failure);
        assert_eq!(reply.response.body(), &encoded, "recovered read serves the same exact bytes");
    }
    // No journal mutation anywhere in the sequence; no partial report ever.
    assert_eq!(fs::read(&service_db).unwrap(), before_service);
    assert_eq!(fs::read(&ledger_db).unwrap(), before_ledger);
    s.close().unwrap();
    pstore.close().unwrap();
}

// --- PA-09 native positive reader witness: REAL qualified binary, empty ledger ---

use std::{
    io::{self as stdio_io, BufRead as _, BufReader, Read as _, Write as _},
    net::TcpStream,
    os::unix::fs::{DirBuilderExt as _, OpenOptionsExt as _},
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    sync::{mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
};

/// Byte-witness of both roots, taken after an intentional setup mutation and
/// re-compared after EVERY read phase (the pre-attribution refusals included,
/// so a mutation can never be masked by a later baseline): the two state
/// databases must never move across the whole READ sequence, and each trust
/// journal only across an explicit Trust write of the setup itself.
/// Inputs: the two real fixture roots. Output: (service state.db, ledger
/// state.db, service trust-actions.json, ledger trust-actions.json) bytes;
/// ONLY `std::io::ErrorKind::NotFound` reads an absent trust journal as empty (its
/// later appearance would then be a detected mutation) — any other error
/// (PermissionDenied and the like) is a hard failure, never masked.
/// Errors: a missing state.db is a hard failure (both roots are initialized
/// before any snapshot); a non-NotFound trust-journal read error propagates
/// with its cause. Effects: read-only.
fn journal_snapshot(
    service_root: &Path,
    ledger_root: &Path,
) -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
    let trust_journal = |root: &Path, name: &str| -> Vec<u8> {
        match fs::read(root.join(".swarm").join(name)) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => panic!("trust journal {name} of {} must be readable (only absence is authorized, got: {e})", root.display()),
        }
    };
    (
        fs::read(service_root.join(".swarm/state.db")).unwrap(),
        fs::read(ledger_root.join(".swarm/state.db")).unwrap(),
        trust_journal(service_root, "trust-actions.json"),
        trust_journal(ledger_root, "trust-actions.json"),
    )
}

/// Shared native-witness prologue (empty and non-empty siblings): reads the
/// REQUIRED `SWARM_ENGINE_BINARY` environment variable, refuses anything that
/// is not a real file, and digests the provided ELF once. Inputs: none (the
/// environment only). Output: (binary path, lowercase sha256 hex of the ELF
/// bytes). Errors: an unset or non-UTF-8 variable, or a path that is not an
/// existing file, is a hard failure — the native witness never fakes a PASS
/// without the real qualified binary (no hash or path is hardcoded in the
/// repository). Effects: read-only.
fn native_binary_from_env() -> (PathBuf, String) {
    let binary = PathBuf::from(
        std::env::var("SWARM_ENGINE_BINARY")
            .expect("SWARM_ENGINE_BINARY = qualified swarm-rust binary (settings v52); the test refuses to fake a PASS without it"),
    );
    assert!(binary.is_file(), "SWARM_ENGINE_BINARY must point to the real ELF: {}", binary.display());
    let binary_sha = {
        use sha2::{Digest as _, Sha256};
        format!("{:x}", Sha256::digest(fs::read(&binary).unwrap()))
    };
    (binary, binary_sha)
}

/// One fresh private fixture root under the repository fixture area (never
/// /tmp): `<area>/<name>-<pid>-<nanos>`, created 0700 FROM BIRTH. Inputs: the
/// canonicalized fixture area, a caller-chosen name, the run's nanos token.
/// Output: the created directory path. Errors: an exclusive-creation failure
/// (existing path, concurrent claim) propagates from `create_dir_mode_0700`,
/// never masked. Effects: one new 0700 directory at the exact path; no parent
/// is created or touched beyond the caller's own area.
fn fresh_fixture_root(area: &std::path::Path, name: &str, nanos: u128) -> PathBuf {
    let dir = area.join(format!("{name}-{}-{nanos}", std::process::id()));
    create_dir_mode_0700(&dir).unwrap();
    dir
}

/// One supported Trust operation on the given root through the public
/// `RootStore::trust`, with the paired-close discipline on EVERY path: the
/// opened store is closed exactly once whether the operation succeeds or is
/// refused, and a double failure carries BOTH codes (a close failure is never
/// masked by the operation's refusal). Shared by the native empty and
/// non-empty tests for the service-root bootstrap (`config-apply`,
/// `blp-show`, `blp-policy-apply`) and every attribution. Inputs: real
/// fixture root, operation name, JSON value, bearer token ("" = local
/// operator bootstrap on a fresh root). Output: the parsed JSON reply.
/// Errors: a refused operation panics with the typed code (after the checked
/// close); serialization of the value or deserialization of the reply panics
/// with its cause. Effects: the intentional Trust write/read named by `op`
/// on that root only.
fn trust_on_root(root: &std::path::Path, op: &str, value: Value, token: &str) -> Value {
    let service = RootStore::open(root, true).unwrap();
    match service.trust(op, &serde_json::to_vec(&value).unwrap(), token) {
        Ok(reply) => {
            service.close().unwrap();
            serde_json::from_slice(&reply.bytes).unwrap()
        }
        Err(e) => {
            let code = e.code();
            match service.close() {
                Ok(()) => panic!("{op} refused: {}", code),
                Err(close_error) => panic!(
                    "{op} refused: {} (close also failed: {})",
                    code,
                    close_error.code()
                ),
            }
        }
    }
}

/// The exact BLP reader policy of the native reader-frontier siblings (empty
/// and non-empty): three levels, no compartment, the four fixture identities
/// cleared (`enrolled-effect` classified, `requester` restricted, `approver`
/// public, `agreement-admin` classified). Inputs: none (pure value).
/// Output: the policy request value for `blp-policy-apply`; the caller owns
/// `event_id`, `expected_revision`, `expected_trust_revision` and `reason`
/// (mode-specific). Errors: none. Effects: none.
fn native_reader_blp_policy() -> Value {
    json!({
        "schema_version":1,
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
    })
}

/// The bind-created web-session secret really generated by the native
/// listener at the announced address (driver-identical derivation, Rust
/// precedent `service.rs`): sha256(host) first 16 hex names the file. The
/// token MUST already exist — the announced bind event is its producer —
/// and is trimmed exactly like the received driver (`paea_effects_ui.cjs`)
/// before being presented as the cookie value. The shape diagnostic never
/// carries the token bytes (secret, never logged). Inputs: real service root,
/// bound `127.0.0.1:<port>` string. Output: the secret (`session-…`).
/// Errors: a missing or unreadable credential file is a hard failure (no
/// retry, no synthesis); a wrong shape is a hard failure whose message
/// contains no secret byte. Effects: read-only.
fn web_session_secret(root: &Path, address: &str) -> String {
    use sha2::{Digest as _, Sha256};
    let hash = format!("{:x}", Sha256::digest(address.as_bytes()));
    let path = root.join(".swarm").join(format!("web-session-{}", &hash[..16]));
    let token = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("bind must have created {}: {}", path.display(), e))
        .trim()
        .to_owned();
    assert!(
        token.starts_with("session-"),
        "native bind secret must be a session token (length kept private: {} bytes)",
        token.len()
    );
    token
}

/// How `Kill` reaches ITS OWN child with SIGINT. `Pidfd` is the kernel-owned
/// identity of the exact spawned process (immune to later pid reuse).
/// `Numeric` is only built when `pidfd_open` itself failed on the
/// demonstrably-alive, owned child: an owned, not-yet-reaped child is alive
/// or zombie, and the kernel cannot reuse its pid before it is reaped, so
/// the numeric signal still cannot reach a foreign process. Only SIGINT is
/// ever sent through either variant (graceful native shutdown, never
/// SIGKILL, no watchdog).
enum SignalChannel {
    Pidfd(std::os::unix::io::OwnedFd),
    Numeric(rustix::process::Pid),
}

/// Ownership of the spawned native listener: the signal channel above is
/// established on the REAL child while it is demonstrably alive, so SIGINT
/// addresses THE spawned process, never a foreign one. The child is
/// consumed (reaped) at most once; after that, neither `sigint_and_wait`
/// nor Drop sends any signal. Ownership is kept until the exit is OBSERVED
/// — a refused signal never loses or abandons the child: the reap happens
/// in every case and the refusal is reported only afterwards, cause
/// preserved. Whatever path reaps the child (caller, Drop during
/// unwinding) records the OBSERVED `ExitStatus` in the shared slot so the
/// failure-path witness can conserve the REAL exit — the drop path never
/// panics (no panic-in-drop abort that would hide the original failure);
/// kill/wait statuses are propagated, printed or recorded, never ignored.
/// Failure output stays in the caller's captured buffers.
struct Kill {
    child: Option<Child>,
    channel: SignalChannel,
    /// Shared observed-exit slot: `None` until the child is reaped, then
    /// `Some(exit)` — written by `sigint_and_wait` and by the drop path
    /// alike, read by the failure witness during unwinding (when the
    /// listener has already been dropped, so the value is final).
    observed_exit: Arc<Mutex<Option<ExitStatus>>>,
}
impl Kill {
    /// Fresh shared exit slot for a listener about to be spawned.
    fn observed_exit_slot() -> Arc<Mutex<Option<ExitStatus>>> {
        Arc::new(Mutex::new(None))
    }

    /// Sends SIGINT to the OWN child through the configured channel
    /// (graceful native shutdown; never SIGKILL) and reaps it, returning
    /// the real observed exit status. The child is consumed: a second call
    /// panics instead of re-signalling a reaped pid. Errors: a signal
    /// refusal (dead or unreapable child) is reported only AFTER the reap —
    /// the panic then carries both the refusal and the observed exit; a
    /// missing child means a caller-contract violation (panic). A wait
    /// failure (own child not reapable) panics with its cause; the child is
    /// then dropped unreaped (zombie until the test process exits), which
    /// is stated instead of hidden, and the slot stays `None` (a `None`
    /// slot at witness time therefore means exactly "no observed exit",
    /// never a faked success). Effects: the shared slot records the
    /// observed exit; none beyond the own child's shutdown.
    fn sigint_and_wait(&mut self) -> ExitStatus {
        let mut child = self
            .child
            .take()
            .expect("sigint_and_wait must be called at most once per child");
        // Ownership is kept across the signal attempt: on refusal the child
        // below is still reaped, and only then is the refusal reported.
        let signal_error = self.send_sigint().err();
        let exit = child
            .wait()
            .unwrap_or_else(|e| panic!("native listener must be reaped: {e}"));
        if let Some(e) = signal_error {
            panic!("SIGINT must reach the own child: {e}; child was still reaped with {exit}");
        }
        *self
            .observed_exit
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(exit);
        exit
    }

    /// Sends SIGINT to the OWN child through the configured channel. Errors:
    /// a refused signal is returned to the caller (never ignored, never
    /// SIGKILL), which keeps ownership and reaps the child anyway. The
    /// return type is the real one shared by BOTH rustix arms
    /// (`pidfd_send_signal`, `kill_process`): `rustix::io::Result<()>`,
    /// i.e. `Result<(), rustix::io::Errno>` — not `std::io::Result`.
    fn send_sigint(&self) -> rustix::io::Result<()> {
        match &self.channel {
            SignalChannel::Pidfd(pidfd) => {
                rustix::process::pidfd_send_signal(pidfd, rustix::process::Signal::INT)
            }
            SignalChannel::Numeric(pid) => {
                rustix::process::kill_process(*pid, rustix::process::Signal::INT)
            }
        }
    }

    /// Sends SIGINT through the configured channel and reaps the child,
    /// returning the observed exit status when the caller needs it (same
    /// contract as `sigint_and_wait`, for the drop path). This runs during
    /// unwinding too, so it NEVER panics: a signal refusal or a wait
    /// failure is printed with its cause (the child may then stay a zombie
    /// until the test process exits — stated, never hidden), and the reap
    /// stays authoritative in every case (the child may legitimately have
    /// exited on its own before the signal). The observed exit — when the
    /// reap succeeded — is recorded in the shared slot before being
    /// returned.
    fn drop_signal_and_wait(&mut self) -> Option<ExitStatus> {
        let mut child = self.child.take()?;
        if let Err(e) = self.send_sigint() {
            eprintln!("SIGINT on own listener failed (reap stays authoritative): {e}");
        }
        match child.wait() {
            Ok(exit) => {
                *self
                    .observed_exit
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(exit);
                Some(exit)
            }
            Err(e) => {
                eprintln!("native listener could not be reaped on drop: {e}");
                None
            }
        }
    }
}
impl Drop for Kill {
    /// SIGINT through the configured channel then blocking reap — only
    /// while the child is not yet consumed; after `sigint_and_wait` no
    /// signal is ever sent and Drop is a no-op. Drop never panics: the
    /// observed exit is printed on the failure path and RECORDED in the
    /// shared slot so the failure witness conserves the REAL exit without
    /// risking a panic-in-drop abort.
    fn drop(&mut self) {
        if let Some(exit) = self.drop_signal_and_wait() {
            if !exit.success() {
                eprintln!("native listener exited abnormally on cleanup drop: {exit}");
            }
        }
    }
}

/// Raw-TCP request to the REAL native listener (driver-identical framing:
/// `Connection: close`, read to EOF — hyper owns the framing). Inputs: bound
/// port, method, path, optional Bearer credential. Output: (status, body
/// bytes). Errors: connect/write/read failures propagate. Effects: none on
/// the roots; this is the whole point of the READ witness.
fn raw_http(
    port: u16,
    method: &str,
    path: &str,
    cookie: Option<&str>,
    authorization: Option<&str>,
) -> (u16, Vec<u8>) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    let mut req = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n"
    );
    if let Some(cookie) = cookie {
        req.push_str(&format!("Cookie: {cookie}\r\n"));
    }
    if let Some(token) = authorization {
        req.push_str(&format!("Authorization: Bearer {token}\r\n"));
    }
    req.push_str("\r\n");
    stream.write_all(req.as_bytes()).unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("HTTP response must carry a header/body separator");
    let head = String::from_utf8_lossy(&raw[..split]).into_owned();
    let status: u16 = head
        .lines()
        .next()
        .unwrap()
        .split(' ')
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    (status, raw[split + 4..].to_vec())
}

/// Event-driven readiness witness of the real listener (driver-identical
/// contract, no polling/sleep/timeout): the FIRST stdout JSON line of the
/// `web 127.0.0.1:0 --json` process must announce the bound address. The
/// SAME reader thread then keeps draining stdout to EOF into a retained
/// shared buffer — nothing after the bind line is discarded (the final
/// `web-stopped` stop-record and the whole stream stay inspectable), and the
/// child's stdout pipe can never fill and block the listener. The buffer is
/// the EXACT and COMPLETE child stdout: it is never searched or truncated by
/// content — a stdout read error is recorded ONLY in the separate error
/// state, never appended to the raw bytes. The reader thread ends at child
/// exit (EOF), so the caller joins it AFTER stopping the child; a later
/// unexpected listener exit cannot hang the test — the process is stopped
/// via `Kill::sigint_and_wait` and its real exit is asserted by the caller.
/// Inputs: the spawned child and its shared drained stderr (included
/// verbatim in every failure, without blocking on the stderr drain). Output:
/// (bound `127.0.0.1:<port>`, reader handle to join after the run, retained
/// FULL stdout bytes, stdout drain-error state — empty when the read to EOF
/// succeeded). Errors: listener exit before the announcement or a non-JSON
/// first line are hard failures. Effects: none.
fn bind_announcement(
    child: &mut Child,
    stderr: &Arc<Mutex<Vec<u8>>>,
) -> (
    String,
    JoinHandle<()>,
    Arc<Mutex<Vec<u8>>>,
    Arc<Mutex<Vec<u8>>>,
) {
    // Snapshot without waiting: `try_lock` never blocks on the drain thread,
    // so readiness diagnosis stays possible while the child is still alive;
    // a poisoned lock still yields the already-drained bytes.
    let cause = || {
        match stderr.try_lock() {
            Ok(buf) => String::from_utf8_lossy(&buf).into_owned(),
            Err(std::sync::TryLockError::Poisoned(poisoned)) => {
                String::from_utf8_lossy(&poisoned.into_inner()).into_owned()
            }
            Err(std::sync::TryLockError::WouldBlock) => {
                "<stderr drain in progress>".into()
            }
        }
    };
    let stdout = child.stdout.take().expect("listener stdout must be piped");
    let (sender, receiver) = mpsc::channel();
    let retained = Arc::new(Mutex::new(Vec::<u8>::new()));
    let stdout_errors = Arc::new(Mutex::new(Vec::<u8>::new()));
    let sink = Arc::clone(&retained);
    let errors = Arc::clone(&stdout_errors);
    let reader = thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => {
                let _ = sender.send(Err("native listener exited before announcement".into()));
            }
            Ok(_) => match serde_json::from_str::<Value>(&line) {
                Ok(event) => {
                    let _ = sender.send(Ok(event));
                }
                Err(e) => {
                    let _ = sender.send(Err(format!("first stdout line is not JSON: {e}")));
                }
            },
            Err(e) => {
                let _ = sender.send(Err(format!("stdout read failed: {e}")));
            }
        }
        // Retain EVERYTHING after the first line: continue reading to EOF
        // (child exit closes the pipe) into the shared retained buffer, so
        // the `web-stopped` stop-record survives for the caller's contract
        // assertion. The raw buffer holds EXACT child bytes — a read error
        // is recorded ONLY in the separate error state (child-controlled
        // content, including text resembling a drain annotation, is never
        // interpreted or truncated). The first-line bytes are appended under
        // the same lock, keeping the whole stdout ordering exact.
        {
            let mut sink = sink
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            sink.extend_from_slice(line.as_bytes());
        }
        drain_to_end(reader, &sink, &errors, "stdout");
    });
    let event = receiver
        .recv()
        .unwrap_or_else(|_| panic!("readiness thread died before announcing"));
    let event = match event {
        Ok(event) => event,
        Err(reason) => panic!("native listener readiness failed: {reason}; stderr: {}", cause()),
    };
    let address = event["address"]
        .as_str()
        .expect("bind event must carry an address string")
        .to_owned();
    assert!(
        address.starts_with("127.0.0.1:"),
        "bind event must announce a 127.0.0.1 address, got: {address}"
    );
    (address, reader, retained, stdout_errors)
}

/// Child-pipe reader loop shared by the REAL stderr drain and the memory
/// guards (the guard exercises the SAME producer code): appends every
/// successfully-read chunk EXACTLY (in order, no content interpretation —
/// child-controlled text that resembles a drain annotation is never
/// searched for or acted upon) to the shared raw buffer, retries
/// `Interrupted` reads, and on any other read failure records the cause
/// ONLY in the separate error state (replacing any earlier one; raw bytes
/// stay untouched) before ending. EOF closes the loop. A poisoned lock
/// still yields the already-drained bytes. Inputs: the blocking reader to
/// exhaust, its shared raw sink, its shared error state, the stream label
/// (diagnostics only). Output: `()` when EOF or a recorded error ended the
/// loop. Errors: none (read failures are recorded in the error state, never
/// panic). Effects: appends to the two shared states only.
fn drain_to_end<R: std::io::Read>(
    mut reader: R,
    sink: &Arc<Mutex<Vec<u8>>>,
    errors: &Arc<Mutex<Vec<u8>>>,
    label: &'static str,
) {
    use std::io::ErrorKind;
    let mut chunk = [0u8; 4096];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                // A poisoned lock still yields the already-drained bytes.
                let mut sink = sink
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                sink.extend_from_slice(&chunk[..n]);
            }
            // Interrupted reads retry; anything else is recorded in the
            // SEPARATE error state (real errors preserved, raw bytes
            // untouched) and ends the drain.
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => {
                let mut errors = errors
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                *errors = format!("{label} drain error: {e}").into_bytes();
                break;
            }
        }
    }
}

/// Drains the listener's stderr incrementally, WITHOUT holding the lock
/// across the blocking read: each chunk is read into a local buffer and only
/// the append takes the mutex, so a caller diagnosing a still-alive child
/// (`try_lock`) is never blocked until EOF. The shared buffer holds the
/// EXACT and COMPLETE child stderr — it is never searched or appended to by
/// content: a stderr read error is recorded ONLY in the separate error
/// state, never interleaved into the raw bytes (child-controlled content is
/// never interpreted). EOF closes at child exit. Inputs: the spawned child.
/// Output: (shared stderr buffer, separate stderr drain-error state — empty
/// when the read to EOF succeeded, drain thread handle to join AFTER the
/// child is stopped). Errors: none (drain failures are recorded in the
/// error state, never panic). Effects: one own thread.
fn drain_stderr(
    child: &mut Child,
) -> (
    Arc<Mutex<Vec<u8>>>,
    Arc<Mutex<Vec<u8>>>,
    JoinHandle<()>,
) {
    let stderr = child.stderr.take().expect("listener stderr must be piped");
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let errors = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&buffer);
    let error_state = Arc::clone(&errors);
    let handle = thread::spawn(move || {
        drain_to_end(stderr, &sink, &error_state, "stderr");
    });
    (buffer, errors, handle)
}

/// Durable raw-output retention for the native witness, driven ONLY by the
/// operator-provided REQUIRED environment variable
/// `SWARM_PAEA_NATIVE_EVIDENCE_DIR` (Codex supplies a fresh path inside the
/// proof folder). There is NO disabled mode: without the variable the
/// witness refuses to run (no PASS without retained evidence). With the
/// variable, the path must NOT already exist — a reused path could mix or
/// overwrite a previous run's evidence, so reuse is refused and NOTHING is
/// created or written. The reservation EXCLUSIVELY creates the fresh
/// directory born 0700 (`DirBuilderExt::mode(0o700)` on the claimed path
/// only: no `create_dir_all`, so no parent is ever touched and no
/// 0755-then-chmod window ever exists; a concurrently created foreign
/// directory can neither be claimed nor re-permissioned) BEFORE the child
/// is spawned. The directory then receives, each through an exclusive-create
/// private temp file 0600 (no overwrite, no symlink follow), fsync of the
/// file, an EXCLUSIVE `hard_link` publication (fails if the name already
/// exists, never overwrites), a directory fsync (durability) and temp
/// removal: `stdout.log` / `stderr.log` (EXACT raw child bytes, never
/// appended to), `drain-stdout.errors.log` / `drain-stderr.errors.log`
/// (drain failures only, kept separate from the raw streams),
/// `spawn.json` (binary path, sha256, argv — no secret, no cookie) and
/// `exit.json` (the real observed exit status; `observed: false` when the
/// child could not be reaped — never a faked success). A partially written temp
/// file on failure keeps the original published file intact and its cause is
/// preserved — panic on the success path, printed during Drop on the failure
/// path — never a fake PASS. The effective path (its canonicalized parent)
/// must not be `/tmp` or a `/tmp` descendant: the operator interdiction is
/// enforced with no write attempted.
struct RawCapture {
    /// Reserved fresh evidence directory; `from_env` guarantees `Some` (it
    /// refuses to run without the required variable), so every consumer can
    /// expect the dir as a wiring invariant.
    dir: Option<PathBuf>,
}

/// Refuses an evidence path inside `/tmp` (operator interdiction: `/tmp` is
/// a tmpfs wiped at every reboot and must never hold project data or
/// evidence). Inputs: the candidate fresh path. Output: `()` for any path
/// whose canonicalized parent is outside `/tmp` (the last component is
/// reserved for the fresh directory itself and does not exist yet, so the
/// PARENT is the checked anchor). Errors: panics naming the path when the
/// parent is `/tmp` or a `/tmp` descendant, and when the parent cannot be
/// canonicalized (a non-resolving parent would let an unchecked location be
/// written) — both BEFORE any filesystem write. Effects: none (read-only
/// resolution).
fn refuse_tmp_evidence_path(dir: &Path) {
    let parent = dir
        .parent()
        .expect("SWARM_PAEA_NATIVE_EVIDENCE_DIR must not be a filesystem root");
    let parent = parent
        .canonicalize()
        .expect("SWARM_PAEA_NATIVE_EVIDENCE_DIR parent must resolve (existing parent contract)");
    assert!(
        !parent.starts_with("/tmp"),
        "SWARM_PAEA_NATIVE_EVIDENCE_DIR must never be under /tmp (operator \
         interdiction, nothing written): {}",
        dir.display()
    );
}

impl RawCapture {
    /// Reads the REQUIRED `SWARM_PAEA_NATIVE_EVIDENCE_DIR` and EXCLUSIVELY
    /// reserves the fresh evidence directory 0700 in the same step. Output: a
    /// `RawCapture` with the reserved directory. Errors: an UNSET variable is
    /// a hard failure — the native witness must never run without a retained-
    /// evidence contract (no PASS without proof), so absence panics instead of
    /// silently disabling retention; a set-but-existing path is refused
    /// BEFORE any write (evidence-reuse refusal; the cause carries the path);
    /// a non-UTF-8 value panics with its cause; a `/tmp`-based path (checked
    /// through the canonicalized parent) is refused with nothing written
    /// (see `refuse_tmp_evidence_path`). The reservation is a SINGLE exclusive
    /// directory creation WITH mode 0700 from the first instant
    /// (`DirBuilderExt::mode` on the provided path only: no `create_dir_all`,
    /// so no parent is ever touched, no 0755-then-chmod window ever exists,
    /// and a concurrently created foreign directory can neither be claimed
    /// nor re-permissioned). Effects: at most one new 0700 directory inside
    /// the operator-provided parent, before any child is spawned; nothing
    /// else on the filesystem is touched.
    fn from_env() -> Self {
        let path = std::env::var("SWARM_PAEA_NATIVE_EVIDENCE_DIR").unwrap_or_else(|e| {
            panic!(
                "SWARM_PAEA_NATIVE_EVIDENCE_DIR is REQUIRED for the native witness \
                 (no PASS without retained evidence; refusing to run disabled): {e}"
            )
        });
        let dir = PathBuf::from(path);
        refuse_tmp_evidence_path(&dir);
        // Single exclusive claim of the FRESH directory, born 0700: reuse and
        // concurrent foreign claims fail here with no write and no mode
        // change on an existing path (freshness + private-mode contracts
        // kept BEFORE the spawn, without a permissive window).
        create_dir_mode_0700(&dir).unwrap_or_else(|e| {
            panic!(
                "SWARM_PAEA_NATIVE_EVIDENCE_DIR must be a FRESH non-existent path \
                 (evidence reuse or concurrent claim refused, nothing written): \
                 {}: {e}",
                dir.display()
            )
        });
        Self { dir: Some(dir) }
    }

    /// Builds the exact `spawn.json` bytes (binary path, sha256, real argv —
    /// no secret, no cookie). Errors: a serialization failure panics
    /// (impossible for this literal-shaped value, stated instead of silently
    /// dropped). Effects: none.
    fn spawn_bytes(&self, binary: &Path, sha: &str, root: &Path) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "binary": binary.display().to_string(),
            "binary_sha256": sha,
            "argv": ["--root", root.display().to_string(), "web", "127.0.0.1:0", "--json"],
            "note": "exact raw child bytes in stdout.log/stderr.log; drain failures (if \
                     any) ONLY in drain-stdout.errors.log/drain-stderr.errors.log, never \
                     in the raw streams; observed exit (observed:false when the child \
                     could not be reaped) in exit.json"
        }))
        .expect("spawn metadata must serialize")
    }

    /// Writes `name` with EXACTLY `bytes` into the evidence directory
    /// reserved by `from_env`, with exclusive ownership and durability:
    /// (1) a private `.name.partial` temp file is created with
    /// `create_new(true)` mode 0600 — exclusive creation that fails if
    /// anything (file, symlink) already bears the temp name and never
    /// follows or overwrites one; (2) the bytes are written and the FILE is
    /// fsync'ed; (3) the file is published by an EXCLUSIVE `hard_link` from
    /// the temp name to `name` — `AlreadyExists` leaves the previously
    /// published file EXACTLY as it was (no overwrite, no truncate, no
    /// rename over a foreign file or symlink target); (4) the directory is
    /// fsync'ed so the published name survives a crash; (5) the temp entry
    /// is removed and the removal fsync'ed with the directory. A failure
    /// before step (3) keeps the original published file intact and the
    /// partial temp carries the cause. Errors: returned as the cause string;
    /// NEVER panics (used from Drop during unwinding too). Effects: at most
    /// the named published file and its transient private temp inside the
    /// reserved directory; nothing else on the filesystem is touched.
    fn try_store(&self, name: &str, bytes: &[u8]) -> Result<(), String> {
        // The evidence directory is always present (from_env refuses to run
        // without one); an absent dir would be a wiring-contract violation,
        // not a silent no-op.
        let dir = self
            .dir
            .as_ref()
            .expect("try_store requires the evidence directory reserved by from_env");
        let sync_dir = || -> Result<(), String> {
            let d = fs::File::open(dir).map_err(|e| format!("dir {}: {e}", dir.display()))?;
            d.sync_all().map_err(|e| format!("dir {} fsync: {e}", dir.display()))
        };
        let temp = dir.join(format!(".{name}.partial"));
        let target = dir.join(name);
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)
            .map_err(|e| format!("exclusive temp {} : {e}", temp.display()))?;
        if let Err(e) = file
            .write_all(bytes)
            .map_err(|e| format!("{}: write: {e}", temp.display()))
        {
            // The partial temp keeps the failed-write evidence; the published
            // name (if any) is untouched.
            return Err(e);
        }
        if let Err(e) = file
            .sync_all()
            .map_err(|e| format!("{}: fsync: {e}", temp.display()))
        {
            return Err(e);
        }
        if let Err(e) = fs::hard_link(&temp, &target)
            .map_err(|e| format!("exclusive publish {}: {e}", target.display()))
        {
            return Err(e);
        }
        sync_dir()?;
        match fs::remove_file(&temp) {
            Ok(()) => sync_dir(),
            Err(e) => Err(format!("temp cleanup {}: {e}", temp.display())),
        }
    }

    /// Panicking wrapper for the SUCCESS path only: a conservation failure
    /// after the run assertions must stay visible (real cause, no fake PASS;
    /// the witness Drop then reprints the cause without a second panic).
    /// Errors: panics with the exact `try_store` cause. Effects: as
    /// `try_store`.
    fn store(&self, name: &str, bytes: &[u8]) {
        if let Err(cause) = self.try_store(name, bytes) {
            panic!("raw evidence conservation failed — {cause}");
        }
    }
}

/// Failure-path raw-output witness of the native listener, SHARED VERBATIM by
/// the empty and non-empty native siblings (module level so both tests own the
/// same struct; the empty test's contract and recipe stay equivalent).
/// Declared BEFORE the `Kill` listener so Rust drops it AFTER: on the failure
/// path the listener Drop has already stopped and reaped the child (recording
/// the real exit in the shared slot) before this witness's Drop runs — so the
/// witness can join BOTH drain threads to EOF without blocking (the pipes
/// close at child death) and conserve the COMPLETE stdout/stderr raw bytes
/// plus the REAL observed exit, the same artifacts as the success path. It
/// holds a clone of the durable `RawCapture`, clones of the four stream
/// buffers (raw stdout/stderr and the two separate drain-error states), the
/// two drain handles and the shared observed-exit slot. No secret byte is ever
/// written to native stdout/stderr (`web-bound`/`web-outcome`/stop-record
/// only; the session secret lives only in its file), so raw-byte diagnostics
/// are safe. The witness also owns the two drain handles: the normal stop
/// phase takes them back for its own asserted joins; any earlier failure
/// leaves them here for the Drop to join to EOF. If a helper failed BEFORE
/// capture started (e.g. a readiness failure), the witness conserves whatever
/// buffers exist and declares the exact frontier — never claiming a full
/// capture that did not happen. Inputs: none at construction; buffers,
/// handles, slot and capture wired right after spawn/announcement.
/// Errors: none (drop never panics).
#[derive(Default)]
struct RawOutputWitness {
    stdout: Arc<Mutex<Vec<u8>>>,
    stdout_errors: Arc<Mutex<Vec<u8>>>,
    stderr: Arc<Mutex<Vec<u8>>>,
    stderr_errors: Arc<Mutex<Vec<u8>>>,
    /// `true` once the corresponding buffer pair is wired to the real
    /// drains — a `false` frontier (failure before wiring) is stated in
    /// the conserved evidence instead of writing empty files that would
    /// fake a capture that never started.
    wired_stdout: bool,
    wired_stderr: bool,
    reader: Option<JoinHandle<()>>,
    stderr_join: Option<JoinHandle<()>>,
    exit_slot: Option<Arc<Mutex<Option<ExitStatus>>>>,
    capture: Option<RawCapture>,
}
impl RawOutputWitness {
    /// Failure-path conservation of the COMPLETE raw child outputs,
    /// identical in kind to the success path: exact raw stdout/stderr
    /// bytes (never searched, never truncated by content), the separate
    /// drain-error states, and the REAL observed exit from the shared
    /// slot (`"observed": false` when the child could not be reaped —
    /// never a faked success). Frontiers are explicit: a missing capture
    /// (no evidence directory) or a missing buffer (failure before
    /// wiring) is stated and only what exists is conserved.
    /// Errors: none (conservation failures keep their cause via
    /// `eprintln`; this runs during unwinding and must never panic).
    /// Effects: at most six files inside the fresh evidence directory.
    fn conserve(&self) {
        let Some(capture) = &self.capture else {
            eprintln!(
                "raw conservation frontier: no evidence directory wired — nothing retained"
            );
            return;
        };
        let store_raw = |name: &str, buffer: &Arc<Mutex<Vec<u8>>>, wired: bool| {
            if !wired {
                eprintln!(
                    "raw conservation frontier: {name} buffer was never wired (failure \
                     before capture start) — nothing conserved for this stream"
                );
                return;
            }
            let bytes = buffer
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone();
            if let Err(cause) = capture.try_store(name, &bytes) {
                eprintln!("raw {name} conservation failed on drop: {cause}");
            }
        };
        let store_errors = |name: &str, buffer: &Arc<Mutex<Vec<u8>>>, wired: bool| {
            if !wired {
                return;
            }
            let bytes = buffer
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone();
            if let Err(cause) = capture.try_store(name, &bytes) {
                eprintln!("drain-error state {name} conservation failed on drop: {cause}");
            }
        };
        store_raw("stdout.log", &self.stdout, self.wired_stdout);
        store_errors("drain-stdout.errors.log", &self.stdout_errors, self.wired_stdout);
        store_raw("stderr.log", &self.stderr, self.wired_stderr);
        store_errors("drain-stderr.errors.log", &self.stderr_errors, self.wired_stderr);
        let exit_record = match &self.exit_slot {
            Some(slot) => {
                let observed = slot
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .as_ref()
                    .map(|exit| json!({
                        "exit_code": exit.code(),
                        "success": exit.success(),
                        "status": format!("{exit}"),
                    }));
                match observed {
                    Some(exit) => json!({"observed": true, "exit": exit}),
                    None => json!({
                        "observed": false,
                        "note": "child could not be reaped — no exit status faked"
                    }),
                }
            }
            None => json!({
                "observed": false,
                "note": "failure before the exit slot was wired — no exit status faked"
            }),
        };
        match serde_json::to_vec(&exit_record) {
            Ok(bytes) => {
                if let Err(cause) = capture.try_store("exit.json", &bytes) {
                    eprintln!("exit.json conservation failed on drop: {cause}");
                }
            }
            Err(e) => eprintln!("exit record serialization failed on drop: {e}"),
        }
    }
}
impl Drop for RawOutputWitness {
    /// Runs on the FAILURE path only (the normal stop phase takes both
    /// handles back and performs the asserted joins + conservation
    /// there). The listener `Kill` — declared AFTER this witness — has
    /// already dropped and reaped the child before this Drop runs, so
    /// BOTH drain threads can be joined to EOF without blocking (the
    /// pipes close at child death); an unjoinable-after-reap thread
    /// cannot occur, and if the child could not be reaped the exit
    /// record states `observed: false` instead of faking anything. The
    /// joins whose handles were consumed by the normal phase (or never
    /// created before a pre-announcement failure) are skipped. The
    /// complete buffers and the real exit are then conserved durably
    /// (see `conserve`) with the exact frontier stated, and the raw
    /// bytes are printed for the operator. Never panics (no
    /// double-panic during unwinding).
    fn drop(&mut self) {
        if !std::thread::panicking() {
            return;
        }
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        if let Some(stderr_join) = self.stderr_join.take() {
            let _ = stderr_join.join();
        }
        self.conserve();
        let stdout_cause = String::from_utf8_lossy(&self.stdout
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()))
        .into_owned();
        let stderr_cause = String::from_utf8_lossy(&self.stderr
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()))
        .into_owned();
        eprintln!(
            "raw listener output on failure — stdout: {stdout_cause}; stderr: {stderr_cause}"
        );
    }
}

/// Spawn the REAL provided binary as the native web listener of the service
/// root (driver-identical argv; stdout piped for the bind event and fully
/// retained by the reader thread, stderr piped and drained incrementally so
/// the real causes are retained without blocking). The signal channel is
/// established IMMEDIATELY after the spawn, while the child is demonstrably
/// alive: a successful `pidfd_open` gives `Kill` the kernel-owned identity
/// of the exact process it may signal; if `pidfd_open` itself fails, the
/// child is NOT abandoned — the fallback `Numeric` channel keeps the pid of
/// this owned, not-yet-reaped child, which the kernel cannot reuse before
/// it is reaped (alive or zombie), so cleanup stays possible without ever
/// signalling a foreign process. The shared observed-exit slot is allocated
/// here and given to `Kill`, so BOTH the caller's explicit stop and the
/// unwinding Drop path record the real exit in the same slot (the value the
/// failure witness conserves). Inputs: provided binary path, real service
/// root. Output: (owned child wrapper, shared stderr buffer, separate stderr
/// drain-error state, stderr drain handle, shared observed-exit slot).
/// Errors: spawn failure propagates (no child exists yet). Effects: one own
/// process, stopped by `Kill::sigint_and_wait` or `Kill`'s Drop.
fn spawn_web(
    binary: &Path,
    root: &Path,
) -> (
    Kill,
    Arc<Mutex<Vec<u8>>>,
    Arc<Mutex<Vec<u8>>>,
    JoinHandle<()>,
    Arc<Mutex<Option<ExitStatus>>>,
) {
    use rustix::process::{pidfd_open, Pid, PidfdFlags};
    let mut child = Command::new(binary)
        .arg("--root")
        .arg(root)
        .args(["web", "127.0.0.1:0", "--json"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("real native binary must spawn");
    let pid = Pid::from_raw(i32::try_from(child.id()).expect("child pid must fit i32"))
        .expect("a spawned child pid is non-zero");
    // Opened while the child is demonstrably alive (this very statement runs
    // right after the spawn, before any wait): the fd is the kernel-owned
    // identity of THE spawned process, immune to later pid reuse. If the
    // open itself fails, the owned un-reaped child still must not leak: the
    // numeric fallback is safe precisely because its pid cannot be reused
    // before this test reaps it (stated, with the refusal kept visible).
    let channel = match pidfd_open(pid, PidfdFlags::empty()) {
        Ok(pidfd) => SignalChannel::Pidfd(pidfd),
        Err(e) => {
            eprintln!("pidfd_open failed ({e}); using the owned-child numeric pid until reap");
            SignalChannel::Numeric(pid)
        }
    };
    let observed_exit = Kill::observed_exit_slot();
    let (stderr, stderr_errors, stderr_join) = drain_stderr(&mut child);
    (
        Kill {
            child: Some(child),
            channel,
            observed_exit: Arc::clone(&observed_exit),
        },
        stderr,
        stderr_errors,
        stderr_join,
        observed_exit,
    )
}

/// Save the distinct ledger root through the REAL CLI settings contract of
/// the provided binary (init → engine-settings show → preview --input →
/// apply --input; change.json lives inside the fixture area, never /tmp).
/// No direct settings-file substitution and no in-process apply: the
/// configured Root is the one the spawned listener will read. The shown
/// revision is validated against the real `SettingCount` contract (a
/// canonical nonnegative decimal STRING — its deserializer refuses JSON
/// numbers) and reused VERBATIM as `expected_revision` (same precedent as
/// `witness_e2e.rs`); `schema_version` (number) and the CLI-produced
/// `values` map already match the `EngineSettingsChange` schema, so no
/// other input is converted. Inputs: the provided binary, the two real
/// roots. Output: (). Errors: any CLI non-success, a revision outside the
/// `SettingCount` contract, or a missing preview token is a hard failure
/// (captured stderr, observed value kept). Effects: the explicit settings
/// mutation of the setup, before the first snapshot.
fn run_settings_step(binary: &Path, service_root: &Path, ledger_root: &Path) {
    let cli = |args: &[&str]| -> Value {
        let output = Command::new(binary)
            .arg("--root")
            .arg(service_root)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("native CLI must run");
        assert!(
            output.status.success(),
            "CLI {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|e| panic!("CLI {args:?} stdout is not JSON: {e}"))
    };
    cli(&["init", "--json"]);
    let shown = cli(&["engine-settings", "show", "--json"]);
    // The real show contract is `SettingCount`: a canonical nonnegative
    // decimal STRING (serde serializes it as a string and its deserializer
    // refuses anything else — numbers included). Reuse the shown value
    // VERBATIM; never convert it to a number (the previous
    // `as_u64().unwrap_or(0)` built `0`, a schema-invalid request refused
    // as `settings-request-invalid`).
    let expected_revision = shown["revision"].as_str().unwrap_or_else(|| {
        panic!(
            "settings show revision must be the canonical decimal string required by SettingCount, got: {}",
            shown["revision"]
        )
    });
    assert!(
        !expected_revision.is_empty()
            && expected_revision.bytes().all(|b| b.is_ascii_digit())
            && (expected_revision.len() == 1 || !expected_revision.starts_with('0')),
        "settings show revision must be a canonical nonnegative decimal string per SettingCount, got: {expected_revision:?}"
    );
    let mut values = shown["values"].clone();
    values["paea_settlement_ledger_root"] = ledger_root.to_str().unwrap().into();
    let mut change = json!({
        "schema_version": shown["schema_version"].clone(),
        "event_id": "paea-native-reader-ledger-root",
        "expected_revision": shown["revision"].clone(),
        "values": values,
        "reason": "PA-09 native positive reader witness: distinct uninitialized ledger root",
        "preview_token": "",
    });
    let input = service_root.join("change.json");
    fs::write(&input, serde_json::to_vec(&change).unwrap()).unwrap();
    let preview = cli(&[
        "engine-settings",
        "preview",
        "--input",
        input.to_str().unwrap(),
        "--json",
    ]);
    change["preview_token"] = preview["preview_token"]
        .as_str()
        .expect("settings preview must return a preview token")
        .into();
    fs::write(&input, serde_json::to_vec(&change).unwrap()).unwrap();
    cli(&["engine-settings", "apply", "--input", input.to_str().unwrap(), "--json"]);
}

/// Run the ONE positive browser witness (TEST-ONLY, armed by
/// `SWARM_PAEA_NATIVE_BROWSER_WITNESS=1`) against the already-configured
/// native listener, from inside the native test between the recovery read
/// and the SIGINT stop. Spawns `tests/paea_reconcile_native_reader_ui.cjs`
/// (repo root, cwd = this manifest's repo root) with Node, passing the
/// address, the generated session cookie, the fixture reader token, a
/// synthetic unknown token, the exact expected report and its digest through
/// the child's PRIVATE STDIN (never argv, never logs). The script's stdout,
/// stderr and real exit status are conserved as new 0600 artifacts in the
/// raw-capture evidence directory, plus a FRESH exclusive 0700 subfolder
/// (`browser-witness-<timestamp>-<suffix>` — never reused, never /tmp: the
/// parent is the already-validated RawCapture dir) holding the script's own
/// named artifacts. A nonzero child exit, a spawn failure or a capture
/// failure panics HERE so the native test's unwinding runs the real cleanup
/// path: `Kill::drop` (graceful SIGINT + blocking reap, exit recorded in the
/// shared slot) then `RawOutputWitness::drop` (drain join + raw
/// conservation) — the browser child's own cleanup is its own finally block
/// (closes only its owned browser). The script owns no secrets beyond this
/// call's stdin; nothing it prints is trusted as evidence beyond the
/// conserved artifacts. On a stdin write/flush failure the child is NOT
/// killed: the pipe is closed (EOF), the child's real stdout/stderr/exit are
/// collected and conserved first, and only then is the failure reported with
/// the original IO cause plus the observed exit — the script parses its
/// input before launching any browser, so a truncated stream cannot orphan
/// a browser behind the panic.
/// Inputs: `capture` (reserved evidence dir), `binary`/`binary_sha` (recorded
/// for provenance), `address` (127.0.0.1:port), `cookie_name`/`cookie_value`
/// (generated session credential), `reader_token` (fixture reader),
/// `unknown_token` (synthetic, refused), `expected` (exact attributed report
/// bytes). Output: `()` on success. Errors: panics on spawn/capture failure
/// or a nonzero script exit (the conserved stderr holds the real cause); a
/// stdin write/flush failure panics only AFTER the child's outputs and exit
/// are conserved, reporting the original IO cause and the observed exit.
/// Effects: creates one fresh 0700 subfolder plus 0600 artifacts under the
/// reserved evidence directory; spawns one `node` child and waits for its
/// exit; writes no product state. Limits: no timeout/watchdog is imposed here
/// (event-driven script; the harness's own test timeout is the outer bound) —
/// an aborted script is conserved, not retried.
fn run_browser_witness(
    capture: &RawCapture,
    binary: &std::path::Path,
    binary_sha: &str,
    address: &str,
    cookie_name: &str,
    cookie_value: &str,
    reader_token: &str,
    unknown_token: &str,
    expected: &[u8],
) {
    use std::io::Write as _;
    use std::process::{Command, Stdio};
    use std::time::{SystemTime, UNIX_EPOCH};
    // Wiring invariant: from_env guarantees a reserved evidence directory.
    let capture_dir = capture
        .dir
        .as_ref()
        .expect("try_store requires the evidence directory reserved by from_env");
    // Fresh exclusive subfolder under the already-validated evidence parent:
    // timestamp + address-derived suffix makes collision practically
    // impossible, and the guarded create is the real guard (mode 0700, fails
    // if the path already exists — no overwrite, no reuse).
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_secs();
    let suffix: String = {
        use sha2::{Digest as _, Sha256};
        let mut h = Sha256::new();
        h.update(address.as_bytes());
        h.update(stamp.to_le_bytes());
        h.update(std::process::id().to_le_bytes());
        format!("{:x}", h.finalize())[..8].to_string()
    };
    let run_dir = capture_dir.join(format!("browser-witness-{stamp}-{suffix}"));
    create_dir_mode_0700(&run_dir).unwrap_or_else(|e| {
        panic!(
            "browser witness: cannot reserve fresh evidence subfolder {}: {e}",
            run_dir.display()
        )
    });
    // Repo root = three levels above this crate's manifest dir (the same
    // pattern the fixture area uses); the script lives in <repo>/tests/.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .ancestors()
        .nth(3)
        .expect("manifest dir must have three ancestors to reach the repo root")
        .to_path_buf();
    let script = repo_root.join("tests").join("paea_reconcile_native_reader_ui.cjs");
    let expected_digest: String = {
        use sha2::{Digest as _, Sha256};
        format!("{:x}", Sha256::digest(expected))
    };
    let stdin_json = serde_json::json!({
        "address": address,
        "cookieName": cookie_name,
        "cookieValue": cookie_value,
        "readerToken": reader_token,
        "unknownToken": unknown_token,
        "expectedReport": String::from_utf8(expected.to_vec())
            .expect("expected report must be UTF-8 JSON"),
        "expectedDigest": expected_digest,
        "parentDir": run_dir,
    })
    .to_string();
    let node = std::env::var("NODE_BIN").unwrap_or_else(|_| "node".to_string());
    let mut child = Command::new(&node)
        .arg(&script)
        .current_dir(&repo_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| {
            panic!("browser witness: cannot spawn {node} with {}: {e}", script.display())
        });
    // Private stdin JSON: write it fully, then drop the handle so the child
    // sees EOF. On a write/flush failure the IO error is RETAINED and the
    // handle is dropped anyway: EOF ends the child's read loop, and the
    // script parses its input BEFORE launching any browser, so a truncated
    // stream yields its own nonzero exit instead of an orphan. No kill here:
    // this function owns this one child (no process-group signal), collects
    // its real stdout/stderr/exit first, and only then reports the original
    // IO cause together with the observed exit — the child's own finally
    // block and exit are never cut short.
    let stdin_io_error: Option<std::io::Error> = {
        let mut stdin = child.stdin.take().expect("stdin was piped");
        let result = stdin
            .write_all(stdin_json.as_bytes())
            .and_then(|()| stdin.flush());
        // Dropping `stdin` here (in every case) closes the pipe: the child's
        // read loop ends at EOF.
        result.err()
    };
    let output = match child.wait_with_output() {
        Ok(out) => out,
        Err(e) => panic!("browser witness: cannot collect script output: {e}"),
    };
    let exit = output.status;
    capture.store("browser-witness.stdout.log", &output.stdout);
    capture.store("browser-witness.stderr.log", &output.stderr);
    capture.store(
        "browser-witness.exit.json",
        &serde_json::to_vec(&json!({
            "script": script.to_str().unwrap_or_default(),
            "cwd": repo_root.to_str().unwrap_or_default(),
            "node": node,
            "binary": binary.to_str().unwrap_or_default(),
            "binary_sha256": binary_sha,
            "address": address,
            "run_dir": run_dir.to_str().unwrap_or_default(),
            "observed": true,
            "stdin_io_error": stdin_io_error.as_ref().map(|e| e.to_string()),
            "exit": {
                "code": exit.code(),
                "success": exit.success(),
                "status": format!("{exit}"),
            },
        }))
        .expect("browser witness exit metadata must serialize"),
    );
    if let Some(io_err) = stdin_io_error {
        panic!(
            "browser witness: cannot write private stdin JSON: {io_err} \
             (child was left to finish: its real exit is {exit}, its outputs are \
             conserved in browser-witness.stdout.log / browser-witness.stderr.log \
             and its run dir recorded in browser-witness.exit.json; any cleanup \
             cause it reported appears in those artifacts)"
        );
    }
    assert!(
        exit.success(),
        "browser witness script must exit 0 (exit {exit}); its stderr is conserved in \
         browser-witness.stderr.log and its run dir recorded in \
         browser-witness.exit.json"
    );
}

/// PA-09 POSITIVE reader witness on the REAL qualified native binary
/// (`SWARM_ENGINE_BINARY`), empty explicit ledger scope. The default six
/// tests cover the in-process transport; THIS test only adds what they do
/// not prove: the real ELF, its real bind event, its really-generated
/// session secret and the real HTTP channel under a configured fixture
/// Trust.
///
/// Preconditions (all refused, never faked): the qualified binary must be
/// passed explicitly through the documented environment (same precedent as
/// `witness_e2e.rs`; no hash or path is hardcoded in the repository).
///
/// Identity scope: the fixture identities of `identity_config` are an
/// explicit same-UID SIMULATION on owned fixture roots (token-bearing
/// subjects authenticated by Bearer credential, never claimed as a real
/// human enrollment, OS identity confinement or operator acceptance).
/// Scope limits: the ledger root stays explicitly uninitialized (report
/// `{"schema_version":1,"settlement":{"state":"uninitialized"},"git":
/// {"state":"uninitialized"}}`), so the POSITIVE non-empty native report is
/// NOT covered here — the dedicated non-empty sibling
/// `reconcile_effects_native_binary_positive_reader_frontier_nonempty_ledger`
/// covers the native non-empty API report (still NOT TESTED by any run at
/// this lot's writing); no durable effect is exercised by this lot beyond
/// the read-only browser witness below.
///
/// Sequence (refusals BEFORE attribution, then the positive read, then the
/// unknown-identity refusal and recovery — the FULL snapshot is re-checked
/// after every read phase, the attribution being the only intentional Trust
/// write between the S1 and S2 baselines):
/// 1. bare session → typed 403 `paea-reader-authentication-required` with
///    no report byte, HEAD empty;
/// 2. known fixture reader (Bearer fixture-admin-token) BEFORE attribution
///    → typed 403 `paea-reader-attribution-required`, no report byte;
/// 3. the exact empty report is computed by the public producer
///    (`paea_reconcile_effect_journals`), both journals asserted
///    `Uninitialized`, its canonical encoding digested with the SAME
///    domain the route recomputes, and attributed through the supported
///    `blp-object-attribute` on the service root;
/// 4. positive GET 200: served bytes EXACTLY the encoding above, JSON
///    states explicit (`settlement.state == "uninitialized"`, `git.state
///    == "uninitialized"`), HEAD mirrors as an empty 200;
/// 5. unknown credential → typed 403 `authorization_required` (Trust now
///    configured), no report byte;
/// 6. recovery: the known reader reads 200 the same exact bytes;
/// 6b. TEST-ONLY browser witness (armed by
///     `SWARM_PAEA_NATIVE_BROWSER_WITNESS=1`, contract on
///     `run_browser_witness`): one read-only headless-browser pass over
///     the already-configured listener between the recovery read and the
///     SIGINT stop — the browser issues GET reads only, so the snapshot
///     discipline of phase 9 still covers it; absent variable = never
///     spawned, the sequence above is unchanged;
/// 7. the own listener child is stopped by SIGINT through its
///    kernel-owned signal channel (pidfd, numeric pid only if `pidfd_open`
///    itself failed on the owned un-reaped child), asserting the real
///    shutdown contract: exit code 0 and the final retained stdout line is
///    the `web-stopped` stop-record; both drain threads join after the
///    stop (taken back from the raw-output witness, whose failure Drop
///    joins them to EOF itself after the listener was reaped);
/// 8. the COMPLETE raw child outputs are conserved durably — the REQUIRED
///    `SWARM_PAEA_NATIVE_EVIDENCE_DIR` must be a FRESH non-existent path
///    (exclusively reserved 0700 FROM BIRTH before the spawn — a reused,
///    concurrently claimed, /tmp or MISSING path is refused with no write
///    and the test does not run at all without it: no PASS without
///    evidence): exact `stdout.log`/`stderr.log` raw child bytes (NEVER
///    searched or truncated by content — a drain error never enters the
///    raw streams), the drain failures only in separate
///    `drain-*.errors.log` sidecars (empty on the success path, asserted),
///    the real `exit.json` from the shared observed-exit slot
///    (`observed: false` when no exit could be reaped — never faked), and
///    `spawn.json` metadata (binary path/sha256/argv — never a secret or a
///    cookie) — written on SUCCESS and FAILURE alike through an
///    exclusive-create 0600 temp file, file fsync, EXCLUSIVE hard_link
///    publication (an existing name is never overwritten) and directory
///    fsync; a partial write keeps the original file and its cause; on the
///    FAILURE path the witness joins both drains to EOF (the child was
///    already reaped by the listener drop) and conserves the COMPLETE
///    streams plus the real exit — a buffer wired after a pre-capture
///    failure is stated as an exact frontier, never claimed as full;
///    bytes emitted before the stdout reader is attached (pre-announcement
///    window) are outside any test-side capture and are not claimed;
/// 9. the FULL four-file snapshot is compared after EVERY read phase
///    (pre-attribution refusals included, then positive/unknown reads, then
///    child stop): the two state databases stay byte-identical throughout
///    and the trust journals only move across the explicit Trust writes of
///    the setup itself — the witness never mutates any root.
///
/// TEST-ONLY fault mode (`SWARM_PAEA_NATIVE_WITNESS_FAULT`, this
/// integration test only — never the product, the ELF or any authority):
/// absent variable = nominal run above, unchanged. The single accepted
/// value `after-bare-refusal` arms a marker panic
/// (`PAEA_NATIVE_EXPECTED_WITNESS_FAILURE`) delivered after the bare-403
/// GET+HEAD control and the pre-attribution refusal snapshot S1, while the
/// listener is still alive, to EXERCISE the real failure path: the
/// listener `Kill::drop` performs the graceful SIGINT + blocking reap
/// (real exit recorded in the shared slot), then `RawOutputWitness::drop`
/// joins both drains to EOF and conserves the complete raw stdout/stderr,
/// the drain-error states and the observed exit — the same artifacts as
/// the success path, with the exact frontier. Any other value is refused
/// BEFORE any evidence reservation or spawn. A run with this value is an
/// EXPECTED 1-FAILED (exit 101): the retained artifacts, not the test
/// verdict, are the evidence.

#[test]
#[ignore = "requires the real qualified native binary passed via SWARM_ENGINE_BINARY (no hardcoded hash in repo); run with --ignored --exact"]
fn reconcile_effects_native_binary_positive_reader_frontier_empty_ledger() {
    let (binary, binary_sha) = native_binary_from_env();
    // Two REALLY distinct private fixture roots under the repository fixture
    // area (never /tmp): the settlement service Root and the distinct,
    // explicitly uninitialized Reconcile ledger Root.
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let area = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../.swarm-fixtures");
    fs::create_dir_all(&area).unwrap();
    let area = area.canonicalize().unwrap();
    let service_root = fresh_fixture_root(&area, "paea-native-reader", nanos);
    let ledger_root = fresh_fixture_root(&area, "paea-native-ledger", nanos);
    // Explicit fixture-root initialization through the public producer; the
    // service Root then gets its ledger path through the REAL CLI settings
    // contract of the provided binary (no direct state substitution).
    for root in [&service_root, &ledger_root] {
        let store = RootStore::initialize(root, &mut SystemEnvironment).unwrap();
        store.close().unwrap();
    }
    assert_ne!(service_root, ledger_root, "the two fixture roots must be really distinct");
    run_settings_step(&binary, &service_root, &ledger_root);
    // Fixture identities + BLP policy on the SERVICE root through the
    // supported Trust operations (paired-close discipline on every path).
    let trust = |op: &str, value: Value, token: &str| {
        trust_on_root(&service_root, op, value, token)
    };
    trust("config-apply", identity_config(), "");
    let blp_revision = trust("blp-show", json!({}), "fixture-admin-token")["record"]
        ["history"]
        .as_array()
        .and_then(|h| h.last())
        .and_then(|e| e["revision"].as_u64())
        .unwrap_or(0);
    let mut policy = native_reader_blp_policy();
    policy["event_id"] = json!("paea-native-reader-policy");
    policy["expected_revision"] = json!(blp_revision);
    policy["expected_trust_revision"] = json!(1);
    policy["reason"] = json!("PA-09 native positive reader policy on the empty explicit ledger");
    trust("blp-policy-apply", policy, "fixture-admin-token");
    // Byte-witness S0 AFTER the intentional setup mutations (settings,
    // identities, policy): the FULL snapshot is compared after every read
    // phase below — the state databases must never move, and the trust
    // journals must not move until the single intentional attribution
    // (compared against the post-attribution snapshot taken below).
    let (baseline_service_db, baseline_ledger_db, baseline_service_trust, baseline_ledger_trust) =
        journal_snapshot(&service_root, &ledger_root);
    // Raw-output recovery witness, declared BEFORE `listener` so Rust drops
    // it AFTER: on the failure path the listener Drop has already stopped and
    // reaped the child (recording the real exit in the shared slot) before
    // this witness's Drop runs — so the witness can join BOTH drain threads
    // to EOF without blocking (the pipes close at child death) and conserve
    // the COMPLETE stdout/stderr raw bytes plus the REAL observed exit, the
    // same artifacts as the success path. It holds a clone of the durable
    // `RawCapture`, clones of the four stream buffers (raw stdout/stderr and
    // the two separate drain-error states), the two drain handles and the
    // shared observed-exit slot — the module-level `RawOutputWitness` struct
    // shared verbatim by the empty and non-empty native siblings. No secret
    // byte is ever written to native stdout/stderr (`web-bound`/`web-outcome`/
    // stop-record only; the session secret lives only in its file), so
    // raw-byte diagnostics are safe. The witness also owns the two drain
    // handles: the normal stop phase takes them back for its own asserted
    // joins; any earlier failure leaves them here for the Drop to join to
    // EOF. If a helper failed BEFORE capture started (e.g. a readiness
    // failure), the witness conserves whatever buffers exist and declares
    // the exact frontier — never claiming a full capture that did not
    // happen. Inputs: none at construction; buffers, handles, slot and
    // capture wired right after spawn/announcement.
    // TEST-ONLY fault-injection gate (this integration test only — never the
    // product, the ELF or any authority): makes the witness's REAL failure
    // path (Kill::drop then RawOutputWitness::drop during unwinding)
    // exercisable. Absent variable => nominal run, unchanged. The single
    // accepted value `after-bare-refusal` arms a marker panic delivered after
    // the bare-403 control and the S1 pre-attribution snapshot. Any other
    // value is refused HERE, BEFORE any evidence reservation or spawn (no
    // watchdog, timer or retry is involved anywhere).
    let witness_fault = match std::env::var("SWARM_PAEA_NATIVE_WITNESS_FAULT") {
        Ok(value) if value == "after-bare-refusal" => Some(value),
        Ok(other) => panic!(
            "SWARM_PAEA_NATIVE_WITNESS_FAULT accepts only the exact value `after-bare-refusal` \
             (absent = nominal run); any other value is refused before any reservation or \
             spawn: {other:?}"
        ),
        Err(std::env::VarError::NotPresent) => None,
        Err(e) => panic!(
            "SWARM_PAEA_NATIVE_WITNESS_FAULT must be readable to be honored (absent = nominal \
             run): {e}"
        ),
    };
    // TEST-ONLY positive browser-witness gate (same scope as the fault gate:
    // this integration test only — never the product): absent variable => the
    // qualified API sequence is UNCHANGED. The exact value `1` arms the ONE
    // browser witness run between the recovery read and the SIGINT stop (see
    // `run_browser_witness`). Any other value is refused HERE, BEFORE any
    // evidence reservation or spawn (no watchdog, timer or retry involved).
    let browser_witness = match std::env::var("SWARM_PAEA_NATIVE_BROWSER_WITNESS") {
        Ok(value) if value == "1" => true,
        Ok(other) => panic!(
            "SWARM_PAEA_NATIVE_BROWSER_WITNESS accepts only the exact value `1` \
             (absent = API-only nominal run); any other value is refused before any \
             reservation or spawn: {other:?}"
        ),
        Err(std::env::VarError::NotPresent) => false,
        Err(e) => panic!(
            "SWARM_PAEA_NATIVE_BROWSER_WITNESS must be readable to be honored (absent = \
             API-only nominal run): {e}"
        ),
    };
    // Start the REAL binary on the configured Root service: port 0, JSON
    // bind event, really-generated session secret, real process identity.
    // Durable raw retention is REQUIRED: the operator provides a FRESH
    // `SWARM_PAEA_NATIVE_EVIDENCE_DIR`, EXCLUSIVELY reserved 0700 from birth
    // inside `from_env` BEFORE the spawn (reuse, a concurrent foreign claim,
    // a /tmp path or a missing variable is refused with nothing written).
    let raw_capture = RawCapture::from_env();
    raw_capture.store("spawn.json", &raw_capture.spawn_bytes(&binary, &binary_sha, &service_root));
    let mut raw_witness = RawOutputWitness::default();
    raw_witness.capture = Some(RawCapture {
        dir: raw_capture.dir.clone(),
    });
    let (mut listener, stderr_buffer, stderr_errors, stderr_join, observed_exit) =
        spawn_web(&binary, &service_root);
    // Stderr is wired (buffer + separate error state + drain handle) the
    // moment it exists — BEFORE the readiness announcement — so a readiness
    // failure still conserves the complete drained stderr and its error
    // state, with the exact frontier (stdout unwired = nothing claimed).
    raw_witness.stderr = Arc::clone(&stderr_buffer);
    raw_witness.stderr_errors = Arc::clone(&stderr_errors);
    raw_witness.exit_slot = Some(Arc::clone(&observed_exit));
    raw_witness.wired_stderr = true;
    let (address, reader, stdout_retained, stdout_errors) = bind_announcement(
        listener.child.as_mut().expect("listener child not yet consumed"),
        &stderr_buffer,
    );
    raw_witness.stdout = Arc::clone(&stdout_retained);
    raw_witness.stdout_errors = Arc::clone(&stdout_errors);
    raw_witness.wired_stdout = true;
    raw_witness.reader = Some(reader);
    raw_witness.stderr_join = Some(stderr_join);
    let port: u16 = address.rsplit(':').next().unwrap().parse().unwrap();
    {
        use sha2::{Digest as _, Sha256};
        let exe_sha = format!("{:x}", Sha256::digest(fs::read(format!("/proc/{}/exe", listener.child.as_ref().expect("listener child not yet consumed").id())).unwrap()));
        assert_eq!(exe_sha, binary_sha, "the running process must be the provided binary");
    }
    let token = web_session_secret(&service_root, &address);
    let address_hash = {
        use sha2::{Digest as _, Sha256};
        format!("{:x}", Sha256::digest(address.as_bytes()))
    };
    let cookie_name = format!("swarm_session_{}", &address_hash[..12]);
    let cookie = format!("{}={}", cookie_name, token);
    // 1. BARE session (no Bearer credential): typed authentication refusal
    //    BEFORE any IO, no report byte, HEAD empty.
    {
        let (status, body) = raw_http(port, "GET", RECONCILE_REPORT, Some(&cookie), None);
        assert_eq!(status, 403, "bare read must be refused: {}", String::from_utf8_lossy(&body));
        assert!(
            String::from_utf8_lossy(&body).contains("paea-reader-authentication-required"),
            "bare refusal must carry the typed code"
        );
        assert!(
            !String::from_utf8_lossy(&body).contains("uninitialized"),
            "bare refusal must not disclose report states"
        );
        let (head_status, head_body) = raw_http(port, "HEAD", RECONCILE_REPORT, Some(&cookie), None);
        assert_eq!(head_status, 403);
        assert!(head_body.is_empty(), "HEAD refusal must have an empty body");
    }
    // 2. Known fixture reader BEFORE attribution: typed attribution refusal,
    //    no report byte (clearance alone invents no provenance).
    {
        let (status, body) = raw_http(port, "GET", RECONCILE_REPORT, Some(&cookie), Some("fixture-admin-token"));
        assert_eq!(status, 403, "unattributed read must be refused: {}", String::from_utf8_lossy(&body));
        assert!(
            String::from_utf8_lossy(&body).contains("paea-reader-attribution-required"),
            "unattributed refusal must carry the typed code"
        );
        assert!(
            !String::from_utf8_lossy(&body).contains("uninitialized"),
            "unattributed refusal must not disclose report states"
        );
    }
    // Full snapshot S1 compared against S0: the PRE-ATTRIBUTION refusal
    // phase (bare 403, HEAD, known-before-attribution 403) must already have
    // moved nothing — a mutation here can no longer be masked by the later
    // post-attribution baseline.
    {
        let (s_db, l_db, s_trust, l_trust) = journal_snapshot(&service_root, &ledger_root);
        assert_eq!(s_db, baseline_service_db, "service state.db mutated by pre-attribution refusals");
        assert_eq!(l_db, baseline_ledger_db, "ledger state.db mutated by pre-attribution refusals");
        assert_eq!(s_trust, baseline_service_trust, "service trust journal mutated by pre-attribution refusals");
        assert_eq!(l_trust, baseline_ledger_trust, "ledger trust journal mutated by pre-attribution refusals");
    }
    // TEST-ONLY fault injection (`SWARM_PAEA_NATIVE_WITNESS_FAULT =
    // after-bare-refusal`, validated BEFORE spawn): the listener is STILL
    // ALIVE here — the bare-403 GET+HEAD control and the pre-attribution
    // refusal phase (snapshot S1) have been observed, no report byte served.
    // This panic exercises the real witness failure path during unwinding:
    // `Kill::drop` first performs the graceful SIGINT + blocking reap (the
    // shared slot records the REAL child exit), then `RawOutputWitness::drop`
    // joins both drains to EOF and conserves the complete raw stdout/stderr,
    // the drain-error states and the observed exit in the evidence
    // directory — the same artifacts as the success path.
    if witness_fault.is_some() {
        panic!("PAEA_NATIVE_EXPECTED_WITNESS_FAILURE: requested fault after the bare-403 control and the pre-attribution refusal snapshot S1 (listener still alive)");
    }
    // 3. The EXACT expected report via the public producer on the real
    //    roots: both journals explicitly Uninitialized, canonical encoding,
    //    digest in the SAME domain the route recomputes (`digest_trust`).
    let expected = {
        let service = RootStore::open(&service_root, false).unwrap();
        let ledger = RootStore::open(&ledger_root, false).unwrap();
        let report = service.paea_reconcile_effect_journals(&ledger).unwrap();
        ledger.close().unwrap();
        service.close().unwrap();
        assert_eq!(
            report.schema_version, 1,
            "the empty report must carry the current schema"
        );
        assert_eq!(
            report.settlement,
            PaeaEffectReconcileJournal::Uninitialized,
            "settlement journal must be explicitly Uninitialized"
        );
        assert_eq!(
            report.git,
            PaeaEffectReconcileJournal::Uninitialized,
            "git journal must be explicitly Uninitialized"
        );
        let encoded = swarm_engine::encode_model(&report).unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&encoded).unwrap(),
            json!({
                "schema_version":1,
                "settlement":{"state":"uninitialized"},
                "git":{"state":"uninitialized"},
            }),
            "the served JSON must be the explicit empty report"
        );
        encoded
    };
    let report_digest = {
        use sha2::{Digest as _, Sha256};
        format!("{:x}", Sha256::digest(&expected))
    };
    // The attribution binds the recomputed digest, the REAL service root and
    // the current policy revision through the supported control operation.
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
            "event_id":"paea-native-reader-attribute",
            "expected_revision":attribute_revision,
            "expected_trust_revision":1,
            "canonical_digest":report_digest,
            "project_path":service_root.to_str().unwrap(),
            "classification":{"level":"classified","compartments":[]},
            "reason":"PA-09 native attribution of the exact empty report",
        }),
        "fixture-admin-token",
    );
    assert_eq!(attribute_reply["attributed"], true, "supported producer must attribute");
    // The attribution is the LAST intentional Trust write of the setup: the
    // full read-sequence baseline S2 is retaken here (the journals
    // legitimately moved across the attribution), then the reads must move
    // nothing.
    let (before_service_db, before_ledger_db, before_service_trust, before_ledger_trust) =
        journal_snapshot(&service_root, &ledger_root);
    // 4. POSITIVE read through the real binary: served bytes EXACTLY the
    //    attributed encoding; explicit JSON states; HEAD mirrors empty 200.
    {
        let (status, body) = raw_http(port, "GET", RECONCILE_REPORT, Some(&cookie), Some("fixture-admin-token"));
        assert_eq!(status, 200, "positive read refused: {}", String::from_utf8_lossy(&body));
        assert_eq!(body, expected, "served bytes must equal the attributed encoding exactly");
        let body_json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body_json["schema_version"], 1);
        assert_eq!(body_json["settlement"]["state"], "uninitialized");
        assert_eq!(body_json["git"]["state"], "uninitialized");
        let (head_status, head_body) = raw_http(port, "HEAD", RECONCILE_REPORT, Some(&cookie), Some("fixture-admin-token"));
        assert_eq!(head_status, 200);
        assert!(head_body.is_empty(), "HEAD mirror must be an empty 200");
    }
    // 5. UNKNOWN credential once Trust is configured: typed authorization
    //    refusal, non-success, no report byte.
    {
        let (status, body) = raw_http(port, "GET", RECONCILE_REPORT, Some(&cookie), Some("unknown-reader-token"));
        assert_eq!(status, 403, "unknown credential must be refused: {}", String::from_utf8_lossy(&body));
        assert!(
            String::from_utf8_lossy(&body).contains("authorization_required"),
            "unknown refusal must carry the typed code"
        );
        assert!(
            !String::from_utf8_lossy(&body).contains("uninitialized"),
            "unknown refusal must not disclose report states"
        );
        // Full snapshot S3 compared against the post-attribution baseline S2:
        // the positive read + HEAD + unknown refusal moved nothing.
        let (s_db, l_db, s_trust, l_trust) = journal_snapshot(&service_root, &ledger_root);
        assert_eq!(s_db, before_service_db, "service state.db mutated by positive/unknown reads");
        assert_eq!(l_db, before_ledger_db, "ledger state.db mutated by positive/unknown reads");
        assert_eq!(s_trust, before_service_trust, "service trust journal mutated by positive/unknown reads");
        assert_eq!(l_trust, before_ledger_trust, "ledger trust journal mutated by positive/unknown reads");
    }
    // 6. RECOVERY: the known reader reads 200 the same exact bytes — the
    //    refusals were identity/attribution-state-driven, channel intact.
    {
        let (status, body) = raw_http(port, "GET", RECONCILE_REPORT, Some(&cookie), Some("fixture-admin-token"));
        assert_eq!(status, 200, "recovery read refused: {}", String::from_utf8_lossy(&body));
        assert_eq!(body, expected, "recovered read must serve the same exact bytes");
    }
    // 6b. TEST-ONLY positive browser witness (armed by
    //     SWARM_PAEA_NATIVE_BROWSER_WITNESS=1, validated BEFORE spawn): the
    //     public attribution is done and the listener is STILL ALIVE here —
    //     one real Chromium run replays the whole reader frontier through the
    //     REAL served page and stores its own evidence. Nominal runs (env
    //     absent) skip this entirely — the qualified API sequence is
    //     unchanged.
    if browser_witness {
        run_browser_witness(
            &raw_capture,
            &binary,
            &binary_sha,
            &address,
            &cookie_name,
            &token,
            "fixture-admin-token",
            "unknown-reader-token",
            &expected,
        );
    }
    // 7. Stop the OWN child with SIGINT and assert the REAL shutdown
    //    contract (verified in `swarm-cli/src/native_web.rs`): a clean
    //    SIGINT shutdown exits 0 and the LAST stdout line is the
    //    `web-stopped` stop-record. The child's own stderr is retained in
    //    the failure message (real causes kept, secret bytes never printed).
    let exit = listener.sigint_and_wait();
    let stderr_cause = String::from_utf8_lossy(&stderr_buffer
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()))
    .into_owned();
    assert_eq!(
        exit.code(),
        Some(0),
        "native listener must exit cleanly (0) on SIGINT per the native shutdown contract, got {exit}; stderr: {stderr_cause}"
    );
    // Join BOTH drain threads now that the child is stopped (EOF reached):
    // the readiness/retention reader first, then the incremental stderr
    // drain. The handles are taken back from the raw-output witness (its
    // failure path will then have nothing left to join — the normal path
    // owns its own asserted joins). Drain read errors live in the SEPARATE
    // error states (joined above), the raw buffers stay exact child bytes,
    // and a nonempty error state on the success path is a hard failure, not
    // a silent annotation.
    let reader = raw_witness.reader.take().expect("stdout reader handle held by the witness");
    let stderr_join = raw_witness.stderr_join.take().expect("stderr drain handle held by the witness");
    reader.join().expect("stdout reader thread must join");
    stderr_join.join().expect("stderr drain thread must join");
    let stdout_drain_errors = stdout_errors
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    assert!(
        stdout_drain_errors.is_empty(),
        "stdout drain must reach EOF without error on the success path: {}",
        String::from_utf8_lossy(&stdout_drain_errors)
    );
    let stderr_drain_errors = stderr_errors
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    assert!(
        stderr_drain_errors.is_empty(),
        "stderr drain must reach EOF without error on the success path: {}",
        String::from_utf8_lossy(&stderr_drain_errors)
    );
    let stdout_final = String::from_utf8_lossy(&stdout_retained
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()))
    .into_owned();
    // Durable conservation of the COMPLETE raw child outputs (SUCCESS path):
    // exact stdout/stderr bytes (never searched or truncated by content),
    // the separate drain-error states (asserted empty above), the REAL
    // observed exit status read from the shared slot (the same source the
    // failure witness uses), and spawn metadata. Run on success too — the
    // evidence directory must exist even for a green run. A conservation
    // failure panics with its real cause (no fake PASS) and the witness Drop
    // then reprints whatever it still holds without a second panic.
    let stdout_bytes = stdout_retained
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    raw_capture.store("stdout.log", &stdout_bytes);
    raw_capture.store("drain-stdout.errors.log", &stdout_drain_errors);
    let stderr_bytes = stderr_buffer
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    raw_capture.store("stderr.log", &stderr_bytes);
    raw_capture.store("drain-stderr.errors.log", &stderr_drain_errors);
    let observed_exit = observed_exit
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    assert_eq!(
        observed_exit.as_ref().map(|e| e.code()),
        Some(exit.code()),
        "the shared exit slot must hold the observed exit equal to the awaited one"
    );
    raw_capture.store(
        "exit.json",
        &serde_json::to_vec(&json!({
            "observed": true,
            "exit": {
                "exit_code": exit.code(),
                "success": exit.success(),
                "status": format!("{exit}"),
            },
        }))
        .expect("exit metadata must serialize"),
    );
    assert!(
        stdout_final.lines().last().is_some_and(|line| line.contains("\"web-stopped\"")),
        "the last retained stdout line must be the web-stopped stop-record per the native shutdown contract; stdout tail: {:?}",
        stdout_final
            .lines()
            .last()
            .map(|line| line.chars().take(120).collect::<String>())
    );
    // 8. Byte-witness AFTER the whole read sequence AND after the child
    //    stop: the state databases never moved (compared to the S0
    //    pre-listener baseline) and the trust journals only moved across the
    //    explicit attribution (compared to the S2 post-attribution snapshot)
    //    — the witness never mutated any root during the reads.
    let (after_service_db, after_ledger_db, after_service_trust, after_ledger_trust) =
        journal_snapshot(&service_root, &ledger_root);
    assert_eq!(after_service_db, baseline_service_db, "service state.db mutated");
    assert_eq!(after_ledger_db, baseline_ledger_db, "ledger state.db mutated");
    assert_eq!(after_service_trust, before_service_trust, "service trust journal mutated by reads");
    assert_eq!(after_ledger_trust, before_ledger_trust, "ledger trust journal mutated");
    // Cleanup covers ONLY these two owned fixture roots (failed runs keep
    // them for inspection, mirroring the received Fixture Drop discipline).
    if !std::thread::panicking() {
        let _ = fs::remove_dir_all(&service_root);
        let _ = fs::remove_dir_all(&ledger_root);
    }
}

/// PA-09 POSITIVE reader witness on the REAL qualified native binary
/// (`SWARM_ENGINE_BINARY`), NON-EMPTY explicit ledger scope — the API half of
/// the native non-empty frontier (the empty sibling above proves the real
/// ELF channel; the default six tests prove the in-process non-empty
/// transport; THIS test adds what neither proves: a REAL recorded settlement
/// effect observed through the REAL ELF's served bytes).
///
/// Preconditions (all refused, never faked): the qualified binary must be
/// passed explicitly through the documented environment (same precedent as
/// `witness_e2e.rs`; no hash or path is hardcoded in the repository); the
/// REQUIRED `SWARM_PAEA_NATIVE_EVIDENCE_DIR` must be a FRESH non-existent
/// path outside /tmp (exclusively reserved 0700 FROM BIRTH before the spawn —
/// no PASS without retained evidence).
///
/// Identity scope: the fixture identities of `identity_config` are an
/// explicit same-UID SIMULATION on owned fixture roots (token-bearing
/// subjects authenticated by Bearer credential, never claimed as a real
/// human enrollment, OS identity confinement or operator acceptance).
///
/// Scope limits: the TEST-ONLY positive browser-witness gate
/// `SWARM_PAEA_NATIVE_BROWSER_WITNESS` has the SAME exact contract as the
/// empty sibling: absent => the received API-only non-empty sequence runs
/// UNCHANGED (nominal, qualified); the exact value `1` => ONE browser
/// witness run (real Chromium, shared script
/// `tests/paea_reconcile_native_reader_ui.cjs` in its CONSISTENT
/// non-empty variant, kind `paea-native-reader-ui-nonempty-v1`, metadata
/// `report_scope: "nonempty"`) is executed between the recovery read and
/// the SIGINT stop, with the EXACT attributed non-empty report — the script
/// derives its in-memory render model from the report itself, asserts the
/// REAL product render (consistent settlement sentence, real finding line
/// with the served `intent_id`/`action_digest`), the same-page unknown
/// refusal purging the previously rendered REAL identifiers, and the
/// identical recovery; any other value is refused BEFORE any evidence
/// reservation or spawn. The browser is GET-only and bearer-scoped, so the
/// phase-9 snapshot discipline still covers the armed variant. The fault
/// gate `SWARM_PAEA_NATIVE_WITNESS_FAULT` is not read by this test: its
/// value (if set) is simply never honored here.
///
/// Sequence (the REAL settlement effect is produced through the SAME public
/// producers as the in-process witness — `work.create`, `config-apply`,
/// `scope_of`, `seed_settlement_effect` — BEFORE any baseline or child; the
/// settlement project root is a THIRD private fixture root kept open as the
/// work store for those producers; refusals BEFORE attribution, then the
/// positive read, then the unknown-identity refusal and recovery — the FULL
/// four-file snapshot is re-checked after every read phase, the attribution
/// being the only intentional Trust write between the S1 and S2 baselines):
/// 1. bare session → typed 403 `paea-reader-authentication-required` with
///    no report byte, HEAD empty;
/// 2. known fixture reader (Bearer fixture-admin-token) BEFORE attribution
///    → typed 403 `paea-reader-attribution-required`, no report byte;
/// 3. the exact NON-EMPTY report is computed by the public producer
///    (`paea_reconcile_effect_journals` on the REAL service+ledger roots):
///    settlement destructured as `Consistent { effect_count: 1, findings }`
///    with the finding `verified` (no refusals), git EXPLICITLY
///    `Uninitialized` (this fixture runs no git schema on the service
///    root — asserted, never assumed), canonical encoding digested with the
///    SAME domain the route recomputes, and attributed through the
///    supported `blp-object-attribute` on the SERVICE root;
/// 4. positive GET 200: served bytes EXACTLY the encoding above, JSON
///    fields asserted from the REAL model (`settlement.state ==
///    "consistent"`, `effectCount == 1`, `findings[0].intent_id/
///    action_digest` equal to the REAL produced identifiers, `verified ==
///    true`, `git.state == "uninitialized"`), HEAD mirrors as an empty 200;
/// 5. unknown credential → typed 403 `authorization_required` (Trust now
///    configured), no report byte — the served refusal must not carry the
///    real intent id, digest or the `"consistent"` state;
/// 6. recovery: the known reader reads 200 the same exact bytes;
/// 6b. TEST-ONLY browser witness (only when armed by
///     `SWARM_PAEA_NATIVE_BROWSER_WITNESS=1`): one real Chromium run
///     replays the whole reader frontier on the NON-EMPTY report through
///     the REAL served page and stores its own evidence (distinct witness
///     kind, GET-only, between two full snapshot comparisons); nominal
///     runs (env absent) skip this entirely;
/// 7. the own listener child is stopped by SIGINT through its
///    kernel-owned signal channel (same contract as the empty sibling):
///    exit code 0, last retained stdout line `web-stopped`; both drain
///    threads join after the stop;
/// 8. the COMPLETE raw child outputs are conserved durably in the fresh
///    evidence directory (`stdout.log`, `stderr.log`, empty
///    `drain-*.errors.log`, `exit.json`, `spawn.json` — never a secret or a
///    cookie), success and failure paths alike (shared `RawOutputWitness`);
/// 9. the FULL four-file snapshot is compared after EVERY read phase: the
///    two state databases never move, the trust journals only across the
///    explicit setup/attribution writes — the witness never mutates any
///    root.
///
/// Refused/unchanged: no invented SQL or state file, no live database, no
/// forged UID/enrollment/payment/provider, no public Git, no reduction of
/// the assertion to the empty report or to a bare hash comparison. Not
/// covered here (explicit): the browser non-empty witness armed by this lot
/// is NOT TESTED yet (no run with `SWARM_PAEA_NATIVE_BROWSER_WITNESS=1`
/// backs this contract — the supervisor runs it personally after
/// stability); any E3/OS-simulated acceptance, any human enrollment proof.
#[test]
#[ignore = "requires the real qualified native binary passed via SWARM_ENGINE_BINARY (no hardcoded hash in repo); run with --ignored --exact"]
fn reconcile_effects_native_binary_positive_reader_frontier_nonempty_ledger() {
    // TEST-ONLY positive browser-witness gate (same scope and exact contract
    // as the empty sibling: this integration test only — never the product),
    // evaluated BEFORE anything else (before evidence reservation, before
    // fixture creation, before spawn): absent variable => the received
    // API-only non-empty sequence is UNCHANGED; the exact value `1` arms the
    // ONE browser witness run between the recovery read and the SIGINT stop
    // (see `run_browser_witness`); any other value is refused HERE, BEFORE
    // any evidence reservation or spawn (no watchdog, timer or retry
    // involved).
    let browser_witness = match std::env::var("SWARM_PAEA_NATIVE_BROWSER_WITNESS") {
        Ok(value) if value == "1" => true,
        Ok(other) => panic!(
            "SWARM_PAEA_NATIVE_BROWSER_WITNESS accepts only the exact value `1` \
             (absent = API-only nominal run); any other value is refused before any \
             reservation or spawn: {other:?}"
        ),
        Err(std::env::VarError::NotPresent) => false,
        Err(e) => panic!(
            "SWARM_PAEA_NATIVE_BROWSER_WITNESS must be readable to be honored (absent = \
             API-only nominal run): {e}"
        ),
    };
    let (binary, binary_sha) = native_binary_from_env();
    // Three REALLY distinct private fixture roots under the repository fixture
    // area (never /tmp, never a copied forest): the settlement SERVICE root
    // (listener + BLP), the distinct LEDGER root the report reconciles, and
    // the third PROJECT root the documented seeding producers require
    // (kept open as the work store; never the report's service root).
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let area = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../.swarm-fixtures");
    fs::create_dir_all(&area).unwrap();
    let area = area.canonicalize().unwrap();
    let service_root = fresh_fixture_root(&area, "paea-native-nonempty-reader", nanos);
    let ledger_root = fresh_fixture_root(&area, "paea-native-nonempty-ledger", nanos);
    let project_root = fresh_fixture_root(&area, "paea-native-nonempty-project", nanos);
    // Explicit fixture-root initialization through the public producer: the
    // service and ledger roots are initialized and closed (the listener owns
    // its opens later); the PROJECT root stays OPEN as the work store
    // (`pstore`) for the seeding producers below, and is closed at the end.
    for root in [&service_root, &ledger_root] {
        let store = RootStore::initialize(root, &mut SystemEnvironment).unwrap();
        store.close().unwrap();
    }
    let mut pstore = RootStore::initialize(&project_root, &mut SystemEnvironment).unwrap();
    assert_ne!(service_root, ledger_root, "service and ledger roots must be really distinct");
    assert_ne!(service_root, project_root, "service and project roots must be really distinct");
    assert_ne!(ledger_root, project_root, "ledger and project roots must be really distinct");
    // PUBLIC fixture work + configuration + seed BEFORE any baseline and
    // before the child: the exact same public producers as the in-process
    // non-empty witness (lines ~809-850 of its received contract) — no
    // invented SQL, no state-file substitution, no live database.
    let work = pstore
        .execute_external(
            "",
            "work.create",
            br#"{"schema_version":1,"event_id":"create","expected_revision":0,"title":"Reconcile native non-empty reader plan","objective":"Exact fictitious effect","scope":"Fixture","criteria":["No duplicate invoice"]}"#,
            &mut SystemEnvironment,
        )
        .unwrap()
        .result
        .work
        .id;
    pstore
        .trust(
            "config-apply",
            &serde_json::to_vec(&identity_config()).unwrap(),
            "",
        )
        .unwrap_or_else(|e| panic!("explicit fixture config refused: {}", e.code()));
    let scope = scope_of(&project_root, &work);
    {
        let ledger = RootStore::open(&ledger_root, true).unwrap();
        seed_settlement_effect(&pstore, &ledger, &scope);
        ledger.close().unwrap();
    }
    // The service Root gets its ledger path through the REAL CLI settings
    // contract of the provided binary (no direct state substitution); then
    // fixture identities + BLP policy on the SERVICE root through the
    // supported Trust operations (paired-close discipline on every path).
    run_settings_step(&binary, &service_root, &ledger_root);
    let trust = |op: &str, value: Value, token: &str| {
        trust_on_root(&service_root, op, value, token)
    };
    trust("config-apply", identity_config(), "");
    let blp_revision = trust("blp-show", json!({}), "fixture-admin-token")["record"]
        ["history"]
        .as_array()
        .and_then(|h| h.last())
        .and_then(|e| e["revision"].as_u64())
        .unwrap_or(0);
    let mut policy = native_reader_blp_policy();
    policy["event_id"] = json!("paea-native-nonempty-reader-policy");
    policy["expected_revision"] = json!(blp_revision);
    policy["expected_trust_revision"] = json!(1);
    policy["reason"] =
        json!("PA-09 native positive reader policy on the non-empty explicit ledger");
    trust("blp-policy-apply", policy, "fixture-admin-token");
    // Byte-witness S0 AFTER the intentional setup mutations (settings,
    // identities, policy, seed): the FULL four-file snapshot is compared
    // after every read phase below — the state databases must never move,
    // and the trust journals must not move until the single intentional
    // attribution.
    let (baseline_service_db, baseline_ledger_db, baseline_service_trust, baseline_ledger_trust) =
        journal_snapshot(&service_root, &ledger_root);
    // Raw-output recovery witness, declared BEFORE `listener` so Rust drops
    // it AFTER (shared struct, same wiring contract as the empty sibling):
    // on the failure path the listener Drop has already stopped and reaped
    // the child, so this witness's Drop joins BOTH drains to EOF and
    // conserves the COMPLETE stdout/stderr raw bytes plus the REAL observed
    // exit.
    let raw_capture = RawCapture::from_env();
    raw_capture.store("spawn.json", &raw_capture.spawn_bytes(&binary, &binary_sha, &service_root));
    let mut raw_witness = RawOutputWitness::default();
    raw_witness.capture = Some(RawCapture {
        dir: raw_capture.dir.clone(),
    });
    let (mut listener, stderr_buffer, stderr_errors, stderr_join, observed_exit) =
        spawn_web(&binary, &service_root);
    raw_witness.stderr = Arc::clone(&stderr_buffer);
    raw_witness.stderr_errors = Arc::clone(&stderr_errors);
    raw_witness.exit_slot = Some(Arc::clone(&observed_exit));
    raw_witness.wired_stderr = true;
    let (address, reader, stdout_retained, stdout_errors) = bind_announcement(
        listener.child.as_mut().expect("listener child not yet consumed"),
        &stderr_buffer,
    );
    raw_witness.stdout = Arc::clone(&stdout_retained);
    raw_witness.stdout_errors = Arc::clone(&stdout_errors);
    raw_witness.wired_stdout = true;
    raw_witness.reader = Some(reader);
    raw_witness.stderr_join = Some(stderr_join);
    let port: u16 = address.rsplit(':').next().unwrap().parse().unwrap();
    {
        use sha2::{Digest as _, Sha256};
        let exe_sha = format!("{:x}", Sha256::digest(fs::read(format!("/proc/{}/exe", listener.child.as_ref().expect("listener child not yet consumed").id())).unwrap()));
        assert_eq!(exe_sha, binary_sha, "the running process must be the provided binary");
    }
    let token = web_session_secret(&service_root, &address);
    let address_hash = {
        use sha2::{Digest as _, Sha256};
        format!("{:x}", Sha256::digest(address.as_bytes()))
    };
    let cookie_name = format!("swarm_session_{}", &address_hash[..12]);
    let cookie = format!("{}={}", cookie_name, token);
    // 1. BARE session (no Bearer credential): typed authentication refusal,
    //    no report byte, HEAD empty.
    {
        let (status, body) = raw_http(port, "GET", RECONCILE_REPORT, Some(&cookie), None);
        assert_eq!(status, 403, "bare read must be refused: {}", String::from_utf8_lossy(&body));
        assert!(
            String::from_utf8_lossy(&body).contains("paea-reader-authentication-required"),
            "bare refusal must carry the typed code"
        );
        assert!(
            !String::from_utf8_lossy(&body).contains("consistent"),
            "bare refusal must not disclose report states"
        );
        let (head_status, head_body) = raw_http(port, "HEAD", RECONCILE_REPORT, Some(&cookie), None);
        assert_eq!(head_status, 403);
        assert!(head_body.is_empty(), "HEAD refusal must have an empty body");
    }
    // 2. Known fixture reader BEFORE attribution: typed attribution refusal,
    //    no report byte (clearance alone invents no provenance).
    {
        let (status, body) = raw_http(port, "GET", RECONCILE_REPORT, Some(&cookie), Some("fixture-admin-token"));
        assert_eq!(status, 403, "unattributed read must be refused: {}", String::from_utf8_lossy(&body));
        assert!(
            String::from_utf8_lossy(&body).contains("paea-reader-attribution-required"),
            "unattributed refusal must carry the typed code"
        );
        assert!(
            !String::from_utf8_lossy(&body).contains("consistent"),
            "unattributed refusal must not disclose report states"
        );
    }
    // Full snapshot S1 compared against S0: the PRE-ATTRIBUTION refusal
    // phase (bare 403, HEAD, known-before-attribution 403) must already have
    // moved nothing.
    {
        let (s_db, l_db, s_trust, l_trust) = journal_snapshot(&service_root, &ledger_root);
        assert_eq!(s_db, baseline_service_db, "service state.db mutated by pre-attribution refusals");
        assert_eq!(l_db, baseline_ledger_db, "ledger state.db mutated by pre-attribution refusals");
        assert_eq!(s_trust, baseline_service_trust, "service trust journal mutated by pre-attribution refusals");
        assert_eq!(l_trust, baseline_ledger_trust, "ledger trust journal mutated by pre-attribution refusals");
    }
    // 3. The EXACT expected NON-EMPTY report via the public producer on the
    //    real roots: settlement Consistent with EXACTLY one real verified
    //    finding, git EXPLICITLY Uninitialized, canonical encoding, digest
    //    in the SAME domain the route recomputes.
    let (expected, real_intent, real_digest) = {
        let service = RootStore::open(&service_root, false).unwrap();
        let ledger = RootStore::open(&ledger_root, false).unwrap();
        let report = service.paea_reconcile_effect_journals(&ledger).unwrap();
        ledger.close().unwrap();
        service.close().unwrap();
        assert_eq!(
            report.schema_version, 1,
            "the non-empty report must carry the current schema"
        );
        let PaeaEffectReconcileJournal::Consistent { effect_count: 1, findings } = &report.settlement
        else {
            panic!("expected a non-empty settlement journal, got {:?}", report.settlement);
        };
        assert_eq!(
            findings.len(),
            1,
            "exactly one real settlement effect must be reconciled: {:?}",
            findings
        );
        assert!(
            findings[0].verified,
            "unexpected refusals: {:?}",
            findings[0].refusals
        );
        assert_eq!(
            report.git,
            PaeaEffectReconcileJournal::Uninitialized,
            "git journal must be explicitly Uninitialized (no git schema in this fixture)"
        );
        let real_intent = findings[0].intent_id.clone();
        let real_digest = findings[0].action_digest.clone();
        let encoded = swarm_engine::encode_model(&report).unwrap();
        (encoded, real_intent, real_digest)
    };
    let report_digest = {
        use sha2::{Digest as _, Sha256};
        format!("{:x}", Sha256::digest(&expected))
    };
    // The attribution binds the recomputed digest of the NON-EMPTY report,
    // the REAL SERVICE root (the root whose journals the report reads and
    // whose Trust state the reader verdict consults — never the ledger or
    // project root) and the current policy revision.
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
            "event_id":"paea-native-nonempty-reader-attribute",
            "expected_revision":attribute_revision,
            "expected_trust_revision":1,
            "canonical_digest":report_digest,
            "project_path":service_root.to_str().unwrap(),
            "classification":{"level":"classified","compartments":[]},
            "reason":"PA-09 native attribution of the exact non-empty report",
        }),
        "fixture-admin-token",
    );
    assert_eq!(attribute_reply["attributed"], true, "supported producer must attribute");
    // The attribution is the LAST intentional Trust write of the setup: the
    // full read-sequence baseline S2 is retaken here (the journals
    // legitimately moved across the attribution), then the reads must move
    // nothing.
    let (before_service_db, before_ledger_db, before_service_trust, before_ledger_trust) =
        journal_snapshot(&service_root, &ledger_root);
    // 4. POSITIVE read through the real binary: served bytes EXACTLY the
    //    attributed encoding; JSON fields asserted from the REAL model (no
    //    invented JSON, no counter alone); HEAD mirrors empty 200.
    {
        let (status, body) = raw_http(port, "GET", RECONCILE_REPORT, Some(&cookie), Some("fixture-admin-token"));
        assert_eq!(status, 200, "positive read refused: {}", String::from_utf8_lossy(&body));
        assert_eq!(body, expected, "served bytes must equal the attributed encoding exactly");
        let body_json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body_json["schema_version"], 1);
        let settlement = &body_json["settlement"];
        assert_eq!(settlement["state"], "consistent");
        assert_eq!(settlement["effectCount"], 1);
        assert_eq!(settlement["findings"][0]["intent_id"], real_intent);
        assert_eq!(settlement["findings"][0]["action_digest"], real_digest);
        assert_eq!(settlement["findings"][0]["verified"], true);
        assert_eq!(body_json["git"]["state"], "uninitialized");
        let (head_status, head_body) = raw_http(port, "HEAD", RECONCILE_REPORT, Some(&cookie), Some("fixture-admin-token"));
        assert_eq!(head_status, 200);
        assert!(head_body.is_empty(), "HEAD mirror must be an empty 200");
    }
    // 5. UNKNOWN credential once Trust is configured: typed authorization
    //    refusal, non-success, NO report byte (no real identifier, no state
    //    disclosure).
    {
        let (status, body) = raw_http(port, "GET", RECONCILE_REPORT, Some(&cookie), Some("unknown-reader-token"));
        assert_eq!(status, 403, "unknown credential must be refused: {}", String::from_utf8_lossy(&body));
        assert!(
            String::from_utf8_lossy(&body).contains("authorization_required"),
            "unknown refusal must carry the typed code"
        );
        let leaked = String::from_utf8_lossy(&body);
        assert!(
            !leaked.contains(&real_intent) && !leaked.contains(&real_digest),
            "unknown refusal must not disclose report identifiers"
        );
        assert!(
            !leaked.contains("consistent"),
            "unknown refusal must not disclose report states"
        );
        // Full snapshot S3 compared against the post-attribution baseline S2:
        // the positive read + HEAD + unknown refusal moved nothing.
        let (s_db, l_db, s_trust, l_trust) = journal_snapshot(&service_root, &ledger_root);
        assert_eq!(s_db, before_service_db, "service state.db mutated by positive/unknown reads");
        assert_eq!(l_db, before_ledger_db, "ledger state.db mutated by positive/unknown reads");
        assert_eq!(s_trust, before_service_trust, "service trust journal mutated by positive/unknown reads");
        assert_eq!(l_trust, before_ledger_trust, "ledger trust journal mutated by positive/unknown reads");
    }
    // 6. RECOVERY: the known reader reads 200 the same exact bytes — the
    //    refusals were identity/attribution-state-driven, channel intact.
    {
        let (status, body) = raw_http(port, "GET", RECONCILE_REPORT, Some(&cookie), Some("fixture-admin-token"));
        assert_eq!(status, 200, "recovery read refused: {}", String::from_utf8_lossy(&body));
        assert_eq!(body, expected, "recovered read must serve the same exact bytes");
    }
    // 6b. TEST-ONLY positive browser witness (armed by
    //     SWARM_PAEA_NATIVE_BROWSER_WITNESS=1, validated BEFORE spawn): the
    //     public attribution is done and the listener is STILL ALIVE here —
    //     one real Chromium run replays the whole reader frontier on the
    //     NON-EMPTY report through the REAL served page (consistent
    //     settlement render with the real finding line, same-page unknown
    //     purge of the REAL identifiers, identical recovery) and stores its
    //     own evidence under the shared capture dir. Nominal runs (env
    //     absent) skip this entirely — the qualified API-only non-empty
    //     sequence is unchanged bit-for-bit. The browser issues GET reads
    //     only, so the phase-9 snapshot discipline still covers it (the
    //     witness runs between two snapshot comparisons, reads never mutate
    //     any root).
    if browser_witness {
        run_browser_witness(
            &raw_capture,
            &binary,
            &binary_sha,
            &address,
            &cookie_name,
            &token,
            "fixture-admin-token",
            "unknown-reader-token",
            &expected,
        );
    }
    // 7. Stop the OWN child with SIGINT and assert the REAL shutdown
    //    contract (same as the empty sibling): exit code 0, last retained
    //    stdout line is the `web-stopped` stop-record.
    let exit = listener.sigint_and_wait();
    let stderr_cause = String::from_utf8_lossy(&stderr_buffer
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()))
    .into_owned();
    assert_eq!(
        exit.code(),
        Some(0),
        "native listener must exit cleanly (0) on SIGINT per the native shutdown contract, got {exit}; stderr: {stderr_cause}"
    );
    // Join BOTH drain threads now that the child is stopped (handles taken
    // back from the raw-output witness), then hard-fail on any drain error
    // state (the raw buffers stay exact child bytes).
    let reader = raw_witness.reader.take().expect("stdout reader handle held by the witness");
    let stderr_join = raw_witness.stderr_join.take().expect("stderr drain handle held by the witness");
    reader.join().expect("stdout reader thread must join");
    stderr_join.join().expect("stderr drain thread must join");
    let stdout_drain_errors = stdout_errors
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    assert!(
        stdout_drain_errors.is_empty(),
        "stdout drain must reach EOF without error on the success path: {}",
        String::from_utf8_lossy(&stdout_drain_errors)
    );
    let stderr_drain_errors = stderr_errors
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    assert!(
        stderr_drain_errors.is_empty(),
        "stderr drain must reach EOF without error on the success path: {}",
        String::from_utf8_lossy(&stderr_drain_errors)
    );
    let stdout_final = String::from_utf8_lossy(&stdout_retained
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()))
    .into_owned();
    // Durable conservation of the COMPLETE raw child outputs (SUCCESS path,
    // same artifacts as the empty sibling): exact stdout/stderr bytes, the
    // separate drain-error states (asserted empty above), the REAL observed
    // exit read from the shared slot, spawn metadata already stored. A
    // conservation failure panics with its real cause (no fake PASS).
    let stdout_bytes = stdout_retained
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    raw_capture.store("stdout.log", &stdout_bytes);
    raw_capture.store("drain-stdout.errors.log", &stdout_drain_errors);
    let stderr_bytes = stderr_buffer
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    raw_capture.store("stderr.log", &stderr_bytes);
    raw_capture.store("drain-stderr.errors.log", &stderr_drain_errors);
    let observed_exit = observed_exit
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    assert_eq!(
        observed_exit.as_ref().map(|e| e.code()),
        Some(exit.code()),
        "the shared exit slot must hold the observed exit equal to the awaited one"
    );
    raw_capture.store(
        "exit.json",
        &serde_json::to_vec(&json!({
            "observed": true,
            "exit": {
                "exit_code": exit.code(),
                "success": exit.success(),
                "status": format!("{exit}"),
            },
        }))
        .expect("exit metadata must serialize"),
    );
    assert!(
        stdout_final.lines().last().is_some_and(|line| line.contains("\"web-stopped\"")),
        "the last retained stdout line must be the web-stopped stop-record per the native shutdown contract; stdout tail: {:?}",
        stdout_final
            .lines()
            .last()
            .map(|line| line.chars().take(120).collect::<String>())
    );
    // 8. Byte-witness AFTER the whole read sequence AND after the child
    //    stop: the state databases never moved (compared to the S0
    //    pre-listener baseline) and the trust journals only moved across the
    //    explicit attribution (compared to the S2 post-attribution snapshot)
    //    — the witness never mutated any root during the reads.
    let (after_service_db, after_ledger_db, after_service_trust, after_ledger_trust) =
        journal_snapshot(&service_root, &ledger_root);
    assert_eq!(after_service_db, baseline_service_db, "service state.db mutated");
    assert_eq!(after_ledger_db, baseline_ledger_db, "ledger state.db mutated");
    assert_eq!(after_service_trust, before_service_trust, "service trust journal mutated by reads");
    assert_eq!(after_ledger_trust, before_ledger_trust, "ledger trust journal mutated");
    // Release the open project store, then cleanup covers ONLY these three
    // owned fixture roots (failed runs keep them for inspection, mirroring
    // the received Fixture Drop discipline).
    pstore.close().unwrap();
    if !std::thread::panicking() {
        let _ = fs::remove_dir_all(&service_root);
        let _ = fs::remove_dir_all(&ledger_root);
        let _ = fs::remove_dir_all(&project_root);
    }
}

/// The exact child-controlled stream text that the REMOVED implementation
/// used to treat as a drain-error marker: a stream may legitimately contain
/// this very line without the producer ever interpreting or truncating it.
/// Inputs/Effects: none (constants). Errors: none.
const MARKER_LIKE_CHILD_LINE: &[u8] = b"\n[stdout drain error: injected-by-child]";
/// Stderr twin of `MARKER_LIKE_CHILD_LINE` (same contract). Errors: none.
const MARKER_LIKE_CHILD_LINE_STDERR: &[u8] = b"\n[stderr drain error: injected-by-child]";

/// A pure in-memory Read source replaying EXACT fixed bytes (possibly
/// non-UTF-8), used by the memory guards to exercise the SHARED producer
/// loop `drain_to_end` — the same code the real stderr drain runs — without
/// spawning any process. Errors: none at construction. Effects: none.
struct FixedSource {
    bytes: Vec<u8>,
    pos: usize,
}
impl FixedSource {
    /// Replays `bytes` exactly, then yields EOF. Errors: none. Effects:
    /// none.
    fn new(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.to_vec(),
            pos: 0,
        }
    }
}
impl std::io::Read for FixedSource {
    /// Copies the remaining bytes chunk by chunk (small chunks to cross
    /// several loop iterations), then reports EOF. Errors: none. Effects:
    /// advances the replay position.
    fn read(&mut self, buf: &mut [u8]) -> stdio_io::Result<usize> {
        if self.pos >= self.bytes.len() {
            return Ok(0);
        }
        let n = buf.len().min(7).min(self.bytes.len() - self.pos);
        buf[..n].copy_from_slice(&self.bytes[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

/// MEMORY GUARD (opt-in, #[ignore]: nothing spawned, nothing written) on the
/// producer contract "the raw buffer keeps ALL child bytes, even when the
/// child itself emits text equal to the retired drain-error markers": the
/// shared `drain_to_end` loop is fed a stream containing, verbatim, both
/// marker-like lines SPLIT ACROSS read chunks (plus a non-UTF-8 byte and a
/// trailing line without newline), and must conserve them ALL in the raw
/// buffer (byte-exact) while the separate error state stays EMPTY — no
/// content search, no truncation, no interpretation. Inputs: none. Output:
/// pass (the assertions are the contract). Errors: assertion failures.
/// Effects: none (pure memory).
#[test]
#[ignore = "memory guard for the drain-producer contract; run with --ignored --exact"]
fn drain_producer_keeps_child_bytes_matching_retired_markers() {
    let mut child_stream = Vec::new();
    child_stream.extend_from_slice(b"line one\n");
    // The marker-like line, split across read chunks by the tiny FixedSource
    // chunks: even a chunk-boundary occurrence must not be interpreted.
    child_stream.extend_from_slice(MARKER_LIKE_CHILD_LINE);
    child_stream.extend_from_slice(b"\nnon-utf8 byte: \xff tail no-newline");
    let sink = Arc::new(Mutex::new(Vec::new()));
    let errors = Arc::new(Mutex::new(Vec::new()));
    drain_to_end(
        FixedSource::new(&child_stream),
        &sink,
        &errors,
        "stdout",
    );
    let raw = sink
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    let recorded = errors
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    assert_eq!(
        raw, child_stream,
        "the raw buffer must keep ALL child bytes, including marker-like text split \
         across chunks and non-UTF-8 bytes"
    );
    assert!(
        recorded.is_empty(),
        "no drain error may be recorded for child-controlled marker-like content: {}",
        String::from_utf8_lossy(&recorded)
    );
}

/// MEMORY GUARD (opt-in, #[ignore]: nothing spawned, nothing written) on the
/// producer contract "a REAL read error goes ONLY to the separate state":
/// a source that yields exact bytes (including the marker-like text) and
/// THEN fails with a real `std::io::Error` must leave the raw buffer byte-exact
/// (the error is never appended to it, in any form) and record the cause in
/// the separate error state with the stream label. Inputs: none. Output:
/// pass. Errors: assertion failures. Effects: none (pure memory).
#[test]
#[ignore = "memory guard for the drain-producer contract; run with --ignored --exact"]
fn drain_producer_real_error_goes_to_separate_state_only() {
    struct ErrorsAtEnd {
        bytes: Vec<u8>,
        pos: usize,
    }
    impl std::io::Read for ErrorsAtEnd {
        fn read(&mut self, buf: &mut [u8]) -> stdio_io::Result<usize> {
            if self.pos >= self.bytes.len() {
                return Err(stdio_io::Error::other("fixture read failure"));
            }
            let n = buf.len().min(7).min(self.bytes.len() - self.pos);
            buf[..n].copy_from_slice(&self.bytes[self.pos..self.pos + n]);
            self.pos += n;
            Ok(n)
        }
    }
    let mut child_stream = Vec::new();
    child_stream.extend_from_slice(b"good bytes\n");
    child_stream.extend_from_slice(MARKER_LIKE_CHILD_LINE_STDERR);
    child_stream.extend_from_slice(b"\xc3\x28"); // invalid UTF-8 sequence
    let sink = Arc::new(Mutex::new(Vec::new()));
    let errors = Arc::new(Mutex::new(Vec::new()));
    drain_to_end(
        ErrorsAtEnd {
            bytes: child_stream.clone(),
            pos: 0,
        },
        &sink,
        &errors,
        "stderr",
    );
    let raw = sink
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    let recorded = errors
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    assert_eq!(
        raw, child_stream,
        "the raw buffer must stay byte-exact child bytes even after a real read error"
    );
    assert!(
        !raw.windows(b"fixture read failure".len())
            .any(|w| w == b"fixture read failure"),
        "the real fixture error annotation must never enter the raw buffer: {}",
        String::from_utf8_lossy(&raw)
    );
    let recorded_text = String::from_utf8_lossy(&recorded).into_owned();
    assert!(
        recorded_text.starts_with("stderr drain error: ") && recorded_text.contains("fixture read failure"),
        "the separate state must carry the real error cause with its stream label: {recorded_text}"
    );
}

/// MEMORY GUARD (opt-in, #[ignore]: nothing spawned, nothing written) on the
/// observer contract "the shared exit slot distinguishes a real observed
/// exit from a faked one": a `Kill`-shaped slot stays `None` until a reap
/// records into it, and `Kill::observed_exit_slot` starts empty — the
/// failure witness must see `None` (never a fabricated success) when the
/// child could not be reaped. Exercises only the shared slot allocation and
/// its None-initialization (the reap recording itself is covered by the
/// native witness). Inputs: none. Output: pass. Errors: assertion failures.
/// Effects: none (pure memory).
#[test]
#[ignore = "memory guard for the observed-exit slot contract; run with --ignored --exact"]
fn observed_exit_slot_starts_none_and_never_fakes_success() {
    let slot = Kill::observed_exit_slot();
    let observed = slot
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    assert!(
        observed.is_none(),
        "a fresh shared exit slot must be None (no faked success): {observed:?}"
    );
}
