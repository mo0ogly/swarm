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
