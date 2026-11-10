//! Extractive summary source ported from the retained Go
//! `internal/engine/memory_summary_source.go` (M20). Builds the
//! `swarm_summary_source_v1` packet an operator hands to a summarization
//! provider: the current task snapshot (its own strict contract JSON) plus the
//! explicitly selected brainstorm exchanges, each with its verbatim text and
//! digest. The packet payload is a string so the exact Go-produced bytes stay
//! verifiable across runtimes. Read-only projection over one stored Work row:
//! no summarization call, no admission, no Work mutation and no authority
//! grant happens here. A read/decode failure refuses instead of publishing a
//! partial packet.

use crate::models::{DialogueRef, Work};
use crate::project_memory::MemoryError;
use swarm_identity_contract::name as safe_name;

/// Go `MemorySummarySourceRequest`: the operator's selection for one summary.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct MemorySummarySourceRequest {
    #[serde(rename = "schema_version")]
    pub schema_version: i64,
    #[serde(rename = "work")]
    pub work: String,
    #[serde(rename = "task")]
    pub task: String,
    #[serde(rename = "expected_revision")]
    pub expected_revision: i64,
    #[serde(rename = "dialogues")]
    pub dialogues: Vec<DialogueRef>,
}

/// Go `MemorySummarySourcePacket`. The payload is a String so other runtimes
/// can verify the exact Go-produced bytes without assuming identical JSON
/// serialization or escaping rules.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MemorySummarySourcePacket {
    #[serde(rename = "schema_version")]
    pub schema_version: i64,
    #[serde(rename = "kind")]
    pub kind: String,
    #[serde(rename = "payload")]
    pub payload: String,
    #[serde(rename = "sha256")]
    pub sha256: String,
}

/// Go `memorySummarySource`: one verbatim source line of the packet.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MemorySummarySource {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "role")]
    pub role: String,
    #[serde(rename = "kind")]
    pub kind: String,
    #[serde(rename = "text")]
    pub text: String,
    #[serde(rename = "sha256")]
    pub sha256: String,
    #[serde(rename = "dialogue", skip_serializing_if = "Option::is_none")]
    pub dialogue: Option<DialogueRef>,
}

/// Wire fields inherited from Go `MemorySummarySourceRequest`; the strict
/// projector refuses unknown fields and invalid types. No defaults beyond
/// Go's zero values.
fn summary_source_shape() -> crate::model_wire::Fields {
    use crate::models::shape_dialogue_ref;
    use crate::model_wire::Kind;
    &[
        ("schema_version", Kind::Int64),
        ("work", Kind::String),
        ("task", Kind::String),
        ("expected_revision", Kind::Int64),
        (
            "dialogues",
            Kind::Array(&Kind::Object(shape_dialogue_ref)),
        ),
    ]
}

/// Go `Store.memorySummarySource` head: transport ceiling first (1 MiB over
/// the raw request, mirroring Go's order), strict decode, then schema/work/
/// task/revision/dialogues validation. Row resolution belongs to the caller's
/// consistency boundary.
///
/// # Errors
///
/// `MemoryError::Refused` with Go's message semantics on transport overflow
/// or schema/work/task/revision/dialogues violations; `MemoryError::Row`
/// retains the strict-decode cause.
pub fn decode_summary_source_request(raw: &[u8]) -> Result<MemorySummarySourceRequest, MemoryError> {
    if raw.len() > crate::project_memory::PROJECT_MEMORY_TRANSPORT_BYTES {
        return Err(MemoryError::Refused(
            "project-summary-source-transport-exceeded",
        ));
    }
    let request: MemorySummarySourceRequest =
        crate::work::decode_mode(raw, summary_source_shape(), true).map_err(MemoryError::Row)?;
    if request.schema_version != 1
        || request.expected_revision < 1
        || !safe_name(request.work.as_bytes())
        || !safe_name(request.task.as_bytes())
        || request.dialogues.is_empty()
    {
        return Err(MemoryError::Refused(
            "project-summary-source-request-schema-invalid",
        ));
    }
    Ok(request)
}

