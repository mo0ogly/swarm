//! Go `memory_usage_decision.go`: the pure M25+IA2.2 usage verdict.
//!
//! Chains the pure identity kernel ([`crate::agent_identity_kernel`]) onto the
//! M25 proposal binding ([`crate::memory_usage_binding`]): the requester needs
//! its read/summarize/transmit requests evaluated; the recipient needs one
//! independent read per transmitted source, under the receive event domain
//! `swarm-memory-receive-event-v1`. Aggregate expiry is the earliest decision
//! expiry. Reuse compares the complete previous proof against a fresh
//! evaluation — no partial verdict ever escapes.
//!
//! No public consumer exists yet: like the binding and the kernel, this is a
//! pure construction awaiting the protected transactional consumer (IA3
//! raccord). The frame is supplied by the caller, never reconstructed from
//! snapshot data.

use crate::agent_identity_kernel::{
    evaluate_agent_identity, Actor, Authority, Caller, Decision, GoTime, Request, Snapshot,
};
use crate::agent_launch_error::AgentLaunchError as LaunchError;
use serde::Serialize;
use swarm_identity_contract::Scope;
use crate::memory_usage_binding::{
    memory_summary_usage_binding, MemoryUsageRequest, MemoryUsageTarget,
};

/// Bridge the M25 binding's local twin of the engine-private actor onto the
/// kernel type. Same JSON shape (the binding's struct predates the kernel
/// port and keeps its own definition for M25 consumers).
fn kernel_actor(actor: crate::memory_usage_binding::Actor) -> Actor {
    Actor {
        principal: actor.principal,
        instance_id: actor.instance_id,
    }
}

/// Same bridge for the binding's request twin.
fn kernel_request(request: &MemoryUsageRequest) -> Request {
    Request {
        schema: request.schema_version,
        event_id: request.event_id.clone(),
        scope: Scope {
            work_id: request.scope.work_id.clone(),
            plan_version: request.scope.plan_version,
            plan_digest: request.scope.plan_digest.clone(),
        },
        grant_id: request.grant_id.clone(),
        operation: request.operation.clone(),
        resource: request.resource.clone(),
        content_digest: request.content_digest.clone(),
    }
}

/// Go `memoryUsageFrame`: private inputs from a future protected consumer, not
/// an enrollment or an attestation constructed by this evaluator. No public
/// JSON admission exists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryUsageFrame {
    pub snapshot: Snapshot,
    pub authority: Authority,
    pub requester: Caller,
    pub recipient: Caller,
    pub at: GoTime,
}

/// Go `memoryUsageDecisionIDs`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryUsageDecisionIds {
    pub event_id: String,
    pub requester_grant: String,
    pub recipient_grant: String,
}

/// Go `memoryUsageDecision`: a complete pure verdict, never a consumption
/// receipt or permission to release bytes after its authority/source frame has
/// changed. No partial verdict escapes.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct MemoryUsageDecision {
    pub binding_digest: String,
    pub requester: Vec<Decision>,
    pub recipient: Vec<Decision>,
    /// Go's zero instant while no decision has been recorded (the struct is
    /// never returned in that state — refusals return no decision at all).
    pub expires_at: Option<GoTime>,
}

fn refusal(code: &'static str) -> LaunchError {
    LaunchError::Refused(code)
}

