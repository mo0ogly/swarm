//! Preparation read-model projections.
//!
//! These types are stored observations only: they carry no validation
//! semantics and are not an execution authority.

use crate::model_wire::{Fields, Kind};
use crate::{ActionPlan, Budget};
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

/// Stored observation of a prepared document snapshot; not validation or execution authority.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PreparationDocument {
    #[serde(default, rename = "source_turn", skip_serializing_if = "String::is_empty")]
    pub source_turn: String,
    #[serde(default, rename = "kind")]
    pub kind: String,
    #[serde(default, rename = "text")]
    pub text: String,
    #[serde(default, rename = "revision")]
    pub revision: i64,
    #[serde(default, rename = "sha256")]
    pub sha256: String,
    #[serde(default, rename = "at")]
    pub at: String,
    #[serde(default, rename = "brief_hash", skip_serializing_if = "String::is_empty")]
    pub brief_hash: String,
    #[serde(default, rename = "method_hash", skip_serializing_if = "String::is_empty")]
    pub method_hash: String,
    #[serde(default, rename = "need_hash", skip_serializing_if = "String::is_empty")]
    pub need_hash: String,
}

/// Check recorded SHA256 against the exact UTF-8 document bytes. This detects
/// inconsistent stored metadata, not authenticity or confinement of a datastore.
/// Callers decide absence and repair: saving replacement bytes remains allowed.
pub(crate) fn document_digest_matches(doc: &PreparationDocument) -> bool {
    use sha2::{Digest,Sha256};
    doc.sha256==format!("{:x}",Sha256::digest(doc.text.as_bytes()))
}
/// A ready verdict needs coherent current plan/brief and adopted-brief bytes.
/// A missing need remains the historical Go case; a present need must be coherent.
/// No mutation occurs and false does not assert the cause or repair anything.
pub(crate) fn verdict_document_digests_match(p: &Preparation) -> bool {
    let Some(docs)=p.documents.as_ref() else {return false};
    let Some(brief)=docs.get("brief") else {return false};
    let Some(plan)=docs.get("plan") else {return false};
    let Some(adopted)=p.adopted_brief.as_deref() else {return false};
    document_digest_matches(brief) && document_digest_matches(plan)
        && document_digest_matches(adopted)
        && docs.get("besoin").is_none_or(document_digest_matches)
}

/// Stored observation of a flattened preparation source reference; not validation or execution authority.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PreparationSource {
    #[serde(default, rename = "path")]
    pub path: String,
    #[serde(default, rename = "start_line", skip_serializing_if = "is_zero")]
    pub start_line: i64,
    #[serde(default, rename = "end_line", skip_serializing_if = "is_zero")]
    pub end_line: i64,
    #[serde(default, rename = "sha256", skip_serializing_if = "String::is_empty")]
    pub sha256: String,
    #[serde(default, rename = "text")]
    pub text: String,
    #[serde(default, rename = "captured_at")]
    pub captured_at: String,
}

/// Stored observation of a preparation verdict; not validation or execution authority.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PreparationVerdict {
    #[serde(default, rename = "plan_hash")]
    pub plan_hash: String,
    #[serde(default, rename = "brief_hash")]
    pub brief_hash: String,
    #[serde(default, rename = "method_hash")]
    pub method_hash: String,
    #[serde(default, rename = "at")]
    pub at: String,
}

/// Stored observation of a plan conversion; not validation or execution authority.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PreparationConversion {
    #[serde(default, rename = "organization_sha256", skip_serializing_if = "String::is_empty")]
    pub organization_sha256: String,
    #[serde(default, rename = "work_id")]
    pub work_id: String,
    #[serde(default, rename = "plan_hash")]
    pub plan_hash: String,
    #[serde(default, rename = "brief_hash")]
    pub brief_hash: String,
    #[serde(default, rename = "method_hash")]
    pub method_hash: String,
    #[serde(default, rename = "task_ids")]
    pub task_ids: Option<Vec<String>>,
    #[serde(default, rename = "spec")]
    pub spec: ActionPlan,
    #[serde(default, rename = "created_at")]
    pub created_at: String,
    #[serde(default, rename = "released_at", skip_serializing_if = "String::is_empty")]
    pub released_at: String,
}

