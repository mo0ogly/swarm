//! Run-limits administration DTO/codec for the G9 public run.
//!
//! Wire-level mirror of `internal/engine/run_limits_admin.go`
//! (SHA-256 dcb133d944ac0ed56283866a370208fc74f43f6be0924a5e1cfb5e8394c15ca4)
//! and nothing else.
//!
//! Authority limits of this module:
//! - Pure DTO/codec: no SQL, store, HTTP, UI, timer, actor or timestamp
//!   generation; no new policy defaults; no hardcoded count/duration/reason
//!   caps (Go's constants are deliberately not copied — positive overrides are
//!   judged only by the caller's saved `RuntimeLimitPolicy`).
//! - Reuses the generated `crate::RunLimits` and `crate::RunLimitsConfigEntry`
//!   DTOs and their shapes verbatim; adds only the two Go DTOs missing from the
//!   generated set (`RunLimitsConfigChange`, `RunLimitsHistoryEntry`) plus the
//!   native `ApiRunLimitsConfigChange` transport.
//! - `preview_token` exists only on the native transport: skipped by every
//!   serialization, redacted in `Debug`, and structurally absent from the
//!   recorded change and its canonical request bytes.
//! - Intrinsic/header validation is separated from current saved-policy
//!   validation, so an exact historical replay is checkable without re-judging
//!   previously accepted ceilings against today's policy.
//! - Input custody, file/SQL/root access, timestamps, replay persistence,
//!   revision conflicts and commit remain the caller's responsibility.
//! - Contribution only: Main inspects, compiles and integrates in the actual
//!   G9 journey; nothing here executes or closes that task.

use crate::model_wire::{Fields, Kind};
use crate::{AgentLaunchError, RunLimits, RunLimitsConfigEntry, RuntimeLimitPolicy};

// ---- Error-carrier adapters: the only touchpoints with the provider-wait codec carrier. ----

/// Stable refusal without any parsed or stored value; mirrors the
/// provider-wait codec's `Refused` carrier.
fn refused(code: &'static str) -> AgentLaunchError {
    AgentLaunchError::Refused(code)
}

/// Preserve the concrete parse or settings cause inside the provider-wait
/// codec's `caused` carrier; no diagnostic flattening.
fn caused(error: impl std::error::Error + Send + Sync + 'static) -> AgentLaunchError {
    AgentLaunchError::caused("run-limits-document-invalid", error)
}

/// Single delegation point to the identity name contract; Go `safeName`
/// (letters, digits, dashes) semantics are owned by that contract, not here.
fn name_ok(value: &str) -> bool {
    swarm_identity_contract::name(value.as_bytes())
}

// ---- Scopes (Go constants, four literal formats). ----

/// Go `ScopeProject`: mission id and scope key must both be empty.
pub const SCOPE_PROJECT: &str = "project";
/// Go `ScopeMission`: named mission id, empty scope key.
pub const SCOPE_MISSION: &str = "mission";
/// Go `ScopeRole`: named mission id and named scope key.
pub const SCOPE_ROLE: &str = "role";
/// Go `ScopeTask`: named mission id and named scope key.
pub const SCOPE_TASK: &str = "task";

fn is_zero_i64(value: &i64) -> bool {
    *value == 0
}

// ---- Missing Go DTOs (exact order, tags, omitempty, Go signed 64-bit ints). ----

