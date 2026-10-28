#![cfg(target_os = "linux")]
//! Browser bootstrap contracts on private Roots; typed HTTP, not browser/TCP proof.
use http::{
    header::{ACCEPT, CONTENT_TYPE, COOKIE, HOST, LOCATION, ORIGIN, SET_COOKIE},
    Request, StatusCode,
};
use std::{fs, net::TcpListener, os::unix::fs::PermissionsExt, path::PathBuf};
use swarm_engine::{EngineSettingsChange, RootStore, SettingCap, SystemEnvironment};
use swarm_web::{HttpReply, NativeHttpService};
struct Fixture {
    root: PathBuf,
    listener: TcpListener,
    key: String,
    host: String,
    service: Option<NativeHttpService>,
}
impl Fixture {
    /// Initialize one private Root, persist cookie policy through preview/apply, then bind.
    fn new(age: Option<i64>) -> Self {
        let root =
            std::env::temp_dir().join(swarm_engine::generate_identifier("login98-").unwrap());
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let store = RootStore::initialize(&root, &mut SystemEnvironment).unwrap();
        let d = store.settings().unwrap();
        let mut values = d.values;
        values.web_cookie_max_age_seconds = Some(SettingCap(age.map(swarm_engine::SettingCount)));
        let mut change = EngineSettingsChange {
            schema_version: d.schema_version,
            event_id: "cookie-fixture".into(),
            expected_revision: d.revision,
            values,
            reason: "Cookie consumer fixture".into(),
            preview_token: String::new(),
        };
        change.preview_token = store.preview_settings(&change).unwrap().preview_token;
        store
            .apply_settings(&change, &mut SystemEnvironment)
            .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let host = listener.local_addr().unwrap().to_string();
        let key = store.web_session(&listener).unwrap().secret().to_owned();
        store.close().unwrap();
        let service = Some(NativeHttpService::open_local(&root, &listener).unwrap());
        Self {
            root,
            listener,
            key,
            host,
            service,
        }
    }
    /// A real scoped request; login omits cookie/CSRF but requires its own Origin check.
    fn request(
        &self,
        method: &str,
        path: &str,
        body: Vec<u8>,
        origin: Option<&str>,
        cookie: Option<&str>,
        json: bool,
    ) -> HttpReply {
        let mut b = Request::builder()
            .method(method)
            .uri(path)
            .header(HOST, &self.host)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded");
        if let Some(origin) = origin {
            b = b.header(ORIGIN, origin)
        }
        if let Some(cookie) = cookie {
            b = b.header(COOKIE, cookie)
        }
        if json {
            b = b.header(ACCEPT, "application/json")
        }
        self.service
            .as_ref()
            .unwrap()
            .handle(b.body(body).unwrap())
            .unwrap()
    }
    /// Explicitly close binding; preserve failed Root and never touch another process.
    fn close(&mut self) {
        self.service.take().unwrap().close().unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(s) = self.service.take() {
            s.close().unwrap();
        }
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.root).unwrap();
        }
    }
}
/// Same page/assets in FR/EN and both themes, no key disclosure or API exception.
#[test]
fn native_login_public_page_assets_keep_host_and_private_api_gates() {
    let f = Fixture::new(Some(3600));
    for lang in ["fr", "en"] {
        for theme in ["etat", "sombre"] {
            let r = f.request(
                "GET",
                &format!("/login?lang={lang}&theme={theme}"),
                vec![],
                None,
                None,
                false,
            );
            assert_eq!(r.response.status(), StatusCode::OK);
            let html = std::str::from_utf8(r.response.body()).unwrap();
            assert!(html.contains(if lang == "en" {
                "Sign in to Swarm"
            } else {
                "Connexion à Swarm"
            }));
            assert!(html.contains(&format!("data-theme=\"{theme}\"")));
            assert!(!html.contains(&f.key));
            assert!(!html.contains("{{"));
            assert!(r.response.headers().get(SET_COOKIE).is_none());
        }
    }
    for (path, kind) in [
        ("/access.css", "text/css"),
        ("/access.js", "application/javascript"),
        ("/wattson_themes.css", "text/css"),
        ("/swarm-design.css", "text/css"),
        ("/swarm-logo.png", "image/png"),
        ("/favicon.ico", "image/png"),
    ] {
        for method in ["GET", "HEAD"] {
            let r = f.request(method, path, vec![], None, None, false);
            assert_eq!(r.response.status(), StatusCode::OK);
            assert!(r.response.headers()[CONTENT_TYPE]
                .to_str()
                .unwrap()
                .starts_with(kind));
            assert_eq!(r.response.body().is_empty(), method == "HEAD");
        }
    }
    let bad = Request::builder()
        .uri("/access.css")
        .header(HOST, "evil.test")
        .body(vec![])
        .unwrap();
    assert_eq!(
        f.service
            .as_ref()
            .unwrap()
            .handle(bad)
            .unwrap()
            .response
            .status(),
        StatusCode::FORBIDDEN
    );
    for path in ["/api/v1/works", "/api/v1/native-settings", "/cockpit.js"] {
        assert_eq!(
            f.request("GET", path, vec![], None, None, false)
                .response
                .status(),
            StatusCode::FORBIDDEN
        );
    }
}
/// Actual saved cookie age and scoped name survive restart; next never escapes origin.
#[test]
fn native_login_saved_cookie_policy_restart_and_redirects_are_real_consumers() {
    for age in [Some(3600), None] {
        let mut f = Fixture::new(age);
        let before = fs::read(f.root.join(".swarm/engine-settings.json")).unwrap();
        let origin = format!("http://{}", f.host);
        let form = form_urlencoded::Serializer::new(String::new())
            .append_pair("token", &f.key)
            .append_pair("next", "/?work=w-example&lang=en")
            .finish();
        let r = f.request(
            "POST",
            "/login",
            form.into_bytes(),
            Some(&origin),
            None,
            false,
        );
        assert_eq!(r.response.status(), StatusCode::SEE_OTHER);
        assert_eq!(r.response.headers()[LOCATION], "/?work=w-example&lang=en");
        let raw = r.response.headers()[SET_COOKIE]
            .to_str()
            .unwrap()
            .to_owned();
        let cookie = cookie::Cookie::parse(raw.clone()).unwrap();
        assert_eq!(cookie.max_age().map(|v| v.whole_seconds()), age);
        assert!(cookie.http_only().unwrap());
        assert_eq!(cookie.same_site(), Some(cookie::SameSite::Strict));
        assert_eq!(cookie.path(), Some("/"));
        let header = format!("{}={}", cookie.name(), cookie.value());
        assert_eq!(
            f.request("GET", "/api/v1/works", vec![], None, Some(&header), false)
                .response
                .body(),
            b"[]"
        );
        f.close();
        f.service = Some(NativeHttpService::open_local(&f.root, &f.listener).unwrap());
        assert_eq!(
            f.request("GET", "/api/v1/works", vec![], None, Some(&header), false)
                .response
                .body(),
            b"[]"
        );
        for next in [
            "https://evil.test/",
            "//evil.test/",
            "/\\evil.test/",
            "",
            "/%0d%0aLocation:evil",
            "/safe\r\nLocation:evil",
        ] {
            let form = form_urlencoded::Serializer::new(String::new())
                .append_pair("token", &f.key)
                .append_pair("next", next)
                .finish();
            let r = f.request(
                "POST",
                "/login",
                form.into_bytes(),
                Some(&origin),
                None,
                true,
            );
            assert_eq!(r.response.status(), StatusCode::OK);
            let value: serde_json::Value = serde_json::from_slice(r.response.body()).unwrap();
            assert_eq!(value["redirect"], "/");
        }
        assert_eq!(
            fs::read(f.root.join(".swarm/engine-settings.json")).unwrap(),
            before
        );
    }
}
/// A valid login precedes each rejection family; invalid input cannot set a cookie.
#[test]
fn native_login_wrong_origin_key_and_ambiguous_forms_refuse_without_mutation() {
    let f = Fixture::new(Some(3600));
    let origin = format!("http://{}", f.host);
    let good = format!("token={}&next=%2F", f.key);
    assert_eq!(
        f.request(
            "POST",
            "/login",
            good.as_bytes().to_vec(),
            Some(&origin),
            None,
            true
        )
        .response
        .status(),
        StatusCode::OK
    );
    let saved = fs::read(f.root.join(".swarm/engine-settings.json")).unwrap();
    let duplicated = Request::builder()
        .method("POST")
        .uri("/login")
        .header(HOST, &f.host)
        .header(ORIGIN, &origin)
        .header(ORIGIN, &origin)
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(good.as_bytes().to_vec())
        .unwrap();
    let refused = f.service.as_ref().unwrap().handle(duplicated).unwrap();
    assert_eq!(refused.response.status(), StatusCode::FORBIDDEN);
    assert!(refused.response.headers().get(SET_COOKIE).is_none());
    for supplied in [None, Some("http://evil.test")] {
        let r = f.request(
            "POST",
            "/login",
            good.as_bytes().to_vec(),
            supplied,
            None,
            true,
        );
        assert_eq!(r.response.status(), StatusCode::FORBIDDEN);
        assert!(r.response.headers().get(SET_COOKIE).is_none());
    }
    for body in [
        b"token=wrong&next=%2F".to_vec(),
        format!("token={}&token={}", f.key, f.key).into_bytes(),
        format!("token={}&next=%2F&next=%2Fevil", f.key).into_bytes(),
        b"token=%GG".to_vec(),
        b"token=%FF".to_vec(),
    ] {
        let r = f.request("POST", "/login?lang=en", body, Some(&origin), None, true);
        assert_eq!(r.response.status(), StatusCode::UNAUTHORIZED);
        assert!(r.response.headers().get(SET_COOKIE).is_none());
        let raw = std::str::from_utf8(r.response.body()).unwrap();
        assert!(!raw.contains(&f.key));
        assert!(raw.contains("Access key rejected"));
    }
    assert_eq!(
        fs::read(f.root.join(".swarm/engine-settings.json")).unwrap(),
        saved
    );
}
