//! Immutable authenticated pre-transport domains. No capsule is a legacy turn,
//! worker request, budget reservation or provider-send capability.
use super::{absent, finished, RootStore, RootStoreError as E, RootInstanceIdentity};
use crate::trust::{self, authentication::{self, Source}, state::State};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use swarm_confidentiality::{Classification, ExecutionDomain};
use super::preparation_call::AdmissionCapture;

/// Explicit admission identity and unchanged legacy-compatible send intent.
/// No actor, output label, destination or execution permission may be supplied.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema_version: u32,
    event_id: String,
    expected_revision: u64,
    expected_trust_revision: i64,
    intent: Value,
}
/// Explicit control attribution for one freshly captured input slice. Unknown
/// duplicate objects retain the canonical attribution conflict, never overwrite.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Attribute {
    schema_version: u32,
    event_id: String,
    expected_revision: u64,
    expected_trust_revision: i64,
    intent: Value,
    source_index: usize,
    classification: Classification,
    reason: String,
}
/// Metadata read selects only a private admission ID, never a filesystem path.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Show {
    schema_version: u32,
    admission_id: String,
    expected_revision: u64,
    expected_trust_revision: i64,
}
/// Private producer-written record. Decoding/framing alone grants nothing;
/// every read reconstructs the domain under current authority and exact sources.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Admission {
    pub(super) schema_version: u32,
    pub(super) admission_id: String,
    pub(super) event_id: String,
    pub(super) principal: String,
    pub(super) root: RootInstanceIdentity,
    pub(super) trust_revision: i64,
    pub(super) policy_revision: u64,
    pub(super) policy_digest: String,
    pub(super) request_digest: String,
    pub(super) classification: Classification,
    pub(super) captured: AdmissionCapture,
}

