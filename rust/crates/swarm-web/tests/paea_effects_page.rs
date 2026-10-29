#![cfg(target_os = "linux")]
//! Embedded PA-08 effect-state page assets: exact presence under session,
//! real byte content (new page, hub link, English catalog entries) and the
//! ordinary access contract (no session refusal, GET/HEAD only). The page is
//! a read-only observation surface; no API behavior, consumption or authority
//! is re-tested here (covered by paea_reconcile.rs / paea_git_call.rs).
use http::{header::COOKIE, header::HOST, Request, StatusCode};
use std::{net::TcpListener, os::unix::fs::PermissionsExt, path::PathBuf};
use swarm_web::{HttpReply, LocalWebAuthority, NativeHttpService};

struct Fixture {
    root: PathBuf,
    listener: TcpListener,
    key: String,
}
impl Fixture {
    /// Real owned service Root in a 0700 temp directory with a session
    /// credential derived from the test listener.
    fn new() -> Self {
        let dir = std::env::temp_dir().join(swarm_engine::generate_identifier("paea-page94-").unwrap());
        std::fs::create_dir(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        let store = swarm_engine::RootStore::initialize(&dir, &mut swarm_engine::SystemEnvironment).unwrap();
        store.close().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let r = swarm_engine::RootStore::open(&dir, true).unwrap();
        let key = r.web_session(&listener).unwrap().secret().to_owned();
        r.close().unwrap();
        Self { root: dir, listener, key }
    }
}
impl Drop for Fixture {
    /// Preserve failed fixtures; remove only this owned successful root.
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }
}
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
/// The new page and its script are served with the right media types and the
/// page really references the two guarded read endpoints; no other asset set.
#[test]
fn paea_effects_page_served_with_content_and_hub_link() {
    let f = Fixture::new();
    let s = NativeHttpService::open_local(&f.root, &f.listener).unwrap();
    let page = get(&f, &s, "/paea-effects.html");
    assert_eq!(page.response.status(), StatusCode::OK, "{:?}", page.failure);
    let body = String::from_utf8(page.response.body().to_vec()).unwrap();
    assert!(body.contains("/api/v1/paea/git-call") || body.contains("paea-effects.js"));
    let script = get(&f, &s, "/paea-effects.js");
    assert_eq!(script.response.status(), StatusCode::OK, "{:?}", script.failure);
    let js = String::from_utf8(script.response.body().to_vec()).unwrap();
    assert!(js.contains("/api/v1/paea/git-call"), "script must consume the status endpoint");
    assert!(js.contains("/api/v1/paea/reconcile-effects"), "script must consume the reconciliation endpoint");
    assert!(!js.contains("method:'POST'"), "read-only page must never POST");
    // The hub really links the new page so the family navigation stays coherent.
    let hub = get(&f, &s, "/paea.html");
    assert_eq!(hub.response.status(), StatusCode::OK);
    assert!(
        String::from_utf8_lossy(hub.response.body()).contains("/paea-effects.html"),
        "hub must link the effect-state page"
    );
    s.close().unwrap();
}
/// The page keeps the ordinary access contract: no session refuses, and the
/// embedded English catalog really carries the new surface's translations.
#[test]
fn paea_effects_page_refuses_without_session_and_en_catalog_covers_it() {
    let f = Fixture::new();
    let s = NativeHttpService::open_local(&f.root, &f.listener).unwrap();
    let a = LocalWebAuthority::from_listener(&f.listener, &f.key).unwrap();
    let no_cookie = Request::builder()
        .method("GET")
        .uri("/paea-effects.html")
        .header(HOST, a.host())
        .body(vec![])
        .unwrap();
    let reply = s.handle(no_cookie).unwrap();
    assert_eq!(reply.response.status(), StatusCode::FORBIDDEN);
    let en = get(&f, &s, "/i18n-en.js");
    assert_eq!(en.response.status(), StatusCode::OK);
    let catalog = String::from_utf8_lossy(en.response.body());
    for french in [
        "États d'effets PAEA",
        "Rapprochement des journaux d'effets (PA-09)",
        "Journal non initialisé : aucun effet rapproché (état explicite, pas un succès vide).",
    ] {
        assert!(catalog.contains(french), "missing EN catalog source entry: {french}");
    }
    s.close().unwrap();
}
