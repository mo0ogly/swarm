//! Historical `PreparationTurn` read model (Go-compatible wire projection).
//!
//! Stored observations only. Everything in this module records preparation
//! turns already recorded; a record proves no process execution. This starts
//! nothing, kills nothing, finishes nothing. `timeout_seconds` and
//! `timeout_maximum_seconds` are what the historical run was configured
//! with — retained observations, never defaults for a new execution. The
//! historical read path observes legacy active rows without adopting them. Native active rows
//! require their private physical-Root envelope. No Go read-side kill/finish
//! heuristic is performed by this DTO.
//!

use crate::model_wire::{Fields, Kind};
use crate::models::{shape_model_route, shape_project_context, shape_usage};
use crate::{ModelRoute, ProjectContext, Usage};
use serde::{Deserialize, Serialize};

/// `serde(skip_serializing_if)` predicate mirroring Go int `omitempty`.
fn is_zero(v: &i64) -> bool {
    *v == 0
}

/// Stored provider answer for a preparation turn (Go `engine.PreparationAnswer`).
///
/// Retained observation of what the provider returned, kept for the record.
/// The text is history, not permission: it does not approve the plan, adopt
/// the brief, or authorize any dispatch. Decoding type-validates; serialization errors propagate through the public consumer.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PreparationAnswer {
    /// Plan text; omitted when empty (Go `omitempty`).
    #[serde(rename = "plan", skip_serializing_if = "String::is_empty")]
    pub plan: String,
    /// Provider message text; always serialized (no `omitempty` in Go).
    #[serde(rename = "message")]
    pub message: String,
    /// Brief text; always serialized (no `omitempty` in Go).
    #[serde(rename = "brief")]
    pub brief: String,
}

/// Wire projection of [`PreparationAnswer`]; pure, no error paths.
pub(crate) fn shape_preparation_answer() -> Fields {
    &[
        ("plan", Kind::String),
        ("message", Kind::String),
        ("brief", Kind::String),
    ]
}

/// Stored plan counts (Go `engine.PreparationPlanSummary`).
///
/// The counts describe the checked proposal at plan time; they never describe
/// completion and are not dispatch eligibility. Pure data: no validation, no
/// error paths.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PreparationPlanSummary {
    /// Missions in the checked proposal.
    #[serde(rename = "missions")]
    pub missions: i64,
    /// Dependencies in the checked proposal.
    #[serde(rename = "dependencies")]
    pub dependencies: i64,
    /// Open questions left by the checked proposal.
    #[serde(rename = "open_questions")]
    pub open_questions: i64,
}

/// Wire projection of [`PreparationPlanSummary`]; pure, no error paths.
pub(crate) fn shape_preparation_plan_summary() -> Fields {
    &[
        ("missions", Kind::Int64),
        ("dependencies", Kind::Int64),
        ("open_questions", Kind::Int64),
    ]
}

