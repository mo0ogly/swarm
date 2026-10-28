//! Field applicability is a versioned API protocol, not an operator policy.
use crate::{Request, Work};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Contract {
    schema_version: u64,
    envelope: Vec<String>,
    operations: BTreeMap<String, Vec<String>>,
    conditions: BTreeMap<String, BTreeMap<String, String>>,
}
/// Safe applicability refusal or a retained trusted protocol/encoding cause.
/// No private request value is displayed and no Work, SQL or authority changes.
#[derive(Debug)]
pub enum RequestFieldsError {
    Unsupported,
    Contract,
    Json(serde_json::Error),
    Encoding(crate::Error),
}
impl RequestFieldsError {
    /// Stable caller-facing class; malformed embedded contracts fail closed.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unsupported => "unsupported_parameter",
            _ => "request_contract_invalid",
        }
    }
    /// Static safe diagnostic; nested causes remain available through Error::source.
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Unsupported => "request-parameter-not-applicable",
            _ => "request-field-contract-invalid",
        }
    }
}
impl fmt::Display for RequestFieldsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.reason())
    }
}
impl Error for RequestFieldsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Json(e) => Some(e),
            Self::Encoding(e) => Some(e),
            _ => None,
        }
    }
}
/// Check meaningful fields after event replay/revision and before new edits.
/// Empty scalars retain legacy omission semantics; explicit empty collections
/// remain meaningful. Conditional intent uses the stored Work/task; this check
/// grants no transition, hierarchy or execution rights and performs no IO.
/// Recorded receipts must replay before this call to preserve their history.
pub(crate) fn validate(w: &Work, kind: &str, r: &Request) -> Result<(), RequestFieldsError> {
    let c: Contract = serde_json::from_slice(include_bytes!(
        "../../../../contracts/work-request-fields.v1.json"
    ))
    .map_err(RequestFieldsError::Json)?;
    let known: BTreeSet<&str> = crate::request::FIELDS_REQUEST
        .iter()
        .map(|(k, _)| *k)
        .collect();
    let mut covered = BTreeSet::new();
    let mut valid = c.schema_version == 1
        && c.operations.len() == 6
        && c.envelope == ["schema_version", "event_id", "expected_revision", "origin"]
        && c.conditions.len() == 1;
    for f in &c.envelope {
        valid = valid && known.contains(f.as_str()) && covered.insert(f.as_str());
    }
    for op in [
        "work.create",
        "work.update",
        "task.add",
        "task.update",
        "checkpoint",
        "ooda",
    ] {
        let fields = c.operations.get(op).ok_or(RequestFieldsError::Contract)?;
        let mut seen = BTreeSet::new();
        valid = valid && !fields.is_empty();
        for f in fields {
            valid = valid && known.contains(f.as_str()) && seen.insert(f.as_str());
            covered.insert(f.as_str());
        }
    }
    let conditions = c
        .conditions
        .get("task.update")
        .ok_or(RequestFieldsError::Contract)?;
    let expected = BTreeMap::from([
        ("outcome".to_owned(), "exits-running-attempt".to_owned()),
        (
            "confirm_contract_revision".to_owned(),
            "hierarchical-definition-amendment".to_owned(),
        ),
        (
            "expected_contract".to_owned(),
            "hierarchical-definition-amendment".to_owned(),
        ),
        (
            "contract_revision_reason".to_owned(),
            "hierarchical-definition-amendment".to_owned(),
        ),
    ]);
    if !valid || covered != known || conditions != &expected {
        return Err(RequestFieldsError::Contract);
    }
    let encoded = crate::encode_work_mutation_intent(r).map_err(RequestFieldsError::Encoding)?;
    let fields: BTreeMap<String, serde_json::Value> =
        serde_json::from_slice(&encoded).map_err(RequestFieldsError::Json)?;
    let selected = c.operations.get(kind).ok_or(RequestFieldsError::Contract)?;
    for field in fields.keys() {
        let mut allowed = c.envelope.contains(field) || selected.contains(field);
        if kind == "task.update" {
            match conditions.get(field).map(String::as_str) {
                Some("exits-running-attempt") => {
                    allowed = allowed
                        && !r.status.is_empty()
                        && r.status != "running"
                        && w.tasks
                            .iter()
                            .flatten()
                            .any(|t| t.id == r.id && t.status == "running")
                }
                Some("hierarchical-definition-amendment") => {
                    allowed = allowed
                        && w.planning.is_some()
                        && crate::mutation_guard::edits_definition(r)
                }
                _ => {}
            }
        }
        if !allowed {
            return Err(RequestFieldsError::Unsupported);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Pure predicate controls complement transactional/public tests; no IO or
    /// authentication is implied by these fixture Work/task values.
    #[test]
    fn conditional_fields_and_explicit_empty_collections_keep_their_intent() {
        let mut w = Work {
            tasks: Some(vec![crate::Task {
                id: "t".into(),
                status: "running".into(),
                ..Default::default()
            }]),
            ..Default::default()
        };
        let good = Request {
            id: "t".into(),
            status: "todo".into(),
            outcome: "interrupted".into(),
            ..Default::default()
        };
        validate(&w, "task.update", &good).unwrap();
        for bad in [
            Request {
                status: String::new(),
                ..good.clone()
            },
            Request {
                status: "running".into(),
                ..good.clone()
            },
            Request {
                id: "t".into(),
                confirm_contract_revision: true,
                ..Default::default()
            },
            Request {
                id: "t".into(),
                expected_contract: "PRIVATE".into(),
                ..Default::default()
            },
            Request {
                id: "t".into(),
                contract_revision_reason: "PRIVATE".into(),
                ..Default::default()
            },
        ] {
            assert_eq!(
                validate(&w, "task.update", &bad).unwrap_err().code(),
                "unsupported_parameter"
            );
        }
        w.tasks.as_mut().unwrap()[0].status = "todo".into();
        assert!(validate(&w, "task.update", &good).is_err());
        validate(
            &w,
            "checkpoint",
            &Request {
                summary: "Saved".into(),
                ..Default::default()
            },
        )
        .unwrap();
        for bad in [
            Request {
                criteria: Some(vec![]),
                ..Default::default()
            },
            Request {
                depends: Some(vec![]),
                ..Default::default()
            },
            Request {
                requirement_prerequisites: Some(BTreeMap::new()),
                ..Default::default()
            },
        ] {
            assert_eq!(
                validate(&w, "checkpoint", &bad).unwrap_err().code(),
                "unsupported_parameter"
            );
        }
        assert!(validate(
            &w,
            "task.add",
            &Request {
                max_attempts: -1,
                ..Default::default()
            }
        )
        .is_err());
    }
}
