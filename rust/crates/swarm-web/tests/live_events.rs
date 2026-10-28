#![cfg(target_os = "linux")]
//! Real TCP server-sent live events on an isolated private Root; no live data,
//! no global watchdog: every wait is on an observable stream byte.
use rusqlite::{params, Connection};
use std::{
    fs,
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
};
use swarm_engine::{encode_model, Agent, RootStore, SystemEnvironment};
use swarm_web::NativeHttpServer;

fn root() -> (PathBuf, String) {
    let p = std::env::temp_dir().join(swarm_engine::generate_identifier("live-sse-").unwrap());
    fs::create_dir(&p).unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
    let mut r = RootStore::initialize(&p, &mut SystemEnvironment).unwrap();
    let work = r.execute_external("", "work.create", br#"{"schema_version":1,"event_id":"c","expected_revision":0,"title":"Live","objective":"Observe","scope":"Fixture","criteria":["Visible"]}"#, &mut SystemEnvironment).unwrap().result.work.id;
    r.close().unwrap();
    let db = Connection::open(p.join(".swarm/state.db")).unwrap();
    let mut a = Agent::default();
    a.id = "a".into();
    a.work_id = work.clone();
    a.task_id = "t".into();
    a.workspace = "/isolated/a".into();
    a.status = "running".into();
    db.execute("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES('a',?,'t','/isolated/a','running',?,'{}')", params![work, encode_model(&a).unwrap()]).unwrap();
    (p, work)
}
fn append_log(p: &std::path::Path, message: &str) {
    Connection::open(p.join(".swarm/state.db"))
        .unwrap()
        .execute(
            "INSERT INTO agent_logs(agent_id,at,kind,message) VALUES('a','t','message',?)",
            [message],
        )
        .unwrap();
}
fn cookie(p: &std::path::Path, address: SocketAddr) -> String {
    use sha2::{Digest, Sha256};
    let session = fs::read_dir(p.join(".swarm"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.file_name().unwrap().to_string_lossy().starts_with("web-session-"))
        .unwrap();
    let key = fs::read_to_string(session).unwrap();
    format!(
        "swarm_session_{:.12}={key}",
        format!("{:x}", Sha256::digest(address.to_string().as_bytes()))
    )
}
/// Read stream bytes until `needle` appears after `from`; return text and end offset.
fn until(stream: &mut TcpStream, seen: &mut String, from: usize, needle: &str) -> usize {
    loop {
        if let Some(i) = seen[from..].find(needle) {
            return from + i + needle.len();
        }
        let mut buffer = [0u8; 4096];
        let n = stream.read(&mut buffer).unwrap();
        assert!(n > 0, "stream ended before {needle:?}: {seen}");
        seen.push_str(&String::from_utf8_lossy(&buffer[..n]));
    }
}
/// The `id:` of the next `change` event after `from`.
fn next_change(stream: &mut TcpStream, seen: &mut String, from: usize) -> (String, usize) {
    let end = until(stream, seen, from, "\nevent: change\n");
    let head = &seen[..end - "\nevent: change\n".len()];
    let id = head.rsplit("id: ").next().unwrap().to_owned();
    assert_eq!(id.len(), 64, "{seen}");
    (id, end)
}
fn exchange(address: SocketAddr, request: String) -> String {
    let mut stream = TcpStream::connect(address).unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut bytes = String::new();
    stream.read_to_string(&mut bytes).unwrap();
    bytes
}

#[test]
fn live_events_push_changes_resume_by_id_refuse_bad_shapes_and_end_on_shutdown() {
    let (p, _) = root();
    let server = NativeHttpServer::bind(&p, None).unwrap();
    let address = server.local_addr().unwrap();
    let cookie = cookie(&p, address);
    let (tx, rx) = tokio::sync::oneshot::channel();
    let handle = std::thread::spawn(move || {
        server.serve_until(
            async {
                let _ = rx.await;
            },
            &mut |_: &swarm_web::NativeJobOutcome| Ok(()),
        )
    });
    // Witness: the fingerprint route itself answers.
    let witness = exchange(address, format!("GET /api/v1/live/fingerprint HTTP/1.1\r\nHost: {address}\r\nCookie: {cookie}\r\nConnection: close\r\n\r\n"));
    assert!(witness.starts_with("HTTP/1.1 200"), "{witness}");
    let mut stream = TcpStream::connect(address).unwrap();
    stream.write_all(format!("GET /api/v1/live/events HTTP/1.1\r\nHost: {address}\r\nCookie: {cookie}\r\n\r\n").as_bytes()).unwrap();
    let mut seen = String::new();
    let headers = until(&mut stream, &mut seen, 0, "\r\n\r\n");
    assert!(seen.starts_with("HTTP/1.1 200"), "{seen}");
    assert!(seen[..headers].contains("text/event-stream"));
    let after_retry = until(&mut stream, &mut seen, headers, "retry: 2000\n\n");
    let (first, offset) = next_change(&mut stream, &mut seen, after_retry);
    assert!(witness.contains(&first), "initial event carries current fingerprint");
    append_log(&p, "one");
    let (second, offset) = next_change(&mut stream, &mut seen, offset);
    assert_ne!(first, second);
    // Resume: Last-Event-ID equal to current state suppresses the first change.
    let mut resumed = TcpStream::connect(address).unwrap();
    resumed.write_all(format!("GET /api/v1/live/events HTTP/1.1\r\nHost: {address}\r\nCookie: {cookie}\r\nLast-Event-ID: {second}\r\n\r\n").as_bytes()).unwrap();
    let mut seen_resumed = String::new();
    let start = until(&mut resumed, &mut seen_resumed, 0, "retry: 2000\n\n");
    append_log(&p, "two");
    let (third, _) = next_change(&mut resumed, &mut seen_resumed, start);
    assert_ne!(third, second, "first change after resume is the new state");
    let (third_main, _) = next_change(&mut stream, &mut seen, offset);
    assert_eq!(third, third_main);
    // Refused shapes take the ordinary path with explicit codes.
    let anonymous = exchange(address, format!("GET /api/v1/live/events HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"));
    assert!(anonymous.starts_with("HTTP/1.1 403"), "{anonymous}");
    let query = exchange(address, format!("GET /api/v1/live/events?x=1 HTTP/1.1\r\nHost: {address}\r\nCookie: {cookie}\r\nConnection: close\r\n\r\n"));
    assert!(query.starts_with("HTTP/1.1 400") && query.contains("live-query-invalid"), "{query}");
    // Shutdown with streams open: serve_until returns and streams reach EOF.
    tx.send(()).unwrap();
    let report = handle.join().unwrap().unwrap();
    assert!(report.completed_jobs >= 4, "probes are observed jobs: {}", report.completed_jobs);
    assert_eq!(report.connection_failures, 0);
    for s in [&mut stream, &mut resumed] {
        let mut rest = Vec::new();
        s.read_to_end(&mut rest).unwrap();
    }
    fs::remove_dir_all(&p).unwrap();
}
