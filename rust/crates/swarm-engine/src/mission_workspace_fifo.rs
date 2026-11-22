//! Lease-owned FIFO registration before capacity/occupancy. No provider starts.
use super::{auxiliary_schema, bound, caused, execute, read_transaction, write, RootStore};
use crate::agent_launch::CandidateHostCheck;
use crate::root_store::{mission_agent, mission_candidates, mission_task_order, mission_workspace_queue};
use crate::workspace_turn;
use crate::work_engine::MutationFence;
use crate::workspace_turn_write::{self as queue, Candidate};
use crate::{MissionLease, MissionSupervisionError as E, Work};

/// A visible candidate refusal, retaining its original typed cause. Callers must
/// inspect these even when other rows were synchronized; this is not a JSON DTO.
pub struct MissionCandidateRefusal {
    pub task: String,
    pub error: E,
}
/// Committed standing queue synchronization, never acquisition or launch. Ready
/// task IDs describe this observation only. Every per-task refusal retains its
/// typed cause. `permission_refusal` is mandatory consumer-visible evidence that
/// synchronization only cleaned obsolete waiting claims after semantic permission
/// withdrawal; it is NEVER authorization or a successful dispatch. Corruption,
/// SQL, stale lease and unknown failures remain Err with transaction cleanup.
pub struct MissionQueueRegistration {
    pub work: String,
    pub revision: i64,
    pub ready: Vec<String>,
    pub refused: Vec<MissionCandidateRefusal>,
    /// Original semantic refusal retained after committed waiting-only cleanup.
    /// Consumers must inspect it before considering any automatic work. None does
    /// not grant execution: ready candidates still need actual fenced admission.
    pub permission_refusal: Option<E>,
}

/// Validate the optional reserved append-only journal without adopting policy.
/// Only a lease-owned writer may request creation; read/grant paths never create
/// missing proof. Reserved-name/schema/index/trigger mismatches fail with cause.
pub(crate) fn schema(db: &rusqlite::Connection, create: bool) -> Result<bool, E> {
    if auxiliary_schema(
        db,
        "workspace_turn_events",
        queue::JOURNAL_DDL,
        "workspace-turn-journal-schema-invalid",
    )? {
        return Ok(true);
    }
    if !create {
        return Ok(false);
    }
    execute(db, queue::JOURNAL_DDL, [])?;
    auxiliary_schema(
        db,
        "workspace_turn_events",
        queue::JOURNAL_DDL,
        "workspace-turn-journal-schema-invalid",
    )
}

/// Check actual start evidence on a discarded Task copy, before resource claims.
/// The shared lifecycle may allocate ephemeral attempt IDs but no Work/SQL/ledger
/// mutation is retained. Provider/host checks read actual configuration; no call
/// or workspace ownership is inferred. Cost/attempt history and fresh evidence
/// are rechecked; the factory still repeats all guards before acquisition/grant.
/// ReadOnly is for diagnostics. VerifyWrite may create a temporary probe file.
fn ready(
    store: &RootStore,
    work: &Work,
    task: &str,
    profile: &crate::LaunchProfile,
    settings: &crate::EngineSettingsDocument,
    host_check: CandidateHostCheck,
) -> Result<String, E> {
    let limits = settings.values.mutation_limits().map_err(E::Settings)?;
    let quality = settings.quality_policy_snapshot().map_err(E::Settings)?;
    let context = crate::TaskLifecycleContext {
        root: &store.root,
        quality_policy: Some(&quality),
        task_limits: &limits.task,
        validation_limits: &limits.policy,
    };
    let selected = work
        .tasks
        .iter()
        .flatten()
        .find(|value| value.id == task)
        .ok_or(E::Refused("mission-candidate-task-missing"))?;
    let request = crate::Request {
        schema_version: 1,
        id: task.into(),
        status: "running".into(),
        owner: profile.provider.clone(),
        origin: "conducteur".into(),
        next: if selected.planning_retry {
            selected.next.clone()
        } else {
            "Examiner le handoff et les preuves après exécution".into()
        },
        ..Default::default()
    };
    if work.planning.is_some() {
        crate::transition_scoped_worker_task(
            work,
            &request,
            &context,
            settings,
            &mut crate::SystemEnvironment,
        )
    } else {
        crate::transition_manual_task(work, &request, &context, &mut crate::SystemEnvironment)
    }
    .map_err(|cause| caused(cause.reason(), cause))?;
    if selected.plan_max_attempts > 0 {
        let count = super::optional(
            &store.db,
            "SELECT count(*) FROM main.agents WHERE work_id=? AND task_id=?",
            rusqlite::params![work.id, task],
            |row| row.get::<_, i64>(0),
        )?
        .ok_or(E::Refused("mission-candidate-history-missing"))?;
        if count >= selected.plan_max_attempts {
            return Err(E::Refused("agent-launch-plan-attempts-exhausted"));
        }
    }
    crate::financial_budget::admission_budget(&store.db, &work.id)
        .map_err(|cause| caused(cause.code(), cause))?;
    crate::root_store::mission_cost_guard::admit(&store.db, &work.id, task, settings)?;
    crate::agent_launch::candidate_workspace(
        &store.root,
        &store.directory,
        work,
        task,
        profile,
        settings,
        host_check,
    )
    .map_err(|cause| caused(cause.code(), cause))
}

