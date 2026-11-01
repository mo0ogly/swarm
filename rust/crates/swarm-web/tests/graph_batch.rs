#![cfg(target_os = "linux")]
//! Typed HTTP over real private Roots. Listener framing/browser are verified by
//! the separate public CLI/browser recipe; no provider or live data is involved.
use http::{
    header::{COOKIE, HOST, ORIGIN},
    Request, StatusCode,
};
use serde_json::{json, Value};
use std::{fs, net::TcpListener, path::PathBuf};
use swarm_engine::{RootStore, SystemEnvironment, WorkEnvironment};
use swarm_web::{LocalWebAuthority, NativeHttpService};
const KEY: &str = "session-0123456789abcdef01234567";
struct Fixture {
    path: PathBuf,
    listener: TcpListener,
    work: String,
}
impl Fixture {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(swarm_engine::generate_identifier("graph-http-").unwrap());
        fs::create_dir(&path).unwrap();
        let mut s = RootStore::initialize(&path, &mut SystemEnvironment).unwrap();
        let work=s.execute_external("","work.create",br#"{"schema_version":1,"event_id":"create","expected_revision":0,"title":"Graph","objective":"Atomic HTTP","scope":"Private","criteria":["Observed"]}"#,&mut SystemEnvironment).unwrap().result.work.id;
        s.close().unwrap();
        Self {
            path,
            listener: TcpListener::bind("127.0.0.1:0").unwrap(),
            work,
        }
    }
    fn authority(&self) -> LocalWebAuthority {
        LocalWebAuthority::from_listener(&self.listener, KEY).unwrap()
    }
    fn request(&self, path: &str, value: &Value) -> Request<Vec<u8>> {
        let a = self.authority();
        Request::builder()
            .method("POST")
            .uri(path)
            .header(HOST, a.host())
            .header(COOKIE, format!("{}={KEY}", a.cookie_name()))
            .header(ORIGIN, format!("http://{}", a.host()))
            .header("x-swarm-csrf", KEY)
            .body(serde_json::to_vec(value).unwrap())
            .unwrap()
    }
    fn base(&self) -> Value {
        json!({"work":self.work,"request":{"schema_version":1,"event_id":"batch","expected_revision":1,"changes":[{"before":null,"after":{"id":"a","title":"A","deliverable":"docs/a","criteria":["Proof"],"depends":[]}},{"before":null,"after":{"id":"b","title":"B","deliverable":"docs/b","criteria":["Proof"],"depends":["a"]}}]}})
    }
    fn counts(&self) -> (i64, usize) {
        let s = RootStore::open(&self.path, false).unwrap();
        let counts = (
            s.read_work(&self.work).unwrap().revision,
            s.events(&self.work).unwrap().len(),
        );
        s.close().unwrap();
        counts
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.path).unwrap()
        }
    }
}
fn call(service: &NativeHttpService, request: Request<Vec<u8>>) -> (StatusCode, Value) {
    let r = service.handle(request).unwrap().response;
    (r.status(), serde_json::from_slice(r.body()).unwrap())
}

