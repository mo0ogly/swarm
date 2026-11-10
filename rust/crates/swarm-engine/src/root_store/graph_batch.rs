//! Read-only graph proposal using current settings and one SQL snapshot.
use super::*;
impl RootStore {
    /// Validate a complete native graph proposal without event, Work or draft
    /// writes. Caller owns transport identity and Root binding. Saved policies,
    /// rights, revision/activity and compensation provenance are checked here
    /// and rechecked at apply. Returns a digest-bound proposal or typed original
    /// decode/storage/rollback/release failure; a preview is never permission.
    pub fn preview_graph_batch<E: WorkEnvironment>(
        &self,
        work: &str,
        raw: &[u8],
        environment: &mut E,
    ) -> Result<crate::GraphBatchPreview, RootStoreError> {
        crate::check_command_input_size(raw).map_err(RootStoreError::Input)?;
        let request = crate::decode_graph_batch(raw)
            .map_err(|e| RootStoreError::Mutation(WorkEngineError::Graph(e)))?;
        let reservation = self
            .directory
            .reserve(false, false)
            .map_err(RootStoreError::Storage)?;
        let result = (|| {
            let settings = self.read_settings()?;
            let limits = settings
                .values
                .mutation_limits()
                .map_err(RootStoreError::Settings)?;
            let actor = environment.operator().map_err(RootStoreError::Runtime)?;
            let tx = self
                .db
                .unchecked_transaction()
                .map_err(RootStoreError::Sql)?;
            let result = (|| {
                let w = crate::read_recorded_work(&tx, work)
                    .map_err(|e| WorkEngineError::Guard(crate::MutationGuardError::Work(e)))?;
                crate::graph_batch::prepare(&tx, &w, &request, &limits, &actor, false)
                    .map(|(_, preview)| preview)
            })();
            let result = match (result, tx.rollback()) {
                (Ok(v), Ok(())) => Ok(v),
                (Err(e), Ok(())) => Err(e),
                (Err(primary), Err(cleanup)) => Err(WorkEngineError::Rollback {
                    primary: Box::new(primary),
                    cleanup,
                }),
                (Ok(_), Err(e)) => Err(WorkEngineError::ReplayRollback(e)),
            };
            result.map_err(RootStoreError::Mutation)
        })();
        finished(result, reservation.close().map_err(RootStoreError::Storage))
    }
}
