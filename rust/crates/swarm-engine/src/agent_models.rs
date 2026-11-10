//! Generated native agent data. Decoding grants no identity or execution authority.
use crate::model_wire::{Fields, Kind};
use serde::{Deserialize, Serialize};
fn is_zero(v: &i64) -> bool {
    *v == 0
}
fn is_false(v: &bool) -> bool {
    !*v
}
fn empty_vec<T>(v: &Option<Vec<T>>) -> bool {
    v.as_ref().is_none_or(Vec::is_empty)
}
// internal/engine/agents_store.go SHA-256 29a04b2234fa0190a7c12ba318c26612e8cbcabc2025010ab03745e19e02039d
/// Go-compatible supplied/retained data, not authenticated identity or permission.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Agent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dialogue_restore: Option<Box<DialogueRestoreReceipt>>,
    #[serde(skip_serializing_if = "is_zero")]
    pub delivery_version: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow: Option<Box<crate::models::AgentWorkflow>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_cooldown: Option<Box<ProviderCooldown>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preflight: Option<Box<PreflightResult>>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub precondition_evidence: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_route: Option<Box<crate::models::ModelRoute>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reported_model: Option<Box<ReportedModel>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Box<crate::models::ContextManifest>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepared_context: Option<Box<crate::models::ContextManifest>>,
    #[serde(skip_serializing_if = "is_false")]
    pub brainstorm: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub reply: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Box<crate::models::Usage>>,
    pub progress: crate::models::AgentProgress,
    // Go value-struct omitempty still serializes this field.
    pub diagnostic: crate::models::AttemptDiagnostic,
    pub recovery: RecoveryState,
    pub limits: crate::models::RunLimits,
    pub id: String,
    pub work_id: String,
    pub task_id: String,
    pub attempt_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub origin: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub relay: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub stop_kind: String,
    pub provider: String,
    pub role: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub parent: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub previous: String,
    pub workspace: String,
    pub status: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub desired: String,
    pub activity: String,
    pub started: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub heartbeat: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub ended: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<Box<i64>>,
    #[serde(skip_serializing_if = "is_zero")]
    pub supervisor_pid: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub supervisor_identity: String,
    #[serde(skip_serializing_if = "is_zero")]
    pub child_pid: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub child_identity: String,
    pub host: String,
    pub timeout_seconds: i64,
    pub capture_output: bool,
    pub prompt: String,
    pub command: String,
    pub args: Option<Vec<String>>,
    pub env_allow: Option<Vec<String>>,
    /// Native v1 opaque/required snapshot; absence stays historical, never a default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_tool_guard_policy: Option<Box<crate::AgentGuardSnapshot>>,
    /// Opaque mission admission provenance; never permission without its canonical
    /// event/claim linkage and current mission checks before the native grant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_mission: Option<crate::GoRaw>,
    /// Opaque native execution constraints; absence is historical, never a permit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_execution: Option<crate::GoRaw>,
    /// Opaque native end observations; never task acceptance or action authority.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_execution_result: Option<crate::GoRaw>,
    /// Opaque current provider declarations and diagnostic quality; no model, invoice or actor authentication.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_provider_facts: Option<crate::GoRaw>,
    /// Internal SQL/body claim observation; never serialized or supplied.
    #[serde(skip)]
    pub(crate) unregistered_claim: bool,
}
impl std::fmt::Debug for Agent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Agent { data: redacted }")
    }
}
/// Static Go field order and types; no validation, IO or authorization.
pub(crate) fn shape_agent() -> Fields {
    &[
        (
            "dialogue_restore",
            Kind::Pointer(&Kind::Object(shape_dialogue_restore_receipt)),
        ),
        ("delivery_version", Kind::Int64),
        (
            "workflow",
            Kind::Pointer(&Kind::Object(crate::models::shape_agent_workflow)),
        ),
        (
            "provider_cooldown",
            Kind::Pointer(&Kind::Object(shape_provider_cooldown)),
        ),
        (
            "preflight",
            Kind::Pointer(&Kind::Object(shape_preflight_result)),
        ),
        ("precondition_evidence", Kind::String),
        ("mode", Kind::String),
        (
            "model_route",
            Kind::Pointer(&Kind::Object(crate::models::shape_model_route)),
        ),
        (
            "reported_model",
            Kind::Pointer(&Kind::Object(shape_reported_model)),
        ),
        (
            "context",
            Kind::Pointer(&Kind::Object(crate::models::shape_context_manifest)),
        ),
        (
            "prepared_context",
            Kind::Pointer(&Kind::Object(crate::models::shape_context_manifest)),
        ),
        ("brainstorm", Kind::Bool),
        ("reply", Kind::String),
        (
            "usage",
            Kind::Pointer(&Kind::Object(crate::models::shape_usage)),
        ),
        (
            "progress",
            Kind::Object(crate::models::shape_agent_progress),
        ),
        (
            "diagnostic",
            Kind::Object(crate::models::shape_attempt_diagnostic),
        ),
        ("recovery", Kind::Object(shape_recovery_state)),
        ("limits", Kind::Object(crate::models::shape_run_limits)),
        ("id", Kind::String),
        ("work_id", Kind::String),
        ("task_id", Kind::String),
        ("attempt_id", Kind::String),
        ("origin", Kind::String),
        ("relay", Kind::String),
        ("stop_kind", Kind::String),
        ("provider", Kind::String),
        ("role", Kind::String),
        ("parent", Kind::String),
        ("previous", Kind::String),
        ("workspace", Kind::String),
        ("status", Kind::String),
        ("desired", Kind::String),
        ("activity", Kind::String),
        ("started", Kind::String),
        ("heartbeat", Kind::String),
        ("ended", Kind::String),
        ("exit_code", Kind::Pointer(&Kind::Int64)),
        ("supervisor_pid", Kind::Int64),
        ("supervisor_identity", Kind::String),
        ("child_pid", Kind::Int64),
        ("child_identity", Kind::String),
        ("host", Kind::String),
        ("timeout_seconds", Kind::Int64),
        ("capture_output", Kind::Bool),
        ("prompt", Kind::String),
        ("command", Kind::String),
        ("args", Kind::Array(&Kind::String)),
        ("env_allow", Kind::Array(&Kind::String)),
        ("native_tool_guard_policy", Kind::Pointer(&Kind::Raw)),
        ("native_mission", Kind::Pointer(&Kind::Raw)),
        ("native_execution", Kind::Pointer(&Kind::Raw)),
        ("native_execution_result", Kind::Pointer(&Kind::Raw)),
        ("native_provider_facts", Kind::Pointer(&Kind::Raw)),
    ]
}
// internal/engine/dialogue_restore.go SHA-256 5bf4294aa733eabd0df9e26e958acbadc200729375ca01a368c21c4f8b2f829e
/// Go-compatible supplied/retained data, not authenticated identity or permission.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DialogueRestoreChoice {
    pub checkpoint_sha256: String,
    pub facts_sha256: String,
    pub confirm: bool,
    pub reason: String,
}
impl std::fmt::Debug for DialogueRestoreChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DialogueRestoreChoice { data: redacted }")
    }
}
/// Static Go field order and types; no validation, IO or authorization.
pub(crate) fn shape_dialogue_restore_choice() -> Fields {
    &[
        ("checkpoint_sha256", Kind::String),
        ("facts_sha256", Kind::String),
        ("confirm", Kind::Bool),
        ("reason", Kind::String),
    ]
}
// internal/engine/dialogue_restore.go SHA-256 5bf4294aa733eabd0df9e26e958acbadc200729375ca01a368c21c4f8b2f829e
/// Go-compatible supplied/retained data, not authenticated identity or permission.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DialogueRestoreReceipt {
    pub source_agent: String,
    pub source_attempt: String,
    pub source_state_sha256: String,
    pub checkpoint_sha256: String,
    pub facts_sha256: String,
    pub root_agent: String,
    pub prior_turns: i64,
    pub reason: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub actor: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub at: String,
    pub decision_recorded: bool,
}
impl std::fmt::Debug for DialogueRestoreReceipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DialogueRestoreReceipt { data: redacted }")
    }
}
/// Static Go field order and types; no validation, IO or authorization.
pub(crate) fn shape_dialogue_restore_receipt() -> Fields {
    &[
        ("source_agent", Kind::String),
        ("source_attempt", Kind::String),
        ("source_state_sha256", Kind::String),
        ("checkpoint_sha256", Kind::String),
        ("facts_sha256", Kind::String),
        ("root_agent", Kind::String),
        ("prior_turns", Kind::Int64),
        ("reason", Kind::String),
        ("actor", Kind::String),
        ("at", Kind::String),
        ("decision_recorded", Kind::Bool),
    ]
}
// internal/engine/agents_store.go SHA-256 29a04b2234fa0190a7c12ba318c26612e8cbcabc2025010ab03745e19e02039d
/// Go-compatible supplied/retained data, not authenticated identity or permission.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Launch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_selection: Option<Box<crate::models::MemorySummarySelection>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dialogue_restore: Option<Box<DialogueRestoreChoice>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepared_skill_refresh: Option<Box<PreparedSkillRefreshReceipt>>,
    #[serde(skip_serializing_if = "empty_vec")]
    pub skills: Option<Vec<crate::models::ActionSkillSelection>>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub precondition_evidence: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub conductor_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub provider_digest: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limits: Option<Box<crate::models::RunLimits>>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub level: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub model_policy_hash: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub plan_brief_hash: String,
    #[serde(skip_serializing_if = "empty_vec")]
    pub references: Option<Vec<crate::models::DialogueRef>>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub context_hash: String,
    #[serde(skip_serializing_if = "is_false")]
    pub brainstorm: bool,
    pub schema_version: i64,
    pub event_id: String,
    pub expected_revision: i64,
    pub task_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub origin: String,
    pub provider: String,
    pub workspace: String,
    pub instruction: String,
    pub role: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub parent: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub previous: String,
    pub timeout_seconds: i64,
    pub capture_output: bool,
}
impl std::fmt::Debug for Launch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Launch { data: redacted }")
    }
}
/// Static Go field order and types; no validation, IO or authorization.
pub(crate) fn shape_launch() -> Fields {
    &[
        (
            "summary_selection",
            Kind::Pointer(&Kind::Object(crate::models::shape_memory_summary_selection)),
        ),
        (
            "dialogue_restore",
            Kind::Pointer(&Kind::Object(shape_dialogue_restore_choice)),
        ),
        (
            "prepared_skill_refresh",
            Kind::Pointer(&Kind::Object(shape_prepared_skill_refresh_receipt)),
        ),
        (
            "skills",
            Kind::Array(&Kind::Object(crate::models::shape_action_skill_selection)),
        ),
        ("precondition_evidence", Kind::String),
        ("conductor_id", Kind::String),
        ("provider_digest", Kind::String),
        ("mode", Kind::String),
        (
            "limits",
            Kind::Pointer(&Kind::Object(crate::models::shape_run_limits)),
        ),
        ("level", Kind::String),
        ("model_policy_hash", Kind::String),
        ("plan_brief_hash", Kind::String),
        (
            "references",
            Kind::Array(&Kind::Object(crate::models::shape_dialogue_ref)),
        ),
        ("context_hash", Kind::String),
        ("brainstorm", Kind::Bool),
        ("schema_version", Kind::Int64),
        ("event_id", Kind::String),
        ("expected_revision", Kind::Int64),
        ("task_id", Kind::String),
        ("origin", Kind::String),
        ("provider", Kind::String),
        ("workspace", Kind::String),
        ("instruction", Kind::String),
        ("role", Kind::String),
        ("parent", Kind::String),
        ("previous", Kind::String),
        ("timeout_seconds", Kind::Int64),
        ("capture_output", Kind::Bool),
    ]
}
// internal/engine/provider_models.go SHA-256 d1c32751ad1420662b630defb18a04e5bdca71e38a0c5e8fe5dc1aa96e6b3a4b
/// Go-compatible supplied/retained data, not authenticated identity or permission.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelChoice {
    pub model: String,
    pub effort: String,
}
impl std::fmt::Debug for ModelChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ModelChoice { data: redacted }")
    }
}
/// Static Go field order and types; no validation, IO or authorization.
pub(crate) fn shape_model_choice() -> Fields {
    &[("model", Kind::String), ("effort", Kind::String)]
}
// internal/engine/provider_models.go SHA-256 d1c32751ad1420662b630defb18a04e5bdca71e38a0c5e8fe5dc1aa96e6b3a4b
/// Go-compatible supplied/retained data, not authenticated identity or permission.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelPolicy {
    pub version: i64,
    pub billing: String,
    pub work_level: String,
    pub page_level: String,
    pub levels: Option<std::collections::BTreeMap<String, ModelChoice>>,
}
impl std::fmt::Debug for ModelPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ModelPolicy { data: redacted }")
    }
}
/// Static Go field order and types; no validation, IO or authorization.
pub(crate) fn shape_model_policy() -> Fields {
    &[
        ("version", Kind::Int64),
        ("billing", Kind::String),
        ("work_level", Kind::String),
        ("page_level", Kind::String),
        ("levels", Kind::Map(&Kind::Object(shape_model_choice))),
    ]
}
// internal/engine/preflight.go SHA-256 f514435ab9c8227834ac4ac038d3adc9927af02111c2e517f5dfa0cba3ba0cba
/// Go-compatible supplied/retained data, not authenticated identity or permission.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PreflightCheck {
    pub name: String,
    pub status: String,
    pub detail: String,
}
impl std::fmt::Debug for PreflightCheck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreflightCheck { data: redacted }")
    }
}
/// Static Go field order and types; no validation, IO or authorization.
pub(crate) fn shape_preflight_check() -> Fields {
    &[
        ("name", Kind::String),
        ("status", Kind::String),
        ("detail", Kind::String),
    ]
}
// internal/engine/preflight.go SHA-256 f514435ab9c8227834ac4ac038d3adc9927af02111c2e517f5dfa0cba3ba0cba
/// Go-compatible supplied/retained data, not authenticated identity or permission.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PreflightResult {
    pub schema_version: i64,
    pub verdict: String,
    pub checked_at: String,
    pub valid_until: String,
    pub scope: String,
    pub provider: String,
    pub workspace: String,
    pub duration_ms: i64,
    pub verification: String,
    pub capabilities: Option<std::collections::BTreeMap<String, String>>,
    pub checks: Option<Vec<PreflightCheck>>,
}
impl std::fmt::Debug for PreflightResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreflightResult { data: redacted }")
    }
}
/// Static Go field order and types; no validation, IO or authorization.
pub(crate) fn shape_preflight_result() -> Fields {
    &[
        ("schema_version", Kind::Int64),
        ("verdict", Kind::String),
        ("checked_at", Kind::String),
        ("valid_until", Kind::String),
        ("scope", Kind::String),
        ("provider", Kind::String),
        ("workspace", Kind::String),
        ("duration_ms", Kind::Int64),
        ("verification", Kind::String),
        ("capabilities", Kind::Map(&Kind::String)),
        ("checks", Kind::Array(&Kind::Object(shape_preflight_check))),
    ]
}
// internal/engine/prepared_skill_refresh.go SHA-256 e3caa2c9b0f92514cf821848df144a69452fee5466401e5207f19c1ae9f95590
/// Go-compatible supplied/retained data, not authenticated identity or permission.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PreparedSkillRefreshReceipt {
    pub prepared_id: String,
    pub original_request_sha256: String,
    pub previous_skills: Option<Vec<crate::models::ActionSkillSelection>>,
    pub reason: String,
    pub actor: String,
}
impl std::fmt::Debug for PreparedSkillRefreshReceipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreparedSkillRefreshReceipt { data: redacted }")
    }
}
/// Static Go field order and types; no validation, IO or authorization.
pub(crate) fn shape_prepared_skill_refresh_receipt() -> Fields {
    &[
        ("prepared_id", Kind::String),
        ("original_request_sha256", Kind::String),
        (
            "previous_skills",
            Kind::Array(&Kind::Object(crate::models::shape_action_skill_selection)),
        ),
        ("reason", Kind::String),
        ("actor", Kind::String),
    ]
}
// internal/engine/agents_store.go SHA-256 29a04b2234fa0190a7c12ba318c26612e8cbcabc2025010ab03745e19e02039d
/// Go-compatible supplied/retained data, not authenticated identity or permission.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Provider {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub api_connection_id: String,
    #[serde(skip_serializing_if = "empty_vec")]
    pub interactive_args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "is_false")]
    pub preflight_required: bool,
    #[serde(skip_serializing_if = "empty_vec")]
    pub preflight_args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub preflight_kind: String,
    #[serde(skip_serializing_if = "empty_vec")]
    pub preflight_capabilities: Option<Vec<String>>,
    #[serde(skip_serializing_if = "is_zero")]
    pub preflight_timeout_seconds: i64,
    #[serde(skip_serializing_if = "is_zero")]
    pub preflight_ttl_seconds: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_policy: Option<Box<ModelPolicy>>,
    #[serde(skip_serializing_if = "is_zero")]
    pub assistant_timeout_seconds: i64,
    // Go value-struct omitempty still serializes this field.
    pub limits: crate::models::RunLimits,
    pub command: String,
    pub args: Option<Vec<String>>,
    pub env_allow: Option<Vec<String>>,
}
impl std::fmt::Debug for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Provider { data: redacted }")
    }
}
/// Static Go field order and types; no validation, IO or authorization.
pub(crate) fn shape_provider() -> Fields {
    &[
        ("api_connection_id", Kind::String),
        ("interactive_args", Kind::Array(&Kind::String)),
        ("preflight_required", Kind::Bool),
        ("preflight_args", Kind::Array(&Kind::String)),
        ("preflight_kind", Kind::String),
        ("preflight_capabilities", Kind::Array(&Kind::String)),
        ("preflight_timeout_seconds", Kind::Int64),
        ("preflight_ttl_seconds", Kind::Int64),
        (
            "model_policy",
            Kind::Pointer(&Kind::Object(shape_model_policy)),
        ),
        ("assistant_timeout_seconds", Kind::Int64),
        ("limits", Kind::Object(crate::models::shape_run_limits)),
        ("command", Kind::String),
        ("args", Kind::Array(&Kind::String)),
        ("env_allow", Kind::Array(&Kind::String)),
    ]
}
// internal/engine/provider_cooldown.go SHA-256 934d6066800bcd6b4a1f351a63de5fd683faf171c2f7222e4a219cc67c572b7f
/// Go-compatible supplied/retained data, not authenticated identity or permission.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProviderCooldown {
    pub provider: String,
    pub observed_at: String,
    #[serde(skip_serializing_if = "is_zero")]
    pub resets_at: i64,
    pub source: String,
    pub signal: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub cleared_at: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub clear_event: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub clear_reason: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub clear_actor: String,
    #[serde(skip_serializing_if = "is_false")]
    pub clear_account_change: bool,
}
impl std::fmt::Debug for ProviderCooldown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ProviderCooldown { data: redacted }")
    }
}
/// Static Go field order and types; no validation, IO or authorization.
pub(crate) fn shape_provider_cooldown() -> Fields {
    &[
        ("provider", Kind::String),
        ("observed_at", Kind::String),
        ("resets_at", Kind::Int64),
        ("source", Kind::String),
        ("signal", Kind::String),
        ("cleared_at", Kind::String),
        ("clear_event", Kind::String),
        ("clear_reason", Kind::String),
        ("clear_actor", Kind::String),
        ("clear_account_change", Kind::Bool),
    ]
}
// internal/engine/agents_store.go SHA-256 29a04b2234fa0190a7c12ba318c26612e8cbcabc2025010ab03745e19e02039d
/// Go-compatible supplied/retained data, not authenticated identity or permission.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Providers {
    pub schema_version: i64,
    pub providers: Option<std::collections::BTreeMap<String, Provider>>,
}
impl std::fmt::Debug for Providers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Providers { data: redacted }")
    }
}
/// Static Go field order and types; no validation, IO or authorization.
pub(crate) fn shape_providers() -> Fields {
    &[
        ("schema_version", Kind::Int64),
        ("providers", Kind::Map(&Kind::Object(shape_provider))),
    ]
}
// internal/engine/agents_store.go SHA-256 29a04b2234fa0190a7c12ba318c26612e8cbcabc2025010ab03745e19e02039d
/// Go-compatible supplied/retained data, not authenticated identity or permission.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RecoveryState {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub category: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub cause_fingerprint: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub observed_category: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub observed_cause_fingerprint: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub operation_id: String,
    pub automatic_attempts_used: i64,
    pub automatic_attempts_max: i64,
    pub budget_remaining: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub next_eligible_at: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub disposition: String,
}
impl std::fmt::Debug for RecoveryState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RecoveryState { data: redacted }")
    }
}
/// Static Go field order and types; no validation, IO or authorization.
pub(crate) fn shape_recovery_state() -> Fields {
    &[
        ("category", Kind::String),
        ("cause_fingerprint", Kind::String),
        ("observed_category", Kind::String),
        ("observed_cause_fingerprint", Kind::String),
        ("operation_id", Kind::String),
        ("automatic_attempts_used", Kind::Int64),
        ("automatic_attempts_max", Kind::Int64),
        ("budget_remaining", Kind::Int64),
        ("next_eligible_at", Kind::String),
        ("disposition", Kind::String),
    ]
}
// internal/engine/agent_reported_model.go SHA-256 4650883eb58441260a7e6fb172635276c04772471ef13b5beef4d1d594b5d43e
/// Go-compatible supplied/retained data, not authenticated identity or permission.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReportedModel {
    pub model: String,
    pub source: String,
    pub at: String,
}
impl std::fmt::Debug for ReportedModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReportedModel { data: redacted }")
    }
}
/// Static Go field order and types; no validation, IO or authorization.
pub(crate) fn shape_reported_model() -> Fields {
    &[
        ("model", Kind::String),
        ("source", Kind::String),
        ("at", Kind::String),
    ]
}
