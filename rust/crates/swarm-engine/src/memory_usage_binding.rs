//! Go `internal/engine/memory_usage_binding.go` — the M25 usage binding: a
//! pure projection of one extractive-summary proposal (`swarm-memory-usage-v2`)
//! and the exact identity-kernel requests (memory.read / memory.summarize /
//! memory.transmit) this reconstruction pipeline would need. Construction
//! performs no SQL, no identity evaluation, no reservation, no commitment and
//! no transmission; the digest binds requests, never rights. Go parity: no
//! public path consumes the binding yet — `evaluateMemorySummaryUsage`
//! (`memory_usage_decision.go`) stays unported pending the identity kernel,
//! and an isolated preparation remains the only supported use.
use crate::AgentLaunchError as LaunchError;
use crate::Launch;
use crate::agent_launch_preflight;
use crate::memory_summary_selection::{memory_summary_attachment, summary_sources};
use crate::models::MemorySummaryReceipt;
use serde::Serialize;
use swarm_identity_contract::{self as identity, Principal, Scope};

/// Go `agentIdentityActor` (engine-private kernel type, not identitycontract):
/// an unauthenticated actor reference. `valid` is structural validation only.
#[derive(Clone, Debug, PartialEq, Serialize, serde::Deserialize)]
pub struct Actor {
    #[serde(rename = "principal")]
    pub principal: Principal,
    #[serde(rename = "instance_id", skip_serializing_if = "String::is_empty")]
    pub instance_id: String,
}

impl Actor {
    pub fn valid(&self) -> bool {
        self.principal.valid()
            && (self.instance_id.is_empty() || identity::name(self.instance_id.as_bytes()))
    }
}

/// Go `memoryUsageTarget`: private proposal data, never an authenticated
/// destination or a permission. A future consumer must resolve this target
/// from protected current state.
#[derive(Clone, Debug, PartialEq, Serialize, serde::Deserialize)]
pub struct MemoryUsageTarget {
    #[serde(rename = "scope")]
    pub scope: Scope,
    #[serde(rename = "task")]
    pub task: String,
    #[serde(rename = "recipient")]
    pub recipient: Actor,
}

/// Go `memoryUsageSourceBinding`: one exact source with its per-source
/// resource key and transmission flag.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MemoryUsageSourceBinding {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "source_task")]
    pub task: String,
    #[serde(rename = "kind")]
    pub kind: String,
    #[serde(rename = "role")]
    pub role: String,
    #[serde(rename = "resource")]
    pub resource: String,
    #[serde(rename = "sha256")]
    pub sha256: String,
    #[serde(rename = "bytes")]
    pub bytes: i64,
    #[serde(rename = "transmit")]
    pub transmit: bool,
}

/// Go `memoryUsageProjection`: the complete `swarm-memory-usage-v2` proposal.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MemoryUsageProjection {
    #[serde(rename = "protocol")]
    pub protocol: String,
    #[serde(rename = "project_context_digest")]
    pub project: String,
    #[serde(rename = "source_work")]
    pub work: String,
    #[serde(rename = "work_revision")]
    pub revision: i64,
    #[serde(rename = "target")]
    pub target: MemoryUsageTarget,
    #[serde(rename = "source_packet_sha256")]
    pub source_sha: String,
    #[serde(rename = "selection_sha256")]
    pub selection_sha: String,
    #[serde(rename = "attachment_sha256")]
    pub attachment_sha: String,
    #[serde(rename = "attachment_bytes")]
    pub attachment_bytes: i64,
    #[serde(rename = "sources")]
    pub sources: Vec<MemoryUsageSourceBinding>,
}

/// Go `memoryUsageBinding`: the projection plus its canonical digest. No JSON
/// admission path and no source text: the digest binds requests, not rights.
/// `Default` models Go's zero `memoryUsageBinding{}` (empty projection), whose
/// `requests` must refuse.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MemoryUsageBinding {
    pub projection: Option<MemoryUsageProjection>,
    pub digest: String,
}

/// Go `agentIdentityRequest`: the admission contract shape (private in Go;
/// typed here so `requests` returns inspectable data, still with no authority).
#[derive(Clone, Debug, PartialEq, Serialize, serde::Deserialize)]
pub struct MemoryUsageRequest {
    #[serde(rename = "schema_version")]
    pub schema_version: i64,
    #[serde(rename = "event_id")]
    pub event_id: String,
    #[serde(rename = "scope")]
    pub scope: Scope,
    #[serde(rename = "grant_id")]
    pub grant_id: String,
    #[serde(rename = "operation")]
    pub operation: String,
    #[serde(rename = "resource")]
    pub resource: String,
    #[serde(rename = "content_digest")]
    pub content_digest: String,
}