/// Go `memorySummarySourceForWork`: resolve the same source contract on a
/// caller-owned Work snapshot. Identity, revision, task nature, exchange
/// availability and duplicate selection are all checked against this one
/// snapshot; the snapshot itself stays untouched. Pure over the caller's
/// data except for the root digest.
///
/// # Errors
///
/// `MemoryError::RevisionConflict` on a stale expected revision;
/// `MemoryError::Refused` on identity mismatch, unknown task, brainstorm
/// target, missing/modified exchange, empty question/answer or a duplicate
/// dialogue selection; `MemoryError::Row` retains a serialization cause.
pub fn summary_source_for_work(
    root: &std::path::Path,
    work: &Work,
    request: &MemorySummarySourceRequest,
) -> Result<MemorySummarySourcePacket, MemoryError> {
    if work.id != request.work {
        return Err(MemoryError::Refused(
            "project-summary-source-work-identity-mismatch",
        ));
    }
    if work.revision != request.expected_revision {
        return Err(MemoryError::RevisionConflict(
            "project-memory-work-changed",
        ));
    }
    let task = work
        .tasks
        .as_deref()
        .unwrap_or_default()
        .iter()
        .find(|t| t.id == request.task)
        .ok_or(MemoryError::Refused("project-memory-task-unknown"))?;
    if task.brainstorm {
        return Err(MemoryError::Refused(
            "project-summary-source-brainstorm-target",
        ));
    }
    // Go hash([]byte(root)): digest of the root path string.
    let project_hash = crate::agent_launch_preflight::hash(root.to_string_lossy().as_bytes());
    // This operator snapshot is deliberately not a delegated worker context:
    // no sibling task/report, tool output or private guard is loaded. The
    // anonymous Go struct's field order is preserved exactly.
    #[derive(serde::Serialize)]
    struct Contract<'a> {
        #[serde(rename = "project_sha256")]
        project: &'a str,
        #[serde(rename = "work_title")]
        title: &'a str,
        #[serde(rename = "work_objective")]
        objective: &'a str,
        #[serde(rename = "work_scope")]
        scope: &'a str,
        #[serde(rename = "work_criteria")]
        criteria: Option<&'a [String]>,
        #[serde(rename = "task")]
        task: &'a crate::models::Task,
    }
    let contract = crate::encode_model(&Contract {
        project: &project_hash,
        title: &work.title,
        objective: &work.objective,
        scope: &work.scope,
        criteria: work.criteria.as_deref(),
        task,
    })
    .map_err(MemoryError::Row)?;
    let snapshot_sha = crate::agent_launch_preflight::hash(&contract);
    let mut sources = vec![MemorySummarySource {
        id: "current-task-snapshot".to_owned(),
        role: "user".to_owned(),
        kind: "task_snapshot".to_owned(),
        text: String::from_utf8(contract).expect("encode_model produces UTF-8"),
        sha256: snapshot_sha,
        dialogue: None,
    }];
    for reference in &request.dialogues {
        if sources
            .iter()
            .skip(1)
            .filter_map(|s| s.dialogue.as_ref())
            .any(|seen| seen == reference)
        {
            return Err(MemoryError::Refused(
                "project-summary-source-dialogue-duplicate",
            ));
        }
        let hit = crate::project_memory::resolve_dialogue(work, reference)?;
        if !crate::text::nonempty(&hit.question) || !crate::text::nonempty(&hit.text) {
            return Err(MemoryError::Refused(
                "project-summary-source-exchange-empty",
            ));
        }
        let ref_json = crate::encode_model(reference).map_err(MemoryError::Row)?;
        let id = format!(
            "dialogue-{}",
            crate::agent_launch_preflight::hash(&ref_json)
        );
        sources.push(MemorySummarySource {
            id: format!("{}-question", id),
            role: "user".to_owned(),
            kind: "operator_question".to_owned(),
            text: hit.question.clone(),
            sha256: crate::agent_launch_preflight::hash(hit.question.as_bytes()),
            dialogue: Some(reference.clone()),
        });
        sources.push(MemorySummarySource {
            id: format!("{}-answer", id),
            role: "assistant".to_owned(),
            kind: "historical_answer".to_owned(),
            text: hit.text.clone(),
            sha256: crate::agent_launch_preflight::hash(hit.text.as_bytes()),
            dialogue: Some(reference.clone()),
        });
    }
    // Go anonymous payload struct: field order preserved exactly.
    #[derive(serde::Serialize)]
    struct Payload<'a> {
        #[serde(rename = "scope")]
        scope: Scope<'a>,
        #[serde(rename = "work_revision")]
        revision: i64,
        #[serde(rename = "project_sha256")]
        project: &'a str,
        #[serde(rename = "audience")]
        audience: &'a str,
        // bool zero: never set, mirroring Go's field with no explicit value.
        #[serde(rename = "provider_history_complete")]
        complete: bool,
        #[serde(rename = "sources")]
        sources: &'a [MemorySummarySource],
    }
    #[derive(serde::Serialize)]
    struct Scope<'a> {
        #[serde(rename = "work")]
        work: &'a str,
        #[serde(rename = "task")]
        task: &'a str,
    }
    let payload = crate::encode_model(&Payload {
        scope: Scope {
            work: &work.id,
            task: &task.id,
        },
        revision: work.revision,
        project: &project_hash,
        audience: "local_operator",
        complete: false,
        sources: &sources,
    })
    .map_err(MemoryError::Row)?;
    let payload_sha = crate::agent_launch_preflight::hash(&payload);
    Ok(MemorySummarySourcePacket {
        schema_version: 1,
        kind: "swarm_summary_source_v1".to_owned(),
        payload: String::from_utf8(payload).expect("encode_model produces UTF-8"),
        sha256: payload_sha,
    })
}

