//! Explicit destination binding and last-use transport authorization for
//! admitted BLP preparation calls. Neither operation is a provider-send
//! capability, OS confinement or confidentiality inheritance for responses.
use super::preparation_admission::{current, filename, Admission};
use super::preparation_call_budget;
use super::{absent, finished, RootStore, RootStoreError as E};
use crate::trust::{
    self,
    authentication::{self, Source},
    state::State,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use swarm_confidentiality::{Classification, ExecutionDomain};

impl RootStore {
    /// Last-use worker-side transport gate for one claimed preparation turn.
    ///
    /// Runs inside the worker's already-held shared egress reservation (shared
    /// re-entry never blocks; the quarantine snapshot is re-checked, strictly
    /// no weaker). The connection name comes from the turn's recorded provider,
    /// never from a caller argument; the caller's actual base_url/model are
    /// echoed like the last-use transport decision, closing the claim→send
    /// TOCTOU. Zero admission receipts matching the turn's (preparation_id,
    /// provider, request_hash) is legacy behavior: the gate returns Ok before
    /// reading any Trust state, so an unconfigured Root proceeds unchanged.
    /// With at least one matching receipt, the full admission capsule is
    /// re-verified — principal taken from the producer-written record, never
    /// supplied; instance, revisions, policy digest, every source re-join and
    /// output label reconstruction — then settings/provider-source freshness,
    /// the destination attribution on the resolved digest for the recorded
    /// connection and the last-use no-write-down check all apply exactly like
    /// the `blp-preparation-transport` decision. Any failure refuses before
    /// the caller's send; the gate itself performs no provider IO, no
    /// reservation and no SQL mutation. This grants no sink authority, OS
    /// confinement or output-reader protection.
    pub fn preparation_worker_transport_gate(
        &self,
        id: &str,
        token: &str,
        base_url: &str,
        model: &str,
    ) -> Result<(), E> {
        self.with_preparation_egress(|| worker_transport_gate(self, id, token, base_url, model))
    }
}

/// Gate body under the caller's shared egress reservation. The turn custody
/// read runs under the material guard; the receipt scan admits only names
/// with the exact admission grammar, and a matching receipt that cannot be
/// read or decoded refuses (fail-closed), never silently authorizes.
fn worker_transport_gate(
    root: &RootStore,
    id: &str,
    token: &str,
    base_url: &str,
    model: &str,
) -> Result<(), E> {
    let custody = super::material_path::with_material_read(root, || {
        super::preparation_call::gated_turn(root, id, token)
    })?;
    let mut matched = Vec::new();
    for name in root.directory.names().map_err(E::Storage)? {
        // Only exact admission grammar names are candidates; anything else in
        // the directory is not an admission receipt and stays unread.
        let hash = std::str::from_utf8(&name)
            .ok()
            .and_then(|n| n.strip_suffix(".receipt"))
            .and_then(|n| n.strip_prefix("blp-admission-"));
        let Some(hash) = hash else { continue };
        if hash.len() != 64 || !hash.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)) {
            continue;
        }
        let name = format!("blp-admission-{hash}.receipt");
        let bytes = root
            .directory
            .read_native_receipt(&name)
            .map_err(|e| if absent(&e) { E::Refused("blp-admission-not-found") } else { E::Storage(e) })?;
        let admission: Admission = serde_json::from_slice(&bytes)
            .map_err(|e| super::preparation_read::cause("blp-admission-record-invalid", e))?;
        if admission.captured.preparation_id == custody.turn.preparation_id
            && admission.captured.provider == custody.turn.provider
            && admission.captured.request_hash == custody.turn.request_hash
        {
            matched.push(admission);
        }
    }
    if matched.is_empty() {
        // Legacy fast path: nothing classified was admitted for this exact
        // intent; proceed without reading any Trust state.
        return Ok(());
    }
    // Classified send path: mirror the last-use transport decision.
    let lock = root.directory.reserve_trust(false).map_err(E::Storage)?;
    let result = (|| {
        let state = match root
            .directory
            .read_bounded(super::trust_store::STATE, super::trust_store::MAX_STATE)
        {
            Ok(bytes) => State::decode(&bytes).map_err(failure)?,
            Err(e) if absent(&e) => State::initial(),
            Err(e) => return Err(E::Storage(e)),
        };
        state.validate().map_err(failure)?;
        state.require_supported().map_err(failure)?;
        let entry = state
            .confidentiality
            .as_ref()
            .ok_or(E::Refused("blp-policy-required"))?
            .current()
            .map_err(failure)?;
        // The principal comes from the producer-written receipt; a receipt
        // whose recorded identity does not survive full re-verification
        // refuses instead of authorizing. No token grants any principal.
        for admission in &matched {
            super::preparation_admission::verify(root, &state, &admission.principal, admission)?;
        }
        if custody.settings_digest != current_settings_digest(root)? {
            return Err(E::Refused("blp-transport-settings-changed"));
        }
        let (destination, provider_source) =
            resolve_destination(root, &custody.turn.provider, Some(base_url), Some(model))?;
        for admission in &matched {
            if admission.captured.provider_source != provider_source {
                return Err(E::Refused("blp-transport-provider-source-changed"));
            }
            if admission.captured.settings_digest != custody.settings_digest {
                return Err(E::Refused("blp-transport-settings-changed"));
            }
        }
        let digest = destination_digest(
            root,
            &destination.id,
            &destination.base_url,
            &destination.model,
            &provider_source,
        )?;
        let project_path = root
            .root
            .to_str()
            .ok_or(E::Refused("blp-attribute-project-invalid"))?;
        let Some((level, _actor, compartments)) = entry.attribution_for(&digest, project_path.as_bytes())
        else {
            return Err(E::Refused("blp-destination-not-attributed"));
        };
        let label = Classification {
            level: level.to_owned(),
            compartments: compartments.to_vec(),
        };
        for admission in &matched {
            let sources = admission.captured.provenance.classifications(
                &admission.captured.prompt,
                entry,
                root.root.as_os_str().as_encoded_bytes(),
            )?;
            // No-write-down is re-derived under the CURRENT policy and the
            // receipt's recorded principal (the worker holds no principal
            // token; possession of a claimed turn is never identity).
            let domain = ExecutionDomain::prepare(
                entry.policy(),
                entry.revision(),
                &admission.principal,
                &sources,
            )
            .map_err(|e| E::Refused(e.code()))?;
            if domain.output_classification() != &admission.classification {
                return Err(E::Refused("blp-admission-record-invalid"));
            }
            domain
                .check_output(entry.policy(), entry.revision(), Some(&label))
                .map_err(|e| E::Refused(e.code()))?;
        }
        observe_budget(root, &custody.turn.preparation_id)?;
        Ok(())
    })();
    finished(result, lock.close().map_err(E::Storage))
}


