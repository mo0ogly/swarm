//! Versioned public evidence contract. Public pins must be acquired separately;
//! signatures establish bytes, never external time, collection or WORM storage.
use super::{
    digest, encoded, hash,
    proof::{self, ProofExport},
    TrustError as E,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::de::{Error as _, MapAccess, SeqAccess, Visitor};

struct Unique;
impl<'de> Deserialize<'de> for Unique {
    /// Inspect each object before a map can erase duplicate keys. The stacked
    /// decoder supplies recursion protection; this pass stores no event payload.
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Unique;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("strict JSON")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Unique, A::Error> {
                let mut seen = std::collections::BTreeSet::new();
                while let Some(k) = a.next_key::<String>()? {
                    if !seen.insert(k) {
                        return Err(A::Error::custom("duplicate public evidence field"));
                    }
                    a.next_value::<Unique>()?;
                }
                Ok(Unique)
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Unique, A::Error> {
                while a.next_element::<Unique>()?.is_some() {}
                Ok(Unique)
            }
            fn visit_str<E: serde::de::Error>(self, _: &str) -> Result<Unique, E> {
                Ok(Unique)
            }
            fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<Unique, E> {
                Ok(Unique)
            }
            fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<Unique, E> {
                Ok(Unique)
            }
            fn visit_u64<E: serde::de::Error>(self, _: u64) -> Result<Unique, E> {
                Ok(Unique)
            }
            fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<Unique, E> {
                Ok(Unique)
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Unique, E> {
                Ok(Unique)
            }
        }
        d.deserialize_any(V)
    }
}
/// Decode only the new public-evidence contract: strict UTF8/Unicode and unique
/// keys at every depth, within the existing command transport bound. No Go
/// replacement normalization is applied; historical HMAC decoding is unchanged.
pub(crate) fn decode<T: serde::de::DeserializeOwned>(raw: &[u8]) -> Result<T, E> {
    if raw.len() > crate::COMMAND_INPUT_MAX_BYTES {
        return Err(E::Invalid("trust-public-input-size"));
    }
    fn parse<T: serde::de::DeserializeOwned>(raw: &[u8]) -> Result<T, E> {
        let mut d = serde_json::Deserializer::from_slice(raw);
        d.disable_recursion_limit();
        let result = T::deserialize(serde_stacker::Deserializer::new(&mut d))?;
        d.end()?;
        Ok(result)
    }
    let _: Unique = parse(raw)?;
    parse(raw)
}
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use time::OffsetDateTime;

/// Operator policy using the existing block name. All fields explicit; positive
/// TTL in seconds even when disabled. Omission is handled by configuration.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SigningPolicy {
    pub enabled: bool,
    pub key_ttl_seconds: i64,
}
impl SigningPolicy {
    /// Enforce the historical Go duration representation, never a business cap.
    pub fn validate(&self) -> Result<(), E> {
        if !(1..=9_223_372_036).contains(&self.key_ttl_seconds) {
            return Err(E::Invalid("trust-signing-key-ttl"));
        }
        Ok(())
    }
}
/// Persisted public key record, not private material or an acquired trust pin.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PublicKey {
    pub key_id: String,
    pub public_key: String,
    pub status: String,
    pub prepared_by: String,
    pub prepared_at: String,
    pub expires_at: String,
    pub configuration_revision: i64,
    pub activated_by: Option<String>,
    pub activated_at: Option<String>,
    pub activated_registry_revision: Option<u64>,
    pub retired_at: Option<String>,
    pub revoked_at: Option<String>,
}
/// Public registry. Private availability is intentionally a separate observation.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Registry {
    pub schema_version: u8,
    pub revision: u64,
    pub keys: BTreeMap<String, PublicKey>,
}
impl Registry {
    /// Construct an unconfigured public registry; creates no key or authority.
    pub fn empty() -> Self {
        Self {
            schema_version: 1,
            revision: 0,
            keys: BTreeMap::new(),
        }
    }
    /// Validate persisted names, chronology and exclusive active/candidate slots.
    pub fn validate(&self) -> Result<(), E> {
        if self.schema_version != 1 {
            return Err(E::Integrity("trust-public-registry-schema"));
        }
        if self.revision==0&&!self.keys.is_empty(){return Err(E::Integrity("trust-public-registry-empty-revision"));}
        let mut active = 0;
        let mut prepared = 0;
        for (id, key) in &self.keys {
            key.validate()?;
            if id != &key.key_id
                || key
                    .activated_registry_revision
                    .is_some_and(|v| v > self.revision)
            {
                return Err(E::Integrity("trust-public-key-id"));
            }
            active += u32::from(key.status == "active");
            prepared += u32::from(key.status == "prepared");
        }
        if active > 1 || prepared > 1 {
            return Err(E::Integrity("trust-public-registry-slots"));
        }
        Ok(())
    }
    /// Checked protocol revision; exhaustion refuses before any mutation.
    pub fn next(&self) -> Result<u64, E> {
        self.revision
            .checked_add(1)
            .ok_or(E::Integrity("trust-public-registry-revision"))
    }
}
/// UTC serialization of an observed server clock, not third-party time.
pub(crate) fn stamp(now: OffsetDateTime) -> Result<String, E> {
    now.to_offset(time::UtcOffset::UTC)
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|_| E::Invalid("trust-public-clock"))
}
fn date(raw: &str) -> Result<OffsetDateTime, E> {
    swarm_wire_json::go_rfc3339::parse(raw).map_err(|_| E::Integrity("trust-public-date"))
}
fn bytes32(raw: &str) -> Result<[u8; 32], E> {
    URL_SAFE_NO_PAD
        .decode(raw)
        .map_err(|_| E::Integrity("trust-public-key-encoding"))?
        .try_into()
        .map_err(|_| E::Integrity("trust-public-key-size"))
}

