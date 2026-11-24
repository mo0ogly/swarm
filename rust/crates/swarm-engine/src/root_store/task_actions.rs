//! Read-only action file projection from attributable Work and Agent records.
use super::{finished, material_path, RootStore, RootStoreError as E};
use crate::{AgentLaunchError, Task, Work};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
struct ExecutionReceipt {
    output_complete: Option<bool>,
    task_actions: Option<crate::task_action_observation::ActionReceipt>,
}

/// Current task contract and the original approved task entry, when mapped.
/// Only this task's plan entry is exposed. The result is plain JSON data.
fn plan(work: &Work, task: &Task) -> Value {
    let approved = work.plans.iter().flatten().rev().find_map(|plan| {
        let (original, _) = plan
            .task_map
            .as_ref()?
            .iter()
            .find(|(_, id)| **id == task.id)?;
        let step = plan
            .spec
            .tasks
            .iter()
            .flatten()
            .find(|step| step.id == *original)?;
        Some(json!({"at": plan.at, "step": step}))
    });
    json!({
        "title": task.title,
        "scope": task.scope_id,
        "deliverable": task.deliverable,
        "criteria": task.criteria.as_deref().unwrap_or_default(),
        "depends": task.depends.as_deref().unwrap_or_default(),
        "next": task.next,
        "approved": approved
    })
}

/// Replays the history already recorded for this task. The operation's
/// explanation belongs to this task; the enclosing decision may span others.
#[cfg(target_os = "linux")]
fn revisions(db: &rusqlite::Connection, work: &str, task: &str) -> Result<Vec<Value>, E> {
    let mut result = Vec::new();
    for event in crate::list_recorded_events(db, work).map_err(E::Events)? {
        if event.kind != b"planning.decide" {
            continue;
        }
        let decision =
            super::planning_history::payload(event.payload.as_deref().unwrap_or_default())
                .map_err(|cause| E::PlanningHistory(Box::new(cause)))?;
        for op in decision
            .operations
            .into_iter()
            .flatten()
            .filter(|op| op.id == task)
        {
            if matches!(op.kind.as_str(), "retry" | "review-retry") {
                result.push(json!({
                    "at": crate::text::go_utf8(&event.at),
                    "kind": op.kind, "instruction": op.next
                }));
            }
        }
    }
    Ok(result)
}

impl RootStore {
    /// Read an existing task's file in a single protected Root read snapshot.
    /// Work/task keys must be valid identifiers, and agent attempts must match
    /// the same task record. Historical receipts without action detail remain
    /// explicitly unavailable. Bad retained receipts and rollback failures retain
    /// their causes. This read changes neither task state nor review authority.
    pub fn task_actions(&self, work_id: &str, task_id: &str) -> Result<Value, E> {
        if !swarm_identity_contract::name(work_id.as_bytes())
            || !swarm_identity_contract::name(task_id.as_bytes())
        {
            return Err(E::Refused("task-actions-key-invalid"));
        }
        material_path::with_material_read(self, || {
            let mut tx = self.db.unchecked_transaction().map_err(E::Sql)?;
            tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
            let result = (|| {
                let work = crate::read_recorded_work(&tx, work_id).map_err(E::Work)?;
                let task = work
                    .tasks
                    .iter()
                    .flatten()
                    .find(|task| task.id == task_id)
                    .ok_or(E::Refused("task-actions-task-missing"))?;
                let agents = crate::agent_store::list_task_agents(&tx, work_id, task_id)
                    .map_err(E::Agent)?;
                let mut attempts = Vec::new();
                for attempt in task.attempts.iter().flatten() {
                    let mut workers = agents
                        .iter()
                        .filter(|agent| agent.role == "worker" && agent.attempt_id == attempt.id);
                    let agent = workers.next();
                    if workers.next().is_some() {
                        return Err(E::Refused("task-actions-attempt-ambiguous"));
                    }
                    let receipt = agent
                        .and_then(|agent| agent.native_execution_result.as_ref())
                        .map(|receipt| {
                            serde_json::from_str::<ExecutionReceipt>(receipt.0.get()).map_err(
                                |cause| {
                                    E::Launch(Box::new(AgentLaunchError::caused(
                                        "task-actions-receipt-invalid",
                                        cause,
                                    )))
                                },
                            )
                        })
                        .transpose()?;
                    let output_complete =
                        receipt.as_ref().and_then(|receipt| receipt.output_complete);
                    let final_observation = receipt.and_then(|receipt| receipt.task_actions);
                    let (source, observation) = if final_observation.is_some() {
                        ("terminal_receipt", final_observation)
                    } else if let Some(raw) =
                        agent.and_then(|agent| agent.native_task_actions.as_ref())
                    {
                        let actions: crate::task_action_observation::ActionReceipt =
                            serde_json::from_str(raw.0.get()).map_err(|cause| {
                                E::Launch(Box::new(AgentLaunchError::caused(
                                    "task-actions-heartbeat-invalid",
                                    cause,
                                )))
                            })?;
                        ("last_heartbeat", Some(actions))
                    } else {
                        ("unavailable", None)
                    };
                    attempts.push(json!({
                        "id": attempt.id, "status": attempt.status,
                        "started": attempt.started, "ended": attempt.ended,
                        "output_complete": output_complete, "source": source,
                        "observation": observation
                    }));
                }
                let instructions = task
                    .instruction_revisions
                    .iter()
                    .flatten()
                    .map(|revision| {
                        json!({"at": revision.at, "state": revision.state,
                        "reason": revision.reason, "instruction": revision.addition,
                        "attempt": revision.used_by_attempt})
                    })
                    .collect::<Vec<_>>();
                let restarts = task
                    .restarts
                    .iter()
                    .flatten()
                    .map(|restart| {
                        json!({"at": restart.at, "reason": restart.reason,
                        "attempt": restart.previous_attempt})
                    })
                    .collect::<Vec<_>>();
                let policy = task
                    .validation_policy
                    .as_ref()
                    .filter(|policy| !policy.authorized_at.is_empty());
                let mode = policy.map(|policy| policy.mode.as_str());
                #[cfg(target_os = "linux")]
                let decisions = Some(revisions(&tx, work_id, task_id)?);
                #[cfg(not(target_os = "linux"))]
                let decisions: Option<Vec<Value>> = None;
                Ok(json!({
                    "schema_version": 1, "work": work_id, "revision": work.revision,
                    "task": task_id, "status": task.status, "plan": plan(&work, task),
                    "attempts": attempts, "adjustments": {"instructions": instructions, "restarts": restarts, "decisions": decisions},
                    "review": {"mode": mode, "awaiting_human": mode == Some("human") && task.status == "submitted"}
                }))
            })();
            finished(result, tx.rollback().map_err(E::Sql))
        })
    }
}