/// Go `evaluateMemorySummaryUsage`.
///
/// # Errors
///
/// Binding and kernel refusals pass through with their codes
/// (`memory-usage-binding-invalid`, `memory-usage-sources-invalid`,
/// `agent-identity-*`). This evaluator adds `memory-usage-project-mismatch`
/// (authority root differs from the project root),
/// `memory-usage-recipient-mismatch` (frame recipient ≠ proposal target) and
/// `memory-usage-decision-stale` (a previous proof no longer matches the fresh
/// complete verdict). On any refusal no partial decision is returned.
pub fn evaluate_memory_summary_usage(
    root: &std::path::Path,
    w: &crate::Work,
    r: &crate::Launch,
    target: &MemoryUsageTarget,
    ids: &MemoryUsageDecisionIds,
    frame: &MemoryUsageFrame,
    previous: Option<&MemoryUsageDecision>,
) -> Result<MemoryUsageDecision, LaunchError> {
    let binding = memory_summary_usage_binding(root, w, r, target)?;
    if frame.authority.project_root != root.to_string_lossy() {
        return Err(refusal("memory-usage-project-mismatch"));
    }
    if frame.recipient.actor != kernel_actor(target.recipient.clone()) {
        return Err(refusal("memory-usage-recipient-mismatch"));
    }
    let requester = binding.requests(&ids.event_id, &ids.requester_grant)?;
    if !swarm_identity_contract::name(ids.recipient_grant.as_bytes()) {
        return Err(refusal("memory-usage-binding-invalid"));
    }
    let projection = binding
        .projection
        .as_ref()
        .ok_or_else(|| refusal("memory-usage-binding-invalid"))?;
    let mut recipient_requests: Vec<Request> = Vec::new();
    for source in &projection.sources {
        if !source.transmit {
            continue;
        }
        let key = crate::encode_model(&ReceiveKey(
            "swarm-memory-receive-event-v1",
            &ids.event_id,
            &source.resource,
        ))
        .map_err(|e| LaunchError::caused("memory-usage-event-encoding", e))?;
        let request = Request {
            schema: 1,
            event_id: format!("memory-receive-{}", crate::agent_launch_preflight::hash(&key)),
            scope: target.scope.clone(),
            grant_id: ids.recipient_grant.clone(),
            operation: "memory.read".to_string(),
            resource: source.resource.clone(),
            content_digest: binding.digest.clone(),
        };
        if !request.valid() {
            return Err(refusal("memory-usage-binding-invalid"));
        }
        recipient_requests.push(request);
    }
    if recipient_requests.is_empty() {
        return Err(refusal("memory-usage-sources-invalid"));
    }
    let mut current = MemoryUsageDecision {
        binding_digest: binding.digest.clone(),
        requester: Vec::new(),
        recipient: Vec::new(),
        expires_at: None,
    };
    let mut append_decision = |caller: &Caller,
                               request: &Request,
                               out: &mut Vec<Decision>|
     -> Result<(), LaunchError> {
        let decision = evaluate_agent_identity(
            &frame.snapshot,
            &frame.authority,
            caller,
            request,
            frame.at,
        )?;
        if current
            .expires_at
            .map_or(true, |current_at| decision.expires_at.map_or(false, |d| d.0 < current_at.0))
        {
            current.expires_at = decision.expires_at;
        }
        out.push(decision);
        Ok(())
    };
    for request in &requester {
        append_decision(&frame.requester, &kernel_request(request), &mut current.requester)?;
    }
    for request in &recipient_requests {
        append_decision(&frame.recipient, request, &mut current.recipient)?;
    }
    // Authenticate and reevaluate both actors before comparing the previous
    // proof. This preserves the kernel's refusal causes on
    // revocation/expiry/denial.
    if let Some(previous) = previous {
        if *previous != current {
            return Err(refusal("memory-usage-decision-stale"));
        }
    }
    Ok(current)
}

