//! Read-only classification of retained intentions before any future dispatch.
//! A snapshot is an observation, never liveness, permission or a recovery action.
use super::{
    bound, caused, claim_revision, finish, owned_heartbeat, read_transaction, root, MissionLease,
    RootStore, RootStoreError, E,
};
use crate::{Agent, AgentLaunchError, EngineSettingsDocument, MissionLeaseClock, SettingCount};
use std::{error::Error, fmt, time::Duration};

/// Coherent retained history under the actual Root and present lease. No entry
/// authorizes execution. Ordering is retained row order, not relaunch priority.
/// IDs are for trusted local callers, not an approved public/HTTP document.
pub struct MissionWorkerContinuationObservation {
    pub work: String,
    pub revision: i64,
    pub settings_revision: SettingCount,
    pub generation: i64,
    pub observed_at: String,
    pub entries: Vec<MissionWorkerContinuationEntry>,
}
/// Minimal coordinates and an observation. No prompt, token, raw Agent or process
/// stamp is copied. Neither this type nor its IDs can construct an owning handle.
pub struct MissionWorkerContinuationEntry {
    pub agent_id: String,
    pub task_id: String,
    pub state: MissionWorkerContinuationState,
}
/// Retained lifecycle classifications, never a permit or an inferred live/dead
/// process. Refused retains the whole typed cause, including historical provenance.
pub enum MissionWorkerContinuationState {
    /// Historical proof and current physical instance match; live consent,
    /// capacity, current profiles and the final private gate remain unchecked.
    QueuedUnregisteredObserved {
        generation: i64,
        current_lease_generation: bool,
    },
    /// Same queued observation with stop intent. No cancellation was performed.
    StopRequestedUnregisteredObserved {
        generation: i64,
        current_lease_generation: bool,
    },
    /// Registration exists in storage, including a retained stopping incident.
    /// An opaque native result is not validated as terminal proof; no OS process
    /// liveness, resource release or recovery authority is established.
    RegisteredObserved,
    /// Complete matching private receipt and terminal ownership were read.
    TerminalVerified {
        receipt_sha256: String,
        output_complete: bool,
        provider_may_have_started: bool,
    },
    /// Known manual origin or a proved automatic non-worker role; no launch scope.
    OutsideAutomaticWorkerScope,
    /// Semantic refusal is kept rather than filtered into an empty success.
    Refused {
        phase: &'static str,
        cause: RootStoreError,
    },
}
/// Observation progress after failure. All returned entries are unusable for an
/// action, including when only a final physical recheck or cleanup failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissionContinuationProgress {
    NotStarted,
    PartialNotUsable,
    /// All rows were read, but rollback, settings reservation close or the final
    /// physical recheck failed. The phase and retained cause identify which;
    /// this variant alone never attributes the failure to cleanup.
    CompleteButUnusable,
}
/// Exact failure and prior observations, never proof of no concurrent mutation.
/// source follows the primary E; callers must also traverse its cleanup branches
/// and every retained Refused entry. No automatic retry or recovery is implied.
pub struct MissionContinuationFailure {
    pub phase: &'static str,
    pub progress: MissionContinuationProgress,
    pub entries: Vec<MissionWorkerContinuationEntry>,
    pub cause: E,
}
impl fmt::Debug for MissionContinuationFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl fmt::Display for MissionContinuationFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "mission-continuation-failed: {}", self.cause.code())
    }
}
impl Error for MissionContinuationFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.cause)
    }
}

/// Only exact semantic refusals are entries. SQL, IO, decoder, composite cleanup
/// and commit failures stop the snapshot; traversing source to find a refusal
/// would hide independent failure. Keep the original wrapper intact either way.
fn semantic_refusal(error: &RootStoreError) -> bool {
    match error {
        RootStoreError::Refused(_) => true,
        RootStoreError::Agent(crate::AgentStoreError::Refused(_)) => true,
        RootStoreError::Launch(error) => match error.as_ref() {
            AgentLaunchError::Refused(_) => true,
            AgentLaunchError::Cause {
                code: "agent-mission-provenance",
                cause,
            } => matches!(cause.downcast_ref::<E>(), Some(E::Refused(_))),
            _ => false,
        },
        _ => false,
    }
}
/// Preserve a direct semantic refusal per intention, or propagate a failure that
/// invalidates the whole observation. No unknown wrapper is treated as harmless.
fn entry_result(
    phase: &'static str,
    result: Result<MissionWorkerContinuationState, RootStoreError>,
) -> Result<MissionWorkerContinuationState, E> {
    match result {
        Ok(value) => Ok(value),
        Err(cause) if semantic_refusal(&cause) => {
            Ok(MissionWorkerContinuationState::Refused { phase, cause })
        }
        Err(cause) => Err(root(cause)),
    }
}