// internal/engine/run_limits_admin.go SHA-256 dcb133d944ac0ed56283866a370208fc74f43f6be0924a5e1cfb5e8394c15ca4
/// Operator request to set (or clear, with zero values) one scope's override.
/// Exact mirror of Go `RunLimitsConfigChange`: declaration order, JSON tags,
/// omitempty, Go signed 64-bit `int` fields as `i64`. This is the *recorded*
/// change type: the native preview token lives only on
/// [`ApiRunLimitsConfigChange`] and never enters this struct or its canonical
/// bytes. `revision` (`expected_revision`) 0 requires that no override exists
/// yet; any other value must match the stored revision (optimistic
/// concurrency — store custody). `rollback_of` 0 means not a rollback; no
/// extra gate is imposed beyond Go's typed semantics.
///
/// Deliberately absent: Go's hardcoded reason cap and value bounds. Positive
/// values are judged by the caller's saved [`RuntimeLimitPolicy`]; the reason
/// may be empty except for the retained observation-mode mission guard.
/// No `Eq` is derived (`RunLimits` is `PartialEq`-only).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct RunLimitsConfigChange {
    #[serde(rename = "schema_version")]
    pub schema_version: i64,
    #[serde(rename = "event_id")]
    pub event_id: String,
    #[serde(rename = "scope")]
    pub scope: String,
    #[serde(rename = "mission_id", skip_serializing_if = "String::is_empty")]
    pub mission: String,
    #[serde(rename = "scope_key", skip_serializing_if = "String::is_empty")]
    pub key: String,
    #[serde(rename = "expected_revision")]
    pub revision: i64,
    #[serde(rename = "values")]
    pub values: RunLimits,
    #[serde(rename = "reason")]
    pub reason: String,
    #[serde(rename = "rollback_of", skip_serializing_if = "is_zero_i64")]
    pub rollback_of: i64,
}

// internal/engine/run_limits_admin.go SHA-256 dcb133d944ac0ed56283866a370208fc74f43f6be0924a5e1cfb5e8394c15ca4
/// One immutable, past-effective revision of one scope; exact mirror of Go
/// `RunLimitsHistoryEntry` (order, tags, omitempty, signed 64-bit ints).
/// `actor`, `reason` and `at` are recorded metadata this codec never
/// validates; `rollback_of` 0 means not a rollback (Go stores NULL for 0).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct RunLimitsHistoryEntry {
    #[serde(rename = "scope")]
    pub scope: String,
    #[serde(rename = "mission_id", skip_serializing_if = "String::is_empty")]
    pub mission_id: String,
    #[serde(rename = "scope_key", skip_serializing_if = "String::is_empty")]
    pub scope_key: String,
    #[serde(rename = "revision")]
    pub revision: i64,
    #[serde(rename = "values")]
    pub values: RunLimits,
    #[serde(rename = "actor")]
    pub actor: String,
    #[serde(rename = "reason")]
    pub reason: String,
    #[serde(rename = "at")]
    pub at: String,
    #[serde(rename = "rollback_of", skip_serializing_if = "is_zero_i64")]
    pub rollback_of: i64,
}

/// Native G9 admin transport: one change plus its `preview_token`. The token
/// is decoded so the caller can verify it, but this type can never emit it:
/// every serialization skips the field and `Debug` redacts it. The recorded
/// change without the token is available via
/// [`ApiRunLimitsConfigChange::original_change`]. No verification, default or
/// persistence of the token happens here.
#[derive(Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ApiRunLimitsConfigChange {
    #[serde(rename = "schema_version")]
    pub schema_version: i64,
    #[serde(rename = "event_id")]
    pub event_id: String,
    #[serde(rename = "scope")]
    pub scope: String,
    #[serde(rename = "mission_id", skip_serializing_if = "String::is_empty")]
    pub mission: String,
    #[serde(rename = "scope_key", skip_serializing_if = "String::is_empty")]
    pub key: String,
    #[serde(rename = "expected_revision")]
    pub revision: i64,
    #[serde(rename = "values")]
    pub values: RunLimits,
    #[serde(rename = "reason")]
    pub reason: String,
    #[serde(rename = "rollback_of", skip_serializing_if = "is_zero_i64")]
    pub rollback_of: i64,
    #[serde(rename = "preview_token", skip_serializing)]
    pub preview_token: String,
}

impl ApiRunLimitsConfigChange {
    /// The recorded change carried by this transport, with the native preview
    /// token structurally removed. Purpose: the only form that may be validated
    /// for a write or encoded into history. Input: this transport. Output: a
    /// token-free [`RunLimitsConfigChange`]. Errors: none. Effects: none.
    /// Authority: the token is not verified, logged or persisted here; the
    /// caller owns token verification.
    pub fn original_change(&self) -> RunLimitsConfigChange {
        RunLimitsConfigChange {
            schema_version: self.schema_version,
            event_id: self.event_id.clone(),
            scope: self.scope.clone(),
            mission: self.mission.clone(),
            key: self.key.clone(),
            revision: self.revision,
            values: self.values.clone(),
            reason: self.reason.clone(),
            rollback_of: self.rollback_of,
        }
    }

