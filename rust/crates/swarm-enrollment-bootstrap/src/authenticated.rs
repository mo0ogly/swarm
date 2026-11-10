//! Closed composite declaration. This is pinned data, never authentication,
//! a listener, an adopted current checkpoint or permission to provision a human.
use super::*;
use std::path::Path;
use swarm_enrollment_state::Head;
use swarm_identity_contract::digest;
/// Explicit personal-source context. Installation/principal derive from the
/// protected nested installation, not a client field. bootstrap_head is the
/// independently supplied initial provisioning head, not current-state discovery.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersonalDeclaration {
    pub authority_public_key: String,
    pub project_context_digest: String,
    pub journal_path: String,
    pub checkpoint_directory: String,
    pub generation: i64,
    pub bootstrap_head: Head,
}
impl PersonalDeclaration {
    /// Borrow the clean existing journal parent name after structural validation.
    /// No IO, root adoption or filesystem protection is established by this path.
    pub(crate) fn journal_directory(&self) -> Result<&str, Error> {
        Path::new(&self.journal_path)
            .parent()
            .and_then(Path::to_str)
            .filter(|p| clean_path(p))
            .ok_or(Error::Document)
    }
}
/// Canonical v3 composition of existing business declaration and personal
/// source. No private signing key, password, secret, caller actor or policy default.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthenticatedDocument {
    pub schema_version: i64,
    pub control: ControlDocument,
    pub personal: PersonalDeclaration,
}
impl AuthenticatedDocument {
    /// Names of all five storage domains, never observations/permissions. Caller
    /// validates complete declaration first. Order is protocol structure, no cap.
    pub(crate) fn storage_roots(&self) -> Result<Vec<&str>, Error> {
        Ok(vec![
            &self.control.installation.configuration_directory,
            &self.control.installation.witness_directory,
            self.control.journal_directory()?,
            &self.personal.checkpoint_directory,
            self.personal.journal_directory()?,
        ])
    }
}
/// Encode closed v3 data after validating existing v2 and explicit personal
/// fields. Root directories must not overlap; personal/business signing context
/// digests differ so signed events are not interchangeable across the two domains.
/// Reusing a protected key is allowed, not required. Personal/grant generations
/// are distinct. This validates neither files, cryptographic history nor a human.
/// Typed shared Pins validation failures remain Binding causes, not empty success.
pub fn encode_authenticated(d: &AuthenticatedDocument) -> Result<Vec<u8>, Error> {
    encode_control(&d.control)?;
    let p = &d.personal;
    if d.schema_version != 3
        || !digest(p.project_context_digest.as_bytes())
        || p.project_context_digest == d.control.project_context_digest
        || p.generation <= 0
        || !p.bootstrap_head.valid()
        || p.bootstrap_head.sequence <= 0
        || !clean_path(&p.journal_path)
        || !clean_path(&p.checkpoint_directory)
    {
        return Err(Error::Document);
    }
    let mut pins = d.control.installation.pins.clone();
    pins.public_key = p.authority_public_key.clone();
    pins.validate().map_err(Error::Binding)?;
    let roots = d.storage_roots()?;
    for (index, root) in roots.iter().enumerate() {
        if roots[..index]
            .iter()
            .any(|previous| overlaps(root, previous))
        {
            return Err(Error::Document);
        }
    }
    wire::encode(d).map_err(Error::Frame)
}
/// Decode exactly canonical closed v3 JSON, including all nested schemas. Unknown,
/// ambiguous, null/absent required fields and alternate encodings refuse; no v1/v2
/// fallback or automatic migration. The result is data, never current authority.
pub fn decode_authenticated(raw: &[u8]) -> Result<AuthenticatedDocument, Error> {
    let d: AuthenticatedDocument = wire::decode(raw).map_err(Error::Frame)?;
    if encode_authenticated(&d)? != raw {
        return Err(Error::Document);
    }
    Ok(d)
}
