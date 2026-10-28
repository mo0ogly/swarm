//! Native activity display policy; versioned defaults preserve Go50/200/500
//! without hiding operator choices in a reader. No execution/acceptance rights.
use crate::AgentLaunchError as E;
use serde::{Deserialize, Serialize};
/// Exact JavaScript/JSON integer representation, not an operator read ceiling.
pub const ACTIVITY_MAX_SAFE_ROWS: i64 = (1i64 << 53) - 1;
/// Required nullable row cap. Null means unlimited; absence is not a fallback.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ActivityReadCap {
    Rows(i64),
    Unlimited(()),
}
impl ActivityReadCap {
    /// Observe the declared cap; no default, clamping, allocation or authority.
    pub fn rows(self) -> Option<i64> {
        match self {
            Self::Rows(n) => Some(n),
            Self::Unlimited(()) => None,
        }
    }
}
/// Editable row objectives. Source0 permits no payload rows; a sentinel detects
/// truncation. Default/max ordering defines a nonempty page, not a business stop.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivityReadValues {
    pub page_default_rows: i64,
    pub page_max_rows: ActivityReadCap,
    pub source_max_rows: ActivityReadCap,
}
/// Recorded request, not operator authentication. Native tokens never persist.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivityReadChange {
    pub schema_version: i64,
    pub event_id: String,
    pub expected_revision: i64,
    pub values: ActivityReadValues,
    pub reason: String,
}
/// Appended observation; actor/time remain unverified recorded text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivityReadRevision {
    pub revision: i64,
    pub values: ActivityReadValues,
    pub actor: String,
    pub at: String,
    pub change: ActivityReadChange,
}
/// Native file ledger. Null/absent history means no rows only at revision0;
/// historical Change headers are not rejudged as current authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivityReadConfig {
    pub schema_version: i64,
    pub revision: i64,
    pub values: ActivityReadValues,
    #[serde(default)]
    pub history: Option<Vec<ActivityReadRevision>>,
}
/// Native submission with optional additional source/settings/instance freshness.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiActivityReadChange {
    pub schema_version: i64,
    pub event_id: String,
    pub expected_revision: i64,
    pub values: ActivityReadValues,
    pub reason: String,
    #[serde(default)]
    preview_token: String,
}
impl ApiActivityReadChange {
    /// Borrow freshness data; never a permission or recorded metadata field.
    pub fn preview_token(&self) -> &str {
        &self.preview_token
    }
    /// Exact operator intent excluding transport-only token; no IO or acceptance.
    pub fn recorded_change(&self) -> ActivityReadChange {
        ActivityReadChange {
            schema_version: self.schema_version,
            event_id: self.event_id.clone(),
            expected_revision: self.expected_revision,
            values: self.values,
            reason: self.reason.clone(),
        }
    }
}
impl std::fmt::Debug for ApiActivityReadChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiActivityReadChange")
            .field("token_present", &!self.preview_token.is_empty())
            .finish_non_exhaustive()
    }
}
/// Validate representability and configured relationships. No fixed row policy,
/// mutation, defaults or promise that the selected history is complete.
pub fn validate_activity_read_values(v: &ActivityReadValues) -> Result<(), E> {
    if !(1..=ACTIVITY_MAX_SAFE_ROWS).contains(&v.page_default_rows)
        || v.page_max_rows
            .rows()
            .is_some_and(|n| n < v.page_default_rows || n > ACTIVITY_MAX_SAFE_ROWS)
        || v.source_max_rows
            .rows()
            .is_some_and(|n| !(0..=ACTIVITY_MAX_SAFE_ROWS).contains(&n))
    {
        return Err(E::Refused("activity-read-values-invalid"));
    }
    Ok(())
}
/// Validate ledger sequence/current values, retaining typed historical metadata.
/// Invalid source is never replaced with defaults; no file access or permission.
pub fn validate_activity_read_config(c: &ActivityReadConfig) -> Result<(), E> {
    let rows = c.history.as_deref().unwrap_or(&[]);
    if c.schema_version != 1 || c.revision < 0 || i64::try_from(rows.len()).ok() != Some(c.revision)
    {
        return Err(E::Refused("activity-read-config-invalid"));
    }
    validate_activity_read_values(&c.values)?;
    for (i, r) in rows.iter().enumerate() {
        if i64::try_from(i).ok().and_then(|n| n.checked_add(1)) != Some(r.revision) {
            return Err(E::Refused("activity-read-history-invalid"));
        }
        validate_activity_read_values(&r.values)?;
    }
    if rows.last().is_some_and(|r| r.values != c.values) {
        return Err(E::Refused("activity-read-history-invalid"));
    }
    Ok(())
}
/// New request schema/name/CAS/reason and choices. No hidden reason/file cap.
pub fn validate_activity_read_change(c: &ActivityReadChange) -> Result<(), E> {
    if c.schema_version != 1
        || !swarm_identity_contract::name(c.event_id.as_bytes())
        || c.expected_revision < 0
        || crate::text::trim_space(&c.reason).is_empty()
    {
        return Err(E::Refused("activity-read-change-invalid"));
    }
    validate_activity_read_values(&c.values)
}
/// Decode native strict fields plus Go string spelling, preserving typed cause.
fn decode<T: serde::de::DeserializeOwned>(raw: &[u8]) -> Result<T, E> {
    swarm_wire_json::decode(raw).map_err(|e| E::caused("activity-read-json-invalid", e))
}
/// Decode supplied versioned defaults; no implicit Root adoption or fallback.
pub fn decode_activity_read_defaults(raw: &[u8]) -> Result<ActivityReadValues, E> {
    let v = decode(raw)?;
    validate_activity_read_values(&v)?;
    Ok(v)
}
/// Decode strict ledger and validate sequence/current values; no IO or mutation.
pub fn decode_activity_read_config(raw: &[u8]) -> Result<ActivityReadConfig, E> {
    let v = decode(raw)?;
    validate_activity_read_config(&v)?;
    Ok(v)
}
/// Decode/validate one exact intent; duplicate/unknown/missing choices refuse.
pub fn decode_api_activity_read_change(raw: &[u8]) -> Result<ApiActivityReadChange, E> {
    let v: ApiActivityReadChange = decode(raw)?;
    validate_activity_read_change(&v.recorded_change())?;
    Ok(v)
}

/// Native CLI read query. Omitted cursor/filter select newest/all; omitted or
/// zero limit selects the saved page default. Schema1 is required for files.
/// No policy value is synthesized and this type performs no IO.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivityReadInput {
    pub schema_version: i64,
    #[serde(default)]
    pub before: String,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub decisions_only: bool,
}
/// Decode one strict CLI query with preserved JSON cause; unknown/duplicate keys,
/// schema mismatch and negative limits refuse before opening the Root.
pub fn decode_activity_read_input(raw: &[u8]) -> Result<ActivityReadInput, E> {
    let v: ActivityReadInput = decode(raw)?;
    if v.schema_version != 1 || v.limit.is_some_and(|n| n < 0) {
        return Err(E::Refused("activity-query-invalid"));
    }
    Ok(v)
}
