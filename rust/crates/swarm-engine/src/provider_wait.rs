//! Provider wait configuration: pure Go-wire DTOs, strict codecs and intrinsic
//! validation for the schema-1 `.swarm/provider-wait.json` document, its
//! recorded change rows and its transport request. Mirrors
//! `internal/engine/provider_wait_config.go` with no storage, network, Root or
//! UI code.
//!
//! # Contracts
//!
//! * Wire fidelity: Go signed-64 scalars, JSON tag names, absent/null zero
//!   values, last-duplicate-key merge and case-folded field matching come from
//!   the existing `work::decode_mode`; this module only supplies the exact
//!   `model_wire::Kind` shapes and typed targets. Struct declaration order
//!   mirrors the Go structs so re-encoded field order matches Go output.
//! * Nullable history: an absent or `null` Go slice is one empty semantic list
//!   ([`ProviderWaitConfig::history`]). The typed field stays `Option<Vec<_>>`
//!   so re-encoding still distinguishes Go `null` from `[]`.
//! * Original typed record values are preserved: stored reasons, actors,
//!   timestamps and embedded changes are never trimmed, rewritten or dropped.
//!   `actor`/`at` are unverified metadata — carried and returned, never
//!   interpreted for identity, ordering or authority.
//! * Stored contract ([`validate_stored_config`]): `schema_version == 1`
//!   ([`PROVIDER_WAIT_SCHEMA`]), `revision == history length`, rows sequential
//!   `1..=revision`, intrinsic value bounds on current values and every row,
//!   current values equal to the last row once the revision is positive.
//!   Historical Change/actor/at metadata retain Go's typed representation without
//!   inventing a second validity gate. New requests are validated separately;
//!   replay compares the first matching recorded change exactly, as Go does.
//! * Bounds: `silence_seconds`/`max_duration_seconds` are nonnegative and
//!   capped by [`PROVIDER_WAIT_MAX_SECONDS`], the legacy nanosecond
//!   `time.Duration` representation bound. That is a technical v1 wire bound,
//!   not a business cutoff; zero disables the corresponding timer at the
//!   consumer (`max_duration_seconds` will control native connection probes).
//!   Stored/source leases must be positive. NEW requested lease choices are
//!   governed exclusively by the passed current
//!   [`PlanningActionPolicy::check_lease`]. Go's operator ceilings — lease
//!   5..300, reason 8..2000, the 1 MiB file bound and history count — are
//!   deliberately not copied.
//! * NEW request reasons are nonempty safe text: nonempty after Go-style whitespace
//!   trimming (JSON decoding already guarantees UTF-8). No further character
//!   policy is invented. Ids use the existing `swarm_identity_contract::name`
//!   contract.
//! * Defaults: [`decode_provider_wait_defaults`] parses exactly the supplied
//!   versioned `config/provider-wait.json` bytes. No IO, no numeric fallback
//!   literals, no policy adoption and no claim of installation.
//! * Transport extension: [`ApiProviderWaitChange`] carries the recorded Go
//!   change plus the native `preview_token` binding.
//!   [`ApiProviderWaitChange::recorded_change`] strips the token, so a token
//!   is never persisted inside the recorded original Go
//!   [`ProviderWaitChange`]; token custody and verification stay with the Root
//!   caller. Preview versus apply remains the caller's routing decision, as in
//!   Go.
//!
//! # Errors
//!
//! Every failure is [`crate::AgentLaunchError`] — `Refused(&'static str)` for
//! rejected values or `caused(&'static str, source)` preserving the original
//! parse/type/settings cause. No other error type is introduced. Decoding
//! alone grants no authority beyond these intrinsic checks.
//!
//! # Side effects
//!
//! None. Pure functions over bytes and typed values: no IO, storage, network,
//! clock, entropy, mutation or policy adoption. Preview/apply custody, file
//! locking, local file bounds, event-id duplicate/idempotency and
//! revision-conflict handling, atomic persistence and public response paths
//! remain with the Root caller. Native planning-wait settings27 remain
//! separately identified until explicit migration; no unified history is
//! invented here.

use serde::{Deserialize, Serialize};

use crate::model_wire::{Fields, Kind};
use crate::planning_action_policy::PlanningActionPolicy;
use crate::AgentLaunchError as E;

/// The only accepted configuration/change schema version (Go v1).
pub const PROVIDER_WAIT_SCHEMA: i64 = 1;

