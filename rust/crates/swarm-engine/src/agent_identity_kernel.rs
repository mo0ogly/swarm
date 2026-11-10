//! Pure agent-identity kernel (Go IA2.2 port: `agent_identity_contract.go` +
//! `agent_identity_authority.go`). Types, validators, the exact-permission
//! evaluator with delegation-chain checks, decision reuse and history-preserving
//! snapshot transitions.
//!
//! Go parity notes:
//! - refusal codes are kebab-case (`agent-identity-*`) following the Rust port
//!   convention; Go's codes are snake_case (`agent_identity_*`);
//! - `time.Time` zero value is modeled as `Option<OffsetDateTime>` where Go
//!   distinguishes zero (unset) from a real instant (grant bounds, caller
//!   expiry, revocation, decision `expires_at`);
//! - snapshot/request/decision digests hash the canonical Go JSON encoding
//!   (`swarm_wire_json::encode`), proven byte-equal in the memory usage
//!   binding parity test;
//! - the frame (`Authority`, `Caller`) is data the caller must obtain from a
//!   protected source: no public JSON admission exists here either, and a
//!   successful evaluation is not a reservation, approval, attested process or
//!   observed effect.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use swarm_identity_contract::{self as identity, Permission, Principal, Scope};
use time::OffsetDateTime;

use crate::agent_launch_error::AgentLaunchError as LaunchError;
use crate::agent_launch_preflight::hash;
use crate::encode_model;

/// Go's `agentIdentityError`: one message for every kernel refusal.
fn identity_error(code: &'static str) -> LaunchError {
    LaunchError::Refused(code)
}

/// `time.Time` JSON field with Go's zero-value semantics: `null`/absent for
/// zero, RFC3339 otherwise. Grants and decisions always carry real instants;
/// revocation/expiry pointers may be unset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GoTime(pub OffsetDateTime);

impl Serialize for GoTime {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        swarm_wire_json::timestamp::serialize(&self.0, serializer)
    }
}

impl<'de> Deserialize<'de> for GoTime {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self(swarm_wire_json::timestamp::deserialize(deserializer)?))
    }
}

/// Engine-private actor: a principal plus an optional runtime instance.
/// Mirrors Go's `agentIdentityActor` (JSON `principal`, `instance_id` omitted
/// when empty so digests match Go's `omitempty`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    pub principal: Principal,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub instance_id: String,
}

impl Actor {
    pub fn valid(&self) -> bool {
        self.principal.valid() && (self.instance_id.is_empty() || identity::name(self.instance_id.as_bytes()))
    }
}

/// Go's `agentIdentityGrant`. Field order and JSON names match Go's struct so
/// `agentIdentityDigest` reproduces Go's canonical encoding byte for byte.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grant {
    pub id: String,
    #[serde(default, skip_serializing_if = "String::is_empty", rename = "parent_grant_id")]
    pub parent_id: String,
    pub issuer: Actor,
    pub holder: Actor,
    pub scope: Scope,
    #[serde(rename = "trust_configuration_revision")]
    pub trust_revision: i64,
    pub permissions: Vec<Permission>,
    pub delegable: Vec<Permission>,
    #[serde(rename = "not_before")]
    pub not_before: Option<GoTime>,
    #[serde(rename = "expires_at")]
    pub expires_at: Option<GoTime>,
    #[serde(rename = "revoked_at", skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<GoTime>,
}

/// Go's `agentIdentityInstance`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Instance {
    pub id: String,
    #[serde(rename = "logical_agent_id")]
    pub logical_agent_id: String,
    pub principal: Principal,
    pub scope: Scope,
    #[serde(rename = "grant_id")]
    pub grant_id: String,
    #[serde(rename = "binding_id")]
    pub binding_id: String,
    #[serde(rename = "revoked_at", skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<GoTime>,
}

/// Go's `agentIdentitySnapshot`. A snapshot is data, including after import or
/// restart; its digest alone grants no authority — [`Authority`] is separate.
/// `deny_unknown_fields` mirrors Go's strict admission decode (`strict()` in
/// Go rejects unknown fields); the kernel itself consumes typed frames.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    #[serde(rename = "schema_version")]
    pub schema: i64,
    pub revision: i64,
    pub grants: BTreeMap<String, Grant>,
    pub instances: BTreeMap<String, Instance>,
}

/// Go's `agentIdentityRequest`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    #[serde(rename = "schema_version")]
    pub schema: i64,
    #[serde(rename = "event_id")]
    pub event_id: String,
    pub scope: Scope,
    #[serde(rename = "grant_id")]
    pub grant_id: String,
    pub operation: String,
    pub resource: String,
    #[serde(rename = "content_digest")]
    pub content_digest: String,
}

/// Go's `agentIdentityDecision`. A complete pure verdict, never a consumption
/// receipt or permission to release bytes after its authority/source frame has
/// changed. `expires_at` models Go's non-pointer `time.Time`: an unset value
/// serializes as Go's zero instant, never as JSON null.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    #[serde(rename = "project_context_digest")]
    pub project_digest: String,
    #[serde(rename = "event_id")]
    pub event_id: String,
    pub actor: Actor,
    #[serde(rename = "request_digest")]
    pub request_digest: String,
    #[serde(rename = "snapshot_digest")]
    pub snapshot_digest: String,
    #[serde(rename = "snapshot_revision")]
    pub snapshot_revision: i64,
    #[serde(rename = "trust_revision")]
    pub trust_revision: i64,
    #[serde(rename = "expires_at")]
    pub expires_at: Option<GoTime>,
}

impl Decision {
    /// Go's zero `time.Time` for an unset decision expiry (0001-01-01T00:00:00Z).
    pub fn go_zero_time() -> GoTime {
        GoTime(
            OffsetDateTime::from_unix_timestamp(-62_135_596_800)
                .expect("Go zero time is representable"),
        )
    }
}

/// Go's `agentIdentityAuthority`. No exported/JSON fields: a request cannot
/// populate the trusted frame. The protected consumer must supply it from an
/// independently protected current authority, including its root-grant
/// allowlist and the actual adopted plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Authority {
    pub project_root: String,
    pub snapshot_digest: String,
    pub snapshot_revision: i64,
    pub trust_revision: i64,
    pub scope: Scope,
    pub roots: std::collections::BTreeSet<String>,
}

