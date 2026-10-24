#![cfg(target_os = "linux")]
//! Protected CSRF bootstrap tests; typed HTTP only until transport integration.
use http::{
    header::{COOKIE, HOST, ORIGIN},
    Request, StatusCode,
};
use std::{fs, net::TcpListener, os::unix::fs::PermissionsExt};
use swarm_engine::{EngineSettingsChange, RootStore, SystemEnvironment};
use swarm_web::{LocalWebAuthority, NativeHttpService};
/// A protected session response must return dedicated CSRF, not the bearer key.
/// The derived value authorizes POST with the real cookie/origin but cannot
/// replace the cookie. Preview stays read-only; no human/agent identity is inferred.
#[test]
fn native_session_returns_scoped_csrf_without_exposing_bearer_authority() {
    let root = std::env::temp_dir().join(swarm_engine::generate_identifier("csrf95-").unwrap());
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let r = RootStore::initialize(&root, &mut SystemEnvironment).unwrap();
    let session = r.web_session(&listener).unwrap();
    let document = r.settings().unwrap();
    let key = session.secret().to_owned();
    r.close().unwrap();
    let before = fs::read(root.join(".swarm/engine-settings.json")).unwrap();
    let authority = LocalWebAuthority::from_listener(&listener, &key).unwrap();
    let host = authority.host().to_owned();
    let cookie = authority.cookie_name().to_owned();
    let service = NativeHttpService::open_local(&root, &listener).unwrap();
    let request =
        |method: &str, path: &str, credential: &str, csrf: Option<&str>, body: Vec<u8>| {
            let mut b = Request::builder()
                .method(method)
                .uri(path)
                .header(HOST, &host)
                .header(COOKIE, format!("{cookie}={credential}"))
                .header(ORIGIN, format!("http://{host}"));
            if let Some(csrf) = csrf {
                b = b.header("X-Swarm-CSRF", csrf)
            }
            b.body(body).unwrap()
        };
    let reply = service
        .handle(request("GET", "/api/v1/native-session", &key, None, vec![]))
        .unwrap();
    assert_eq!(reply.response.status(), StatusCode::OK);
    assert!(reply.failure.is_none());
    let value: serde_json::Value = serde_json::from_slice(reply.response.body()).unwrap();
    let csrf = value["csrf_token"].as_str().unwrap();
    assert_eq!(csrf.len(), 64);
    assert!(csrf != key);
    assert!(!std::str::from_utf8(reply.response.body())
        .unwrap()
        .contains(&key));
    let change = EngineSettingsChange {
        schema_version: document.schema_version,
        event_id: "csrf-preview".into(),
        expected_revision: document.revision,
        values: document.values,
        reason: "Dedicated CSRF fixture".into(),
        preview_token: String::new(),
    };
    let reply = service
        .handle(request(
            "POST",
            "/api/v1/native-settings/preview",
            &key,
            Some(csrf),
            serde_json::to_vec(&change).unwrap(),
        ))
        .unwrap();
    assert_eq!(reply.response.status(), StatusCode::OK);
    assert_eq!(
        fs::read(root.join(".swarm/engine-settings.json")).unwrap(),
        before
    );
    let reply = service
        .handle(request("GET", "/api/v1/native-session", csrf, None, vec![]))
        .unwrap();
    assert_eq!(reply.response.status(), StatusCode::FORBIDDEN);
    assert!(reply.failure.is_some());
    service.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}
