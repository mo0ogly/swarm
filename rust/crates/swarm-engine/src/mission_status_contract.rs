//! Historical Go mission status output types, with no aggregate implementation.
//!
//! Source: Git commit 553593a2ffa3823d16784a6fedc291eebf390837, engine Go files
//! listed beside each declaration. These are supplied observations, not public
//! mutation requests, validated permissions or a replacement for NativeStatus.
//! Constructors/default projections are deliberately absent: callers must obtain
//! every field from its actual producer. No Root/SQL/process/provider/clock IO,
//! freshness check, fallback, reservation, acceptance or retry occurs here.
//!
//! Go slices use `Option<Vec<T>>` to preserve nil/null versus an explicit empty
//! array. Omitempty slices omit both; pointer absence is distinct from Some(false)
//! or Some(0). Go int/int64 use i64 for the Linux64 reference, uint64 remains u64.
//! Floating cost/usage reuses GoFloat serialization: nonfinite values fail rather
//! than becoming null. Strings/timestamps and producer-owned enum text are copied
//! unchanged; these types do not validate their business meaning or sanitize data.
//! No raw Debug implementation is derived for these potentially private records.
//!
//! Review of this increment: serde omission and integer widths are protocol
//! representation, not operator policies. No limits, timeouts or policy defaults
//! are introduced. Existing typed serializer errors propagate to the caller;
//! Root release/output/flush handling remains the aggregate/CLI owner's duty.
//! Targeted tests below cover wire shape only, not real producer evidence,
//! matching Go binary provenance, runtime parity or independent acceptance.
use serde::Serialize;

/// Existing exact Go wire types, reused without alternate cost/measurement models.
/// Their normal serialization preserves nullable usage and finite cost errors;
/// no observation, decoding or validation is invoked by these re-exports.
pub use crate::{AgentProgress, AttemptDiagnostic, CostTotal, ProviderCooldown, Usage};

/// Go omitempty slice representation only: absence and empty both omit. This
/// performs no IO or policy selection and never fabricates a successful result.
fn empty_slice<T>(value: &Option<Vec<T>>) -> bool {
    value.as_ref().is_none_or(Vec::is_empty)
}

// HEAD:internal/engine/mission_status.go SHA256 552552072eb73e5ca1d479d833256af86876033330292de77eca256147e62f9c
/// One task presentation, separating attempt/process facts from validation.
/// Requires producer-classified result, waits, recovery and actions; plain stored
/// Task/Agent state is insufficient. Optional diagnostics remain absent when nil.
/// Wire declaration: `HEAD:internal/engine/mission_status.go:11`.
#[derive(Clone, PartialEq, Serialize)]
pub struct MissionTask {
    /// Go `Waits` (mission_status.go:12); supplied data, not inferred.
    #[serde(rename = "waiting_on")]
    pub waits: Option<Vec<MissionWait>>,
    /// Go `RecoveryPreview` (mission_status.go:13); supplied data, not inferred.
    pub recovery_preview: RecoveryPreview,
    /// Go `Primary` (mission_status.go:14); supplied data, not inferred.
    #[serde(rename = "primary_action")]
    pub primary: MissionPrimaryAction,
    /// Go `AttemptsUsed` (mission_status.go:15); supplied data, not inferred.
    pub attempts_used: i64,
    /// Go `AttemptsAllowed` (mission_status.go:16); supplied data, not inferred.
    pub attempts_allowed: i64,
    /// Go `AttemptLimitReached` (mission_status.go:17); supplied data, not inferred.
    pub attempt_limit_reached: bool,
    /// Go `ID` (mission_status.go:18); supplied data, not inferred.
    pub id: String,
    /// Go `Title` (mission_status.go:19); supplied data, not inferred.
    pub title: String,
    /// Go `State` (mission_status.go:20); supplied data, not inferred.
    pub state: String,
    /// Go `Reason` (mission_status.go:21); supplied data, not inferred.
    pub reason: String,
    /// Go `Action` (mission_status.go:22); supplied data, not inferred.
    pub action: String,
    /// Go `Label` (mission_status.go:23); supplied data, not inferred.
    pub label: String,
    /// Go `Target` (mission_status.go:24); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub target: String,
    /// Go `Impact` (mission_status.go:25); supplied data, not inferred.
    pub impact: i64,
    /// Go `Deliverable` (mission_status.go:26); supplied data, not inferred.
    pub deliverable: String,
    /// Go `ValidationMode` (mission_status.go:27); supplied data, not inferred.
    pub validation_mode: String,
    /// Go `ValidationReceipt` (mission_status.go:28); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub validation_receipt: String,
    /// Go `Result` (mission_status.go:29); supplied data, not inferred.
    pub result: ResultPresentation,
    /// Go `Understanding` (mission_status.go:30); supplied data, not inferred.
    pub understanding: MissionUnderstanding,
    /// Go `Diagnostic` (mission_status.go:31); supplied data, not inferred.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<Box<AttemptDiagnostic>>,
    /// Go `Attempt` (mission_status.go:32); supplied data, not inferred.
    pub attempt: MissionAttemptProjection,
}

/// Producer-selected explanation, actor and next step; no decision is applied.
/// Wire declaration: `HEAD:internal/engine/mission_status.go:34`.
#[derive(Clone, PartialEq, Serialize)]
pub struct MissionUnderstanding {
    /// Go `What` (mission_status.go:35); supplied data, not inferred.
    pub what: String,
    /// Go `NextStep` (mission_status.go:36); supplied data, not inferred.
    pub next_step: String,
    /// Go `Actor` (mission_status.go:37); supplied data, not inferred.
    pub actor: String,
    /// Go `ActorKind` (mission_status.go:38); supplied data, not inferred.
    pub actor_kind: String,
    /// Go `Situation` (mission_status.go:39); supplied data, not inferred.
    pub situation: String,
}

