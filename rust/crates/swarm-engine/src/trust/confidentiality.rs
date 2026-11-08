//! Revisioned BLP policy administration and data-only diagnostics. This tranche
//! does not classify project contents, launch a provider or activate confinement.
//! Current Trust authentication remains mandatory even for historical replays.
use super::{authentication::{self, Source}, state::State, TrustError as E};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use swarm_confidentiality::{Classification, ExecutionDomain, Policy};
use time::OffsetDateTime;

/// Persisted append-only policy history. Each entry binds the Trust revision at
/// its admission; a subsequent identity/configuration change makes it stale.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Record {
    pub schema_version: u32,
    pub history: Vec<Entry>,
}

/// One attributed policy revision, with no credentials or source contents.
/// `attributions` (added after first persistence) holds object attributions
/// produced by the supported `blp-object-attribute` control operation against
/// this policy revision; an absent field decodes as none, so retained
/// histories replay unchanged.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Entry {
    revision: u64,
    trust_revision: i64,
    event_id: String,
    actor: String,
    at: String,
    reason: String,
    policy_digest: String,
    policy: Policy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    attributions: Vec<Attribution>,
}

/// One control-attributed object classification bound to the exact canonical
/// action bytes and the project root holding them, valid under the policy
/// revision that produced it. Produced only by an authorized I3/admin
/// operation; consumed by re-reading this persisted record, never by any
/// request-supplied label or clearance inference.
///
/// Exact-once identity: an attribution is admitted only when no other
/// attribution of the SAME entry already binds the same
/// (`canonical_digest`, `project_path`, `policy_revision`) triple. The first
/// control attribution of an object under a policy revision is therefore the
/// only one: a later event that would store a contradicting label is refused
/// (see `blp-object-attribute`), and any label change is an explicit policy
/// decision recorded as a new revision, never an in-place overwrite or an
/// implicit declassification. Across revisions the new label must dominate
/// every previously recorded label for the same digest/project. An ordinary
/// policy change never supplies BLP-06 human declassification. No per-id
/// exception exists.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Attribution {
    canonical_digest: String,
    project_path: Vec<u8>,
    policy_revision: u64,
    level: String,
    compartments: Vec<String>,
    actor: String,
    at: String,
    reason: String,
}

/// All policy fields are explicit. No enabled flag can claim runtime protection.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Change {
    schema_version: u32,
    event_id: String,
    expected_revision: u64,
    expected_trust_revision: i64,
    reason: String,
    policy: Policy,
}

/// Diagnostic supplied labels are untrusted examples, never object attribution.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Check {
    schema_version: u32,
    expected_revision: u64,
    sources: Vec<Option<Classification>>,
    destination: Option<Classification>,
}

/// Control input for `blp-object-attribute`: one object attribution produced
/// by an authorized I3/admin operator, bound to the exact canonical action
/// digest and project root. The classification is explicit, validated under
/// the current policy, and never inferred from clearance, labels or history.
/// `project_path` is a readable absolute UTF-8 filesystem path string (the
/// persisted attribution keeps the exact bytes, so a non-UTF-8 root can never
/// be attributed through this contract); validation refuses empty, relative
/// or NUL-containing paths.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AttributeChange {
    schema_version: u32,
    event_id: String,
    expected_revision: u64,
    expected_trust_revision: i64,
    canonical_digest: String,
    project_path: String,
    classification: Option<Classification>,
    reason: String,
}

/// Explicit null cannot erase or deactivate a stored policy history.
pub(crate) fn present<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Record>, D::Error> {
    Record::deserialize(d).map(Some)
}

