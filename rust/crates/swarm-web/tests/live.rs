#![cfg(target_os = "linux")]
//! Read-only live observation routes over an owned private Root; never live data.
use http::{
    header::{CONTENT_TYPE, COOKIE, HOST},
    Request, StatusCode,
};
use rusqlite::{params, Connection};
use serde_json::json;
use std::{fs, net::TcpListener, os::unix::fs::PermissionsExt, path::PathBuf};
use swarm_engine::{encode_model, Agent, RootStore, SystemEnvironment};
// Explicit fixture count matches the versioned live-log default, not a shared
// production constant or proof of telemetry policy.
const TEST_LOG_PAGE_ROWS:usize=100;
use swarm_web::{HttpReply, LocalWebAuthority, NativeHttpService};

struct Fixture {
    root: PathBuf,
    service: Option<NativeHttpService>,
    host: String,
    cookie: String,
    csrf: String,
    works: [String; 2],
}
impl Fixture {
    /// Two Works; agents in several statuses, logs and PTY chunks seeded
    /// directly in the owned test database (fixture only, not a runtime path).
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(swarm_engine::generate_identifier("live-web-").unwrap());
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let mut r = RootStore::initialize(&root, &mut SystemEnvironment).unwrap();
        let mut create = |title: &str, event: &str| {
            let raw = format!(r#"{{"schema_version":1,"event_id":"{event}","expected_revision":0,"title":"{title}","objective":"Observe","scope":"Fixture","criteria":["Visible"]}}"#);
            r.execute_external("", "work.create", raw.as_bytes(), &mut SystemEnvironment)
                .unwrap()
                .result
                .work
                .id
        };
        let works = [create("Alpha", "a"), create("Beta", "b")];
        // Planned tasks of Alpha (initial phase): t2 depends on t1; t9 never starts.
        let mut revision = 1;
        for (id, depends) in [("t1", "[]"), ("t2", r#"["t1"]"#), ("t9", r#"["t2"]"#)] {
            let raw = format!(r#"{{"schema_version":1,"event_id":"add-{id}","expected_revision":{revision},"id":"{id}","title":"Task {id}","deliverable":"Report {id}","criteria":["Proof"],"depends":{depends}}}"#);
            revision = r
                .execute_external(&works[0], "task.add", raw.as_bytes(), &mut SystemEnvironment)
                .unwrap()
                .result
                .work
                .revision;
        }
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let key = r.web_session(&listener).unwrap().secret().to_owned();
        r.close().unwrap();
        let db = Connection::open(root.join(".swarm/state.db")).unwrap();
        let seed = |id: &str, work: &str, task: &str, status: &str, parent: &str| {
            let mut a = Agent::default();
            a.id = id.into();
            a.work_id = work.into();
            a.task_id = task.into();
            a.workspace = format!("/isolated/{id}");
            a.status = status.into();
            a.role = format!("role-{id}");
            a.parent = parent.into();
            a.provider = "fixture".into();
            a.prompt = "PRIVATE prompt".into();
            if id == "lead" {
                a.host = swarm_engine::SupervisorIdentity::current().unwrap().host().into();
                a.limits.max_tool_calls = 40;
                let memory = swarm_engine::MemoryReference {
                    id: "mem-1".into(),
                    version: 3,
                    sha256: "SECRET-DIGEST".into(),
                    ..Default::default()
                };
                a.context = Some(Box::new(swarm_engine::ContextManifest {
                    bytes: 2048,
                    included: Some(vec!["a.md".into(), "b.md".into()]),
                    memory: Some(vec![memory]),
                    ..Default::default()
                }));
                a.native_provider_facts = Some(swarm_engine::GoRaw(serde_json::value::RawValue::from_string(r#"{"schema_version":1,"source":"fixture","usage_state":"invalid","model_state":"observed","invalid_usage_events":2,"first_usage_error":"agent-usage-fixture","last_valid_usage":{"input_tokens":5}}"#.into()).unwrap()));
            }
            db.execute("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)", params![id, work, task, a.workspace, status, encode_model(&a).unwrap(), b"{}".as_slice()]).unwrap();
        };
        seed("lead", &works[0], "t1", "running", "");
        seed("child", &works[0], "t2", "completed", "lead");
        // Finished attempt with a retained Claude stream capture (terminal-cleaned).
        let capture = r#"{"type":"system","model":"m"} {"type":"assistant","message":{"usage":{"input_tokens":3},"content":[{"type":"tool_use","id":"u1","name":"Bash","input":{"command":"PRIVATE cmd"}}]}} {"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"u1","is_error":true,"content":"x"}]}}"#;
        let result = serde_json::json!({"schema_version": 1, "capture": capture, "capture_truncated": true});
        db.execute("UPDATE agents SET body = json_set(body, '$.native_execution_result', json(?)) WHERE id = 'child'", [result.to_string()]).unwrap();
        seed("other", &works[1], "t3", "failed", "");
        for i in 0..(TEST_LOG_PAGE_ROWS + 3) {
            db.execute(
                "INSERT INTO agent_logs(agent_id,at,kind,message) VALUES('lead','2026-10-21T10:00:00Z','message',?)",
                [format!("line {i}")],
            )
            .unwrap();
        }
        db.execute(
            "INSERT INTO agent_logs(agent_id,at,kind,message) VALUES('other','2026-10-21T10:00:00Z','message','not lead')",
            [],
        )
        .unwrap();
        db.execute(
            "INSERT INTO terminal_events(agent_id,data,cols,rows) VALUES('lead',?,80,24)",
            [b"\x1b[32m$ cargo test\r\n".to_vec()],
        )
        .unwrap();
        drop(db);
        let a = LocalWebAuthority::from_listener(&listener, &key).unwrap();
        let host = a.host().to_owned();
        let cookie = format!("{}={key}", a.cookie_name());
        let csrf = a.csrf_token().to_owned();
        let service = Some(NativeHttpService::open_local(&root, &listener).unwrap());
        Self {
            root,
            service,
            host,
            cookie,
            csrf,
            works,
        }
    }
    fn send(&self, method: &str, path: &str, authorized: bool) -> HttpReply {
        let mut b = Request::builder()
            .method(method)
            .uri(path)
            .header(HOST, &self.host);
        if authorized {
            b = b.header(COOKIE, &self.cookie)
        }
        self.service
            .as_ref()
            .unwrap()
            .handle(b.body(vec![]).unwrap())
            .unwrap()
    }
    /// POST with exact Origin and, when `csrf`, the session CSRF token.
    fn post(&self, path: &str, body: &[u8], csrf: bool) -> (StatusCode, serde_json::Value) {
        let mut b = Request::builder()
            .method("POST")
            .uri(path)
            .header(HOST, &self.host)
            .header(COOKIE, &self.cookie)
            .header("origin", format!("http://{}", self.host));
        if csrf {
            b = b.header("x-swarm-csrf", &self.csrf);
        }
        let reply = self.service.as_ref().unwrap().handle(b.body(body.to_vec()).unwrap()).unwrap();
        let status = reply.response.status();
        (status, serde_json::from_slice(reply.response.body()).unwrap_or(serde_json::Value::Null))
    }
    fn json(&self, path: &str) -> (StatusCode, serde_json::Value) {
        let reply = self.send("GET", path, true);
        let status = reply.response.status();
        (
            status,
            serde_json::from_slice(reply.response.body()).unwrap(),
        )
    }
    fn agent_path(&self, agent: &str, work: &str, extra: &str) -> String {
        format!("/api/v1/live/agent?agent={agent}&work={work}{extra}")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(s) = self.service.take() {
            s.close().unwrap()
        }
        fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
fn overview_lists_every_work_with_light_agent_views_and_counts() {
    let f = Fixture::new();
    let (status, body) = f.json("/api/v1/live/overview");
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["totals"]["running"], 1);
    assert_eq!(body["totals"]["completed"], 1);
    assert_eq!(body["totals"]["failed"], 1);
    let works = body["works"].as_array().unwrap();
    assert_eq!(works.len(), 2);
    let alpha = works.iter().find(|w| w["id"] == f.works[0]).unwrap();
    assert_eq!(alpha["title"], "Alpha");
    assert_eq!(alpha["counts"]["running"], 1);
    let agents = alpha["agents"].as_array().unwrap();
    assert_eq!(agents.len(), 2);
    let lead = agents.iter().find(|a| a["id"] == "lead").unwrap();
    assert_eq!(lead["active"], true);
    assert_eq!(lead["role"], "role-lead");
    assert_eq!(lead["provider_facts"]["usage_state"], "invalid");
    assert_eq!(lead["provider_facts"]["invalid_usage_events"], 2);
    assert_eq!(lead["provider_facts"]["first_usage_error"], "agent-usage-fixture");
    assert!(lead["provider_facts"].get("last_valid_usage").is_none(), "history stays out");
    let child = agents.iter().find(|a| a["id"] == "child").unwrap();
    assert_eq!(child["parent"], "lead");
    assert_eq!(child["active"], false);
    assert!(child.get("provider_facts").is_none(), "absent facts stay absent");
    assert_eq!(lead["limits"]["max_tool_calls"], 40);
    assert_eq!(lead["context"]["bytes"], 2048);
    assert_eq!(lead["context"]["included"], 2);
    assert_eq!(lead["context"]["memory"], 1);
    // Active agent on this host with no recorded PIDs: observed, nothing read.
    assert_eq!(lead["resources"]["observation"], "observed");
    assert_eq!(lead["resources"]["provider"]["state"], "not-recorded");
    assert!(lead["resources"]["provider"].get("rss_bytes").is_none());
    assert!(child.get("resources").is_none(), "finished agents are not probed in the overview");
    // Private/heavy fields never reach the polling payload.
    let text = serde_json::to_string(&body).unwrap();
    assert!(!text.contains("PRIVATE prompt"));
    assert!(!text.contains("/isolated/"));
}

#[test]
fn plan_graph_links_planned_tasks_to_live_agents() {
    let f = Fixture::new();
    let (status, body) = f.json("/api/v1/live/overview");
    assert_eq!(status, StatusCode::OK);
    let alpha = body["works"].as_array().unwrap().iter().find(|w| w["id"] == f.works[0]).unwrap();
    let plan = alpha["plan"].as_array().unwrap();
    let ids: Vec<_> = plan.iter().map(|t| t["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["t1", "t2", "t9"]);
    assert_eq!(plan[1]["depends"], serde_json::json!(["t1"]));
    assert_eq!(plan[2]["attempts"], 0, "never started");
    assert_eq!(plan[0]["deliverable"], "Report t1");
    assert!(!body.to_string().contains("PRIVATE prompt"));
    let (_, agent) = f.json(&format!("/api/v1/live/agent?agent=lead&work={}", f.works[0]));
    assert_eq!(agent["agent"]["planned_task"]["id"], "t1");
    assert_eq!(agent["agent"]["planned_task"]["criteria"], 1);
    assert!(agent["agent"]["transcript"].is_null(), "running attempt: no transcript yet");
    let (_, child) = f.json(&format!("/api/v1/live/agent?agent=child&work={}", f.works[0]));
    let t = &child["agent"]["transcript"];
    assert_eq!(t["format"], "claude");
    assert_eq!(t["counts"]["tool"], 1);
    assert_eq!((t["entries"][3]["kind"].as_str(), t["entries"][3]["error"].as_bool()), (Some("tool_result"), Some(true)));
    assert_eq!(t["capture_truncated"], true);
    assert!(!child.to_string().contains("PRIVATE cmd"), "tool input never served");
    assert_eq!(agent["agent"]["cache_policy"], serde_json::json!({"miss_ratio_percent": 50, "min_input_tokens": 1000}));
    let (_, other) = f.json(&format!("/api/v1/live/agent?agent=other&work={}", f.works[1]));
    assert!(other["agent"]["planned_task"].is_null(), "task not in the Work");
}

#[test]
fn alerts_fire_once_resolve_and_refuse_bad_shapes() {
    let f = Fixture::new();
    let (status, empty) = f.json("/api/v1/live/alerts");
    assert_eq!(status, StatusCode::OK);
    assert_eq!(empty["alerts"]["open"].as_array().unwrap().len(), 0, "no store yet");
    let (status, first) = f.post("/api/v1/live/alerts/evaluate", b"", true);
    assert_eq!(status, StatusCode::OK, "{first}");
    let fired: Vec<_> = first["sync"]["fired"].as_array().unwrap().iter().map(|a| (a["rule"].as_str().unwrap().to_owned(), a["subject"].as_str().unwrap().to_owned())).collect();
    // lead carries invalid declared usage in the fixture; other has failed.
    assert_eq!(fired, [("agent-failed".to_owned(), "other".to_owned()), ("agent-usage-invalid".to_owned(), "lead".to_owned())]);
    // Installation defaults (settings v33): 30-day retention, no per-attempt cap; nothing due yet.
    assert_eq!(first["retention"]["policy"], json!({"days": 30, "max_events_per_attempt": null}));
    assert_eq!(first["purged"], json!({"records": 0, "alerts": 0, "tokens": 0}));
    assert!(first["retention"]["last_purge"].is_null());
    let (_, again) = f.post("/api/v1/live/alerts/evaluate", b" {} ", true);
    assert_eq!(again["sync"]["fired"].as_array().unwrap().len(), 0, "idempotent");
    let (_, view) = f.json("/api/v1/live/alerts");
    assert_eq!(view["alerts"]["open"][0]["level"], "critical");
    // GET routes never write the console store (alerts view, agent view).
    let store = f.root.join(".swarm/telemetry.db");
    let before = fs::read(&store).unwrap();
    assert_eq!(f.json("/api/v1/live/alerts").0, StatusCode::OK);
    assert_eq!(f.json(&f.agent_path("lead", &f.works[0], "")).0, StatusCode::OK);
    assert_eq!(fs::read(&store).unwrap(), before, "observational GET");
    // A newer attempt on the failed task clears agent-failed (subject still present).
    let db = Connection::open(f.root.join(".swarm/state.db")).unwrap();
    db.execute("UPDATE agents SET body = json_set(body, '$.started', '2026-10-22T09:00:00Z') WHERE id = 'other'", []).unwrap();
    db.execute(
        "INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) SELECT 'retry', work_id, task_id, cwd, 'running',
           json_set(body, '$.id', 'retry', '$.status', 'running', '$.started', '2026-10-22T10:00:00Z'), request FROM agents WHERE id = 'other'",
        [],
    )
    .unwrap();
    drop(db);
    let (status, cleared) = f.post("/api/v1/live/alerts/evaluate", b"", true);
    assert_eq!(status, StatusCode::OK, "{cleared}");
    let resolved = cleared["sync"]["resolved"].as_array().unwrap();
    assert_eq!((resolved.len(), &resolved[0]["subject"], &resolved[0]["resolution"]), (1, &json!("other"), &json!("cleared")));
    assert_eq!(cleared["sync"]["fired"].as_array().unwrap().len(), 0);
    assert_eq!(cleared["alerts"]["history"][0]["subject"], "other");
    assert_eq!(f.post("/api/v1/live/alerts/evaluate", b"", false).0, StatusCode::FORBIDDEN, "CSRF required");
    assert_eq!(f.post("/api/v1/live/alerts/evaluate", b"{\"x\":1}", true).0, StatusCode::BAD_REQUEST);
    assert_eq!(f.json("/api/v1/live/alerts/evaluate").0, StatusCode::METHOD_NOT_ALLOWED);
}

#[test]
fn usage_gap_alert_compares_declared_and_telemetry_output_tokens() {
    let f = Fixture::new();
    // child (completed) declares 100 output tokens; its telemetry calls sum to 40.
    let db = Connection::open(f.root.join(".swarm/state.db")).unwrap();
    db.execute("UPDATE agents SET body = json_set(body, '$.usage', json('{\"input_tokens\":1,\"output_tokens\":100,\"source\":\"fixture\",\"scope\":\"fixture\",\"at\":\"2026-10-22T10:00:00Z\"}')) WHERE id = 'child'", []).unwrap();
    drop(db);
    let mut store = swarm_engine::TelemetryStore::open(&f.root, true).unwrap();
    let body = serde_json::json!({"resourceLogs": [{"scopeLogs": [{"logRecords": [
        {"timeUnixNano": "1", "body": {"stringValue": "claude_code.api_request"}, "attributes": [{"key": "output_tokens", "value": {"stringValue": "40"}}]}]}]}]});
    let batch = swarm_engine::normalize_logs(body.to_string().as_bytes(), 1).unwrap();
    store.record("child", &batch, 1, swarm_engine::StoreLimits::default()).unwrap();
    store.close().unwrap();
    let (status, first) = f.post("/api/v1/live/alerts/evaluate", b"", true);
    assert_eq!(status, StatusCode::OK, "{first}");
    let gap: Vec<_> = first["sync"]["fired"].as_array().unwrap().iter().filter(|a| a["rule"] == "agent-usage-mismatch").map(|a| a["subject"].clone()).collect();
    assert_eq!(gap, [json!("child")], "60 % gap above the 10 % installation default");
}

#[test]
fn review_and_planning_runs_show_their_telemetry() {
    let f = Fixture::new();
    // Without job tables (older Root): empty run list, unknown run refused.
    let (_, overview) = f.json("/api/v1/live/overview");
    assert!(overview["works"].as_array().unwrap().iter().all(|w| w["runs"] == json!([])), "{overview}");
    let path = |run: &str, work: &str| format!("/api/v1/live/run?run={run}&work={work}");
    let (_, before) = f.json("/api/v1/live/fingerprint");
    // Minimal job tables (engine column names; fixture only, not a runtime path).
    let db = Connection::open(f.root.join(".swarm/state.db")).unwrap();
    db.execute_batch("CREATE TABLE native_review_jobs(id TEXT PRIMARY KEY, work_id TEXT NOT NULL, task_id TEXT NOT NULL, phase TEXT NOT NULL);
        CREATE TABLE native_planning_runs(claim_id TEXT PRIMARY KEY, work_id TEXT NOT NULL, scope_id TEXT NOT NULL, phase TEXT NOT NULL, prepared_at TEXT NOT NULL);").unwrap();
    db.execute("INSERT INTO native_review_jobs VALUES('j1', ?1, 't1', 'permitted')", [&f.works[0]]).unwrap();
    db.execute("INSERT INTO native_planning_runs VALUES('c1', ?1, 'root', 'applied', '2026-10-22T10:00:00Z')", [&f.works[0]]).unwrap();
    drop(db);
    let (_, after) = f.json("/api/v1/live/fingerprint");
    assert_ne!(before["fingerprint"], after["fingerprint"], "a run change wakes the console");
    let (_, overview) = f.json("/api/v1/live/overview");
    let alpha = overview["works"].as_array().unwrap().iter().find(|w| w["id"] == json!(f.works[0])).unwrap();
    let runs: Vec<_> = alpha["runs"].as_array().unwrap().iter().map(|r| (r["subject"].clone(), r["active"].clone())).collect();
    assert_eq!(runs, [(json!("review:j1"), json!(true)), (json!("planning:c1"), json!(false))], "{overview}");
    // No store yet: the run exists, its telemetry is `no-token`.
    let (status, view) = f.json(&path("review:j1", &f.works[0]));
    assert_eq!((status, view["run"]["target"].as_str(), view["telemetry"]["state"].as_str()), (StatusCode::OK, Some("t1"), Some("no-token")), "{view}");
    // Telemetry recorded under the review subject is shown for that run only.
    let mut store = swarm_engine::TelemetryStore::open(&f.root, true).unwrap();
    let body = json!({"resourceLogs": [{"scopeLogs": [{"logRecords": [
        {"timeUnixNano": "1000000", "body": {"stringValue": "claude_code.api_request"}, "attributes": [{"key": "output_tokens", "value": {"stringValue": "7"}}]}]}]}]});
    let batch = swarm_engine::normalize_logs(body.to_string().as_bytes(), 1).unwrap();
    store.record("review:j1", &batch, 1, swarm_engine::StoreLimits::default()).unwrap();
    store.close().unwrap();
    let (_, view) = f.json(&path("review:j1", &f.works[0]));
    assert_eq!((view["telemetry"]["state"].as_str(), view["telemetry"]["calls"][0]["fields"]["output_tokens"].as_str()), (Some("received"), Some("7")), "{view}");
    assert!(view.get("estimated_cost").is_none() && view["telemetry"].get("estimated_cost").is_none(), "provider not persisted: no estimate");
    let (_, other) = f.json(&path("planning:c1", &f.works[0]));
    assert_eq!((other["run"]["phase"].as_str(), other["telemetry"]["calls"].as_array().map(Vec::len)), (Some("applied"), Some(0)), "{other}");
    let cases = [
        (path("review:j1", &f.works[1]), StatusCode::NOT_FOUND, "live-run-unknown"),
        (path("review:ghost", &f.works[0]), StatusCode::NOT_FOUND, "live-run-unknown"),
        (path("lead", &f.works[0]), StatusCode::BAD_REQUEST, "live-query-invalid"),
        (path("review:", &f.works[0]), StatusCode::BAD_REQUEST, "live-query-invalid"),
        (path("review:j1", &f.works[0]) + "&run=review:j1", StatusCode::BAD_REQUEST, "live-query-invalid"),
        (path("review:j1", &f.works[0]) + "&x=1", StatusCode::BAD_REQUEST, "live-query-invalid"),
        ("/api/v1/live/run?run=review:j1".to_owned(), StatusCode::BAD_REQUEST, "live-query-invalid"),
    ];
    for (path, status, code) in cases {
        let (got, body) = f.json(&path);
        assert_eq!((got, body["error"].as_str()), (status, Some(code)), "{path}");
    }
}

#[test]
fn budget_alerts_warn_at_the_saved_share_then_flag_overspend() {
    let f = Fixture::new();
    // Public budget operation: limit 1 USD, reserve 0.1 per launch.
    let mut r = RootStore::open(&f.root, true).unwrap();
    let revision = r.read_work(&f.works[0]).unwrap().revision;
    let raw = format!(r#"{{"schema_version":1,"event_id":"budget","expected_revision":{revision},"budget":{{"limit_usd":1,"reserve_per_launch_usd":0.1,"estimate_source":"Fixture","reference_date":"2026-10-22"}}}}"#);
    r.execute_external(&f.works[0], "budget.configure", raw.as_bytes(), &mut SystemEnvironment).unwrap();
    r.close().unwrap();
    let set_cost = |agent: &str, cost: f64| {
        let db = Connection::open(f.root.join(".swarm/state.db")).unwrap();
        let usage = format!(r#"{{"input_tokens":1,"output_tokens":1,"provider_reported_cost_usd":{cost},"source":"fixture","scope":"fixture","at":"2026-10-22T10:00:00Z"}}"#);
        db.execute("UPDATE agents SET body = json_set(body, '$.usage', json(?1)) WHERE id = ?2", params![usage, agent]).unwrap();
    };
    let budget_rules = |view: &serde_json::Value| -> Vec<(String, String)> {
        view["alerts"]["open"].as_array().unwrap().iter()
            .filter(|a| a["subject"].as_str().unwrap().starts_with("budget:"))
            .map(|a| (a["rule"].as_str().unwrap().to_owned(), a["subject"].as_str().unwrap().to_owned())).collect()
    };
    let subject = format!("budget:{}", f.works[0]);
    // Witness: 0.5 declared of 1.0, below the 80 % installation default.
    set_cost("lead", 0.5);
    let (status, first) = f.post("/api/v1/live/alerts/evaluate", b"", true);
    assert_eq!(status, StatusCode::OK, "{first}");
    assert!(budget_rules(&first).is_empty(), "{first}");
    assert_eq!(first["alert_evaluation_seconds"], json!(60), "saved v35 cadence reported");
    set_cost("child", 0.35);
    let (_, warned) = f.post("/api/v1/live/alerts/evaluate", b"", true);
    assert_eq!(budget_rules(&warned), [("work-budget-threshold".to_owned(), subject.clone())], "0.85 ≥ 80 %");
    set_cost("child", 0.6);
    let (_, over) = f.post("/api/v1/live/alerts/evaluate", b"", true);
    assert_eq!(budget_rules(&over), [("work-budget-overspent".to_owned(), subject.clone())], "1.1 > limit: overspend replaces the warning");
    let resolved: Vec<_> = over["sync"]["resolved"].as_array().unwrap().iter().map(|a| a["rule"].clone()).collect();
    assert!(resolved.contains(&json!("work-budget-threshold")), "{over}");
    // Overview carries the budget figures; the other Work has none (Null).
    let (_, overview) = f.json("/api/v1/live/overview");
    let by_id = |id: &str| overview["works"].as_array().unwrap().iter().find(|w| w["id"] == json!(id)).unwrap().clone();
    let b = &by_id(&f.works[0])["budget"];
    assert_eq!((b["limit_usd"].as_f64(), b["reported_usd"].as_f64().map(|v| (v * 100.0).round())), (Some(1.0), Some(110.0)), "{b}");
    assert!(by_id(&f.works[1])["budget"].is_null());
    let (_, rules) = f.json("/api/v1/live/alerts");
    let ids: Vec<_> = rules["rules"].as_array().unwrap().iter().map(|r| r["rule"].as_str().unwrap().to_owned()).collect();
    assert!(ids.contains(&"work-budget-overspent".to_owned()) && ids.contains(&"work-budget-threshold".to_owned()));
    assert!(rules["server_evaluation"].is_null(), "no server loop in the typed in-process service");
    // An invalid declared cost makes this Work's budget unreadable: its open
    // overspend alert stays open, other rules still run, the overview shows a state.
    set_cost("child", -1.0);
    let (status, broken) = f.post("/api/v1/live/alerts/evaluate", b"", true);
    assert_eq!(status, StatusCode::OK, "one Work's budget does not stop the evaluation: {broken}");
    assert_eq!(broken["budget_errors"][0]["work"], json!(f.works[0]), "{broken}");
    assert_eq!(budget_rules(&broken), [("work-budget-overspent".to_owned(), subject.clone())], "kept open, not resolved as cleared");
    assert!(broken["alerts"]["open"].as_array().unwrap().iter().any(|a| a["rule"] == "agent-failed"), "other rules evaluated");
    let (status, overview) = f.json("/api/v1/live/overview");
    assert_eq!(status, StatusCode::OK);
    let alpha = overview["works"].as_array().unwrap().iter().find(|w| w["id"] == json!(f.works[0])).unwrap().clone();
    assert_eq!(alpha["budget"]["state"], json!("unavailable"), "{alpha}");
}

#[test]
fn annotations_queue_create_change_and_refuse_bad_requests() {
    let f = Fixture::new();
    let path = format!("/api/v1/live/annotations?work={}", f.works[0]);
    let (status, first) = f.json(&path);
    assert_eq!(status, StatusCode::OK, "{first}");
    assert_eq!((first["policy"].as_bool(), first["note_max_chars"].as_i64()), (Some(true), Some(2000)));
    let queue: Vec<_> = first["queue"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_owned()).collect();
    assert!(queue.contains(&"child".to_owned()) && !queue.contains(&"lead".to_owned()), "finished attempts only: {first}");
    let body = |agent: &str, work: &str, verdict: &str, note: &str, rev: i64| {
        serde_json::to_vec(&json!({"schema_version": 1, "work": work, "agent": agent, "verdict": verdict, "note": note, "expected_revision": rev})).unwrap()
    };
    let (status, created) = f.post("/api/v1/live/annotations", &body("child", &f.works[0], "bad", "wrong file\n\tedited", 0), true);
    assert_eq!((status, created["annotation"]["revision"].as_i64()), (StatusCode::OK, Some(1)), "{created}");
    let (_, after) = f.json(&path);
    assert!(!after["queue"].as_array().unwrap().contains(&json!("child")), "annotated attempt leaves the queue");
    let item = after["attempts"].as_array().unwrap().iter().find(|a| a["agent_id"] == "child").unwrap();
    assert_eq!(item["annotation"]["note"], json!("wrong file\n\tedited"));
    let (_, agent) = f.json(&f.agent_path("child", &f.works[0], ""));
    assert_eq!(agent["agent"]["annotation"]["verdict"], json!("bad"), "harness view carries it");
    let (status, changed) = f.post("/api/v1/live/annotations", &body("child", &f.works[0], "good", "", 1), true);
    assert_eq!((status, changed["annotation"]["revision"].as_i64()), (StatusCode::OK, Some(2)));
    let long = "é".repeat(2001);
    let cases = [
        (body("child", &f.works[0], "good", "", 1), StatusCode::CONFLICT, "annotation-revision-conflict"),
        (body("lead", &f.works[0], "good", "", 0), StatusCode::CONFLICT, "annotation-agent-active"),
        (body("ghost", &f.works[0], "good", "", 0), StatusCode::NOT_FOUND, "live-agent-unknown"),
        (body("child", &f.works[1], "good", "", 2), StatusCode::BAD_REQUEST, "live-agent-work-mismatch"),
        (body("child", &f.works[0], "great", "", 2), StatusCode::BAD_REQUEST, "annotation-verdict-invalid"),
        (body("child", &f.works[0], "good", &long, 2), StatusCode::BAD_REQUEST, "annotation-note-too-long"),
        (body("child", &f.works[0], "good", "bell\u{7}", 2), StatusCode::BAD_REQUEST, "annotation-note-invalid"),
        (b"{\"schema_version\":1}".to_vec(), StatusCode::BAD_REQUEST, "annotation-request-invalid"),
    ];
    for (raw, status, code) in cases {
        let (got, reply) = f.post("/api/v1/live/annotations", &raw, true);
        assert_eq!((got, reply["error"].as_str()), (status, Some(code)), "{}", String::from_utf8_lossy(&raw));
    }
    // 2000 characters exactly are accepted (bound inclusive, counted in characters).
    let (status, _) = f.post("/api/v1/live/annotations", &body("child", &f.works[0], "unsure", &"é".repeat(2000), 2), true);
    assert_eq!(status, StatusCode::OK);
    let (status, refused) = f.post("/api/v1/live/annotations", &body("child", &f.works[0], "good", "", 3), false);
    assert_ne!(status, StatusCode::OK, "no CSRF, no write: {refused}");
    for bad in [format!("/api/v1/live/annotations?work={}&x=1", f.works[0]), "/api/v1/live/annotations".to_owned()] {
        assert_eq!(f.json(&bad).1["error"], json!("live-query-invalid"), "{bad}");
    }
}

#[test]
fn dataset_exports_recorded_facts_by_selection_without_content() {
    let f = Fixture::new();
    let path = |work: &str, select: &str| format!("/api/v1/live/dataset?work={work}&select={select}");
    // Failed attempt of Beta: task, cause and outcome, no content.
    let (status, failed) = f.json(&path(&f.works[1], "failed"));
    assert_eq!(status, StatusCode::OK, "{failed}");
    let entries = failed["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1, "{failed}");
    assert_eq!((entries[0]["agent_id"].as_str(), entries[0]["cause"]["source"].as_str()), (Some("other"), Some("unclassified")));
    assert_eq!(entries[0]["telemetry"]["state"], json!("no-token"));
    let raw = serde_json::to_string(&failed).unwrap();
    assert!(!raw.contains("PRIVATE"), "no prompt, capture or tool content: {raw}");
    // Alpha: the completed child is not "failed"; "all" has it; "bad" needs a bad verdict.
    assert!(f.json(&path(&f.works[0], "failed")).1["entries"].as_array().unwrap().is_empty());
    let (_, all) = f.json(&path(&f.works[0], "all"));
    let ids: Vec<_> = all["entries"].as_array().unwrap().iter().map(|e| e["agent_id"].clone()).collect();
    assert_eq!(ids, [json!("child")], "active lead excluded: {all}");
    assert_eq!(all["entries"][0]["task"]["title"], json!("Task t2"));
    assert!(f.json(&path(&f.works[0], "bad")).1["entries"].as_array().unwrap().is_empty(), "no annotation yet");
    let note = serde_json::to_vec(&json!({"schema_version": 1, "work": f.works[0], "agent": "child", "verdict": "bad", "note": "regression", "expected_revision": 0})).unwrap();
    assert_eq!(f.post("/api/v1/live/annotations", &note, true).0, StatusCode::OK);
    let (_, bad) = f.json(&path(&f.works[0], "bad"));
    assert_eq!((bad["entries"].as_array().unwrap().len(), bad["entries"][0]["annotation"]["note"].as_str()), (1, Some("regression")));
    for wrong in [path(&f.works[0], "everything"), format!("/api/v1/live/dataset?work={}", f.works[0]), path(&f.works[0], "all") + "&select=bad"] {
        assert_eq!(f.json(&wrong).1["error"], json!("live-query-invalid"), "{wrong}");
    }
    let (status, missing) = f.json(&path("w-missing", "all"));
    assert_eq!((status, missing["error"].as_str()), (StatusCode::NOT_FOUND, Some("work-missing")), "unknown Work refused: {missing}");
}

#[test]
fn finished_attempts_are_evaluated_once_and_shown_in_the_harness() {
    let f = Fixture::new();
    let (_, before) = f.json(&f.agent_path("child", &f.works[0], ""));
    assert_eq!(before["agent"]["evaluations"], json!([]), "no store yet");
    let (status, first) = f.post("/api/v1/live/alerts/evaluate", b"", true);
    assert_eq!((status, first["evaluated"].as_u64()), (StatusCode::OK, Some(2)), "child and other finished, lead running: {first}");
    let (_, again) = f.post("/api/v1/live/alerts/evaluate", b"", true);
    assert_eq!(again["evaluated"], json!(0), "evaluated once");
    let (_, child) = f.json(&f.agent_path("child", &f.works[0], ""));
    let evals = child["agent"]["evaluations"].as_array().unwrap();
    let outcome = evals.iter().find(|e| e["evaluator"] == "outcome").unwrap();
    assert_eq!((outcome["passed"].as_bool(), outcome["detail"].as_str()), (Some(true), Some("status=completed")), "{child}");
    let (_, other) = f.json(&f.agent_path("other", &f.works[1], ""));
    let failed = other["agent"]["evaluations"].as_array().unwrap().iter().find(|e| e["evaluator"] == "outcome").unwrap().clone();
    assert_eq!(failed["passed"], json!(false));
    let (_, lead) = f.json(&f.agent_path("lead", &f.works[0], ""));
    assert_eq!(lead["agent"]["evaluations"], json!([]), "running attempt not evaluated");
}

#[test]
fn task_scenarios_are_saved_judged_on_recorded_facts_and_refused_when_invalid() {
    let f = Fixture::new();
    let get = format!("/api/v1/live/scenario?work={}&task=t2", f.works[0]);
    let (status, empty) = f.json(&get);
    assert_eq!((status, empty["scenario"].is_null(), empty["attempt"]["agent_id"].as_str()), (StatusCode::OK, true, Some("child")), "{empty}");
    let body = |task: &str, rev: i64, expectations: serde_json::Value| {
        serde_json::to_vec(&json!({"schema_version": 1, "work": f.works[0], "task": task, "expected_revision": rev, "scenario": {"schema_version": 1, "expectations": expectations}})).unwrap()
    };
    let expectations = json!([{"kind": "outcome", "status": "completed"}, {"kind": "tool-used", "tool": "Bash"}, {"kind": "outcome", "status": "failed"}]);
    let (status, saved) = f.post("/api/v1/live/scenario", &body("t2", 0, expectations.clone()), true);
    assert_eq!((status, saved["revision"].as_i64()), (StatusCode::OK, Some(1)), "{saved}");
    let (_, judged) = f.json(&get);
    let verdicts: Vec<_> = judged["results"].as_array().unwrap().iter().map(|r| r["passed"].clone()).collect();
    assert_eq!(verdicts, [json!(true), json!(null), json!(false)], "no telemetry: tool expectation not judgeable: {judged}");
    let (_, overview) = f.json("/api/v1/live/overview");
    let alpha = overview["works"].as_array().unwrap().iter().find(|w| w["id"] == json!(f.works[0])).unwrap();
    assert_eq!(alpha["scenarios"]["t2"], json!({"passed": 1, "failed": 1, "undecided": 1}));
    let beta = overview["works"].as_array().unwrap().iter().find(|w| w["id"] == json!(f.works[1])).unwrap();
    assert_eq!(beta["scenarios"], json!({}), "same task id in another Work is not judged by this scenario");
    let cases = [
        (body("t2", 0, expectations.clone()), StatusCode::CONFLICT, "scenario-revision-conflict"),
        (body("t-none", 0, expectations.clone()), StatusCode::NOT_FOUND, "live-task-unknown"),
        (body("t2", 1, json!([])), StatusCode::BAD_REQUEST, "scenario-empty"),
        (body("t2", 1, json!([{"kind": "tool-used", "tool": ""}])), StatusCode::BAD_REQUEST, "scenario-value-invalid"),
        (body("t2", 1, json!([{"kind": "guess"}])), StatusCode::BAD_REQUEST, "scenario-request-invalid"),
    ];
    for (raw, status, code) in cases {
        let (got, reply) = f.post("/api/v1/live/scenario", &raw, true);
        assert_eq!((got, reply["error"].as_str()), (status, Some(code)), "{}", String::from_utf8_lossy(&raw));
    }
    assert_eq!(f.json(&format!("/api/v1/live/scenario?work={}&task=t-none", f.works[0])).1["error"], json!("live-task-unknown"));
    let (status, _) = f.post("/api/v1/live/scenario", &body("t2", 1, expectations.clone()), false);
    assert_ne!(status, StatusCode::OK, "no CSRF, no write");
    assert_eq!(f.json(&get).1["scenario"]["revision"], json!(1), "unchanged after the refused write");
    // A stored specification that no longer decodes is refused, not skipped.
    Connection::open(f.root.join(".swarm/telemetry.db")).unwrap().execute("UPDATE scenarios SET spec = '{}'", []).unwrap();
    let (status, broken) = f.json(&get);
    assert_eq!((status, broken["error"].as_str()), (StatusCode::INTERNAL_SERVER_ERROR, Some("scenario-stored-invalid")), "{broken}");
    assert_eq!(f.json(&format!("/api/v1/live/scenario?work={}", f.works[0])).1["error"], json!("live-query-invalid"));
}

#[test]
fn agent_route_pages_logs_and_terminal_by_cursor() {
    let f = Fixture::new();
    let (status, first) = f.json(&f.agent_path("lead", &f.works[0], ""));
    assert_eq!(status, StatusCode::OK);
    assert_eq!(first["agent"]["id"], "lead");
    assert_eq!(first["agent"]["context_memory"][0]["id"], "mem-1");
    assert_eq!(first["agent"]["context_memory"][0]["version"], 3);
    assert!(!serde_json::to_string(&first).unwrap().contains("SECRET-DIGEST"), "no memory digest or content");
    assert_eq!(first["agent"]["resources"]["supervisor"]["state"], "not-recorded");
    let logs = first["logs"]["items"].as_array().unwrap();
    assert_eq!(logs.len());
    assert_eq!(logs[0]["message"], "line 0");
    assert_eq!(first["logs"]["more"], true);
    assert!(logs.iter().all(|l| l["message"] != "not lead"));
    assert_eq!(first["terminal"]["items"][0]["text"], "\x1b[32m$ cargo test\r\n");
    assert_eq!(first["terminal"]["items"][0]["invalid_utf8"], false);
    let next = first["logs"]["next"].as_i64().unwrap();
    let term = first["terminal"]["next"].as_i64().unwrap();
    let (_, second) = f.json(&f.agent_path(
        "lead",
        &f.works[0],
        &format!("&logs_after={next}&terminal_after={term}"),
    ));
    assert_eq!(second["logs"]["items"].as_array().unwrap().len(), 3);
    assert_eq!(second["logs"]["more"], false);
    assert!(second["terminal"]["items"].as_array().unwrap().is_empty());
    assert_eq!(second["terminal"]["next"], term);
}

#[test]
fn refusals_are_explicit_after_a_valid_witness() {
    let f = Fixture::new();
    assert_eq!(
        f.send("GET", &f.agent_path("lead", &f.works[0], ""), true)
            .response
            .status(),
        StatusCode::OK
    );
    let cases = [
        (f.agent_path("ghost", &f.works[0], ""), StatusCode::NOT_FOUND, "live-agent-unknown"),
        (f.agent_path("lead", &f.works[1], ""), StatusCode::BAD_REQUEST, "live-agent-work-mismatch"),
        (f.agent_path("lead", &f.works[0], "&logs_after=-1"), StatusCode::BAD_REQUEST, "live-cursor-invalid"),
        (f.agent_path("lead", &f.works[0], "&agent=x"), StatusCode::BAD_REQUEST, "live-query-invalid"),
        ("/api/v1/live/overview?x=1".to_owned(), StatusCode::BAD_REQUEST, "live-query-invalid"),
    ];
    for (path, status, code) in cases {
        let (got, body) = f.json(&path);
        assert_eq!((got, body["error"].as_str().unwrap()), (status, code), "{path}");
    }
    // In-process typed dispatch has no stream: explicit 501, never an empty 200.
    let (status, body) = f.json("/api/v1/live/events");
    assert_eq!((status, body["error"].as_str().unwrap()), (StatusCode::NOT_IMPLEMENTED, "live-events-transport-required"));
    let (status, body) = f.json("/api/v1/live/fingerprint");
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["fingerprint"].as_str().unwrap().len(), 64);
    let post = f.send("POST", "/api/v1/live/overview", true);
    // Without CSRF the access gate refuses first; with it the route refuses the method.
    assert!(matches!(post.response.status(), StatusCode::FORBIDDEN | StatusCode::METHOD_NOT_ALLOWED));
    for path in ["/api/v1/live/overview", "/live.html", "/live/live.js"] {
        assert_ne!(f.send("GET", path, false).response.status(), StatusCode::OK, "{path}");
    }
}

#[test]
fn embedded_pages_are_served_with_types_and_head_has_no_body() {
    let f = Fixture::new();
    for (path, kind) in [
        ("/live.html", "text/html; charset=utf-8"),
        ("/live-session.html", "text/html; charset=utf-8"),
        ("/live/live.js", "application/javascript; charset=utf-8"),
        ("/live/live.css", "text/css; charset=utf-8"),
    ] {
        let reply = f.send("GET", path, true);
        assert_eq!(reply.response.status(), StatusCode::OK, "{path}");
        assert_eq!(reply.response.headers()[CONTENT_TYPE], kind);
        assert!(!reply.response.body().is_empty());
        let head = f.send("HEAD", path, true);
        assert_eq!(head.response.status(), StatusCode::OK);
        assert!(head.response.body().is_empty());
    }
    // Harness page: fresh nonce in CSP and meta, different per response.
    let nonce = |r: &HttpReply| {
        let csp = r.response.headers()["content-security-policy"].to_str().unwrap().to_owned();
        let nonce = csp.split("'nonce-").nth(1).unwrap().split('\'').next().unwrap().to_owned();
        let page = std::str::from_utf8(r.response.body()).unwrap();
        assert!(page.contains(&format!("<meta name=\"swarm-style-nonce\" content=\"{nonce}\">")));
        assert!(!nonce.is_empty());
        nonce
    };
    let a = nonce(&f.send("GET", "/live-session.html", true));
    let b = nonce(&f.send("GET", "/live-session.html", true));
    assert_ne!(a, b);
    assert!(!f.send("GET", "/live.html", true).response.headers()["content-security-policy"].to_str().unwrap().contains("nonce"));
    let html = f.send("GET", "/live.html", true);
    let page = std::str::from_utf8(html.response.body()).unwrap();
    for asset in ["/live/live-terminal.js", "/live/xterm.css"] {
        assert_eq!(f.send("GET", asset, true).response.status(), StatusCode::OK, "{asset}");
    }
    for asset in ["/live/live-common.js", "/live/i18n.js", "/live/english.js", "/live/themes.css", "/live/swarm-design.css"] {
        assert!(page.contains(asset), "{asset}");
        assert_eq!(f.send("GET", asset, true).response.status(), StatusCode::OK, "{asset}");
    }
}
