//! Go-compatible event hashes, originals and HMAC anchors. Hash integrity does
//! not authenticate a clock, protect from same-UID writers or establish time.
use super::{digest, hash, proof_types::*, state::State, TrustError as E};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Explicit historical preimage order/labels; hash itself is intentionally absent.
#[derive(Serialize)]
struct HashEvidence<'a> {
    sequence: i64,
    #[serde(rename = "PreviousHash")]
    previous_hash: &'a str,
    #[serde(rename = "Kind")]
    kind: &'a str,
    #[serde(rename = "Actor")]
    actor: &'a str,
    #[serde(rename = "Integrity")]
    integrity: i64,
    #[serde(rename = "ObjectID")]
    object_id: &'a str,
    #[serde(rename = "ContentHash")]
    content_hash: &'a str,
    #[serde(rename = "OriginalHash")]
    original_hash: &'a str,
    #[serde(rename = "Result")]
    result: &'a str,
    #[serde(rename = "Reason")]
    reason: &'a str,
    #[serde(rename = "At")]
    at: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    identity_context: &'a Option<IdentityContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    policy_context: &'a Option<ProofPolicyContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    effect_receipt: &'a Option<EffectProof>,
}
/// Hash a complete immutable event with the legacy Go preimage. Missing optional
/// contexts preserve old bytes; present contexts are part of the signed content.
pub(crate) fn evidence_hash(e: &Evidence) -> Result<String, E> {
    digest(&HashEvidence {
        sequence: e.sequence,
        previous_hash: &e.previous_hash,
        kind: &e.kind,
        actor: &e.actor,
        integrity: e.integrity,
        object_id: &e.object_id,
        content_hash: &e.content_digest,
        original_hash: &e.original_digest,
        result: &e.result,
        reason: &e.reason,
        at: &e.at,
        identity_context: &e.identity_context,
        policy_context: &e.policy_context,
        effect_receipt: &e.effect_receipt,
    })
}
/// Historical export schema2. Original values are base64-encoded exact bytes.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProofExport {
    pub originals: BTreeMap<String, String>,
    pub schema_version: i64,
    pub simulation: bool,
    pub evidence: Vec<Evidence>,
}
/// Historical symmetric anchor, not a public signature or independently kept pin.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Anchor {
    pub schema_version: i64,
    pub sequence: i64,
    pub hash: String,
    pub signature: String,
}
/// Return only originals referenced by this prefix; never configuration secrets.
pub(crate) fn export(st: &State) -> Result<ProofExport, E> {
    let mut originals = BTreeMap::new();
    for e in &st.evidence {
        let raw = st
            .originals
            .get(&e.original_digest)
            .ok_or(E::Integrity("trust-original-missing"))?;
        originals.insert(e.original_digest.clone(), raw.clone());
    }
    Ok(ProofExport {
        originals,
        schema_version: 2,
        simulation: true,
        evidence: st.evidence.clone(),
    })
}
/// Validate chain and original bytes before a new effect or anchor. Rejections
/// leave state/counters untouched; a stored replay is handled separately.
pub(crate) fn verify_chain(exp: &ProofExport) -> Result<(), E> {
    if exp.schema_version != 2 || !exp.simulation {
        return Err(E::Integrity("trust-export-schema"));
    }
    let mut previous = String::new();
    let mut used = BTreeSet::new();
    for (index, e) in exp.evidence.iter().enumerate() {
        if e.sequence
            != i64::try_from(index + 1)
                .map_err(|_| E::Integrity("trust-sequence-representation"))?
            || e.previous_hash != previous
            || evidence_hash(e)? != e.hash
        {
            return Err(E::Integrity("trust-evidence-chain"));
        }
        let raw = STANDARD
            .decode(
                exp.originals
                    .get(&e.original_digest)
                    .ok_or(E::Integrity("trust-original-missing"))?,
            )
            .map_err(|_| E::Integrity("trust-original-base64"))?;
        if hash(&raw) != e.original_digest
            || swarm_wire_json::decode::<serde_json::Value>(&raw).is_err()
        {
            return Err(E::Integrity("trust-original-altered"));
        }
        verify_context(e, &exp.evidence[..index])?;
        used.insert(&e.original_digest);
        previous = e.hash.clone();
    }
    if used.len() != exp.originals.len() {
        return Err(E::Integrity("trust-original-unlinked"));
    }
    Ok(())
}
/// Check the semantic links in new contexts, independently of the event hash.
/// Historical events without contexts remain verifiable; they are not upgraded.
fn verify_context(e: &Evidence, prior: &[Evidence]) -> Result<(), E> {
    let Some(policy) = &e.policy_context else {
        return if e.effect_receipt.is_none() {
            Ok(())
        } else {
            Err(E::Integrity("trust-context-without-policy"))
        };
    };
    if policy.schema_version != 1 || policy.revision < 1 {
        return Err(E::Integrity("trust-policy-context"));
    }
    policy.policy.validate().map_err(E::Invalid)?;
    if let Some(roles)=&policy.role_separation{roles.validate_shape()?;}
    if let Some(roles)=policy.role_separation.as_ref().filter(|r|r.enabled){
        if e.kind=="approve"&&e.result=="approved"{
            let request=prior.iter().find(|p|p.kind=="request"&&p.result=="admitted"&&p.object_id==e.object_id&&p.content_digest==e.content_digest).ok_or(E::Integrity("trust-role-request-proof"))?;
            let mandate=request.identity_context.as_ref().and_then(|i|i.delegation_id.as_deref()).ok_or(E::Integrity("trust-role-mandate-context"))?;
            let admitted=prior.iter().find(|p|p.kind=="mandate-create"&&p.result=="admitted"&&p.object_id==mandate).ok_or(E::Integrity("trust-role-mandate-proof"))?;
            if roles.conflicts(&[("admin",&admitted.actor),("requester",&request.actor),("approver",&e.actor)])?{return Err(E::Integrity("trust-role-approval-conflict"));}
        }
    }
    let controls: serde_json::Value = serde_json::from_str(policy.controls.raw())?;
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("../../config/trust-controls-defaults.json"))?;
    if !controls.as_object().is_some_and(|c| {
        c.len() == expected.as_object().map_or(0, |v| v.len())
            && expected
                .as_object()
                .unwrap()
                .keys()
                .all(|k| c.get(k).is_some_and(|v| !v.is_null()))
    }) {
        return Err(E::Integrity("trust-controls-context"));
    }
    let positive = matches!(e.kind.as_str(), "commit" | "reconcile")
        && matches!(e.result.as_str(), "committed" | "unknown" | "reconciled");
    let Some(receipt) = &e.effect_receipt else {
        return if positive {
            Err(E::Integrity("trust-effect-context-required"))
        } else {
            Ok(())
        };
    };
    if !positive || receipt.schema_version != 1 {
        return Err(E::Integrity("trust-effect-context"));
    }
    #[derive(Serialize)]
    struct Parameters<'a> {
        amount_minor: i64,
        currency: &'a str,
        beneficiary: &'a str,
    }
    let (native_hash, key, resource, committed, parameters) = match receipt.connector_id.as_str() {
        "local-ledger-simulation" => {
            let n: LedgerEntry = serde_json::from_str(receipt.native_result.raw())?;
            if n.sequence < 1
                || !n.simulation
                || n.action_id != e.object_id
                || n.content_digest != e.content_digest
            {
                return Err(E::Integrity("trust-native-context"));
            }
            let params = super::encoded(&Parameters {
                amount_minor: n.amount_minor,
                currency: &n.currency,
                beneficiary: &n.beneficiary,
            })?;
            (
                digest(&n)?,
                n.idempotency_key,
                n.resource,
                n.committed_at,
                params,
            )
        }
        "local-access-simulation" => {
            let n: AccessEntry = serde_json::from_str(receipt.native_result.raw())?;
            if n.sequence < 1
                || !n.simulation
                || n.action_id != e.object_id
                || n.content_digest != e.content_digest
            {
                return Err(E::Integrity("trust-native-context"));
            }
            let params = super::encoded(&n.access)?;
            (
                digest(&n)?,
                n.idempotency_key,
                n.resource,
                n.committed_at,
                params,
            )
        }
        _ => return Err(E::Unsupported("trust-proof-connector")),
    };
    if !swarm_identity_contract::name(key.as_bytes()) || !super::command::resource(&resource) {
        return Err(E::Integrity("trust-native-identity"));
    }
    if receipt.legacy_receipt {
        return if receipt.intent.is_none()
            && receipt.result.is_none()
            && receipt.connector_id == "local-ledger-simulation"
        {
            Ok(())
        } else {
            Err(E::Integrity("trust-legacy-receipt"))
        };
    };
    let intent = receipt
        .intent
        .as_ref()
        .ok_or(E::Integrity("trust-intent-required"))?;
    let result = receipt
        .result
        .as_ref()
        .ok_or(E::Integrity("trust-result-required"))?;
    if intent.intent_id != format!("intent-{}", e.content_digest)
        || intent.idempotency_key != key
        || intent.resource != resource
        || result.committed_at != committed
        || hash(intent.canonical_parameters.raw().as_bytes()) != hash(&parameters)
        || intent.schema_version != 1
        || result.schema_version != 1
        || intent.action_digest != e.content_digest
        || intent.connector_id != receipt.connector_id
        || intent.connector_version != 1
        || result.intent_id != intent.intent_id
        || result.result_digest != native_hash
        || result.state != e.result
        || result.observed_at != e.at
        || result.connector_receipt != format!("receipt-{}", digest(intent)?)
    {
        return Err(E::Integrity("trust-effect-links"));
    }
    if intent.approval_digests.is_empty()
        || !intent.approval_digests.iter().all(|hash| {
            prior.iter().any(|p| {
                p.hash == *hash
                    && p.kind == "approve"
                    && p.result == "approved"
                    && p.object_id == e.object_id
                    && p.content_digest == e.content_digest
            })
        })
        || !prior.iter().any(|p| {
            p.kind == "mandate-create"
                && p.result == "admitted"
                && p.content_digest == intent.mandate_digest
        })
    {
        return Err(E::Integrity("trust-effect-authorizations"));
    }
    // Reconciliation authenticates readback of the historical effect, not a new
    // write under today's participant grouping. Earlier commit proof remains checked.
    if let Some(roles)=policy.role_separation.as_ref().filter(|_|e.kind=="commit"){
        let admitted=prior.iter().find(|p|p.kind=="mandate-create"&&p.result=="admitted"&&p.content_digest==intent.mandate_digest).ok_or(E::Integrity("trust-role-mandate-proof"))?;
        let request=prior.iter().find(|p|p.kind=="request"&&p.result=="admitted"&&p.object_id==e.object_id&&p.content_digest==e.content_digest).ok_or(E::Integrity("trust-role-request-proof"))?;
        for hash in &intent.approval_digests{
            let approval=prior.iter().find(|p|p.hash==*hash).ok_or(E::Integrity("trust-role-approval-proof"))?;
            if roles.conflicts(&[("admin",&admitted.actor),("requester",&request.actor),("approver",&approval.actor),("effect",&e.actor)])?{return Err(E::Integrity("trust-role-effect-conflict"));}
        }
    }
    Ok(())
}
/// Produce a symmetric HMAC anchor after complete prefix validation. Key comes
/// from a separate private source; no export-provided key is implicitly trusted.
pub(crate) fn anchor(exp: &ProofExport, key: &[u8]) -> Result<Anchor, E> {
    use hmac::{Hmac, KeyInit, Mac};
    verify_chain(exp)?;
    let last = exp
        .evidence
        .last()
        .ok_or(E::Invalid("aucune preuve à ancrer"))?;
    if key.len() != 32 {
        return Err(E::Invalid("clé de preuve de 32 octets requise"));
    }
    let mut mac = Hmac::<sha2_csrf::Sha256>::new_from_slice(key)
        .map_err(|_| E::Invalid("clé de preuve invalide"))?;
    mac.update(format!("{}:{}", last.sequence, last.hash).as_bytes());
    let signature = mac
        .finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Ok(Anchor {
        schema_version: 1,
        sequence: last.sequence,
        hash: last.hash.clone(),
        signature,
    })
}
/// Verify exact length/order/content and separately supplied symmetric key.
/// An intact anchor without an independent checkpoint does not detect rollback.
pub(crate) fn verify(exp: &ProofExport, pin: &Anchor, key: &[u8]) -> Result<(), E> {
    use subtle::ConstantTimeEq;
    let expected = anchor(exp, key)?;
    if pin.schema_version != 1
        || pin.sequence != expected.sequence
        || pin.hash != expected.hash
        || !bool::from(
            pin.signature
                .as_bytes()
                .ct_eq(expected.signature.as_bytes()),
        )
    {
        return Err(E::Integrity("trust-anchor-mismatch"));
    }
    Ok(())
}
/// Decode exactly32 hex bytes. Private material is never included in errors.
pub(crate) fn decode_key(raw: &str) -> Result<Vec<u8>, E> {
    let raw = raw.strip_suffix('\n').unwrap_or(raw);
    if raw.len() != 64 || !raw.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(E::Invalid("clé de preuve de 32 octets requise"));
    }
    (0..64)
        .step_by(2)
        .map(|n| {
            u8::from_str_radix(&raw[n..n + 2], 16)
                .map_err(|_| E::Invalid("clé de preuve hexadécimale requise"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    /// Build a real authorized producer chain before injecting any proof fault.
    /// Fixed time/credentials are fixtures, never runtime defaults or authority.
    fn fixture() -> ProofExport {
        let now = time::OffsetDateTime::from_unix_timestamp(1792880000).unwrap();
        let mut st = State::initial();
        let cfg = json!({"schema_version":1,"event_id":"config","expected_revision":0,"reason":"Proof integrity fixture","policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":true},"identities":[{"subject":"requester","token":"fixture-requester-token","integrity":1,"roles":["requester"]},{"subject":"approver","token":"fixture-approver-token","integrity":2,"roles":["approver"]},{"subject":"admin","token":"fixture-admin-token","integrity":3,"roles":["admin","effect"]}]});
        super::super::config::change(
            &mut st,
            &serde_json::to_vec(&cfg).unwrap(),
            false,
            now,
            "fixture",
        )
        .unwrap();
        fn run(
            st: &mut State,
            op: &str,
            token: &str,
            mut v: serde_json::Value,
            now: time::OffsetDateTime,
        ) -> serde_json::Value {
            v["schema_version"] = 1.into();
            v["event_id"] = op.into();
            serde_json::from_slice(
                &super::super::operations::execute(
                    st,
                    op,
                    &serde_json::to_vec(&v).unwrap(),
                    token,
                    now,
                )
                .unwrap()
                .bytes,
            )
            .unwrap()
        }
        run(
            &mut st,
            "mandate-create",
            "fixture-admin-token",
            json!({"mandate_id":"m","subject":"requester","actions":["settle"],"resource":"invoice/one","max_amount_minor":10,"currency":"EUR","beneficiary":"supplier"}),
            now,
        );
        let a = run(
            &mut st,
            "request",
            "fixture-requester-token",
            json!({"mandate_id":"m","resource":"invoice/one","amount_minor":9,"currency":"EUR","beneficiary":"supplier","idempotency_key":"one"}),
            now,
        );
        run(
            &mut st,
            "approve",
            "fixture-approver-token",
            json!({"action_id":a["id"],"mandate_id":"m","content_digest":a["content_digest"],"resource":"invoice/one","amount_minor":9,"currency":"EUR","beneficiary":"supplier"}),
            now,
        );
        run(
            &mut st,
            "commit",
            "fixture-admin-token",
            json!({"action_id":a["id"]}),
            now,
        );
        export(&st).unwrap()
    }
    /// Intact producer witness then cryptographic and semantic faults, including a
    /// freshly rehashed forged event. Merely verifying its hash would miss this.
    #[test]
    fn proof_rejects_alteration_truncation_reorder_and_resigned_semantic_forgery() {
        let original = fixture();
        verify_chain(&original).unwrap();
        let key = [7u8; 32];
        let pin = anchor(&original, &key).unwrap();
        verify(&original, &pin, &key).unwrap();
        assert!(verify(&original, &pin, &[8u8; 32]).is_err());
        let mut changed = original.clone();
        changed.evidence.pop();
        assert!(verify(&changed, &pin, &key).is_err());
        let mut changed = original.clone();
        changed.evidence.swap(0, 1);
        assert!(verify_chain(&changed).is_err());
        let mut changed = original.clone();
        let first = changed.evidence[0].original_digest.clone();
        changed.originals.insert(first, STANDARD.encode(b"{}"));
        assert!(verify_chain(&changed).is_err());
        let mut changed = original.clone();
        let e = changed.evidence.last_mut().unwrap();
        let receipt = e.effect_receipt.as_mut().unwrap();
        let intent = receipt.intent.as_mut().unwrap();
        intent.canonical_parameters = super::super::json::CompactJson::new(
            r#"{"amount_minor":10,"currency":"EUR","beneficiary":"supplier"}"#,
        )
        .unwrap();
        receipt.result.as_mut().unwrap().connector_receipt =
            format!("receipt-{}", digest(intent).unwrap());
        e.hash = evidence_hash(e).unwrap();
        assert!(verify_chain(&changed).is_err());
    }
}
