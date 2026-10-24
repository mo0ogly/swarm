//! Owned scoped-worker delivery signals, never autonomous planner decisions.
use crate::{AgentLaunchError as E, ExchangeArtifact, PlanningEvent, Work};
use sha2::{Digest, Sha256};
/// Record an actually settled current attempt without accepting a task. Manual
/// Work is unchanged. Go-compatible deterministic IDs deduplicate identical
/// signals; conflicting IDs/invalid ownership refuse the caller's transaction.
/// An open scope becomes ready; a closed scope retains its state and receives
/// evidence without being reopened. Its supervisor still owns the decision.
pub(crate) fn ended(
    w: &mut Work,
    task: &str,
    attempt: &str,
    outcome: &str,
    at: &str,
) -> Result<(), E> {
    let bytes = crate::encode_model(&["attempt-ended", attempt])
        .map_err(|e| E::caused("agent-scope-signal-encoding", e))?;
    append(
        w,
        task,
        attempt,
        PlanningEvent {
            id: format!("plan-{:x}", Sha256::digest(bytes)),
            kind: "attempt_ended".into(),
            message: format!("{outcome} : processus récolté ; validation requise"),
            at: at.into(),
            ..Default::default()
        },
    )
}
/// Bind an explicit operator handoff's checked report bytes to its owning scope.
/// No synthetic attempt, reviewer call or accepted quality is created. Caller
/// verifies actual native completion/path/hash and commits the enclosing event.
pub(crate) fn handoff(
    w: &mut Work,
    task: &str,
    attempt: &str,
    report: ExchangeArtifact,
    at: &str,
) -> Result<(), E> {
    let bytes = crate::encode_model(&["report", task, attempt])
        .map_err(|e| E::caused("agent-scope-signal-encoding", e))?;
    append(
        w,
        task,
        attempt,
        PlanningEvent {
            id: format!("plan-{:x}", Sha256::digest(bytes)),
            kind: "handoff".into(),
            message: "Rapport remis explicitement par l’opérateur ; preuves et validation requises"
                .into(),
            handoff: Some(Box::new(report.clone())),
            artifacts: Some(vec![report]),
            at: at.into(),
            ..Default::default()
        },
    )
}
/// Validate before mutation; build a private candidate, validate again, then
/// publish it. Duplicate checks ignore only the later observation timestamp.
/// No ancestor/peer inbox receives the worker's result implicitly.
fn append(w: &mut Work, task: &str, attempt: &str, mut event: PlanningEvent) -> Result<(), E> {
    if w.planning.is_none() {
        return Ok(());
    }
    crate::validate_planning_state(w).map_err(|e| E::caused("agent-scope-state-invalid", e))?;
    swarm_wire_json::go_rfc3339::parse(&event.at)
        .map_err(|e| E::caused("agent-scope-clock-invalid", e))?;
    let t = w
        .tasks
        .iter()
        .flatten()
        .find(|t| t.id == task)
        .ok_or(E::Refused("agent-scope-task-missing"))?;
    if t.attempts
        .iter()
        .flatten()
        .last()
        .is_none_or(|a| a.id != attempt)
    {
        return Err(E::Refused("agent-scope-attempt-changed"));
    }
    event.scope = t.scope_id.clone();
    event.task = task.into();
    event.attempt = attempt.into();
    let mut next = w.clone();
    let planning = next
        .planning
        .as_mut()
        .ok_or(E::Refused("agent-scope-planning-missing"))?;
    if let Some(prior) = planning.inbox.iter().flatten().find(|e| e.id == event.id) {
        event.at = prior.at.clone();
        if prior != &event {
            return Err(E::Refused("agent-scope-signal-conflict"));
        }
        return Ok(());
    }
    let scope = planning
        .scopes
        .iter_mut()
        .flatten()
        .find(|s| s.id == event.scope)
        .ok_or(E::Refused("agent-scope-owner-missing"))?;
    if scope.state != "closed" {
        scope.state = "ready".into();
    }
    planning.inbox.get_or_insert_default().push(event);
    crate::validate_planning_state(&next).map_err(|e| E::caused("agent-scope-state-invalid", e))?;
    *w = next;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    /// A completed process may deliver evidence after scope closure without
    /// reopening it; no new start, peer delivery or acceptance is authorized.
    #[test]
    fn closed_scope_receives_terminal_evidence_without_reopening_and_conflicts_refuse() {
        let mut w=crate::decode_work(br#"{"schema_version":1,"id":"work","criteria":["owned"],"tasks":[{"id":"task","scope_id":"child","requirements":["req-1"],"status":"blocked","attempts":[{"id":"attempt","status":"completed"}]}],"planning":{"version":1,"max_tasks":2,"max_decisions":2,"max_activations":2,"scopes":[{"id":"root","revision":1,"state":"ready"},{"id":"child","parent":"root","revision":1,"state":"closed","requirements":["req-1"]}]}}"#).unwrap();
        let at = "2026-10-20T00:00:00Z";
        ended(&mut w, "task", "attempt", "completed", at).unwrap();
        assert_eq!(
            w.planning.as_ref().unwrap().scopes.as_ref().unwrap()[1].state,
            "closed"
        );
        let before = w.clone();
        ended(
            &mut w,
            "task",
            "attempt",
            "completed",
            "2026-10-20T00:00:01Z",
        )
        .unwrap();
        assert_eq!(w, before);
        assert_eq!(
            ended(&mut w, "task", "attempt", "failed", at)
                .unwrap_err()
                .code(),
            "agent-scope-signal-conflict"
        );
        assert_eq!(w, before);
        assert_eq!(
            ended(&mut w, "task", "foreign", "completed", at)
                .unwrap_err()
                .code(),
            "agent-scope-attempt-changed"
        );
        assert_eq!(w, before);
    }
}
