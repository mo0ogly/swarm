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

/// Explicit null cannot erase or deactivate a stored policy history.
pub(crate) fn present<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Record>, D::Error> {
    Record::deserialize(d).map(Some)
}

impl Record {
    /// Validate exact monotonic history, schema, catalog compatibility and policy
    /// hashes. No identity or rollback resistance is inferred from stored bytes.
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
            if index>0 {
                self.history[index-1].policy.validate_successor(&entry.policy)
                    .map_err(|_|E::Integrity("blp-catalog-history-invalid"))?;
                if entry.trust_revision < self.history[index-1].trust_revision { return Err(E::Integrity("blp-authority-history-invalid")); }
            }
        }
        Ok(())
    }
    /// Latest validated entry. Empty/corrupt history refuses, never defaults.
    fn current(&self) -> Result<&Entry, E> {
        self.validate()?;
        self.history.last().ok_or(E::Integrity("blp-history-invalid"))
    }
}

/// Summarize configured state without disclosing grants, labels or history to
/// the general operator projection. It explicitly does not claim enforcement.
pub(crate) fn capability(st: &State) -> Result<Value, E> {
    let current=st.confidentiality.as_ref().map(Record::current).transpose()?;
    Ok(json!({"schema_version":1,"configured":current.is_some(),
        "policy_revision":current.map(|e|e.revision.to_string()),
        "authority_current":current.is_some_and(|e|e.trust_revision==st.revision),
        "runtime_enforced":false,"declassification_available":false,
        "scope":"policy-administration-and-data-only-diagnostics",
        "reason":"blp-producer-and-sink-integration-required"}))
}

/// Execute fixed BLP operations under the caller's Trust reservation. All
/// administration requires a current authenticated I3/admin; the local operator
/// shortcut is deliberately absent. Preview/check/show never mutate. Apply
/// binds the exact proposal, actor and current Trust/BLP revisions, appends one
/// attributed proof and preserves idempotence. No input creates human identity.
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
    if actor.integrity!=3 || !actor.roles.iter().any(|r|r=="admin") { return Err(E::Authorization); }
    if op=="blp-show" {
        let value:std::collections::BTreeMap<String,Value>=swarm_wire_json::decode(raw)?;
        if !value.is_empty() { return Err(E::Invalid("blp-show-body-invalid")); }
        return Ok((super::encoded(&json!({"trust_revision":st.revision.to_string(),"capability":capability(st)?,"record":st.confidentiality}))?,None));
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