/// One ordered coordination phase. Optional time/relative text is retained
/// only when supplied; this DTO never measures age or infers responsibility.
/// Wire declaration: `HEAD:internal/engine/mission_status.go:84`.
#[derive(Clone, PartialEq, Serialize)]
pub struct MissionCoordinationPhase {
    /// Go `Kind` (mission_status.go:85); supplied data, not inferred.
    pub kind: String,
    /// Go `Label` (mission_status.go:86); supplied data, not inferred.
    pub label: String,
    /// Go `Summary` (mission_status.go:87); supplied data, not inferred.
    pub summary: String,
    /// Go `Actor` (mission_status.go:88); supplied data, not inferred.
    pub actor: String,
    /// Go `NextStep` (mission_status.go:89); supplied data, not inferred.
    pub next_step: String,
    /// Go `At` (mission_status.go:90); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub at: String,
    /// Go `Relative` (mission_status.go:91); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub relative: String,
}

/// Complete historical mission status document, without a native schema wrapper.
/// Every field is supplied by its producer; serialization grants no launch or
/// acceptance and does not recompute counters, freshness, runtime or completion.
/// Wire declaration: `HEAD:internal/engine/mission_status.go:93`.
#[derive(Clone, PartialEq, Serialize)]
pub struct MissionStatus {
    /// Go `Changes` (mission_status.go:94); supplied data, not inferred.
    #[serde(rename = "changes_since_visit")]
    pub changes: MissionChanges,
    /// Go `Spending` (mission_status.go:95); supplied data, not inferred.
    pub spending: MissionSpending,
    /// Go `Guidance` (mission_status.go:96); supplied data, not inferred.
    pub guidance: MissionGuidance,
    /// Go `Runtime` (mission_status.go:97); supplied data, not inferred.
    pub runtime: RuntimeHealth,
    /// Go `ProviderCooldowns` (mission_status.go:98); supplied data, not inferred.
    #[serde(skip_serializing_if = "empty_slice")]
    pub provider_cooldowns: Option<Vec<ProviderCooldown>>,
    /// Go `EvidenceStage` (mission_status.go:99); supplied data, not inferred.
    pub evidence_stage: String,
    /// Go `Organization` (mission_status.go:100); supplied data, not inferred.
    pub organization: Organization,
    /// Go `Authorized` (mission_status.go:101); supplied data, not inferred.
    pub authorized: bool,
    /// Go `Enabled` (mission_status.go:102); supplied data, not inferred.
    pub enabled: bool,
    /// Go `Paused` (mission_status.go:103); supplied data, not inferred.
    pub paused: bool,
    /// Go `Summary` (mission_status.go:104); supplied data, not inferred.
    pub summary: String,
    /// Go `Next` (mission_status.go:105); supplied data, not inferred.
    pub next: String,
    /// Go `Validated` (mission_status.go:106); supplied data, not inferred.
    pub validated: i64,
    /// Go `Review` (mission_status.go:107); supplied data, not inferred.
    pub review: i64,
    /// Go `Running` (mission_status.go:108); supplied data, not inferred.
    pub running: i64,
    /// Go `ActiveAgents` (mission_status.go:109); supplied data, not inferred.
    pub active_agents: i64,
    /// Go `UncertainAgents` (mission_status.go:110); supplied data, not inferred.
    pub uncertain_agents: i64,
    /// Go `Total` (mission_status.go:111); supplied data, not inferred.
    pub total: i64,
    /// Go `Supervision` (mission_status.go:112); supplied data, not inferred.
    pub supervision: MissionSupervision,
    /// Go `Understanding` (mission_status.go:113); supplied data, not inferred.
    pub understanding: MissionUnderstanding,
    /// Go `Coordination` (mission_status.go:114); supplied data, not inferred.
    pub coordination: Option<Vec<MissionCoordinationPhase>>,
    /// Go `Tasks` (mission_status.go:115); supplied data, not inferred.
    pub tasks: Option<Vec<MissionTask>>,
}

// HEAD:internal/engine/mission_insights.go SHA256 41e4ab16b113fcac6a68838ca205337af3bfd2c9835510b02652aea6635d1d3e
/// One recorded prerequisite presentation; no wait, dependency check or scheduling.
/// Wire declaration: `HEAD:internal/engine/mission_insights.go:16`.
#[derive(Clone, PartialEq, Serialize)]
pub struct MissionWait {
    /// Go `Task` (mission_insights.go:17); supplied data, not inferred.
    pub task: String,
    /// Go `Title` (mission_insights.go:18); supplied data, not inferred.
    pub title: String,
    /// Go `State` (mission_insights.go:19); supplied data, not inferred.
    pub state: String,
    /// Go `Reason` (mission_insights.go:20); supplied data, not inferred.
    pub reason: String,
}

/// Producer-observed recovery facts, never approval to retry or reuse evidence.
/// Absent dialogue/refusal/evidence remain absent; nil required slices remain null.
/// Wire declaration: `HEAD:internal/engine/mission_insights.go:22`.
#[derive(Clone, PartialEq, Serialize)]
pub struct RecoveryPreview {
    /// Go `Dialogue` (mission_insights.go:23); supplied data, not inferred.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dialogue: Option<Box<DialogueRecoveryPreview>>,
    /// Go `Work` (mission_insights.go:24); supplied data, not inferred.
    pub work: String,
    /// Go `Task` (mission_insights.go:25); supplied data, not inferred.
    pub task: String,
    /// Go `Revision` (mission_insights.go:26); supplied data, not inferred.
    pub revision: i64,
    /// Go `Agent` (mission_insights.go:27); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub agent: String,
    /// Go `Kept` (mission_insights.go:28); supplied data, not inferred.
    pub kept: Option<Vec<String>>,
    /// Go `Redone` (mission_insights.go:29); supplied data, not inferred.
    pub redone: Option<Vec<String>>,
    /// Go `Correction` (mission_insights.go:30); supplied data, not inferred.
    pub correction: String,
    /// Go `Criteria` (mission_insights.go:31); supplied data, not inferred.
    pub criteria: Option<Vec<String>>,
    /// Go `Limits` (mission_insights.go:32); supplied data, not inferred.
    pub limits: String,
    /// Go `SinceRefusal` (mission_insights.go:33); supplied data, not inferred.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since_refusal: Option<Box<MissionChanges>>,
    /// Go `Evidence` (mission_insights.go:34); supplied data, not inferred.
    #[serde(skip_serializing_if = "empty_slice")]
    pub evidence: Option<Vec<RecoveryEvidence>>,
    /// Go `Remaining` (mission_insights.go:35); supplied data, not inferred.
    #[serde(rename = "remaining_criteria")]
    pub remaining: Option<Vec<String>>,
    /// Go `RefusalNote` (mission_insights.go:36); supplied data, not inferred.
    pub refusal_note: String,
    /// Go `EvidenceNote` (mission_insights.go:37); supplied data, not inferred.
    pub evidence_note: String,
}