/// Legacy technical bound for one wait: the largest whole second count still
/// representable by Go's nanosecond `time.Duration` (`i64::MAX / 1e9`, the v1
/// wire's 64-bit overflow guard). A representation bound, not a business
/// cutoff; zero silence/max disables the corresponding timer at the consumer.
pub const PROVIDER_WAIT_MAX_SECONDS: i64 = i64::MAX / 1_000_000_000;

/// Go `ProviderWaitValues`: the three waiting knobs. Absence/null decode to
/// Go zero values; serde names are the exact Go JSON tags.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProviderWaitValues {
    pub silence_seconds: i64,
    pub max_duration_seconds: i64,
    pub planning_lease_seconds: i64,
}

/// Go `ProviderWaitChange`: the recorded change exactly as stored in history
/// rows and accepted on the Go wire. No native extension fields, so a preview
/// token can never be persisted inside it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProviderWaitChange {
    pub schema_version: i64,
    pub event_id: String,
    pub expected_revision: i64,
    pub values: ProviderWaitValues,
    pub reason: String,
}

/// Go `ProviderWaitRevision`: one applied history row. `actor`/`at` are
/// unverified operator metadata preserved verbatim; they authorize nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProviderWaitRevision {
    pub revision: i64,
    pub values: ProviderWaitValues,
    pub actor: String,
    pub at: String,
    pub change: ProviderWaitChange,
}

/// Go `ProviderWaitConfig`: the schema-1 stored document and response view.
/// `history` keeps Go's null/empty slice distinction on the wire while
/// [`ProviderWaitConfig::history`] presents it as one empty semantic list.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProviderWaitConfig {
    pub schema_version: i64,
    pub revision: i64,
    pub values: ProviderWaitValues,
    pub history: Option<Vec<ProviderWaitRevision>>,
}

impl ProviderWaitConfig {
    /// Semantic history view: an absent or `null` Go slice is an empty list.
    pub fn history(&self) -> &[ProviderWaitRevision] {
        self.history.as_deref().unwrap_or(&[])
    }
}

/// Typed transport request: the recorded Go change fields plus the native
/// `preview_token` extension. The token is transport-only and never recorded.
/// Deliberately no `Serialize`/`Debug`: a token must not leak through logs or
/// echoed payloads.
#[derive(Default, Deserialize)]
#[serde(default)]
pub struct ApiProviderWaitChange {
    pub(crate) schema_version: i64,
    pub(crate) event_id: String,
    pub(crate) expected_revision: i64,
    pub(crate) values: ProviderWaitValues,
    pub(crate) reason: String,
    pub(crate) preview_token: String,
}

impl ApiProviderWaitChange {
    /// Native preview binding token; transport-only, never persisted.
    pub fn preview_token(&self) -> &str {
        &self.preview_token
    }

    /// The original Go change exactly as it must be recorded and replayed,
    /// with the native token extension removed.
    pub fn recorded_change(&self) -> ProviderWaitChange {
        ProviderWaitChange {
            schema_version: self.schema_version,
            event_id: self.event_id.clone(),
            expected_revision: self.expected_revision,
            values: self.values,
            reason: self.reason.clone(),
        }
    }
}

