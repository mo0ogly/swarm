//! Process-owned signing material, bound to retained datastore descriptors.
//! No private key file/import, provider call or fallback to the HMAC key.
use super::{finished, RootBinding, RootStore, RootStoreError as R};
use crate::trust::{
    self,
    public_evidence::{self, PublicKey, Registry, SigningPolicy},
    state::State,
    TrustError as E,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::SigningKey;
use serde::Deserialize;
use serde_json::json;
use std::{
    collections::BTreeMap,
    sync::{Mutex, OnceLock},
};
use time::OffsetDateTime;
use zeroize::Zeroizing;

struct RuntimeKey {
    binding: RootBinding,
    key: SigningKey,
    scope: super::RootInstanceIdentity,
}
static KEYS: OnceLock<Mutex<BTreeMap<String, RuntimeKey>>> = OnceLock::new();
fn failure(e: E) -> R {
    R::Trust(Box::new(e))
}
/// Lock process-owned capabilities; poisoned state refuses without a fallback.
fn keys() -> Result<std::sync::MutexGuard<'static, BTreeMap<String, RuntimeKey>>, R> {
    KEYS.get_or_init(|| Mutex::new(BTreeMap::new()))
        .lock()
        .map_err(|_| R::Refused("trust-signing-memory-unavailable"))
}
/// Decode the persisted signing block and enforce its duration representation.
/// Missing/invalid configuration refuses; this never synthesizes enabled rights.
fn policy(st: &State) -> Result<SigningPolicy, R> {
    let controls = st.controls().map_err(failure)?;
    let raw = controls
        .get("evidence_signing")
        .ok_or(R::Refused("trust-signing-policy-missing"))?;
    let policy: SigningPolicy =
        swarm_wire_json::decode(raw.raw().as_bytes()).map_err(|e| failure(e.into()))?;
    policy.validate().map_err(failure)?;
    Ok(policy)
}
/// Observe key availability only after matching owned Root and DB descriptors.
/// Public lifecycle and private process availability are intentionally distinct.
pub(super) fn view(store: &RootStore, st: &State) -> Result<serde_json::Value, R> {
    prune(store, st)?;
    let keys = keys()?;
    let mut available = BTreeMap::new();
    if let Some(registry) = &st.public_evidence_registry {
        for id in registry.keys.keys() {
            available.insert(
                id.clone(),
                match keys.get(id) {
                    Some(k) => k.binding.matches(store)?,
                    None => false,
                },
            );
        }
    }
    Ok(
        json!({"private_available":available,"private_storage":"process-memory-only","restart_requires_new_key":true,"same_uid_isolation_qualified":false}),
    )
}
/// Drop capabilities made stale by another process, configuration change or
/// expiry. This changes process memory only; preserve explicit close failures.
fn prune(store: &RootStore, st: &State) -> Result<(), R> {
    let scope = store.instance()?;
    let now = OffsetDateTime::now_utc();
    let mut keys = keys()?;
    let mut remove = Vec::new();
    for (id, runtime) in keys.iter() {
        if runtime.scope != scope {
            continue;
        }
        let retain = if let Some(record) = st
            .public_evidence_registry
            .as_ref()
            .and_then(|r| r.keys.get(id))
        {
            let expires = swarm_wire_json::go_rfc3339::parse(&record.expires_at)
                .map_err(|_| failure(E::Integrity("trust-key-expiration")))?;
            matches!(record.status.as_str(), "active" | "prepared")
                && record.configuration_revision == st.revision
                && now < expires
        } else {
            false
        };
        if !retain {
            remove.push(id.clone());
        }
    }
    for id in remove {
        if let Some(runtime) = keys.remove(&id) {
            drop(runtime.key);
            runtime
                .binding
                .close()
                .map_err(|primary| R::TrustPublished {
                    primary: Box::new(primary),
                    publication: "signing-material-removed",
                })?;
        }
    }
    Ok(())
}
/// Remove one owned private capability and explicitly close its retained handles.
/// Foreign-instance material is left untouched. Local removal occurs even if
/// close fails; callers must retain that observable loss.
fn remove(store: &RootStore, id: &str) -> Result<(), R> {
    let mut keys = keys()?;
    if let Some(k) = keys.get(id) {
        if !k.binding.matches(store)? {
            return Ok(());
        }
    }
    if let Some(k) = keys.remove(id) {
        drop(k.key);
        k.binding.close().map_err(|primary| R::TrustPublished {
            primary: Box::new(primary),
            publication: "signing-material-removed",
        })?;
    }
    Ok(())
}
#[derive(Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct Change {
    schema_version: u8,
    event_id: String,
    expected_registry_revision: u64,
    reason: String,
    #[serde(default)]
    key_id: String,
}