/// One report input-comparison observation, not a favorable review verdict.
/// Wire declaration: `HEAD:internal/engine/mission_insights.go:43`.
#[derive(Clone, PartialEq, Serialize)]
pub struct RecoveryEvidence {
    /// Go `Report` (mission_insights.go:44); supplied data, not inferred.
    pub report: String,
    /// Go `State` (mission_insights.go:45); supplied data, not inferred.
    pub state: String,
    /// Go `Reason` (mission_insights.go:46); supplied data, not inferred.
    pub reason: String,
}

/// One producer-classified historical change; task/title omit only empty text.
/// Wire declaration: `HEAD:internal/engine/mission_insights.go:48`.
#[derive(Clone, PartialEq, Serialize)]
pub struct MissionChange {
    /// Go `Revision` (mission_insights.go:49); supplied data, not inferred.
    pub revision: i64,
    /// Go `At` (mission_insights.go:50); supplied data, not inferred.
    pub at: String,
    /// Go `Category` (mission_insights.go:51); supplied data, not inferred.
    pub category: String,
    /// Go `Label` (mission_insights.go:52); supplied data, not inferred.
    pub label: String,
    /// Go `Task` (mission_insights.go:53); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub task: String,
    /// Go `Title` (mission_insights.go:54); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub title: String,
}

/// Changes relative to the supplied operator visit and recorded revision.
/// No visit is read/marked and no window or category is chosen by this DTO.
/// Wire declaration: `HEAD:internal/engine/mission_insights.go:56`.
#[derive(Clone, PartialEq, Serialize)]
pub struct MissionChanges {
    /// Go `From` (mission_insights.go:57); supplied data, not inferred.
    #[serde(rename = "from_revision")]
    pub from: i64,
    /// Go `To` (mission_insights.go:58); supplied data, not inferred.
    #[serde(rename = "to_revision")]
    pub to: i64,
    /// Go `First` (mission_insights.go:59); supplied data, not inferred.
    #[serde(rename = "first_visit")]
    pub first: bool,
    /// Go `More` (mission_insights.go:60); supplied data, not inferred.
    pub more: bool,
    /// Go `Items` (mission_insights.go:61); supplied data, not inferred.
    pub items: Option<Vec<MissionChange>>,
}

/// One measured spending group. Nullable process duration is not zero;
/// incomplete usage/tools/duration counters remain explicit observations.
/// Wire declaration: `HEAD:internal/engine/mission_insights.go:63`.
#[derive(Clone, PartialEq, Serialize)]
pub struct SpendingRow {
    /// Go `Kind` (mission_insights.go:64); supplied data, not inferred.
    pub kind: String,
    /// Go `ID` (mission_insights.go:65); supplied data, not inferred.
    pub id: String,
    /// Go `Label` (mission_insights.go:66); supplied data, not inferred.
    pub label: String,
    /// Go `Calls` (mission_insights.go:67); supplied data, not inferred.
    #[serde(rename = "recorded_calls")]
    pub calls: i64,
    /// Go `Tools` (mission_insights.go:68); supplied data, not inferred.
    #[serde(rename = "observed_tool_calls")]
    pub tools: i64,
    /// Go `UnknownTools` (mission_insights.go:69); supplied data, not inferred.
    #[serde(rename = "attempts_with_incomplete_tool_measurement")]
    pub unknown_tools: i64,
    /// Go `Input` (mission_insights.go:70); supplied data, not inferred.
    #[serde(rename = "input_tokens")]
    pub input: i64,
    /// Go `Output` (mission_insights.go:71); supplied data, not inferred.
    #[serde(rename = "output_tokens")]
    pub output: i64,
    /// Go `MissingUsage` (mission_insights.go:72); supplied data, not inferred.
    #[serde(rename = "calls_without_usage")]
    pub missing_usage: i64,
    /// Go `Cost` (mission_insights.go:73); supplied data, not inferred.
    pub cost: CostTotal,
    /// Go `Reviews` (mission_insights.go:77); supplied data, not inferred.
    #[serde(rename = "reviews_recorded")]
    pub reviews: i64,
    /// Go `TaskRetries` (mission_insights.go:78); supplied data, not inferred.
    pub task_retries: i64,
    /// Go `DurationMS` (mission_insights.go:79); supplied data, not inferred.
    #[serde(rename = "recorded_process_ms")]
    pub duration_ms: Option<Box<i64>>,
    /// Go `MissingDurations` (mission_insights.go:80); supplied data, not inferred.
    #[serde(rename = "attempts_without_duration")]
    pub missing_durations: i64,
}