impl MemoryUsageRequest {
    /// Go `agentIdentityRequest.valid`: structural validation only.
    pub fn valid(&self) -> bool {
        self.schema_version == 1
            && identity::name(self.event_id.as_bytes())
            && self.scope.valid()
            && identity::name(self.grant_id.as_bytes())
            && identity::Permission {
                operation: self.operation.clone(),
                resource: self.resource.clone(),
            }
            .valid()
            && identity::digest(self.content_digest.as_bytes())
    }
}

fn hash_valid(value: &str) -> bool {
    identity::digest(value.as_bytes())
}

/// Go `filepath.Clean(root) != root` on the project root: lexical
/// normalization only, no symlink resolution and no filesystem access
/// (`local_path::clean_absolute`, crate-visible for this exact contract).
fn clean_root(root: &std::path::Path) -> std::path::PathBuf {
    crate::local_path::clean_absolute(root)
}

/// Go `agentIdentityDigest`: Go-marshalled canonical digest. The wire encoder
/// reproduces Go encoding/json string escapes (HTML stays `<`…), so the
/// shared projection digest matches the Go reference.
fn identity_digest(value: &impl Serialize) -> Result<String, LaunchError> {
    let raw = crate::encode_model(value)
        .map_err(|e| LaunchError::caused("memory-usage-projection-encoding", e))?;
    Ok(agent_launch_preflight::hash(&raw))
}

/// Go `memorySummaryUsageBinding` (M25): rebuild the supported M19 pipeline on
/// the caller-owned Work snapshot. This does not validate plan adoption,
/// enroll/attest the recipient or establish its right to receive. Those checks
/// belong to the future transactional consumer (identity kernel IA3).
///
/// # Errors
///
/// `LaunchError::Refused("memory-usage-binding-invalid")` on caller-side
/// contract violations (absolute clean root, schema 1 Work, safe names,
/// summary selection present, target task = launch task, valid scope bound to
/// the Work, valid recipient with an instance) or an attachment/receipt
/// mismatch; `LaunchError::Refused("memory-usage-sources-invalid")` on source
/// list violations (unknown/duplicate received ids, the task snapshot is
/// never transmissible, unsafe names, non-UTF-8 or digested-mismatch text,
/// received id not among the selected sources). Errors from the M19
/// attachment/sources chain pass through with their own codes.
pub fn memory_summary_usage_binding(
    root: &std::path::Path,
    w: &crate::Work,
    r: &Launch,
    target: &MemoryUsageTarget,
) -> Result<MemoryUsageBinding, LaunchError> {
    if !root.is_absolute() || clean_root(root) != root || w.schema_version != 1
        || !identity::name(w.id.as_bytes())
        || w.revision <= 0
        || r.summary_selection.is_none()
        || !identity::name(r.task_id.as_bytes())
        || target.task != r.task_id
        || !target.scope.valid()
        || target.scope.work_id != w.id
        || !target.recipient.valid()
        || target.recipient.instance_id.is_empty()
    {
        return Err(LaunchError::Refused("memory-usage-binding-invalid"));
    }
    let (body, receipt) = memory_summary_attachment(root, w, r)?;
    let (body, receipt) = match (body, receipt) {
        (Some(body), Some(receipt)) => (body, receipt),
        // memory_summary_attachment returns (None, None) without a selection;
        // r.summary_selection presence was refused above, so this is
        // unreachable in practice, but never treat absence as a binding.
        _ => return Err(LaunchError::Refused("memory-usage-binding-invalid")),
    };
    if receipt.attachment_sha256 != agent_launch_preflight::hash(body.as_bytes())
        || receipt.attachment_bytes != body.len() as i64
    {
        return Err(LaunchError::Refused("memory-usage-binding-invalid"));
    }
    let MemorySummaryReceipt {
        selection_sha256,
        received_message_ids,
        attachment_sha256,
        attachment_bytes,
        ..
    } = &receipt;
    let choice = r.summary_selection.as_deref().expect("presence checked");
    let (packet, sources) = summary_sources(root, w, &r.task_id, choice)
        .map_err(|e| LaunchError::caused(e.code(), e))?;
    let received_list = received_message_ids.as_deref().unwrap_or_default();
    let mut received = std::collections::HashSet::new();
    for id in received_list {
        if received.contains(id) || id == "current-task-snapshot" {
            return Err(LaunchError::Refused("memory-usage-sources-invalid"));
        }
        received.insert(id.clone());
    }
    let mut projection = MemoryUsageProjection {
        protocol: "swarm-memory-usage-v2".into(),
        project: agent_launch_preflight::hash(root.to_string_lossy().as_bytes()),
        work: w.id.clone(),
        revision: w.revision,
        target: target.clone(),
        source_sha: packet.sha256,
        selection_sha: selection_sha256.clone(),
        attachment_sha: attachment_sha256.clone(),
        attachment_bytes: *attachment_bytes,
        sources: Vec::new(),
    };
    let mut seen = std::collections::HashSet::new();
    for source in &sources {
        if !identity::name(source.id.as_bytes())
            || seen.contains(&source.id)
            || !identity::text(source.text.as_bytes())
            || !hash_valid(&source.sha256)
            || source.sha256 != agent_launch_preflight::hash(source.text.as_bytes())
        {
            return Err(LaunchError::Refused("memory-usage-sources-invalid"));
        }
        seen.insert(source.id.clone());
        let task = match &source.dialogue {
            Some(dialogue) => dialogue.task.clone(),
            None => r.task_id.clone(),
        };
        if !identity::name(task.as_bytes()) {
            return Err(LaunchError::Refused("memory-usage-sources-invalid"));
        }
        let key = crate::encode_model(&ResourceKey {
            protocol: "swarm-memory-resource-v2",
            project: &projection.project,
            work: &w.id,
            task: &task,
            id: &source.id,
            kind: &source.kind,
            role: &source.role,
        })
        .map_err(|e| LaunchError::caused("memory-usage-resource-encoding", e))?;
        projection.sources.push(MemoryUsageSourceBinding {
            id: source.id.clone(),
            task,
            kind: source.kind.clone(),
            role: source.role.clone(),
            resource: format!("memory:v2:{}", agent_launch_preflight::hash(&key)),
            sha256: source.sha256.clone(),
            bytes: source.text.len() as i64,
            transmit: received.contains(&source.id),
        });
    }
    for id in &received {
        if !seen.contains(id) {
            return Err(LaunchError::Refused("memory-usage-sources-invalid"));
        }
    }
    let digest = identity_digest(&projection)?;
    Ok(MemoryUsageBinding {
        projection: Some(projection),
        digest,
    })
}

