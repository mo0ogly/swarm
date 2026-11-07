#![cfg(target_os = "linux")]
//! In-process typed HTTP contract for the versioned-memory catalog GET route
//! (Go `/api/v1/project-memory`): statuses with Go reason text, folded literal
//! `q` filter, POST/unknown-query refusals, and no mutation on any outcome.
//! No network listener, live mission, provider or authority is implied.
use http::{
    header::{COOKIE, HOST, ORIGIN},
    Request, StatusCode,
};
use std::{fs, net::TcpListener, path::PathBuf};
use swarm_engine::{RootStore, SystemEnvironment};
use swarm_web::{LocalWebAuthority, NativeHttpService};

const KEY: &str = "session-0123456789abcdef01234567";

struct Fixture {
    root: PathBuf,
    #[allow(dead_code)]
    listener: TcpListener,
    work: String,
}

impl Fixture {
    fn new() -> Self {
        let root = swarm_engine::fixture_root("web-memcat-");
        let mut s = RootStore::initialize(&root, &mut SystemEnvironment).unwrap();
        let work = s
            .execute_external(
                "",
                "work.create",
                &serde_json::to_vec(&serde_json::json!({
                    "schema_version":1,"event_id":"create","expected_revision":0,
                    "title":"Mémoire","objective":"Preuve","scope":"Fixture","criteria":["Preuve"]
                }))
                .unwrap(),
                &mut SystemEnvironment,
            )
            .unwrap()
            .result
            .work
            .id;
        s.close().unwrap();
        Self {
            root,
            listener: TcpListener::bind("127.0.0.1:0").unwrap(),
            work,
        }
    }
    fn authority(&self) -> LocalWebAuthority {
        LocalWebAuthority::from_listener(&self.listener, KEY).unwrap()
    }
    fn request(&self, method: &str, path: &str) -> Request<Vec<u8>> {
        let a = self.authority();
        let mut r = Request::builder()
            .method(method)
            .uri(path)
            .header(HOST, a.host())
            .header(COOKIE, format!("{}={KEY}", a.cookie_name()))
            .body(Vec::new())
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

/// Splice one two-version memory entry directly into the stored Work.
fn splice_memory(f: &Fixture) {
    use sha2::{Digest, Sha256};
    let db = rusqlite::Connection::open(f.root.join(".swarm/state.db")).unwrap();
    swarm_engine::register_identity_plan_projection(&db).unwrap();
    let raw: Vec<u8> = db
        .query_row(
            "SELECT body FROM works WHERE id=?",
            [&f.work],
            |row| row.get(0),
        )
        .unwrap();
    let mut doc: swarm_engine::Work = swarm_engine::decode_work(&raw).unwrap();
    doc.retex = Some(vec![swarm_engine::Retex {
        id: "retex-1".into(),
        status: "verifie".into(),
        nature: "fait".into(),
        problem: "problème".into(),
        proposal: "proposition".into(),
        criteria: "critère".into(),
        proof: "preuve".into(),
        task: "task-1".into(),
        ..Default::default()
    }]);
    let retex_sha = swarm_engine::memory_source_sha(&doc, &doc.retex.as_ref().unwrap()[0]).unwrap();
    let project_hash = format!(
        "{:x}",
        Sha256::digest(f.root.to_string_lossy().as_bytes())
    );
    let sealed = |version: i64, body: &str| -> swarm_engine::MemoryVersion {
        let mut v = swarm_engine::MemoryVersion {
            version,
            action: if version == 1 { "admit" } else { "replace" }.into(),
            kind: "procedure".into(),
            body: body.into(),
            conditions: "Recheck".into(),
            source_retex: "retex-1".into(),
            project_sha256: project_hash.clone(),
            source_sha256: retex_sha.clone(),
            ..Default::default()
        };
        v.sha256 = swarm_engine::memory_version_sha(&v).unwrap();
        v
    };
    doc.project_memory = Some(vec![swarm_engine::ProjectMemory {
        id: "lesson".into(),
        imported: false,
        versions: Some(vec![
            sealed(1, "Première étape vérifiée"),
            sealed(2, "Version courante à jour"),
        ]),
    }]);
    let body = swarm_engine::encode_model(&doc).unwrap();
    db.execute(
        "UPDATE works SET body=? WHERE id=?",
        rusqlite::params![body, f.work],
    )
    .unwrap();
}

#[test]
fn memory_catalog_get_reports_statuses_filter_and_refusals() {
    let f = Fixture::new();
    splice_memory(&f);
    let service = NativeHttpService::open(&f.root, f.authority()).unwrap();

    // GET returns the catalog with Go's statuses and reason text.
    let reply = service.handle(f.request("GET", "/api/v1/project-memory")).unwrap();
    assert_eq!(reply.response.status(), StatusCode::OK);
    assert!(reply.failure.is_none());
    assert!(!reply.recorded_work.is_some());
    let catalog: serde_json::Value = serde_json::from_slice(reply.response.body()).unwrap();
    assert_eq!(catalog["method"], "literal");
    assert_eq!(catalog["query"], "");
    let entries = catalog["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["status"], "historical");
    assert_eq!(entries[0]["reason"], "Version historique, non sélectionnable");
    assert_eq!(entries[0]["ref"]["work"], f.work);
    assert_eq!(entries[1]["status"], "unavailable");
    assert_eq!(
        entries[1]["reason"],
        "Un fait sourcé et une tâche APEX acceptée avec des preuves fraîches sont requis."
    );

    // Folded literal filter with an accented uppercase query.
    let reply = service
        .handle(f.request("GET", "/api/v1/project-memory?q=%C3%89TAPE"))
        .unwrap();
    assert_eq!(reply.response.status(), StatusCode::OK);
    let catalog: serde_json::Value = serde_json::from_slice(reply.response.body()).unwrap();
    assert_eq!(catalog["query"], "etape");
    assert_eq!(catalog["entries"].as_array().unwrap().len(), 1);
    assert_eq!(catalog["entries"][0]["ref"]["version"], 1);

    // HEAD is allowed and returns no surprise body policy change.
    let reply = service.handle(f.request("HEAD", "/api/v1/project-memory")).unwrap();
    assert_eq!(reply.response.status(), StatusCode::OK);

    // POST without an action query member refuses as invalid; no mutation.
    let before = f.snapshot_state();
    let reply = service.handle(f.request("POST", "/api/v1/project-memory")).unwrap();
    assert!(reply.failure.is_some());
    assert_eq!(f.snapshot_state(), before);

    // Unknown or repeated query members refuse.
    for path in [
        "/api/v1/project-memory?x=1",
        "/api/v1/project-memory?q=a&q=b",
    ] {
        let reply = service.handle(f.request("GET", path)).unwrap();
        assert!(reply.failure.is_some(), "{path}");
        assert_eq!(f.snapshot_state(), before);
    }
}

impl Fixture {
    /// Cheap invariant: works table is byte-stable across every refused call.
    fn snapshot_state(&self) -> Vec<u8> {
        let db = rusqlite::Connection::open_with_flags(
            self.root.join(".swarm/state.db"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let raw: Vec<u8> = db
            .query_row(
                "SELECT body FROM works WHERE id=?",
                [&self.work],
                |row| row.get(0),
            )
            .unwrap();
        db.close().unwrap();
        raw
    }
    /// A POST request with a typed JSON body on the memory route.
    fn post(&self, action: &str, body: serde_json::Value) -> Request<Vec<u8>> {
        let mut r = self.request("POST", &format!("/api/v1/project-memory?action={action}"));
        *r.body_mut() = serde_json::to_vec(&body).unwrap();
        r
    }
}

/// The POST mutation surface honors the same Go contract as the CLI: a
/// selection preview on an executing todo task returns the proposed outcome
/// (token, empty preview, revision) without mutating; the commit with the
/// preview token stamps the decision and reports the recorded Work; admit on
/// a source-unfresh Work refuses on both preview and commit routes with the
/// stable cause and nothing lands; `summary-source` and unknown actions
/// refuse as unported.
#[test]
fn memory_post_previews_selects_and_refuses_unfresh_and_unported() {
    let f = Fixture::new();
    // One executing todo task for the selection route.
    {
        let mut s = RootStore::open(&f.root, true).unwrap();
        let revision: i64 = {
            let db = rusqlite::Connection::open(f.root.join(".swarm/state.db")).unwrap();
            db.query_row(
                "SELECT revision FROM works WHERE id=?",
                [&f.work],
                |row| row.get(0),
            )
            .unwrap()
        };
        s.execute_external(
            &f.work,
            "task.add",
            &serde_json::to_vec(&serde_json::json!({
                "schema_version":1,"event_id":"task","expected_revision":revision,
                "id":"task-worker","title":"Worker","deliverable":"Report","criteria":["Proof"]
            }))
            .unwrap(),
            &mut SystemEnvironment,
        )
        .unwrap();
        s.close().unwrap();
    }
    splice_memory(&f);
    let service = NativeHttpService::open(&f.root, f.authority()).unwrap();

    // Selection preview succeeds read-only and carries the token binding.
    let reply = service
        .handle(f.post(
            "select-preview",
            serde_json::json!({
                "schema_version":1,"event_id":"select-1","expected_revision":2,
                "work":f.work,"reason":"operator decision",
                "task":"task-worker","selections":[]
            }),
        ))
        .unwrap();
    assert_eq!(
        reply.response.status(),
        StatusCode::OK,
        "preview failed: {:?} body={}",
        reply.failure.as_ref().map(|e| e.to_string()),
        String::from_utf8_lossy(reply.response.body())
    );
    assert!(reply.failure.is_none());
    let preview: serde_json::Value = serde_json::from_slice(reply.response.body()).unwrap();
    assert!(!preview["preview_token"].as_str().unwrap().is_empty());
    assert_eq!(preview["preview"]["action"], "");
    assert_eq!(preview["revision"], 2);
    let before = f.snapshot_state();

    // The commit stamps the executing task's decision and reports the Work.
    let reply = service
        .handle(f.post(
            "select",
            serde_json::json!({
                "schema_version":1,"event_id":"select-2","expected_revision":2,
                "work":f.work,"reason":"operator decision",
                "task":"task-worker","selections":[],
                "preview_token":preview["preview_token"]
            }),
        ))
        .unwrap();
    assert_eq!(reply.response.status(), StatusCode::OK, "{:?}", reply.failure);
    assert!(reply.failure.is_none());
    let work: serde_json::Value = serde_json::from_slice(reply.response.body()).unwrap();
    let tasks = work["tasks"].as_array().unwrap();
    let worker = tasks.iter().find(|t| t["id"] == "task-worker").unwrap();
    assert_eq!(worker["memory_decision"]["event_id"], "select-2");

    // Admit preview and commit both refuse on this Work: its RETEX is
    // verified but its source task was never accepted (source-unfresh). The
    // selection commit advanced the revision to 3.
    let admit_body = serde_json::json!({
        "schema_version":1,"event_id":"admit-1","expected_revision":3,
        "work":f.work,"reason":"operator decision",
        "id":"mem-1","kind":"experience","body":"Lesson",
        "conditions":"Recheck","source_retex":"retex-1"
    });
    for action in ["admit-preview", "admit"] {
        let reply = service.handle(f.post(action, admit_body.clone())).unwrap();
        assert!(reply.failure.is_some(), "{action}");
        // The HTTP surface exposes the stable refusal family; the exact
        // source-unfresh reason is asserted at the CLI and engine layers.
        let text = format!("{:?}", reply.failure.as_ref().map(|e| e.to_string()));
        assert!(text.contains("project-memory-refused"), "{text}");
    }
    // The decision stamped above is the only change; the refused admits left
    // no memory entry (the spliced "lesson" entry is the only one).
    let after = f.snapshot_state();
    let doc: swarm_engine::Work = swarm_engine::decode_work(&after).unwrap();
    let entries = doc.project_memory.as_deref().unwrap_or_default();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, "lesson");

    // Unknown and M20 actions refuse as unported; nothing mutates.
    let before = f.snapshot_state();
    for action in ["summary-source", "erase", ""] {
        let path = if action.is_empty() {
            "/api/v1/project-memory".to_string()
        } else {
            format!("/api/v1/project-memory?action={action}")
        };
        let reply = service
            .handle(f.post(&path, serde_json::json!({"schema_version":1})))
            .or_else(|_| {
                // An empty action is a query-refusal raised before decode; the
                // handle call itself still returns a reply envelope.
                service.handle(f.request("POST", &path))
            })
            .unwrap();
        assert!(reply.failure.is_some(), "{action}");
    }
    assert_eq!(f.snapshot_state(), before);
}