/// Complete supplied call/row/attempt ledger presentation, not an invoice.
/// Nil and empty arrays remain distinct; missing measurements are never estimated.
/// Wire declaration: `HEAD:internal/engine/mission_insights.go:82`.
#[derive(Clone, PartialEq, Serialize)]
pub struct MissionSpending {
    /// Go `Calls` (mission_insights.go:83); supplied data, not inferred.
    pub calls: Option<Vec<SpendingCall>>,
    /// Go `ObservedAt` (mission_insights.go:84); supplied data, not inferred.
    pub observed_at: String,
    /// Go `Source` (mission_insights.go:85); supplied data, not inferred.
    #[serde(rename = "measurement_source")]
    pub source: String,
    /// Go `Rows` (mission_insights.go:86); supplied data, not inferred.
    pub rows: Option<Vec<SpendingRow>>,
    /// Go `Controls` (mission_insights.go:87); supplied data, not inferred.
    #[serde(rename = "recorded_control_executions")]
    pub controls: i64,
    /// Go `Retries` (mission_insights.go:88); supplied data, not inferred.
    #[serde(rename = "worker_retries")]
    pub retries: i64,
    /// Go `Note` (mission_insights.go:89); supplied data, not inferred.
    pub note: String,
    /// Go `Attempts` (mission_insights.go:90); supplied data, not inferred.
    pub attempts: Option<Vec<AttemptLedger>>,
}

/// One recorded attempt ledger, keeping process state separate from task acceptance.
/// Nullable duration and missing-usage flags do not manufacture measured zero.
/// Wire declaration: `HEAD:internal/engine/mission_insights.go:98`.
#[derive(Clone, PartialEq, Serialize)]
pub struct AttemptLedger {
    /// Go `Task` (mission_insights.go:99); supplied data, not inferred.
    pub task: String,
    /// Go `Label` (mission_insights.go:100); supplied data, not inferred.
    pub label: String,
    /// Go `Agent` (mission_insights.go:101); supplied data, not inferred.
    pub agent: String,
    /// Go `Attempt` (mission_insights.go:102); supplied data, not inferred.
    pub attempt: String,
    /// Go `Role` (mission_insights.go:103); supplied data, not inferred.
    pub role: String,
    /// Go `RequestedModel` (mission_insights.go:104); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub requested_model: String,
    /// Go `ObservedModel` (mission_insights.go:105); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub observed_model: String,
    /// Go `ProcessState` (mission_insights.go:106); supplied data, not inferred.
    pub process_state: String,
    /// Go `Accepted` (mission_insights.go:107); supplied data, not inferred.
    #[serde(rename = "task_acceptance_recorded")]
    pub accepted: bool,
    /// Go `Measurement` (mission_insights.go:108); supplied data, not inferred.
    pub measurement: String,
    /// Go `Calls` (mission_insights.go:109); supplied data, not inferred.
    #[serde(rename = "recorded_calls")]
    pub calls: i64,
    /// Go `Tools` (mission_insights.go:110); supplied data, not inferred.
    #[serde(rename = "observed_tool_calls")]
    pub tools: i64,
    /// Go `Measured` (mission_insights.go:111); supplied data, not inferred.
    pub measured: bool,
    /// Go `Reads` (mission_insights.go:112); supplied data, not inferred.
    #[serde(rename = "tool_reads")]
    pub reads: i64,
    /// Go `Writes` (mission_insights.go:113); supplied data, not inferred.
    #[serde(rename = "tool_writes")]
    pub writes: i64,
    /// Go `Unclassified` (mission_insights.go:114); supplied data, not inferred.
    #[serde(rename = "tool_unclassified")]
    pub unclassified: i64,
    /// Go `Tests` (mission_insights.go:115); supplied data, not inferred.
    pub tests: String,
    /// Go `Errors` (mission_insights.go:116); supplied data, not inferred.
    #[serde(rename = "tool_errors_total")]
    pub errors: i64,
    /// Go `Repeats` (mission_insights.go:117); supplied data, not inferred.
    #[serde(rename = "tool_repeats_total")]
    pub repeats: i64,
    /// Go `Degraded` (mission_insights.go:118); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub degraded: String,
    /// Go `Input` (mission_insights.go:119); supplied data, not inferred.
    #[serde(rename = "input_tokens")]
    pub input: i64,
    /// Go `Output` (mission_insights.go:120); supplied data, not inferred.
    #[serde(rename = "output_tokens")]
    pub output: i64,
    /// Go `MissingUsage` (mission_insights.go:121); supplied data, not inferred.
    #[serde(rename = "usage_missing")]
    pub missing_usage: bool,
    /// Go `Cost` (mission_insights.go:122); supplied data, not inferred.
    pub cost: CostTotal,
    /// Go `Started` (mission_insights.go:126); supplied data, not inferred.
    pub started: String,
    /// Go `Ended` (mission_insights.go:127); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub ended: String,
    /// Go `DurationMS` (mission_insights.go:128); supplied data, not inferred.
    #[serde(rename = "recorded_process_ms")]
    pub duration_ms: Option<Box<i64>>,
}

// HEAD:internal/engine/mission_guidance.go SHA256 69895d5bab15756c47957f1fc7a2cf5d93893391683ec00fb5cdbec07596db33
/// Read-only producer-selected mission guidance and primary action; no grant.
/// Wire declaration: `HEAD:internal/engine/mission_guidance.go:6`.
#[derive(Clone, PartialEq, Serialize)]
pub struct MissionGuidance {
    /// Go `What` (mission_guidance.go:7); supplied data, not inferred.
    pub what: String,
    /// Go `Next` (mission_guidance.go:8); supplied data, not inferred.
    pub next: String,
    /// Go `Actor` (mission_guidance.go:9); supplied data, not inferred.
    pub actor: String,
    /// Go `Primary` (mission_guidance.go:10); supplied data, not inferred.
    pub primary: MissionPrimaryAction,
}

