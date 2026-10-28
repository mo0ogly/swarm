//! Explicit v1 canonical bytes; independent of serde_json map-feature ordering.
use crate::{Action, AdmissionContext, ApprovalBinding, Error, Proposal, SCHEMA_VERSION};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Immutable data fingerprint and business key. Creation is not admission or
/// authorization: consumers owe authenticated context, fresh policy, exact
/// approval, sink identity, last-use authority and business-state verification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalAction {
    bytes: Vec<u8>,
    digest: String,
    intent_id: String,
    policy_digest: String,
}

impl CanonicalAction {
    /// Borrow exact paea-json-v1 bytes; transport request_id is not included.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Borrow lowercase SHA-256 of the exact canonical bytes, not a signature.
    pub fn digest(&self) -> &str {
        &self.digest
    }
    /// Borrow the domain-derived intent digest, independent of transport keys,
    /// mandate/policy changes and alternative amounts/candidates for that intent.
    /// A future service must enforce uniqueness; this getter stores nothing.
    pub fn intent_id(&self) -> &str {
        &self.intent_id
    }
    /// Compare valid approval binding bytes only. A true result authenticates
    /// nobody and establishes neither separation of duties nor last-use validity.
    pub fn matches_approval(&self, approval: &ApprovalBinding) -> Result<bool, Error> {
        approval.validate()?;
        Ok(approval.action_digest == self.digest && approval.policy_digest == self.policy_digest)
    }
}

/// Emit deterministic JSON with sorted schema keys, minimal serde_json escaping,
/// exact integers and ordered arrays. String normalization is never implicit.
/// Refuses floats even if future fields accidentally introduce one. No IO.
fn write(value: &Value, out: &mut Vec<u8>) -> Result<(), Error> {
    match value {
        Value::Object(map) => {
            out.push(b'{');
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            for (i, key) in keys.into_iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                serde_json::to_writer(&mut *out, key).map_err(Error::Json)?;
                out.push(b':');
                write(&map[key], out)?;
            }
            out.push(b'}');
        }
        Value::Array(values) => {
            out.push(b'[');
            for (i, item) in values.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write(item, out)?;
            }
            out.push(b']');
        }
        Value::Number(number) if !number.is_i64() && !number.is_u64() => {
            return Err(Error::Invalid("protection-floating-number-forbidden"))
        }
        _ => serde_json::to_writer(out, value).map_err(Error::Json)?,
    }
    Ok(())
}

/// Compute canonical action and domain-derived intent from validated data.
/// Payment intent is project+invoice, not version/amount/request key. Git intent
/// is project+repository+direct ref+expected old OID, not the candidate/request key.
/// Those identities prevent transport aliases from defining business uniqueness;
/// only a future effect service can enforce it. Label sets are sorted, source
/// order is preserved. All original inputs remain unchanged. No external state,
/// clock, permission, effect, signature or policy decision is produced.
pub fn canonicalize(
    proposal: &Proposal,
    context: &AdmissionContext,
) -> Result<CanonicalAction, Error> {
    proposal.validate()?;
    context.validate()?;
    let intent = match &proposal.action {
        Action::Payment { invoice_id, .. } => {
            json!({"kind":"payment.settle","project_id":context.project_id,"invoice_id":invoice_id})
        }
        Action::Git {
            repository_id,
            target_ref,
            expected_old_oid,
            ..
        } => {
            json!({"kind":"repository.promote","project_id":context.project_id,"repository_id":repository_id,"target_ref":target_ref,"expected_old_oid":expected_old_oid})
        }
    };
    let mut intent_bytes = b"paea-intent-v1\n".to_vec();
    write(&intent, &mut intent_bytes)?;
    let mut normalized = context.clone();
    for source in &mut normalized.sources {
        source.confidentiality_labels.sort();
    }
    let value = json!({"schema_version":SCHEMA_VERSION,"canonicalization":"paea-json-v1","action":proposal.action,"context":normalized});
    let mut bytes = Vec::new();
    write(&value, &mut bytes)?;
    Ok(CanonicalAction {
        digest: format!("{:x}", Sha256::digest(&bytes)),
        intent_id: format!("{:x}", Sha256::digest(&intent_bytes)),
        policy_digest: context.policy_digest.clone(),
        bytes,
    })
}
