//! Bounded Go-compatible create/save intent; no authority or persistence.
use crate::model_wire::{Fields, Kind};
use serde::{Deserialize, Serialize};
/// Supported draft intention. expected_revision is required/non-null. Action and
/// ID are supplied explicitly; adapters must not invent either. Unsupported
/// nonzero preparation transitions/fields refuse rather than disappearing;
/// zero-valued legacy fields retain Go canonical omission.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct PreparationCommand {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub adopt_brief: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_work_revision: Option<i64>,
    #[serde(default, skip_serializing_if = "empty_decisions")]
    pub decisions: Option<Vec<serde_json::Value>>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub turn_id: String,
    pub version: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub preparation_id: String,
    pub event_id: String,
    pub action: String,
    pub expected_revision: Option<i64>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub work_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub method: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub document: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sha256: String,
}
/// Exact supported wire shape. Numeric/name lengths are protocol representation,
/// while text/title bounds come exclusively from the persisted editing policy.
fn shape() -> Fields {
    &[
        ("organization", Kind::Pointer(&Kind::Raw)),
        ("source", Kind::Pointer(&Kind::Raw)),
        ("budget", Kind::Pointer(&Kind::Raw)),
        ("adopt_brief", Kind::Bool),
        ("expected_work_revision", Kind::Pointer(&Kind::Int64)),
        ("decisions", Kind::Array(&Kind::Raw)),
        ("turn_id", Kind::String),
        ("version", Kind::Int64),
        ("preparation_id", Kind::String),
        ("event_id", Kind::String),
        ("action", Kind::String),
        ("expected_revision", Kind::Pointer(&Kind::Int64)),
        ("work_id", Kind::String),
        ("title", Kind::String),
        ("method", Kind::String),
        ("document", Kind::String),
        ("text", Kind::String),
        ("sha256", Kind::String),
    ]
}
/// Decode Go case-folding/null/duplicate semantics with unknown fields refused.
/// Typed UTF8/JSON failures preserve their cause; no IO or state change occurs.
pub(crate) fn decode(raw: &[u8]) -> Result<PreparationCommand, crate::WorkError> {
    crate::work::decode_mode(raw, shape(), true)
}
/// Verify identity, revision and action-specific fields using supplied byte caps.
/// Null caps mean unlimited; no fallback, truncation, provider call or adoption.
pub(crate) fn validate(
    r: &PreparationCommand,
    document_cap: Option<usize>,
    title_cap: Option<usize>,
) -> Result<(), crate::RootStoreError> {
    use crate::RootStoreError as E;
    if r.organization.is_some()
        || r.source.is_some()
        || r.budget.is_some()
        || r.adopt_brief
        || r.expected_work_revision.is_some()
        || !empty_decisions(&r.decisions)
        || !r.turn_id.is_empty()
        || !r.sha256.is_empty()
        || r.version != 1
        || !key(&r.event_id)
        || r.expected_revision.is_none_or(|n| n < 0)
        || (!r.work_id.is_empty() && !key(&r.work_id))
        || document_cap.is_some_and(|n| r.text.len() > n)
        || title_cap.is_some_and(|n| r.title.len() > n)
    {
        return Err(E::Refused("preparation-request-invalid"));
    }
    match r.action.as_str() {
        "create"
            if r.preparation_id.is_empty()
                && r.expected_revision == Some(0)
                && r.document.is_empty() =>
        {
            Ok(())
        }
        "save"
            if key(&r.preparation_id)
                && matches!(r.document.as_str(), "besoin" | "brief" | "plan")
                && r.work_id.is_empty()
                && r.title.is_empty()
                && r.method.is_empty() =>
        {
            Ok(())
        }
        _ => Err(E::Refused("preparation-request-invalid")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Decode plus supplied fixture caps, identical to the public writer path.
    fn checked(
        raw: &[u8],
        cap: Option<usize>,
    ) -> Result<PreparationCommand, crate::RootStoreError> {
        let r = crate::decode_preparation_command(raw)?;
        validate(&r, cap, Some(240))?;
        Ok(r)
    }
    /// Pure Go-wire merge/canonical-byte witness; no IO or authority.
    #[test]
    fn glm_go_codec_canonical_order_and_merge() {
        let raw=br#"{"Version":1,"event_id":"evt-1","action":"create","expected_revision":0,"title":"a","title":"b","sha256":""}"#;
        let r = checked(raw, Some(16000)).unwrap();
        assert_eq!(r.title, "b");
        assert_eq!(crate::encode_model(&r).unwrap(),br#"{"version":1,"event_id":"evt-1","action":"create","expected_revision":0,"title":"b"}"#);
        let merged=checked(br#"{"version":1,"event_id":"evt-1","action":"create","expected_revision":0,"title":"a","title":null}"#,None).unwrap();
        assert_eq!(merged.title, "a");
    }
    /// Malformed/foreign requests refuse before storage; fixture inputs only.
    #[test]
    fn glm_refuses_missing_null_revision_types_and_foreign_fields() {
        for field in [
            "",
            ",\"expected_revision\":null",
            ",\"expected_revision\":-1",
            ",\"expected_revision\":\"0\"",
            ",\"expected_revision\":99999999999999999999",
            ",\"expected_revision\":0,\"source\":{}",
            ",\"expected_revision\":0,\"adopt_brief\":true",
            ",\"expected_revision\":0,\"wat\":1",
        ] {
            let raw =
                format!("{{\"version\":1,\"event_id\":\"evt-1\",\"action\":\"create\"{field}}}");
            assert!(checked(raw.as_bytes(), None).is_err(), "{raw}");
        }
    }
    /// Caller byte caps and protocol names are distinct; no saved policy changes.
    #[test]
    fn glm_exact_byte_caps_and_protocol_keys() {
        let mut r = checked(
            br#"{"version":1,"event_id":"evt-1","action":"create","expected_revision":0}"#,
            None,
        )
        .unwrap();
        r.text = "é".into();
        assert!(validate(&r, Some(1), None).is_err());
        assert!(validate(&r, Some(2), None).is_ok());
        r.event_id = "a".repeat(128);
        assert!(validate(&r, None, None).is_ok());
        r.event_id.push('a');
        assert!(validate(&r, None, None).is_err());
        r.event_id = "é".into();
        assert!(validate(&r, None, None).is_err());
        r.event_id = "a".into();
        r.action = "save".into();
        r.preparation_id = "prep-x".into();
        r.document = "plan".into();
        r.title = "Forbidden".into();
        assert!(validate(&r, None, None).is_err());
        r.title.clear();
        assert!(validate(&r, None, None).is_ok());
    }
}

/// Go omitempty for an explicitly false protocol field; no operator policy.
fn is_false(value: &bool) -> bool {
    !*value
}
/// Go omitempty for absent/empty decisions; nonempty decisions are unsupported.
fn empty_decisions(value: &Option<Vec<serde_json::Value>>) -> bool {
    value.as_ref().is_none_or(Vec::is_empty)
}
/// Exact preparation protocol name: 1..=128 ASCII bytes. This deliberately
/// differs from the generic agent identity name's 100-byte representation.
pub(crate) fn key(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}
