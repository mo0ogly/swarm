//! `swarm memory list` end-to-end over a real initialized Root: empty catalog
//! on a fresh Root, one entry per stored version with Go's reason text after a
//! direct fixture splice, and usage refusals for the unported subcommands.
//! Isolated tempfile Root; no live mission, provider or authority implied.
#![cfg(target_os = "linux")]

use std::io;

fn seed_work(root: &std::path::Path) -> String {
    let mut env = swarm_engine::SystemEnvironment;
    let mut store = swarm_engine::RootStore::initialize(root, &mut env).unwrap();
    let raw = serde_json::to_vec(&serde_json::json!({
        "schema_version":1,"event_id":"create","expected_revision":0,
        "title":"Catalogue","objective":"Preuve","scope":"Fixture","criteria":["Preuve"]
    }))
    .unwrap();
    let created = store
        .execute_external("", "work.create", &raw, &mut env)
        .unwrap();
    let id = created.result.work.id.clone();
    store.close().unwrap();
    id
}

fn splice_two_versions(root: &std::path::Path, id: &str, project_hash: &str) {
    use sha2::{Digest, Sha256};
    let db = rusqlite::Connection::open(root.join(".swarm/state.db")).unwrap();
    swarm_engine::register_identity_plan_projection(&db).unwrap();
    let raw: Vec<u8> = db
        .query_row("SELECT body FROM works WHERE id=?", [id], |row| row.get(0))
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
    let sealed = |version: i64, body: &str| -> swarm_engine::MemoryVersion {
        let mut v = swarm_engine::MemoryVersion {
            version,
            action: if version == 1 { "admit" } else { "replace" }.into(),
            kind: "procedure".into(),
            body: body.into(),
            conditions: "Recheck".into(),
            source_retex: "retex-1".into(),
            project_sha256: project_hash.into(),
            source_sha256: retex_sha.clone(),
            ..Default::default()
        };
        v.sha256 = swarm_engine::memory_version_sha(&v).unwrap();
        v
    };
    doc.project_memory = Some(vec![swarm_engine::ProjectMemory {
        id: "lesson".into(),
        imported: false,
        versions: Some(vec![sealed(1, "Première étape"), sealed(2, "Version courante")]),
    }]);
    let body = swarm_engine::encode_model(&doc).unwrap();
    db.execute(
        "UPDATE works SET body=? WHERE id=?",
        rusqlite::params![body, id],
    )
    .unwrap();
}

#[test]
fn memory_list_reports_empty_catalog_on_fresh_root() {
    let root = tempfile::TempDir::new().unwrap();
    seed_work(root.path());
    let args = ["memory", "list", "--root", root.path().to_str().unwrap(), "--json"]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut error = Vec::new();
    assert_eq!(swarm_cli::run(&args, "en", &mut out, &mut error), 0);
    let catalog: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(catalog["method"], "literal");
    assert_eq!(catalog["query"], "");
    assert_eq!(catalog["entries"].as_array().unwrap().len(), 0);
}

#[test]
fn memory_list_reports_versions_with_go_reasons_and_folded_query_refusal() {
    let root = tempfile::TempDir::new().unwrap();
    let id = seed_work(root.path());
    let project_hash = {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(root.path().to_string_lossy().as_bytes()))
    };
    splice_two_versions(root.path(), &id, &project_hash);
    let args = ["memory", "list", "--root", root.path().to_str().unwrap(), "--json"]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut error = Vec::new();
    assert_eq!(swarm_cli::run(&args, "en", &mut out, &mut error), 0);
    let catalog: serde_json::Value = serde_json::from_slice(&out).unwrap();
    let entries = catalog["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["status"], "historical");
    assert_eq!(
        entries[0]["reason"],
        "Version historique, non sélectionnable"
    );
    assert_eq!(entries[0]["ref"]["version"], 1);
    assert_eq!(entries[0]["work_revision"], 1);
    assert_eq!(entries[0]["source_title"], "Catalogue");
    // The latest version is structurally bound but source-unfresh (no accepted
    // task): the stable unavailable status with Go's verifiedRetex reason.
    assert_eq!(entries[1]["status"], "unavailable");
    assert_eq!(
        entries[1]["reason"],
        "Un fait sourcé et une tâche APEX acceptée avec des preuves fraîches sont requis."
    );
}

#[test]
fn memory_rejects_unported_subcommands_and_extra_arguments() {
    let root = tempfile::TempDir::new().unwrap();
    seed_work(root.path());
    let mut error = Vec::new();
    let mut out = Vec::new();
    // The mutation surface and M20 summary-source are later G2 increments:
    // usage refusal, no fabricated behavior.
    for args in [
        vec!["memory", "admit"],
        vec!["memory", "summary-source"],
        vec!["memory"],
        vec!["memory", "list", "extra"],
    ] {
        let args = args
            .into_iter()
            .map(String::from)
            .chain(["--root".into(), root.path().to_str().unwrap().into()])
            .collect::<Vec<_>>();
        assert_eq!(
            swarm_cli::run(&args, "en", &mut out, &mut error),
            2,
            "args: {args:?}"
        );
        out.clear();
        error.clear();
    }
}

#[test]
fn memory_list_human_output_adds_localized_heading() {
    let root = tempfile::TempDir::new().unwrap();
    seed_work(root.path());
    let args = ["memory", "list", "--root", root.path().to_str().unwrap()]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut error = Vec::new();
    assert_eq!(swarm_cli::run(&args, "fr", &mut out, &mut error), 0);
    let text = String::from_utf8(out).unwrap();
    assert!(text.starts_with("Catalogue mémoire\n"), "{text}");
    let mut out = Vec::new();
    let args = ["memory", "list", "--root", root.path().to_str().unwrap()]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    assert_eq!(swarm_cli::run(&args, "en", &mut out, &mut error), 0);
    assert!(String::from_utf8(out).unwrap().starts_with("Memory catalog\n"));
    let _ = io::sink();
}