/// Stable safe wrapper; diagnostics never include prompt or credential bytes.
fn failure(e: trust::TrustError) -> E { E::Trust(Box::new(e)) }

/// Domain-separated opaque digest of the exact effective provider account and
/// endpoint an admitted call would use. The stored key is never an input; it is
/// covered transitively through the provider-source catalogue digests. Changing
/// any component yields a different digest, so a new explicit attribution is
/// required before the classified send — never a silent retarget.
fn destination_digest(
    root: &RootStore,
    connection: &str,
    base_url: &str,
    model: &str,
    provider_source: &str,
) -> Result<String, E> {
    trust::digest(&json!({"kind":"blp-provider-account","root":root.root,"connection":connection,
        "base_url":base_url,"model":model,"provider_source":provider_source})).map_err(failure)
}

/// Resolve the exact effective account/endpoint this Root currently serves for
/// `name` and check the caller's echo of the observable parts. The echo is a
/// TOCTOU defense for the producer: the attribution is granted only for the
/// account the operator actually saw, not for whatever is configured at apply
/// time. Malformed echo refusals are distinct from unknown-connection refusals.
fn resolve_destination(root: &RootStore, name: &str, expected_base_url: Option<&str>,
    expected_model: Option<&str>) -> Result<(crate::provider_connection_catalogue::ConnectionState, String), E> {
    let settings = root.read_settings()?;
    let (c, provider_source) = root.preparation_api_provider(name, &settings.values)?;
    if let Some(expected)=expected_base_url {
        if expected!=c.base_url { return Err(E::Refused("blp-destination-echo-mismatch")); }
    }
    if let Some(expected)=expected_model {
        if expected!=c.model { return Err(E::Refused("blp-destination-echo-mismatch")); }
    }
    Ok((c, provider_source))
}

