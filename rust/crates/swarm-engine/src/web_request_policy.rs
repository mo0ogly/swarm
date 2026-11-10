//! Web request timeout policy: pure Go-wire DTOs, strict codecs and intrinsic
//! validation for the schema-1 `.swarm/web-requests.json` document, its
//! recorded history rows and its transport request. Mirrors
//! `internal/engine/web_requests_config.go`
//! (SHA-256 7e06c63b5c2739eeb2e6b7d3823368c7e484e436803411bfed5de7a5424751eb)
//! with no storage, network, lock, Root or UI code.
//!
//! # Contracts
//!
//! * Wire fidelity: Go signed-64 scalars, JSON tag names, absent/null zero
//!   values, last-duplicate-key merge and case-folded field matching come from
//!   the existing `work::decode_mode`; this module only supplies the exact
//!   `model_wire::Kind` shapes and typed targets. Struct declaration order
//!   mirrors the Go structs so re-encoded field order matches Go output.
//! * Go's custom `WebRequestsValues.UnmarshalJSON` is reproduced as a raw
//!   pre-pass (`guard_values`/`guard_occurrence`) over the real document using
//!   the shared `model_wire::{Boundaries, object, array, fold}` helpers: EVERY
//!   present `values` occurrence — the config root, a change/transport root, a
//!   history row and a history row's embedded change — must carry all three
//!   millisecond fields, and the last duplicate of each must be nonnull. This
//!   is the optional/pointer shape split: the `values` key itself is optional
//!   (absent ⇒ no custom unmarshal call ⇒ Go zero values; present `null` ⇒ the
//!   custom unmarshaler receives the null literal and refuses), while inside
//!   an occurrence Go's `*int64` wire makes an absent field and a null field
//!   the same nil refusal. An earlier duplicate occurrence that a later one
//!   overwrites is still refused, exactly as Go's decoder invokes the custom
//!   unmarshaler once per occurrence. The typed DTOs stay plain serde and a
//!   wholly absent `values` key decodes to zeros; plain `serde_json::Value` is
//!   deliberately never used for these guards because it loses duplicate
//!   keys. Scalar int64 syntax/range checking stays with `decode_mode`'s
//!   normal projection, which validates every occurrence it merges.
//! * Stored contract ([`validate_web_request_config`]): `schema_version == 1`
//!   ([`WEB_REQUEST_SCHEMA`]), `revision >= 0` and equal to the history
//!   length, rows sequential `1..=revision`, intrinsic bounds (nonnegative,
//!   ≤ [`WEB_REQUEST_MAX_SAFE_INTEGER_MS`]) on the current values and every
//!   row, and current values equal to the last row once the revision is
//!   positive. Historical Change/actor/at metadata retain Go's typed
//!   representation without a second validity gate: stale, empty or `null`
//!   historical changes retain their typed values (typed zero for null) and never
//!   rejected, and event-id replay or revision conflicts stay caller custody.
//! * Bounds: the only value ceiling is [`WEB_REQUEST_MAX_SAFE_INTEGER_MS`],
//!   the JSON/JavaScript exact-integer representation bound — a technical v1
//!   wire bound, not a waiting policy; zero disables browser cancellation at
//!   the consumer. Go's operator ceilings — reason 8..2000, the 1 MiB file
//!   bound and the 16 KiB request body cap — are deliberately not copied.
//! * NEW request reasons are nonempty safe text: nonempty after Go-style
//!   whitespace trimming (`crate::text::trim_space`; JSON decoding already
//!   guarantees UTF-8). No further character policy is invented. Ids use the
//!   existing `swarm_identity_contract::name` contract (Go `safeName`).
//! * Defaults: [`decode_web_request_defaults`] parses exactly the supplied
//!   versioned `config/web-requests.json` bytes. No IO, no numeric fallback
//!   literals, no policy adoption and no claim of installation.
//! * Transport extension: [`ApiWebRequestChange`] carries the recorded Go
//!   change plus the native `preview_token` binding. The token is skipped by
//!   serialization, redacted in `Debug` and structurally absent from
//!   [`ApiWebRequestChange::recorded_change`], so it can never be persisted
//!   inside the recorded original Go [`WebRequestChange`]; token custody and
//!   verification stay with the Root caller. Preview versus apply remains the
//!   caller's routing decision, as in Go.
//! * Nullable history: an absent or `null` Go slice stays `None` on
//!   [`WebRequestConfig::history`] and re-encodes as Go `null`; an explicit
//!   `[]` stays an empty list.
//!
//! # Errors
//!
//! Every failure is [`crate::AgentLaunchError`] — `Refused(&'static str)` for
//! rejected values or `caused(&'static str, source)` preserving the original
//! scan/parse/type cause. Codes: `web-request-values-required` (incomplete
//! present values occurrence), `web-request-values-invalid` (bounds/divergence),
//! `web-request-config-invalid`, `web-request-history-invalid`,
//! `web-request-change-invalid`, `web-request-defaults-invalid`,
//! `api-web-request-change-invalid`. No other error type is introduced;
//! decoding alone grants no authority beyond these intrinsic checks.
//!
//! # Side effects
//!
//! None. Pure functions over bytes and typed values: no IO, storage, network,
//! locking, clock, entropy, mutation or policy adoption. Input custody, the
//! regular-file/1 MiB read guard, file locking, event-id duplicate/idempotency
//! replay, revision-conflict handling, actor identity, timestamps, atomic
//! persistence and public response paths remain with the Root caller.
//!
//! # Integration
//!
//! Authorized GLM contribution: Main inspects, compiles and integrates this
//! module; nothing here executes, persists or closes a task.