/// Execute an attributed key decision under the caller's exclusive reservation.
/// Current I3/admin authority is required even locally; replay returns current
/// public lifecycle, not a claim that lost private material was reconstructed.
pub(super) fn change(
    store: &RootStore,
    st: &mut State,
    op: &str,
    raw: &[u8],
    token: crate::trust::authentication::Source<'_>,
    now: OffsetDateTime,
) -> Result<Vec<u8>, R> {
    let c: Change = public_evidence::decode(raw).map_err(failure)?;
    if c.schema_version != 1
        || !swarm_identity_contract::name(c.event_id.as_bytes())
        || c.reason.trim().len() < 8
        || c.reason.len() > 2000
    {
        return Err(failure(E::Invalid("trust-key-command")));
    }
    let actor = trust::operations::authenticated(st, token).map_err(failure)?;
    if actor.integrity != 3 || !actor.roles.iter().any(|r| r == "admin") {
        return Err(failure(E::Authorization));
    }
    let fingerprint=trust::digest(&json!({"domain":"trust-public-key-change-v1","operation":op,"command":c,"actor":actor.subject})).map_err(failure)?;
    prune(store, st)?;
    if let Some(prior) = st.event_digests.get(&c.event_id) {
        if prior != &fingerprint {
            return Err(failure(E::Conflict("event_conflict")));
        }
        return trust::encoded(&json!({"schema_version":1,"replayed":true,"registry":st.public_evidence_registry,"runtime":view(store,st)?})).map_err(failure);
    }
    let mut registry = st
        .public_evidence_registry
        .clone()
        .unwrap_or_else(Registry::empty);
    if registry.revision != c.expected_registry_revision {
        return Err(failure(E::Conflict("revision_conflict")));
    }
    let next = registry.next().map_err(failure)?;
    let at = public_evidence::stamp(now).map_err(failure)?;
    let signing = policy(st)?;
    if op != "evidence-key-revoke" {
        st.require_supported().map_err(failure)?;
        if !signing.enabled {
            return Err(failure(E::Authorization));
        }
    }
    let id = match op {
        "evidence-key-prepare" => {
            prepare(store, st, &mut registry, &c, &actor.subject, now, &signing)?
        }
        "evidence-key-activate" => activate(store, st, &mut registry, &c, &actor.subject, now)?,
        "evidence-key-revoke" => {
            let key = registry
                .keys
                .get_mut(&c.key_id)
                .ok_or_else(|| failure(E::Invalid("trust-key-absent")))?;
            if key.status == "revoked" {
                return Err(failure(E::Conflict("trust-key-already-revoked")));
            }
            remove(store, &c.key_id)?;
            key.status = "revoked".into();
            key.revoked_at = Some(at);
            c.key_id.clone()
        }
        _ => return Err(failure(E::Unsupported("trust-key-operation"))),
    };
    let completed = (|| {
        registry.revision = next;
        registry.validate().map_err(failure)?;
        let status = registry
            .keys
            .get(&id)
            .ok_or_else(|| failure(E::Integrity("trust-key-record-missing")))?
            .status
            .clone();
        let content = trust::digest(&registry).map_err(failure)?;
        st.public_evidence_registry = Some(registry);
        let mut cmd: trust::command::Command =
            swarm_wire_json::decode(b"{}").map_err(|e| failure(e.into()))?;
        cmd.event_id = c.event_id.clone();
        trust::operations::append(
            st, op, &cmd, &actor, &id, &content, &status, &c.reason, raw, now, None,
        )
        .map_err(failure)?;
        let bytes=trust::encoded(&json!({"schema_version":1,"replayed":false,"key_id":id,"registry":st.public_evidence_registry,"runtime":view(store,st)?})).map_err(failure)?;
        st.event_digests.insert(c.event_id.clone(), fingerprint);
        st.event_replies.insert(
            c.event_id,
            trust::json::CompactJson::new(
                std::str::from_utf8(&bytes)
                    .map_err(|_| failure(E::Integrity("trust-key-reply-utf8")))?,
            )
            .map_err(|e| failure(e.into()))?,
        );
        Ok(bytes)
    })();
    match completed {
        Ok(bytes) => Ok(bytes),
        Err(primary) if op == "evidence-key-prepare" => finished(Err(primary), remove(store, &id)),
        Err(primary) => Err(R::TrustPublished {
            primary: Box::new(primary),
            publication: "signing-capability-changed",
        }),
    }
}
/// Generate once into zeroizing memory after enforcing process nondumpability.
/// Retained handles prevent inode reuse; lost memory is never regenerated by ID.
fn prepare(
    store: &RootStore,
    st: &State,
    registry: &mut Registry,
    c: &Change,
    actor: &str,
    now: OffsetDateTime,
    p: &SigningPolicy,
) -> Result<String, R> {
    if !c.key_id.is_empty() || registry.keys.values().any(|k| k.status == "prepared") {
        return Err(failure(E::Conflict("trust-key-candidate-exists")));
    }
    nix::sys::prctl::set_dumpable(false).map_err(|e| {
        R::Launch(Box::new(crate::AgentLaunchError::caused(
            "trust-signing-nondumpable",
            e,
        )))
    })?;
    if nix::sys::prctl::get_dumpable().map_err(|e| {
        R::Launch(Box::new(crate::AgentLaunchError::caused(
            "trust-signing-nondumpable-read",
            e,
        )))
    })? {
        return Err(R::Refused("trust-signing-dumpable"));
    }
    let mut seed = Zeroizing::new([0u8; 32]);
    getrandom::getrandom(seed.as_mut()).map_err(|e| {
        R::Launch(Box::new(crate::AgentLaunchError::caused(
            "trust-signing-entropy",
            e,
        )))
    })?;
    let key = SigningKey::from_bytes(&seed);
    let public = key.verifying_key().to_bytes();
    let id = trust::hash(&public);
    if registry.keys.contains_key(&id) {
        return Err(failure(E::Conflict("trust-key-collision")));
    }
    let expires = now
        .checked_add(time::Duration::seconds(p.key_ttl_seconds))
        .ok_or_else(|| failure(E::Invalid("trust-key-expiration")))?;
    let record = PublicKey {
        key_id: id.clone(),
        public_key: URL_SAFE_NO_PAD.encode(public),
        status: "prepared".into(),
        prepared_by: actor.into(),
        prepared_at: public_evidence::stamp(now).map_err(failure)?,
        expires_at: public_evidence::stamp(expires).map_err(failure)?,
        configuration_revision: st.revision,
        activated_by: None,
        activated_at: None,
        activated_registry_revision: None,
        retired_at: None,
        revoked_at: None,
    };
    record.validate().map_err(failure)?;
    let mut keys = keys()?;
    let scope = store.instance()?;
    let binding = store.bind_root()?;
    if keys.contains_key(&id) {
        return finished(
            Err(failure(E::Conflict("trust-key-collision"))),
            binding.close(),
        );
    }
    keys.insert(
        id.clone(),
        RuntimeKey {
            binding,
            key,
            scope,
        },
    );
    registry.keys.insert(id.clone(), record);
    Ok(id)
}