/// Wire request recording one explicit destination label for the exact account
/// and endpoint of one `api-` connection under the current policy revision.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AttributeRequest {
    schema_version: u32,
    event_id: String,
    expected_revision: u64,
    expected_trust_revision: i64,
    connection: String,
    classification: Classification,
    /// Observable account/endpoint the actor saw; each optional field must
    /// match the resolved value exactly or the producer refuses.
    expected_base_url: Option<String>,
    expected_model: Option<String>,
    reason: String,
}

/// Wire request asking for a last-use go/no-go on one admitted capsule. No
/// field selects or overrides any control; the request only names the capsule
/// and the exact account/endpoint the caller intends to contact.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TransportRequest {
    schema_version: u32,
    admission_id: String,
    expected_revision: u64,
    expected_trust_revision: i64,
    connection: String,
    /// Observable account/endpoint the caller intends to contact; each
    /// optional field must match the currently resolved value exactly.
    expected_base_url: Option<String>,
    expected_model: Option<String>,
}

/// Reply metadata for a granted last-use transport decision. Fixed safe
/// fields only: no prompt, no key, no admission bytes, no execution grant.
fn transport_metadata(admission: &Admission, destination_digest: &str, destination: &crate::provider_connection_catalogue::ConnectionState,
    label: &Classification, actor: &str, source_count: usize) -> Result<Vec<u8>, E> {
    trust::encoded(&json!({"schema_version":1,"authorized":true,"state":"transport-authorized-not-sent",
        "admission_id":admission.admission_id,"principal":admission.principal,
        "preparation_id":admission.captured.preparation_id,
        "destination_digest":destination_digest,
        "destination":{"connection":destination.id,"base_url":destination.base_url,"model":destination.model},
        "destination_label":label,"destination_actor":actor,
        "policy_revision":admission.policy_revision.to_string(),
        "trust_revision":admission.trust_revision.to_string(),
        "source_count":source_count,
        "output_classification":admission.classification,
        "runtime_authorized":false,"runtime_enforced":false})).map_err(failure)
}

