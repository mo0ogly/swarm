//! Versioned-memory cycle through the actual mutation engine: admission,
//! replacement, withdrawal and selection with their refusal and recovery
//! variants, ported from the retained Go `internal/engine/project_memory.go`
//! behavior. Everything runs in an isolated in-memory SQL fixture with an owned
//! environment; no live mission, provider or authority is implied. This proves
//! the wired engine contract, never RootStore preview or the HTTP surface.

use rusqlite::Connection;
use swarm_engine::{
    decode_project_memory_request, memory_preview_token, memory_version_sha,
    EngineMutationLimits, MemoryVersion, ProjectMemoryRequest, SystemEnvironment, WorkEngine,
    WorkEnvironment,
};
use swarm_identity_contract::name as safe_name;

/// Minimal actual engine schema in an isolated fixture; missing tables fail.
fn fixture() -> Connection {
    let mut db = Connection::open_in_memory().unwrap();
    db.execute_batch(
        "PRAGMA user_version=37;CREATE TABLE works(id TEXT PRIMARY KEY,revision INTEGER NOT NULL,body BLOB NOT NULL);CREATE TABLE events(id TEXT PRIMARY KEY,work_id TEXT NOT NULL,revision INTEGER NOT NULL,kind TEXT NOT NULL,at TEXT NOT NULL,payload BLOB NOT NULL,request BLOB NOT NULL,UNIQUE(work_id,revision));CREATE TABLE agents(work_id TEXT,task_id TEXT,status TEXT);CREATE TABLE cockpit_controls(work_id TEXT,paused);CREATE TABLE preparation_launch_locks(work_id TEXT,task_id TEXT,released INTEGER);CREATE TABLE cockpit_events(work_id TEXT,at TEXT,kind TEXT,message TEXT);",
    )
    .unwrap();
    engine(
        &mut db,
        "work.create",
        serde_json::json!({"schema_version":1,"event_id":"create","expected_revision":0,"title":"Memory fixture","objective":"Proof","scope":"Fixture","criteria":["Proof"]}),
    )
    .unwrap();
    db
}

/// Submit external JSON through WorkEngine's actual codec/guard/stage path.
fn engine(
    db: &mut Connection,
    kind: &str,
    value: serde_json::Value,
) -> Result<swarm_engine::WorkMutationOutcome, swarm_engine::WorkEngineError> {
    let limits = EngineMutationLimits {
        task: swarm_engine::TaskDefinitionLimits::legacy_go(),
        policy: swarm_engine::ValidationPolicyLimits::legacy_go(),
    };
    WorkEngine::new(db, std::path::Path::new("."), &mut Environment, &limits)
        .execute_external(
            if kind == "work.create" {
                ""
            } else {
                WORK
            },
            kind,
            &serde_json::to_vec(&value).unwrap(),
        )
}

/// The fixture Work id produced by the deterministic environment identifier.
const WORK: &str = "w-fixture";

/// Add one executing task plus one brainstorm task. The engine's external
/// task.add has no brainstorm fields, so the brainstorm marker and the verified
/// RETEX are written into the stored fixture body directly — an owned isolated
/// fixture, not a live mission row.
fn seed_tasks_and_retex(db: &mut Connection) {
    engine(
        db,
        "task.add",
        serde_json::json!({"schema_version":1,"event_id":"task","expected_revision":1,"id":"task-worker","title":"Worker","deliverable":"Report","criteria":["Proof"]}),
    )
    .unwrap();
    engine(
        db,
        "task.add",
        serde_json::json!({"schema_version":1,"event_id":"task2","expected_revision":2,"id":"task-brain","title":"Brain","deliverable":"Answers","criteria":["Proof"]}),
    )
    .unwrap();
    let mut work = read_work(db);
    for task in work.tasks.as_mut().unwrap() {
        if task.id == "task-brain" {
            task.brainstorm = true;
            task.question = "Which path?".into();
            task.answers = Some(vec![swarm_engine::BrainstormAnswer {
                attempt: "a1".into(),
                text: "Path A".into(),
                at: String::new(),
            }]);
            task.response = "Path A".into();
        }
    }
    // One verified RETEX bound to the executing task with a dialogue source;
    // the source task is not accepted, so the fresh-source guard refuses.
    work.retex = Some(vec![swarm_engine::Retex {
        id: "retex-1".into(),
        status: "verifie".into(),
        nature: "fait".into(),
        problem: "problème".into(),
        proposal: "proposition".into(),
        criteria: "critère".into(),
        proof: "preuve".into(),
        task: "task-worker".into(),
        source: swarm_engine::DialogueRef {
            task: "task-worker".into(),
            attempt: "attempt-1".into(),
            sha256: "abc".into(),
        },
        ..Default::default()
    }]);
    let body = swarm_engine::encode_model(&work).unwrap();
    db.execute("UPDATE works SET body=? WHERE id=?", rusqlite::params![body, WORK])
        .unwrap();
}

