//! Business return mediation, separate from write/invocation admission.
use super::TrustError as E;
use crate::{
    biba::{check_biba, BibaAccess, Integrity},
    trust_types::TrustIdentity,
};
use serde_json::{json, Value};

/// Return an ordinary I2 object only when the current subject may read it.
/// Otherwise the named status-acknowledgement procedure emits only a validated
/// identifier and a closed server state. It neither relabels the original nor
/// grants an effect. Apply to fresh replies and historical replays alike.
/// Malformed protected replies refuse; no private parameters are diagnostic.
pub(crate) fn mediate(op: &str, actor: &TrustIdentity, value: Value) -> Result<Value, E> {
    if !matches!(
        op,
        "mandate-create" | "mandate-revoke" | "request" | "approve" | "commit" | "reconcile"
    ) || value.get("state").and_then(Value::as_str) == Some("denied")
    {
        return Ok(value);
    }
    let subject = match actor.integrity {
        1 => Integrity::I1,
        2 => Integrity::I2,
        3 => Integrity::I3,
        _ => return Err(E::Authorization),
    };
    if check_biba(subject, Integrity::I2, BibaAccess::Read).is_ok() {
        return Ok(value);
    }
    let id = value
        .get("id")
        .or_else(|| value.get("object_id"))
        .and_then(Value::as_str)
        .filter(|s| swarm_identity_contract::name(s.as_bytes()))
        .ok_or(E::Integrity("trust-response-identity"))?;
    let state = match op {
        "mandate-create" => "admitted",
        "mandate-revoke" => "revoked",
        _ => value
            .get("state")
            .and_then(Value::as_str)
            .filter(|s| {
                matches!(
                    *s,
                    "pending_approval" | "approved" | "committed" | "unknown" | "reconciled"
                )
            })
            .ok_or(E::Integrity("trust-response-state"))?,
    };
    Ok(
        json!({"schema_version":1,"kind":"trust-status-ack-v1","object_id":id,"state":state,"content_redacted":true}),
    )
}