// Exact Go wire shapes; work::decode_mode supplies scalar/null/case/merge.
static VALUES_FIELDS: Fields = &[
    ("silence_seconds", Kind::Int64),
    ("max_duration_seconds", Kind::Int64),
    ("planning_lease_seconds", Kind::Int64),
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
fn request_shape() -> Fields {
    API_CHANGE_FIELDS
}

/// Technical v1 duration bounds only (nonnegative, within the legacy
/// nanosecond representation). No operator ceiling is applied here.
fn durations_within_v1(v: &ProviderWaitValues) -> bool {
    (0..=PROVIDER_WAIT_MAX_SECONDS).contains(&v.silence_seconds)
        && (0..=PROVIDER_WAIT_MAX_SECONDS).contains(&v.max_duration_seconds)
}

/// Intrinsic bounds for stored or source-supplied values: nonnegative
/// silence/max within the technical v1 bound and a positive lease. No policy
/// is consulted; Go's 5..300 lease ceiling is not copied.
/// Error: `provider-wait-values-invalid`.
pub fn validate_stored_values(v: &ProviderWaitValues) -> Result<(), E> {
    if !durations_within_v1(v) || v.planning_lease_seconds < 1 {
        return Err(E::Refused("provider-wait-values-invalid"));
    }
    Ok(())
}

/// Full stored-document contract: schema/revision bookkeeping, sequential
/// rows, per-row value bounds and (once the revision is positive) current values
/// equal to the last row, matching Go's read acceptance. Historical Change,
/// actor and at are typed, unauthenticated metadata; no new semantic rejection
/// is added for absent/null/inconsistent changes or repeated event ids. New
/// requests remain checked before exact first-match replay; no authority follows.
/// Re-run on any newly composed config before persistence.
/// Errors: `provider-wait-config-invalid`, `provider-wait-history-invalid`,
/// `provider-wait-values-invalid`.
pub fn validate_stored_config(c: &ProviderWaitConfig) -> Result<(), E> {
    if c.schema_version != PROVIDER_WAIT_SCHEMA || c.revision < 0 {
        return Err(E::Refused("provider-wait-config-invalid"));
    }
    let rows = c.history();
    if i64::try_from(rows.len()).ok() != Some(c.revision) {
        return Err(E::Refused("provider-wait-history-invalid"));
    }
    validate_stored_values(&c.values)?;
    for (index, row) in rows.iter().enumerate() {
        if Some(row.revision) != i64::try_from(index).ok().and_then(|v| v.checked_add(1)) {
            return Err(E::Refused("provider-wait-history-invalid"));
        }
        validate_stored_values(&row.values)?;
    }
    if let Some(last) = rows.last() {
        if c.values != last.values {
            return Err(E::Refused("provider-wait-values-invalid"));
        }
    }
    Ok(())
}

/// Strictly decode the stored schema-1 document (Go `strict` semantics: no
/// unknown fields, Go scalar/null/case/duplicate merge) and validate the full
/// stored contract. Pure: reading the file stays with the Root caller.
/// Errors: `provider-wait-config-invalid` (caused/refused) plus the
/// [`validate_stored_config`] statics.
pub fn decode_provider_wait_config(raw: &[u8]) -> Result<ProviderWaitConfig, E> {
    let c: ProviderWaitConfig = crate::work::decode_mode(raw, config_shape(), true)
        .map_err(|e| E::caused("provider-wait-config-invalid", e))?;
    validate_stored_config(&c)?;
    Ok(c)
}

/// Strictly decode the supplied versioned `config/provider-wait.json` source
/// defaults and apply the intrinsic stored-value bounds. The caller supplies
/// the exact versioned bytes; no IO, no fallback literals, no policy adoption
/// and no claim of installation. Error: `provider-wait-defaults-invalid`.
pub fn decode_provider_wait_defaults(raw: &[u8]) -> Result<ProviderWaitValues, E> {
    let v: ProviderWaitValues = crate::work::decode_mode(raw, values_shape(), true)
        .map_err(|e| E::caused("provider-wait-defaults-invalid", e))?;
    validate_stored_values(&v)?;
    Ok(v)
}

/// Strictly decode one transport change request (recorded Go change fields
/// plus the native `preview_token` extension) and check intrinsic format:
/// schema 1, contract `event_id`, nonnegative expected revision, nonempty safe
/// reason and technical duration bounds. The lease choice is NOT checked here:
/// NEW values are governed by [`crate::RootStore::preview_provider_wait`] under the caller's
/// current policy. The token is carried unpersisted; no authority is granted.
/// Error: `api-provider-wait-change-invalid` (caused/refused).
pub fn decode_api_provider_wait_change(raw: &[u8]) -> Result<ApiProviderWaitChange, E> {
    let r: ApiProviderWaitChange = crate::work::decode_mode(raw, request_shape(), true)
        .map_err(|e| E::caused("api-provider-wait-change-invalid", e))?;
    if r.schema_version != PROVIDER_WAIT_SCHEMA
        || !swarm_identity_contract::name(r.event_id.as_bytes())
        || r.expected_revision < 0
        || crate::text::trim_space(&r.reason).is_empty()
        || !durations_within_v1(&r.values)
    {
        return Err(E::Refused("api-provider-wait-change-invalid"));
    }
    Ok(r)
}

/// Validate one NEW requested value choice before it is applied: technical
/// duration bounds for silence/max, and the lease exclusively via the passed
/// current [`PlanningActionPolicy::check_lease`] (which itself enforces a
/// positive minimum). No default lease is selected and no saved deadline is
/// touched. Errors: `provider-wait-request-values-invalid`, or
/// `provider-wait-lease-outside-policy` carrying the settings cause.
pub fn validate_requested_values(
    values: &ProviderWaitValues,
    policy: &PlanningActionPolicy,
) -> Result<(), E> {
    if !durations_within_v1(values) {
        return Err(E::Refused("provider-wait-request-values-invalid"));
    }
    policy
        .check_lease(values.planning_lease_seconds)
        .map_err(|e| E::caused("provider-wait-lease-outside-policy", e))
}
