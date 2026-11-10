//! Typed v1 representations. Validation is data-only, never authority or policy.
use crate::{Error, State, SCHEMA_VERSION};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use swarm_identity_contract::{digest, name};

/// Non-authoritative proposal. Transport request_id is deliberately excluded
/// from canonical action and business intent identity; new keys confer no rights.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    pub schema_version: u32,
    pub request_id: String,
    pub action: Action,
}

/// Supported domain representations, not a catalogue of authorized operations.
/// Amounts use a positive canonical decimal string bounded by the existing i64
/// effect representation. This technical bound is not a monetary operator limit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Action {
    #[serde(rename = "payment.settle")]
    Payment {
        invoice_id: String,
        invoice_version: String,
        amount_minor: String,
        currency: String,
        beneficiary_account_id: String,
    },
    #[serde(rename = "repository.promote")]
    Git {
        repository_id: String,
        target_ref: String,
        expected_old_oid: String,
        candidate_oid: String,
    },
}

/// Source lineage observed by a future protected producer. Ordered sources
/// preserve actual context order; confidentiality labels form an unordered set.
/// A label, version or digest supplied here does not establish provenance.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRef {
    pub source_id: String,
    pub version: String,
    pub digest: String,
    pub integrity: u8,
    pub confidentiality_labels: Vec<String>,
}

/// Shape of control-attributed context, not an authenticated context constructor.
/// Public consumers must derive these fields from their protected authorities;
/// deserializing this type proves neither identity nor current policy/revocation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdmissionContext {
    pub project_id: String,
    pub principal_id: String,
    pub instance_id: String,
    pub mandate_id: String,
    pub mandate_revision: i64,
    pub policy_revision: i64,
    pub policy_digest: String,
    pub tool_manifest_digest: String,
    pub sources: Vec<SourceRef>,
}

/// Data binding for an approval. Matching bytes does not authenticate approver,
/// verify separation of duties, establish attention or check approval expiration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalBinding {
    pub schema_version: u32,
    pub action_digest: String,
    pub policy_digest: String,
    pub approver_id: String,
}

/// Data shape for a sink observation; unsigned bytes alone are not a receipt.
/// The future service must bind/sign it and establish the observation independently.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectReceipt {
    pub schema_version: u32,
    pub action_digest: String,
    pub intent_id: String,
    pub sink_id: String,
    pub outcome: State,
    pub observation_digest: String,
}

/// Existing SHA-1/SHA-256 Git object-name representations, not revision syntax.
fn oid(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        && value.bytes().any(|b| b != b'0')
}

/// Direct branch ref grammar used by the fixture promotion domain. No revision
/// expressions, pseudorefs or symbolic targets are represented. Physical binding,
/// current OID and symbolic-ref rejection remain the effect service's obligation.
fn branch(value: &str) -> bool {
    value.starts_with("refs/heads/")
        && !value.ends_with('.')
        && !value.contains("..")
        && !value.contains("@{")
        && !value.chars().any(char::is_control)
        && !value
            .bytes()
            .any(|b| b == b' ' || b == 127 || b"~^:?*[\\".contains(&b))
        && value
            .split('/')
            .all(|s| !s.is_empty() && !s.starts_with('.') && !s.ends_with(".lock"))
}