impl Record {
    /// Validate exact monotonic history, schema, catalog compatibility, policy
    /// hashes and every stored attribution. No identity or rollback resistance
    /// is inferred from stored bytes.
    ///
    /// Attribution contract (storage/reopen): each attribution must bind the
    /// entry it rides on (`policy_revision` == the entry's own revision), the
    /// exact canonical-action digest shape (lowercase 64-hex, validated as
    /// data form; the binding to real bytes stays the caller's prepared
    /// canonicalization re-checked at consumption), a readable absolute UTF-8
    /// project path, a named actor, an RFC3339 timestamp, a non-empty reason
    /// and a classification the entry's own policy accepts (a real catalog
    /// level and known, duplicate-free compartments). Attributions are
    /// exact-once per entry: two attributions sharing a (canonical digest,
    /// project path) key within one entry invalidate the history, mirroring
    /// the producer's `blp-attribution-conflict` admission refusal — a stored
    /// contradicting pair would let `attribution_for` serve whichever copy
    /// decodes first. An attribution with an
    /// empty or unknown level can never be read back coherently by the reader
    /// guard (`check_read`), so it invalidates the history at decode time
    /// instead of producing an incoherent success at consumption. Errors:
    /// malformed attribution fields give `blp-history-invalid`; a
    /// cross-revision decrease gives `blp-attribution-history-downgrade`
    /// (both Integrity); invalid policy gives `blp-policy-invalid`. These
    /// failures refuse the whole
    /// state at every open, replay and read — never a partial tolerance or
    /// a default level. Limits: this validates the stored record's internal
    /// coherence, including monotone classifications across policy revisions;
    /// it does not re-run the producer's operator validation at
    /// consumption time and claims no rollback protection.
    pub fn validate(&self) -> Result<(), E> {
        if self.schema_version != 1 || self.history.is_empty() {
            return Err(E::Integrity("blp-history-invalid"));
        }
        let mut ids=std::collections::BTreeSet::new();
        for (index, entry) in self.history.iter().enumerate() {
            if entry.revision != u64::try_from(index).ok().and_then(|i|i.checked_add(1)).ok_or(E::Integrity("blp-revision-overflow"))?
                || entry.trust_revision < 1 || !swarm_identity_contract::name(entry.event_id.as_bytes())
                || !ids.insert(&entry.event_id) || !swarm_identity_contract::name(entry.actor.as_bytes())
                || entry.reason.trim().is_empty()
                || OffsetDateTime::parse(&entry.at,&time::format_description::well_known::Rfc3339).is_err()
                || super::digest(&entry.policy)? != entry.policy_digest
            { return Err(E::Integrity("blp-history-invalid")); }
            entry.policy.validate().map_err(|_|E::Integrity("blp-policy-invalid"))?;
            // Exact-once binding at storage level too: the producer refuses a
            // second attribution for the same (policy_revision, digest,
            // project) triple (`blp-attribution-conflict`) before mutating,
            // so a stored history holding two attributions for the same
            // (digest, project) inside one entry is a poisoned record — e.g.
            // a contradicting re-classification injected by side effect —
            // and must refuse at reopen rather than let `attribution_for`
            // silently serve whichever copy comes first.
            let mut bound=std::collections::BTreeSet::new();
            for attribution in &entry.attributions {
                if attribution.policy_revision!=entry.revision
                    || attribution.canonical_digest.len()!=64
                    || !attribution.canonical_digest.bytes().all(|b|b.is_ascii_hexdigit()&&b.to_ascii_lowercase()==b)
                    || attribution.project_path.is_empty()
                    || attribution.project_path.contains(&0)
                    || attribution.project_path[0]!=b'/'
                    || std::str::from_utf8(&attribution.project_path).is_err()
                    || !swarm_identity_contract::name(attribution.actor.as_bytes())
                    || OffsetDateTime::parse(&attribution.at,&time::format_description::well_known::Rfc3339).is_err()
                    || attribution.reason.trim().is_empty()
                { return Err(E::Integrity("blp-history-invalid")); }
                if !bound.insert((attribution.canonical_digest.clone(),attribution.project_path.clone()))
                { return Err(E::Integrity("blp-history-invalid")); }
                entry.policy.dominates(&Classification{level:attribution.level.clone(),
                    compartments:attribution.compartments.clone()},
                    &Classification{level:attribution.level.clone(),
                    compartments:attribution.compartments.clone()})
                    .map_err(|_|E::Integrity("blp-history-invalid"))?;
            }
            if index>0 {
                self.history[index-1].policy.validate_successor(&entry.policy)
                    .map_err(|_|E::Integrity("blp-catalog-history-invalid"))?;
                if entry.trust_revision < self.history[index-1].trust_revision { return Err(E::Integrity("blp-authority-history-invalid")); }
            }
        }
        // Catalog evolution was validated above. Every old label remains
        // interpretable in its successor; changing policy cannot erase an
        // object's prior level or compartments. Check retained histories too,
        // so a pre-fix or injected downgrade cannot authorize a reader.
        let mut previous=std::collections::BTreeMap::new();
        for entry in &self.history {
            for attribution in &entry.attributions {
                let key=(attribution.canonical_digest.as_str(),attribution.project_path.as_slice());
                if let Some(prior)=previous.insert(key,attribution) {
                    if !classification_dominates(&entry.policy,attribution,prior)? {
                        return Err(E::Integrity("blp-attribution-history-downgrade"));
                    }
                }
            }
        }
        Ok(())
    }
    /// Latest validated entry. Empty/corrupt history refuses, never defaults.
    /// Readable crate-wide: the reader guard re-reads the current entry for
    /// its own currency/attribution checks with the same validation.
    pub(crate) fn current(&self) -> Result<&Entry, E> {
        self.validate()?;
        self.history.last().ok_or(E::Integrity("blp-history-invalid"))
    }
}

