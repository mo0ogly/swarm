//! Optional operator policy for transaction-scoped role incompatibilities.
//! Principal groups are administrator assertions, never proof of human identity.
use super::{command::Command, json::Fields, state::State, TrustError as E};
use crate::trust_types::TrustIdentity;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Persisted explicit policy. Omission preserves legacy behavior; every field
/// is mandatory when present. No timeout, budget, role or person is inferred.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RoleSeparation {
    pub enabled: bool,
    pub incompatible_roles: Vec<[String; 2]>,
    #[serde(deserialize_with = "principals")]
    pub principals: BTreeMap<String, String>,
}

/// Check current participant groups before admission, approval, effect or replay.
/// Reconciliation only reads an existing effect and retains its own checks.
/// Missing mappings refuse; this pure function grants no identity or rights.
pub(crate) fn conflict(st: &State, op: &str, c: &Command, a: &TrustIdentity) -> Result<bool, E> {
    let Some(policy) = &st.role_separation else {
        return Ok(false);
    };
    if !policy.enabled {
        return Ok(false);
    }
    let action = if matches!(op,"approve"|"commit"){st.actions.get(super::operations::target(c))}else{None};
    let mandate_id = action
        .and_then(|v| v["mandate_id"].as_str())
        .unwrap_or(&c.mandate_id);
    let mandate = st.mandates.get(mandate_id);
    let mut participants = Vec::new();
    match op {
        "mandate-create" => {
            participants.push(("admin", a.subject.as_str()));
            participants.push(("requester", c.subject.as_str()));
        }
        "request" | "approve" | "commit" => {
            let Some(m) = mandate else {
                // Fresh missing targets use the ordinary attributed denial.
                // A cached reply cannot stand in for absent current context.
                if st.event_replies.contains_key(&c.event_id) { return Err(E::Authorization); }
                return Ok(false);
            };
            participants.push((
                "admin",
                m["delegator"]
                    .as_str()
                    .ok_or(E::Integrity("trust-mandate-delegator"))?,
            ));
            let requester = if op == "request" {
                a.subject.as_str()
            } else {
                let Some(action) = action else {
                    if st.event_replies.contains_key(&c.event_id) { return Err(E::Authorization); }
                    return Ok(false);
                };
                action["requester"]
                    .as_str()
                    .ok_or(E::Integrity("trust-action-requester"))?
            };
            participants.push(("requester", requester));
            if op == "approve" {
                participants.push(("approver", a.subject.as_str()));
            }
            if op == "commit" {
                if let Some(approver) = action.and_then(|v| v["approver"].as_str()) {
                    participants.push(("approver", approver));
                }
                participants.push(("effect", a.subject.as_str()));
            }
        }
        _ => return Ok(false),
    }
    policy.conflicts(&participants)
}
/// Reject repeated principal names before map collection can erase a conflict.
fn principals<'de, D: Deserializer<'de>>(d: D) -> Result<BTreeMap<String, String>, D::Error> {
    let Fields(values) = Fields::deserialize(d)?;
    values
        .into_iter()
        .map(|(k, v)| {
            serde_json::from_str(v.get())
                .map(|v| (k, v))
                .map_err(serde::de::Error::custom)
        })
        .collect()
}
/// An omitted optional block is allowed; explicit null is not a policy.
pub(crate) fn present<'de, D: Deserializer<'de>>(d: D) -> Result<Option<RoleSeparation>, D::Error> {
    RoleSeparation::deserialize(d).map(Some)
}
impl RoleSeparation {
    /// Validate the fixed consumed role vocabulary, pair uniqueness and names.
    /// Empty conflict lists explicitly impose no extra conflicts, even enabled.
    pub fn validate_shape(&self) -> Result<(), E> {
        let mut seen = BTreeSet::new();
        for pair in &self.incompatible_roles {
            if pair[0] == pair[1]
                || pair
                    .iter()
                    .any(|r| !matches!(r.as_str(), "requester" | "approver" | "effect" | "admin"))
            {
                return Err(E::Invalid("trust-role-separation-pair"));
            }
            let mut key = pair.clone();
            key.sort();
            if !seen.insert(key) {
                return Err(E::Invalid("trust-role-separation-duplicate-pair"));
            }
        }
        if self.principals.iter().any(|(s, p)| {
            !swarm_identity_contract::name(s.as_bytes())
                || !swarm_identity_contract::name(p.as_bytes())
        }) {
            return Err(E::Invalid("trust-role-separation-principal"));
        }
        Ok(())
    }
    /// Bind operator declarations to configured subjects. Enabled policy needs
    /// a complete mapping, including disabled identities; no same-name fallback.
    pub fn validate(&self, identities: &[TrustIdentity]) -> Result<(), E> {
        self.validate_shape()?;
        if self
            .principals
            .keys()
            .any(|s| !identities.iter().any(|i| i.subject == *s))
            || self.enabled
                && identities
                    .iter()
                    .any(|i| !self.principals.contains_key(&i.subject))
        {
            return Err(E::Invalid("trust-role-separation-mapping"));
        }
        Ok(())
    }
    /// Compare actual participants, not all globally assigned roles. Missing
    /// mappings refuse when enabled; this never authenticates a subject.
    pub fn conflicts(&self, participants: &[(&str, &str)]) -> Result<bool, E> {
        if !self.enabled {
            return Ok(false);
        }
        for (n, (role, subject)) in participants.iter().enumerate() {
            let group = self.principals.get(*subject).ok_or(E::Authorization)?;
            for (other, person) in &participants[..n] {
                if self.principals.get(*person).ok_or(E::Authorization)? == group
                    && self
                        .incompatible_roles
                        .iter()
                        .any(|p| p[0] == *role && p[1] == *other || p[1] == *role && p[0] == *other)
                {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}