impl PublicKey {
    /// Check the independently pinned record before trusting its public bytes.
    /// Revocation rejects historical signatures; retired keys retain their period.
    pub fn validate(&self) -> Result<(), E> {
        let key = bytes32(&self.public_key)?;
        if self.public_key != URL_SAFE_NO_PAD.encode(key)
            || self.key_id != hash(&key)
            || self.configuration_revision < 1
            || !swarm_identity_contract::name(self.prepared_by.as_bytes())
        {
            return Err(E::Integrity("trust-public-key-record"));
        }
        let verifying =
            VerifyingKey::from_bytes(&key).map_err(|_| E::Integrity("trust-public-key-invalid"))?;
        if verifying.is_weak() {
            return Err(E::Integrity("trust-public-key-weak"));
        }
        let prepared = date(&self.prepared_at)?;
        let expires = date(&self.expires_at)?;
        if expires <= prepared {
            return Err(E::Integrity("trust-public-key-period"));
        }
        let activated = match (
            &self.activated_by,
            &self.activated_at,
            self.activated_registry_revision,
        ) {
            (None, None, None) => None,
            (Some(by), Some(at), Some(rev))
                if by != &self.prepared_by
                    && swarm_identity_contract::name(by.as_bytes())
                    && rev > 0 =>
            {
                let at = date(at)?;
                if at < prepared || at >= expires {
                    return Err(E::Integrity("trust-public-key-activation"));
                }
                Some(at)
            }
            _ => return Err(E::Integrity("trust-public-key-activation")),
        };
        if let Some(raw) = &self.retired_at {
            let retired = date(raw)?;
            if activated.is_none_or(|at| retired < at)
                || self
                    .revoked_at
                    .as_ref()
                    .is_some_and(|raw| date(raw).map_or(true, |revoked| revoked < retired))
            {
                return Err(E::Integrity("trust-public-retirement"));
            }
        }
        if let Some(raw) = &self.revoked_at {
            let revoked = date(raw)?;
            if revoked < prepared || activated.is_some_and(|at| revoked < at) {
                return Err(E::Integrity("trust-public-revocation"));
            }
        }
        match self.status.as_str() {
            "prepared"
                if activated.is_none()
                    && self.retired_at.is_none()
                    && self.revoked_at.is_none() => {}
            "active"
                if activated.is_some()
                    && self.retired_at.is_none()
                    && self.revoked_at.is_none() => {}
            "retired"
                if activated.is_some()
                    && self
                        .retired_at
                        .as_ref()
                        .is_some_and(|v| date(v).is_ok_and(|at| Some(at) >= activated))
                    && self.revoked_at.is_none() => {}
            "revoked"
                if self
                    .revoked_at
                    .as_ref()
                    .is_some_and(|v| date(v).is_ok_and(|at| at >= prepared)) => {}
            _ => return Err(E::Integrity("trust-public-key-status")),
        }
        Ok(())
    }
}
/// New Rust contract, distinct from historical HMAC schema1 and Go schema3.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PublicAnchor {
    pub schema_version: u8,
    pub domain: String,
    pub key_id: String,
    pub registry_revision: u64,
    pub configuration_revision: i64,
    pub sequence: i64,
    pub hash: String,
    pub export_digest: String,
    pub signed_at: String,
    pub signature: String,
}
const DOMAIN: &str = "swarm-rust-public-evidence-v1";
/// Fixed field order and wire codec define this contract's signed bytes. This
/// is not RFC8785; no signature field participates in its own preimage.
fn preimage(anchor: &PublicAnchor) -> Result<Vec<u8>, E> {
    #[derive(Serialize)]
    struct Signed<'a> {
        domain: &'a str,
        schema_version: u8,
        key_id: &'a str,
        registry_revision: u64,
        configuration_revision: i64,
        sequence: i64,
        hash: &'a str,
        export_digest: &'a str,
        signed_at: &'a str,
    }
    encoded(&Signed {
        domain: &anchor.domain,
        schema_version: anchor.schema_version,
        key_id: &anchor.key_id,
        registry_revision: anchor.registry_revision,
        configuration_revision: anchor.configuration_revision,
        sequence: anchor.sequence,
        hash: &anchor.hash,
        export_digest: &anchor.export_digest,
        signed_at: &anchor.signed_at,
    })
}
/// Sign an already verified complete export with the exact active key. The
/// storage owner must first verify current principal, policy, Root and revision.
pub(crate) fn sign(
    export: &ProofExport,
    key: &PublicKey,
    registry_revision: u64,
    private: &SigningKey,
    now: OffsetDateTime,
) -> Result<PublicAnchor, E> {
    proof::verify_chain(export)?;
    key.validate()?;
    if key.status != "active"
        || now >= date(&key.expires_at)?
        || key
            .activated_at
            .as_ref()
            .is_none_or(|v| date(v).map_or(true, |at| now < at))
        || URL_SAFE_NO_PAD.encode(private.verifying_key().to_bytes()) != key.public_key
    {
        return Err(E::Authorization);
    }
    let last = export
        .evidence
        .last()
        .ok_or(E::Invalid("aucune preuve à ancrer"))?;
    let mut anchor = PublicAnchor {
        schema_version: 1,
        domain: DOMAIN.into(),
        key_id: key.key_id.clone(),
        registry_revision,
        configuration_revision: key.configuration_revision,
        sequence: last.sequence,
        hash: last.hash.clone(),
        export_digest: digest(export)?,
        signed_at: stamp(now)?,
        signature: String::new(),
    };
    anchor.signature = URL_SAFE_NO_PAD.encode(private.sign(&preimage(&anchor)?).to_bytes());
    Ok(anchor)
}

