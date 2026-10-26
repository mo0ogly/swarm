//! Native local root and versioned settings. This is development integration,
//! not HTTP authentication, OS confinement or permission to share a live Go root.
#![cfg(unix)]
use crate::{
    canonical_project_root, local_storage::LocalDirectory, EngineSettingsChange,
    EngineSettingsDocument, EngineSettingsError, EngineSettingsPreview, EngineSettingsValues,
    LocalPathError, LocalStorageError, RecordedEvent, RecordedEventError, RecordedWorkError,
    RuntimeError, SettingCount, Work, WorkEngine, WorkEngineError, WorkEnvironment, WorkError,
    WorkMutationOutcome,
};
#[cfg(target_os = "linux")]
mod mission_status_review;
#[cfg(target_os = "linux")]
mod mission_report_relay;
#[cfg(target_os = "linux")]
pub use mission_report_relay::MissionReportRelayOutcome;
mod agent_log_history;
pub use agent_log_history::HistoricalAgentLog;
use rusqlite::{Connection, OpenFlags};
use std::{
    error::Error,
    fmt, fs,
    os::unix::fs::{DirBuilderExt, MetadataExt},
    path::{Path, PathBuf},
    time::Duration,
};
#[cfg(not(target_os = "linux"))]
use std::os::unix::fs::PermissionsExt;
#[cfg(target_os = "linux")]
mod planning_native;
#[cfg(target_os = "linux")]
mod planning_step;
#[cfg(target_os = "linux")]
mod task_validation;
#[cfg(target_os = "linux")]
mod review_preview;
#[cfg(target_os = "linux")]
mod native_review_context;
#[cfg(target_os = "linux")]
mod managed_publication;
#[cfg(target_os = "linux")]
mod native_transaction;
#[cfg(target_os = "linux")]
pub(crate) use managed_publication::PrecheckedAcceptance;
#[cfg(target_os = "linux")]
pub use managed_publication::{ManagedPublicationPreview, ManagedPublicationOutcome};
#[cfg(target_os = "linux")]
mod native_worker_retry;
#[cfg(target_os = "linux")]
mod review_recovery;
#[cfg(target_os = "linux")]
pub(crate) use native_worker_retry::WorkerCompletionInput;
#[cfg(target_os = "linux")]
pub use review_preview::{TaskReviewPreview, TaskReviewInputs, TaskReviewCheck, ReviewInputSource, TaskReviewOutcome, execute_review_gate};
#[cfg(target_os = "linux")]
pub use planning_native::{execute_planning_gate, PlanningRunOutcome};
#[cfg(target_os = "linux")]
pub use planning_step::PlanningStepOutcome;
#[cfg(target_os = "linux")]
pub use task_validation::{execute_validation_gate, TaskValidationPreview, TaskValidationOutcome, TaskValidationCandidate, NativeValidationEvidence};
#[cfg(target_os = "linux")]
pub(crate) use task_validation::require_validation_idle;
#[cfg(target_os = "linux")]
mod admin_budget;
#[cfg(target_os = "linux")]
pub use admin_budget::{AdminBudgetPolicy,AdminBudgetChange,AdminBudgetEntry,AdminBudgetView,AdminBudgetOutcome,AdminBudgetReservationRequest,AdminBudgetReservation};
mod material_path;
mod trust_store;
mod quotas;
pub use quotas::QuotaPreview;
const SETTINGS: &str = "engine-settings.json";
use crate::root_schema::INITIAL_SCHEMA;
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyRetry {
    schema_version: i64,
    busy_retries: i64,
    busy_retry_delay_ms: i64,
}
fn absent(error: &LocalStorageError) -> bool {
    error
        .source()
        .and_then(|e| e.downcast_ref::<rustix::io::Errno>())
        .is_some_and(|e| *e == rustix::io::Errno::NOENT)
}
fn initial_values(directory: &LocalDirectory) -> Result<EngineSettingsValues, RootStoreError> {
    let mut values =
        EngineSettingsValues::installation_defaults().map_err(RootStoreError::Settings)?;
    match directory.read_legacy_retry() {
        Ok(raw) => {
            let old: LegacyRetry = serde_json::from_slice(&raw)
                .map_err(|e| RootStoreError::Settings(EngineSettingsError::Format(e)))?;
            if old.schema_version != 1
                || !(0..=10).contains(&old.busy_retries)
                || !(0..=1000).contains(&old.busy_retry_delay_ms)
            {
                return Err(RootStoreError::Refused("legacy-storage-retry-invalid"));
            }
            values.storage_busy_retries = Some(SettingCount(old.busy_retries));
            values.storage_busy_retry_delay_ms = SettingCount(old.busy_retry_delay_ms);
        }
        Err(e) if absent(&e) => {}
        Err(e) => return Err(RootStoreError::Storage(e)),
    }
    values.validate().map_err(RootStoreError::Settings)?;
    Ok(values)
}
pub enum RootStoreError {
    Schema(Box<crate::RootSchemaError>),
    /// Actual SQLite descriptor observation failed; no pathname fallback is safe.
    /// Boxed cause retains the identity component's operation and cleanup causes.
    #[cfg(target_os = "linux")]
    DatabaseIdentity(Box<dyn Error + Send + Sync>),
    /// Content-free Biba/codec refusal; original typed cause retained.
    Trust(Box<crate::TrustError>),
    /// Recorded trust reply or visibly uncertain file publication. No rollback
    /// is inferred; consumer must expose this marker through close/output errors.
    TrustPublished { primary: Box<Self>, publication: &'static str },
    Input(crate::CommandInputError),
    Path(LocalPathError),
    Storage(LocalStorageError),
    Providers(crate::ProviderConfigurationError),
    Sql(rusqlite::Error),
    Settings(EngineSettingsError),
    Pricing(crate::PricingError),
    /// Preparation codec/IO boundary, including draft writes; cause retained.
    Preparation(Box<preparation_read::PreparationReadError>),
    /// Known preparation receipt survived a later close/encoding/output failure.
    /// Reread this event; no rollback, new mutation or plan approval is implied.
    PreparationRecorded { primary: Box<Self>, id: String, revision: i64 },
    Agent(crate::AgentStoreError),
    /// Read-only declared-event contract failure; original safe typed cause retained.
    Guard(crate::AgentGuardError),
    /// Native lifecycle SQLite policy exhausted; original cause/history retained.
    AgentStorageBusy {
        cause: Box<Self>,
        attempts: u64,
        settings_revision: SettingCount,
    },
    #[cfg(target_os = "linux")]
    Launch(Box<crate::AgentLaunchError>),
    RunLimits(crate::RuntimeLimitError),
    RunLimitsBody(WorkError),
    /// Lifecycle transaction committed before a later cleanup failure. Re-read;
    /// neither rollback nor task acceptance follows from this diagnostic.
    LifecycleRecorded {
        primary: Box<Self>,
    },
    /// WorkEngine returned known recorded state (including replay) before a late
    /// release/representation error. Preserve identity/revision and causes; no
    /// new-write, rollback, acceptance or permission inference follows.
    WorkRecorded {
        primary: Box<Self>,
        work: String,
        revision: i64,
    },
    /// Known saved configuration (including replay) or a visible publication
    /// survived a later durability/cleanup failure. Inspect the original state
    /// and re-read; this proves neither new bytes, crash durability nor authority
    /// for an unchanged retry.
    Published {
        primary: Box<Self>,
    },
    Runtime(RuntimeError),
    Work(RecordedWorkError),
    Events(RecordedEventError),
    Mutation(WorkEngineError),
    Encode(WorkError),
    Gate(crate::GateContractError),
    Context(crate::ProjectContextError),
    Workflow(crate::AgentWorkflowError),
    PlanningReport(crate::PlanningReportError),
    Review(crate::ReviewEvidenceError),
    Reply(crate::ReviewReplyError),
    ControlHistory(crate::ControlHistoryError),
    ValidationEvidence(crate::ValidationEvidenceError),
    Projection(crate::WorkProjectionError),
    #[cfg(target_os = "linux")]
    Git(crate::GitReportError),
    #[cfg(target_os = "linux")]
    GitMetadata(crate::GitMetadataError),
    #[cfg(target_os = "linux")]
    View(crate::WorkViewError),
    SchemaCommit {
        cause: rusqlite::Error,
        rolled_back: bool,
        cleanup: Option<Box<rusqlite::Error>>,
    },
    Refused(&'static str),
    Cleanup {
        primary: Option<Box<Self>>,
        cleanup: Box<Self>,
    },
    Busy {
        cause: Box<WorkEngineError>,
        attempts: u64,
        settings_revision: SettingCount,
    },
}
impl RootStoreError {
    /// Identify only a clean, durably recorded planner interruption. Cleanup,
    /// output, storage or other wrappers do not qualify. This is advisory local
    /// outcome data, never permission: a caller must also observe its own actual
    /// stop source and preserve known Work before returning a stopped command.
    #[cfg(target_os="linux")]
    pub fn recorded_planner_interruption(&self)->Option<(&str,i64)>{
        match self{
            Self::WorkRecorded{primary,work,revision}=>match primary.as_ref(){
                Self::Refused("planning-execution-interrupted")=>Some((work,*revision)),
                Self::WorkRecorded{..}=>primary.recorded_planner_interruption(),
                _=>None,
            },
            _=>None,
        }
    }
    /// Return the live role's clean, durably recorded interruption metadata.
    /// An outer cleanup/unmarked failure refuses classification; expected call
    /// failures inside the private publisher marker remain retained as causes.
    /// This is not permission or business success; require an actual own signal.
    #[cfg(target_os="linux")]
    pub fn recorded_execution_interruption(&self)->Option<(&'static str,&str,i64)>{
        match self{
            Self::WorkRecorded{primary,work,revision}=>match primary.as_ref(){
                Self::Refused("planning-execution-interrupted")=>Some(("planner",work,*revision)),
                Self::Runtime(runtime)=>std::error::Error::source(runtime).and_then(|e|e.downcast_ref::<crate::execution_interrupt::CompletedRoleInterrupt>()).map(|m|(m.role(),work.as_str(),*revision)),
                Self::WorkRecorded{..}=>primary.recorded_execution_interruption(),
                _=>None,
            },_=>None,
        }
    }
    /// Observe a typed configuration-preservation marker through compound Root
    /// cleanup. It can mean known prior/replayed data or visible uncertain bytes;
    /// false is not proof of rollback. Never infer authority, new write or retry.
    pub fn configuration_may_be_recorded(&self) -> bool {
        let mut pending = vec![self];
        while let Some(error) = pending.pop() {
            match error {
                Self::Published { .. } => return true,
                Self::Cleanup { primary, cleanup } => {
                    pending.push(cleanup);
                    if let Some(primary) = primary {
                        pending.push(primary);
                    }
                }
                _ => {}
            }
        }
        false
    }
    /// Find known preparation receipt metadata through every established error
    /// wrapper. None means unknown, never rollback; historical receipts grant no
    /// approval. No IO or implicit retry occurs.
    pub fn recorded_preparation(&self) -> Option<(&str, i64)> {
        let mut pending: Vec<&(dyn Error + 'static)> = vec![self];
        let mut seen: Vec<*const dyn Error> = vec![];
        let mut recorded: Option<(&str, i64)> = None;
        while let Some(error) = pending.pop() {
            let pointer = error as *const dyn Error;
            if seen.iter().any(|prior| std::ptr::eq(*prior, pointer)) {
                continue;
            }
            seen.push(pointer);
            if let Some(Self::PreparationRecorded { id: work, revision, .. }) = error.downcast_ref::<Self>() {
                match recorded.as_mut() {
                    Some((known, latest)) if *known == work.as_str() => {
                        *latest = (*latest).max(*revision);
                    }
                    None => recorded = Some((work.as_str(), *revision)),
                    _ => {}
                }
            }
            for (_, child) in crate::native_error_links(error).children.into_iter().rev() {
                pending.push(child);
            }
        }
        recorded
    }
    /// Inspect all typed causes for known recorded Work metadata without IO.
    /// Keep the first known Work identity and its greatest explicitly recorded
    /// revision, including nested launch/custom-IO/cleanup causes. Revisions of
    /// different Works are never compared. Typed-pointer cycle detection keeps
    /// transparent wrappers distinct without imposing an arbitrary depth cap.
    /// None means unknown, never rollback. A known revision is not necessarily
    /// current and grants no retry, execution authority or acceptance.
    pub fn recorded_work(&self) -> Option<(&str, i64)> {
        let mut pending: Vec<&(dyn Error + 'static)> = vec![self];
        let mut seen: Vec<*const dyn Error> = vec![];
        let mut recorded: Option<(&str, i64)> = None;
        while let Some(error) = pending.pop() {
            let pointer = error as *const dyn Error;
            if seen.iter().any(|prior| std::ptr::eq(*prior, pointer)) {
                continue;
            }
            seen.push(pointer);
            if let Some(Self::WorkRecorded { work, revision, .. }) = error.downcast_ref::<Self>() {
                match recorded.as_mut() {
                    Some((known, latest)) if *known == work.as_str() => {
                        *latest = (*latest).max(*revision);
                    }
                    None => recorded = Some((work.as_str(), *revision)),
                    _ => {}
                }
            }
            for (_, child) in crate::native_error_links(error).children.into_iter().rev() {
                pending.push(child);
            }
        }
        recorded
    }
    /// Find known/uncertain trust publication across the common wrapper graph,
    /// including custom IO and secondary cleanup causes. None means unknown,
    /// never rollback; this performs no IO, retry or authority transition.
    pub fn trust_publication(&self)->Option<&'static str>{
        let mut pending:Vec<&(dyn Error+'static)>=vec![self];let mut seen:Vec<*const dyn Error>=vec![];
        while let Some(error)=pending.pop(){let pointer=error as *const dyn Error;if seen.iter().any(|p|std::ptr::eq(*p,pointer)){continue}seen.push(pointer);
            if let Some(Self::TrustPublished{publication,..})=error.downcast_ref::<Self>(){return Some(publication)}
            for (_,child) in crate::native_error_links(error).children.into_iter().rev(){pending.push(child);}
        }None
    }
    /// Recover the typed trust refusal for historical CLI status selection.
    /// Walk every established wrapper without printing private cause strings.
    pub fn trust_failure(&self)->Option<&crate::TrustError>{
        let mut pending:Vec<&(dyn Error+'static)>=vec![self];let mut seen:Vec<*const dyn Error>=vec![];
        while let Some(error)=pending.pop(){let pointer=error as *const dyn Error;if seen.iter().any(|p|std::ptr::eq(*p,pointer)){continue}seen.push(pointer);
            if let Some(cause)=error.downcast_ref::<crate::TrustError>(){return Some(cause)}
            for (_,child) in crate::native_error_links(error).children.into_iter().rev(){pending.push(child);}
        }None
    }
    /// Stable safe failure family for CLI/API consumers, without supplied data.
    /// Classification changes no state and does not imply rollback, termination
    /// or acceptance; inspect retained commit/cleanup variants when applicable.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Trust(e)=>e.code(),
            Self::TrustPublished{..}=>"trust-state-preserved",
            Self::Schema(e) => e.code(),
            #[cfg(target_os = "linux")]
            Self::DatabaseIdentity(_) => "root-database-identity",
            Self::Input(e) => e.code(),
            Self::Path(e) => e.code(),
            Self::Storage(e) => e.code(),
            Self::Providers(e) => e.code(),
            Self::Settings(e) => e.reason(),
            Self::Pricing(e) => e.code(),
            Self::Preparation(e) => e.code(),
            Self::PreparationRecorded { .. } => "preparation-state-preserved",
            Self::Agent(e) => e.code(),
            Self::Guard(e) => e.code(),
            Self::AgentStorageBusy { .. } => "agent-storage-busy",
            #[cfg(target_os = "linux")]
            Self::Launch(e) => e.code(),
            Self::RunLimits(e) => e.code(),
            Self::RunLimitsBody(_) => "runtime-limit-document-invalid",
            Self::LifecycleRecorded { .. } => "agent-state-preserved",
            Self::WorkRecorded { .. } => "work-state-preserved",
            Self::Published { .. } => "root-state-preserved",
            Self::Runtime(e) => e.code(),
            Self::Work(e) => e.code(),
            Self::Events(e) => e.code(),
            Self::Mutation(e) => e.code(),
            Self::Encode(_) => "root-settings-encoding",
            Self::Gate(e) => e.code(),
            Self::Context(e) => e.code(),
            Self::Workflow(e) => e.code(),
            Self::PlanningReport(e) => e.code(),
            Self::Review(e) => e.code(),
            Self::Reply(e) => e.code(),
            Self::ControlHistory(e) => e.code(),
            Self::ValidationEvidence(e) => e.code(),
            Self::Projection(e) => e.code(),
            #[cfg(target_os = "linux")]
            Self::Git(e) => e.code(),
            #[cfg(target_os = "linux")]
            Self::GitMetadata(e) => e.code(),
            #[cfg(target_os = "linux")]
            Self::View(e) => e.code(),
            Self::SchemaCommit {
                rolled_back: true, ..
            } => "root-schema-commit-refused",
            Self::SchemaCommit { .. } => "root-schema-commit-uncertain",
            Self::Sql(_) => "root-storage",
            Self::Refused(s) => s,
            Self::Cleanup { .. } => "root-cleanup",
            Self::Busy { .. } => "root-storage-busy",
        }
    }
}
/// Render stable diagnostics, including safe static mutation-contract reasons.
/// Private SQL/IO/request details remain in typed sources; callers must inspect
/// commit/reconciliation state separately and must not infer rollback from text.
impl fmt::Display for RootStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mutation(e) => fmt::Display::fmt(e, f),
            Self::Pricing(e) => fmt::Display::fmt(e, f),
            Self::Agent(e) => fmt::Display::fmt(e, f),
            Self::AgentStorageBusy {
                attempts,
                settings_revision,
                ..
            } => write!(
                f,
                "{} (storage_attempts={attempts}; settings_revision={})",
                self.code(),
                settings_revision.0
            ),
            #[cfg(target_os = "linux")]
            Self::Launch(e) => fmt::Display::fmt(e, f),
            Self::LifecycleRecorded { .. } => {
                f.write_str("agent state recorded; cleanup failed; re-read before retrying")
            }
            Self::PreparationRecorded { id, revision, .. } => write!(f,"preparation-state-preserved ({id}, revision {revision}); re-read the same event before retrying"),
            Self::WorkRecorded { work, revision, .. } => write!(f,"work state recorded ({work}, revision {revision}); operation refused or failed; re-read before retrying"),
            Self::Published { .. } => {
                f.write_str("root-state-preserved; re-read recorded configuration before retrying")
            }
            Self::Context(e) => fmt::Display::fmt(e, f),
            Self::Workflow(e) => fmt::Display::fmt(e, f),
            Self::PlanningReport(e) => fmt::Display::fmt(e, f),
            Self::Review(e) => fmt::Display::fmt(e, f),
            Self::Reply(e) => fmt::Display::fmt(e, f),
            Self::ControlHistory(e) => fmt::Display::fmt(e, f),
            Self::ValidationEvidence(e) => fmt::Display::fmt(e, f),
            Self::Projection(e) => fmt::Display::fmt(e, f),
            #[cfg(target_os = "linux")]
            Self::Git(e) => fmt::Display::fmt(e, f),
            #[cfg(target_os = "linux")]
            Self::GitMetadata(e) => fmt::Display::fmt(e, f),
            #[cfg(target_os = "linux")]
            Self::View(e) => fmt::Display::fmt(e, f),
            _ => f.write_str(self.code()),
        }
    }
}
impl fmt::Debug for RootStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for RootStoreError {
    /// Expose the original typed primary cause without rendering private data.
    /// Compound variants retain cleanup separately; traversing this chain alone
    /// does not establish successful rollback or a completed lifecycle effect.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Trust(e)=>Some(e.as_ref()),
            Self::TrustPublished{primary,..}=>Some(primary.as_ref()),
            Self::Schema(e) => Some(e.as_ref()),
            #[cfg(target_os = "linux")]
            Self::DatabaseIdentity(e) => Some(e.as_ref()),
            Self::Input(e) => Some(e),
            Self::Path(e) => Some(e),
            Self::Storage(e) => Some(e),
            Self::Providers(e) => Some(e),
            Self::Sql(e) => Some(e),
            Self::Settings(e) => Some(e),
            Self::Pricing(e) => Some(e),
            Self::Preparation(e) => Some(e.as_ref()),
            Self::PreparationRecorded { primary, .. } => Some(primary.as_ref()),
            Self::Agent(e) => Some(e),
            Self::Guard(e) => Some(e),
            Self::AgentStorageBusy { cause, .. } => Some(cause.as_ref()),
            #[cfg(target_os = "linux")]
            Self::Launch(e) => Some(e.as_ref()),
            Self::RunLimits(e) => Some(e),
            Self::RunLimitsBody(e) => Some(e),
            Self::LifecycleRecorded { primary } => Some(primary.as_ref()),
            Self::WorkRecorded { primary, .. } => Some(primary.as_ref()),
            Self::Published { primary } => Some(primary.as_ref()),
            Self::Runtime(e) => Some(e),
            Self::Work(e) => Some(e),
            Self::Events(e) => Some(e),
            Self::Mutation(e) => Some(e),
            Self::Encode(e) => Some(e),
            Self::Gate(e) => Some(e),
            Self::Context(e) => Some(e),
            Self::Workflow(e) => Some(e),
            Self::PlanningReport(e) => Some(e),
            Self::Review(e) => Some(e),
            Self::Reply(e) => Some(e),
            Self::ControlHistory(e) => Some(e),
            Self::ValidationEvidence(e) => Some(e),
            Self::Projection(e) => Some(e),
            #[cfg(target_os = "linux")]
            Self::Git(e) => Some(e),
            #[cfg(target_os = "linux")]
            Self::GitMetadata(e) => Some(e),
            #[cfg(target_os = "linux")]
            Self::View(e) => Some(e),
            Self::SchemaCommit { cause, .. } => Some(cause),
            Self::Cleanup {
                primary: Some(e), ..
            } => Some(e.as_ref()),
            Self::Cleanup { cleanup, .. } => Some(cleanup.as_ref()),
            Self::Busy { cause, .. } => Some(cause.as_ref()),
            Self::Refused(_) => None,
        }
    }
}
/// Join an operation result with mandatory cleanup without losing either cause.
/// A cleanup failure turns success into an error; paired failures retain both.
/// This performs no IO, retry or rollback itself and establishes no committed
/// state: callers must separately report already-persisted effects when relevant.
pub(crate) fn finished<T>(
    result: Result<T, RootStoreError>,
    cleanup: Result<(), RootStoreError>,
) -> Result<T, RootStoreError> {
    match (result, cleanup) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(RootStoreError::Cleanup {
            primary: None,
            cleanup: Box::new(e),
        }),
        (Err(primary), Err(cleanup)) => Err(RootStoreError::Cleanup {
            primary: Some(Box::new(primary)),
            cleanup: Box::new(cleanup),
        }),
    }
}
fn io_error(e: std::io::Error) -> RootStoreError {
    RootStoreError::Runtime(RuntimeError::new(crate::RuntimeOperation::Output, e))
}
fn local_regular(path: &Path, private: bool) -> Result<(), RootStoreError> {
    let m = fs::symlink_metadata(path).map_err(io_error)?;
    if !m.is_file()
        || m.uid() != nix::unistd::geteuid().as_raw()
        || m.nlink() != 1
        || (private && m.mode() & 0o077 != 0)
    {
        return Err(RootStoreError::Refused("root-owned-regular-file-required"));
    }
    Ok(())
}
#[cfg(target_os = "linux")]
#[path = "provider_configuration_store.rs"]
mod provider_configuration_store;
#[cfg(target_os = "linux")]
#[path = "provider_policy_store.rs"]
mod provider_policy_store;