use serde::{Deserialize, Serialize};

use crate::model_wire::{array, fold, object, Boundaries, Error as WireError, Fields, Kind};
use crate::AgentLaunchError as E;

/// The only accepted configuration/change schema version (Go v1).
pub const WEB_REQUEST_SCHEMA: i64 = 1;

/// Technical wire bound for one timeout: the largest exact integer shared by
/// Go's int64 values and JSON/JavaScript number representation (`1<<53 - 1`).
/// A representation bound, not a waiting policy; zero disables cancellation.
pub const WEB_REQUEST_MAX_SAFE_INTEGER_MS: i64 = (1 << 53) - 1;

/// Go `WebRequestsValues`: the three browser-cancellation timeouts in
/// milliseconds. Absence of the whole object decodes to Go zero values; a
/// PRESENT occurrence must carry all three fields nonnull — enforced on the
/// raw wire by `guard_values` because Go's custom `UnmarshalJSON` refuses any
/// incomplete occurrence, including one a later duplicate overwrites.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WebRequestValues {
    pub general_ms: i64,
    pub snapshot_ms: i64,
    pub connection_test_ms: i64,
}

/// Go `WebRequestsChange`: the recorded change exactly as stored in history
/// rows and accepted on the Go wire. No native extension fields, so a preview
/// token can never be persisted inside it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WebRequestChange {
    pub schema_version: i64,
    pub event_id: String,
    pub expected_revision: i64,
    pub values: WebRequestValues,
    pub reason: String,
}

/// Go `WebRequestsRevision`: one applied history row. `actor`, `at` and the
/// embedded `change` are recorded metadata with typed values preserved — this codec
/// never validates them (a `null` change stays typed zero).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WebRequestRevision {
    pub revision: i64,
    pub values: WebRequestValues,
    pub actor: String,
    pub at: String,
    pub change: WebRequestChange,
}

/// Go `WebRequestsConfig`: the schema-1 stored document and response view.
/// `history` keeps Go's null/empty slice distinction on the wire.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WebRequestConfig {
    pub schema_version: i64,
    pub revision: i64,
    pub values: WebRequestValues,
    pub history: Option<Vec<WebRequestRevision>>,
}

/// Typed transport request: the recorded Go change fields plus the native
/// `preview_token` extension. The token is transport-only and never recorded;
/// serialization skips it and `Debug` redacts it, so it cannot leak through
/// logs or echoed payloads. Deliberately no token verification or custody.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiWebRequestChange {
    pub schema_version: i64,
    pub event_id: String,
    pub expected_revision: i64,
    pub values: WebRequestValues,
    pub reason: String,
    #[serde(skip_serializing)]
    preview_token: String,
}

impl ApiWebRequestChange {
    /// Borrowed native preview binding token; transport-only, never persisted,
    /// verified or logged here. The Root caller owns verification.
    pub fn preview_token(&self) -> &str {
        &self.preview_token
    }

    /// The original Go change exactly as it must be recorded and replayed,
    /// with the native token extension structurally removed. No serialization
    /// of this value can ever contain the token.
    pub fn recorded_change(&self) -> WebRequestChange {
        WebRequestChange {
            schema_version: self.schema_version,
            event_id: self.event_id.clone(),
            expected_revision: self.expected_revision,
            values: self.values,
            reason: self.reason.clone(),
        }
    }
}

/// `Debug` never emits the preview token, so logs cannot leak it through this
/// type (serialization already skips it at the serde layer).
impl std::fmt::Debug for ApiWebRequestChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiWebRequestChange")
            .field("schema_version", &self.schema_version)
            .field("event_id", &self.event_id)
            .field("expected_revision", &self.expected_revision)
            .field("values", &self.values)
            .field("reason", &self.reason)
            .field("preview_token", &"[redacted]")
            .finish()
    }
}

// Exact Go wire shapes; work::decode_mode supplies scalar/null/case/merge.
static VALUES_FIELDS: Fields = &[
    ("general_ms", Kind::Int64),
    ("snapshot_ms", Kind::Int64),
    ("connection_test_ms", Kind::Int64),
];
fn values_shape() -> Fields {
    VALUES_FIELDS
}

static CHANGE_FIELDS: Fields = &[
    ("schema_version", Kind::Int64),
    ("event_id", Kind::String),
    ("expected_revision", Kind::Int64),
    ("values", Kind::Object(values_shape)),
    ("reason", Kind::String),
];
fn change_shape() -> Fields {
    CHANGE_FIELDS
}

