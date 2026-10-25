//! One explicit worker-intention admission. No watch, provider or role orchestration.
//! Root/lease-bound replay precedes candidate synchronization and resource probes.
//! Typed outcomes preserve known synchronization/admission effects and refusals.
//! Scoped SDK qualification implies neither a complete watch nor global acceptance.
#![cfg(target_os = "linux")]
use super::{RootStore, RootStoreError};
use crate::{
    AgentLaunchError, AgentLaunchOutcome, MissionLease, MissionQueueRegistration,
    MissionSupervisionError as E, MissionWorkerRequest, MissionWorkspaceBlocker, WorkEnvironment,
};
use std::{error::Error, fmt};

/// A caller operation identity and expected Work revision, never a supplied task,
/// profile, permission, retry policy or process identity. The engine selects the
/// task. Schema1 and canonical event IDs are protocol representation invariants.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MissionWorkerDispatchRequest {
    pub schema_version: i64,
    pub event_id: String,
    pub expected_revision: i64,
}
impl MissionWorkerDispatchRequest {
    /// Check representation before Root/SQL work. No mutation or authority follows.
    pub fn validate(&self) -> Result<(), E> {
        if self.schema_version != 1
            || self.expected_revision < 1
            || !swarm_identity_contract::name(self.event_id.as_bytes())
        {
            return Err(E::Refused("mission-dispatch-request-invalid"));
        }
        Ok(())
    }
}

/// A resource exclusion retained with its exact original typed cause or observed
/// FIFO blocker. Neither entry authorizes another launch or refunds consumption.
pub enum MissionDispatchResourceRefusal {
    Fifo {
        task: String,
        blocker: MissionWorkspaceBlocker,
    },
    Occupied {
        task: String,
        cause: RootStoreError,
    },
    Capacity {
        task: String,
        cause: RootStoreError,
    },
}

/// Effects of queue synchronization when a later phase fails. A committed value
/// is retained intact, including permission_refusal and per-task readiness causes.
/// Missing projection after a recorded/uncertain operation is never an empty queue.
pub enum MissionDispatchQueueEffect {
    NotAttempted,
    Committed(MissionQueueRegistration),
    RecordedProjectionUnavailable,
    CommitRollbackConfirmed,
    OutcomeUnknown,
}

/// Known stage of intention admission after a failure. Attempted does not mean
/// unchanged: inspect the exact Root error, including recorded/commit/cleanup
/// variants, then reread this identity before recovery. No new identity is chosen.
pub enum MissionDispatchPreparationEffect {
    NotAttempted,
    ReplayAttempted { event_id: String },
    PreviewAttempted { event_id: String, task_id: String },
    AdmissionAttempted { event_id: String, task_id: String },
}

/// Exact typed operation failure, not a flattened public diagnostic. Consumers
/// must inspect both this cause and all causes retained in synchronized results.
pub enum MissionDispatchCause {
    Mission(Box<E>),
    Root(Box<RootStoreError>),
}
impl MissionDispatchCause {
    /// Retain the original mission error without a flattened or copied diagnostic.
    fn mission(error: E) -> Self {
        Self::Mission(Box::new(error))
    }
    /// Retain the actual Root error, including every effect/cleanup wrapper.
    fn root(error: RootStoreError) -> Self {
        Self::Root(Box::new(error))
    }
}
impl fmt::Debug for MissionDispatchCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl fmt::Display for MissionDispatchCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mission(error) => f.write_str(error.code()),
            Self::Root(error) => f.write_str(error.code()),
        }
    }
}
impl Error for MissionDispatchCause {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(match self {
            Self::Mission(error) => error.as_ref(),
            Self::Root(error) => error.as_ref(),
        })
    }
}

