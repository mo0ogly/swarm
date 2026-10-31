#![cfg(target_os = "linux")]
//! Embedded PAEA approver-consent page assets: exact presence under session,
//! real byte content (held-command consent form, hub link, English catalog
//! entries) and the ordinary access contract (no session refusal, GET).
//! Unlike the read-only effects page, this surface deliberately carries ONE
//! explicit POST — the approval channel with the Enrollment credential and an
//! EMPTY body, the frozen command being held server-side — so the script must
//! contain it and must NOT carry a command file input; no route behavior,
//! consumption or authority is re-tested here (covered by paea_git_call.rs
//! and the browser qualification recipe).
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
        let dir = std::env::temp_dir().join(swarm_engine::generate_identifier("paea-consent-").unwrap());
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
/// The consent page and its script are served with the right media types; the
/// script reads both guarded observations and the session CSRF token, and it
/// really carries the single approval POST with the Enrollment credential —
/// the deliberate opposite of the effects page's no-POST guard.
#[test]
fn paea_consent_page_served_with_consent_form_and_hub_link() {
    let f = Fixture::new();
    let s = NativeHttpService::open_local(&f.root, &f.listener).unwrap();
    let page = get(&f, &s, "/paea-consent.html");
    assert_eq!(page.response.status(), StatusCode::OK, "{:?}", page.failure);
    let body = String::from_utf8(page.response.body().to_vec()).unwrap();
    assert!(body.contains("paea-consent.js"), "page must load its script");
    // Held-command contract: NO file input — the frozen command stays
    // server-side; the operator submits only their enrolled password.
    assert!(body.contains("consent-password"), "page must hold the enrolled-password input");
    assert!(!body.contains("consent-command"), "held-command page must not carry a frozen-bytes file input");
    assert!(
        body.contains("La commande figée est détenue par le service"),
        "page must state the held-command contract to the operator"
    );
    let script = get(&f, &s, "/paea-consent.js");
    assert_eq!(script.response.status(), StatusCode::OK, "{:?}", script.failure);
    let js = String::from_utf8(script.response.body().to_vec()).unwrap();
    assert!(js.contains("/api/v1/paea/git-call/approval"), "script must consume the approval channel");
    assert!(js.contains("method:'POST'"), "consent page must carry the one explicit approval POST");
    assert!(js.contains("Uint8Array(0)"), "approval POST must send an EMPTY body under the held-command contract");
    assert!(js.contains("'Enrollment '"), "approval POST must carry the enrolled credential");
    assert!(js.contains("/api/v1/paea/git-call/preview"), "script must present the exact prepared action before consent");
    assert!(js.contains("/api/v1/native-session"), "script must read the session CSRF token");
    assert!(js.contains("trust_state_may_be_recorded"), "script must demand a state reread on possible recorded trust effects");
    // The hub really links the new page so the family navigation stays coherent.
    let hub = get(&f, &s, "/paea.html");
    assert_eq!(hub.response.status(), StatusCode::OK);
    assert!(
        String::from_utf8_lossy(hub.response.body()).contains("/paea-consent.html"),
        "hub must link the consent page"
    );
    s.close().unwrap();
}
/// The page keeps the ordinary access contract: no session refuses, and the
/// embedded English catalog really carries the new surface's translations.
#[test]
fn paea_consent_page_refuses_without_session_and_en_catalog_covers_it() {
    let f = Fixture::new();
    let s = NativeHttpService::open_local(&f.root, &f.listener).unwrap();
    let a = LocalWebAuthority::from_listener(&f.listener, &f.key).unwrap();
    let no_cookie = Request::builder()
        .method("GET")
        .uri("/paea-consent.html")
        .header(HOST, a.host())
        .body(vec![])
        .unwrap();
    let reply = s.handle(no_cookie).unwrap();
    assert_eq!(reply.response.status(), StatusCode::FORBIDDEN);
    let en = get(&f, &s, "/i18n-en.js");
    assert_eq!(en.response.status(), StatusCode::OK);
    let catalog = String::from_utf8_lossy(en.response.body());
    for french in [
        "Approbation de l’action préparée",
        "Consentement réel d’approbateur enrôlé : cette page exécute la commande Trust figée exacte préparée au démarrage, sur le Root projet lié, sous votre autorité enrôlée personnelle. Un refus reste un refus visible ; aucun succès n’est jamais fabriqué. Aucun appel Git n’est déclenché ici.",
        "La commande figée est détenue par le service : le navigateur ne transmet aucun octet de commande, uniquement votre mot de passe enrôlé. Vérifiez hors canal que l’empreinte affichée ci-dessus est bien celle de la commande que vous acceptez ; la route ne peut pas comparer à votre place un octet qu’elle ne reçoit pas.",
        "Aucune commande d’approbation n’est liée au démarrage du service. La préparation deux phases (octets figés + permission provisionnée) reste la voie réelle ; cette page ne prépare rien.",
        "Préparation périmée : la révision de confiance actuelle diffère de la révision déclarée. Aucun consentement n’est sollicité sur un état périmé ; reprendre la préparation avant toute suite.",
        "Refus enregistré par le moteur : la commande n’a pas produit d’accord. La raison et la suite indiquée s’affichent telles quelles ; relire l’état avant toute reprise.",
    ] {
        assert!(catalog.contains(french), "missing EN catalog source entry: {french}");
    }
    s.close().unwrap();
}
