//! Work-scoped historical exchange read. No send/ACK/consumption/visit mutation.
use super::{finished, MissionStatusExchange, RootStore, RootStoreError as E};
impl RootStore {
    /// List all recorded exchanges ordered by created_at then id for an existing
    /// Work. One read-only SQL snapshot binds Work validation and rows; actual
    /// stored nil/empty artifacts and optional Go fields remain distinct.
    /// No content is interpreted as authenticated identity, delivery, permission
    /// or live agent state. Artifacts are declarations, never file reads.
    /// Unknown Work, malformed JSON/types, statement/finalization and rollback
    /// failures retain causes; callers must also check Root close and output.
    /// No new pagination/retention/execution policy or implicit write is added.
    pub fn agent_exchanges(&self, work: &str) -> Result<Vec<MissionStatusExchange>, E> {
        let mut tx = self.db.unchecked_transaction().map_err(E::Sql)?;
        tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
        let result = (|| {
            crate::read_recorded_work(&tx, work).map_err(E::Work)?;
            super::mission_status_read::exchanges(&tx, work)
        })();
        finished(result, tx.rollback().map_err(E::Sql))
    }
}
