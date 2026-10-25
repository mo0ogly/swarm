//! Per-action Biba reference monitor. Fixed simulated effects only; no provider,
//! Linux confinement or authority is conferred by deserializing its data.
pub(crate) mod command;
pub(crate) mod config;
pub mod error;
pub(crate) mod json;
pub(crate) mod operations;
pub(crate) mod proof;
pub(crate) mod proof_types;
pub(crate) mod state;
pub use error::TrustError;
use serde::Serialize;
use sha2::{Digest, Sha256};
/// Observable reply retained through Root close/output failures. Publication
/// describes recorded/replayed or uncertain state, never client receipt or a
/// new effect inferred from replay. The caller owes checked delivery.
pub struct TrustReply {
    pub bytes: Vec<u8>,
    pub publication: Option<&'static str>,
}
/// Hash complete bytes with SHA256. Domain separation belongs to each producer.
pub(crate) fn hash(raw: &[u8]) -> String {
    Sha256::digest(raw)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
/// Go-compatible JSON string escaping and typed field order; no authentication.
pub(crate) fn encoded<T: Serialize>(value: &T) -> Result<Vec<u8>, TrustError> {
    swarm_wire_json::encode(value).map_err(Into::into)
}
/// Digest the complete typed Go preimage, retaining encoding errors.
pub(crate) fn digest<T: Serialize>(value: &T) -> Result<String, TrustError> {
    encoded(value).map(|raw| hash(&raw))
}
/// Pure verification with a separately supplied symmetric key and anchor. No
/// Root, credentials, clock or provider is consulted; malformed/missing material
/// refuses. Integrity does not establish an independent checkpoint or time.
pub fn verify_trust(raw: &[u8]) -> Result<Vec<u8>, TrustError> {
    let c: command::Command = swarm_wire_json::decode(raw)?;
    let export = c.export.ok_or(TrustError::Invalid("export requis"))?;
    let anchor = c.anchor.ok_or(TrustError::Invalid("ancre requise"))?;
    let key = proof::decode_key(&c.verification_key)?;
    proof::verify(&export, &anchor, &key)?;
    encoded(
        &serde_json::json!({"schema_version":1,"verified":true,"simulation":true,"evidence_count":export.evidence.len(),"sequence":anchor.sequence,"hash":anchor.hash}),
    )
}
