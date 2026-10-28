//! Additive v2 installation declaration. Its digest/checkpoint must be supplied
//! independently; neither canonical bytes nor these labels authenticate a human.
use super::*;
use std::path::Path;
use swarm_enrollment_state::Head;
use swarm_identity_contract::{digest, text, Scope};

/// Closed v2 declaration for a current-source startup observation. The v1
/// installation declaration remains nested unchanged. authority_head is an
/// independently approved checkpoint, not a value learned from this journal.
/// Updating the journal makes an old checkpoint stale; no automatic adoption.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlDocument {
    pub schema_version: i64,
    pub installation: Document,
    pub journal_path: String,
    pub project_context_digest: String,
    pub authority_head: Head,
    pub enrollment_id: String,
    pub generation: i64,
    pub scope: Scope,
    pub trust_configuration_revision: i64,
}
impl ControlDocument {
    /// Derive the clean journal parent after validating the complete declaration.
    /// No file access or permission is granted by this pathname calculation.
    pub(crate) fn journal_directory(&self) -> Result<&str, Error> {
        Path::new(&self.journal_path)
            .parent()
            .and_then(Path::to_str)
            .filter(|p| clean_path(p))
            .ok_or(Error::Document)
    }
}
/// Encode exact validated v2 bytes without opening storage or issuing authority.
/// Reject unknown versions, invalid context/checkpoint and overlapping roots.
/// Configuration, witness and journal roots must be logically distinct; the
/// Linux loader additionally checks actual ownership, modes and physical roots.
pub fn encode_control(d: &ControlDocument) -> Result<Vec<u8>, Error> {
    encode(&d.installation)?;
    if d.schema_version != 2
        || !clean_path(&d.journal_path)
        || !digest(d.project_context_digest.as_bytes())
        || !d.authority_head.valid()
        || d.authority_head.sequence <= 0
        || !text(d.enrollment_id.as_bytes())
        || d.generation <= 0
        || d.generation > d.authority_head.sequence
        || !d.scope.valid()
        || d.trust_configuration_revision <= 0
    {
        return Err(Error::Document);
    }
    let parent = d.journal_directory()?;
    if overlaps(parent, &d.installation.configuration_directory)
        || overlaps(parent, &d.installation.witness_directory)
    {
        return Err(Error::Document);
    }
    wire::encode(d).map_err(Error::Frame)
}
/// Require exact canonical v2 JSON, including nested installation and context.
/// No v1 migration, null/default substitution or authority inference occurs.
pub fn decode_control(raw: &[u8]) -> Result<ControlDocument, Error> {
    let d: ControlDocument = wire::decode(raw).map_err(Error::Frame)?;
    if encode_control(&d)? != raw {
        return Err(Error::Document);
    }
    Ok(d)
}
