//! Native engine construction. Request data is not authority or a running engine.
#![forbid(unsafe_code)]
mod biba;
pub use biba::{Integrity, BibaAccess, BibaDenied, check_biba};
mod trust_types;
#[cfg(unix)]
mod trust;
#[cfg(unix)]
pub use trust::{TrustError,TrustReply,verify_trust};
#[cfg(target_os = "linux")]
mod agent_launch;
#[cfg(target_os = "linux")]
pub use agent_launch::AgentLaunchOutcome;
mod agent_launch_error;
mod task_model;
mod task_report;
pub use task_model::{decode_task_model_request, TaskModelPreview, TaskModelRequest};
pub use task_report::{decode_task_report_request, TaskReportRequest};
#[cfg(target_os = "linux")]
mod agent_reviewer_preflight;
#[cfg(target_os = "linux")]
#[cfg(target_os = "linux")]
mod assistant_provider;
#[cfg(target_os = "linux")]
pub use assistant_provider::{prepare_assistant_provider, AssistantProviderError};
mod agent_worker_context;
pub use agent_launch_error::AgentLaunchError;
mod agent_tool_guard;
pub use agent_tool_guard::{
    AgentGuardError, AgentGuardObservation, AgentGuardStop, AgentGuardVisibility, AgentToolGuard,
};
mod agent_guard_trace;
pub use agent_guard_trace::{
    decode_agent_guard_trace, replay_agent_guard, AgentGuardEvent, AgentGuardReplay,
    AgentGuardTrace,
};
mod agent_guard_policy;
pub use agent_guard_policy::{AgentGuardPolicy, AgentGuardSnapshot};
#[cfg(target_os = "linux")]
mod agent_completion_journal;
#[cfg(target_os = "linux")]
mod agent_execution_gate;
#[cfg(target_os = "linux")]
mod agent_execution_preflight;
mod agent_execution_snapshot;
#[cfg(target_os = "linux")]
mod agent_execution_store;
mod agent_scope_signals;
#[cfg(target_os = "linux")]
mod native_provider_exec;
#[cfg(target_os = "linux")]
pub use agent_execution_gate::execute_agent_gate;
pub use agent_execution_snapshot::{
    decode_agent_execution_snapshot, AgentExecutionPhase, AgentExecutionSnapshot,
    AgentExecutionSnapshotError,
};
mod agent_execution_policy;
mod agent_native_stream;
mod agent_provider_facts;
#[cfg(target_os = "linux")]
mod agent_supervision;
#[cfg(target_os = "linux")]
pub use agent_supervision::AgentExecutionOutcome;
mod reviewer_bootstrap;
#[cfg(target_os = "linux")]
mod managed_repository;
mod reviewer_creation_policy;
pub use reviewer_creation_policy::ReviewerCreationDefaults;
mod mission_dispatch_policy;
pub use mission_dispatch_policy::MissionDispatchPolicy;
#[cfg(target_os = "linux")]
pub mod mission_status_contract;
#[cfg(target_os="linux")]
mod execution_interrupt;
#[cfg(target_os="linux")]
pub use execution_interrupt::{ExecutionInterrupt,ExecutionInterruptReason};
mod version_identity;
pub use version_identity::{binary_version,decode_binary_version,binary_version_observation,compare_version_observations,BuildIdentityError};
#[cfg(target_os = "linux")]
pub mod mission_status_insights;
#[cfg(target_os = "linux")]
pub mod mission_status_tasks;
#[cfg(target_os = "linux")]
pub mod mission_status_spending;
#[cfg(target_os = "linux")]
mod mission_status_runtime;
mod mission_organization;
pub use mission_organization::{
    validate_mission_organization, validate_mission_reviewer_capacity, MissionOrganizationError,
};
mod mission_control_request;
pub use mission_control_request::{decode_mission_priority_request, decode_mission_control_request, decode_mission_start_request, MissionControlAction, MissionControlRequest, MissionControlView};
mod mission_supervision;
pub use mission_supervision::{
    MissionLeaseClock, MissionSupervisionError, MissionSupervisionPolicy,
};
#[cfg(target_os = "linux")]
pub use root_store::{
    MissionAutonomy, MissionConsentObservation, MissionControlSettings, MissionControlsObservation,
    CostSummary, CostTotal,
    TaskFreshnessEvidence, MissionResultObservation, MissionStatusObservation,
    MissionStatusInputs, MissionStatusVisit, MissionStatusPlanningCall, MissionStatusExchange, MissionStatusExchangeArtifact,
    MissionWorkspaceQueue, MissionWorkspaceTurn, MissionWorkspaceBlocker,
    MissionQueueRegistration, MissionCandidateRefusal,
    MissionWorkerAdoptionRequest, MissionWorkerAdoptionOutcome, MissionWorkerRecovery,
    MissionWorkerDispatchRequest, MissionWorkerDispatchOutcome, MissionWorkerDispatchFailure,
    MissionDispatchCause, MissionDispatchQueueEffect, MissionDispatchPreparationEffect,
    MissionDispatchResourceRefusal,
    MissionWorkerContinuationObservation, MissionWorkerContinuationEntry,
    MissionWorkerContinuationState, MissionContinuationFailure, MissionContinuationProgress,
    MissionTaskOrder, MissionTaskPosition, MissionDispatchObservation, MissionLease, MissionSupervisionObservation,
    MissionSupervisionState, MissionWorkReceipt, MissionWorkerRequest, MissionCycleFailure, MissionCycleMonitor, MissionCycleCause, MissionCycleDiagnostics,
};
#[cfg(unix)]
pub use root_store::HistoricalAgentLog;
mod agent_heartbeat;
pub use agent_heartbeat::{observe_agent_heartbeat, AgentHeartbeatObservation};
mod agent_terminal_text;
mod terminal_clip;
pub use agent_execution_policy::AgentExecutionPolicy;
mod agent_launch_policy;
#[cfg(target_os = "linux")]
mod agent_launch_preflight;
pub use agent_launch_policy::AgentLaunchPolicy;
mod model_routing_policy;
pub use model_routing_policy::{
    AgentModelRoutingPolicy, ModelPolicyCheck, ModelPolicyCheckRequest,
};
mod provider_configuration;
mod provider_policy_import;
pub use provider_policy_import::{
    decode_provider_policy_change, ProviderPolicyChange, ProviderPolicyExport,
    ProviderPolicyMutation,
};
#[cfg(target_os = "linux")]
mod provider_connection_catalogue;
pub use provider_configuration::{
    decode_provider_configuration, decode_provider_configuration_change, ProviderConfiguration,
    ProviderConfigurationBackups, ProviderConfigurationChange, ProviderConfigurationError,
    ProviderConfigurationInspection, ProviderConfigurationMutation,
};
mod provider_model_arguments;
#[cfg(target_os = "linux")]
mod provider_model_route;
pub use provider_model_arguments::{compile_provider_model_arguments, ProviderModelSyntax};
mod agent_models;
pub use agent_models::{
    Agent, DialogueRestoreChoice, DialogueRestoreReceipt, Launch, ModelChoice, ModelPolicy,
    PreflightCheck, PreflightResult, PreparedSkillRefreshReceipt, Provider, ProviderCooldown,
    Providers, RecoveryState, ReportedModel,
};
mod agent_contract;
pub use agent_contract::{decode_agent, decode_agent_row, decode_launch, decode_providers};
#[cfg(unix)]
mod agent_storage_retry;
mod agent_live_read;
#[cfg(target_os = "linux")]
mod agent_resources;
#[cfg(target_os = "linux")]
pub use agent_resources::{observe_agent_resources, AgentResources, ProcessSlot, ProcessTable};
pub use agent_live_read::{
    read_agent_logs_after, read_live_fingerprint, read_terminal_events_after, AgentLogEntry, CursorPage, TerminalChunk,
    LIVE_PAGE_ROWS,
};
mod live_alerts;
pub use live_alerts::{alert_conditions, alert_subjects, budget_conditions, usage_mismatch_conditions, AlertCondition, WorkBudgetState, ALERT_RULES};
mod agent_transcript;
pub use agent_transcript::{typed_transcript, Transcript, TranscriptEntry};
mod attempt_evaluations;
pub use attempt_evaluations::{evaluate_attempt, evaluation_sampled, AttemptEvaluation, EvaluationPolicy};
mod task_scenarios;
pub use task_scenarios::{evaluate_scenario, validate_scenario, Expectation, ExpectationResult, ScenarioSpec};
mod failure_groups;
pub use failure_groups::{failure_cause, failure_groups, FailureGroup, FailureMember};
mod telemetry_runs;
pub use telemetry_runs::TelemetryRun;
mod telemetry_normalize;
pub use telemetry_normalize::{
    normalize_logs, normalize_metrics, normalize_traces, NormalizedBatch, Temporality, TelemetryInputError, TelemetryKind,
    TelemetryRecord, TELEMETRY_FIELD_MAX_CHARS,
};
#[cfg(target_os = "linux")]
mod telemetry_endpoint;
#[cfg(target_os = "linux")]
pub use telemetry_endpoint::{
    apply_saved_retention, launch_telemetry_environment, publish_telemetry_endpoint, LaunchTelemetry, read_telemetry_endpoint, retract_telemetry_endpoint,
    TelemetryEndpoint, TELEMETRY_ENDPOINT,
};
#[cfg(unix)]
mod telemetry_store;
#[cfg(unix)]
pub use telemetry_store::{
    valid_token_syntax, AgentTelemetry, Annotation, ANNOTATION_VERDICTS, RecordOutcome, StoredEvaluation, TelemetryCall, TelemetryStore, TelemetryStoreError,
    TelemetryLatency, TelemetrySpan, TelemetryTool, TelemetryToolCall, TelemetryTotal, AlertRow, AlertSync, AlertView, PurgeReport, PurgeState, StoreLimits, TELEMETRY_DB, TELEMETRY_SCHEMA_VERSION, TELEMETRY_TOKEN_HEX,
};
mod agent_store;
pub use agent_store::{
    cancel_queued_agent, execute_agent_stop, list_agents, read_agent, request_agent_stop,
    AgentStoreError,
};
#[cfg(target_os = "linux")]
mod supervisor_identity;
#[cfg(target_os = "linux")]
pub use agent_store::{claim_agent_supervisor, SupervisorClaim};
#[cfg(target_os = "linux")]
pub use supervisor_identity::{
    observe_linux_process, parse_process_start_stamp, LinuxProcessObservation, SupervisorIdentity,
    SupervisorIdentityError,
};
#[cfg(target_os = "linux")]
mod work_view;
#[cfg(target_os = "linux")]
pub use work_view::{encode_work_view, CurrentGateObservation, WorkViewError, WorkViewObservation};
mod public_json;
pub use public_json::pretty_public_json;
mod command_input;
mod control_history;
mod work_projection;
#[cfg(unix)]
pub use command_input::read_command_file;
pub use command_input::{
    check_command_input_size, read_command_document, CommandInputError, CommandInputFailure,
    CommandInputOperation, CommandInputTransport, COMMAND_INPUT_MAX_BYTES,
};
pub use control_history::{read_validation_control_history, ControlHistoryError, EvidenceControl};
#[cfg(feature = "test-fixtures")]
pub use work_projection::project_work_validation as fixture_project_work_validation;
pub use work_projection::{
    recent_work_decisions, wrap_work_text, EvidenceAcceptance, EvidenceControls,
    EvidenceReportReview, TaskEvidence, TaskValidation, WorkProjectionError, WorkValidation,
    WorkViewLanguage, WorkViewLimits,
};
mod agent_workflow;
pub use agent_workflow::{
    assemble_agent_workflow, read_agent_workflow, AgentWorkflowError, AgentWorkflowObservation,
};
mod run_limits;
pub use run_limits::{
    decode_runtime_limit_preview, resolve_run_limits, RuntimeLimitError, RuntimeLimitPolicy,
    RuntimeLimitPreview,
};
mod engine_settings;
mod engine_settings_catalog;
mod gate_contract;
mod gate_evaluation;
mod managed_receipt;
pub use managed_receipt::read_managed_receipt;
mod gate_json;
mod gate_request;
pub use gate_request::{
    decode_gate_request, encode_gate_request_intent, GateRequest, GateRequestError,
};
mod quality_policy;
pub use gate_contract::{
    check_gate_evidence, prepare_gate_record, validate_required_plan_checks, GateContractError,
    GateSubmission,
};
pub use gate_evaluation::{
    evaluate_gate_method2, evaluate_gate_with_policy, gate_document_method, BoundGateEvaluation,
    GateEvaluationError,
};
pub use quality_policy::{QualityPolicySnapshot, QualityPolicyValues};
mod validation_evidence;
pub use validation_evidence::{
    check_validation_evidence, validation_policy_digest, ValidationEvidenceError,
    ValidationEvidenceObservation,
};
mod validation_output;
pub use engine_settings_catalog::{
    EngineSettingApplication, EngineSettingChoice, EngineSettingField, EngineSettingsCatalog,
    ModelPolicyFormat, SettingUnit,
};
pub use validation_output::ValidationOutput;
#[cfg(target_os = "linux")]
mod validation_run;
#[cfg(target_os = "linux")]
pub use validation_run::{
    run_validation_control, ValidationRun, ValidationRunCleanup, ValidationRunError,
};
#[cfg(target_os = "linux")]
mod git_metadata;
#[cfg(target_os = "linux")]
mod git_report;
#[cfg(target_os = "linux")]
pub use git_metadata::{
    observe_git_metadata, GitMetadataDiagnostic, GitMetadataError, GitMetadataField,
    GitMetadataIssue, GitMetadataObservation,
};
#[cfg(target_os = "linux")]
pub use git_report::{hash_git_report, GitCommandOutput, GitReportError, GitReportObservation};
mod environment;
#[cfg(unix)]
mod local_storage;
#[cfg(target_os = "linux")]
mod connected_database_identity;
#[cfg(unix)]
mod root_schema;
#[cfg(unix)]
mod root_store;
pub use engine_settings::{
    EngineSettingsChange, EngineSettingsDocument, EngineSettingsEntry, EngineSettingsError,
    EngineSettingsPreview, EngineSettingsValues, SettingCap, SettingCount,
};
#[cfg(all(unix, feature = "test-fixtures"))]
pub use local_storage::AtomicStorageFault;
#[cfg(unix)]
pub use local_storage::{LocalStorageError, LocalStorageOperation, PublicationState};
#[cfg(unix)]
pub use root_schema::{RootSchemaError, RootSchemaFailure};
#[cfg(target_os = "linux")]
pub use root_store::RootBinding;
#[cfg(unix)]
pub use root_store::QuotaPreview;
#[cfg(target_os = "linux")]
pub use root_store::RootManagedReviewGitObservation;
#[cfg(unix)]
pub use root_store::{
    GateEvidenceObservation, RootManagedReceiptObservation, RootManagedReviewAnchorObservation,
    RootMutationOutcome, RootReviewEvidenceObservation, RootStore, RootStoreError,
    RootValidationEvidenceObservation, RootWorkValidationObservation,
};
mod pricing;
pub use pricing::{
    decode_rate_change, decode_rate_quote, quote_model_rate, validate_model_rate,
    validate_rate_change, ModelRate, PricingError, RateCatalogue, RateChange, RateQuote,
    RateQuoteRequest, RATE_CATALOGUE_MAX_BYTES,
};
mod financial_budget;
mod identity_plan;
mod identity_plan_rules;
mod identity_plan_upgrade;
mod instruction;
mod instruction_learning;
mod memory_files;
mod local_path;
mod model_wire;
mod models;
mod mutation_guard;
pub use financial_budget::{
    check_launch_budget, check_reserved_agent_budget, check_reserved_planning_start_budget,
    decode_budget_change, reserve_planning_call, validate_budget, validate_budget_change, Budget,
    BudgetChange, BudgetError,
};
mod planning_rejection;
pub use planning_rejection::{
    planning_rejection_state, planning_rejection_unchanged, PlanningRejectionError,
};
mod planning_budget;
mod planning_control;
mod planning_report;
pub use planning_budget::{
    check_planning_activation_budget, check_planning_task_budget, planning_scope_ancestors,
    planning_scope_task_count, PlanningBudgetDimension, PlanningBudgetError,
};
pub use planning_report::{
    decode_planning_report_event, read_planning_report, PlanningReportContent, PlanningReportError,
};
mod planning_request;
mod planning_state;
pub use planning_control::{
    apply_planning_control, decode_planning_request, encode_planning_request,
    validate_planning_control_request, validate_planning_intent_size, PlanningControlAction,
    PlanningControlError, PlanningRequestError,
};
pub use planning_request::{ManagedRepositoryRequest, PlanningOperation, PlanningRequest};
mod recorded_change;
mod recorded_command;
mod recorded_event;
mod recorded_work;
mod request;
mod sqlite_value;
mod task_gate_defaults;
pub use task_gate_defaults::{ManualGatePhaseDefaults, MANUAL_GATE_PHASES};
mod request_operation_fields;
mod task_definition;
mod text;
mod validation_policy;
mod work;
pub use request_operation_fields::RequestFieldsError;
mod work_definition;
mod work_engine;
pub use environment::{
    format_utc_timestamp, generate_identifier, go_command_text, identifier_from_entropy,
    observe_git, operator_identity, system_timestamp, GitField, GitIssue, GitIssueCause,
    GitObservation, RuntimeError, RuntimeOperation, SystemEnvironment, WorkEnvironment,
};
pub use identity_plan::{
    identity_plan_digest, identity_plan_digest_for, register_identity_plan_projection,
    IdentityPlanError, IdentityPlanProjection,
};
pub use identity_plan_rules::IDENTITY_PLAN_HEADS_SQL;
pub use identity_plan_upgrade::{
    read_identity_plan_stamp, upgrade_identity_plan_projection, IdentityPlanStamp,
    IdentityPlanUpgradeError,
};
pub use instruction::{composed_task_instructions, update_task_instructions, InstructionError};
pub use local_path::GO_SYMLINK_TRAVERSAL_LIMIT;
pub use local_path::{
    apply_checkpoint, canonical_owned_project_path, canonical_project_root, local_directory,
    local_file, CheckpointError, LocalPathError, PathOperation, PathRefusal,
};
pub use model_wire::{GoFloat, GoRaw};
pub use models::*;
pub use mutation_guard::{
    inspect_recorded_mutation, task_review_contract, ContractRevisionRefusal, MutationGuardError,
    MutationInspection,
};
pub use planning_state::{
    apply_planning_validation_signals, planning_validation_event_id, validate_judge_state,
    validate_planning_state, validate_requirement_prerequisites, PlanningRefusal,
    PlanningStateError,
};
pub use recorded_change::{
    stage_recorded_change, stage_recorded_change_with_hook, RecordedChange, RecordedChangeError,
    RecordedChangeOutcome,
};
pub use recorded_command::{
    reserve_recorded_command, RecordedCommandError, RECORDED_COMMAND_SCHEMA_VERSION,
};
pub use recorded_event::{
    encode_recorded_events, list_recorded_events, RecordedEvent, RecordedEventError,
};
pub use recorded_work::{list_recorded_works, read_recorded_work, RecordedWorkError};
pub use request::{Request, ValidationControl, ValidationPolicy};
use std::fmt;
use swarm_wire_json::{self as wire, projection};
pub use task_definition::{
    apply_task_metadata, update_task_definition, validate_task_dependency_graph, DefinitionContext,
    TaskDefinitionError, TaskDefinitionLimits,
};
pub use validation_policy::{
    normalize_validation_policy, validation_policy_covers_task, ValidationPolicyError,
    ValidationPolicyLimits,
};
pub use work::{decode_event, decode_task, decode_work, decode_work_row, encode_model, WorkError};
pub use work_definition::encode_work_mutation_intent;
pub use work_definition::{
    apply_ooda, apply_task_addition, apply_work_creation, apply_work_definition,
    validate_work_request, DefinitionRefusal, WorkDefinitionError,
};
pub use work_engine::{EngineMutationLimits, WorkEngine, WorkEngineError, WorkMutationOutcome};
pub enum Error {
    Frame(serde_json::Error),
    Projection(projection::Error),
    Version,
}
impl Error {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Version => "version",
            _ => "format",
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Frame(e) => write!(
                f,
                "engine request: format ({:?}, line {}, column {})",
                e.classify(),
                e.line(),
                e.column()
            ),
            Self::Projection(e) => write!(f, "engine request: {e}"),
            Self::Version => write!(f, "engine request: schema_version must equal 1"),
        }
    }
}
impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EngineRequestError")
            .field("code", &self.code())
            .field("message", &self.to_string())
            .finish()
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Frame(e) => Some(e),
            Self::Projection(e) => Some(e),
            _ => None,
        }
    }
}
/// Canonical Go command field shape for outer transport adapters. Reuses the
/// generated producer descriptor, including null/case/duplicate behavior; this
/// data descriptor grants no role, origin, business permission or mutation.
pub const fn request_fields() -> swarm_wire_json::projection::Fields {
    request::FIELDS_REQUEST
}
/// Decode Go's strict command DTO. Schema/business rules belong to consumers.
pub fn decode_request(raw: &[u8]) -> Result<Request, Error> {
    let value = projection::project(raw, request::FIELDS_REQUEST, true).map_err(|e| match e {
        projection::Error::Json(e) => Error::Frame(e),
        e => Error::Projection(e),
    })?;
    serde_json::from_value(value).map_err(Error::Frame)
}
pub fn encode_request(value: &Request) -> Result<Vec<u8>, Error> {
    wire::encode(value).map_err(Error::Frame)
}
/// Clear the untrusted origin label at the external command boundary. This
/// reproduces CLI attribution hygiene; it authenticates no human or role.
pub fn decode_external_request(raw: &[u8]) -> Result<Request, Error> {
    let mut value = decode_request(raw)?;
    if value.schema_version != 1 {
        return Err(Error::Version);
    }
    value.origin.clear();
    Ok(value)
}

