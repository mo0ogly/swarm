//! Generated Work/Task/Event data graph; no execution authority.
use super::model_wire::{Fields, GoFloat, GoRaw, Kind};
use super::{ValidationControl, ValidationPolicy};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
fn is_zero(v: &i64) -> bool {
    *v == 0
}
fn is_false(v: &bool) -> bool {
    !*v
}
fn empty_vec<T>(v: &Option<Vec<T>>) -> bool {
    v.as_ref().is_none_or(Vec::is_empty)
}
fn empty_map<K, V>(v: &Option<BTreeMap<K, V>>) -> bool {
    v.as_ref().is_none_or(BTreeMap::is_empty)
}
// internal/engine/plan_contract.go SHA-256 1dc52ac99dee53d507d7ee00f994ecd6f7e3e820b41ac1dc749c12f09b44b0a8
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ActionPlan {
    // Product line 32
    #[serde(rename = "product")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product: Option<Box<ProductPlan>>,
    // Version line 33
    #[serde(rename = "version")]
    pub version: i64,
    // Objective line 34
    #[serde(rename = "objective")]
    pub objective: String,
    // Assumptions line 35
    #[serde(rename = "assumptions")]
    pub assumptions: Option<Vec<String>>,
    // Questions line 36
    #[serde(rename = "questions")]
    pub questions: Option<Vec<PlanQuestion>>,
    // Tasks line 37
    #[serde(rename = "tasks")]
    pub tasks: Option<Vec<PlanMission>>,
}
pub(crate) fn shape_action_plan() -> Fields {
    &[
        ("product", Kind::Pointer(&Kind::Object(shape_product_plan))),
        ("version", Kind::Int64),
        ("objective", Kind::String),
        ("assumptions", Kind::Array(&Kind::String)),
        ("questions", Kind::Array(&Kind::Object(shape_plan_question))),
        ("tasks", Kind::Array(&Kind::Object(shape_plan_mission))),
    ]
}
// internal/engine/action_skills.go SHA-256 55a787ab8b74049c648c32c414914bcf1dbc2bd2737e1e35629854d51127bb6f
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ActionSkill {
    // Path line 18
    #[serde(rename = "path")]
    pub path: String,
    // SHA256 line 19
    #[serde(rename = "sha256")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub sha256: String,
    // Name line 20
    #[serde(rename = "name")]
    pub name: String,
    // Description line 21
    #[serde(rename = "description")]
    pub description: String,
    // Bytes line 22
    #[serde(rename = "bytes")]
    pub bytes: i64,
    // Available line 23
    #[serde(rename = "available")]
    pub available: bool,
    // Reason line 24
    #[serde(rename = "reason")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub reason: String,
}
pub(crate) fn shape_action_skill() -> Fields {
    &[
        ("path", Kind::String),
        ("sha256", Kind::String),
        ("name", Kind::String),
        ("description", Kind::String),
        ("bytes", Kind::Int64),
        ("available", Kind::Bool),
        ("reason", Kind::String),
    ]
}
// internal/engine/action_skills.go SHA-256 55a787ab8b74049c648c32c414914bcf1dbc2bd2737e1e35629854d51127bb6f
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ActionSkillSelection {
    // Path line 14
    #[serde(rename = "path")]
    pub path: String,
    // SHA256 line 15
    #[serde(rename = "sha256")]
    pub sha256: String,
}
pub(crate) fn shape_action_skill_selection() -> Fields {
    &[("path", Kind::String), ("sha256", Kind::String)]
}
// internal/engine/agents_store.go SHA-256 29a04b2234fa0190a7c12ba318c26612e8cbcabc2025010ab03745e19e02039d
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentProgress {
    // ElapsedSeconds line 80
    #[serde(rename = "elapsed_seconds")]
    #[serde(skip_serializing_if = "is_zero")]
    pub elapsed_seconds: i64,
    // TimeThresholdExceeded line 81
    #[serde(rename = "time_threshold_exceeded")]
    #[serde(skip_serializing_if = "is_false")]
    pub time_threshold_exceeded: bool,
    // ToolWaitSeconds line 82
    #[serde(rename = "tool_wait_seconds")]
    #[serde(skip_serializing_if = "is_zero")]
    pub tool_wait_seconds: i64,
    // ToolTimeThresholdExceeded line 83
    #[serde(rename = "tool_time_threshold_exceeded")]
    #[serde(skip_serializing_if = "is_false")]
    pub tool_time_threshold_exceeded: bool,
    // Action line 84
    #[serde(rename = "action")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub action: String,
    // Detail line 85
    #[serde(rename = "detail")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub detail: String,
    // ToolCalls line 86
    #[serde(rename = "tool_calls")]
    pub tool_calls: i64,
    // ToolResults line 87
    #[serde(rename = "tool_results")]
    pub tool_results: i64,
    // PendingTools line 88
    #[serde(rename = "pending_tools")]
    pub pending_tools: i64,
    // LastTool line 89
    #[serde(rename = "last_tool")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub last_tool: String,
    // LastResult line 90
    #[serde(rename = "last_result_at")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub last_result_at: String,
    // LastOutput line 91
    #[serde(rename = "last_output_at")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub last_output_at: String,
    // OutputState line 92
    #[serde(rename = "output_state")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub output_state: String,
    // ToolState line 93
    #[serde(rename = "tool_state")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub tool_state: String,
    // Vitality line 94
    #[serde(rename = "provider_vitality")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub provider_vitality: String,
    // Degraded line 95
    #[serde(rename = "degraded")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub degraded: String,
    // MetricsVersion line 98
    #[serde(rename = "metrics_version")]
    #[serde(skip_serializing_if = "is_zero")]
    pub metrics_version: i64,
    // Reads line 99
    #[serde(rename = "tool_reads")]
    #[serde(skip_serializing_if = "is_zero")]
    pub tool_reads: i64,
    // Writes line 100
    #[serde(rename = "tool_writes")]
    #[serde(skip_serializing_if = "is_zero")]
    pub tool_writes: i64,
    // Unclassified line 101
    #[serde(rename = "tool_unclassified")]
    #[serde(skip_serializing_if = "is_zero")]
    pub tool_unclassified: i64,
    // Errors line 102
    #[serde(rename = "tool_errors_total")]
    #[serde(skip_serializing_if = "is_zero")]
    pub tool_errors_total: i64,
    // Repeats line 103
    #[serde(rename = "tool_repeats_total")]
    #[serde(skip_serializing_if = "is_zero")]
    pub tool_repeats_total: i64,
}
pub(crate) fn shape_agent_progress() -> Fields {
    &[
        ("elapsed_seconds", Kind::Int64),
        ("time_threshold_exceeded", Kind::Bool),
        ("tool_wait_seconds", Kind::Int64),
        ("tool_time_threshold_exceeded", Kind::Bool),
        ("action", Kind::String),
        ("detail", Kind::String),
        ("tool_calls", Kind::Int64),
        ("tool_results", Kind::Int64),
        ("pending_tools", Kind::Int64),
        ("last_tool", Kind::String),
        ("last_result_at", Kind::String),
        ("last_output_at", Kind::String),
        ("output_state", Kind::String),
        ("tool_state", Kind::String),
        ("provider_vitality", Kind::String),
        ("degraded", Kind::String),
        ("metrics_version", Kind::Int64),
        ("tool_reads", Kind::Int64),
        ("tool_writes", Kind::Int64),
        ("tool_unclassified", Kind::Int64),
        ("tool_errors_total", Kind::Int64),
        ("tool_repeats_total", Kind::Int64),
    ]
}
// internal/engine/agent_workflow.go SHA-256 d623574c657a9a8152df596999655d7e40206986332eedb6cedbd2f7e238efc8
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentWorkflow {
    // Skills line 15
    #[serde(rename = "skills")]
    #[serde(skip_serializing_if = "empty_vec")]
    pub skills: Option<Vec<ActionSkill>>,
    // Project line 16
    #[serde(rename = "project")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<Box<ProjectContext>>,
    // Version line 17
    #[serde(rename = "version")]
    pub version: i64,
    // Role line 18
    #[serde(rename = "role")]
    pub role: String,
    // Methods line 19
    #[serde(rename = "methods")]
    pub methods: Option<Vec<String>>,
    // SHA256 line 20
    #[serde(rename = "sha256")]
    pub sha256: String,
}
pub(crate) fn shape_agent_workflow() -> Fields {
    &[
        ("skills", Kind::Array(&Kind::Object(shape_action_skill))),
        (
            "project",
            Kind::Pointer(&Kind::Object(shape_project_context)),
        ),
        ("version", Kind::Int64),
        ("role", Kind::String),
        ("methods", Kind::Array(&Kind::String)),
        ("sha256", Kind::String),
    ]
}
// internal/engine/plan_contract.go SHA-256 1dc52ac99dee53d507d7ee00f994ecd6f7e3e820b41ac1dc749c12f09b44b0a8
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ApprovedPlan {
    // TaskMap line 40
    #[serde(rename = "task_map")]
    #[serde(skip_serializing_if = "empty_map")]
    pub task_map: Option<BTreeMap<String, String>>,
    // Source line 41
    #[serde(rename = "source")]
    pub source: String,
    // BriefHash line 42
    #[serde(rename = "brief_hash")]
    pub brief_hash: String,
    // ResponseHash line 43
    #[serde(rename = "response_hash")]
    pub response_hash: String,
    // Spec line 44
    #[serde(rename = "spec")]
    pub spec: ActionPlan,
    // TaskIDs line 45
    #[serde(rename = "task_ids")]
    pub task_ids: Option<Vec<String>>,
    // At line 46
    #[serde(rename = "at")]
    pub at: String,
}
pub(crate) fn shape_approved_plan() -> Fields {
    &[
        ("task_map", Kind::Map(&Kind::String)),
        ("source", Kind::String),
        ("brief_hash", Kind::String),
        ("response_hash", Kind::String),
        ("spec", Kind::Object(shape_action_plan)),
        ("task_ids", Kind::Array(&Kind::String)),
        ("at", Kind::String),
    ]
}
// internal/engine/model.go SHA-256 9b8f2f15d6af6c046cc3b7c909a4a8bf3155b9dd1e6c4ce04612758a763c9086
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Attempt {
    // ID line 196
    #[serde(rename = "id")]
    pub id: String,
    // Status line 197
    #[serde(rename = "status")]
    pub status: String,
    // Started line 198
    #[serde(rename = "started")]
    pub started: String,
    // Ended line 199
    #[serde(rename = "ended")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub ended: String,
}
pub(crate) fn shape_attempt() -> Fields {
    &[
        ("id", Kind::String),
        ("status", Kind::String),
        ("started", Kind::String),
        ("ended", Kind::String),
    ]
}
// internal/engine/agents_store.go SHA-256 29a04b2234fa0190a7c12ba318c26612e8cbcabc2025010ab03745e19e02039d
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AttemptDiagnostic {
    // AgentID line 106
    #[serde(rename = "agent_id")]
    pub agent_id: String,
    // AttemptID line 107
    #[serde(rename = "attempt_id")]
    pub attempt_id: String,
    // ObservedErrors line 108
    #[serde(rename = "observed_errors")]
    pub observed_errors: i64,
    // ConsecutiveErrorLimit line 109
    #[serde(rename = "consecutive_error_limit")]
    pub consecutive_error_limit: i64,
    // LimitReached line 110
    #[serde(rename = "limit_reached")]
    pub limit_reached: bool,
    // ConsecutiveLimitReached line 111
    #[serde(rename = "consecutive_error_limit_reached")]
    pub consecutive_error_limit_reached: bool,
    // LimitReason line 112
    #[serde(rename = "limit_reason")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub limit_reason: String,
    // Summary line 113
    #[serde(rename = "summary")]
    pub summary: String,
    // Items line 114
    #[serde(rename = "items")]
    pub items: Option<Vec<DiagnosticItem>>,
}
pub(crate) fn shape_attempt_diagnostic() -> Fields {
    &[
        ("agent_id", Kind::String),
        ("attempt_id", Kind::String),
        ("observed_errors", Kind::Int64),
        ("consecutive_error_limit", Kind::Int64),
        ("limit_reached", Kind::Bool),
        ("consecutive_error_limit_reached", Kind::Bool),
        ("limit_reason", Kind::String),
        ("summary", Kind::String),
        ("items", Kind::Array(&Kind::Object(shape_diagnostic_item))),
    ]
}
// internal/engine/model.go SHA-256 9b8f2f15d6af6c046cc3b7c909a4a8bf3155b9dd1e6c4ce04612758a763c9086
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AutomaticValidation {
    // DeliveryVersion line 81
    #[serde(rename = "delivery_version")]
    #[serde(skip_serializing_if = "is_zero")]
    pub delivery_version: i64,
    // Attempt line 82
    #[serde(rename = "attempt_id")]
    pub attempt_id: String,
    // Revision line 83
    #[serde(rename = "revision")]
    pub revision: i64,
    // CandidateSHA line 88
    #[serde(rename = "candidate_sha")]
    pub candidate_sha: String,
    // Producer line 89
    #[serde(rename = "producer_agent_id")]
    pub producer_agent_id: String,
    // Controller line 90
    #[serde(rename = "controller")]
    pub controller: String,
    // PolicyDigest line 91
    #[serde(rename = "policy_digest")]
    pub policy_digest: String,
    // Policy line 92
    #[serde(rename = "policy")]
    pub policy: ValidationPolicy,
    // Artifacts line 93
    #[serde(rename = "artifacts")]
    pub artifacts: Option<BTreeMap<String, String>>,
    // Controls line 94
    #[serde(rename = "controls")]
    pub controls: Option<Vec<ValidationControlResult>>,
    // Receipt line 95
    #[serde(rename = "receipt")]
    pub receipt: String,
    // State line 96
    #[serde(rename = "state")]
    pub state: String,
    // Reason line 97
    #[serde(rename = "reason")]
    pub reason: String,
    // At line 98
    #[serde(rename = "at")]
    pub at: String,
}
pub(crate) fn shape_automatic_validation() -> Fields {
    &[
        ("delivery_version", Kind::Int64),
        ("attempt_id", Kind::String),
        ("revision", Kind::Int64),
        ("candidate_sha", Kind::String),
        ("producer_agent_id", Kind::String),
        ("controller", Kind::String),
        ("policy_digest", Kind::String),
        ("policy", Kind::Object(shape_validation_policy)),
        ("artifacts", Kind::Map(&Kind::String)),
        (
            "controls",
            Kind::Array(&Kind::Object(shape_validation_control_result)),
        ),
        ("receipt", Kind::String),
        ("state", Kind::String),
        ("reason", Kind::String),
        ("at", Kind::String),
    ]
}
// internal/engine/model.go SHA-256 9b8f2f15d6af6c046cc3b7c909a4a8bf3155b9dd1e6c4ce04612758a763c9086
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BrainstormAnswer {
    // Attempt line 27
    #[serde(rename = "attempt")]
    pub attempt: String,
    // Text line 28
    #[serde(rename = "text")]
    pub text: String,
    // At line 29
    #[serde(rename = "at")]
    pub at: String,
}
pub(crate) fn shape_brainstorm_answer() -> Fields {
    &[
        ("attempt", Kind::String),
        ("text", Kind::String),
        ("at", Kind::String),
    ]
}
// internal/engine/dialogue_context.go SHA-256 510d596e9f20ee3e27c3791b3ebf1ef2c96cf2d3eb348907b5e76725f99bf079
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ContextManifest {
    // Summary line 27
    #[serde(rename = "summary")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<Box<MemorySummaryReceipt>>,
    // Memory line 28
    #[serde(rename = "memory")]
    #[serde(skip_serializing_if = "empty_vec")]
    pub memory: Option<Vec<MemoryReference>>,
    // MemoryDecision line 29
    #[serde(rename = "memory_decision")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_decision: Option<Box<MemorySelectionDecision>>,
    // SHA256 line 30
    #[serde(rename = "sha256")]
    pub sha256: String,
    // Bytes line 31
    #[serde(rename = "bytes")]
    pub bytes: i64,
    // Included line 32
    #[serde(rename = "included")]
    pub included: Option<Vec<String>>,
    // Excluded line 33
    #[serde(rename = "excluded")]
    pub excluded: Option<Vec<String>>,
    // References line 34
    #[serde(rename = "references")]
    pub references: Option<Vec<DialogueRef>>,
    // Paths line 35
    #[serde(rename = "paths")]
    pub paths: Option<Vec<String>>,
}
pub(crate) fn shape_context_manifest() -> Fields {
    &[
        (
            "summary",
            Kind::Pointer(&Kind::Object(shape_memory_summary_receipt)),
        ),
        ("memory", Kind::Array(&Kind::Object(shape_memory_reference))),
        (
            "memory_decision",
            Kind::Pointer(&Kind::Object(shape_memory_selection_decision)),
        ),
        ("sha256", Kind::String),
        ("bytes", Kind::Int64),
        ("included", Kind::Array(&Kind::String)),
        ("excluded", Kind::Array(&Kind::String)),
        ("references", Kind::Array(&Kind::Object(shape_dialogue_ref))),
        ("paths", Kind::Array(&Kind::String)),
    ]
}
// internal/engine/corrective_recovery.go SHA-256 032a71d86ebb2d1fc40ef4b3cbdaa68e01feb6362d8de44368578cbb20049696
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CorrectiveRecovery {
    // MissingReport line 13
    #[serde(rename = "missing_report")]
    #[serde(skip_serializing_if = "is_false")]
    pub missing_report: bool,
    // Result line 14
    #[serde(rename = "result_commit")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub result_commit: String,
    // Event line 15
    #[serde(rename = "event_id")]
    pub event_id: String,
    // Revision line 16
    #[serde(rename = "authorized_revision")]
    pub authorized_revision: i64,
    // Review line 17
    #[serde(rename = "review_id")]
    pub review_id: String,
    // Attempt line 18
    #[serde(rename = "attempt_id")]
    pub attempt_id: String,
    // Actor line 19
    #[serde(rename = "actor")]
    pub actor: String,
    // At line 20
    #[serde(rename = "at")]
    pub at: String,
    // Reason line 21
    #[serde(rename = "reason")]
    pub reason: String,
    // Instruction line 22
    #[serde(rename = "instruction")]
    pub instruction: String,
}
pub(crate) fn shape_corrective_recovery() -> Fields {
    &[
        ("missing_report", Kind::Bool),
        ("result_commit", Kind::String),
        ("event_id", Kind::String),
        ("authorized_revision", Kind::Int64),
        ("review_id", Kind::String),
        ("attempt_id", Kind::String),
        ("actor", Kind::String),
        ("at", Kind::String),
        ("reason", Kind::String),
        ("instruction", Kind::String),
    ]
}
// internal/engine/agents_store.go SHA-256 29a04b2234fa0190a7c12ba318c26612e8cbcabc2025010ab03745e19e02039d
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DiagnosticItem {
    // Category line 117
    #[serde(rename = "category")]
    pub category: String,
    // Label line 118
    #[serde(rename = "label")]
    pub label: String,
    // Count line 119
    #[serde(rename = "count")]
    pub count: i64,
    // Cause line 120
    #[serde(rename = "cause")]
    pub cause: String,
    // Consequence line 121
    #[serde(rename = "consequence")]
    pub consequence: String,
    // Action line 122
    #[serde(rename = "action")]
    pub action: String,
    // ActionKind line 123
    #[serde(rename = "action_kind")]
    pub action_kind: String,
    // Unknown line 124
    #[serde(rename = "unknown")]
    #[serde(skip_serializing_if = "is_false")]
    pub unknown: bool,
    // Traces line 125
    #[serde(rename = "traces")]
    pub traces: Option<Vec<String>>,
}
pub(crate) fn shape_diagnostic_item() -> Fields {
    &[
        ("category", Kind::String),
        ("label", Kind::String),
        ("count", Kind::Int64),
        ("cause", Kind::String),
        ("consequence", Kind::String),
        ("action", Kind::String),
        ("action_kind", Kind::String),
        ("unknown", Kind::Bool),
        ("traces", Kind::Array(&Kind::String)),
    ]
}
// internal/engine/dialogue_context.go SHA-256 510d596e9f20ee3e27c3791b3ebf1ef2c96cf2d3eb348907b5e76725f99bf079
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DialogueRef {
    // Task line 9
    #[serde(rename = "task")]
    pub task: String,
    // Attempt line 10
    #[serde(rename = "attempt")]
    pub attempt: String,
    // SHA256 line 11
    #[serde(rename = "sha256")]
    pub sha256: String,
}
pub(crate) fn shape_dialogue_ref() -> Fields {
    &[
        ("task", Kind::String),
        ("attempt", Kind::String),
        ("sha256", Kind::String),
    ]
}
// internal/engine/score.go SHA-256 449fd157d53cc656f103984a997ec8d11297c660df5aaf6f32f1a3e94f1df62d
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Evaluation {
    // Method line 18
    #[serde(rename = "method_version")]
    pub method_version: String,
    // Scope line 19
    #[serde(rename = "scope_id")]
    pub scope_id: String,
    // Phase line 20
    #[serde(rename = "phase")]
    pub phase: String,
    // ConfigDigest line 21
    #[serde(rename = "config_digest")]
    pub config_digest: String,
    // ApplicabilityDigest line 22
    #[serde(rename = "applicability_digest")]
    pub applicability_digest: String,
    // Artifacts line 23
    #[serde(rename = "artifacts")]
    pub artifacts: Option<BTreeMap<String, String>>,
    // Scores line 24
    #[serde(rename = "domain_scores")]
    pub domain_scores: Option<BTreeMap<String, GoFloat>>,
    // Quality line 25
    #[serde(rename = "quality")]
    pub quality: Option<Box<GoFloat>>,
    // Provisional line 26
    #[serde(rename = "provisional")]
    pub provisional: bool,
    // Band line 27
    #[serde(rename = "quality_band")]
    pub quality_band: String,
    // Progress line 28
    #[serde(rename = "progress")]
    pub progress: Progress,
    // Severity line 29
    #[serde(rename = "max_severity")]
    pub max_severity: String,
    // Blockers line 30
    #[serde(rename = "blockers")]
    pub blockers: Option<Vec<String>>,
    // Allowed line 31
    #[serde(rename = "allowed")]
    pub allowed: bool,
    // Ship line 32
    #[serde(rename = "ship_allowed")]
    pub ship_allowed: bool,
}
pub(crate) fn shape_evaluation() -> Fields {
    &[
        ("method_version", Kind::String),
        ("scope_id", Kind::String),
        ("phase", Kind::String),
        ("config_digest", Kind::String),
        ("applicability_digest", Kind::String),
        ("artifacts", Kind::Map(&Kind::String)),
        ("domain_scores", Kind::Map(&Kind::Float64)),
        ("quality", Kind::Pointer(&Kind::Float64)),
        ("provisional", Kind::Bool),
        ("quality_band", Kind::String),
        ("progress", Kind::Object(shape_progress)),
        ("max_severity", Kind::String),
        ("blockers", Kind::Array(&Kind::String)),
        ("allowed", Kind::Bool),
        ("ship_allowed", Kind::Bool),
    ]
}
// internal/engine/model.go SHA-256 9b8f2f15d6af6c046cc3b7c909a4a8bf3155b9dd1e6c4ce04612758a763c9086
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Event {
    // ID line 230
    #[serde(rename = "id")]
    pub id: String,
    // WorkID line 231
    #[serde(rename = "work_id")]
    pub work_id: String,
    // Revision line 232
    #[serde(rename = "revision")]
    pub revision: i64,
    // Kind line 233
    #[serde(rename = "kind")]
    pub kind: String,
    // At line 234
    #[serde(rename = "at")]
    pub at: String,
    // Payload line 235
    #[serde(rename = "payload")]
    pub payload: GoRaw,
}
pub(crate) fn shape_event() -> Fields {
    &[
        ("id", Kind::String),
        ("work_id", Kind::String),
        ("revision", Kind::Int64),
        ("kind", Kind::String),
        ("at", Kind::String),
        ("payload", Kind::Raw),
    ]
}
// internal/engine/agent_exchange.go SHA-256 73e0991f3f455f6d7cd6c5016799c26940abb60d69d6bd2f45691c1405f3fa92
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExchangeArtifact {
    // Path line 60
    #[serde(rename = "path")]
    pub path: String,
    // SHA256 line 61
    #[serde(rename = "sha256")]
    pub sha256: String,
}
pub(crate) fn shape_exchange_artifact() -> Fields {
    &[("path", Kind::String), ("sha256", Kind::String)]
}
// internal/engine/score.go SHA-256 449fd157d53cc656f103984a997ec8d11297c660df5aaf6f32f1a3e94f1df62d
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GateRecord {
    // Name line 40
    #[serde(rename = "name")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    // Document line 41
    #[serde(rename = "document")]
    pub document: GoRaw,
    // Evaluation line 42
    #[serde(rename = "evaluation")]
    pub evaluation: Evaluation,
    // At line 43
    #[serde(rename = "at")]
    pub at: String,
}
pub(crate) fn shape_gate_record() -> Fields {
    &[
        ("name", Kind::String),
        ("document", Kind::Raw),
        ("evaluation", Kind::Object(shape_evaluation)),
        ("at", Kind::String),
    ]
}
// internal/engine/model.go SHA-256 9b8f2f15d6af6c046cc3b7c909a4a8bf3155b9dd1e6c4ce04612758a763c9086
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GitState {
    // Head line 202
    #[serde(rename = "head")]
    pub head: String,
    // Branch line 203
    #[serde(rename = "branch")]
    pub branch: String,
    // Changes line 204
    #[serde(rename = "changes")]
    pub changes: String,
}
pub(crate) fn shape_git_state() -> Fields {
    &[
        ("head", Kind::String),
        ("branch", Kind::String),
        ("changes", Kind::String),
    ]
}
// internal/engine/historical_requalification.go SHA-256 28df882dcda883630a4ad6e5d5c8ae938b681e8e3a7b48aa70aa576cdcf0d04e
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HistoricalRequalification {
    // Event line 15
    #[serde(rename = "event_id")]
    pub event_id: String,
    // Digest line 16
    #[serde(rename = "request_sha256")]
    pub request_sha256: String,
    // ProofKey line 17
    #[serde(rename = "proof_key")]
    pub proof_key: String,
    // Agent line 18
    #[serde(rename = "agent_id")]
    pub agent_id: String,
    // Attempt line 19
    #[serde(rename = "attempt_id")]
    pub attempt_id: String,
    // Result line 20
    #[serde(rename = "result_commit")]
    pub result_commit: String,
    // Candidate line 21
    #[serde(rename = "previous_candidate")]
    pub previous_candidate: String,
    // Contract line 22
    #[serde(rename = "contract_sha256")]
    pub contract_sha256: String,
    // Status line 23
    #[serde(rename = "previous_status")]
    pub previous_status: String,
    // Gate line 24
    #[serde(rename = "previous_gate")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_gate: Option<Box<GateRecord>>,
    // Validation line 25
    #[serde(rename = "previous_validation")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_validation: Option<Box<AutomaticValidation>>,
    // Review line 26
    #[serde(rename = "previous_review")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_review: Option<Box<IndependentReview>>,
    // Reason line 27
    #[serde(rename = "reason")]
    pub reason: String,
    // At line 28
    #[serde(rename = "at")]
    pub at: String,
}
pub(crate) fn shape_historical_requalification() -> Fields {
    &[
        ("event_id", Kind::String),
        ("request_sha256", Kind::String),
        ("proof_key", Kind::String),
        ("agent_id", Kind::String),
        ("attempt_id", Kind::String),
        ("result_commit", Kind::String),
        ("previous_candidate", Kind::String),
        ("contract_sha256", Kind::String),
        ("previous_status", Kind::String),
        (
            "previous_gate",
            Kind::Pointer(&Kind::Object(shape_gate_record)),
        ),
        (
            "previous_validation",
            Kind::Pointer(&Kind::Object(shape_automatic_validation)),
        ),
        (
            "previous_review",
            Kind::Pointer(&Kind::Object(shape_independent_review)),
        ),
        ("reason", Kind::String),
        ("at", Kind::String),
    ]
}
// internal/engine/independent_review.go SHA-256 8aaa695b6bcc95e61684c0cbc099dd34be73cb18374a14c3a7ea8b2d604f20f5
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct IndependentReview {
    // ResultEvent line 31
    #[serde(rename = "result_event")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub result_event: String,
    // FailureCode line 32
    #[serde(rename = "failure_code")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub failure_code: String,
    // ReportArtifacts line 33
    #[serde(rename = "report_artifacts")]
    #[serde(skip_serializing_if = "empty_map")]
    pub report_artifacts: Option<BTreeMap<String, String>>,
    // FragmentJournal line 34
    #[serde(rename = "fragment_journal")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fragment_journal: Option<Box<ManagedFragmentJournalAnchor>>,
    // ModelRoute line 35
    #[serde(rename = "model_route")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_route: Option<Box<ModelRoute>>,
    // BatchPlanDigest line 36
    #[serde(rename = "batch_plan_sha256")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub batch_plan_sha256: String,
    // BatchProviderDigest line 37
    #[serde(rename = "batch_provider_sha256")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub batch_provider_sha256: String,
    // Batches line 38
    #[serde(rename = "batches")]
    #[serde(skip_serializing_if = "empty_vec")]
    pub batches: Option<Vec<ManagedReviewBatchVerdict>>,
    // ReplyPath line 39
    #[serde(rename = "reply_path")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub reply_path: String,
    // ReplyDigest line 40
    #[serde(rename = "reply_sha256")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub reply_sha256: String,
    // TimeoutSeconds line 41
    #[serde(rename = "timeout_seconds")]
    #[serde(skip_serializing_if = "is_zero")]
    pub timeout_seconds: i64,
    // Workflow line 42
    #[serde(rename = "workflow")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow: Option<Box<AgentWorkflow>>,
    // GitReport line 43
    #[serde(rename = "git_report")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub git_report: String,
    // CandidateSHA line 44
    #[serde(rename = "candidate_commit")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub candidate_commit: String,
    // PreviousCandidate line 45
    #[serde(rename = "previous_candidate")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub previous_candidate: String,
    // Receipt line 46
    #[serde(rename = "receipt")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub receipt: String,
    // ReceiptDigest line 47
    #[serde(rename = "receipt_sha256")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub receipt_sha256: String,
    // Context line 48
    #[serde(rename = "context")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub context: String,
    // ContextDigest line 49
    #[serde(rename = "context_sha256")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub context_sha256: String,
    // ManagedTasks line 50
    #[serde(rename = "managed_tasks")]
    #[serde(skip_serializing_if = "empty_vec")]
    pub managed_tasks: Option<Vec<ManagedTaskReview>>,
    // ID line 51
    #[serde(rename = "id")]
    pub id: String,
    // Attempt line 52
    #[serde(rename = "attempt")]
    pub attempt: String,
    // Producer line 53
    #[serde(rename = "producer")]
    pub producer: String,
    // Reviewer line 54
    #[serde(rename = "reviewer")]
    pub reviewer: String,
    // Report line 55
    #[serde(rename = "report")]
    pub report: String,
    // Digest line 56
    #[serde(rename = "sha256")]
    pub sha256: String,
    // Contract line 57
    #[serde(rename = "contract")]
    pub contract: String,
    // State line 58
    #[serde(rename = "state")]
    pub state: String,
    // Reason line 59
    #[serde(rename = "reason")]
    pub reason: String,
    // Criteria line 60
    #[serde(rename = "criteria")]
    #[serde(skip_serializing_if = "empty_vec")]
    pub criteria: Option<Vec<ReviewCriterion>>,
    // Started line 61
    #[serde(rename = "started")]
    pub started: String,
    // Finished line 62
    #[serde(rename = "finished")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub finished: String,
    // Usage line 63
    #[serde(rename = "usage")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Box<Usage>>,
}
pub(crate) fn shape_independent_review() -> Fields {
    &[
        ("result_event", Kind::String),
        ("failure_code", Kind::String),
        ("report_artifacts", Kind::Map(&Kind::String)),
        (
            "fragment_journal",
            Kind::Pointer(&Kind::Object(shape_managed_fragment_journal_anchor)),
        ),
        (
            "model_route",
            Kind::Pointer(&Kind::Object(shape_model_route)),
        ),
        ("batch_plan_sha256", Kind::String),
        ("batch_provider_sha256", Kind::String),
        (
            "batches",
            Kind::Array(&Kind::Object(shape_managed_review_batch_verdict)),
        ),
        ("reply_path", Kind::String),
        ("reply_sha256", Kind::String),
        ("timeout_seconds", Kind::Int64),
        (
            "workflow",
            Kind::Pointer(&Kind::Object(shape_agent_workflow)),
        ),
        ("git_report", Kind::String),
        ("candidate_commit", Kind::String),
        ("previous_candidate", Kind::String),
        ("receipt", Kind::String),
        ("receipt_sha256", Kind::String),
        ("context", Kind::String),
        ("context_sha256", Kind::String),
        (
            "managed_tasks",
            Kind::Array(&Kind::Object(shape_managed_task_review)),
        ),
        ("id", Kind::String),
        ("attempt", Kind::String),
        ("producer", Kind::String),
        ("reviewer", Kind::String),
        ("report", Kind::String),
        ("sha256", Kind::String),
        ("contract", Kind::String),
        ("state", Kind::String),
        ("reason", Kind::String),
        (
            "criteria",
            Kind::Array(&Kind::Object(shape_review_criterion)),
        ),
        ("started", Kind::String),
        ("finished", Kind::String),
        ("usage", Kind::Pointer(&Kind::Object(shape_usage))),
    ]
}
// internal/engine/instruction_revision.go SHA-256 8fb06af1125ed0785f282e77be4607e81e4a76e32f1a1ee36a4863cb6efc8615
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct InstructionRevision {
    // ID line 16
    #[serde(rename = "id")]
    pub id: String,
    // Addition line 17
    #[serde(rename = "addition")]
    pub addition: String,
    // Reason line 18
    #[serde(rename = "reason")]
    pub reason: String,
    // Scope line 19
    #[serde(rename = "scope")]
    pub scope: String,
    // Holder line 20
    #[serde(rename = "holder")]
    pub holder: String,
    // Generation line 21
    #[serde(rename = "generation")]
    pub generation: i64,
    // Inputs line 22
    #[serde(rename = "input_events")]
    pub input_events: Option<Vec<String>>,
    // Decision line 23
    #[serde(rename = "decision")]
    pub decision: String,
    // At line 24
    #[serde(rename = "at")]
    pub at: String,
    // State line 25
    #[serde(rename = "state")]
    pub state: String,
    // AttemptsAtCreation line 26
    #[serde(rename = "attempts_at_creation")]
    pub attempts_at_creation: i64,
    // BeforeSHA line 27
    #[serde(rename = "before_sha256")]
    pub before_sha256: String,
    // AfterSHA line 28
    #[serde(rename = "after_sha256")]
    pub after_sha256: String,
    // UsedByAttempt line 29
    #[serde(rename = "used_by_attempt")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub used_by_attempt: String,
    // RejectedAt line 30
    #[serde(rename = "rejected_at")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub rejected_at: String,
    // RejectedReason line 31
    #[serde(rename = "rejected_reason")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub rejected_reason: String,
}
pub(crate) fn shape_instruction_revision() -> Fields {
    &[
        ("id", Kind::String),
        ("addition", Kind::String),
        ("reason", Kind::String),
        ("scope", Kind::String),
        ("holder", Kind::String),
        ("generation", Kind::Int64),
        ("input_events", Kind::Array(&Kind::String)),
        ("decision", Kind::String),
        ("at", Kind::String),
        ("state", Kind::String),
        ("attempts_at_creation", Kind::Int64),
        ("before_sha256", Kind::String),
        ("after_sha256", Kind::String),
        ("used_by_attempt", Kind::String),
        ("rejected_at", Kind::String),
        ("rejected_reason", Kind::String),
    ]
}
// internal/engine/managed_integration_retry.go SHA-256 297962b946fdfc3e57297c10f1fff9dcea444c2f6562ef32dafb7193084212c1
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct IntegrationRetry {
    // RuntimeBudgetDigest line 18
    #[serde(rename = "control_runtime_budget_sha256")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub control_runtime_budget_sha256: String,
    // Event line 19
    #[serde(rename = "event_id")]
    pub event_id: String,
    // RequestDigest line 20
    #[serde(rename = "request_sha256")]
    pub request_sha256: String,
    // Agent line 21
    #[serde(rename = "agent_id")]
    pub agent_id: String,
    // Attempt line 22
    #[serde(rename = "attempt_id")]
    pub attempt_id: String,
    // Result line 23
    #[serde(rename = "result_commit")]
    pub result_commit: String,
    // Previous line 24
    #[serde(rename = "failed_base")]
    pub failed_base: String,
    // Candidate line 25
    #[serde(rename = "expected_candidate")]
    pub expected_candidate: String,
    // Contract line 26
    #[serde(rename = "contract_sha256")]
    pub contract_sha256: String,
    // Failure line 27
    #[serde(rename = "original_failure")]
    pub original_failure: String,
    // Evidence line 28
    #[serde(rename = "failure_evidence")]
    pub failure_evidence: String,
    // EvidenceDigest line 29
    #[serde(rename = "evidence_sha256")]
    pub evidence_sha256: String,
    // Legacy line 30
    #[serde(rename = "legacy_missing_output")]
    pub legacy_missing_output: bool,
    // Reason line 31
    #[serde(rename = "reason")]
    pub reason: String,
    // At line 32
    #[serde(rename = "at")]
    pub at: String,
}
pub(crate) fn shape_integration_retry() -> Fields {
    &[
        ("control_runtime_budget_sha256", Kind::String),
        ("event_id", Kind::String),
        ("request_sha256", Kind::String),
        ("agent_id", Kind::String),
        ("attempt_id", Kind::String),
        ("result_commit", Kind::String),
        ("failed_base", Kind::String),
        ("expected_candidate", Kind::String),
        ("contract_sha256", Kind::String),
        ("original_failure", Kind::String),
        ("failure_evidence", Kind::String),
        ("evidence_sha256", Kind::String),
        ("legacy_missing_output", Kind::Bool),
        ("reason", Kind::String),
        ("at", Kind::String),
    ]
}
// internal/engine/planning_judge.go SHA-256 053019b5a85ab2f292edca745178700117cbf0bfd588194d981e5dc24d9bcf38
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct JudgeCycleRecord {
    // Verdict line 68
    #[serde(rename = "verdict")]
    pub verdict: String,
    // Reason line 69
    #[serde(rename = "reason")]
    pub reason: String,
    // EvidenceRef line 70
    #[serde(rename = "evidence_ref")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub evidence_ref: String,
    // At line 71
    #[serde(rename = "at")]
    pub at: String,
    // RenewedAt line 72
    #[serde(rename = "renewed_at")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub renewed_at: String,
    // Cycle line 73
    #[serde(rename = "cycle")]
    pub cycle: i64,
}
pub(crate) fn shape_judge_cycle_record() -> Fields {
    &[
        ("verdict", Kind::String),
        ("reason", Kind::String),
        ("evidence_ref", Kind::String),
        ("at", Kind::String),
        ("renewed_at", Kind::String),
        ("cycle", Kind::Int64),
    ]
}
// internal/engine/planning_judge.go SHA-256 053019b5a85ab2f292edca745178700117cbf0bfd588194d981e5dc24d9bcf38
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct JudgeState {
    // Verdict line 54
    #[serde(rename = "verdict")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub verdict: String,
    // Reason line 55
    #[serde(rename = "reason")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub reason: String,
    // EvidenceRef line 56
    #[serde(rename = "evidence_ref")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub evidence_ref: String,
    // At line 57
    #[serde(rename = "at")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub at: String,
    // Cycle line 58
    #[serde(rename = "cycle")]
    pub cycle: i64,
    // MaxCycles line 59
    #[serde(rename = "max_cycles")]
    pub max_cycles: i64,
    // Closed line 60
    #[serde(rename = "closed")]
    pub closed: bool,
    // History line 61
    #[serde(rename = "history")]
    #[serde(skip_serializing_if = "empty_vec")]
    pub history: Option<Vec<JudgeCycleRecord>>,
}
pub(crate) fn shape_judge_state() -> Fields {
    &[
        ("verdict", Kind::String),
        ("reason", Kind::String),
        ("evidence_ref", Kind::String),
        ("at", Kind::String),
        ("cycle", Kind::Int64),
        ("max_cycles", Kind::Int64),
        ("closed", Kind::Bool),
        (
            "history",
            Kind::Array(&Kind::Object(shape_judge_cycle_record)),
        ),
    ]
}
// internal/engine/model.go SHA-256 9b8f2f15d6af6c046cc3b7c909a4a8bf3155b9dd1e6c4ce04612758a763c9086
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LaunchProfile {
    // Skills line 183
    #[serde(rename = "skills")]
    #[serde(skip_serializing_if = "empty_vec")]
    pub skills: Option<Vec<ActionSkillSelection>>,
    // Limits line 184
    #[serde(rename = "limits")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limits: Option<Box<RunLimits>>,
    // Provider line 185
    #[serde(rename = "provider")]
    pub provider: String,
    // Role line 186
    #[serde(rename = "role")]
    pub role: String,
    // Workspace line 187
    #[serde(rename = "workspace")]
    pub workspace: String,
    // Instruction line 188
    #[serde(rename = "instruction")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub instruction: String,
    // Level line 189
    #[serde(rename = "level")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub level: String,
    // Timeout line 190
    #[serde(rename = "timeout_seconds")]
    #[serde(skip_serializing_if = "is_zero")]
    pub timeout_seconds: i64,
    // Capture line 191
    #[serde(rename = "capture_output")]
    #[serde(skip_serializing_if = "is_false")]
    pub capture_output: bool,
    // Updated line 192
    #[serde(rename = "updated")]
    pub updated: String,
    // Actor line 193
    #[serde(rename = "actor")]
    pub actor: String,
}
pub(crate) fn shape_launch_profile() -> Fields {
    &[
        (
            "skills",
            Kind::Array(&Kind::Object(shape_action_skill_selection)),
        ),
        ("limits", Kind::Pointer(&Kind::Object(shape_run_limits))),
        ("provider", Kind::String),
        ("role", Kind::String),
        ("workspace", Kind::String),
        ("instruction", Kind::String),
        ("level", Kind::String),
        ("timeout_seconds", Kind::Int64),
        ("capture_output", Kind::Bool),
        ("updated", Kind::String),
        ("actor", Kind::String),
    ]
}
// internal/engine/managed_review_fragment_store.go SHA-256 c561116865ff7ff3b514e9ec328b9e5b609ca164e0eee614cd5921873b02151b
#[derive(Default, Deserialize)]
pub struct ManagedFragmentJournalAnchor {
    // WorkflowPrompt line 16
    #[serde(rename = "workflow_prompt")]
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub workflow_prompt: String,
    // WorkflowPromptDigest line 17
    #[serde(rename = "workflow_prompt_sha256")]
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub workflow_prompt_sha256: String,
    // RetryDirective line 18
    #[serde(rename = "retry_directive")]
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_directive: Option<Box<ManagedReviewRetryDirective>>,
    // ReplannedFrom line 19
    #[serde(rename = "replanned_from")]
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replanned_from: Option<Box<ManagedFragmentJournalAnchor>>,
    // ResumeCalls line 20
    #[serde(rename = "resume_calls")]
    #[serde(default)]
    #[serde(skip_serializing_if = "empty_vec")]
    pub resume_calls: Option<Vec<String>>,
    // InspectionTask line 21
    #[serde(rename = "inspection_task")]
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub inspection_task: String,
    // InspectionAttempt line 22
    #[serde(rename = "inspection_attempt")]
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub inspection_attempt: String,
    // InspectionProducer line 23
    #[serde(rename = "inspection_producer")]
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub inspection_producer: String,
    // FinalJournal line 24
    #[serde(rename = "final_journal")]
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub final_journal: String,
    // FinalJournalDigest line 25
    #[serde(rename = "final_journal_sha256")]
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub final_journal_sha256: String,
    // WorkflowDigest line 26
    #[serde(rename = "workflow_sha256")]
    #[serde(default)]
    pub workflow_sha256: String,
    // ModelConfigDigest line 27
    #[serde(rename = "model_config_sha256")]
    #[serde(default)]
    pub model_config_sha256: String,
    // Plan line 28
    #[serde(rename = "plan")]
    #[serde(default)]
    pub plan: String,
    // PlanDigest line 29
    #[serde(rename = "plan_sha256")]
    #[serde(default)]
    pub plan_sha256: String,
    // Journal line 30
    #[serde(rename = "journal")]
    #[serde(default)]
    pub journal: String,
    // JournalDigest line 31
    #[serde(rename = "journal_sha256")]
    #[serde(default)]
    pub journal_sha256: String,
}
pub(crate) fn shape_managed_fragment_journal_anchor() -> Fields {
    &[
        ("workflow_prompt", Kind::String),
        ("workflow_prompt_sha256", Kind::String),
        (
            "retry_directive",
            Kind::Pointer(&Kind::Object(shape_managed_review_retry_directive)),
        ),
        (
            "replanned_from",
            Kind::Pointer(&Kind::Object(shape_managed_fragment_journal_anchor)),
        ),
        ("resume_calls", Kind::Array(&Kind::String)),
        ("inspection_task", Kind::String),
        ("inspection_attempt", Kind::String),
        ("inspection_producer", Kind::String),
        ("final_journal", Kind::String),
        ("final_journal_sha256", Kind::String),
        ("workflow_sha256", Kind::String),
        ("model_config_sha256", Kind::String),
        ("plan", Kind::String),
        ("plan_sha256", Kind::String),
        ("journal", Kind::String),
        ("journal_sha256", Kind::String),
    ]
}
#[derive(Serialize)]
struct AnchorPrefix<'a> {
    #[serde(rename = "workflow_prompt")]
    #[serde(skip_serializing_if = "String::is_empty")]
    workflow_prompt: &'a String,
    #[serde(rename = "workflow_prompt_sha256")]
    #[serde(skip_serializing_if = "String::is_empty")]
    workflow_prompt_sha256: &'a String,
    #[serde(rename = "retry_directive")]
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_directive: &'a Option<Box<ManagedReviewRetryDirective>>,
}
#[derive(Serialize)]
struct AnchorSuffix<'a> {
    #[serde(rename = "resume_calls")]
    #[serde(skip_serializing_if = "empty_vec")]
    resume_calls: &'a Option<Vec<String>>,
    #[serde(rename = "inspection_task")]
    #[serde(skip_serializing_if = "String::is_empty")]
    inspection_task: &'a String,
    #[serde(rename = "inspection_attempt")]
    #[serde(skip_serializing_if = "String::is_empty")]
    inspection_attempt: &'a String,
    #[serde(rename = "inspection_producer")]
    #[serde(skip_serializing_if = "String::is_empty")]
    inspection_producer: &'a String,
    #[serde(rename = "final_journal")]
    #[serde(skip_serializing_if = "String::is_empty")]
    final_journal: &'a String,
    #[serde(rename = "final_journal_sha256")]
    #[serde(skip_serializing_if = "String::is_empty")]
    final_journal_sha256: &'a String,
    #[serde(rename = "workflow_sha256")]
    workflow_sha256: &'a String,
    #[serde(rename = "model_config_sha256")]
    model_config_sha256: &'a String,
    #[serde(rename = "plan")]
    plan: &'a String,
    #[serde(rename = "plan_sha256")]
    plan_sha256: &'a String,
    #[serde(rename = "journal")]
    journal: &'a String,
    #[serde(rename = "journal_sha256")]
    journal_sha256: &'a String,
}
impl Serialize for ManagedFragmentJournalAnchor {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut output = Vec::new();
        let mut suffixes = Vec::new();
        let mut node = self;
        loop {
            let prefix = serde_json::to_vec(&AnchorPrefix {
                workflow_prompt: &node.workflow_prompt,
                workflow_prompt_sha256: &node.workflow_prompt_sha256,
                retry_directive: &node.retry_directive,
            })
            .map_err(serde::ser::Error::custom)?;
            let suffix = serde_json::to_vec(&AnchorSuffix {
                resume_calls: &node.resume_calls,
                inspection_task: &node.inspection_task,
                inspection_attempt: &node.inspection_attempt,
                inspection_producer: &node.inspection_producer,
                final_journal: &node.final_journal,
                final_journal_sha256: &node.final_journal_sha256,
                workflow_sha256: &node.workflow_sha256,
                model_config_sha256: &node.model_config_sha256,
                plan: &node.plan,
                plan_sha256: &node.plan_sha256,
                journal: &node.journal,
                journal_sha256: &node.journal_sha256,
            })
            .map_err(serde::ser::Error::custom)?;
            output.push(b'{');
            if prefix.len() > 2 {
                output.extend_from_slice(&prefix[1..prefix.len() - 1]);
                output.push(b',');
            }
            if let Some(child) = node.replanned_from.as_deref() {
                output.extend_from_slice(b"\"replanned_from\":");
                suffixes.push(suffix);
                node = child;
            } else {
                output.extend_from_slice(&suffix[1..suffix.len() - 1]);
                output.push(b'}');
                break;
            }
        }
        for suffix in suffixes.into_iter().rev() {
            output.push(b',');
            output.extend_from_slice(&suffix[1..suffix.len() - 1]);
            output.push(b'}');
        }
        let text = String::from_utf8(output).map_err(serde::ser::Error::custom)?;
        let raw =
            serde_json::value::RawValue::from_string(text).map_err(serde::ser::Error::custom)?;
        raw.serialize(s)
    }
}
impl Drop for ManagedFragmentJournalAnchor {
    fn drop(&mut self) {
        let mut next = self.replanned_from.take();
        while let Some(mut node) = next {
            next = node.replanned_from.take();
        }
    }
}
impl Clone for ManagedFragmentJournalAnchor {
    fn clone(&self) -> Self {
        let mut chain = Vec::new();
        let mut next = Some(self);
        while let Some(node) = next {
            chain.push(node);
            next = node.replanned_from.as_deref();
        }
        let mut previous = None;
        for node in chain.into_iter().rev() {
            previous = Some(Box::new(Self {
                replanned_from: previous,
                workflow_prompt: node.workflow_prompt.clone(),
                workflow_prompt_sha256: node.workflow_prompt_sha256.clone(),
                retry_directive: node.retry_directive.clone(),
                resume_calls: node.resume_calls.clone(),
                inspection_task: node.inspection_task.clone(),
                inspection_attempt: node.inspection_attempt.clone(),
                inspection_producer: node.inspection_producer.clone(),
                final_journal: node.final_journal.clone(),
                final_journal_sha256: node.final_journal_sha256.clone(),
                workflow_sha256: node.workflow_sha256.clone(),
                model_config_sha256: node.model_config_sha256.clone(),
                plan: node.plan.clone(),
                plan_sha256: node.plan_sha256.clone(),
                journal: node.journal.clone(),
                journal_sha256: node.journal_sha256.clone(),
            }));
        }
        *previous.expect("anchor clone has a root")
    }
}
impl PartialEq for ManagedFragmentJournalAnchor {
    fn eq(&self, other: &Self) -> bool {
        let (mut left, mut right) = (Some(self), Some(other));
        loop {
            match (left, right) {
                (None, None) => return true,
                (Some(a), Some(b)) => {
                    if a.workflow_prompt != b.workflow_prompt
                        || a.workflow_prompt_sha256 != b.workflow_prompt_sha256
                        || a.retry_directive != b.retry_directive
                        || a.resume_calls != b.resume_calls
                        || a.inspection_task != b.inspection_task
                        || a.inspection_attempt != b.inspection_attempt
                        || a.inspection_producer != b.inspection_producer
                        || a.final_journal != b.final_journal
                        || a.final_journal_sha256 != b.final_journal_sha256
                        || a.workflow_sha256 != b.workflow_sha256
                        || a.model_config_sha256 != b.model_config_sha256
                        || a.plan != b.plan
                        || a.plan_sha256 != b.plan_sha256
                        || a.journal != b.journal
                        || a.journal_sha256 != b.journal_sha256
                    {
                        return false;
                    }
                    left = a.replanned_from.as_deref();
                    right = b.replanned_from.as_deref();
                }
                _ => return false,
            }
        }
    }
}
impl std::fmt::Debug for ManagedFragmentJournalAnchor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut depth = 0usize;
        let mut node = self.replanned_from.as_deref();
        while let Some(v) = node {
            depth += 1;
            node = v.replanned_from.as_deref();
        }
        f.debug_struct("ManagedFragmentJournalAnchor")
            .field("replanned_from_depth", &depth)
            .finish()
    }
}
// internal/engine/managed_git.go SHA-256 c8a62e0ba38a8c19f18815655f84ce6620fd4f78505216842b1d472de29626a5
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ManagedRepository {
    // Subdir line 26
    #[serde(rename = "project_subdir")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub project_subdir: String,
    // Source line 27
    #[serde(rename = "source")]
    pub source: String,
    // Storage line 28
    #[serde(rename = "storage")]
    pub storage: String,
    // Base line 29
    #[serde(rename = "base_commit")]
    pub base_commit: String,
    // Candidate line 30
    #[serde(rename = "candidate_commit")]
    pub candidate_commit: String,
    // Snapshot line 31
    #[serde(rename = "snapshot_digest")]
    pub snapshot_digest: String,
    // Included line 32
    #[serde(rename = "included")]
    #[serde(skip_serializing_if = "empty_vec")]
    pub included: Option<Vec<String>>,
    // RequestDigest line 33
    #[serde(rename = "request_digest")]
    pub request_digest: String,
}
pub(crate) fn shape_managed_repository() -> Fields {
    &[
        ("project_subdir", Kind::String),
        ("source", Kind::String),
        ("storage", Kind::String),
        ("base_commit", Kind::String),
        ("candidate_commit", Kind::String),
        ("snapshot_digest", Kind::String),
        ("included", Kind::Array(&Kind::String)),
        ("request_digest", Kind::String),
    ]
}
// internal/engine/independent_review.go SHA-256 8aaa695b6bcc95e61684c0cbc099dd34be73cb18374a14c3a7ea8b2d604f20f5
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ManagedReviewBatchVerdict {
    // RetryFeedback line 67
    #[serde(rename = "retry_feedback")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub retry_feedback: String,
    // ID line 68
    #[serde(rename = "id")]
    pub id: String,
    // Tasks line 69
    #[serde(rename = "tasks")]
    pub tasks: Option<Vec<String>>,
    // Context line 70
    #[serde(rename = "context")]
    pub context: String,
    // ContextDigest line 71
    #[serde(rename = "context_sha256")]
    pub context_sha256: String,
    // ReplyPath line 72
    #[serde(rename = "reply_path")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub reply_path: String,
    // ReplyDigest line 73
    #[serde(rename = "reply_sha256")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub reply_sha256: String,
    // State line 74
    #[serde(rename = "state")]
    pub state: String,
    // Started line 75
    #[serde(rename = "started")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub started: String,
    // Finished line 76
    #[serde(rename = "finished")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub finished: String,
}
pub(crate) fn shape_managed_review_batch_verdict() -> Fields {
    &[
        ("retry_feedback", Kind::String),
        ("id", Kind::String),
        ("tasks", Kind::Array(&Kind::String)),
        ("context", Kind::String),
        ("context_sha256", Kind::String),
        ("reply_path", Kind::String),
        ("reply_sha256", Kind::String),
        ("state", Kind::String),
        ("started", Kind::String),
        ("finished", Kind::String),
    ]
}
// internal/engine/managed_review_retry_delivery.go SHA-256 7aec051895efb15b343508062ab34bab7a8e2e202695fcfaeb4c2e7454abb59b
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ManagedReviewRetryDirective {
    // CitationDiagnostic line 16
    #[serde(rename = "citation_diagnostic")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citation_diagnostic: Option<Box<ManagedReviewCitationDiagnostic>>,
    // Decision line 17
    #[serde(rename = "decision")]
    pub decision: String,
    // Scope line 18
    #[serde(rename = "scope")]
    pub scope: String,
    // ResultEvent line 19
    #[serde(rename = "result_event")]
    pub result_event: String,
    // Review line 20
    #[serde(rename = "review")]
    pub review: String,
    // Candidate line 21
    #[serde(rename = "candidate_commit")]
    pub candidate_commit: String,
    // Attempt line 22
    #[serde(rename = "attempt")]
    pub attempt: String,
    // FailureCode line 23
    #[serde(rename = "failure_code")]
    pub failure_code: String,
    // Cause line 24
    #[serde(rename = "cause")]
    pub cause: String,
    // Instruction line 25
    #[serde(rename = "instruction")]
    pub instruction: String,
}
pub(crate) fn shape_managed_review_retry_directive() -> Fields {
    &[
        (
            "citation_diagnostic",
            Kind::Pointer(&Kind::Object(shape_managed_review_citation_diagnostic)),
        ),
        ("decision", Kind::String),
        ("scope", Kind::String),
        ("result_event", Kind::String),
        ("review", Kind::String),
        ("candidate_commit", Kind::String),
        ("attempt", Kind::String),
        ("failure_code", Kind::String),
        ("cause", Kind::String),
        ("instruction", Kind::String),
    ]
}
// internal/engine/managed_review.go SHA-256 9f21630185c1523a0327d94eb736393490d1368fa23eae4eef959b708cec29d2
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ManagedTaskReview {
    // Task line 26
    #[serde(rename = "task")]
    pub task: String,
    // Attempt line 27
    #[serde(rename = "attempt")]
    pub attempt: String,
    // Producer line 28
    #[serde(rename = "producer")]
    pub producer: String,
    // Contract line 29
    #[serde(rename = "contract")]
    pub contract: String,
    // Policy line 30
    #[serde(rename = "policy")]
    pub policy: String,
    // Report line 31
    #[serde(rename = "report")]
    pub report: String,
    // ReportDigest line 32
    #[serde(rename = "report_sha256")]
    pub report_sha256: String,
    // Reason line 33
    #[serde(rename = "reason")]
    pub reason: String,
    // Criteria line 34
    #[serde(rename = "criteria")]
    pub criteria: Option<Vec<ReviewCriterion>>,
}
pub(crate) fn shape_managed_task_review() -> Fields {
    &[
        ("task", Kind::String),
        ("attempt", Kind::String),
        ("producer", Kind::String),
        ("contract", Kind::String),
        ("policy", Kind::String),
        ("report", Kind::String),
        ("report_sha256", Kind::String),
        ("reason", Kind::String),
        (
            "criteria",
            Kind::Array(&Kind::Object(shape_review_criterion)),
        ),
    ]
}
// internal/engine/model.go SHA-256 9b8f2f15d6af6c046cc3b7c909a4a8bf3155b9dd1e6c4ce04612758a763c9086
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ManualOverride {
    // Reason line 19
    #[serde(rename = "reason")]
    pub reason: String,
    // Actor line 20
    #[serde(rename = "actor")]
    pub actor: String,
    // At line 21
    #[serde(rename = "at")]
    pub at: String,
    // PreviousStatus line 22
    #[serde(rename = "previous_status")]
    pub previous_status: String,
    // Revision line 23
    #[serde(rename = "revision")]
    pub revision: i64,
}
pub(crate) fn shape_manual_override() -> Fields {
    &[
        ("reason", Kind::String),
        ("actor", Kind::String),
        ("at", Kind::String),
        ("previous_status", Kind::String),
        ("revision", Kind::Int64),
    ]
}
// internal/engine/project_memory.go SHA-256 16aa64873f1a26f102a9c1e26d4021425bf063ea7ff1b58c7634a423ec77070d
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoryReference {
    // Work line 20
    #[serde(rename = "work")]
    pub work: String,
    // ID line 21
    #[serde(rename = "id")]
    pub id: String,
    // Version line 22
    #[serde(rename = "version")]
    pub version: i64,
    // SHA256 line 23
    #[serde(rename = "sha256")]
    pub sha256: String,
}
pub(crate) fn shape_memory_reference() -> Fields {
    &[
        ("work", Kind::String),
        ("id", Kind::String),
        ("version", Kind::Int64),
        ("sha256", Kind::String),
    ]
}
// internal/engine/project_memory.go SHA-256 16aa64873f1a26f102a9c1e26d4021425bf063ea7ff1b58c7634a423ec77070d
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MemorySelectionDecision {
    // EventID line 27
    #[serde(rename = "event_id")]
    pub event_id: String,
    // Actor line 28
    #[serde(rename = "actor")]
    pub actor: String,
    // Reason line 29
    #[serde(rename = "reason")]
    pub reason: String,
    // At line 30
    #[serde(rename = "at")]
    pub at: String,
}
pub(crate) fn shape_memory_selection_decision() -> Fields {
    &[
        ("event_id", Kind::String),
        ("actor", Kind::String),
        ("reason", Kind::String),
        ("at", Kind::String),
    ]
}
// internal/engine/memory_summary_selection.go SHA-256 09151f72964711349efd2038f9aef0ea137851c967638ceadc38e9fea5f12c52
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MemorySummaryProtection {
    // ID line 22
    #[serde(rename = "id")]
    pub id: String,
    // Message line 23
    #[serde(rename = "message")]
    pub message: String,
    // Kind line 24
    #[serde(rename = "kind")]
    pub kind: String,
    // State line 25
    #[serde(rename = "state")]
    pub state: String,
}
pub(crate) fn shape_memory_summary_protection() -> Fields {
    &[
        ("id", Kind::String),
        ("message", Kind::String),
        ("kind", Kind::String),
        ("state", Kind::String),
    ]
}
// internal/engine/memory_summary_selection.go SHA-256 09151f72964711349efd2038f9aef0ea137851c967638ceadc38e9fea5f12c52
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MemorySummaryReceipt {
    // Schema line 29
    #[serde(rename = "schema_version")]
    pub schema_version: i64,
    // Selection line 30
    #[serde(rename = "selection")]
    pub selection: MemorySummarySelection,
    // SelectionSHA line 31
    #[serde(rename = "selection_sha256")]
    pub selection_sha256: String,
    // Work line 32
    #[serde(rename = "work")]
    pub work: String,
    // Task line 33
    #[serde(rename = "task")]
    pub task: String,
    // PreparedRevision line 34
    #[serde(rename = "prepared_revision")]
    pub prepared_revision: i64,
    // SourceCount line 35
    #[serde(rename = "source_message_count")]
    pub source_message_count: i64,
    // Received line 36
    #[serde(rename = "received_message_ids")]
    pub received_message_ids: Option<Vec<String>>,
    // Omitted line 37
    #[serde(rename = "omitted_message_ids")]
    pub omitted_message_ids: Option<Vec<String>>,
    // SnapshotInjected line 38
    #[serde(rename = "operator_snapshot_injected")]
    pub operator_snapshot_injected: bool,
    // AttachmentSHA line 39
    #[serde(rename = "attachment_sha256")]
    pub attachment_sha256: String,
    // AttachmentBytes line 40
    #[serde(rename = "attachment_bytes")]
    pub attachment_bytes: i64,
    // QueuedRevision line 41
    #[serde(rename = "queued_revision")]
    #[serde(skip_serializing_if = "is_zero")]
    pub queued_revision: i64,
    // QueuedSourceSHA line 42
    #[serde(rename = "queued_source_sha256")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub queued_source_sha256: String,
    // CheckedBeforeStart line 43
    #[serde(rename = "checked_before_start_at")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub checked_before_start_at: String,
    // CheckedSourceSHA line 44
    #[serde(rename = "checked_source_sha256")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub checked_source_sha256: String,
}
pub(crate) fn shape_memory_summary_receipt() -> Fields {
    &[
        ("schema_version", Kind::Int64),
        ("selection", Kind::Object(shape_memory_summary_selection)),
        ("selection_sha256", Kind::String),
        ("work", Kind::String),
        ("task", Kind::String),
        ("prepared_revision", Kind::Int64),
        ("source_message_count", Kind::Int64),
        ("received_message_ids", Kind::Array(&Kind::String)),
        ("omitted_message_ids", Kind::Array(&Kind::String)),
        ("operator_snapshot_injected", Kind::Bool),
        ("attachment_sha256", Kind::String),
        ("attachment_bytes", Kind::Int64),
        ("queued_revision", Kind::Int64),
        ("queued_source_sha256", Kind::String),
        ("checked_before_start_at", Kind::String),
        ("checked_source_sha256", Kind::String),
    ]
}
// internal/engine/memory_summary_selection.go SHA-256 09151f72964711349efd2038f9aef0ea137851c967638ceadc38e9fea5f12c52
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MemorySummarySelection {
    // Schema line 13
    #[serde(rename = "schema_version")]
    pub schema_version: i64,
    // SourceSHA line 14
    #[serde(rename = "source_sha256")]
    pub source_sha256: String,
    // Dialogues line 15
    #[serde(rename = "dialogues")]
    pub dialogues: Option<Vec<DialogueRef>>,
    // Cutoff line 16
    #[serde(rename = "cutoff")]
    pub cutoff: i64,
    // Protected line 17
    #[serde(rename = "protected")]
    pub protected: Option<Vec<MemorySummaryProtection>>,
    // Reason line 18
    #[serde(rename = "reason")]
    pub reason: String,
}
pub(crate) fn shape_memory_summary_selection() -> Fields {
    &[
        ("schema_version", Kind::Int64),
        ("source_sha256", Kind::String),
        ("dialogues", Kind::Array(&Kind::Object(shape_dialogue_ref))),
        ("cutoff", Kind::Int64),
        (
            "protected",
            Kind::Array(&Kind::Object(shape_memory_summary_protection)),
        ),
        ("reason", Kind::String),
    ]
}
// internal/engine/project_memory.go SHA-256 16aa64873f1a26f102a9c1e26d4021425bf063ea7ff1b58c7634a423ec77070d
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoryVersion {
    // Version line 87
    #[serde(rename = "version")]
    pub version: i64,
    // Action line 88
    #[serde(rename = "action")]
    pub action: String,
    // Kind line 89
    #[serde(rename = "kind")]
    pub kind: String,
    // Body line 90
    #[serde(rename = "body")]
    pub body: String,
    // Conditions line 91
    #[serde(rename = "conditions")]
    pub conditions: String,
    // SourceRetex line 92
    #[serde(rename = "source_retex")]
    pub source_retex: String,
    // SourceSHA256 line 93
    #[serde(rename = "source_sha256")]
    pub source_sha256: String,
    // ProjectSHA256 line 94
    #[serde(rename = "project_sha256")]
    pub project_sha256: String,
    // Actor line 95
    #[serde(rename = "actor")]
    pub actor: String,
    // Reason line 96
    #[serde(rename = "reason")]
    pub reason: String,
    // At line 97
    #[serde(rename = "at")]
    pub at: String,
    // SHA256 line 98
    #[serde(rename = "sha256")]
    pub sha256: String,
}
pub(crate) fn shape_memory_version() -> Fields {
    &[
        ("version", Kind::Int64),
        ("action", Kind::String),
        ("kind", Kind::String),
        ("body", Kind::String),
        ("conditions", Kind::String),
        ("source_retex", Kind::String),
        ("source_sha256", Kind::String),
        ("project_sha256", Kind::String),
        ("actor", Kind::String),
        ("reason", Kind::String),
        ("at", Kind::String),
        ("sha256", Kind::String),
    ]
}
// internal/engine/provider_models.go SHA-256 d1c32751ad1420662b630defb18a04e5bdca71e38a0c5e8fe5dc1aa96e6b3a4b
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelRoute {
    // Level line 26
    #[serde(rename = "level")]
    pub level: String,
    // Model line 27
    #[serde(rename = "model")]
    pub model: String,
    // Effort line 28
    #[serde(rename = "effort")]
    pub effort: String,
    // Billing line 29
    #[serde(rename = "billing")]
    pub billing: String,
    // PolicyHash line 30
    #[serde(rename = "policy_hash")]
    pub policy_hash: String,
    // Reason line 31
    #[serde(rename = "reason")]
    pub reason: String,
}
pub(crate) fn shape_model_route() -> Fields {
    &[
        ("level", Kind::String),
        ("model", Kind::String),
        ("effort", Kind::String),
        ("billing", Kind::String),
        ("policy_hash", Kind::String),
        ("reason", Kind::String),
    ]
}
// internal/engine/plan_contract.go SHA-256 1dc52ac99dee53d507d7ee00f994ecd6f7e3e820b41ac1dc749c12f09b44b0a8
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlanMission {
    // Phase line 15
    #[serde(rename = "phase")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub phase: String,
    // ID line 16
    #[serde(rename = "id")]
    pub id: String,
    // Title line 17
    #[serde(rename = "title")]
    pub title: String,
    // Role line 18
    #[serde(rename = "role")]
    pub role: String,
    // Scope line 19
    #[serde(rename = "scope")]
    pub scope: String,
    // Deliverable line 20
    #[serde(rename = "deliverable")]
    pub deliverable: String,
    // Depends line 21
    #[serde(rename = "depends")]
    pub depends: Option<Vec<String>>,
    // Criteria line 22
    #[serde(rename = "criteria")]
    pub criteria: Option<Vec<String>>,
    // Proof line 23
    #[serde(rename = "proof")]
    pub proof: String,
    // Entry line 24
    #[serde(rename = "entry")]
    pub entry: String,
    // Validation line 25
    #[serde(rename = "validation")]
    pub validation: String,
    // Delivery line 26
    #[serde(rename = "delivery")]
    pub delivery: String,
    // Stop line 27
    #[serde(rename = "stop")]
    pub stop: String,
    // MaxAttempts line 28
    #[serde(rename = "max_attempts")]
    pub max_attempts: i64,
    // MaxToolCalls line 29
    #[serde(rename = "max_tool_calls")]
    pub max_tool_calls: i64,
}
pub(crate) fn shape_plan_mission() -> Fields {
    &[
        ("phase", Kind::String),
        ("id", Kind::String),
        ("title", Kind::String),
        ("role", Kind::String),
        ("scope", Kind::String),
        ("deliverable", Kind::String),
        ("depends", Kind::Array(&Kind::String)),
        ("criteria", Kind::Array(&Kind::String)),
        ("proof", Kind::String),
        ("entry", Kind::String),
        ("validation", Kind::String),
        ("delivery", Kind::String),
        ("stop", Kind::String),
        ("max_attempts", Kind::Int64),
        ("max_tool_calls", Kind::Int64),
    ]
}
// internal/engine/plan_contract.go SHA-256 1dc52ac99dee53d507d7ee00f994ecd6f7e3e820b41ac1dc749c12f09b44b0a8
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlanQuestion {
    // Question line 11
    #[serde(rename = "question")]
    pub question: String,
    // Answer line 12
    #[serde(rename = "answer")]
    pub answer: String,
}
pub(crate) fn shape_plan_question() -> Fields {
    &[("question", Kind::String), ("answer", Kind::String)]
}
// internal/engine/brainstorm.go SHA-256 dc1dd724d07d6d963de53d13048609c619adbc3b7ba726d4234f06bb72b8ae10
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlanningBrief {
    // Task line 13
    #[serde(rename = "task")]
    pub task: String,
    // Path line 14
    #[serde(rename = "path")]
    pub path: String,
    // SHA256 line 15
    #[serde(rename = "sha256")]
    pub sha256: String,
    // Text line 16
    #[serde(rename = "text")]
    pub text: String,
    // At line 17
    #[serde(rename = "at")]
    pub at: String,
}
pub(crate) fn shape_planning_brief() -> Fields {
    &[
        ("task", Kind::String),
        ("path", Kind::String),
        ("sha256", Kind::String),
        ("text", Kind::String),
        ("at", Kind::String),
    ]
}
// internal/engine/planning_delivery.go SHA-256 01bf9ce7df57441449fa2f89be8e353b0b2faa424b109577ae85c4966217ecb1
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlanningDelivery {
    // JudgeAvailable line 15
    #[serde(rename = "judge_available")]
    #[serde(skip_serializing_if = "is_false")]
    pub judge_available: bool,
    // JudgeCycle line 16
    #[serde(rename = "judge_cycle")]
    #[serde(skip_serializing_if = "is_zero")]
    pub judge_cycle: i64,
    // RenewAvailable line 17
    #[serde(rename = "renew_available")]
    #[serde(skip_serializing_if = "is_false")]
    pub renew_available: bool,
    // JudgeEndAvailable line 18
    #[serde(rename = "judge_end_available")]
    #[serde(skip_serializing_if = "is_false")]
    pub judge_end_available: bool,
    // TaskBudget line 19
    #[serde(rename = "task_budget")]
    #[serde(skip_serializing_if = "is_zero")]
    pub task_budget: i64,
    // ActivationBudget line 20
    #[serde(rename = "activation_budget")]
    #[serde(skip_serializing_if = "is_zero")]
    pub activation_budget: i64,
    // DelegationUnavailable line 21
    #[serde(rename = "delegation_unavailable")]
    #[serde(skip_serializing_if = "is_false")]
    pub delegation_unavailable: bool,
    // ClosureUnavailable line 22
    #[serde(rename = "closure_unavailable")]
    #[serde(skip_serializing_if = "is_false")]
    pub closure_unavailable: bool,
    // ExplorationAvailable line 23
    #[serde(rename = "exploration_available")]
    #[serde(skip_serializing_if = "is_false")]
    pub exploration_available: bool,
    // UnresolvedReviews line 24
    #[serde(rename = "unresolved_review_sources")]
    #[serde(skip_serializing_if = "empty_map")]
    pub unresolved_review_sources: Option<BTreeMap<String, ReviewFailureRouting>>,
    // ReviewRetryUnavailable line 25
    #[serde(rename = "review_retry_unavailable")]
    #[serde(skip_serializing_if = "is_false")]
    pub review_retry_unavailable: bool,
    // ProductionRetryUnavailable line 26
    #[serde(rename = "production_retry_unavailable")]
    #[serde(skip_serializing_if = "is_false")]
    pub production_retry_unavailable: bool,
    // HistoricalOnly line 27
    #[serde(rename = "historical_only")]
    #[serde(skip_serializing_if = "is_false")]
    pub historical_only: bool,
    // SHA256 line 28
    #[serde(rename = "context_sha256")]
    pub context_sha256: String,
    // Events line 29
    #[serde(rename = "events")]
    pub events: Option<Vec<String>>,
    // Reports line 30
    #[serde(rename = "reports")]
    #[serde(skip_serializing_if = "empty_vec")]
    pub reports: Option<Vec<ExchangeArtifact>>,
    // Decision line 31
    #[serde(rename = "decision")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub decision: String,
    // Unavailable line 32
    #[serde(rename = "unavailable_reports")]
    #[serde(skip_serializing_if = "empty_map")]
    pub unavailable_reports: Option<BTreeMap<String, String>>,
}
pub(crate) fn shape_planning_delivery() -> Fields {
    &[
        ("judge_available", Kind::Bool),
        ("judge_cycle", Kind::Int64),
        ("renew_available", Kind::Bool),
        ("judge_end_available", Kind::Bool),
        ("task_budget", Kind::Int64),
        ("activation_budget", Kind::Int64),
        ("delegation_unavailable", Kind::Bool),
        ("closure_unavailable", Kind::Bool),
        ("exploration_available", Kind::Bool),
        (
            "unresolved_review_sources",
            Kind::Map(&Kind::Object(shape_review_failure_routing)),
        ),
        ("review_retry_unavailable", Kind::Bool),
        ("production_retry_unavailable", Kind::Bool),
        ("historical_only", Kind::Bool),
        ("context_sha256", Kind::String),
        ("events", Kind::Array(&Kind::String)),
        (
            "reports",
            Kind::Array(&Kind::Object(shape_exchange_artifact)),
        ),
        ("decision", Kind::String),
        ("unavailable_reports", Kind::Map(&Kind::String)),
    ]
}
// internal/engine/planning.go SHA-256 a0e508c1dfef64383383ed9e3fa3a8482e8fd6682f39735c815e2ac71de95b0e
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlanningEvent {
    // Exploration line 64
    #[serde(rename = "exploration")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exploration: Option<Box<PlanningExploration>>,
    // RejectionKey line 65
    #[serde(rename = "rejection_key")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub rejection_key: String,
    // RunObservation line 66
    #[serde(rename = "run_observation")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_observation: Option<Box<RunObservation>>,
    // RunDecision line 67
    #[serde(rename = "run_decision")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub run_decision: String,
    // RunReason line 68
    #[serde(rename = "run_reason")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub run_reason: String,
    // Handoff line 69
    #[serde(rename = "handoff")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handoff: Option<Box<ExchangeArtifact>>,
    // ID line 70
    #[serde(rename = "id")]
    pub id: String,
    // Scope line 71
    #[serde(rename = "scope")]
    pub scope: String,
    // Kind line 72
    #[serde(rename = "kind")]
    pub kind: String,
    // Task line 73
    #[serde(rename = "task")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub task: String,
    // Attempt line 74
    #[serde(rename = "attempt")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub attempt: String,
    // Message line 75
    #[serde(rename = "message")]
    pub message: String,
    // Artifacts line 76
    #[serde(rename = "artifacts")]
    #[serde(skip_serializing_if = "empty_vec")]
    pub artifacts: Option<Vec<ExchangeArtifact>>,
    // Decision line 77
    #[serde(rename = "decision")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub decision: String,
    // At line 78
    #[serde(rename = "at")]
    pub at: String,
}
pub(crate) fn shape_planning_event() -> Fields {
    &[
        (
            "exploration",
            Kind::Pointer(&Kind::Object(shape_planning_exploration)),
        ),
        ("rejection_key", Kind::String),
        (
            "run_observation",
            Kind::Pointer(&Kind::Object(shape_run_observation)),
        ),
        ("run_decision", Kind::String),
        ("run_reason", Kind::String),
        (
            "handoff",
            Kind::Pointer(&Kind::Object(shape_exchange_artifact)),
        ),
        ("id", Kind::String),
        ("scope", Kind::String),
        ("kind", Kind::String),
        ("task", Kind::String),
        ("attempt", Kind::String),
        ("message", Kind::String),
        (
            "artifacts",
            Kind::Array(&Kind::Object(shape_exchange_artifact)),
        ),
        ("decision", Kind::String),
        ("at", Kind::String),
    ]
}
// internal/engine/planning_exploration.go SHA-256 b5b140a17abdc17d8fea13e91a9d3507b00fbc583c9364ffdb58153f7760d735
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlanningExploration {
    // Scope line 21
    #[serde(rename = "scope")]
    pub scope: String,
    // ScopeRevision line 22
    #[serde(rename = "scope_revision")]
    pub scope_revision: i64,
    // Generation line 23
    #[serde(rename = "generation")]
    pub generation: i64,
    // Revision line 24
    #[serde(rename = "observed_revision")]
    pub observed_revision: i64,
    // Decision line 25
    #[serde(rename = "request_decision")]
    pub request_decision: String,
    // Candidate line 26
    #[serde(rename = "candidate_commit")]
    pub candidate_commit: String,
    // Request line 27
    #[serde(rename = "request")]
    pub request: PlanningExplorationRequest,
    // Entries line 28
    #[serde(rename = "entries")]
    #[serde(skip_serializing_if = "empty_vec")]
    pub entries: Option<Vec<String>>,
    // Sources line 29
    #[serde(rename = "sources")]
    #[serde(skip_serializing_if = "empty_vec")]
    pub sources: Option<Vec<ReviewSource>>,
    // Digest line 30
    #[serde(rename = "sha256")]
    pub sha256: String,
}
pub(crate) fn shape_planning_exploration() -> Fields {
    &[
        ("scope", Kind::String),
        ("scope_revision", Kind::Int64),
        ("generation", Kind::Int64),
        ("observed_revision", Kind::Int64),
        ("request_decision", Kind::String),
        ("candidate_commit", Kind::String),
        ("request", Kind::Object(shape_planning_exploration_request)),
        ("entries", Kind::Array(&Kind::String)),
        ("sources", Kind::Array(&Kind::Object(shape_review_source))),
        ("sha256", Kind::String),
    ]
}
// internal/engine/planning_exploration.go SHA-256 b5b140a17abdc17d8fea13e91a9d3507b00fbc583c9364ffdb58153f7760d735
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlanningExplorationRequest {
    // Mode line 15
    #[serde(rename = "mode")]
    pub mode: String,
    // Candidate line 16
    #[serde(rename = "candidate_commit")]
    pub candidate_commit: String,
    // Directory line 17
    #[serde(rename = "directory")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub directory: String,
    // Files line 18
    #[serde(rename = "files")]
    #[serde(skip_serializing_if = "empty_vec")]
    pub files: Option<Vec<String>>,
}
pub(crate) fn shape_planning_exploration_request() -> Fields {
    &[
        ("mode", Kind::String),
        ("candidate_commit", Kind::String),
        ("directory", Kind::String),
        ("files", Kind::Array(&Kind::String)),
    ]
}
// internal/engine/planning.go SHA-256 a0e508c1dfef64383383ed9e3fa3a8482e8fd6682f39735c815e2ac71de95b0e
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlanningRejectionWait {
    // Event line 60
    #[serde(rename = "event_id")]
    pub event_id: String,
    // StateSHA256 line 61
    #[serde(rename = "state_sha256")]
    pub state_sha256: String,
}
pub(crate) fn shape_planning_rejection_wait() -> Fields {
    &[("event_id", Kind::String), ("state_sha256", Kind::String)]
}
// internal/engine/planning.go SHA-256 a0e508c1dfef64383383ed9e3fa3a8482e8fd6682f39735c815e2ac71de95b0e
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlanningScope {
    // RejectionWait line 38
    #[serde(rename = "rejection_wait")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejection_wait: Option<Box<PlanningRejectionWait>>,
    // ModelSelection line 39
    #[serde(rename = "model_selection")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_selection: Option<Box<RoleModel>>,
    // LastModel line 40
    #[serde(rename = "last_model")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_model: Option<Box<RoleModel>>,
    // Delivery line 41
    #[serde(rename = "delivery")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery: Option<Box<PlanningDelivery>>,
    // Workflow line 42
    #[serde(rename = "workflow")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow: Option<Box<AgentWorkflow>>,
    // TaskLimit line 43
    #[serde(rename = "max_tasks")]
    #[serde(skip_serializing_if = "is_zero")]
    pub max_tasks: i64,
    // ActivationLimit line 44
    #[serde(rename = "max_activations")]
    #[serde(skip_serializing_if = "is_zero")]
    pub max_activations: i64,
    // Activations line 45
    #[serde(rename = "activations")]
    pub activations: i64,
    // ID line 46
    #[serde(rename = "id")]
    pub id: String,
    // Parent line 47
    #[serde(rename = "parent")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub parent: String,
    // Objective line 48
    #[serde(rename = "objective")]
    pub objective: String,
    // Requirements line 49
    #[serde(rename = "requirements")]
    pub requirements: Option<Vec<String>>,
    // Revision line 50
    #[serde(rename = "revision")]
    pub revision: i64,
    // Generation line 51
    #[serde(rename = "generation")]
    pub generation: i64,
    // Holder line 52
    #[serde(rename = "holder")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub holder: String,
    // Until line 53
    #[serde(rename = "lease_until")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub lease_until: String,
    // State line 54
    #[serde(rename = "state")]
    pub state: String,
}
pub(crate) fn shape_planning_scope() -> Fields {
    &[
        (
            "rejection_wait",
            Kind::Pointer(&Kind::Object(shape_planning_rejection_wait)),
        ),
        (
            "model_selection",
            Kind::Pointer(&Kind::Object(shape_role_model)),
        ),
        ("last_model", Kind::Pointer(&Kind::Object(shape_role_model))),
        (
            "delivery",
            Kind::Pointer(&Kind::Object(shape_planning_delivery)),
        ),
        (
            "workflow",
            Kind::Pointer(&Kind::Object(shape_agent_workflow)),
        ),
        ("max_tasks", Kind::Int64),
        ("max_activations", Kind::Int64),
        ("activations", Kind::Int64),
        ("id", Kind::String),
        ("parent", Kind::String),
        ("objective", Kind::String),
        ("requirements", Kind::Array(&Kind::String)),
        ("revision", Kind::Int64),
        ("generation", Kind::Int64),
        ("holder", Kind::String),
        ("lease_until", Kind::String),
        ("state", Kind::String),
    ]
}
// internal/engine/planning.go SHA-256 a0e508c1dfef64383383ed9e3fa3a8482e8fd6682f39735c815e2ac71de95b0e
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlanningState {
    // QuotaAuthorization line 14
    #[serde(rename = "quota_authorization")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota_authorization: Option<Box<QuotaAuthorization>>,
    // ReviewerRequired line 15
    #[serde(rename = "reviewer_required")]
    #[serde(skip_serializing_if = "is_false")]
    pub reviewer_required: bool,
    // Reviewer line 16
    #[serde(rename = "reviewer")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewer: Option<Box<ReviewerConfig>>,
    // ModelRoute line 17
    #[serde(rename = "model_route")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_route: Option<Box<ModelRoute>>,
    // HumanReviewAuthorized line 18
    #[serde(rename = "human_review_authorized")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub human_review_authorized: String,
    // Checks line 19
    #[serde(rename = "checks")]
    #[serde(skip_serializing_if = "empty_map")]
    pub checks: Option<BTreeMap<String, Option<Vec<ValidationControl>>>>,
    // Repository line 20
    #[serde(rename = "repository")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<Box<ManagedRepository>>,
    // Provider line 21
    #[serde(rename = "provider")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub provider: String,
    // ProviderDigest line 22
    #[serde(rename = "provider_digest")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub provider_digest: String,
    // Activations line 23
    #[serde(rename = "activations")]
    pub activations: i64,
    // MaxActivations line 24
    #[serde(rename = "max_activations")]
    pub max_activations: i64,
    // Failure line 25
    #[serde(rename = "failure")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub failure: String,
    // Version line 26
    #[serde(rename = "version")]
    pub version: i64,
    // Paused line 27
    #[serde(rename = "paused")]
    pub paused: bool,
    // MaxTasks line 28
    #[serde(rename = "max_tasks")]
    pub max_tasks: i64,
    // MaxScopes line 29
    #[serde(rename = "max_scopes")]
    #[serde(skip_serializing_if = "is_zero")]
    pub max_scopes: i64,
    // MaxDepth line 30
    #[serde(rename = "max_depth")]
    #[serde(skip_serializing_if = "is_zero")]
    pub max_depth: i64,
    // MaxDecisions line 31
    #[serde(rename = "max_decisions")]
    pub max_decisions: i64,
    // Decisions line 32
    #[serde(rename = "decisions")]
    pub decisions: i64,
    // Scopes line 33
    #[serde(rename = "scopes")]
    pub scopes: Option<Vec<PlanningScope>>,
    // Inbox line 34
    #[serde(rename = "inbox")]
    pub inbox: Option<Vec<PlanningEvent>>,
    // Judge line 35
    #[serde(rename = "judge")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub judge: Option<Box<JudgeState>>,
}
pub(crate) fn shape_planning_state() -> Fields {
    &[
        (
            "quota_authorization",
            Kind::Pointer(&Kind::Object(shape_quota_authorization)),
        ),
        ("reviewer_required", Kind::Bool),
        (
            "reviewer",
            Kind::Pointer(&Kind::Object(shape_reviewer_config)),
        ),
        (
            "model_route",
            Kind::Pointer(&Kind::Object(shape_model_route)),
        ),
        ("human_review_authorized", Kind::String),
        (
            "checks",
            Kind::Map(&Kind::Array(&Kind::Object(shape_validation_control))),
        ),
        (
            "repository",
            Kind::Pointer(&Kind::Object(shape_managed_repository)),
        ),
        ("provider", Kind::String),
        ("provider_digest", Kind::String),
        ("activations", Kind::Int64),
        ("max_activations", Kind::Int64),
        ("failure", Kind::String),
        ("version", Kind::Int64),
        ("paused", Kind::Bool),
        ("max_tasks", Kind::Int64),
        ("max_scopes", Kind::Int64),
        ("max_depth", Kind::Int64),
        ("max_decisions", Kind::Int64),
        ("decisions", Kind::Int64),
        ("scopes", Kind::Array(&Kind::Object(shape_planning_scope))),
        ("inbox", Kind::Array(&Kind::Object(shape_planning_event))),
        ("judge", Kind::Pointer(&Kind::Object(shape_judge_state))),
    ]
}
// internal/engine/product_plan.go SHA-256 61e4dba172381644c4b1177c68f027a13965ceecf1d98a64bcd2467f21f00f78
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProductPlan {
    // Mode line 12
    #[serde(rename = "mode")]
    pub mode: String,
    // Journeys line 13
    #[serde(rename = "journeys")]
    pub journeys: Option<Vec<UserJourney>>,
    // Stories line 14
    #[serde(rename = "stories")]
    pub stories: Option<Vec<UserStory>>,
}
pub(crate) fn shape_product_plan() -> Fields {
    &[
        ("mode", Kind::String),
        ("journeys", Kind::Array(&Kind::Object(shape_user_journey))),
        ("stories", Kind::Array(&Kind::Object(shape_user_story))),
    ]
}
// internal/engine/score.go SHA-256 449fd157d53cc656f103984a997ec8d11297c660df5aaf6f32f1a3e94f1df62d
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Progress {
    // Passed line 35
    #[serde(rename = "passed")]
    pub passed: i64,
    // Applicable line 36
    #[serde(rename = "applicable")]
    pub applicable: i64,
    // Percent line 37
    #[serde(rename = "percent")]
    pub percent: Option<Box<GoFloat>>,
}
pub(crate) fn shape_progress() -> Fields {
    &[
        ("passed", Kind::Int64),
        ("applicable", Kind::Int64),
        ("percent", Kind::Pointer(&Kind::Float64)),
    ]
}
// internal/engine/project_profile.go SHA-256 86347411459c49a028e478207259df84c8dda53a41eee32ac8fb6d16f9881f35
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectContext {
    // Name line 34
    #[serde(rename = "name")]
    pub name: String,
    // Role line 35
    #[serde(rename = "role")]
    pub role: String,
    // ProfileSHA256 line 36
    #[serde(rename = "profile_sha256")]
    pub profile_sha256: String,
    // Source line 37
    #[serde(rename = "source")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub source: String,
    // Sources line 38
    #[serde(rename = "sources")]
    pub sources: Option<Vec<ProjectSource>>,
    // SHA256 line 39
    #[serde(rename = "sha256")]
    pub sha256: String,
}
pub(crate) fn shape_project_context() -> Fields {
    &[
        ("name", Kind::String),
        ("role", Kind::String),
        ("profile_sha256", Kind::String),
        ("source", Kind::String),
        ("sources", Kind::Array(&Kind::Object(shape_project_source))),
        ("sha256", Kind::String),
    ]
}
// internal/engine/project_memory.go SHA-256 16aa64873f1a26f102a9c1e26d4021425bf063ea7ff1b58c7634a423ec77070d
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectMemory {
    // ID line 102
    #[serde(rename = "id")]
    pub id: String,
    // Imported line 103
    #[serde(rename = "imported")]
    #[serde(skip_serializing_if = "is_false")]
    pub imported: bool,
    // Versions line 104
    #[serde(rename = "versions")]
    pub versions: Option<Vec<MemoryVersion>>,
}
pub(crate) fn shape_project_memory() -> Fields {
    &[
        ("id", Kind::String),
        ("imported", Kind::Bool),
        ("versions", Kind::Array(&Kind::Object(shape_memory_version))),
    ]
}
// internal/engine/project_profile.go SHA-256 86347411459c49a028e478207259df84c8dda53a41eee32ac8fb6d16f9881f35
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectSource {
    // Path line 29
    #[serde(rename = "path")]
    pub path: String,
    // SHA256 line 30
    #[serde(rename = "sha256")]
    pub sha256: String,
    // Bytes line 31
    #[serde(rename = "bytes")]
    pub bytes: i64,
}
pub(crate) fn shape_project_source() -> Fields {
    &[
        ("path", Kind::String),
        ("sha256", Kind::String),
        ("bytes", Kind::Int64),
    ]
}
// internal/engine/provider_relay.go SHA-256 21a7733dcdbc225baa15b924b320d36b32ee5134434014e9acf71d51da945370
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProviderRelayDecision {
    // AgentID line 22
    #[serde(rename = "agent_id")]
    pub agent_id: String,
    // Action line 23
    #[serde(rename = "action")]
    pub action: String,
    // Provider line 24
    #[serde(rename = "provider")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub provider: String,
    // EventID line 25
    #[serde(rename = "event_id")]
    pub event_id: String,
    // Digest line 26
    #[serde(rename = "view_digest")]
    pub view_digest: String,
    // Actor line 27
    #[serde(rename = "actor")]
    pub actor: String,
    // At line 28
    #[serde(rename = "at")]
    pub at: String,
}
pub(crate) fn shape_provider_relay_decision() -> Fields {
    &[
        ("agent_id", Kind::String),
        ("action", Kind::String),
        ("provider", Kind::String),
        ("event_id", Kind::String),
        ("view_digest", Kind::String),
        ("actor", Kind::String),
        ("at", Kind::String),
    ]
}
// internal/engine/quotas.go SHA-256 91138131f7215dfe4678d65ee1b451cb566f52f47ec0da13ddc6b9260aac7077
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct QuotaAuthorization {
    // BeforeScopes line 26
    #[serde(rename = "before_scope_activations")]
    #[serde(skip_serializing_if = "empty_map")]
    pub before_scope_activations: Option<BTreeMap<String, i64>>,
    // AfterScopes line 27
    #[serde(rename = "after_scope_activations")]
    #[serde(skip_serializing_if = "empty_map")]
    pub after_scope_activations: Option<BTreeMap<String, i64>>,
    // Before line 28
    #[serde(rename = "before")]
    pub before: QuotaValues,
    // After line 29
    #[serde(rename = "after")]
    pub after: QuotaValues,
    // Actor line 30
    #[serde(rename = "actor")]
    pub actor: String,
    // At line 31
    #[serde(rename = "at")]
    pub at: String,
    // Reason line 32
    #[serde(rename = "reason")]
    pub reason: String,
}
pub(crate) fn shape_quota_authorization() -> Fields {
    &[
        ("before_scope_activations", Kind::Map(&Kind::Int64)),
        ("after_scope_activations", Kind::Map(&Kind::Int64)),
        ("before", Kind::Object(shape_quota_values)),
        ("after", Kind::Object(shape_quota_values)),
        ("actor", Kind::String),
        ("at", Kind::String),
        ("reason", Kind::String),
    ]
}
// internal/engine/quotas.go SHA-256 91138131f7215dfe4678d65ee1b451cb566f52f47ec0da13ddc6b9260aac7077
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct QuotaValues {
    // ReviewContextTokens line 12
    #[serde(rename = "review_context_tokens")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_context_tokens: Option<Box<i64>>,
    // ReviewContextFiles line 13
    #[serde(rename = "review_context_files")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_context_files: Option<Box<i64>>,
    // ReviewSourceBytes line 14
    #[serde(rename = "review_source_bytes")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_source_bytes: Option<Box<i64>>,
    // ReviewContextBytes line 15
    #[serde(rename = "review_context_bytes")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_context_bytes: Option<Box<i64>>,
    // ReviewFiles line 16
    #[serde(rename = "review_delivery_files")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_delivery_files: Option<Box<i64>>,
    // Scopes line 17
    #[serde(rename = "planning_scopes")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planning_scopes: Option<Box<i64>>,
    // Depth line 18
    #[serde(rename = "delegation_depth")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delegation_depth: Option<Box<i64>>,
    // Tasks line 19
    #[serde(rename = "planning_tasks")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planning_tasks: Option<Box<i64>>,
    // JudgeCycles line 20
    #[serde(rename = "judge_cycles")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub judge_cycles: Option<Box<i64>>,
    // Planning line 21
    #[serde(rename = "planning_activations")]
    pub planning_activations: i64,
    // Decisions line 22
    #[serde(rename = "planning_decisions")]
    pub planning_decisions: i64,
    // Reviews line 23
    #[serde(rename = "review_calls")]
    pub review_calls: Option<Box<i64>>,
}
pub(crate) fn shape_quota_values() -> Fields {
    &[
        ("review_context_tokens", Kind::Pointer(&Kind::Int64)),
        ("review_context_files", Kind::Pointer(&Kind::Int64)),
        ("review_source_bytes", Kind::Pointer(&Kind::Int64)),
        ("review_context_bytes", Kind::Pointer(&Kind::Int64)),
        ("review_delivery_files", Kind::Pointer(&Kind::Int64)),
        ("planning_scopes", Kind::Pointer(&Kind::Int64)),
        ("delegation_depth", Kind::Pointer(&Kind::Int64)),
        ("planning_tasks", Kind::Pointer(&Kind::Int64)),
        ("judge_cycles", Kind::Pointer(&Kind::Int64)),
        ("planning_activations", Kind::Int64),
        ("planning_decisions", Kind::Int64),
        ("review_calls", Kind::Pointer(&Kind::Int64)),
    ]
}
// internal/engine/recovered_result.go SHA-256 e584a63a06359e5e4f35f01317eb406d5d1c85396bc37d0bee182d10eb5aa088
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RecoveredResult {
    // ProofKey line 16
    #[serde(rename = "proof_key")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub proof_key: String,
    // ReplacesResult line 17
    #[serde(rename = "replaces_result")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub replaces_result: String,
    // PriorReview line 18
    #[serde(rename = "prior_review_id")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub prior_review_id: String,
    // Event line 19
    #[serde(rename = "event_id")]
    pub event_id: String,
    // RequestDigest line 20
    #[serde(rename = "request_sha256")]
    pub request_sha256: String,
    // Agent line 21
    #[serde(rename = "agent_id")]
    pub agent_id: String,
    // Attempt line 22
    #[serde(rename = "attempt_id")]
    pub attempt_id: String,
    // Result line 23
    #[serde(rename = "result_commit")]
    pub result_commit: String,
    // Tree line 24
    #[serde(rename = "tree")]
    pub tree: String,
    // Actor line 25
    #[serde(rename = "actor")]
    pub actor: String,
    // At line 26
    #[serde(rename = "at")]
    pub at: String,
    // Reason line 27
    #[serde(rename = "reason")]
    pub reason: String,
    // ProcessStatus line 28
    #[serde(rename = "original_process_status")]
    pub original_process_status: String,
}
pub(crate) fn shape_recovered_result() -> Fields {
    &[
        ("proof_key", Kind::String),
        ("replaces_result", Kind::String),
        ("prior_review_id", Kind::String),
        ("event_id", Kind::String),
        ("request_sha256", Kind::String),
        ("agent_id", Kind::String),
        ("attempt_id", Kind::String),
        ("result_commit", Kind::String),
        ("tree", Kind::String),
        ("actor", Kind::String),
        ("at", Kind::String),
        ("reason", Kind::String),
        ("original_process_status", Kind::String),
    ]
}
// internal/engine/retex_model.go SHA-256 ff4f37000ad5f3da5c29511a96378095ee3cc98c45c4699c0a45936d25600456
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Retex {
    // Impact line 4
    #[serde(rename = "impact")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub impact: String,
    // OpportunityKind line 5
    #[serde(rename = "opportunity_kind")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub opportunity_kind: String,
    // SourceKind line 6
    #[serde(rename = "source_kind")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub source_kind: String,
    // ID line 7
    #[serde(rename = "id")]
    pub id: String,
    // Source line 8
    #[serde(rename = "source")]
    pub source: DialogueRef,
    // Problem line 9
    #[serde(rename = "problem")]
    pub problem: String,
    // Nature line 10
    #[serde(rename = "nature")]
    pub nature: String,
    // Proposal line 11
    #[serde(rename = "proposal")]
    pub proposal: String,
    // Effort line 12
    #[serde(rename = "effort")]
    pub effort: String,
    // Risk line 13
    #[serde(rename = "risk")]
    pub risk: String,
    // Criteria line 14
    #[serde(rename = "criteria")]
    pub criteria: String,
    // Proof line 15
    #[serde(rename = "proof")]
    pub proof: String,
    // Status line 16
    #[serde(rename = "status")]
    pub status: String,
    // Task line 17
    #[serde(rename = "task")]
    pub task: String,
    // At line 18
    #[serde(rename = "at")]
    pub at: String,
    // ExportPending line 19
    #[serde(rename = "export_pending")]
    #[serde(skip_serializing_if = "is_false")]
    pub export_pending: bool,
    // ExportHash line 20
    #[serde(rename = "export_hash")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub export_hash: String,
    // ExportError line 21
    #[serde(rename = "export_error")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub export_error: String,
    // Lesson line 22
    #[serde(rename = "lesson")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub lesson: String,
}
pub(crate) fn shape_retex() -> Fields {
    &[
        ("impact", Kind::String),
        ("opportunity_kind", Kind::String),
        ("source_kind", Kind::String),
        ("id", Kind::String),
        ("source", Kind::Object(shape_dialogue_ref)),
        ("problem", Kind::String),
        ("nature", Kind::String),
        ("proposal", Kind::String),
        ("effort", Kind::String),
        ("risk", Kind::String),
        ("criteria", Kind::String),
        ("proof", Kind::String),
        ("status", Kind::String),
        ("task", Kind::String),
        ("at", Kind::String),
        ("export_pending", Kind::Bool),
        ("export_hash", Kind::String),
        ("export_error", Kind::String),
        ("lesson", Kind::String),
    ]
}
// internal/engine/model.go SHA-256 9b8f2f15d6af6c046cc3b7c909a4a8bf3155b9dd1e6c4ce04612758a763c9086
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Revalidation {
    // PreviousArtifacts line 33
    #[serde(rename = "previous_artifacts")]
    pub previous_artifacts: Option<BTreeMap<String, String>>,
    // Config line 34
    #[serde(rename = "config_digest")]
    pub config_digest: String,
    // Report line 35
    #[serde(rename = "report")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub report: String,
    // ReportHash line 36
    #[serde(rename = "report_hash")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub report_hash: String,
}
pub(crate) fn shape_revalidation() -> Fields {
    &[
        ("previous_artifacts", Kind::Map(&Kind::String)),
        ("config_digest", Kind::String),
        ("report", Kind::String),
        ("report_hash", Kind::String),
    ]
}
// internal/engine/managed_review_coordination.go SHA-256 000f14fba98ff32d8906e0411f260a03f543daa1e0e778488e8754fc308970f1
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReviewCoordinationLot {
    // ID line 14
    #[serde(rename = "id")]
    pub id: String,
    // Kind line 15
    #[serde(rename = "kind")]
    pub kind: String,
    // Objective line 16
    #[serde(rename = "objective")]
    pub objective: String,
    // Files line 17
    #[serde(rename = "files")]
    pub files: Option<Vec<String>>,
    // Criteria line 18
    #[serde(rename = "criteria")]
    pub criteria: Option<Vec<String>>,
    // Depends line 19
    #[serde(rename = "depends")]
    pub depends: Option<Vec<String>>,
}
pub(crate) fn shape_review_coordination_lot() -> Fields {
    &[
        ("id", Kind::String),
        ("kind", Kind::String),
        ("objective", Kind::String),
        ("files", Kind::Array(&Kind::String)),
        ("criteria", Kind::Array(&Kind::String)),
        ("depends", Kind::Array(&Kind::String)),
    ]
}
// internal/engine/managed_review_coordination.go SHA-256 000f14fba98ff32d8906e0411f260a03f543daa1e0e778488e8754fc308970f1
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReviewCoordinationProposal {
    // Candidate line 22
    #[serde(rename = "candidate_commit")]
    pub candidate_commit: String,
    // Evidence line 23
    #[serde(rename = "evidence_sha256")]
    pub evidence_sha256: String,
    // Lots line 24
    #[serde(rename = "lots")]
    pub lots: Option<Vec<ReviewCoordinationLot>>,
    // FinalReview line 25
    #[serde(rename = "final_review")]
    pub final_review: String,
}
pub(crate) fn shape_review_coordination_proposal() -> Fields {
    &[
        ("candidate_commit", Kind::String),
        ("evidence_sha256", Kind::String),
        (
            "lots",
            Kind::Array(&Kind::Object(shape_review_coordination_lot)),
        ),
        ("final_review", Kind::String),
    ]
}
// internal/engine/managed_review_coordination.go SHA-256 000f14fba98ff32d8906e0411f260a03f543daa1e0e778488e8754fc308970f1
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReviewCoordinationRecord {
    // Proposal line 28
    #[serde(rename = "proposal")]
    pub proposal: ReviewCoordinationProposal,
    // Digest line 29
    #[serde(rename = "sha256")]
    pub sha256: String,
    // Scope line 30
    #[serde(rename = "scope")]
    pub scope: String,
    // Decision line 31
    #[serde(rename = "decision")]
    pub decision: String,
    // State line 32
    #[serde(rename = "state")]
    pub state: String,
    // Order line 33
    #[serde(rename = "order")]
    pub order: Option<Vec<String>>,
}
pub(crate) fn shape_review_coordination_record() -> Fields {
    &[
        ("proposal", Kind::Object(shape_review_coordination_proposal)),
        ("sha256", Kind::String),
        ("scope", Kind::String),
        ("decision", Kind::String),
        ("state", Kind::String),
        ("order", Kind::Array(&Kind::String)),
    ]
}
// internal/engine/independent_review.go SHA-256 8aaa695b6bcc95e61684c0cbc099dd34be73cb18374a14c3a7ea8b2d604f20f5
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReviewCriterion {
    // Index line 113
    #[serde(rename = "index")]
    pub index: i64,
    // Verdict line 114
    #[serde(rename = "verdict")]
    pub verdict: String,
    // Evidence line 115
    #[serde(rename = "evidence")]
    pub evidence: String,
}
pub(crate) fn shape_review_criterion() -> Fields {
    &[
        ("index", Kind::Int64),
        ("verdict", Kind::String),
        ("evidence", Kind::String),
    ]
}
// internal/engine/managed_review_sources.go SHA-256 e4af44691e4d1d2da8b7792a231e2832203415367b0efc2e12d1dbfcf39136b8
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReviewSource {
    // Path line 18
    #[serde(rename = "path")]
    pub path: String,
    // Blob line 19
    #[serde(rename = "git_blob")]
    pub git_blob: String,
    // Digest line 20
    #[serde(rename = "sha256")]
    pub sha256: String,
    // Bytes line 21
    #[serde(rename = "bytes")]
    pub bytes: i64,
    // Content line 22
    #[serde(rename = "content")]
    pub content: String,
}
pub(crate) fn shape_review_source() -> Fields {
    &[
        ("path", Kind::String),
        ("git_blob", Kind::String),
        ("sha256", Kind::String),
        ("bytes", Kind::Int64),
        ("content", Kind::String),
    ]
}
// internal/engine/independent_review.go SHA-256 8aaa695b6bcc95e61684c0cbc099dd34be73cb18374a14c3a7ea8b2d604f20f5
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReviewerConfig {
    // InputContextTokens line 15
    #[serde(rename = "input_context_tokens")]
    #[serde(skip_serializing_if = "is_zero")]
    pub input_context_tokens: i64,
    // MaxContextFiles line 16
    #[serde(rename = "max_context_files")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_context_files: Option<Box<i64>>,
    // MaxSourceBytes line 17
    #[serde(rename = "max_source_bytes")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_source_bytes: Option<Box<i64>>,
    // MaxContextBytes line 18
    #[serde(rename = "max_context_bytes")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_context_bytes: Option<Box<i64>>,
    // MaxFiles line 19
    #[serde(rename = "max_files")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_files: Option<Box<i64>>,
    // ModelSelection line 20
    #[serde(rename = "model_selection")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_selection: Option<Box<RoleModel>>,
    // TimeoutSeconds line 21
    #[serde(rename = "timeout_seconds")]
    #[serde(skip_serializing_if = "is_zero")]
    pub timeout_seconds: i64,
    // Failure line 22
    #[serde(rename = "failure")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub failure: String,
    // Provider line 23
    #[serde(rename = "provider")]
    pub provider: String,
    // ProviderDigest line 24
    #[serde(rename = "provider_digest")]
    pub provider_digest: String,
    // ModelRoute line 25
    #[serde(rename = "model_route")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_route: Option<Box<ModelRoute>>,
    // MaxCalls line 26
    #[serde(rename = "max_calls")]
    pub max_calls: i64,
    // Calls line 27
    #[serde(rename = "calls")]
    pub calls: i64,
    // Authorized line 28
    #[serde(rename = "authorized")]
    pub authorized: String,
}
pub(crate) fn shape_reviewer_config() -> Fields {
    &[
        ("input_context_tokens", Kind::Int64),
        ("max_context_files", Kind::Pointer(&Kind::Int64)),
        ("max_source_bytes", Kind::Pointer(&Kind::Int64)),
        ("max_context_bytes", Kind::Pointer(&Kind::Int64)),
        ("max_files", Kind::Pointer(&Kind::Int64)),
        (
            "model_selection",
            Kind::Pointer(&Kind::Object(shape_role_model)),
        ),
        ("timeout_seconds", Kind::Int64),
        ("failure", Kind::String),
        ("provider", Kind::String),
        ("provider_digest", Kind::String),
        (
            "model_route",
            Kind::Pointer(&Kind::Object(shape_model_route)),
        ),
        ("max_calls", Kind::Int64),
        ("calls", Kind::Int64),
        ("authorized", Kind::String),
    ]
}
// internal/engine/role_model.go SHA-256 5ba48e5ebf640d7dc37d08f00544db18e57a58e6f6d8d24ca282489d4455a72d
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RoleModel {
    // Provider line 11
    #[serde(rename = "provider")]
    pub provider: String,
    // ProviderDigest line 12
    #[serde(rename = "provider_digest")]
    pub provider_digest: String,
    // Route line 13
    #[serde(rename = "route")]
    pub route: Option<Box<ModelRoute>>,
    // Actor line 14
    #[serde(rename = "actor")]
    pub actor: String,
    // At line 15
    #[serde(rename = "at")]
    pub at: String,
}
pub(crate) fn shape_role_model() -> Fields {
    &[
        ("provider", Kind::String),
        ("provider_digest", Kind::String),
        ("route", Kind::Pointer(&Kind::Object(shape_model_route))),
        ("actor", Kind::String),
        ("at", Kind::String),
    ]
}
// internal/engine/run_limits.go SHA-256 6bb48452ed56abe5984c4759ca2105aaf2b15098eaa2511a5de8c8a88daf9157
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RunLimits {
    // ObservationMode line 10
    #[serde(rename = "observation_mode")]
    #[serde(skip_serializing_if = "is_zero")]
    pub observation_mode: i64,
    // SilenceSeconds line 11
    #[serde(rename = "silence_seconds")]
    pub silence_seconds: i64,
    // ToolSeconds line 12
    #[serde(rename = "tool_seconds")]
    pub tool_seconds: i64,
    // MaxToolCalls line 13
    #[serde(rename = "max_tool_calls")]
    pub max_tool_calls: i64,
    // MaxRepeatedCalls line 14
    #[serde(rename = "max_repeated_calls")]
    pub max_repeated_calls: i64,
    // MaxConsecutiveErrors line 15
    #[serde(rename = "max_consecutive_errors")]
    pub max_consecutive_errors: i64,
}
pub(crate) fn shape_run_limits() -> Fields {
    &[
        ("observation_mode", Kind::Int64),
        ("silence_seconds", Kind::Int64),
        ("tool_seconds", Kind::Int64),
        ("max_tool_calls", Kind::Int64),
        ("max_repeated_calls", Kind::Int64),
        ("max_consecutive_errors", Kind::Int64),
    ]
}
// internal/engine/run_limits_admin.go SHA-256 dcb133d944ac0ed56283866a370208fc74f43f6be0924a5e1cfb5e8394c15ca4
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RunLimitsConfigEntry {
    // Scope line 36
    #[serde(rename = "scope")]
    pub scope: String,
    // MissionID line 37
    #[serde(rename = "mission_id")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub mission_id: String,
    // ScopeKey line 38
    #[serde(rename = "scope_key")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub scope_key: String,
    // Values line 39
    #[serde(rename = "values")]
    pub values: RunLimits,
    // Revision line 40
    #[serde(rename = "revision")]
    pub revision: i64,
    // Actor line 41
    #[serde(rename = "actor")]
    pub actor: String,
    // Reason line 42
    #[serde(rename = "reason")]
    pub reason: String,
    // Updated line 43
    #[serde(rename = "updated")]
    pub updated: String,
}
pub(crate) fn shape_run_limits_config_entry() -> Fields {
    &[
        ("scope", Kind::String),
        ("mission_id", Kind::String),
        ("scope_key", Kind::String),
        ("values", Kind::Object(shape_run_limits)),
        ("revision", Kind::Int64),
        ("actor", Kind::String),
        ("reason", Kind::String),
        ("updated", Kind::String),
    ]
}
// internal/engine/run_supervision.go SHA-256 c4b53f49951102f8fe9778ac3b8b8a8f1868ef63ba38b601bd8e07343b84c01f
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RunObservation {
    // Diagnostic line 13
    #[serde(rename = "diagnostic")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<Box<AttemptDiagnostic>>,
    // Agent line 14
    #[serde(rename = "agent_id")]
    pub agent_id: String,
    // Trigger line 15
    #[serde(rename = "trigger")]
    pub trigger: String,
    // ThresholdSeconds line 16
    #[serde(rename = "threshold_seconds")]
    pub threshold_seconds: i64,
    // Progress line 17
    #[serde(rename = "progress")]
    pub progress: AgentProgress,
}
pub(crate) fn shape_run_observation() -> Fields {
    &[
        (
            "diagnostic",
            Kind::Pointer(&Kind::Object(shape_attempt_diagnostic)),
        ),
        ("agent_id", Kind::String),
        ("trigger", Kind::String),
        ("threshold_seconds", Kind::Int64),
        ("progress", Kind::Object(shape_agent_progress)),
    ]
}
// internal/engine/dialogue_context.go SHA-256 510d596e9f20ee3e27c3791b3ebf1ef2c96cf2d3eb348907b5e76725f99bf079
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SavedContext {
    // Agent line 20
    #[serde(rename = "agent")]
    pub agent: String,
    // At line 21
    #[serde(rename = "at")]
    pub at: String,
    // Prompt line 22
    #[serde(rename = "prompt")]
    pub prompt: String,
    // Manifest line 23
    #[serde(rename = "manifest")]
    pub manifest: ContextManifest,
}
pub(crate) fn shape_saved_context() -> Fields {
    &[
        ("agent", Kind::String),
        ("at", Kind::String),
        ("prompt", Kind::String),
        ("manifest", Kind::Object(shape_context_manifest)),
    ]
}
// internal/engine/model.go SHA-256 9b8f2f15d6af6c046cc3b7c909a4a8bf3155b9dd1e6c4ce04612758a763c9086
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Task {
    // MemorySelections line 102
    #[serde(rename = "memory_selections")]
    #[serde(skip_serializing_if = "empty_vec")]
    pub memory_selections: Option<Vec<MemoryReference>>,
    // MemoryDecision line 103
    #[serde(rename = "memory_decision")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_decision: Option<Box<MemorySelectionDecision>>,
    // MemorySelectionImported line 104
    #[serde(rename = "memory_selection_imported")]
    #[serde(skip_serializing_if = "is_false")]
    pub memory_selection_imported: bool,
    // InstructionBase line 105
    #[serde(rename = "instruction_base")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instruction_base: Option<Box<String>>,
    // InstructionOperatorBase line 106
    #[serde(rename = "instruction_operator_base")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instruction_operator_base: Option<Box<String>>,
    // InstructionRevisions line 107
    #[serde(rename = "instruction_revisions")]
    #[serde(skip_serializing_if = "empty_vec")]
    pub instruction_revisions: Option<Vec<InstructionRevision>>,
    // ProviderRelayDecision line 108
    #[serde(rename = "provider_relay_decision")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_relay_decision: Option<Box<ProviderRelayDecision>>,
    // LegacyReportSubmissions line 109
    #[serde(rename = "legacy_report_submissions")]
    #[serde(skip_serializing_if = "empty_vec")]
    pub legacy_report_submissions: Option<Vec<Attempt>>,
    // ReviewCoordination line 110
    #[serde(rename = "review_coordination")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_coordination: Option<Box<ReviewCoordinationRecord>>,
    // Restarts line 111
    #[serde(rename = "restarts")]
    #[serde(skip_serializing_if = "empty_vec")]
    pub restarts: Option<Vec<TaskRestart>>,
    // ModelSelection line 112
    #[serde(rename = "model_selection")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_selection: Option<Box<TaskModel>>,
    // Requalifications line 113
    #[serde(rename = "requalifications")]
    #[serde(skip_serializing_if = "empty_vec")]
    pub requalifications: Option<Vec<HistoricalRequalification>>,
    // IntegrationRetries line 114
    #[serde(rename = "integration_retries")]
    #[serde(skip_serializing_if = "empty_vec")]
    pub integration_retries: Option<Vec<IntegrationRetry>>,
    // BatchReviewResume line 115
    #[serde(rename = "batch_review_resume")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_review_resume: Option<Box<IndependentReview>>,
    // RecoveredResult line 116
    #[serde(rename = "recovered_result")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovered_result: Option<Box<RecoveredResult>>,
    // CorrectiveRecovery line 117
    #[serde(rename = "corrective_recovery")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corrective_recovery: Option<Box<CorrectiveRecovery>>,
    // PreviousReviews line 118
    #[serde(rename = "previous_reviews")]
    #[serde(skip_serializing_if = "empty_vec")]
    pub previous_reviews: Option<Vec<IndependentReview>>,
    // IndependentReview line 119
    #[serde(rename = "independent_review")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub independent_review: Option<Box<IndependentReview>>,
    // ScopeID line 120
    #[serde(rename = "scope_id")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub scope_id: String,
    // Requirements line 121
    #[serde(rename = "requirements")]
    #[serde(skip_serializing_if = "empty_vec")]
    pub requirements: Option<Vec<String>>,
    // ValidationPolicy line 122
    #[serde(rename = "validation_policy")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_policy: Option<Box<ValidationPolicy>>,
    // AutoValidation line 123
    #[serde(rename = "automatic_validation")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automatic_validation: Option<Box<AutomaticValidation>>,
    // EvidenceStaleReason line 124
    #[serde(rename = "evidence_stale_reason")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub evidence_stale_reason: String,
    // LaunchHeld line 125
    #[serde(rename = "launch_held")]
    #[serde(skip_serializing_if = "is_false")]
    pub launch_held: bool,
    // Profile line 126
    #[serde(rename = "launch_profile")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launch_profile: Option<Box<LaunchProfile>>,
    // Revalidation line 127
    #[serde(rename = "revalidation")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revalidation: Option<Box<Revalidation>>,
    // PlanChecks line 128
    #[serde(rename = "plan_checks")]
    #[serde(skip_serializing_if = "empty_map")]
    pub plan_checks: Option<BTreeMap<String, String>>,
    // PlanBriefHash line 129
    #[serde(rename = "plan_brief_hash")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub plan_brief_hash: String,
    // PlanRole line 130
    #[serde(rename = "plan_role")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub plan_role: String,
    // PlanningRetry line 131
    #[serde(rename = "planning_retry")]
    #[serde(skip_serializing_if = "is_false")]
    pub planning_retry: bool,
    // PlanMaxAttempts line 132
    #[serde(rename = "plan_max_attempts")]
    #[serde(skip_serializing_if = "is_zero")]
    pub plan_max_attempts: i64,
    // PlanToolLimit line 133
    #[serde(rename = "plan_tool_limit")]
    #[serde(skip_serializing_if = "is_zero")]
    pub plan_tool_limit: i64,
    // Contexts line 134
    #[serde(rename = "contexts")]
    #[serde(skip_serializing_if = "empty_vec")]
    pub contexts: Option<Vec<SavedContext>>,
    // Answers line 135
    #[serde(rename = "answers")]
    #[serde(skip_serializing_if = "empty_vec")]
    pub answers: Option<Vec<BrainstormAnswer>>,
    // Question line 136
    #[serde(rename = "question")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub question: String,
    // Response line 137
    #[serde(rename = "response")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub response: String,
    // ResponseError line 138
    #[serde(rename = "response_error")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub response_error: String,
    // Brainstorm line 139
    #[serde(rename = "brainstorm")]
    #[serde(skip_serializing_if = "is_false")]
    pub brainstorm: bool,
    // Override line 140
    #[serde(rename = "manual_override")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_override: Option<Box<ManualOverride>>,
    // ID line 141
    #[serde(rename = "id")]
    pub id: String,
    // Title line 142
    #[serde(rename = "title")]
    pub title: String,
    // Owner line 143
    #[serde(rename = "owner")]
    pub owner: String,
    // Deliverable line 144
    #[serde(rename = "deliverable")]
    pub deliverable: String,
    // Criteria line 145
    #[serde(rename = "criteria")]
    pub criteria: Option<Vec<String>>,
    // Depends line 146
    #[serde(rename = "depends")]
    pub depends: Option<Vec<String>>,
    // Status line 147
    #[serde(rename = "status")]
    pub status: String,
    // Blocker line 148
    #[serde(rename = "blocker")]
    pub blocker: String,
    // Next line 149
    #[serde(rename = "next")]
    pub next: String,
    // Attempts line 150
    #[serde(rename = "attempts")]
    pub attempts: Option<Vec<Attempt>>,
    // Gate line 151
    #[serde(rename = "gate")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate: Option<Box<GateRecord>>,
}
pub(crate) fn shape_task() -> Fields {
    &[
        (
            "memory_selections",
            Kind::Array(&Kind::Object(shape_memory_reference)),
        ),
        (
            "memory_decision",
            Kind::Pointer(&Kind::Object(shape_memory_selection_decision)),
        ),
        ("memory_selection_imported", Kind::Bool),
        ("instruction_base", Kind::Pointer(&Kind::String)),
        ("instruction_operator_base", Kind::Pointer(&Kind::String)),
        (
            "instruction_revisions",
            Kind::Array(&Kind::Object(shape_instruction_revision)),
        ),
        (
            "provider_relay_decision",
            Kind::Pointer(&Kind::Object(shape_provider_relay_decision)),
        ),
        (
            "legacy_report_submissions",
            Kind::Array(&Kind::Object(shape_attempt)),
        ),
        (
            "review_coordination",
            Kind::Pointer(&Kind::Object(shape_review_coordination_record)),
        ),
        ("restarts", Kind::Array(&Kind::Object(shape_task_restart))),
        (
            "model_selection",
            Kind::Pointer(&Kind::Object(shape_task_model)),
        ),
        (
            "requalifications",
            Kind::Array(&Kind::Object(shape_historical_requalification)),
        ),
        (
            "integration_retries",
            Kind::Array(&Kind::Object(shape_integration_retry)),
        ),
        (
            "batch_review_resume",
            Kind::Pointer(&Kind::Object(shape_independent_review)),
        ),
        (
            "recovered_result",
            Kind::Pointer(&Kind::Object(shape_recovered_result)),
        ),
        (
            "corrective_recovery",
            Kind::Pointer(&Kind::Object(shape_corrective_recovery)),
        ),
        (
            "previous_reviews",
            Kind::Array(&Kind::Object(shape_independent_review)),
        ),
        (
            "independent_review",
            Kind::Pointer(&Kind::Object(shape_independent_review)),
        ),
        ("scope_id", Kind::String),
        ("requirements", Kind::Array(&Kind::String)),
        (
            "validation_policy",
            Kind::Pointer(&Kind::Object(shape_validation_policy)),
        ),
        (
            "automatic_validation",
            Kind::Pointer(&Kind::Object(shape_automatic_validation)),
        ),
        ("evidence_stale_reason", Kind::String),
        ("launch_held", Kind::Bool),
        (
            "launch_profile",
            Kind::Pointer(&Kind::Object(shape_launch_profile)),
        ),
        (
            "revalidation",
            Kind::Pointer(&Kind::Object(shape_revalidation)),
        ),
        ("plan_checks", Kind::Map(&Kind::String)),
        ("plan_brief_hash", Kind::String),
        ("plan_role", Kind::String),
        ("planning_retry", Kind::Bool),
        ("plan_max_attempts", Kind::Int64),
        ("plan_tool_limit", Kind::Int64),
        ("contexts", Kind::Array(&Kind::Object(shape_saved_context))),
        (
            "answers",
            Kind::Array(&Kind::Object(shape_brainstorm_answer)),
        ),
        ("question", Kind::String),
        ("response", Kind::String),
        ("response_error", Kind::String),
        ("brainstorm", Kind::Bool),
        (
            "manual_override",
            Kind::Pointer(&Kind::Object(shape_manual_override)),
        ),
        ("id", Kind::String),
        ("title", Kind::String),
        ("owner", Kind::String),
        ("deliverable", Kind::String),
        ("criteria", Kind::Array(&Kind::String)),
        ("depends", Kind::Array(&Kind::String)),
        ("status", Kind::String),
        ("blocker", Kind::String),
        ("next", Kind::String),
        ("attempts", Kind::Array(&Kind::Object(shape_attempt))),
        ("gate", Kind::Pointer(&Kind::Object(shape_gate_record))),
    ]
}
// internal/engine/task_model.go SHA-256 7ad0ed951d0f3a0611a2f57074cef3194f385ea94f1aba37788a09dcf6524055
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TaskModel {
    // Provider line 13
    #[serde(rename = "provider")]
    pub provider: String,
    // Route line 14
    #[serde(rename = "route")]
    pub route: ModelRoute,
    // Actor line 15
    #[serde(rename = "actor")]
    pub actor: String,
    // At line 16
    #[serde(rename = "at")]
    pub at: String,
}
pub(crate) fn shape_task_model() -> Fields {
    &[
        ("provider", Kind::String),
        ("route", Kind::Object(shape_model_route)),
        ("actor", Kind::String),
        ("at", Kind::String),
    ]
}
// internal/engine/task_restart.go SHA-256 9444aeb059eeeac5970a44f9cb529d81ae9ad18f8370588e3cf188e7fa29607d
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TaskRestart {
    // Event line 14
    #[serde(rename = "event_id")]
    pub event_id: String,
    // Attempt line 15
    #[serde(rename = "previous_attempt")]
    pub previous_attempt: String,
    // Candidate line 16
    #[serde(rename = "base_candidate")]
    pub base_candidate: String,
    // Reason line 17
    #[serde(rename = "reason")]
    pub reason: String,
    // Actor line 18
    #[serde(rename = "actor")]
    pub actor: String,
    // At line 19
    #[serde(rename = "at")]
    pub at: String,
    // PreviousBlocker line 20
    #[serde(rename = "previous_blocker")]
    pub previous_blocker: String,
    // Recovered line 21
    #[serde(rename = "previous_recovered_result")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_recovered_result: Option<Box<RecoveredResult>>,
}
pub(crate) fn shape_task_restart() -> Fields {
    &[
        ("event_id", Kind::String),
        ("previous_attempt", Kind::String),
        ("base_candidate", Kind::String),
        ("reason", Kind::String),
        ("actor", Kind::String),
        ("at", Kind::String),
        ("previous_blocker", Kind::String),
        (
            "previous_recovered_result",
            Kind::Pointer(&Kind::Object(shape_recovered_result)),
        ),
    ]
}
// internal/engine/budgets.go SHA-256 70a16ccf9de07c4070cfc329fcae0e56b3b5a2182d3985ff0008c1f7d8e124e0
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Usage {
    // CacheRead line 178
    #[serde(rename = "cache_read_input_tokens")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<Box<i64>>,
    // CacheCreation line 179
    #[serde(rename = "cache_creation_input_tokens")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<Box<i64>>,
    // CachedInput line 180
    #[serde(rename = "cached_input_tokens")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_input_tokens: Option<Box<i64>>,
    // ReportedCost line 181
    #[serde(rename = "provider_reported_cost_usd")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_reported_cost_usd: Option<Box<GoFloat>>,
    // Input line 182
    #[serde(rename = "input_tokens")]
    pub input_tokens: i64,
    // Output line 183
    #[serde(rename = "output_tokens")]
    pub output_tokens: i64,
    // Source line 184
    #[serde(rename = "source")]
    pub source: String,
    // Scope line 185
    #[serde(rename = "scope")]
    pub scope: String,
    // At line 186
    #[serde(rename = "at")]
    pub at: String,
}
pub(crate) fn shape_usage() -> Fields {
    &[
        ("cache_read_input_tokens", Kind::Pointer(&Kind::Int64)),
        ("cache_creation_input_tokens", Kind::Pointer(&Kind::Int64)),
        ("cached_input_tokens", Kind::Pointer(&Kind::Int64)),
        ("provider_reported_cost_usd", Kind::Pointer(&Kind::Float64)),
        ("input_tokens", Kind::Int64),
        ("output_tokens", Kind::Int64),
        ("source", Kind::String),
        ("scope", Kind::String),
        ("at", Kind::String),
    ]
}
// internal/engine/product_plan.go SHA-256 61e4dba172381644c4b1177c68f027a13965ceecf1d98a64bcd2467f21f00f78
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UserJourney {
    // ID line 18
    #[serde(rename = "id")]
    pub id: String,
    // Title line 19
    #[serde(rename = "title")]
    pub title: String,
    // Goal line 20
    #[serde(rename = "goal")]
    pub goal: String,
    // StoryIDs line 21
    #[serde(rename = "story_ids")]
    pub story_ids: Option<Vec<String>>,
}
pub(crate) fn shape_user_journey() -> Fields {
    &[
        ("id", Kind::String),
        ("title", Kind::String),
        ("goal", Kind::String),
        ("story_ids", Kind::Array(&Kind::String)),
    ]
}
// internal/engine/product_plan.go SHA-256 61e4dba172381644c4b1177c68f027a13965ceecf1d98a64bcd2467f21f00f78
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UserStory {
    // ID line 25
    #[serde(rename = "id")]
    pub id: String,
    // Title line 26
    #[serde(rename = "title")]
    pub title: String,
    // User line 27
    #[serde(rename = "user")]
    pub user: String,
    // Value line 28
    #[serde(rename = "value")]
    pub value: String,
    // Criteria line 29
    #[serde(rename = "criteria")]
    pub criteria: Option<Vec<String>>,
    // Complexity line 30
    #[serde(rename = "complexity")]
    pub complexity: i64,
    // Risk line 31
    #[serde(rename = "risk")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub risk: String,
    // Depends line 32
    #[serde(rename = "depends")]
    pub depends: Option<Vec<String>>,
    // TaskIDs line 33
    #[serde(rename = "task_ids")]
    pub task_ids: Option<Vec<String>>,
}
pub(crate) fn shape_user_story() -> Fields {
    &[
        ("id", Kind::String),
        ("title", Kind::String),
        ("user", Kind::String),
        ("value", Kind::String),
        ("criteria", Kind::Array(&Kind::String)),
        ("complexity", Kind::Int64),
        ("risk", Kind::String),
        ("depends", Kind::Array(&Kind::String)),
        ("task_ids", Kind::Array(&Kind::String)),
    ]
}
pub(crate) fn shape_validation_control() -> Fields {
    &[
        ("environment_exit_codes", Kind::Array(&Kind::Int64)),
        ("review_output", Kind::Bool),
        ("inputs", Kind::Array(&Kind::String)),
        ("id", Kind::String),
        ("command", Kind::Array(&Kind::String)),
        ("criteria", Kind::Array(&Kind::Int64)),
        ("justification", Kind::String),
        ("dir", Kind::String),
        ("timeout_seconds", Kind::Int64),
    ]
}
// internal/engine/model.go SHA-256 9b8f2f15d6af6c046cc3b7c909a4a8bf3155b9dd1e6c4ce04612758a763c9086
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ValidationControlResult {
    // CommandRuntimeLimits line 61
    #[serde(rename = "command_runtime_limits")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_runtime_limits: Option<Box<RunLimitsConfigEntry>>,
    // RuntimeLimits line 62
    #[serde(rename = "runtime_limits")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_limits: Option<Box<RunLimitsConfigEntry>>,
    // EnvironmentFailure line 63
    #[serde(rename = "environment_failure")]
    #[serde(skip_serializing_if = "is_false")]
    pub environment_failure: bool,
    // ReviewOutput line 64
    #[serde(rename = "review_output")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub review_output: String,
    // OutputBytes line 65
    #[serde(rename = "output_bytes")]
    #[serde(skip_serializing_if = "is_zero")]
    pub output_bytes: i64,
    // ReviewOutputTruncated line 66
    #[serde(rename = "review_output_truncated")]
    #[serde(skip_serializing_if = "is_false")]
    pub review_output_truncated: bool,
    // ID line 67
    #[serde(rename = "id")]
    pub id: String,
    // Command line 68
    #[serde(rename = "command")]
    #[serde(skip_serializing_if = "empty_vec")]
    pub command: Option<Vec<String>>,
    // Executed line 69
    #[serde(rename = "executed")]
    pub executed: bool,
    // Passed line 70
    #[serde(rename = "passed")]
    pub passed: bool,
    // ExitCode line 71
    #[serde(rename = "exit_code")]
    pub exit_code: i64,
    // OutputHash line 72
    #[serde(rename = "output_sha256")]
    pub output_sha256: String,
    // Summary line 73
    #[serde(rename = "summary")]
    pub summary: String,
    // Started line 74
    #[serde(rename = "started_at")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub started_at: String,
    // Finished line 75
    #[serde(rename = "finished_at")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub finished_at: String,
    // WallDurationMS line 76
    #[serde(rename = "wall_duration_ms")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wall_duration_ms: Option<Box<i64>>,
    // CPUDurationMS line 77
    #[serde(rename = "cpu_duration_ms")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_duration_ms: Option<Box<i64>>,
}
pub(crate) fn shape_validation_control_result() -> Fields {
    &[
        (
            "command_runtime_limits",
            Kind::Pointer(&Kind::Object(shape_run_limits_config_entry)),
        ),
        (
            "runtime_limits",
            Kind::Pointer(&Kind::Object(shape_run_limits_config_entry)),
        ),
        ("environment_failure", Kind::Bool),
        ("review_output", Kind::String),
        ("output_bytes", Kind::Int64),
        ("review_output_truncated", Kind::Bool),
        ("id", Kind::String),
        ("command", Kind::Array(&Kind::String)),
        ("executed", Kind::Bool),
        ("passed", Kind::Bool),
        ("exit_code", Kind::Int64),
        ("output_sha256", Kind::String),
        ("summary", Kind::String),
        ("started_at", Kind::String),
        ("finished_at", Kind::String),
        ("wall_duration_ms", Kind::Pointer(&Kind::Int64)),
        ("cpu_duration_ms", Kind::Pointer(&Kind::Int64)),
    ]
}
pub(crate) fn shape_validation_policy() -> Fields {
    &[
        ("mode", Kind::String),
        (
            "controls",
            Kind::Array(&Kind::Object(shape_validation_control)),
        ),
        ("authorized_at", Kind::String),
        ("actor", Kind::String),
    ]
}
// internal/engine/model.go SHA-256 9b8f2f15d6af6c046cc3b7c909a4a8bf3155b9dd1e6c4ce04612758a763c9086
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Work {
    // RequirementPrerequisites line 207
    #[serde(rename = "requirement_prerequisites")]
    #[serde(skip_serializing_if = "empty_map")]
    pub requirement_prerequisites: Option<BTreeMap<String, Option<Vec<String>>>>,
    // Planning line 208
    #[serde(rename = "planning")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planning: Option<Box<PlanningState>>,
    // Profile line 209
    #[serde(rename = "launch_profile")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launch_profile: Option<Box<LaunchProfile>>,
    // Plans line 210
    #[serde(rename = "plans")]
    #[serde(skip_serializing_if = "empty_vec")]
    pub plans: Option<Vec<ApprovedPlan>>,
    // Retex line 211
    #[serde(rename = "retex")]
    #[serde(skip_serializing_if = "empty_vec")]
    pub retex: Option<Vec<Retex>>,
    // ProjectMemory line 212
    #[serde(rename = "project_memory")]
    #[serde(skip_serializing_if = "empty_vec")]
    pub project_memory: Option<Vec<ProjectMemory>>,
    // PlanningBrief line 213
    #[serde(rename = "planning_brief")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planning_brief: Option<Box<PlanningBrief>>,
    // Schema line 214
    #[serde(rename = "schema_version")]
    pub schema_version: i64,
    // ID line 215
    #[serde(rename = "id")]
    pub id: String,
    // Revision line 216
    #[serde(rename = "revision")]
    pub revision: i64,
    // Title line 217
    #[serde(rename = "title")]
    pub title: String,
    // Objective line 218
    #[serde(rename = "objective")]
    pub objective: String,
    // Scope line 219
    #[serde(rename = "scope")]
    pub scope: String,
    // Criteria line 220
    #[serde(rename = "criteria")]
    pub criteria: Option<Vec<String>>,
    // Next line 221
    #[serde(rename = "next")]
    pub next: String,
    // Summary line 222
    #[serde(rename = "summary")]
    pub summary: String,
    // Memory line 223
    #[serde(rename = "memory")]
    pub memory: Option<Vec<String>>,
    // Created line 224
    #[serde(rename = "created")]
    pub created: String,
    // Updated line 225
    #[serde(rename = "updated")]
    pub updated: String,
    // Git line 226
    #[serde(rename = "git")]
    pub git: GitState,
    // Tasks line 227
    #[serde(rename = "tasks")]
    pub tasks: Option<Vec<Task>>,
}
pub(crate) fn shape_work() -> Fields {
    &[
        (
            "requirement_prerequisites",
            Kind::Map(&Kind::Array(&Kind::String)),
        ),
        (
            "planning",
            Kind::Pointer(&Kind::Object(shape_planning_state)),
        ),
        (
            "launch_profile",
            Kind::Pointer(&Kind::Object(shape_launch_profile)),
        ),
        ("plans", Kind::Array(&Kind::Object(shape_approved_plan))),
        ("retex", Kind::Array(&Kind::Object(shape_retex))),
        (
            "project_memory",
            Kind::Array(&Kind::Object(shape_project_memory)),
        ),
        (
            "planning_brief",
            Kind::Pointer(&Kind::Object(shape_planning_brief)),
        ),
        ("schema_version", Kind::Int64),
        ("id", Kind::String),
        ("revision", Kind::Int64),
        ("title", Kind::String),
        ("objective", Kind::String),
        ("scope", Kind::String),
        ("criteria", Kind::Array(&Kind::String)),
        ("next", Kind::String),
        ("summary", Kind::String),
        ("memory", Kind::Array(&Kind::String)),
        ("created", Kind::String),
        ("updated", Kind::String),
        ("git", Kind::Object(shape_git_state)),
        ("tasks", Kind::Array(&Kind::Object(shape_task))),
    ]
}
// internal/engine/managed_review_citation_feedback.go SHA-256 350b8c70e19c52663cd835ac2e3ca7e07f8031776ba0a443e3e0c216b862db61
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ManagedReviewCitationDiagnostic {
    // Version line 34
    #[serde(rename = "version")]
    pub version: i64,
    // Review line 35
    #[serde(rename = "review")]
    pub review: String,
    // Candidate line 36
    #[serde(rename = "candidate_commit")]
    pub candidate_commit: String,
    // CallID line 37
    #[serde(rename = "call_id")]
    pub call_id: String,
    // ReplyDigest line 38
    #[serde(rename = "reply_sha256")]
    pub reply_sha256: String,
    // Issues line 39
    #[serde(rename = "issues")]
    pub issues: Option<Vec<ManagedReviewCitationIssue>>,
}
pub(crate) fn shape_managed_review_citation_diagnostic() -> Fields {
    &[
        ("version", Kind::Int64),
        ("review", Kind::String),
        ("candidate_commit", Kind::String),
        ("call_id", Kind::String),
        ("reply_sha256", Kind::String),
        (
            "issues",
            Kind::Array(&Kind::Object(shape_managed_review_citation_issue)),
        ),
    ]
}
// internal/engine/managed_review_citation_feedback.go SHA-256 350b8c70e19c52663cd835ac2e3ca7e07f8031776ba0a443e3e0c216b862db61
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ManagedReviewCitationIssue {
    // Packet line 26
    #[serde(rename = "packet")]
    pub packet: i64,
    // Artifact line 27
    #[serde(rename = "artifact")]
    pub artifact: i64,
    // Need line 28
    #[serde(rename = "need_index")]
    pub need_index: i64,
    // Quote line 29
    #[serde(rename = "rejected_quote")]
    pub rejected_quote: String,
    // QuoteDigest line 30
    #[serde(rename = "quote_sha256")]
    pub quote_sha256: String,
    // Originals line 31
    #[serde(rename = "originals")]
    pub originals: Option<Vec<ManagedReviewCitationOriginal>>,
}
pub(crate) fn shape_managed_review_citation_issue() -> Fields {
    &[
        ("packet", Kind::Int64),
        ("artifact", Kind::Int64),
        ("need_index", Kind::Int64),
        ("rejected_quote", Kind::String),
        ("quote_sha256", Kind::String),
        (
            "originals",
            Kind::Array(&Kind::Object(shape_managed_review_citation_original)),
        ),
    ]
}
// internal/engine/managed_review_citation_feedback.go SHA-256 350b8c70e19c52663cd835ac2e3ca7e07f8031776ba0a443e3e0c216b862db61
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ManagedReviewCitationOriginal {
    // Name line 18
    #[serde(rename = "name")]
    pub name: String,
    // Digest line 19
    #[serde(rename = "sha256")]
    pub sha256: String,
    // Line line 20
    #[serde(rename = "line")]
    pub line: i64,
    // Offset line 21
    #[serde(rename = "byte_offset")]
    pub byte_offset: i64,
    // Text line 22
    #[serde(rename = "original_text")]
    pub original_text: String,
    // Truncated line 23
    #[serde(rename = "truncated")]
    #[serde(skip_serializing_if = "is_false")]
    pub truncated: bool,
}
pub(crate) fn shape_managed_review_citation_original() -> Fields {
    &[
        ("name", Kind::String),
        ("sha256", Kind::String),
        ("line", Kind::Int64),
        ("byte_offset", Kind::Int64),
        ("original_text", Kind::String),
        ("truncated", Kind::Bool),
    ]
}
// internal/engine/review_failure_routing.go SHA-256 8fc0398e804a835b392fce4d3219c594e6e70604766789fa0e540fb637d71bf9
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReviewFailureRouting {
    // ResultEvent line 51
    #[serde(rename = "result_event")]
    pub result_event: String,
    // Review line 52
    #[serde(rename = "review_id")]
    pub review_id: String,
    // Attempt line 53
    #[serde(rename = "attempt")]
    pub attempt: String,
    // Candidate line 54
    #[serde(rename = "candidate_commit")]
    pub candidate_commit: String,
    // Code line 55
    #[serde(rename = "failure_code")]
    pub failure_code: String,
    // Owner line 56
    #[serde(rename = "owner")]
    pub owner: String,
    // Operation line 57
    #[serde(rename = "operation")]
    pub operation: String,
    // ProducerCorrection line 58
    #[serde(rename = "producer_correction_required")]
    pub producer_correction_required: bool,
}
pub(crate) fn shape_review_failure_routing() -> Fields {
    &[
        ("result_event", Kind::String),
        ("review_id", Kind::String),
        ("attempt", Kind::String),
        ("candidate_commit", Kind::String),
        ("failure_code", Kind::String),
        ("owner", Kind::String),
        ("operation", Kind::String),
        ("producer_correction_required", Kind::Bool),
    ]
}
#[cfg(feature = "test-fixtures")]
pub fn fixture_model_roundtrip(name: &str, raw: &[u8]) -> Result<Vec<u8>, super::WorkError> {
    match name {
        "ActionPlan" => super::work::decode::<ActionPlan>(raw, shape_action_plan())
            .and_then(|v| super::encode_model(&v)),
        "ActionSkill" => super::work::decode::<ActionSkill>(raw, shape_action_skill())
            .and_then(|v| super::encode_model(&v)),
        "ActionSkillSelection" => {
            super::work::decode::<ActionSkillSelection>(raw, shape_action_skill_selection())
                .and_then(|v| super::encode_model(&v))
        }
        "AgentProgress" => super::work::decode::<AgentProgress>(raw, shape_agent_progress())
            .and_then(|v| super::encode_model(&v)),
        "AgentWorkflow" => super::work::decode::<AgentWorkflow>(raw, shape_agent_workflow())
            .and_then(|v| super::encode_model(&v)),
        "ApprovedPlan" => super::work::decode::<ApprovedPlan>(raw, shape_approved_plan())
            .and_then(|v| super::encode_model(&v)),
        "Attempt" => super::work::decode::<Attempt>(raw, shape_attempt())
            .and_then(|v| super::encode_model(&v)),
        "AttemptDiagnostic" => {
            super::work::decode::<AttemptDiagnostic>(raw, shape_attempt_diagnostic())
                .and_then(|v| super::encode_model(&v))
        }
        "AutomaticValidation" => {
            super::work::decode::<AutomaticValidation>(raw, shape_automatic_validation())
                .and_then(|v| super::encode_model(&v))
        }
        "BrainstormAnswer" => {
            super::work::decode::<BrainstormAnswer>(raw, shape_brainstorm_answer())
                .and_then(|v| super::encode_model(&v))
        }
        "ContextManifest" => super::work::decode::<ContextManifest>(raw, shape_context_manifest())
            .and_then(|v| super::encode_model(&v)),
        "CorrectiveRecovery" => {
            super::work::decode::<CorrectiveRecovery>(raw, shape_corrective_recovery())
                .and_then(|v| super::encode_model(&v))
        }
        "DiagnosticItem" => super::work::decode::<DiagnosticItem>(raw, shape_diagnostic_item())
            .and_then(|v| super::encode_model(&v)),
        "DialogueRef" => super::work::decode::<DialogueRef>(raw, shape_dialogue_ref())
            .and_then(|v| super::encode_model(&v)),
        "Evaluation" => super::work::decode::<Evaluation>(raw, shape_evaluation())
            .and_then(|v| super::encode_model(&v)),
        "Event" => {
            super::work::decode::<Event>(raw, shape_event()).and_then(|v| super::encode_model(&v))
        }
        "ExchangeArtifact" => {
            super::work::decode::<ExchangeArtifact>(raw, shape_exchange_artifact())
                .and_then(|v| super::encode_model(&v))
        }
        "GateRecord" => super::work::decode::<GateRecord>(raw, shape_gate_record())
            .and_then(|v| super::encode_model(&v)),
        "GitState" => super::work::decode::<GitState>(raw, shape_git_state())
            .and_then(|v| super::encode_model(&v)),
        "HistoricalRequalification" => super::work::decode::<HistoricalRequalification>(
            raw,
            shape_historical_requalification(),
        )
        .and_then(|v| super::encode_model(&v)),
        "IndependentReview" => {
            super::work::decode::<IndependentReview>(raw, shape_independent_review())
                .and_then(|v| super::encode_model(&v))
        }
        "InstructionRevision" => {
            super::work::decode::<InstructionRevision>(raw, shape_instruction_revision())
                .and_then(|v| super::encode_model(&v))
        }
        "IntegrationRetry" => {
            super::work::decode::<IntegrationRetry>(raw, shape_integration_retry())
                .and_then(|v| super::encode_model(&v))
        }
        "JudgeCycleRecord" => {
            super::work::decode::<JudgeCycleRecord>(raw, shape_judge_cycle_record())
                .and_then(|v| super::encode_model(&v))
        }
        "JudgeState" => super::work::decode::<JudgeState>(raw, shape_judge_state())
            .and_then(|v| super::encode_model(&v)),
        "LaunchProfile" => super::work::decode::<LaunchProfile>(raw, shape_launch_profile())
            .and_then(|v| super::encode_model(&v)),
        "ManagedFragmentJournalAnchor" => super::work::decode::<ManagedFragmentJournalAnchor>(
            raw,
            shape_managed_fragment_journal_anchor(),
        )
        .and_then(|v| super::encode_model(&v)),
        "ManagedRepository" => {
            super::work::decode::<ManagedRepository>(raw, shape_managed_repository())
                .and_then(|v| super::encode_model(&v))
        }
        "ManagedReviewBatchVerdict" => super::work::decode::<ManagedReviewBatchVerdict>(
            raw,
            shape_managed_review_batch_verdict(),
        )
        .and_then(|v| super::encode_model(&v)),
        "ManagedReviewRetryDirective" => super::work::decode::<ManagedReviewRetryDirective>(
            raw,
            shape_managed_review_retry_directive(),
        )
        .and_then(|v| super::encode_model(&v)),
        "ManagedTaskReview" => {
            super::work::decode::<ManagedTaskReview>(raw, shape_managed_task_review())
                .and_then(|v| super::encode_model(&v))
        }
        "ManualOverride" => super::work::decode::<ManualOverride>(raw, shape_manual_override())
            .and_then(|v| super::encode_model(&v)),
        "MemoryReference" => super::work::decode::<MemoryReference>(raw, shape_memory_reference())
            .and_then(|v| super::encode_model(&v)),
        "MemorySelectionDecision" => {
            super::work::decode::<MemorySelectionDecision>(raw, shape_memory_selection_decision())
                .and_then(|v| super::encode_model(&v))
        }
        "MemorySummaryProtection" => {
            super::work::decode::<MemorySummaryProtection>(raw, shape_memory_summary_protection())
                .and_then(|v| super::encode_model(&v))
        }
        "MemorySummaryReceipt" => {
            super::work::decode::<MemorySummaryReceipt>(raw, shape_memory_summary_receipt())
                .and_then(|v| super::encode_model(&v))
        }
        "MemorySummarySelection" => {
            super::work::decode::<MemorySummarySelection>(raw, shape_memory_summary_selection())
                .and_then(|v| super::encode_model(&v))
        }
        "MemoryVersion" => super::work::decode::<MemoryVersion>(raw, shape_memory_version())
            .and_then(|v| super::encode_model(&v)),
        "ModelRoute" => super::work::decode::<ModelRoute>(raw, shape_model_route())
            .and_then(|v| super::encode_model(&v)),
        "PlanMission" => super::work::decode::<PlanMission>(raw, shape_plan_mission())
            .and_then(|v| super::encode_model(&v)),
        "PlanQuestion" => super::work::decode::<PlanQuestion>(raw, shape_plan_question())
            .and_then(|v| super::encode_model(&v)),
        "PlanningBrief" => super::work::decode::<PlanningBrief>(raw, shape_planning_brief())
            .and_then(|v| super::encode_model(&v)),
        "PlanningDelivery" => {
            super::work::decode::<PlanningDelivery>(raw, shape_planning_delivery())
                .and_then(|v| super::encode_model(&v))
        }
        "PlanningEvent" => super::work::decode::<PlanningEvent>(raw, shape_planning_event())
            .and_then(|v| super::encode_model(&v)),
        "PlanningExploration" => {
            super::work::decode::<PlanningExploration>(raw, shape_planning_exploration())
                .and_then(|v| super::encode_model(&v))
        }
        "PlanningExplorationRequest" => super::work::decode::<PlanningExplorationRequest>(
            raw,
            shape_planning_exploration_request(),
        )
        .and_then(|v| super::encode_model(&v)),
        "PlanningRejectionWait" => {
            super::work::decode::<PlanningRejectionWait>(raw, shape_planning_rejection_wait())
                .and_then(|v| super::encode_model(&v))
        }
        "PlanningScope" => super::work::decode::<PlanningScope>(raw, shape_planning_scope())
            .and_then(|v| super::encode_model(&v)),
        "PlanningState" => super::work::decode::<PlanningState>(raw, shape_planning_state())
            .and_then(|v| super::encode_model(&v)),
        "ProductPlan" => super::work::decode::<ProductPlan>(raw, shape_product_plan())
            .and_then(|v| super::encode_model(&v)),
        "Progress" => super::work::decode::<Progress>(raw, shape_progress())
            .and_then(|v| super::encode_model(&v)),
        "ProjectContext" => super::work::decode::<ProjectContext>(raw, shape_project_context())
            .and_then(|v| super::encode_model(&v)),
        "ProjectMemory" => super::work::decode::<ProjectMemory>(raw, shape_project_memory())
            .and_then(|v| super::encode_model(&v)),
        "ProjectSource" => super::work::decode::<ProjectSource>(raw, shape_project_source())
            .and_then(|v| super::encode_model(&v)),
        "ProviderRelayDecision" => {
            super::work::decode::<ProviderRelayDecision>(raw, shape_provider_relay_decision())
                .and_then(|v| super::encode_model(&v))
        }
        "QuotaAuthorization" => {
            super::work::decode::<QuotaAuthorization>(raw, shape_quota_authorization())
                .and_then(|v| super::encode_model(&v))
        }
        "QuotaValues" => super::work::decode::<QuotaValues>(raw, shape_quota_values())
            .and_then(|v| super::encode_model(&v)),
        "RecoveredResult" => super::work::decode::<RecoveredResult>(raw, shape_recovered_result())
            .and_then(|v| super::encode_model(&v)),
        "Retex" => {
            super::work::decode::<Retex>(raw, shape_retex()).and_then(|v| super::encode_model(&v))
        }
        "Revalidation" => super::work::decode::<Revalidation>(raw, shape_revalidation())
            .and_then(|v| super::encode_model(&v)),
        "ReviewCoordinationLot" => {
            super::work::decode::<ReviewCoordinationLot>(raw, shape_review_coordination_lot())
                .and_then(|v| super::encode_model(&v))
        }
        "ReviewCoordinationProposal" => super::work::decode::<ReviewCoordinationProposal>(
            raw,
            shape_review_coordination_proposal(),
        )
        .and_then(|v| super::encode_model(&v)),
        "ReviewCoordinationRecord" => {
            super::work::decode::<ReviewCoordinationRecord>(raw, shape_review_coordination_record())
                .and_then(|v| super::encode_model(&v))
        }
        "ReviewCriterion" => super::work::decode::<ReviewCriterion>(raw, shape_review_criterion())
            .and_then(|v| super::encode_model(&v)),
        "ReviewSource" => super::work::decode::<ReviewSource>(raw, shape_review_source())
            .and_then(|v| super::encode_model(&v)),
        "ReviewerConfig" => super::work::decode::<ReviewerConfig>(raw, shape_reviewer_config())
            .and_then(|v| super::encode_model(&v)),
        "RoleModel" => super::work::decode::<RoleModel>(raw, shape_role_model())
            .and_then(|v| super::encode_model(&v)),
        "RunLimits" => super::work::decode::<RunLimits>(raw, shape_run_limits())
            .and_then(|v| super::encode_model(&v)),
        "RunLimitsConfigEntry" => {
            super::work::decode::<RunLimitsConfigEntry>(raw, shape_run_limits_config_entry())
                .and_then(|v| super::encode_model(&v))
        }
        "RunObservation" => super::work::decode::<RunObservation>(raw, shape_run_observation())
            .and_then(|v| super::encode_model(&v)),
        "SavedContext" => super::work::decode::<SavedContext>(raw, shape_saved_context())
            .and_then(|v| super::encode_model(&v)),
        "Task" => {
            super::work::decode::<Task>(raw, shape_task()).and_then(|v| super::encode_model(&v))
        }
        "TaskModel" => super::work::decode::<TaskModel>(raw, shape_task_model())
            .and_then(|v| super::encode_model(&v)),
        "TaskRestart" => super::work::decode::<TaskRestart>(raw, shape_task_restart())
            .and_then(|v| super::encode_model(&v)),
        "Usage" => {
            super::work::decode::<Usage>(raw, shape_usage()).and_then(|v| super::encode_model(&v))
        }
        "UserJourney" => super::work::decode::<UserJourney>(raw, shape_user_journey())
            .and_then(|v| super::encode_model(&v)),
        "UserStory" => super::work::decode::<UserStory>(raw, shape_user_story())
            .and_then(|v| super::encode_model(&v)),
        "ValidationControl" => {
            super::work::decode::<ValidationControl>(raw, shape_validation_control())
                .and_then(|v| super::encode_model(&v))
        }
        "ValidationControlResult" => {
            super::work::decode::<ValidationControlResult>(raw, shape_validation_control_result())
                .and_then(|v| super::encode_model(&v))
        }
        "ValidationPolicy" => {
            super::work::decode::<ValidationPolicy>(raw, shape_validation_policy())
                .and_then(|v| super::encode_model(&v))
        }
        "Work" => {
            super::work::decode::<Work>(raw, shape_work()).and_then(|v| super::encode_model(&v))
        }
        "managedReviewCitationDiagnostic" => {
            super::work::decode::<ManagedReviewCitationDiagnostic>(
                raw,
                shape_managed_review_citation_diagnostic(),
            )
            .and_then(|v| super::encode_model(&v))
        }
        "managedReviewCitationIssue" => super::work::decode::<ManagedReviewCitationIssue>(
            raw,
            shape_managed_review_citation_issue(),
        )
        .and_then(|v| super::encode_model(&v)),
        "managedReviewCitationOriginal" => super::work::decode::<ManagedReviewCitationOriginal>(
            raw,
            shape_managed_review_citation_original(),
        )
        .and_then(|v| super::encode_model(&v)),
        "reviewFailureRouting" => {
            super::work::decode::<ReviewFailureRouting>(raw, shape_review_failure_routing())
                .and_then(|v| super::encode_model(&v))
        }
        _ => Err(super::WorkError::Projection(
            super::model_wire::Error::Invalid("unknown fixture model"),
        )),
    }
}