static REVISION_ELEMENT: Kind = Kind::Object(revision_shape);

static REVISION_FIELDS: Fields = &[
    ("revision", Kind::Int64),
    ("values", Kind::Object(values_shape)),
    ("actor", Kind::String),
    ("at", Kind::String),
    ("change", Kind::Object(change_shape)),
];
fn revision_shape() -> Fields {
    REVISION_FIELDS
}

static CONFIG_FIELDS: Fields = &[
    ("schema_version", Kind::Int64),
    ("revision", Kind::Int64),
    ("values", Kind::Object(values_shape)),
    ("history", Kind::Array(&REVISION_ELEMENT)),
];
fn config_shape() -> Fields {
    CONFIG_FIELDS
}

static API_CHANGE_FIELDS: Fields = &[
    ("schema_version", Kind::Int64),
    ("event_id", Kind::String),
    ("expected_revision", Kind::Int64),
    ("values", Kind::Object(values_shape)),
    ("reason", Kind::String),
    ("preview_token", Kind::String),
];
fn api_change_shape() -> Fields {
    API_CHANGE_FIELDS
}

/// Failure of one values-presence guard pass.
enum ValuesGuard {
    /// A present `values` occurrence leaves one of the three millisecond
    /// pointers nil (field absent, or its last duplicate is `null`) — Go's
    /// custom `UnmarshalJSON` refuses such an occurrence, and a `null`
    /// occurrence reaches the custom unmarshaler with the null literal.
    Required,
    /// The raw structure could not be scanned; the scanner cause is preserved.
    Structure(WireError),
}

/// The three Go `*int64` wire fields every present values occurrence needs.
const VALUES_FIELD_NAMES: [&str; 3] = ["general_ms", "snapshot_ms", "connection_test_ms"];

/// Validate ONE present `values` occurrence against Go's custom
/// `WebRequestsValues.UnmarshalJSON`: the three millisecond fields must each
/// appear and the last duplicate of each must be a nonnull value (Go's `*int64`
/// wire leaves the pointer nil for both an absent field and a `null` one, and
/// the required check then refuses). Unknown or extra keys are ignored here
/// and refused by `decode_mode`'s strict projection; scalar syntax and range
/// stay with `decode_mode` too, which validates every merged occurrence.
fn guard_occurrence(raw: &[u8], boundaries: &Boundaries) -> Result<(), ValuesGuard> {
    let raw = raw.trim_ascii();
    if raw == b"null" {
        // A present `values: null` still reaches Go's custom unmarshaler.
        return Err(ValuesGuard::Required);
    }
    let folds = VALUES_FIELD_NAMES.map(fold);
    let mut last: [Option<&[u8]>; 3] = [None; 3];
    for (key, value) in object(raw, boundaries).map_err(ValuesGuard::Structure)? {
        let folded = fold(&key);
        for (slot, expected) in last.iter_mut().zip(folds.iter()) {
            if folded == *expected {
                *slot = Some(value);
            }
        }
    }
    for value in last {
        let Some(value) = value else {
            return Err(ValuesGuard::Required);
        };
        if value.trim_ascii() == b"null" {
            return Err(ValuesGuard::Required);
        }
    }
    Ok(())
}

