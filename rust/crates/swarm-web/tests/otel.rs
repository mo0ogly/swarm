#![cfg(target_os = "linux")]
//! Real TCP OTLP/JSON ingest on an isolated private Root (LIVE-UX-03 E2E-1).
//! Tokens are issued through the engine API, as the launch path will do once
//! it exists; no live Root, no provider process, no global watchdog.
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::{
    fs,
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
use swarm_engine::{encode_model, Agent, RootStore, SystemEnvironment, TelemetryStore};
use swarm_web::NativeHttpServer;

fn root() -> (PathBuf, String) {
    let p = std::env::temp_dir().join(swarm_engine::generate_identifier("otel-").unwrap());
    fs::create_dir(&p).unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
    let mut r = RootStore::initialize(&p, &mut SystemEnvironment).unwrap();
    let work = r.execute_external("", "work.create", br#"{"schema_version":1,"event_id":"c","expected_revision":0,"title":"Otel","objective":"Observe","scope":"Fixture","criteria":["Visible"]}"#, &mut SystemEnvironment).unwrap().result.work.id;
    r.close().unwrap();
    let db = Connection::open(p.join(".swarm/state.db")).unwrap();
    for (id, status) in [("a", "running"), ("b", "running"), ("c", "completed")] {
        let mut agent = Agent::default();
        agent.id = id.into();
        agent.work_id = work.clone();
        agent.task_id = format!("t-{id}");
        agent.workspace = format!("/isolated/{id}");
        agent.status = status.into();
        db.execute(
            "INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,'{}')",
            params![id, work, agent.task_id, agent.workspace, status, encode_model(&agent).unwrap()],
        )
        .unwrap();
    }
    (p, work)
}
fn cookie(p: &Path, address: SocketAddr) -> String {
    use sha2::{Digest, Sha256};
    let session = fs::read_dir(p.join(".swarm"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.file_name().unwrap().to_string_lossy().starts_with("web-session-"))
        .unwrap();
    let key = fs::read_to_string(session).unwrap();
    format!("swarm_session_{:.12}={key}", format!("{:x}", Sha256::digest(address.to_string().as_bytes())))
}
fn exchange(address: SocketAddr, request: &[u8]) -> String {
    let mut stream = TcpStream::connect(address).unwrap();
    stream.write_all(request).unwrap();
    let mut bytes = String::new();
    stream.read_to_string(&mut bytes).unwrap();
    bytes
}
fn post(address: SocketAddr, path: &str, headers: &str, body: &str) -> String {
    exchange(
        address,
        format!("POST {path} HTTP/1.1\r\nHost: {address}\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes(),
    )
}
fn json_auth(token: &str) -> String {
    format!("Content-Type: application/json\r\nAuthorization: Bearer {token}\r\n")
}
fn body(reply: &str) -> Value {
    serde_json::from_str(reply.split("\r\n\r\n").nth(1).unwrap()).unwrap()
}
fn attr(key: &str, value: &str) -> Value {
    json!({"key": key, "value": {"stringValue": value}})
}
fn logs(records: Vec<Value>) -> String {
    json!({"resourceLogs": [{
        "resource": {"attributes": [attr("swarm.agent_id", "b")]},
        "scopeLogs": [{"logRecords": records}]
    }]})
    .to_string()
}

#[test]
fn otel_ingest_attaches_to_token_agent_is_idempotent_and_refuses_each_variant() {
    let (p, work) = root();
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
    let agent_view = |id: &str| {
        let reply = exchange(address, format!("GET /api/v1/live/agent?agent={id}&work={work} HTTP/1.1\r\nHost: {address}\r\nCookie: {cookie}\r\nConnection: close\r\n\r\n").as_bytes());
        assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");
        body(&reply)["agent"]["telemetry"].clone()
    };
    // Before any token: no store, explicit state, every request refused.
    assert_eq!(agent_view("a")["state"], "no-token");
    let early = post(address, "/api/otel/v1/logs", &json_auth(&"a".repeat(64)), "{}");
    assert!(early.starts_with("HTTP/1.1 401") && early.contains("telemetry-token-invalid"), "{early}");

    let store = TelemetryStore::open(&p, true).unwrap();
    let token_a = store.issue_token("a", 1).unwrap();
    let token_c = store.issue_token("c", 1).unwrap();
    store.close().unwrap();
    assert_eq!(agent_view("a")["state"], "waiting");

    // Witness: token A, body claiming agent b, plus a content attribute.
    let call = json!({"timeUnixNano": "1700000000000000000", "body": {"stringValue": "claude_code.api_request"},
        "attributes": [attr("model", "m1"), attr("input_tokens", "12"), attr("prompt", "SECRET-PROMPT")]});
    let tool = json!({"timeUnixNano": "1700000001000000000", "body": {"stringValue": "claude_code.tool_result"},
        "attributes": [attr("tool_name", "Bash"), attr("success", "false"), attr("duration_ms", "40")]});
    let witness = logs(vec![call.clone(), tool]);
    let fingerprint = || body(&exchange(address, format!("GET /api/v1/live/fingerprint HTTP/1.1\r\nHost: {address}\r\nCookie: {cookie}\r\nConnection: close\r\n\r\n").as_bytes()))["fingerprint"].clone();
    let before = fingerprint();
    let ok = post(address, "/api/otel/v1/logs", &json_auth(&token_a), &witness);
    assert_ne!(fingerprint(), before, "telemetry arrival changes the live fingerprint");
    assert!(ok.starts_with("HTTP/1.1 200"), "{ok}");
    assert_eq!(body(&ok), json!({}));
    let replay = post(address, "/api/otel/v1/logs", &json_auth(&token_a), &witness);
    assert!(replay.starts_with("HTTP/1.1 200"), "{replay}");
    let a = agent_view("a");
    assert_eq!(a["state"], "received");
    assert_eq!(a["provenance"], "provider-declared-unauthenticated");
    assert_eq!(a["calls"].as_array().unwrap().len(), 1, "replay is idempotent: {a}");
    assert_eq!(a["calls"][0]["fields"]["input_tokens"], "12");
    assert_eq!(a["tools"][0]["failures"], 1);
    assert!(!a.to_string().contains("SECRET-PROMPT"));
    assert_eq!(agent_view("b")["state"], "no-token", "body attribute never attaches");

    // Partial success and metrics.
    let partial = post(address, "/api/otel/v1/logs", &json_auth(&token_a), &logs(vec![json!(1), json!({"body": {"stringValue": "codex.new_kind"}})]));
    assert!(partial.starts_with("HTTP/1.1 200"), "{partial}");
    assert_eq!(body(&partial)["partialSuccess"]["rejectedLogRecords"], "1");
    let metrics = json!({"resourceMetrics": [{"scopeMetrics": [{"metrics": [{"name": "claude_code.token.usage",
        "sum": {"aggregationTemporality": 1, "dataPoints": [{"timeUnixNano": "1", "asDouble": 30, "attributes": [attr("type", "output")]}]}}]}]}]}).to_string();
    let m = post(address, "/api/otel/v1/metrics", &json_auth(&token_a), &metrics);
    assert!(m.starts_with("HTTP/1.1 200"), "{m}");
    let a = agent_view("a");
    assert_eq!(a["totals"][0]["value"], 30.0);
    assert_eq!(a["unknown"]["new_kind"], 1);
    assert_eq!(a["rejected"], 1);

    // Refusals, each with its stable status and code.
    let cases = [
        (post(address, "/api/otel/v1/logs", "Content-Type: application/json\r\n", "{}"), "401", "telemetry-token-invalid"),
        (post(address, "/api/otel/v1/logs", &format!("Cookie: {cookie}\r\nContent-Type: application/json\r\n"), "{}"), "401", "telemetry-token-invalid"),
        (post(address, "/api/otel/v1/logs", &json_auth(&"f".repeat(64)), "{}"), "401", "telemetry-token-invalid"),
        (post(address, "/api/otel/v1/logs", &json_auth(&token_c), "{}"), "401", "telemetry-token-invalid"),
        (post(address, "/api/otel/v1/logs", &format!("Content-Type: application/x-protobuf\r\nAuthorization: Bearer {token_a}\r\n"), "x"), "415", "telemetry-format-unsupported"),
        (post(address, "/api/otel/v1/logs", &format!("{}Content-Encoding: gzip\r\n", json_auth(&token_a)), "x"), "415", "telemetry-format-unsupported"),
        (post(address, "/api/otel/v1/metrics", &json_auth(&token_a), "not json"), "400", "telemetry-body-invalid"),
        (exchange(address, format!("GET /api/otel/v1/logs HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n").as_bytes()), "405", "telemetry-method-invalid"),
        (exchange(address, format!("POST /api/otel/v1/logs HTTP/1.1\r\nHost: evil.example\r\n{}Content-Length: 2\r\nConnection: close\r\n\r\n{{}}", json_auth(&token_a)).as_bytes()), "403", "web-access-refused"),
    ];
    for (reply, status, code) in &cases {
        assert!(reply.starts_with(&format!("HTTP/1.1 {status}")) && reply.contains(code), "{status} {code}: {reply}");
    }
    // Revocation: explicit, and implicit when the agent stops being active.
    let store = TelemetryStore::open(&p, false).unwrap();
    assert_eq!(store.revoke_agent("a", 2).unwrap(), 1);
    store.close().unwrap();
    let revoked = post(address, "/api/otel/v1/logs", &json_auth(&token_a), &witness);
    assert!(revoked.starts_with("HTTP/1.1 401"), "{revoked}");
    let store = TelemetryStore::open(&p, false).unwrap();
    let token_b = store.issue_token("b", 3).unwrap();
    store.close().unwrap();
    assert!(post(address, "/api/otel/v1/logs", &json_auth(&token_b), "{}").starts_with("HTTP/1.1 200"));
    Connection::open(p.join(".swarm/state.db")).unwrap().execute("UPDATE agents SET status='stopped' WHERE id='b'", []).unwrap();
    let stopped = post(address, "/api/otel/v1/logs", &json_auth(&token_b), "{}");
    assert!(stopped.starts_with("HTTP/1.1 401"), "{stopped}");
    // Points received before revocation stay readable.
    assert_eq!(agent_view("a")["calls"].as_array().unwrap().len(), 1);

    tx.send(()).unwrap();
    let report = handle.join().unwrap().unwrap();
    assert_eq!(report.connection_failures, 0);
    for name in ["state.db", "telemetry.db"] {
        let bytes = fs::read(p.join(".swarm").join(name)).unwrap();
        for secret in [&token_a, &token_b, &token_c] {
            assert!(!bytes.windows(64).any(|w| w == secret.as_bytes()), "token in {name}");
        }
        assert!(!bytes.windows(13).any(|w| w == b"SECRET-PROMPT"), "content in {name}");
    }
    fs::remove_dir_all(&p).unwrap();
}
