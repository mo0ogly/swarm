//! Generated PlanningRequest data; no action, identity or authority is established.
use crate::model_wire::{Fields, Kind};
use crate::{ExchangeArtifact, ValidationControl};
use serde::ser::SerializeMap;
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
// internal/engine/managed_git.go SHA-256 c8a62e0ba38a8c19f18815655f84ce6620fd4f78505216842b1d472de29626a5
/// Supplied Go-compatible request data; decoding grants no authority or action.
#[derive(Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ManagedRepositoryRequest {
    // Go Path, line 21
    pub path: String,
    // Go CommittedOnly, line 22
    pub committed_only: bool,
    // Go Include, line 23
    #[serde(skip_serializing_if = "empty_vec")]
    pub include_dirty: Option<Vec<String>>,
}
pub(crate) fn shape_managed_repository_request() -> Fields {
    &[
        ("path", Kind::String),
        ("committed_only", Kind::Bool),
        ("include_dirty", Kind::Array(&Kind::String)),
    ]
}
// internal/engine/planning.go SHA-256 66ee75df2ce0490e2d362b23030ed033e4e8cf1db402c5448b4464e1af5f62eb
/// Supplied Go-compatible request data; decoding grants no authority or action.
#[derive(Clone, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PlanningOperation {
    // Go JudgeVerdict, line 81
    #[serde(skip_serializing_if = "String::is_empty")]
    pub judge_verdict: String,
    // Go JudgeCycle, line 82
    #[serde(skip_serializing_if = "is_zero")]
    pub judge_cycle: i64,
    // Go ExpectedCandidate, line 83
    #[serde(skip_serializing_if = "String::is_empty")]
    pub expected_candidate: String,
    // Go MaxTasks, line 84
    #[serde(skip_serializing_if = "is_zero")]
    pub max_tasks: i64,
    // Go MaxActivations, line 85
    #[serde(skip_serializing_if = "is_zero")]
    pub max_activations: i64,
    // Go Kind, line 86
    pub kind: String,
    // Go ID, line 87
    pub id: String,
    // Go Title, line 88
    pub title: String,
    // Go Requirements, line 89
    pub requirements: Option<Vec<String>>,
    // Go Deliverable, line 90
    #[serde(skip_serializing_if = "String::is_empty")]
    pub deliverable: String,
    // Go Criteria, line 91
    #[serde(skip_serializing_if = "empty_vec")]
    pub criteria: Option<Vec<String>>,
    // Go Depends, line 92
    #[serde(skip_serializing_if = "empty_vec")]
    pub depends: Option<Vec<String>>,
    // Go Next, line 93
    #[serde(skip_serializing_if = "String::is_empty")]
    pub next: String,
}
pub(crate) fn shape_planning_operation() -> Fields {
    &[
        ("judge_verdict", Kind::String),
        ("judge_cycle", Kind::Int64),
        ("expected_candidate", Kind::String),
        ("max_tasks", Kind::Int64),
        ("max_activations", Kind::Int64),
        ("kind", Kind::String),
        ("id", Kind::String),
        ("title", Kind::String),
        ("requirements", Kind::Array(&Kind::String)),
        ("deliverable", Kind::String),
        ("criteria", Kind::Array(&Kind::String)),
        ("depends", Kind::Array(&Kind::String)),
        ("next", Kind::String),
    ]
}
// internal/engine/planning.go SHA-256 66ee75df2ce0490e2d362b23030ed033e4e8cf1db402c5448b4464e1af5f62eb
/// Supplied Go-compatible request data; decoding grants no authority or action.
#[derive(Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct PlanningRequest {
    // Go InstructionRevisionID, line 98
    #[serde(skip_serializing_if = "String::is_empty")]
    pub instruction_revision_id: String,
    // Go MissingReport, line 101
    #[serde(skip_serializing_if = "is_false")]
    pub missing_report: bool,
    // Go ScopeFiles, line 104
    #[serde(skip_serializing_if = "empty_vec")]
    pub scope_files: Option<Vec<String>>,
    // Go ConfirmReviewUnknownCorrection, line 105
    #[serde(skip_serializing_if = "is_false")]
    pub confirm_review_unknown_correction: bool,
    // Go ConfirmReviewErrorRepair, line 106
    #[serde(skip_serializing_if = "is_false")]
    pub confirm_review_error_repair: bool,
    // Go MaxReviewCalls, line 107
    #[serde(skip_serializing_if = "is_zero")]
    pub max_review_calls: i64,
    // Go ResultCommit, line 108
    #[serde(skip_serializing_if = "String::is_empty")]
    pub result_commit: String,
    // Go ExpectedCandidate, line 109
    #[serde(skip_serializing_if = "String::is_empty")]
    pub expected_candidate: String,
    // Go ResultTree, line 110
    #[serde(skip_serializing_if = "String::is_empty")]
    pub result_tree: String,
    // Go ConfirmRecovery, line 111
    #[serde(skip_serializing_if = "is_false")]
    pub confirm_recovery: bool,
    // Go ReviewID, line 112
    #[serde(skip_serializing_if = "String::is_empty")]
    pub review_id: String,
    // Go RecoveryInstruction, line 113
    #[serde(skip_serializing_if = "String::is_empty")]
    pub recovery_instruction: String,
    // Go ReviewTimeoutSeconds, line 114
    #[serde(skip_serializing_if = "is_zero")]
    pub review_timeout_seconds: i64,
    // Go Level, line 115
    #[serde(skip_serializing_if = "String::is_empty")]
    pub level: String,
    // Go PolicyHash, line 116
    #[serde(skip_serializing_if = "String::is_empty")]
    pub model_policy_hash: String,
    // Go Checks, line 117
    #[serde(skip_serializing_if = "empty_map")]
    pub checks: Option<BTreeMap<String, Option<Vec<ValidationControl>>>>,
    // Go Repository, line 118
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<Box<ManagedRepositoryRequest>>,
    // Go Provider, line 119
    #[serde(skip_serializing_if = "String::is_empty")]
    pub provider: String,
    // Go MaxActivations, line 120
    #[serde(skip_serializing_if = "is_zero")]
    pub max_activations: i64,
    // Go Agent, line 121
    #[serde(skip_serializing_if = "String::is_empty")]
    pub agent_id: String,
    // Go Task, line 122
    #[serde(skip_serializing_if = "String::is_empty")]
    pub task_id: String,
    // Go Attempt, line 123
    #[serde(skip_serializing_if = "String::is_empty")]
    pub attempt_id: String,
    // Go Artifacts, line 124
    #[serde(skip_serializing_if = "empty_vec")]
    pub artifacts: Option<Vec<ExchangeArtifact>>,
    // Go Schema, line 125
    pub schema_version: i64,
    // Go EventID, line 126
    pub event_id: String,
    // Go Revision, line 127
    pub expected_revision: i64,
    // Go Scope, line 128
    #[serde(skip_serializing_if = "String::is_empty")]
    pub scope: String,
    // Go ScopeRevision, line 129
    #[serde(skip_serializing_if = "is_zero")]
    pub scope_revision: i64,
    // Go Holder, line 130
    #[serde(skip_serializing_if = "String::is_empty")]
    pub holder: String,
    // Go Generation, line 131
    #[serde(skip_serializing_if = "is_zero")]
    pub generation: i64,
    // Go LeaseSeconds, line 132
    #[serde(skip_serializing_if = "is_zero")]
    pub lease_seconds: i64,
    // Go MaxTasks, line 133
    #[serde(skip_serializing_if = "is_zero")]
    pub max_tasks: i64,
    // Go MaxDecisions, line 134
    #[serde(skip_serializing_if = "is_zero")]
    pub max_decisions: i64,
    // Go Inputs, line 135
    #[serde(skip_serializing_if = "empty_vec")]
    pub input_events: Option<Vec<String>>,
    // Go Operations, line 136
    #[serde(skip_serializing_if = "empty_vec")]
    pub operations: Option<Vec<PlanningOperation>>,
    // Go Reason, line 137
    #[serde(skip_serializing_if = "String::is_empty")]
    pub reason: String,
    // Go JudgeVerdict, line 138
    #[serde(skip_serializing_if = "String::is_empty")]
    pub judge_verdict: String,
    // Go JudgeCycle, line 139
    #[serde(skip_serializing_if = "is_zero")]
    pub judge_cycle: i64,
    // Go MaxJudgeCycles, line 140
    #[serde(skip_serializing_if = "is_zero")]
    pub max_judge_cycles: i64,
}
pub(crate) fn shape_planning_request() -> Fields {
    &[
        ("instruction_revision_id", Kind::String),
        ("missing_report", Kind::Bool),
        ("scope_files", Kind::Array(&Kind::String)),
        ("confirm_review_unknown_correction", Kind::Bool),
        ("confirm_review_error_repair", Kind::Bool),
        ("max_review_calls", Kind::Int64),
        ("result_commit", Kind::String),
        ("expected_candidate", Kind::String),
        ("result_tree", Kind::String),
        ("confirm_recovery", Kind::Bool),
        ("review_id", Kind::String),
        ("recovery_instruction", Kind::String),
        ("review_timeout_seconds", Kind::Int64),
        ("level", Kind::String),
        ("model_policy_hash", Kind::String),
        (
            "checks",
            Kind::Map(&Kind::Array(&Kind::Object(
                crate::models::shape_validation_control,
            ))),
        ),
        (
            "repository",
            Kind::Pointer(&Kind::Object(shape_managed_repository_request)),
        ),
        ("provider", Kind::String),
        ("max_activations", Kind::Int64),
        ("agent_id", Kind::String),
        ("task_id", Kind::String),
        ("attempt_id", Kind::String),
        (
            "artifacts",
            Kind::Array(&Kind::Object(crate::models::shape_exchange_artifact)),
        ),
        ("schema_version", Kind::Int64),
        ("event_id", Kind::String),
        ("expected_revision", Kind::Int64),
        ("scope", Kind::String),
        ("scope_revision", Kind::Int64),
        ("holder", Kind::String),
        ("generation", Kind::Int64),
        ("lease_seconds", Kind::Int64),
        ("max_tasks", Kind::Int64),
        ("max_decisions", Kind::Int64),
        ("input_events", Kind::Array(&Kind::String)),
        (
            "operations",
            Kind::Array(&Kind::Object(shape_planning_operation)),
        ),
        ("reason", Kind::String),
        ("judge_verdict", Kind::String),
        ("judge_cycle", Kind::Int64),
        ("max_judge_cycles", Kind::Int64),
    ]
}
// Actual PlanningOperation.MarshalJSON producer SHA-256 44e9f9fbca6080f927e6163eb1f776860ba390a2124c05e403ecbfdd3f00d04d
impl Serialize for PlanningOperation {
    /// Encode the actual Go dependency-revision wire contract: an explicit empty
    /// depends list is retained only for revise-dependencies. Serializer failures
    /// propagate; no IO, Work edits, operation or authority is established.
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut out = s.serialize_map(None)?;
        if !String::is_empty(&self.judge_verdict) {
            out.serialize_entry("judge_verdict", &self.judge_verdict)?;
        }
        if !is_zero(&self.judge_cycle) {
            out.serialize_entry("judge_cycle", &self.judge_cycle)?;
        }
        if !String::is_empty(&self.expected_candidate) {
            out.serialize_entry("expected_candidate", &self.expected_candidate)?;
        }
        if !is_zero(&self.max_tasks) {
            out.serialize_entry("max_tasks", &self.max_tasks)?;
        }
        if !is_zero(&self.max_activations) {
            out.serialize_entry("max_activations", &self.max_activations)?;
        }
        out.serialize_entry("kind", &self.kind)?;
        out.serialize_entry("id", &self.id)?;
        out.serialize_entry("title", &self.title)?;
        out.serialize_entry("requirements", &self.requirements)?;
        if !String::is_empty(&self.deliverable) {
            out.serialize_entry("deliverable", &self.deliverable)?;
        }
        if !empty_vec(&self.criteria) {
            out.serialize_entry("criteria", &self.criteria)?;
        }
        if !empty_vec(&self.depends) {
            out.serialize_entry("depends", &self.depends)?;
        }
        if !String::is_empty(&self.next) {
            out.serialize_entry("next", &self.next)?;
        }
        if self.kind == "revise-dependencies" && self.depends.as_ref().is_some_and(Vec::is_empty) {
            out.serialize_entry("depends", &self.depends)?;
        }
        out.end()
    }
}