    /// Canonical token-free request bytes for this native transport.
    /// Purpose: the exact blob comparable with Go-legacy stored requests.
    /// Input: this transport (token stripped via `original_change`).
    /// Output: canonical bytes (internal `canonical_request_bytes` encoding).
    /// Errors: serialization cause preserved via `caused`. Effects: none.
    /// Authority: none — persistence and comparison remain caller custody.
    pub fn canonical_request(&self) -> Result<Vec<u8>, AgentLaunchError> {
        canonical_request_bytes(&self.original_change())
    }

    /// Borrowed preview token for caller-side verification only. Purpose:
    /// access without copy. Output: the token as received. Errors: none.
    /// Effects: none. Authority: this codec never verifies, serializes, logs
    /// or persists it.
    pub fn preview_token(&self) -> &str {
        &self.preview_token
    }
}

/// `Debug` never emits the preview token, so logs cannot leak it through this
/// type (serialization already skips it at the serde layer).
impl std::fmt::Debug for ApiRunLimitsConfigChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiRunLimitsConfigChange")
            .field("schema_version", &self.schema_version)
            .field("event_id", &self.event_id)
            .field("scope", &self.scope)
            .field("mission_id", &self.mission)
            .field("scope_key", &self.key)
            .field("expected_revision", &self.revision)
            .field("values", &self.values)
            .field("reason", &self.reason)
            .field("rollback_of", &self.rollback_of)
            .field("preview_token", &"[redacted]")
            .finish()
    }
}

// ---- Wire shapes (crate-internal, in the generated DTOs' tag order). ----

/// Wire shape of [`RunLimitsConfigChange`]; `values` reuses the existing
/// generated `crate::models::shape_run_limits`.
pub(crate) fn shape_run_limits_config_change() -> Fields {
    &[
        ("schema_version", Kind::Int64),
        ("event_id", Kind::String),
        ("scope", Kind::String),
        ("mission_id", Kind::String),
        ("scope_key", Kind::String),
        ("expected_revision", Kind::Int64),
        ("values", Kind::Object(crate::models::shape_run_limits)),
        ("reason", Kind::String),
        ("rollback_of", Kind::Int64),
    ]
}

/// Wire shape of the native transport: the recorded shape plus `preview_token`
/// (decode-only on the value level; the field is never serialized).
pub(crate) fn shape_api_run_limits_config_change() -> Fields {
    &[
        ("schema_version", Kind::Int64),
        ("event_id", Kind::String),
        ("scope", Kind::String),
        ("mission_id", Kind::String),
        ("scope_key", Kind::String),
        ("expected_revision", Kind::Int64),
        ("values", Kind::Object(crate::models::shape_run_limits)),
        ("reason", Kind::String),
        ("rollback_of", Kind::Int64),
        ("preview_token", Kind::String),
    ]
}

/// Wire shape of [`RunLimitsHistoryEntry`].
pub(crate) fn shape_run_limits_history_entry() -> Fields {
    &[
        ("scope", Kind::String),
        ("mission_id", Kind::String),
        ("scope_key", Kind::String),
        ("revision", Kind::Int64),
        ("values", Kind::Object(crate::models::shape_run_limits)),
        ("actor", Kind::String),
        ("reason", Kind::String),
        ("at", Kind::String),
        ("rollback_of", Kind::Int64),
    ]
}

// ---- Strict decoders (actual work::decode_mode; Go null/case/duplicate semantics). ----

/// Strict decoder for a recorded (token-free) change request.
/// Purpose: parse operator-shaped change JSON exactly as Go's strict parsing
/// accepts it. Input: raw JSON bytes of one change. Output: the decoded
/// change; unknown fields are refused; Go null, case-insensitive-key and
/// duplicate-key (last wins) semantics are inherited from
/// `crate::work::decode_mode`. Errors: `caused` preserves the parse cause.
/// Effects: none. Authority: none — output is unvalidated input; callers must
/// submit to Root's header validation and, for new writes, saved policy.
pub fn decode_run_limits_config_change(
    raw: &[u8],
) -> Result<RunLimitsConfigChange, AgentLaunchError> {
    crate::work::decode_mode(raw, shape_run_limits_config_change(), true).map_err(caused)
}

