//! Exact private proposal persistence codec. Never a file writer or a substitute
//! for protected independent checkpoints/current account authority.
use super::*;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

/// Closed persistence representation. Exactly two events form this business
/// operation, not a generic batch policy. Root/principal are never learned here.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Saved {
    schema_version: i64,
    base: Head,
    changes: [String; 2],
}

/// Canonical base64url signed bytes. Preserve concrete decode errors without
/// formatting private verifier data; alternate encodings refuse structurally.
fn bytes(value: &str) -> Result<Vec<u8>, Error> {
    let decoded = URL_SAFE_NO_PAD.decode(value).map_err(Error::Encoding)?;
    if URL_SAFE_NO_PAD.encode(&decoded) != value {
        return Err(Error::Proposal);
    }
    Ok(decoded)
}
impl Source {
    /// Encode sensitive exact signed proposal bytes after checking this source's
    /// independent binding and re-verifying the full codec/chain. Does not write
    /// a file, touch SQLite or establish current/committed state. Caller must use
    /// protected atomic intent/checkpoint storage, never logs or public exports.
    /// No plaintext secret, new salt/signature or implicit grant is produced.
    pub fn encode_prepared(&self, ctx: &Context, p: &Prepared) -> Result<Vec<u8>, Error> {
        if p.expected != self.expected || p.key != self.key || p.project != self.project {
            return Err(Error::Binding);
        }
        let d = Saved {
            schema_version: 1,
            base: p.base.clone(),
            changes: [
                URL_SAFE_NO_PAD.encode(&p.raws[0]),
                URL_SAFE_NO_PAD.encode(&p.raws[1]),
            ],
        };
        let raw = wire::encode(&d).map_err(Error::Frame)?;
        let restored = self.restore_prepared(ctx, &raw)?;
        if restored.next != p.next || restored.record_digest != p.record_digest {
            return Err(Error::Proposal);
        }
        Ok(raw)
    }
    /// Restore only an exact canonical, correctly signed two-event personal
    /// proposal on this source's independently selected key/project/identity.
    /// Verify chain and exact revoke-bootstrap/enroll-account shape, then the
    /// canonical credential's installation/principal/generation and costs.
    /// It does not consult SQLite (works under writer contention), learn a root,
    /// adopt a checkpoint, prove human presence or consume bootstrap. A signature
    /// means the protected signing authority approved these exact bytes; it is
    /// not a fresh installation-secret check. enroll still validates current CAS,
    /// history and irreversible resource state before any publication.
    pub fn restore_prepared(&self, ctx: &Context, raw: &[u8]) -> Result<Prepared, Error> {
        let d: Saved = wire::decode(raw).map_err(Error::Frame)?;
        if d.schema_version != 1 || wire::encode(&d).map_err(Error::Frame)? != raw {
            return Err(Error::Proposal);
        }
        let raws = [bytes(&d.changes[0])?, bytes(&d.changes[1])?];
        let preview = self
            .journal
            .preview_batch(ctx, &d.base, &[&raws[0], &raws[1]])
            .map_err(Error::State)?;
        // The journal's actual verifier already enforces closed signed Change
        // fields, canonical encoding, project/key binding and both signatures.
        // This consumer only narrows those verified events to its own operation.
        let first: serde_json::Value = serde_json::from_slice(&raws[0]).map_err(Error::Frame)?;
        let second: serde_json::Value = serde_json::from_slice(&raws[1]).map_err(Error::Frame)?;
        if first["operation"] != "revoke"
            || first["id"] != BOOTSTRAP_ID
            || first["body"] != ""
            || second["operation"] != "enroll"
            || second["id"] != ACCOUNT_ID
        {
            return Err(Error::Proposal);
        }
        let body = bytes(second["body"].as_str().ok_or(Error::Proposal)?)?;
        let record = credentials::decode(&body).map_err(Error::Credential)?;
        if record.installation_digest != self.expected.installation_digest
            || record.principal != self.expected.principal
            || record.generation != self.expected.generation
        {
            return Err(Error::Binding);
        }
        let record_digest = credentials::fingerprint(&record).map_err(Error::Credential)?;
        Ok(Prepared {
            base: d.base,
            next: preview.predicted_head().clone(),
            record_digest,
            expected: self.expected.clone(),
            key: self.key,
            project: self.project,
            raws,
        })
    }
}