/// Go's `agentIdentityCaller`. Non-agent callers are resolved through Trust;
/// runtime callers additionally need a verified binding supplied by the
/// protected broker (not implemented here either).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Caller {
    pub project_root: String,
    pub actor: Actor,
    pub trust_revision: i64,
    pub expires_at: Option<GoTime>,
    pub binding_id: String,
    pub verified_at: GoTime,
    pub session_digest: String,
}

/// Canonical digest of any kernel value: Go JSON encoding then SHA-256, the
/// same function Go's `agentIdentityDigest` applies.
pub fn identity_digest<T: Serialize>(value: &T) -> Result<String, LaunchError> {
    let encoded = encode_model(value)
        .map_err(|e| LaunchError::caused("agent-identity-encoding", e))?;
    Ok(hash(&encoded))
}

fn digest_valid(value: &str) -> bool {
    swarm_identity_contract::digest(value.as_bytes())
}

fn text(value: &str) -> bool {
    swarm_identity_contract::text(value.as_bytes())
}

impl Request {
    pub fn valid(&self) -> bool {
        self.schema == 1
            && identity::name(self.event_id.as_bytes())
            && self.scope.valid()
            && identity::name(self.grant_id.as_bytes())
            && Permission {
                operation: self.operation.clone(),
                resource: self.resource.clone(),
            }
            .valid()
            && digest_valid(&self.content_digest)
    }
}

/// Go's `agentIdentityPermissionSet`: nil/invalid/duplicate → None.
fn permission_set(
    permissions: &[Permission],
) -> Option<std::collections::BTreeSet<(String, String)>> {
    let mut set = std::collections::BTreeSet::new();
    for permission in permissions {
        if !permission.valid() || !set.insert((permission.operation.clone(), permission.resource.clone())) {
            return None;
        }
    }
    Some(set)
}

fn subset(child: &[Permission], parent: &std::collections::BTreeSet<(String, String)>) -> bool {
    child
        .iter()
        .all(|p| parent.contains(&(p.operation.clone(), p.resource.clone())))
}