/// One historical preparation turn, mirroring Go `engine.PreparationTurn`
/// field-for-field: declaration order, JSON tag names, and `omitempty`.
///
/// Retained observation of a recorded turn — not permission, not
/// dispatch eligibility, not an execution authority. Sensitive fields:
/// `prompt` carries private source text and is never serialized (see its
/// doc); `question`/`answer` record provider interaction; the hash fields
/// are integrity observations of stored artifacts, not authenticity
/// guarantees; `pid`/`process_stamp`/`supervisor_pid`/`supervisor_stamp`/
/// `host` are process observations, not handles to act on. Timeouts are
/// historical configuration, never defaults for new runs. Decoding
/// type-validates via serde and the wire layer; serialization can fail through its public consumer (and never emits `prompt`).
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PreparationTurn {
    /// Project snapshot observed for the turn (Go `omitempty`).
    #[serde(rename = "project", skip_serializing_if = "Option::is_none")]
    pub project: Option<Box<ProjectContext>>,
    /// Model route observed for the turn (Go `omitempty`); decoded through
    /// the existing `ModelRoute` type and wire arena, so ordered/alias/null/
    /// duplicate semantics are unchanged. No raw JSON Value here.
    #[serde(rename = "model_route", skip_serializing_if = "Option::is_none")]
    pub model_route: Option<Box<ModelRoute>>,
    /// Context mode used; omitted when empty.
    #[serde(rename = "context_mode", skip_serializing_if = "String::is_empty")]
    pub context_mode: String,
    /// Turn target; omitted when empty.
    #[serde(rename = "target", skip_serializing_if = "String::is_empty")]
    pub target: String,
    /// Retained flag: the turn fed the plan. History, not eligibility.
    #[serde(rename = "used_in_plan")]
    pub used_in_plan: bool,
    /// Stored plan counts (Go `omitempty` on the pointer).
    #[serde(rename = "plan_summary", skip_serializing_if = "Option::is_none")]
    pub plan_summary: Option<Box<PreparationPlanSummary>>,
    /// Go field `Used`; the Go JSON tag `used_in_brief` is preserved exactly.
    /// Retained flag: the turn's brief was adopted. History, not eligibility.
    #[serde(rename = "used_in_brief")]
    pub used: bool,
    /// Stored turn identifier.
    #[serde(rename = "id")]
    pub id: String,
    /// Stored parent preparation identifier.
    #[serde(rename = "preparation_id")]
    pub preparation_id: String,
    /// Stored status string; see [`PreparationTurn::active`].
    #[serde(rename = "status")]
    pub status: String,
    /// Stored creation timestamp (string, as Go stores it).
    #[serde(rename = "created_at")]
    pub created_at: String,
    /// Stored end timestamp; omitted when empty (Go `omitempty`).
    #[serde(rename = "ended_at", skip_serializing_if = "String::is_empty")]
    pub ended_at: String,
    /// Go field `Revision`; Go JSON tag `source_revision`. Observed source
    /// revision for the turn.
    #[serde(rename = "source_revision")]
    pub revision: i64,
    /// Stored method hash; integrity observation, not authenticity.
    #[serde(rename = "method_hash")]
    pub method_hash: String,
    /// Stored provider name as observed.
    #[serde(rename = "provider")]
    pub provider: String,
    /// Stored provider digest; integrity observation, not authenticity.
    #[serde(rename = "provider_digest")]
    pub provider_digest: String,
    /// Stored request hash; integrity observation, not authenticity.
    #[serde(rename = "request_hash")]
    pub request_hash: String,
    /// Stored question text (may contain user content); retained for record.
    #[serde(rename = "question")]
    pub question: String,
    /// Private source text (Go `prompt,omitempty`). Sensitive: decoding and
    /// type-validation still work, but serialization always omits this field,
    /// matching Go public dialogue reads even if a caller forgets to clear it.
    /// This readonly DTO must never serialize stored provider requests; a future
    /// runner needs its own private persistence contract.
    #[serde(rename = "prompt", skip_serializing)]
    pub prompt: String,
    /// Stored provider answer (Go `omitempty` on the pointer).
    #[serde(rename = "answer", skip_serializing_if = "Option::is_none")]
    pub answer: Option<Box<PreparationAnswer>>,
    /// Stored error text; omitted when empty.
    #[serde(rename = "error", skip_serializing_if = "String::is_empty")]
    pub error: String,
    /// Stored usage totals (Go `omitempty` on the pointer).
    #[serde(rename = "usage", skip_serializing_if = "Option::is_none")]
    pub usage: Option<Box<Usage>>,
    /// Retained staleness flag as recorded.
    #[serde(rename = "stale")]
    pub stale: bool,
    /// Historical timeout the run was configured with; an observation, never
    /// a default for a new execution. Always serialized (no `omitempty`).
    #[serde(rename = "timeout_seconds")]
    pub timeout_seconds: i64,
    /// Go field `TimeoutMaximum`; Go JSON tag `timeout_maximum_seconds`.
    /// Historical observation; omitted when zero (Go `omitempty`).
    #[serde(rename = "timeout_maximum_seconds", skip_serializing_if = "is_zero")]
    pub timeout_maximum: i64,
    /// Observed worker pid; omitted when zero (Go `omitempty`). History,
    /// not a handle to act on.
    #[serde(rename = "pid", skip_serializing_if = "is_zero")]
    pub pid: i64,
    /// Observed process stamp; omitted when empty.
    #[serde(rename = "process_stamp", skip_serializing_if = "String::is_empty")]
    pub process_stamp: String,
    /// Observed supervisor pid; omitted when zero (Go `omitempty`). History,
    /// not a handle to act on.
    #[serde(rename = "supervisor_pid", skip_serializing_if = "is_zero")]
    pub supervisor_pid: i64,
    /// Observed supervisor stamp; omitted when empty.
    #[serde(rename = "supervisor_stamp", skip_serializing_if = "String::is_empty")]
    pub supervisor_stamp: String,
    /// Observed host; omitted when empty.
    #[serde(rename = "host", skip_serializing_if = "String::is_empty")]
    pub host: String,
}

impl PreparationTurn {
    /// Pure classifier over the retained `status` string: true only for the
    /// stored statuses `pending`, `running`, or `stopping`.
    ///
    /// Observation, not permission: no I/O, no clock, no elapsed-time
    /// heuristic, no policy constants, and no kill/finish behavior. It does
    /// not decide whether a row may be read; the Root reader refuses
    /// legacy active rows (501) and binds native active metadata separately;
    /// neither guard is implemented in this pure classifier. Any other status string
    /// (including empty or unknown) yields `false`; `false` does not claim
    /// the cause. Pure function, no error paths.
    pub fn active(&self) -> bool {
        matches!(self.status.as_str(), "pending" | "running" | "stopping")
    }
}

/// Wire projection of [`PreparationTurn`]; field order matches the Go struct.
///
/// `prompt` stays in the shape because the historical record carries it and
/// decoding must type-validate it; its suppression on serialization lives on
/// [`PreparationTurn::prompt`], not here. Pure, no error paths.
pub(crate) fn shape_preparation_turn() -> Fields {
    &[
        ("project", Kind::Pointer(&Kind::Object(shape_project_context))),
        ("model_route", Kind::Pointer(&Kind::Object(shape_model_route))),
        ("context_mode", Kind::String),
        ("target", Kind::String),
        ("used_in_plan", Kind::Bool),
        ("plan_summary", Kind::Pointer(&Kind::Object(shape_preparation_plan_summary))),
        ("used_in_brief", Kind::Bool),
        ("id", Kind::String),
        ("preparation_id", Kind::String),
        ("status", Kind::String),
        ("created_at", Kind::String),
        ("ended_at", Kind::String),
        ("source_revision", Kind::Int64),
        ("method_hash", Kind::String),
        ("provider", Kind::String),
        ("provider_digest", Kind::String),
        ("request_hash", Kind::String),
        ("question", Kind::String),
        ("prompt", Kind::String),
        ("answer", Kind::Pointer(&Kind::Object(shape_preparation_answer))),
        ("error", Kind::String),
        ("usage", Kind::Pointer(&Kind::Object(shape_usage))),
        ("stale", Kind::Bool),
        ("timeout_seconds", Kind::Int64),
        ("timeout_maximum_seconds", Kind::Int64),
        ("pid", Kind::Int64),
        ("process_stamp", Kind::String),
        ("supervisor_pid", Kind::Int64),
        ("supervisor_stamp", Kind::String),
        ("host", Kind::String),
    ]
}