mod task_lifecycle;
pub use task_lifecycle::{
    check_manual_task_freshness, transition_manual_task, transition_scoped_worker_task,
    TaskLifecycleContext, TaskLifecycleError,
};

mod context_policy;
pub use context_policy::ContextPolicyLimits;

mod project_context;
pub use project_context::{
    check_project_context_freshness, read_project_context, read_review_delivery,
    recognizable_project_secret, ProjectContextError, ProjectContextObservation,
};

mod review_quote;
pub use review_quote::{review_evidence_present, ReviewQuoteError};
mod review_reply;
pub use review_reply::{
    parse_review_reply, ParsedReviewReply, ReviewQuoteOriginal, ReviewReplyError,
    ReviewReplyLimits, ReviewReplyState,
};
mod quota_reason_policy;
pub use quota_reason_policy::QuotaReasonPolicy;
mod quotas;
pub use quotas::{QuotaChange, QuotaView, decode_quota_change, quota_view, apply_quota_change};
mod review_contract;
pub use review_contract::{managed_review_contract, review_contract};

#[cfg(target_os = "linux")]
mod managed_review_git;
#[cfg(target_os = "linux")]
pub use managed_review_git::{inspect_managed_review_git, ManagedReviewGitObservation};
mod managed_review_files;
pub use managed_review_files::{check_managed_review_files, ManagedReviewFilesObservation};
mod managed_review_context;
mod managed_review_citations;
mod managed_review_reply;
mod managed_review_protocol;
mod managed_review_output;
mod managed_review_record;
pub use managed_review_context::{
    decode_managed_review_context, DeliveryCriterion, ManagedDelivery, ManagedReviewBaseline,
    ManagedReviewContext, ManagedReviewTaskContext,
};
mod managed_review_anchor;
pub use managed_review_anchor::{inspect_managed_review_anchor, ManagedReviewAnchorObservation};
mod review_evidence;
pub use review_evidence::{
    check_current_review_evidence, check_review_evidence, ReviewEvidenceError,
    ReviewEvidenceObservation,
};

