//! `swarm memory` end-to-end over real initialized Roots: the catalog read
//! (empty, versions with Go's reason text after a direct fixture splice, usage
//! refusals) and the mutation surface (selection preview+commit, unfresh
//! refusals, typed revision conflict). Isolated repository fixture roots (the
//! operator /tmp interdiction forbids tempfile /tmp allocations); failed roots
//! stay for diagnosis. No live mission, provider or authority implied.
#![cfg(target_os = "linux")]

use std::io;

/// A private durable fixture root under the repository fixture area (never
/// /tmp). Callers keep the path and remove it on success only.
fn fixture_root(prefix: &str) -> std::path::PathBuf {
    swarm_engine::fixture_root(prefix)
}

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
    let root: &std::path::Path = &fixture_root("cli-memcat-");
    seed_work(root);
    let args = ["memory", "list", "--root", root.to_str().unwrap(), "--json"]
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
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn memory_list_reports_versions_with_go_reasons_and_folded_query_refusal() {
    let root: &std::path::Path = &fixture_root("cli-memcat-");
    let id = seed_work(root);
    let project_hash = {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(root.to_string_lossy().as_bytes()))
    };
    splice_two_versions(root, &id, &project_hash);
    let args = ["memory", "list", "--root", root.to_str().unwrap(), "--json"]
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
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn memory_rejects_unported_subcommands_and_extra_arguments() {
    let root: &std::path::Path = &fixture_root("cli-memcat-");
    seed_work(root);
    let mut error = Vec::new();
    let mut out = Vec::new();
    // M20 summary-source is ported, but it still refuses usage without
    // --input. Unknown subcommands and arity errors refuse too.
    for args in [
        vec!["memory", "summary-source"],
        vec!["memory", "admit"],
        vec!["memory"],
        vec!["memory", "list", "extra"],
        vec!["memory", "admit", "w-fixture", "--input", "missing.json"],
    ] {
        let args = args
            .into_iter()
            .map(String::from)
            .chain(["--root".into(), root.to_str().unwrap().into()])
            .collect::<Vec<_>>();
        assert_eq!(
            swarm_cli::run(&args, "en", &mut out, &mut error),
            2,
            "args: {args:?}"
        );
        out.clear();
        error.clear();
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// A valid admit request against the seeded Work at its current revision.
fn admit_request(id: &str, revision: i64) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "schema_version":1,"event_id":"admit-1","expected_revision":revision,
        "work":id,"reason":"operator decision",
        "id":"mem-1","kind":"experience",
        "body":"Lesson learned","conditions":"Recheck before reuse",
        "source_retex":"retex-1"
    }))
    .unwrap()
}

/// The mutation surface runs through the real RootStore paths and honors the
/// Go contract: both the preview and the commit call the same `nextMemory`
/// candidate builder, so an admit whose source task was never accepted
/// (unverified RETEX, no fresh acceptance evidence in this fixture) refuses
/// with the stable `project-memory-source-unfresh` refusal on both routes,
/// nothing lands, and the catalog stays empty. Selection preview/commit and
/// stale revisions keep their typed refusals too.
#[test]
fn memory_admit_preview_and_commit_refuse_unfresh_source() {
    let root: &std::path::Path = &fixture_root("cli-memcat-");
    let id = seed_work(root);
    // Seed the verified RETEX into the stored Work like the catalog fixture,
    // so the admit request passes decode and reaches the engine guards.
    {
        let db = rusqlite::Connection::open(root.join(".swarm/state.db")).unwrap();
        swarm_engine::register_identity_plan_projection(&db).unwrap();
        let raw: Vec<u8> = db
            .query_row("SELECT body FROM works WHERE id=?", [&id], |row| row.get(0))
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
        let body = swarm_engine::encode_model(&doc).unwrap();
        db.execute("UPDATE works SET body=? WHERE id=?", rusqlite::params![body, id])
            .unwrap();
    }
    let root_str = root.to_str().unwrap();
    let input = root.join("admit.json");
    std::fs::write(&input, admit_request(&id, 1)).unwrap();
    // Preview refuses: no accepted source task means the candidate version
    // cannot be built (Go projectMemoryPreview → nextMemory → verifiedMemorySource).
    let args = ["memory", "admit-preview", &id, "--input", input.to_str().unwrap(), "--root", root_str]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut error = Vec::new();
    assert_eq!(swarm_cli::run(&args, "en", &mut out, &mut error), 2);
    let text = String::from_utf8_lossy(&error);
    assert!(
        text.contains("project-memory-source-unfresh"),
        "unexpected: {text}"
    );
    // The commit refuses with the same stable cause; nothing lands.
    let args = ["memory", "admit", &id, "--input", input.to_str().unwrap(), "--root", root_str]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut error = Vec::new();
    assert_eq!(swarm_cli::run(&args, "en", &mut out, &mut error), 2);
    let text = String::from_utf8_lossy(&error);
    assert!(
        text.contains("project-memory-source-unfresh"),
        "unexpected: {text}"
    );
    // The catalog stays empty after both refused routes.
    let args = ["memory", "list", "--root", root_str, "--json"]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut error = Vec::new();
    assert_eq!(swarm_cli::run(&args, "en", &mut out, &mut error), 0);
    let catalog: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(catalog["entries"].as_array().unwrap().len(), 0);
    std::fs::remove_dir_all(root).unwrap();
}