/// Owned datastore binding retaining both .swarm and the connected SQLite file.
/// The directory descriptor and O_PATH database witness prevent inode reuse
/// while comparing subsequent opens. Not deserializable from request data and
/// not a cryptographic identity, path lock or same-UID confinement. The owner
/// must explicitly close both handles; possession alone authorizes no operation.
#[cfg(target_os = "linux")]
pub struct RootBinding {
    directory: LocalDirectory,
    database: crate::connected_database_identity::ConnectedDatabaseIdentity,
}
/// Physical storage observations emitted by an actual retained Root binding.
/// Serialization preserves canonical historical linkage, never permission:
/// consumers compare against their own connected DB and owned directory.
#[cfg(target_os = "linux")]
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RootInstanceIdentity {
    pub(crate) directory: (u64, u64),
    pub(crate) database: (u64, u64),
}
/// Lift identity failures into the established Root primary/cleanup tree so
/// public error traversal retains secondary close/finalize causes as well.
/// This is classification only: it performs no IO or retry and proves no rollback.
#[cfg(target_os = "linux")]
fn database_identity_error(mut error: crate::connected_database_identity::ConnectedDatabaseIdentityError)
    -> RootStoreError
{
    let cleanup = error.cleanup.take();
    let primary = RootStoreError::DatabaseIdentity(Box::new(error));
    match cleanup {
        Some(cleanup) => RootStoreError::Cleanup {
            primary: Some(Box::new(primary)),
            cleanup: Box::new(database_identity_error(*cleanup)),
        },
        None => primary,
    }
}
/// Capture the actual SQLite connection and check its association with the owned
/// directory's state.db. A pathname stat alone cannot establish this association.
/// The temporary O_PATH witness is always explicitly closed; both observation
/// and cleanup errors survive. No mutation, permission or OS confinement follows.
#[cfg(target_os = "linux")]
pub(crate) fn observe_instance(db: &Connection, directory: &LocalDirectory)
    -> Result<RootInstanceIdentity, RootStoreError>
{
    let database = crate::connected_database_identity::ConnectedDatabaseIdentity::capture(db)
        .map_err(database_identity_error)?;
    let result = (|| {
        let observed = database.identity();
        if directory.database_identity().map_err(RootStoreError::Storage)? != observed {
            return Err(RootStoreError::Refused("root-database-instance-changed"));
        }
        Ok(RootInstanceIdentity {
            directory: directory.identity().map_err(RootStoreError::Storage)?,
            database: observed,
        })
    })();
    finished(result, database.close().map_err(database_identity_error))
}
/// Read operator values through the owned directory, without SQL. Only genuine
/// absence uses initialization defaults/legacy import. Invalid settings and
/// storage/close failures retain their causes; no binding or right follows.
#[cfg(target_os = "linux")]
fn opening_values(directory: &LocalDirectory) -> Result<EngineSettingsValues, RootStoreError> {
    match directory.read(SETTINGS) {
        Ok(raw) => Ok(EngineSettingsDocument::decode(&raw)
            .map_err(RootStoreError::Settings)?.values),
        Err(error) if absent(&error) => initial_values(directory),
        Err(error) => Err(RootStoreError::Storage(error)),
    }
}
/// Apply the saved timeout before the first identity SELECT, using only the
/// already-owned settings directory. Initialization uses the same validated
/// defaults as open_mode. No rights follow; SQL/format/storage/close causes
/// survive. Later observations preserve policy; caller closes the connection.
#[cfg(target_os = "linux")]
fn bootstrap_instance(db: &Connection, directory: &LocalDirectory)
    -> Result<RootInstanceIdentity, RootStoreError>
{
    let values = opening_values(directory)?;
    db.busy_timeout(Duration::from_millis(values.storage_busy_timeout_ms.0 as u64))
        .map_err(RootStoreError::Sql)?;
    observe_instance(db, directory)
}
// Component qualification only. Production gates retain their historical wire
// protocol until explicit consumer/owner wiring and fresh public evidence.
#[cfg(all(test, target_os = "linux"))]
mod gate_root_channel;
#[cfg(target_os = "linux")]
impl RootBinding {
    /// Borrow the actual directory and SQLite O_PATH witnesses for the private
    /// test-only gate component. This neither transfers ownership nor grants an
    /// execution; the sender still owes its real job/peer/permission checks.
    #[cfg(test)]
    fn gate_descriptors(&self) -> [std::os::fd::BorrowedFd<'_>; 2] {
        [self.directory.gate_descriptor(), self.database.gate_descriptor()]
    }

    /// Adopt two received gate descriptors before opening the child database.
    /// Validate DB O_PATH and real directory modes/ownership, retaining every
    /// partial-adoption close cause. The caller must separately verify its real
    /// sender and compare this binding to the actual Root before material reads.
    #[cfg(test)]
    fn adopt_gate_descriptors(directory: std::os::fd::OwnedFd, database: std::os::fd::OwnedFd)
        -> Result<Self, RootStoreError>
    {
        let database = match crate::connected_database_identity::ConnectedDatabaseIdentity::adopt_gate_descriptor(database) {
            Ok(database) => database,
            Err(error) => return finished(Err(database_identity_error(error)),
                nix::unistd::close(directory).map_err(|error| RootStoreError::Launch(Box::new(crate::AgentLaunchError::caused("gate-root-directory-close", error))))),
        };
        match LocalDirectory::adopt_gate_descriptor(directory) {
            Ok(directory) => Ok(Self { directory, database }),
            Err(error) => finished(Err(RootStoreError::Storage(error)),
                database.close().map_err(database_identity_error)),
        }
    }
    /// Borrow the owned original datastore directory for descriptor-relative
    /// resources. This is an internal observation capability, not a grant or a
    /// pathname refresh; the binding owner remains responsible for checked close.
    pub(crate) fn directory(&self) -> &LocalDirectory {
        &self.directory
    }
    /// Compare both retained storage identities with an actual connected Root.
    /// Returns false on another instance; observation errors retain safe causes.
    /// No data read/write, path confinement or process/agent identity is proved.
    pub fn matches(&self, store: &RootStore) -> Result<bool, RootStoreError> {
        Ok(self.instance()? == store.instance()?)
    }
    /// Release both owned CLOEXEC handles explicitly, preserving both close errors.
    /// No data mutation or acceptance follows; using a closed binding is prevented
    /// by ownership. Implicit Drop is not evidence of a successful checked close.
    pub fn close(self) -> Result<(), RootStoreError> {
        let database = self.database.close()
            .map_err(database_identity_error);
        finished(database, self.directory.close().map_err(RootStoreError::Storage))
    }
    /// Read the two retained descriptor identities, not the current pathname.
    /// The owned handles prevent inode reuse for this binding's lifetime. This
    /// observes provenance; callers still owe current permission and fencing.
    pub(crate) fn instance(&self) -> Result<RootInstanceIdentity, RootStoreError> {
        Ok(RootInstanceIdentity {
            directory: self.directory.identity().map_err(RootStoreError::Storage)?,
            database: self.database.identity(),
        })
    }
}
#[cfg(target_os = "linux")]
#[path = "mission_controls.rs"]
mod mission_controls;
#[cfg(target_os = "linux")]
#[path = "mission_control_write.rs"]
mod mission_control_write;

#[path = "mission_start.rs"]
mod mission_start;

#[cfg(target_os = "linux")]
#[path = "mission_task_order.rs"]
mod mission_task_order;
#[cfg(target_os="linux")]
#[path="mission_candidates.rs"]
mod mission_candidates;
#[cfg(target_os = "linux")]
pub use mission_task_order::{MissionTaskOrder, MissionTaskPosition};

#[cfg(target_os = "linux")]
#[path = "mission_workspace_queue.rs"]
mod mission_workspace_queue;
#[cfg(target_os = "linux")]
pub use mission_workspace_queue::{MissionWorkspaceQueue, MissionWorkspaceTurn, MissionWorkspaceBlocker};

#[cfg(target_os = "linux")]
#[path = "mission_worker_dispatch.rs"]
mod mission_worker_dispatch;
#[cfg(target_os = "linux")]
pub use mission_worker_dispatch::{
    MissionWorkerDispatchRequest, MissionWorkerDispatchOutcome, MissionWorkerDispatchFailure,
    MissionDispatchCause, MissionDispatchQueueEffect, MissionDispatchPreparationEffect,
    MissionDispatchResourceRefusal,
};