#[cfg(unix)]
pub use root_store::RootReviewReplyObservation;

#[cfg(unix)]
pub use root_store::RootControlHistoryObservation;

pub use work::encode_public_model;

#[cfg(target_os = "linux")]
mod web_session;
#[cfg(target_os = "linux")]
pub use web_session::{valid_web_session_credential, WebSessionCredential, WebSessionError};

mod web_policy;
pub use web_policy::{WebTransportConstraints, WebTransportPolicy, WEB_POLICY_FIELDS};

mod planning_action_policy;
#[cfg(target_os = "linux")]
mod planning_boot_clock;
#[cfg(target_os = "linux")]
pub use planning_boot_clock::{PlanningBootInstant, PlanningBootTimer};
#[cfg(target_os = "linux")]
mod native_diagnostics;
#[cfg(target_os = "linux")]
mod native_receipt;
#[cfg(target_os = "linux")]
pub use native_diagnostics::{native_error_links, NativeErrorLinks};
#[cfg(target_os = "linux")]
mod planning_run_material;
#[cfg(target_os = "linux")]
mod planning_run_store;
#[cfg(target_os = "linux")]
mod planning_run_usage;
mod planning_wait_policy;
pub use planning_action_policy::{PlanningActionPolicy, PlanningDecisionPolicy};
pub use planning_wait_policy::{PlanningWaitPolicy, PlanningWaitStop};