/// Suggested action presentation; displaying it applies no mutation or permission.
/// Wire declaration: `HEAD:internal/engine/mission_guidance.go:12`.
#[derive(Clone, PartialEq, Serialize)]
pub struct MissionPrimaryAction {
    /// Go `Kind` (mission_guidance.go:13); supplied data, not inferred.
    pub kind: String,
    /// Go `Label` (mission_guidance.go:14); supplied data, not inferred.
    pub label: String,
    /// Go `Effect` (mission_guidance.go:15); supplied data, not inferred.
    pub effect: String,
    /// Go `Task` (mission_guidance.go:16); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub task: String,
    /// Go `Tone` (mission_guidance.go:17); supplied data, not inferred.
    pub tone: String,
}

// HEAD:internal/engine/mission_supervision.go SHA256 a078550f9d75ce86e570ef87600a7d345f87f66862939233bda61aa45baed952
/// Retained conductor action and producer-rendered age, not live process proof.
/// Wire declaration: `HEAD:internal/engine/mission_supervision.go:39`.
#[derive(Clone, PartialEq, Serialize)]
pub struct MissionSupervisionAction {
    /// Go `At` (mission_supervision.go:40); supplied data, not inferred.
    pub at: String,
    /// Go `Relative` (mission_supervision.go:41); supplied data, not inferred.
    pub relative: String,
    /// Go `Actor` (mission_supervision.go:42); supplied data, not inferred.
    pub actor: String,
    /// Go `Kind` (mission_supervision.go:43); supplied data, not inferred.
    pub kind: String,
    /// Go `Summary` (mission_supervision.go:44); supplied data, not inferred.
    pub summary: String,
}

/// Historical Go supervision projection, distinct from native lease observations.
/// The producer owes clock/policy/last-action classification; this type performs
/// none and must not relabel a native Present/LegacyUnverified state automatically.
/// Wire declaration: `HEAD:internal/engine/mission_supervision.go:47`.
#[derive(Clone, PartialEq, Serialize)]
pub struct MissionSupervision {
    /// Go `State` (mission_supervision.go:48); supplied data, not inferred.
    pub state: String,
    /// Go `Source` (mission_supervision.go:49); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub source: String,
    /// Go `ObservedAt` (mission_supervision.go:50); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub observed_at: String,
    /// Go `LastCheckAt` (mission_supervision.go:51); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub last_check_at: String,
    /// Go `LastCheckRelative` (mission_supervision.go:52); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub last_check_relative: String,
    /// Go `ReconciledAt` (mission_supervision.go:53); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub reconciled_at: String,
    /// Go `LastError` (mission_supervision.go:54); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub last_error: String,
    /// Go `NextCheckAt` (mission_supervision.go:55); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub next_check_at: String,
    /// Go `NextCheckRelative` (mission_supervision.go:56); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub next_check_relative: String,
    /// Go `Late` (mission_supervision.go:57); supplied data, not inferred.
    pub late: bool,
    /// Go `ClockIssue` (mission_supervision.go:58); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub clock_issue: String,
    /// Go `LastAction` (mission_supervision.go:59); supplied data, not inferred.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_action: Option<Box<MissionSupervisionAction>>,
}

// HEAD:internal/engine/organization.go SHA256 b1bda71fbf5f9caff756e9e236cd068412f450555ea45e8207316b45fcc8e701
/// Factual organization presentation, not structural validation or authority.
/// Issues retain nil/empty meaning; Ready must be supplied by the actual producer.
/// Wire declaration: `HEAD:internal/engine/organization.go:11`.
#[derive(Clone, PartialEq, Serialize)]
pub struct Organization {
    /// Go `Ready` (organization.go:12); supplied data, not inferred.
    pub ready: bool,
    /// Go `Label` (organization.go:13); supplied data, not inferred.
    pub label: String,
    /// Go `Issues` (organization.go:14); supplied data, not inferred.
    pub issues: Option<Vec<String>>,
    /// Go `Next` (organization.go:15); supplied data, not inferred.
    pub next: String,
    /// Go `Verification` (organization.go:16); supplied data, not inferred.
    pub verification: String,
}

// HEAD:internal/engine/result_presentation.go SHA256 f60ec2bb6b6497d3848b3e0c51c22ccb5371dc26e40c2685f914678bf46cd243
/// Producer-classified process/report/validation presentation.
/// A process end or stored accepted flag alone does not establish these facts.
/// Wire declaration: `HEAD:internal/engine/result_presentation.go:11`.
#[derive(Clone, PartialEq, Serialize)]
pub struct ResultPresentation {
    /// Go `ProcessLabel` (result_presentation.go:12); supplied data, not inferred.
    pub process_label: String,
    /// Go `ReportLabel` (result_presentation.go:13); supplied data, not inferred.
    pub report_label: String,
    /// Go `ValidationLabel` (result_presentation.go:14); supplied data, not inferred.
    pub validation_label: String,
    /// Go `State` (result_presentation.go:15); supplied data, not inferred.
    pub state: String,
    /// Go `Label` (result_presentation.go:16); supplied data, not inferred.
    pub label: String,
    /// Go `ProcessState` (result_presentation.go:17); supplied data, not inferred.
    pub process_state: String,
    /// Go `ReportState` (result_presentation.go:18); supplied data, not inferred.
    pub report_state: String,
    /// Go `ValidationState` (result_presentation.go:19); supplied data, not inferred.
    pub validation_state: String,
    /// Go `Reason` (result_presentation.go:20); supplied data, not inferred.
    pub reason: String,
    /// Go `NextStep` (result_presentation.go:21); supplied data, not inferred.
    pub next_step: String,
    /// Go `AttemptID` (result_presentation.go:22); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub attempt_id: String,
    /// Go `ReportID` (result_presentation.go:23); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub report_id: String,
    /// Go `GateID` (result_presentation.go:24); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub gate_id: String,
    /// Go `ReceiptID` (result_presentation.go:25); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub receipt_id: String,
}

