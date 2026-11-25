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
fn plan(work: &Work, task: &Task, planned_next: Option<&str>) -> Value {
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
        "planned_next": planned_next,
        "approved": approved
    })
}

#[derive(Deserialize)]
struct TaskEdit {
    id: Option<String>,
    status: Option<String>,
    next: Option<String>,
}

#[derive(Default)]
struct History {
    planned_next: Option<String>,
    adjustments: Vec<Value>,
}

/// Recover task instructions in Work revision order. A status transition may
/// replace the current next step but cannot rewrite the recorded plan. Planning
/// retries and task edits keep per-task text only. Corrupt payloads stay errors.
fn history(db: &rusqlite::Connection, work: &str, task: &str) -> Result<History, E> {
    let mut history = History::default();
    for event in crate::list_recorded_events(db, work).map_err(E::Events)? {
        let raw = event.payload.as_deref().unwrap_or_default();
        if matches!(event.kind.as_slice(), b"task.add" | b"task.update") {
            let text = crate::text::go_utf8(raw);
            let edit: TaskEdit = serde_json::from_str(&text).map_err(|cause| {
                E::Launch(Box::new(AgentLaunchError::caused(
                    "task-actions-edit-invalid",
                    cause,
                )))
            })?;
            if edit.id.as_deref() != Some(task) {
                continue;
            }
            let Some(next) = edit.next.filter(|value| !value.is_empty()) else {
                continue;
            };
            if edit.status.as_deref().is_none_or(str::is_empty) {
                history.planned_next = Some(next.clone());
            }
            if event.kind == b"task.update" {
                history.adjustments.push(json!({
                    "at": crate::text::go_utf8(&event.at), "kind": "task_update", "instruction": next
                }));
            }
        }
        #[cfg(target_os = "linux")]
        if event.kind == b"planning.decide" {
            let decision = super::planning_history::payload(raw)
                .map_err(|cause| E::PlanningHistory(Box::new(cause)))?;
            for op in decision
                .operations
                .into_iter()
                .flatten()
                .filter(|op| op.id == task)
            {
                if op.kind == "task" && !op.next.is_empty() {
                    history.planned_next = Some(op.next);
                } else if matches!(op.kind.as_str(), "retry" | "review-retry") {
                    history.adjustments.push(json!({
                        "at": crate::text::go_utf8(&event.at), "kind": op.kind, "instruction": op.next
                    }));
                }
            }
        }
    }
    Ok(history)
}

impl RootStore {
    /// Preview the existing acceptance transition for a submitted human-mode
    /// task at the given Work revision. Uses saved policy and current evidence
    /// under a protected read snapshot. Refusals retain typed causes. No event,
    /// permission, reservation for a later write, or acceptance is recorded.
    pub fn preview_task_human_review(&self, work_id: &str, task_id: &str, revision: i64) -> Result<(), E> {
        use crate::{SystemEnvironment, TaskLifecycleContext, WorkEngineError};
        if revision < 1 || !swarm_identity_contract::name(work_id.as_bytes())
            || !swarm_identity_contract::name(task_id.as_bytes())
        {
            return Err(E::Refused("task-review-query-invalid"));
        }
        material_path::with_material_read(self, || {
            let settings = self.read_settings()?;
            let limits = settings.values.mutation_limits().map_err(E::Settings)?;
            let quality = settings.values.has_quality_policy()
                .then(|| settings.quality_policy_snapshot())
                .transpose()
                .map_err(E::Settings)?;
            let mut tx = self.db.unchecked_transaction().map_err(E::Sql)?;
            tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
            let result = (|| {
                let work = crate::read_recorded_work(&tx, work_id).map_err(E::Work)?;
                if work.revision != revision {
                    return Err(E::Refused("task-review-revision-changed"));
                }
                let task = work.tasks.iter().flatten().find(|task| task.id == task_id)
                    .ok_or(E::Refused("task-actions-task-missing"))?;
                if task.status != "submitted" {
                    return Err(E::Refused("task-review-submission-required"));
                }
                if task.validation_policy.as_ref().is_none_or(|p| p.mode != "human" || p.authorized_at.is_empty()) {
                    return Err(E::Refused("task-review-human-policy-required"));
                }
                #[cfg(target_os = "linux")]
                {
                    crate::work_engine::require_native_task_idle(&tx, work_id, task_id, "task-active-agents")
                        .map_err(E::Mutation)?;
                }
                let request = crate::Request {
                    schema_version: 1,
                    id: task_id.into(),
                    status: "accepted".into(),
                    ..Default::default()
                };
                let context = TaskLifecycleContext {
                    root: &self.root,
                    quality_policy: quality.as_ref(),
                    task_limits: &limits.task,
                    validation_limits: &limits.policy,
                };
                let mut environment = SystemEnvironment;
                if work.planning.is_some() {
                    crate::task_lifecycle::transition_scoped_reviewed_task(
                        &work, &request, &context, &settings, &mut environment,
                    )
                } else {
                    crate::transition_manual_task(&work, &request, &context, &mut environment)
                }.map(|_| ()).map_err(|e| E::Mutation(WorkEngineError::Lifecycle(e)))
            })();
            finished(result, tx.rollback().map_err(E::Sql))
        })
    }

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
                let history = history(&tx, work_id, task_id)?;
                Ok(json!({
                    "schema_version": 1, "work": work_id, "revision": work.revision,
                    "task": task_id, "status": task.status, "plan": plan(&work, task, history.planned_next.as_deref()),
                    "attempts": attempts, "adjustments": {"instructions": instructions, "restarts": restarts, "decisions": history.adjustments},
                    "review": {"mode": mode, "awaiting_human": mode == Some("human") && task.status == "submitted"}
                }))
            })();
            finished(result, tx.rollback().map_err(E::Sql))
        })
    }
}