impl Proposal {
    /// Validate version, transport identifier and domain representation without
    /// consulting policy, invoice/account existence, Git or caller authority.
    pub fn validate(&self) -> Result<(), Error> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(Error::Invalid("protection-schema-version"));
        }
        if !name(self.request_id.as_bytes()) {
            return Err(Error::Invalid("protection-request-id"));
        }
        match &self.action {
            Action::Payment {
                invoice_id,
                invoice_version,
                amount_minor,
                currency,
                beneficiary_account_id,
            } => {
                if !name(invoice_id.as_bytes())
                    || !name(invoice_version.as_bytes())
                    || !name(beneficiary_account_id.as_bytes())
                {
                    return Err(Error::Invalid("protection-payment-identifier"));
                }
                if !amount_minor
                    .as_bytes()
                    .first()
                    .is_some_and(|b| (b'1'..=b'9').contains(b))
                    || !amount_minor.bytes().all(|b| b.is_ascii_digit())
                    || amount_minor.parse::<i64>().is_err()
                {
                    return Err(Error::Invalid("protection-amount-representation"));
                }
                if currency.len() != 3 || !currency.bytes().all(|b| b.is_ascii_uppercase()) {
                    return Err(Error::Invalid("protection-currency-representation"));
                }
            }
            Action::Git {
                repository_id,
                target_ref,
                expected_old_oid,
                candidate_oid,
            } => {
                if !name(repository_id.as_bytes()) || !branch(target_ref) {
                    return Err(Error::Invalid("protection-repository-target"));
                }
                if !oid(expected_old_oid)
                    || !oid(candidate_oid)
                    || expected_old_oid.len() != candidate_oid.len()
                {
                    return Err(Error::Invalid("protection-git-oid"));
                }
                if expected_old_oid == candidate_oid {
                    return Err(Error::Invalid("protection-git-no-change"));
                }
            }
        }
        Ok(())
    }
}

impl AdmissionContext {
    /// Check representations and coherent source keys, never the truth of claims.
    /// Integrity I0..I3 and 1..100-byte identifiers reuse the existing protocol;
    /// positive signed revisions match persisted engine revision representation.
    /// Empty sources are structurally valid; policy may require actual evidence.
    pub fn validate(&self) -> Result<(), Error> {
        if [
            &self.project_id,
            &self.principal_id,
            &self.instance_id,
            &self.mandate_id,
        ]
        .iter()
        .any(|s| !name(s.as_bytes()))
        {
            return Err(Error::Invalid("protection-context-identifier"));
        }
        if self.mandate_revision <= 0 || self.policy_revision <= 0 {
            return Err(Error::Invalid("protection-context-revision"));
        }
        if !digest(self.policy_digest.as_bytes()) || !digest(self.tool_manifest_digest.as_bytes()) {
            return Err(Error::Invalid("protection-context-digest"));
        }
        let mut keys = BTreeSet::new();
        for source in &self.sources {
            if !name(source.source_id.as_bytes())
                || !name(source.version.as_bytes())
                || !digest(source.digest.as_bytes())
                || source.integrity > 3
            {
                return Err(Error::Invalid("protection-source-representation"));
            }
            if !keys.insert((&source.source_id, &source.version)) {
                return Err(Error::Invalid("protection-source-duplicate"));
            }
            let mut labels = BTreeSet::new();
            for label in &source.confidentiality_labels {
                if !name(label.as_bytes()) || !labels.insert(label) {
                    return Err(Error::Invalid("protection-source-label"));
                }
            }
        }
        Ok(())
    }
}

impl ApprovalBinding {
    /// Check binding shape only; no signature, authority or current-state check.
    pub fn validate(&self) -> Result<(), Error> {
        if self.schema_version != SCHEMA_VERSION
            || !name(self.approver_id.as_bytes())
            || !digest(self.action_digest.as_bytes())
            || !digest(self.policy_digest.as_bytes())
        {
            return Err(Error::Invalid("protection-approval-binding"));
        }
        Ok(())
    }
}

impl EffectReceipt {
    /// Validate receipt representation and allowed observations. UNKNOWN remains
    /// an outcome, never fabricated denial. No signature or actual effect is proven.
    pub fn validate(&self) -> Result<(), Error> {
        if self.schema_version != SCHEMA_VERSION
            || !name(self.sink_id.as_bytes())
            || [
                &self.action_digest,
                &self.intent_id,
                &self.observation_digest,
            ]
            .iter()
            .any(|s| !digest(s.as_bytes()))
            || !matches!(
                self.outcome,
                State::Committed
                    | State::Denied
                    | State::Unknown
                    | State::ReconciledCommitted
                    | State::ReconciledNotCommitted
            )
        {
            return Err(Error::Invalid("protection-effect-receipt"));
        }
        Ok(())
    }
}