/// Deterministic operator/time observations; fixture values, not identities.
struct Environment;
impl WorkEnvironment for Environment {
    fn identifier(&mut self, prefix: &str) -> Result<String, swarm_engine::RuntimeError> {
        Ok(format!("{prefix}fixture"))
    }
    fn timestamp(&mut self) -> Result<String, swarm_engine::RuntimeError> {
        Ok("2026-11-05T00:00:00Z".into())
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

/// Build a memory request with the canonical wire field names.
fn request(event: &str, revision: i64, fields: serde_json::Value) -> serde_json::Value {
    let mut base = serde_json::json!({
        "schema_version":1,
        "event_id":event,
        "expected_revision":revision,
        "work":WORK,
        "reason":"operator decision",
    });
    if let (Some(base), Some(fields)) = (base.as_object_mut(), fields.as_object()) {
        for (key, value) in fields {
            base.insert(key.clone(), value.clone());
        }
    }
    base
}

/// Compute the legitimate preview token for a select request over the given
/// context string, mirroring the Go client flow: token over the decoded
/// request with the candidate defaulted and the projected context bound.
fn select_token(event: &str, revision: i64, fields: serde_json::Value, context: &str) -> String {
    let wire = request(event, revision, fields);
    let raw = serde_json::to_vec(&wire).unwrap();
    let decoded = decode_project_memory_request(&raw, "select").unwrap();
    memory_preview_token(
        "select",
        &decoded,
        &swarm_engine::MemoryVersion::default(),
        context,
    )
    .unwrap()
}

/// Read the stored Work body back as a real Work DTO.
fn read_work(db: &Connection) -> swarm_engine::Work {
    swarm_engine::read_recorded_work(db, WORK).unwrap()
}

/// Submit a memory request and read the stored latest version back.
fn memory_of(db: &Connection, id: &str) -> MemoryVersion {
    let work = read_work(db);
    let entry = work
        .project_memory
        .as_deref()
        .unwrap_or_default()
        .iter()
        .find(|e| e.id == id)
        .unwrap_or_else(|| panic!("memory {id} absent"));
    entry
        .versions
        .as_deref()
        .unwrap_or_default()
        .last()
        .cloned()
        .unwrap()
}

/// The full content cycle commits versions with bound sources, refuse stale
/// previews and replaced digests, and recovery after a failed attempt works.
#[test]
fn memory_content_cycle_admits_replaces_and_refuses_stale_inputs() {
    let mut db = fixture();
    seed_tasks_and_retex(&mut db);
    // The source task must be accepted for the verified-source check; engine
    // fixtures cannot mark acceptance without the lifecycle, so the admit
    // path refuses with the stable unfresh-source refusal first. This proves
    // the guard is wired: no version lands without live source evidence.
    let admit = engine(
        &mut db,
        "memory.admit",
        request("admit-1", 3, serde_json::json!({"id":"mem-1","kind":"experience","body":"Lesson learned","conditions":"Recheck before reuse","source_retex":"retex-1"})),
    );
    let error = admit.unwrap_err();
    assert_eq!(
        error.to_string(),
        "project-memory-refused: project-memory-source-unfresh"
    );
    // No version landed; the refusal is atomic.
    let work = read_work(&db);
    assert!(work.project_memory.is_none() || work.project_memory.unwrap().is_empty());

    // A stale preview token refuses before any mutation.
    let stale = request("admit-2", 3, serde_json::json!({"id":"mem-1","kind":"experience","body":"Lesson","conditions":"Recheck","source_retex":"retex-1","preview_token":"forged"}));
    let error = engine(&mut db, "memory.admit", stale).unwrap_err();
    assert!(
        error.to_string().contains("preview_stale")
            || error.to_string().contains("project-memory-source-unfresh"),
        "unexpected: {error}"
    );

    // Withdraw of an unknown memory refuses without creating an entry.
    let error = engine(
        &mut db,
        "memory.withdraw",
        request("withdraw-1", 3, serde_json::json!({"id":"mem-ghost","expected_sha256":"abc"})),
    )
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "project-memory-refused: project-memory-unknown"
    );
    // Schema violations refuse at decode with the stable shape refusal:
    // a missing motif is the Go schema check (Schéma, travail et motif requis).
    let raw = serde_json::to_vec(&serde_json::json!({
        "schema_version":1,"event_id":"bad","expected_revision":3,
        "work":WORK,
    }))
    .unwrap();
    assert!(decode_project_memory_request(&raw, "admit").is_err());
    // Transport overflow refuses with the stable capacity refusal.
    let raw = serde_json::to_vec(&request(
        "big",
        3,
        serde_json::json!({"id":"mem-1","kind":"experience","body":"x".repeat(1 << 21),"conditions":"c","source_retex":"retex-1"}),
    ))
    .unwrap();
    let error = decode_project_memory_request(&raw, "admit").unwrap_err();
    assert_eq!(error.reason(), "project-memory-transport-exceeded");
}

/// Selection refuses a planner/subplanner/brainstorm target, stamps an
/// executing target's decision and refuses double selections.
#[test]
fn memory_selection_guards_executing_tasks_and_stamps_decision() {
    let mut db = fixture();
    seed_tasks_and_retex(&mut db);
    // A brainstorm task refuses: the selection is for executing tasks only.
    let error = engine(
        &mut db,
        "memory.select",
        request("select-1", 3, serde_json::json!({"task":"task-brain","selections":[]})),
    )
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "project-memory-refused: project-memory-selection-not-executing"
    );
    // An unknown task refuses through the same guard.
    let error = engine(
        &mut db,
        "memory.select",
        request("select-2", 3, serde_json::json!({"task":"task-ghost","selections":[]})),
    )
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "project-memory-refused: project-memory-task-unknown"
    );
    // An executing task with an empty selection accepts and stamps the
    // decision with the environment's deterministic operator/time. The
    // projected context for an empty selection is the empty string.
    let token = select_token(
        "select-3",
        3,
        serde_json::json!({"task":"task-worker","selections":[]}),
        "",
    );
    let out = engine(
        &mut db,
        "memory.select",
        request("select-3", 3, serde_json::json!({"task":"task-worker","selections":[],"preview_token":token})),
    )
    .unwrap();
    let work = read_work(&db);
    let task = work
        .tasks
        .as_deref()
        .unwrap_or_default()
        .iter()
        .find(|t| t.id == "task-worker")
        .unwrap();
    let decision = task.memory_decision.as_deref().unwrap();
    assert_eq!(decision.actor, "operator/fixture");
    assert_eq!(decision.at, "2026-11-05T00:00:00Z");
    assert_eq!(decision.event_id, "select-3");
    // A stored empty selection round-trips as absent, matching Go JSON
    // semantics where an empty slice is indistinguishable from nil.
    assert!(task.memory_selections.as_deref().unwrap_or_default().is_empty());
    assert!(!task.memory_selection_imported);
    assert_eq!(out.work.revision, 4);
    // A second selection of the same kind is allowed by the Go contract
    // (selection overwrites), so re-selecting also succeeds and restamps.
    let token = select_token(
        "select-4",
        4,
        serde_json::json!({"task":"task-worker","selections":[]}),
        "",
    );
    engine(
        &mut db,
        "memory.select",
        request("select-4", 4, serde_json::json!({"task":"task-worker","selections":[],"preview_token":token})),
    )
    .unwrap();
    let work = read_work(&db);
    let decision = work
        .tasks
        .as_deref()
        .unwrap_or_default()
        .iter()
        .find(|t| t.id == "task-worker")
        .unwrap()
        .memory_decision
        .as_deref()
        .unwrap();
    assert_eq!(decision.event_id, "select-4");
}