#[test]
fn preview_apply_replay_and_compensation_use_one_atomic_root_operation() {
    let f = Fixture::new();
    let service = NativeHttpService::open(&f.path, f.authority()).unwrap();
    let mut r = f.base();
    let (status, p) = call(&service, f.request("/api/v1/graph-batch/preview", &r));
    assert_eq!(status, StatusCode::OK);
    assert_eq!(p["changes"].as_array().unwrap().len(), 2);
    assert_eq!(f.counts(), (1, 1));
    assert_eq!(
        call(&service, f.request("/api/v1/graph-batch/apply", &r)).0,
        StatusCode::CONFLICT
    );
    assert_eq!(f.counts(), (1, 1));
    r["request"]["preview_digest"] = p["preview_digest"].clone();
    let (status, applied) = call(&service, f.request("/api/v1/graph-batch/apply", &r));
    assert_eq!(status, StatusCode::OK);
    assert_eq!(applied["event_id"], "batch");
    assert_eq!(applied["applied_revision"], 2);
    assert_eq!(applied["work"]["tasks"].as_array().unwrap().len(), 2);
    assert_eq!(applied["replayed"], false);
    assert_eq!(f.counts(), (2, 2));
    // Complete definitions cannot use the legacy task.update empty-as-absent
    // semantics. Both protected routes must expose a typed refusal, no event.
    let task: swarm_engine::Task =
        serde_json::from_value(applied["work"]["tasks"][0].clone()).unwrap();
    let before = swarm_engine::GraphTaskDefinition::observed(&task);
    for (field, reason) in [
        ("title", "task-title-blank"),
        ("deliverable", "task-deliverable-blank"),
    ] {
        for value in ["", " \t\n"] {
            let mut after = serde_json::to_value(&before).unwrap();
            after[field] = value.into();
            let invalid = json!({"work":f.work,"request":{"schema_version":1,"event_id":"blank-definition","expected_revision":2,"changes":[{"before":before,"after":after}],"preview_digest":"0".repeat(64)}});
            for route in ["/api/v1/graph-batch/preview", "/api/v1/graph-batch/apply"] {
                let (status, refusal) = call(&service, f.request(route, &invalid));
                assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
                assert_eq!(refusal["failure"]["reason"], reason);
                assert_eq!(f.counts(), (2, 2));
            }
        }
    }
    assert_eq!(
        call(&service, f.request("/api/v1/graph-batch/apply", &r)).1["replayed"],
        true
    );
    assert_eq!(f.counts(), (2, 2));
    let mut undo = json!({"work":f.work,"request":{"schema_version":1,"event_id":"undo","expected_revision":2,"changes":[],"undo_of":"batch"}});
    let (status, p) = call(&service, f.request("/api/v1/graph-batch/preview", &undo));
    assert_eq!(status, StatusCode::OK);
    assert!(p["changes"]
        .as_array()
        .unwrap()
        .iter()
        .all(|c| c["after"].is_null()));
    undo["request"]["preview_digest"] = p["preview_digest"].clone();
    let (status, v) = call(&service, f.request("/api/v1/graph-batch/apply", &undo));
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v["work"]["revision"], 3);
    assert!(v["work"]["tasks"].as_array().unwrap().is_empty());
    assert_eq!(f.counts(), (3, 3));
    service.close().unwrap();
}
#[test]
fn csrf_unknown_fields_and_invalid_cycle_refuse_without_any_event() {
    let f = Fixture::new();
    let service = NativeHttpService::open(&f.path, f.authority()).unwrap();
    let r = f.base();
    assert_eq!(
        call(&service, f.request("/api/v1/graph-batch/preview", &r)).0,
        StatusCode::OK
    );
    let mut denied = f.request("/api/v1/graph-batch/apply", &r);
    denied.headers_mut().remove("x-swarm-csrf");
    assert_eq!(call(&service, denied).0, StatusCode::FORBIDDEN);
    let mut unknown = r.clone();
    unknown["request"]["origin"] = "engine".into();
    assert_eq!(
        call(&service, f.request("/api/v1/graph-batch/preview", &unknown)).0,
        StatusCode::BAD_REQUEST
    );
    unknown = r.clone();
    unknown["request"]["changes"][0]["after"]["status"] = "accepted".into();
    assert_eq!(
        call(&service, f.request("/api/v1/graph-batch/preview", &unknown)).0,
        StatusCode::BAD_REQUEST
    );
    let mut cycle = r;
    cycle["request"]["changes"][0]["after"]["depends"] = json!(["b"]);
    assert_eq!(
        call(&service, f.request("/api/v1/graph-batch/preview", &cycle)).0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(f.counts(), (1, 1));
    service.close().unwrap();
}
#[test]
fn graph_rights_are_not_bypassed_by_the_new_route_or_replay() {
    let f = Fixture::new();
    let service = NativeHttpService::open(&f.path, f.authority()).unwrap();
    let mut r = f.base();
    let (status, p) = call(&service, f.request("/api/v1/graph-batch/preview", &r));
    assert_eq!(status, StatusCode::OK);
    r["request"]["preview_digest"] = p["preview_digest"].clone();
    let db = rusqlite::Connection::open(f.path.join(".swarm/state.db")).unwrap();
    let actor = SystemEnvironment.operator().unwrap();
    db.execute("INSERT INTO graph_draft_authorizations(work_id,actor,revision,prepare,apply,scopes,updated) VALUES(?,?,1,1,0,?,?)",rusqlite::params![f.work,actor,b"null","fixture"]).unwrap();
    assert_eq!(
        call(&service, f.request("/api/v1/graph-batch/apply", &r)).0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(f.counts(), (1, 1));
    db.execute(
        "UPDATE graph_draft_authorizations SET revision=2,apply=1",
        [],
    )
    .unwrap();
    let (_, p) = call(&service, f.request("/api/v1/graph-batch/preview", &r));
    r["request"]["preview_digest"] = p["preview_digest"].clone();
    assert_eq!(
        call(&service, f.request("/api/v1/graph-batch/apply", &r)).0,
        StatusCode::OK
    );
    db.execute(
        "UPDATE graph_draft_authorizations SET revision=3,apply=0",
        [],
    )
    .unwrap();
    assert_eq!(
        call(&service, f.request("/api/v1/graph-batch/apply", &r)).0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(f.counts(), (2, 2));
    db.close().unwrap();
    service.close().unwrap();
}
