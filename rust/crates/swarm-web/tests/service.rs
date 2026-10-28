#![cfg(target_os = "linux")]
//! In-process typed HTTP→actual Root contracts. No network/listener loop is claimed.
use http::{
    header::{COOKIE, HOST, ORIGIN},
    Request, StatusCode,
};
use std::{error::Error, fs, net::TcpListener, path::PathBuf};
use swarm_engine::{RootStore, SystemEnvironment};
use swarm_web::{LocalWebAuthority, NativeHttpService};
const KEY: &str = "session-0123456789abcdef01234567";
struct Fixture {
    root: PathBuf,
    listener: TcpListener,
    work: String,
}
impl Fixture {
    /// Public operations establish meaningful isolated Work/tasks before HTTP faults.
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(swarm_engine::generate_identifier("web-root-").unwrap());
        fs::create_dir(&root).unwrap();
        let mut s = RootStore::initialize(&root, &mut SystemEnvironment).unwrap();
        let work=s.execute_external("","work.create",br#"{"schema_version":1,"event_id":"create","expected_revision":0,"title":"Web","objective":"Preserve Root guards","scope":"Fixture","criteria":["Observable state"]}"#,&mut SystemEnvironment).unwrap().result.work.id;
        for (id, revision) in [("task", 1), ("other", 2)] {
            let r = serde_json::json!({"schema_version":1,"event_id":format!("add-{id}"),"expected_revision":revision,"id":id,"title":id,"deliverable":"Report","criteria":["Proof"]});
            s.execute_external(
                &work,
                "task.add",
                &serde_json::to_vec(&r).unwrap(),
                &mut SystemEnvironment,
            )
            .unwrap();
        }
        s.close().unwrap();
        Self {
            root,
            listener: TcpListener::bind("127.0.0.1:0").unwrap(),
            work,
        }
    }
    /// Observe real committed model/history through the public native store.
    fn snapshot(&self) -> (swarm_engine::Work, Vec<swarm_engine::RecordedEvent>) {
        let s = RootStore::open(&self.root, false).unwrap();
        let value = (
            s.read_work(&self.work).unwrap(),
            s.events(&self.work).unwrap(),
        );
        s.close().unwrap();
        value
    }
    /// Trusted listener/credential fixture supplies authority, never request JSON.
    fn authority(&self) -> LocalWebAuthority {
        LocalWebAuthority::from_listener(&self.listener, KEY).unwrap()
    }
    /// Typical existing Go task action, including nested values the server derives.
    fn action(&self, event: &str) -> serde_json::Value {
        serde_json::json!({"kind":"task","work":self.work,"task":"task","event_id":event,"expected_revision":self.snapshot().0.revision,"request":{"title":"Changed through HTTP","id":"other","schema_version":99,"expected_revision":999,"event_id":"forged","origin":"engine"}})
    }
    /// Build exact typed protected request; body bytes are preserved for framing tests.
    fn request(&self, method: &str, path: &str, body: Vec<u8>) -> Request<Vec<u8>> {
        let a = self.authority();
        let mut r = Request::builder()
            .method(method)
            .uri(path)
            .header(HOST, a.host())
            .header(COOKIE, format!("{}={KEY}", a.cookie_name()))
            .body(body)
            .unwrap();
        if method != "GET" && method != "HEAD" {
            r.headers_mut()
                .insert(ORIGIN, format!("http://{}", a.host()).parse().unwrap());
            r.headers_mut().insert("x-swarm-csrf", KEY.parse().unwrap());
        }
        r
    }
}
impl Drop for Fixture {
    /// Keep failed roots for diagnosis; delete only this owned root on success.
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.root).unwrap();
        }
    }
}
/// Valid mutation witness before auth faults; server derives task/event/revision/
/// schema/origin and Root commits/replays exactly once. Body is the actual Work.
#[test]
fn protected_task_action_uses_root_and_replays_without_forged_origin() {
    let f = Fixture::new();
    let service = NativeHttpService::open(&f.root, f.authority()).unwrap();
    let body = serde_json::to_vec(&f.action("change")).unwrap();
    let reply = service
        .handle(f.request("POST", "/api/v1/action", body.clone()))
        .unwrap();
    assert_eq!(reply.response.status(), StatusCode::OK);
    assert!(reply.failure.is_none());
    let work: serde_json::Value = serde_json::from_slice(reply.response.body()).unwrap();
    assert_eq!(work["revision"], 4);
    assert_eq!(reply.recorded_work, Some((f.work.clone(), 4)));
    assert!(!reply.configuration_may_be_recorded);
    assert_eq!(work["tasks"][0]["title"], "Changed through HTTP");
    assert_eq!(work["tasks"][1]["title"], "other");
    let after = f.snapshot();
    assert_eq!(after.1.len(), 4);
    let replay = service
        .handle(f.request("POST", "/api/v1/action", body))
        .unwrap();
    assert_eq!(replay.response.body(), reply.response.body());
    assert_eq!(replay.recorded_work, reply.recorded_work);
    assert_eq!(f.snapshot(), after);
    let db = rusqlite::Connection::open_with_flags(
        f.root.join(".swarm/state.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let intent: Vec<u8> = db
        .query_row(
            "SELECT CAST(request AS BLOB) FROM events WHERE id='change'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    db.close().unwrap();
    let intent: serde_json::Value = serde_json::from_slice(&intent).unwrap();
    assert_eq!(intent["schema_version"], 1);
    assert_eq!(intent["id"], "task");
    assert_eq!(intent["event_id"], "change");
    assert_eq!(intent["expected_revision"], 3);
    assert!(intent.get("origin").is_none() || intent["origin"] == "");
    for field in ["cookie", "origin", "x-swarm-csrf"] {
        let before = f.snapshot();
        let mut r = f.request(
            "POST",
            "/api/v1/action",
            serde_json::to_vec(&f.action(&format!("denied-{field}"))).unwrap(),
        );
        r.headers_mut().remove(field);
        let reply = service.handle(r).unwrap();
        assert_eq!(reply.response.status(), StatusCode::FORBIDDEN);
        assert!(reply.failure.is_some());
        assert_eq!(f.snapshot(), before);
    }
    service.close().unwrap();
}
/// Cookie/CSRF success is insufficient: stale revision and task lifecycle guards
/// still refuse without events/model mutation, then a fresh operator edit recovers.
#[test]
fn protected_root_refusals_are_not_empty_success_and_recover() {
    let f = Fixture::new();
    let service = NativeHttpService::open(&f.root, f.authority()).unwrap();
    let valid = f.action("witness");
    assert_eq!(
        service
            .handle(f.request(
                "POST",
                "/api/v1/action",
                serde_json::to_vec(&valid).unwrap()
            ))
            .unwrap()
            .response
            .status(),
        StatusCode::OK
    );
    let before = f.snapshot();
    let mut stale = f.action("stale");
    stale["expected_revision"] = 1.into();
    let reply = service
        .handle(f.request(
            "POST",
            "/api/v1/action",
            serde_json::to_vec(&stale).unwrap(),
        ))
        .unwrap();
    assert_eq!(reply.response.status(), StatusCode::CONFLICT);
    assert_eq!(reply.failure.as_ref().unwrap().code(), "revision_conflict");
    assert_eq!(f.snapshot(), before);
    let mut invalid = f.action("invalid-lifecycle");
    invalid["request"] = serde_json::json!({"status":"accepted"});
    let reply = service
        .handle(f.request(
            "POST",
            "/api/v1/action",
            serde_json::to_vec(&invalid).unwrap(),
        ))
        .unwrap();
    assert!(!reply.response.status().is_success());
    assert!(reply.failure.is_some());
    assert_eq!(f.snapshot(), before);
    let recovered = service
        .handle(f.request(
            "POST",
            "/api/v1/action",
            serde_json::to_vec(&f.action("recovered")).unwrap(),
        ))
        .unwrap();
    assert_eq!(recovered.response.status(), StatusCode::OK);
    assert_eq!(f.snapshot().0.revision, before.0.revision + 1);
    service.close().unwrap();
}
/// Request methods, unsupported operations, JSON framing/types and common input
/// transport bounds refuse visibly, preserving committed Work/event state.
#[test]
fn route_method_schema_and_body_limits_refuse_without_mutation() {
    let f = Fixture::new();
    let service = NativeHttpService::open(&f.root, f.authority()).unwrap();
    let valid = serde_json::to_vec(&f.action("valid")).unwrap();
    assert_eq!(
        service
            .handle(f.request("POST", "/api/v1/action", valid.clone()))
            .unwrap()
            .response
            .status(),
        StatusCode::OK
    );
    let before = f.snapshot();
    for (method, path, body, status) in [
        (
            "GET",
            "/api/v1/action",
            valid.clone(),
            StatusCode::METHOD_NOT_ALLOWED,
        ),
        (
            "POST",
            "/api/v1/unknown",
            valid.clone(),
            StatusCode::NOT_FOUND,
        ),
        (
            "POST",
            "/api/v1/action",
            b"{} {}".to_vec(),
            StatusCode::BAD_REQUEST,
        ),
        (
            "POST",
            "/api/v1/action",
            b"{\"kind\":\"start\"}".to_vec(),
            StatusCode::NOT_IMPLEMENTED,
        ),
        (
            "POST",
            "/api/v1/action",
            vec![b' '; swarm_engine::COMMAND_INPUT_MAX_BYTES + 1],
            StatusCode::PAYLOAD_TOO_LARGE,
        ),
    ] {
        let reply = service.handle(f.request(method, path, body)).unwrap();
        assert_eq!(reply.response.status(), status);
        assert!(reply.failure.is_some());
        assert!(!reply.response.body().is_empty());
        assert_eq!(f.snapshot(), before);
    }
    let mut invalid = f.action("bad-field");
    invalid["actor"] = serde_json::json!("forged");
    let reply = service
        .handle(f.request(
            "POST",
            "/api/v1/action",
            serde_json::to_vec(&invalid).unwrap(),
        ))
        .unwrap();
    assert_eq!(reply.response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(f.snapshot(), before);
    service.close().unwrap();
}
/// Protected list is complete, read-only and JSON; capabilities disclose the
/// native common transport bound/limited profile, never credential/session data.
#[test]
fn list_capabilities_and_head_expose_only_observed_supported_contract() {
    let f = Fixture::new();
    let service = NativeHttpService::open(&f.root, f.authority()).unwrap();
    let before = f.snapshot();
    let reply = service
        .handle(f.request("GET", "/api/v1/works", vec![]))
        .unwrap();
    assert_eq!(reply.response.status(), StatusCode::OK);
    let list: Vec<swarm_engine::Work> = serde_json::from_slice(reply.response.body()).unwrap();
    assert_eq!(list, vec![before.0.clone()]);
    let reply = service
        .handle(f.request("GET", "/api/v1/native-capabilities", vec![]))
        .unwrap();
    assert_eq!(reply.response.status(), StatusCode::OK);
    let v: serde_json::Value = serde_json::from_slice(reply.response.body()).unwrap();
    assert_eq!(
        v["command_input_transport"]["max_bytes"],
        swarm_engine::COMMAND_INPUT_MAX_BYTES.to_string()
    );
    assert_eq!(v["actions"], serde_json::json!(["task"]));
    assert!(!String::from_utf8_lossy(reply.response.body()).contains(KEY));
    assert_eq!(reply.response.headers()["cache-control"], "no-store");
    assert_eq!(
        reply.response.headers()["x-content-type-options"],
        "nosniff"
    );
    assert!(!reply.response.headers()["content-security-policy"].is_empty());
    let head = service
        .handle(f.request("HEAD", "/api/v1/works", vec![]))
        .unwrap();
    assert_eq!(head.response.status(), StatusCode::OK);
    assert!(head.response.body().is_empty());
    assert_eq!(f.snapshot(), before);
    service.close().unwrap();
}
/// Trusted bootstrap checks Root once; denied requests still precede all business
/// Root opens/body decode and cannot reveal a moved path or initialize anything.
#[test]
fn authentication_precedes_business_operations_and_json_decode_after_bootstrap() {
    let f = Fixture::new();
    let service = NativeHttpService::open(&f.root, f.authority()).unwrap();
    let held = f.root.with_extension("owned-hidden");
    assert!(!held.exists());
    fs::rename(&f.root, &held).unwrap();
    let mut request = f.request("POST", "/api/v1/action", b"PRIVATE invalid JSON".to_vec());
    request.headers_mut().remove(COOKIE);
    let observation = service.handle(request);
    fs::rename(&held, &f.root).unwrap();
    let reply = observation.unwrap();
    assert_eq!(reply.response.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        reply.failure.as_ref().unwrap().code(),
        "web-session-required"
    );
    assert!(!String::from_utf8_lossy(reply.response.body()).contains("PRIVATE"));
    service.close().unwrap();
}

/// Go outer field casing/nulls and duplicate value-struct merging use the same
/// canonical Request producer shape. A malformed number still refuses before
/// Root mutation; quoted decimal is not silently accepted as a revision.
#[test]
fn action_wire_go_case_null_and_duplicate_struct_merge_reach_the_real_task() {
    let f = Fixture::new();
    let service = NativeHttpService::open(&f.root, f.authority()).unwrap();
    let raw = format!(
        r#"{{"kind":"task","KIND":null,"WORK":"{}","task":"task","event_id":"merged","expected_revision":3,"EXPECTED_REVISION":null,"request":{{"title":"First survives null","origin":"engine"}},"REQUEST":{{"title":null}}}}"#,
        f.work
    );
    let reply = service
        .handle(f.request("POST", "/api/v1/action", raw.into_bytes()))
        .unwrap();
    assert_eq!(reply.response.status(), StatusCode::OK);
    assert_eq!(
        f.snapshot().0.tasks.as_ref().unwrap()[0].title,
        "First survives null"
    );
    let before = f.snapshot();
    let mut invalid = f.action("quoted-number");
    invalid["expected_revision"] = serde_json::json!("4");
    let reply = service
        .handle(f.request(
            "POST",
            "/api/v1/action",
            serde_json::to_vec(&invalid).unwrap(),
        ))
        .unwrap();
    assert_eq!(reply.response.status(), StatusCode::BAD_REQUEST);
    assert!(reply.failure.as_ref().unwrap().source().is_some());
    assert_eq!(f.snapshot(), before);
    service.close().unwrap();
}

/// Go listWorkRows explicitly initializes []Work{}; a legitimate empty Root is
/// [], whereas an unavailable/corrupt Root remains a visible failure, not [].
#[test]
fn initialized_empty_work_list_is_distinct_from_root_failure() {
    let root = std::env::temp_dir().join(swarm_engine::generate_identifier("web-empty-").unwrap());
    fs::create_dir(&root).unwrap();
    RootStore::initialize(&root, &mut SystemEnvironment)
        .unwrap()
        .close()
        .unwrap();
    let f = Fixture {
        root,
        listener: TcpListener::bind("127.0.0.1:0").unwrap(),
        work: String::new(),
    };
    let service = NativeHttpService::open(&f.root, f.authority()).unwrap();
    let reply = service
        .handle(f.request("GET", "/api/v1/works", vec![]))
        .unwrap();
    assert_eq!(reply.response.status(), StatusCode::OK);
    assert_eq!(reply.response.body(), b"[]");
    assert!(reply.failure.is_none());
    service.close().unwrap();
    let bad = f.root.join("missing-root");
    let failure = NativeHttpService::open(&bad, f.authority()).err().unwrap();
    assert!(failure.source().is_some());
    assert!(!bad.exists());
}

/// A capability configured for one project must not silently open another project
/// when its pathname is replaced. Both roots/moves belong only to this fixture.
/// Restore original names before asserting, retaining original failure evidence.
#[test]
fn configured_project_does_not_follow_a_replaced_root_path() {
    let a = Fixture::new();
    let b = Fixture::new();
    let service = NativeHttpService::open(&a.root, a.authority()).unwrap();
    let witness = service
        .handle(a.request("GET", "/api/v1/works", vec![]))
        .unwrap();
    assert_eq!(witness.response.status(), StatusCode::OK);
    let valid_write = service
        .handle(a.request(
            "POST",
            "/api/v1/action",
            serde_json::to_vec(&a.action("valid-before-swap")).unwrap(),
        ))
        .unwrap();
    assert_eq!(valid_write.response.status(), StatusCode::OK);
    let before_b = b.snapshot();
    let foreign_action = serde_json::to_vec(&b.action("foreign-write")).unwrap();
    let held = a.root.with_extension("owned-parked");
    assert!(!held.exists());
    fs::rename(&a.root, &held).unwrap();
    fs::rename(&b.root, &a.root).unwrap();
    let observation = service.handle(a.request("GET", "/api/v1/works", vec![]));
    let foreign_write = service.handle(a.request("POST", "/api/v1/action", foreign_action));
    fs::rename(&a.root, &b.root).unwrap();
    fs::rename(&held, &a.root).unwrap();
    let foreign_write = foreign_write.unwrap();
    assert_eq!(foreign_write.response.status(), StatusCode::CONFLICT);
    assert_eq!(
        foreign_write.failure.as_ref().unwrap().code(),
        "web-root-binding-changed"
    );
    assert_eq!(b.snapshot(), before_b);
    let reply = observation.unwrap();
    assert_eq!(
        reply.response.status(),
        StatusCode::CONFLICT,
        "old project capability followed a replacement project: {}",
        String::from_utf8_lossy(reply.response.body())
    );
    assert_eq!(
        reply.failure.as_ref().unwrap().code(),
        "web-root-binding-changed"
    );
    let recovered = service
        .handle(a.request("GET", "/api/v1/works", vec![]))
        .unwrap();
    assert_eq!(recovered.response.status(), StatusCode::OK);
    let works: Vec<swarm_engine::Work> = serde_json::from_slice(recovered.response.body()).unwrap();
    assert_eq!(works[0].id, a.work);
    service.close().unwrap();
}

/// Actual Root session bootstrap supplies a gate; real disk bytes authorize typed requests.
/// No TCP HTTP traffic, browser login or authenticated human/agent is claimed.
#[test]
fn persisted_session_bootstrap_authorizes_then_refuses_csrf_and_preserves_work() {
    use sha2::{Digest, Sha256};
    let f = Fixture::new();
    let before = f.snapshot();
    let service = NativeHttpService::open_local(&f.root, &f.listener).unwrap();
    let host = f.listener.local_addr().unwrap().to_string();
    let hash = format!("{:x}", Sha256::digest(host.as_bytes()));
    let path = f
        .root
        .join(".swarm")
        .join(format!("web-session-{}", &hash[..16]));
    let token = fs::read_to_string(&path).unwrap();
    let authority = LocalWebAuthority::from_listener(&f.listener, &token).unwrap();
    let request = || {
        Request::builder()
            .method("GET")
            .uri("/api/v1/works")
            .header(HOST, &host)
            .header(COOKIE, format!("{}={token}", authority.cookie_name()))
            .body(Vec::new())
            .unwrap()
    };
    let reply = service.handle(request()).unwrap();
    assert_eq!(reply.response.status(), StatusCode::OK);
    assert!(reply.failure.is_none());
    let works: Vec<swarm_engine::Work> = serde_json::from_slice(reply.response.body()).unwrap();
    assert_eq!(works.len(), 1);
    assert_eq!(works[0].id, f.work);
    let denied = Request::builder()
        .method("POST")
        .uri("/api/v1/action")
        .header(HOST, &host)
        .header(COOKIE, format!("{}={token}", authority.cookie_name()))
        .header(ORIGIN, format!("http://{host}"))
        .body(serde_json::to_vec(&f.action("denied-session")).unwrap())
        .unwrap();
    let reply = service.handle(denied).unwrap();
    assert_eq!(reply.response.status(), StatusCode::FORBIDDEN);
    assert!(reply.failure.is_some());
    assert_eq!(f.snapshot(), before);
    service.close().unwrap();
    let service = NativeHttpService::open_local(&f.root, &f.listener).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), token);
    assert_eq!(
        service.handle(request()).unwrap().response.status(),
        StatusCode::OK
    );
    service.close().unwrap();
    let r = RootStore::open(&f.root, true).unwrap();
    let session = r.web_session(&f.listener).unwrap();
    assert!(LocalWebAuthority::from_session(&f.listener, &session).is_ok());
    let other = TcpListener::bind("127.0.0.1:0").unwrap();
    assert_eq!(
        LocalWebAuthority::from_session(&other, &session)
            .unwrap_err()
            .code(),
        "web-session-listener-mismatch"
    );
    r.close().unwrap();
    fs::write(&path, b"invalid-existing-session").unwrap();
    let error = match NativeHttpService::open_local(&f.root, &f.listener) {
        Ok(s) => {
            s.close().unwrap();
            panic!("invalid session accepted")
        }
        Err(e) => e,
    };
    assert_eq!(error.code(), "web-session-credential-invalid");
    assert!(!error.session_state_may_be_recorded());
    assert_eq!(fs::read(&path).unwrap(), b"invalid-existing-session");
    assert_eq!(f.snapshot(), before);
    fs::write(&path, &token).unwrap();
    NativeHttpService::open_local(&f.root, &f.listener)
        .unwrap()
        .close()
        .unwrap();
    // Existing Fixture::drop checks owned cleanup after this scope.
}
