//! Native worker completion inputs and explicit failed-attempt recovery fences.
use super::{finished, RootStore, RootStoreError as E};
use crate::{PlanningEvent, Work};
use sha2::{Digest, Sha256};

/// Safe owned-scope input built from original controller evidence, not a DTO
/// assertion or permission. No raw capture/prompt, host or process stamp leaks.
#[derive(serde::Serialize)]
pub(crate) struct WorkerCompletionInput {
    pub event: String,
    pub task: String,
    pub attempt: String,
    pub agent: String,
    pub outcome: String,
    pub receipt_sha256: String,
    pub output_complete: bool,
    pub provider_may_have_started: bool,
}
/// Internal read purpose; no public request can choose a history/grant mode.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Purpose { Context, Retry, Execution }

/// Verify one actual current/prior native terminal attempt and its exact logical
/// end signal in a readonly Root/SQL snapshot. Kernel original, full terminal
/// Agent and reservation must agree. Failed/interrupted retry requires a new
/// canonical decision at admission/execution; no PID/lease/message grants it.
/// All IO/codec/rollback/close causes survive; reads create no new call or state.
fn observe(root: &std::path::Path, supplied: &Work, task_id: &str, purpose: Purpose) -> Result<WorkerCompletionInput, E> {
    let store = RootStore::open(root, false)?;
    let result = (|| {
        let mut tx = store.db.unchecked_transaction().map_err(E::Sql)?;
        let read = (|| {
            let current = crate::read_recorded_work(&tx, &supplied.id).map_err(E::Work)?;
            let mut given = supplied.tasks.iter().flatten().filter(|t| t.id == task_id);
            let task = given.next().ok_or(E::Refused("native-worker-task-required"))?;
            let mut stored = current.tasks.iter().flatten().filter(|t| t.id == task_id);
            if given.next().is_some() || stored.next() != Some(task) || stored.next().is_some() || current.revision != supplied.revision {
                return Err(E::Refused("native-worker-source-changed"));
            }
            let planning = current.planning.as_deref().ok_or(E::Refused("native-worker-planning-required"))?;
            if task.plan_role != "worker" || !planning.scopes.iter().flatten().any(|s| s.id == task.scope_id) { return Err(E::Refused("native-worker-scope-required")); }
            let attempts = task.attempts.as_deref().unwrap_or_default();
            let index = attempts.len().checked_sub(if purpose == Purpose::Execution {2} else {1}).ok_or(E::Refused("native-worker-attempt-required"))?;
            let attempt = &attempts[index];
            if attempt.ended.is_empty() || purpose != Purpose::Context && !matches!(attempt.status.as_str(), "failed" | "interrupted") {
                return Err(E::Refused("native-worker-failed-attempt-required"));
            }
            if purpose == Purpose::Retry && (task.status != "blocked" || task.independent_review.is_some())
                || purpose == Purpose::Execution && (task.status != "running" || task.planning_retry || task.independent_review.is_some()) {
                return Err(E::Refused("native-worker-retry-state-invalid"));
            }
            let agents = crate::list_agents(&tx, &current.id).map_err(E::Agent)?;
            let mut matched = agents.iter().filter(|a| a.task_id == task.id && a.attempt_id == attempt.id);
            let agent = matched.next().ok_or(E::Refused("native-worker-agent-required"))?;
            if matched.next().is_some() { return Err(E::Refused("native-worker-agent-ambiguous")); }
            let original = crate::agent_completion_journal::observe_terminal(&tx, &store.directory, &agent.id)?;
            if original.agent != *agent || agent.work_id != current.id || agent.role != "worker" || agent.status != attempt.status {
                return Err(E::Refused("native-worker-completion-changed"));
            }
            let bytes = crate::encode_model(&["attempt-ended", attempt.id.as_str()]).map_err(E::Encode)?;
            let event_id = format!("plan-{:x}", Sha256::digest(bytes));
            let mut events = planning.inbox.iter().flatten().filter(|e| e.id == event_id);
            let event = events.next().ok_or(E::Refused("native-worker-end-event-required"))?;
            if events.next().is_some() { return Err(E::Refused("native-worker-end-event-ambiguous")); }
            let expected = PlanningEvent { id: event_id.clone(), task: task.id.clone(), scope: task.scope_id.clone(), attempt: attempt.id.clone(),
                kind: "attempt_ended".into(), message: format!("{} : processus récolté ; validation requise", agent.status), at: agent.ended.clone(), decision: event.decision.clone(), ..Default::default() };
            if event != &expected { return Err(E::Refused("native-worker-end-event-changed")); }
            if purpose == Purpose::Execution || purpose == Purpose::Retry && task.planning_retry {
                super::native_review_context::verify_production_retry_decision_for_input(&tx, &current, task, &event_id)?;
            }
            Ok(WorkerCompletionInput { event: event_id, task: task.id.clone(), attempt: attempt.id.clone(), agent: agent.id.clone(), outcome: agent.status.clone(),
                receipt_sha256: original.receipt_sha256, output_complete: original.output_complete, provider_may_have_started: original.provider_may_have_started })
        })();
        tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
        finished(read, tx.rollback().map_err(E::Sql))
    })();
    finished(result, store.close())
}
impl RootStore {
    /// Verify a selected current end event before model input; historical events
    /// are handled by their existing acknowledgement-only contract. Returns safe
    /// metadata and exact original hash, with no process/source/permission change.
    pub(crate) fn worker_completion_for_context(root: &std::path::Path, work: &Work, event: &PlanningEvent) -> Result<WorkerCompletionInput, E> {
        let observed = observe(root, work, &event.task, Purpose::Context)?;
        if observed.event != event.id || observed.attempt != event.attempt || !work.planning.iter().flat_map(|p| p.inbox.iter().flatten()).any(|e| e == event) {
            return Err(E::Refused("native-worker-input-event-changed"));
        }
        Ok(observed)
    }
    /// Require an actual failed/interrupted native attempt before same-task
    /// correction. A persisted retry signal additionally needs its exact current
    /// canonical decision/acknowledgement. No attempt/reserve/reset/acceptance.
    pub(crate) fn failed_worker_retry_input(root: &std::path::Path, work: &Work, task: &str) -> Result<WorkerCompletionInput, E> {
        observe(root, work, task, Purpose::Retry)
    }
    /// Recheck the immediately preceding failed native attempt at each execution
    /// stage after the new attempt is recorded. Kernel receipt, end signal and
    /// canonical correction remain mandatory; no archived review is invented.
    pub(crate) fn verify_failed_worker_retry_execution(root: &std::path::Path, work: &Work, task: &str) -> Result<(), E> {
        observe(root, work, task, Purpose::Execution).map(|_| ())
    }
}
