//! Exact correspondence of supplied approval-shaped data, never authorization.
use crate::{canonicalize, decode_approval, decode_document, DocumentIdentity, Error, SCHEMA_VERSION};
use serde::Serialize;
use sha2::{Digest, Sha256};

/// Closed v1 report binding two original byte slices to content/policy equality.
/// A mismatch is valid data with named fields; malformed inputs return an error
/// instead. No approver identity, signature, current authority, attention,
/// expiration, separation of duties or effect is established by either outcome.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ApprovalReport {
    pub schema_version: u32,
    pub scope: &'static str,
    pub document: DocumentIdentity,
    pub approval_sha256: String,
    pub binding_matches: bool,
    pub mismatched_fields: Vec<&'static str>,
    pub authority: &'static str,
    pub approval_authenticity: &'static str,
    pub approval_validity: &'static str,
    pub effect: &'static str,
}

/// Strictly decode the original action document and unsigned approval data, then
/// reuse v1 canonicalization and matches_approval. Return exact raw SHA256 hashes,
/// action/intent identity, equality and ordered differing binding field names;
/// never copy the declared approver name into the report or authenticate it.
/// JSON/representation causes propagate with safe codes and no partial report.
/// Pure: no Root, IO, clock, policy lookup, signature, approval or effect.
pub fn check_approval_documents(document_raw: &[u8], approval_raw: &[u8]) -> Result<ApprovalReport, Error> {
    let document = decode_document(document_raw)?;
    let approval = decode_approval(approval_raw)?;
    let action = canonicalize(&document.proposal, &document.context)?;
    let binding_matches = action.matches_approval(&approval)?;
    let mut mismatched_fields = Vec::new();
    if approval.action_digest != action.digest() {
        mismatched_fields.push("action_digest");
    }
    if approval.policy_digest != document.context.policy_digest {
        mismatched_fields.push("policy_digest");
    }
    Ok(ApprovalReport {
        schema_version: SCHEMA_VERSION,
        scope: "offline-data-only",
        document: DocumentIdentity {
            document_sha256: format!("{:x}", Sha256::digest(document_raw)),
            action_digest: action.digest().to_owned(),
            intent_id: action.intent_id().to_owned(),
        },
        approval_sha256: format!("{:x}", Sha256::digest(approval_raw)),
        binding_matches,
        mismatched_fields,
        authority: "not-established",
        approval_authenticity: "not-established",
        approval_validity: "not-established",
        effect: "not-established",
    })
}
