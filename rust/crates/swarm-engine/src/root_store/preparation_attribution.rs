//! Authenticated control attribution of an actual captured prompt slice.
//! This post-capture producer grants no transport or admission authority.
use super::{RootStore, RootStoreError as E};
use crate::trust::{self, authentication, state::State};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Explicit control decision. Hash/path are deliberately absent: the opened
/// Root and validated captured manifest supply them, never the request.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema_version: u32,
    event_id: String,
    expected_revision: u64,
    expected_trust_revision: i64,
    preparation_id: String,
    turn_id: String,
    expected_turn_revision: i64,
    source_index: usize,
    classification: swarm_confidentiality::Classification,
    reason: String,
}

/// Under the caller's exclusive Trust reservation, authenticate I3/admin and
/// check the explicit label/fresh authority BEFORE looking up a turn. Resolve
/// exact captured bytes and delegate to the canonical attribution consumer,
/// preserving uniqueness, monotonic history, replay and publication semantics.
/// Capture selectors are encoded into its fingerprinted reason so an event
/// cannot replay against another turn/index/revision even for identical bytes.
/// The outer Trust consumer publishes state only on success; all source/SQL/
/// codec failures refuse without publication. Returned hashes/counts contain no
/// source text but remain restricted to the cleared control administrator.
/// This does not establish durable physical lineage, admission identity, fixed
/// execution domain, classified transport or protection of every result reader.
pub(super) fn execute(root: &RootStore, state: &mut State, raw: &[u8],
    source: authentication::Source<'_>, now: OffsetDateTime)
    -> Result<(Vec<u8>, Option<&'static str>), E>
{
    let failure = |e| E::Trust(Box::new(e));
    state.require_supported().map_err(failure)?;
    let actor = authentication::authenticated(state, source).map_err(failure)?;
    authentication::require_administrator(&actor).map_err(failure)?;
    let request: Request = swarm_wire_json::decode(raw).map_err(|e| failure(e.into()))?;
    if request.schema_version != 1 || !swarm_identity_contract::name(request.event_id.as_bytes())
        || !crate::valid_preparation_key(&request.preparation_id)
        || !crate::valid_preparation_key(&request.turn_id) || request.expected_turn_revision < 1
        || request.reason.trim().is_empty() || request.reason.chars().any(char::is_control)
    { return Err(E::Refused("blp-preparation-attribute-invalid")); }
    let entry = state.confidentiality.as_ref().ok_or(E::Refused("blp-policy-required"))?
        .current().map_err(failure)?;
    if request.expected_trust_revision != state.revision
        || request.expected_revision != entry.revision() || entry.trust_revision() != state.revision
    { return Err(E::Refused("blp-policy-stale")); }
    entry.policy().check_read(&actor.subject, Some(&request.classification))
        .map_err(|e| E::Refused(e.code()))?;
    let (digest, count) = super::preparation_call::attribution_source(root,
        &request.preparation_id, &request.turn_id, request.expected_turn_revision, request.source_index)?;
    let path = root.root.to_str().ok_or(E::Refused("blp-attribute-project-invalid"))?;
    // Canonical JSON carries the operation namespace and complete selector,
    // including the original reason, with no ambiguous string concatenation.
    let context = serde_json::json!({"operation":"blp-preparation-attribute","request":request});
    let reason = String::from_utf8(trust::encoded(&context).map_err(failure)?)
        .map_err(|_| E::Refused("blp-preparation-attribute-invalid"))?;
    let input = trust::encoded(&serde_json::json!({"schema_version":1,
        "event_id":request.event_id,"expected_revision":request.expected_revision,
        "expected_trust_revision":request.expected_trust_revision,
        "canonical_digest":digest,"project_path":path,
        "classification":request.classification,"reason":reason})).map_err(failure)?;
    let (bytes, publication) = trust::confidentiality::execute(state, "blp-object-attribute", &input, source, now)
        .map_err(failure)?;
    let mut reply: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| failure(e.into()))?;
    reply["producer_captured"] = true.into();
    reply["preparation_id"] = request.preparation_id.into();
    reply["turn_id"] = request.turn_id.into();
    reply["turn_revision"] = request.expected_turn_revision.to_string().into();
    reply["source_index"] = request.source_index.into();
    reply["source_count"] = count.into();
    Ok((trust::encoded(&reply).map_err(failure)?, publication))
}