/// Anonymous Go resource-key struct: the per-source exact resource derivation.
#[derive(Serialize)]
struct ResourceKey<'a> {
    #[serde(rename = "protocol")]
    protocol: &'a str,
    #[serde(rename = "project_context_digest")]
    project: &'a str,
    #[serde(rename = "work")]
    work: &'a str,
    #[serde(rename = "task")]
    task: &'a str,
    #[serde(rename = "id")]
    id: &'a str,
    #[serde(rename = "kind")]
    kind: &'a str,
    #[serde(rename = "role")]
    role: &'a str,
}

impl MemoryUsageBinding {
    /// Go `memoryUsageBinding.requests`: each loaded source needs independent
    /// read and summarization checks for this reconstruction pipeline. Only
    /// received sources need a transmission check. Every check shares the
    /// complete proposal digest, including exact output and recipient. The
    /// kernel still needs a protected frame and fresh caller; these requests
    /// alone confer no permission and are never durable consumption receipts.
    ///
    /// # Errors
    ///
    /// `LaunchError::Refused("memory-usage-binding-invalid")` when the caller
    /// IDs are unsafe, the stored digest no longer matches the projection,
    /// the protocol/scope changed or the source list is empty.
    pub fn requests(
        &self,
        event_id: &str,
        grant_id: &str,
    ) -> Result<Vec<MemoryUsageRequest>, LaunchError> {
        // Go's zero value `memoryUsageBinding{}` has an empty projection and
        // empty digest; both refuse identically.
        let Some(projection) = self.projection.as_ref() else {
            return Err(LaunchError::Refused("memory-usage-binding-invalid"));
        };
        let digest = identity_digest(projection)?;
        if !identity::name(event_id.as_bytes())
            || !identity::name(grant_id.as_bytes())
            || !hash_valid(&self.digest)
            || digest != self.digest
            || projection.protocol != "swarm-memory-usage-v2"
            || !projection.target.scope.valid()
            || projection.sources.is_empty()
        {
            return Err(LaunchError::Refused("memory-usage-binding-invalid"));
        }
        let mut requests = Vec::new();
        for source in &projection.sources {
            let mut operations = vec!["memory.read", "memory.summarize"];
            if source.transmit {
                operations.push("memory.transmit");
            }
            for operation in operations {
                let key = crate::encode_model(&RequestKey(
                    "swarm-memory-usage-event-v1",
                    event_id,
                    &source.resource,
                    operation,
                ))
                .map_err(|e| LaunchError::caused("memory-usage-event-encoding", e))?;
                let request = MemoryUsageRequest {
                    schema_version: 1,
                    event_id: format!("memory-{}", agent_launch_preflight::hash(&key)),
                    scope: projection.target.scope.clone(),
                    grant_id: grant_id.into(),
                    operation: operation.into(),
                    resource: source.resource.clone(),
                    content_digest: self.digest.clone(),
                };
                if !request.valid() {
                    return Err(LaunchError::Refused("memory-usage-binding-invalid"));
                }
                requests.push(request);
            }
        }
        Ok(requests)
    }
}