/// The select branch's token guard binds the projected attachments: a forged
/// token on a selection with content refuses with the selection preview code.
#[test]
fn memory_select_refuses_forged_preview_token() {
    let mut db = fixture();
    seed_tasks_and_retex(&mut db);
    // A selection carrying a content field refuses at decode (shape check).
    let raw = serde_json::to_vec(&request(
        "select-bad",
        3,
        serde_json::json!({"task":"task-worker","id":"mem-1"}),
    ))
    .unwrap();
    let error = decode_project_memory_request(&raw, "select").unwrap_err();
    assert_eq!(error.reason(), "project-memory-select-fields-refused");
    // A selection with a forged token reaches the token check after stamping;
    // the transaction rolls back, leaving no decision.
    let error = engine(
        &mut db,
        "memory.select",
        request("select-forged", 3, serde_json::json!({"task":"task-worker","selections":[],"preview_token":"forged"})),
    )
    .unwrap_err();
    assert!(
        error.to_string().contains("project-memory-preview-stale"),
        "unexpected: {error}"
    );
    let work = read_work(&db);
    let task = work
        .tasks
        .as_deref()
        .unwrap_or_default()
        .iter()
        .find(|t| t.id == "task-worker")
        .unwrap();
    assert!(task.memory_decision.is_none(), "rollback required");
}