/// Go request event key: the anonymous `[]string` JSON array.
#[derive(Serialize)]
struct ReceiveKey<'a>(&'a str, &'a str, &'a str);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_identity_kernel::{
        identity_digest, validate_agent_identity_transition, Grant, Instance,
    };
    use crate::models::{BrainstormAnswer, MemorySummaryProtection};
    use std::collections::BTreeMap;

    /// Go `memoryUsageFixture` (pure): a fixture root under the repository
    /// fixture area, never /tmp; the binding only reads the in-memory Work
    /// snapshot, so no packet file is written.
    struct Fixture {
        root: std::path::PathBuf,
        work: crate::Work,
        launch: crate::Launch,
        target: MemoryUsageTarget,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn go_time(raw: &str) -> GoTime {
        GoTime(
            time::OffsetDateTime::parse(
                raw,
                &time::format_description::well_known::Rfc3339,
            )
            .expect("fixture time is representable"),
        )
    }

    fn fixture_clock() -> GoTime {
        go_time("2026-10-17T00:00:00Z")
    }

    fn fixture() -> Fixture {
        let root = crate::fixture_area::fixture_root("memory-usage-decision-");
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
        let mut launch = crate::Launch {
            summary_selection: Some(Box::new(crate::MemorySummarySelection {
                schema_version: 1,
                cutoff: 4,
                reason: "Explicit memory choice".into(),
                ..Default::default()
            })),
            ..Default::default()
        };
        launch.task_id = "target".into();
        launch.role = "worker".into();
        for hit in crate::project_memory::dialogue_hits(&work) {
            launch
                .summary_selection
                .as_mut()
                .unwrap()
                .dialogues
                .get_or_insert_with(Vec::new)
                .push(hit.ref_);
        }
        refresh_selection(&root, &work, "target", launch.summary_selection.as_mut().unwrap());
        let target = MemoryUsageTarget {
            scope: swarm_identity_contract::Scope {
                work_id: work.id.clone(),
                plan_version: 2,
                plan_digest: crate::agent_launch_preflight::hash(b"fixture protected scope"),
            },
            task: launch.task_id.clone(),
            recipient: crate::memory_usage_binding::Actor {
                principal: swarm_identity_contract::Principal {
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
    /// the resolved source anchors, preserving the explicit annotations. The
    /// task parameter follows the launch under test (Go passes `r.TaskID`).
    fn refresh_selection(
        root: &std::path::Path,
        work: &crate::Work,
        task: &str,
        choice: &mut crate::MemorySummarySelection,
    ) {
        let (packet, sources) =
            crate::memory_summary_selection::summary_sources(root, work, task, choice)
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

    struct DecisionFixture {
        root: std::path::PathBuf,
        work: crate::Work,
        launch: crate::Launch,
        target: MemoryUsageTarget,
        ids: MemoryUsageDecisionIds,
        frame: MemoryUsageFrame,
    }

    /// Go `memoryUsageAuthority` + `memoryDecisionSetup`: the kernel fixture
    /// with a requester service grant and a recipient grant scoped to the
    /// transmitted sources. Pure — no Trust, no attested process.
    fn decision_setup() -> DecisionFixture {
        let f = fixture();
        let binding = memory_summary_usage_binding(&f.root, &f.work, &f.launch, &f.target)
            .expect("fixture binding");
        let requests = binding
            .requests("decision-event", "reader-grant")
            .expect("fixture requests");
        let at = fixture_clock();
        let requester_actor = Actor {
            principal: swarm_identity_contract::Principal {
                id: "requester".into(),
                kind: "service".into(),
                issuer: "urn:fixture:issuer".into(),
                subject: "requester".into(),
            },
            instance_id: String::new(),
        };
        let mut rights: Vec<swarm_identity_contract::Permission> = Vec::new();
        for q in &requests {
            rights.push(swarm_identity_contract::Permission {
                operation: q.operation.clone(),
                resource: q.resource.clone(),
            });
        }
        let reader_grant = Grant {
            id: "reader-grant".into(),
            parent_id: String::new(),
            issuer: requester_actor.clone(),
            holder: requester_actor.clone(),
            scope: binding.projection.as_ref().unwrap().target.scope.clone(),
            trust_revision: 3,
            permissions: rights,
            delegable: Vec::new(),
            not_before: Some(GoTime(at.0 - time::Duration::minutes(1))),
            expires_at: Some(GoTime(at.0 + time::Duration::minutes(1))),
            revoked_at: None,
        };
        let mut grants = BTreeMap::new();
        grants.insert("reader-grant".to_string(), reader_grant);
        let snapshot = Snapshot {
            schema: 1,
            revision: 1,
            grants,
            instances: BTreeMap::new(),
        };
        let mut authority = Authority {
            project_root: f.root.to_string_lossy().into_owned(),
            snapshot_digest: identity_digest(&snapshot).expect("fixture digest"),
            snapshot_revision: 1,
            trust_revision: 3,
            scope: binding.projection.as_ref().unwrap().target.scope.clone(),
            roots: ["reader-grant".to_string()].into_iter().collect(),
        };
        let requester = Caller {
            project_root: authority.project_root.clone(),
            actor: requester_actor,
            trust_revision: 3,
            expires_at: None,
            binding_id: String::new(),
            verified_at: at,
            session_digest: String::new(),
        };
        // Recipient grant: one read right per transmitted source, expiring
        // before the requester grant so the aggregate tracks it.
        let mut recipient_rights: Vec<swarm_identity_contract::Permission> = Vec::new();
        for source in &binding.projection.as_ref().unwrap().sources {
            if source.transmit {
                recipient_rights.push(swarm_identity_contract::Permission {
                    operation: "memory.read".into(),
                    resource: source.resource.clone(),
                });
            }
        }
        let recipient_actor = kernel_actor(f.target.recipient.clone());
        let recipient_grant = Grant {
            id: "recipient-grant".into(),
            parent_id: String::new(),
            issuer: f
                .frame_recipient_issuer(&requester),
            holder: recipient_actor.clone(),
            scope: f.target.scope.clone(),
            trust_revision: 3,
            permissions: recipient_rights,
            delegable: Vec::new(),
            not_before: Some(GoTime(at.0 - time::Duration::minutes(1))),
            expires_at: Some(GoTime(at.0 + time::Duration::seconds(30))),
            revoked_at: None,
        };
        let mut decision_snapshot = snapshot.clone();
        decision_snapshot
            .grants
            .insert("recipient-grant".to_string(), recipient_grant);
        decision_snapshot.instances.insert(
            "receiver-instance".to_string(),
            Instance {
                id: "receiver-instance".into(),
                logical_agent_id: "receiver-logical".into(),
                principal: recipient_actor.principal.clone(),
                scope: f.target.scope.clone(),
                grant_id: "recipient-grant".into(),
                binding_id: "recipient-channel".into(),
                revoked_at: None,
            },
        );
        authority.roots.insert("recipient-grant".to_string());
        authority.snapshot_digest = identity_digest(&decision_snapshot).expect("fixture digest");
        let recipient = Caller {
            project_root: authority.project_root.clone(),
            actor: recipient_actor,
            trust_revision: 3,
            expires_at: None,
            binding_id: "recipient-channel".into(),
            verified_at: at,
            session_digest: String::new(),
        };
        DecisionFixture {
            root: f.root.clone(),
            work: f.work.clone(),
            launch: f.launch.clone(),
            target: f.target.clone(),
            ids: MemoryUsageDecisionIds {
                event_id: "decision-event".into(),
                requester_grant: "reader-grant".into(),
                recipient_grant: "recipient-grant".into(),
            },
            frame: MemoryUsageFrame {
                snapshot: decision_snapshot,
                authority,
                requester,
                recipient,
                at,
            },
        }
    }

    impl Fixture {
        fn frame_recipient_issuer(
            &self,
            requester: &Caller,
        ) -> Actor {
            // Go: Issuer: requester.actor — the requester (service principal,
            // empty instance) issued the recipient grant.
            let mut issuer = requester.actor.clone();
            issuer.instance_id = String::new();
            issuer
        }
    }

    impl DecisionFixture {
        fn refresh(&mut self) {
            self.frame.authority.snapshot_digest =
                identity_digest(&self.frame.snapshot).expect("fixture digest");
            self.frame.authority.snapshot_revision = self.frame.snapshot.revision;
        }

        fn clone_state(&self) -> DecisionFixture {
            DecisionFixture {
                root: self.root.clone(),
                work: self.work.clone(),
                launch: self.launch.clone(),
                target: self.target.clone(),
                ids: self.ids.clone(),
                frame: self.frame.clone(),
            }
        }

        fn evaluate(&self, previous: Option<&MemoryUsageDecision>) -> Result<MemoryUsageDecision, LaunchError> {
            evaluate_memory_summary_usage(
                &self.root,
                &self.work,
                &self.launch,
                &self.target,
                &self.ids,
                &self.frame,
                previous,
            )
        }

        fn change_grant(&mut self, id: &str, change: impl FnOnce(&mut Grant)) {
            let mut g = self.frame.snapshot.grants.get(id).expect("grant exists").clone();
            change(&mut g);
            self.frame.snapshot.grants.insert(id.to_string(), g);
        }
    }

    fn refusal_code(err: &LaunchError) -> Option<&'static str> {
        match err {
            LaunchError::Refused(code) => Some(code),
            _ => None,
        }
    }

    fn require_refusal(
        got: MemoryUsageDecision,
        err: LaunchError,
        code: &'static str,
        context: &str,
    ) {
        assert!(
            got.binding_digest.is_empty()
                && got.requester.is_empty()
                && got.recipient.is_empty(),
            "{context}: refusal exposed a full or partial decision"
        );
        assert_eq!(
            refusal_code(&err),
            Some(code),
            "{context}: want refusal {code}, got {err:?}"
        );
    }

    /// Go `TestMemoryUsageResourceSnapshotTaskBoundary` (already covered for
    /// the binding in M25 tests): the decision re-derives it through the same
    /// binding call, so this module verifies the full-verdict invariants.
    #[test]
    fn checks_both_actors_before_returning() {
        let f = decision_setup();
        let got = f.evaluate(None).expect("complete decision");
        assert_eq!(got.requester.len(), 13, "5 reads + 5 summaries + 3 transmissions");
        assert_eq!(got.recipient.len(), 3, "one read per transmitted source");
        assert_eq!(
            got.expires_at,
            Some(GoTime(f.frame.at.0 + time::Duration::seconds(30))),
            "aggregate outlived the recipient grant"
        );
        let mut events = std::collections::HashSet::new();
        for decision in got.requester.iter().chain(got.recipient.iter()) {
            assert!(
                events.insert(decision.event_id.clone()),
                "requester and recipient events alias"
            );
        }
        for decision in &got.recipient {
            assert_eq!(
                decision.actor.principal.subject, "receiver-subject",
                "recipient decision lost actual actor"
            );
        }
        let again = f.evaluate(Some(&got)).expect("exact current reuse");
        assert_eq!(got, again, "exact reuse diverged");
    }

    /// Go `TestMemoryUsageDecisionRejectsEveryIncompleteAuthority`.
    #[test]
    fn rejects_every_incomplete_authority() {
        let mut base = decision_setup();
        let previous = base.evaluate(None).expect("initial decision");
        let mut cases: Vec<(&str, &'static str, Box<dyn Fn(&mut DecisionFixture)>, bool)> = Vec::new();
        cases.push(("last-requester-right", "agent-identity-permission-denied", Box::new(|f: &mut DecisionFixture| {
            let id = f.ids.requester_grant.clone();
            f.change_grant(&id, |g| { g.permissions.pop(); });
        }), false));
        cases.push(("last-recipient-right", "agent-identity-permission-denied", Box::new(|f: &mut DecisionFixture| {
            let id = f.ids.recipient_grant.clone();
            f.change_grant(&id, |g| { g.permissions.pop(); });
        }), false));
        cases.push(("missing-recipient-caller", "memory-usage-recipient-mismatch", Box::new(|f: &mut DecisionFixture| {
            f.frame.recipient.actor = Actor {
                principal: swarm_identity_contract::Principal {
                    id: String::new(), kind: String::new(), issuer: String::new(), subject: String::new(),
                },
                instance_id: String::new(),
            };
        }), false));
        cases.push(("other-recipient-subject", "memory-usage-recipient-mismatch", Box::new(|f: &mut DecisionFixture| {
            f.frame.recipient.actor.principal.subject = "another-subject".into();
        }), false));
        cases.push(("other-recipient-instance", "memory-usage-recipient-mismatch", Box::new(|f: &mut DecisionFixture| {
            f.frame.recipient.actor.instance_id = "another-instance".into();
        }), false));
        cases.push(("missing-recipient-channel", "agent-identity-binding-mismatch", Box::new(|f: &mut DecisionFixture| {
            f.frame.recipient.binding_id = String::new();
        }), false));
        cases.push(("unregistered-recipient", "agent-identity-binding-mismatch", Box::new(|f: &mut DecisionFixture| {
            f.frame.snapshot.instances.remove("receiver-instance");
        }), false));
        cases.push(("revoked-recipient-instance", "agent-identity-binding-mismatch", Box::new(|f: &mut DecisionFixture| {
            let at = f.frame.at;
            if let Some(i) = f.frame.snapshot.instances.get_mut("receiver-instance") {
                i.revoked_at = Some(at);
            }
        }), false));
        cases.push(("revoked-recipient-grant", "agent-identity-grant-inactive", Box::new(|f: &mut DecisionFixture| {
            let at = f.frame.at;
            let id = f.ids.recipient_grant.clone();
            f.change_grant(&id, |g| { g.revoked_at = Some(at); });
        }), false));
        cases.push(("expired-recipient-grant", "agent-identity-grant-inactive", Box::new(|f: &mut DecisionFixture| {
            let at = f.frame.at;
            let id = f.ids.recipient_grant.clone();
            f.change_grant(&id, |g| { g.expires_at = Some(at); });
        }), false));
        cases.push(("revoked-requester-grant", "agent-identity-grant-inactive", Box::new(|f: &mut DecisionFixture| {
            let at = f.frame.at;
            let id = f.ids.requester_grant.clone();
            f.change_grant(&id, |g| { g.revoked_at = Some(at); });
        }), false));
        cases.push(("stale-requester", "agent-identity-caller-stale", Box::new(|f: &mut DecisionFixture| {
            f.frame.requester.verified_at = GoTime(f.frame.at.0 - time::Duration::seconds(1));
        }), false));
        cases.push(("stale-recipient", "agent-identity-caller-stale", Box::new(|f: &mut DecisionFixture| {
            f.frame.recipient.verified_at = GoTime(f.frame.at.0 - time::Duration::seconds(1));
        }), false));
        cases.push(("expired-recipient-caller", "agent-identity-caller-stale", Box::new(|f: &mut DecisionFixture| {
            let at = f.frame.at;
            f.frame.recipient.expires_at = Some(at);
        }), false));
        cases.push(("invalid-event", "memory-usage-binding-invalid", Box::new(|f: &mut DecisionFixture| {
            f.ids.event_id = String::new();
        }), false));
        cases.push(("missing-recipient-grant", "memory-usage-binding-invalid", Box::new(|f: &mut DecisionFixture| {
            f.ids.recipient_grant = String::new();
        }), false));
        cases.push(("foreign-control-project", "memory-usage-project-mismatch", Box::new(|f: &mut DecisionFixture| {
            f.frame.authority.project_root = f.root.join("other-project").to_string_lossy().into_owned();
        }), false));
        cases.push(("missing-clock", "agent-identity-clock-required", Box::new(|f: &mut DecisionFixture| {
            f.frame.at = crate::agent_identity_kernel::Decision::go_zero_time();
        }), false));
        cases.push(("unanchored-snapshot", "agent-identity-snapshot-changed", Box::new(|f: &mut DecisionFixture| {
            let id = f.ids.requester_grant.clone();
            f.change_grant(&id, |g| {
                g.permissions.push(swarm_identity_contract::Permission {
                    operation: "tool.invoke".into(),
                    resource: "other-tool".into(),
                });
            });
        }), true));
        for (name, code, change, unanchored) in cases {
            let mut f = base.clone_state();
            change(&mut f);
            if !unanchored {
                f.refresh();
            }
            let got = f.evaluate(Some(&previous));
            match got {
                Ok(decision) => panic!("{name}: incomplete authority accepted {decision:?}"),
                Err(err) => require_refusal(
                    MemoryUsageDecision::default(),
                    err,
                    code,
                    name,
                ),
            }
            let valid = base.evaluate(Some(&previous));
            match valid {
                Ok(v) => assert_eq!(v.recipient.len(), 3, "{name}: damaged subsequent valid use"),
                Err(err) => panic!("{name}: isolated refusal damaged subsequent valid use: {err:?}"),
            }
        }
        let _ = &mut base;
    }

    /// Go `TestMemoryUsageDecisionRejectsEditedOrPartialPrevious`.
    #[test]
    fn rejects_edited_or_partial_previous() {
        let f = decision_setup();
        let previous = f.evaluate(None).expect("initial decision");
        let mut cases: Vec<(&str, Box<dyn Fn(&mut MemoryUsageDecision)>)> = Vec::new();
        cases.push(("missing-requester", Box::new(|d: &mut MemoryUsageDecision| {
            d.requester.pop();
        })));
        cases.push(("missing-recipient", Box::new(|d: &mut MemoryUsageDecision| {
            d.recipient.pop();
        })));
        cases.push(("reordered-recipient", Box::new(|d: &mut MemoryUsageDecision| {
            d.recipient.swap(0, 1);
        })));
        cases.push(("edited-binding", Box::new(|d: &mut MemoryUsageDecision| {
            d.binding_digest = crate::agent_launch_preflight::hash(b"another binding");
        })));
        cases.push(("edited-expiration", Box::new(|d: &mut MemoryUsageDecision| {
            d.expires_at = Some(GoTime(d.expires_at.unwrap().0 + time::Duration::seconds(1)));
        })));
        cases.push(("edited-actor", Box::new(|d: &mut MemoryUsageDecision| {
            d.requester[0].actor.principal.subject = "forged".into();
        })));
        cases.push(("roles-swapped", Box::new(|d: &mut MemoryUsageDecision| {
            std::mem::swap(&mut d.requester, &mut d.recipient);
        })));
        cases.push(("empty", Box::new(|d: &mut MemoryUsageDecision| {
            *d = MemoryUsageDecision {
                binding_digest: String::new(),
                requester: Vec::new(),
                recipient: Vec::new(),
                expires_at: None,
            };
        })));
        for (name, change) in cases {
            let mut candidate = previous.clone();
            change(&mut candidate);
            let got = f.evaluate(Some(&candidate));
            match got {
                Ok(decision) => panic!("{name}: edited previous accepted {decision:?}"),
                Err(err) => require_refusal(MemoryUsageDecision::default(), err, "memory-usage-decision-stale", name),
            }
        }
        let got = f.evaluate(Some(&previous)).expect("editing a copy kept the original proof valid");
        assert_eq!(got, previous, "editing a copy damaged original proof");
    }

    /// Go `TestMemoryUsageDecisionRebuildsAndRechecksBeforeReuse`.
    #[test]
    fn rebuilds_and_rechecks_before_reuse() {
        let base = decision_setup();
        let previous = base.evaluate(None).expect("initial decision");

        // changed-selection
        {
            let mut f = base.clone_state();
            f.launch.summary_selection.as_mut().unwrap().reason = "Another exact selection".into();
            let got = f.evaluate(Some(&previous));
            match got {
                Ok(d) => panic!("changed-selection accepted {d:?}"),
                Err(err) => require_refusal(MemoryUsageDecision::default(), err, "memory-usage-decision-stale", "changed-selection"),
            }
        }
        // stale-answer
        {
            let mut f = base.clone_state();
            f.work.tasks.as_mut().unwrap()[2].answers.as_mut().unwrap()[0].text = "Changed stored answer".into();
            let got = f.evaluate(Some(&previous));
            assert!(got.is_err(), "stale-answer reused a decision bound to other source bytes");
        }
        // other-task-snapshot
        {
            let mut f = base.clone_state();
            f.launch.task_id = "other".into();
            f.target.task = "other".into();
            refresh_selection(
                &f.root,
                &f.work,
                &f.launch.task_id.clone(),
                f.launch.summary_selection.as_mut().unwrap(),
            );
            let got = f.evaluate(Some(&previous));
            match got {
                Ok(d) => panic!("other-task-snapshot accepted {d:?}"),
                Err(err) => require_refusal(MemoryUsageDecision::default(), err, "agent-identity-permission-denied", "other-task-snapshot"),
            }
        }
        // new-authority
        {
            let mut f = base.clone_state();
            f.frame.snapshot.revision += 1;
            f.refresh();
            let got = f.evaluate(Some(&previous));
            match got {
                Ok(d) => panic!("new-authority accepted {d:?}"),
                Err(err) => require_refusal(MemoryUsageDecision::default(), err, "memory-usage-decision-stale", "new-authority"),
            }
            f.evaluate(None).expect("fresh evaluation under current authority");
        }
        // new-grant
        {
            let mut f = base.clone_state();
            let mut g = f.frame.snapshot.grants["reader-grant"].clone();
            let original = g.clone();
            g.id = "new-requester-grant".into();
            f.ids.requester_grant = g.id.clone();
            f.frame.snapshot.grants.insert(g.id.clone(), g);
            f.frame.authority.roots.insert("new-requester-grant".into());
            f.frame.snapshot.revision += 1;
            validate_agent_identity_transition(&original_snapshot(&original, &f), &f.frame.snapshot)
                .expect("fixture proposed an illegal registry transition");
            f.refresh();
            let got = f.evaluate(Some(&previous));
            match got {
                Ok(d) => panic!("new-grant accepted {d:?}"),
                Err(err) => require_refusal(MemoryUsageDecision::default(), err, "memory-usage-decision-stale", "new-grant"),
            }
        }
        // reauthenticate-then-expire
        {
            let base_at = base.frame.at;
            let mut f = base.clone_state();
            f.frame.at = GoTime(base_at.0 + time::Duration::seconds(1));
            f.frame.requester.verified_at = f.frame.at;
            f.frame.recipient.verified_at = f.frame.at;
            f.evaluate(Some(&previous)).expect("fresh actors could not reuse still-current decision");
            f.frame.at = GoTime(base_at.0 + time::Duration::seconds(31));
            f.frame.requester.verified_at = f.frame.at;
            f.frame.recipient.verified_at = f.frame.at;
            let got = f.evaluate(Some(&previous));
            match got {
                Ok(d) => panic!("reauthenticate-then-expire accepted {d:?}"),
                Err(err) => require_refusal(MemoryUsageDecision::default(), err, "agent-identity-grant-inactive", "reauthenticate-then-expire"),
            }
        }
    }

    /// Snapshot with only the original reader-grant — the "before" side of the
    /// new-grant transition (grant added under a new ID is a legal append).
    fn original_snapshot(original: &Grant, _f: &DecisionFixture) -> Snapshot {
        let mut grants = BTreeMap::new();
        grants.insert(original.id.clone(), original.clone());
        Snapshot {
            schema: 1,
            revision: 1,
            grants,
            instances: BTreeMap::new(),
        }
    }

    /// Go `TestMemoryUsageRequestsUseIndependentKernelRightsAndFreshness`:
    /// every request the binding emits evaluates independently; removing any
    /// one right denies the whole flow.
    #[test]
    fn requests_use_independent_kernel_rights_and_freshness() {
        let f = fixture();
        let binding = memory_summary_usage_binding(&f.root, &f.work, &f.launch, &f.target)
            .expect("fixture binding");
        let requests = binding
            .requests("usage-event", "reader-grant")
            .expect("fixture requests");
        let at = fixture_clock();
        let actor_v = Actor {
            principal: swarm_identity_contract::Principal {
                id: "requester".into(),
                kind: "service".into(),
                issuer: "urn:fixture:issuer".into(),
                subject: "requester".into(),
            },
            instance_id: String::new(),
        };
        let mut rights: Vec<swarm_identity_contract::Permission> = Vec::new();
        for q in &requests {
            rights.push(swarm_identity_contract::Permission {
                operation: q.operation.clone(),
                resource: q.resource.clone(),
            });
        }
        let grant = Grant {
            id: "reader-grant".into(),
            parent_id: String::new(),
            issuer: actor_v.clone(),
            holder: actor_v.clone(),
            scope: binding.projection.as_ref().unwrap().target.scope.clone(),
            trust_revision: 3,
            permissions: rights,
            delegable: Vec::new(),
            not_before: Some(GoTime(at.0 - time::Duration::minutes(1))),
            expires_at: Some(GoTime(at.0 + time::Duration::minutes(1))),
            revoked_at: None,
        };
        let mut grants = BTreeMap::new();
        grants.insert("reader-grant".to_string(), grant);
        let snapshot = Snapshot {
            schema: 1,
            revision: 1,
            grants,
            instances: BTreeMap::new(),
        };
        let authority = Authority {
            project_root: f.root.to_string_lossy().into_owned(),
            snapshot_digest: identity_digest(&snapshot).expect("fixture digest"),
            snapshot_revision: 1,
            trust_revision: 3,
            scope: binding.projection.as_ref().unwrap().target.scope.clone(),
            roots: ["reader-grant".to_string()].into_iter().collect(),
        };
        let caller = Caller {
            project_root: authority.project_root.clone(),
            actor: actor_v,
            trust_revision: 3,
            expires_at: None,
            binding_id: String::new(),
            verified_at: at,
            session_digest: String::new(),
        };
        for q in &requests {
            evaluate_agent_identity(&snapshot, &authority, &caller, &kernel_request(q), at)
                .unwrap_or_else(|e| panic!("independent request refused: {e:?}"));
        }
        for missing in ["memory.read", "memory.summarize", "memory.transmit"] {
            let mut reduced = snapshot.clone();
            let mut grant = reduced.grants["reader-grant"].clone();
            grant
                .permissions
                .retain(|p| p.operation != missing);
            reduced.grants.insert("reader-grant".into(), grant);
            let mut authority = authority.clone();
            authority.snapshot_digest = identity_digest(&reduced).expect("digest");
            for q in &requests {
                if q.operation == missing {
                    assert!(
                        evaluate_agent_identity(&reduced, &authority, &caller, &kernel_request(q), at).is_err(),
                        "{missing}: right not enforced independently"
                    );
                }
            }
        }
        // Stale caller: same requests, earlier verification instant.
        let stale = Caller {
            verified_at: GoTime(at.0 - time::Duration::seconds(1)),
            ..caller.clone()
        };
        assert!(
            evaluate_agent_identity(&snapshot, &authority, &stale, &kernel_request(&requests[0]), at).is_err(),
            "stale caller accepted"
        );
    }
}
