//! Saved log/terminal returned-row choices. Initial100/200 values live only
//! in versioned configuration. No execution, retention or acceptance authority.
use crate::AgentLaunchError as E;
use serde::{Deserialize, Serialize};
/// Exact JavaScript/JSON integer representation, not an operator read ceiling.
pub const LOG_MAX_SAFE_ROWS: i64 = (1i64 << 53) - 1;
/// Required nullable row cap. Null means unlimited; absence is not a fallback.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LogReadCap {
    Rows(i64),
    Unlimited(()),
}
impl LogReadCap {
    /// Observe the declared cap; no default, clamping, allocation or authority.
    pub fn rows(self) -> Option<i64> {
        match self {
            Self::Rows(n) => Some(n),
            Self::Unlimited(()) => None,
        }
    }
}
/// Required nullable returned-row choices for each real reader. Null selects
/// all matching retained rows; a finite choice must be positive. These values
/// affect display/transport pagination, never retention, execution or acceptance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogReadValues {
    pub search_page_rows: LogReadCap,
    pub history_page_rows: LogReadCap,
    pub live_log_page_rows: LogReadCap,
    pub terminal_page_rows: LogReadCap,
}
/// Recorded request, not operator authentication. Native tokens never persist.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogReadChange {
    pub schema_version: i64,
    pub event_id: String,
    pub expected_revision: i64,
    pub values: LogReadValues,
    pub reason: String,
}
/// Appended observation; actor/time remain unverified recorded text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogReadRevision {
    pub revision: i64,
    pub values: LogReadValues,
    pub actor: String,
    pub at: String,
    pub change: LogReadChange,
}
/// Native file ledger. Null/absent history means no rows only at revision0;
/// historical Change headers are not rejudged as current authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogReadConfig {
    pub schema_version: i64,
    pub revision: i64,
    pub values: LogReadValues,
    #[serde(default)]
    pub history: Option<Vec<LogReadRevision>>,
}
/// Native submission with optional additional source/settings/instance freshness.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiLogReadChange {
    pub schema_version: i64,
    pub event_id: String,
    pub expected_revision: i64,
    pub values: LogReadValues,
    pub reason: String,
    #[serde(default)]
    preview_token: String,
}
impl ApiLogReadChange {
    /// Borrow freshness data; never a permission or recorded metadata field.
    pub fn preview_token(&self) -> &str {
        &self.preview_token
    }
    /// Exact operator intent excluding transport-only token; no IO or acceptance.
    pub fn recorded_change(&self) -> LogReadChange {
        LogReadChange {
            schema_version: self.schema_version,
            event_id: self.event_id.clone(),
            expected_revision: self.expected_revision,
            values: self.values,
            reason: self.reason.clone(),
        }
    }
}
impl std::fmt::Debug for ApiLogReadChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiLogReadChange")
            .field("token_present", &!self.preview_token.is_empty())
            .finish_non_exhaustive()
    }
}
/// Validate exact integer representation and nonempty finite pages. No fixed row policy,
/// mutation, defaults or promise that the selected history is complete.
pub fn validate_log_read_values(v: &LogReadValues) -> Result<(), E> {
    for cap in [
        v.search_page_rows,
        v.history_page_rows,
        v.live_log_page_rows,
        v.terminal_page_rows,
    ] {
        if cap
            .rows()
            .is_some_and(|n| !(1..=LOG_MAX_SAFE_ROWS).contains(&n))
        {
            return Err(E::Refused("log-read-values-invalid"));
        }
    }
    Ok(())
}
/// Validate ledger sequence/current values, retaining typed historical metadata.
/// Invalid source is never replaced with defaults; no file access or permission.
pub fn validate_log_read_config(c: &LogReadConfig) -> Result<(), E> {
    let rows = c.history.as_deref().unwrap_or(&[]);
    if c.schema_version != 1 || c.revision < 0 || i64::try_from(rows.len()).ok() != Some(c.revision)
    {
        return Err(E::Refused("log-read-config-invalid"));
    }
    validate_log_read_values(&c.values)?;
    for (i, r) in rows.iter().enumerate() {
        if i64::try_from(i).ok().and_then(|n| n.checked_add(1)) != Some(r.revision) {
            return Err(E::Refused("log-read-history-invalid"));
        }
        validate_log_read_values(&r.values)?;
    }
    if rows.last().is_some_and(|r| r.values != c.values) {
        return Err(E::Refused("log-read-history-invalid"));
    }
    Ok(())
}
/// New request schema/name/CAS/reason and choices. No hidden reason/file cap.
pub fn validate_log_read_change(c: &LogReadChange) -> Result<(), E> {
    if c.schema_version != 1
        || !swarm_identity_contract::name(c.event_id.as_bytes())
        || c.expected_revision < 0
        || crate::text::trim_space(&c.reason).is_empty()
    {
        return Err(E::Refused("log-read-change-invalid"));
    }
    validate_log_read_values(&c.values)
}
/// Decode native strict fields plus Go string spelling, preserving typed cause.
fn decode<T: serde::de::DeserializeOwned>(raw: &[u8]) -> Result<T, E> {
    swarm_wire_json::decode(raw).map_err(|e| E::caused("log-read-json-invalid", e))
}
/// Decode supplied versioned defaults; no implicit Root adoption or fallback.
pub fn decode_log_read_defaults(raw: &[u8]) -> Result<LogReadValues, E> {
    let v = decode(raw)?;
    validate_log_read_values(&v)?;
    Ok(v)
}
/// Decode strict ledger and validate sequence/current values; no IO or mutation.
pub fn decode_log_read_config(raw: &[u8]) -> Result<LogReadConfig, E> {
    let v = decode(raw)?;
    validate_log_read_config(&v)?;
    Ok(v)
}
/// Decode/validate one exact intent; duplicate/unknown/missing choices refuse.
pub fn decode_api_log_read_change(raw: &[u8]) -> Result<ApiLogReadChange, E> {
    let v: ApiLogReadChange = decode(raw)?;
    validate_log_read_change(&v.recorded_change())?;
    Ok(v)
}

/// Native filtered log query. Omitted cursor/filter select all matching rows
/// after zero; page choices always come from the saved source, not this input.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogSearchInput {
    pub schema_version: i64,
    #[serde(default)]
    pub after: i64,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub kind: String,
}
/// Decode one strict native query before Root IO. Signed cursors match Go log
/// queries; unknown/duplicate fields and unsupported schema preserve errors.
pub fn decode_log_search_input(raw: &[u8]) -> Result<LogSearchInput, E> {
    let v: LogSearchInput = decode(raw)?;
    if v.schema_version != 1 {
        return Err(E::Refused("log-query-input-invalid"));
    }
    Ok(v)
}