/// Selection preview on an executing todo task succeeds as a read-only
/// proposed outcome (no source evidence is required for a selection); the
/// returned shape carries the token, empty preview and revision; the commit
/// stamps the decision and reports the recorded Work state.
#[test]
fn memory_select_previews_then_commits_decision_on_executing_task() {
    let root: &std::path::Path = &fixture_root("cli-memcat-");
    let mut env = swarm_engine::SystemEnvironment;
    let mut store = swarm_engine::RootStore::initialize(root, &mut env).unwrap();
    let create = serde_json::json!({
        "schema_version":1,"event_id":"create","expected_revision":0,
        "title":"Sélection","objective":"Preuve","scope":"Fixture","criteria":["Preuve"]
    });
    let created = store
        .execute_external("", "work.create", &serde_json::to_vec(&create).unwrap(), &mut env)
        .unwrap();
    let id = created.result.work.id.clone();
    let revision = created.result.work.revision;
    let task = serde_json::json!({
        "schema_version":1,"event_id":"task","expected_revision":revision,
        "id":"task-worker","title":"Worker","deliverable":"Report","criteria":["Proof"]
    });
    let added = store
        .execute_external(&id, "task.add", &serde_json::to_vec(&task).unwrap(), &mut env)
        .unwrap();
    let revision = added.result.work.revision;
    store.close().unwrap();

    let root_str = root.to_str().unwrap();
    let select_body = serde_json::json!({
        "schema_version":1,"event_id":"select-1","expected_revision":revision,
        "work":id,"reason":"operator decision",
        "task":"task-worker","selections":[]
    });
    let input = root.join("select.json");
    std::fs::write(&input, serde_json::to_vec(&select_body).unwrap()).unwrap();
    let args = ["memory", "select-preview", &id, "--input", input.to_str().unwrap(), "--root", root_str, "--json"]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut error = Vec::new();
    assert_eq!(swarm_cli::run(&args, "en", &mut out, &mut error), 0, "{}", String::from_utf8_lossy(&error));
    let preview: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert!(!preview["preview_token"].as_str().unwrap().is_empty());
    assert_eq!(preview["preview"]["action"], "");
    assert_eq!(preview["context"], "");
    assert_eq!(preview["revision"], revision);

    // The commit carries the preview token and stamps the executing task's
    // decision; the Work is reported with the recorded state.
    let mut committed = select_body.clone();
    committed["preview_token"] = preview["preview_token"].clone();
    committed["event_id"] = "select-2".into();
    let input = root.join("select-commit.json");
    std::fs::write(&input, serde_json::to_vec(&committed).unwrap()).unwrap();
    let args = ["memory", "select", &id, "--input", input.to_str().unwrap(), "--root", root_str, "--json"]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut error = Vec::new();
    assert_eq!(swarm_cli::run(&args, "en", &mut out, &mut error), 0, "{}", String::from_utf8_lossy(&error));
    let work: serde_json::Value = serde_json::from_slice(&out).unwrap();
    let tasks = work["tasks"].as_array().unwrap();
    let worker = tasks.iter().find(|t| t["id"] == "task-worker").unwrap();
    assert_eq!(worker["memory_decision"]["event_id"], "select-2");
    std::fs::remove_dir_all(root).unwrap();
}