/// Stored observation of a preparation; not validation or execution authority.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Preparation {
    #[serde(default, rename = "sources", skip_serializing_if = "empty_vec")]
    pub sources: Option<Vec<PreparationSource>>,
    #[serde(default, rename = "budget", skip_serializing_if = "Option::is_none")]
    pub budget: Option<Box<Budget>>,
    #[serde(default, rename = "conversion", skip_serializing_if = "Option::is_none")]
    pub conversion: Option<Box<PreparationConversion>>,
    #[serde(default, rename = "id")]
    pub id: String,
    #[serde(default, rename = "work_id", skip_serializing_if = "String::is_empty")]
    pub work_id: String,
    #[serde(default, rename = "title")]
    pub title: String,
    #[serde(default, rename = "method")]
    pub method: String,
    #[serde(default, rename = "method_hash", skip_serializing_if = "String::is_empty")]
    pub method_hash: String,
    #[serde(default, rename = "revision")]
    pub revision: i64,
    #[serde(default, rename = "created_at")]
    pub created_at: String,
    #[serde(default, rename = "updated_at")]
    pub updated_at: String,
    #[serde(default, rename = "documents")]
    pub documents: Option<BTreeMap<String, PreparationDocument>>,
    #[serde(default, rename = "adopted_brief", skip_serializing_if = "Option::is_none")]
    pub adopted_brief: Option<Box<PreparationDocument>>,
    #[serde(default, rename = "verdict", skip_serializing_if = "Option::is_none")]
    pub verdict: Option<Box<PreparationVerdict>>,
    #[serde(default, rename = "plan_ready")]
    pub plan_ready: bool,
    #[serde(default, rename = "receipt_historical", skip_serializing_if = "is_false")]
    pub receipt_historical: bool,
}

const BUDGET_FIELDS: Fields = &[
    ("limit_usd", Kind::Float64),
    ("reserve_per_launch_usd", Kind::Float64),
    ("estimate_source", Kind::String),
    ("reference_date", Kind::String),
    ("updated", Kind::String),
    ("actor", Kind::String),
];

/// Exact shared Budget wire shape for persisted data and explicit envelope inputs;
/// decoding grants no authority and business validation remains validate_budget.
pub(crate) fn shape_budget() -> Fields {
    BUDGET_FIELDS
}

/// Supported operator source reference, excluding captured text/timestamp. These
/// structured fields preserve Go duplicate merges, scalar-null and case folding.
pub(crate) fn shape_preparation_reference()->Fields { &[
    ("path",Kind::String),("start_line",Kind::Int64),("end_line",Kind::Int64),("sha256",Kind::String),
] }
/// Initial CLI/human organization wire shape, before business validation. The
/// nested object must be projected while original member order is still known.
pub(crate) fn shape_preparation_organization()->Fields { &[
    ("worker_provider",Kind::String),("provider",Kind::String),("level",Kind::String),
    ("model_policy_hash",Kind::String),("workspace",Kind::String),("validation",Kind::String),
    ("max_tasks",Kind::Int64),("max_calls",Kind::Int64),
] }
/// Known Go JSON field metadata; decoding validates types without granting authority.
pub(crate) fn shape_preparation_document() -> Fields {
    &[
        ("source_turn", Kind::String),
        ("kind", Kind::String),
        ("text", Kind::String),
        ("revision", Kind::Int64),
        ("sha256", Kind::String),
        ("at", Kind::String),
        ("brief_hash", Kind::String),
        ("method_hash", Kind::String),
        ("need_hash", Kind::String),
    ]
}

/// Known Go JSON field metadata; decoding validates types without granting authority.
fn shape_preparation_source() -> Fields {
    &[
        ("path", Kind::String),
        ("start_line", Kind::Int64),
        ("end_line", Kind::Int64),
        ("sha256", Kind::String),
        ("text", Kind::String),
        ("captured_at", Kind::String),
    ]
}

