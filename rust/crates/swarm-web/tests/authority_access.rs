#![cfg(target_os = "linux")]
//! v52 `authority` access mode on a real service: assertions signed by the
//! daemon key open protected routes; the owner cookie no longer does; replay,
//! request mismatch, stale and foreign-key assertions refuse. Fixture Root in
//! the configured fixture area (never /tmp).
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::SigningKey;
use http::{header::{COOKIE, HOST, ORIGIN}, Request, StatusCode};
use std::{fs, net::TcpListener, os::unix::fs::PermissionsExt, path::PathBuf};
use swarm_authority::assertion::{body_digest, sign, Claims, HEADER};
use swarm_engine::{EngineSettingsChange, RootStore, SystemEnvironment};
use swarm_web::{HttpReply, NativeHttpService};

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64
}

fn claims(method: &str, path: &str, body: &[u8], at: i64, nonce: &str) -> Claims {
    Claims { v: 1, subject: "ana".into(), roles: vec!["operator".into()], integrity: 2, method: method.into(), path: path.into(), body_sha256: body_digest(body), issued_at: at, nonce: nonce.into() }
}

#[test]
fn authority_mode_requires_fresh_single_use_bound_assertion() {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../.swarm-fixtures");
    fs::create_dir_all(&base).unwrap();
    let root = base.join(swarm_engine::generate_identifier("authority-access-").unwrap());
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let daemon = SigningKey::from_bytes(&[9; 32]);
    let store = RootStore::initialize(&root, &mut SystemEnvironment).unwrap();
    let d = store.settings().unwrap();
    let mut values = d.values;
    values.web_access_mode = Some("authority".into());
    values.web_authority_public_key = Some(URL_SAFE_NO_PAD.encode(daemon.verifying_key().to_bytes()));
    let mut change = EngineSettingsChange { schema_version: d.schema_version, event_id: "authority-mode".into(), expected_revision: d.revision, values, reason: "Authority mode fixture".into(), preview_token: String::new() };
    change.preview_token = store.preview_settings(&change).unwrap().preview_token;
    store.apply_settings(&change, &mut SystemEnvironment).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let host = listener.local_addr().unwrap().to_string();
    let key = store.web_session(&listener).unwrap().secret().to_owned();
    store.close().unwrap();
    let service = NativeHttpService::open_local(&root, &listener).unwrap();
    let send = |method: &str, path: &str, body: &[u8], header: Option<String>, cookie: bool| -> HttpReply {
        let mut b = Request::builder().method(method).uri(path).header(HOST, &host);
        if let Some(h) = header { b = b.header(HEADER, h); }
        if cookie { b = b.header(COOKIE, format!("swarm_session={key}")).header(ORIGIN, format!("http://{host}")).header("x-swarm-csrf", &key); }
        service.handle(b.body(body.to_vec()).unwrap()).unwrap()
    };
    let t = now();
    // Witness: signed GET opens the protected route.
    let ok = send("GET", "/api/v1/native-version", b"", Some(sign(&daemon, &claims("GET", "/api/v1/native-version", b"", t, "nonce-0000000000001")).unwrap()), false);
    assert_eq!(ok.response.status(), StatusCode::OK);
    // The owner cookie only keeps the launcher's two read-only probes.
    for probe in ["/api/v1/native-version", "/api/v1/native-capabilities"] {
        assert_eq!(send("GET", probe, b"", None, true).response.status(), StatusCode::OK, "{probe}");
        assert_eq!(send("GET", probe, b"", None, false).response.status(), StatusCode::FORBIDDEN, "{probe} without key");
    }
    let owner = send("GET", "/api/v1/works", b"", None, true);
    assert_eq!(owner.response.status(), StatusCode::FORBIDDEN);
    assert_eq!(owner.failure.as_ref().unwrap().code(), "web-assertion-required");
    let probe_query = send("GET", "/api/v1/native-version?x=1", b"", None, true);
    assert_eq!(probe_query.failure.as_ref().unwrap().code(), "web-assertion-required");
    let owner_post = send("POST", "/api/v1/logout", b"", None, true);
    assert_eq!(owner_post.failure.as_ref().unwrap().code(), "web-assertion-required");
    // Replay of the same header refuses.
    let h = sign(&daemon, &claims("GET", "/api/v1/native-version", b"", t, "nonce-0000000000002")).unwrap();
    assert_eq!(send("GET", "/api/v1/native-version", b"", Some(h.clone()), false).response.status(), StatusCode::OK);
    let replay = send("GET", "/api/v1/native-version", b"", Some(h), false);
    assert_eq!(replay.failure.as_ref().unwrap().code(), "assertion-replayed");
    // Bound to path and body; stale; foreign key.
    let other_path = send("GET", "/api/v1/works", b"", Some(sign(&daemon, &claims("GET", "/api/v1/native-version", b"", t, "nonce-0000000000003")).unwrap()), false);
    assert_eq!(other_path.failure.as_ref().unwrap().code(), "assertion-request-mismatch");
    let body = send("POST", "/api/v1/logout", b"x", Some(sign(&daemon, &claims("POST", "/api/v1/logout", b"", t, "nonce-0000000000004")).unwrap()), false);
    assert_eq!(body.failure.as_ref().unwrap().code(), "assertion-request-mismatch");
    let stale = send("GET", "/api/v1/native-version", b"", Some(sign(&daemon, &claims("GET", "/api/v1/native-version", b"", t - 3600, "nonce-0000000000005")).unwrap()), false);
    assert_eq!(stale.failure.as_ref().unwrap().code(), "assertion-stale");
    let forged = send("GET", "/api/v1/native-version", b"", Some(sign(&SigningKey::from_bytes(&[1; 32]), &claims("GET", "/api/v1/native-version", b"", t, "nonce-0000000000006")).unwrap()), false);
    assert_eq!(forged.failure.as_ref().unwrap().code(), "assertion-signature");
    for r in [&owner, &replay, &other_path, &body, &stale, &forged] {
        assert_ne!(r.response.status(), StatusCode::OK);
    }
    service.close().unwrap();
    fs::remove_dir_all(&root).unwrap();
}