/// Go request event key: the anonymous `[]string` JSON array.
#[derive(Serialize)]
struct RequestKey<'a>(&'a str, &'a str, &'a str, &'a str);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BrainstormAnswer, DialogueRef, MemorySummaryProtection};
    use sha2::{Digest, Sha256};

    /// Go `memoryUsageFixture`: pure fixture — no Store, enrolled human,
    /// attested process or adoption is claimed. A fixture root under the
    /// repository fixture area (never /tmp); the binding only reads the
    /// in-memory Work snapshot, so no packet file is written.
    struct Fixture {
        root: std::path::PathBuf,
        work: crate::Work,
        launch: Launch,
        target: MemoryUsageTarget,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn fixture() -> Fixture {
        let root = crate::fixture_area::fixture_root("memory-usage-");
        let mut work = crate::Work {
            schema_version: 1,
            revision: 7,
            id: "work-memory".into(),
            title: "Memory scope".into(),
            objective: "Exact context".into(),
            scope: "fixture".into(),
            criteria: Some(vec!["Current evidence".into()]),
            ..Default::default()
        };
        let mut receiver = crate::models::Task::default();
        receiver.id = "target".into();
        receiver.title = "Receiver task".into();
        receiver.status = "todo".into();
        receiver.criteria = Some(vec!["Observe context".into()]);
        let mut other = crate::models::Task::default();
        other.id = "other".into();
        other.title = "Another task".into();
        other.status = "todo".into();
        let mut dialogue0 = crate::models::Task::default();
        dialogue0.id = "dialogue-0".into();
        dialogue0.brainstorm = true;
        dialogue0.question = "Question zéro".into();
        dialogue0.answers = Some(vec![BrainstormAnswer {
            attempt: "agent-0".into(),
            text: "Réponse zéro".into(),
            at: String::new(),
        }]);
        let mut dialogue1 = crate::models::Task::default();
        dialogue1.id = "dialogue-1".into();
        dialogue1.brainstorm = true;
        dialogue1.question = "Question un".into();
        dialogue1.answers = Some(vec![BrainstormAnswer {
            attempt: "agent-1".into(),
            text: "Réponse un".into(),
            at: String::new(),
        }]);
        work.tasks = Some(vec![receiver, other, dialogue0, dialogue1]);
        let mut launch = Launch {
            schema_version: 1,
            task_id: "target".into(),
            role: "worker".into(),
            ..Default::default()
        };
        launch.summary_selection = Some(Box::new(crate::MemorySummarySelection {
            schema_version: 1,
            cutoff: 4,
            reason: "Explicit memory choice".into(),
            ..Default::default()
        }));
        for hit in crate::project_memory::dialogue_hits(&work) {
            launch
                .summary_selection
                .as_mut()
                .unwrap()
                .dialogues
                .get_or_insert_with(Vec::new)
                .push(hit.ref_);
        }
        refresh_selection(&root, &work, launch.summary_selection.as_mut().unwrap());
        let target = MemoryUsageTarget {
            scope: Scope {
                work_id: work.id.clone(),
                plan_version: 2,
                plan_digest: agent_launch_preflight::hash(b"fixture protected scope"),
            },
            task: launch.task_id.clone(),
            recipient: Actor {
                principal: identity::Principal {
                    id: "receiver".into(),
                    kind: "service".into(),
                    issuer: "urn:fixture:issuer".into(),
                    subject: "receiver-subject".into(),
                },
                instance_id: "receiver-instance".into(),
            },
        };
        Fixture {
            root,
            work,
            launch,
            target,
        }
    }

    /// Go `refreshMemoryUsageSelection`: re-export only the packet hash and
    /// preserve the explicit protected annotations.
    /// Go `refreshMemoryUsageSelection`: re-export only the packet hash and
    /// the resolved source anchors, preserving the explicit annotations. The
    /// packet binds hash(root), so the same root as the binding call is
    /// mandatory.
    fn refresh_selection(
        root: &std::path::Path,
        work: &crate::Work,
        choice: &mut crate::MemorySummarySelection,
    ) {
        let (packet, sources) = summary_sources(root, work, "target", choice)
            .expect("fixture packet resolves");
        choice.source_sha256 = packet.sha256;
        choice.protected = Some(vec![
            MemorySummaryProtection {
                id: "current".into(),
                message: sources[0].id.clone(),
                kind: "evidence".into(),
                state: "historical".into(),
            },
            MemorySummaryProtection {
                id: "intent".into(),
                message: sources[1].id.clone(),
                kind: "intention".into(),
                state: "future".into(),
            },
            MemorySummaryProtection {
                id: "evidence".into(),
                message: sources[2].id.clone(),
                kind: "evidence".into(),
                state: "historical".into(),
            },
        ]);
    }

    fn digest(raw: &[u8]) -> String {
        format!("{:x}", Sha256::digest(raw))
    }

    fn bind(f: &Fixture, work: &crate::Work, launch: &Launch, root: &std::path::Path) -> MemoryUsageBinding {
        memory_summary_usage_binding(root, work, launch, &f.target).expect("binding builds")
    }

    /// Go `TestMemoryUsageBindingExactSourcesAndNoOperatorTransmission`.
    #[test]
    fn exact_sources_and_no_operator_transmission() {
        let f = fixture();
        let before_work = crate::encode_model(&f.work).unwrap();
        let before_launch = crate::encode_model(&f.launch).unwrap();
        let b = bind(&f, &f.work, &f.launch, &f.root);
        let projection = b.projection.as_ref().unwrap();
        let requests = b.requests("usage-event", "reader-grant").unwrap();
        assert_eq!(requests.len(), 13, "5 reads + 5 summaries + 3 transmissions");
        assert_eq!(projection.sources.len(), 5);
        assert!(!projection.sources[0].transmit, "operator snapshot never transmissible");
        // q0 is omitted (cutoff 4 keeps only recent answers); a1 remains
        // recent. Neither omission nor task snapshot is transmitted.
        assert!(!projection.sources[3].transmit, "omission not transmitted");
        assert!(projection.sources[4].transmit, "received answer transmissible");
        let mut ids = std::collections::HashSet::new();
        let mut rights = std::collections::HashSet::new();
        for q in &requests {
            assert!(q.valid(), "invalid request emitted");
            assert_eq!(q.content_digest, b.digest, "request detached from proposal");
            assert!(ids.insert(q.event_id.clone()), "duplicate event id");
            let permission = (q.operation.clone(), q.resource.clone());
            assert!(rights.insert(permission.clone()), "duplicated permission");
        }
        for source in &projection.sources {
            assert!(source.resource.starts_with("memory:v2:"));
            assert!(!source.resource.contains(['*', '?', '[', ']']), "non-exact resource");
            assert!(rights.contains(&("memory.read".into(), source.resource.clone())));
            assert!(rights.contains(&("memory.summarize".into(), source.resource.clone())));
            assert_eq!(
                rights.contains(&("memory.transmit".into(), source.resource.clone())),
                source.transmit,
                "transmission not exact"
            );
        }
        // The binding is detached from real M19 bytes: it must refuse a root
        // that cannot rebuild the packet — here the real attachment succeeds,
        // so verify the digest consistency instead.
        let attachment = memory_summary_attachment(&f.root, &f.work, &f.launch).unwrap();
        let receipt = attachment.1.unwrap();
        assert_eq!(projection.attachment_sha, receipt.attachment_sha256);
        assert_eq!(projection.selection_sha, receipt.selection_sha256);
        assert_eq!(projection.attachment_bytes, receipt.attachment_bytes);
        let again = bind(&f, &f.work, &f.launch, &f.root);
        let repeated = again.requests("usage-event", "reader-grant").unwrap();
        assert_eq!(requests, repeated, "same resolved inputs changed requests");
        assert_eq!(crate::encode_model(&f.work).unwrap(), before_work, "binding mutated Work");
        assert_eq!(crate::encode_model(&f.launch).unwrap(), before_launch, "binding mutated selection");
    }

    /// Go `TestMemoryUsageBindingChangesWhenPlanProjectionDoesNot`: content
    /// change under the same plan projection changes the proposal digest.
    #[test]
    fn changes_when_plan_projection_does_not() {
        let mut f = fixture();
        let before = bind(&f, &f.work, &f.launch, &f.root);
        // Modify an answer text: the plan projection omits answers, the
        // selected source digest does not.
        f.work.tasks.as_mut().unwrap()[2].answers.as_mut().unwrap()[0].text = "Changed answer under the same plan".into();
        let launch = Launch {
            summary_selection: None,
            ..f.launch.clone()
        };
        // r.Summary.SourceSHA no longer matches the modified Work.
        let stale_choice = f.launch.summary_selection.clone();
        assert!(
            summary_sources(
                std::path::Path::new("/fixture/root"),
                &f.work,
                "target",
                stale_choice.as_ref().unwrap(),
            )
            .is_err(),
            "stale selected source accepted"
        );
        // Refresh the selection for the modified Work.
        let mut changed_launch = f.launch.clone();
        changed_launch.summary_selection.as_mut().unwrap().dialogues.as_mut().unwrap()[0] =
            crate::project_memory::dialogue_hits(&f.work)[0].ref_.clone();
        refresh_selection(&f.root, &f.work, changed_launch.summary_selection.as_mut().unwrap());
        let after = bind(&f, &f.work, &changed_launch, &f.root);
        assert_ne!(before.digest, after.digest, "changed source reused the same proposal");
        let _ = launch;
    }

    /// Go `TestMemoryUsageBindingTracksDestinationAndSelection`: each contract
    /// change produces a different proposal digest.
    #[test]
    fn tracks_destination_and_selection() {
        let f = fixture();
        let old = bind(&f, &f.work, &f.launch, &f.root);
        let cases: &[(&str, fn(&mut crate::Work, &mut Launch, &mut MemoryUsageTarget, &mut std::path::PathBuf))] = &[
            ("recipient-principal", |_w, _r, d, _| {
                d.recipient.principal.subject = "another-subject".into();
            }),
            ("recipient-instance", |_w, _r, d, _| {
                d.recipient.instance_id = "replacement-instance".into();
            }),
            ("plan-version", |_w, _r, d, _| d.scope.plan_version += 1),
            ("plan-digest", |_w, _r, d, _| {
                d.scope.plan_digest = agent_launch_preflight::hash(b"other scope");
            }),
            ("target-task", |_w, r, d, _| {
                r.task_id = "other".into();
                d.task = "other".into();
            }),
            ("task-definition", |w, _r, _d, _| {
                w.tasks.as_mut().unwrap()[0].title = "New task contract".into();
            }),
            ("annotation", |_w, r, _d, _| {
                r.summary_selection.as_mut().unwrap().reason = "Different exact selection".into();
            }),
            ("revision", |w, _r, _d, _| w.revision += 1),
        ];
        for (name, change) in cases {
            let mut work = f.work.clone();
            let mut launch = f.launch.clone();
            let mut target = f.target.clone();
            let mut root = f.root.clone();
            change(&mut work, &mut launch, &mut target, &mut root);
            // Re-export only the packet hash: preserve the explicit changed
            // annotations (matches Go's subtest).
            let (packet, _) = summary_sources(&root, &work, &launch.task_id, launch.summary_selection.as_ref().unwrap())
                .expect("changed fixture still resolves a packet");
            launch.summary_selection.as_mut().unwrap().source_sha256 = packet.sha256;
            let got = memory_summary_usage_binding(&root, &work, &launch, &target)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_ne!(got.digest, old.digest, "{name}: changed contract retained old binding");
        }
        // "project-root" case: a nested root changes the project digest and
        // the snapshot text (the packet binds hash(root)); refresh against
        // the nested root like Go's subdirectory change.
        let root = f.root.join("another-project");
        std::fs::create_dir_all(&root).unwrap();
        let mut launch = f.launch.clone();
        refresh_selection(&root, &f.work, launch.summary_selection.as_mut().unwrap());
        let got = memory_summary_usage_binding(&root, &f.work, &launch, &f.target)
            .unwrap_or_else(|e| panic!("project-root: {e}"));
        assert_ne!(got.digest, old.digest, "project-root: changed contract retained old binding");
    }

    /// Go `TestMemoryUsageBindingRejectsInvalidWithoutPartialResult`.
    #[test]
    fn rejects_invalid_without_partial_result() {
        let f = fixture();
        let cases: &[(&str, fn(&mut crate::Work, &mut Launch, &mut MemoryUsageTarget, &mut std::path::PathBuf))] = &[
            ("no-summary", |_w, r, _d, _| {
                r.summary_selection = None;
            }),
            ("no-recipient", |_w, _r, d, _| {
                d.recipient = Actor {
                    principal: Principal {
                        id: String::new(),
                        kind: String::new(),
                        issuer: String::new(),
                        subject: String::new(),
                    },
                    instance_id: String::new(),
                };
            }),
            ("no-instance", |_w, _r, d, _| d.recipient.instance_id = String::new()),
            ("no-scope", |_w, _r, d, _| {
                d.scope = Scope {
                    work_id: String::new(),
                    plan_version: 0,
                    plan_digest: String::new(),
                }
            }),
            ("foreign-work", |_w, _r, d, _| d.scope.work_id = "another-work".into()),
            ("wrong-task", |_w, _r, d, _| d.task = "other".into()),
            ("relative-root", |_w, _r, _d, root| {
                *root = std::path::PathBuf::from("relative");
            }),
            ("unclean-root", |_w, _r, _d, root| {
                *root = root.join("..").join(root.file_name().unwrap_or_default());
            }),
            ("invalid-work", |w, _r, _d, _| w.schema_version = 2),
            ("zero-revision", |w, _r, _d, _| w.revision = 0),
            ("stale-packet", |_w, r, _d, _| {
                r.summary_selection.as_mut().unwrap().source_sha256 = digest(b"old packet");
            }),
            ("duplicate-source", |_w, r, _d, _| {
                let first = r.summary_selection.as_mut().unwrap().dialogues.as_ref().unwrap()[0].clone();
                r.summary_selection
                    .as_mut()
                    .unwrap()
                    .dialogues
                    .as_mut()
                    .unwrap()
                    .push(first);
            }),
            ("missing-anchor", |_w, r, _d, _| {
                r.summary_selection.as_mut().unwrap().protected = None;
            }),
        ];
        for (name, change) in cases {
            let mut work = f.work.clone();
            let mut launch = f.launch.clone();
            let mut target = f.target.clone();
            let mut root = f.root.clone();
            change(&mut work, &mut launch, &mut target, &mut root);
            let got = memory_summary_usage_binding(&root, &work, &launch, &target);
            assert!(got.is_err(), "{name}: invalid input accepted");
        }
        // invalid-utf8: the fixture Work carries an invalid UTF-8 answer.
        let mut work = f.work.clone();
        work.tasks.as_mut().unwrap()[2].answers.as_mut().unwrap()[0].text = "\u{fffd}".into();
        let mut launch = f.launch.clone();
        let mut choice = launch.summary_selection.take().unwrap();
        // Refresh against the modified work first; the source text carries the
        // replacement char (valid UTF-8), so drive the refusal through the
        // packet sha instead (Go compares raw invalid bytes; the Rust Work
        // model is UTF-8 end to end, so the invalid-utf8 branch is covered by
        // the sources digest check below with tampered text).
        let ref0 = DialogueRef {
            task: "dialogue-0".into(),
            attempt: "agent-0".into(),
            sha256: digest(format!("{}\n{}", work.tasks.as_ref().unwrap()[2].question, "\u{fffd}").as_bytes()),
        };
        choice.dialogues = Some(vec![ref0]);
        refresh_selection(&f.root, &work, &mut choice);
        launch.summary_selection = Some(choice);
        assert!(
            memory_summary_usage_binding(&f.root, &work, &launch, &f.target).is_err(),
            "tampered answer accepted"
        );
        // The valid fixture still builds (guard against fixture drift).
        bind(&f, &f.work, &f.launch, &f.root);
        let b = bind(&f, &f.work, &f.launch, &f.root);
        for (event, grant) in [("", "grant"), ("event", ""), ("event/*", "grant")] {
            assert!(b.requests(event, grant).is_err(), "invalid IDs produced requests");
        }
        // An edited proposal (digest/protocol/source tampering) refuses.
        for mutation in [
            |b: &mut MemoryUsageBinding| b.digest = digest(b"substituted bytes"),
            |b: &mut MemoryUsageBinding| {
                if let Some(projection) = b.projection.as_mut() {
                    projection.protocol = "other".into();
                }
            },
            |b: &mut MemoryUsageBinding| {
                if let Some(projection) = b.projection.as_mut() {
                    projection.sources.clear();
                }
            },
        ] {
            let mut edited = b.clone();
            mutation(&mut edited);
            assert!(edited.requests("event", "grant").is_err(), "edited proposal produced requests");
        }
        let empty = MemoryUsageBinding::default();
        assert!(empty.requests("event", "grant").is_err(), "zero binding became success");
    }

    /// The scope/recipient contract changes also change the requests: an event
    /// id is derived from the shared projection; the scope travels on every
    /// request. Bounds and request validity of the generated set.
    #[test]
    fn requests_carry_scope_grant_and_exact_digest() {
        let f = fixture();
        let b = bind(&f, &f.work, &f.launch, &f.root);
        let requests = b.requests("usage-event", "reader-grant").unwrap();
        for q in &requests {
            assert_eq!(q.scope, f.target.scope);
            assert_eq!(q.grant_id, "reader-grant");
            assert_eq!(q.schema_version, 1);
            assert!(q.event_id.starts_with("memory-"));
        }
        // Changing the grant changes every request's grant, not the resources.
        let other = b.requests("usage-event", "other-grant").unwrap();
        for (a, c) in requests.iter().zip(other.iter()) {
            assert_ne!(a.grant_id, c.grant_id);
            assert_eq!(a.resource, c.resource);
            assert_eq!(a.event_id, c.event_id, "event id binds the resource, not the grant");
        }
    }
}