/// Activate an exact fresh candidate by a distinct current administrator. Rotate
/// the prior active key to a historical record and drop its private capability.
fn activate(
    store: &RootStore,
    st: &State,
    registry: &mut Registry,
    c: &Change,
    actor: &str,
    now: OffsetDateTime,
) -> Result<String, R> {
    let candidate = registry
        .keys
        .get(&c.key_id)
        .ok_or_else(|| failure(E::Invalid("trust-key-absent")))?;
    if candidate.status != "prepared"
        || candidate.configuration_revision != st.revision
        || candidate.prepared_by == actor
    {
        return Err(failure(E::Authorization));
    }
    if st.role_separation.as_ref().is_some_and(|p| {
        p.enabled && p.principals.get(actor) == p.principals.get(&candidate.prepared_by)
    }) {
        return Err(failure(E::Authorization));
    }
    if !st.identities.iter().any(|i| {
        i.subject == candidate.prepared_by
            && !i.disabled
            && i.integrity == 3
            && i.roles.iter().any(|r| r == "admin")
    }) {
        return Err(failure(E::Authorization));
    }
    let expires = swarm_wire_json::go_rfc3339::parse(&candidate.expires_at)
        .map_err(|_| failure(E::Integrity("trust-key-expiration")))?;
    let prepared = swarm_wire_json::go_rfc3339::parse(&candidate.prepared_at)
        .map_err(|_| failure(E::Integrity("trust-key-prepared-at")))?;
    if now < prepared {
        return Err(R::Refused("trust-signing-clock-regressed"));
    }
    if now >= expires {
        return Err(R::Refused("trust-signing-key-expired"));
    }
    {
        let keys = keys()?;
        let key = keys
            .get(&c.key_id)
            .ok_or(R::Refused("trust-signing-private-unavailable"))?;
        if !key.binding.matches(store)? {
            return Err(R::Refused("trust-signing-root-mismatch"));
        }
    }
    let at = public_evidence::stamp(now).map_err(failure)?;
    let next = registry.next().map_err(failure)?;
    let old: Vec<_> = registry
        .keys
        .iter()
        .filter(|(_, k)| k.status == "active")
        .map(|(id, _)| id.clone())
        .collect();
    for id in old {
        remove(store, &id)?;
        let key = registry
            .keys
            .get_mut(&id)
            .ok_or(R::Refused("trust-key-registry-changed"))?;
        key.status = "retired".into();
        key.retired_at = Some(at.clone());
    }
    let key = registry
        .keys
        .get_mut(&c.key_id)
        .ok_or(R::Refused("trust-key-registry-changed"))?;
    key.status = "active".into();
    key.activated_by = Some(actor.into());
    key.activated_at = Some(at);
    key.activated_registry_revision = Some(next);
    Ok(c.key_id.clone())
}
/// Sign the current full export only under fresh I3/admin, enabled policy, exact
/// configuration and process-owned private material. Never persist the seed.
pub(super) fn anchor(
    store: &RootStore,
    st: &State,
    raw: &[u8],
    token: crate::trust::authentication::Source<'_>,
    now: OffsetDateTime,
) -> Result<Vec<u8>, R> {
    let body: serde_json::Value = swarm_wire_json::decode(raw).map_err(|e| failure(e.into()))?;
    if !body.as_object().is_some_and(|o| o.is_empty()) {
        return Err(failure(E::Invalid("trust-public-anchor-body")));
    }
    st.require_supported().map_err(failure)?;
    let actor = trust::operations::authenticated(st, token).map_err(failure)?;
    if actor.integrity != 3 || !actor.roles.iter().any(|r| r == "admin") || !policy(st)?.enabled {
        return Err(failure(E::Authorization));
    }
    prune(store, st)?;
    let registry = st
        .public_evidence_registry
        .as_ref()
        .ok_or(R::Refused("trust-signing-key-absent"))?;
    let key = registry
        .keys
        .values()
        .find(|k| k.status == "active" && k.configuration_revision == st.revision)
        .ok_or(R::Refused("trust-signing-key-inactive"))?;
    let keys = keys()?;
    let runtime = keys
        .get(&key.key_id)
        .ok_or(R::Refused("trust-signing-private-unavailable"))?;
    if !runtime.binding.matches(store)? {
        return Err(R::Refused("trust-signing-root-mismatch"));
    }
    let export = trust::proof::export(st).map_err(failure)?;
    let anchor = public_evidence::sign(&export, key, registry.revision, &runtime.key, now)
        .map_err(failure)?;
    let export_bytes = trust::encoded(&export).map_err(failure)?;
    let anchor_bytes = trust::encoded(&anchor).map_err(failure)?;
    trust::encoded(&json!({"schema_version":1,"kind":"trust-public-anchor-bundle-v1","anchor":anchor,"export_base64":URL_SAFE_NO_PAD.encode(export_bytes),"anchor_base64":URL_SAFE_NO_PAD.encode(anchor_bytes),"simulation":true})).map_err(failure)
}