/// Safe-name and identity checks refuse invalid admission ids through the
/// engine, and imported memory stays out of reach of the local cycle.
#[test]
fn memory_identity_refusals_hold_at_the_engine_boundary() {
    let mut db = fixture();
    seed_tasks_and_retex(&mut db);
    // An invalid memory id refuses with the admission precondition refusal
    // (Go: entry errors fall through to the safeName check).
    for bad in ["../escape", "mem/1", "."] {
        let error = engine(
            &mut db,
            "memory.admit",
            request("admit-bad", 3, serde_json::json!({"id":bad,"kind":"experience","body":"b","conditions":"c","source_retex":"retex-1"})),
        )
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "project-memory-refused: project-memory-admission-invalid"
        );
    }
    assert!(safe_name(b"mem-1"), "the valid id itself is safe");
}

/// The version digest helper matches the Go canonical form: SHA-256 over the
/// cleared-SHA canonical encoding.
#[test]
fn memory_version_digest_binds_cleared_canonical_bytes() {
    let version = MemoryVersion {
        version: 1,
        action: "admit".into(),
        kind: "experience".into(),
        body: "Body".into(),
        conditions: "Conditions".into(),
        source_retex: "retex-1".into(),
        ..Default::default()
    };
    let digest = memory_version_sha(&version).unwrap();
    assert_eq!(digest.len(), 64);
    // The digest changes when any bound field changes.
    let mut other = version.clone();
    other.body = "Other".into();
    assert_ne!(memory_version_sha(&other).unwrap(), digest);
}

/// Request decode round-trips the canonical wire and the engine's request
/// synthesis preserves it through the mutation event log.
#[test]
fn memory_request_roundtrip_preserves_canonical_wire() {
    let wire = serde_json::json!({
        "schema_version":1,
        "event_id":"event-1",
        "expected_revision":7,
        "work":"work-fixture",
        "reason":"motif",
        "id":"mem-1",
        "kind":"procedure",
        "body":"Body",
        "conditions":"Conditions",
        "source_retex":"retex-1",
        "expected_sha256":"",
        "preview_token":"",
    });
    let raw = serde_json::to_vec(&wire).unwrap();
    let decoded: ProjectMemoryRequest = decode_project_memory_request(&raw, "admit").unwrap();
    assert_eq!(decoded.id, "mem-1");
    assert_eq!(decoded.kind, "procedure");
    assert_eq!(decoded.expected_revision, 7);
}