/// Read-only summary source over one opened Root: the Work row is the complete
/// consistency boundary (task definition and selected answers from the same
/// stored row), mirroring Go's single `readWorkRow`. The caller owns Root
/// open/close and binding; this function performs no settings read,
/// reservation or write.
pub fn summary_source_from_connection(
    connection: &rusqlite::Connection,
    root: &std::path::Path,
    raw: &[u8],
) -> Result<MemorySummarySourcePacket, MemoryError> {
    let request = decode_summary_source_request(raw)?;
    let work = crate::read_recorded_work(connection, &request.work).map_err(|e| match e {
        crate::recorded_work::RecordedWorkError::Sql(sql) => MemoryError::Sql(sql),
        crate::recorded_work::RecordedWorkError::Work(row) => MemoryError::Row(row),
        crate::recorded_work::RecordedWorkError::Missing
        | crate::recorded_work::RecordedWorkError::Finalize { .. } => {
            MemoryError::Refused("project-memory-work-unknown")
        }
    })?;
    summary_source_for_work(root, &work, &request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode_model;
    use crate::models::BrainstormAnswer;
    use sha2::{Digest, Sha256};

    /// Go summarySourceFixture: one executing task + one brainstorm task with
    /// two recorded exchanges, mirroring memorySeed's old-0/old-1 shape.
    fn fixture_work() -> Work {
        let mut work = Work {
            schema_version: 1,
            revision: 7,
            id: "w-fixture".to_owned(),
            title: "Contrôler les preuves".to_owned(),
            objective: "Preuve".to_owned(),
            scope: "Fixture".to_owned(),
            criteria: Some(vec!["tests PASS".to_owned()]),
            ..Default::default()
        };
        let mut worker = crate::models::Task::default();
        worker.id = "t1".to_owned();
        worker.title = "Contrôler les preuves".to_owned();
        worker.deliverable = "rapport de test".to_owned();
        worker.criteria = Some(vec!["tests PASS".to_owned()]);
        worker.owner = "session".to_owned();
        worker.next = "lancer le test".to_owned();
        let mut brain = crate::models::Task::default();
        brain.id = "old-0".to_owned();
        brain.brainstorm = true;
        brain.question = "Décision numéro 0".to_owned();
        brain.answers = Some(vec![BrainstormAnswer {
            attempt: "a0".to_owned(),
            text: "Réponse 0".to_owned(),
            at: String::new(),
        }]);
        brain.response = "Réponse 0".to_owned();
        let mut brain2 = crate::models::Task::default();
        brain2.id = "old-1".to_owned();
        brain2.brainstorm = true;
        brain2.question = "Décision numéro 1".to_owned();
        brain2.answers = Some(vec![BrainstormAnswer {
            attempt: "a1".to_owned(),
            text: "Réponse 1".to_owned(),
            at: String::new(),
        }]);
        brain2.response = "Réponse 1".to_owned();
        work.tasks = Some(vec![worker, brain, brain2]);
        work
    }

    fn request(work: &Work, task: &str, revision: i64, dialogues: Vec<DialogueRef>) -> MemorySummarySourceRequest {
        MemorySummarySourceRequest {
            schema_version: 1,
            work: work.id.clone(),
            task: task.to_owned(),
            expected_revision: revision,
            dialogues,
        }
    }

    fn dialogue(task: &str, attempt: &str, question: &str, answer: &str) -> DialogueRef {
        // Digest binds question + "\n" + answer, exactly like dialogueHits.
        let digest = format!(
            "{:x}",
            Sha256::digest(format!("{}\n{}", question, answer).as_bytes())
        );
        DialogueRef {
            task: task.to_owned(),
            attempt: attempt.to_owned(),
            sha256: digest,
        }
    }

    fn digest(raw: &[u8]) -> String {
        format!("{:x}", Sha256::digest(raw))
    }

    #[test]
    fn summary_source_refuses_schema_revision_names_and_empty_dialogues() {
        // Schema/validation refusals live in the decode head, mirroring Go's
        // Store.memorySummarySource (schema → revision → names → dialogues).
        let wire = |fields: serde_json::Value| serde_json::to_vec(&fields).unwrap();
        let refused = |raw: &[u8]| {
            decode_summary_source_request(raw)
                .err()
                .expect("refused")
                .reason()
        };
        let mut q = serde_json::json!({
            "schema_version":1,
            "work":"w-fixture",
            "task":"t1",
            "expected_revision":7,
            "dialogues":[{"task":"old-0","attempt":"a0","sha256":"abc"}]
        });
        assert!(decode_summary_source_request(&wire(q.clone())).is_ok());
        q["schema_version"] = 2.into();
        assert_eq!(
            refused(&wire(q.clone())),
            "project-summary-source-request-schema-invalid"
        );
        q["schema_version"] = 1.into();
        q["expected_revision"] = 0.into();
        assert_eq!(
            refused(&wire(q.clone())),
            "project-summary-source-request-schema-invalid"
        );
        q["expected_revision"] = 7.into();
        q["work"] = "../escape".into();
        assert_eq!(
            refused(&wire(q.clone())),
            "project-summary-source-request-schema-invalid"
        );
        q["work"] = "w-fixture".into();
        q["task"] = "../escape".into();
        assert_eq!(
            refused(&wire(q.clone())),
            "project-summary-source-request-schema-invalid"
        );
        q["task"] = "t1".into();
        q["dialogues"] = serde_json::json!([]);
        assert_eq!(
            refused(&wire(q)),
            "project-summary-source-request-schema-invalid"
        );
    }

    #[test]
    fn summary_source_refuses_decode_violations_and_transport_overflow() {
        // Unknown field: Go strict decode refuses (decode cause retained).
        let raw = br#"{"schema_version":1,"work":"w","task":"t","expected_revision":1,"dialogues":[{"task":"a","attempt":"b","sha256":"c"}],"automatic":true}"#;
        assert_eq!(
            decode_summary_source_request(raw)
                .err()
                .expect("unknown field refused")
                .reason(),
            "format"
        );
        // Wrong type for expected_revision.
        let raw = br#"{"schema_version":1,"work":"w","task":"t","expected_revision":"1","dialogues":[]}"#;
        assert!(decode_summary_source_request(raw).is_err());
        // Transport ceiling: Go checks the raw length first, at 1 MiB.
        let big = vec![b'a'; crate::project_memory::PROJECT_MEMORY_TRANSPORT_BYTES + 1];
        let mut big_request = br#"{"schema_version":1,"work":"w","task":"t","expected_revision":1,"dialogues":[{"task":"","attempt":"","sha256":""}]}"#.to_vec();
        big_request.extend(big);
        assert_eq!(
            decode_summary_source_request(&big_request)
                .err()
                .expect("transport overflow refused")
                .reason(),
            "project-summary-source-transport-exceeded"
        );
    }

    #[test]
    fn summary_source_refuses_identity_revision_task_and_brainstorm() {
        let work = fixture_work();
        let root = std::path::Path::new("/fixture/root");
        let mut q = request(&work, "t1", 7, vec![dialogue("old-0", "a0", "Décision numéro 0", "Réponse 0")]);
        q.work = "w-other".to_owned();
        assert_eq!(
            summary_source_for_work(root, &work, &q)
                .err()
                .expect("identity mismatch refused")
                .reason(),
            "project-summary-source-work-identity-mismatch"
        );
        q.work = work.id.clone();
        q.expected_revision = 6;
        assert_eq!(
            summary_source_for_work(root, &work, &q)
                .err()
                .expect("stale revision refused")
                .code(),
            "revision-conflict"
        );
        q.expected_revision = 7;
        q.task = "absent".to_owned();
        assert_eq!(
            summary_source_for_work(root, &work, &q)
                .err()
                .expect("unknown task refused")
                .reason(),
            "project-memory-task-unknown"
        );
        q.task = "old-0".to_owned();
        assert_eq!(
            summary_source_for_work(root, &work, &q)
                .err()
                .expect("brainstorm target refused")
                .reason(),
            "project-summary-source-brainstorm-target"
        );
    }

    #[test]
    fn summary_source_refuses_duplicate_missing_and_empty_exchanges() {
        let work = fixture_work();
        let root = std::path::Path::new("/fixture/root");
        let ref0 = dialogue("old-0", "a0", "Décision numéro 0", "Réponse 0");
        let q = request(&work, "t1", 7, vec![ref0.clone(), ref0.clone()]);
        assert_eq!(
            summary_source_for_work(root, &work, &q)
                .err()
                .expect("duplicate refused")
                .reason(),
            "project-summary-source-dialogue-duplicate"
        );
        // A modified sha256 means the exchange is absent or modified.
        let mut altered = ref0.clone();
        altered.sha256 = "0".repeat(64);
        let q = request(&work, "t1", 7, vec![altered]);
        assert_eq!(
            summary_source_for_work(root, &work, &q)
                .err()
                .expect("altered exchange refused")
                .reason(),
            "dialogue-exchange-missing"
        );
        // Empty answer text: the exchange has no stored answer.
        let mut work_empty = fixture_work();
        work_empty
            .tasks
            .as_mut()
            .unwrap()
            .iter_mut()
            .find(|t| t.id == "old-1")
            .unwrap()
            .answers = Some(vec![BrainstormAnswer {
            attempt: "a1".to_owned(),
            text: String::new(),
            at: String::new(),
        }]);
        let ref1_empty = dialogue("old-1", "a1", "Décision numéro 1", "");
        let q = request(&work_empty, "t1", 7, vec![ref1_empty]);
        assert_eq!(
            summary_source_for_work(root, &work_empty, &q)
                .err()
                .expect("empty answer refused")
                .reason(),
            "project-summary-source-exchange-empty"
        );
    }

    #[test]
    fn summary_source_exact_packet_shape_and_no_mutation() {
        let work = fixture_work();
        let before = encode_model(&work).unwrap();
        let root = std::path::Path::new("/fixture/root");
        let ref0 = dialogue("old-0", "a0", "Décision numéro 0", "Réponse 0");
        let ref1 = dialogue("old-1", "a1", "Décision numéro 1", "Réponse 1");
        let packet = summary_source_for_work(root, &work, &request(&work, "t1", 7, vec![ref0, ref1]))
            .expect("packet built");
        assert_eq!(packet.schema_version, 1);
        assert_eq!(packet.kind, "swarm_summary_source_v1");
        assert_eq!(packet.sha256, digest(packet.payload.as_bytes()));
        // Payload structure: scope, revision, project hash, audience, the
        // never-set complete flag, then one snapshot + 2×2 dialogue sources.
        let value: serde_json::Value = serde_json::from_str(&packet.payload).unwrap();
        assert_eq!(value["scope"]["work"], "w-fixture");
        assert_eq!(value["scope"]["task"], "t1");
        assert_eq!(value["work_revision"], 7);
        assert_eq!(value["audience"], "local_operator");
        assert_eq!(value["provider_history_complete"], false);
        let sources = value["sources"].as_array().unwrap();
        assert_eq!(sources.len(), 5);
        assert_eq!(sources[0]["id"], "current-task-snapshot");
        assert_eq!(sources[0]["role"], "user");
        assert_eq!(sources[0]["kind"], "task_snapshot");
        let snapshot_text = sources[0]["text"].as_str().unwrap();
        assert!(snapshot_text.contains("tests PASS"), "snapshot carries the contract JSON");
        let snapshot: serde_json::Value = serde_json::from_str(snapshot_text).unwrap();
        assert_eq!(snapshot["work_title"], "Contrôler les preuves");
        assert_eq!(snapshot["work_criteria"], serde_json::json!(["tests PASS"]));
        assert_eq!(snapshot["task"]["id"], "t1");
        assert_eq!(sources[1]["role"], "user");
        assert_eq!(sources[1]["kind"], "operator_question");
        assert_eq!(sources[1]["text"], "Décision numéro 0");
        assert_eq!(sources[2]["role"], "assistant");
        assert_eq!(sources[2]["kind"], "historical_answer");
        assert_eq!(sources[2]["text"], "Réponse 0");
        for source in &sources[1..] {
            let text = source["text"].as_str().unwrap();
            assert_eq!(source["sha256"], serde_json::Value::String(digest(text.as_bytes())));
            let dialogue = &source["dialogue"];
            let ref_json = encode_model(&serde_json::from_value::<DialogueRef>(dialogue.clone()).unwrap()).unwrap();
            let id = format!("dialogue-{}", digest(&ref_json));
            assert_eq!(source["id"], serde_json::Value::String(format!(
                "{}-{}",
                id,
                if source["kind"] == "operator_question" { "question" } else { "answer" }
            )));
        }
        // The unselected sibling exchange stays out of the payload.
        assert!(!packet.payload.contains("Réponse 5"));
        assert!(!packet.payload.contains("old-5"));
        // No Work mutation: identical encoded bytes.
        assert_eq!(encode_model(&work).unwrap(), before);
    }

    #[test]
    fn summary_source_decode_accepts_valid_request() {
        let raw = br#"{"schema_version":1,"work":"w-fixture","task":"t1","expected_revision":7,"dialogues":[{"task":"old-0","attempt":"a0","sha256":"abc"}]}"#;
        let q = decode_summary_source_request(raw).expect("valid request decodes");
        assert_eq!(q.schema_version, 1);
        assert_eq!(q.work, "w-fixture");
        assert_eq!(q.task, "t1");
        assert_eq!(q.expected_revision, 7);
        assert_eq!(q.dialogues.len(), 1);
        assert_eq!(q.dialogues[0].task, "old-0");
    }
}