// HEAD:internal/engine/graph_draft_projection.go SHA256 38c5e1c245ca5ab559b00e629adadb283b65e66fffd79d8274cf904bdff6d9ae
/// Selected attempt/model/cost observations, not a fresh execution permit.
/// Wire declaration: `HEAD:internal/engine/graph_draft_projection.go:22`.
#[derive(Clone, PartialEq, Serialize)]
pub struct MissionAttemptProjection {
    /// Go `AgentID` (graph_draft_projection.go:23); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub agent_id: String,
    /// Go `AttemptID` (graph_draft_projection.go:24); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub attempt_id: String,
    /// Go `Role` (graph_draft_projection.go:25); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub role: String,
    /// Go `ProcessState` (graph_draft_projection.go:26); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub process_state: String,
    /// Go `Activity` (graph_draft_projection.go:27); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub activity: String,
    /// Go `Validation` (graph_draft_projection.go:28); supplied data, not inferred.
    #[serde(rename = "validation_state")]
    pub validation: String,
    /// Go `RequestedModel` (graph_draft_projection.go:29); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub requested_model: String,
    /// Go `ObservedModel` (graph_draft_projection.go:30); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub observed_model: String,
    /// Go `CostState` (graph_draft_projection.go:31); supplied data, not inferred.
    pub cost_state: String,
}

// HEAD:internal/engine/runtime_health.go SHA256 29c01aee5a08b905b8e2030c1e628b8e71611d0a41a87cbe9912b3229c536ba9
/// One physical volume observation with unsigned64 byte/inode counts.
/// No filesystem probe, threshold, availability fallback or reservation occurs.
/// Wire declaration: `HEAD:internal/engine/runtime_health.go:16`.
#[derive(Clone, PartialEq, Serialize)]
pub struct StorageVolume {
    /// Go `Kind` (runtime_health.go:17); supplied data, not inferred.
    pub kind: String,
    /// Go `Path` (runtime_health.go:18); supplied data, not inferred.
    pub path: String,
    /// Go `Available` (runtime_health.go:19); supplied data, not inferred.
    #[serde(rename = "available_bytes")]
    pub available: u64,
    /// Go `Inodes` (runtime_health.go:20); supplied data, not inferred.
    #[serde(rename = "available_inodes")]
    pub inodes: u64,
    /// Go `State` (runtime_health.go:21); supplied data, not inferred.
    pub state: String,
}

/// Supplied storage/version diagnosis, never computed from consent or a lease.
/// The producer must supply real observations and effective policy; serialization
/// neither probes the host nor establishes future launch authority.
/// Wire declaration: `HEAD:internal/engine/runtime_health.go:24`.
#[derive(Clone, PartialEq, Serialize)]
pub struct RuntimeHealth {
    /// Go `State` (runtime_health.go:25); supplied data, not inferred.
    pub state: String,
    /// Go `LaunchAllowed` (runtime_health.go:26); supplied data, not inferred.
    pub launch_allowed: bool,
    /// Go `ObservedAt` (runtime_health.go:27); supplied data, not inferred.
    pub observed_at: String,
    /// Go `Message` (runtime_health.go:28); supplied data, not inferred.
    pub message: String,
    /// Go `Next` (runtime_health.go:29); supplied data, not inferred.
    #[serde(rename = "next_step")]
    pub next: String,
    /// Go `Volumes` (runtime_health.go:30); supplied data, not inferred.
    pub volumes: Option<Vec<StorageVolume>>,
    /// Go `Version` (runtime_health.go:31); supplied data, not inferred.
    pub version: ServerVersion,
    /// Go `Diagnostic` (runtime_health.go:32); supplied data, not inferred.
    #[serde(rename = "version_diagnostic")]
    pub diagnostic: VersionDiagnostic,
}

/// Optional source comparison, retaining unknown commit/modified/match as null.
/// Wire declaration: `HEAD:internal/engine/runtime_health.go:35`.
#[derive(Clone, PartialEq, Serialize)]
pub struct SourceVersion {
    /// Go `Available` (runtime_health.go:36); supplied data, not inferred.
    pub available: bool,
    /// Go `Commit` (runtime_health.go:37); supplied data, not inferred.
    pub commit: Option<Box<String>>,
    /// Go `Modified` (runtime_health.go:38); supplied data, not inferred.
    pub modified: Option<Box<bool>>,
    /// Go `Compared` (runtime_health.go:39); supplied data, not inferred.
    pub compared: bool,
    /// Go `MatchesBinary` (runtime_health.go:40); supplied data, not inferred.
    pub matches_binary: Option<Box<bool>>,
}

/// Binary identity, optional source comparison and release history.
/// Go internal json:"-" compatibility fields are intentionally absent here.
/// Wire declaration: `HEAD:internal/engine/runtime_health.go:46`.
#[derive(Clone, PartialEq, Serialize)]
pub struct ServerVersion {
    /// Go `Binary` (runtime_health.go:47); supplied data, not inferred.
    pub binary: BinaryVersion,
    /// Go `Sources` (runtime_health.go:48); supplied data, not inferred.
    #[serde(rename = "source")]
    pub sources: SourceVersion,
    /// Go `History` (runtime_health.go:49); supplied data, not inferred.
    pub history: VersionHistory,
}

// HEAD:internal/engine/version.go SHA256 22f2ab4f4bd2ef0dfd0c7a1567874e9b356b24684d17aae8b7ef29c53a96567b
/// Identified binary metadata. Unknown optional metadata serializes as null;
/// no version/commit/build identity is invented from this type or current sources.
/// Wire declaration: `HEAD:internal/engine/version.go:26`.
#[derive(Clone, PartialEq, Serialize)]
pub struct BinaryVersion {
    /// Go `Version` (version.go:27); supplied data, not inferred.
    pub version: String,
    /// Go `Commit` (version.go:28); supplied data, not inferred.
    pub commit: Option<Box<String>>,
    /// Go `Modified` (version.go:29); supplied data, not inferred.
    pub modified: Option<Box<bool>>,
    /// Go `BuildDate` (version.go:30); supplied data, not inferred.
    pub build_date: Option<Box<String>>,
    /// Go `Provenance` (version.go:31); supplied data, not inferred.
    pub provenance: String,
}