/// Failure after any stage, preserving known synchronized effects and the exact
/// preparation identity. source exposes only the primary typed cause; synchronized
/// per-task causes remain explicit fields. Close/flush/heartbeat callers must keep
/// this value alongside their independent failures and never retry by a new ID.
pub struct MissionWorkerDispatchFailure {
    pub phase: &'static str,
    pub queue: MissionDispatchQueueEffect,
    pub preparation: MissionDispatchPreparationEffect,
    pub resources: Vec<MissionDispatchResourceRefusal>,
    pub cause: MissionDispatchCause,
}
impl fmt::Debug for MissionWorkerDispatchFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl fmt::Display for MissionWorkerDispatchFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "mission-dispatch-{}: {}", self.phase, self.cause)
    }
}
impl Error for MissionWorkerDispatchFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.cause)
    }
}

/// One queued intention or an explicit no-admission result. NoAdmission can
/// include committed waiting-claim cleanup, permission withdrawal, readiness
/// refusals and resource exclusions; it is not a successful empty dispatch.
/// Prepared/Replayed never imply process_started, accepted or authenticated actor.
pub enum MissionWorkerDispatchOutcome {
    Prepared {
        queue: MissionQueueRegistration,
        intention: Box<AgentLaunchOutcome>,
        resources: Vec<MissionDispatchResourceRefusal>,
    },
    Replayed {
        queue: Option<MissionQueueRegistration>,
        intention: Box<AgentLaunchOutcome>,
        resources: Vec<MissionDispatchResourceRefusal>,
    },
    NoAdmission {
        queue: MissionQueueRegistration,
        resources: Vec<MissionDispatchResourceRefusal>,
    },
}

/// Classify ONLY direct resource refusals from the common preview consumer.
/// The consumer returns a direct Refused only after clean rollback/settings close;
/// composite, storage, commit, recorded and unknown wrappers always stop this pass.
/// No Error::source traversal is used to hide a secondary cleanup failure.
fn clean_resource_refusal(error: &RootStoreError) -> bool {
    matches!(error, RootStoreError::Launch(cause) if matches!(cause.as_ref(),
        AgentLaunchError::Refused("agent-launch-workspace-busy" | "agent-launch-integration-busy")))
}

/// Only the common fence's direct typed capacity refusal is a global resource
/// stop. Exact downcasts retain the whole wrapper shape; cleanup/commit/recorded
/// wrappers never match even if they contain a capacity refusal as a cause.
fn clean_capacity_refusal(error: &RootStoreError) -> bool {
    let RootStoreError::Launch(cause) = error else {
        return false;
    };
    let AgentLaunchError::Cause {
        code: "agent-launch-mission-fence",
        cause,
    } = cause.as_ref()
    else {
        return false;
    };
    matches!(cause.downcast_ref::<crate::WorkEngineError>(), Some(crate::WorkEngineError::Mission(cause))
        if matches!(cause.as_ref(), E::Refused("mission-agent-slots-occupied")))
}

/// Retain only explicit queue effect knowledge from the original writer contract.
/// Other failures remain unknown rather than being guessed to have rolled back.
fn failed_queue_effect(error: &E) -> MissionDispatchQueueEffect {
    match error {
        E::Recorded { .. } => MissionDispatchQueueEffect::RecordedProjectionUnavailable,
        E::Commit {
            rollback_confirmed: true,
            cleanup: None,
            ..
        } => MissionDispatchQueueEffect::CommitRollbackConfirmed,
        _ => MissionDispatchQueueEffect::OutcomeUnknown,
    }
}

/// Validate the committed synchronization observation BEFORE any NoAdmission
/// branch, including permission cleanup or an empty ready set. Stale snapshots
/// remain errors with their real committed queue effects, never success-idle.
fn synchronized_revision(
    queue: &MissionQueueRegistration,
    work: &str,
    revision: i64,
) -> Result<(), E> {
    if queue.work != work || queue.revision != revision {
        return Err(E::Refused("revision_conflict"));
    }
    Ok(())
}