/// Pure structural lifecycle check. No PID probe, freshness or time heuristic is
/// used. Registered phases require a nonempty Go RFC3339 heartbeat; a malformed
/// timestamp retains its parse cause and invalidates the whole snapshot rather
/// than being downgraded to a semantic per-entry refusal. Legacy registration
/// gaps and contradictory fields refuse. Stopping may retain an opaque native
/// result without proving its contents, terminal completion or recovery rights.
fn lifecycle(agent: &Agent) -> Result<bool, RootStoreError> {
    let refuse = || RootStoreError::Refused("mission-continuation-lifecycle-incoherent");
    if agent.has_unregistered_claim() || !matches!(agent.desired.as_str(), "" | "stop") {
        return Err(refuse());
    }
    if agent.status == "queued" {
        if agent.supervisor_pid != 0
            || !agent.supervisor_identity.is_empty()
            || agent.child_pid != 0
            || !agent.child_identity.is_empty()
            || !agent.heartbeat.is_empty()
            || !agent.ended.is_empty()
            || agent.exit_code.is_some()
            || agent.native_execution_result.is_some()
        {
            return Err(refuse());
        }
        return Ok(true);
    }
    if matches!(agent.status.as_str(), "starting" | "running" | "stopping") {
        if agent.supervisor_pid < 1
            || agent.supervisor_identity.is_empty()
            || agent.host.is_empty()
            || agent.heartbeat.is_empty()
            || !agent.ended.is_empty()
            || (agent.status != "stopping" && agent.native_execution_result.is_some())
            || agent.exit_code.is_some()
            || agent.child_pid < 0
            || (agent.child_pid == 0) != agent.child_identity.is_empty()
            || matches!(agent.status.as_str(), "running" | "stopping") && agent.child_pid < 1
        {
            return Err(refuse());
        }
        swarm_wire_json::go_rfc3339::parse(&agent.heartbeat).map_err(|error| {
            RootStoreError::Launch(Box::new(AgentLaunchError::caused(
                "mission-continuation-heartbeat-invalid",
                error,
            )))
        })?;
        return Ok(false);
    }
    if matches!(
        agent.status.as_str(),
        "completed" | "failed" | "interrupted" | "cancelled"
    ) {
        return Ok(false);
    }
    Err(RootStoreError::Refused(
        "mission-continuation-status-unsupported",
    ))
}

