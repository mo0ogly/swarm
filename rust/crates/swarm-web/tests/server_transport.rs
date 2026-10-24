#![cfg(target_os = "linux")]
//! Real TCP witness, rejection and graceful drain on an isolated private Root.
use std::{
    fs,
    io::{Read, Write},
    net::TcpStream,
    os::unix::fs::PermissionsExt,
};
use swarm_engine::{RootStore, SystemEnvironment};
use swarm_web::NativeHttpServer;
/// Complete HTTP/1 exchange over real loopback, with no global test watchdog.
fn exchange(address: std::net::SocketAddr, request: String) -> String {
    let mut stream = TcpStream::connect(address).unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    stream.flush().unwrap();
    let mut bytes = String::new();
    stream.read_to_string(&mut bytes).unwrap();
    bytes
}
/// Witness authenticated read first; then unauthenticated/Host/mutation refusals
/// must leave Work/settings unchanged and shutdown must close owned resources.
#[test]
fn native_transport_real_tcp_gates_before_root_and_drains_owned_jobs() {
    let p = std::env::temp_dir().join(swarm_engine::generate_identifier("transport95-").unwrap());
    fs::create_dir(&p).unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
    RootStore::initialize(&p, &mut SystemEnvironment)
        .unwrap()
        .close()
        .unwrap();
    let server = NativeHttpServer::bind(&p, None).unwrap();
    let address = server.local_addr().unwrap();
    let session = fs::read_dir(p.join(".swarm"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("web-session-")
        })
        .unwrap();
    let key = fs::read_to_string(session).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    drop(listener);
    // Cookie namespace is a protocol projection, not a second authority constructor.
    use sha2::{Digest, Sha256};
    let namespace = format!(
        "swarm_session_{:.12}",
        format!("{:x}", Sha256::digest(address.to_string().as_bytes()))
    );
    let cookie = format!("{namespace}={key}");
    let before = fs::read(p.join(".swarm/engine-settings.json")).unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel();
    let handle = std::thread::spawn(move || {
        // Fixture-only acknowledgement; not a durable journal or supervisor receipt.
        server.serve_until(
            async {
                let _ = rx.await;
            },
            &mut |_: &swarm_web::NativeJobOutcome| Ok(()),
        )
    });
    let response=exchange(address,format!("GET /api/v1/works HTTP/1.1\r\nHost: {address}\r\nCookie: {cookie}\r\nConnection: close\r\n\r\n"));
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(response.contains("[]"));
    let response = exchange(
        address,
        format!("GET /api/v1/works HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"),
    );
    assert!(response.starts_with("HTTP/1.1 403"));
    let response=exchange(address,format!("GET /api/v1/works HTTP/1.1\r\nHost: attacker.invalid\r\nCookie: {cookie}\r\nConnection: close\r\n\r\n"));
    assert!(response.starts_with("HTTP/1.1 403"));
    let response=exchange(address,format!("GET /api/v1/native-session HTTP/1.1\r\nHost: {address}\r\nCookie: {cookie}\r\nConnection: close\r\n\r\n"));
    assert!(response.starts_with("HTTP/1.1 200"));
    assert!(response.contains("hmac-sha256-v1"));
    assert!(!response.contains(&key));
    let capabilities=exchange(address,format!("GET /api/v1/native-capabilities HTTP/1.1\r\nHost: {address}\r\nCookie: {cookie}\r\nConnection: close\r\n\r\n"));
    assert!(capabilities.starts_with("HTTP/1.1 200"));
    let capabilities: serde_json::Value =
        serde_json::from_str(capabilities.split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(capabilities["actions"], serde_json::json!(["task"]));
    assert_eq!(
        capabilities["settings"],
        serde_json::json!(["describe", "show", "preview", "apply"])
    );
    assert_eq!(capabilities["profile"], "rust-native-http1-observed");
    let response=exchange(address,format!("POST /api/v1/action HTTP/1.1\r\nHost: {address}\r\nCookie: {cookie}\r\nOrigin: http://{address}\r\nX-Swarm-CSRF: {key}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"));
    assert!(response.starts_with("HTTP/1.1 400"), "{response}");
    tx.send(()).unwrap();
    let report = handle.join().unwrap().unwrap();
    assert_eq!(report.completed_jobs, 4);
    assert_eq!(report.accepted_connections, 6);
    assert_eq!(report.connection_failures, 0);
    assert_eq!(report.rejected_requests, 2);
    assert!(report
        .first_request_failure
        .unwrap()
        .to_string()
        .starts_with("web-"));
    assert!(report.first_service_failure.is_some());
    assert_eq!(
        fs::read(p.join(".swarm/engine-settings.json")).unwrap(),
        before
    );
    let r = RootStore::open(&p, false).unwrap();
    assert!(r.list_works().unwrap().is_empty());
    r.close().unwrap();
    assert!(TcpStream::connect(address).is_err());
    fs::remove_dir_all(p).unwrap();
}

/// Saved header/body limits are consumed by actual Hyper parsing/collection.
/// A valid same-server witness precedes faults; recovery remains usable and no
/// refused request reaches Root. No arbitrary whole-test deadline is installed.
#[test]
fn native_transport_enforces_saved_header_and_body_limits_then_recovers() {
    use sha2::{Digest, Sha256};
    use swarm_engine::{EngineSettingsChange, SettingCap, SettingCount};
    let p = std::env::temp_dir()
        .join(swarm_engine::generate_identifier("transport-limits95-").unwrap());
    fs::create_dir(&p).unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
    let r = RootStore::initialize(&p, &mut SystemEnvironment).unwrap();
    let document = r.settings().unwrap();
    let mut values = document.values;
    values.web_http_max_headers = Some(SettingCount(4));
    values.web_http_body_max_bytes = Some(SettingCap(Some(SettingCount(2))));
    let mut change = EngineSettingsChange {
        schema_version: document.schema_version,
        event_id: "transport-limits".into(),
        expected_revision: document.revision,
        values,
        reason: "Owned actual Hyper limits witness".into(),
        preview_token: String::new(),
    };
    change.preview_token = r.preview_settings(&change).unwrap().preview_token;
    r.apply_settings(&change, &mut SystemEnvironment).unwrap();
    r.close().unwrap();
    let server = NativeHttpServer::bind(&p, None).unwrap();
    assert_eq!(server.policy().max_headers, 4);
    assert_eq!(server.policy().body_max_bytes, Some(2));
    let address = server.local_addr().unwrap();
    let key = fs::read_to_string(
        fs::read_dir(p.join(".swarm"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("web-session-")
            })
            .unwrap(),
    )
    .unwrap();
    let namespace = format!(
        "swarm_session_{:.12}",
        format!("{:x}", Sha256::digest(address.to_string().as_bytes()))
    );
    let cookie = format!("{namespace}={key}");
    let before = fs::read(p.join(".swarm/engine-settings.json")).unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel();
    let handle = std::thread::spawn(move || {
        // Fixture-only acknowledgement; not a durable journal or supervisor receipt.
        server.serve_until(
            async {
                let _ = rx.await;
            },
            &mut |_: &swarm_web::NativeJobOutcome| Ok(()),
        )
    });
    let valid = || {
        format!("GET /api/v1/works HTTP/1.1\r\nHost: {address}\r\nCookie: {cookie}\r\nConnection: close\r\nContent-Length: 2\r\n\r\nab")
    };
    assert!(exchange(address, valid()).starts_with("HTTP/1.1 200"));
    let response=exchange(address,format!("GET /api/v1/works HTTP/1.1\r\nHost: {address}\r\nCookie: {cookie}\r\nConnection: close\r\nContent-Length: 3\r\n\r\nabc"));
    assert!(response.starts_with("HTTP/1.1 413"), "{response}");
    let response=exchange(address,format!("GET /api/v1/works HTTP/1.1\r\nHost: {address}\r\nCookie: {cookie}\r\nConnection: close\r\nX-Extra-1: a\r\nX-Extra-2: b\r\n\r\n"));
    assert!(response.starts_with("HTTP/1.1 431"), "{response}");
    assert!(exchange(address, valid()).starts_with("HTTP/1.1 200"));
    tx.send(()).unwrap();
    let report = handle.join().unwrap().unwrap();
    assert_eq!(report.completed_jobs, 2);
    assert_eq!(report.rejected_requests, 1);
    assert_eq!(report.connection_failures, 1);
    assert!(matches!(
        report.first_request_failure,
        Some(swarm_web::NativeRequestError::Body(_))
    ));
    assert!(report.first_connection_failure.unwrap().is_parse());
    assert_eq!(
        fs::read(p.join(".swarm/engine-settings.json")).unwrap(),
        before
    );
    let r = RootStore::open(&p, false).unwrap();
    assert!(r.list_works().unwrap().is_empty());
    r.close().unwrap();
    fs::remove_dir_all(p).unwrap();
}

/// Read one protocol header block; the100-continue barrier proves the server has
/// acquired the lane and is awaiting the body, without sleeps or polling budgets.
fn header_block(stream: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    while !bytes.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        assert_eq!(stream.read(&mut byte).unwrap(), 1);
        bytes.push(byte[0]);
    }
    String::from_utf8(bytes).unwrap()
}
/// Occupying a Root body lane does not consume the Admin lane. Saturation refuses
/// immediately, releasing the body permits recovery. Shutdown with an unread body
/// cancels only pre-Root IO, creates no extra job and closes owned connections.
#[test]
fn native_transport_root_admin_lanes_and_pre_root_shutdown_are_distinct() {
    use sha2::{Digest, Sha256};
    use swarm_engine::{EngineSettingsChange, SettingCap, SettingCount};
    let p =
        std::env::temp_dir().join(swarm_engine::generate_identifier("transport-lanes95-").unwrap());
    fs::create_dir(&p).unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
    let r = RootStore::initialize(&p, &mut SystemEnvironment).unwrap();
    let document = r.settings().unwrap();
    let mut values = document.values;
    values.web_root_worker_threads = Some(SettingCount(1));
    values.web_admin_worker_threads = Some(SettingCount(1));
    values.web_http_body_timeout_ms = Some(SettingCap(None));
    let mut change = EngineSettingsChange {
        schema_version: document.schema_version,
        event_id: "transport-lanes".into(),
        expected_revision: document.revision,
        values,
        reason: "Owned lane barrier fixture".into(),
        preview_token: String::new(),
    };
    change.preview_token = r.preview_settings(&change).unwrap().preview_token;
    r.apply_settings(&change, &mut SystemEnvironment).unwrap();
    r.close().unwrap();
    let server = NativeHttpServer::bind(&p, None).unwrap();
    let address = server.local_addr().unwrap();
    let key = fs::read_to_string(
        fs::read_dir(p.join(".swarm"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("web-session-")
            })
            .unwrap(),
    )
    .unwrap();
    let namespace = format!(
        "swarm_session_{:.12}",
        format!("{:x}", Sha256::digest(address.to_string().as_bytes()))
    );
    let cookie = format!("{namespace}={key}");
    let before = fs::read(p.join(".swarm/engine-settings.json")).unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel();
    let handle = std::thread::spawn(move || {
        // Fixture-only acknowledgement; not a durable journal or supervisor receipt.
        server.serve_until(
            async {
                let _ = rx.await;
            },
            &mut |_: &swarm_web::NativeJobOutcome| Ok(()),
        )
    });
    let get = |path: &str| {
        format!("GET {path} HTTP/1.1\r\nHost: {address}\r\nCookie: {cookie}\r\nConnection: close\r\n\r\n")
    };
    assert!(exchange(address, get("/api/v1/works")).starts_with("HTTP/1.1 200"));
    let pending = || {
        let mut stream = TcpStream::connect(address).unwrap();
        stream.write_all(format!("GET /api/v1/works HTTP/1.1\r\nHost: {address}\r\nCookie: {cookie}\r\nConnection: close\r\nContent-Length: 1\r\nExpect: 100-continue\r\n\r\n").as_bytes()).unwrap();
        stream.flush().unwrap();
        assert!(header_block(&mut stream).starts_with("HTTP/1.1 100"));
        stream
    };
    let mut occupied = pending();
    let response = exchange(address, get("/api/v1/works"));
    assert!(response.starts_with("HTTP/1.1 503"), "{response}");
    let response = exchange(address, get("/api/v1/native-settings"));
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    occupied.write_all(b"x").unwrap();
    occupied.flush().unwrap();
    let mut response = String::new();
    occupied.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200"));
    drop(occupied);
    let mut interrupted = pending();
    tx.send(()).unwrap();
    let report = handle.join().unwrap().unwrap();
    let mut byte = [0];
    assert_eq!(interrupted.read(&mut byte).unwrap(), 0);
    assert_eq!(report.completed_jobs, 3);
    assert_eq!(report.rejected_requests, 1);
    assert_eq!(
        report.first_request_failure.unwrap().to_string(),
        "web-workers-saturated"
    );
    assert_eq!(
        fs::read(p.join(".swarm/engine-settings.json")).unwrap(),
        before
    );
    let r = RootStore::open(&p, false).unwrap();
    assert!(r.list_works().unwrap().is_empty());
    r.close().unwrap();
    fs::remove_dir_all(p).unwrap();
}