/// Strict decoder for the native transport: the recorded shape plus
/// `preview_token`. Purpose: parse the native admin payload while keeping the
/// token isolated. Input: raw JSON bytes (token optional for Go-legacy
/// payloads). Output: the decoded transport; unknown fields refused; Go
/// null/case/duplicate semantics via `decode_mode`. Errors: `caused`
/// preserves the parse cause. Effects: none. Authority: none — the token is
/// only carried (never serialized, logged or persisted); values remain
/// unvalidated until the caller runs the header/policy gates.
pub fn decode_api_run_limits_config_change(
    raw: &[u8],
) -> Result<ApiRunLimitsConfigChange, AgentLaunchError> {
    crate::work::decode_mode(raw, shape_api_run_limits_config_change(), true).map_err(caused)
}

/// Strict decoder for one immutable history entry (stored row body or served
/// history JSON). Purpose: typed read-back of history. Input: raw JSON bytes
/// of one entry. Output: the decoded entry; unknown fields refused; Go
/// null/case/duplicate semantics via `decode_mode`. Errors: `caused` preserves
/// the parse cause. Effects: none. Authority: none — recorded metadata
/// (`actor`/`at`/`reason`) is never validated here.
pub fn decode_run_limits_history_entry(
    raw: &[u8],
) -> Result<RunLimitsHistoryEntry, AgentLaunchError> {
    crate::work::decode_mode(raw, shape_run_limits_history_entry(), true).map_err(caused)
}

/// Strict decoder reusing the existing generated [`RunLimitsConfigEntry`] DTO
/// and its existing shape verbatim (no duplicate DTO is defined here).
/// Purpose: typed read-back of a current-entry payload. Input: raw JSON bytes
/// of one entry. Output: the decoded entry. Errors: `caused` preserves the
/// parse cause. Effects: none. Authority: none — no store, right or policy
/// judgement is involved.
pub fn decode_run_limits_config_entry(
    raw: &[u8],
) -> Result<RunLimitsConfigEntry, AgentLaunchError> {
    crate::work::decode_mode(raw, crate::models::shape_run_limits_config_entry(), true)
        .map_err(caused)
}

// ---- Header/scope syntax helper (four literal Go formats + name contract). ----

/// Scope syntax gate matching Go `validRunLimitsScope` exactly: `project`
/// (mission id and key empty), `mission` (named mission id, empty key),
/// `role`/`task` (named mission id and named key); anything else unknown.
/// Input: scope, mission id and scope key as received on the wire.
/// Output: `Ok(())` for a syntactically valid tuple. Errors: stable `Refused`
/// code for an unknown scope or fields not matching the scope's format; names
/// are judged solely by the `swarm_identity_contract::name` contract.
/// Effects: none (pure). Authority: syntax only — this neither verifies that a
/// mission or role exists, nor grants any read/write right on the scope.
pub fn valid_run_limits_scope(
    scope: &str,
    mission: &str,
    key: &str,
) -> Result<(), AgentLaunchError> {
    match scope {
        SCOPE_PROJECT => {
            if !mission.is_empty() || !key.is_empty() {
                return Err(refused("run-limits-scope-project-fields"));
            }
            Ok(())
        }
        SCOPE_MISSION => {
            if !key.is_empty() {
                return Err(refused("run-limits-scope-mission-key"));
            }
            if !name_ok(mission) {
                return Err(refused("run-limits-scope-mission-name"));
            }
            Ok(())
        }
        SCOPE_ROLE | SCOPE_TASK => {
            if !name_ok(mission) || !name_ok(key) {
                return Err(refused("run-limits-scope-role-task-names"));
            }
            Ok(())
        }
        _ => Err(refused("run-limits-scope-unknown")),
    }
}

// ---- Intrinsic/header validation, split from CURRENT saved-policy validation. ----

