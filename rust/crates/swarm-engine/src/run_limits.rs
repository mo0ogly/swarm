//! Effective limits for future launches; no process or permission is created.
use crate::RunLimits;
use std::{error::Error, fmt};

/// Saved defaults and nullable definition ceilings, constructed only by validated
/// settings. Frozen attempts retain their own budgets independently of these caps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeLimitPolicy {
    defaults: [i64; 5],
    ceilings: [Option<i64>; 5],
}
impl RuntimeLimitPolicy {
    /// Settings validation owns positivity and cross-field bounds before creation.
    pub(crate) fn new(defaults: [i64; 5], ceilings: [Option<i64>; 5]) -> Self {
        Self { defaults, ceilings }
    }
    /// Return saved nullable definition ceilings in canonical five-field order.
    /// Observational only: no default adoption, mutation or execution authority.
    pub(crate) fn definition_ceilings(&self)->[Option<i64>;5]{self.ceilings}
    /// Validate an override without substituting defaults. Zero inherits; positive
    /// values obey the saved nullable ceilings. Observation is a protocol enum,
    /// not permission: only an authoritative administrative consumer may supply it.
    pub fn validate_override(&self, value: &RunLimits) -> Result<(), RuntimeLimitError> {
        if !matches!(value.observation_mode, 0 | 1) {
            return Err(RuntimeLimitError("runtime-observation-mode-invalid"));
        }
        for (value, ceiling) in counts(value).into_iter().zip(self.ceilings) {
            if value < 0 || ceiling.is_some_and(|c| value > c) {
                return Err(RuntimeLimitError("runtime-limit-outside-saved-policy"));
            }
        }
        Ok(())
    }
    /// Freeze zero provider fields from saved defaults, checking all values.
    /// No legacy constant fallback, timer, retry or consumed-counter change occurs.
    pub fn normalize(&self, value: &RunLimits) -> Result<RunLimits, RuntimeLimitError> {
        self.validate_override(value)?;
        let mut fields = counts(value);
        for (field, default) in fields.iter_mut().zip(self.defaults) {
            if *field == 0 {
                *field = default;
            }
        }
        Ok(from_counts(fields, value.observation_mode))
    }
}
/// Safe refusal without private values; pure resolution has no underlying IO cause.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RuntimeLimitError(&'static str);
impl RuntimeLimitError {
    /// Stable diagnostic; a refused preview never grants launch/observation rights.
    pub fn code(&self) -> &'static str {
        self.0
    }
}
impl fmt::Display for RuntimeLimitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl fmt::Debug for RuntimeLimitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for RuntimeLimitError {}
/// Canonical five-field order shared with typed settings, not an operator cap.
fn counts(value: &RunLimits) -> [i64; 5] {
    [
        value.silence_seconds,
        value.tool_seconds,
        value.max_tool_calls,
        value.max_repeated_calls,
        value.max_consecutive_errors,
    ]
}
/// Reconstruct a complete owned value; no inherited counters or side effects.
fn from_counts(fields: [i64; 5], observation_mode: i64) -> RunLimits {
    RunLimits {
        silence_seconds: fields[0],
        tool_seconds: fields[1],
        max_tool_calls: fields[2],
        max_repeated_calls: fields[3],
        max_consecutive_errors: fields[4],
        observation_mode,
    }
}
/// Apply explicit administrative duration observations, preserving call budgets.
/// Positive values may replace a historical threshold; zero inherits unchanged.
fn administrative_durations(fields: &mut [i64; 5], admin: &[i64; 5]) {
    for index in 0..2 {
        if admin[index] > 0 {
            fields[index] = admin[index];
        }
    }
}

/// Resolve future-launch limits under an explicitly saved policy. Provider zero
/// inherits installation defaults; a mission can only tighten. Administrative
/// durations may replace observation thresholds, while call/error budgets and
/// positive previous-attempt ceilings remain restrictive. Only current Admin
/// observation mode applies. Inputs are supplied metadata: caller must read Admin
/// and prior attempt from the authoritative launch transaction and verify rights.
/// Invalid/negative/excessive values or mission-supplied observation refuse. This
/// performs no IO, timer, reservation, launch, task acceptance or counter reset.
pub fn resolve_run_limits(
    provider: &RunLimits,
    mission: &RunLimits,
    admin: &RunLimits,
    previous: Option<&RunLimits>,
    policy: &RuntimeLimitPolicy,
) -> Result<RunLimits, RuntimeLimitError> {
    if provider.observation_mode != 0 || mission.observation_mode != 0 {
        return Err(RuntimeLimitError("runtime-observation-requires-admin"));
    }
    policy.validate_override(admin)?;
    let mut provider = provider.clone();
    if admin.silence_seconds > 0 {
        provider.silence_seconds = admin.silence_seconds;
    }
    if admin.tool_seconds > 0 {
        provider.tool_seconds = admin.tool_seconds;
    }
    let normalized = policy.normalize(&provider)?;
    policy.validate_override(mission)?;
    let mut fields = counts(&normalized);
    let request = counts(mission);
    let administrative = counts(admin);
    for (value, requested) in fields.iter_mut().zip(request) {
        if requested > *value {
            return Err(RuntimeLimitError("runtime-mission-cannot-relax-provider"));
        }
        if requested > 0 {
            *value = requested;
        }
    }
    for (value, ceiling) in fields.iter_mut().zip(administrative) {
        if ceiling > 0 && ceiling < *value {
            *value = ceiling;
        }
    }
    if let Some(previous) = previous {
        if !matches!(previous.observation_mode, 0 | 1) || counts(previous).iter().any(|v| *v < 0) {
            return Err(RuntimeLimitError("runtime-previous-limits-invalid"));
        }
        for (value, ceiling) in fields.iter_mut().zip(counts(previous)) {
            if ceiling > 0 && ceiling < *value {
                *value = ceiling;
            }
        }
        administrative_durations(&mut fields, &administrative);
        for index in 0..2 {
            if request[index] > 0 && request[index] < fields[index] {
                fields[index] = request[index];
            }
        }
    }
    Ok(from_counts(fields, admin.observation_mode))
}

