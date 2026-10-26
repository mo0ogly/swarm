//! Offline debt resolution. Supplied snapshots are data, never a trusted registry.
use crate::{strict_json, Action, Error, Proposal, SCHEMA_VERSION};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use swarm_identity_contract::name;

/// Explicit registry snapshot; no defaults, IO, signature or history is inferred.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    schema_version: u32,
    project_id: String,
    revision: i64,
    debts: Vec<Debt>,
}

/// Stable object and exact current settlement terms. Aliases are supplier scoped.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Debt {
    debt_id: String,
    supplier_id: String,
    invoice_references: Vec<String>,
    invoice_version: String,
    amount_minor: String,
    currency: String,
    beneficiary_account_id: String,
}

/// Untrusted lookup with explicit expected versions; terms come only from data
/// in the supplied registry. This request carries no credential or permission.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema_version: u32,
    request_id: String,
    project_id: String,
    supplier_id: String,
    invoice_reference: String,
    expected_registry_revision: i64,
    expected_invoice_version: String,
}

/// Offline result binding exact raw input bytes, without authenticating them.
/// `proposal.action.invoice_id` is the stable debt object ID, preserving v1
/// canonicalization. Consumers must separately bind project, registry provenance,
/// freshness and ID continuity; this report admits no action and records no effect.
#[derive(Debug, Serialize)]
pub struct PaymentResolution {
    pub schema_version: u32,
    pub scope: &'static str,
    pub authority: &'static str,
    pub registry_authenticity: &'static str,
    pub effect: &'static str,
    pub registry_sha256: String,
    pub request_sha256: String,
    pub registry_revision: i64,
    pub proposal: Proposal,
}

impl Debt {
    /// Copy registry terms into a validated v1 proposal with a supplied transport
    /// ID. Validation checks representations only; no account or debt is queried.
    fn proposal(&self, request_id: &str) -> Result<Proposal, Error> {
        let value = Proposal {
            schema_version: SCHEMA_VERSION,
            request_id: request_id.into(),
            action: Action::Payment {
                invoice_id: self.debt_id.clone(),
                invoice_version: self.invoice_version.clone(),
                amount_minor: self.amount_minor.clone(),
                currency: self.currency.clone(),
                beneficiary_account_id: self.beneficiary_account_id.clone(),
            },
        };
        value.validate()?;
        Ok(value)
    }
}

/// Resolve exact supplier/reference to one stable debt in an explicit snapshot.
/// Both inputs use strict JSON (duplicates/unknown fields/floats rejected).
/// Validate the WHOLE snapshot, including unrelated debts, before any lookup;
/// reject duplicate debt IDs or supplier/reference pairs, stale versions, project
/// mismatch and absent references with safe typed errors. No input is normalized.
/// Empty registries are valid data but every lookup refuses. Representation bounds
/// are those of v1; there is no new operator amount/volume/time/retry policy.
/// No IO or mutation occurs. Raw hashes prove bytes only, not authenticity,
/// freshness, continuity between snapshots, semantic debt uniqueness or authority.
pub fn resolve_payment_documents(
    raw_registry: &[u8],
    raw_request: &[u8],
) -> Result<PaymentResolution, Error> {
    let registry: Registry = strict_json::decode(raw_registry)?;
    let request: Request = strict_json::decode(raw_request)?;
    if registry.schema_version != SCHEMA_VERSION || request.schema_version != SCHEMA_VERSION {
        return Err(Error::Invalid("protection-schema-version"));
    }
    if registry.revision <= 0
        || request.expected_registry_revision <= 0
        || !name(registry.project_id.as_bytes())
        || !name(request.project_id.as_bytes())
        || !name(request.request_id.as_bytes())
        || !name(request.supplier_id.as_bytes())
        || !name(request.invoice_reference.as_bytes())
        || !name(request.expected_invoice_version.as_bytes())
    {
        return Err(Error::Invalid("protection-registry-identifier"));
    }
    let mut ids = BTreeSet::new();
    let mut aliases = BTreeSet::new();
    for debt in &registry.debts {
        debt.proposal(&request.request_id)?;
        if !name(debt.supplier_id.as_bytes()) || debt.invoice_references.is_empty() {
            return Err(Error::Invalid("protection-registry-identifier"));
        }
        if !ids.insert(&debt.debt_id) {
            return Err(Error::Invalid("protection-registry-ambiguous"));
        }
        for reference in &debt.invoice_references {
            if !name(reference.as_bytes()) {
                return Err(Error::Invalid("protection-registry-identifier"));
            }
            if !aliases.insert((&debt.supplier_id, reference)) {
                return Err(Error::Invalid("protection-registry-ambiguous"));
            }
        }
    }
    if registry.project_id != request.project_id {
        return Err(Error::Invalid("protection-registry-project-mismatch"));
    }
    if registry.revision != request.expected_registry_revision {
        return Err(Error::Invalid("protection-registry-stale"));
    }
    let debt = registry
        .debts
        .iter()
        .find(|debt| {
            debt.supplier_id == request.supplier_id
                && debt.invoice_references.contains(&request.invoice_reference)
        })
        .ok_or(Error::Invalid("protection-registry-reference-unknown"))?;
    if debt.invoice_version != request.expected_invoice_version {
        return Err(Error::Invalid("protection-invoice-stale"));
    }
    Ok(PaymentResolution {
        schema_version: SCHEMA_VERSION,
        scope: "offline-data-only",
        authority: "not-established",
        registry_authenticity: "not-established",
        effect: "not-established",
        registry_sha256: format!("{:x}", Sha256::digest(raw_registry)),
        request_sha256: format!("{:x}", Sha256::digest(raw_request)),
        registry_revision: registry.revision,
        proposal: debt.proposal(&request.request_id)?,
    })
}
