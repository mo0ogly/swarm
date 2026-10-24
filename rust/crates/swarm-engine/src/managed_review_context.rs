//! Go-compatible review context data. Decoding proves no identity or review authority.
use crate::{
    model_wire::{Fields, Kind},
    GoRaw, IndependentReview, ManagedTaskReview, ReviewSource, ValidationControlResult, WorkError,
};
use serde::{Deserialize, Serialize};

fn zero(value: &i64) -> bool {
    *value == 0
}
fn no(value: &bool) -> bool {
    !*value
}
fn empty<T>(value: &Option<Vec<T>>) -> bool {
    value.as_ref().is_none_or(Vec::is_empty)
}

/// Supplied review context, including opaque receipt and optional historical baseline.
/// None of these declarations establishes that evidence is authentic or current.
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ManagedReviewContext {
    #[serde(skip_serializing_if = "zero")]
    pub citation_contract_version: i64,
    #[serde(skip_serializing_if = "zero")]
    pub inspection_scope_version: i64,
    #[serde(skip_serializing_if = "no")]
    pub inspection_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepted_baseline: Option<Box<ManagedReviewBaseline>>,
    #[serde(skip_serializing_if = "empty")]
    pub sources: Option<Vec<ReviewSource>>,
    pub candidate_commit: String,
    pub previous_candidate: String,
    pub diff: String,
    pub receipt: GoRaw,
    pub tasks: Option<Vec<ManagedReviewTaskContext>>,
}
impl std::fmt::Debug for ManagedReviewContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManagedReviewContext")
            .field("citation_contract_version", &self.citation_contract_version)
            .field("tasks", &self.tasks.as_ref().map(Vec::len))
            .field("baseline_present", &self.accepted_baseline.is_some())
            .finish()
    }
}
/// One task's supplied delivery, controls and declared review binding, not a verdict.
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ManagedReviewTaskContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery: Option<Box<ManagedDelivery>>,
    pub task: String,
    pub title: String,
    pub deliverable: String,
    pub criteria: Option<Vec<String>>,
    pub report: String,
    pub controls: Option<Vec<ValidationControlResult>>,
    pub binding: ManagedTaskReview,
}
/// Historical reviews carried as data; recursively checking their files/protocol is a separate operation.
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ManagedReviewBaseline {
    #[serde(skip_serializing_if = "zero")]
    pub version: i64,
    pub candidate_commit: String,
    pub accepted_reviews: Option<Vec<IndependentReview>>,
}
/// Producer completeness declaration; `outcome` and criteria do not grant task acceptance.
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ManagedDelivery {
    pub version: i64,
    pub task: String,
    pub attempt: String,
    pub contract: String,
    pub outcome: String,
    pub criteria: Option<Vec<DeliveryCriterion>>,
}
/// Supplied criterion status and references; coverage and permissions remain to be checked.
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DeliveryCriterion {
    pub index: i64,
    pub status: String,
    pub reason: String,
    pub controls: Option<Vec<String>>,
    pub evidence: Option<Vec<String>>,
}
fn shape_context() -> Fields {
    &[
        ("citation_contract_version", Kind::Int64),
        ("inspection_scope_version", Kind::Int64),
        ("inspection_only", Kind::Bool),
        (
            "accepted_baseline",
            Kind::Pointer(&Kind::Object(shape_baseline)),
        ),
        (
            "sources",
            Kind::Array(&Kind::Object(crate::models::shape_review_source)),
        ),
        ("candidate_commit", Kind::String),
        ("previous_candidate", Kind::String),
        ("diff", Kind::String),
        ("receipt", Kind::Raw),
        ("tasks", Kind::Array(&Kind::Object(shape_task))),
    ]
}
fn shape_task() -> Fields {
    &[
        ("delivery", Kind::Pointer(&Kind::Object(shape_delivery))),
        ("task", Kind::String),
        ("title", Kind::String),
        ("deliverable", Kind::String),
        ("criteria", Kind::Array(&Kind::String)),
        ("report", Kind::String),
        (
            "controls",
            Kind::Array(&Kind::Object(
                crate::models::shape_validation_control_result,
            )),
        ),
        (
            "binding",
            Kind::Object(crate::models::shape_managed_task_review),
        ),
    ]
}
fn shape_baseline() -> Fields {
    &[
        ("version", Kind::Int64),
        ("candidate_commit", Kind::String),
        (
            "accepted_reviews",
            Kind::Array(&Kind::Object(crate::models::shape_independent_review)),
        ),
    ]
}
fn shape_delivery() -> Fields {
    &[
        ("version", Kind::Int64),
        ("task", Kind::String),
        ("attempt", Kind::String),
        ("contract", Kind::String),
        ("outcome", Kind::String),
        ("criteria", Kind::Array(&Kind::Object(shape_criterion))),
    ]
}
fn shape_criterion() -> Fields {
    &[
        ("index", Kind::Int64),
        ("status", Kind::String),
        ("reason", Kind::String),
        ("controls", Kind::Array(&Kind::String)),
        ("evidence", Kind::Array(&Kind::String)),
    ]
}
/// Decode the actual Go review context DTO without turning supplied data into authority.
///
/// UTF-8 input must contain one JSON value. Go case matching, null behavior,
/// duplicate-object/slice updates, nil versus empty and opaque receipt numbers
/// are preserved. `deny_unknown_fields=true` mirrors Go's strict decoder for all
/// structured descendants; false mirrors json.Unmarshal. Opaque receipt contents
/// remain raw JSON in either mode. This performs no file, Git or identity check.
///
/// # Errors
/// Returns [`WorkError`] for malformed UTF-8/JSON, wrong known-field types,
/// nonrepresentable values or unknown structured fields in strict mode.
pub fn decode_managed_review_context(
    raw: &[u8],
    deny_unknown_fields: bool,
) -> Result<ManagedReviewContext, WorkError> {
    crate::work::decode_mode(raw, shape_context(), deny_unknown_fields)
}