/// The exact-permission evaluator with delegation-chain checks. The caller must
/// provide a fresh, protected frame and time under the eventual commitment
/// protocol. This pure evaluator neither locks nor commits either Trust JSON or
/// mission SQLite. There is intentionally no implicit root role, unconfigured
/// fallback, external self-asserted actor or imported authority.
///
/// Errors: [`LaunchError::Refused`] with the kebab-case counterpart of Go's
/// refusal code (`agent-identity-clock-required`, `-request-invalid`,
/// `-caller-stale`, `-authority-unavailable`, `-snapshot-changed`,
/// `-holder-mismatch`, `-binding-mismatch`, `-permission-denied`,
/// `-grant-inactive`, `-delegation-invalid`, `-root-untrusted`). No mutation of
/// any input; no partial decision escapes.
pub fn evaluate_agent_identity(
    snapshot: &Snapshot,
    authority: &Authority,
    caller: &Caller,
    request: &Request,
    at: GoTime,
) -> Result<Decision, LaunchError> {
    let zero = OffsetDateTime::UNIX_EPOCH;
    let _ = zero;
    // Go's time.IsZero(): year 1, January 1, 00:00:00 UTC.
    let is_zero = |t: GoTime| t.0.unix_timestamp() == -62_135_596_800 && t.0.nanosecond() == 0;
    if is_zero(at) {
        return Err(identity_error("agent-identity-clock-required"));
    }
    if !request.valid() {
        return Err(identity_error("agent-identity-request-invalid"));
    }
    if !caller.actor.valid()
        || caller.verified_at != at
        || caller.trust_revision <= 0
        || caller.trust_revision != authority.trust_revision
        || caller.expires_at.map_or(false, |expires| !(at.0 < expires.0))
    {
        return Err(identity_error("agent-identity-caller-stale"));
    }
    if snapshot.schema != 1
        || snapshot.revision <= 0
        || snapshot.revision != authority.snapshot_revision
        || !digest_valid(&authority.snapshot_digest)
        || !authority.scope.valid()
        || request.scope != authority.scope
        || !std::path::Path::new(&authority.project_root).is_absolute()
        || crate::local_path::clean_absolute(std::path::Path::new(&authority.project_root))
            .to_string_lossy()
            != authority.project_root
        || caller.project_root != authority.project_root
    {
        return Err(identity_error("agent-identity-authority-unavailable"));
    }
    let digest = identity_digest(snapshot)?;
    if digest != authority.snapshot_digest {
        return Err(identity_error("agent-identity-snapshot-changed"));
    }
    if authority.roots.is_empty() {
        return Err(identity_error("agent-identity-authority-unavailable"));
    }
    let grant = snapshot
        .grants
        .get(&request.grant_id)
        .ok_or_else(|| identity_error("agent-identity-holder-mismatch"))?;
    if grant.holder != caller.actor {
        return Err(identity_error("agent-identity-holder-mismatch"));
    }
    if caller.actor.instance_id.is_empty() {
        if !caller.binding_id.is_empty() {
            return Err(identity_error("agent-identity-binding-mismatch"));
        }
    } else {
        let instance = snapshot
            .instances
            .get(&caller.actor.instance_id)
            .ok_or_else(|| identity_error("agent-identity-binding-mismatch"))?;
        if instance.id != caller.actor.instance_id
            || !identity::name(instance.logical_agent_id.as_bytes())
            || instance.principal != caller.actor.principal
            || instance.scope != request.scope
            || instance.grant_id != grant.id
            || !text(&instance.binding_id)
            || caller.binding_id != instance.binding_id
            || instance.revoked_at.is_some()
        {
            return Err(identity_error("agent-identity-binding-mismatch"));
        }
    }
    let permission = (request.operation.clone(), request.resource.clone());
    let permissions = permission_set(&grant.permissions);
    let valid = permissions.is_some();
    let denied = !permissions.as_ref().map_or(false, |set| set.contains(&permission));
    if !valid || denied {
        return Err(identity_error("agent-identity-permission-denied"));
    }
    let mut expires = grant
        .expires_at
        .ok_or_else(|| identity_error("agent-identity-grant-inactive"))?;
    if caller
        .expires_at
        .map_or(false, |caller_expires| caller_expires.0 < expires.0)
    {
        expires = caller.expires_at.expect("checked above");
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut id = request.grant_id.clone();
    loop {
        let current = snapshot
            .grants
            .get(&id)
            .ok_or_else(|| identity_error("agent-identity-grant-inactive"))?;
        if current.id != id
            || seen.contains(&id)
            || !identity::name(id.as_bytes())
            || !current.issuer.valid()
            || !current.holder.valid()
            || current.scope != request.scope
            || current.trust_revision != authority.trust_revision
            || current.revoked_at.is_some()
            || current.not_before.is_none()
            || current.expires_at.is_none()
            || current.not_before.unwrap().0 >= current.expires_at.unwrap().0
            || at.0 < current.not_before.unwrap().0
            || !(at.0 < current.expires_at.unwrap().0)
        {
            return Err(identity_error("agent-identity-grant-inactive"));
        }
        seen.insert(id.clone());
        let rights = permission_set(&current.permissions);
        let delegable = permission_set(&current.delegable);
        if rights.as_ref().map_or(true, |set| set.is_empty())
            || rights.is_none()
            || delegable.is_none()
            || !subset(&current.delegable, delegable.as_ref().expect("checked above"))
        {
            return Err(identity_error("agent-identity-delegation-invalid"));
        }
        if current.expires_at.expect("checked above").0 < expires.0 {
            expires = current.expires_at.expect("checked above");
        }
        if current.parent_id.is_empty() {
            if !authority.roots.contains(&id) {
                return Err(identity_error("agent-identity-root-untrusted"));
            }
            break;
        }
        let parent = snapshot
            .grants
            .get(&current.parent_id)
            .ok_or_else(|| identity_error("agent-identity-delegation-invalid"))?;
        if current.issuer != parent.holder
            || current.scope != parent.scope
            || current.not_before.unwrap().0 < parent.not_before.unwrap_or(current.not_before.unwrap()).0
            || current.expires_at.unwrap().0 > parent.expires_at.unwrap_or(current.expires_at.unwrap()).0
        {
            return Err(identity_error("agent-identity-delegation-invalid"));
        }
        let parent_delegable = permission_set(&parent.delegable);
        let parent_valid = parent_delegable.is_some();
        if !parent_valid
            || !subset(&current.permissions, parent_delegable.as_ref().expect("checked above"))
            || !subset(&current.delegable, parent_delegable.as_ref().expect("checked above"))
        {
            return Err(identity_error("agent-identity-delegation-invalid"));
        }
        // Referencing a parent's old instance is not permission to delegate on
        // its behalf after that instance was revoked or replaced.
        if !parent.holder.instance_id.is_empty() {
            let instance = snapshot
                .instances
                .get(&parent.holder.instance_id)
                .ok_or_else(|| identity_error("agent-identity-delegation-invalid"))?;
            if instance.id != parent.holder.instance_id
                || instance.principal != parent.holder.principal
                || instance.scope != parent.scope
                || instance.grant_id != parent.id
                || instance.revoked_at.is_some()
            {
                return Err(identity_error("agent-identity-delegation-invalid"));
            }
        }
        id = current.parent_id.clone();
    }
    let request_digest = identity_digest(request)?;
    Ok(Decision {
        project_digest: hash(caller.project_root.as_bytes()),
        event_id: request.event_id.clone(),
        actor: caller.actor.clone(),
        request_digest,
        snapshot_digest: digest,
        snapshot_revision: snapshot.revision,
        trust_revision: caller.trust_revision,
        expires_at: Some(GoTime(expires.0.to_offset(time::UtcOffset::UTC))),
    })
}

/// Go's `reuseAgentIdentityDecision`: authenticate and reevaluate first, then
/// compare the entire binding. It does not mark the decision consumed; the
/// transactional consumer must do so.
pub fn reuse_agent_identity_decision(
    previous: &Decision,
    snapshot: &Snapshot,
    authority: &Authority,
    caller: &Caller,
    request: &Request,
    at: GoTime,
) -> Result<Decision, LaunchError> {
    let current = evaluate_agent_identity(snapshot, authority, caller, request, at)?;
    if &current != previous {
        return Err(identity_error("agent-identity-decision-stale"));
    }
    Ok(current)
}

/// Go's `agentIdentityRevocationPreserved`.
fn revocation_preserved(previous: Option<GoTime>, next: Option<GoTime>) -> bool {
    if let Some(next) = next {
        // Go's IsZero on a non-nil pointer target.
        if next.0.unix_timestamp() == -62_135_596_800 && next.0.nanosecond() == 0 {
            return false;
        }
    }
    match previous {
        None => true,
        Some(previous) => next.map_or(false, |next| next.0 == previous.0),
    }
}

/// Go's `validateAgentIdentitySnapshotRecords`.
fn validate_snapshot_records(next: &Snapshot) -> Result<(), LaunchError> {
    if next.schema != 1 || next.revision <= 0 {
        return Err(identity_error("agent-identity-transition-invalid"));
    }
    for (id, grant) in &next.grants {
        let permissions = permission_set(&grant.permissions);
        let delegable = permission_set(&grant.delegable);
        if id != &grant.id
            || !identity::name(id.as_bytes())
            || !grant.issuer.valid()
            || !grant.holder.valid()
            || !grant.scope.valid()
            || grant.trust_revision <= 0
            || permissions.is_none()
            || permissions.as_ref().map_or(true, |set| set.is_empty())
            || delegable.is_none()
            || !subset(&grant.delegable, delegable.as_ref().expect("checked above"))
            || grant.not_before.is_none()
            || grant.not_before.unwrap().0 >= grant.expires_at.unwrap_or(grant.not_before.unwrap()).0
            || (!grant.parent_id.is_empty() && !identity::name(grant.parent_id.as_bytes()))
        {
            return Err(identity_error("agent-identity-transition-invalid"));
        }
    }
    for (id, instance) in &next.instances {
        if id != &instance.id
            || !identity::name(id.as_bytes())
            || !identity::name(instance.logical_agent_id.as_bytes())
            || !instance.principal.valid()
            || !instance.scope.valid()
            || !identity::name(instance.grant_id.as_bytes())
            || !text(&instance.binding_id)
        {
            return Err(identity_error("agent-identity-transition-invalid"));
        }
    }
    Ok(())
}

/// Go's `validateAgentIdentityTransition`. This validates a proposed update,
/// not its authority or its publication. An already published grant/instance
/// cannot be rewritten or erased to reactivate rights. A replacement requires
/// a new ID and an explicit newly authorized record. The future writer must
/// check this against the protected current state and commit its new anchor
/// atomically; comparing with an imported old state is insufficient.
pub fn validate_agent_identity_transition(
    previous: &Snapshot,
    next: &Snapshot,
) -> Result<(), LaunchError> {
    if previous.schema != 1
        || next.schema != 1
        || previous.revision <= 0
        || next.revision <= previous.revision
        || next.revision != previous.revision + 1
    {
        return Err(identity_error("agent-identity-transition-invalid"));
    }
    for (id, old) in &previous.grants {
        let Some(current) = next.grants.get(id) else {
            return Err(identity_error("agent-identity-history-changed"));
        };
        if !revocation_preserved(old.revoked_at, current.revoked_at) {
            return Err(identity_error("agent-identity-history-changed"));
        }
        let mut before = old.clone();
        before.revoked_at = None;
        let mut after = current.clone();
        after.revoked_at = None;
        if identity_digest(&before)? != identity_digest(&after)? {
            return Err(identity_error("agent-identity-history-changed"));
        }
    }
    for (id, old) in &previous.instances {
        let Some(current) = next.instances.get(id) else {
            return Err(identity_error("agent-identity-history-changed"));
        };
        if !revocation_preserved(old.revoked_at, current.revoked_at) {
            return Err(identity_error("agent-identity-history-changed"));
        }
        let mut before = old.clone();
        before.revoked_at = None;
        let mut after = current.clone();
        after.revoked_at = None;
        if before != after {
            return Err(identity_error("agent-identity-history-changed"));
        }
    }
    validate_snapshot_records(next)
}

/// The instant Go's `time.Date(2026, 10, 17, 0, 0, 0, 0, time.UTC)` produces —
/// the fixed fixture clock used by the Go kernel and usage tests.
pub(crate) fn fixture_at() -> GoTime {
    GoTime(
        OffsetDateTime::parse("2026-10-17T00:00:00Z", &time::format_description::well_known::Rfc3339)
            .expect("fixture clock is representable"),
    )
}

fn go_time(raw: &str) -> GoTime {
    GoTime(
        OffsetDateTime::parse(raw, &time::format_description::well_known::Rfc3339)
            .expect("fixture time is representable"),
    )
}

fn principal(id: &str, kind: &str, issuer: &str, subject: &str) -> Principal {
    Principal {
        id: id.to_string(),
        kind: kind.to_string(),
        issuer: issuer.to_string(),
        subject: subject.to_string(),
    }
}

fn actor(principal: Principal, instance_id: &str) -> Actor {
    Actor {
        principal,
        instance_id: instance_id.to_string(),
    }
}

fn scope(work_id: &str, plan_version: i64, plan_digest: &str) -> Scope {
    Scope {
        work_id: work_id.to_string(),
        plan_version,
        plan_digest: plan_digest.to_string(),
    }
}

fn permission(operation: &str, resource: &str) -> Permission {
    Permission {
        operation: operation.to_string(),
        resource: resource.to_string(),
    }
}

fn fixture_request() -> Request {
    Request {
        schema: 1,
        event_id: "create-1".to_string(),
        scope: scope("work-1", 3, &hash(b"adopted-plan".as_slice())),
        grant_id: "grant-child".to_string(),
        operation: "agent.create".to_string(),
        resource: "task/child".to_string(),
        content_digest: hash(b"exact launch and context".as_slice()),
    }
}

/// Fixture-only stand-in for the protected current-authority producer.
/// Production must not derive its trusted frame this way from untrusted
/// snapshot data — same limit as Go's `refreshTestAuthority`.
struct TestState {
    snapshot: Snapshot,
    authority: Authority,
    caller: Caller,
    request: Request,
    at: GoTime,
}

fn grant(
    id: &str,
    parent_id: &str,
    issuer: Actor,
    holder: Actor,
    scope_v: Scope,
    trust_revision: i64,
    permissions: Vec<Permission>,
    delegable: Vec<Permission>,
    not_before: GoTime,
    expires_at: GoTime,
) -> Grant {
    Grant {
        id: id.to_string(),
        parent_id: parent_id.to_string(),
        issuer,
        holder,
        scope: scope_v,
        trust_revision,
        permissions,
        delegable,
        not_before: Some(not_before),
        expires_at: Some(expires_at),
        revoked_at: None,
    }
}

fn change_grant(snapshot: &mut Snapshot, id: &str, change: impl FnOnce(&mut Grant)) {
    let mut g = snapshot.grants.get(id).expect("grant exists").clone();
    change(&mut g);
    snapshot.grants.insert(id.to_string(), g);
}

fn authority_fixture() -> TestState {
    let at = fixture_at();
    let request = fixture_request();
    let issuer = actor(
        principal("requester", "service", "urn:fixture:issuer", "requester"),
        "",
    );
    let mut instance_actor = issuer.clone();
    instance_actor.instance_id = "instance-child".to_string();
    let permission_v = permission(&request.operation, &request.resource);
    let root = grant(
        "grant-root",
        "",
        issuer.clone(),
        issuer.clone(),
        request.scope.clone(),
        3,
        vec![permission_v.clone()],
        vec![permission_v.clone()],
        go_time("2026-10-16T23:59:00Z"),
        go_time("2026-10-17T01:00:00Z"),
    );
    let mut child = root.clone();
    child.id = "grant-child".to_string();
    child.parent_id = "grant-root".to_string();
    child.holder = instance_actor.clone();
    child.not_before = Some(go_time("2026-10-16T23:59:59Z"));
    child.expires_at = Some(go_time("2026-10-17T01:00:00Z"));
    child.delegable = Vec::new();
    let mut grants = BTreeMap::new();
    grants.insert("grant-root".to_string(), root);
    grants.insert("grant-child".to_string(), child);
    let mut instances = BTreeMap::new();
    instances.insert(
        "instance-child".to_string(),
        Instance {
            id: "instance-child".to_string(),
            logical_agent_id: "logical-child".to_string(),
            principal: instance_actor.principal.clone(),
            scope: request.scope.clone(),
            grant_id: "grant-child".to_string(),
            binding_id: "verified-channel-1".to_string(),
            revoked_at: None,
        },
    );
    let snapshot = Snapshot {
        schema: 1,
        revision: 4,
        grants,
        instances,
    };
    let mut caller = Caller {
        project_root: "/fixture/project".to_string(),
        actor: instance_actor,
        trust_revision: 3,
        expires_at: None,
        binding_id: "verified-channel-1".to_string(),
        verified_at: at,
        session_digest: String::new(),
    };
    let mut authority = Authority {
        project_root: caller.project_root.clone(),
        snapshot_digest: String::new(),
        snapshot_revision: 0,
        trust_revision: caller.trust_revision,
        scope: request.scope.clone(),
        roots: ["grant-root".to_string()].into_iter().collect(),
    };
    authority.snapshot_digest = identity_digest(&snapshot).expect("fixture digest");
    authority.snapshot_revision = snapshot.revision;
    caller.project_root = authority.project_root.clone();
    TestState {
        snapshot,
        authority,
        caller,
        request,
        at,
    }
}

impl TestState {
    /// Fixture-only stand-in for the protected current-authority producer.
    /// Production must not derive its trusted frame this way from untrusted
    /// or persisted snapshot data.
    fn refresh_test_authority(&mut self) {
        self.authority.snapshot_digest = identity_digest(&self.snapshot).expect("fixture digest");
        self.authority.snapshot_revision = self.snapshot.revision;
    }

    fn clone_state(&self) -> TestState {
        TestState {
            snapshot: self.snapshot.clone(),
            authority: self.authority.clone(),
            caller: self.caller.clone(),
            request: self.request.clone(),
            at: self.at,
        }
    }

    fn evaluate(&self) -> Result<Decision, LaunchError> {
        evaluate_agent_identity(
            &self.snapshot,
            &self.authority,
            &self.caller,
            &self.request,
            self.at,
        )
    }

    fn reuse(&self, previous: &Decision) -> Result<Decision, LaunchError> {
        reuse_agent_identity_decision(
            previous,
            &self.snapshot,
            &self.authority,
            &self.caller,
            &self.request,
            self.at,
        )
    }
}

fn refusal_code(err: &LaunchError) -> Option<&'static str> {
    match err {
        LaunchError::Refused(code) => Some(code),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authority_permits_exact_attenuated_request() {
        let f = authority_fixture();
        let before = f.snapshot.clone();
        let decision = f.evaluate().expect("exact attenuated request permitted");
        let digest = identity_digest(&f.request).expect("request digest");
        assert_eq!(decision.request_digest, digest);
        assert_eq!(decision.actor, f.caller.actor);
        assert_eq!(decision.event_id, f.request.event_id);
        assert_eq!(decision.snapshot_revision, 4);
        assert_eq!(decision.expires_at, Some(go_time("2026-10-17T01:00:00Z")));
        assert_eq!(f.snapshot, before, "evaluation mutated registry");
        f.reuse(&decision).expect("exact replay reuse");
    }

    #[test]
    fn rejects_cached_caller() {
        let f = authority_fixture();
        let later = GoTime(f.at.0.replace_minute(f.at.0.minute() + 1).expect("later clock"));
        let result = evaluate_agent_identity(
            &f.snapshot,
            &f.authority,
            &f.caller,
            &f.request,
            later,
        );
        assert!(
            result.is_err(),
            "a previously resolved caller was reused at a later decision without fresh authentication"
        );
    }

    #[test]
    fn authority_rejects_invalid_chains_without_mutation() {
        let base = authority_fixture();
        let mut cases: Vec<(String, Box<dyn Fn(&mut TestState)>)> = Vec::new();
        cases.push(("zero-clock".into(), Box::new(|f: &mut TestState| {
            f.at = Decision::go_zero_time();
        })));
        cases.push(("missing-authority".into(), Box::new(|f: &mut TestState| {
            f.authority = Authority {
                project_root: String::new(),
                snapshot_digest: String::new(),
                snapshot_revision: 0,
                trust_revision: 0,
                scope: scope("", 0, ""),
                roots: Default::default(),
            };
        })));
        cases.push(("missing-principal".into(), Box::new(|f: &mut TestState| {
            f.caller.actor.principal = principal("", "", "", "");
        })));
        cases.push(("foreign-issuer".into(), Box::new(|f: &mut TestState| {
            f.caller.actor.principal.issuer = "https://other-issuer.invalid".into();
        })));
        cases.push(("foreign-subject".into(), Box::new(|f: &mut TestState| {
            f.caller.actor.principal.subject = "homonym".into();
        })));
        cases.push(("kind-confusion".into(), Box::new(|f: &mut TestState| {
            f.caller.actor.principal.kind = "human".into();
        })));
        cases.push(("missing-binding".into(), Box::new(|f: &mut TestState| {
            f.caller.binding_id = String::new();
        })));
        cases.push(("copied-binding".into(), Box::new(|f: &mut TestState| {
            f.caller.binding_id = "another-channel".into();
        })));
        cases.push(("copied-instance".into(), Box::new(|f: &mut TestState| {
            f.caller.actor.instance_id = "instance-other".into();
        })));
        cases.push(("missing-instance".into(), Box::new(|f: &mut TestState| {
            f.snapshot.instances.remove(f.caller.actor.instance_id.as_str());
        })));
        cases.push(("revoked-instance".into(), Box::new(|f: &mut TestState| {
            let id = f.caller.actor.instance_id.clone();
            if let Some(i) = f.snapshot.instances.get_mut(&id) {
                i.revoked_at = Some(f.at);
            }
        })));
        cases.push(("wrong-instance-grant".into(), Box::new(|f: &mut TestState| {
            let id = f.caller.actor.instance_id.clone();
            if let Some(i) = f.snapshot.instances.get_mut(&id) {
                i.grant_id = "grant-root".into();
            }
        })));
        cases.push(("expired-caller".into(), Box::new(|f: &mut TestState| {
            f.caller.expires_at = Some(f.at);
        })));
        cases.push(("stale-caller".into(), Box::new(|f: &mut TestState| {
            f.caller.trust_revision -= 1;
        })));
        cases.push(("foreign-work".into(), Box::new(|f: &mut TestState| {
            f.request.scope.work_id = "work-other".into();
        })));
        cases.push(("stale-plan".into(), Box::new(|f: &mut TestState| {
            f.authority.scope.plan_version += 1;
        })));
        cases.push(("changed-plan".into(), Box::new(|f: &mut TestState| {
            f.authority.scope.plan_digest = hash(b"new-plan".as_slice());
        })));
        cases.push(("missing-root".into(), Box::new(|f: &mut TestState| {
            f.authority.roots.remove("grant-root");
        })));
        cases.push(("untrusted-root".into(), Box::new(|f: &mut TestState| {
            f.authority.roots = ["unrelated".to_string()].into_iter().collect();
        })));
        cases.push(("missing-parent".into(), Box::new(|f: &mut TestState| {
            f.snapshot.grants.remove("grant-root");
        })));
        cases.push(("cycle".into(), Box::new(|f: &mut TestState| {
            let actor_v = f.caller.actor.clone();
            let at = f.at;
            change_grant(&mut f.snapshot, "grant-root", |g| {
                g.parent_id = "grant-child".into();
                g.issuer = actor_v.clone();
                g.not_before = Some(GoTime(at.0.replace_second(at.0.second().saturating_sub(1)).unwrap_or(at.0)));
                g.expires_at = Some(GoTime(at.0.replace_minute(at.0.minute() + 1).unwrap_or(at.0)));
            });
            let perms = f.snapshot.grants["grant-child"].permissions.clone();
            change_grant(&mut f.snapshot, "grant-child", |g| {
                g.delegable = perms;
            });
        })));
        cases.push(("self-parent".into(), Box::new(|f: &mut TestState| {
            let perms = f.snapshot.grants["grant-child"].permissions.clone();
            change_grant(&mut f.snapshot, "grant-child", |g| {
                g.parent_id = g.id.clone();
                g.issuer = g.holder.clone();
                g.delegable = perms;
            });
        })));
        cases.push(("forged-parent".into(), Box::new(|f: &mut TestState| {
            change_grant(&mut f.snapshot, "grant-child", |g| {
                g.issuer.principal.id = "bob".into();
                g.issuer.principal.subject = "bob".into();
            });
        })));
        cases.push(("future-child".into(), Box::new(|f: &mut TestState| {
            let at = f.at;
            change_grant(&mut f.snapshot, "grant-child", |g| {
                g.not_before = Some(GoTime(at.0.replace_second(at.0.second() + 1).unwrap_or(at.0)));
            });
        })));
        cases.push(("expired-child".into(), Box::new(|f: &mut TestState| {
            let at = f.at;
            change_grant(&mut f.snapshot, "grant-child", |g| {
                g.expires_at = Some(at);
            });
        })));
        cases.push(("expired-parent".into(), Box::new(|f: &mut TestState| {
            f.at = GoTime(f.at.0.replace_hour(f.at.0.hour() + 1).unwrap_or(f.at.0));
        })));
        cases.push(("revoked-child".into(), Box::new(|f: &mut TestState| {
            let at = f.at;
            change_grant(&mut f.snapshot, "grant-child", |g| {
                g.revoked_at = Some(at);
            });
        })));
        cases.push(("revoked-parent".into(), Box::new(|f: &mut TestState| {
            let at = f.at;
            change_grant(&mut f.snapshot, "grant-root", |g| {
                g.revoked_at = Some(at);
            });
        })));
        cases.push(("time-escalation".into(), Box::new(|f: &mut TestState| {
            let at = f.at;
            change_grant(&mut f.snapshot, "grant-child", |g| {
                g.expires_at = Some(GoTime(at.0.replace_hour(at.0.hour() + 2).unwrap_or(at.0)));
            });
        })));
        cases.push(("start-escalation".into(), Box::new(|f: &mut TestState| {
            // Go: f.at.Add(-time.Hour) = 2026-10-16T23:00:00Z — real hour
            // arithmetic, not component replacement (hour 0 - 1 saturates).
            change_grant(&mut f.snapshot, "grant-child", |g| {
                g.not_before = Some(GoTime(f.at.0 - time::Duration::hours(1)));
            });
        })));
        cases.push(("permission-escalation".into(), Box::new(|f: &mut TestState| {
            change_grant(&mut f.snapshot, "grant-child", |g| {
                g.permissions.push(permission("memory.transmit", "memory/private"));
            });
        })));
        cases.push(("nondelegable-parent".into(), Box::new(|f: &mut TestState| {
            change_grant(&mut f.snapshot, "grant-root", |g| {
                g.delegable = Vec::new();
            });
        })));
        cases.push(("invalid-delegable".into(), Box::new(|f: &mut TestState| {
            change_grant(&mut f.snapshot, "grant-child", |g| {
                g.delegable = vec![permission("memory.read", "other")];
            });
        })));
        cases.push(("duplicate-permission".into(), Box::new(|f: &mut TestState| {
            change_grant(&mut f.snapshot, "grant-child", |g| {
                let first = g.permissions[0].clone();
                g.permissions.push(first);
            });
        })));
        cases.push(("wildcard-permission".into(), Box::new(|f: &mut TestState| {
            change_grant(&mut f.snapshot, "grant-root", |g| {
                g.delegable.push(permission("agent.create", "*"));
            });
        })));
        cases.push(("unknown-operation".into(), Box::new(|f: &mut TestState| {
            f.request.operation = "admin.grant".into();
        })));
        cases.push(("wrong-resource".into(), Box::new(|f: &mut TestState| {
            f.request.resource = "task/other".into();
        })));
        cases.push(("parent-other-plan".into(), Box::new(|f: &mut TestState| {
            change_grant(&mut f.snapshot, "grant-root", |g| {
                g.scope.plan_version += 1;
            });
        })));
        cases.push(("old-grant-after-reactivation".into(), Box::new(|f: &mut TestState| {
            f.authority.trust_revision += 1;
            f.caller.trust_revision += 1;
        })));
        for (name, change) in cases {
            let mut f = base.clone_state();
            change(&mut f);
            f.caller.verified_at = f.at; // fixture stand-in for a fresh runtime attestation
            f.refresh_test_authority();
            let before = identity_digest(&f.snapshot).expect("snapshot digest");
            let result = f.evaluate();
            let err = result.expect_err(&format!("invalid authority accepted: {name}"));
            let code = refusal_code(&err)
                .unwrap_or_else(|| panic!("case {name}: non-refusal error {err:?}"));
            assert!(
                code.starts_with("agent-identity-"),
                "case {name}: unexpected refusal {code}"
            );
            let after = identity_digest(&f.snapshot).expect("snapshot digest");
            assert_eq!(before, after, "case {name}: denial mutated registry");
        }
    }

    #[test]
    fn decision_reauthentication_and_exact_replay() {
        let base = authority_fixture();
        let previous = base.evaluate().expect("initial decision");
        let mut cases: Vec<(String, Box<dyn Fn(&mut TestState)>)> = Vec::new();
        cases.push(("different-event".into(), Box::new(|f: &mut TestState| {
            f.request.event_id = "create-2".into();
        })));
        cases.push(("different-content".into(), Box::new(|f: &mut TestState| {
            f.request.content_digest = hash(b"changed launch".as_slice());
        })));
        cases.push(("different-caller".into(), Box::new(|f: &mut TestState| {
            f.caller.actor.principal.id = "bob".into();
        })));
        cases.push(("expired".into(), Box::new(|f: &mut TestState| {
            f.at = previous.expires_at.expect("decision expiry");
        })));
        cases.push(("revoked-then-replay".into(), Box::new(|f: &mut TestState| {
            let at = f.at;
            change_grant(&mut f.snapshot, "grant-root", |g| {
                g.revoked_at = Some(at);
            });
            f.snapshot.revision += 1;
            f.refresh_test_authority();
        })));
        cases.push(("current-snapshot-changed".into(), Box::new(|f: &mut TestState| {
            f.snapshot.revision += 1;
            f.refresh_test_authority();
        })));
        for (name, change) in cases {
            let mut f = base.clone_state();
            change(&mut f);
            f.caller.verified_at = f.at;
            assert!(
                f.reuse(&previous).is_err(),
                "old result returned without fresh exact authorization: {name}"
            );
        }
    }

    #[test]
    fn snapshot_is_data_not_imported_authority() {
        let f = authority_fixture();
        let raw = encode_model(&f.snapshot).expect("snapshot encoding");
        let reopened: Snapshot = swarm_wire_json::decode(&raw).expect("reopened snapshot");
        // An imported snapshot with an empty authority frame stays inactive.
        let empty_authority = Authority {
            project_root: String::new(),
            snapshot_digest: String::new(),
            snapshot_revision: 0,
            trust_revision: 0,
            scope: scope("", 0, ""),
            roots: Default::default(),
        };
        assert!(
            evaluate_agent_identity(&reopened, &empty_authority, &f.caller, &f.request, f.at).is_err(),
            "imported snapshot became active authority"
        );
        evaluate_agent_identity(&reopened, &f.authority, &f.caller, &f.request, f.at)
            .expect("snapshot with its real authority evaluates");
        let old = reopened;
        let mut f2 = f.clone_state();
        f2.snapshot.revision += 1;
        f2.refresh_test_authority();
        assert!(
            evaluate_agent_identity(&old, &f2.authority, &f2.caller, &f2.request, f2.at).is_err(),
            "old authentic snapshot restored active rights"
        );
        let full_raw = String::from_utf8(raw).expect("utf8 snapshot");
        for body in [
            "null".to_string(),
            "{}".to_string(),
            format!("{full_raw} {{}}"),
            r#"{"schema_version":1,"revision":1,"grants":{},"instances":{},"roots":{"grant-root":true}}"#.to_string(),
            r#"{"schema_version":1,"revision":1,"revision":2,"grants":{},"instances":{}}"#.to_string(),
        ] {
            assert!(
                swarm_wire_json::decode::<Snapshot>(body.as_bytes()).is_err(),
                "invalid/self-authorizing snapshot accepted: {body}"
            );
        }
    }

    #[test]
    fn snapshot_tampering_cannot_refresh_its_own_authority() {
        let base = authority_fixture();
        let mut cases: Vec<(String, Box<dyn Fn(&mut TestState)>)> = Vec::new();
        cases.push(("alter-grant".into(), Box::new(|f: &mut TestState| {
            change_grant(&mut f.snapshot, "grant-child", |g| {
                g.expires_at = Some(GoTime(g.expires_at.expect("expiry").0.replace_hour(
                    g.expires_at.unwrap().0.hour() + 1,
                ).unwrap_or(g.expires_at.unwrap().0)));
            });
        })));
        cases.push(("remove-revocation".into(), Box::new(|f: &mut TestState| {
            f.snapshot.revision -= 1;
        })));
        cases.push(("alter-instance".into(), Box::new(|f: &mut TestState| {
            let id = f.caller.actor.instance_id.clone();
            if let Some(i) = f.snapshot.instances.get_mut(&id) {
                i.binding_id = "other-channel".into();
            }
        })));
        for (name, change) in cases {
            let mut f = base.clone_state();
            change(&mut f);
            assert!(
                f.evaluate().is_err(),
                "snapshot accepted without the independent current frame: {name}"
            );
        }
    }

    #[test]
    fn multilevel_delegation_revocation() {
        let mut f = authority_fixture();
        let perms = f.snapshot.grants["grant-child"].permissions.clone();
        change_grant(&mut f.snapshot, "grant-child", |g| {
            g.delegable = perms;
        });
        let parent = f.snapshot.grants["grant-child"].clone();
        let mut grandchild = parent.clone();
        grandchild.id = "grant-grandchild".to_string();
        grandchild.parent_id = parent.id.clone();
        grandchild.issuer = parent.holder.clone();
        let mut holder = grandchild.holder.clone();
        holder.instance_id = "instance-grandchild".to_string();
        grandchild.holder = holder.clone();
        grandchild.delegable = Vec::new();
        f.snapshot.grants.insert(grandchild.id.clone(), grandchild.clone());
        f.snapshot.instances.insert(
            holder.instance_id.clone(),
            Instance {
                id: holder.instance_id.clone(),
                logical_agent_id: "logical-grandchild".to_string(),
                principal: holder.principal.clone(),
                scope: grandchild.scope.clone(),
                grant_id: grandchild.id.clone(),
                binding_id: "verified-channel-2".to_string(),
                revoked_at: None,
            },
        );
        f.caller.actor = holder;
        f.caller.binding_id = "verified-channel-2".to_string();
        f.request.grant_id = grandchild.id.clone();
        f.refresh_test_authority();
        f.evaluate().expect("three-level delegation evaluates");
        for ancestor in ["grant-root", "grant-child"] {
            let mut copy = f.clone_state();
            let at = copy.at;
            change_grant(&mut copy.snapshot, ancestor, |g| {
                g.revoked_at = Some(at);
            });
            copy.snapshot.revision += 1;
            copy.refresh_test_authority();
            validate_agent_identity_transition(&f.snapshot, &copy.snapshot)
                .expect("revocation transition is legal");
            assert!(
                evaluate_agent_identity(
                    &copy.snapshot,
                    &copy.authority,
                    &copy.caller,
                    &copy.request,
                    copy.at
                )
                .is_err(),
                "descendant retained ancestor's revoked authority: {ancestor}"
            );
        }
        let mut copy = f.clone_state();
        let instance_id = parent.holder.instance_id.clone();
        let at = copy.at;
        if let Some(i) = copy.snapshot.instances.get_mut(&instance_id) {
            i.revoked_at = Some(at);
        }
        copy.refresh_test_authority();
        assert!(
            evaluate_agent_identity(
                &copy.snapshot,
                &copy.authority,
                &copy.caller,
                &copy.request,
                copy.at
            )
            .is_err(),
            "descendant used revoked parent instance"
        );
    }

    #[test]
    fn transition_preserves_history_and_revocation() {
        let f = authority_fixture();
        let mut next = f.clone_state();
        next.snapshot.revision += 1;
        let at = next.at;
        change_grant(&mut next.snapshot, "grant-root", |g| {
            g.revoked_at = Some(at);
        });
        validate_agent_identity_transition(&f.snapshot, &next.snapshot)
            .expect("legal revocation transition");
        let mut cases: Vec<(String, Box<dyn Fn(&mut TestState)>)> = Vec::new();
        cases.push(("remove-grant".into(), Box::new(|f: &mut TestState| {
            f.snapshot.grants.remove("grant-child");
        })));
        cases.push(("rewrite-holder".into(), Box::new(|f: &mut TestState| {
            change_grant(&mut f.snapshot, "grant-child", |g| {
                g.holder.instance_id = "replacement".into();
            });
        })));
        cases.push(("rewrite-plan".into(), Box::new(|f: &mut TestState| {
            change_grant(&mut f.snapshot, "grant-child", |g| {
                g.scope.plan_version += 1;
            });
        })));
        cases.push(("extend-expiry".into(), Box::new(|f: &mut TestState| {
            change_grant(&mut f.snapshot, "grant-child", |g| {
                g.expires_at = Some(GoTime(g.expires_at.unwrap().0.replace_second(
                    g.expires_at.unwrap().0.second() + 1,
                ).unwrap_or(g.expires_at.unwrap().0)));
            });
        })));
        cases.push(("clear-revocation".into(), Box::new(|f: &mut TestState| {
            change_grant(&mut f.snapshot, "grant-root", |g| {
                g.revoked_at = None;
            });
        })));
        cases.push(("move-revocation".into(), Box::new(|f: &mut TestState| {
            let moved = GoTime(f.at.0.replace_second(f.at.0.second() + 1).unwrap_or(f.at.0));
            change_grant(&mut f.snapshot, "grant-root", |g| {
                g.revoked_at = Some(moved);
            });
        })));
        cases.push(("remove-instance".into(), Box::new(|f: &mut TestState| {
            f.snapshot.instances.remove("instance-child");
        })));
        cases.push(("reuse-binding".into(), Box::new(|f: &mut TestState| {
            let id = "instance-child".to_string();
            if let Some(i) = f.snapshot.instances.get_mut(&id) {
                i.binding_id = "replacement-binding".into();
            }
        })));
        cases.push(("reuse-logical-agent".into(), Box::new(|f: &mut TestState| {
            let id = "instance-child".to_string();
            if let Some(i) = f.snapshot.instances.get_mut(&id) {
                i.logical_agent_id = "other".into();
            }
        })));
        cases.push(("skip-revision".into(), Box::new(|f: &mut TestState| {
            f.snapshot.revision += 1;
        })));
        cases.push(("rollback-revision".into(), Box::new(|f: &mut TestState| {
            f.snapshot.revision -= 2;
        })));
        for (name, change) in cases {
            let mut candidate = next.clone_state();
            candidate.snapshot.revision += 1;
            change(&mut candidate);
            assert!(
                validate_agent_identity_transition(&next.snapshot, &candidate.snapshot).is_err(),
                "history/revocation rewritten: {name}"
            );
        }
        let mut updated = next.clone_state();
        updated.snapshot.revision += 1;
        let mut replacement = updated.snapshot.grants["grant-child"].clone();
        replacement.id = "grant-replacement".to_string();
        updated.snapshot.grants.insert(replacement.id.clone(), replacement);
        validate_agent_identity_transition(&next.snapshot, &updated.snapshot)
            .expect("new-ID replacement is legal");
        assert!(
            next.snapshot.grants["grant-root"].revoked_at.is_some()
                && updated.snapshot.grants["grant-root"].revoked_at.is_some(),
            "replacement erased revoked ancestor"
        );
    }

    #[test]
    fn principal_keeps_opaque_issuer_and_subject() {
        let p = principal("service-1", "service", "https://[::1]/issuer", "service[eu]");
        assert!(p.valid(), "valid opaque issuer/subject confused with wildcard syntax");
        assert!(
            !permission("memory.read", "memory/*").valid(),
            "wildcard permission accepted"
        );
    }
}
