//! Generated from internal/engine/model.go by generate_request_models.py.
//! Data only: typed origin/policy fields grant no authority.
// Go model SHA-256: 9b8f2f15d6af6c046cc3b7c909a4a8bf3155b9dd1e6c4ce04612758a763c9086
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use swarm_wire_json::projection::{Fields, Kind};
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
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ValidationControl {
    // Go EnvironmentExitCodes, model.go:49
    #[serde(skip_serializing_if = "empty_vec")]
    pub environment_exit_codes: Option<Vec<i64>>,
    // Go ReviewOutput, model.go:50
    #[serde(skip_serializing_if = "is_false")]
    pub review_output: bool,
    // Go Inputs, model.go:51
    #[serde(skip_serializing_if = "empty_vec")]
    pub inputs: Option<Vec<String>>,
    // Go ID, model.go:52
    pub id: String,
    // Go Command, model.go:53
    pub command: Option<Vec<String>>,
    // Go Criteria, model.go:54
    pub criteria: Option<Vec<i64>>,
    // Go Justification, model.go:55
    pub justification: String,
    // Go Dir, model.go:56
    #[serde(skip_serializing_if = "String::is_empty")]
    pub dir: String,
    // Go Timeout, model.go:57
    #[serde(skip_serializing_if = "is_zero")]
    pub timeout_seconds: i64,
}
pub(crate) const FIELDS_VALIDATION_CONTROL: Fields = &[
    (
        "environment_exit_codes",
        Kind::Array {
            element: &Kind::Int64,
            nil: true,
        },
    ),
    ("review_output", Kind::Bool),
    (
        "inputs",
        Kind::Array {
            element: &Kind::String,
            nil: true,
        },
    ),
    ("id", Kind::String),
    (
        "command",
        Kind::Array {
            element: &Kind::String,
            nil: true,
        },
    ),
    (
        "criteria",
        Kind::Array {
            element: &Kind::Int64,
            nil: true,
        },
    ),
    ("justification", Kind::String),
    ("dir", Kind::String),
    ("timeout_seconds", Kind::Int64),
];
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ValidationPolicy {
    // Go Mode, model.go:42
    pub mode: String,
    // Go Controls, model.go:43
    pub controls: Option<Vec<ValidationControl>>,
    // Go Authorized, model.go:44
    #[serde(skip_serializing_if = "String::is_empty")]
    pub authorized_at: String,
    // Go Actor, model.go:45
    #[serde(skip_serializing_if = "String::is_empty")]
    pub actor: String,
}
pub(crate) const FIELDS_VALIDATION_POLICY: Fields = &[
    ("mode", Kind::String),
    (
        "controls",
        Kind::Array {
            element: &Kind::Object(FIELDS_VALIDATION_CONTROL),
            nil: true,
        },
    ),
    ("authorized_at", Kind::String),
    ("actor", Kind::String),
];
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Request {
    // Go RequirementPrerequisites, model.go:238
    #[serde(skip_serializing_if = "empty_map")]
    pub requirement_prerequisites: Option<BTreeMap<String, Option<Vec<String>>>>,
    // Go ConfirmContractRevision, model.go:239
    #[serde(skip_serializing_if = "is_false")]
    pub confirm_contract_revision: bool,
    // Go ExpectedContract, model.go:240
    #[serde(skip_serializing_if = "String::is_empty")]
    pub expected_contract: String,
    // Go ContractRevisionReason, model.go:241
    #[serde(skip_serializing_if = "String::is_empty")]
    pub contract_revision_reason: String,
    // Go MaxAttempts, model.go:242
    #[serde(skip_serializing_if = "is_zero")]
    pub max_attempts: i64,
    // Go MaxToolCalls, model.go:243
    #[serde(skip_serializing_if = "is_zero")]
    pub max_tool_calls: i64,
    // Go Schema, model.go:244
    pub schema_version: i64,
    // Go EventID, model.go:245
    pub event_id: String,
    // Go Revision, model.go:246
    pub expected_revision: i64,
    // Go ID, model.go:247
    #[serde(skip_serializing_if = "String::is_empty")]
    pub id: String,
    // Go Title, model.go:248
    #[serde(skip_serializing_if = "String::is_empty")]
    pub title: String,
    // Go Objective, model.go:249
    #[serde(skip_serializing_if = "String::is_empty")]
    pub objective: String,
    // Go Scope, model.go:250
    #[serde(skip_serializing_if = "String::is_empty")]
    pub scope: String,
    // Go Criteria, model.go:251
    #[serde(skip_serializing_if = "empty_vec")]
    pub criteria: Option<Vec<String>>,
    // Go Owner, model.go:252
    #[serde(skip_serializing_if = "String::is_empty")]
    pub owner: String,
    // Go Deliverable, model.go:253
    #[serde(skip_serializing_if = "String::is_empty")]
    pub deliverable: String,
    // Go Depends, model.go:254
    #[serde(skip_serializing_if = "empty_vec")]
    pub depends: Option<Vec<String>>,
    // Go Status, model.go:255
    #[serde(skip_serializing_if = "String::is_empty")]
    pub status: String,
    // Go Blocker, model.go:256
    #[serde(skip_serializing_if = "String::is_empty")]
    pub blocker: String,
    // Go Next, model.go:257
    #[serde(skip_serializing_if = "String::is_empty")]
    pub next: String,
    // Go Summary, model.go:258
    #[serde(skip_serializing_if = "String::is_empty")]
    pub summary: String,
    // Go Memory, model.go:259
    #[serde(skip_serializing_if = "empty_vec")]
    pub memory: Option<Vec<String>>,
    // Go Outcome, model.go:260
    #[serde(skip_serializing_if = "String::is_empty")]
    pub outcome: String,
    // Go ValidationPolicy, model.go:261
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_policy: Option<ValidationPolicy>,
    // Go Observation, model.go:262
    #[serde(skip_serializing_if = "String::is_empty")]
    pub observation: String,
    // Go Orientation, model.go:263
    #[serde(skip_serializing_if = "String::is_empty")]
    pub orientation: String,
    // Go Decision, model.go:264
    #[serde(skip_serializing_if = "String::is_empty")]
    pub decision: String,
    // Go Result, model.go:265
    #[serde(skip_serializing_if = "String::is_empty")]
    pub result: String,
    // Go Origin, model.go:273
    #[serde(skip_serializing_if = "String::is_empty")]
    pub origin: String,
}
pub(crate) const FIELDS_REQUEST: Fields = &[
    (
        "requirement_prerequisites",
        Kind::Map(&Kind::Array {
            element: &Kind::String,
            nil: true,
        }),
    ),
    ("confirm_contract_revision", Kind::Bool),
    ("expected_contract", Kind::String),
    ("contract_revision_reason", Kind::String),
    ("max_attempts", Kind::Int64),
    ("max_tool_calls", Kind::Int64),
    ("schema_version", Kind::Int64),
    ("event_id", Kind::String),
    ("expected_revision", Kind::Int64),
    ("id", Kind::String),
    ("title", Kind::String),
    ("objective", Kind::String),
    ("scope", Kind::String),
    (
        "criteria",
        Kind::Array {
            element: &Kind::String,
            nil: true,
        },
    ),
    ("owner", Kind::String),
    ("deliverable", Kind::String),
    (
        "depends",
        Kind::Array {
            element: &Kind::String,
            nil: true,
        },
    ),
    ("status", Kind::String),
    ("blocker", Kind::String),
    ("next", Kind::String),
    ("summary", Kind::String),
    (
        "memory",
        Kind::Array {
            element: &Kind::String,
            nil: true,
        },
    ),
    ("outcome", Kind::String),
    (
        "validation_policy",
        Kind::OptionalObject(FIELDS_VALIDATION_POLICY),
    ),
    ("observation", Kind::String),
    ("orientation", Kind::String),
    ("decision", Kind::String),
    ("result", Kind::String),
    ("origin", Kind::String),
];
// Request.MarshalJSON in task_definition.go appends an explicit empty depends
// after the legacy payload; nonempty depends retains its ordinary field order.
impl Serialize for Request {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_map(None)?;
        if !empty_map(&self.requirement_prerequisites) {
            out.serialize_entry("requirement_prerequisites", &self.requirement_prerequisites)?;
        }
        if !is_false(&self.confirm_contract_revision) {
            out.serialize_entry("confirm_contract_revision", &self.confirm_contract_revision)?;
        }
        if !String::is_empty(&self.expected_contract) {
            out.serialize_entry("expected_contract", &self.expected_contract)?;
        }
        if !String::is_empty(&self.contract_revision_reason) {
            out.serialize_entry("contract_revision_reason", &self.contract_revision_reason)?;
        }
        if !is_zero(&self.max_attempts) {
            out.serialize_entry("max_attempts", &self.max_attempts)?;
        }
        if !is_zero(&self.max_tool_calls) {
            out.serialize_entry("max_tool_calls", &self.max_tool_calls)?;
        }
        out.serialize_entry("schema_version", &self.schema_version)?;
        out.serialize_entry("event_id", &self.event_id)?;
        out.serialize_entry("expected_revision", &self.expected_revision)?;
        if !String::is_empty(&self.id) {
            out.serialize_entry("id", &self.id)?;
        }
        if !String::is_empty(&self.title) {
            out.serialize_entry("title", &self.title)?;
        }
        if !String::is_empty(&self.objective) {
            out.serialize_entry("objective", &self.objective)?;
        }
        if !String::is_empty(&self.scope) {
            out.serialize_entry("scope", &self.scope)?;
        }
        if !empty_vec(&self.criteria) {
            out.serialize_entry("criteria", &self.criteria)?;
        }
        if !String::is_empty(&self.owner) {
            out.serialize_entry("owner", &self.owner)?;
        }
        if !String::is_empty(&self.deliverable) {
            out.serialize_entry("deliverable", &self.deliverable)?;
        }
        if !empty_vec(&self.depends) {
            out.serialize_entry("depends", &self.depends)?;
        }
        if !String::is_empty(&self.status) {
            out.serialize_entry("status", &self.status)?;
        }
        if !String::is_empty(&self.blocker) {
            out.serialize_entry("blocker", &self.blocker)?;
        }
        if !String::is_empty(&self.next) {
            out.serialize_entry("next", &self.next)?;
        }
        if !String::is_empty(&self.summary) {
            out.serialize_entry("summary", &self.summary)?;
        }
        if !empty_vec(&self.memory) {
            out.serialize_entry("memory", &self.memory)?;
        }
        if !String::is_empty(&self.outcome) {
            out.serialize_entry("outcome", &self.outcome)?;
        }
        if !Option::is_none(&self.validation_policy) {
            out.serialize_entry("validation_policy", &self.validation_policy)?;
        }
        if !String::is_empty(&self.observation) {
            out.serialize_entry("observation", &self.observation)?;
        }
        if !String::is_empty(&self.orientation) {
            out.serialize_entry("orientation", &self.orientation)?;
        }
        if !String::is_empty(&self.decision) {
            out.serialize_entry("decision", &self.decision)?;
        }
        if !String::is_empty(&self.result) {
            out.serialize_entry("result", &self.result)?;
        }
        if !String::is_empty(&self.origin) {
            out.serialize_entry("origin", &self.origin)?;
        }
        if let Some(depends) = self.depends.as_ref().filter(|v| v.is_empty()) {
            out.serialize_entry("depends", depends)?;
        }
        out.end()
    }
}