mod planning_activation;
#[cfg(target_os = "linux")]
mod planning_call_inputs;
#[cfg(target_os = "linux")]
mod planning_call_preview;
#[cfg(target_os = "linux")]
pub use planning_call_preview::{PlanningCallPreview, PlanningCallPreviewRequest};
mod planning_decision;
mod planning_judge;
mod provider_response;
pub use provider_response::{ProviderResponse,ProviderResponseStream};
mod review_output;
pub use review_output::{ReviewOutputStream, ReviewProviderOutput};
mod planning_output;
pub use planning_output::{PlanningOutputCheck, PlanningOutputStream, PlanningProviderOutput};
mod planning_proposal;
pub use planning_proposal::{decode_planning_proposal, PlanningProposal};
mod planning_scope_context;

#[cfg(target_os = "linux")]
pub use root_store::{execute_planning_gate, PlanningRunOutcome, execute_review_gate, TaskReviewOutcome};

#[cfg(target_os = "linux")]
pub use root_store::PlanningStepOutcome;
#[cfg(target_os = "linux")]
pub use root_store::{execute_validation_gate, TaskValidationPreview, TaskValidationOutcome, TaskValidationCandidate, TaskReviewPreview, TaskReviewInputs, TaskReviewCheck, ReviewInputSource, NativeValidationEvidence, ManagedPublicationPreview, ManagedPublicationOutcome};