/// Intrinsic/header validation retained from Go `configureRunLimits`, in Go's
/// order, minus store and saved-policy custody. Checks, in order:
/// 1. observation guard: a non-zero observation mode requires the mission
///    scope and a nonblank reason (`crate::text::trim_space`);
/// 2. `schema_version == 1`;
/// 3. `event_id` satisfies the name contract (Go `safeName`);
/// 4. the four literal scope formats;
/// 5. the 0/1 observation protocol (the protocol piece Go enforced inside its
///    hardcoded bounds; positive ceilings are deliberately NOT re-checked here; negative values refuse).
/// Input: a decoded change. Output: `Ok(())` or the first failing guard.
/// Errors: stable `Refused` codes. Effects: none.
/// Authority: deliberately does NOT validate reason length, value magnitudes,
/// `expected_revision`/`rollback_of` beyond Go's typed semantics (0 = first
/// write / not a rollback), mission or role existence, or any historical
/// metadata — those are saved policy or caller custody.
pub fn validate_change_header(change: &RunLimitsConfigChange) -> Result<(), AgentLaunchError> {
    if change.values.observation_mode != 0
        && (change.scope != SCOPE_MISSION || crate::text::trim_space(&change.reason).is_empty())
    {
        return Err(refused("run-limits-observation-requires-mission-reason"));
    }
    if change.schema_version != 1 {
        return Err(refused("run-limits-schema-version"));
    }
    if !name_ok(&change.event_id) {
        return Err(refused("run-limits-event-id-name"));
    }
    valid_run_limits_scope(&change.scope, &change.mission, &change.key)?;
    if !matches!(change.values.observation_mode, 0 | 1) {
        return Err(refused("run-limits-observation-mode"));
    }
    // Nonnegative values are intrinsic zero-inherits wire semantics. Positive
    // magnitudes are checked only against current saved policy for NEW writes.
    if [change.values.silence_seconds,change.values.tool_seconds,change.values.max_tool_calls,change.values.max_repeated_calls,change.values.max_consecutive_errors].iter().any(|value|*value<0){
        return Err(refused("run-limits-negative-value"));
    }
    Ok(())
}

/// Current saved-policy validation for NEW writes only: delegates to
/// `RuntimeLimitPolicy::validate_override` — every positive field must obey
/// the saved nullable ceilings, zero inherits, observation 0/1 enforced.
/// Input: a change plus the caller's saved policy (no default policy exists
/// here). Output: `Ok(())` or `caused` preserving the settings error cause.
/// Effects: none. Authority: never apply this to historical replay — old
/// accepted ceilings must not be re-judged against today's policy (see
/// [`verify_historical_replay`]).
pub fn validate_change_policy(
    change: &RunLimitsConfigChange,
    policy: &RuntimeLimitPolicy,
) -> Result<(), AgentLaunchError> {
    policy.validate_override(&change.values).map_err(|error|AgentLaunchError::caused(error.code(),error))
}

/// Full gate for a new write: intrinsic/header validation, then current
/// saved-policy validation, in that order. Input: change and saved policy.
/// Output: `Ok(())` or the first failure (refusal code or preserved
/// settings/parse cause). Errors: as for the two component checks.
/// Effects: none. Authority: no IO or persistence; the caller owns
/// revision-conflict handling, idempotent event storage, actor identity,
/// timestamps and commit.
pub fn validate_new_change(
    change: &RunLimitsConfigChange,
    policy: &RuntimeLimitPolicy,
) -> Result<(), AgentLaunchError> {
    validate_change_header(change)?;
    validate_change_policy(change, policy)
}

// ---- Canonical request bytes and exact historical replay. ----

/// Canonical recorded-request bytes: [`crate::encode_model`] of the change,
/// Go `json.Marshal`-compatible in declaration-order fields and string
/// escaping (`<`, `>`, `&`, U+2028/9). This — never plain `serde_json` output —
/// is the byte form comparable against Go-legacy `run_limits_history.request`
/// blobs. Input: a recorded change (token-free by construction). Output:
/// canonical bytes. Errors: `caused` preserves the serialization cause.
/// Effects: none. Authority: none — callers own what is persisted and when.
pub fn canonical_request_bytes(
    change: &RunLimitsConfigChange,
) -> Result<Vec<u8>, AgentLaunchError> {
    crate::encode_model(change).map_err(caused)
}

