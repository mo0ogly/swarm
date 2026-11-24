//! Protected metadata for recorded inter-scope and worker status signals.
use super::{finished, material_path, planning_history, RootStore, RootStoreError as E};
use crate::{
    planning_decision::event_id, planning_rejection::SCOPE_ESCALATION_PREFIX,
    AgentLaunchError as A, PlanningEvent,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const CLOSED: &str = "scope_closed";
const ESCALATED: &str = crate::planning_rejection::SCOPE_ESCALATION_KIND;

#[derive(Deserialize)]
struct Context {
    work: String,
    scope: ContextScope,
    events: Vec<PlanningEvent>,
}
#[derive(Deserialize)]
struct ContextScope {
    id: String,
}

fn domain(error: A) -> E {
    E::Launch(Box::new(error))
}

/// Match the earliest recorded context that contains each exact signal. This
/// establishes delivery to the planning context, without inferring provider
/// startup, comprehension or a successful decision. Raw contexts stay private.
fn receipts(
    db: &rusqlite::Connection,
    work: &str,
    events: &[&PlanningEvent],
) -> Result<BTreeMap<String, Value>, E> {
    if events.is_empty() || !crate::planning_call_inputs::schema(db, false).map_err(domain)? {
        return Ok(BTreeMap::new());
    }
    let pending: BTreeMap<_, _> = events
        .iter()
        .map(|event| (event.id.as_str(), *event))
        .collect();
    let scopes: BTreeSet<_> = events.iter().map(|event| event.scope.as_str()).collect();
    crate::agent_launch::query(db,
        "SELECT c.claim_id,c.scope_id,c.recorded_at,c.context_json,c.context_sha256,c.claim_revision,         e.work_id,e.revision,e.kind FROM main.native_planning_claims c          LEFT JOIN main.events e ON e.id=c.claim_id WHERE c.work_id=? ORDER BY c.claim_revision",
        [work], |rows| {
            let mut result = BTreeMap::new();
            while let Some(row) = rows.next()? {
                let scope: String = row.get(1)?;
                if !scopes.contains(scope.as_str()) { continue; }
                let at: String = row.get(2)?;
                let context: String = row.get(3)?;
                let hash: String = row.get(4)?;
                let revision: i64 = row.get(5)?;
                let owner: Option<String> = row.get(6)?;
                let event_revision: Option<i64> = row.get(7)?;
                let kind: Option<String> = row.get(8)?;
                if owner.as_deref() != Some(work) || event_revision != Some(revision)
                    || kind.as_deref() != Some("planning.claim")
                    || format!("{:x}", Sha256::digest(context.as_bytes())) != hash
                {
                    return Err(A::Refused("orchestration-signal-claim-binding"));
                }
                let envelope: Context = serde_json::from_str(&context)
                    .map_err(|e| A::caused("orchestration-signal-context-invalid", e))?;
                if envelope.work != work || envelope.scope.id != scope {
                    return Err(A::Refused("orchestration-signal-context-owner"));
                }
                for event in envelope.events {
                    let Some(sent) = pending.get(event.id.as_str()) else {continue};
                    if event.scope != scope || sent.scope != scope || event.kind != sent.kind
                        || event.at != sent.at || event.message != sent.message
                        || event.task != sent.task || event.attempt != sent.attempt
                    {
                        return Err(A::Refused("orchestration-signal-context-event-mismatch"));
                    }
                    let claim: String = row.get(0)?;
                    result.entry(event.id).or_insert_with(|| json!({
                        "claim":claim, "at":at, "context_sha256":hash
                    }));
                }
            }
            Ok(result)
        }).map_err(domain)
}

impl RootStore {
    /// Read signals for one existing Work in a single rooted SQL snapshot.
    /// Transmission uses canonical inbox entries. Reception requires a matched
    /// retained planning context. Processing requires a matching saved decision.
    /// No provider context, raw attachment or new permission is returned.
    /// Missing historical proofs remain absent; read and binding errors propagate.
    pub fn orchestration_signals(&self, id: &str) -> Result<Value, E> {
        if !swarm_identity_contract::name(id.as_bytes()) {
            return Err(E::Refused("orchestration-signal-work-invalid"));
        }
        material_path::with_material_read(self, || {
            let mut tx = self.db.unchecked_transaction().map_err(E::Sql)?;
            tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
            let result = (|| {
                let work = crate::read_recorded_work(&tx, id).map_err(E::Work)?;
                let inbox = work
                    .planning
                    .as_ref()
                    .and_then(|p| p.inbox.as_deref())
                    .unwrap_or_default();
                let signals: Vec<_> = inbox
                    .iter()
                    .filter(|e| {
                        matches!(
                            e.kind.as_str(),
                            CLOSED | ESCALATED | "attempt_ended" | "handoff"
                        )
                    })
                    .collect();
                let mut received = receipts(&tx, id, &signals)?;
                let mut closures = BTreeMap::new();
                let mut decisions = BTreeMap::new();
                if !signals.is_empty() {
                    for event in crate::list_recorded_events(&tx, id).map_err(E::Events)? {
                        if event.kind != b"planning.decide" {
                            continue;
                        }
                        let uid = crate::text::go_utf8(&event.id);
                        let at = crate::text::go_utf8(&event.at);
                        let request =
                            planning_history::payload(event.payload.as_deref().unwrap_or_default())
                                .map_err(|e| E::PlanningHistory(Box::new(e)))?;
                        if request
                            .operations
                            .iter()
                            .flatten()
                            .any(|op| op.kind == "close")
                        {
                            closures.insert(
                                event_id(&[&uid, "closed", &request.scope]).map_err(domain)?,
                                request.scope.clone(),
                            );
                        }
                        decisions.insert(uid.to_string(), (at, request));
                    }
                }
                let rows = signals.into_iter().map(|e| {
                    let sender = match e.kind.as_str() {
                        CLOSED => closures.get(&e.id).map(String::as_str),
                        ESCALATED => e.id.strip_prefix(SCOPE_ESCALATION_PREFIX).and_then(|id|
                            inbox.iter().find(|original| original.id == id && original.kind == "planning_rejected"))
                            .map(|original| original.scope.as_str()),
                        _ => Some(e.task.as_str()).filter(|task| !task.is_empty()),
                    };
                    let processed = decisions.get(&e.decision).filter(|(_, d)|
                        d.scope == e.scope && d.input_events.iter().flatten().any(|id| id == &e.id))
                        .map(|(at, d)| json!({
                            "decision":e.decision, "at":at, "reason":d.reason,
                            "operations":d.operations.iter().flatten().map(|op| &op.kind).collect::<Vec<_>>()
                        }));
                    json!({
                        "id":e.id, "kind":e.kind, "from":sender, "to":e.scope, "at":e.at, "message":e.message,
                        "received":received.remove(&e.id), "processed":processed
                    })
                }).collect::<Vec<_>>();
                Ok(
                    json!({"schema_version":1,"work":work.id,"revision":work.revision,"signals":rows}),
                )
            })();
            finished(result, tx.rollback().map_err(E::Sql))
        })
    }
}
