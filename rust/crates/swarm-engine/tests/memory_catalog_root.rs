#![cfg(unix)]
//! RootStore versioned-memory catalog: stored versions report their status
//! with Go's reason text, the folded literal query filters body+conditions,
//! and one invalid stored row refuses the whole catalog (never a partial
//! collection). Isolated fixture Root; no live mission, provider or authority
//! implied.

use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use swarm_engine::{
    decode_work, encode_model, fixture_root, memory_source_sha, memory_version_sha, MemoryVersion,
    RootStore, SystemEnvironment, Work, WorkEnvironment,
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        Self(fixture_root("swarm-memcat-"))
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
}

/// Deterministic fixture observations; no identity value.
struct Environment;
impl WorkEnvironment for Environment {
    fn identifier(&mut self, prefix: &str) -> Result<String, swarm_engine::RuntimeError> {
        Ok(format!("{prefix}fixture"))
    }
    fn timestamp(&mut self) -> Result<String, swarm_engine::RuntimeError> {
        Ok("2026-11-07T00:00:00Z".into())
    }
    fn operator(&mut self) -> Result<String, swarm_engine::RuntimeError> {
        Ok("operator/fixture".into())
    }
    fn git(
        &mut self,
        _: &std::path::Path,
    ) -> Result<swarm_engine::GitObservation, swarm_engine::RuntimeError> {
        Ok(swarm_engine::GitObservation {
            state: Default::default(),
            issues: vec![],
        })
    }
}

fn request(event: &str, revision: i64, fields: serde_json::Value) -> Vec<u8> {
    let mut v = fields;
    v["schema_version"] = 1.into();
    v["event_id"] = event.into();
    v["expected_revision"] = revision.into();
    serde_json::to_vec(&v).unwrap()
}

/// One initialized Root with a fixture Work; returns the fixture and Work id.
fn fixture_root_with_work() -> (Fixture, String) {
    let f = Fixture::new();
    let mut env = SystemEnvironment;
    let mut store = RootStore::initialize(&f.0, &mut env).unwrap();
    let created = store
        .execute_external(
            "",
            "work.create",
            &request(
                "create",
                0,
                serde_json::json!({"title":"Catalogue mémoire","objective":"Preuve","scope":"Fixture","criteria":["Preuve"]}),
            ),
            &mut env,
        )
        .unwrap();
    let id = created.result.work.id.clone();
    store.close().unwrap();
    (f, id)
}

/// Open only this owned fixture store with the actual projection helpers.
fn open_db(f: &Fixture) -> rusqlite::Connection {
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    swarm_engine::register_identity_plan_projection(&db).unwrap();
    db
}

/// Read the stored Work body through its own connection.
fn stored_work(f: &Fixture, id: &str) -> Work {
    let raw: Vec<u8> = open_db(f)
        .query_row("SELECT body FROM works WHERE id=?", [id], |row| row.get(0))
        .unwrap();
    decode_work(&raw).unwrap()
}

/// Rewrite the stored Work body through its own connection.
fn rewrite(f: &Fixture, id: &str, doc: &Work) {
    let body = encode_model(doc).unwrap();
    open_db(f)
        .execute("UPDATE works SET body=? WHERE id=?", rusqlite::params![body, id])
        .unwrap();
}

fn verified_retex_doc(work: &Work) -> Work {
    let mut doc = work.clone();
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
    doc
}

fn sealed_version(version: i64, body: &str, project_hash: &str, retex_sha: String) -> MemoryVersion {
    let mut v = MemoryVersion {
        version,
        action: if version == 1 { "admit" } else { "replace" }.into(),
        kind: "procedure".into(),
        body: body.into(),
        conditions: "Recheck".into(),
        source_retex: "retex-1".into(),
        project_sha256: project_hash.into(),
        source_sha256: retex_sha,
        ..Default::default()
    };
    v.sha256 = memory_version_sha(&v).unwrap();
    v
}