/// Compare the opaque stored request bytes with Go-canonical candidate bytes.
/// Returns the same typed candidate on exact match, or event-conflict otherwise.
/// It never parses or rejudges a stored request: Go compares bytes, including a
/// malformed/foreign recorded blob. Caller owns candidate validation, scope,
/// current saved policy for NEW writes, current-entry read and transaction.
/// No IO, token verification, mutation or authority is supplied by this codec.
pub fn verify_historical_replay(
    recorded_request: &[u8],
    candidate: &RunLimitsConfigChange,
) -> Result<RunLimitsConfigChange, AgentLaunchError> {
    if canonical_request_bytes(candidate)?.as_slice() != recorded_request {
        return Err(refused("run-limits-event-conflict"));
    }
    Ok(candidate.clone())
}

#[cfg(test)]
mod run_limits_admin_codec_tests {
    use super::*;

    /// Saved operator policy stand-in for tests only (arbitrary numbers, not
    /// Go's constants and not a module default).
    fn test_policy() -> RuntimeLimitPolicy {
        RuntimeLimitPolicy::new(
            [10, 20, 30, 2, 1],
            [Some(600), Some(600), Some(50), Some(7), Some(9)],
        )
    }

    /// Identifiers chosen for the Go safeName contract (letters, digits,
    /// dashes); the delegation to swarm_identity_contract::name is itself part
    /// of what these tests observe.
    fn sample_change() -> RunLimitsConfigChange {
        RunLimitsConfigChange {
            schema_version: 1,
            event_id: "g9-event-1".to_string(),
            scope: SCOPE_PROJECT.to_string(),
            mission: String::new(),
            key: String::new(),
            revision: 2,
            values: RunLimits {
                observation_mode: 0,
                silence_seconds: 30,
                tool_seconds: 60,
                max_tool_calls: 10,
                max_repeated_calls: 2,
                max_consecutive_errors: 1,
            },
            reason: "r".to_string(),
            rollback_of: 0,
        }
    }

    /// Canonical request bytes match Go json.Marshal: declaration order,
    /// omitempty and HTML escaping. This is why plain serde_json bytes must
    /// never be compared against Go legacy stored requests.
    #[test]
    fn canonical_request_go_order_escaping_roundtrip() {
        let mut change = sample_change();
        change.reason = "fix & roll <out>".to_string();
        let canonical = canonical_request_bytes(&change).unwrap();
        assert_eq!(
            std::str::from_utf8(&canonical).unwrap(),
            concat!(
                r#"{"schema_version":1,"event_id":"g9-event-1","scope":"project","expected_revision":2,"values":{"silence_seconds":30,"tool_seconds":60,"max_tool_calls":10,"max_repeated_calls":2,"max_consecutive_errors":1},"reason":"fix \u0026 roll \u003cout\u003e"}"#
            )
        );
        assert_eq!(decode_run_limits_config_change(&canonical).unwrap(), change);
    }