/// Prevalidate every present `values` occurrence inside `raw` before the
/// normal `work::decode_mode` projection, guided by the same shape that
/// projection uses. The walk mirrors `model_wire`'s exact-then-fold field
/// matching: every key whose fold matches `values` and whose shape is the
/// values object is checked by [`guard_occurrence`], change objects are
/// descended and history arrays are walked row by row (covering
/// `history[].values` and `history[].change.values`), while `null` objects,
/// null arrays and null rows carry no occurrences (Go ignores null structs and
/// slices without calling the custom unmarshaler, and duplicate keys keep
/// merging). Structural failures preserve the scanner cause. The typed DTO and
/// plain serde cannot express this per-occurrence gate, and
/// `serde_json::Value` would lose the duplicate keys it depends on.
fn guard_values(raw: &[u8], shape: Fields) -> Result<(), ValuesGuard> {
    let boundaries = Boundaries::new(raw).map_err(ValuesGuard::Structure)?;
    let values_fold = fold("values");
    let mut pending: Vec<(&[u8], Fields)> = vec![(raw, shape)];
    while let Some((raw, fields)) = pending.pop() {
        let raw = raw.trim_ascii();
        if raw == b"null" {
            continue;
        }
        for (key, value) in object(raw, &boundaries).map_err(ValuesGuard::Structure)? {
            let Some(&(_, kind)) = fields
                .iter()
                .find(|(name, _)| *name == key)
                .or_else(|| fields.iter().find(|(name, _)| fold(name) == fold(&key)))
            else {
                continue; // unknown keys are decode_mode's strict refusal
            };
            if fold(&key) == values_fold {
                if matches!(kind, Kind::Object(_)) {
                    guard_occurrence(value, &boundaries)?;
                }
                continue;
            }
            match kind {
                Kind::Object(sub) => pending.push((value, sub())),
                Kind::Array(element) => {
                    let value = value.trim_ascii();
                    if value == b"null" {
                        continue; // Go nil slice: no occurrences, stays null
                    }
                    if let Kind::Object(sub) = *element {
                        for row in array(value, &boundaries).map_err(ValuesGuard::Structure)? {
                            pending.push((row, sub()));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

/// Intrinsic bounds for stored, source-supplied or requested values: each
/// timeout is a nonnegative exact integer within
/// [`WEB_REQUEST_MAX_SAFE_INTEGER_MS`]. No operator ceiling is applied; zero
/// disables the corresponding cancellation at the consumer.
/// Error: `web-request-values-invalid`.
pub fn validate_web_request_values(v: &WebRequestValues) -> Result<(), E> {
    for milliseconds in [v.general_ms, v.snapshot_ms, v.connection_test_ms] {
        if !(0..=WEB_REQUEST_MAX_SAFE_INTEGER_MS).contains(&milliseconds) {
            return Err(E::Refused("web-request-values-invalid"));
        }
    }
    Ok(())
}

/// Full stored-document contract, matching Go's read acceptance: schema 1,
/// nonnegative revision equal to the history length, rows sequential
/// `1..=revision`, intrinsic bounds on current values and every row, and
/// current values equal to the last row once the revision is positive.
/// Historical Change, actor and at metadata are typed, unauthenticated
/// records — no new semantic rejection is added for stale, empty or `null`
/// changes, repeated event ids or absent history. Re-run on any newly
/// composed config before persistence.
/// Errors: `web-request-config-invalid`, `web-request-history-invalid`,
/// `web-request-values-invalid`.
pub fn validate_web_request_config(c: &WebRequestConfig) -> Result<(), E> {
    if c.schema_version != WEB_REQUEST_SCHEMA || c.revision < 0 {
        return Err(E::Refused("web-request-config-invalid"));
    }
    let rows = c.history.as_deref().unwrap_or(&[]);
    if i64::try_from(rows.len()).ok() != Some(c.revision) {
        return Err(E::Refused("web-request-history-invalid"));
    }
    validate_web_request_values(&c.values)?;
    for (index, row) in rows.iter().enumerate() {
        if Some(row.revision) != i64::try_from(index).ok().and_then(|v| v.checked_add(1)) {
            return Err(E::Refused("web-request-history-invalid"));
        }
        validate_web_request_values(&row.values)?;
    }
    if let Some(last) = rows.last() {
        if c.values != last.values {
            return Err(E::Refused("web-request-values-invalid"));
        }
    }
    Ok(())
}

/// Strictly decode the stored schema-1 document (Go `strict` semantics: no
/// unknown fields, Go scalar/null/case/duplicate merge) after the raw
/// per-occurrence values gate, then validate the full stored contract. Pure:
/// reading `.swarm/web-requests.json` stays with the Root caller.
/// Errors: `web-request-config-invalid` (caused/refused) plus the
/// [`validate_web_request_config`] statics and `web-request-values-required`
/// for any incomplete present `values` occurrence.
pub fn decode_web_request_config(raw: &[u8]) -> Result<WebRequestConfig, E> {
    guard_values(raw, config_shape()).map_err(|failure| match failure {
        ValuesGuard::Required => E::Refused("web-request-values-required"),
        ValuesGuard::Structure(error) => E::caused("web-request-config-invalid", error),
    })?;
    let config: WebRequestConfig = crate::work::decode_mode(raw, config_shape(), true)
        .map_err(|error| E::caused("web-request-config-invalid", error))?;
    validate_web_request_config(&config)?;
    Ok(config)
}

/// Strictly decode the supplied versioned `config/web-requests.json` source
/// defaults — the document root IS one `values` occurrence, so Go's custom
/// unmarshaler gate applies to it directly (a present null refuses; a wholly
/// different shape is a preserved type error) — then apply the intrinsic value
/// bounds. The caller supplies the exact versioned bytes; no IO, no fallback
/// literals, no policy adoption and no claim of installation.
/// Errors: `web-request-defaults-invalid` (caused) plus
/// `web-request-values-required` and `web-request-values-invalid`.
pub fn decode_web_request_defaults(raw: &[u8]) -> Result<WebRequestValues, E> {
    let boundaries =
        Boundaries::new(raw).map_err(|error| E::caused("web-request-defaults-invalid", error))?;
    guard_occurrence(raw, &boundaries).map_err(|failure| match failure {
        ValuesGuard::Required => E::Refused("web-request-values-required"),
        ValuesGuard::Structure(error) => E::caused("web-request-defaults-invalid", error),
    })?;
    let values: WebRequestValues = crate::work::decode_mode(raw, values_shape(), true)
        .map_err(|error| E::caused("web-request-defaults-invalid", error))?;
    validate_web_request_values(&values)?;
    Ok(values)
}

/// Strictly decode one transport change request (recorded Go change fields
/// plus the native `preview_token` extension) and check the new-request
/// format via [`validate_web_request_change`]: schema 1, contract `event_id`,
/// nonnegative expected revision, nonempty trimmed reason and value bounds.
/// Go's 8..2000 reason cap is deliberately absent. The token is carried
/// unpersisted and unverified; no authority is granted.
/// Errors: `api-web-request-change-invalid` (caused) plus
/// `web-request-values-required` and the [`validate_web_request_change`]
/// statics.
pub fn decode_api_web_request_change(raw: &[u8]) -> Result<ApiWebRequestChange, E> {
    guard_values(raw, api_change_shape()).map_err(|failure| match failure {
        ValuesGuard::Required => E::Refused("web-request-values-required"),
        ValuesGuard::Structure(error) => E::caused("api-web-request-change-invalid", error),
    })?;
    let request: ApiWebRequestChange = crate::work::decode_mode(raw, api_change_shape(), true)
        .map_err(|error| E::caused("api-web-request-change-invalid", error))?;
    validate_web_request_change(&request.recorded_change())?;
    Ok(request)
}

/// Validate one NEW requested change before it is applied — the Go
/// `changeWebRequests` header guards minus the store, the 8..2000 reason cap
/// and every historical-metadata gate: `schema_version == 1`, `event_id`
/// satisfying the `swarm_identity_contract::name` contract (Go `safeName`),
/// nonnegative `expected_revision`, and a reason that is nonempty after
/// Go-style whitespace trimming; then the intrinsic value bounds. Deliberately
/// NOT checked here: file/body size caps, event-id replay, revision conflicts,
/// actor identity and timestamps — caller custody. Errors:
/// `web-request-change-invalid` (header) and `web-request-values-invalid`
/// (bounds).
pub fn validate_web_request_change(change: &WebRequestChange) -> Result<(), E> {
    if change.schema_version != WEB_REQUEST_SCHEMA
        || !swarm_identity_contract::name(change.event_id.as_bytes())
        || change.expected_revision < 0
        || crate::text::trim_space(&change.reason).is_empty()
    {
        return Err(E::Refused("web-request-change-invalid"));
    }
    validate_web_request_values(&change.values)
}

#[cfg(test)]
mod web_request_policy_tests {
    use super::*;
    use crate::AgentLaunchError;

    fn values(general: i64, snapshot: i64, connection: i64) -> WebRequestValues {
        WebRequestValues {
            general_ms: general,
            snapshot_ms: snapshot,
            connection_test_ms: connection,
        }
    }

    /// The supplied versioned config/web-requests.json bytes decode to the
    /// three stored timeouts with no fallback literals involved.
    #[test]
    fn defaults_decode_versioned_source_bytes() {
        let raw = br#"{
  "general_ms": 15000,
  "snapshot_ms": 60000,
  "connection_test_ms": 65000
}"#;
        assert_eq!(
            decode_web_request_defaults(raw).unwrap(),
            values(15000, 60000, 65000)
        );
    }

    /// The defaults root IS one values occurrence: a present-but-partial or
    /// null document is Go's custom-unmarshaler refusal, while unknown fields
    /// stay a strict decode refusal.
    #[test]
    fn defaults_present_root_requires_three_nonnull_fields() {
        assert!(matches!(
            decode_web_request_defaults(br#"{"general_ms":1,"snapshot_ms":2}"#),
            Err(AgentLaunchError::Refused("web-request-values-required"))
        ));
        assert!(matches!(
            decode_web_request_defaults(b"null"),
            Err(AgentLaunchError::Refused("web-request-values-required"))
        ));
        assert!(matches!(
            decode_web_request_defaults(br#"{}"#),
            Err(AgentLaunchError::Refused("web-request-values-required"))
        ));
        assert!(decode_web_request_defaults(
            br#"{"general_ms":1,"snapshot_ms":2,"connection_test_ms":3,"extra":4}"#
        )
        .is_err());
    }

    /// A wholly absent `values` key never reaches the unmarshal gate and
    /// decodes to Go zero values — accepted only while the stored contract
    /// still holds (revision 0 with no rows); a positive revision diverges.
    #[test]
    fn config_missing_whole_values_defaults_zero_only_at_revision_zero() {
        let config = decode_web_request_config(br#"{"schema_version":1,"revision":0}"#).unwrap();
        assert_eq!(config.values, WebRequestValues::default());
        assert_eq!(config.history, None);
        let populated = br#"{"schema_version":1,"revision":1,"history":[{"revision":1,"values":{"general_ms":5,"snapshot_ms":5,"connection_test_ms":5},"actor":"op","at":"t","change":{"schema_version":1,"event_id":"e-1","expected_revision":0,"values":{"general_ms":5,"snapshot_ms":5,"connection_test_ms":5},"reason":"r"}}]}"#;
        assert!(matches!(
            decode_web_request_config(populated),
            Err(AgentLaunchError::Refused("web-request-values-invalid"))
        ));
    }

    /// Every PRESENT values occurrence — config root, request root, history
    /// row and history row's change — refuses partial or null shapes.
    #[test]
    fn present_partial_or_null_values_refuse_everywhere() {
        let config_partial = br#"{"schema_version":1,"revision":0,"values":{"general_ms":1}}"#;
        assert!(matches!(
            decode_web_request_config(config_partial),
            Err(AgentLaunchError::Refused("web-request-values-required"))
        ));
        let config_null = br#"{"schema_version":1,"revision":0,"values":null}"#;
        assert!(matches!(
            decode_web_request_config(config_null),
            Err(AgentLaunchError::Refused("web-request-values-required"))
        ));
        let request_empty = br#"{"schema_version":1,"event_id":"web-1","expected_revision":0,"values":{},"reason":"r"}"#;
        assert!(matches!(
            decode_api_web_request_change(request_empty),
            Err(AgentLaunchError::Refused("web-request-values-required"))
        ));
        let row_partial = br#"{"schema_version":1,"revision":1,"values":{"general_ms":1,"snapshot_ms":1,"connection_test_ms":1},"history":[{"revision":1,"values":{"general_ms":1},"actor":"op","at":"t","change":{"schema_version":1,"event_id":"e-1","expected_revision":0,"values":{"general_ms":1,"snapshot_ms":1,"connection_test_ms":1},"reason":"r"}}]}"#;
        assert!(matches!(
            decode_web_request_config(row_partial),
            Err(AgentLaunchError::Refused("web-request-values-required"))
        ));
        let change_null = br#"{"schema_version":1,"revision":1,"values":{"general_ms":1,"snapshot_ms":1,"connection_test_ms":1},"history":[{"revision":1,"values":{"general_ms":1,"snapshot_ms":1,"connection_test_ms":1},"actor":"op","at":"t","change":{"schema_version":1,"event_id":"e-1","expected_revision":0,"values":null,"reason":"r"}}]}"#;
        assert!(matches!(
            decode_web_request_config(change_null),
            Err(AgentLaunchError::Refused("web-request-values-required"))
        ));
    }

    /// Go's decoder runs the custom unmarshaler once per occurrence: an
    /// earlier duplicate that a later complete one overwrites is still
    /// refused, and a later null occurrence refuses too.
    #[test]
    fn duplicate_partial_then_complete_values_refuse() {
        let partial_first = br#"{"schema_version":1,"revision":0,"values":{"general_ms":1},"values":{"general_ms":1,"snapshot_ms":2,"connection_test_ms":3}}"#;
        assert!(matches!(
            decode_web_request_config(partial_first),
            Err(AgentLaunchError::Refused("web-request-values-required"))
        ));
        let null_second = br#"{"schema_version":1,"revision":0,"values":{"general_ms":1,"snapshot_ms":2,"connection_test_ms":3},"values":null}"#;
        assert!(matches!(
            decode_web_request_config(null_second),
            Err(AgentLaunchError::Refused("web-request-values-required"))
        ));
    }

    /// Case-folded keys and complete duplicate occurrences decode with Go's
    /// last-wins merge.
    #[test]
    fn case_folded_and_duplicate_keys_take_last_value() {
        let folded = br#"{"schema_version":1,"revision":0,"VALUES":{"General_MS":10,"general_ms":15,"Snapshot_ms":60,"CONNECTION_TEST_MS":65}}"#;
        assert_eq!(
            decode_web_request_config(folded).unwrap().values,
            values(15, 60, 65)
        );
        let duplicated = br#"{"schema_version":1,"revision":0,"values":{"general_ms":1,"snapshot_ms":1,"connection_test_ms":1},"values":{"general_ms":7,"snapshot_ms":8,"connection_test_ms":9}}"#;
        assert_eq!(
            decode_web_request_config(duplicated).unwrap().values,
            values(7, 8, 9)
        );
    }

    /// `history: null` stays Go nil on decode and re-encode, `[]` stays an
    /// empty list, and stale historical Change metadata — including a null
    /// change object — is preserved verbatim with no second validity gate.
    #[test]
    fn nullable_history_and_stale_change_metadata_are_preserved() {
        let nulled = br#"{"schema_version":1,"revision":0,"values":{"general_ms":0,"snapshot_ms":0,"connection_test_ms":0},"history":null}"#;
        let config = decode_web_request_config(nulled).unwrap();
        assert_eq!(config.history, None);
        let encoded = crate::encode_model(&config).unwrap();
        assert!(std::str::from_utf8(&encoded)
            .unwrap()
            .contains(r#""history":null"#));

        let empty = br#"{"schema_version":1,"revision":0,"values":{"general_ms":0,"snapshot_ms":0,"connection_test_ms":0},"history":[]}"#;
        assert_eq!(
            decode_web_request_config(empty).unwrap().history,
            Some(vec![])
        );

        let stale = br#"{"schema_version":1,"revision":1,"values":{"general_ms":1,"snapshot_ms":2,"connection_test_ms":3},"history":[{"revision":1,"values":{"general_ms":1,"snapshot_ms":2,"connection_test_ms":3},"actor":"op","at":"2020-01-01T00:00:00Z","change":{"schema_version":9,"event_id":"","expected_revision":-5,"values":{"general_ms":1,"snapshot_ms":2,"connection_test_ms":3},"reason":""}}]}"#;
        let config = decode_web_request_config(stale).unwrap();
        let row = &config.history.as_ref().unwrap()[0];
        assert_eq!(row.change.schema_version, 9);
        assert_eq!(row.change.event_id, "");
        assert_eq!(row.change.expected_revision, -5);
        assert_eq!(row.actor, "op");

        let null_change = br#"{"schema_version":1,"revision":1,"values":{"general_ms":1,"snapshot_ms":2,"connection_test_ms":3},"history":[{"revision":1,"values":{"general_ms":1,"snapshot_ms":2,"connection_test_ms":3},"actor":"op","at":"t","change":null}]}"#;
        let config = decode_web_request_config(null_change).unwrap();
        assert_eq!(
            config.history.as_ref().unwrap()[0].change,
            WebRequestChange::default()
        );
    }

    /// The native preview token is decoded for caller verification but never
    /// serialized, never debug-printed and structurally absent from the
    /// recorded change.
    #[test]
    fn native_preview_token_is_transport_only() {
        let raw = br#"{"schema_version":1,"event_id":"web-1","expected_revision":2,"values":{"general_ms":1,"snapshot_ms":2,"connection_test_ms":3},"reason":"r","preview_token":"sekrit"}"#;
        let request = decode_api_web_request_change(raw).unwrap();
        assert_eq!(request.preview_token(), "sekrit");
        assert_eq!(
            request.recorded_change(),
            WebRequestChange {
                schema_version: 1,
                event_id: "web-1".to_string(),
                expected_revision: 2,
                values: values(1, 2, 3),
                reason: "r".to_string(),
            }
        );
        let encoded = crate::encode_model(&request).unwrap();
        let text = std::str::from_utf8(&encoded).unwrap();
        assert!(!text.contains("sekrit"));
        assert!(!text.contains("preview_token"));
        assert!(!format!("{:?}", request).contains("sekrit"));
    }

    /// The only value bound is the nonnegative exact-integer representation
    /// bound, enforced on the typed validator and on every decode path.
    #[test]
    fn value_bounds_are_nonnegative_safe_integers() {
        assert_eq!(WEB_REQUEST_MAX_SAFE_INTEGER_MS, 9_007_199_254_740_991);
        assert!(validate_web_request_values(&values(
            WEB_REQUEST_MAX_SAFE_INTEGER_MS,
            WEB_REQUEST_MAX_SAFE_INTEGER_MS,
            WEB_REQUEST_MAX_SAFE_INTEGER_MS
        ))
        .is_ok());
        assert!(matches!(
            validate_web_request_values(&values(WEB_REQUEST_MAX_SAFE_INTEGER_MS + 1, 0, 0)),
            Err(AgentLaunchError::Refused("web-request-values-invalid"))
        ));
        assert!(matches!(
            validate_web_request_values(&values(0, -1, 0)),
            Err(AgentLaunchError::Refused("web-request-values-invalid"))
        ));
        let exact = br#"{"schema_version":1,"event_id":"web-1","expected_revision":0,"values":{"general_ms":9007199254740991,"snapshot_ms":0,"connection_test_ms":0},"reason":"r"}"#;
        assert!(decode_api_web_request_change(exact).is_ok());
        let over = br#"{"schema_version":1,"event_id":"web-1","expected_revision":0,"values":{"general_ms":9007199254740992,"snapshot_ms":0,"connection_test_ms":0},"reason":"r"}"#;
        assert!(decode_api_web_request_change(over).is_err());
        let negative = br#"{"schema_version":1,"event_id":"web-1","expected_revision":0,"values":{"general_ms":-1,"snapshot_ms":0,"connection_test_ms":0},"reason":"r"}"#;
        assert!(decode_api_web_request_change(negative).is_err());
    }

    /// Stored acceptance mirrors Go's read checks: schema, nonnegative
    /// revision equal to history length, sequential rows and current/last
    /// equality — with no historical-metadata gate.
    #[test]
    fn stored_contract_refuses_schema_sequence_and_divergence() {
        let schema = br#"{"schema_version":2,"revision":0}"#;
        assert!(matches!(
            decode_web_request_config(schema),
            Err(AgentLaunchError::Refused("web-request-config-invalid"))
        ));
        let negative = br#"{"schema_version":1,"revision":-1}"#;
        assert!(matches!(
            decode_web_request_config(negative),
            Err(AgentLaunchError::Refused("web-request-config-invalid"))
        ));
        let length = br#"{"schema_version":1,"revision":1}"#;
        assert!(matches!(
            decode_web_request_config(length),
            Err(AgentLaunchError::Refused("web-request-history-invalid"))
        ));
        let sequence = br#"{"schema_version":1,"revision":2,"values":{"general_ms":1,"snapshot_ms":2,"connection_test_ms":3},"history":[{"revision":2,"values":{"general_ms":1,"snapshot_ms":2,"connection_test_ms":3},"actor":"","at":"","change":null},{"revision":2,"values":{"general_ms":1,"snapshot_ms":2,"connection_test_ms":3},"actor":"","at":"","change":null}]}"#;
        assert!(matches!(
            decode_web_request_config(sequence),
            Err(AgentLaunchError::Refused("web-request-history-invalid"))
        ));
        let divergent = br#"{"schema_version":1,"revision":1,"values":{"general_ms":9,"snapshot_ms":2,"connection_test_ms":3},"history":[{"revision":1,"values":{"general_ms":1,"snapshot_ms":2,"connection_test_ms":3},"actor":"","at":"","change":null}]}"#;
        assert!(matches!(
            decode_web_request_config(divergent),
            Err(AgentLaunchError::Refused("web-request-values-invalid"))
        ));
    }

    /// New-request guards: schema 1, name-contract event id, nonnegative
    /// expected revision, nonempty trimmed reason. Go's 8..2000 reason cap is
    /// deliberately absent ("r" is accepted).
    #[test]
    fn new_request_header_guards_without_go_caps() {
        let base = WebRequestChange {
            schema_version: 1,
            event_id: "web-1".to_string(),
            expected_revision: 0,
            values: values(1, 2, 3),
            reason: "r".to_string(),
        };
        assert!(validate_web_request_change(&base).is_ok());
        let mut change = base.clone();
        change.schema_version = 2;
        assert!(matches!(
            validate_web_request_change(&change),
            Err(AgentLaunchError::Refused("web-request-change-invalid"))
        ));
        change = base.clone();
        change.event_id = "bad id".to_string();
        assert!(validate_web_request_change(&change).is_err());
        change = base.clone();
        change.expected_revision = -1;
        assert!(matches!(
            validate_web_request_change(&change),
            Err(AgentLaunchError::Refused("web-request-change-invalid"))
        ));
        change = base.clone();
        change.reason = "   ".to_string();
        assert!(validate_web_request_change(&change).is_err());
    }

    /// A transport request without a token decodes and validates; a negative
    /// expected revision refuses through the same combined gate.
    #[test]
    fn api_change_decodes_and_validates() {
        let raw = br#"{"schema_version":1,"event_id":"web-1","expected_revision":3,"values":{"general_ms":15000,"snapshot_ms":60000,"connection_test_ms":65000},"reason":"tighten limits"}"#;
        let request = decode_api_web_request_change(raw).unwrap();
        assert_eq!(request.preview_token(), "");
        assert_eq!(
            request.recorded_change().values,
            values(15000, 60000, 65000)
        );
        let negative = br#"{"schema_version":1,"event_id":"web-1","expected_revision":-1,"values":{"general_ms":1,"snapshot_ms":1,"connection_test_ms":1},"reason":"r"}"#;
        assert!(matches!(
            decode_api_web_request_change(negative),
            Err(AgentLaunchError::Refused("web-request-change-invalid"))
        ));
    }

    /// Re-encoding follows Go's declaration order (change fields inside
    /// history rows included) and round-trips through the stored decoder.
    #[test]
    fn config_encodes_in_go_field_order_and_round_trips() {
        let config = WebRequestConfig {
            schema_version: 1,
            revision: 1,
            values: values(1, 2, 3),
            history: Some(vec![WebRequestRevision {
                revision: 1,
                values: values(1, 2, 3),
                actor: "op".to_string(),
                at: "t".to_string(),
                change: WebRequestChange {
                    schema_version: 1,
                    event_id: "e-1".to_string(),
                    expected_revision: 0,
                    values: values(1, 2, 3),
                    reason: "r".to_string(),
                },
            }]),
        };
        let encoded = crate::encode_model(&config).unwrap();
        assert_eq!(
            std::str::from_utf8(&encoded).unwrap(),
            r#"{"schema_version":1,"revision":1,"values":{"general_ms":1,"snapshot_ms":2,"connection_test_ms":3},"history":[{"revision":1,"values":{"general_ms":1,"snapshot_ms":2,"connection_test_ms":3},"actor":"op","at":"t","change":{"schema_version":1,"event_id":"e-1","expected_revision":0,"values":{"general_ms":1,"snapshot_ms":2,"connection_test_ms":3},"reason":"r"}}]}"#
        );
        assert_eq!(decode_web_request_config(&encoded).unwrap(), config);
    }

    /// Strict decoding refuses unknown fields at every level, as Go's
    /// `strict` helper does on both the stored file and the request body.
    #[test]
    fn strict_decoding_refuses_unknown_fields() {
        assert!(
            decode_web_request_config(br#"{"schema_version":1,"revision":0,"extra":1}"#).is_err()
        );
        assert!(decode_web_request_config(br#"{"schema_version":1,"revision":0,"values":{"general_ms":1,"snapshot_ms":2,"connection_test_ms":3,"extra":4}}"#).is_err());
        assert!(decode_web_request_defaults(
            br#"{"general_ms":1,"snapshot_ms":2,"connection_test_ms":3,"extra":4}"#
        )
        .is_err());
    }
}