// HEAD:internal/engine/version_history.go SHA256 291be0ab44add644f6152c5bcd7420981e4d23efe2aab0ea11a2b770e5d338c2
/// Retained French/English version summary; no translation or publication.
/// Wire declaration: `HEAD:internal/engine/version_history.go:24`.
#[derive(Clone, PartialEq, Serialize)]
pub struct LocalizedSummary {
    /// Go `FR` (version_history.go:25); supplied data, not inferred.
    pub fr: String,
    /// Go `EN` (version_history.go:26); supplied data, not inferred.
    pub en: String,
}

/// Retained commit identity/link, not proof that a binary was built from it.
/// Wire declaration: `HEAD:internal/engine/version_history.go:29`.
#[derive(Clone, PartialEq, Serialize)]
pub struct VersionCommit {
    /// Go `SHA` (version_history.go:30); supplied data, not inferred.
    pub sha: String,
    /// Go `URL` (version_history.go:31); supplied data, not inferred.
    pub url: String,
}

/// One retained release entry; collections preserve supplied order and nil.
/// Wire declaration: `HEAD:internal/engine/version_history.go:34`.
#[derive(Clone, PartialEq, Serialize)]
pub struct VersionRelease {
    /// Go `Version` (version_history.go:35); supplied data, not inferred.
    pub version: String,
    /// Go `Date` (version_history.go:36); supplied data, not inferred.
    pub date: String,
    /// Go `Summary` (version_history.go:37); supplied data, not inferred.
    pub summary: LocalizedSummary,
    /// Go `Commits` (version_history.go:38); supplied data, not inferred.
    pub commits: Option<Vec<VersionCommit>>,
    /// Go `ReleaseURL` (version_history.go:39); supplied data, not inferred.
    pub release_url: String,
}

/// Embedded candidate changes, not a published release or installed delivery.
/// Wire declaration: `HEAD:internal/engine/version_history.go:43`.
#[derive(Clone, PartialEq, Serialize)]
pub struct LocalVersionChanges {
    /// Go `Summary` (version_history.go:44); supplied data, not inferred.
    pub summary: LocalizedSummary,
    /// Go `Items` (version_history.go:45); supplied data, not inferred.
    pub items: Option<Vec<LocalizedSummary>>,
}

/// Observed release-history availability and content.
/// A producer load failure is not silently replaced by a constructor here.
/// Wire declaration: `HEAD:internal/engine/version_history.go:54`.
#[derive(Clone, PartialEq, Serialize)]
pub struct VersionHistory {
    /// Go `Schema` (version_history.go:55); supplied data, not inferred.
    #[serde(rename = "schema_version")]
    pub schema: i64,
    /// Go `Available` (version_history.go:56); supplied data, not inferred.
    pub available: bool,
    /// Go `Releases` (version_history.go:57); supplied data, not inferred.
    pub releases: Option<Vec<VersionRelease>>,
    /// Go `LocalChanges` (version_history.go:58); supplied data, not inferred.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_changes: Option<Box<LocalVersionChanges>>,
}

// HEAD:internal/engine/version_diagnostic.go SHA256 f06a9e71d14fc20f853e0f22ab8c4325dbeeac7a83daad23895f4321c8d8dc1e
/// One role-specific version observation with nullable unknown facts.
/// Empty hint is omitted; available does not authenticate the supplied metadata.
/// Wire declaration: `HEAD:internal/engine/version_diagnostic.go:20`.
#[derive(Clone, PartialEq, Serialize)]
pub struct VersionObservation {
    /// Go `Available` (version_diagnostic.go:21); supplied data, not inferred.
    pub available: bool,
    /// Go `Version` (version_diagnostic.go:22); supplied data, not inferred.
    pub version: Option<Box<String>>,
    /// Go `Commit` (version_diagnostic.go:23); supplied data, not inferred.
    pub commit: Option<Box<String>>,
    /// Go `Modified` (version_diagnostic.go:24); supplied data, not inferred.
    pub modified: Option<Box<bool>>,
    /// Go `Provenance` (version_diagnostic.go:25); supplied data, not inferred.
    pub provenance: String,
    /// Go `Hint` (version_diagnostic.go:26); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub hint: String,
}

/// One producer-diagnosed role difference; no comparison is performed.
/// Wire declaration: `HEAD:internal/engine/version_diagnostic.go:29`.
#[derive(Clone, PartialEq, Serialize)]
pub struct VersionDifference {
    /// Go `Left` (version_diagnostic.go:30); supplied data, not inferred.
    pub left: String,
    /// Go `Right` (version_diagnostic.go:31); supplied data, not inferred.
    pub right: String,
}

/// Supplied installed/server/candidate comparison, not a binary comparison service.
/// Unknown observations and nil differences remain distinct from verified equality.
/// Wire declaration: `HEAD:internal/engine/version_diagnostic.go:34`.
#[derive(Clone, PartialEq, Serialize)]
pub struct VersionDiagnostic {
    /// Go `InstalledCLI` (version_diagnostic.go:35); supplied data, not inferred.
    pub installed_cli: VersionObservation,
    /// Go `ActiveServer` (version_diagnostic.go:36); supplied data, not inferred.
    pub active_server: VersionObservation,
    /// Go `Candidate` (version_diagnostic.go:37); supplied data, not inferred.
    pub candidate: VersionObservation,
    /// Go `State` (version_diagnostic.go:38); supplied data, not inferred.
    pub state: String,
    /// Go `Differences` (version_diagnostic.go:39); supplied data, not inferred.
    pub differences: Option<Vec<VersionDifference>>,
    /// Go `NextStep` (version_diagnostic.go:40); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub next_step: String,
}