/// Read canonical ownership for all origins before scope filtering. Automatic
/// Original proof precedes classification; queued additionally requires Binding4
/// actual-instance comparison. BeforeGrant and resource admission are not run.
fn observe_entry(
    store: &RootStore,
    lease: &MissionLease,
    settings: &EngineSettingsDocument,
    agent: &Agent,
) -> Result<MissionWorkerContinuationState, E> {
    crate::agent_store::verify_agent_ownership(&store.db, agent)
        .map_err(|error| root(RootStoreError::Agent(error)))?;
    let result = (|| {
        if matches!(agent.origin.as_str(), "" | "operateur") && agent.native_mission.is_none() {
            return Ok(MissionWorkerContinuationState::OutsideAutomaticWorkerScope);
        }
        if agent.origin != "conducteur" {
            return Err(RootStoreError::Refused(
                "mission-continuation-origin-unsupported",
            ));
        }
        let queued = lifecycle(agent)?;
        let binding = super::super::mission_agent::inspect(
            &store.db,
            agent,
            settings,
            super::super::mission_agent::Check::Original,
        )
        .map_err(|error| RootStoreError::Launch(Box::new(error)))?
        .ok_or(RootStoreError::Refused("mission-agent-binding-required"))?;
        if agent.role != "worker" {
            return Ok(MissionWorkerContinuationState::OutsideAutomaticWorkerScope);
        }
        if queued {
            super::super::mission_agent::inspect(
                &store.db,
                agent,
                settings,
                super::super::mission_agent::Check::BeforeClaim(&store.directory),
            )
            .map_err(|error| RootStoreError::Launch(Box::new(error)))?
            .ok_or(RootStoreError::Refused("mission-agent-binding-required"))?;
            let current_lease_generation =
                binding.generation == lease.generation && binding.conductor == lease.conductor;
            return Ok(if agent.desired == "stop" {
                MissionWorkerContinuationState::StopRequestedUnregisteredObserved {
                    generation: binding.generation,
                    current_lease_generation,
                }
            } else {
                MissionWorkerContinuationState::QueuedUnregisteredObserved {
                    generation: binding.generation,
                    current_lease_generation,
                }
            });
        }
        if agent.is_active() {
            return Ok(MissionWorkerContinuationState::RegisteredObserved);
        }
        if agent.supervisor_pid < 1 {
            return Err(RootStoreError::Refused(
                "mission-continuation-terminal-unregistered-unproved",
            ));
        }
        let observed = crate::agent_completion_journal::observe_terminal(
            &store.db,
            &store.directory,
            &agent.id,
        )?;
        Ok(MissionWorkerContinuationState::TerminalVerified {
            receipt_sha256: observed.receipt_sha256,
            output_complete: observed.output_complete,
            provider_may_have_started: observed.provider_may_have_started,
        })
    })();
    entry_result("intention", result)
}

impl RootStore {
    /// Observe retained intentions in one SQL snapshot under this actual Root and
    /// current-process lease. The lease's SQL owner, frozen policy/clock and claim
    /// history must match. Settings reservation spans reads and checked rollback;
    /// its saved busy timeout applies, without a new retry or business deadline.
    /// A final physical Root check occurs after SQL/settings cleanup. It cannot
    /// reserve future rights or establish filesystem/SQL atomicity or liveness.
    ///
    /// No lifecycle/heartbeat/cycle/FIFO writes, signals, provider, schema repair,
    /// new files, retention or acceptance occur. Shared settings locking and
    /// SQLite reads may have their documented connection/recovery effects. Caller
    /// still owes lease.close and Root.close. Errors retain partial observations
    /// as unusable and all cleanup causes; none becomes an empty success.
    pub fn observe_mission_worker_continuation(
        &self,
        lease: &MissionLease,
    ) -> Result<MissionWorkerContinuationObservation, Box<MissionContinuationFailure>> {
        let now = crate::system_timestamp().map_err(|error| {
            Box::new(MissionContinuationFailure {
                phase: "clock",
                progress: MissionContinuationProgress::NotStarted,
                entries: Vec::new(),
                cause: caused("mission-continuation-clock", error),
            })
        })?;
        self.observe_continuation_at(lease, &now)
    }