impl RootStore {
    /// Select and prepare at most one actual queued worker under a real lease.
    /// Arity is the explicit command contract, not an operator concurrency cap.
    /// Replay lookup/fencing occurs BEFORE synchronization or new selection; a
    /// changed historical profile refuses without starting a replacement intent.
    /// All readiness contenders synchronize before slots/occupancy/FIFO filtering.
    /// Only then does the common resource preview probe admission. Managed tasks
    /// use rollback-only resource observation on the future path, creating no
    /// copy/Agent and probing no nonexistent host workspace; actual admission
    /// repeats these guards and performs full copy/host inspection. Clean direct
    /// occupancy refusals can yield to a disjoint lower-ranked candidate; any
    /// compound/unknown failure stops with all earlier committed effects retained.
    /// A direct capacity refusal stops the whole pass instead of trying other tasks.
    /// Factory repeats actual current guards before atomic Agent/turn admission.
    ///
    /// No provider runs, no watch/planner/reviewer/judge is started, and no task is
    /// accepted. The caller owns actual heartbeat maintenance and lease/Root close,
    /// output/flush and recovery of uncertain outcomes by this SAME event identity.
    /// No new retry/deadline, policy fallback or artificial slot count is introduced.
    /// Operation IDs share the automatic Agent.start namespace. Replaying a real
    /// original determines its historical task; this does not cryptographically
    /// prove it came from this selector or create fresh permission. Manual/foreign
    /// Work/revision conflicts must refuse in the narrow replay helper, never sync.
    pub fn dispatch_mission_worker<Ev: WorkEnvironment>(
        &mut self,
        lease: &MissionLease,
        request: &MissionWorkerDispatchRequest,
        environment: &mut Ev,
    ) -> Result<MissionWorkerDispatchOutcome, Box<MissionWorkerDispatchFailure>> {
        let early = |phase, cause| MissionWorkerDispatchFailure {
            phase,
            queue: MissionDispatchQueueEffect::NotAttempted,
            preparation: MissionDispatchPreparationEffect::NotAttempted,
            resources: Vec::new(),
            cause,
        };
        request
            .validate()
            .map_err(|e| early("request", MissionDispatchCause::mission(e)))?;
        let work = super::mission_supervision_store::bound_mission_work(self, lease)
            .map_err(|e| early("binding", MissionDispatchCause::mission(e)))?
            .to_owned();
        let replay = self
            .replay_mission_dispatch_original(lease, request, environment)
            .map_err(|cause| MissionWorkerDispatchFailure {
                phase: "replay",
                queue: MissionDispatchQueueEffect::NotAttempted,
                preparation: MissionDispatchPreparationEffect::ReplayAttempted {
                    event_id: request.event_id.clone(),
                },
                resources: Vec::new(),
                cause: MissionDispatchCause::root(cause),
            })?;
        if let Some(intention) = replay {
            return Ok(MissionWorkerDispatchOutcome::Replayed {
                queue: None,
                intention: Box::new(intention),
                resources: Vec::new(),
            });
        }
        let current = self
            .read_work(&work)
            .map_err(|e| early("work", MissionDispatchCause::root(e)))?;
        if current.revision != request.expected_revision {
            return Err(Box::new(early(
                "revision",
                MissionDispatchCause::mission(E::Refused("revision_conflict")),
            )));
        }
        let queue = self.sync_mission_workspace_queue(lease).map_err(|cause| {
            MissionWorkerDispatchFailure {
                phase: "synchronization",
                queue: failed_queue_effect(&cause),
                preparation: MissionDispatchPreparationEffect::NotAttempted,
                resources: Vec::new(),
                cause: MissionDispatchCause::mission(cause),
            }
        })?;
        let mut resources = Vec::new();
        if let Err(cause) = synchronized_revision(&queue, &work, request.expected_revision) {
            return Err(Box::new(MissionWorkerDispatchFailure {
                phase: "synchronized-revision",
                queue: MissionDispatchQueueEffect::Committed(queue),
                preparation: MissionDispatchPreparationEffect::NotAttempted,
                resources,
                cause: MissionDispatchCause::mission(cause),
            }));
        }
        if queue.permission_refusal.is_some() || queue.ready.is_empty() {
            return Ok(MissionWorkerDispatchOutcome::NoAdmission { queue, resources });
        }
        let view = match self.mission_workspace_queue(&work) {
            Ok(view) => view,
            Err(cause) => {
                return Err(Box::new(MissionWorkerDispatchFailure {
                    phase: "queue-observation",
                    queue: MissionDispatchQueueEffect::Committed(queue),
                    preparation: MissionDispatchPreparationEffect::NotAttempted,
                    resources,
                    cause: MissionDispatchCause::mission(cause),
                }))
            }
        };
        if view.work != work || view.revision != request.expected_revision {
            return Err(Box::new(MissionWorkerDispatchFailure {
                phase: "queue-observation-revision",
                queue: MissionDispatchQueueEffect::Committed(queue),
                preparation: MissionDispatchPreparationEffect::NotAttempted,
                resources,
                cause: MissionDispatchCause::mission(E::Refused("revision_conflict")),
            }));
        }
        for task in &queue.ready {
            if let Some(blocker) = view.blockers.iter().find(|blocker| &blocker.task == task) {
                resources.push(MissionDispatchResourceRefusal::Fifo {
                    task: task.clone(),
                    blocker: blocker.clone(),
                });
                continue;
            }
            let chosen = MissionWorkerRequest {
                schema_version: 1,
                event_id: request.event_id.clone(),
                expected_revision: request.expected_revision,
                task_id: task.clone(),
            };
            match self.preview_mission_worker_resources(lease, &chosen, environment) {
                Ok(_) => {}
                Err(cause) if clean_capacity_refusal(&cause) => {
                    resources.push(MissionDispatchResourceRefusal::Capacity {
                        task: task.clone(),
                        cause,
                    });
                    return Ok(MissionWorkerDispatchOutcome::NoAdmission { queue, resources });
                }
                Err(cause) if clean_resource_refusal(&cause) => {
                    resources.push(MissionDispatchResourceRefusal::Occupied {
                        task: task.clone(),
                        cause,
                    });
                    continue;
                }
                Err(cause) => {
                    return Err(Box::new(MissionWorkerDispatchFailure {
                        phase: "resource-preview",
                        preparation: MissionDispatchPreparationEffect::PreviewAttempted {
                            event_id: request.event_id.clone(),
                            task_id: task.clone(),
                        },
                        queue: MissionDispatchQueueEffect::Committed(queue),
                        resources,
                        cause: MissionDispatchCause::root(cause),
                    }))
                }
            }
            let intention = match self.prepare_mission_worker(lease, &chosen, false, environment) {
                Ok(intention) => intention,
                Err(cause) => {
                    return Err(Box::new(MissionWorkerDispatchFailure {
                        phase: "admission",
                        preparation: MissionDispatchPreparationEffect::AdmissionAttempted {
                            event_id: request.event_id.clone(),
                            task_id: task.clone(),
                        },
                        queue: MissionDispatchQueueEffect::Committed(queue),
                        resources,
                        cause: MissionDispatchCause::root(cause),
                    }))
                }
            };
            return Ok(if intention.created {
                MissionWorkerDispatchOutcome::Prepared {
                    queue,
                    intention: Box::new(intention),
                    resources,
                }
            } else {
                MissionWorkerDispatchOutcome::Replayed {
                    queue: Some(queue),
                    intention: Box::new(intention),
                    resources,
                }
            });
        }
        Ok(MissionWorkerDispatchOutcome::NoAdmission { queue, resources })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Data-only stale/cleanup projections do not fabricate real registrations.
    #[test]
    fn stale_sync_is_refused_even_for_permission_cleanup_or_empty_ready() {
        for withdrawn in [false, true] {
            let mut queue = MissionQueueRegistration {
                work: "work".into(),
                revision: 2,
                ready: Vec::new(),
                refused: Vec::new(),
                permission_refusal: withdrawn.then_some(E::Refused("planning-mission-paused")),
            };
            assert!(synchronized_revision(&queue, "work", 1).is_err());
            assert!(synchronized_revision(&queue, "other", 2).is_err());
            assert!(synchronized_revision(&queue, "work", 2).is_ok());
            queue.ready.push("task".into());
            assert!(synchronized_revision(&queue, "work", 1).is_err());
        }
    }
    /// Pure wire checks and classification, not a SQL rollback or integration proof.
    #[test]
    fn exact_resource_refusals_never_hide_cleanup_or_commit() {
        let direct = || {
            RootStoreError::Launch(Box::new(AgentLaunchError::Refused(
                "agent-launch-workspace-busy",
            )))
        };
        assert!(clean_resource_refusal(&direct()));
        assert!(!clean_resource_refusal(&RootStoreError::Launch(Box::new(
            AgentLaunchError::Cleanup {
                primary: Box::new(AgentLaunchError::Refused("agent-launch-workspace-busy")),
                cleanup: Box::new(AgentLaunchError::Refused("fixture-rollback-failed")),
            }
        ))));
        assert!(!clean_resource_refusal(
            &RootStoreError::LifecycleRecorded {
                primary: Box::new(direct())
            }
        ));
        assert!(!clean_resource_refusal(&RootStoreError::Launch(Box::new(
            AgentLaunchError::Commit {
                cause: Box::new(AgentLaunchError::Refused("agent-launch-workspace-busy")),
                rollback_confirmed: true,
                cleanup: None,
            }
        ))));
        let mut request = MissionWorkerDispatchRequest {
            schema_version: 1,
            event_id: "operation".into(),
            expected_revision: 1,
        };
        assert!(request.validate().is_ok());
        request.expected_revision = 0;
        assert!(request.validate().is_err());
        request.expected_revision = 1;
        request.schema_version = 2;
        assert!(request.validate().is_err());
        request.schema_version = 1;
        request.event_id.clear();
        assert!(request.validate().is_err());
        assert!(
            serde_json::from_value::<MissionWorkerDispatchRequest>(serde_json::json!({
                "schema_version":1,"event_id":"operation","expected_revision":1,"permit":true
            }))
            .is_err()
        );
    }

    /// Capacity and transaction variants are typed fixture projections only;
    /// this verifies classification, never claims an actual SQL rollback.
    #[test]
    fn capacity_stops_globally_and_unknown_queue_effects_remain_unknown() {
        let capacity = || {
            RootStoreError::Launch(Box::new(AgentLaunchError::caused(
                "agent-launch-mission-fence",
                crate::WorkEngineError::Mission(Box::new(E::Refused(
                    "mission-agent-slots-occupied",
                ))),
            )))
        };
        assert!(clean_capacity_refusal(&capacity()));
        assert!(!clean_resource_refusal(&capacity()));
        assert!(!clean_capacity_refusal(
            &RootStoreError::LifecycleRecorded {
                primary: Box::new(capacity())
            }
        ));
        assert!(!clean_capacity_refusal(&RootStoreError::Launch(Box::new(
            AgentLaunchError::Refused("mission-agent-slots-occupied")
        ))));
        assert!(matches!(
            failed_queue_effect(&E::Recorded {
                primary: Box::new(E::Refused("fixture-close"))
            }),
            MissionDispatchQueueEffect::RecordedProjectionUnavailable
        ));
        let commit = |rollback_confirmed| E::Commit {
            cause: Box::new(rusqlite::Error::InvalidQuery),
            rollback_confirmed,
            cleanup: None,
        };
        assert!(matches!(
            failed_queue_effect(&commit(true)),
            MissionDispatchQueueEffect::CommitRollbackConfirmed
        ));
        assert!(matches!(
            failed_queue_effect(&commit(false)),
            MissionDispatchQueueEffect::OutcomeUnknown
        ));
        assert!(matches!(
            failed_queue_effect(&E::Cleanup {
                primary: Box::new(E::Refused("fixture-primary")),
                cleanup: Box::new(E::Sql(rusqlite::Error::InvalidQuery)),
            }),
            MissionDispatchQueueEffect::OutcomeUnknown
        ));
    }
}