/// Compare two already attributed labels under a validated successor catalog.
/// This pure consistency check preserves level and compartments, including
/// historical ones; malformed labels return an integrity error. It grants no
/// classification authority, declassification or permission to read content.
fn classification_dominates(policy:&Policy, next:&Attribution, prior:&Attribution) -> Result<bool,E> {
    policy.dominates(
        &Classification{level:next.level.clone(),compartments:next.compartments.clone()},
        &Classification{level:prior.level.clone(),compartments:prior.compartments.clone()},
    ).map_err(|_|E::Integrity("blp-history-invalid"))
}

impl Entry {
    /// Trust revision the entry was admitted at; the reader guard compares it
    /// to the current state revision to decide currency without mutation.
    pub(crate) fn trust_revision(&self) -> i64 { self.trust_revision }
    /// Current policy's revision, for attribution-binding comparisons.
    pub(crate) fn revision(&self) -> u64 { self.revision }
    /// The validated policy of this entry; no credentials or object bytes.
    pub(crate) fn policy(&self) -> &Policy { &self.policy }
    /// Find the control attribution binding the exact canonical digest and
    /// project path under THIS entry's policy revision. `None` is the explicit
    /// unknown-provenance observed state, never a fallback to another entry
    /// (a stale entry's attribution cannot authorize a current read). At most
    /// one attribution matches: the admission contract refuses a second
    /// attribution of the same triple, so this lookup never depends on
    /// iteration order.
    pub(crate) fn attribution_for(
        &self,
        canonical_digest: &str,
        project_path: &[u8],
    ) -> Option<(&str, &str, &[String])> {
        self.attributions.iter().find(|a| {
            a.policy_revision == self.revision
                && a.canonical_digest == canonical_digest
                && a.project_path.as_slice() == project_path
        }).map(|a| (a.level.as_str(), a.actor.as_str(), a.compartments.as_slice()))
    }
}

/// Summarize configured state without disclosing grants, labels or history to
/// the general operator projection. Quarantine and the Internet-socket refusal
/// are explicit scoped blocks, never a claim of transversal BLP enforcement or
/// classified-send authority.
pub(crate) fn capability(st: &State) -> Result<Value, E> {
    let current=st.confidentiality.as_ref().map(Record::current).transpose()?;
    Ok(json!({"schema_version":1,"configured":current.is_some(),
        "policy_revision":current.map(|e|e.revision.to_string()),
        "authority_current":current.is_some_and(|e|e.trust_revision==st.revision),
        "runtime_enforced":false,"declassification_available":false,
        "preparation_quarantined":current.is_some_and(|e|e.policy.preparation_quarantine),
        "provider_internet_denied":current.is_some_and(|e|e.policy.provider_internet_denied),
        "scope":"policy-administration-diagnostics-and-preparation-quarantine",
        "reason":"blp-producer-and-sink-integration-required"}))
}