/// An anchor and its separately acquired trusted public record. Neither becomes
/// trusted merely because it appears in the same verification input document.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Checkpoint {
    anchor: PublicAnchor,
    trusted_key: PublicKey,
}
fn checkpoint<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Checkpoint>, D::Error> {
    Checkpoint::deserialize(d).map(Some)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VerifyInput {
    export: ProofExport,
    anchor: PublicAnchor,
    trusted_key: PublicKey,
    #[serde(default, deserialize_with = "checkpoint")]
    trusted_checkpoint: Option<Checkpoint>,
}
/// Verify bytes, exact key/registry binding and historical signing period using
/// only the caller's trusted pin. This never consults a Root or current clock.
fn verify_anchor(export: &ProofExport, a: &PublicAnchor, key: &PublicKey) -> Result<(), E> {
    proof::verify_chain(export)?;
    key.validate()?;
    let last = export
        .evidence
        .last()
        .ok_or(E::Integrity("trust-public-empty-export"))?;
    let at = date(&a.signed_at)?;
    if a.schema_version != 1
        || a.domain != DOMAIN
        || a.key_id != key.key_id
        || a.configuration_revision != key.configuration_revision
        || a.registry_revision < key.activated_registry_revision.unwrap_or(u64::MAX)
        || a.sequence != last.sequence
        || a.hash != last.hash
        || a.export_digest != digest(export)?
        || !matches!(key.status.as_str(), "active" | "retired")
        || at >= date(&key.expires_at)?
        || key
            .activated_at
            .as_ref()
            .is_none_or(|v| date(v).map_or(true, |start| at < start))
        || key
            .retired_at
            .as_ref()
            .is_some_and(|v| date(v).map_or(true, |end| at > end))
    {
        return Err(E::Integrity("trust-public-anchor-binding"));
    }
    let public = VerifyingKey::from_bytes(&bytes32(&key.public_key)?)
        .map_err(|_| E::Integrity("trust-public-key-invalid"))?;
    let signature = URL_SAFE_NO_PAD
        .decode(&a.signature)
        .map_err(|_| E::Integrity("trust-public-signature-encoding"))?;
    if URL_SAFE_NO_PAD.encode(&signature) != a.signature {
        return Err(E::Integrity("trust-public-signature-canonical"));
    }
    let signature = Signature::from_slice(&signature)
        .map_err(|_| E::Integrity("trust-public-signature-size"))?;
    public
        .verify_strict(&preimage(a)?, &signature)
        .map_err(|_| E::Integrity("trust-public-signature"))
}
/// Pure public verification. A separate checkpoint authenticates an exact prefix
/// and rejects authentic rollback/divergence. Omission explicitly reports false;
/// null/incomplete checkpoints refuse. No external service or time is inferred.
pub fn verify(raw: &[u8]) -> Result<Vec<u8>, E> {
    let input: VerifyInput = decode(raw)?;
    verify_anchor(&input.export, &input.anchor, &input.trusted_key)?;
    let checkpoint_verified = if let Some(c) = &input.trusted_checkpoint {
        if c.anchor.sequence < 1 || c.anchor.sequence > input.anchor.sequence {
            return Err(E::Integrity("trust-public-checkpoint-rollback"));
        }
        let length = usize::try_from(c.anchor.sequence)
            .map_err(|_| E::Integrity("trust-public-prefix-size"))?;
        let mut prefix = input.export.clone();
        prefix.evidence.truncate(length);
        prefix
            .originals
            .retain(|key, _| prefix.evidence.iter().any(|e| &e.original_digest == key));
        verify_anchor(&prefix, &c.anchor, &c.trusted_key)?;
        true
    } else {
        false
    };
    encoded(
        &serde_json::json!({"schema_version":1,"verified":true,"signature_verified":true,"checkpoint_verified":checkpoint_verified,"sequence":input.anchor.sequence.to_string(),"hash":input.anchor.hash,"key_id":input.anchor.key_id,"simulation":true,"third_party_time_verified":false,"physical_immutability_verified":false}),
    )
}