/// Attribute one explicit destination label for the exact current account and
/// endpoint of one `api-` connection, under the outer exclusive Trust lock
/// held by the caller (the `trust_scoped_reserved` mutating dispatch). Requires
/// a current I3/admin whose clearance dominates the declared label, the
/// capsule-free `blp-object-attribute` producer contract (uniqueness per
/// (digest, project, revision), no declassification, monotone renewal) and the
/// operator's echo of base_url/model. The reason carries request digests only,
/// never user text. Publication/replay ownership stays with that producer.
/// The stored key never enters the reason or any reply.
pub(super) fn destination_attribute(root: &RootStore, state: &mut State, raw: &[u8], source: Source<'_>, now: time::OffsetDateTime)
    -> Result<(Vec<u8>, Option<&'static str>), E>
{
    state.require_supported().map_err(failure)?;
    let actor=authentication::authenticated(state,source).map_err(failure)?;
    authentication::require_administrator(&actor).map_err(failure)?;
    let req: AttributeRequest=swarm_wire_json::decode(raw).map_err(|e|failure(e.into()))?;
    if req.schema_version!=1 || !swarm_identity_contract::name(req.event_id.as_bytes())
        || req.reason.trim().is_empty() || req.reason.chars().any(char::is_control)
        || !swarm_identity_contract::name(req.connection.as_bytes())
        || !req.connection.starts_with("api-")
    { return Err(E::Refused("blp-destination-invalid")); }
    let entry=current(state,req.expected_revision,req.expected_trust_revision)?;
    entry.policy().check_read(&actor.subject,Some(&req.classification)).map_err(|e|E::Refused(e.code()))?;
    let (connection,provider_source)=resolve_destination(root,&req.connection,req.expected_base_url.as_deref(),req.expected_model.as_deref())?;
    let digest=destination_digest(root,&connection.id,&connection.base_url,&connection.model,&provider_source)?;
    let reason=serde_json::to_string(&json!({"operation":"blp-preparation-destination-attribute",
        "request_digest":trust::digest(&req).map_err(failure)?,"reason":req.reason})).map_err(|e|failure(e.into()))?;
    let input=trust::encoded(&json!({"schema_version":1,"event_id":req.event_id,
        "expected_revision":req.expected_revision,"expected_trust_revision":req.expected_trust_revision,
        "canonical_digest":digest,"project_path":root.root.to_str().ok_or(E::Refused("blp-attribute-project-invalid"))?,
        "classification":req.classification,"reason":reason})).map_err(failure)?;
    let (bytes,publication)=trust::confidentiality::execute(state,"blp-object-attribute",&input,source,now).map_err(failure)?;
    let mut result: Value=serde_json::from_slice(&bytes).map_err(|e|failure(e.into()))?;
    result["producer_captured"]=true.into();result["capture_phase"]="destination-binding".into();
    result["destination_digest"]=digest.clone().into();
    // Observable parts only: the stored key never enters any reply.
    result["destination"]=json!({"connection":connection.id,"base_url":connection.base_url,"model":connection.model});
    Ok((trust::encoded(&result).map_err(failure)?,publication))
}

/// Decide the last-use go/no-go for one admitted capsule against the exact
/// account/endpoint the caller intends to contact. The shared egress wrapper
/// serializes this decision against configuration/BLP-policy application and
/// refuses an explicit current preparation quarantine before any observation;
/// the Trust snapshot is taken inside it. Full capsule re-verification
/// (principal, instance identity, revisions, policy digest, every source
/// re-join, output label reconstruction) precedes destination checks:
/// `provider_source` and `settings_digest` must be unchanged since admission,
/// the destination label must exist for the CURRENT policy entry on the
/// resolved digest, the reconstructed execution domain must dominate that
/// label without write-down, and the recorded preparation/work budget
/// envelopes must still admit a next reserve. No provider IO, no turn,
/// reservation or SQL mutation: an authorization reply is an observed
/// decision, never an emission. Replays return the fixed decision under the
/// same freshness rules. Late lock-close failures retain known/uncertain
/// effects via the egress wrapper's cleanup folding. This grants no sink
/// authority, OS confinement or output-reader protection.
pub(super) fn execute(root: &RootStore, _op: &str, raw: &[u8], source: Source<'_>) -> Result<trust::TrustReply, E> {
    root.with_preparation_egress(|| {
        let req: TransportRequest=swarm_wire_json::decode(raw).map_err(|e|failure(e.into()))?;
        if req.schema_version!=1 { return Err(E::Refused("blp-transport-invalid")); }
        let admission_name=filename(&req.admission_id)?;
        // The admission receipt grammar and the transport request grammar are
        // distinct; possession of an ID is never a read permission.
        if !swarm_identity_contract::name(req.connection.as_bytes())
            || !req.connection.starts_with("api-")
        { return Err(E::Refused("blp-transport-invalid")); }
        // Short shared Trust reservation inside the egress lock, matching the
        // uniform lock order egress -> trust -> settings.
        let lock=root.directory.reserve_trust(false).map_err(E::Storage)?;
        let result=(|| {
            let state=match root.directory.read_bounded(super::trust_store::STATE,super::trust_store::MAX_STATE) {
                Ok(bytes)=>State::decode(&bytes).map_err(failure)?,
                Err(e) if absent(&e)=>State::initial(),Err(e)=>return Err(E::Storage(e)),
            };state.validate().map_err(failure)?;state.require_supported().map_err(failure)?;
            let actor=authentication::authenticated(&state,source).map_err(failure)?;
            let entry=state.confidentiality.as_ref().ok_or(E::Refused("blp-policy-required"))?.current().map_err(failure)?;
            if !entry.policy().clearances.contains_key(&actor.subject) { return Err(E::Refused("blp-access-denied")); }
            current(&state,req.expected_revision,req.expected_trust_revision)?;
            let bytes=root.directory.read_native_receipt(&admission_name).map_err(|e|if absent(&e){E::Refused("blp-admission-not-found")}else{E::Storage(e)})?;
            let admission: Admission=serde_json::from_slice(&bytes).map_err(|e|super::preparation_read::cause("blp-admission-record-invalid",e))?;
            if admission.admission_id!=req.admission_id { return Err(E::Refused("blp-admission-record-invalid")); }
            // Full capsule re-verification under the current policy: owner,
            // record shape, Root instance, revisions, policy digest, every
            // source re-join and output label reconstruction.
            let source_count=super::preparation_admission::verify(root,&state,&actor.subject,&admission)?;
            // Endpoint/account resolution is the live catalogue read; the
            // caller's echo pins the observable parts it intends to contact.
            let (destination,provider_source)=resolve_destination(root,&req.connection,req.expected_base_url.as_deref(),req.expected_model.as_deref())?;
            if admission.captured.provider!=req.connection { return Err(E::Refused("blp-transport-connection-changed")); }
            if admission.captured.provider_source!=provider_source { return Err(E::Refused("blp-transport-provider-source-changed")); }
            if admission.captured.settings_digest!=current_settings_digest(root)? { return Err(E::Refused("blp-transport-settings-changed")); }
            let digest=destination_digest(root,&destination.id,&destination.base_url,&destination.model,&provider_source)?;
            // Only a label recorded on the CURRENT policy entry for THIS exact
            // digest and this Root's project path authorizes; a stale entry's
            // attribution is unknown provenance, never a fallback.
            let project_path=root.root.to_str().ok_or(E::Refused("blp-attribute-project-invalid"))?;
            let Some((level,destination_actor,compartments))=entry.attribution_for(&digest,project_path.as_bytes())
            else { return Err(E::Refused("blp-destination-not-attributed")); };
            let label=Classification{level:level.to_owned(),compartments:compartments.to_vec()};
            // Last-use no-write-down: the reconstructed fixed output domain of
            // THIS capsule must dominate the recorded destination label under
            // freshly loaded policy data. A changed policy refuses even if the
            // new clearance is broader; rebuild and reauthorize from admission.
            let sources=admission.captured.provenance.classifications(&admission.captured.prompt,entry,root.root.as_os_str().as_encoded_bytes())?;
            let domain=ExecutionDomain::prepare(entry.policy(),entry.revision(),&actor.subject,&sources).map_err(|e|E::Refused(e.code()))?;
            domain.check_output(entry.policy(),entry.revision(),Some(&label)).map_err(|e|E::Refused(e.code()))?;
            observe_budget(root,&admission.captured.preparation_id)?;
            let bytes=transport_metadata(&admission,&digest,&destination,&label,destination_actor,source_count)?;
            Ok(trust::TrustReply{bytes,publication:None})
        })();
        finished(result,lock.close().map_err(E::Storage))
    })
}

/// Read-only budget-envelope observation for the admitted preparation under
/// the shared material guard and an explicit rollback: ledger sums over
/// `reserved`/`estimated` reservations against the currently saved
/// preparation and linked-work envelopes. No turn id, witness row, write or
/// reservation is produced; a later claim still runs its own full
/// transaction-scoped checks. An unavailable preparation refuses instead of
/// authorizing on a default; read/rollback cleanup preserves the primary
/// refusal via [`finished`].
fn observe_budget(root: &RootStore, preparation_id: &str) -> Result<(), E> {
    super::material_path::with_material_read(root, || {
        let mut tx=root.db.unchecked_transaction().map_err(E::Sql)?;
        tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
        let result=(|| {
            let p=super::preparation_read::stored(&tx,&preparation_id)?;
            preparation_call_budget::observe(&tx,&p)
        })();
        super::finished(result, tx.rollback().map_err(E::Sql))
    })
}

/// Reconstruct the settings digest the admission recorded, from the CURRENT
/// saved settings history. A truncated/missing history refuses instead of
/// fabricating a digest; a different digest fails the caller's freshness check.
fn current_settings_digest(root: &RootStore) -> Result<String, E> {
    let settings=root.read_settings()?;
    settings.history.last().map(|e|e.digest.clone()).ok_or(E::Refused("preparation-settings-history-missing"))
}