    /// Internal deterministic-clock adapter. The supplied timestamp is only a
    /// clock observation, never an authentication input or a replacement lease.
    fn observe_continuation_at(
        &self,
        lease: &MissionLease,
        now: &str,
    ) -> Result<MissionWorkerContinuationObservation, Box<MissionContinuationFailure>> {
        let mut entries = Vec::new();
        let mut phase = "binding";
        let mut progress = MissionContinuationProgress::NotStarted;
        let operation = (|| {
            bound(self, lease)?;
            phase = "settings-reservation";
            let reservation = self
                .directory
                .reserve(false, false)
                .map_err(|e| root(RootStoreError::Storage(e)))?;
            let result = (|| {
                phase = "settings";
                let settings = self.read_settings().map_err(root)?;
                self.db
                    .busy_timeout(Duration::from_millis(
                        u64::try_from(settings.values.storage_busy_timeout_ms.0)
                            .map_err(|error| caused("mission-storage-timeout-invalid", error))?,
                    ))
                    .map_err(E::Sql)?;
                phase = "snapshot";
                read_transaction(&self.db, || {
                    phase = "lease";
                    let heartbeat = owned_heartbeat(&self.db, lease)?;
                    if lease.policy.observe(&heartbeat, now)? != MissionLeaseClock::Present {
                        return Err(E::Refused("mission-lease-not-present"));
                    }
                    if claim_revision(
                        self,
                        &lease.work,
                        lease.generation,
                        &lease.conductor,
                        &lease.policy,
                    )? != Some(lease.settings_revision)
                    {
                        return Err(E::Refused("mission-claim-provenance-invalid"));
                    }
                    phase = "storage-version";
                    let version =
                        super::optional(&self.db, "PRAGMA main.user_version", [], |row| {
                            row.get::<_, i64>(0)
                        })?;
                    if version != Some(crate::RECORDED_COMMAND_SCHEMA_VERSION) {
                        return Err(E::Refused("agent-launch-storage-epoch"));
                    }
                    phase = "work";
                    let work = self.read_work(&lease.work).map_err(root)?;
                    phase = "intentions";
                    let agents = self.list_agents(&lease.work).map_err(root)?;
                    progress = MissionContinuationProgress::PartialNotUsable;
                    for agent in agents {
                        if agent.work_id != lease.work {
                            return Err(E::Refused("mission-continuation-work-mismatch"));
                        }
                        let state = observe_entry(self, lease, &settings, &agent)?;
                        entries.push(MissionWorkerContinuationEntry {
                            agent_id: agent.id,
                            task_id: agent.task_id,
                            state,
                        });
                    }
                    phase = "snapshot-final-binding";
                    bound(self, lease)?;
                    progress = MissionContinuationProgress::CompleteButUnusable;
                    Ok((work.revision, settings.revision))
                })
            })();
            // Preserve snapshot failure alongside independent reservation-close
            // failure. After row collection, rollback, this settings close and
            // the final physical recheck must all succeed before entries are usable.
            let result = finish(
                result,
                reservation
                    .close()
                    .map_err(|e| root(RootStoreError::Storage(e))),
            );
            let metadata = result?;
            phase = "final-binding";
            bound(self, lease)?;
            Ok(metadata)
        })();
        match operation {
            Ok((revision, settings_revision)) => Ok(MissionWorkerContinuationObservation {
                work: lease.work.clone(),
                revision,
                settings_revision,
                generation: lease.generation,
                observed_at: now.to_owned(),
                entries,
            }),
            Err(cause) => Err(Box::new(MissionContinuationFailure {
                phase,
                progress,
                entries,
                cause,
            })),
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
#[path = "mission_worker_continuation/ledger_sdk.rs"]
mod ledger_sdk;

#[cfg(test)]
mod tests {
    use super::*;
    /// Structural lifecycle must never turn stored registration/PIDs into OS
    /// liveness, nor turn a legacy starting gap into a new queued attempt.
    #[test]
    fn continuation_lifecycle_refuses_registration_gaps_and_contradictions() {
        let mut agent = Agent {
            status: "queued".into(),
            ..Default::default()
        };
        assert!(lifecycle(&agent).unwrap());
        agent.child_pid = 1;
        assert!(lifecycle(&agent).is_err());
        agent.child_pid = 0;
        agent.status = "starting".into();
        assert!(lifecycle(&agent).is_err());
        agent.supervisor_pid = i64::MAX;
        agent.supervisor_identity = "retained-stale-stamp".into();
        agent.host = "retained-host".into();
        agent.heartbeat = "2026-10-22T10:00:00Z".into();
        assert!(!lifecycle(&agent).unwrap());
        agent.status = "running".into();
        assert!(lifecycle(&agent).is_err());
        agent.child_pid = i64::MAX;
        agent.child_identity = "retained-child-stamp".into();
        assert!(!lifecycle(&agent).unwrap());
        agent.native_execution_result = Some(crate::GoRaw(
            serde_json::value::to_raw_value(&true).unwrap(),
        ));
        assert!(lifecycle(&agent).is_err());
        agent.native_execution_result = None;
        agent.status = "unknown".into();
        assert!(lifecycle(&agent).is_err());
    }
    /// Project only the structural fields read by lifecycle from a registered
    /// native Agent. Values are inert fixture inputs, never OS identity or
    /// persisted admission evidence; starting legitimately has no child yet.
    fn registered_lifecycle_fixture(status: &str) -> Agent {
        let has_child = status != "starting";
        Agent {
            status: status.into(),
            supervisor_pid: 101,
            supervisor_identity: "fixture-supervisor-stamp".into(),
            host: "fixture-host".into(),
            heartbeat: "2026-10-22T10:00:00Z".into(),
            child_pid: if has_child { 102 } else { 0 },
            child_identity: if has_child {
                "fixture-child-stamp".into()
            } else {
                String::new()
            },
            ..Default::default()
        }
    }
    /// Quarantine retains stopping, a child and incomplete native observations,
    /// without ended/exit_code. The same result is premature in starting/running.
    /// Pure classification does not execute quarantine, verify its receipt or
    /// establish liveness, resource release or terminal completion.
    #[test]
    fn continuation_lifecycle_preserves_quarantine_and_refuses_premature_results() {
        let result = crate::GoRaw(
            serde_json::value::to_raw_value(&serde_json::json!({
                "schema_version": 1,
                "kernel_reaped": true,
                "recovery_required": true,
                "workspace_release_confirmed": false,
                "incident": "output_drain_incomplete",
                "output_complete": false,
                "exit_code": 0
            }))
            .unwrap(),
        );
        for status in ["starting", "running"] {
            let mut agent = registered_lifecycle_fixture(status);
            assert!(!lifecycle(&agent).unwrap(), "valid {status} control");
            agent.native_execution_result = Some(result.clone());
            assert!(matches!(
                lifecycle(&agent),
                Err(RootStoreError::Refused(
                    "mission-continuation-lifecycle-incoherent"
                ))
            ));
        }
        let mut agent = registered_lifecycle_fixture("stopping");
        assert!(!lifecycle(&agent).unwrap(), "valid stopping control");
        agent.native_execution_result = Some(result);
        assert!(!lifecycle(&agent).expect("quarantine stays registered"));
    }
    /// Every registered native phase has an atomic heartbeat from claim, pulse
    /// or quarantine. Empty or malformed timestamps must refuse, even when all
    /// PIDs and stamps remain coherent. Empty is a semantic refusal; malformed
    /// retains the parse cause and invalidates the snapshot. This checks
    /// representation, not age/liveness; every variant has an accepted control.
    #[test]
    fn continuation_lifecycle_refuses_registered_with_invalid_heartbeat() {
        for status in ["starting", "running", "stopping"] {
            for heartbeat in ["", "not-a-timestamp"] {
                let mut agent = registered_lifecycle_fixture(status);
                assert!(!lifecycle(&agent).unwrap(), "valid {status} control");
                agent.heartbeat = heartbeat.into();
                let error = lifecycle(&agent).expect_err("invalid heartbeat must refuse");
                if heartbeat.is_empty() {
                    assert!(matches!(
                        error,
                        RootStoreError::Refused("mission-continuation-lifecycle-incoherent")
                    ));
                } else {
                    assert_eq!(error.code(), "mission-continuation-heartbeat-invalid");
                    let RootStoreError::Launch(launch) = &error else {
                        panic!("malformed heartbeat must retain its launch wrapper");
                    };
                    let AgentLaunchError::Cause { cause, .. } = launch.as_ref() else {
                        panic!("malformed heartbeat must retain its parse cause");
                    };
                    assert!(cause
                        .downcast_ref::<swarm_wire_json::go_rfc3339::GoTimestampError>()
                        .is_some());
                    assert!(matches!(
                        entry_result("intention", Err(error)),
                        Err(E::Root(_))
                    ));
                }
            }
        }
    }
    /// A semantic refusal is retained; a composite containing it cannot hide an
    /// IO/cleanup failure and may not be downgraded to a harmless per-row result.
    #[test]
    fn continuation_refusals_preserve_wrappers_and_stop_on_cleanup_or_storage() {
        let direct =
            || RootStoreError::Launch(Box::new(AgentLaunchError::Refused("fixture-refusal")));
        assert!(matches!(
            entry_result("proof", Err(direct())).unwrap(),
            MissionWorkerContinuationState::Refused { .. }
        ));
        let composite = RootStoreError::Launch(Box::new(AgentLaunchError::Cleanup {
            primary: Box::new(AgentLaunchError::Refused("fixture-refusal")),
            cleanup: Box::new(AgentLaunchError::caused(
                "fixture-close",
                std::io::Error::other("fixture-private"),
            )),
        }));
        let error = entry_result("proof", Err(composite)).err().unwrap();
        assert!(matches!(error, E::Root(_)));
        assert!(entry_result(
            "query",
            Err(RootStoreError::Sql(rusqlite::Error::InvalidQuery))
        )
        .is_err());
        let direct_provenance = RootStoreError::Launch(Box::new(AgentLaunchError::caused(
            "agent-mission-provenance",
            E::Refused("legacy-instance"),
        )));
        assert!(semantic_refusal(&direct_provenance));
        let bad_provenance = RootStoreError::Launch(Box::new(AgentLaunchError::caused(
            "agent-mission-provenance",
            E::Sql(rusqlite::Error::InvalidQuery),
        )));
        assert!(!semantic_refusal(&bad_provenance));
    }
    /// Failure rendering contains only static phase/code, not prior entry IDs or
    /// the private IO source. Typed primary/secondary causes remain accessible.
    #[test]
    fn continuation_failure_display_keeps_private_observations_out() {
        let error = MissionContinuationFailure {
            phase: "snapshot",
            progress: MissionContinuationProgress::PartialNotUsable,
            entries: vec![MissionWorkerContinuationEntry {
                agent_id: "private-agent".into(),
                task_id: "private-task".into(),
                state: MissionWorkerContinuationState::OutsideAutomaticWorkerScope,
            }],
            cause: E::Cleanup {
                primary: Box::new(E::Refused("fixture-query")),
                cleanup: Box::new(caused(
                    "fixture-close",
                    std::io::Error::other("private-secret"),
                )),
            },
        };
        let text = format!("{error:?}");
        assert!(!text.contains("private"));
        assert!(error.source().is_some());
        assert!(error.cause.cleanup().is_some());
        assert_eq!(error.entries.len(), 1);
    }
    /// Create an isolated Root, Work and real claim using supported operations.
    /// No SQL manufactures an Agent, permission, receipt or owning handle.
    fn fixture() -> (std::path::PathBuf, RootStore, MissionLease, String) {
        use std::os::unix::fs::PermissionsExt;
        let path =
            std::env::temp_dir().join(crate::generate_identifier("continuation126-").unwrap());
        std::fs::create_dir(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut environment = crate::SystemEnvironment;
        let mut store = RootStore::initialize(&path, &mut environment).unwrap();
        let request = serde_json::json!({"schema_version":1,"event_id":"continuation-create",
            "expected_revision":0,"title":"Continuation fixture","objective":"Observe retained intentions",
            "scope":"Owned inert fixture","criteria":["No observation mutation"]});
        let work = store
            .execute_external(
                "",
                "work.create",
                &serde_json::to_vec(&request).unwrap(),
                &mut environment,
            )
            .unwrap()
            .result
            .work;
        let now = crate::system_timestamp().unwrap();
        let lease = store
            .claim_mission_supervision(&work.id, "continuation-fixture", &now)
            .unwrap();
        (path, store, lease, now)
    }
    /// Close owned handles before removing the isolated fixture. This helper
    /// cannot be used with a live Root and never hides a close failure.
    fn close_fixture(path: std::path::PathBuf, store: RootStore, lease: MissionLease) {
        lease.close().unwrap();
        store.close().unwrap();
        std::fs::remove_dir_all(path).unwrap();
    }
    /// Real public setup and two coherent observations preserve SQL changes,
    /// Work/lease state, saved settings and the existing local file inventory.
    #[test]
    fn continuation_real_empty_root_is_coherent_without_sql_or_file_mutation() {
        let (path, store, lease, now) = fixture();
        let changes = store.db.total_changes();
        let settings = std::fs::read(path.join(".swarm/engine-settings.json")).unwrap();
        let mut names = store.directory.names().unwrap();
        names.sort();
        let heartbeat = owned_heartbeat(&store.db, &lease).unwrap();
        let first = store.observe_continuation_at(&lease, &now).unwrap();
        let second = store.observe_continuation_at(&lease, &now).unwrap();
        assert_eq!(first.revision, second.revision);
        assert_eq!(first.settings_revision, second.settings_revision);
        assert!(first.entries.is_empty() && second.entries.is_empty());
        assert_eq!(first.generation, lease.generation());
        assert_eq!(store.db.total_changes(), changes);
        assert!(store.db.is_autocommit());
        assert_eq!(owned_heartbeat(&store.db, &lease).unwrap(), heartbeat);
        assert_eq!(
            std::fs::read(path.join(".swarm/engine-settings.json")).unwrap(),
            settings
        );
        let mut after = store.directory.names().unwrap();
        after.sort();
        assert_eq!(names, after);
        close_fixture(path, store, lease);
    }
    /// The exact saved clock policy, stopped ownership and another physical Root
    /// refuse before a report; these are observations, not takeover/retry rules.
    #[test]
    fn continuation_real_lease_refuses_clock_stop_and_foreign_root() {
        let (path, store, lease, now) = fixture();
        assert!(store.observe_continuation_at(&lease, &now).is_ok());
        let at = super::super::parse(&now).unwrap();
        let stale = at
            + time::Duration::milliseconds(lease.policy.stale_after_ms.0)
            + time::Duration::nanoseconds(1);
        let stale =
            crate::format_utc_timestamp(stale.unix_timestamp(), i64::from(stale.nanosecond()))
                .unwrap();
        let error = store.observe_continuation_at(&lease, &stale).err().unwrap();
        assert_eq!(error.cause.code(), "mission-lease-not-present");
        assert!(error.entries.is_empty());
        let future = at
            - time::Duration::milliseconds(lease.policy.future_tolerance_ms.0)
            - time::Duration::nanoseconds(1);
        let future =
            crate::format_utc_timestamp(future.unix_timestamp(), i64::from(future.nanosecond()))
                .unwrap();
        assert_eq!(
            store
                .observe_continuation_at(&lease, &future)
                .err()
                .unwrap()
                .cause
                .code(),
            "mission-lease-not-present"
        );
        let (other_path, other, other_lease, _) = fixture();
        assert!(other.observe_continuation_at(&lease, &now).is_err());
        close_fixture(other_path, other, other_lease);
        store
            .update_mission_supervision(&lease, &now, true)
            .unwrap();
        assert_eq!(
            store
                .observe_continuation_at(&lease, &now)
                .err()
                .unwrap()
                .cause
                .code(),
            "mission-lease-lost"
        );
        assert!(store.db.is_autocommit());
        close_fixture(path, store, lease);
    }
    /// A valid witness precedes isolated negative SQL corruption. An unavailable
    /// agent table must never become an empty report; rollback restores autocommit.
    #[test]
    fn continuation_real_query_failure_is_not_empty_success() {
        let (path, store, lease, now) = fixture();
        let absent = Agent {
            id: "absent-manual".into(),
            origin: "operateur".into(),
            work_id: lease.work.clone(),
            ..Default::default()
        };
        let missing = observe_entry(&store, &lease, &store.settings().unwrap(), &absent);
        assert!(matches!(missing, Err(E::Root(_))));

        assert!(store
            .observe_continuation_at(&lease, &now)
            .unwrap()
            .entries
            .is_empty());
        store
            .db
            .execute_batch("ALTER TABLE main.agents RENAME TO continuation126_parked_agents")
            .unwrap();
        let failure = store.observe_continuation_at(&lease, &now).err().unwrap();
        assert_eq!(failure.phase, "intentions");
        assert!(matches!(failure.cause, E::Root(_)));
        assert!(store.db.is_autocommit());
        store
            .db
            .execute_batch("ALTER TABLE main.continuation126_parked_agents RENAME TO agents")
            .unwrap();
        assert!(store
            .observe_continuation_at(&lease, &now)
            .unwrap()
            .entries
            .is_empty());
        close_fixture(path, store, lease);
    }
}