/// Known Go JSON field metadata; decoding validates types without granting authority.
fn shape_preparation_verdict() -> Fields {
    &[
        ("plan_hash", Kind::String),
        ("brief_hash", Kind::String),
        ("method_hash", Kind::String),
        ("at", Kind::String),
    ]
}

/// Known Go JSON field metadata; decoding validates types without granting authority.
fn shape_preparation_conversion() -> Fields {
    &[
        ("organization_sha256", Kind::String),
        ("work_id", Kind::String),
        ("plan_hash", Kind::String),
        ("brief_hash", Kind::String),
        ("method_hash", Kind::String),
        ("task_ids", Kind::Array(&Kind::String)),
        ("spec", Kind::Object(crate::models::shape_action_plan)),
        ("created_at", Kind::String),
        ("released_at", Kind::String),
    ]
}

const PREPARATION_FIELDS: Fields = &[
    ("sources", Kind::Array(&Kind::Object(shape_preparation_source))),
    ("budget", Kind::Pointer(&Kind::Object(shape_budget))),
    ("conversion", Kind::Pointer(&Kind::Object(shape_preparation_conversion))),
    ("id", Kind::String),
    ("work_id", Kind::String),
    ("title", Kind::String),
    ("method", Kind::String),
    ("method_hash", Kind::String),
    ("revision", Kind::Int64),
    ("created_at", Kind::String),
    ("updated_at", Kind::String),
    ("documents", Kind::Map(&Kind::Object(shape_preparation_document))),
    ("adopted_brief", Kind::Pointer(&Kind::Object(shape_preparation_document))),
    ("verdict", Kind::Pointer(&Kind::Object(shape_preparation_verdict))),
    ("plan_ready", Kind::Bool),
    ("receipt_historical", Kind::Bool),
];

/// Known Go JSON field metadata; decoding validates types without granting authority.
pub(crate) fn shape_preparation() -> Fields {
    PREPARATION_FIELDS
}

/// Current method catalogue metadata. Availability/hash observes source bytes,
/// never receipt by an agent, plan validation or execution authority.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PreparationMethod {
    pub id: String, pub title: String, pub phases: Vec<String>, pub paths: Vec<String>,
    pub sha256: String, pub available: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub reason: String,
}

