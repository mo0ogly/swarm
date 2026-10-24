#![cfg(target_os = "linux")]
//! Actual TCP settings changes on owned Roots, distinct from UI/provider execution.
use std::{
    fs,
    io::{Read, Write},
    net::TcpStream,
    os::unix::fs::PermissionsExt,
};
use swarm_engine::{EngineSettingsChange, RootStore, SettingCap, SettingCount, SystemEnvironment};
use swarm_web::NativeHttpServer;
/// Real HTTP/1 exchange, no watchdog or retry. Complete Content-Length is byte length.
fn exchange(
    address: std::net::SocketAddr,
    host_cookie: &str,
    path: &str,
    body: Option<&str>,
) -> String {
    let method = if body.is_some() { "POST" } else { "GET" };
    let raw = body.unwrap_or("");
    let mut stream = TcpStream::connect(address).unwrap();
    stream.write_all(format!("{method} {path} HTTP/1.1\r\nHost: {address}\r\n{host_cookie}Content-Length: {}\r\nConnection: close\r\n\r\n{raw}",raw.len()).as_bytes()).unwrap();
    stream.flush().unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}
/// Require both status and usable body; an empty or refused reply is not success.
fn json(response: String) -> serde_json::Value {
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    serde_json::from_str(response.split_once("\r\n\r\n").unwrap().1).unwrap()
}
/// Valid read precedes mutation; actual preview/apply/replay must preserve history
/// and make the existing Root report consumer use the new saved cap.
#[test]
fn native_tcp_settings_preview_apply_replay_use_real_saved_consumer() {
    use sha2::{Digest, Sha256};
    let p =
        std::env::temp_dir().join(swarm_engine::generate_identifier("tcp-mutation96-").unwrap());
    fs::create_dir(&p).unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
    let r = RootStore::initialize(&p, &mut SystemEnvironment).unwrap();
    let d = r.settings().unwrap();
    r.close().unwrap();
    fs::write(p.join("report.md"), b"report").unwrap();
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
    let cookie = format!("Cookie: {namespace}={key}\r\nOrigin: http://{address}\r\n");
    let before = fs::read(p.join(".swarm/engine-settings.json")).unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel();
    let (events, observed) = std::sync::mpsc::channel();
    let handle = std::thread::spawn(move || {
        server.serve_until(
            async {
                let _ = rx.await;
            },
            &mut |outcome: &swarm_web::NativeJobOutcome| {
                events
                    .send(serde_json::to_value(outcome.record()).map_err(std::io::Error::other)?)
                    .map_err(std::io::Error::other)
            },
        )
    });
    let session = json(exchange(address, &cookie, "/api/v1/native-session", None));
    let csrf = session["csrf_token"].as_str().unwrap();
    assert_ne!(csrf, key);
    let headers = format!("{cookie}X-Swarm-CSRF: {csrf}\r\n");
    let mut values = d.values;
    values.review_delivery_max_bytes = Some(SettingCap(Some(SettingCount(3))));
    let mut change = EngineSettingsChange {
        schema_version: d.schema_version,
        event_id: "tcp-settings96".into(),
        expected_revision: d.revision,
        values,
        reason: "Owned actual TCP outcome witness".into(),
        preview_token: String::new(),
    };
    let preview = json(exchange(
        address,
        &headers,
        "/api/v1/native-settings/preview",
        Some(&serde_json::to_string(&change).unwrap()),
    ));
    assert_eq!(
        fs::read(p.join(".swarm/engine-settings.json")).unwrap(),
        before
    );
    change.preview_token = preview["preview_token"].as_str().unwrap().into();
    let raw = serde_json::to_string(&change).unwrap();
    let applied = json(exchange(
        address,
        &headers,
        "/api/v1/native-settings/apply",
        Some(&raw),
    ));
    assert_eq!(applied["replayed"], false);
    assert_eq!(applied["document"]["revision"], "2");
    let saved = fs::read(p.join(".swarm/engine-settings.json")).unwrap();
    assert_ne!(saved, before);
    let replay = json(exchange(
        address,
        &headers,
        "/api/v1/native-settings/apply",
        Some(&raw),
    ));
    assert_eq!(replay["replayed"], true);
    assert_eq!(
        fs::read(p.join(".swarm/engine-settings.json")).unwrap(),
        saved
    );
    // Restore by a new public preview/apply, then replay the older event. Its
    // historical revision2 must remain distinct from current saved revision3.
    let doc2: swarm_engine::EngineSettingsDocument =
        serde_json::from_value(applied["document"].clone()).unwrap();
    let mut restored = doc2.values;
    restored.review_delivery_max_bytes = Some(SettingCap(Some(SettingCount(64))));
    let mut restore = EngineSettingsChange {
        schema_version: doc2.schema_version,
        event_id: "tcp-settings96-restore".into(),
        expected_revision: doc2.revision,
        values: restored,
        reason: "Owned restore before old-event replay".into(),
        preview_token: String::new(),
    };
    restore.preview_token = json(exchange(
        address,
        &headers,
        "/api/v1/native-settings/preview",
        Some(&serde_json::to_string(&restore).unwrap()),
    ))["preview_token"]
        .as_str()
        .unwrap()
        .into();
    let restored = json(exchange(
        address,
        &headers,
        "/api/v1/native-settings/apply",
        Some(&serde_json::to_string(&restore).unwrap()),
    ));
    assert_eq!(restored["document"]["revision"], "3");
    let current = fs::read(p.join(".swarm/engine-settings.json")).unwrap();
    let old_replay = json(exchange(
        address,
        &headers,
        "/api/v1/native-settings/apply",
        Some(&raw),
    ));
    assert_eq!(old_replay["replayed"], true);
    assert_eq!(old_replay["document"]["revision"], "3");
    assert_eq!(
        fs::read(p.join(".swarm/engine-settings.json")).unwrap(),
        current
    );
    tx.send(()).unwrap();
    let report = handle.join().unwrap().unwrap();
    assert_eq!(report.completed_jobs, 7);
    let records: Vec<serde_json::Value> = observed.into_iter().collect();
    assert_eq!(records.len(), 7);
    assert_eq!(
        records[2]["recorded_configuration"]["event_id"],
        "tcp-settings96"
    );
    assert_eq!(records[2]["recorded_configuration"]["event_revision"], "2");
    assert_eq!(
        records[2]["recorded_configuration"]["current_revision"],
        "2"
    );
    assert_eq!(records[2]["recorded_configuration"]["replayed"], false);
    assert_eq!(records[3]["recorded_configuration"]["replayed"], true);
    assert_eq!(records[5]["recorded_configuration"]["event_revision"], "3");
    assert_eq!(
        records[6]["recorded_configuration"]["event_id"],
        "tcp-settings96"
    );
    assert_eq!(records[6]["recorded_configuration"]["event_revision"], "2");
    assert_eq!(
        records[6]["recorded_configuration"]["current_revision"],
        "3"
    );
    assert_eq!(records[6]["recorded_configuration"]["replayed"], true);

    let r = RootStore::open(&p, false).unwrap();
    assert_eq!(r.settings().unwrap().revision, SettingCount(3));
    assert_eq!(r.review_delivery("report.md").unwrap(), "report");
    assert!(r.list_works().unwrap().is_empty());
    r.close().unwrap();
    fs::remove_dir_all(p).unwrap();
}