/// Execute fixed BLP operations under the caller's Trust reservation. All
/// administration requires a current authenticated I3/admin; the local operator
/// shortcut is deliberately absent. Preview/check/show never mutate. Apply
/// binds the exact proposal, actor and current Trust/BLP revisions, appends one
/// attributed proof and preserves idempotence. Object attribution cannot lower
/// any previously recorded classification across revisions; that requires the
/// separate, unavailable BLP-06 contract. No input creates human identity.
pub(crate) fn execute(st: &mut State, op: &str, raw: &[u8], source: Source<'_>, now: OffsetDateTime)
    -> Result<(Vec<u8>, Option<&'static str>), E>
{
    st.require_supported()?;
    let actor=authentication::authenticated(st,source)?;
    if op=="blp-check" {
        let c:Check=swarm_wire_json::decode(raw)?;
        if c.schema_version!=1 { return Err(E::Invalid("blp-check-invalid")); }
        let entry=st.confidentiality.as_ref().ok_or(E::Conflict("blp-policy-required"))?.current()?;
        if c.expected_revision!=entry.revision || entry.trust_revision!=st.revision {
            return Err(E::Conflict("blp-policy-stale"));
        }
        let result=ExecutionDomain::prepare(&entry.policy,entry.revision,&actor.subject,&c.sources)
            .and_then(|domain| domain.check_output(&entry.policy,entry.revision,c.destination.as_ref())
                .map(|()|domain.output_classification().clone()));
        return Ok((super::encoded(&match result {
            Ok(label)=>json!({"schema_version":1,"diagnostic_only":true,"runtime_authorized":false,"allowed_by_supplied_labels":true,"output_classification":label,"policy_revision":entry.revision.to_string()}),
            Err(e)=>json!({"schema_version":1,"diagnostic_only":true,"runtime_authorized":false,"allowed_by_supplied_labels":false,"reason":e.code(),"policy_revision":entry.revision.to_string()}),
        })?,None));
    }
    authentication::require_administrator(&actor)?;
    if op=="blp-show" {
        let value:std::collections::BTreeMap<String,Value>=swarm_wire_json::decode(raw)?;
        if !value.is_empty() { return Err(E::Invalid("blp-show-body-invalid")); }
        return Ok((super::encoded(&json!({"trust_revision":st.revision.to_string(),"capability":capability(st)?,"record":st.confidentiality}))?,None));
    }
    if op=="blp-object-attribute" {
        let c:AttributeChange=swarm_wire_json::decode(raw)?;
        if c.schema_version!=1 || !swarm_identity_contract::name(c.event_id.as_bytes())
            || c.reason.trim().is_empty() || c.reason.chars().any(char::is_control)
        { return Err(E::Invalid("blp-attribute-change-invalid")); }
        // Exact canonical-action digest shape (lowercase 64-hex), validated as
        // data form only; the binding to real bytes is the caller's prepared
        // canonicalization, re-checked at consumption by the reader guard.
        if c.canonical_digest.len()!=64 || !c.canonical_digest.bytes().all(|b|b.is_ascii_hexdigit()&&b.to_ascii_lowercase()==b)
        { return Err(E::Invalid("blp-attribute-digest-invalid")); }
        if c.project_path.is_empty() || c.project_path.contains('\0') || !c.project_path.starts_with('/')
        { return Err(E::Invalid("blp-attribute-project-invalid")); }
        // Policy must be configured, current and at the exact declared BLP
        // revision: attribution is only meaningful under a current authority.
        let entry=st.confidentiality.as_ref().ok_or(E::Conflict("blp-policy-required"))?.current()?;
        if c.expected_trust_revision!=st.revision { return Err(E::Conflict("blp-trust-revision-conflict")); }
        if c.expected_revision!=entry.revision || entry.trust_revision!=st.revision {
            return Err(E::Conflict("blp-policy-stale"));
        }
        // The attributed classification is explicit and validated under the
        // current policy by an actor whose clearance dominates it. An absent
        // or null classification refuses typed: the reader contract evaluates
        // an attribution with `check_read(Some(label))` under the recorded
        // policy, so an admitted "unclassified" attribution would store an
        // empty level that can never be read back coherently (always
        // `blp-classification-invalid`). Declaring an object unclassified is
        // an explicit policy decision — record it as a policy revision —
        // never a null-field side effect of this producer.
        let Some(label)=&c.classification else {
            return Err(E::Invalid("blp-attribute-classification-required"));
        };
        entry.policy.validate().and_then(|_|entry.policy.check_read(&actor.subject,Some(label))).map_err(|e|E::Invalid(e.code()))?;
        let fingerprint=super::digest(&json!({"operation":"blp-object-attribute","actor":actor.subject,"change":c}))?;
        if let Some(prior)=st.event_digests.get(&c.event_id) {
            if prior!=&fingerprint { return Err(E::Conflict("event_conflict")); }
            let current=st.confidentiality.as_ref().ok_or(E::Integrity("blp-replay-history-missing"))?.current()?;
            // Freshness is the replayed event's OWN recorded effect: on a
            // well-formed history the fingerprint pins expected_revision, so
            // a fresh replay's own attribution is necessarily on the CURRENT
            // entry. Free-text `reason` is a human note, never an identity:
            // another object's attribution sharing it does not freshen this
            // event, and a history where the own effect is absent refuses
            // stale instead of replaying a reply with no stored effect.
            if current.event_id!=c.event_id && !current.attributions.iter().any(|a|
                a.policy_revision==c.expected_revision
                && a.canonical_digest==c.canonical_digest
                && a.project_path==c.project_path.as_bytes())
            { return Err(E::Conflict("blp-replay-stale")); }
            let reply=st.event_replies.get(&c.event_id).ok_or(E::Integrity("blp-replay-missing"))?;
            return Ok((reply.raw().as_bytes().to_vec(),Some("replayed")));
        }
        let classification=label.clone();
        let value=json!({"schema_version":1,"attributed":true,"canonical_digest":c.canonical_digest,
            "policy_revision":entry.revision.to_string(),"trust_revision":st.revision.to_string(),
            "classification":classification,"actor":actor.subject,"reason":c.reason,"runtime_enforced":false});
        let bytes=super::encoded(&value)?;
        let stamp=super::operations::stamp(now)?;
        let attribution=Attribution{canonical_digest:c.canonical_digest.clone(),
            project_path:c.project_path.clone().into_bytes(),
            policy_revision:entry.revision,level:classification.level,
            compartments:classification.compartments,
            actor:actor.subject.clone(),at:stamp.clone(),reason:c.reason};
        // Exact-once admission: one control attribution per object under a
        // policy revision. A second event binding the same
        // (digest, project, revision) triple refuses — whether it would
        // contradict or repeat the recorded label (the identical event
        // replays above through its own event id) — so no contradicting pair
        // can ever be stored and no classification is changed or erased by
        // side effect. A new revision permits only equal/stricter renewal.
        // Checked before the current entry is borrowed mutably for the push.
        if st.confidentiality.as_ref().ok_or(E::Integrity("blp-history-missing"))?
            .history.last().ok_or(E::Integrity("blp-history-missing"))?
            .attributions.iter().any(|a|a.policy_revision==entry.revision
            && a.canonical_digest==c.canonical_digest && a.project_path==c.project_path.as_bytes())
        { return Err(E::Conflict("blp-attribution-conflict")); }
        // The validated history is monotone, so its latest matching entry
        // contains all earlier restrictions. Revisions never reset this floor.
        let record=st.confidentiality.as_ref().ok_or(E::Integrity("blp-history-missing"))?;
        if let Some(prior)=record.history.iter().rev().flat_map(|e|e.attributions.iter())
            .find(|a|a.canonical_digest==attribution.canonical_digest && a.project_path==attribution.project_path) {
            if !classification_dominates(&entry.policy,&attribution,prior)? {
                return Err(E::Conflict("blp-declassification-required"));
            }
        }
        let current_entry=st.confidentiality.as_mut().ok_or(E::Integrity("blp-history-missing"))?
            .history.last_mut().ok_or(E::Integrity("blp-history-missing"))?;
        // The attribution binds to the CURRENT entry in place: it inherits the
        // entry's revision/trust_revision so currency stays exact, and the
        // policy digest is unchanged by definition (no catalog transition).
        current_entry.attributions.push(attribution);
        let command:super::command::Command=swarm_wire_json::decode(b"{}")?;
        super::operations::append(st,op,&command,&actor,"blp-attribution",&super::hash(&bytes),"recorded",
            "Object attribution recorded under the current BLP policy; runtime integration not established",raw,now,None)?;
        st.event_digests.insert(c.event_id.clone(),fingerprint);
        let reply_text=std::str::from_utf8(&bytes).map_err(|_|E::Integrity("blp-reply-encoding"))?;
        st.event_replies.insert(c.event_id,super::json::CompactJson::new(reply_text)?);
        return Ok((bytes,Some("recorded")));
    }
    if !matches!(op,"blp-policy-preview"|"blp-policy-apply") { return Err(E::Invalid("blp-operation-invalid")); }
    let c:Change=swarm_wire_json::decode(raw)?;
    if c.schema_version!=1 || !swarm_identity_contract::name(c.event_id.as_bytes())
        || c.reason.trim().is_empty() || c.reason.chars().any(char::is_control)
    { return Err(E::Invalid("blp-change-invalid")); }
    c.policy.validate().map_err(|_|E::Invalid("blp-policy-invalid"))?;
    for principal in c.policy.clearances.keys() {
        if !st.identities.iter().any(|i|&i.subject==principal&&!i.disabled) { return Err(E::Invalid("blp-principal-unrecognized")); }
    }
    let fingerprint=super::digest(&json!({"operation":"blp-policy-apply","actor":actor.subject,"change":c}))?;
    // Authentication and freshness precede replay. A previous snapshot cannot
    // revive a grant after an identity/configuration or later policy change.
    if c.expected_trust_revision!=st.revision { return Err(E::Conflict("blp-trust-revision-conflict")); }
    if let Some(prior)=st.event_digests.get(&c.event_id) {
        if op=="blp-policy-preview" { return Err(E::Conflict("blp-event-already-recorded")); }
        if prior!=&fingerprint { return Err(E::Conflict("event_conflict")); }
        let current=st.confidentiality.as_ref().ok_or(E::Integrity("blp-replay-history-missing"))?.current()?;
        if current.event_id!=c.event_id || current.trust_revision!=st.revision { return Err(E::Conflict("blp-replay-stale")); }
        let reply=st.event_replies.get(&c.event_id).ok_or(E::Integrity("blp-replay-missing"))?;
        return Ok((reply.raw().as_bytes().to_vec(),Some("replayed")));
    }
    let previous=st.confidentiality.as_ref().map(Record::current).transpose()?;
    let revision=previous.map_or(0,|e|e.revision);
    if c.expected_revision!=revision { return Err(E::Conflict("blp-revision-conflict")); }
    if let Some(previous)=previous { previous.policy.validate_successor(&c.policy).map_err(|_|E::Invalid("blp-catalog-transition-invalid"))?; }
    let next=revision.checked_add(1).ok_or(E::Conflict("blp-revision-exhausted"))?;
    let policy_digest=super::digest(&c.policy)?;
    let value=json!({"schema_version":1,"current_revision":revision.to_string(),"proposed_revision":next.to_string(),
        "trust_revision":st.revision.to_string(),"policy_digest":policy_digest,"policy":c.policy,
        "applied":op=="blp-policy-apply","runtime_enforced":false});
    let bytes=super::encoded(&value)?;
    if op=="blp-policy-preview" { return Ok((bytes,None)); }
    let entry=Entry{revision:next,trust_revision:st.revision,event_id:c.event_id.clone(),actor:actor.subject.clone(),
        attributions:Vec::new(),
        at:super::operations::stamp(now)?,reason:c.reason,policy_digest,policy:c.policy};
    st.confidentiality.get_or_insert_with(||Record{schema_version:1,history:vec![]}).history.push(entry);
    let command:super::command::Command=swarm_wire_json::decode(b"{}")?;
    super::operations::append(st,op,&command,&actor,"blp-policy",&super::hash(&bytes),"recorded",
        "BLP policy recorded; runtime integration not established",raw,now,None)?;
    st.event_digests.insert(c.event_id.clone(),fingerprint);
    let reply_text=std::str::from_utf8(&bytes).map_err(|_|E::Integrity("blp-reply-encoding"))?;
    st.event_replies.insert(c.event_id,super::json::CompactJson::new(reply_text)?);
    Ok((bytes,Some("recorded")))
}
