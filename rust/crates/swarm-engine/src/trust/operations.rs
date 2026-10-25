//! Authenticated pure transitions. Storage owner serializes and publishes once;
//! this module cannot start a process, write a file, contact a provider or pay.
use super::{
    command::{self, Command},
    digest, encoded, hash,
    json::CompactJson,
    proof,
    proof_types::*,
    state::State,
    TrustError as E,
};
use crate::biba::{check_biba, BibaAccess, Integrity};
use crate::trust_types::{credential_valid, token_hash, TrustIdentity};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;
use serde_json::{json, Value};
use subtle::ConstantTimeEq;
use time::OffsetDateTime;
/// Current authenticated local identity; caller-supplied claims are irrelevant.
/// Active federated policy must already have passed State::require_supported.
pub(crate) fn authenticated(st: &State, token: &str) -> Result<TrustIdentity, E> {
    if !credential_valid(token) {
        return Err(E::Authorization);
    }
    let digest = token_hash(token);
    st.identities
        .iter()
        .find(|id| bool::from(id.token_hash.as_bytes().ct_eq(digest.as_bytes())) && !id.disabled)
        .cloned()
        .ok_or(E::Authorization)
}
/// Stable replay authority binds subject, effective level and ordered role set.
fn authority(actor: &TrustIdentity) -> Result<String, E> {
    #[derive(Serialize)]
    #[allow(non_snake_case)]
    struct Authority<'a> {
        Subject: &'a str,
        Integrity: i64,
        Roles: Vec<String>,
    }
    let mut roles = actor.roles.clone();
    roles.sort();
    digest(&Authority {
        Subject: &actor.subject,
        Integrity: actor.integrity,
        Roles: roles,
    })
}
/// Parse recorded timestamps with the already shared Go RFC3339 contract.
fn timestamp(raw: &str) -> Result<OffsetDateTime, E> {
    swarm_wire_json::go_rfc3339::parse(raw).map_err(|_| E::Invalid("date RFC3339 requise"))
}
/// Format the server-observed UTC clock; a serialization error refuses safely.
fn stamp(now: OffsetDateTime) -> Result<String, E> {
    now.to_offset(time::UtcOffset::UTC)
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|_| E::Invalid("horloge non représentable"))
}
/// Recorded core strings are validated by their producer; missing values refuse
/// in each transition rather than becoming authority or a fabricated identifier.
fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}
fn number(v: &Value, key: &str) -> i64 {
    v.get(key).and_then(Value::as_i64).unwrap_or(0)
}
fn role(a: &TrustIdentity, want: &str) -> bool {
    a.roles.iter().any(|r| r == want)
}
/// Fixed labels are protocol invariants; serialized agent claims never select one.
fn level(value: i64) -> Result<Integrity, E> {
    match value {
        0 => Ok(Integrity::I0),
        1 => Ok(Integrity::I1),
        2 => Ok(Integrity::I2),
        3 => Ok(Integrity::I3),
        _ => Err(E::Integrity("trust-identity-level")),
    }
}
/// Gate a normal access through the shared strict monitor and current role level.
fn access(a: &TrustIdentity, object: i64, kind: BibaAccess) -> Result<(), E> {
    check_biba(level(a.integrity)?, level(object)?, kind).map_err(|_| E::Authorization)
}
/// Recheck the stored mandate at current clock/revision; no child chain fallback.
fn active(st: &State, id: &str, now: OffsetDateTime) -> Result<String, E> {
    let Some(m) = st.mandates.get(id) else {
        return Ok("mandat absent".into());
    };
    if !text(m, "parent_mandate_id").is_empty() {
        return Err(E::Unsupported("trust-delegated-mandate-not-ported"));
    }
    if !text(m, "revoked_at").is_empty() {
        return Ok("mandat révoqué".into());
    }
    if now < timestamp(text(m, "starts_at"))? {
        return Ok("mandat inactif".into());
    }
    if now >= timestamp(text(m, "expires_at"))? {
        return Ok("mandat expiré".into());
    }
    if number(m, "policy_revision") != st.revision {
        return Ok("révision de configuration du mandat périmée".into());
    }
    Ok(String::new())
}
/// Match the exact explicit protected target, retaining historical resource alias.
fn target<'a>(cmd: &'a Command) -> &'a str {
    if cmd.action_id.is_empty() {
        &cmd.resource
    } else {
        &cmd.action_id
    }
}
fn resource_matches(cmd: &Command, a: &Value) -> bool {
    cmd.action_id.is_empty() || cmd.resource.is_empty() || cmd.resource == text(a, "resource")
}
/// Exact historical action fingerprint; JSON field labels/order are contractual.
fn action_hash(cmd: &Command, requester: &str) -> Result<String, E> {
    #[derive(Serialize)]
    #[allow(non_snake_case)]
    struct Fields<'a> {
        MandateID: &'a str,
        Requester: &'a str,
        Resource: &'a str,
        Amount: i64,
        Currency: &'a str,
        Beneficiary: &'a str,
        IdempotencyKey: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        data_context: &'a Option<CompactJson>,
    }
    let base = digest(&Fields {
        MandateID: &cmd.mandate_id,
        Requester: requester,
        Resource: &cmd.resource,
        Amount: cmd.amount_minor,
        Currency: &cmd.currency,
        Beneficiary: &cmd.beneficiary,
        IdempotencyKey: &cmd.idempotency_key,
        data_context: &cmd.data_context,
    })?;
    if cmd.connector.is_none() {
        return Ok(base);
    }
    #[derive(Serialize)]
    #[allow(non_snake_case)]
    struct Selected<'a> {
        Base: String,
        Connector: &'a Option<ConnectorSelection>,
    }
    digest(&Selected {
        Base: base,
        Connector: &cmd.connector,
    })
}
/// Hash all protected mandate terms, including revocation/version and selection.
fn mandate_hash(m: &Value) -> Result<String, E> {
    #[derive(Serialize)]
    #[allow(non_snake_case)]
    struct Fields<'a> {
        ID: &'a str,
        Subject: &'a str,
        Delegator: &'a str,
        Actions: Vec<String>,
        Resource: &'a str,
        MaxAmount: i64,
        Currency: &'a str,
        Beneficiary: &'a str,
        Version: i64,
        PolicyRev: i64,
        Integrity: i64,
        StartsAt: &'a str,
        ExpiresAt: &'a str,
        RevokedAt: &'a str,
    }
    let actions: Vec<String> = serde_json::from_value(
        m.get("actions")
            .cloned()
            .ok_or(E::Integrity("trust-mandate-actions"))?,
    )?;
    let base = digest(&Fields {
        ID: text(m, "id"),
        Subject: text(m, "subject"),
        Delegator: text(m, "delegator"),
        Actions: actions,
        Resource: text(m, "resource"),
        MaxAmount: number(m, "max_amount_minor"),
        Currency: text(m, "currency"),
        Beneficiary: text(m, "beneficiary"),
        Version: number(m, "version"),
        PolicyRev: number(m, "policy_revision"),
        Integrity: number(m, "integrity"),
        StartsAt: text(m, "starts_at"),
        ExpiresAt: text(m, "expires_at"),
        RevokedAt: text(m, "revoked_at"),
    })?;
    if m.get("connector").is_none_or(Value::is_null) {
        return Ok(base);
    }
    #[derive(Serialize)]
    #[allow(non_snake_case)]
    struct Selected {
        Base: String,
        Connector: ConnectorSelection,
    }
    digest(&Selected {
        Base: base,
        Connector: serde_json::from_value(m["connector"].clone())?,
    })
}
/// Bound connector identifiers are fixed supported protocol capabilities. An
/// arbitrary provider/plugin/command/URL never becomes an executable effect.
fn connector(cmd: &Command) -> Result<&str, E> {
    let Some(c) = &cmd.connector else {
        return Ok("local-ledger-simulation");
    };
    if c.version != 1 {
        return Err(E::Invalid("version de connecteur invalide"));
    }
    match c.id.as_str() {
        "local-ledger-simulation" if c.access.is_none() => Ok(&c.id),
        "local-access-simulation"
            if c.access.as_ref().is_some_and(|a| {
                swarm_identity_contract::name(a.subject.as_bytes())
                    && swarm_identity_contract::name(a.role.as_bytes())
            }) =>
        {
            Ok(&c.id)
        }
        _ => Err(E::Invalid(
            "connecteur simulé inconnu ou paramètres invalides",
        )),
    }
}
/// Recover an exact prior response only for the same currently authenticated
/// authority. Existing history may be read even when later chain checks refuse.
fn replay(
    st: &State,
    op: &str,
    cmd: &mut Command,
    actor: &TrustIdentity,
) -> Result<Option<Vec<u8>>, E> {
    let Some(prior) = st.event_digests.get(&cmd.event_id) else {
        return Ok(None);
    };
    let current = command::fingerprint(op, cmd)?;
    if prior != &current {
        return Err(E::Conflict("event_conflict"));
    }
    if st.event_authorities.get(&cmd.event_id) != Some(&authority(actor)?) {
        return Err(E::Authorization);
    }
    let reply = st
        .event_replies
        .get(&cmd.event_id)
        .ok_or(E::Integrity("trust-reply-missing"))?;
    Ok(Some(reply.raw().as_bytes().to_vec()))
}
/// Append one official immutable audit object, linking exact original bytes,
/// effective policy and authenticated identity. No private credential is stored.
fn append(
    st: &mut State,
    op: &str,
    cmd: &Command,
    actor: &TrustIdentity,
    object: &str,
    content: &str,
    result: &str,
    reason: &str,
    raw: &[u8],
    now: OffsetDateTime,
    effect: Option<EffectProof>,
) -> Result<(), E> {
    let at = stamp(now)?;
    let original = hash(raw);
    let mandate = if !cmd.action_id.is_empty() {
        st.actions
            .get(&cmd.action_id)
            .map(|a| text(a, "mandate_id"))
            .unwrap_or("")
    } else {
        &cmd.mandate_id
    };
    let delegation = st
        .mandates
        .get(mandate)
        .filter(|m| text(m, "subject") == actor.subject)
        .map(|_| mandate.to_string());
    let context = IdentityContext {
        schema_version: 1,
        configuration_revision: st.revision,
        principal_id: actor.subject.clone(),
        principal_kind: "local".into(),
        issuer: Some("urn:swarm:local".into()),
        subject: Some(actor.subject.clone()),
        assurance: IdentityAssurance {
            method: "local_bearer".into(),
            acr: String::new(),
            provider_authenticated_at: None,
        },
        attributes: IdentityAttributes {
            role_mapping_digest: String::new(),
            roles: actor.roles.clone(),
            integrity: actor.integrity,
        },
        delegation_id: delegation,
        authenticated_at: Some(at.clone()),
        expires_at: None,
        token_digest: actor.token_hash.clone(),
    };
    let controls = CompactJson::new(
        std::str::from_utf8(&encoded(&st.controls()?)?)
            .map_err(|_| E::Integrity("trust-controls-encoding"))?,
    )?;
    let policy = ProofPolicyContext {
        schema_version: 1,
        revision: st.revision,
        policy: st.policy.clone(),
        controls,
    };
    let mut event = Evidence {
        policy_context: Some(policy),
        effect_receipt: effect,
        identity_context: Some(context),
        sequence: i64::try_from(st.evidence.len() + 1)
            .map_err(|_| E::Integrity("trust-sequence-representation"))?,
        previous_hash: st
            .evidence
            .last()
            .map(|e| e.hash.clone())
            .unwrap_or_default(),
        hash: String::new(),
        kind: op.into(),
        actor: actor.subject.clone(),
        integrity: actor.integrity,
        object_id: object.into(),
        content_digest: content.into(),
        original_digest: original.clone(),
        result: result.into(),
        reason: reason.into(),
        at,
    };
    event.hash = proof::evidence_hash(&event)?;
    st.evidence.push(event);
    st.originals.insert(original, STANDARD.encode(raw));
    Ok(())
}
/// Pure transition outcome. The owner must publish even a retained business
/// refusal, and must preserve this known outcome through release/output errors.
pub(crate) struct Transition {
    pub bytes: Vec<u8>,
    pub publication: Option<&'static str>,
}
/// Execute the historical core path under the owner's exclusive reservation.
/// Authentication, replay, proof and current policy precede any business edit.
pub(crate) fn execute(
    st: &mut State,
    op: &str,
    raw: &[u8],
    token: &str,
    now: OffsetDateTime,
) -> Result<Transition, E> {
    let mut cmd = command::decode(raw)?;
    st.require_supported()?;
    let actor = authenticated(st, token)?;
    if let Some(bytes) = replay(st, op, &mut cmd, &actor)? {
        return Ok(Transition {
            bytes,
            publication: Some("replayed"),
        });
    }
    proof::verify_chain(&proof::export(st)?)?;
    if cmd.data_context.is_some() || !cmd.parent_mandate_id.is_empty() {
        return Err(E::Unsupported("trust-advanced-admission-not-ported"));
    }
    let connector_id = connector(&cmd)?.to_string();
    if connector_id == "local-access-simulation"
        && (cmd.amount_minor != 0
            || cmd.max_amount_minor != 0
            || !cmd.currency.is_empty()
            || !cmd.beneficiary.is_empty())
    {
        return Err(E::Invalid("paramètres monétaires interdits pour un accès"));
    }
    let at = stamp(now)?;
    let before = st.evidence.len();
    let actions_before = st.actions.len();
    let mut effect = None;
    let value = match op {
        "direct-write" | "elevate" | "elevate-integrity" => {
            json!({"state":"denied","reason":"accès direct ou auto-élévation Biba refusé","next_action":"demander une admission autorisée"})
        }
        "mandate-create" => {
            if !role(&actor, "admin") || actor.integrity != 3 {
                json!({"state":"denied","reason":"administrateur I3 requis"})
            } else {
                if !swarm_identity_contract::name(cmd.mandate_id.as_bytes())
                    || !swarm_identity_contract::name(cmd.subject.as_bytes())
                    || !command::resource(&cmd.resource)
                    || cmd.actions.is_empty()
                    || connector_id == "local-ledger-simulation"
                        && (cmd.max_amount_minor <= 0
                            || cmd.currency.len() != 3
                            || cmd.currency.to_uppercase() != cmd.currency
                            || cmd.beneficiary.is_empty())
                {
                    return Err(E::Invalid("mandat invalide"));
                }
                if st.mandates.contains_key(&cmd.mandate_id) {
                    return Err(E::Conflict("conflict"));
                }
                let start = if cmd.starts_at.is_empty() {
                    now
                } else {
                    timestamp(&cmd.starts_at)?
                };
                let expiry = if !cmd.expires_at.is_empty() {
                    timestamp(&cmd.expires_at)?
                } else if st.policy.mandate_ttl_seconds == 0 {
                    timestamp("9999-01-01T00:00:00Z")?
                } else {
                    start
                        .checked_add(time::Duration::seconds(st.policy.mandate_ttl_seconds))
                        .ok_or(E::Invalid("expiration non représentable"))?
                };
                if expiry <= start {
                    return Err(E::Invalid("expiration de mandat invalide"));
                }
                let mut m = json!({"id":cmd.mandate_id,"subject":cmd.subject,"delegator":actor.subject,"actions":cmd.actions,"resource":cmd.resource,"max_amount_minor":cmd.max_amount_minor,"currency":cmd.currency,"beneficiary":cmd.beneficiary,"version":1,"policy_revision":st.revision,"integrity":2,"starts_at":stamp(start.replace_nanosecond(0).map_err(|_|E::Invalid("date invalide"))?)?,"expires_at":stamp(expiry.replace_nanosecond(0).map_err(|_|E::Invalid("date invalide"))?)?,"content_digest":"","original_digest":hash(raw)});
                if let Some(c) = &cmd.connector {
                    m["connector"] = serde_json::to_value(c)?;
                }
                m["content_digest"] = mandate_hash(&m)?.into();
                st.mandates.insert(cmd.mandate_id.clone(), m.clone());
                m
            }
        }
        "mandate-revoke" => {
            if !role(&actor, "admin") || actor.integrity != 3 {
                json!({"state":"denied","reason":"administrateur I3 requis"})
            } else if let Some(mut m) = st.mandates.get(&cmd.mandate_id).cloned() {
                if text(&m, "revoked_at").is_empty() {
                    m["revoked_at"] = at.clone().into();
                    m["version"] = number(&m, "version")
                        .checked_add(1)
                        .ok_or(E::Integrity("trust-version-representation"))?
                        .into();
                }
                m["content_digest"] = mandate_hash(&m)?.into();
                st.mandates.insert(cmd.mandate_id.clone(), m.clone());
                m
            } else {
                json!({"state":"denied","reason":"mandat absent"})
            }
        }
        "request" => {
            let mut reason = if !role(&actor, "requester") || actor.integrity < 1 {
                "demandeur authentifié requis".into()
            } else {
                active(st, &cmd.mandate_id, now)?
            };
            let content = action_hash(&cmd, &actor.subject)?;
            if reason.is_empty() {
                let m = &st.mandates[&cmd.mandate_id];
                if text(m, "subject") != actor.subject {
                    reason = "mauvais sujet".into();
                } else if text(m, "resource") != cmd.resource
                    || text(m, "currency") != cmd.currency
                    || text(m, "beneficiary") != cmd.beneficiary
                    || serde_json::to_value(&cmd.connector)?
                        != m.get("connector").cloned().unwrap_or(Value::Null)
                    || connector_id == "local-ledger-simulation"
                        && (cmd.amount_minor <= 0
                            || cmd.amount_minor > number(m, "max_amount_minor")
                            || !m["actions"]
                                .as_array()
                                .is_some_and(|v| v.iter().any(|a| a == "settle")))
                    || connector_id == "local-access-simulation"
                        && !m["actions"]
                            .as_array()
                            .is_some_and(|v| v.iter().any(|a| a == "grant-access"))
                {
                    reason = "paramètres hors mandat".into();
                } else if !swarm_identity_contract::name(cmd.idempotency_key.as_bytes()) {
                    reason = "clé d’idempotence invalide".into();
                }
            }
            if !reason.is_empty() {
                json!({"state":"denied","reason":reason})
            } else {
                if st.actions.values().any(|a| {
                    text(a, "idempotency_key") == cmd.idempotency_key
                        && text(a, "content_digest") != content
                }) {
                    return Err(E::Conflict("idempotency_conflict"));
                }
                let id = format!("act-{}", &content[..20]);
                if let Some(existing) = st.actions.get(&id) {
                    existing.clone()
                } else {
                    let mut action = json!({"id":id,"mandate_id":cmd.mandate_id,"requester":actor.subject,"resource":cmd.resource,"amount_minor":cmd.amount_minor,"currency":cmd.currency,"beneficiary":cmd.beneficiary,"idempotency_key":cmd.idempotency_key,"content_digest":content,"original_digest":hash(raw),"state":"pending_approval","next_action":"approbation séparée requise","created_at":at,"updated_at":at});
                    if let Some(c) = &cmd.connector {
                        action["connector"] = serde_json::to_value(c)?;
                    }
                    st.actions.insert(id, action.clone());
                    action
                }
            }
        }
        "approve" => {
            if let Some(mut a) = st.actions.get(target(&cmd)).cloned() {
                let mut reason = if !role(&actor, "approver")
                    || access(&actor, 2, BibaAccess::Write).is_err()
                    || st.policy.require_separation && actor.subject == text(&a, "requester")
                {
                    "séparation demandeur/approbateur ou intégrité insuffisante"
                } else if !resource_matches(&cmd, &a) {
                    "ressource explicite contradictoire"
                } else if text(&a, "state") != "pending_approval" {
                    "action n’attend plus d’approbation"
                } else if cmd.content_digest != text(&a, "content_digest")
                    || cmd.mandate_id != text(&a, "mandate_id")
                    || cmd.amount_minor != number(&a, "amount_minor")
                    || cmd.currency != text(&a, "currency")
                    || cmd.beneficiary != text(&a, "beneficiary")
                    || serde_json::to_value(&cmd.connector)?
                        != a.get("connector").cloned().unwrap_or(Value::Null)
                {
                    "accord non exact"
                } else {
                    ""
                }
                .to_string();
                if reason.is_empty() {
                    reason = active(st, text(&a, "mandate_id"), now)?;
                }
                if reason.is_empty() {
                    a["approver"] = actor.subject.clone().into();
                    a["state"] = "approved".into();
                    a["reason"] = "".into();
                    a["next_action"] = "engager le connecteur simulé".into();
                    a["updated_at"] = at.clone().into();
                    st.actions.insert(text(&a, "id").into(), a.clone());
                    a
                } else {
                    json!({"state":"denied","reason":reason})
                }
            } else {
                json!({"state":"denied","reason":"action absente"})
            }
        }
        "commit" | "reconcile" => {
            if !role(&actor, "effect") || access(&actor, 2, BibaAccess::Invoke).is_err() {
                json!({"state":"denied","reason":"service d’effet I2 authentifié requis"})
            } else if let Some(a) = st.actions.get(target(&cmd)).cloned() {
                if !resource_matches(&cmd, &a) {
                    json!({"state":"denied","reason":"ressource explicite contradictoire"})
                } else if op == "commit"
                    && matches!(text(&a, "state"), "committed" | "reconciled" | "unknown")
                {
                    a
                } else {
                    let (result, snapshot) = effect_transition(st, &a, &actor, &cmd, op, now)?;
                    effect = snapshot;
                    result
                }
            } else {
                json!({"state":"denied","reason":"action absente"})
            }
        }
        "read-object" | "read-original" => {
            let a = st
                .actions
                .get(&cmd.action_id)
                .ok_or(E::Invalid("action absente"))?;
            let label = if op == "read-original" { 1 } else { 2 };
            access(&actor, label, BibaAccess::Read)?;
            let value = if op == "read-object" {
                encoded(a)?
            } else {
                let bytes = STANDARD
                    .decode(
                        st.originals
                            .get(text(a, "original_digest"))
                            .ok_or(E::Integrity("trust-original-missing"))?,
                    )
                    .map_err(|_| E::Integrity("trust-original-base64"))?;
                bytes
            };
            return Ok(Transition {
                bytes: value,
                publication: None,
            });
        }
        _ => return Err(E::Unsupported("opération Actions de confiance non portée")),
    };
    let denied = text(&value, "state") == "denied";
    let mut value = value;
    if denied {
        value["business_mutated"] = false.into();
        value["next_action"] = "corriger le mandat, l’identité ou l’accord".into();
    }
    let object = if denied {
        if !cmd.action_id.is_empty() {
            cmd.action_id.clone()
        } else if !cmd.mandate_id.is_empty() {
            cmd.mandate_id.clone()
        } else {
            cmd.resource.clone()
        }
    } else {
        text(&value, "id").to_string()
    };
    let content = if denied {
        match op {
            "request" => action_hash(&cmd, &actor.subject)?,
            "direct-write" | "elevate" | "elevate-integrity" => digest(&cmd)?,
            "approve" | "commit" | "reconcile" => st
                .actions
                .get(target(&cmd))
                .map(|a| text(a, "content_digest").to_string())
                .unwrap_or_default(),
            _ => String::new(),
        }
    } else {
        text(&value, "content_digest").into()
    };
    let result = if denied {
        "denied"
    } else if op == "mandate-create" || op == "request" {
        "admitted"
    } else if op == "mandate-revoke" {
        "revoked"
    } else {
        text(&value, "state")
    };
    let reason = if denied {
        text(&value, "reason")
    } else if op == "mandate-create" {
        "endorsement Biba I3 vers nouvel objet I2"
    } else if op == "request" {
        "nouvel objet I2; original I1 conservé avec ses octets et son digest"
    } else if op == "approve" {
        "accord exact"
    } else if op == "reconcile" {
        "effet unique confirmé par reçu stable"
    } else {
        text(&value, "reason")
    };
    let record = denied
        || matches!(op, "mandate-create" | "mandate-revoke" | "approve")
        || op == "request" && st.actions.len() > actions_before
        || effect.is_some();
    if record {
        append(
            st, op, &cmd, &actor, &object, &content, result, reason, raw, now, effect,
        )?;
    }
    let bytes = encoded(&value)?;
    st.event_digests
        .insert(cmd.event_id.clone(), command::fingerprint(op, &mut cmd)?);
    st.event_authorities
        .insert(cmd.event_id.clone(), authority(&actor)?);
    st.event_replies.insert(
        cmd.event_id,
        CompactJson::new(
            std::str::from_utf8(&bytes).map_err(|_| E::Integrity("trust-reply-encoding"))?,
        )?,
    );
    if st.evidence.len() < before {
        return Err(E::Integrity("trust-evidence-regressed"));
    }
    Ok(Transition {
        bytes,
        publication: Some("recorded"),
    })
}
/// Revalidate protected action content against its mandate at the effect boundary.
/// No public command can pass an alternative action or native destination here.
fn bound_action(st: &State, a: &Value) -> Result<(), E> {
    let stored = st.actions.get(text(a, "id")).ok_or(E::Authorization)?;
    if digest(stored)? != digest(a)? {
        return Err(E::Authorization);
    }
    let m = st
        .mandates
        .get(text(a, "mandate_id"))
        .ok_or(E::Authorization)?;
    let mut cmd: Command = swarm_wire_json::decode(b"{}")?;
    cmd.mandate_id = text(a, "mandate_id").into();
    cmd.resource = text(a, "resource").into();
    cmd.amount_minor = number(a, "amount_minor");
    cmd.currency = text(a, "currency").into();
    cmd.beneficiary = text(a, "beneficiary").into();
    cmd.idempotency_key = text(a, "idempotency_key").into();
    cmd.connector = a
        .get("connector")
        .filter(|v| !v.is_null())
        .cloned()
        .map(serde_json::from_value)
        .transpose()?;
    let id = connector(&cmd)?;
    let operation = if id == "local-access-simulation" {
        "grant-access"
    } else {
        "settle"
    };
    if !m["actions"]
        .as_array()
        .is_some_and(|v| v.iter().any(|a| a == operation))
    {
        return Err(E::Authorization);
    }
    if action_hash(&cmd, text(a, "requester"))? != text(a, "content_digest")
        || mandate_hash(m)? != text(m, "content_digest")
        || text(m, "subject") != text(a, "requester")
        || text(m, "resource") != cmd.resource
        || text(m, "currency") != cmd.currency
        || text(m, "beneficiary") != cmd.beneficiary
        || a.get("connector") != m.get("connector")
        || id == "local-ledger-simulation"
            && (cmd.amount_minor <= 0 || cmd.amount_minor > number(m, "max_amount_minor"))
    {
        return Err(E::Authorization);
    }
    Ok(())
}
/// Produce an immutable snapshot from the single matching native effect.
fn snapshot(st: &State, a: &Value) -> Result<EffectProof, E> {
    let key = text(a, "idempotency_key");
    let receipt = st
        .connector_receipts
        .get(key)
        .ok_or(E::Integrity("trust-receipt-missing"))?;
    let id = a
        .get("connector")
        .map(|c| text(c, "id"))
        .unwrap_or("local-ledger-simulation");
    let raw = if id == "local-access-simulation" {
        let entries: Vec<_> = st
            .access_ledger
            .iter()
            .filter(|e| e.idempotency_key == key)
            .collect();
        if entries.len() != 1 {
            return Err(E::Integrity("trust-native-receipt-count"));
        }
        encoded(entries[0])?
    } else {
        let entries: Vec<_> = st
            .ledger
            .iter()
            .filter(|e| e.idempotency_key == key)
            .collect();
        if entries.len() != 1 {
            return Err(E::Integrity("trust-native-receipt-count"));
        }
        encoded(entries[0])?
    };
    let intent = receipt.intent.clone();
    let result = receipt.result.clone();
    Ok(EffectProof {
        schema_version: 1,
        connector_id: id.into(),
        legacy_receipt: intent.is_none() && result.is_none(),
        intent,
        result,
        native_result: CompactJson::new(
            std::str::from_utf8(&raw).map_err(|_| E::Integrity("trust-native-encoding"))?,
        )?,
    })
}
/// Fixed SPI boundary: current authority, action, policy and approvals are read
/// again before the sole native entry. Reconciliation only reads that entry.
fn effect_transition(
    st: &mut State,
    a: &Value,
    actor: &TrustIdentity,
    cmd: &Command,
    op: &str,
    now: OffsetDateTime,
) -> Result<(Value, Option<EffectProof>), E> {
    proof::verify_chain(&proof::export(st)?)?;
    st.require_supported()?;
    bound_action(st, a)?;
    if !role(actor, "effect") || access(actor, 2, BibaAccess::Invoke).is_err() {
        return Err(E::Authorization);
    }
    let key = text(a, "idempotency_key");
    let mut action = a.clone();
    let at = stamp(now)?;
    if op == "reconcile" {
        let Some(receipt) = st.connector_receipts.get(key) else {
            return Ok((
                json!({"state":"denied","reason":"reçu connecteur absent ou incompatible"}),
                None,
            ));
        };
        if receipt.action_id != text(a, "id") || receipt.content_digest != text(a, "content_digest")
        {
            return Ok((
                json!({"state":"denied","reason":"reçu connecteur absent ou incompatible"}),
                None,
            ));
        }
        let proof = snapshot(st, a)?;
        let native: Value = serde_json::from_str(proof.native_result.raw())?;
        if text(&native, "action_id") != text(a, "id")
            || text(&native, "content_digest") != text(a, "content_digest")
            || text(&native, "resource") != text(a, "resource")
            || text(&native, "idempotency_key") != key
            || native.get("simulation") != Some(&Value::Bool(true))
            || proof.connector_id == "local-access-simulation"
                && native.get("access") != a["connector"].get("access")
            || proof.connector_id == "local-ledger-simulation"
                && (number(&native, "amount_minor") != number(a, "amount_minor")
                    || text(&native, "currency") != text(a, "currency")
                    || text(&native, "beneficiary") != text(a, "beneficiary"))
        {
            return Ok((
                json!({"state":"denied","reason":"reçu connecteur absent ou incompatible"}),
                None,
            ));
        }
        if let Some(result) = &proof.result {
            let native_hash = if proof.connector_id == "local-ledger-simulation" {
                digest(&serde_json::from_value::<LedgerEntry>(native.clone())?)?
            } else {
                digest(&serde_json::from_value::<AccessEntry>(native.clone())?)?
            };
            if result.result_digest != native_hash {
                return Err(E::Integrity("trust-native-result-altered"));
            }
        }
        if text(a, "state") != "unknown" {
            return Ok((a.clone(), None));
        }
        let ready = timestamp(text(a, "updated_at"))?
            .checked_add(time::Duration::seconds(st.policy.unknown_reconcile_seconds))
            .ok_or(E::Integrity("trust-reconcile-time-representation"))?;
        if now < ready {
            return Ok((
                json!({"state":"denied","reason":"délai de réconciliation non écoulé"}),
                None,
            ));
        }
        action["state"] = "reconciled".into();
        action["reason"] = "".into();
        action["next_action"] = "consulter ou exporter les preuves".into();
        action["updated_at"] = at.clone().into();
        if let Some(result) = st
            .connector_receipts
            .get_mut(key)
            .and_then(|r| r.result.as_mut())
        {
            result.state = "reconciled".into();
            result.reconciliation_required = false;
            result.diagnostic_code.clear();
            result.observed_at = at;
        }
        st.actions.insert(text(a, "id").into(), action.clone());
        return Ok((action, Some(snapshot(st, a)?)));
    }
    let reason = active(st, text(a, "mandate_id"), now)?;
    if !reason.is_empty() {
        return Ok((json!({"state":"denied","reason":reason}), None));
    }
    if text(a, "state") != "approved" {
        return Ok((json!({"state":"denied","reason":"accord absent"}), None));
    }
    if st.connector_receipts.contains_key(key) {
        return Err(E::Conflict("idempotency_conflict"));
    }
    let approvals: Vec<String> = st
        .evidence
        .iter()
        .filter(|e| {
            e.kind == "approve"
                && e.object_id == text(a, "id")
                && e.content_digest == text(a, "content_digest")
                && e.actor == text(a, "approver")
                && e.result == "approved"
        })
        .map(|e| e.hash.clone())
        .collect();
    if approvals.is_empty()
        || text(a, "approver").is_empty()
        || st.policy.require_separation && text(a, "approver") == text(a, "requester")
    {
        return Err(E::Authorization);
    }
    let connector_id = a
        .get("connector")
        .map(|c| text(c, "id"))
        .unwrap_or("local-ledger-simulation");
    #[derive(Serialize)]
    struct Parameters<'a> {
        amount_minor: i64,
        currency: &'a str,
        beneficiary: &'a str,
    }
    let parameters = if connector_id == "local-access-simulation" {
        encoded(&serde_json::from_value::<AccessParameters>(
            a["connector"]["access"].clone(),
        )?)?
    } else {
        encoded(&Parameters {
            amount_minor: number(a, "amount_minor"),
            currency: text(a, "currency"),
            beneficiary: text(a, "beneficiary"),
        })?
    };
    let m = &st.mandates[text(a, "mandate_id")];
    let intent = EffectIntent {
        schema_version: 1,
        intent_id: format!("intent-{}", text(a, "content_digest")),
        action_digest: text(a, "content_digest").into(),
        mandate_digest: text(m, "content_digest").into(),
        approval_digests: approvals,
        connector_id: connector_id.into(),
        connector_version: 1,
        resource: text(a, "resource").into(),
        canonical_parameters: CompactJson::new(
            std::str::from_utf8(&parameters)
                .map_err(|_| E::Integrity("trust-parameters-encoding"))?,
        )?,
        idempotency_key: key.into(),
        deadline: text(m, "expires_at").into(),
    };
    let result_digest = if connector_id == "local-access-simulation" {
        let entry = AccessEntry {
            sequence: i64::try_from(st.access_ledger.len() + 1)
                .map_err(|_| E::Integrity("trust-sequence-representation"))?,
            action_id: text(a, "id").into(),
            content_digest: text(a, "content_digest").into(),
            idempotency_key: key.into(),
            resource: text(a, "resource").into(),
            access: serde_json::from_value(a["connector"]["access"].clone())?,
            committed_at: at.clone(),
            simulation: true,
        };
        let hash = digest(&entry)?;
        st.access_ledger.push(entry);
        hash
    } else {
        let entry = LedgerEntry {
            sequence: i64::try_from(st.ledger.len() + 1)
                .map_err(|_| E::Integrity("trust-sequence-representation"))?,
            action_id: text(a, "id").into(),
            content_digest: text(a, "content_digest").into(),
            idempotency_key: key.into(),
            amount_minor: number(a, "amount_minor"),
            currency: text(a, "currency").into(),
            beneficiary: text(a, "beneficiary").into(),
            resource: text(a, "resource").into(),
            committed_at: at.clone(),
            simulation: true,
        };
        let hash = digest(&entry)?;
        st.ledger.push(entry);
        hash
    };
    let state = if cmd.lose_response {
        "unknown"
    } else {
        "committed"
    };
    let result = EffectResult {
        schema_version: 1,
        intent_id: intent.intent_id.clone(),
        connector_receipt: format!("receipt-{}", digest(&intent)?),
        state: state.into(),
        committed_at: at.clone(),
        observed_at: at.clone(),
        result_digest,
        reconciliation_required: cmd.lose_response,
        diagnostic_code: if cmd.lose_response {
            "response_lost".into()
        } else {
            String::new()
        },
    };
    st.connector_receipts.insert(
        key.into(),
        Receipt {
            intent: Some(intent),
            result: Some(result),
            content_digest: text(a, "content_digest").into(),
            action_id: text(a, "id").into(),
            state: "committed".into(),
        },
    );
    action["state"] = state.into();
    action["reason"] = if cmd.lose_response {
        "réponse du connecteur perdue après engagement"
    } else {
        ""
    }
    .into();
    action["next_action"] = if cmd.lose_response {
        "réconcilier avant toute nouvelle tentative"
    } else {
        "consulter ou exporter les preuves"
    }
    .into();
    action["updated_at"] = at.into();
    st.actions.insert(text(a, "id").into(), action.clone());
    Ok((action, Some(snapshot(st, a)?)))
}