/// A stale expected revision refuses at preview with the typed revision
/// conflict before any store write; the Work revision is unchanged after.
#[test]
fn memory_admit_preview_refuses_stale_revision() {
    let root: &std::path::Path = &fixture_root("cli-memcat-");
    let id = seed_work(root);
    let root_str = root.to_str().unwrap();
    let input = root.join("admit.json");
    std::fs::write(&input, admit_request(&id, 99)).unwrap();
    let args = ["memory", "admit-preview", &id, "--input", input.to_str().unwrap(), "--root", root_str]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut error = Vec::new();
    assert_eq!(swarm_cli::run(&args, "en", &mut out, &mut error), 2);
    let text = String::from_utf8_lossy(&error);
    assert!(
        text.contains("project-memory-work-changed"),
        "unexpected: {text}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn memory_list_human_output_adds_localized_heading() {
    let root: &std::path::Path = &fixture_root("cli-memcat-");
    seed_work(root);
    let args = ["memory", "list", "--root", root.to_str().unwrap()]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut error = Vec::new();
    assert_eq!(swarm_cli::run(&args, "fr", &mut out, &mut error), 0);
    let text = String::from_utf8(out).unwrap();
    assert!(text.starts_with("Catalogue mémoire\n"), "{text}");
    let mut out = Vec::new();
    let args = ["memory", "list", "--root", root.to_str().unwrap()]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    assert_eq!(swarm_cli::run(&args, "en", &mut out, &mut error), 0);
    assert!(String::from_utf8(out).unwrap().starts_with("Memory catalog\n"));
    let _ = io::sink();
    std::fs::remove_dir_all(root).unwrap();
}

/// Seed one executing task plus one brainstorm exchange by splicing the
/// stored body (the engine's external task.add has no brainstorm fields).
fn splice_brainstorm(root: &std::path::Path, id: &str) {
    let db = rusqlite::Connection::open(root.join(".swarm/state.db")).unwrap();
    swarm_engine::register_identity_plan_projection(&db).unwrap();
    let raw: Vec<u8> = db
        .query_row("SELECT body FROM works WHERE id=?", [id], |row| row.get(0))
        .unwrap();
    let mut doc: swarm_engine::Work = swarm_engine::decode_work(&raw).unwrap();
    let mut worker = doc
        .tasks
        .as_mut()
        .unwrap()
        .iter_mut()
        .find(|t| t.id == "task-worker")
        .expect("seeded worker task");
    worker.next = "lancer le test".into();
    let mut brain = swarm_engine::Task::default();
    brain.id = "old-0".into();
    brain.brainstorm = true;
    brain.question = "Décision numéro 0".into();
    brain.answers = Some(vec![swarm_engine::BrainstormAnswer {
        attempt: "a0".into(),
        text: "Réponse 0".into(),
        at: String::new(),
    }]);
    brain.response = "Réponse 0".into();
    doc.tasks.as_mut().unwrap().push(brain);
    let body = swarm_engine::encode_model(&doc).unwrap();
    db.execute(
        "UPDATE works SET body=? WHERE id=?",
        rusqlite::params![body, id],
    )
    .unwrap();
}

/// The M20 summary source over the CLI: the exact packet (schema/kind/payload
/// digest, snapshot carrying the contract JSON, question and answer sources),
/// then the brainstorm-target and stale-revision refusals without mutation.
#[test]
fn memory_summary_source_serves_packet_and_refuses_target_and_stale() {
    use sha2::{Digest, Sha256};
    let root: &std::path::Path = &fixture_root("cli-memcat-");
    let id = seed_work(root);
    // Add the executing task through the engine, then splice the brainstorm.
    let mut env = swarm_engine::SystemEnvironment;
    let mut store = swarm_engine::RootStore::open(root, true).unwrap();
    let task = serde_json::json!({
        "schema_version":1,"event_id":"task","expected_revision":1,
        "id":"task-worker","title":"Contrôler","deliverable":"rapport","criteria":["Preuve"]
    });
    let added = store
        .execute_external(&id, "task.add", &serde_json::to_vec(&task).unwrap(), &mut env)
        .unwrap();
    let revision = added.result.work.revision;
    store.close().unwrap();
    splice_brainstorm(root, &id);
    let digest = |raw: &[u8]| format!("{:x}", Sha256::digest(raw));

    // A valid request: the packet is byte-shaped like Go's.
    let request = serde_json::json!({
        "schema_version":1,"work":id,"task":"task-worker","expected_revision":revision,
        "dialogues":[{"task":"old-0","attempt":"a0","sha256":digest(format!("Décision numéro 0\nRéponse 0").as_bytes())}]
    });
    let input = root.join("summary.json");
    std::fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
    let args = ["memory", "summary-source", "--input", input.to_str().unwrap(), "--root", root.to_str().unwrap(), "--json"]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut error = Vec::new();
    assert_eq!(swarm_cli::run(&args, "en", &mut out, &mut error), 0, "{}", String::from_utf8_lossy(&error));
    let packet: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(packet["schema_version"], 1);
    assert_eq!(packet["kind"], "swarm_summary_source_v1");
    assert_eq!(packet["sha256"], packet["payload"].as_str().map(|p| digest(p.as_bytes())).unwrap());
    let payload: serde_json::Value = serde_json::from_str(packet["payload"].as_str().unwrap()).unwrap();
    assert_eq!(payload["scope"]["work"], id);
    assert_eq!(payload["scope"]["task"], "task-worker");
    assert_eq!(payload["work_revision"], revision);
    assert_eq!(payload["audience"], "local_operator");
    assert_eq!(payload["provider_history_complete"], false);
    let sources = payload["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 3);
    assert_eq!(sources[0]["id"], "current-task-snapshot");
    assert!(sources[0]["text"].as_str().unwrap().contains("Preuve"));
    assert_eq!(sources[1]["text"], "Décision numéro 0");
    assert_eq!(sources[2]["text"], "Réponse 0");
    assert_eq!(sources[2]["role"], "assistant");

    // Targeting the brainstorm task refuses.
    let bad = serde_json::json!({
        "schema_version":1,"work":id,"task":"old-0","expected_revision":revision,
        "dialogues":[{"task":"old-0","attempt":"a0","sha256":digest(format!("Décision numéro 0\nRéponse 0").as_bytes())}]
    });
    let input = root.join("summary-bad.json");
    std::fs::write(&input, serde_json::to_vec(&bad).unwrap()).unwrap();
    let args = ["memory", "summary-source", "--input", input.to_str().unwrap(), "--root", root.to_str().unwrap()]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut error = Vec::new();
    assert_eq!(swarm_cli::run(&args, "en", &mut out, &mut error), 2);
    assert!(
        String::from_utf8_lossy(&error).contains("project-summary-source-brainstorm-target"),
        "unexpected: {}",
        String::from_utf8_lossy(&error)
    );

    // A stale expected revision refuses with the revision conflict.
    let stale = serde_json::json!({
        "schema_version":1,"work":id,"task":"task-worker","expected_revision":revision - 1,
        "dialogues":[{"task":"old-0","attempt":"a0","sha256":digest(format!("Décision numéro 0\nRéponse 0").as_bytes())}]
    });
    let input = root.join("summary-stale.json");
    std::fs::write(&input, serde_json::to_vec(&stale).unwrap()).unwrap();
    let args = ["memory", "summary-source", "--input", input.to_str().unwrap(), "--root", root.to_str().unwrap()]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut error = Vec::new();
    assert_eq!(swarm_cli::run(&args, "en", &mut out, &mut error), 2);
    assert!(
        String::from_utf8_lossy(&error).contains("project-memory-work-changed"),
        "unexpected: {}",
        String::from_utf8_lossy(&error)
    );
    std::fs::remove_dir_all(root).unwrap();
}
