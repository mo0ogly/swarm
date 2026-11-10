//! Pure comparison of validated PAEA documents, never an authorization decision.
use crate::{canonicalize, decode_document, Error};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// Byte-bound observation of one supplied document; hashes are not signatures.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DocumentIdentity {
    pub document_sha256: String,
    pub action_digest: String,
    pub intent_id: String,
}

/// Content comparison only. Empty paths never establish approval freshness or
/// authority. Field values are omitted; digests/change metadata still need an
/// authorized recipient. Sources are reported as one ordered collection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DocumentComparison {
    pub schema_version: u32,
    pub authority: &'static str,
    pub effect: &'static str,
    pub approval_validity: &'static str,
    pub before: DocumentIdentity,
    pub after: DocumentIdentity,
    pub changed_paths: Vec<String>,
    pub canonical_content_changed: bool,
    pub intent_changed: bool,
    pub transport_request_changed: bool,
}

/// Compare direct fields of serialized typed protocol objects, without values.
/// Keys originate only from our Action/AdmissionContext serializers, never from
/// a caller-controlled map. Arrays (sources) stay whole; missing variant fields
/// count as changes. Unexpected internal object shapes refuse safely with no IO.
fn fields(before: &Value, after: &Value, prefix: &str) -> Result<Vec<String>, Error> {
    let before = before
        .as_object()
        .ok_or(Error::Invalid("protection-comparison-shape-invalid"))?;
    let after = after
        .as_object()
        .ok_or(Error::Invalid("protection-comparison-shape-invalid"))?;
    let keys: BTreeSet<_> = before.keys().chain(after.keys()).collect();
    Ok(keys
        .into_iter()
        .filter(|key| before.get(*key) != after.get(*key))
        .map(|key| format!("{prefix}/{key}"))
        .collect())
}

/// Decode and validate two raw documents, canonicalize both, then compare their
/// actual canonical bytes and direct action/context fields. Invalid JSON or
/// representation returns the existing safe Error before any result is returned.
/// Request IDs are compared separately; normalized label sets do not change
/// content, while source order does. Inputs are unchanged; no IO, policy lookup,
/// credential, clock, approval or effect occurs. Valid supplied context remains
/// unauthenticated. Changed content needs protected control review; equal content
/// also establishes no current mandate, approval, tool freshness or permission.
pub fn compare_documents(before_raw: &[u8], after_raw: &[u8]) -> Result<DocumentComparison, Error> {
    let before_document = decode_document(before_raw)?;
    let after_document = decode_document(after_raw)?;
    let before = canonicalize(&before_document.proposal, &before_document.context)?;
    let after = canonicalize(&after_document.proposal, &after_document.context)?;
    let old: Value = serde_json::from_slice(before.bytes()).map_err(Error::Json)?;
    let new: Value = serde_json::from_slice(after.bytes()).map_err(Error::Json)?;
    let mut changed_paths = fields(&old["action"], &new["action"], "/action")?;
    changed_paths.extend(fields(&old["context"], &new["context"], "/context")?);
    changed_paths.sort();
    Ok(DocumentComparison {
        schema_version: crate::SCHEMA_VERSION,
        authority: "not-established",
        effect: "not-established",
        approval_validity: "not-established",
        before: DocumentIdentity {
            document_sha256: format!("{:x}", Sha256::digest(before_raw)),
            action_digest: before.digest().to_owned(),
            intent_id: before.intent_id().to_owned(),
        },
        after: DocumentIdentity {
            document_sha256: format!("{:x}", Sha256::digest(after_raw)),
            action_digest: after.digest().to_owned(),
            intent_id: after.intent_id().to_owned(),
        },
        changed_paths,
        canonical_content_changed: before.bytes() != after.bytes(),
        intent_changed: before.intent_id() != after.intent_id(),
        transport_request_changed: before_document.proposal.request_id
            != after_document.proposal.request_id,
    })
}