#[cfg(target_os = "linux")]
pub use root_store::{AdminBudgetPolicy,AdminBudgetChange,AdminBudgetEntry,AdminBudgetView,AdminBudgetOutcome,AdminBudgetReservationRequest,AdminBudgetReservation};

#[cfg(target_os = "linux")]
mod workspace_turn;

#[cfg(target_os="linux")]
mod workspace_turn_write;

#[cfg(target_os = "linux")]
pub use root_store::MissionReportRelayOutcome;

mod preparation_models;
pub use preparation_models::{Preparation, PreparationDocument, PreparationSource, PreparationVerdict, PreparationConversion};
pub use engine_settings::{PreparationReadPolicy, PreparationWritePolicy, PreparationPlanPolicy};
pub use root_store::{PreparationReadError, PreparationReadObservation};
pub use preparation_models::PreparationMethod;
pub use root_store::PreparationList;

pub use root_store::{HistoricalBudgetView, HistoricalBudgetShow, HistoricalBudgetPreview};

mod preparation_command;
pub use preparation_command::PreparationCommand;
/// Decode the supported draft command without accessing storage or adopting a
/// plan. Unknown fields/actions remain refused by the writer; codec causes are
/// retained under a safe preparation boundary. Caller owes policy and identity.
pub fn decode_preparation_command(raw:&[u8])->Result<PreparationCommand,RootStoreError>{
    preparation_command::decode(raw).map_err(|e|root_store::preparation_read::cause("preparation-request-invalid",e))
}

/// Validate a preparation/event protocol key (1..=128 ASCII alphanumeric, dash
/// or underscore). Pure syntax; it grants no identity, ownership or authority.
pub fn valid_preparation_key(value:&str)->bool{preparation_command::key(value)}

#[cfg(target_os = "linux")]
mod managed_publication_record;
