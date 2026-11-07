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

    // POST stays refused as unported: the mutation surface is a later G2
    // increment; no invented behavior, no mutation.
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
}