/// Current automatic worker-start refusals in one recorded Work snapshot.
/// Codes contain only static engine classifications. An empty list means that
/// this observation found no refusal in its covered checks.
#[derive(Debug, serde::Serialize)]
pub struct MissionLaunchDiagnostics {
    pub work: String,
    pub revision: i64,
    pub refusals: Vec<MissionLaunchRefusal>,
}

/// One refusal for the whole mission (null task) or for an identified task in
/// that mission. Other missions and filesystem/provider data stay private.
#[derive(Debug, serde::Serialize)]
pub struct MissionLaunchRefusal {
    pub task: Option<String>,
    pub codes: Vec<String>,
}

impl MissionLaunchRefusal {
    /// Keep the primary and typed nested reasons in diagnostic order without
    /// formatting process output, paths, or other free-form error data.
    fn from_error(task: Option<String>, error: &E) -> Self {
        let mut codes = vec![error.code().to_owned()];
        for cause in crate::native_diagnostics::project(error) {
            for code in cause.labels {
                if !codes.contains(&code) {
                    codes.push(code);
                }
            }
        }
        Self { task, codes }
    }
}

/// Preserve known semantic permission refusals for both observation and queue
/// cleanup. Unknown or compound failures remain errors with their original cause.
fn launch_permission(
    store: &RootStore,
    work: &Work,
) -> Result<Result<crate::MissionControlSettings, E>, E> {
    match mission_agent::permission(&store.db, work) {
        Ok((controls, _)) => Ok(Ok(controls)),
        Err(error @ E::Refused(
            "planning-mission-paused"
            | "mission_archived"
            | "mission-controls-required"
            | "mission-autonomy-required"
            | "mission-consent-required",
        )) => Ok(Err(error)),
        Err(error) => Err(error),
    }
}

impl RootStore {
    /// Observe automatic worker start refusals from the current Work, controls,
    /// task admission checks and saved FIFO in a SQL read snapshot. Saved settings
    /// and host checks are observations. A later launch rechecks them and requires
    /// its lease, workspace and factory admission. No provider invocation,
    /// attempt, queue registration, reservation or approval is performed.
    /// Unknown Work and SQL/queue/settings observation failures preserve their
    /// typed cause. A task preflight refusal preserves its typed codes.
    pub fn mission_launch_diagnostics(&self, id: &str) -> Result<MissionLaunchDiagnostics, E> {
        let settings = self.settings().map_err(|cause| E::Root(Box::new(cause)))?;
        read_transaction(&self.db, || {
            let work = crate::read_recorded_work(&self.db, id)
                .map_err(|cause| E::Root(Box::new(crate::RootStoreError::Work(cause))))?;
            let mut refusals = Vec::new();
            match launch_permission(self, &work)? {
                Err(error) => refusals.push(MissionLaunchRefusal::from_error(None, &error)),
                Ok(controls) => {
                    let capacity = mission_agent::check_capacity(
                        &self.db, id, None, controls.slots, &settings,
                    );
                    let full = match capacity {
                        Ok(()) => false,
                        Err(E::Refused("mission-agent-slots-occupied")) => true,
                        Err(error) => return Err(error),
                    };
                    let order = mission_task_order::read_order(&self.db, &work)?;
                    let turns = workspace_turn::read(&self.db).map_err(mission_workspace_queue::error)?;
                    let mut ready_tasks = Vec::new();
                    for candidate in mission_candidates::select(&work, &order)? {
                        let task = candidate.task_id;
                        match candidate.profile.and_then(|profile| {
                            ready(self, &work, &task, &profile, &settings, CandidateHostCheck::ReadOnly)
                        }) {
                            Err(E::Refused("mission-candidates-status-ineligible")) => {}
                            Err(error) => refusals.push(MissionLaunchRefusal::from_error(Some(task), &error)),
                            Ok(workspace) => ready_tasks.push((task, workspace)),
                        }
                    }
                    let candidates: Vec<_> = ready_tasks.iter().map(|(task_id, workspace)| {
                        Candidate { task_id, workspace }
                    }).collect();
                    let blocked = queue::preview_waiting_blockers(&turns, id, &candidates)
                        .map_err(mission_workspace_queue::error)?;
                    for (task, workspace) in ready_tasks {
                        let error = if blocked.contains(&task) {
                            E::Refused("workspace-turn-fifo-blocked")
                        } else if full {
                            E::Refused("mission-agent-slots-occupied")
                        } else {
                            match crate::agent_launch::guards_owned(
                                &self.db, id, &task, &workspace, None,
                            ) {
                                Ok(()) => continue,
                                Err(cause) => caused(cause.code(), cause),
                            }
                        };
                        refusals.push(MissionLaunchRefusal::from_error(Some(task), &error));
                    }
                }
            }
            Ok(MissionLaunchDiagnostics {
                work: work.id,
                revision: work.revision,
                refusals,
            })
        })
    }