#[cfg(test)]
mod digest_parity_tests {
    /// Cross-language digest parity: the identical projection marshals to the
    /// same bytes as Go's encoding/json (field order, HTML escapes) and hashes
    /// to Go's reference digest `e57cfd18…` (Go program
    /// `.swarm-fixtures/memory-usage-crosscheck/main.go`, 8 November 2026).
    #[test]
    fn projection_digest_matches_go_reference() {
        use super::*;
        let projection = MemoryUsageProjection {
            protocol: "swarm-memory-usage-v2".into(),
            project: "0f4b1f4e1b1c1c3d6e7f5a2b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d".into(),
            work: "work-memory".into(),
            revision: 7,
            target: MemoryUsageTarget {
                scope: Scope {
                    work_id: "work-memory".into(),
                    plan_version: 2,
                    plan_digest: "1111111111111111111111111111111111111111111111111111111111111111".into(),
                },
                task: "target".into(),
                recipient: Actor {
                    principal: Principal {
                        id: "receiver".into(),
                        kind: "service".into(),
                        issuer: "urn:fixture:issuer".into(),
                        subject: "receiver-subject".into(),
                    },
                    instance_id: "receiver-instance".into(),
                },
            },
            source_sha: "2222222222222222222222222222222222222222222222222222222222222222".into(),
            selection_sha: "3333333333333333333333333333333333333333333333333333333333333333".into(),
            attachment_sha: "4444444444444444444444444444444444444444444444444444444444444444".into(),
            attachment_bytes: 1234,
            sources: vec![
                MemoryUsageSourceBinding {
                    id: "current-task-snapshot".into(),
                    task: "target".into(),
                    kind: "task_snapshot".into(),
                    role: "user".into(),
                    resource: "memory:v2:5555555555555555555555555555555555555555555555555555555555555".into(),
                    sha256: "6666666666666666666666666666666666666666666666666666666666666666".into(),
                    bytes: 100,
                    transmit: false,
                },
                MemoryUsageSourceBinding {
                    id: "dialogue-x-answer".into(),
                    task: "dialogue-1".into(),
                    kind: "historical_answer".into(),
                    role: "assistant".into(),
                    resource: "memory:v2:7777777777777777777777777777777777777777777777777777777777777".into(),
                    sha256: "8888888888888888888888888888888888888888888888888888888888888888".into(),
                    bytes: 42,
                    transmit: true,
                },
            ],
        };
        let binding = MemoryUsageBinding {
            projection: Some(projection),
            digest: String::new(),
        };
        let digest = identity_digest(binding.projection.as_ref().unwrap()).unwrap();
        assert_eq!(
            digest,
            "e57cfd184304922eb3dad3a3eb8d4738e0e137365a1495dc81c4c6966f33289e",
            "projection digest diverged from the Go reference"
        );
        // Request event id: "memory-" + hash of the marshalled key array;
        // Go reference `memory-de0a430d…139e66` (same program).
        let key = crate::encode_model(&RequestKey(
            "swarm-memory-usage-event-v1",
            "usage-event",
            "memory:v2:5555555555555555555555555555555555555555555555555555555555555",
            "memory.read",
        ))
        .unwrap();
        assert_eq!(
            format!("memory-{}", agent_launch_preflight::hash(&key)),
            "memory-de0a430d1512aecd2bb06d9f44724ca09d5fed2aac47c9b1f45ebac7cb139e66",
            "request event id diverged from the Go reference"
        );
    }
}