#[test]
fn root_catalog_reports_statuses_query_filter_and_folds_accents() {
    let (f, id) = fixture_root_with_work();
    // Go `hash([]byte(s.root))`: SHA-256 over the root path bytes.
    let project_hash = {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(f.0.to_string_lossy().as_bytes()))
    };
    // One verified RETEX fact; the final version binds its exact source digest.
    let work = stored_work(&f, &id);
    let doc = verified_retex_doc(&work);
    let retex_sha = memory_source_sha(&doc, &doc.retex.as_ref().unwrap()[0]).unwrap();
    rewrite(&f, &id, &doc);
    let v1 = sealed_version(1, "Première étape vérifiée", &project_hash, retex_sha.clone());
    let v2 = sealed_version(2, "Version courante à jour", &project_hash, retex_sha);
    let mut doc = stored_work(&f, &id);
    doc.project_memory = Some(vec![swarm_engine::ProjectMemory {
        id: "lesson".into(),
        imported: false,
        versions: Some(vec![v1, v2]),
    }]);
    rewrite(&f, &id, &doc);

    let store = RootStore::open(&f.0, false).unwrap();
    let catalog = store.memory_catalog("").unwrap();
    assert_eq!(catalog.method, "literal");
    assert_eq!(catalog.query, "");
    assert_eq!(catalog.entries.len(), 2);
    let historical = &catalog.entries[0];
    assert_eq!(historical.status, "historical");
    assert_eq!(historical.reason, "Version historique, non sélectionnable");
    assert_eq!(historical.reference.version, 1);
    assert_eq!(historical.work_revision, 1);
    assert_eq!(historical.source_title, "Catalogue mémoire");
    let latest = &catalog.entries[1];
    // Structurally valid but source-unfresh (task-1 is not accepted): the
    // catalog reports unavailable with Go's verifiedRetex reason text.
    assert_eq!(latest.status, "unavailable");
    assert_eq!(
        latest.reason,
        "Un fait sourcé et une tâche APEX acceptée avec des preuves fraîches sont requis."
    );

    // Folded literal query: accents case-fold through both sides and the
    // folded trimmed query is echoed back, as in Go.
    let filtered = store.memory_catalog("ÉTAPE").unwrap();
    assert_eq!(filtered.entries.len(), 1);
    assert_eq!(filtered.entries[0].reference.version, 1);
    assert_eq!(filtered.query, "etape");
    let miss = store.memory_catalog("inconnu-xyz").unwrap();
    assert!(miss.entries.is_empty());
    store.close().unwrap();
}

#[test]
fn root_catalog_refuses_whole_catalog_on_invalid_row() {
    let (f, id) = fixture_root_with_work();
    // One valid work precedes one invalid stored row: JSON that the
    // identity-projection UDF hashes fine but `decode_work_row` refuses
    // (no Work schema shape). This is exactly how an external corruption
    // appears to a reader — write-path guards never saw it.
    let doc = stored_work(&f, &id);
    let mut doc = verified_retex_doc(&doc);
    doc.project_memory = Some(vec![swarm_engine::ProjectMemory {
        id: "lesson".into(),
        imported: false,
        versions: Some(vec![]),
    }]);
    rewrite(&f, &id, &doc);
    open_db(&f)
        .execute(
            "INSERT INTO works(id,revision,body) VALUES('broken',1,?1)",
            rusqlite::params![&br#"{"x":1}"#[..]],
        )
        .unwrap();

    let store = RootStore::open(&f.0, false).unwrap();
    let error = store.memory_catalog("").unwrap_err();
    assert!(
        error.to_string().contains("row-incoherent"),
        "unexpected: {error}"
    );
    // The invalid row is retained exactly; nothing was rewritten or dropped.
    let retained: Vec<u8> = open_db(&f)
        .query_row(
            "SELECT body FROM works WHERE id='broken'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(retained, br#"{"x":1}"#.to_vec());
    store.close().unwrap();
}

#[test]
fn root_catalog_imported_entry_reports_unavailable_with_go_text() {
    let (f, id) = fixture_root_with_work();
    let doc = stored_work(&f, &id);
    let mut doc = verified_retex_doc(&doc);
    doc.project_memory = Some(vec![swarm_engine::ProjectMemory {
        id: "lesson".into(),
        imported: true,
        versions: Some(vec![MemoryVersion {
            version: 1,
            action: "admit".into(),
            kind: "procedure".into(),
            body: "imported elsewhere".into(),
            source_retex: "retex-1".into(),
            project_sha256: "other-project".into(),
            sha256: "unknown".into(),
            ..Default::default()
        }]),
    }]);
    rewrite(&f, &id, &doc);

    let store = RootStore::open(&f.0, false).unwrap();
    let catalog = store.memory_catalog("").unwrap();
    assert_eq!(catalog.entries.len(), 1);
    assert_eq!(catalog.entries[0].status, "unavailable");
    assert_eq!(
        catalog.entries[0].reason,
        "Mémoire importée ou d’un autre projet : admission locale requise"
    );
    store.close().unwrap();
}