/// Stable safe wrapper; diagnostics never include prompt or credential bytes.
fn failure(e: trust::TrustError) -> E { E::Trust(Box::new(e)) }
/// Require an already authenticated principal's current policy grant before any
/// context/file lookup. The returned entry remains under the caller's Trust lock.
pub(super) fn current(state: &State, revision: u64, trust_revision: i64)
    -> Result<&trust::confidentiality::Entry, E>
{
    let entry=state.confidentiality.as_ref().ok_or(E::Refused("blp-policy-required"))?.current().map_err(failure)?;
    if revision!=entry.revision() || trust_revision!=state.revision || entry.trust_revision()!=state.revision {
        return Err(E::Refused("blp-policy-stale"));
    }
    Ok(entry)
}
/// Domain-separated opaque identity scoped to actor, parent and idempotence key;
/// neither the digest nor possession of the ID is a permission or OS boundary.
fn identity(principal: &str, preparation: &str, event: &str) -> Result<String,E> {
    Ok(format!("ba-{}",trust::digest(&json!({"operation":"blp-preparation-admit",
        "principal":principal,"preparation_id":preparation,"event_id":event})).map_err(failure)?))
}
/// Technical filename grammar. Exact hash width is protocol representation,
/// not an operator policy or caller-selected path.
pub(super) fn filename(id: &str) -> Result<String,E> {
    let hash=id.strip_prefix("ba-").ok_or(E::Refused("blp-admission-invalid"))?;
    if hash.len()!=64 || !hash.bytes().all(|c|c.is_ascii_digit() || (b'a'..=b'f').contains(&c)) {
        return Err(E::Refused("blp-admission-invalid"));
    }
    Ok(format!("blp-admission-{hash}.receipt"))
}
/// Stable ordinary-staleness refusal code of `verify`'s policy-currency block
/// (recorded trust revision, policy revision or policy digest no longer
/// current). Shared with the dialogue reader's after-verify result adapter in
/// `preparation_dialogue.rs` so producer and consumer never diverge.
pub(super) const POLICY_CHANGED: &str = "blp-admission-policy-changed";
/// Verify complete stored bytes, owner, Root and current policy before exposing
/// metadata. Reconstruct the exact fixed label; stale grants/policies or any
/// mismatch refuse. This checks consistency, not malicious same-UID rollback.
pub(super) fn verify(root: &RootStore, state: &State, principal: &str, admission: &Admission) -> Result<usize,E> {
    if admission.principal!=principal { return Err(E::Refused("blp-admission-not-found")); }
    if admission.schema_version!=1 || admission.captured.source_revision<1
        || !crate::valid_preparation_key(&admission.captured.preparation_id)
        || !swarm_identity_contract::name(admission.captured.provider.as_bytes())
        || !swarm_identity_contract::name(admission.event_id.as_bytes())
        || admission.admission_id!=identity(principal,&admission.captured.preparation_id,&admission.event_id)?
        || admission.request_digest.len()!=64 || !admission.request_digest.bytes().all(|c|c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    { return Err(E::Refused("blp-admission-record-invalid")); }
    if admission.root!=root.instance()? { return Err(E::Refused("preparation-root-instance-changed")); }
    let entry=state.confidentiality.as_ref().ok_or(E::Refused("blp-policy-required"))?.current().map_err(failure)?;
    if admission.trust_revision!=state.revision || entry.trust_revision()!=state.revision
        || admission.policy_revision!=entry.revision()
        || admission.policy_digest!=trust::digest(entry.policy()).map_err(failure)? {
        return Err(E::Refused(POLICY_CHANGED));
    }
    let sources=admission.captured.provenance.classifications(&admission.captured.prompt,entry,root.root.as_os_str().as_encoded_bytes())?;
    let domain=ExecutionDomain::prepare(entry.policy(),entry.revision(),principal,&sources).map_err(|e|E::Refused(e.code()))?;
    if domain.output_classification()!=&admission.classification { return Err(E::Refused("blp-admission-record-invalid")); }
    Ok(sources.len())
}
/// Whether one fully verified CURRENT admission can stand as replacement
/// authority for one ordinary-stale admission of the same opened Root and
/// unchanged captured lineage, for the dialogue reader's public renewal
/// recovery only. Inputs are two producer-written records as stored; the
/// caller must already have run `verify` on `current` and observed success,
/// and on `stale` the exact `Refused(POLICY_CHANGED)` result — this function
/// re-checks neither currency nor record validity and performs no IO, decode
/// or mutation. Returns `Ok(true)` only when: same principal; same Root
/// instance identity; same recorded output classification (strict `==`); and
/// the captured lineage is byte-identical — every `AdmissionCapture` field
/// compared directly (preparation_id, source_revision, provider,
/// provider_source, request_level, target, context_mode, request_hash,
/// settings_digest, prompt) plus the prompt provenance `Manifest` compared
/// through its serialized bytes, so provenance equality requires identical
/// parts, digests and schema. Returns `Ok(false)` for ordinary semantic
/// inequality: a different principal, Root instance, label, captured field or
/// serialized provenance content. An actual provenance serialization error is
/// NOT converted to `Ok(false)`: it is returned as the typed
/// `RootStoreError::Trust` cause through this module's existing failure
/// conversion, so the caller propagates the original safe diagnostic and
/// refuses the whole read instead of silently reporting non-coverage or
/// serving. Deliberately NOT compared:
/// `admission_id`, `event_id` and `request_digest` (a renewal legitimately
/// redesigns them), and `trust_revision`, `policy_revision`, `policy_digest`
/// (their divergence is the staleness being covered). Limitation recorded:
/// coverage requires the STRICTLY same output classification — a current
/// admission with a higher label does NOT cover a lower-labeled stale one,
/// so no general higher-label coverage is implemented and a renewal that
/// raises the label keeps the stale 403 until re-admission at the same
/// label; no mission/date/event-key exception exists. A current admission
/// from a different principal, source capture or classification never
/// covers an old stale one. Second limitation: the `Err` branch is
/// contract-preserved but not evidence-backed — no fault is test-injected,
/// because a valid producer-written `Manifest` representation cannot fail to
/// serialize.
pub(super) fn equivalent_current_covers(stale: &Admission, current: &Admission) -> Result<bool,E> {
    if stale.principal!=current.principal || stale.root!=current.root
        || stale.classification!=current.classification {
        return Ok(false);
    }
    let a=&stale.captured; let b=&current.captured;
    if a.preparation_id!=b.preparation_id || a.source_revision!=b.source_revision
        || a.provider!=b.provider || a.provider_source!=b.provider_source
        || a.request_level!=b.request_level || a.target!=b.target
        || a.context_mode!=b.context_mode || a.request_hash!=b.request_hash
        || a.settings_digest!=b.settings_digest || a.prompt!=b.prompt {
        return Ok(false);
    }
    let left=serde_json::to_vec(&a.provenance).map_err(|e|failure(e.into()))?;
    let right=serde_json::to_vec(&b.provenance).map_err(|e|failure(e.into()))?;
    Ok(left==right)
}

/// Return fixed metadata only to the current authenticated owner. No prompt,
/// worker gate, key or supplied label becomes an execution capability.
fn metadata(admission: &Admission, source_count: usize) -> Result<Vec<u8>,E> {
    trust::encoded(&json!({"schema_version":1,"admitted":true,"state":"admitted-no-transport",
        "admission_id":admission.admission_id,"principal":admission.principal,
        "preparation_id":admission.captured.preparation_id,"source_revision":admission.captured.source_revision.to_string(),
        "policy_revision":admission.policy_revision.to_string(),"trust_revision":admission.trust_revision.to_string(),
        "prompt_sha256":trust::hash(admission.captured.prompt.as_bytes()),"source_count":source_count,
        "output_classification":admission.classification,"runtime_authorized":false,"runtime_enforced":false})).map_err(failure)
}

/// Attribute a freshly captured source under the outer exclusive Trust lock.
/// The current I3/admin must dominate its explicit label before source lookup.
/// Reuses the unchanged send producer but records no turn/reservation/IO. Only
/// a digest of the request enters the canonical event reason, never user text.
/// Canonical attribution owns uniqueness, no-declassification and publication.
pub(super) fn attribute(root: &RootStore, state: &mut State, raw: &[u8], source: Source<'_>, now: time::OffsetDateTime)
    -> Result<(Vec<u8>,Option<&'static str>),E>
{
    state.require_supported().map_err(failure)?;
    let actor=authentication::authenticated(state,source).map_err(failure)?;
    authentication::require_administrator(&actor).map_err(failure)?;
    let req: Attribute=swarm_wire_json::decode(raw).map_err(|e|failure(e.into()))?;
    if req.schema_version!=1 || !swarm_identity_contract::name(req.event_id.as_bytes())
        || req.reason.trim().is_empty() || req.reason.chars().any(char::is_control) { return Err(E::Refused("blp-admission-invalid")); }
    let entry=current(state,req.expected_revision,req.expected_trust_revision)?;
    entry.policy().check_read(&actor.subject,Some(&req.classification)).map_err(|e|E::Refused(e.code()))?;
    let captured=root.capture_admission_intent(&trust::encoded(&req.intent).map_err(failure)?)?;
    let (digest,count)=captured.provenance.source_digest(&captured.prompt,req.source_index)?;
    let reason=serde_json::to_string(&json!({"operation":"blp-preparation-input-attribute",
        "request_digest":trust::digest(&req).map_err(failure)?,"source_index":req.source_index,"reason":req.reason})).map_err(|e|failure(e.into()))?;
    let input=trust::encoded(&json!({"schema_version":1,"event_id":req.event_id,
        "expected_revision":req.expected_revision,"expected_trust_revision":req.expected_trust_revision,
        "canonical_digest":digest,"project_path":root.root.to_str().ok_or(E::Refused("blp-attribute-project-invalid"))?,
        "classification":req.classification,"reason":reason})).map_err(failure)?;
    let (bytes,publication)=trust::confidentiality::execute(state,"blp-object-attribute",&input,source,now).map_err(failure)?;
    let mut result: Value=serde_json::from_slice(&bytes).map_err(|e|failure(e.into()))?;
    result["producer_captured"]=true.into();result["capture_phase"]="pre-admission".into();
    result["source_index"]=req.source_index.into();result["source_count"]=count.into();
    Ok((trust::encoded(&result).map_err(failure)?,publication))
}

/// Execute immutable admission/metadata read under a held current Trust snapshot.
/// Authenticate/grant before lookup; admission captures and joins EVERY source
/// before create-only publication. Replays never recapture or change the fixed
/// context. No SQL/business turn/budget/Trust-journal mutation or provider IO.
/// Private receipts use observed native framing, not a new operator size cap.
/// Late publication/sync/Root/lock-close failures retain known/uncertain effects.
/// This grants no sink authority, OS confinement or output-reader protection.
pub(super) fn execute(root: &RootStore, op: &str, raw: &[u8], source: Source<'_>) -> Result<trust::TrustReply,E> {
    let readonly=op=="blp-preparation-admission-show";
    if !readonly && !root.writable { return Err(E::Refused("read-only-root")); }
    let initial=root.instance()?;
    let lock=root.directory.reserve_trust(!readonly).map_err(E::Storage)?;
    let mut publication=None;
    let result=(|| {
        let state=match root.directory.read_bounded(super::trust_store::STATE,super::trust_store::MAX_STATE) {
            Ok(bytes)=>State::decode(&bytes).map_err(failure)?,
            Err(e) if absent(&e)=>State::initial(),Err(e)=>return Err(E::Storage(e)),
        };state.validate().map_err(failure)?;state.require_supported().map_err(failure)?;
        let actor=authentication::authenticated(&state,source).map_err(failure)?;
        let entry=state.confidentiality.as_ref().ok_or(E::Refused("blp-policy-required"))?.current().map_err(failure)?;
        if !entry.policy().clearances.contains_key(&actor.subject) { return Err(E::Refused("blp-access-denied")); }
        if readonly {
            let req: Show=swarm_wire_json::decode(raw).map_err(|e|failure(e.into()))?;
            if req.schema_version!=1 { return Err(E::Refused("blp-admission-invalid")); }
            current(&state,req.expected_revision,req.expected_trust_revision)?;
            let bytes=root.directory.read_native_receipt(&filename(&req.admission_id)?).map_err(|e|if absent(&e){E::Refused("blp-admission-not-found")}else{E::Storage(e)})?;
            let admission: Admission=serde_json::from_slice(&bytes).map_err(|e|super::preparation_read::cause("blp-admission-record-invalid",e))?;
            if admission.admission_id!=req.admission_id { return Err(E::Refused("blp-admission-record-invalid")); }
            return Ok(trust::TrustReply{bytes:metadata(&admission,verify(root,&state,&actor.subject,&admission)?)?,publication:None});
        }
        let req: Request=swarm_wire_json::decode(raw).map_err(|e|failure(e.into()))?;
        if req.schema_version!=1 || !swarm_identity_contract::name(req.event_id.as_bytes()) { return Err(E::Refused("blp-admission-invalid")); }
        current(&state,req.expected_revision,req.expected_trust_revision)?;
        let intent_bytes=trust::encoded(&req.intent).map_err(failure)?;
        let intent=super::preparation_call::decode_send(&intent_bytes)?;
        if !crate::valid_preparation_key(&intent.preparation_id) { return Err(E::Refused("blp-admission-invalid")); }
        let id=identity(&actor.subject,&intent.preparation_id,&req.event_id)?;
        let name=filename(&id)?;
        let fingerprint=trust::digest(&json!({"operation":op,"principal":actor.subject,"request":req})).map_err(failure)?;
        match root.directory.read_native_receipt(&name) {
            Ok(bytes)=>{
                let admission: Admission=serde_json::from_slice(&bytes).map_err(|e|super::preparation_read::cause("blp-admission-record-invalid",e))?;
                if admission.admission_id!=id || admission.request_digest!=fingerprint { return Err(E::Refused("blp-admission-event-conflict")); }
                let bytes=metadata(&admission,verify(root,&state,&actor.subject,&admission)?)?;
                publication=Some("admission-replayed");return Ok(trust::TrustReply{bytes,publication});
            }
            Err(e) if absent(&e)=>{}, Err(e)=>return Err(E::Storage(e)),
        }
        let captured=root.capture_admission_intent(&intent_bytes)?;
        let sources=captured.provenance.classifications(&captured.prompt,entry,root.root.as_os_str().as_encoded_bytes())?;
        let domain=ExecutionDomain::prepare(entry.policy(),entry.revision(),&actor.subject,&sources).map_err(|e|E::Refused(e.code()))?;
        let admission=Admission{schema_version:1,admission_id:id,event_id:req.event_id,principal:actor.subject.clone(),
            root:initial.clone(),trust_revision:state.revision,policy_revision:entry.revision(),
            policy_digest:trust::digest(entry.policy()).map_err(failure)?,request_digest:fingerprint,
            classification:domain.output_classification().clone(),captured};
        let bytes=metadata(&admission,sources.len())?;
        let body=trust::encoded(&admission).map_err(failure)?;
        let framed=crate::native_receipt::encode(&body).map_err(|e|super::preparation_read::cause("blp-admission-encode",e))?;
        if root.instance()?!=initial { return Err(E::Refused("preparation-root-instance-changed")); }
        if let Err(e)=root.directory.publish_new_with_fault(&name,&framed,crate::local_storage::AtomicStorageFault::None) {
            if e.publication==crate::PublicationState::RenamedDurabilityUnknown { publication=Some("admission-durability-unknown"); }
            return Err(E::Storage(e));
        }
        publication=Some("admission-recorded");
        if root.instance()?!=initial { return Err(E::Refused("preparation-root-instance-changed")); }
        Ok(trust::TrustReply{bytes,publication})
    })();
    finished(result,lock.close().map_err(E::Storage)).map_err(|primary|match publication {
        Some(publication)=>E::TrustPublished{primary:Box::new(primary),publication},None=>primary,
    })
}
