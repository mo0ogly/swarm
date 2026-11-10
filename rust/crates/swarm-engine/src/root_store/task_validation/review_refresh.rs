//! Explicit stale-proof renewal, never a new verdict or permission to execute.
use super::{journal, managed_execution, publication, runtime::Receipt, store};
use crate::{
    root_store::{
        native_transaction::{self, Change},
        RootStore, RootStoreError as E,
    },
    AgentLaunchError as A,
};
use serde::Serialize;

/// Recorded archive/clear operation. Original jobs, receipts, attempts, call
/// counts and charges remain; the task needs new controls and a new review.
#[derive(Serialize)]
pub struct ReviewRefreshOutcome {
    pub review_id: String,
    pub replayed: bool,
    pub provider_started: bool,
    pub task_accepted: bool,
    pub work: crate::Work,
}
impl RootStore {
    /// Archive a completed managed opinion only after an initial input changed,
    /// or its historical frame lacks that binding. The strict native task/event/
    /// revision envelope supplies no actor, hash or permission. Requires an idle
    /// submitted task and verified original reviewer/control custody. Fresh R1,
    /// invalid IO, active jobs and conflicting events refuse without mutation.
    /// Rechecks change inside the protected writer; preserves every historical
    /// receipt and consumption. Exact replay returns the retained archive result,
    /// never a provider/worker/control call. Late errors retain known Work state.
    pub fn refresh_task_review(&self, work: &str, raw: &[u8]) -> Result<ReviewRefreshOutcome, E> {
        if !self.writable {
            return Err(E::Refused("root-read-only"));
        }
        let request = super::decode(raw).map_err(|e| E::Launch(Box::new(e)))?;
        let mut known = None;
        let result = super::super::material_path::with_material_operation(self, || {
            crate::agent_storage_retry::execute_borrowed(
                &self.db,
                &self.directory,
                false,
                |_, _| {
                    let value = native_transaction::apply(self, |tx| {
                        let mut current = crate::read_recorded_work(tx, work).map_err(E::Work)?;
                        let old: Option<(String, Vec<u8>)> = crate::agent_launch::query(
                            tx,
                            "SELECT kind,request FROM events WHERE id=? AND work_id=?",
                            rusqlite::params![request.event_id, work],
                            |rows| rows.next()?.map(|r| Ok((r.get(0)?, r.get(1)?))).transpose(),
                        )
                        .map_err(|e| E::Launch(Box::new(e)))?;
                        if let Some((kind, body)) = old {
                            let value: serde_json::Value =
                                serde_json::from_slice(&body).map_err(|e| {
                                    E::Launch(Box::new(A::caused("review-refresh-event", e)))
                                })?;
                            if kind != "task.review-refresh"
                                || value["request"]
                                    != serde_json::to_value(&request).map_err(|e| {
                                        E::Launch(Box::new(A::caused("review-refresh-request", e)))
                                    })?
                            {
                                return Err(E::Refused("review-refresh-event-conflict"));
                            }
                            let id = value["review_id"]
                                .as_str()
                                .ok_or(E::Refused("review-refresh-event-invalid"))?
                                .to_owned();
                            let original = self.recorded_native_review(tx, work, &id)?;
                            let task = current
                                .tasks
                                .iter()
                                .flatten()
                                .find(|t| t.id == request.task_id)
                                .ok_or(E::Refused("review-refresh-task-required"))?;
                            if !task
                                .previous_reviews
                                .iter()
                                .flatten()
                                .any(|r| r == &original.review)
                            {
                                return Err(E::Refused("review-refresh-history-required"));
                            }
                            return Ok(Change::Observe(ReviewRefreshOutcome {
                                review_id: id,
                                replayed: true,
                                provider_started: false,
                                task_accepted: false,
                                work: current,
                            }));
                        }
                        if current.revision != request.expected_revision {
                            return Err(E::Refused("review-refresh-revision-conflict"));
                        }
                        super::require_validation_idle(tx, work, &request.task_id)
                            .map_err(|e| E::Launch(Box::new(e)))?;
                        Self::require_idle_task_review(tx, work, &request.task_id)
                            .map_err(|e| E::Launch(Box::new(e)))?;
                        let task = current
                            .tasks
                            .iter()
                            .flatten()
                            .find(|t| t.id == request.task_id)
                            .ok_or(E::Refused("review-refresh-task-required"))?;
                        if task.status != "submitted"
                            || !current
                                .planning
                                .as_deref()
                                .is_some_and(|p| p.repository.is_some())
                        {
                            return Err(E::Refused("review-refresh-managed-submission-required"));
                        }
                        let review = task
                            .independent_review
                            .as_deref()
                            .ok_or(E::Refused("review-refresh-opinion-required"))?
                            .clone();
                        if review.state != "passed" {
                            return Err(E::Refused("review-refresh-completed-opinion-required"));
                        }
                        let original = self.recorded_native_review(tx, work, &review.id)?;
                        if original.review != review
                            || original.task != task.id
                            || !original.verdict_applicable
                        {
                            return Err(E::Refused("review-refresh-original-changed"));
                        }
                        let validation = task
                            .automatic_validation
                            .as_deref()
                            .ok_or(E::Refused("review-refresh-controls-required"))?;
                        let id =
                            store::recorded_id_for_receipt(tx, work, &task.id, &validation.receipt)
                                .map_err(|e| E::Launch(Box::new(e)))?
                                .ok_or(E::Refused("review-refresh-controls-required"))?;
                        let (receipt, hash): (Receipt, String) =
                            journal::read(self, &journal::name("job", &id))?;
                        let job = publication::recorded_original(self, &receipt, &hash)?;
                        if hash != original.validation_receipt_sha256
                            || !publication::all_passed(&receipt)
                        {
                            return Err(E::Refused("review-refresh-control-original-changed"));
                        }
                        let candidate = receipt
                            .managed_candidate
                            .as_ref()
                            .ok_or(E::Refused("review-refresh-managed-original-required"))?;
                        let initial = managed_execution::retained_inputs(self, &job, candidate)?;
                        if let Some(initial) = &initial {
                            let mut expected = receipt.artifacts.clone();
                            expected.remove(&review.report);
                            if &expected != initial {
                                return Err(E::Refused("review-refresh-input-original-changed"));
                            }
                            if !super::input_inventory::changed(
                                &self.root,
                                std::path::Path::new(&candidate.workspace),
                                initial,
                            )? {
                                return Err(E::Refused("review-refresh-original-still-applicable"));
                            }
                        }
                        let id = review.id.clone();
                        let target = current
                            .tasks
                            .as_mut()
                            .and_then(|t| t.iter_mut().find(|t| t.id == request.task_id))
                            .ok_or(E::Refused("review-refresh-task-required"))?;
                        target.previous_reviews.get_or_insert_default().push(review);
                        target.independent_review = None;
                        target.automatic_validation = None;
                        target.gate = None;
                        target.evidence_stale_reason =
                            "managed-inputs-require-new-controls-and-review".into();
                        current.revision = current
                            .revision
                            .checked_add(1)
                            .ok_or(E::Refused("review-refresh-revision-overflow"))?;
                        current.updated = crate::system_timestamp().map_err(E::Runtime)?;
                        let body=crate::encode_model(&serde_json::json!({"request":request,"review_id":id,"historical_unbound":initial.is_none()})).map_err(E::Encode)?;
                        let staged = crate::stage_recorded_change_with_hook(
                            tx,
                            crate::RecordedChange {
                                work: &current,
                                expected_revision: request.expected_revision,
                                event: &request.event_id,
                                kind: "task.review-refresh",
                                request: &body,
                            },
                            |_, _| Ok(()),
                        )
                        .map_err(|e| E::Launch(Box::new(A::caused("review-refresh-stage", e))))?;
                        if !matches!(staged, crate::RecordedChangeOutcome::Staged) {
                            return Err(E::Refused("review-refresh-stage-conflict"));
                        }
                        Ok(Change::Mutate(ReviewRefreshOutcome {
                            review_id: id,
                            replayed: false,
                            provider_started: false,
                            task_accepted: false,
                            work: current,
                        }))
                    })?;
                    known = Some((value.work.id.clone(), value.work.revision));
                    Ok(value)
                },
            )
        });
        result.map_err(|error| match known {
            Some((work, revision)) => E::WorkRecorded {
                primary: Box::new(error),
                work,
                revision,
            },
            None => error,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Use an actually reviewed owned fixture after input drift. A connection-
    /// local SQL fault must preserve Work/review/controls and never claim the
    /// rolled-back revision as recorded. This SDK creates no positive evidence.
    #[test]
    #[ignore = "requires genuine isolated stale-input fixture"]
    fn refresh_rollback_preserves_original_review_and_recorded_state() {
        let path = std::path::PathBuf::from(std::env::var_os("SWARM_GIETH_REFRESH_ROOT").unwrap());
        let work = std::env::var("SWARM_GIETH_REFRESH_WORK").unwrap();
        let raw = std::fs::read(std::env::var_os("SWARM_GIETH_REFRESH_REQUEST").unwrap()).unwrap();
        let root = RootStore::open(&path, true).unwrap();
        let before = root.read_work(&work).unwrap();
        root.db.execute_batch("CREATE TEMP TRIGGER gi_refresh_rollback BEFORE INSERT ON main.events BEGIN SELECT RAISE(ABORT,'owned-refresh-rollback'); END;").unwrap();
        let error = root
            .refresh_task_review(&work, &raw)
            .err()
            .expect("SQL fault must refuse");
        assert!(!matches!(error, E::WorkRecorded { .. }));
        assert_eq!(root.read_work(&work).unwrap(), before);
        root.db
            .execute_batch("DROP TRIGGER gi_refresh_rollback")
            .unwrap();
        root.close().unwrap();
        println!("GI_REFRESH_ROLLBACK_PRESERVED");
    }
}
