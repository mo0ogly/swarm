//! Coherent queue observations, separate from eligibility and FIFO acquisition.
use super::{
    mission_supervision_store::{caused, read_transaction},
    RootStore, RootStoreError,
};
use crate::workspace_turn::{self, TurnState, WorkspaceTurnError};
use crate::MissionSupervisionError as E;

/// One saved queue row; decimal ordinal preserves exact SQLite i64 in JSON.
/// Workspace/owner text is recorded metadata, not authenticated process identity.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct MissionWorkspaceTurn {
    pub seq: String,
    pub work: String,
    pub task: String,
    pub workspace: String,
    pub state: &'static str,
    pub agent: String,
    pub requested_at: String,
    pub acquired_at: String,
    pub released_at: String,
}

/// First older overlapping standing row for a waiting task of the requested Work.
/// This is an observation, never a reserved slot, turn acquisition or launch grant.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct MissionWorkspaceBlocker {
    pub task: String,
    pub candidate_seq: String,
    pub older_seq: String,
    pub older_work: String,
    pub older_task: String,
}

/// One read snapshot of a Work and the datastore-wide recorded queue. Blockers
/// concern only this Work; all turns remain visible to explain cross-Work order.
/// This observation writes no table and acquires no lease or turn. FIFO writers
/// separately require an owning lease and atomic admission; this view grants none.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct MissionWorkspaceQueue {
    pub work: String,
    pub revision: i64,
    pub turns: Vec<MissionWorkspaceTurn>,
    pub blockers: Vec<MissionWorkspaceBlocker>,
    pub scope: &'static str,
}

/// Translate every queue failure into the established mission error tree, keeping
/// independent statement-finalize causes visible to existing CLI/Root consumers.
/// No private SQL/path data is formatted here and no rejection becomes empty data.
pub(crate) fn error(cause: WorkspaceTurnError) -> E {
    match cause {
        WorkspaceTurnError::Invalid(code) => E::Refused(code),
        WorkspaceTurnError::Sql(cause) => E::Sql(cause),
        WorkspaceTurnError::Timestamp(cause) => caused("workspace-turn-timestamp-invalid", cause),
        WorkspaceTurnError::Finalize { primary, cleanup } => E::Cleanup {
            primary: Box::new(error(*primary)),
            cleanup: Box::new(E::Sql(cleanup)),
        },
    }
}

impl RootStore {
    /// Read an existing Work and strict global queue in one SQL snapshot, then
    /// calculate older overlapping blockers for its waiting rows. Requires this
    /// opened Root authority, not a pathname supplied by queue metadata. Unknown
    /// Work, malformed rows, SQL/finalize/rollback failures refuse with preserved
    /// causes. No settings adoption, reservation, acquisition, provider, admission
    /// or permission follows; callers own Root close and output/flush handling.
    pub fn mission_workspace_queue(&self, work: &str) -> Result<MissionWorkspaceQueue, E> {
        read_transaction(&self.db, || {
            let saved = crate::read_recorded_work(&self.db, work)
                .map_err(|cause| E::Root(Box::new(RootStoreError::Work(cause))))?;
            let turns = workspace_turn::read(&self.db).map_err(error)?;
            let blockers = workspace_turn::blockers(&turns, work).map_err(error)?;
            Ok(MissionWorkspaceQueue {
                work: saved.id,
                revision: saved.revision,
                turns: turns
                    .into_iter()
                    .map(|turn| MissionWorkspaceTurn {
                        seq: turn.seq.to_string(),
                        work: turn.work_id,
                        task: turn.task_id,
                        workspace: turn.workspace,
                        state: match turn.state {
                            TurnState::Waiting => "waiting",
                            TurnState::Acquired => "acquired",
                            TurnState::Released => "released",
                        },
                        agent: turn.agent_id,
                        requested_at: turn.requested_at,
                        acquired_at: turn.acquired_at,
                        released_at: turn.released_at,
                    })
                    .collect(),
                blockers: blockers
                    .into_iter()
                    .map(|blocker| MissionWorkspaceBlocker {
                        task: blocker.task_id,
                        candidate_seq: blocker.candidate_seq.to_string(),
                        older_seq: blocker.older_seq.to_string(),
                        older_work: blocker.older_work_id,
                        older_task: blocker.older_task_id,
                    })
                    .collect(),
                scope: "recorded-workspace-queue-observation-only-no-acquisition-or-launch",
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A typed compound read/finalize error keeps both causes after translation;
    /// this is an error-tree projection, not an injected SQLite finalize failure.
    #[test]
    fn finalize_translation_keeps_primary_and_secondary() {
        let actual = error(WorkspaceTurnError::Finalize {
            primary: Box::new(WorkspaceTurnError::Invalid(
                "workspace-turn-invalid-fixture",
            )),
            cleanup: rusqlite::Error::InvalidParameterName("fixture".into()),
        });
        match actual {
            E::Cleanup { primary, cleanup } => {
                assert_eq!(primary.code(), "workspace-turn-invalid-fixture");
                assert!(matches!(
                    *cleanup,
                    E::Sql(rusqlite::Error::InvalidParameterName(_))
                ));
            }
            other => panic!("wrong error family: {}", other.code()),
        }
    }
}