/// Recompute the `plan_ready` freshness projection for a stored preparation.
///
/// Mirrors the Go `preparationFreshness`: unconditionally clears
/// `plan_ready` first, then only sets it when a verdict and adopted brief
/// exist, the receipt is not historical, the adopted brief's need hash
/// matches the "besoin" document (a missing key yields the empty string,
/// matching Go's zero map value), the supplied current method hash matches
/// both the verdict and the preparation's stored method hash, and the
/// brief/plan document hashes match the verdict.
///
/// `current_method_hash` is the already-resolved hash of `p.method`; `None`
/// means the method is explicitly unavailable (Go's resolution error), not
/// silently-ignored IO. This function performs no I/O, accepts no external
/// input, and mutates nothing except `plan_ready`; it is not an execution
/// or validation authority.
pub(crate) fn apply_freshness(
    p: &mut crate::preparation_models::Preparation,
    current_method_hash: Option<&str>,
) {
    p.plan_ready = false;
    let besoin_hash = p
        .documents
        .as_ref()
        .and_then(|d| d.get("besoin"))
        .map(|d| d.sha256.as_str())
        .unwrap_or("");
    let (verdict, brief) = match (&p.verdict, &p.adopted_brief) {
        (Some(v), Some(b)) if !p.receipt_historical && b.need_hash == besoin_hash => (v, b),
        _ => return,
    };
    let current = match current_method_hash {
        Some(h) => h,
        None => return,
    };
    let doc = |key: &str| {
        p.documents
            .as_ref()
            .and_then(|d| d.get(key))
            .map(|d| d.sha256.as_str())
            .unwrap_or("")
    };
    p.plan_ready = current == verdict.method_hash
        && p.method_hash == verdict.method_hash
        && brief.sha256 == verdict.brief_hash
        && doc("brief") == verdict.brief_hash
        && doc("plan") == verdict.plan_hash;
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Go's ordered known fields, null map/zero bool spelling and malformed
    /// values remain distinct; these DTOs establish no validation authority.
    #[test]
    fn preparation_codec_go_fields_and_false_presence() {
        let decode = |raw: &[u8]| crate::work::decode::<Preparation>(raw, shape_preparation());
        let p = decode(br#"{"id":"p-one","plan_ready":true,"PLAN_READY":null,"documents":null}"#).unwrap();
        assert!(p.plan_ready);
        let empty = decode(br#"{}"#).unwrap();
        let wire = crate::encode_model(&empty).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&wire).unwrap();
        assert_eq!(value["plan_ready"], false);
        assert!(value["documents"].is_null());
        assert!(value.get("receipt_historical").is_none());
        assert!(decode(br#"{"id":1}"#).is_err());
        assert!(decode(br#"{"documents":{"plan":{"revision":"bad"}}}"#).is_err());
        assert_eq!(decode(br#"{"id":"first","ID":"last"}"#).unwrap().id, "last");
    }
    /// Fresh declared hashes witness the projection before unavailable/missing,
    /// changed need/method and historical-receipt variants. No text is accepted.
    #[test]
    fn preparation_freshness_current_stale_and_historical() {
        let document = |sha: &str| PreparationDocument { sha256: sha.into(), ..Default::default() };
        let mut p = Preparation { method_hash: "method".into(), documents: Some(BTreeMap::from([
            ("besoin".into(), document("need")), ("brief".into(), document("brief")), ("plan".into(), document("plan"))])),
            adopted_brief: Some(Box::new(PreparationDocument { need_hash: "need".into(), ..document("brief") })),
            verdict: Some(Box::new(PreparationVerdict { method_hash: "method".into(), brief_hash: "brief".into(), plan_hash: "plan".into(), ..Default::default() })), ..Default::default() };
        apply_freshness(&mut p, Some("method")); assert!(p.plan_ready);
        apply_freshness(&mut p, None); assert!(!p.plan_ready);
        apply_freshness(&mut p, Some("changed")); assert!(!p.plan_ready);
        apply_freshness(&mut p, Some("method")); assert!(p.plan_ready);
        p.receipt_historical = true; apply_freshness(&mut p, Some("method")); assert!(!p.plan_ready);
        p.receipt_historical = false; p.documents.as_mut().unwrap().remove("besoin");
        apply_freshness(&mut p, Some("method")); assert!(!p.plan_ready);
    }
    /// Current defaults contain the v41 read group; a genuine v40 fixture cannot
    /// silently acquire new bounds. Partial/negative groups fail authoritative decode.
    #[test]
    fn preparation_policy_explicit_defaults_legacy_and_unlimited() {
        let values = crate::EngineSettingsValues::installation_defaults().unwrap();
        assert!(values.schema_version() >= 41);
        assert_eq!(values.preparation_read_policy().unwrap(), crate::PreparationReadPolicy {
            list_limit: Some(100), history_limit: Some(50), method_max_bytes: Some(131072) });
        let mut raw = serde_json::to_value(crate::engine_settings::historical_settings_fixture(40)).unwrap();
        for key in ["preparation_list_limit", "preparation_history_limit", "preparation_method_max_bytes"] { raw.as_object_mut().unwrap().remove(key); }
        let legacy = crate::EngineSettingsValues::decode(&serde_json::to_vec(&raw).unwrap()).unwrap();
        assert_eq!(legacy.schema_version(), 40); assert!(legacy.preparation_read_policy().is_err());
        raw["preparation_list_limit"] = serde_json::Value::Null;
        assert!(crate::EngineSettingsValues::decode(&serde_json::to_vec(&raw).unwrap()).is_err());
        raw["preparation_history_limit"] = serde_json::Value::Null; raw["preparation_method_max_bytes"] = serde_json::Value::Null;
        let unlimited = crate::EngineSettingsValues::decode(&serde_json::to_vec(&raw).unwrap()).unwrap();
        assert_eq!(unlimited.preparation_read_policy().unwrap().list_limit, None);
        raw["preparation_list_limit"] = "-1".into();
        assert!(crate::EngineSettingsValues::decode(&serde_json::to_vec(&raw).unwrap()).is_err());
    }
}