/// Persisted max-connections1 refuses a second accepted socket, then recovers
/// after a protocol100-continue barrier releases the first. This tests the real
/// connection semaphore, distinct from Root worker saturation and OS backlog.
#[test]
fn native_transport_connection_cap_is_persisted_consumed_and_released() {
    use sha2::{Digest, Sha256};
    use swarm_engine::{EngineSettingsChange, SettingCap, SettingCount};
    let p = std::env::temp_dir()
        .join(swarm_engine::generate_identifier("transport-connections95-").unwrap());
    fs::create_dir(&p).unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
    let r = RootStore::initialize(&p, &mut SystemEnvironment).unwrap();
    let document = r.settings().unwrap();
    let mut values = document.values;
    values.web_http_max_connections = Some(SettingCap(Some(SettingCount(1))));
    values.web_http_body_timeout_ms = Some(SettingCap(None));
    let mut change = EngineSettingsChange {
        schema_version: document.schema_version,
        event_id: "transport-connections".into(),
        expected_revision: document.revision,
        values,
        reason: "Owned actual connection-cap fixture".into(),
        preview_token: String::new(),
    };
    change.preview_token = r.preview_settings(&change).unwrap().preview_token;
    r.apply_settings(&change, &mut SystemEnvironment).unwrap();
    r.close().unwrap();
    let server = NativeHttpServer::bind(&p, None).unwrap();
    let address = server.local_addr().unwrap();
    let key = fs::read_to_string(
        fs::read_dir(p.join(".swarm"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("web-session-")
            })
            .unwrap(),
    )
    .unwrap();
    let namespace = format!(
        "swarm_session_{:.12}",
        format!("{:x}", Sha256::digest(address.to_string().as_bytes()))
    );
    let cookie = format!("{namespace}={key}");
    let before = fs::read(p.join(".swarm/engine-settings.json")).unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel();
    let handle = std::thread::spawn(move || {
        // Fixture-only acknowledgement; not a durable journal or supervisor receipt.
        server.serve_until(
            async {
                let _ = rx.await;
            },
            &mut |_: &swarm_web::NativeJobOutcome| Ok(()),
        )
    });
    let get = || {
        format!("GET /api/v1/works HTTP/1.1\r\nHost: {address}\r\nCookie: {cookie}\r\nConnection: close\r\n\r\n")
    };
    assert!(exchange(address, get()).starts_with("HTTP/1.1 200"));
    let mut occupied = TcpStream::connect(address).unwrap();
    occupied.write_all(format!("GET /api/v1/works HTTP/1.1\r\nHost: {address}\r\nCookie: {cookie}\r\nConnection: close\r\nContent-Length: 1\r\nExpect: 100-continue\r\n\r\n").as_bytes()).unwrap();
    occupied.flush().unwrap();
    assert!(header_block(&mut occupied).starts_with("HTTP/1.1 100"));
    let mut excess = TcpStream::connect(address).unwrap();
    let mut byte = [0];
    assert_eq!(excess.read(&mut byte).unwrap(), 0);
    drop(excess);
    occupied.write_all(b"x").unwrap();
    occupied.flush().unwrap();
    let mut response = String::new();
    occupied.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200"));
    drop(occupied);
    assert!(exchange(address, get()).starts_with("HTTP/1.1 200"));
    tx.send(()).unwrap();
    let report = handle.join().unwrap().unwrap();
    assert_eq!(report.accepted_connections, 4);
    assert_eq!(report.rejected_connections, 1);
    assert_eq!(report.completed_jobs, 3);
    assert_eq!(report.connection_failures, 0);
    assert_eq!(
        fs::read(p.join(".swarm/engine-settings.json")).unwrap(),
        before
    );
    fs::remove_dir_all(p).unwrap();
}
