//! Transactional draft creation/save/method selection, without plan adoption or agent execution.
use super::{
    finished,
    preparation_read::{cause, stored},
    RootStore, RootStoreError as E,
};
use crate::{Preparation, PreparationCommand, PreparationDocument, WorkEnvironment};
use rusqlite::{params, Connection, Transaction};
use sha2::{Digest, Sha256};

/// Encode a Go-compatible stored DTO without exposing document text on failure.
fn encode(value: &impl serde::Serialize) -> Result<Vec<u8>, E> {
    crate::encode_model(value).map_err(|e| cause("preparation-encode", e))
}
/// Execute one intent inside the caller's immediate transaction. Receipt replay
/// compares canonical intent, returns its historical snapshot and never rewrites
/// a document. Every staged row is read back before commit; triggers/corruption,
/// stale CAS and conflicting events refuse the entire transaction. Current
/// receipts must equal stored content before projection; historical receipts
/// have no corresponding current-content witness. No adoption or OS confinement.
fn apply(
    root: &RootStore,
    db: &Connection,
    r: &PreparationCommand,
    method: &str,
    policy: &crate::PreparationReadPolicy,
    at: &str,
    causes: &mut Vec<E>,
) -> Result<Preparation, E> {
    let raw = encode(r)?;
    let id = if r.action == "create" {
        format!(
            "prep-{}",
            &format!("{:x}", Sha256::digest(r.event_id.as_bytes()))[..24]
        )
    } else {
        r.preparation_id.clone()
    };
    let old:Option<(Vec<u8>,Vec<u8>)>=crate::agent_launch::query(db,"SELECT CAST(request AS BLOB),CAST(response AS BLOB) FROM preparation_commands WHERE preparation_id=?1 AND event_id=?2",params![id,r.event_id],|rows|Ok(rows.next()?.map(|row|Ok::<_,rusqlite::Error>((row.get(0)?,row.get(1)?))).transpose()?)).map_err(|e|E::Launch(Box::new(e)))?;
    if let Some((request, response)) = old {
        if request != raw {
            return Err(E::Refused("preparation-event-conflict"));
        }
        let mut p: Preparation =
            crate::work::decode(&response, crate::preparation_models::shape_preparation())
                .map_err(|e| cause("preparation-receipt-invalid", e))?;
        let current = stored(db, &id)?;
        if p.id != id || p.revision < 1 || p.revision > current.revision
            || (p.revision == current.revision && p != current) {
            return Err(E::Refused("preparation-receipt-identity-mismatch"));
        }
        p.receipt_historical = p.revision != current.revision;
        // This writer cannot grant approval, even when replaying an old receipt.
        root.preparation_project(&mut p, policy, causes)?;
        return Ok(p);
    }
    let mut p = if r.action == "create" {
        if !super::preparation_methods::known(method) {
            return Err(E::Refused("preparation-method-unavailable"));
        }
        if !r.work_id.is_empty() {
            crate::read_recorded_work(db, &r.work_id).map_err(E::Work)?;
        }
        let observed = super::preparation_methods::catalogue(&root.root, policy.method_max_bytes)?;
        let selected = observed
            .value
            .iter()
            .find(|m| m.id == method)
            .ok_or(E::Refused("preparation-method-unavailable"))?;
        let method_hash = if selected.available {
            selected.sha256.clone()
        } else {
            String::new()
        };
        causes.extend(observed.causes);
        Preparation {
            id: id.clone(),
            work_id: r.work_id.clone(),
            title: if crate::text::trim_space(&r.title).is_empty() {
                "Nouvelle préparation".into()
            } else {
                crate::text::trim_space(&r.title).into()
            },
            method: method.into(),
            method_hash,
            created_at: at.into(),
            documents: Some(Default::default()),
            ..Default::default()
        }
    } else {
        stored(db, &id)?
    };
    if r.expected_revision != Some(p.revision) {
        return Err(E::Refused("preparation-revision-conflict"));
    }
    let previous = p.revision;
    if r.action == "method" {
        if !super::preparation_methods::known(method) {
            return Err(E::Refused("preparation-method-unavailable"));
        }
        let observed = super::preparation_methods::catalogue(&root.root, policy.method_max_bytes)?;
        let selected = observed.value.iter().find(|m| m.id == method)
            .ok_or(E::Refused("preparation-method-unavailable"))?;
        let available = selected.available;
        if available {
            p.method = selected.id.clone();
            p.method_hash = selected.sha256.clone();
        }
        if !available {
            // Source diagnostics are causes of this domain refusal, not a
            // transaction cleanup failure. Preserve siblings without changing
            // CLI exit5 / HTTP400 into root-cleanup exit2 / HTTP500.
            let primary = observed.causes.into_iter().fold(
                E::Refused("preparation-method-unavailable"),
                |primary, cleanup| E::Cleanup {
                    primary: Some(Box::new(primary)), cleanup: Box::new(cleanup),
                },
            );
            return Err(cause("preparation-method-unavailable", primary));
        }
        causes.extend(observed.causes);
    }
    let mut document = None;
    if r.action == "save" || !r.text.is_empty() {
        let kind = if r.action == "create" {
            "besoin"
        } else {
            &r.document
        };
        let documents = p.documents.get_or_insert_with(Default::default);
        let revision = documents
            .get(kind)
            .map_or(0, |d| d.revision)
            .checked_add(1)
            .ok_or(E::Refused("preparation-revision-overflow"))?;
        if revision < 1 {
            return Err(E::Refused("preparation-document-revision-invalid"));
        }
        let doc = PreparationDocument {
            kind: kind.into(),
            text: r.text.clone(),
            revision,
            sha256: format!("{:x}", Sha256::digest(r.text.as_bytes())),
            at: at.into(),
            need_hash: if kind == "brief" {
                documents
                    .get("besoin")
                    .map(|d| d.sha256.clone())
                    .unwrap_or_default()
            } else {
                String::new()
            },
            brief_hash: if kind == "plan" {
                p.adopted_brief
                    .as_ref()
                    .map(|d| d.sha256.clone())
                    .unwrap_or_default()
            } else {
                String::new()
            },
            method_hash: if kind == "plan" {
                p.method_hash.clone()
            } else {
                String::new()
            },
            ..Default::default()
        };
        documents.insert(kind.into(), doc.clone());
        document = Some(doc);
    }
    p.revision = p
        .revision
        .checked_add(1)
        .ok_or(E::Refused("preparation-revision-overflow"))?;
    p.updated_at = at.into();
    p.verdict = None;
    p.plan_ready = false;
    p.receipt_historical = false;
    let body = encode(&p)?;
    let changed = if r.action == "create" {
        db.execute(
            "INSERT INTO preparations(id,work_id,revision,body) VALUES(?1,?2,?3,?4)",
            params![
                id,
                if p.work_id.is_empty() {
                    None
                } else {
                    Some(&p.work_id)
                },
                p.revision,
                body
            ],
        )
    } else {
        db.execute(
            "UPDATE preparations SET revision=?1,body=?2 WHERE id=?3 AND revision=?4",
            params![p.revision, body, id, previous],
        )
    }
    .map_err(|e| E::Launch(Box::new(crate::AgentLaunchError::from(e))))?;
    if changed != 1 || stored(db, &id)? != p {
        return Err(E::Refused("preparation-write-readback-mismatch"));
    }
    if let Some(doc) = document {
        let raw = encode(&doc)?;
        if db.execute("INSERT INTO preparation_documents(preparation_id,kind,revision,body) VALUES(?1,?2,?3,?4)",params![id,doc.kind,doc.revision,raw]).map_err(|e|E::Launch(Box::new(crate::AgentLaunchError::from(e))))?!=1 {return Err(E::Refused("preparation-history-write-mismatch"));}
        let back:Vec<u8>=crate::agent_launch::query(db,"SELECT CAST(body AS BLOB) FROM preparation_documents WHERE preparation_id=?1 AND kind=?2 AND revision=?3",params![id,doc.kind,doc.revision],|rows|Ok(rows.next()?.ok_or(crate::AgentLaunchError::Refused("preparation-history-missing"))?.get(0)?)).map_err(|e|E::Launch(Box::new(e)))?;
        if back != raw {
            return Err(E::Refused("preparation-history-write-mismatch"));
        }
    }
    if db.execute("INSERT INTO preparation_commands(preparation_id,event_id,request,response) VALUES(?1,?2,?3,?4)",params![id,r.event_id,raw,body]).map_err(|e|E::Launch(Box::new(crate::AgentLaunchError::from(e))))?!=1 {return Err(E::Refused("preparation-receipt-write-mismatch"));}
    let receipt:(Vec<u8>,Vec<u8>)=crate::agent_launch::query(db,"SELECT CAST(request AS BLOB),CAST(response AS BLOB) FROM preparation_commands WHERE preparation_id=?1 AND event_id=?2",params![id,r.event_id],|rows|{let row=rows.next()?.ok_or(crate::AgentLaunchError::Refused("preparation-receipt-missing"))?;Ok((row.get(0)?,row.get(1)?))}).map_err(|e|E::Launch(Box::new(e)))?;
    if receipt != (raw, body) {
        return Err(E::Refused("preparation-receipt-write-mismatch"));
    }
    Ok(p)
}

