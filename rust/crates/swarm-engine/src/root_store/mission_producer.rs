//! Shared current completed-worker evidence for automatic mission operations.
//! Returns verified data, never a lease, execution grant or task acceptance.
use super::{LocalDirectory,RootStoreError as E};
use crate::{Agent,EngineSettingsDocument,Work};
use rusqlite::Connection;
/// Internal operation contract, never a request flag or operator policy.
/// Report discovery concerns automatic workers; controls/review can consume a
/// manually launched native worker with permission-time physical provenance.
#[derive(Clone,Copy,PartialEq,Eq)]
pub(super) enum OriginRequirement { AutomaticWorker, NativeWorker }
/// Preserve the original relay's typed safe mission cause wrapper, including
/// cleanup children. No supplied error Display is rendered or converted to None.
fn caused(code:&'static str,error:impl std::error::Error+Send+Sync+'static)->E{
    E::Mutation(crate::WorkEngineError::Mission(Box::new(crate::MissionSupervisionError::Cause{code,cause:Box::new(error)})))
}
/// Recheck the current completed task/attempt and original native terminal
/// receipt in the caller's coherent transaction. Automatic producers require
/// their original mission binding; manual producers require their original
/// permission-time directory/connected-DB metadata. Legacy absence refuses.
/// Current mission consent/mode/pause/archive are required for either origin;
/// callers separately owe their actual owning lease and frozen epoch/sequence.
/// Source/schema/SQL/IO errors retain typed causes, no history is upgraded or
/// mutated and no parent actor/model/OS confinement is authenticated.
pub(super) fn completed(db: &Connection, directory: &LocalDirectory, work: &Work, agent_id: &str, settings: &EngineSettingsDocument, requirement: OriginRequirement) -> Result<Agent, E> {
    let row = crate::read_agent(db, agent_id).map_err(E::Agent)?;
    let task = work.tasks.iter().flatten().find(|t| t.id == row.task_id)
        .ok_or(E::Refused("mission-report-task-missing"))?;
    let row = crate::task_report::completed_producer(db, &work.id, task, agent_id)
        .map_err(|e| E::Launch(Box::new(e)))?;
    if requirement==OriginRequirement::AutomaticWorker || row.native_mission.is_some() || row.origin=="conducteur" {
        let binding = super::mission_agent::inspect(db, &row, settings, super::mission_agent::Check::Original)
            .map_err(|e| E::Launch(Box::new(e)))?
            .ok_or(E::Refused("mission-report-automatic-worker-required"))?;
        let actual = super::observe_instance(db, directory)?;
        if binding.root_instance.as_ref() != Some(&actual) { return Err(E::Refused("mission-report-worker-instance-mismatch")); }
    } else {
        super::agent_execution_origin::require(db,directory,agent_id).map_err(|e|E::Launch(Box::new(e)))?;
    }
    crate::agent_completion_journal::observe_terminal(db, directory, agent_id)?;
    super::mission_controls::require_planning_mission_active(db, &work.id)
        .map_err(|e| caused("mission-report-controls", e))?;
    let consent = super::mission_controls::consent(db, &work.id)
        .map_err(|e| caused("mission-report-consent", e))?;
    let controls = super::mission_controls::controls(db, &work.id)
        .map_err(|e| caused("mission-report-controls", e))?;
    if !consent.is_some_and(|v| v.enabled) || !controls.is_some_and(|v| !v.paused && v.autonomy == super::mission_controls::MissionAutonomy::Automatic) {
        return Err(E::Refused("mission-report-automatic-consent-required"));
    }
    Ok(row)
}