    /// Synchronize actual ready contenders under this Root's owning process lease
    /// and current settings in one writer transaction. Ignores available slots and
    /// occupied workspaces so a waiting task keeps its place. Existing acquired
    /// rows stay held; obsolete waiting rows release with full journal history.
    /// Ready means standing claim only, never grant: actual launch rechecks all
    /// guards. Known semantic pause/archive/control/consent withdrawals synchronize
    /// an empty candidate set, releasing waiting claims while preserving acquired
    /// Agents, and return their original cause in `permission_refusal`. Consumers
    /// MUST handle that field; cleanup never grants execution or renews a lease.
    /// Malformed/missing storage proof, SQL, settings, unknown permission variants,
    /// lease/commit/rollback/release failures remain Err with original causes.
    /// No provider runs, attempts/reservations/consumption are not persisted.
    /// Caller must handle the result/refusals and close lease/Root explicitly.
    pub fn sync_mission_workspace_queue(
        &mut self,
        lease: &MissionLease,
    ) -> Result<MissionQueueRegistration, E> {
        bound(self, lease)?;
        write(self, |settings| {
            let now =
                crate::system_timestamp().map_err(|cause| caused("mission-queue-clock", cause))?;
            lease
                .check(&self.db, &lease.work, &now)
                .map_err(|cause| E::Root(Box::new(crate::RootStoreError::Mutation(cause))))?;
            let work = crate::read_recorded_work(&self.db, &lease.work)
                .map_err(|cause| E::Root(Box::new(crate::RootStoreError::Work(cause))))?;
            let (controls, permission_refusal) =
                match launch_permission(self, &work)? {
                    Ok(controls) => (Some(controls), None),
                    Err(error) => (None, Some(error)),
                };
            let mut accepted = Vec::new();
            let mut refused = Vec::new();
            if let Some(controls) = controls {
                settings
                    .values
                    .mission_dispatch_policy()
                    .map_err(E::Settings)?
                    .slots(Some(controls.slots))
                    .map_err(E::Settings)?;
                let order = mission_task_order::read_order(&self.db, &work)?;
                let candidates = mission_candidates::select(&work, &order)?;
                for candidate in candidates {
                    let result = candidate.profile.and_then(|profile| {
                        ready(self, &work, &candidate.task_id, &profile, settings, CandidateHostCheck::VerifyWrite)
                    });
                    match result {
                        Ok(path) => accepted.push((candidate.task_id, path)),
                        Err(error) => refused.push(MissionCandidateRefusal {
                            task: candidate.task_id,
                            error,
                        }),
                    }
                }
            }
            schema(&self.db, true)?;
            let values: Vec<_> = accepted
                .iter()
                .map(|(task_id, workspace)| Candidate { task_id, workspace })
                .collect();
            queue::sync_waiting(&self.db, &work.id, &values, &now)
                .map_err(crate::root_store::mission_workspace_queue::error)?;
            let after =
                crate::system_timestamp().map_err(|cause| caused("mission-queue-clock", cause))?;
            lease
                .check(&self.db, &lease.work, &after)
                .map_err(|cause| E::Root(Box::new(crate::RootStoreError::Mutation(cause))))?;
            Ok(MissionQueueRegistration {
                work: work.id,
                revision: work.revision,
                ready: accepted.into_iter().map(|value| value.0).collect(),
                refused,
                permission_refusal,
            })
        })
    }
}