#[path = "mission_agent.rs"]
pub(crate) mod mission_agent;
#[path = "mission_cost_guard.rs"]
mod mission_cost_guard;
pub use mission_agent::MissionWorkerRequest;
#[cfg(target_os = "linux")]
pub(crate) use mission_control_write::stage as stage_mission_control;
// Shared transaction guard; global controls use the same existing boundary.
#[cfg(target_os = "linux")]
pub(crate) use mission_controls::require_planning_mission_active;
#[cfg(target_os = "linux")]
pub use mission_controls::{
    MissionAutonomy, MissionConsentObservation, MissionControlSettings, MissionControlsObservation,
    MissionDispatchObservation,
};
#[cfg(target_os = "linux")]
pub(crate) mod agent_execution_origin;
#[cfg(target_os = "linux")]
mod mission_producer;
#[cfg(target_os = "linux")]
#[path = "mission_supervision_store.rs"]
mod mission_supervision_store;
#[cfg(target_os = "linux")]
pub(crate) mod reported_costs;
#[cfg(target_os = "linux")]
pub use reported_costs::{CostSummary, CostTotal};
#[cfg(target_os = "linux")]
mod mission_status_read;
#[cfg(target_os = "linux")]
mod mission_status_evidence;
#[cfg(target_os = "linux")]
mod mission_status_results;
#[cfg(target_os = "linux")]
mod mission_status_historical;
#[cfg(target_os = "linux")]
mod mission_status_readiness;
#[cfg(target_os = "linux")]
mod mission_status_reports;
#[cfg(target_os = "linux")]
mod mission_status_activity;
#[cfg(target_os = "linux")]
pub use mission_status_historical::MissionStatusObservation;
#[cfg(target_os = "linux")]
pub use mission_status_results::MissionResultObservation;
#[cfg(target_os = "linux")]
pub use mission_status_evidence::TaskFreshnessEvidence;
#[cfg(target_os = "linux")]
pub use mission_status_read::{MissionStatusInputs, MissionStatusVisit, MissionStatusPlanningCall, MissionStatusExchange, MissionStatusExchangeArtifact};
#[cfg(target_os = "linux")]
pub use mission_supervision_store::MissionWorkerRecovery;
pub use mission_supervision_store::{MissionQueueRegistration, MissionCandidateRefusal, MissionWorkerAdoptionRequest, MissionWorkerAdoptionOutcome};
#[cfg(target_os = "linux")]
pub use mission_supervision_store::{
    MissionWorkerContinuationObservation, MissionWorkerContinuationEntry,
    MissionWorkerContinuationState, MissionContinuationFailure, MissionContinuationProgress,
};
#[cfg(target_os="linux")]
pub use mission_supervision_store::{MissionLease, MissionWorkReceipt, MissionCycleFailure, MissionCycleMonitor, MissionCycleCause, MissionCycleDiagnostics};
#[cfg(target_os = "linux")]
#[path = "mission_supervision_read.rs"]
mod mission_supervision_read;
#[cfg(target_os = "linux")]
pub use mission_supervision_read::{MissionSupervisionObservation, MissionSupervisionState};
pub struct RootStore {
    root: PathBuf,
    db: Connection,
    directory: LocalDirectory,
    writable: bool,
    #[cfg(target_os="linux")]
    interruption: Option<crate::ExecutionInterrupt>,
}
/// Join a known Work result with release, preserving returned state on any late
/// error (including replay). Never claim a new mutation or known state for a
/// failed/uncertain command. Retain release causes; checked attempt arithmetic
/// is representation only, not a retry policy. No IO/retry/rollback occurs here.
fn work_finished(
    result: Result<crate::WorkMutationOutcome, RootStoreError>,
    cleanup: Result<(), RootStoreError>,
    settings_revision: SettingCount,
    previous_attempts: u64,
) -> Result<RootMutationOutcome, RootStoreError> {
    let recorded = result
        .as_ref()
        .ok()
        .map(|v| (v.work.id.clone(), v.work.revision));
    let joined = finished(result, cleanup).and_then(|result| {
        let storage_attempts = previous_attempts
            .checked_add(1)
            .ok_or(RootStoreError::Refused("storage-retry-representation"))?;
        Ok(RootMutationOutcome {
            result,
            settings_revision,
            storage_attempts,
        })
    });
    match (joined, recorded) {
        (Err(primary), Some((work, revision))) => Err(RootStoreError::WorkRecorded {
            primary: Box::new(primary),
            work,
            revision,
        }),
        (result, _) => result,
    }
}
/// Join apply/replay settings outcome with release. Known saved state and visible
/// publication with uncertain durability remain top-level preserved configuration;
/// preview/read/before-write failures are never marked as publication. Original
/// typed causes survive; this aggregation performs no IO/retry/rollback.
fn settings_finished(
    result: Result<(EngineSettingsDocument, bool), RootStoreError>,
    cleanup: Result<(), RootStoreError>,
) -> Result<(EngineSettingsDocument, bool), RootStoreError> {
    let recorded = result.is_ok()
        || result.as_ref().err().is_some_and(|error| match error {
            RootStoreError::Published { .. } => true,
            RootStoreError::Storage(e) => {
                e.publication != crate::local_storage::PublicationState::NotPublished
            }
            _ => false,
        });
    match finished(result, cleanup) {
        Err(error @ RootStoreError::Published { .. }) => Err(error),
        Err(primary) if recorded => Err(RootStoreError::Published {
            primary: Box::new(primary),
        }),
        result => result,
    }
}
pub struct RootMutationOutcome {
    pub result: WorkMutationOutcome,
    pub settings_revision: SettingCount,
    pub storage_attempts: u64,
}
impl RootStore {
    /// Scope a local cooperative interruption source to this Root object only.
    /// This changes no persisted settings/consent, creates no permission and never
    /// reaches unrelated Root connections (including heartbeat). The callback
    /// retains all normal admission/cleanup/publication duties. Restore prior
    /// scope even on unwind, then propagate the original panic unchanged.
    #[cfg(target_os="linux")]
    pub fn with_execution_interrupt<T>(&mut self,source:&crate::ExecutionInterrupt,operation:impl FnOnce(&mut Self)->T)->T{
        let previous=self.interruption.replace(source.clone());
        let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||operation(self)));
        self.interruption=previous;
        match result{Ok(value)=>value,Err(payload)=>std::panic::resume_unwind(payload)}
    }
    /// Open an existing canonical project Root with its private local layout.
    /// `writable` selects read/write access; it never initializes a missing Root
    /// or grants an action. Linux requires FILESTAT and associates the actual
    /// connected SQLite inode with the owned directory, refusing missing
    /// capability, wrong instance or unsafe layout. Observation/cleanup errors
    /// retain their causes. SQLite connection/schema reads can perform their own
    /// journal recovery: nominal read-only use is not a blanket zero-effect proof.
    /// This is local instance binding, not hostile same-UID OS confinement.
    pub fn open(root: &Path, writable: bool) -> Result<Self, RootStoreError> {
        Self::open_mode(root, writable, false)
    }
    /// Open/connect the actual Root and validate its physical/schema association.
    /// Initialization additionally prepares private physical storage before SQL
    /// initialization and may publish files/schema/settings that cannot co-commit.
    /// Existing opens do not initialize. Retained descriptors and FILESTAT bind
    /// the connection, while every failure closes acquired resources with causes.
    /// The bootstrap zero busy timeout precedes even the identity SELECT; saved
    /// Admin values replace it before ordinary Root SQL. Later binding reads
    /// preserve that effective policy rather than resetting it.
    /// SQLite's own recovery effects still need separate qualification; no path
    /// lock, business grant or global rollback follows from an Err.
    fn open_mode(root: &Path, writable: bool, initialize: bool) -> Result<Self, RootStoreError> {
        let root = canonical_project_root(root).map_err(RootStoreError::Path)?;
        let dir = root.join(".swarm");
        if initialize {
            match fs::symlink_metadata(&dir) {
                Ok(m) if !m.is_dir() => {
                    return Err(RootStoreError::Refused("root-local-directory-required"))
                }
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => fs::DirBuilder::new()
                    .mode(0o700)
                    .create(&dir)
                    .map_err(io_error)?,
                Err(e) => return Err(io_error(e)),
            }
        }
        let directory = LocalDirectory::open(&dir).map_err(RootStoreError::Storage)?;
        if let Err(e) = directory.validate_layout() {
            return finished(
                Err(RootStoreError::Storage(e)),
                directory.close().map_err(RootStoreError::Storage),
            );
        }
        #[cfg(target_os = "linux")]
        if initialize {
            if let Err(error) = directory.prepare_database_for_initialization() {
                return finished(Err(RootStoreError::Storage(error)),
                    directory.close().map_err(RootStoreError::Storage));
            }
        }
        let path = dir.join("state.db");
        let exists = match fs::symlink_metadata(&path) {
            Ok(_) => {
                if let Err(error) = local_regular(&path, !initialize || cfg!(target_os = "linux")) {
                    return finished(
                        Err(error),
                        directory.close().map_err(RootStoreError::Storage),
                    );
                }
                true
            }
            Err(e) if initialize && e.kind() == std::io::ErrorKind::NotFound => false,
            Err(e) => {
                return finished(
                    Err(io_error(e)),
                    directory.close().map_err(RootStoreError::Storage),
                )
            }
        };
        if !exists && !initialize {
            return finished(
                Err(RootStoreError::Refused("root-initialization-required")),
                directory.close().map_err(RootStoreError::Storage),
            );
        }
        let flags = if writable {
            OpenFlags::SQLITE_OPEN_READ_WRITE
        } else {
            OpenFlags::SQLITE_OPEN_READ_ONLY
        } | if initialize && !cfg!(target_os = "linux") {
            OpenFlags::SQLITE_OPEN_CREATE
        } else {
            OpenFlags::empty()
        } | OpenFlags::SQLITE_OPEN_NOFOLLOW
            | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let mut db = match Connection::open_with_flags(&path, flags) {
            Ok(db) => db,
            Err(e) => {
                return finished(
                    Err(RootStoreError::Sql(e)),
                    directory.close().map_err(RootStoreError::Storage),
                )
            }
        };
        // Validate the connected main file against the retained directory before
        // user_version, settings-derived PRAGMAs or schema writes. Initialization
        // published/repaired private state.db before SQLite connected, so no chmod
        // through a mutable project pathname is required after this boundary.
        #[cfg(target_os = "linux")]
        if let Err(error) = bootstrap_instance(&db, &directory) {
            let result = finished(Err(error), db.close().map_err(|(_, error)| RootStoreError::Sql(error)));
            return finished(result, directory.close().map_err(RootStoreError::Storage));
        }
        let prepared = (|| {
            // Linux established this before its initial diagnostic SELECT.
            // Other platforms have no diagnostic capture, but must still replace
            // the constructor handler before their first application SQL.
            #[cfg(not(target_os = "linux"))]
            db.busy_timeout(Duration::ZERO).map_err(RootStoreError::Sql)?;
            let values = match directory.read(SETTINGS) {
                Ok(raw) => {
                    EngineSettingsDocument::decode(&raw)
                        .map_err(RootStoreError::Settings)?
                        .values
                }
                Err(error) => {
                    if error
                        .source()
                        .and_then(|e| e.downcast_ref::<rustix::io::Errno>())
                        .is_some_and(|e| *e == rustix::io::Errno::NOENT)
                    {
                        initial_values(&directory)?
                    } else {
                        return Err(RootStoreError::Storage(error));
                    }
                }
            };
            db.busy_timeout(Duration::from_millis(
                values.storage_busy_timeout_ms.0 as u64,
            ))
            .map_err(RootStoreError::Sql)?;
            let version: i64 = db
                .query_row("PRAGMA main.user_version", [], |r| r.get(0))
                .map_err(RootStoreError::Sql)?;
            if version != 37 && !(initialize && version == 0) {
                return Err(RootStoreError::Refused(if (1..37).contains(&version) {
                    "storage_upgrade_required"
                } else {
                    "storage_version_conflict"
                }));
            }
            db.pragma_update(None, "foreign_keys", true)
                .map_err(RootStoreError::Sql)?;
            db.pragma_update(None, "synchronous", "FULL")
                .map_err(RootStoreError::Sql)?;
            crate::register_identity_plan_projection(&db).map_err(RootStoreError::Sql)?;
            if version == 0 {
                let count: i64 = db
                    .query_row(
                        "SELECT count(*)FROM main.sqlite_schema WHERE name NOT LIKE 'sqlite_%'",
                        [],
                        |r| r.get(0),
                    )
                    .map_err(RootStoreError::Sql)?;
                if count != 0 {
                    return Err(RootStoreError::Refused(
                        "root-nonempty-uninitialized-storage",
                    ));
                }
                let mut tx = db.transaction().map_err(RootStoreError::Sql)?;
                tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
                if let Err(primary) = tx.execute_batch(INITIAL_SCHEMA) {
                    return finished(
                        Err(RootStoreError::Sql(primary)),
                        tx.rollback().map_err(RootStoreError::Sql),
                    );
                }
                if let Err(cause) = tx.commit() {
                    let active = !db.is_autocommit();
                    let cleanup = if active {
                        db.execute_batch("ROLLBACK").err().map(Box::new)
                    } else {
                        None
                    };
                    return Err(RootStoreError::SchemaCommit {
                        cause,
                        rolled_back: active && cleanup.is_none() && db.is_autocommit(),
                        cleanup,
                    });
                }
            }
            crate::root_schema::verify_root_schema(&db)
                .map_err(|e| RootStoreError::Schema(Box::new(e)))?;
            #[cfg(not(target_os = "linux"))]
            if initialize {
                fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).map_err(io_error)?;
            }
            Ok(())
        })();
        if let Err(error) = prepared {
            let result = finished(
                Err(error),
                db.close().map_err(|(_, e)| RootStoreError::Sql(e)),
            );
            return finished(result, directory.close().map_err(RootStoreError::Storage));
        }
        Ok(Self {
            root,
            db,
            directory,
            writable,
            #[cfg(target_os="linux")]
            interruption:None,
        })
    }
    /// Idempotent local initialization; filesystem and SQLite cannot co-commit.
    /// If a later publication fails, preserve completed schema/data and repeat
    /// initialization only after diagnosing that failure. No history is reset.
    /// Linux prepares a private main file through the retained directory before
    /// connecting, then verifies connected DB/directory association before any
    /// schema write. Permission repair uses an owned O_PATH procfs magic link;
    /// partial file publication or repair is preserved if a later stage refuses.
    pub fn initialize<E: WorkEnvironment>(
        root: &Path,
        environment: &mut E,
    ) -> Result<Self, RootStoreError> {
        let store = Self::open_mode(root, true, true)?;
        let result = (|| {
            let reservation = store
                .directory
                .reserve(true, true)
                .map_err(RootStoreError::Storage)?;
            let result = (|| {
                match store.directory.read(SETTINGS) {
                    Ok(bytes) => {
                        EngineSettingsDocument::decode(&bytes).map_err(RootStoreError::Settings)?;
                    }
                    Err(e) => {
                        let absent = e
                            .source()
                            .and_then(|cause| cause.downcast_ref::<rustix::io::Errno>())
                            .is_some_and(|e| *e == rustix::io::Errno::NOENT);
                        if !absent {
                            return Err(RootStoreError::Storage(e));
                        }
                        let doc = EngineSettingsDocument::initial(
                            initial_values(&store.directory)?,
                            environment.operator().map_err(RootStoreError::Runtime)?,
                            environment.timestamp().map_err(RootStoreError::Runtime)?,
                        )
                        .map_err(RootStoreError::Settings)?;
                        let bytes = crate::encode_model(&doc).map_err(RootStoreError::Encode)?;
                        store
                            .directory
                            .publish(SETTINGS, &bytes)
                            .map_err(RootStoreError::Storage)?;
                    }
                }
                store
                    .directory
                    .initialize_layout()
                    .map_err(RootStoreError::Storage)?;
                Ok(())
            })();
            finished(result, reservation.close().map_err(RootStoreError::Storage))
        })();
        if let Err(error) = result {
            return finished(Err(error), store.close());
        }
        Ok(store)
    }
    /// Whether a telemetry ingest subject is still running (LIVE-UX): an agent
    /// id (status queued/starting/running/stopping), `review:<job id>` or
    /// `planning:<claim id>` (job phase claimed/registered/permitted/
    /// unresolved). Unknown subjects, and job tables absent from an older
    /// Root, are not running. Other read failures keep their cause. Read only.
    pub fn telemetry_subject_active(&self, subject: &str) -> Result<bool, RootStoreError> {
        let job = |table: &str, key: &str, id: &str| -> Result<bool, RootStoreError> {
            let sql = format!("SELECT phase FROM {table} WHERE {key} = ?1");
            match self.db.query_row(&sql, [id], |r| r.get::<_, String>(0)) {
                Ok(phase) => Ok(crate::telemetry_runs::ACTIVE_RUN_PHASES.contains(&phase.as_str())),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
                Err(rusqlite::Error::SqliteFailure(_, Some(message))) if message.starts_with("no such table") => Ok(false),
                Err(e) => Err(RootStoreError::Launch(Box::new(crate::AgentLaunchError::caused("telemetry-subject-read", e)))),
            }
        };
        if let Some(id) = subject.strip_prefix("review:") {
            return job("native_review_jobs", "id", id);
        }
        if let Some(claim) = subject.strip_prefix("planning:") {
            return job("native_planning_runs", "claim_id", claim);
        }
        match self.read_agent(subject) {
            Ok(agent) => Ok(matches!(agent.status.as_str(), "queued" | "starting" | "running" | "stopping")),
            Err(RootStoreError::Agent(crate::AgentStoreError::Missing)) => Ok(false),
            Err(e) => Err(e),
        }
    }
    /// Review jobs and planning runs of `work` that may carry provider
    /// telemetry (LIVE-UX console), newest first per kind; see
    /// [`crate::TelemetryRun`]. `only` keeps one subject. Job tables absent
    /// from an older Root give no row; other read failures keep their cause
    /// (`telemetry-runs-read`). Read only: no table creation, no write.
    pub fn telemetry_runs(&self, work: &str, only: Option<&str>) -> Result<Vec<crate::TelemetryRun>, RootStoreError> {
        crate::telemetry_runs::read_telemetry_runs(&self.db, Some(work), only)
            .map_err(|e| RootStoreError::Launch(Box::new(crate::AgentLaunchError::caused("telemetry-runs-read", e))))
    }
    /// Change marker of every review job and planning run phase (all
    /// Works), folded into the live fingerprint so a run that starts or
    /// ends wakes the console. Empty when no job table exists. Same errors
    /// as [`Self::telemetry_runs`]. Read only.
    pub fn telemetry_runs_marker(&self) -> Result<String, RootStoreError> {
        let runs = crate::telemetry_runs::read_telemetry_runs(&self.db, None, None)
            .map_err(|e| RootStoreError::Launch(Box::new(crate::AgentLaunchError::caused("telemetry-runs-read", e))))?;
        Ok(runs.iter().map(|r| format!("{}={}\n", r.subject, r.phase)).collect())
    }
    /// Budget figures of one Work for the L7 alert rules and the console
    /// (see [`crate::WorkBudgetState`]), read in one read transaction:
    /// saved policy, engine committed estimates and declared worker costs.
    /// `None` when no policy is saved (the unlimited contract). Refusals keep
    /// their cause: malformed policy or ledgers (`live-budget-read`), unknown
    /// Work or invalid declared costs (see [`Self::reported_costs`]). Read
    /// only: no reservation, refund or launch decision.
    pub fn work_budget_state(&self, work: &str) -> Result<Option<crate::WorkBudgetState>, RootStoreError> {
        let mut tx = self.db.unchecked_transaction().map_err(RootStoreError::Sql)?;
        tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
        let budget_error = |e| RootStoreError::Launch(Box::new(crate::AgentLaunchError::caused("live-budget-read", e)));
        let result = (|| {
            let Some(budget) = crate::financial_budget::read_budget(&tx, work).map_err(budget_error)? else {
                return Ok(None);
            };
            let committed = crate::financial_budget::committed_estimates(&tx, work).map_err(budget_error)?;
            let costs = reported_costs::read(&tx, work)?;
            Ok(Some(crate::WorkBudgetState {
                work_id: work.to_owned(),
                limit_usd: budget.limit_usd.0,
                reserve_per_launch_usd: budget.reserve_per_launch_usd.0,
                committed_usd: committed,
                reported_usd: costs.total.reported_usd.0,
                attempts_without_cost: costs.total.attempts_without_cost,
            }))
        })();
        finished(result, tx.rollback().map_err(RootStoreError::Sql))
    }
    /// Read one Work's saved budget for display (LIVE-UX: launch-reserve use).
    /// `None` is the existing unlimited contract (no policy saved); a
    /// malformed saved policy refuses with its cause (`live-budget-read`).
    /// Read only; reserves no amount and grants no launch.
    pub fn work_budget(&self, work: &str) -> Result<Option<crate::Budget>, RootStoreError> {
        crate::financial_budget::read_budget(&self.db, work)
            .map_err(|e| RootStoreError::Launch(Box::new(crate::AgentLaunchError::caused("live-budget-read", e))))
    }
    /// Read the project price catalogue without creating missing data or a lock.
    /// A missing file is explicitly empty; malformed, unsafe or oversized files
    /// refuse with causes preserved. Does not infer prices, budgets or authority.
    pub fn rate_catalogue(&self) -> Result<crate::RateCatalogue, RootStoreError> {
        match self
            .directory
            .read_bounded("model-rates.json", crate::RATE_CATALOGUE_MAX_BYTES)
        {
            Ok(raw) => crate::RateCatalogue::decode(&raw).map_err(RootStoreError::Pricing),
            Err(e) if absent(&e) => Ok(crate::RateCatalogue::empty()),
            Err(e) => Err(RootStoreError::Storage(e)),
        }
    }
    /// Append one validated price under the Go-compatible exclusive pricing lock.
    /// Checks exact current digest, replaces supplied metadata with engine
    /// observations and atomically publishes a compact compatible JSON catalogue.
    /// Never edits old versions, reservations or mission budgets. Root must be
    /// writable; unsafe paths, invalid intent/observations, stale digest, format,
    /// entropy, write and cleanup failures refuse with typed causes. A Published
    /// error explicitly preserves a successful append before cleanup failure;
    /// uncertain rename/durability errors retain LocalStorageError publication.
    /// This local operation proves no authenticated actor or Go/Rust live coexistence.
    pub fn save_rate<E: WorkEnvironment>(
        &self,
        change: &crate::RateChange,
        environment: &mut E,
    ) -> Result<crate::ModelRate, RootStoreError> {
        self.save_rate_impl(
            change,
            environment,
            crate::local_storage::AtomicStorageFault::None,
        )
    }
    /// Explicit isolated-fixture injection into the actual save path. Not
    /// compiled into production; never invoke against a live/shared Go root.
    #[cfg(feature = "test-fixtures")]
    pub fn fixture_save_rate<E: WorkEnvironment>(
        &self,
        change: &crate::RateChange,
        environment: &mut E,
        fault: crate::AtomicStorageFault,
    ) -> Result<crate::ModelRate, RootStoreError> {
        self.save_rate_impl(change, environment, fault)
    }
    /// Shared publication path; fault is a compile-gated test device, never an
    /// execution policy. Preserve original catalogue before rename and expose
    /// post-rename durability uncertainty instead of claiming rollback.
    fn save_rate_impl<E: WorkEnvironment>(
        &self,
        change: &crate::RateChange,
        environment: &mut E,
        fault: crate::local_storage::AtomicStorageFault,
    ) -> Result<crate::ModelRate, RootStoreError> {
        if !self.writable {
            return Err(RootStoreError::Refused("root-read-only"));
        }
        crate::validate_rate_change(change).map_err(RootStoreError::Pricing)?;
        let reservation = self
            .directory
            .reserve_pricing()
            .map_err(RootStoreError::Storage)?;
        let mut published = false;
        let result = (|| {
            let mut catalogue = self.rate_catalogue()?;
            if catalogue.digest != change.expected_digest {
                return Err(RootStoreError::Refused("revision_conflict"));
            }
            let mut rate = change.rate.clone();
            rate.version = environment
                .identifier("rate-")
                .map_err(RootStoreError::Runtime)?;
            rate.actor = environment.operator().map_err(RootStoreError::Runtime)?;
            rate.updated = environment.timestamp().map_err(RootStoreError::Runtime)?;
            if !swarm_identity_contract::name(rate.version.as_bytes())
                || !crate::text::nonempty(&rate.actor)
            {
                return Err(RootStoreError::Refused(
                    "pricing-engine-observations-invalid",
                ));
            }
            swarm_wire_json::go_rfc3339::parse(&rate.updated)
                .map_err(|e| RootStoreError::Pricing(crate::PricingError::Clock(e)))?;
            if catalogue
                .rates
                .iter()
                .flatten()
                .any(|old| old.version == rate.version)
            {
                return Err(RootStoreError::Refused("pricing-version-already-present"));
            }
            catalogue
                .rates
                .get_or_insert_with(Vec::new)
                .push(rate.clone());
            catalogue.digest.clear();
            let raw = crate::encode_model(&catalogue)
                .map_err(|e| RootStoreError::Pricing(crate::PricingError::Encode(e)))?;
            if raw.len() > crate::RATE_CATALOGUE_MAX_BYTES {
                return Err(RootStoreError::Refused(
                    "pricing-catalogue-format-too-large",
                ));
            }
            self.directory
                .publish_with_fault("model-rates.json", &raw, fault)
                .map_err(RootStoreError::Storage)?;
            published = true;
            Ok(rate)
        })();
        match finished(result, reservation.close().map_err(RootStoreError::Storage)) {
            Err(primary) if published => Err(RootStoreError::Published {
                primary: Box::new(primary),
            }),
            result => result,
        }
    }
    /// Read-only current readiness of a native claim, never a process permit.
    /// Strict DTO, owned live fences, original input/event hashes, workflow/model/
    /// provider/report freshness and funding are checked under one settings/read
    /// transaction. Missing legacy bindings require a new claim, not backfill.
    /// Explicit rollback/close causes survive; caller owes Root.close and a fresh
    /// execution gate later. No provider, mutation or authenticated agent is made.
    #[cfg(target_os = "linux")]
    pub fn preview_planning_call(
        &mut self,
        work: &str,
        raw: &[u8],
    ) -> Result<crate::PlanningCallPreview, RootStoreError> {
        let request = crate::planning_call_preview::decode(raw)
            .map_err(|e| RootStoreError::Launch(Box::new(e)))?;
        let root = &self.root;
        let directory = &self.directory;
        crate::agent_storage_retry::execute(&mut self.db, directory, false, |db, settings| {
            let mut tx = db.transaction().map_err(RootStoreError::Sql)?;
            tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
            let result = crate::planning_call_preview::inspect(&tx, root, settings, work, &request)
                .map_err(|e| RootStoreError::Launch(Box::new(e)));
            finished(result, tx.rollback().map_err(RootStoreError::Sql))
        })
    }
    /// Run a prepared manual agent in this foreground Swarm CLI image. Rechecks
    /// saved policy/context/identity/budget, registers a gated actual child, owns
    /// signals/wait/drain and atomically settles only the matching attempt.
    /// No provider exit accepts a task. Unresolved ownership or uncertain storage
    /// preserves active state for recovery; typed causes and cleanup survive.
    /// Requires writable local operator root and the CLI private helper dispatch.
    #[cfg(target_os = "linux")]
    pub fn run_agent(&mut self, id: &str) -> Result<crate::AgentExecutionOutcome, RootStoreError> {
        if !self.writable {
            return Err(RootStoreError::Refused("root-read-only"));
        }
        crate::agent_supervision::run(&mut self.db, &self.root, &self.directory, id)
    }
    /// Read-only helper-side startup snapshot under saved SQLite/settings policy.
    /// The current process must match the committed child/parent/stamp/gate phase.
    /// Explicit rollback and lock cleanup retain causes; no lifecycle is changed.
    /// The internal caller still must close Root and recheck host/TTL before exec.
    #[cfg(target_os = "linux")]
    pub(crate) fn prepare_agent_gate(
        &mut self,
        id: &str,
    ) -> Result<crate::agent_execution_preflight::ExecutionPreparation, RootStoreError> {
        let root = &self.root;
        let dir = &self.directory;
        crate::agent_storage_retry::execute(&mut self.db, dir, false, |db, settings| {
            let mut tx = db
                .transaction()
                .map_err(|e| RootStoreError::Agent(crate::AgentStoreError::Sql(e)))?;
            tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
            let result = crate::agent_execution_preflight::inspect(
                &tx,
                root,
                dir,
                settings,
                id,
                crate::agent_execution_preflight::ExecutionStage::Child,
            )
            .map_err(|e| RootStoreError::Launch(Box::new(e)));
            finished(
                result,
                tx.rollback()
                    .map_err(|e| RootStoreError::Agent(crate::AgentStoreError::Sql(e))),
            )
        })
    }
    /// Observe one stored agent using authoritative SQL lifecycle columns.
    /// No process/host identity, live supervisor, permission or acceptance inferred.
    /// Missing/corrupt rows and SQL errors refuse with original typed causes.
    pub fn read_agent(&self, id: &str) -> Result<crate::Agent, RootStoreError> {
        crate::read_agent(&self.db, id).map_err(RootStoreError::Agent)
    }
    /// Replay caller-declared events against one consistent retained Agent row.
    /// Read transaction checks requested/body ID and SQL ownership/workspace;
    /// this consistency is not identity authentication. Frozen metadata alone is
    /// consumed, never current defaults. No lifecycle or reservation is changed,
    /// no process is started/stopped, no task accepted. Decode, ownership, policy,
    /// clock, SQL and rollback causes survive; caller must close this RootStore.
    pub fn replay_agent_guard(
        &mut self,
        id: &str,
        trace: &crate::AgentGuardTrace,
    ) -> Result<crate::AgentGuardReplay, RootStoreError> {
        let tx = self.db.transaction().map_err(RootStoreError::Sql)?;
        let result = (|| {
            let agent = crate::read_agent(&tx, id).map_err(RootStoreError::Agent)?;
            if agent.id != id {
                return Err(RootStoreError::Refused("agent-row-identity-mismatch"));
            }
            crate::agent_store::verify_agent_ownership(&tx, &agent)
                .map_err(RootStoreError::Agent)?;
            crate::replay_agent_guard(&agent, trace).map_err(RootStoreError::Guard)
        })();
        finished(result, tx.rollback().map_err(RootStoreError::Sql))
    }
    /// Atomically register this actual Linux process on a queued local agent.
    /// Writable root and matching stored host/body are required. Typed read,
    /// kernel, conflict, rollback and uncertain-commit errors remain explicit.
    /// Success is registration only: callers still owe all preparation/context/
    /// identity/permission/admission checks before provider start, including a
    /// persisted stop intention. Never releases reservations or starts a child.
    /// Uses saved SQLite policy; uncertain commits/cleanup failures never retry.
    #[cfg(target_os = "linux")]
    pub fn claim_agent(&mut self, id: &str) -> Result<crate::SupervisorClaim, RootStoreError> {
        if !self.writable {
            return Err(RootStoreError::Refused("root-read-only"));
        }
        crate::agent_storage_retry::execute(&mut self.db, &self.directory, true, |db, settings| {
            crate::agent_store::claim_agent_supervisor_bound(db, id, &self.directory, settings)
                .map_err(RootStoreError::Agent)
        })
    }
    /// Preview future limits using saved settings and actual project→mission→role→task
    /// overrides from one read transaction, holding the settings reservation.
    /// Supplied provider/prior data is diagnostic, not authenticated launch state.
    /// Invalid names/schema/revisions/JSON/policy or cleanup refuse explicitly.
    /// No write, reservation, provider, timer, counter reset or acceptance occurs.
    pub fn preview_agent_limits(
        &mut self,
        work: &str,
        role: &str,
        task: &str,
        input: &crate::RuntimeLimitPreview,
    ) -> Result<crate::RunLimits, RootStoreError> {
        if input.schema_version != 1
            || [work, role, task]
                .iter()
                .any(|v| !swarm_identity_contract::name(v.as_bytes()))
        {
            return Err(RootStoreError::Refused(
                "runtime-limit-preview-input-invalid",
            ));
        }
        let lock = self
            .directory
            .reserve(false, false)
            .map_err(RootStoreError::Storage)?;
        let result = (|| {
            let settings = self.read_settings()?;
            let policy = settings
                .values
                .runtime_limit_policy()
                .map_err(RootStoreError::Settings)?;
            let tx = self.db.transaction().map_err(RootStoreError::Sql)?;
            let result = (|| {
                let admin = crate::run_limits::read_configured_runtime_limits(
                    &tx, &policy, work, role, task,
                )?;
                crate::resolve_run_limits(
                    &input.provider,
                    &input.mission,
                    &admin,
                    input.previous.as_ref(),
                    &policy,
                )
                .map_err(RootStoreError::RunLimits)
            })();
            match result {
                Ok(value) => {
                    tx.commit().map_err(RootStoreError::Sql)?;
                    Ok(value)
                }
                Err(primary) => finished(Err(primary), tx.rollback().map_err(RootStoreError::Sql)),
            }
        })();
        finished(result, lock.close().map_err(RootStoreError::Storage))
    }
    /// Prepare or preview one manual worker launch under current saved policy.
    /// Engine constructs the context/attempt/estimate; no supplied Agent is accepted.
    /// Holds settings stable and atomically records all SQL effects, or rolls back
    /// preview/refusal. Host write probe is temporary; no provider process starts.
    /// Registered intent grants neither authentication nor execution permission.
    /// Post-commit reservation cleanup reports recorded state; reread/replay by ID.
    /// Direct SQLite BUSY uses freshly read saved timeout/retries/delay, with
    /// reservation release between attempts and no fixed extra ceiling. Zero
    /// retries means one attempt; null may wait indefinitely. Only confirmed
    /// autocommit after refusal is retried; cleanup/commit/nested-layer failures
    /// remain explicit. Storage attempts never relaunch a provider/accepted task.
    #[cfg(target_os = "linux")]
    pub fn prepare_agent_launch<E: WorkEnvironment>(
        &mut self,
        work: &str,
        r: &crate::Launch,
        preview: bool,
        environment: &mut E,
    ) -> Result<crate::AgentLaunchOutcome, RootStoreError> {
        self.prepare_agent_launch_with_fence(work, r, preview, environment, None)
    }
    /// Common manual-intent factory under current persisted policy. Only Root
    /// installs the optional typed lease; request JSON cannot disable or forge it.
    /// New admissions and guarded replay check ownership inside the actual SQL
    /// transaction. Errors retain recorded/commit/rollback/release distinctions;
    /// the nested fence cause never becomes a direct retryable SQLite refusal.
    #[cfg(target_os = "linux")]
    fn prepare_agent_launch_with_fence<E: WorkEnvironment>(
        &mut self,
        work: &str,
        r: &crate::Launch,
        preview: bool,
        environment: &mut E,
        fence: Option<&dyn crate::work_engine::MutationFence>,
    ) -> Result<crate::AgentLaunchOutcome, RootStoreError> {
        if !self.writable {
            return Err(RootStoreError::Refused("root-read-only"));
        }
        let root = &self.root;
        let directory = &self.directory;
        crate::agent_storage_retry::execute(&mut self.db, directory, !preview, |db, settings| {
            crate::agent_launch::prepare(
                db,
                crate::agent_launch::PreparationRoot {
                    root,
                    dir: directory,
                    settings,
                    fence,
                },
                work,
                r,
                preview,
                environment,
            )
            .map_err(|e| RootStoreError::Launch(Box::new(e)))
        })
    }

    /// Cancel an unregistered queued manual or proven automatic intention.
    /// Automatic provenance is checked historically; revoked consent/lease does not
    /// prevent cancellation of a proven unstarted intention under saved policies.
    /// Agent, matching task outcome/event, reservation release and log commit
    /// together. Scoped operator workers also signal their owning scope under
    /// actual saved settings. Other/newer attempts stay unchanged; registered/managed or
    /// inconsistent state refuses without mutation. No process signal or accepted
    /// task is produced. Post-commit settings-lock cleanup failure explicitly
    /// preserves the recorded lifecycle result; reread before retrying.
    /// Saved SQLite policy is refreshed; retries require confirmed rollback.
    pub fn cancel_queued_agent<E: WorkEnvironment>(
        &mut self,
        id: &str,
        environment: &mut E,
    ) -> Result<crate::Agent, RootStoreError> {
        if !self.writable {
            return Err(RootStoreError::Refused("root-read-only"));
        }
        let root = &self.root;
        crate::agent_storage_retry::execute(&mut self.db, &self.directory, true, |db, settings| {
            let limits = settings
                .values
                .mutation_limits()
                .map_err(RootStoreError::Settings)?;
            let context = crate::TaskLifecycleContext {
                root,
                quality_policy: None,
                task_limits: &limits.task,
                validation_limits: &limits.policy,
            };
            crate::agent_store::cancel_queued_agent_with_settings(
                db,
                id,
                &context,
                environment,
                Some(settings),
            )
            .map_err(RootStoreError::Agent)
        })
    }

    /// Reconcile a queued manual attempt or settle an already reaped native
    /// process from its private durable completion receipt. Registered processes
    /// require the original supervisor to have departed; missing/stale/incomplete
    /// proof refuses. No provider execution, signal, refund or acceptance occurs.
    /// Current saved SQLite policy applies; post-commit cleanup remains explicit.
    #[cfg(target_os = "linux")]
    pub fn reconcile_agent<E: WorkEnvironment>(
        &mut self,
        id: &str,
        environment: &mut E,
    ) -> Result<crate::Agent, RootStoreError> {
        if !self.writable {
            return Err(RootStoreError::Refused("root-read-only"));
        }
        let agent = self.read_agent(id)?;
        if agent.supervisor_pid == 0 {
            return self.cancel_queued_agent(id, environment);
        }
        crate::agent_completion_journal::recover(&mut self.db, &self.root, &self.directory, id)
    }

    /// Read all retained attempts newest first, without a recent-UI count ceiling.
    /// Invalid history refuses the read; no missing prices/usage or live identity
    /// is invented. This is observation only and changes no stored state.
    pub fn list_agents(&self, work: &str) -> Result<Vec<crate::Agent>, RootStoreError> {
        crate::list_agents(&self.db, work).map_err(RootStoreError::Agent)
    }
    /// Read one bounded page of an agent's recorded log rows after `after`.
    /// Observation only: no lifecycle, lease or process effect. A negative
    /// cursor is refused; an unknown agent yields an empty page, so callers
    /// confirm the agent with [`Self::read_agent`] first.
    pub fn agent_logs_after(
        &self,
        agent: &str,
        after: i64,
    ) -> Result<crate::CursorPage<crate::AgentLogEntry>, RootStoreError> {
        crate::read_agent_logs_after(&self.db, agent, after).map_err(RootStoreError::Agent)
    }
    /// Digest of all state shown by the live views; see
    /// [`crate::read_live_fingerprint`]. Observation only, no write.
    pub fn live_fingerprint(&self) -> Result<String, RootStoreError> {
        crate::read_live_fingerprint(&self.db).map_err(RootStoreError::Agent)
    }
    /// Read one bounded page of an agent's recorded PTY chunks after `after`.
    /// Same contract as [`Self::agent_logs_after`]; never opens the terminal
    /// control socket, so it grants no input or lease.
    pub fn terminal_events_after(
        &self,
        agent: &str,
        after: i64,
    ) -> Result<crate::CursorPage<crate::TerminalChunk>, RootStoreError> {
        crate::read_terminal_events_after(&self.db, agent, after).map_err(RootStoreError::Agent)
    }
    /// Persist a stop intention and one idempotent command event on a writable root.
    /// Timestamp comes from the engine; all SQL errors/rollback/commit uncertainty
    /// remain explicit. Success means desired=stop, never confirmed termination,
    /// released workspace/reservation or task acceptance. Caller must read back
    /// if later output/close fails; supervision is a separate pending contract.
    /// Fresh saved SQLite policy governs safe BUSY retries without process relaunch.
    pub fn stop_agent<E: WorkEnvironment>(
        &mut self,
        id: &str,
        environment: &mut E,
    ) -> Result<crate::Agent, RootStoreError> {
        if !self.writable {
            return Err(RootStoreError::Refused("root-read-only"));
        }
        crate::agent_storage_retry::execute(&mut self.db, &self.directory, true, |db, _settings| {
            let at = environment.timestamp().map_err(RootStoreError::Runtime)?;
            crate::execute_agent_stop(db, id, &at).map_err(RootStoreError::Agent)
        })
    }
    /// Check complete retained JSONL output against current saved response/event/
    /// decision policies without a provider or mutation. Caller owns input framing
    /// and Root.close; CLI uses the documented command transport boundary. Safe
    /// typed read/parser causes survive. This validates data only: no Work/scope,
    /// lease/delivery ownership, supported action, invoice or permission is proved.
    pub fn check_planning_output(
        &self,
        raw: &[u8],
    ) -> Result<crate::PlanningOutputCheck, RootStoreError> {
        let settings = self.settings()?;
        let error = |e| RootStoreError::Launch(Box::new(e));
        let mut stream = crate::PlanningOutputStream::new(&settings.values).map_err(error)?;
        stream.feed(raw).map_err(error)?;
        let result = stream.finish().map_err(error)?;
        Ok(crate::PlanningOutputCheck {
            schema_version: 1,
            proposal_valid: true,
            input_count: result.proposal.input_events.len(),
            operation_count: result.proposal.operations.len(),
            reason_bytes: result.proposal.reason.len(),
            provider_facts: result.provider_facts,
            process_started: false,
            decision_applied: false,
        })
    }
    /// Read validated saved settings/history under the shared reservation and
    /// the existing saved storage contention budget/delay. Only a clean lock
    /// conflict retries before reading; zero retries refuses immediately with
    /// its typed cause/budget, null may wait indefinitely. Release, malformed
    /// policy and compound failures do not retry. No default adoption, history
    /// mutation, mission/provider retry or authority grant; saved is not active.
    pub fn settings(&self) -> Result<EngineSettingsDocument, RootStoreError> {
        crate::agent_storage_retry::execute_borrowed(
            &self.db, &self.directory, false, |_, settings| Ok(settings.clone()),
        )
    }
    /// Read this root's validated saved settings without adopting defaults.
    /// Owned-directory IO, syntax/history/policy errors retain typed causes.
    fn read_settings(&self) -> Result<EngineSettingsDocument, RootStoreError> {
        Self::read_settings_from(&self.directory)
    }
    /// Shared adapter read from an already checked local directory. No mutation
    /// or authority grant; caller owns reservation/lifetime and checks cleanup.
    pub(crate) fn read_settings_from(
        directory: &LocalDirectory,
    ) -> Result<EngineSettingsDocument, RootStoreError> {
        EngineSettingsDocument::decode(&directory.read(SETTINGS).map_err(RootStoreError::Storage)?)
            .map_err(RootStoreError::Settings)
    }
    /// Preview a complete change against the actual current saved revision. No
    /// publication or runtime application occurs. Schema/value/revision, read and
    /// release failures remain typed; apply must recheck freshness and authority.
    pub fn preview_settings(
        &self,
        r: &EngineSettingsChange,
    ) -> Result<EngineSettingsPreview, RootStoreError> {
        self.settings()?
            .preview(r)
            .map_err(RootStoreError::Settings)
    }
    /// Apply or replay a saved settings proposal under its exclusive reservation.
    /// Validate revision/preview token and preserve policy/history; no Work usage
    /// is reset. Known/replayed or visibly uncertain publication followed by
    /// release failure retains Published, requiring readback before another apply.
    /// A pre-write refusal is never relabeled as publication or successful apply.
    /// Network-group values require the next server start; this publishes saved
    /// configuration, not an observation of active sockets/workers.
    pub fn apply_settings<E: WorkEnvironment>(
        &self,
        r: &EngineSettingsChange,
        environment: &mut E,
    ) -> Result<(EngineSettingsDocument, bool), RootStoreError> {
        if !self.writable {
            return Err(RootStoreError::Refused("root-read-only"));
        }
        let reservation = self
            .directory
            .reserve(true, false)
            .map_err(RootStoreError::Storage)?;
        let result = (|| {
            let current = self.read_settings()?;
            if current.replay(r).map_err(RootStoreError::Settings)? {
                return Ok((current, true));
            }
            let preview = current.preview(r).map_err(RootStoreError::Settings)?;
            if r.preview_token.is_empty() || r.preview_token != preview.preview_token {
                return Err(RootStoreError::Refused("settings-preview-stale-or-missing"));
            }
            let (doc, replayed) = current
                .apply(
                    r,
                    environment.operator().map_err(RootStoreError::Runtime)?,
                    environment.timestamp().map_err(RootStoreError::Runtime)?,
                )
                .map_err(RootStoreError::Settings)?;
            if !replayed {
                let bytes = crate::encode_model(&doc).map_err(RootStoreError::Encode)?;
                self.directory
                    .publish(SETTINGS, &bytes)
                    .map_err(RootStoreError::Storage)?;
            }
            Ok((doc, replayed))
        })();
        settings_finished(result, reservation.close().map_err(RootStoreError::Storage))
    }
    /// Preview the current task choice under saved settings without SQL writes.
    /// Strict request/revision, idle guards and provider resolution apply. Local
    /// actor/time are unverified observations, not identity or launch permission;
    /// source/storage/release errors retain causes. Apply must recheck everything.
    pub fn preview_task_model<E: WorkEnvironment>(
        &self,
        work: &str,
        raw: &[u8],
        environment: &mut E,
    ) -> Result<crate::TaskModelPreview, RootStoreError> {
        crate::check_command_input_size(raw).map_err(RootStoreError::Input)?;
        let request = crate::decode_task_model_request(raw)
            .map_err(|e| RootStoreError::Launch(Box::new(e)))?;
        let reservation = self
            .directory
            .reserve(false, false)
            .map_err(RootStoreError::Storage)?;
        let result = (|| {
            let settings = self.read_settings()?;
            crate::task_model::preview(&self.db, &self.root, work, &request, &settings, environment)
                .map_err(|e| RootStoreError::Launch(Box::new(e)))
        })();
        finished(result, reservation.close().map_err(RootStoreError::Storage))
    }
    /// Retain the directory and actual SQLite identities with owned CLOEXEC handles.
    /// The caller can compare future Root opens without trusting a pathname or
    /// allowing inode reuse. This observes storage, grants no permissions and
    /// does not lock project files/pathnames. Caller owes explicit binding close.
    #[cfg(target_os = "linux")]
    pub fn bind_root(&self) -> Result<RootBinding, RootStoreError> {
        // Validate connection/directory association before duplicating either
        // capability; all partially acquired handles receive checked cleanup.
        let expected = self.instance()?;
        let database = crate::connected_database_identity::ConnectedDatabaseIdentity::capture(&self.db)
            .map_err(database_identity_error)?;
        if database.identity() != expected.database {
            return finished(Err(RootStoreError::Refused("root-database-instance-changed")),
                database.close().map_err(database_identity_error));
        }
        match self.directory.duplicate() {
            Ok(directory) => Ok(RootBinding { directory, database }),
            Err(error) => finished(Err(RootStoreError::Storage(error)),
                database.close().map_err(database_identity_error)),
        }
    }
    /// Observe this Root's actual connected DB and its owned directory association.
    /// No permission or path confinement is inferred; failures preserve original
    /// observation and cleanup causes, with no fallback to pathname identity.
    #[cfg(target_os = "linux")]
    pub(crate) fn instance(&self) -> Result<RootInstanceIdentity, RootStoreError> {
        observe_instance(&self.db, &self.directory)
    }
    /// Create/reuse a Go-compatible local bearer file for an actual bound loopback
    /// listener. Requires a writable trusted Root; no Work/DB/settings mutation or
    /// business permission is granted. Private descriptor-anchored publication is
    /// atomic NOREPLACE, rejects unsafe/invalid existing files and preserves typed
    /// storage/entropy/close/uncertain-durability causes. No expiry/rotation, server
    /// loop or agent authentication. Caller must check Root close separately and
    /// retain known credential state if a later startup step fails.
    #[cfg(target_os = "linux")]
    pub fn web_session(
        &self,
        listener: &std::net::TcpListener,
    ) -> Result<crate::WebSessionCredential, crate::WebSessionError> {
        if !self.writable {
            return Err(crate::WebSessionError::Refused("web-session-readonly"));
        }
        crate::web_session::load_or_create(&self.directory, listener)
    }
    pub fn read_work(&self, id: &str) -> Result<Work, RootStoreError> {
        crate::read_recorded_work(&self.db, id).map_err(RootStoreError::Work)
    }
    pub fn list_works(&self) -> Result<Vec<Work>, RootStoreError> {
        crate::list_recorded_works(&self.db).map_err(RootStoreError::Work)
    }
    pub fn events(&self, id: &str) -> Result<Vec<RecordedEvent>, RootStoreError> {
        crate::list_recorded_events(&self.db, id).map_err(RootStoreError::Events)
    }
    fn with_context_policy<T>(
        &self,
        read: impl FnOnce(&crate::ContextPolicyLimits) -> Result<T, crate::ProjectContextError>,
    ) -> Result<T, RootStoreError> {
        let reservation = self
            .directory
            .reserve(false, false)
            .map_err(RootStoreError::Storage)?;
        let result = (|| {
            let limits = self
                .read_settings()?
                .values
                .context_policy_limits()
                .map_err(RootStoreError::Settings)?;
            read(&limits).map_err(RootStoreError::Context)
        })();
        finished(result, reservation.close().map_err(RootStoreError::Storage))
    }
    /// Read effective project context under the settings reservation. This
    /// starts no agent and grants no authority to the returned instructions.
    pub fn project_context(
        &self,
        role: &str,
    ) -> Result<Option<crate::ProjectContextObservation>, RootStoreError> {
        self.with_context_policy(|limits| crate::read_project_context(&self.root, role, limits))
    }
    /// Read a supplied planning event's complete declared report under one current
    /// settings reservation. Requires explicit v14 report-policy adoption, even
    /// for an event with no selected report (which then returns None).
    /// Path/read/format/digest/policy/close failures preserve causes; no mutation,
    /// event acknowledgement, authorship, receipt, identity, authorization or
    /// future freshness lease is established. Caller owns event provenance.
    pub fn planning_report(
        &self,
        event: &crate::PlanningEvent,
    ) -> Result<Option<crate::PlanningReportContent>, RootStoreError> {
        let reservation = self
            .directory
            .reserve(false, false)
            .map_err(RootStoreError::Storage)?;
        let result = (|| {
            let cap = self
                .read_settings()?
                .values
                .planning_report_limit()
                .map_err(RootStoreError::Settings)?;
            crate::read_planning_report(&self.root, event, cap)
                .map_err(RootStoreError::PlanningReport)
        })();
        finished(result, reservation.close().map_err(RootStoreError::Storage))
    }
    /// Observe compiled role guidance and ordinary project context under one
    /// current settings reservation. Requires explicit v13 body policy adoption;
    /// never nests the independent project_context lock. Read/codec/policy/close
    /// errors preserve both causes. No mutation, actor, identity/authority,
    /// execution, acceptance or later-action lease is established by this read.
    pub fn agent_workflow(
        &self,
        role: &str,
    ) -> Result<crate::AgentWorkflowObservation, RootStoreError> {
        let reservation = self
            .directory
            .reserve(false, false)
            .map_err(RootStoreError::Storage)?;
        let result = (|| {
            let doc = self.read_settings()?;
            let cap = doc
                .values
                .agent_workflow_body_limit()
                .map_err(RootStoreError::Settings)?;
            let limits = doc
                .values
                .context_policy_limits()
                .map_err(RootStoreError::Settings)?;
            let mut observed = crate::read_agent_workflow(&self.root, role, cap, &limits)
                .map_err(RootStoreError::Workflow)?;
            observed.settings_revision = Some(doc.revision);
            Ok(observed)
        })();
        finished(result, reservation.close().map_err(RootStoreError::Storage))
    }
    /// Reobserve framing and compare complete prompt bytes/hash, workflow metadata
    /// and base count. Declared hashes alone do not authenticate an observation.
    /// Current saved policy is rechecked; unrelated revision/cap changes do not
    /// alone stale unchanged allowed text. Read/context/policy/close errors keep
    /// causes; mismatched/forged data refuses as stale. This grants no identity,
    /// authority, agent receipt, mutation or freshness lease for a future action.
    pub fn check_agent_workflow(
        &self,
        previous: &crate::AgentWorkflowObservation,
        role: &str,
    ) -> Result<(), RootStoreError> {
        let current = self.agent_workflow(role)?;
        if current.workflow != previous.workflow
            || current.prompt != previous.prompt
            || current.prompt_sha256 != previous.prompt_sha256
            || current.body_bytes != previous.body_bytes
        {
            return Err(RootStoreError::Workflow(
                crate::AgentWorkflowError::Refused("agent-workflow-stale"),
            ));
        }
        Ok(())
    }
    /// Read an ordinary local document using the current persisted review cap.
    /// This selects no task attachment and grants no review or disclosure right.
    pub fn review_delivery(&self, name: &str) -> Result<String, RootStoreError> {
        self.with_context_policy(|limits| crate::read_review_delivery(&self.root, name, limits))
    }
    /// Reread sources and the current configured limits before checking a saved
    /// context digest. No cache, mutation, identity claim or later-action lease.
    pub fn check_project_context(
        &self,
        previous: Option<&crate::ProjectContext>,
        role: &str,
    ) -> Result<(), RootStoreError> {
        self.with_context_policy(|limits| {
            crate::check_project_context_freshness(&self.root, previous, role, limits)
        })
    }
    /// Observe actual stored ordinary or simple managed review data under the
    /// current settings reservation. Managed reviews include common bindings and
    /// candidate/files/baselines/Git; unsupported batches/fragments still refuse.
    /// Returns observed row/settings revisions without mutation. Typed primary
    /// and reservation-close causes are retained together. This supplies no
    /// identity, review authority, acceptance or later-action lease.
    /// On Linux, material reads verify before/after that the project pathname
    /// names this opened datastore. Missing/replaced material roots and checked
    /// close failures refuse with causes; this is not an atomic snapshot or lease.
    pub fn inspect_review_evidence(
        &self,
        work: &str,
        task: &str,
    ) -> Result<RootReviewEvidenceObservation, RootStoreError> {
        material_path::with_material_read(self, || {
            let work = self.read_work(work)?;
            let settings = self.read_settings()?;
            let review = crate::check_current_review_evidence(&self.root, &work, task, &settings)
                .map_err(RootStoreError::Review)?;
            Ok(RootReviewEvidenceObservation {
                scope: "data-consistency-only",
                work_id: work.id,
                work_revision: work.revision,
                task_id: task.into(),
                settings_revision: settings.revision,
                review,
            })
        })
    }
    /// Read the unique actual stored managed task's machine receipt under its
    /// current settings reservation. Returns None only for undeclared evidence;
    /// malformed/oversize/stale data and primary/release errors refuse with causes.
    /// Revisions describe a read-only observation, not identity, freshness,
    /// independent approval, task acceptance or a future action lease.
    /// On Linux, material reads verify before/after that the project pathname
    /// names this opened datastore. Missing/replaced material roots and checked
    /// close failures refuse with causes; this is not an atomic snapshot or lease.
    pub fn inspect_managed_receipt(
        &self,
        work: &str,
        task: &str,
    ) -> Result<RootManagedReceiptObservation, RootStoreError> {
        material_path::with_material_read(self, || {
            let work = self.read_work(work)?;
            let settings = self.read_settings()?;
            let receipt = crate::read_managed_receipt(&self.root, &work, task, &settings)
                .map_err(RootStoreError::Review)?;
            Ok(RootManagedReceiptObservation {
                scope: "receipt-data-consistency-only",
                work_id: work.id,
                work_revision: work.revision,
                task_id: task.into(),
                settings_revision: settings.revision,
                receipt,
            })
        })
    }
    /// Read retained controls for the unique actual stored task, excluding its
    /// current receipt. Work/history/settings reads and checked release occur
    /// under a shared settings reservation. No supplied Work, receipt or freshness
    /// flag is accepted. History remains data, not current acceptance or identity.
    /// SQL/row/codec/finalization/reservation errors retain their causes; no state
    /// changes occur. Returned revisions describe this observation, not a later lease.
    pub fn inspect_control_history(
        &self,
        work: &str,
        task: &str,
    ) -> Result<RootControlHistoryObservation, RootStoreError> {
        let reservation = self
            .directory
            .reserve(false, false)
            .map_err(RootStoreError::Storage)?;
        let result = (|| {
            let work = self.read_work(work)?;
            let settings = self.read_settings()?;
            let mut tasks = work.tasks.iter().flatten().filter(|t| t.id == task);
            let selected = tasks
                .next()
                .ok_or(RootStoreError::Refused("control-history-task-absent"))?;
            if tasks.next().is_some() {
                return Err(RootStoreError::Refused("control-history-task-ambiguous"));
            }
            let history = crate::read_validation_control_history(
                &self.db,
                &work.id,
                selected,
                selected.automatic_validation.as_deref(),
            )
            .map_err(RootStoreError::ControlHistory)?;
            Ok(RootControlHistoryObservation {
                work_id: work.id,
                work_revision: work.revision,
                task_id: task.into(),
                settings_revision: settings.revision,
                history,
            })
        })();
        finished(result, reservation.close().map_err(RootStoreError::Storage))
    }
    /// Check current validation data for the unique actual stored task and
    /// persisted output/quality policy under a shared settings reservation.
    /// Reads actual artifacts; opens no provider and mutates no state. Returned
    /// metadata describe this read, not authentication, acceptance or a later lease.
    /// Missing/duplicate tasks, settings, receipt/artifact and release errors
    /// refuse with their cause; no caller-supplied freshness or policy is accepted.
    /// On Linux, material reads verify before/after that the project pathname
    /// names this opened datastore. Missing/replaced roots refuse with retained
    /// causes. This is not an atomic filesystem snapshot or a later-action lease.
    pub fn inspect_validation_evidence(
        &self,
        work: &str,
        task: &str,
    ) -> Result<RootValidationEvidenceObservation, RootStoreError> {
        material_path::with_material_read(self, || {
            let work = self.read_work(work)?;
            let settings = self.read_settings()?;
            let mut tasks = work.tasks.iter().flatten().filter(|t| t.id == task);
            let selected = tasks
                .next()
                .ok_or(RootStoreError::Refused("validation-evidence-task-absent"))?;
            if tasks.next().is_some() {
                return Err(RootStoreError::Refused(
                    "validation-evidence-task-ambiguous",
                ));
            }
            let output = settings
                .values
                .validation_output_limit()
                .map_err(RootStoreError::Settings)?;
            let quality = if selected.validation_policy.as_deref().is_some_and(|p| {
                p.mode == "automatic" && p.controls.as_ref().is_some_and(|c| !c.is_empty())
            }) {
                Some(
                    settings
                        .quality_policy_snapshot()
                        .map_err(RootStoreError::Settings)?,
                )
            } else {
                None
            };
            let evidence =
                crate::check_validation_evidence(&self.root, selected, quality.as_ref(), output)
                    .map_err(RootStoreError::ValidationEvidence)?;
            Ok(RootValidationEvidenceObservation {
                work_id: work.id,
                work_revision: work.revision,
                task_id: task.into(),
                settings_revision: settings.revision,
                evidence,
            })
        })
    }
    /// Derive manual or ordinary hierarchical validation from the actual row,
    /// files, retained history and saved settings under a shared reservation.
    /// Current review data is required for hierarchical freshness, even waivers.
    /// Simple managed reads compose saved receipt policy and current Git review;
    /// batches/fragments/platform limitations refuse. Unexpected review IO/codec, SQL,
    /// settings and checked-release failures preserve their causes and refuse.
    /// No supplied freshness, mutation, identity/authority, agent launch,
    /// execution provenance or acceptance follows from this read.
    /// On Linux, material reads verify before/after that the project pathname
    /// names this opened datastore. Missing/replaced roots refuse with retained
    /// causes. This is not an atomic filesystem snapshot or a later-action lease.
    pub fn inspect_work_validation(
        &self,
        work: &str,
        language: crate::WorkViewLanguage,
    ) -> Result<RootWorkValidationObservation, RootStoreError> {
        material_path::with_material_read(self, || {
            let work = self.read_work(work)?;
            let settings = self.read_settings()?;
            let validation = crate::work_projection::project_work_validation(
                &self.db, &self.root, &work, &settings, language,
            )
            .map_err(RootStoreError::Projection)?;
            Ok(RootWorkValidationObservation {
                work_id: work.id,
                work_revision: work.revision,
                settings_revision: settings.revision,
                validation,
            })
        })
    }
    /// Parse a supplied reply against the actual stored task count and current
    /// persisted policy under a shared settings reservation. Supplied originals
    /// remain unauthenticated; success grants no reviewer authority, mutation,
    /// acceptance or later-action freshness. Returns observed revision metadata.
    /// Row/settings/parse/allocation/cleanup errors preserve their causes. A missing
    /// or duplicate task refuses; no supplied Work or policy overrides are accepted.
    pub fn inspect_review_reply(
        &self,
        work: &str,
        task: &str,
        raw: &[u8],
        originals: &[crate::ReviewQuoteOriginal<'_>],
    ) -> Result<RootReviewReplyObservation, RootStoreError> {
        let reservation = self
            .directory
            .reserve(false, false)
            .map_err(RootStoreError::Storage)?;
        let result = (|| {
            let work = self.read_work(work)?;
            let settings = self.read_settings()?;
            let limits = settings
                .values
                .review_reply_limits()
                .map_err(RootStoreError::Settings)?;
            let mut tasks = work.tasks.iter().flatten().filter(|t| t.id == task);
            let selected = tasks
                .next()
                .ok_or(RootStoreError::Refused("review-reply-task-absent"))?;
            if tasks.next().is_some() {
                return Err(RootStoreError::Refused("review-reply-task-ambiguous"));
            }
            let count = selected.criteria.as_ref().map_or(0, Vec::len);
            let parsed = crate::parse_review_reply(raw, count, originals, limits)
                .map_err(RootStoreError::Reply)?;
            Ok(RootReviewReplyObservation {
                work_id: work.id,
                work_revision: work.revision,
                task_id: task.into(),
                settings_revision: settings.revision,
                parsed,
            })
        })();
        finished(result, reservation.close().map_err(RootStoreError::Storage))
    }
    #[cfg(target_os = "linux")]
    fn work_view_inner(
        &self,
        id: &str,
        expected: Option<i64>,
        language: crate::WorkViewLanguage,
    ) -> Result<crate::WorkViewObservation, RootStoreError> {
        let work = self.read_work(id)?;
        if expected.is_some_and(|r| r != work.revision) {
            return Err(RootStoreError::Refused("view-work-revision-conflict"));
        }
        let settings = self.read_settings()?;
        let validation = crate::work_projection::project_work_validation(
            &self.db, &self.root, &work, &settings, language,
        )
        .map_err(RootStoreError::Projection)?;
        let events = self.events(id)?;
        let mut gates = std::collections::BTreeMap::new();
        for task in work.tasks.iter().flatten() {
            if let Some(gate) = task.gate.as_deref() {
                let policy = settings
                    .quality_policy_snapshot()
                    .map_err(RootStoreError::Settings)?;
                let observation = match crate::gate_contract::evaluate_observed(
                    gate.document.0.get().as_bytes(),
                    &self.root,
                    &gate.evaluation.phase,
                    &policy,
                ) {
                    Ok(evaluation) => crate::CurrentGateObservation {
                        valid: true,
                        evaluation: Some(evaluation),
                        error: None,
                        cause: None,
                    },
                    Err(cause) => crate::CurrentGateObservation {
                        valid: false,
                        evaluation: None,
                        error: Some(cause.to_string()),
                        cause: Some(cause),
                    },
                };
                gates.insert(task.id.clone(), observation);
            }
        }
        let git = crate::observe_git_metadata(&self.root, &settings)
            .map_err(RootStoreError::GitMetadata)?;
        let mut view = crate::WorkViewObservation {
            work,
            validation,
            events,
            current_gates: gates,
            git,
            resume_markdown: String::new(),
        };
        view.resume_markdown =
            crate::work_view::render_work_view(&self.root, &view, &settings, language)
                .map_err(RootStoreError::View)?;
        Ok(view)
    }
    /// Read an actual Work, saved policy, events, files and Git under one shared
    /// reservation. No view file is written. Malformed selected events, missing
    /// policy and cleanup errors refuse with retained causes; no authority follows.
    #[cfg(target_os = "linux")]
    pub fn inspect_work_view(
        &self,
        id: &str,
        language: crate::WorkViewLanguage,
    ) -> Result<crate::WorkViewObservation, RootStoreError> {
        let reservation = self
            .directory
            .reserve(false, false)
            .map_err(RootStoreError::Storage)?;
        let result = self.work_view_inner(id, None, language);
        finished(result, reservation.close().map_err(RootStoreError::Storage))
    }
    /// Atomically publish a fresh local summary under an exclusive reservation.
    /// An optional expected revision refuses stale post-mutation rendering before
    /// publication. Errors preserve existing files; an earlier committed Work
    /// mutation is not rolled back by rendering, storage or cleanup failure.
    #[cfg(target_os = "linux")]
    pub fn render_work_view(
        &self,
        id: &str,
        expected: Option<i64>,
        language: crate::WorkViewLanguage,
    ) -> Result<crate::WorkViewObservation, RootStoreError> {
        if !self.writable {
            return Err(RootStoreError::Refused("root-read-only"));
        }
        let reservation = self
            .directory
            .reserve(true, false)
            .map_err(RootStoreError::Storage)?;
        let result = (|| {
            let view = self.work_view_inner(id, expected, language)?;
            let directory = LocalDirectory::open(&self.root.join(".swarm/views"))
                .map_err(RootStoreError::Storage)?;
            let published = directory
                .publish(
                    &format!("{}.md", view.work.id),
                    view.resume_markdown.as_bytes(),
                )
                .map_err(RootStoreError::Storage);
            finished(
                published,
                directory.close().map_err(RootStoreError::Storage),
            )?;
            Ok(view)
        })();
        finished(result, reservation.close().map_err(RootStoreError::Storage))
    }
    /// Render the stored Work list using actual current validations and saved
    /// presentation policy under one shared reservation. No Git command or file
    /// publication occurs. SQL/codec/projection/settings/release errors refuse.
    #[cfg(target_os = "linux")]
    pub fn inspect_work_list_text(
        &self,
        language: crate::WorkViewLanguage,
    ) -> Result<String, RootStoreError> {
        let reservation = self
            .directory
            .reserve(false, false)
            .map_err(RootStoreError::Storage)?;
        let result = (|| {
            let settings = self.read_settings()?;
            let works = self
                .list_works()?
                .into_iter()
                .map(|w| {
                    let v = crate::work_projection::project_work_validation(
                        &self.db, &self.root, &w, &settings, language,
                    )
                    .map_err(RootStoreError::Projection)?;
                    Ok((w, v))
                })
                .collect::<Result<Vec<_>, RootStoreError>>()?;
            crate::work_view::render_work_list(&works, &settings, language)
                .map_err(RootStoreError::View)
        })();
        finished(result, reservation.close().map_err(RootStoreError::Storage))
    }
    /// Read actual Git display metadata under the current saved policy and a
    /// shared settings reservation. Fixed commands disable external filters,
    /// hooks/fsmonitor and optional index writes. Field failures stay explicit;
    /// protection/settings/process/release errors retain their causes. No Work
    /// changes, repository initialization, actor authentication or lease follows.
    #[cfg(target_os = "linux")]
    pub fn inspect_git_metadata(&self) -> Result<crate::GitMetadataObservation, RootStoreError> {
        let reservation = self
            .directory
            .reserve(false, false)
            .map_err(RootStoreError::Storage)?;
        let result = (|| {
            let settings = self.read_settings()?;
            crate::observe_git_metadata(&self.root, &settings).map_err(RootStoreError::GitMetadata)
        })();
        finished(result, reservation.close().map_err(RootStoreError::Storage))
    }
    /// Hash a committed Git report using the actual current settings journal.
    ///
    /// The relative repository must resolve beneath this root. A shared settings
    /// reservation covers reading policy, executing the fixed blob read and
    /// checked release. No supplied policy is accepted and no state is changed.
    /// This is a local observation API, not authentication, protocol validation
    /// or permission to accept any Work. Repository contents can change later.
    ///
    /// # Errors
    /// Preserves reservation/settings, Git path/input/process/deadline/read and
    /// cleanup failures, including an unreaped owned process for explicit recovery.
    #[cfg(target_os = "linux")]
    pub fn inspect_git_report(
        &self,
        repository: &str,
        candidate: &str,
        report: &str,
        version: i64,
    ) -> Result<crate::GitReportObservation, RootStoreError> {
        let reservation = self
            .directory
            .reserve(false, false)
            .map_err(RootStoreError::Storage)?;
        let result = (|| {
            let settings = self.read_settings()?;
            crate::hash_git_report(
                &self.root, repository, candidate, report, version, &settings,
            )
            .map_err(RootStoreError::Git)
        })();
        finished(result, reservation.close().map_err(RootStoreError::Storage))
    }
    /// Observe candidate/files/context/Git for the actual stored Work and current settings.
    ///
    /// A shared settings reservation covers the stored row read and fixed Git
    /// inspection. Context hash and parsing use the same bytes. Historical files
    /// follow the persisted depth policy; batch/fragment protocols still refuse. This mutates no state and grants
    /// neither reviewer identity, task acceptance nor freshness for a later action.
    /// Errors retain row/settings/codec/artifact/Git and reservation-close causes.
    #[cfg(target_os = "linux")]
    /// On Linux, material reads verify before/after that the project pathname
    /// names this opened datastore. Missing/replaced roots refuse with retained
    /// causes. This is not an atomic filesystem snapshot or a later-action lease.
    pub fn inspect_managed_review_git(
        &self,
        work: &str,
        task: &str,
    ) -> Result<RootManagedReviewGitObservation, RootStoreError> {
        material_path::with_material_read(self, || {
            let work = self.read_work(work)?;
            let settings = self.read_settings()?;
            let observation = crate::inspect_managed_review_git(&self.root, &work, task, &settings)
                .map_err(RootStoreError::Review)?;
            Ok(RootManagedReviewGitObservation {
                work_id: work.id,
                work_revision: work.revision,
                settings_revision: settings.revision,
                observation,
            })
        })
    }
    /// Observe candidate/file bindings from the stored `work` and unique `task`.
    ///
    /// Reads the actual Work row and settings journal under a shared settings
    /// reservation, returning their observed revisions and the limited anchor.
    /// No caller-supplied Work/settings are accepted and no state is modified.
    /// The reservation stabilizes settings only, not Work or files after reading.
    ///
    /// Success supplies neither Git/protocol/identity verification nor authority
    /// to accept a task or perform a later action. Such callers must revalidate
    /// current state and required evidence under their own mutation contract.
    ///
    /// # Errors
    /// Returns [`RootStoreError`] for reservation, row/settings decoding, SQL,
    /// candidate/file evidence or reservation-close failure. Primary and cleanup
    /// errors are preserved together when both operations fail.
    /// On Linux, material reads verify before/after that the project pathname
    /// names this opened datastore. Missing/replaced material roots and checked
    /// close failures refuse with causes; this is not an atomic snapshot or lease.
    pub fn inspect_managed_review_anchor(
        &self,
        work: &str,
        task: &str,
    ) -> Result<RootManagedReviewAnchorObservation, RootStoreError> {
        material_path::with_material_read(self, || {
            let work = self.read_work(work)?;
            let settings = self.read_settings()?;
            let anchor = crate::inspect_managed_review_anchor(&self.root, &work, task)
                .map_err(RootStoreError::Review)?;
            Ok(RootManagedReviewAnchorObservation {
                work_id: work.id,
                work_revision: work.revision,
                settings_revision: settings.revision,
                anchor,
            })
        })
    }
    /// Read an observed gate result under the settings reservation. This is not
    /// acceptance, review/identity verification or a promise of future freshness.
    /// On Linux, material reads verify before/after that the project pathname
    /// names this opened datastore. Missing/replaced material roots and checked
    /// close failures refuse with causes; this is not an atomic snapshot or lease.
    pub fn inspect_gate_evidence(
        &self,
        work: &str,
        task: &str,
    ) -> Result<GateEvidenceObservation, RootStoreError> {
        material_path::with_material_read(self, || {
            let policy = self
                .read_settings()?
                .quality_policy_snapshot()
                .map_err(RootStoreError::Settings)?;
            let work = self.read_work(work)?;
            let task = work
                .tasks
                .as_deref()
                .unwrap_or_default()
                .iter()
                .find(|t| t.id == task)
                .ok_or(RootStoreError::Refused("gate-task-not-found"))?;
            let evaluation = crate::check_gate_evidence(task, &self.root, &policy)
                .map_err(RootStoreError::Gate)?;
            Ok(GateEvidenceObservation {
                work_id: work.id,
                work_revision: work.revision,
                task_id: task.id.clone(),
                evaluation,
                policy,
            })
        })
    }
    /// Apply supported Work/gate/planning/task-model/report controls using actual saved settings,
    /// reservation, busy policy and transaction owner. Complete planning intents
    /// preserve replay; pause/resume revoke lease data only. Returns durable state
    /// or typed preparation/commit/rollback/release causes; uncertain commit is
    /// not blindly retried and usage is never reset. No transport authentication,
    /// provider run, process stop or caller identity is established here. Local
    /// enablement consumes saved requirement/request limits; on Linux a configured
    /// provider adds a checked configuration snapshot and initial reviewer after
    /// replay, without a process or cooldown mutation. Repository remains unported. Durable replay precedes
    /// new canonical-size policy checks. A known successful/replayed Work outcome
    /// followed by release failure carries WorkRecorded metadata through callers;
    /// an uncertain commit never becomes a known recorded outcome.
    pub fn execute_external<E: WorkEnvironment>(
        &mut self,
        id: &str,
        kind: &str,
        raw: &[u8],
        environment: &mut E,
    ) -> Result<RootMutationOutcome, RootStoreError> {
        self.execute_with_fence(id, kind, raw, environment, None)
    }
    /// Share saved settings, reservation/retry and known-outcome cleanup for
    /// operator and supervisor routes. The optional guard represents the caller
    /// boundary, not a configurable bypass of mission ownership. Mission always
    /// supplies it; each retry revalidates inside the SQL transaction. No rights,
    /// budget, authenticated actor or implicit provider execution are granted.
    fn execute_with_fence<E: WorkEnvironment>(
        &mut self,
        id: &str,
        kind: &str,
        raw: &[u8],
        environment: &mut E,
        fence: Option<&dyn crate::work_engine::MutationFence>,
    ) -> Result<RootMutationOutcome, RootStoreError> {
        if !self.writable {
            return Err(RootStoreError::Refused("root-read-only"));
        }
        crate::check_command_input_size(raw).map_err(RootStoreError::Input)?;
        let mut attempts = 0u64;
        let mut previous: Option<WorkEngineError> = None;
        loop {
            let reservation = self
                .directory
                .reserve(false, false)
                .map_err(RootStoreError::Storage)?;
            let doc = match self.read_settings() {
                Ok(d) => d,
                Err(e) => {
                    return finished(Err(e), reservation.close().map_err(RootStoreError::Storage))
                }
            };
            if attempts > 0
                && doc
                    .values
                    .storage_busy_retries
                    .is_some_and(|max| attempts > max.0 as u64)
            {
                let e = RootStoreError::Busy {
                    cause: Box::new(match previous {
                        Some(e) => e,
                        None => {
                            return finished(
                                Err(RootStoreError::Refused("storage-retry-state")),
                                reservation.close().map_err(RootStoreError::Storage),
                            )
                        }
                    }),
                    attempts,
                    settings_revision: doc.revision,
                };
                return finished(Err(e), reservation.close().map_err(RootStoreError::Storage));
            }
            let prepared = (|| {
                self.db
                    .busy_timeout(Duration::from_millis(
                        doc.values.storage_busy_timeout_ms.0 as u64,
                    ))
                    .map_err(RootStoreError::Sql)?;
                let limits = doc
                    .values
                    .mutation_limits()
                    .map_err(RootStoreError::Settings)?;
                let policy = if kind == "gate" || doc.values.has_quality_policy() {
                    Some(
                        doc.quality_policy_snapshot()
                            .map_err(RootStoreError::Settings)?,
                    )
                } else {
                    None
                };
                Ok((limits, policy))
            })();
            let (limits, policy) = match prepared {
                Ok(v) => v,
                Err(e) => {
                    return finished(Err(e), reservation.close().map_err(RootStoreError::Storage))
                }
            };
            let mut policy_environment = crate::environment::PolicyEnvironment {
                inner: environment,
                settings: &doc,
            };
            let result =
                WorkEngine::new(&mut self.db, &self.root, &mut policy_environment, &limits)
                    .with_quality_policy(policy.as_ref())
                    .with_planning_settings(&doc)
                    .with_mutation_fence(fence)
                    .execute_external(id, kind, raw);
            let cleanup = reservation.close().map_err(RootStoreError::Storage);
            if cleanup.is_err() {
                return work_finished(
                    result.map_err(RootStoreError::Mutation),
                    cleanup,
                    doc.revision,
                    attempts,
                );
            }
            attempts = attempts
                .checked_add(1)
                .ok_or(RootStoreError::Refused("storage-retry-representation"))?;
            match result {
                Ok(result) => {
                    return Ok(RootMutationOutcome {
                        result,
                        settings_revision: doc.revision,
                        storage_attempts: attempts,
                    })
                }
                Err(error) => {
                    let mut source: Option<&(dyn Error + 'static)> = Some(&error);
                    let mut busy = false;
                    while let Some(e) = source {
                        if let Some(sql) = e.downcast_ref::<rusqlite::Error>() {
                            busy = sql
                                .sqlite_error()
                                .is_some_and(|e| e.extended_code & 255 == 5);
                            if busy {
                                break;
                            }
                        }
                        source = e.source();
                    }
                    if !busy || error.requires_reconciliation() {
                        return Err(RootStoreError::Mutation(error));
                    }
                    if doc
                        .values
                        .storage_busy_retries
                        .is_some_and(|max| attempts > max.0 as u64)
                    {
                        return Err(RootStoreError::Busy {
                            cause: Box::new(error),
                            attempts,
                            settings_revision: doc.revision,
                        });
                    }
                    previous = Some(error);
                    std::thread::sleep(Duration::from_millis(
                        doc.values.storage_busy_retry_delay_ms.0 as u64,
                    ));
                }
            }
        }
    }
    pub fn close(self) -> Result<(), RootStoreError> {
        let result = self.db.close().map_err(|(_, e)| RootStoreError::Sql(e));
        finished(
            result,
            self.directory.close().map_err(RootStoreError::Storage),
        )
    }
}

/// Observation bound to a recorded Work revision and supplied artifact reads.
/// It grants no permission to execute, share or accept a task.
#[derive(serde::Serialize)]
pub struct GateEvidenceObservation {
    pub work_id: String,
    pub work_revision: i64,
    pub task_id: String,
    pub evaluation: crate::Evaluation,
    pub policy: crate::QualityPolicySnapshot,
}
/// Actual row/journal revisions and a limited managed-review anchor observation.
///
/// Revisions identify the read snapshot; they reserve no future action or files.
/// This wrapper adds neither authority nor Git/protocol verification to `anchor`.
#[derive(Debug, serde::Serialize)]
pub struct RootManagedReviewAnchorObservation {
    pub work_id: String,
    pub work_revision: i64,
    pub settings_revision: SettingCount,
    pub anchor: crate::ManagedReviewAnchorObservation,
}
/// Read-only machine receipt and observed revisions, not an authenticated verdict.
#[derive(serde::Serialize)]
pub struct RootManagedReceiptObservation {
    pub scope: &'static str,
    pub work_id: String,
    pub work_revision: i64,
    pub task_id: String,
    pub settings_revision: SettingCount,
    pub receipt: Option<crate::AutomaticValidation>,
}
#[derive(Debug, serde::Serialize)]
pub struct RootReviewEvidenceObservation {
    pub scope: &'static str,
    pub work_id: String,
    pub work_revision: i64,
    pub task_id: String,
    pub settings_revision: SettingCount,
    pub review: Option<crate::ReviewEvidenceObservation>,
}

/// Actual row/policy revisions observed during candidate/files/context/Git inspection.
/// These revisions are diagnostics, not a future mutation lease or acceptance proof.
#[cfg(target_os = "linux")]
#[derive(Debug, serde::Serialize)]
pub struct RootManagedReviewGitObservation {
    pub work_id: String,
    pub work_revision: i64,
    pub settings_revision: SettingCount,
    pub observation: crate::ManagedReviewGitObservation,
}

/// Parsing observation using a real stored task and journal; supplied text is not authenticated.
#[derive(serde::Serialize)]
pub struct RootReviewReplyObservation {
    pub work_id: String,
    pub work_revision: i64,
    pub task_id: String,
    pub settings_revision: SettingCount,
    pub parsed: crate::ParsedReviewReply,
}

/// Historical controls observed from an actual stored task; no current verdict or authority.
#[derive(serde::Serialize)]
pub struct RootControlHistoryObservation {
    pub work_id: String,
    pub work_revision: i64,
    pub task_id: String,
    pub settings_revision: SettingCount,
    pub history: Vec<crate::EvidenceControl>,
}

/// Stored current validation data only; no identity, authority or execution provenance.
#[derive(serde::Serialize)]
pub struct RootValidationEvidenceObservation {
    pub work_id: String,
    pub work_revision: i64,
    pub task_id: String,
    pub settings_revision: SettingCount,
    pub evidence: Option<crate::ValidationEvidenceObservation>,
}

/// Read-only manual/ordinary/simple managed validation; no acceptance, identity or later permit.
#[derive(serde::Serialize)]
pub struct RootWorkValidationObservation {
    pub work_id: String,
    pub work_revision: i64,
    pub settings_revision: SettingCount,
    pub validation: crate::WorkValidation,
}

#[cfg(test)]
mod work_release_tests {
    use super::*;
    /// Result aggregation, not an actual OS fault: known outcome survives release,
    /// replay and count overflow; no pre-commit error is relabeled as success.
    #[test]
    fn work_outer_release_retains_known_recorded_state() {
        let outcome = || crate::WorkMutationOutcome {
            work: crate::Work {
                id: "w-fixture".into(),
                revision: 3,
                ..Default::default()
            },
            replayed: false,
            git_issues: vec![],
        };
        let witness = work_finished(Ok(outcome()), Ok(()), SettingCount(1), 0).unwrap();
        assert_eq!(witness.result.work.revision, 3);
        assert_eq!(witness.storage_attempts, 1);
        let error = work_finished(
            Ok(outcome()),
            Err(RootStoreError::Refused("fixture-work-release")),
            SettingCount(1),
            0,
        )
        .err()
        .unwrap();
        assert_eq!(error.code(), "work-state-preserved");
        assert_eq!(error.recorded_work(), Some(("w-fixture", 3)));
        assert!(std::error::Error::source(&error).is_some());
        match error {
            RootStoreError::WorkRecorded { primary, .. } => assert!(matches!(
                *primary,
                RootStoreError::Cleanup { primary: None, .. }
            )),
            _ => panic!("lost recorded state"),
        }
        let mut replay = outcome();
        replay.replayed = true;
        let error = work_finished(
            Ok(replay),
            Err(RootStoreError::Refused("fixture-work-release")),
            SettingCount(1),
            0,
        )
        .err()
        .unwrap();
        assert_eq!(error.recorded_work(), Some(("w-fixture", 3)));
        let error = work_finished(Ok(outcome()), Ok(()), SettingCount(1), u64::MAX)
            .err()
            .unwrap();
        assert_eq!(error.recorded_work(), Some(("w-fixture", 3)));
        let error = work_finished(
            Err(RootStoreError::Refused("fixture-precommit")),
            Err(RootStoreError::Refused("fixture-work-release")),
            SettingCount(1),
            0,
        )
        .err()
        .unwrap();
        assert!(error.recorded_work().is_none());
        match error {
            RootStoreError::Cleanup {
                primary: Some(primary),
                cleanup,
            } => {
                assert!(matches!(
                    *primary,
                    RootStoreError::Refused("fixture-precommit")
                ));
                assert!(matches!(
                    *cleanup,
                    RootStoreError::Refused("fixture-work-release")
                ));
            }
            _ => panic!("lost both failure causes"),
        }
        let recorded = RootStoreError::WorkRecorded {
            primary: Box::new(RootStoreError::Refused("fixture-release")),
            work: "w-fixture".into(),
            revision: 3,
        };
        let compound = RootStoreError::Cleanup {
            primary: Some(Box::new(recorded)),
            cleanup: Box::new(RootStoreError::Refused("fixture-extra-close")),
        };
        assert_eq!(compound.recorded_work(), Some(("w-fixture", 3)));
    }
}

#[cfg(test)]
mod settings_release_tests {
    use super::*;
    /// A saved/replayed settings result must survive outer-release classification.
    /// This models aggregation, not an actual OS release failure.
    #[test]
    fn settings_outer_release_retains_known_recorded_configuration() {
        let doc = || {
            EngineSettingsDocument::initial(
                EngineSettingsValues::installation_defaults().unwrap(),
                "fixture".into(),
                "2026-10-20T00:00:00Z".into(),
            )
            .unwrap()
        };
        assert!(settings_finished(Ok((doc(), false)), Ok(())).is_ok());
        let error = settings_finished(
            Ok((doc(), false)),
            Err(RootStoreError::Refused("fixture-settings-release")),
        )
        .err()
        .unwrap();
        assert_eq!(
            error.code(),
            "root-state-preserved",
            "known settings outcome lost after release"
        );
        assert!(error.configuration_may_be_recorded());
        for replayed in [false, true] {
            let error = settings_finished(
                Ok((doc(), replayed)),
                Err(RootStoreError::Refused("fixture-settings-release")),
            )
            .err()
            .unwrap();
            assert!(error.configuration_may_be_recorded());
            assert!(std::error::Error::source(&error).is_some());
        }
        let error = settings_finished(
            Err(RootStoreError::Refused("fixture-before-publication")),
            Err(RootStoreError::Refused("fixture-settings-release")),
        )
        .err()
        .unwrap();
        assert!(!error.configuration_may_be_recorded());
        let error = settings_finished(
            Err(RootStoreError::Published {
                primary: Box::new(RootStoreError::Refused("fixture-visible-uncertain")),
            }),
            Err(RootStoreError::Refused("fixture-settings-release")),
        )
        .err()
        .unwrap();
        assert!(error.configuration_may_be_recorded());
        match error {
            RootStoreError::Published { primary } => match *primary {
                RootStoreError::Cleanup {
                    primary: Some(prior),
                    cleanup,
                } => {
                    assert!(matches!(*prior, RootStoreError::Published { .. }));
                    assert!(matches!(
                        *cleanup,
                        RootStoreError::Refused("fixture-settings-release")
                    ));
                }
                _ => panic!("lost settings compound causes"),
            },
            _ => panic!("lost settings marker"),
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod planning_lease_pause_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    struct Fixture(PathBuf);
    impl Drop for Fixture {
        /// Remove only the owned isolated fixture after success; retain failures.
        fn drop(&mut self) {
            if !std::thread::panicking() {
                std::fs::remove_dir_all(&self.0).unwrap();
            }
        }
    }
    /// Public Work/claim/renew transactions against an isolated real Root. Only
    /// cockpit controls are seeded to model persisted legacy Go control rows;
    /// this is not proof that an unported Rust pause writer ran. Healthy renewal
    /// precedes pause/corrupt-boolean refusals and recovery without counter reset.
    #[test]
    fn planning_lease_renewal_respects_persisted_mission_pause_and_recovers() {
        let path =
            std::env::temp_dir().join(crate::generate_identifier("lease-pause115-").unwrap());
        std::fs::create_dir(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let fixture = Fixture(path);
        println!("owned legacy-controls fixture: {}", fixture.0.display());
        let mut env = crate::SystemEnvironment;
        let mut store = RootStore::initialize(&fixture.0, &mut env).unwrap();
        let created=store.execute_external("","work.create",&serde_json::to_vec(&serde_json::json!({"schema_version":1,"event_id":"create","expected_revision":0,"title":"Lease pause","objective":"Respect persisted mission pause","scope":"Owned fixture","criteria":["Paused lease cannot renew"]})).unwrap(),&mut env).unwrap().result.work;
        let enabled = store
            .execute_external(
                &created.id,
                "planning.enable",
                &crate::encode_planning_request(&crate::PlanningRequest {
                    schema_version: 1,
                    event_id: "enable".into(),
                    expected_revision: created.revision,
                    max_activations: 3,
                    max_tasks: 3,
                    max_decisions: 3,
                    ..Default::default()
                })
                .unwrap(),
                &mut env,
            )
            .unwrap()
            .result
            .work;
        let request = crate::PlanningRequest {
            schema_version: 1,
            event_id: "claim".into(),
            expected_revision: enabled.revision,
            scope: "root".into(),
            scope_revision: 1,
            holder: "owned-planner".into(),
            lease_seconds: 300,
            ..Default::default()
        };
        let claimed = store
            .execute_external(
                &created.id,
                "planning.claim",
                &crate::encode_planning_request(&request).unwrap(),
                &mut env,
            )
            .unwrap()
            .result
            .work;
        store.db.execute("INSERT INTO cockpit_controls(work_id,paused,autonomy,slots) VALUES (?1,0,'manuel',1)",[&created.id]).unwrap();
        let renewal = crate::PlanningRequest {
            event_id: "healthy-renew".into(),
            expected_revision: claimed.revision,
            generation: 1,
            ..request.clone()
        };
        let healthy = store
            .execute_external(
                &created.id,
                "planning.lease-renew",
                &crate::encode_planning_request(&renewal).unwrap(),
                &mut env,
            )
            .unwrap()
            .result
            .work;
        assert_eq!(healthy.planning.as_ref().unwrap().activations, 1);
        let renewal = crate::PlanningRequest {
            event_id: "paused-renew".into(),
            expected_revision: healthy.revision,
            ..renewal
        };
        let before = store.read_work(&created.id).unwrap();
        store
            .db
            .execute(
                "UPDATE cockpit_controls SET paused=1 WHERE work_id=?1",
                [&created.id],
            )
            .unwrap();
        let refused = store.execute_external(
            &created.id,
            "planning.lease-renew",
            &crate::encode_planning_request(&renewal).unwrap(),
            &mut env,
        );
        assert!(refused.is_err(), "Persisted mission pause was ignored");
        assert_eq!(refused.err().unwrap().code(), "planning-mission-paused");
        assert_eq!(store.read_work(&created.id).unwrap(), before);
        store
            .db
            .execute(
                "UPDATE cockpit_controls SET paused=2 WHERE work_id=?1",
                [&created.id],
            )
            .unwrap();
        let refused = store.execute_external(
            &created.id,
            "planning.lease-renew",
            &crate::encode_planning_request(&renewal).unwrap(),
            &mut env,
        );
        assert_eq!(refused.err().unwrap().code(), "planning-controls-invalid");
        assert_eq!(store.read_work(&created.id).unwrap(), before);
        store
            .db
            .execute(
                "UPDATE cockpit_controls SET paused=0 WHERE work_id=?1",
                [&created.id],
            )
            .unwrap();
        let recovered = store
            .execute_external(
                &created.id,
                "planning.lease-renew",
                &crate::encode_planning_request(&renewal).unwrap(),
                &mut env,
            )
            .unwrap()
            .result
            .work;
        assert_eq!(recovered.planning.as_ref().unwrap().activations, 1);
        assert_eq!(recovered.planning.as_ref().unwrap().decisions, 0);
        store.close().unwrap();
    }

    /// Build an owned local-only Work through public transactions; labels keep
    /// eventual global planning-call identifiers distinct across every fixture.
    fn planning_guard_case(
        store: &mut RootStore,
        label: &str,
        kind: &str,
    ) -> (crate::Work, crate::PlanningRequest) {
        let label = label.replace('.', "-");
        let mut env = crate::SystemEnvironment;
        let created=store.execute_external("","work.create",&serde_json::to_vec(&serde_json::json!({"schema_version":1,"event_id":format!("create-{label}"),"expected_revision":0,"title":"Planning hook fixture","objective":"Respect canonical mission state","scope":"Owned fixture","criteria":["No paused or archived mutations"]})).unwrap(),&mut env).unwrap().result.work;
        let enabled = store
            .execute_external(
                &created.id,
                "planning.enable",
                &crate::encode_planning_request(&crate::PlanningRequest {
                    schema_version: 1,
                    event_id: format!("enable-{label}"),
                    expected_revision: created.revision,
                    max_activations: 3,
                    max_tasks: 3,
                    max_decisions: 3,
                    ..Default::default()
                })
                .unwrap(),
                &mut env,
            )
            .unwrap()
            .result
            .work;
        let mut request = crate::PlanningRequest {
            schema_version: 1,
            event_id: format!("claim-{label}"),
            expected_revision: enabled.revision,
            scope: "root".into(),
            scope_revision: 1,
            holder: "owned-planner".into(),
            lease_seconds: 300,
            ..Default::default()
        };
        let work = if kind == "planning.claim" {
            enabled
        } else {
            store
                .execute_external(
                    &created.id,
                    "planning.claim",
                    &crate::encode_planning_request(&request).unwrap(),
                    &mut env,
                )
                .unwrap()
                .result
                .work
        };
        request.expected_revision = work.revision;
        request.event_id = format!("target-{label}");
        if kind != "planning.claim" {
            request.generation = 1;
        }
        if kind == "planning.decide" {
            request.lease_seconds = 0;
            request.reason = "Observe original input".into();
            request.input_events = Some(vec![format!("enable-{label}")]);
        }
        (work, request)
    }
    /// Healthy public action precedes each legacy-control fault. SQL seeds only
    /// isolated persisted pause/archive fixtures, never live state or an assertion
    /// that a Rust pause/archive writer ran. All failures aggregate so one missing
    /// guard cannot hide another; accepted faulty mutations stay on failed Roots.
    #[test]
    fn planning_transaction_hooks_pause_archive_and_corrupt_controls_are_atomic() {
        let path =
            std::env::temp_dir().join(crate::generate_identifier("planning-hooks116-").unwrap());
        std::fs::create_dir(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let fixture = Fixture(path);
        println!("owned legacy-hook fixture: {}", fixture.0.display());
        let mut store = RootStore::initialize(&fixture.0, &mut crate::SystemEnvironment).unwrap();
        let mut failures = Vec::new();
        for kind in ["planning.claim", "planning.decide", "planning.lease-renew"] {
            for (fault, expected) in [
                ("pause", "planning-mission-paused"),
                ("archive", "mission_archived"),
                ("malformed", "planning-controls-invalid"),
            ] {
                let label = format!("{kind}-{fault}-healthy");
                let (work, request) = planning_guard_case(&mut store, &label, kind);
                store.db.execute("INSERT INTO cockpit_controls(work_id,paused,autonomy,slots) VALUES (?1,0,'manuel',1)",[&work.id]).unwrap();
                store
                    .execute_external(
                        &work.id,
                        kind,
                        &crate::encode_planning_request(&request).unwrap(),
                        &mut crate::SystemEnvironment,
                    )
                    .unwrap();
                let label = format!("{kind}-{fault}-fault");
                let (work, request) = planning_guard_case(&mut store, &label, kind);
                store.db.execute("INSERT INTO cockpit_controls(work_id,paused,autonomy,slots) VALUES (?1,0,'manuel',1)",[&work.id]).unwrap();
                let before = store.read_work(&work.id).unwrap();
                match fault {
                    "archive" => {
                        store.db.execute("INSERT INTO mission_lifecycle(work_id,archived_at,archive_path,archive_digest) VALUES (?1,'2026-10-20T12:00:00Z','owned-fixture',?2)",rusqlite::params![&work.id,"0".repeat(64)]).unwrap();
                    }
                    _ => {
                        store
                            .db
                            .execute(
                                "UPDATE cockpit_controls SET paused=?1 WHERE work_id=?2",
                                rusqlite::params![if fault == "pause" { 1 } else { 2 }, &work.id],
                            )
                            .unwrap();
                    }
                }
                let result = store.execute_external(
                    &work.id,
                    kind,
                    &crate::encode_planning_request(&request).unwrap(),
                    &mut crate::SystemEnvironment,
                );
                match result {
                    Ok(_) => {
                        failures.push(format!("{kind}/{fault}: accepted faulty mutation"));
                        continue;
                    }
                    Err(e) => {
                        if e.code() != expected {
                            failures.push(format!(
                                "{kind}/{fault}: expected {expected}, observed {}",
                                e.code()
                            ));
                        }
                    }
                }
                if store.read_work(&work.id).unwrap() != before {
                    failures.push(format!("{kind}/{fault}: changed Work on refusal"));
                }
                store
                    .db
                    .execute("DELETE FROM mission_lifecycle WHERE work_id=?1", [&work.id])
                    .unwrap();
                store
                    .db
                    .execute(
                        "UPDATE cockpit_controls SET paused=0 WHERE work_id=?1",
                        [&work.id],
                    )
                    .unwrap();
                store
                    .execute_external(
                        &work.id,
                        kind,
                        &crate::encode_planning_request(&request).unwrap(),
                        &mut crate::SystemEnvironment,
                    )
                    .unwrap();
            }
        }
        store.close().unwrap();
        assert!(failures.is_empty(), "{}", failures.join("; "));
    }

    /// A healthy public claim precedes each isolated auxiliary-schema fault.
    /// Only test schema is injected; rejection must roll back Work/event/financial
    /// reserve, and explicit fixture repair permits the same uncommitted event.
    #[test]
    fn planning_claim_input_schema_refuses_atomically_then_recovers() {
        for fault in ["view", "ddl", "index", "trigger"] {
            for healthy in [true, false] {
                let path = std::env::temp_dir()
                    .join(crate::generate_identifier("claim-input117-").unwrap());
                std::fs::create_dir(&path).unwrap();
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
                let fixture = Fixture(path);
                println!("owned claim-input fixture: {}", fixture.0.display());
                let mut store =
                    RootStore::initialize(&fixture.0, &mut crate::SystemEnvironment).unwrap();
                let (work, r) = planning_guard_case(&mut store, "claim-input", "planning.claim");
                let before = store.read_work(&work.id).unwrap();
                if !healthy {
                    match fault {
                        "view" => store
                            .db
                            .execute_batch("CREATE VIEW native_planning_claims AS SELECT 1")
                            .unwrap(),
                        "ddl" => store
                            .db
                            .execute_batch("CREATE TABLE native_planning_claims(untrusted TEXT)")
                            .unwrap(),
                        "index" => {
                            crate::planning_call_inputs::schema(&store.db, true).unwrap();
                            store.db.execute_batch("CREATE INDEX claim_input_extra ON native_planning_claims(scope_id)").unwrap();
                        }
                        _ => {
                            crate::planning_call_inputs::schema(&store.db, true).unwrap();
                            store.db.execute_batch("CREATE TRIGGER claim_input_extra AFTER INSERT ON native_planning_claims BEGIN SELECT RAISE(IGNORE); END").unwrap();
                        }
                    }
                }
                let result = store.execute_external(
                    &work.id,
                    "planning.claim",
                    &crate::encode_planning_request(&r).unwrap(),
                    &mut crate::SystemEnvironment,
                );
                if !healthy {
                    assert_eq!(
                        result.err().unwrap().code(),
                        "planning-claim-input-schema-invalid"
                    );
                    assert_eq!(store.read_work(&work.id).unwrap(), before);
                    for table in ["events", "planning_calls"] {
                        let sql = format!("SELECT count(*) FROM {table} WHERE id=?1");
                        let count: i64 = store
                            .db
                            .query_row(&sql, [&r.event_id], |row| row.get(0))
                            .unwrap();
                        assert_eq!(count, 0);
                    }
                    store
                        .db
                        .execute_batch(match fault {
                            "view" => "DROP VIEW native_planning_claims",
                            "ddl" => "DROP TABLE native_planning_claims",
                            "index" => "DROP INDEX claim_input_extra",
                            _ => "DROP TRIGGER claim_input_extra",
                        })
                        .unwrap();
                    store
                        .execute_external(
                            &work.id,
                            "planning.claim",
                            &crate::encode_planning_request(&r).unwrap(),
                            &mut crate::SystemEnvironment,
                        )
                        .unwrap();
                } else {
                    result.unwrap();
                }
                let count: i64 = store
                    .db
                    .query_row(
                        "SELECT count(*) FROM native_planning_claims WHERE claim_id=?1",
                        [&r.event_id],
                        |row| row.get(0),
                    )
                    .unwrap();
                assert_eq!(count, 1);
                assert_eq!(
                    store
                        .read_work(&work.id)
                        .unwrap()
                        .planning
                        .as_ref()
                        .unwrap()
                        .activations,
                    1
                );
                store.close().unwrap();
            }
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod bootstrap_busy_tests {
    use super::*;
    use std::{
        io::{BufRead, BufReader, Write},
        process::{Child, Command, Stdio},
        sync::atomic::{AtomicUsize, Ordering},
    };

    static BUSY_CALLS: AtomicUsize = AtomicUsize::new(0);

    /// Count actual SQLite contention without SQL reentrancy, sleeping or retry.
    fn refuse_busy(_: i32) -> bool {
        BUSY_CALLS.fetch_add(1, Ordering::SeqCst);
        false
    }

    /// Own only a fresh private fixture; failed assertions preserve its files.
    struct Fixture(PathBuf);
    impl Fixture {
        /// Initialize and close a genuine Root through its supported API.
        fn new() -> Self {
            let path = std::env::temp_dir()
                .join(crate::generate_identifier("bootstrap-busy201-").unwrap());
            fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
            let store = RootStore::initialize(&path, &mut crate::SystemEnvironment).unwrap();
            store.close().unwrap();
            Self(path)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            if !std::thread::panicking() {
                fs::remove_dir_all(&self.0).unwrap();
            }
        }
    }

    /// Independent process holds a real DELETE-journal exclusive lock. EOF or
    /// explicit release rolls it back; parent cleanup waits for its actual exit.
    struct LockOwner(Child);
    impl LockOwner {
        /// Wait for the child's explicit lock-acquired acknowledgement; EOF
        /// fails the prerequisite rather than inventing a contention result.
        fn acquire(path: &Path) -> Self {
            let script = "import sqlite3,sys\nc=sqlite3.connect(sys.argv[1],timeout=0)\nassert c.execute('PRAGMA journal_mode=DELETE').fetchone()[0]=='delete'\nc.execute('BEGIN EXCLUSIVE')\nprint('READY',flush=True)\nsys.stdin.readline()\nc.rollback()\nc.close()\n";
            let mut child = Command::new("python3")
                .args(["-c", script]).arg(path)
                .stdin(Stdio::piped()).stdout(Stdio::piped())
                .stderr(Stdio::inherit()).spawn().unwrap();
            let mut ready = String::new();
            BufReader::new(child.stdout.take().unwrap()).read_line(&mut ready).unwrap();
            let owner = Self(child);
            assert_eq!(ready, "READY\n", "independent lock prerequisite failed");
            owner
        }
    }
    impl Drop for LockOwner {
        fn drop(&mut self) {
            if let Some(mut input) = self.0.stdin.take() {
                // Closing stdin also releases the child if publication fails.
                let sent = input.write_all(b"release\n");
                drop(input);
                let status = self.0.wait().unwrap();
                assert!(status.success(), "lock owner failed: {status}");
                sent.unwrap();
            }
        }
    }

    /// A valid unlocked observation precedes the fault. Under an independent
    /// exclusive lock, capture really calls the installed handler, whereas the
    /// bootstrap phase removes it BEFORE that same capture. No duration cutoff
    /// or replacement builtin can satisfy this ordering guard. Release restores
    /// a real successful capture; the test grants no agent permission.
    #[test]
    fn root_bootstrap_clears_constructor_handler_before_identity_sql() {
        let fixture = Fixture::new();
        // Zero is now an explicit saved operator input for this refusal case,
        // rather than an unconditional bootstrap policy hidden in production.
        let store = RootStore::open(&fixture.0, true).unwrap();
        let document = store.settings().unwrap();
        let mut values = document.values.clone();
        values.storage_busy_timeout_ms = SettingCount(0);
        let mut change = EngineSettingsChange {
            schema_version: document.schema_version, event_id:"zero-bootstrap-fixture".into(),
            expected_revision:document.revision, values,
            reason:"Explicit zero-timeout contention witness".into(), preview_token:String::new(),
        };
        change.preview_token = store.preview_settings(&change).unwrap().preview_token;
        store.apply_settings(&change, &mut crate::SystemEnvironment).unwrap();
        store.close().unwrap();
        let path = fixture.0.join(".swarm/state.db");
        let directory = LocalDirectory::open(&fixture.0.join(".swarm")).unwrap();
        let db = Connection::open_with_flags(&path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW
            | OpenFlags::SQLITE_OPEN_NO_MUTEX).unwrap();
        assert_eq!(db.query_row("PRAGMA busy_timeout", [], |r| r.get::<_, i64>(0)).unwrap(), 5000);
        observe_instance(&db, &directory).unwrap();
        db.close().unwrap();
        // Do not reuse a schema-warmed connection: initial capture must face the
        // same first-SQL boundary as open_mode after its constructor returns.
        let db = Connection::open_with_flags(&path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW
            | OpenFlags::SQLITE_OPEN_NO_MUTEX).unwrap();
        db.busy_handler(Some(refuse_busy)).unwrap();
        let owner = LockOwner::acquire(&path);
        BUSY_CALLS.store(0, Ordering::SeqCst);
        let before = observe_instance(&db, &directory).expect_err("lock must refuse capture");
        assert_eq!(before.code(), "root-database-identity");
        assert!(BUSY_CALLS.load(Ordering::SeqCst) > 0, "control did not enter busy handler");
        BUSY_CALLS.store(0, Ordering::SeqCst);
        let after = bootstrap_instance(&db, &directory).expect_err("lock must still refuse capture");
        assert_eq!(after.code(), "root-database-identity");
        assert_eq!(BUSY_CALLS.load(Ordering::SeqCst), 0, "bootstrap SQL preceded handler reset");
        drop(owner);
        bootstrap_instance(&db, &directory).unwrap();
        assert_eq!(db.query_row("PRAGMA busy_timeout", [], |r| r.get::<_, i64>(0)).unwrap(), 0);
        db.close().unwrap();
        directory.close().unwrap();
    }

    /// Persist a nonzero Admin setting through the actual preview/apply API,
    /// reopen Root, and retain its effective value through all binding reads.
    /// The nonzero value is fixture data, not a new product policy/default.
    #[test]
    fn root_binding_preserves_saved_admin_busy_timeout() {
        let fixture = Fixture::new();
        let store = RootStore::open(&fixture.0, true).unwrap();
        let document = store.settings().unwrap();
        let mut values = document.values.clone();
        values.storage_busy_timeout_ms = SettingCount(173);
        let mut change = EngineSettingsChange {
            schema_version: document.schema_version,
            event_id: "fixture-bootstrap-timeout".into(),
            expected_revision: document.revision,
            values, reason: "Verify binding preserves saved timeout".into(),
            preview_token: String::new(),
        };
        change.preview_token = store.preview_settings(&change).unwrap().preview_token;
        store.apply_settings(&change, &mut crate::SystemEnvironment).unwrap();
        store.close().unwrap();
        let store = RootStore::open(&fixture.0, true).unwrap();
        let timeout = || store.db.query_row("PRAGMA busy_timeout", [], |r| r.get::<_, i64>(0)).unwrap();
        assert_eq!(timeout(), 173);
        let binding = store.bind_root().unwrap();
        assert!(binding.matches(&store).unwrap());
        assert_eq!(timeout(), 173);
        binding.close().unwrap();
        store.close().unwrap();
    }

    /// Cold first-SQL identity capture must consume the saved operator timeout,
    /// not only restore it after capture. Values are fixture inputs, not policy.
    #[test]
    fn root_bootstrap_consumes_saved_timeout_at_first_sql() {
        let fixture = Fixture::new();
        let store = RootStore::open(&fixture.0, true).unwrap();
        let document = store.settings().unwrap();
        let mut values = document.values.clone();
        values.storage_busy_timeout_ms = SettingCount(173);
        let mut change = EngineSettingsChange {
            schema_version: document.schema_version,
            event_id: "cold-bootstrap-timeout".into(),
            expected_revision: document.revision,
            values,
            reason: "Verify saved policy before first identity SELECT".into(),
            preview_token: String::new(),
        };
        change.preview_token = store.preview_settings(&change).unwrap().preview_token;
        store.apply_settings(&change, &mut crate::SystemEnvironment).unwrap();
        store.close().unwrap();
        let directory = LocalDirectory::open(&fixture.0.join(".swarm")).unwrap();
        let db = Connection::open_with_flags(fixture.0.join(".swarm/state.db"),
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW
            | OpenFlags::SQLITE_OPEN_NO_MUTEX).unwrap();
        bootstrap_instance(&db, &directory).unwrap();
        assert_eq!(db.query_row("PRAGMA busy_timeout", [], |r| r.get::<_, i64>(0)).unwrap(),173);
        db.close().unwrap();
        directory.close().unwrap();
    }
}

#[cfg(all(test, target_os = "linux"))]
mod material_path_tests;

#[cfg(test)]
mod migration_review_error_tests {
    use super::*;
    /// A precommit refusal has no known Work. Newer same-Work commits behind
    /// launch/custom-IO/cleanup survive an older stage snapshot; no rollback,
    /// current revision, execution or cross-Work ordering is inferred.
    #[test]
    fn migration_review_error_latest_known_revision_survives_older_wrapper(){
        assert!(RootStoreError::Refused("fixture").recorded_work().is_none());
        let recorded=|work:&str,revision|RootStoreError::WorkRecorded{primary:Box::new(RootStoreError::Refused("fixture")),work:work.into(),revision};
        let error=RootStoreError::WorkRecorded{work:"fixture-work".into(),revision:5,primary:Box::new(RootStoreError::Cleanup{
            primary:Some(Box::new(recorded("fixture-work",7))),
            cleanup:Box::new(RootStoreError::Launch(Box::new(crate::AgentLaunchError::caused("fixture-late-output",std::io::Error::other(recorded("fixture-work",9))))))
        })};
        assert_eq!(error.recorded_work(),Some(("fixture-work",9)));
        let cross=RootStoreError::WorkRecorded{work:"fixture-work".into(),revision:5,primary:Box::new(recorded("different-work",100))};
        assert_eq!(cross.recorded_work(),Some(("fixture-work",5)));
    }
}

#[cfg(all(test,target_os="linux"))]
mod interruption_scope_tests {
    use super::*;
    /// Pure outcome guard: only the exact clean recorded stop qualifies. A
    /// secondary cleanup or other refusal must remain a failed command.
    #[test]
    fn interruption_marker_rejects_cleanup_and_other_failures(){
        let clean=||RootStoreError::WorkRecorded{primary:Box::new(RootStoreError::Refused("planning-execution-interrupted")),work:"fixture-work".into(),revision:3};
        assert_eq!(clean().recorded_planner_interruption(),Some(("fixture-work",3)));
        let failed=RootStoreError::Cleanup{primary:Some(Box::new(clean())),cleanup:Box::new(RootStoreError::Refused("fixture-close"))};
        assert!(failed.recorded_planner_interruption().is_none());
        let other=RootStoreError::WorkRecorded{primary:Box::new(RootStoreError::Refused("planning-provider-exit-unsuccessful")),work:"fixture-work".into(),revision:3};
        assert!(other.recorded_planner_interruption().is_none());
    }
    /// Private metadata shape only, not actual publication or authority. Runtime
    /// marker must be typed and outer cleanup/format lookalikes never qualify.
    #[test]
    fn interruption_role_marker_preserves_original_and_rejects_outer_failure(){
        for (role,name) in [(crate::execution_interrupt::InterruptedRole::Controls,"controls"),(crate::execution_interrupt::InterruptedRole::Reviewer,"reviewer")]{
            let marker=crate::execution_interrupt::completed(role,RootStoreError::Refused("fixture-original-failure"));
            let recorded=RootStoreError::WorkRecorded{primary:Box::new(marker),work:"fixture-work".into(),revision:4};
            assert_eq!(recorded.recorded_execution_interruption(),Some((name,"fixture-work",4)));
            let labels=crate::native_diagnostics::project(&recorded);
            assert!(labels.iter().any(|cause|cause.labels.iter().any(|label|label=="fixture-original-failure")));
            let failed=RootStoreError::Cleanup{primary:Some(Box::new(recorded)),cleanup:Box::new(RootStoreError::Refused("fixture-close-failed"))};
            assert!(failed.recorded_execution_interruption().is_none());
        }
        let text=RootStoreError::WorkRecorded{primary:Box::new(RootStoreError::Runtime(crate::RuntimeError::new(crate::RuntimeOperation::Platform,std::io::Error::other("execution-interrupted")))),work:"fixture-work".into(),revision:4};
        assert!(text.recorded_execution_interruption().is_none());
    }
    /// Actual initialized idle Root: nested local scopes restore prior state,
    /// including the exact panic payload. No business mutation/permission occurs.
    #[test]
    fn interruption_scope_restores_nested_state_and_unwind(){
        use std::os::unix::fs::PermissionsExt;
        let path=std::env::temp_dir().join(crate::generate_identifier("interrupt-scope-").unwrap());
        std::fs::create_dir(&path).unwrap();std::fs::set_permissions(&path,std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut root=RootStore::initialize(&path,&mut crate::SystemEnvironment).unwrap();
        let source=crate::ExecutionInterrupt::new().unwrap();let inner=crate::ExecutionInterrupt::new().unwrap();
        inner.request(crate::ExecutionInterruptReason::Terminate).unwrap();
        root.with_execution_interrupt(&source,|root|{
            assert_eq!(root.interruption.as_ref().unwrap().reason().unwrap(),None);
            root.with_execution_interrupt(&inner,|root|assert_eq!(root.interruption.as_ref().unwrap().reason().unwrap(),Some(crate::ExecutionInterruptReason::Terminate)));
            assert_eq!(root.interruption.as_ref().unwrap().reason().unwrap(),None);
        });
        assert!(root.interruption.is_none());
        let panic=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||root.with_execution_interrupt(&source,|_|std::panic::panic_any(123_u32))));
        assert_eq!(*panic.unwrap_err().downcast::<u32>().unwrap(),123);
        assert!(root.interruption.is_none());root.close().unwrap();std::fs::remove_dir_all(path).unwrap();
    }
}

pub(crate) mod preparation_read;
#[cfg(target_os = "linux")]
mod preparation_source;
#[cfg(target_os = "linux")]
mod preparation_methods;
pub use preparation_read::{PreparationReadError, PreparationReadObservation, PreparationList};

mod budget_history;
pub use budget_history::{HistoricalBudgetView, HistoricalBudgetShow, HistoricalBudgetPreview};

#[cfg(target_os = "linux")]
mod preparation_write;
#[cfg(target_os = "linux")]
mod preparation_conversion;
mod preparation_plan;