/// Read-only limit-preview data. Supplied provider/prior values are observations,
/// not authenticated stored attempts or permission to launch. Admin is never input.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct RuntimeLimitPreview {
    pub schema_version: i64,
    pub provider: RunLimits,
    pub mission: RunLimits,
    pub previous: Option<RunLimits>,
}
/// Decode strict preview fields through the Go-compatible shape validator.
/// The RootStore consumer checks schema1 before reading policy.
/// Unknown fields/types/ranges fail; no IO, policy adoption or execution occurs.
pub fn decode_runtime_limit_preview(raw: &[u8]) -> Result<RuntimeLimitPreview, crate::WorkError> {
    use crate::model_wire::Kind;
    let value: RuntimeLimitPreview = crate::work::decode_mode(
        raw,
        &[
            ("schema_version", Kind::Int64),
            ("provider", Kind::Object(crate::models::shape_run_limits)),
            ("mission", Kind::Object(crate::models::shape_run_limits)),
            (
                "previous",
                Kind::Pointer(&Kind::Object(crate::models::shape_run_limits)),
            ),
        ],
        true,
    )?;
    Ok(value)
}

/// Read actual project→mission→role→task overrides on the caller's connection.
/// Caller holds the read or admission transaction and saved-settings reservation;
/// missing rows inherit, corrupt revisions/body/observation decisions refuse.
/// This makes no write, opens no alternate snapshot and grants no launch rights.
#[cfg(unix)]
pub(crate) fn read_configured_runtime_limits(
    tx: &rusqlite::Connection,
    policy: &RuntimeLimitPolicy,
    work: &str,
    role: &str,
    task: &str,
) -> Result<RunLimits, crate::RootStoreError> {
    use crate::root_store::finished;
    use crate::RootStoreError;
    let mut admin = crate::RunLimits::default();
    for (scope, mission, key) in [
        ("project", "", ""),
        ("mission", work, ""),
        ("role", work, role),
        ("task", work, task),
    ] {
        let mut statement=tx.prepare("SELECT revision,body,reason FROM main.run_limits_config WHERE scope=? AND mission_id=? AND scope_key=?").map_err(RootStoreError::Sql)?;
        let result = (|| {
            let mut rows = statement
                .query(rusqlite::params![scope, mission, key])
                .map_err(RootStoreError::Sql)?;
            let Some(row) = rows.next().map_err(RootStoreError::Sql)? else {
                return Ok(None);
            };
            let revision: i64 = row.get(0).map_err(RootStoreError::Sql)?;
            if revision < 1 {
                return Err(RootStoreError::Refused(
                    "runtime-limit-config-revision-invalid",
                ));
            }
            let raw = match row.get_ref(1).map_err(RootStoreError::Sql)? {
                rusqlite::types::ValueRef::Text(v) | rusqlite::types::ValueRef::Blob(v) => v,
                _ => return Err(RootStoreError::Refused("runtime-limit-config-body-invalid")),
            };
            let value: crate::RunLimits =
                crate::work::decode_mode(raw, crate::models::shape_run_limits(), false)
                    .map_err(RootStoreError::RunLimitsBody)?;
            policy
                .validate_override(&value)
                .map_err(RootStoreError::RunLimits)?;
            if value.observation_mode != 0 {
                let reason: String = row.get(2).map_err(RootStoreError::Sql)?;
                if scope != "mission" || crate::text::trim_space(&reason).is_empty() {
                    return Err(RootStoreError::Refused(
                        "runtime-observation-requires-mission-admin",
                    ));
                }
            }
            Ok(Some(value))
        })();
        if let Some(value) = finished(result, statement.finalize().map_err(RootStoreError::Sql))? {
            if value.observation_mode != 0 {
                if scope != "mission" {
                    return Err(RootStoreError::Refused(
                        "runtime-observation-requires-mission-admin",
                    ));
                }
                admin.observation_mode = value.observation_mode;
            }
            for (target, value) in [
                (&mut admin.silence_seconds, value.silence_seconds),
                (&mut admin.tool_seconds, value.tool_seconds),
                (&mut admin.max_tool_calls, value.max_tool_calls),
                (&mut admin.max_repeated_calls, value.max_repeated_calls),
                (
                    &mut admin.max_consecutive_errors,
                    value.max_consecutive_errors,
                ),
            ] {
                if value != 0 {
                    *target = value;
                }
            }
        }
    }

    Ok(admin)
}
