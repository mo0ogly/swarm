#![cfg(target_os = "linux")]
//! Native Admin API contracts against a real private Root, without TCP HTTP/browser claims.
use http::{
    header::{COOKIE, HOST, ORIGIN},
    Request, StatusCode,
};
use std::{fs, net::TcpListener, os::unix::fs::PermissionsExt, path::PathBuf};
use swarm_engine::{
    EngineSettingsChange, EngineSettingsDocument, RootStore, SettingCap, SettingCount,
    SystemEnvironment,
};
use swarm_web::{HttpReply, LocalWebAuthority, NativeHttpService};
struct Fixture {
    root: PathBuf,
    listener: TcpListener,
    key: String,
}
impl Fixture {
    /// Real owned Root and persisted session; all test data stays under this0700root.
    fn new() -> Self {
        let root = std::env::temp_dir()
            .join(swarm_engine::generate_identifier("settings-http93-").unwrap());
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let r = RootStore::initialize(&root, &mut SystemEnvironment).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let key = r.web_session(&listener).unwrap().secret().to_owned();
        r.close().unwrap();
        fs::write(root.join("report.md"), b"report").unwrap();
        Self {
            root,
            listener,
            key,
        }
    }
    /// Protect every typed request through real listener-scoped persisted bytes.
    fn request(&self, method: &str, path: &str, raw: Vec<u8>) -> Request<Vec<u8>> {
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
    }
    /// Reopen actual saved settings; this does not substitute a policy evaluator.
    fn document(&self) -> EngineSettingsDocument {
        let r = RootStore::open(&self.root, false).unwrap();
        let d = r.settings().unwrap();
        assert!(r.list_works().unwrap().is_empty());
        r.close().unwrap();
        d
    }
    /// Build one complete native proposal; preview token is deliberately not adopted yet.
    fn change(&self, event: &str, cap: i64) -> EngineSettingsChange {
        let d = self.document();
        let mut values = d.values;
        values.review_delivery_max_bytes = Some(SettingCap(Some(SettingCount(cap))));
        EngineSettingsChange {
            schema_version: d.schema_version,
            event_id: event.into(),
            expected_revision: d.revision,
            values,
            reason: "HTTP saved consumer proof".into(),
            preview_token: String::new(),
        }
    }
    /// Observe exact bytes for no-op/refusal/replay assertions.
    fn saved(&self) -> Vec<u8> {
        fs::read(self.root.join(".swarm/engine-settings.json")).unwrap()
    }
    /// Existing Root report consumer must immediately use persisted current policy.
    fn report(&self) -> Result<String, swarm_engine::RootStoreError> {
        let r = RootStore::open(&self.root, false).unwrap();
        let result = r.review_delivery("report.md");
        r.close().unwrap();
        result
    }
}
impl Drop for Fixture {
    /// Preserve failed fixtures; remove only this owned successful test root.
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.root).unwrap();
        }
    }
}
/// Assert meaningful JSON on a successful response; failure bodies are not success.
fn json(reply: HttpReply) -> serde_json::Value {
    assert_eq!(reply.response.status(), StatusCode::OK);
    assert!(reply.failure.is_none());
    serde_json::from_slice(reply.response.body()).unwrap()
}
/// Shared catalogue/document and preview/apply contract, actual report consumer,
/// exact replay and persisted reopen; no provider invocation or policy fallback.
#[test]
fn native_settings_preview_apply_replay_and_real_consumer_follow_saved_values() {
    let f = Fixture::new();
    let s = NativeHttpService::open_local(&f.root, &f.listener).unwrap();
    let before = f.saved();
    assert_eq!(f.report().unwrap(), "report");
    let catalog = json(
        s.handle(f.request("GET", "/api/v1/native-settings/describe", vec![]))
            .unwrap(),
    );
    assert_eq!(
        catalog["document_schema_version"],
        f.document().schema_version
    );
    assert!(catalog["fields"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["name"] == "review_delivery_max_bytes"));
    let shown = json(
        s.handle(f.request("GET", "/api/v1/native-settings", vec![]))
            .unwrap(),
    );
    assert_eq!(shown["revision"], "1");
    assert_eq!(f.saved(), before);
    let mut change = f.change("cap-report", 3);
    let preview = json(
        s.handle(f.request(
            "POST",
            "/api/v1/native-settings/preview",
            serde_json::to_vec(&change).unwrap(),
        ))
        .unwrap(),
    );
    assert_eq!(f.saved(), before);
    assert_eq!(f.report().unwrap(), "report");
    change.preview_token = preview["preview_token"].as_str().unwrap().to_owned();
    let raw = serde_json::to_vec(&change).unwrap();
    let reply = s
        .handle(f.request("POST", "/api/v1/native-settings/apply", raw.clone()))
        .unwrap();
    assert!(reply.configuration_may_be_recorded);
    assert!(reply.recorded_work.is_none());
    let applied = json(reply);
    assert_eq!(applied["replayed"], false);
    assert_eq!(f.document().revision, SettingCount(2));
    assert!(f.report().is_err());
    let saved = f.saved();
    let reply = s
        .handle(f.request("POST", "/api/v1/native-settings/apply", raw))
        .unwrap();
    assert!(reply.configuration_may_be_recorded);
    let replay = json(reply);
    assert_eq!(replay["replayed"], true);
    assert_eq!(f.saved(), saved);
    let mut change = f.change("restore-report", 64);
    let preview = json(
        s.handle(f.request(
            "POST",
            "/api/v1/native-settings/preview",
            serde_json::to_vec(&change).unwrap(),
        ))
        .unwrap(),
    );
    change.preview_token = preview["preview_token"].as_str().unwrap().into();
    json(
        s.handle(f.request(
            "POST",
            "/api/v1/native-settings/apply",
            serde_json::to_vec(&change).unwrap(),
        ))
        .unwrap(),
    );
    assert_eq!(f.report().unwrap(), "report");
    s.close().unwrap();
}
/// A valid witness precedes every rejected input; auth, strict shape, method,
/// revision and preview token failures preserve saved bytes and recover.
#[test]
fn native_settings_refusals_preserve_history_and_never_return_empty_success() {
    let f = Fixture::new();
    let s = NativeHttpService::open_local(&f.root, &f.listener).unwrap();
    let before = f.saved();
    for fault in ["auth", "method", "empty", "unknown", "stale", "token"] {
        json(
            s.handle(f.request("GET", "/api/v1/native-settings", vec![]))
                .unwrap(),
        );
        let mut change = f.change("refused", 64);
        let (method, path, raw) = match fault {
            "method" => ("PUT", "/api/v1/native-settings", vec![]),
            "empty" => ("POST", "/api/v1/native-settings/preview", vec![]),
            "unknown" => {
                let mut v = serde_json::to_value(&change).unwrap();
                v["root"] = "/other-root".into();
                (
                    "POST",
                    "/api/v1/native-settings/preview",
                    serde_json::to_vec(&v).unwrap(),
                )
            }
            "stale" => {
                change.expected_revision = SettingCount(0);
                (
                    "POST",
                    "/api/v1/native-settings/preview",
                    serde_json::to_vec(&change).unwrap(),
                )
            }
            _ => (
                "POST",
                "/api/v1/native-settings/apply",
                serde_json::to_vec(&change).unwrap(),
            ),
        };
        let mut request = f.request(method, path, raw);
        if fault == "auth" {
            request.headers_mut().remove(COOKIE);
        }
        let reply = s.handle(request).unwrap();
        assert!(!reply.response.status().is_success(), "{fault}");
        assert_eq!(
            reply.response.status(),
            match fault {
                "auth" => StatusCode::FORBIDDEN,
                "method" => StatusCode::METHOD_NOT_ALLOWED,
                "stale" | "token" => StatusCode::CONFLICT,
                _ => StatusCode::BAD_REQUEST,
            },
            "{fault}"
        );
        assert!(reply.failure.is_some());
        assert!(!reply.configuration_may_be_recorded);
        assert!(reply.recorded_work.is_none());
        assert!(!reply.response.body().is_empty());
        assert_eq!(f.saved(), before, "{fault}");
        json(
            s.handle(f.request("GET", "/api/v1/native-settings", vec![]))
                .unwrap(),
        );
    }
    let head = s
        .handle(f.request("HEAD", "/api/v1/native-settings", vec![]))
        .unwrap();
    assert_eq!(head.response.status(), StatusCode::OK);
    assert!(head.response.body().is_empty());
    s.close().unwrap();
}
/// A replaced Root pathname cannot redirect catalogue/read/preview/apply to another
/// project. Restore paths before assertions; both schema-valid Roots are retained on failure.
#[test]
fn native_settings_routes_recheck_the_retained_root_descriptor_before_access() {
    let f = Fixture::new();
    let other = Fixture::new();
    let s = NativeHttpService::open_local(&f.root, &f.listener).unwrap();
    json(
        s.handle(f.request("GET", "/api/v1/native-settings", vec![]))
            .unwrap(),
    );
    let raw = serde_json::to_vec(&f.change("binding", 64)).unwrap();
    let before = other.saved();
    let parked = f.root.with_extension("parked");
    fs::rename(&f.root, &parked).unwrap();
    fs::rename(&other.root, &f.root).unwrap();
    let mut statuses = vec![];
    for (method, path) in [
        ("GET", "/api/v1/native-settings"),
        ("GET", "/api/v1/native-settings/describe"),
        ("POST", "/api/v1/native-settings/preview"),
        ("POST", "/api/v1/native-settings/apply"),
    ] {
        let reply = s.handle(f.request(method, path, raw.clone())).unwrap();
        statuses.push((reply.response.status(), reply.failure.unwrap().code()));
    }
    let unchanged = fs::read(f.root.join(".swarm/engine-settings.json")).unwrap() == before;
    fs::rename(&f.root, &other.root).unwrap();
    fs::rename(&parked, &f.root).unwrap();
    assert!(unchanged);
    assert!(statuses.iter().all(
        |(status, code)| *status == StatusCode::CONFLICT && *code == "web-root-binding-changed"
    ));
    json(
        s.handle(f.request("GET", "/api/v1/native-settings", vec![]))
            .unwrap(),
    );
    s.close().unwrap();
}