    /// Plain serde_json output is not the canonical recorded form.
    #[test]
    fn plain_serde_json_is_not_canonical() {
        let mut change = sample_change();
        change.reason = "a&b".to_string();
        let plain = serde_json::to_string(&change).unwrap();
        let canonical_bytes = canonical_request_bytes(&change).unwrap();
        let canonical = std::str::from_utf8(&canonical_bytes).unwrap();
        assert!(plain.contains('&'));
        assert!(canonical.contains(r#"\u0026"#));
        assert_ne!(plain, canonical);
    }

    /// The native transport decodes preview_token for caller verification but
    /// it never serializes, never appears in Debug, and never enters the
    /// recorded change or its canonical bytes.
    #[test]
    fn native_token_isolated_from_recorded_change() {
        let raw = br#"{"schema_version":1,"event_id":"g9-event-1","scope":"project","expected_revision":2,"values":{"silence_seconds":30,"tool_seconds":60,"max_tool_calls":10,"max_repeated_calls":2,"max_consecutive_errors":1},"reason":"r","preview_token":"sekret"}"#;
        let native = decode_api_run_limits_config_change(raw).unwrap();
        assert_eq!(native.preview_token(), "sekret");
        let original = native.original_change();
        assert_eq!(original, sample_change());
        let original_bytes = canonical_request_bytes(&original).unwrap();
        let native_bytes = crate::encode_model(&native).unwrap();
        assert!(!std::str::from_utf8(&original_bytes).unwrap().contains("sekret"));
        assert!(!std::str::from_utf8(&native_bytes).unwrap().contains("sekret"));
        assert!(!format!("{:?}", native).contains("sekret"));
        assert_eq!(native.canonical_request().unwrap(), original_bytes);
    }

    /// Strict decoding inherits Go semantics from work::decode_mode: unknown
    /// fields refused, case-insensitive keys, duplicate keys last-wins, null
    /// leaving Go zero values.
    #[test]
    fn strict_decode_go_null_case_duplicate_semantics() {
        let unknown = br#"{"schema_version":1,"event_id":"e-1","scope":"project","expected_revision":0,"values":{},"reason":"","extra":1}"#;
        assert!(decode_run_limits_config_change(unknown).is_err());
        let quirks = br#"{"SCHEMA_VERSION":1,"reason":"a","reason":"b","event_id":"e-1","scope":"project","expected_revision":0,"values":null}"#;
        let change = decode_run_limits_config_change(quirks).unwrap();
        assert_eq!(change.schema_version, 1);
        assert_eq!(change.reason, "b");
        assert_eq!(change.values, RunLimits::default());
    }

    /// Retained Go header guards: schema, event-id name contract, four literal
    /// scope formats, observation-mode mission+nonblank-reason guard, 0/1
    /// protocol; expected_revision/rollback_of carry no extra gates.
    #[test]
    fn header_guards_match_go() {
        let mut change = sample_change();
        assert!(validate_change_header(&change).is_ok());
        change.schema_version = 2;
        assert!(validate_change_header(&change).is_err());
        change.schema_version = 1;
        change.scope = SCOPE_ROLE.to_string();
        assert!(validate_change_header(&change).is_err());
        change.mission = "m-1".to_string();
        change.key = "worker".to_string();
        assert!(validate_change_header(&change).is_ok());
        change.values.observation_mode = 1;
        assert!(validate_change_header(&change).is_err());
        change.scope = SCOPE_MISSION.to_string();
        change.key = String::new();
        change.reason = "   ".to_string();
        assert!(validate_change_header(&change).is_err());
        change.reason = "  incident review  ".to_string();
        assert!(validate_change_header(&change).is_ok());
        change.values.observation_mode = 7;
        assert!(validate_change_header(&change).is_err());
        change.values.observation_mode = 0;
        change.scope = "cluster".to_string();
        assert!(validate_change_header(&change).is_err());
        change.scope = SCOPE_PROJECT.to_string();
        assert!(validate_change_header(&change).is_err());
        change.mission = String::new();
        assert!(validate_change_header(&change).is_ok());
        // Saved policy still applies to new writes (zero inherits).
        assert!(validate_change_policy(&change, &test_policy()).is_ok());
    }

    /// Historical replay verifies exact bytes plus header protocol only: a
    /// recorded change exceeding the CURRENT saved policy still replays (no
    /// re-judgement), while different content under the same event identity
    /// conflicts.
    #[test]
    fn replay_is_header_only_and_byte_exact() {
        let tight = RuntimeLimitPolicy::new(
            [1, 1, 1, 1, 1],
            [Some(1), Some(1), Some(1), Some(1), Some(1)],
        );
        let mut change = sample_change();
        change.values = RunLimits {
            observation_mode: 0,
            silence_seconds: 9000,
            tool_seconds: 9000,
            max_tool_calls: 9000,
            max_repeated_calls: 90,
            max_consecutive_errors: 90,
        };
        // A new write under today's tight saved policy would be refused...
        assert!(validate_change_policy(&change, &tight).is_err());
        // ...but an exact historical replay of the same recorded bytes is not
        // re-judged against it.
        let recorded = canonical_request_bytes(&change).unwrap();
        assert_eq!(verify_historical_replay(&recorded, &change).unwrap(), change);
        // Different content under the same event identity is a conflict.
        let mut other = change.clone();
        other.reason = "different".to_string();
        assert!(verify_historical_replay(&recorded, &other).is_err());
    }

    /// Negative current inputs have no inheritance meaning; old policy ceilings
    /// are distinct and never re-applied by the exact-byte replay helper.
    #[test]
    fn negative_request_values_refuse_intrinsically(){
        let mut change=sample_change();change.values.max_tool_calls=-1;
        assert!(matches!(validate_change_header(&change),Err(AgentLaunchError::Refused("run-limits-negative-value"))));
    }

    /// A malformed stored request is a content conflict like Go, never a
    /// new historical parse/metadata gate or an empty successful replay.
    #[test]
    fn malformed_recorded_request_is_an_exact_byte_conflict(){
        let change=sample_change();
        assert!(matches!(verify_historical_replay(b"not-json",&change),Err(AgentLaunchError::Refused("run-limits-event-conflict"))));
    }

    /// History rows decode with the exact Go tag set; rollback_of omits zero
    /// on encode and decodes when present; recorded metadata is not validated.
    #[test]
    fn history_entry_roundtrip_rollback_omitempty() {
        let entry = RunLimitsHistoryEntry {
            scope: SCOPE_MISSION.to_string(),
            mission_id: "m-1".to_string(),
            scope_key: String::new(),
            revision: 3,
            values: RunLimits {
                observation_mode: 1,
                silence_seconds: 0,
                tool_seconds: 0,
                max_tool_calls: 0,
                max_repeated_calls: 0,
                max_consecutive_errors: 0,
            },
            actor: "op".to_string(),
            reason: "observe incident".to_string(),
            at: "2025-01-01T00:00:00Z".to_string(),
            rollback_of: 2,
        };
        let bytes = crate::encode_model(&entry).unwrap();
        assert!(std::str::from_utf8(&bytes).unwrap().contains(r#""rollback_of":2"#));
        assert_eq!(decode_run_limits_history_entry(&bytes).unwrap(), entry);
        let mut cleared = entry.clone();
        cleared.rollback_of = 0;
        let bytes = crate::encode_model(&cleared).unwrap();
        assert!(!std::str::from_utf8(&bytes).unwrap().contains("rollback_of"));
        assert_eq!(decode_run_limits_history_entry(&bytes).unwrap(), cleared);
    }

    /// The generated RunLimitsConfigEntry DTO and shape are reused verbatim.
    #[test]
    fn config_entry_reuses_generated_dto() {
        let raw = br#"{"scope":"project","values":{"silence_seconds":0,"tool_seconds":0,"max_tool_calls":0,"max_repeated_calls":0,"max_consecutive_errors":0},"revision":0,"actor":"","reason":"","updated":""}"#;
        let entry = decode_run_limits_config_entry(raw).unwrap();
        assert_eq!(entry.scope, "project");
        assert_eq!(entry.revision, 0);
        assert_eq!(entry.values, RunLimits::default());
    }
}

/// Go rollback transport plus a decode-only native freshness token. No Serialize
/// or Debug: tokens cannot enter generic output/recorded changes. No IO/authority.
#[derive(Clone,Default,serde::Deserialize)]
#[serde(default)]
pub struct ApiRunLimitsRollback {
    pub scope:String,
    #[serde(rename="mission_id")]pub mission:String,
    #[serde(rename="scope_key")]pub key:String,
    pub to_revision:i64,pub event_id:String,pub reason:String,
    #[serde(rename="expected_revision")]pub revision:i64,
    preview_token:String,
}
impl ApiRunLimitsRollback{
    /// Construct the private mutation intent; store resolves historical values
    /// in its own snapshot. Schema1 is this rollback protocol, not policy/default
    /// adoption. The preview token remains transport-only and is never recorded.
    pub fn as_change(&self)->ApiRunLimitsConfigChange{
        ApiRunLimitsConfigChange{schema_version:1,event_id:self.event_id.clone(),scope:self.scope.clone(),mission:self.mission.clone(),key:self.key.clone(),revision:self.revision,values:RunLimits::default(),reason:self.reason.clone(),rollback_of:self.to_revision,preview_token:self.preview_token.clone()}
    }
}
/// Strict Go null/case/duplicate projection for rollback input. Unknown fields
/// refuse with original cause; no historical lookup or authorization occurs.
pub fn decode_api_run_limits_rollback(raw:&[u8])->Result<ApiRunLimitsRollback,AgentLaunchError>{
    crate::work::decode_mode(raw,&[
        ("scope",Kind::String),("mission_id",Kind::String),("scope_key",Kind::String),
        ("to_revision",Kind::Int64),("event_id",Kind::String),("reason",Kind::String),
        ("expected_revision",Kind::Int64),("preview_token",Kind::String)
    ],true).map_err(caused)
}