// HEAD:internal/engine/dialogue_recovery.go SHA256 54f4fe348e6e90045730c3c21f28657eb97a955e831f36e2aa2325b2959d8471
/// One diagnostic check supplied by recovery; no check or provider probe runs.
/// Wire declaration: `HEAD:internal/engine/dialogue_recovery.go:13`.
#[derive(Clone, PartialEq, Serialize)]
pub struct DialogueRecoveryCheck {
    /// Go `Code` (dialogue_recovery.go:14); supplied data, not inferred.
    pub code: String,
    /// Go `Status` (dialogue_recovery.go:15); supplied data, not inferred.
    pub status: String,
    /// Go `Message` (dialogue_recovery.go:16); supplied data, not inferred.
    pub message: String,
}

/// Stored dialogue recovery diagnosis, not consent or a successful preflight.
/// Boundary may be null; facts hash, observation time and checks are producer data.
/// Wire declaration: `HEAD:internal/engine/dialogue_recovery.go:21`.
#[derive(Clone, PartialEq, Serialize)]
pub struct DialogueRecoveryPreview {
    /// Go `Boundary` (dialogue_recovery.go:22); supplied data, not inferred.
    pub boundary: Option<Box<DialogueBoundaryView>>,
    /// Go `Path` (dialogue_recovery.go:23); supplied data, not inferred.
    #[serde(rename = "recommended_path")]
    pub path: String,
    /// Go `Recommendation` (dialogue_recovery.go:24); supplied data, not inferred.
    pub recommendation: String,
    /// Go `ResumeSupported` (dialogue_recovery.go:25); supplied data, not inferred.
    pub resume_supported: bool,
    /// Go `DecisionRecorded` (dialogue_recovery.go:26); supplied data, not inferred.
    pub decision_recorded: bool,
    /// Go `DecisionOwner` (dialogue_recovery.go:27); supplied data, not inferred.
    pub decision_owner: String,
    /// Go `ObservedAt` (dialogue_recovery.go:28); supplied data, not inferred.
    pub observed_at: String,
    /// Go `FactsSHA256` (dialogue_recovery.go:29); supplied data, not inferred.
    pub facts_sha256: String,
    /// Go `Checks` (dialogue_recovery.go:30); supplied data, not inferred.
    pub checks: Option<Vec<DialogueRecoveryCheck>>,
}

// HEAD:internal/engine/dialogue_checkpoint_store.go SHA256 8bd9fbe111a07d3efc27fe2e574ad77a52af4b4eb238a496e1a25f9fbd5088a2
/// Stored dialogue boundary view excluding private guards/tool arguments.
/// Boundary-confirmed/restart-supported flags are observations, not restart grants.
/// Wire declaration: `HEAD:internal/engine/dialogue_checkpoint_store.go:72`.
#[derive(Clone, PartialEq, Serialize)]
pub struct DialogueBoundaryView {
    /// Go `Available` (dialogue_checkpoint_store.go:73); supplied data, not inferred.
    pub available: bool,
    /// Go `Turn` (dialogue_checkpoint_store.go:74); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub turn: String,
    /// Go `Status` (dialogue_checkpoint_store.go:75); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub status: String,
    /// Go `Phase` (dialogue_checkpoint_store.go:76); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub phase: String,
    /// Go `Confirmed` (dialogue_checkpoint_store.go:77); supplied data, not inferred.
    #[serde(rename = "boundary_confirmed")]
    pub confirmed: bool,
    /// Go `RestartSupported` (dialogue_checkpoint_store.go:78); supplied data, not inferred.
    pub restart_supported: bool,
    /// Go `Session` (dialogue_checkpoint_store.go:79); supplied data, not inferred.
    #[serde(rename = "session_id")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub session: String,
    /// Go `At` (dialogue_checkpoint_store.go:80); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub at: String,
    /// Go `SHA256` (dialogue_checkpoint_store.go:81); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub sha256: String,
    /// Go `Scope` (dialogue_checkpoint_store.go:82); supplied data, not inferred.
    pub scope: String,
    /// Go `Progress` (dialogue_checkpoint_store.go:83); supplied data, not inferred.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<Box<AgentProgress>>,
    /// Go `Usage` (dialogue_checkpoint_store.go:84); supplied data, not inferred.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Box<Usage>>,
}

// HEAD:internal/engine/spending_calls.go SHA256 17d274b7795388da08bed63add537b6ad6ca2a8fe4e7c47c8796cbdc25ea97cf
/// One recorded attempt/reservation, never proof that inference was sent.
/// Missing Usage remains null; requested/observed model attribution is not inferred.
/// Wire declaration: `HEAD:internal/engine/spending_calls.go:14`.
#[derive(Clone, PartialEq, Serialize)]
pub struct SpendingCall {
    /// Go `ID` (spending_calls.go:15); supplied data, not inferred.
    pub id: String,
    /// Go `RecordType` (spending_calls.go:16); supplied data, not inferred.
    pub record_type: String,
    /// Go `Kind` (spending_calls.go:17); supplied data, not inferred.
    pub kind: String,
    /// Go `Role` (spending_calls.go:18); supplied data, not inferred.
    pub role: String,
    /// Go `Scope` (spending_calls.go:19); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub scope: String,
    /// Go `Task` (spending_calls.go:20); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub task: String,
    /// Go `Attempt` (spending_calls.go:21); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub attempt: String,
    /// Go `Provider` (spending_calls.go:22); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub provider: String,
    /// Go `RequestedModel` (spending_calls.go:23); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub requested_model: String,
    /// Go `ObservedModel` (spending_calls.go:24); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub observed_model: String,
    /// Go `State` (spending_calls.go:25); supplied data, not inferred.
    pub state: String,
    /// Go `RegisteredAt` (spending_calls.go:26); supplied data, not inferred.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub registered_at: String,
    /// Go `Usage` (spending_calls.go:27); supplied data, not inferred.
    pub usage: Option<Box<Usage>>,
}

#[cfg(test)]
pub(crate) mod tests;