impl RootStore {
    /// Create/save/select a draft method or replay its immutable event receipt on this opened
    /// Root. Requires writable Root, complete enabled saved v42 policy, bounded
    /// valid request and exact revision. Stable settings and retained physical
    /// Root association guard an immediate transaction. Original rollback/commit,
    /// release and close causes survive. Uncertain commit reports
    /// preparation-commit-uncertain; a late failure after a known receipt carries
    /// preparation-state-preserved. Reread/replay the SAME event; never infer rollback. Unavailable method IO is retained in the observation sidecar; a draft
    /// remains writable on create with an empty method hash, without claiming freshness.
    /// Method selection requires an explicit available method, updates its hash,
    /// preserves documents/history and clears approval, even when selecting the
    /// same method. Missing/unsafe sources refuse without mutation.
    /// No provider, mission start, conversion or plan acceptance occurs.
    pub fn preparation_command<W: WorkEnvironment>(
        &self,
        raw: &[u8],
        environment: &mut W,
    ) -> Result<super::PreparationReadObservation<Preparation>, E> {
        if !self.writable {
            return Err(E::Refused("root-read-only"));
        }
        let r = crate::preparation_command::decode(raw)
            .map_err(|e| cause("preparation-request-invalid", e))?;
        let at = environment.timestamp().map_err(E::Runtime)?;
        let binding = self.bind_root()?;
        let mut known = None;
        let mut causes = Vec::new();
        let result = crate::agent_storage_retry::execute_borrowed(
            &self.db,
            &self.directory,
            false,
            |db, settings| {
                super::material_path::verify_material_path(self, &binding)?;
                if !binding.matches(self)? {
                    return Err(E::Refused("preparation-root-instance-changed"));
                }
                let policy = settings
                    .values
                    .preparation_write_policy()
                    .map_err(E::Settings)?;
                if !policy.enabled {
                    return Err(E::Refused("preparation-write-disabled"));
                }
                crate::preparation_command::validate(
                    &r,
                    policy.document_max_bytes,
                    policy.title_max_bytes,
                )
                .map_err(|e| cause("preparation-request-invalid", e))?;
                let read = settings
                    .values
                    .preparation_read_policy()
                    .map_err(E::Settings)?;
                let method = if r.action == "create" && r.method.is_empty() {
                    policy.default_method.as_str()
                } else if r.method == "audit_pdca" {
                    "audit-pdca"
                } else {
                    &r.method
                };
                let mut tx =
                    Transaction::new_unchecked(db, rusqlite::TransactionBehavior::Immediate)
                        .map_err(|e| E::Launch(Box::new(crate::AgentLaunchError::from(e))))?;
                tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
                let changed = apply(self, &tx, &r, method, &read, &at, &mut causes).and_then(|p| {
                    super::material_path::verify_material_path(self, &binding)?;
                    if !binding.matches(self)? {
                        return Err(E::Refused("preparation-root-instance-changed"));
                    }
                    Ok(p)
                });
                match changed {
                    Err(primary) => finished(Err(primary), tx.rollback().map_err(E::Sql)),
                    Ok(p) => match tx.commit() {
                        Ok(()) => {
                            known = Some((p.id.clone(), p.revision));
                            super::material_path::verify_material_path(self, &binding)?;
                            if !binding.matches(self)? {
                                return Err(E::Refused("preparation-root-instance-changed"));
                            }
                            Ok(p)
                        }
                        Err(e) => {
                            let mut primary = cause("preparation-commit-uncertain", e);
                            if !db.is_autocommit() {
                                primary = match finished::<()>(
                                    Err(primary),
                                    db.execute_batch("ROLLBACK").map_err(E::Sql),
                                ) {
                                    Err(e) => e,
                                    Ok(()) => unreachable!(),
                                };
                            }
                            Err(primary)
                        }
                    },
                }
            },
        );
        let result = finished(result, binding.close());
        let result = match result {
            Ok(value) => Ok(super::PreparationReadObservation { value, causes }),
            Err(primary) => causes
                .into_iter()
                .fold(Err(primary), |result, cause| finished(result, Err(cause))),
        };
        match (result, known) {
            (Err(primary), Some((id, revision))) => Err(E::PreparationRecorded {
                primary: Box::new(primary),
                id,
                revision,
            }),
            (result, _) => result,
        }
    }
}
