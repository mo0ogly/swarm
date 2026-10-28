//! Exact offline action content for human inspection, never an approval.
use crate::{canonicalize, decode_document, DocumentIdentity, Error, SCHEMA_VERSION};
use serde::Serialize;
use sha2::{Digest, Sha256};

/// Versioned data report binding one original document to its exact canonical text.
/// Declared context remains unauthenticated. Hashes and readable terms establish
/// neither current authority, approval validity nor an effect. Canonical text
/// retains all integer lexemes and source order; request_id stays outside it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InspectionReport {
    pub schema_version: u32,
    pub scope: &'static str,
    pub document: DocumentIdentity,
    pub canonicalization: &'static str,
    pub canonical: String,
    pub authority: &'static str,
    pub effect: &'static str,
    pub approval_validity: &'static str,
}

/// Strictly validate an original v1 document, then expose its exact canonical
/// content, raw SHA256 and existing action/intent identities. RawValue checks
/// JSON/UTF-8 and borrows its text without numeric conversion or reserialization.
/// Existing JSON/representation errors retain their typed cause and safe code;
/// no partial report, IO, Root, policy lookup, approval or effect is produced.
/// Canonicalization and the v1 action/intent preimages are unchanged.
pub fn inspect_document(raw: &[u8]) -> Result<InspectionReport, Error> {
    let document = decode_document(raw)?;
    let action = canonicalize(&document.proposal, &document.context)?;
    let canonical: &serde_json::value::RawValue =
        serde_json::from_slice(action.bytes()).map_err(Error::Json)?;
    Ok(InspectionReport {
        schema_version: SCHEMA_VERSION,
        scope: "offline-data-only",
        document: DocumentIdentity {
            document_sha256: format!("{:x}", Sha256::digest(raw)),
            action_digest: action.digest().to_owned(),
            intent_id: action.intent_id().to_owned(),
        },
        canonicalization: "paea-json-v1",
        canonical: canonical.get().to_owned(),
        authority: "not-established",
        effect: "not-established",
        approval_validity: "not-established",
    })
}
