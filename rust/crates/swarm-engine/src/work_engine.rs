//! Transaction-owning native Work command assembly on a supplied v37 connection.
//! Root opening/migrations, authenticated web transport, task status transitions
//! and persisted Admin policy loading remain separate integration boundaries.
use crate::{
    apply_checkpoint, apply_ooda, apply_planning_validation_signals, apply_task_metadata,
    apply_work_creation, apply_work_definition, decode_external_request,
    encode_work_mutation_intent, inspect_recorded_mutation, stage_recorded_change,
    validate_work_request, CheckpointError, DefinitionContext, GitIssue, MutationGuardError,
    MutationInspection, PlanningStateError, RecordedChange, RecordedChangeError,
    RecordedChangeOutcome, Request, RuntimeError, TaskDefinitionError, TaskDefinitionLimits,
    ValidationPolicyLimits, Work, WorkDefinitionError, WorkEnvironment,
};
use rusqlite::{Connection, DropBehavior, Transaction};
use std::{collections::BTreeMap, error::Error, fmt, path::Path};
/// Additional Root-owned admission, never authorization supplied by a request.
/// Used by Work commands and manual agent-intent factories within their SQL
/// transaction before staging and before completion, including guarded replay. It
/// must retain typed refusal/cleanup causes; no caller JSON can install a guard.
/// Send/Sync preserves command assembler mobility to native worker threads.
pub(crate) trait MutationFence: Send + Sync {
    fn check(&self, db: &Connection, work: &str, now: &str) -> Result<(), WorkEngineError>;
    /// Recheck worker admission with the actual reserved Root settings. Default
    /// preserves the existing manual lease semantics; no automatic authority.
    fn check_agent(&self, db:&Connection,work:&str,now:&str,_settings:&crate::EngineSettingsDocument)->Result<(),WorkEngineError>{self.check(db,work,now)}
    /// No automatic provenance exists on ordinary fences. These are protocol
    /// capabilities selected by engine-owned types, not operator policy defaults.
    fn mission_conductor(&self)->Option<&str>{None}
    /// Preserve a configured profile only on the explicit automatic factory.
    fn mission_profile(&self)->Option<&crate::LaunchProfile>{None}
    /// Build automatic metadata from the actual Agent and canonical event in the
    /// caller transaction. Automatic fences may acquire its exact FIFO turn;
    /// errors require caller rollback and do not imply a committed launch.
    /// Ordinary commands remain manual and cannot supply a binding in JSON.
    fn mission_binding(&self,_db:&Connection,_agent:&crate::Agent,_receipt:MutationFenceReceipt<'_>)->Result<Option<crate::GoRaw>,WorkEngineError>{Ok(None)}
    /// Verify only the exact finishing planner original for closure. Default
    /// denies: a lease, body flag or process tuple is no completion authority.
    /// Caller owns the same transaction; errors preserve the candidate unchanged.
    #[cfg(target_os = "linux")]
    fn finishing_planner(&self, _db: &Connection, _run: &crate::planning_run_store::Run, _event: &str, _now: &str) -> Result<(), WorkEngineError> {
        Err(WorkEngineError::PlanningActivation(Box::new(crate::AgentLaunchError::Refused("planning-closure-native-run-active"))))
    }
    /// Stage a new action's provenance in the caller transaction. Replay is not
    /// restaged. Failure rolls back the command; this is not a commit receipt yet.
    fn stage_receipt(
        &self,
        db: &Connection,
        receipt: MutationFenceReceipt<'_>,
    ) -> Result<(), WorkEngineError>;
}
/// Canonical persisted event identity and Root's current settings, not request
/// assertions of actor, ownership or authority. Borrowed only while staging.
pub(crate) struct MutationFenceReceipt<'a> {
    pub work: &'a str,
    pub event: &'a str,
    pub kind: &'a str,
    pub work_revision: i64,
    pub command_settings: &'a crate::EngineSettingsDocument,
}
pub struct EngineMutationLimits {
    pub task: TaskDefinitionLimits,
    pub policy: ValidationPolicyLimits,
}
pub enum WorkEngineError {
    Request(crate::Error),
    Definition(WorkDefinitionError),
    Task(TaskDefinitionError),
    Checkpoint(CheckpointError),
    Planning(PlanningStateError),
    PlanningRequest(crate::PlanningRequestError),
    PlanningEncoding(crate::WorkError),
    PlanningControl(crate::PlanningControlError),
    PlanningBootstrap(Box<crate::AgentLaunchError>),
    PlanningActivation(Box<crate::AgentLaunchError>),
    ValidationController(Box<crate::AgentLaunchError>),
    Budget(crate::BudgetError),
    TaskModel(crate::AgentLaunchError),
    Quota(crate::AgentLaunchError),
    TaskReport(crate::AgentLaunchError),
    Guard(MutationGuardError),
    Change(RecordedChangeError),
    Runtime(RuntimeError),
    Sql(rusqlite::Error),
    /// Mission ownership/time refusal; original and cleanup causes are retained.
    Mission(Box<crate::MissionSupervisionError>),
    Gate(crate::GateContractError),
    GateRequest(crate::GateRequestError),
    Lifecycle(crate::TaskLifecycleError),
    UnportedStatus,
    Overflow,
    Rollback {
        primary: Box<Self>,
        cleanup: rusqlite::Error,
    },
    ReplayRollback(rusqlite::Error),
    TransactionLost {
        primary: Box<Self>,
    },
    Commit {
        cause: rusqlite::Error,
        cleanup: Option<Box<rusqlite::Error>>,
        transaction_ended: bool,
        rolled_back: bool,
    },
}
impl WorkEngineError {
    /// Return static storage/contract/codec families without request content.
    /// Planning decoding and intent encoding retain distinct typed causes; this
    /// classification performs no IO, retry, state change or authority decision.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Request(crate::Error::Version) => "invalid_schema",
            Self::Request(e) => e.code(),
            Self::Gate(e) => e.code(),
            Self::GateRequest(e) => e.code(),
            Self::Lifecycle(e) => e.code(),
            Self::Definition(e) => e.code(),
            Self::Task(e) => e.code(),
            Self::Checkpoint(e) => e.code(),
            Self::Planning(_) => "format",
            Self::PlanningRequest(e) => e.code(),
            Self::PlanningEncoding(_) => "planning-request-encoding",
            Self::PlanningControl(e) => e.code(),
            Self::PlanningBootstrap(e) | Self::PlanningActivation(e) | Self::ValidationController(e) => e.code(),
            Self::Budget(e) => e.code(),
            Self::TaskModel(e) | Self::Quota(e) => e.code(),
            Self::TaskReport(e) => e.code(),
            Self::Guard(e) => e.code(),
            Self::Change(e) => e.code(),
            Self::Runtime(e) => e.code(),
            Self::Sql(_) => "command-storage",
            Self::Mission(e) => e.code(),
            Self::UnportedStatus => "command_not_ported",
            Self::Overflow => "recorded-change-overflow",
            Self::Rollback { .. } | Self::ReplayRollback(_) => "command-rollback",
            Self::TransactionLost { .. } => "command-transaction-lost",
            Self::Commit {
                rolled_back: true, ..
            } => "command-commit-refused",
            Self::Commit { .. } => "command-commit-uncertain",
        }
    }
    /// A commit error or transaction loss requires observing actual state before
    /// deciding recovery. Mission cleanup/commit markers also refuse automatic
    /// storage retry. No retry, event renaming or consumption reset occurs.
    pub fn requires_reconciliation(&self) -> bool {
        matches!(
            self,
            Self::TransactionLost { .. }
                | Self::Commit {
                    rolled_back: false,
                    ..
                }
                | Self::Rollback { .. }
                | Self::ReplayRollback(_)
        ) || matches!(self, Self::Mission(e) if e.requires_reconciliation())
    }
}
/// Public diagnostics include only static contract reasons. Original request,
/// IO, SQL and process details remain in typed sources for authorized diagnosis.
impl fmt::Display for WorkEngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Definition(e) => fmt::Display::fmt(e, f),
            Self::PlanningRequest(e) => fmt::Display::fmt(e, f),
            Self::PlanningControl(e) => fmt::Display::fmt(e, f),
            Self::PlanningBootstrap(e) | Self::PlanningActivation(e) | Self::ValidationController(e) => fmt::Display::fmt(e, f),
            Self::Budget(e) => fmt::Display::fmt(e, f),
            Self::TaskModel(e) | Self::Quota(e) => fmt::Display::fmt(e, f),
            Self::TaskReport(e) => fmt::Display::fmt(e, f),
            Self::Task(e) => fmt::Display::fmt(e, f),
            Self::Gate(e) => fmt::Display::fmt(e, f),
            Self::Lifecycle(e) => fmt::Display::fmt(e, f),
            _ => f.write_str(self.code()),
        }
    }
}
impl fmt::Debug for WorkEngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for WorkEngineError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Request(e) => Some(e),
            Self::Gate(e) => Some(e),
            Self::GateRequest(e) => Some(e),
            Self::Lifecycle(e) => Some(e),
            Self::Definition(e) => Some(e),
            Self::Task(e) => Some(e),
            Self::Checkpoint(e) => Some(e),
            Self::Planning(e) => Some(e),
            Self::PlanningRequest(e) => Some(e),
            Self::PlanningEncoding(e) => Some(e),
            Self::PlanningControl(e) => Some(e),
            Self::PlanningBootstrap(e) | Self::PlanningActivation(e) | Self::ValidationController(e) => Some(e.as_ref()),
            Self::Budget(e) => Some(e),
            Self::TaskModel(e) | Self::Quota(e) => Some(e),
            Self::TaskReport(e) => Some(e),
            Self::Guard(e) => Some(e),
            Self::Change(e) => Some(e),
            Self::Runtime(e) => Some(e),
            Self::Sql(e) => Some(e),
            Self::Mission(e) => Some(e.as_ref()),
            Self::ReplayRollback(e) => Some(e),
            Self::Rollback { primary, .. } | Self::TransactionLost { primary } => Some(primary),
            Self::Commit { cause, .. } => Some(cause),
            _ => None,
        }
    }
}
pub struct WorkMutationOutcome {
    pub work: Work,
    pub replayed: bool,
    pub git_issues: Vec<GitIssue>,
}
impl fmt::Debug for WorkMutationOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorkMutationOutcome")
            .field("replayed", &self.replayed)
            .field("git_issue_count", &self.git_issues.len())
            .finish_non_exhaustive()
    }
}
pub struct WorkEngine<'a, E: WorkEnvironment> {
    db: &'a mut Connection,
    root: &'a Path,
    environment: &'a mut E,
    limits: &'a EngineMutationLimits,
    quality_policy: Option<&'a crate::QualityPolicySnapshot>,
    planning_settings: Option<&'a crate::EngineSettingsDocument>,
    fence: Option<&'a dyn MutationFence>,
}
impl<'a, E: WorkEnvironment> WorkEngine<'a, E> {
    /// The caller owns database/root selection and must supply effective settings,
    /// not an agent-supplied authority assertion. This opens or migrates no root.
    pub fn new(
        db: &'a mut Connection,
        root: &'a Path,
        environment: &'a mut E,
        limits: &'a EngineMutationLimits,
    ) -> Self {
        Self {
            db,
            root,
            environment,
            limits,
            quality_policy: None,
            planning_settings: None,
            fence: None,
        }
    }
    /// Observed effective policy supplied by the Root owner under its lock.
    /// A caller-supplied snapshot alone is not proof of authority/currentness.
    pub fn with_quality_policy(mut self, policy: Option<&'a crate::QualityPolicySnapshot>) -> Self {
        self.quality_policy = policy;
        self
    }
    /// Attach actual observed settings for planning/task-model controls under the Root
    /// owner's reservation. This pointer grants no authority or freshness lease;
    /// no IO, default adoption or operation occurs in this builder.
    pub fn with_planning_settings(mut self, settings: &'a crate::EngineSettingsDocument) -> Self {
        self.planning_settings = Some(settings);
        self
    }
    /// Attach Root's additional supervisor guard. Manual local operator commands
    /// retain their existing contract; the mission route always supplies a lease.
    /// This pointer grants no rights, adopts no defaults and performs no IO.
    pub(crate) fn with_mutation_fence(mut self, fence: Option<&'a dyn MutationFence>) -> Self {
        self.fence = fence;
        self
    }
    /// Authoritative external DTO boundary: origin is cleared before internal
    /// intent encoding, reservation and replay. Output is returned only after
    /// commit, or after explicit rollback of a read-only replay transaction.
    /// Planning controls use their complete Go DTO/intent and pretransaction
    /// guards; replay precedes candidate policy/clock/lease edits. Enablement
    /// consumes explicit v12 request/requirement policy and observes provider/
    /// reviewer metadata. On Linux, committed-only repository enablement retains
    /// a private bare copy and pending/final manifests before SQL publication;
    /// filesystem preparation survives a SQL rollback for checked recovery.
    /// Dirty snapshots and managed claims/execution/settlement remain unported.
    /// v24 claims freeze owned workflow/context and leases;
    /// v25 decisions apply task/delegation, same-task native production retry
    /// or exclusive scoped closure atomically.
    /// Explicit lease-renew preserves generation/consumption and rechecks the
    /// original live fence before staging; it is not a judge-cycle renewal.
    /// Claim/decide/lease-renew read canonical mission pause/archive state in this
    /// transaction after replay. Claim reserves the common financial estimate
    /// atomically with counters/Work; any later refusal rolls that reservation back.
    /// Recorded replay neither rechecks a new admission nor reserves a second call.
    /// Current native-review input is verified through its original chain: passed
    /// supports ordinary actions; changes_requested supports only same-task retry.
    /// Managed integration, exploration and run-observation decisions remain unported. Closure
    /// requires fresh accepted descendants and internal physical ownership guards.
    /// This boundary starts
    /// no agent process; native execution has separate Root operations.
    /// Budget configuration
    /// uses the complete Go DTO with submitted actor/time cleared before replay;
    /// its policy and cockpit event share the Work transaction, preserving every
    /// reservation. Task-model uses the strict inherited request and common native
    /// provider resolver; active attempts refuse changes and replay precedes source
    /// reads. Native task.submit binds a real current completed worker/attempt and
    /// report digest without creating a synthetic attempt or changing reservations.
    /// Provider-enabled planning performs source-observed reviewer bootstrap only
    /// after replay; catalogue/close failures roll back with original safe causes.
    /// This records configuration, never a provider process or dispatch consent.
    /// Scoped submissions deliver a checked handoff to the owning scope using
    /// an engine timestamp; submission does not perform a supervisor decision.
    /// A Root mission fence admits before staging and rechecks before commit in
    /// this same transaction; refusal rolls back all staged effects. Replays also
    /// require current ownership. Manual Root commands keep their existing path.
    /// Quota configuration uses the same replay/transaction path and saved audit
    /// reason policy. It cannot reset consumption, transfer a live planner/review
    /// owner or configure unsupported client windows. Replay precedes current
    /// policy checks; a new change revalidates its complete candidate.
    /// Global mission pause/stop preserve consumption and physical run ownership,
    /// revoke logical planner claims, and pause departures atomically; stop also
    /// records consent=false. Unpause preserves mode/consent, never dispatches or
    /// enables automatic execution. Missing controls adopt only manual denial and
    /// saved initial slots, without inheriting SQL automatic defaults.
    /// Mission start validates current organization/profile/slot sources and stores
    /// explicit local consent; resume requires a current true record and preserves
    /// its profile/history. Neither action starts a conductor or provider process.
    /// Priority changes exact signed ordinal metadata and its audit atomically,
    /// under the same event/revision/replay guard; no execution control changes.
    /// Actual operator/time are supplied by the engine environment;
    /// they do not by themselves establish an authenticated web/agent identity.
    pub fn execute_external(
        &mut self,
        work: &str,
        kind: &str,
        raw: &[u8],
    ) -> Result<WorkMutationOutcome, WorkEngineError> {
        let mission = if let Some(action) = crate::MissionControlAction::from_kind(kind) {
            Some(((if action==crate::MissionControlAction::Start {crate::decode_mission_start_request(raw)} else if action==crate::MissionControlAction::Priority {crate::decode_mission_priority_request(raw)} else {crate::decode_mission_control_request(raw)}).map_err(|e| WorkEngineError::Mission(Box::new(e)))?, action))
        } else { None };
        let quota = if kind == "quotas.configure" {
            Some(crate::decode_quota_change(raw).map_err(WorkEngineError::Quota)?)
        } else { None };
        let task_report = if kind == "task.submit" {
            Some(crate::decode_task_report_request(raw).map_err(WorkEngineError::TaskReport)?)
        } else {
            None
        };
        let task_model = if kind == "task.model" {
            Some(crate::decode_task_model_request(raw).map_err(WorkEngineError::TaskModel)?)
        } else {
            None
        };
        let planning = if let Some(action) = crate::PlanningControlAction::from_kind(kind) {
            let request =
                crate::decode_planning_request(raw).map_err(WorkEngineError::PlanningRequest)?;
            crate::planning_control::validate_planning_control_action(&request, action)
                .map_err(WorkEngineError::PlanningControl)?;
            Some((request, action))
        } else {
            None
        };
        let budget = if kind == "budget.configure" {
            let mut change = crate::decode_budget_change(raw).map_err(WorkEngineError::Budget)?;
            crate::validate_budget_change(&change).map_err(WorkEngineError::Budget)?;
            change.budget.actor.clear();
            change.budget.updated.clear();
            Some(change)
        } else {
            None
        };
        let gate = if kind == "gate" {
            Some(crate::decode_gate_request(raw).map_err(WorkEngineError::GateRequest)?)
        } else {
            None
        };
        let (request, intent) = if let Some((change, _)) = &mission {
            (Request{schema_version:change.schema_version,event_id:change.event_id.clone(),expected_revision:change.expected_revision,id:change.task_id.clone(),..Default::default()},
             crate::encode_model(change).map_err(|e|WorkEngineError::Mission(Box::new(crate::mission_control_request::cause("mission-control-request-encoding",e))))?)
        } else if let Some(report) = &task_report {
            (
                Request {
                    schema_version: report.schema_version,
                    event_id: report.event_id.clone(),
                    expected_revision: report.expected_revision,
                    id: report.id.clone(),
                    status: report.status.clone(),
                    ..Default::default()
                },
                crate::encode_model(report).map_err(|e| {
                    WorkEngineError::TaskReport(crate::AgentLaunchError::caused(
                        "task-report-request-encoding",
                        e,
                    ))
                })?,
            )
        } else if let Some(change) = &quota {
            (Request {
                schema_version: change.schema_version,
                event_id: change.event_id.clone(),
                expected_revision: change.expected_revision,
                ..Default::default()
            }, crate::encode_model(change).map_err(|e| WorkEngineError::Quota(
                crate::AgentLaunchError::caused("quota-request-encoding", e)))?)
        } else if let Some(change) = &task_model {
            (
                Request {
                    schema_version: change.schema_version,
                    event_id: change.event_id.clone(),
                    expected_revision: change.expected_revision,
                    id: change.task_id.clone(),
                    ..Default::default()
                },
                crate::encode_model(change).map_err(|e| {
                    WorkEngineError::TaskModel(crate::AgentLaunchError::caused(
                        "task-model-request-encoding",
                        e,
                    ))
                })?,
            )
        } else if let Some((planning, _)) = &planning {
            (
                Request {
                    schema_version: planning.schema_version,
                    event_id: planning.event_id.clone(),
                    expected_revision: planning.expected_revision,
                    ..Default::default()
                },
                crate::encode_planning_request(planning)
                    .map_err(WorkEngineError::PlanningEncoding)?,
            )
        } else if let Some(change) = &budget {
            (
                Request {
                    schema_version: change.schema_version,
                    event_id: change.event_id.clone(),
                    expected_revision: change.expected_revision,
                    ..Default::default()
                },
                crate::encode_model(change)
                    .map_err(|error| WorkEngineError::Budget(crate::BudgetError::Decode(error)))?,
            )
        } else if let Some(gate) = &gate {
            (
                Request {
                    schema_version: gate.schema_version,
                    event_id: gate.event_id.clone(),
                    expected_revision: gate.expected_revision,
                    id: gate.task_id.clone(),
                    ..Default::default()
                },
                crate::encode_gate_request_intent(gate).map_err(WorkEngineError::GateRequest)?,
            )
        } else {
            let request = decode_external_request(raw).map_err(WorkEngineError::Request)?;
            validate_work_request(kind, &request).map_err(WorkEngineError::Definition)?;
            let intent = encode_work_mutation_intent(&request).map_err(WorkEngineError::Request)?;
            (request, intent)
        };
        let mut tx = self.db.transaction().map_err(WorkEngineError::Sql)?;
        let result = (|| {
            if let Some(fence) = self.fence {
                let at = self
                    .environment
                    .timestamp()
                    .map_err(WorkEngineError::Runtime)?;
                fence.check(&tx, work, &at)?;
            }
            let outcome = apply(
                &mut tx,
                self.root,
                self.environment,
                self.limits,
                Invocation {
                    work,
                    kind,
                    request: &request,
                    intent: &intent,
                    mission: mission.as_ref().map(|(r,a)|(r,*a)),
                    quota: quota.as_ref(),
                    task_model: task_model.as_ref(),
                    task_report: task_report.as_ref(),
                    gate: gate.as_ref(),
                    budget: budget.as_ref(),
                    quality_policy: self.quality_policy,
                    planning: planning
                        .as_ref()
                        .map(|(request, action)| (request, *action)),
                    planning_settings: self.planning_settings,
                    fence: self.fence,
                },
            )?;
            if let Some(fence) = self.fence {
                if !outcome.replayed {
                    let command_settings = self.planning_settings.ok_or_else(|| {
                        WorkEngineError::Mission(Box::new(crate::MissionSupervisionError::Refused(
                            "mission-command-settings-missing",
                        )))
                    })?;
                    fence.stage_receipt(
                        &tx,
                        MutationFenceReceipt {
                            work,
                            event: &request.event_id,
                            kind,
                            work_revision: outcome.work.revision,
                            command_settings,
                        },
                    )?;
                }
                let at = self
                    .environment
                    .timestamp()
                    .map_err(WorkEngineError::Runtime)?;
                fence.check(&tx, work, &at)?;
            }
            Ok(outcome)
        })();
        tx.set_drop_behavior(DropBehavior::Ignore);
        match result {
            Ok(outcome) if outcome.replayed => {
                tx.rollback().map_err(WorkEngineError::ReplayRollback)?;
                Ok(outcome)
            }
            Ok(outcome) => match tx.commit() {
                Ok(()) => Ok(outcome),
                Err(cause) => {
                    let active_before_cleanup = !self.db.is_autocommit();
                    let cleanup = if self.db.is_autocommit() {
                        None
                    } else {
                        self.db.execute_batch("ROLLBACK").err().map(Box::new)
                    };
                    Err(WorkEngineError::Commit {
                        cause,
                        rolled_back: active_before_cleanup
                            && cleanup.is_none()
                            && self.db.is_autocommit(),
                        cleanup,
                        transaction_ended: self.db.is_autocommit(),
                    })
                }
            },
            Err(primary) => {
                if tx.is_autocommit() {
                    drop(tx);
                    return Err(WorkEngineError::TransactionLost {
                        primary: Box::new(primary),
                    });
                }
                match tx.rollback() {
                    Ok(()) => Err(primary),
                    Err(cleanup) => Err(WorkEngineError::Rollback {
                        primary: Box::new(primary),
                        cleanup,
                    }),
                }
            }
        }
    }
}
/// Exclude conflicting registered worker/control/review ownership in the same
/// transaction as closure, retry or instruction learning/rejection. Runtime statuses are protocol, not a policy
/// ceiling; unknown/unresolved ownership is never released by time or PID data.
/// Safe caller-owned refusal labels and original SQL/cleanup causes propagate.
#[cfg(target_os = "linux")]
fn require_native_task_idle(db: &Connection, work: &str, task: &str, worker_refusal: &'static str) -> Result<(), WorkEngineError> {
    let result = (|| {
        crate::root_store::require_validation_idle(db, work, task)?;
        crate::RootStore::require_idle_task_review(db, work, task)?;
        let active = crate::agent_launch::query(db,
            "SELECT id FROM main.agents WHERE work_id=? AND task_id=? AND status IN ('queued','starting','running','stopping') LIMIT 1",
            rusqlite::params![work, task], |rows| Ok(rows.next()?.is_some()))?;
        if active { return Err(crate::AgentLaunchError::Refused(worker_refusal)); }
        Ok(())
    })();
    result.map_err(|e| WorkEngineError::PlanningActivation(Box::new(e)))
}
struct Invocation<'a> {
    work: &'a str,
    kind: &'a str,
    request: &'a Request,
    intent: &'a [u8],
    mission: Option<(&'a crate::MissionControlRequest, crate::MissionControlAction)>,
    quota: Option<&'a crate::QuotaChange>,
    task_model: Option<&'a crate::TaskModelRequest>,
    task_report: Option<&'a crate::TaskReportRequest>,
    gate: Option<&'a crate::GateRequest>,
    budget: Option<&'a crate::BudgetChange>,
    quality_policy: Option<&'a crate::QualityPolicySnapshot>,
    planning: Option<(&'a crate::PlanningRequest, crate::PlanningControlAction)>,
    planning_settings: Option<&'a crate::EngineSettingsDocument>,
    fence: Option<&'a dyn MutationFence>,
}
/// Stage one supported Work mutation inside the caller-owned transaction.
/// Replay/revision guards precede edits; actual environment observations and
/// supplied Root limits govern the candidate. Returns staged/replayed state or
/// a typed guard, validation, observation, overflow or storage error. Git reads
/// and transaction writes may occur; this function never commits. The caller
/// must roll back/discard on error and separately reconcile a failed commit.
/// New planning claims also require no retained native call ownership in this
/// same transaction, before any reservation; exact claim replay stays pure.
/// Unstarted scoped abandonment/reopening additionally requires current settings
/// and no registered worker/review/validation ownership in that transaction.
/// No supplied stamp or declared gate authenticates a reviewer or runs an agent.
fn apply<E: WorkEnvironment>(
    tx: &mut Transaction<'_>,
    root: &Path,
    environment: &mut E,
    limits: &EngineMutationLimits,
    invocation: Invocation<'_>,
) -> Result<WorkMutationOutcome, WorkEngineError> {
    let Invocation {
        work,
        kind,
        request: r,
        intent,
        gate,
        mission,
        quota,
        task_model,
        task_report,
        budget,
        quality_policy,
        planning,
        planning_settings,
        fence,
    } = invocation;
    let inspection =
        inspect_recorded_mutation(tx, work, kind, &r.event_id, r.expected_revision, intent)
            .map_err(WorkEngineError::Guard)?;
    if kind != "gate"
        && mission.is_none()
        && planning.is_none()
        && budget.is_none()
        && quota.is_none()
        && task_model.is_none()
        && task_report.is_none()
    {
        let empty = Work::default();
        let target = match &inspection {
            MutationInspection::Replayed(_) => None,
            MutationInspection::Existing(w) => Some(w.as_ref()),
            MutationInspection::Create => Some(&empty),
        };
        if let Some(target) = target {
            crate::request_operation_fields::validate(target, kind, r)
                .map_err(|e| WorkEngineError::Definition(WorkDefinitionError::RequestFields(e)))?;
        }
    }
    let mut candidate = match inspection {
        MutationInspection::Replayed(w) => {
            return Ok(WorkMutationOutcome {
                work: *w,
                replayed: true,
                git_issues: vec![],
            })
        }
        MutationInspection::Existing(w) => *w,
        MutationInspection::Create => Work {
            schema_version: 1,
            id: environment
                .identifier("w-")
                .map_err(WorkEngineError::Runtime)?,
            created: environment.timestamp().map_err(WorkEngineError::Runtime)?,
            tasks: Some(vec![]),
            ..Default::default()
        },
    };
    if planning.is_some() {
        let settings = planning_settings.ok_or(WorkEngineError::PlanningControl(
            crate::PlanningControlError::Refused("planning-policy-not-supplied"),
        ))?;
        crate::validate_planning_intent_size(intent.len(), settings)
            .map_err(WorkEngineError::PlanningControl)?;
    }
    #[cfg(target_os = "linux")]
    if kind == "gate" || kind == "task.update" && matches!(r.status.as_str(), "accepted" | "running") {
        let task = if kind == "gate" {gate.ok_or(WorkEngineError::UnportedStatus)?.task_id.as_str()} else {r.id.as_str()};
        crate::root_store::require_validation_idle(tx,&candidate.id,task).map_err(|e|WorkEngineError::ValidationController(Box::new(e)))?;
    }
    let decision_origin =
        matches!(kind, "planning.decide" | "planning.lease-renew").then(|| candidate.clone());
    let before: BTreeMap<_, _> = if candidate.planning.is_some() {
        candidate
            .tasks
            .iter()
            .flatten()
            .map(|t| (t.id.clone(), t.status.clone()))
            .collect()
    } else {
        BTreeMap::new()
    };
    let mut git_issues = vec![];
    #[cfg(target_os = "linux")]
    let mut planning_inputs = None;
    match kind {
        "mission.pause" | "mission.unpause" | "mission.stop" | "mission.start" | "mission.resume" | "mission.priority" => {
            let (request, action)=mission.ok_or(WorkEngineError::UnportedStatus)?;
            let settings=planning_settings.ok_or_else(||WorkEngineError::Mission(Box::new(crate::MissionSupervisionError::Refused("mission-control-settings-required"))))?;
            let at=environment.timestamp().map_err(WorkEngineError::Runtime)?;
            let actor=environment.operator().map_err(WorkEngineError::Runtime)?;
            #[cfg(target_os="linux")]
            crate::root_store::stage_mission_control(tx,root,&mut candidate,action,request,settings,(&at,&actor)).map_err(|e|WorkEngineError::Mission(Box::new(e)))?;
            #[cfg(not(target_os="linux"))]
            {let _=(request,action,settings,at,actor);return Err(WorkEngineError::UnportedStatus);}
        }
        "quotas.configure" => {
            let change = quota.ok_or(WorkEngineError::UnportedStatus)?;
            let settings = planning_settings.ok_or(WorkEngineError::Quota(
                crate::AgentLaunchError::Refused("quota-settings-required")))?;
            let policy = settings.values.quota_reason_policy().map_err(|e|
                WorkEngineError::Quota(crate::AgentLaunchError::caused(e.reason(), e)))?;
            let at = environment.timestamp().map_err(WorkEngineError::Runtime)?;
            let actor = environment.operator().map_err(WorkEngineError::Runtime)?;
            crate::apply_quota_change(&mut candidate, change, &actor, &at, &policy)
                .map_err(WorkEngineError::Quota)?;
        }
        "task.submit" => {
            let request = task_report.ok_or(WorkEngineError::UnportedStatus)?;
            let settings = planning_settings.ok_or(WorkEngineError::TaskReport(
                crate::AgentLaunchError::Refused("task-report-settings-required"),
            ))?;
            crate::task_report::apply(tx, root, &mut candidate, request, settings, environment)
                .map_err(WorkEngineError::TaskReport)?;
        }
        "task.model" => {
            let request = task_model.ok_or(WorkEngineError::UnportedStatus)?;
            let settings = planning_settings.ok_or(WorkEngineError::TaskModel(
                crate::AgentLaunchError::Refused("task-model-settings-required"),
            ))?;
            crate::task_model::apply(tx, root, &mut candidate, request, settings, environment)
                .map_err(WorkEngineError::TaskModel)?;
        }
        "budget.configure" => {
            let change = budget.ok_or(WorkEngineError::UnportedStatus)?;
            let at = environment.timestamp().map_err(WorkEngineError::Runtime)?;
            let actor = environment.operator().map_err(WorkEngineError::Runtime)?;
            crate::financial_budget::write_budget(tx, &candidate.id, &change.budget, &actor, &at)
                .map_err(WorkEngineError::Budget)?;
        }
        "planning.enable"
        | "planning.claim"
        | "planning.decide"
        | "planning.lease-renew"
        | "planning.judge"
        | "planning.renew"
        | "planning.pause"
        | "planning.resume"
        | "planning.reject-instructions" => {
            let (request, action) = planning.ok_or(WorkEngineError::UnportedStatus)?;
            let settings = planning_settings.ok_or(WorkEngineError::PlanningControl(
                crate::PlanningControlError::Refused("planning-policy-not-supplied"),
            ))?;
            let at = environment.timestamp().map_err(WorkEngineError::Runtime)?;
            if action.requires_active_mission() {
                #[cfg(target_os = "linux")]
                crate::root_store::require_planning_mission_active(tx, &candidate.id).map_err(
                    |e| {
                        WorkEngineError::PlanningActivation(Box::new(
                            crate::AgentLaunchError::caused(e.code(), e),
                        ))
                    },
                )?;
                #[cfg(not(target_os = "linux"))]
                return Err(WorkEngineError::UnportedStatus);
            }
            if action == crate::PlanningControlAction::Claim {
                #[cfg(target_os = "linux")]
                {
                    // Replay already returned above. A logical lease expiry is
                    // not physical call completion; refuse before any new spend.
                    crate::planning_run_store::require_claim_available(
                        tx,
                        &candidate.id,
                        &request.scope,
                    )
                    .map_err(|e| WorkEngineError::PlanningActivation(Box::new(e)))?;
                    let (prepared, input) = crate::planning_activation::prepare_claim(
                        root, &candidate, request, &at, settings,
                    )
                    .map_err(|e| WorkEngineError::PlanningActivation(Box::new(e)))?;
                    candidate = prepared;
                    planning_inputs = Some(input);
                    crate::reserve_planning_call(
                        tx,
                        &candidate.id,
                        &request.scope,
                        &request.event_id,
                        &at,
                    )
                    .map_err(WorkEngineError::Budget)?;
                }
                #[cfg(not(target_os = "linux"))]
                {
                    return Err(WorkEngineError::UnportedStatus);
                }
            } else if action == crate::PlanningControlAction::LeaseRenew {
                candidate = crate::planning_activation::renew(&candidate, request, &at, settings)
                    .map_err(|e| WorkEngineError::PlanningActivation(Box::new(e)))?;
            } else if matches!(action, crate::PlanningControlAction::Judge | crate::PlanningControlAction::CycleRenew) {
                #[cfg(target_os = "linux")]
                crate::planning_run_store::require_claim_available(tx, &candidate.id, "root")
                    .map_err(|e| WorkEngineError::PlanningActivation(Box::new(crate::AgentLaunchError::caused("planning-judge-native-owner-active",e))))?;
                #[cfg(not(target_os = "linux"))]
                return Err(WorkEngineError::UnportedStatus);
                let kind = if action == crate::PlanningControlAction::Judge { "judge" } else { "renew" };
                candidate = crate::planning_judge::operator_candidate(&candidate, request, kind, &at, settings)
                    .map_err(|e| WorkEngineError::PlanningActivation(Box::new(e)))?;
            } else if action == crate::PlanningControlAction::RejectInstructions {
                let prepared=crate::instruction_learning::reject_candidate(&candidate,request,&at)
                    .map_err(|e|WorkEngineError::PlanningActivation(Box::new(e)))?;
                #[cfg(target_os="linux")]
                require_native_task_idle(tx,&candidate.id,&request.task_id,"instruction-worker-active")?;
                #[cfg(not(target_os="linux"))]
                return Err(WorkEngineError::UnportedStatus);
                candidate=prepared;
            } else if action == crate::PlanningControlAction::Decide {
                let prepared = crate::planning_decision::decide(root, &candidate, request, &at, settings)
                    .map_err(|e| WorkEngineError::PlanningActivation(Box::new(e)))?;
                #[cfg(target_os = "linux")]
                if request.operations.iter().flatten().any(|op| op.kind == "close") {
                    // Replay returned before preparation. Enumerate the original
                    // hierarchy: SQL ownership, not logical lease time, excludes
                    // another live/unknown planner in every descendant scope.
                    for scope in candidate.planning.iter().flat_map(|p| p.scopes.iter().flatten()) {
                        let ancestors = crate::planning_scope_ancestors(&candidate, &scope.id).map_err(|e| WorkEngineError::PlanningActivation(Box::new(crate::AgentLaunchError::caused("planning-closure-tree-invalid", e))))?;
                        if scope.id != request.scope && !ancestors.iter().any(|s| s.id == request.scope) { continue; }
                        for task in candidate.tasks.iter().flatten().filter(|t| t.scope_id == scope.id) {
                            require_native_task_idle(tx, &candidate.id, &task.id, "planning-closure-worker-active")?;
                        }
                        for run in crate::planning_run_store::scope_runs(tx, &candidate.id, &scope.id)
                            .map_err(|e| WorkEngineError::PlanningActivation(Box::new(e)))? {
                            if run.phase.retains_ownership() {
                                fence.ok_or_else(|| WorkEngineError::PlanningActivation(Box::new(crate::AgentLaunchError::Refused("planning-closure-native-run-active"))))?
                                    .finishing_planner(tx, &run, &request.event_id, &at)?;
                            }
                        }
                    }
                }
                #[cfg(target_os = "linux")]
                for op in request.operations.iter().flatten().filter(|op| matches!(op.kind.as_str(), "retry" | "review-retry" | "revise-instructions")) {
                    require_native_task_idle(tx, &candidate.id, &op.id, "planning-retry-worker-active")?;
                }
                candidate = prepared;
            } else {
                // Keep the original request in mutation intent/replay. Only this
                // trusted Root adapter may attach source-observed repository data.
                let mut local_request=request.clone();
                if action==crate::PlanningControlAction::Enable {local_request.repository=None;}
                let provider = if action == crate::PlanningControlAction::Enable
                    && !request.provider.is_empty()
                {
                    crate::planning_control::validate_planning_enable_target(&candidate, &local_request)
                        .map_err(WorkEngineError::PlanningControl)?;
                    #[cfg(target_os = "linux")]
                    {
                        Some(
                            crate::reviewer_bootstrap::prepare(root, request, &at, settings)
                                .map_err(|e| WorkEngineError::PlanningBootstrap(Box::new(e)))?,
                        )
                    }
                    #[cfg(not(target_os = "linux"))]
                    {
                        return Err(WorkEngineError::UnportedStatus);
                    }
                } else {
                    None
                };
                candidate = crate::planning_control::apply_planning_control_prepared(
                    &candidate,
                    action,
                    &local_request,
                    &at,
                    settings,
                    provider.as_ref(),
                )
                .map_err(WorkEngineError::PlanningControl)?;
                if action==crate::PlanningControlAction::Enable {
                    if let Some(repository)=request.repository.as_deref(){
                        #[cfg(target_os = "linux")]
                        {
                            let prepared=crate::managed_repository::prepare(root,&candidate.id,repository,settings)
                                .map_err(|e|WorkEngineError::PlanningBootstrap(Box::new(e)))?;
                            candidate.planning.as_mut().ok_or(WorkEngineError::UnportedStatus)?.repository=Some(Box::new(prepared));
                            crate::validate_planning_state(&candidate).map_err(|e|WorkEngineError::PlanningControl(crate::PlanningControlError::State(e)))?;
                        }
                        #[cfg(not(target_os = "linux"))]
                        return Err(WorkEngineError::UnportedStatus);
                    }
                }
            }
        }
        "work.create" | "checkpoint" => {
            if kind == "work.create" {
                apply_work_creation(&mut candidate, r, crate::GitState::default())
                    .map_err(WorkEngineError::Definition)?;
            } else {
                apply_checkpoint(&mut candidate, r, root, crate::GitState::default())
                    .map_err(WorkEngineError::Checkpoint)?;
            }
            let observed = environment.git(root).map_err(WorkEngineError::Runtime)?;
            git_issues = observed.issues;
            candidate.git = observed.state;
        }
        "work.update" => {
            apply_work_definition(&mut candidate, r).map_err(WorkEngineError::Definition)?
        }
        "task.add" => {
            // Root supplies the saved criterion-count policy; pure constructors
            // have no authority to invent a runtime fallback or hidden ceiling.
            if r.criteria
                .as_ref()
                .is_some_and(|c| c.len() > limits.task.max_criteria)
            {
                return Err(WorkEngineError::Task(crate::TaskDefinitionError::Refused(
                    "task-criteria-invalid",
                )));
            }
            let defaults = limits
                .task
                .manual_gate_phases
                .as_ref()
                .ok_or(WorkEngineError::Task(TaskDefinitionError::Refused(
                    "task-gate-defaults-not-configured",
                )))?;
            crate::work_definition::apply_task_addition_with_defaults(&mut candidate, r, defaults)
                .map_err(WorkEngineError::Definition)?
        }
        "ooda" => apply_ooda(&mut candidate, r).map_err(WorkEngineError::Definition)?,
        "gate" => {
            let refused = |reason| WorkEngineError::Gate(crate::GateContractError::Refused(reason));
            let gate = gate.ok_or_else(|| refused("gate-request-not-prepared"))?;
            let policy =
                quality_policy.ok_or_else(|| refused("gate-policy-observation-required"))?;
            let index = candidate
                .tasks
                .as_deref()
                .unwrap_or_default()
                .iter()
                .position(|t| t.id == gate.task_id)
                .ok_or_else(|| refused("gate-task-not-found"))?;
            let mut prepared = crate::gate_contract::prepare_gate_document(
                &candidate
                    .tasks
                    .as_ref()
                    .ok_or_else(|| refused("gate-task-not-found"))?[index],
                &gate.name,
                &gate.phase,
                gate.document.0.get().as_bytes(),
                root,
                policy,
            )
            .map_err(WorkEngineError::Gate)?;
            let at = environment.timestamp().map_err(WorkEngineError::Runtime)?;
            if !crate::text::nonempty(&at) {
                return Err(WorkEngineError::Gate(crate::GateContractError::Refused(
                    "gate-observation-time-required",
                )));
            }
            prepared
                .gate
                .as_mut()
                .ok_or_else(|| refused("gate-preparation-incomplete"))?
                .at = at;
            candidate
                .tasks
                .as_mut()
                .ok_or_else(|| refused("gate-task-not-found"))?[index] = prepared;
        }
        "task.update" => {
            if !r.status.is_empty() {
                let context = crate::TaskLifecycleContext {
                    root,
                    quality_policy,
                    task_limits: &limits.task,
                    validation_limits: &limits.policy,
                };
                let prepared = if candidate.planning.is_some() && matches!(r.status.as_str(), "accepted" | "todo" | "abandoned") {
                    let settings = planning_settings.ok_or(WorkEngineError::Lifecycle(
                        crate::TaskLifecycleError::HierarchyNotPorted,
                    ))?;
                    if r.status == "accepted" {
                        crate::task_lifecycle::transition_scoped_reviewed_task(
                            &candidate, r, &context, settings, environment,
                        )
                    } else {
                        #[cfg(target_os = "linux")]
                        require_native_task_idle(tx, &candidate.id, &r.id, "scoped-initial-decision-worker-active")?;
                        #[cfg(not(target_os = "linux"))]
                        return Err(WorkEngineError::Lifecycle(crate::TaskLifecycleError::HierarchyNotPorted));
                        crate::task_lifecycle::transition_scoped_initial_decision(
                            &candidate, r, &context, settings, environment,
                        )
                    }
                } else {
                    crate::transition_manual_task(&candidate, r, &context, environment)
                }.map_err(WorkEngineError::Lifecycle)?;
                let task = candidate
                    .tasks
                    .as_mut()
                    .and_then(|tasks| tasks.iter_mut().find(|t| t.id == r.id))
                    .ok_or(WorkEngineError::UnportedStatus)?;
                *task = prepared;
            } else {
                let at = if r.validation_policy.is_some() {
                    environment.timestamp().map_err(WorkEngineError::Runtime)?
                } else {
                    String::new()
                };
                let actor = if r.validation_policy.is_some() {
                    environment.operator().map_err(WorkEngineError::Runtime)?
                } else {
                    String::new()
                };
                let context = DefinitionContext {
                    authorized_at: &at,
                    actor: &actor,
                    task_limits: limits.task.clone(),
                    policy_limits: limits.policy.clone(),
                };
                apply_task_metadata(&mut candidate, r, &context).map_err(WorkEngineError::Task)?;
            }
        }
        _ => return Err(WorkEngineError::UnportedStatus),
    }
    // Shared task-status signals remain checked for manual and planning controls;
    // hierarchical task lifecycle transitions are still explicitly unported.
    apply_planning_validation_signals(&mut candidate, &before, &r.event_id, "")
        .map_err(WorkEngineError::Planning)?;
    candidate.revision = candidate
        .revision
        .checked_add(1)
        .ok_or(WorkEngineError::Overflow)?;
    candidate.updated = environment.timestamp().map_err(WorkEngineError::Runtime)?;
    if matches!(kind, "planning.claim" | "planning.lease-renew") {
        let (request, _) = planning.ok_or(WorkEngineError::UnportedStatus)?;
        crate::planning_activation::validate_publication(
            &candidate,
            &request.scope,
            &candidate.updated,
        )
        .map_err(|e| WorkEngineError::PlanningActivation(Box::new(e)))?;
    }
    if let Some(origin) = decision_origin {
        let (request, _) = planning.ok_or(WorkEngineError::UnportedStatus)?;
        crate::planning_decision::fence(&origin, request, &candidate.updated)
            .map_err(|e| WorkEngineError::PlanningActivation(Box::new(e)))?;
    }
    let staged = stage_recorded_change(
        tx,
        RecordedChange {
            work: &candidate,
            expected_revision: r.expected_revision,
            event: &r.event_id,
            kind,
            request: intent,
        },
    )
    .map_err(WorkEngineError::Change)?;
    match staged {
        RecordedChangeOutcome::Staged => {
            #[cfg(target_os = "linux")]
            if let Some(input) = planning_inputs {
                let (request, action) = planning.ok_or(WorkEngineError::UnportedStatus)?;
                if action != crate::PlanningControlAction::Claim {
                    return Err(WorkEngineError::UnportedStatus);
                }
                crate::planning_call_inputs::stage(tx, &candidate, request, &input)
                    .map_err(|e| WorkEngineError::PlanningActivation(Box::new(e)))?;
            }
            Ok(WorkMutationOutcome {
                work: candidate,
                replayed: false,
                git_issues,
            })
        }
        RecordedChangeOutcome::Replayed(work) => Ok(WorkMutationOutcome {
            work: *work,
            replayed: true,
            git_issues: vec![],
        }),
    }
}
