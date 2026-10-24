#![cfg(target_os = "linux")]
//! Private typed bootstrap/asset boundary, not a native browser or global cockpit proof.
use http::{
    header::{CONTENT_TYPE, COOKIE, HOST, ORIGIN},
    Request, StatusCode,
};
use std::{fs, net::TcpListener, os::unix::fs::PermissionsExt, path::PathBuf};
use swarm_engine::{RootStore, SystemEnvironment};
use swarm_web::{HttpReply, LocalWebAuthority, NativeHttpService};
struct Fixture {
    root: PathBuf,
    service: Option<NativeHttpService>,
    host: String,
    cookie: String,
    key: String,
}
impl Fixture {
    /// Own one private Root and scoped persisted session; never select live data.
    fn new() -> Self {
        let root = std::env::temp_dir()
            .join(swarm_engine::generate_identifier("native-admin99-").unwrap());
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let r = RootStore::initialize(&root, &mut SystemEnvironment).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let key = r.web_session(&listener).unwrap().secret().to_owned();
        r.close().unwrap();
        let a = LocalWebAuthority::from_listener(&listener, &key).unwrap();
        let host = a.host().to_owned();
        let cookie = format!("{}={key}", a.cookie_name());
        let service = Some(NativeHttpService::open_local(&root, &listener).unwrap());
        Self {
            root,
            service,
            host,
            cookie,
            key,
        }
    }
    /// Actual typed service authorization; only explicit caller cookie is attached.
    fn get(&self, method: &str, path: &str, authorized: bool) -> HttpReply {
        let mut b = Request::builder()
            .method(method)
            .uri(path)
            .header(HOST, &self.host);
        if authorized {
            b = b.header(COOKIE, &self.cookie)
        }
        self.service
            .as_ref()
            .unwrap()
            .handle(b.body(vec![]).unwrap())
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(s) = self.service.take() {
            s.close().unwrap()
        }
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.root).unwrap();
        }
    }
}
/// A valid API witness precedes native landing checks; exact asset allowlist only.
#[test]
fn native_admin_landing_and_assets_are_protected_without_secret_or_path_fallback() {
    let f = Fixture::new();
    assert_eq!(f.get("GET", "/api/v1/works", true).response.body(), b"[]");
    let before = fs::read(f.root.join(".swarm/engine-settings.json")).unwrap();
    for lang in ["fr", "en"] {
        for theme in ["etat", "sombre"] {
            let r = f.get("GET", &format!("/?lang={lang}&theme={theme}"), true);
            assert_eq!(r.response.status(), StatusCode::OK);
            assert!(r.response.headers()[CONTENT_TYPE]
                .to_str()
                .unwrap()
                .starts_with("text/html"));
            let html = std::str::from_utf8(r.response.body()).unwrap();
            assert!(html.contains(&format!("lang=\"{lang}\"")));
            assert!(html.contains(&format!("data-theme=\"{theme}\"")));
            assert!(html.contains("/native-runtime/app.js"));
            assert!(!html.contains(&f.key));
        }
    }
    for (path, kind) in [
        ("/native-runtime/base.css", "text/css"),
        ("/native-runtime/admin.css", "text/css"),
        ("/native-runtime/app.css", "text/css"),
        ("/native-runtime/english.js", "application/javascript"),
        ("/native-runtime/i18n.js", "application/javascript"),
        ("/native-runtime/settings.js", "application/javascript"),
        ("/native-runtime/app.js", "application/javascript"),
    ] {
        for method in ["GET", "HEAD"] {
            let r = f.get(method, path, true);
            assert_eq!(r.response.status(), StatusCode::OK);
            assert!(r.response.headers()[CONTENT_TYPE]
                .to_str()
                .unwrap()
                .starts_with(kind));
            assert_eq!(r.response.body().is_empty(), method == "HEAD");
            assert_eq!(
                f.get(method, path, false).response.status(),
                StatusCode::FORBIDDEN
            );
        }
    }
    let guest = f.get("GET", "/", false);
    assert_eq!(guest.response.status(), StatusCode::OK);
    let html = std::str::from_utf8(guest.response.body()).unwrap();
    assert!(html.contains("Connexion à Swarm"));
    assert!(!html.contains("/native-runtime/app.js"));
    assert!(!html.contains(&f.key));
    assert_eq!(
        f.get("GET", "/native-runtime.html", false)
            .response
            .status(),
        StatusCode::FORBIDDEN
    );
    let ambiguous = Request::builder()
        .uri("/")
        .header(HOST, &f.host)
        .header(COOKIE, format!("{}; {}", f.cookie, f.cookie))
        .body(vec![])
        .unwrap();
    assert_eq!(
        f.service
            .as_ref()
            .unwrap()
            .handle(ambiguous)
            .unwrap()
            .response
            .status(),
        StatusCode::FORBIDDEN
    );
    for path in [
        "/native-runtime/../Cargo.toml",
        "/native-runtime/missing.js",
    ] {
        assert_eq!(
            f.get("GET", path, true).response.status(),
            StatusCode::NOT_FOUND
        );
    }
    assert_eq!(
        fs::read(f.root.join(".swarm/engine-settings.json")).unwrap(),
        before
    );
}
/// Host and CSRF gates precede unsupported methods; valid CSRF never mutates assets.
#[test]
fn native_admin_assets_keep_host_csrf_method_and_no_mutation_contract() {
    let f = Fixture::new();
    assert_eq!(
        f.get("GET", "/api/v1/works", true).response.status(),
        StatusCode::OK
    );
    let before = fs::read(f.root.join(".swarm/engine-settings.json")).unwrap();
    let wrong = Request::builder()
        .uri("/native-runtime/app.js")
        .header(HOST, "evil.test")
        .header(COOKIE, &f.cookie)
        .body(vec![])
        .unwrap();
    assert_eq!(
        f.service
            .as_ref()
            .unwrap()
            .handle(wrong)
            .unwrap()
            .response
            .status(),
        StatusCode::FORBIDDEN
    );
    let session = f.get("GET", "/api/v1/native-session", true);
    let value: serde_json::Value = serde_json::from_slice(session.response.body()).unwrap();
    for csrf in [None, Some(value["csrf_token"].as_str().unwrap())] {
        let mut b = Request::builder()
            .method("POST")
            .uri("/native-runtime/app.js")
            .header(HOST, &f.host)
            .header(COOKIE, &f.cookie)
            .header(ORIGIN, format!("http://{}", f.host));
        if let Some(csrf) = csrf {
            b = b.header("X-Swarm-CSRF", csrf)
        }
        let r = f
            .service
            .as_ref()
            .unwrap()
            .handle(b.body(vec![]).unwrap())
            .unwrap();
        assert_eq!(
            r.response.status(),
            if csrf.is_some() {
                StatusCode::METHOD_NOT_ALLOWED
            } else {
                StatusCode::FORBIDDEN
            }
        );
    }
    assert_eq!(
        fs::read(f.root.join(".swarm/engine-settings.json")).unwrap(),
        before
    );
}
