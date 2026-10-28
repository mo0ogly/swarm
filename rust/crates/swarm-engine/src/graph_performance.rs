//! Pure Go-wire graph measurement objectives. No renderer tuning, provider,
//! file/Root access or execution authority. Defaults are supplied versioned bytes.
use crate::{
    model_wire::{Fields, Kind},
    AgentLaunchError as E,
};
use serde::{Deserialize, Serialize};
/// Largest exact JSON/JavaScript integer; representation, not operator policy.
pub const GRAPH_EXACT_INTEGER_MAX: i64 = (1i64 << 53) - 1;
/// One measured graph load: at least two cards define this graph recipe and a
/// positive render p95 target. No elapsed result or optimization is implied.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphPerformanceLoad {
    pub cards: i64,
    pub render_p95_ms: i64,
}
/// Saved objectives and sample count. Go nil loads remain None; only current/
/// historical values validation requires at least one unique load.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphPerformanceValues {
    pub keyboard_p95_ms: i64,
    pub samples: i64,
    pub loads: Option<Vec<GraphPerformanceLoad>>,
}
/// Recorded Go change. Metadata header is validated only for new submissions;
/// invalid historical headers remain typed data, never authenticated authority.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphPerformanceChange {
    pub schema_version: i64,
    pub event_id: String,
    pub expected_revision: i64,
    pub values: GraphPerformanceValues,
    pub reason: String,
}
/// One historical measurement policy; actor/time are unverified observations.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphPerformanceRevision {
    pub revision: i64,
    pub values: GraphPerformanceValues,
    pub actor: String,
    pub at: String,
    pub change: GraphPerformanceChange,
}
/// Compatible schema1 file, including Go nil/empty history distinction.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphPerformanceConfig {
    pub schema_version: i64,
    pub revision: i64,
    pub values: GraphPerformanceValues,
    pub history: Option<Vec<GraphPerformanceRevision>>,
}
/// Native transport with a nonpersisted optional freshness token. Token is not
/// permission; callers own custody, concurrency, publication and error effects.
#[derive(Clone, Default, Deserialize)]
#[serde(default)]
pub struct ApiGraphPerformanceChange {
    pub schema_version: i64,
    pub event_id: String,
    pub expected_revision: i64,
    pub values: GraphPerformanceValues,
    pub reason: String,
    preview_token: String,
}
impl ApiGraphPerformanceChange {
    /// Borrow the opaque transport token; never serialize or treat it as rights.
    pub fn preview_token(&self) -> &str {
        &self.preview_token
    }
    /// Recover the full Go intent with token structurally excluded from history.
    pub fn recorded_change(&self) -> GraphPerformanceChange {
        GraphPerformanceChange {
            schema_version: self.schema_version,
            event_id: self.event_id.clone(),
            expected_revision: self.expected_revision,
            values: self.values.clone(),
            reason: self.reason.clone(),
        }
    }
}
impl std::fmt::Debug for ApiGraphPerformanceChange {
    /// Keep private freshness material out of diagnostic formatting.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiGraphPerformanceChange")
            .field("schema_version", &self.schema_version)
            .field("expected_revision", &self.expected_revision)
            .field("preview_token", &"[redacted]")
            .finish_non_exhaustive()
    }
}
/// Shared Go model shapes: scalar null/duplicate/case semantics stay with the
/// established projector; no recursive ad hoc JSON Value/default implementation.
fn load_shape() -> Fields {
    &[("cards", Kind::Int64), ("render_p95_ms", Kind::Int64)]
}
fn values_shape() -> Fields {
    &[
        ("keyboard_p95_ms", Kind::Int64),
        ("samples", Kind::Int64),
        ("loads", Kind::Array(&Kind::Object(load_shape))),
    ]
}
fn change_shape() -> Fields {
    &[
        ("schema_version", Kind::Int64),
        ("event_id", Kind::String),
        ("expected_revision", Kind::Int64),
        ("values", Kind::Object(values_shape)),
        ("reason", Kind::String),
    ]
}
fn revision_shape() -> Fields {
    &[
        ("revision", Kind::Int64),
        ("values", Kind::Object(values_shape)),
        ("actor", Kind::String),
        ("at", Kind::String),
        ("change", Kind::Object(change_shape)),
    ]
}
fn config_shape() -> Fields {
    &[
        ("schema_version", Kind::Int64),
        ("revision", Kind::Int64),
        ("values", Kind::Object(values_shape)),
        ("history", Kind::Array(&Kind::Object(revision_shape))),
    ]
}
fn api_shape() -> Fields {
    &[
        ("schema_version", Kind::Int64),
        ("event_id", Kind::String),
        ("expected_revision", Kind::Int64),
        ("values", Kind::Object(values_shape)),
        ("reason", Kind::String),
        ("preview_token", Kind::String),
    ]
}
/// Validate recipe invariants and exact representation, not resource/time policy.
/// No mutation or measured performance verdict. Repeated card counts refuse.
pub fn validate_graph_performance_values(v: &GraphPerformanceValues) -> Result<(), E> {
    let representable = |n, minimum| n >= minimum && n <= GRAPH_EXACT_INTEGER_MAX;
    if !representable(v.keyboard_p95_ms, 1) || !representable(v.samples, 1) {
        return Err(E::Refused("graph-performance-values-invalid"));
    }
    let loads = v.loads.as_deref().unwrap_or(&[]);
    if loads.is_empty()
        || i64::try_from(loads.len())
            .ok()
            .is_none_or(|n| n > GRAPH_EXACT_INTEGER_MAX)
    {
        return Err(E::Refused("graph-performance-loads-invalid"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for row in loads {
        if !representable(row.cards, 2)
            || !representable(row.render_p95_ms, 1)
            || !seen.insert(row.cards)
        {
            return Err(E::Refused("graph-performance-loads-invalid"));
        }
    }
    Ok(())
}
/// Validate schema/revision/sequence/current==last and actual historical values.
/// No extra historical Change header, actor/time, unique-event or reason gate.
pub fn validate_graph_performance_config(c: &GraphPerformanceConfig) -> Result<(), E> {
    let rows = c.history.as_deref().unwrap_or(&[]);
    if c.schema_version != 1 || c.revision < 0 || i64::try_from(rows.len()).ok() != Some(c.revision)
    {
        return Err(E::Refused("graph-performance-config-invalid"));
    }
    validate_graph_performance_values(&c.values)?;
    for (i, row) in rows.iter().enumerate() {
        if i64::try_from(i).ok().and_then(|n| n.checked_add(1)) != Some(row.revision) {
            return Err(E::Refused("graph-performance-history-invalid"));
        }
        validate_graph_performance_values(&row.values)?;
    }
    if rows.last().is_some_and(|row| row.values != c.values) {
        return Err(E::Refused("graph-performance-history-invalid"));
    }
    Ok(())
}
/// New request header plus recipe values; no old Go8..2000 reason cutoff.
/// Storage caps, CAS/replay and freshness remain the authoritative Root's job.
pub fn validate_graph_performance_change(c: &GraphPerformanceChange) -> Result<(), E> {
    if c.schema_version != 1
        || c.expected_revision < 0
        || !swarm_identity_contract::name(c.event_id.as_bytes())
        || crate::text::trim_space(&c.reason).is_empty()
    {
        return Err(E::Refused("graph-performance-change-invalid"));
    }
    validate_graph_performance_values(&c.values)
}
/// Strict Go-compatible supplied default bytes; malformed data never fallback.
pub fn decode_graph_performance_defaults(raw: &[u8]) -> Result<GraphPerformanceValues, E> {
    let v = crate::work::decode_mode(raw, values_shape(), true)
        .map_err(|e| E::caused("graph-performance-defaults-invalid", e))?;
    validate_graph_performance_values(&v)?;
    Ok(v)
}
/// Strict compatible file decoder; retains original typed metadata and causes.
pub fn decode_graph_performance_config(raw: &[u8]) -> Result<GraphPerformanceConfig, E> {
    let c = crate::work::decode_mode(raw, config_shape(), true)
        .map_err(|e| E::caused("graph-performance-source-invalid", e))?;
    validate_graph_performance_config(&c)?;
    Ok(c)
}
/// Strict transport-only token decode with new-request validation. No IO/rights.
pub fn decode_api_graph_performance_change(raw: &[u8]) -> Result<ApiGraphPerformanceChange, E> {
    let r: ApiGraphPerformanceChange = crate::work::decode_mode(raw, api_shape(), true)
        .map_err(|e| E::caused("api-graph-performance-change-invalid", e))?;
    validate_graph_performance_change(&r.recorded_change())?;
    Ok(r)
}
#[cfg(test)]
mod tests {
    use super::*;
    /// Actual supplied versioned defaults, no inferred fallback values.
    #[test]
    fn versioned_defaults_and_recipe_invariants() {
        let mut v = decode_graph_performance_defaults(include_bytes!(
            "../../../../config/graph-performance.json"
        ))
        .unwrap();
        assert_eq!(v.loads.as_ref().unwrap().len(), 3);
        v.samples = 0;
        assert!(validate_graph_performance_values(&v).is_err());
        v.samples = 1;
        v.loads.as_mut().unwrap()[0].cards = 1;
        assert!(validate_graph_performance_values(&v).is_err());
        v.loads = None;
        assert!(validate_graph_performance_values(&v).is_err());
    }
    /// Duplicate card workloads refuse; exact integer bound is shared wire only.
    #[test]
    fn unique_loads_and_exact_integer_bound() {
        let mut v = decode_graph_performance_defaults(include_bytes!(
            "../../../../config/graph-performance.json"
        ))
        .unwrap();
        let copy = v.loads.as_ref().unwrap()[0].clone();
        v.loads.as_mut().unwrap().push(copy);
        assert!(validate_graph_performance_values(&v).is_err());
        v.loads.as_mut().unwrap().pop();
        v.samples = GRAPH_EXACT_INTEGER_MAX;
        assert!(validate_graph_performance_values(&v).is_ok());
        v.samples += 1;
        assert!(validate_graph_performance_values(&v).is_err());
    }
    /// Go struct duplicate/case/null scalar merge is delegated without narrowing.
    #[test]
    fn duplicate_case_and_null_struct_merge() {
        let r=decode_api_graph_performance_change(br#"{"SCHEMA_VERSION":1,"event_id":"g","expected_revision":0,"values":{"keyboard_p95_ms":3,"samples":1,"loads":[{"cards":2,"render_p95_ms":1}]},"values":{"keyboard_p95_ms":7},"values":null,"reason":"ok"}"#).unwrap();
        assert_eq!(r.values.keyboard_p95_ms, 7);
        assert_eq!(r.values.samples, 1);
        assert_eq!(r.values.loads.as_ref().unwrap().len(), 1);
    }
    /// Historical Change headers stay unjudged while current/row values stay valid.
    #[test]
    fn null_history_and_historical_change_metadata() {
        let v = decode_graph_performance_defaults(include_bytes!(
            "../../../../config/graph-performance.json"
        ))
        .unwrap();
        let mut c = GraphPerformanceConfig {
            schema_version: 1,
            revision: 0,
            values: v.clone(),
            history: None,
        };
        assert!(validate_graph_performance_config(&c).is_ok());
        c.revision = 1;
        c.history = Some(vec![GraphPerformanceRevision {
            revision: 1,
            values: v,
            actor: String::new(),
            at: String::new(),
            change: GraphPerformanceChange::default(),
        }]);
        let raw = crate::encode_model(&c).unwrap();
        assert_eq!(decode_graph_performance_config(&raw).unwrap(), c);
        c.history.as_mut().unwrap()[0].revision = 2;
        assert!(validate_graph_performance_config(&c).is_err());
    }
    /// Token never enters the recorded change or diagnostics; unknown keys refuse.
    #[test]
    fn token_exclusion_and_strict_unknown() {
        let raw=br#"{"schema_version":1,"event_id":"g","expected_revision":0,"values":{"keyboard_p95_ms":3,"samples":1,"loads":[{"cards":2,"render_p95_ms":1}]},"reason":"ok","preview_token":"private-test-token"}"#;
        let r = decode_api_graph_performance_change(raw).unwrap();
        assert_eq!(r.preview_token(), "private-test-token");
        assert!(!format!("{:?}", r).contains("private-test-token"));
        assert!(
            !String::from_utf8(crate::encode_model(&r.recorded_change()).unwrap())
                .unwrap()
                .contains("preview_token")
        );
        assert!(decode_api_graph_performance_change(br#"{"unknown":0}"#).is_err());
    }
}
