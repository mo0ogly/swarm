//! Transactional preparation edits and explicit new-work conversion; never agent execution.
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
/// Stable saved configuration and source observations selected by the caller's
/// settings reservation; this context grants no new Root or execution authority.
struct WriteContext<'a> {
    root: &'a RootStore,
    policy: &'a crate::PreparationReadPolicy,
    settings: &'a crate::EngineSettingsValues,
    at: &'a str,
    document: &'a crate::EngineSettingsDocument,
}
/// Resolve an available capability hash, retaining original safe source causes.
/// Unknown/unavailable sources refuse before mutation; other catalogue warnings
/// remain observation causes. No source path is supplied by an intention.
fn current_method_hash(root: &RootStore, method: &str, policy: &crate::PreparationReadPolicy, causes: &mut Vec<E>) -> Result<String,E> {
    let method=if method=="audit_pdca" {"audit-pdca"} else {method};
    if !super::preparation_methods::known(method) { return Err(E::Refused("preparation-method-unavailable")); }
    let observed=super::preparation_methods::catalogue(&root.root,policy.method_max_bytes)?;
    let selected=observed.value.iter().find(|m|m.id==method).ok_or(E::Refused("preparation-method-unavailable"))?;
    if !selected.available {
        let primary=observed.causes.into_iter().fold(E::Refused("preparation-method-unavailable"),|primary,cleanup|E::Cleanup{primary:Some(Box::new(primary)),cleanup:Box::new(cleanup)});
        return Err(cause("preparation-method-unavailable",primary));
    }
    let hash=selected.sha256.clone();causes.extend(observed.causes);Ok(hash)
}
/// Execute one intent inside the caller's immediate transaction. Receipt replay
/// compares schema-projected canonical intent (including retained Go/old Rust
/// representations without rewriting their bytes), returns its historical snapshot and never rewrites
/// a document. Every staged row is read back before commit; triggers/corruption,
/// stale CAS and conflicting events refuse the entire transaction. Current
/// receipts must equal stored content before projection; historical receipts
/// have no corresponding current-content witness. Brief adoption copies only
/// the current nonblank document matching the requested hash and need hash. It
/// appends no history. Answers append one plan revision while retaining provenance
/// and invalidating verdicts. create-missions delegates new or bound-work materialization
/// to the same transaction; release-plan explicitly releases the snapshot at a
/// Work CAS, preserving draft verdicts. Neither operation calls a provider or
/// grants organization configuration or OS confinement. budget validates the shared
/// finite USD envelope, stamps current local actor/time and preserves documents,
/// verdicts, reservations and consumption; it never sets a Work envelope.
fn apply<W:WorkEnvironment>(
    context: &WriteContext<'_>,
    db: &Connection,
    r: &PreparationCommand,
    method: &str,
    causes: &mut Vec<E>,
    environment: &mut W,
) -> Result<Preparation, E> {
    let root=context.root;let policy=context.policy;let at=context.at;
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
            // Original Go and earlier Rust receipts have different object order
            // and omitted zero members. Project both through the actual wire
            // schema before comparison, without rewriting their original bytes.
            let recorded=crate::preparation_command::decode(&request)
                .map_err(|e|cause("preparation-receipt-invalid",e))?;
            if encode(&recorded)?!=raw {return Err(E::Refused("preparation-event-conflict"));}
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
    let original_request=r;
    let proposal=if r.action=="use-proposal"{
        let method=current_method_hash(root,&p.method,policy,causes)?;
        Some(super::preparation_proposal::edit(db,&p,r,&method,context.settings)?)
    }else{None};
    let r=proposal.as_ref().unwrap_or(r);
    let previous = p.revision;
    if r.action == "budget" {
        let raw=serde_json::to_vec(r.budget.as_ref().ok_or(E::Refused("preparation-request-invalid"))?)
            .map_err(|e|cause("preparation-budget-invalid",e))?;
        let mut budget:crate::Budget=crate::work::decode_mode(&raw,crate::preparation_models::shape_budget(),true)
            .map_err(|e|cause("preparation-budget-invalid",e))?;
        crate::validate_budget(&budget).map_err(|e|cause("preparation-budget-invalid",e))?;
        budget.actor=environment.operator().map_err(E::Runtime)?;
        budget.updated=at.into();p.budget=Some(Box::new(budget));
    }
    if matches!(r.action.as_str(),"source-add"|"source-remove") {
        super::preparation_attachments::edit(root,&mut p,r,context.settings,at)?;
    }
    if r.action == "method" {
        p.method_hash=current_method_hash(root,method,policy,causes)?;
        p.method=method.into();
    }
    if r.action == "validate-plan" {
        let limits=context.settings.preparation_plan_policy().map_err(E::Settings)?;
        let task=context.settings.mutation_limits().map_err(E::Settings)?.task;
        let hash=current_method_hash(root,&p.method,policy,causes)?;
        if hash!=p.method_hash { return Err(E::Refused("preparation-stale-method")); }
        let docs=p.documents.as_ref();
        let brief=p.adopted_brief.as_deref().ok_or(E::Refused("preparation-stale-brief"))?;
        let brief_hash=docs.and_then(|d|d.get("brief")).map_or("",|d|d.sha256.as_str());
        let need_hash=docs.and_then(|d|d.get("besoin")).map_or("",|d|d.sha256.as_str());
        if brief.sha256!=brief_hash || brief.need_hash!=need_hash { return Err(E::Refused("preparation-stale-brief")); }
        let plan=docs.and_then(|d|d.get("plan")).ok_or(E::Refused("preparation-stale-plan"))?;
        if plan.brief_hash!=brief.sha256 || plan.method_hash!=hash { return Err(E::Refused("preparation-stale-plan")); }
        if r.sha256.is_empty() || r.sha256!=plan.sha256 { return Err(E::Refused("preparation-stale-document")); }
        if !crate::preparation_models::verdict_document_digests_match(&p) { return Err(E::Refused("preparation-document-digest-mismatch")); }
        super::preparation_plan::validate(plan.text.as_bytes(),&p.method,&limits,&task)?;
        p.verdict=Some(Box::new(crate::PreparationVerdict { plan_hash:plan.sha256.clone(),brief_hash:brief.sha256.clone(),method_hash:hash,at:at.into() }));
    }
    if r.action == "adopt-brief" {
        let documents = p.documents.as_ref();
        let brief = documents.and_then(|d| d.get("brief"))
            .ok_or(E::Refused("preparation-stale-document"))?;
        if crate::text::trim_space(&brief.text).is_empty()
            || r.sha256.is_empty() || r.sha256 != brief.sha256 {
            return Err(E::Refused("preparation-stale-document"));
        }
        let need_hash = documents.and_then(|d| d.get("besoin"))
            .map_or("", |d| d.sha256.as_str());
        if brief.need_hash != need_hash {
            return Err(E::Refused("preparation-stale-brief"));
        }
        if !crate::preparation_models::document_digest_matches(brief)
            || documents.and_then(|d|d.get("besoin")).is_some_and(|d|!crate::preparation_models::document_digest_matches(d)) {
            return Err(E::Refused("preparation-document-digest-mismatch"));
        }
        p.adopted_brief = Some(Box::new(brief.clone()));
    }
    let materialization=match r.action.as_str(){
        "revise-missions"=>Some(super::preparation_revision::revise(root,db,&mut p,r,context.document,at,environment,causes)?),
        "create-missions"=>Some(super::preparation_conversion::materialize(root,db,&mut p,r,context.document,at,environment,causes)?),
        "release-plan"=>Some(super::preparation_release::release(db,&mut p,r,at)?),
        "configure-team"=>Some(super::preparation_organization::configure(root,db,&mut p,r,context.document,at,environment,causes)?),
        _=>None,
    };
    let mut document = None;
    if r.action == "answer-questions" {
        let original=p.documents.as_ref().and_then(|docs|docs.get("plan"))
            .ok_or(E::Refused("preparation-stale-document"))?;
        if r.sha256.is_empty() || r.sha256!=original.sha256 {
            return Err(E::Refused("preparation-stale-document"));
        }
        if !crate::preparation_models::document_digest_matches(original) {
            return Err(E::Refused("preparation-document-digest-mismatch"));
        }
        let limits=context.settings.preparation_plan_policy().map_err(E::Settings)?;
        let task=context.settings.mutation_limits().map_err(E::Settings)?.task;
        let edit=context.settings.preparation_write_policy().map_err(E::Settings)?;
        let text=super::preparation_plan::answer_questions(original.text.as_bytes(),
            r.decisions.as_deref().ok_or(E::Refused("preparation-request-invalid"))?,
            &p.method,&limits,&task,edit.document_max_bytes)?;
        // Do not derive new source hashes: answers never rebase an obsolete plan.
        let mut doc=original.clone();
        doc.revision=doc.revision.checked_add(1).filter(|n|*n>0)
            .ok_or(E::Refused("preparation-revision-overflow"))?;
        doc.sha256=format!("{:x}",Sha256::digest(text.as_bytes()));
        doc.text=text;doc.at=at.into();
        p.documents.as_mut().ok_or(E::Refused("preparation-stale-document"))?
            .insert("plan".into(),doc.clone());
        document=Some(doc);
    }
    if r.action == "save" || !r.text.is_empty() {
        let kind = if r.action == "create" {
            "besoin"
        } else {
            &r.document
        };
        // Verify only provenance consumed by this save. Replacement writes can
        // repair the damaged target without requiring direct database changes.
        let source=if kind=="brief" {p.documents.as_ref().and_then(|d|d.get("besoin"))}
            else if kind=="plan" {p.adopted_brief.as_deref()} else {None};
        if source.is_some_and(|d|!crate::preparation_models::document_digest_matches(d)) {
            return Err(E::Refused("preparation-document-digest-mismatch"));
        }
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
    if original_request.action=="use-proposal"{
        let doc=document.as_mut().ok_or(E::Refused("preparation-proposal-document-missing"))?;
        doc.source_turn=original_request.turn_id.clone();
        p.documents.as_mut().ok_or(E::Refused("preparation-proposal-document-missing"))?.insert(doc.kind.clone(),doc.clone());
        if original_request.adopt_brief{p.adopted_brief=Some(Box::new(doc.clone()));}
    }
    p.revision = p
        .revision
        .checked_add(1)
        .ok_or(E::Refused("preparation-revision-overflow"))?;
    p.updated_at = at.into();
    if !matches!(r.action.as_str(),"validate-plan"|"create-missions"|"revise-missions"|"release-plan"|"configure-team"|"budget") { p.verdict = None; }
    p.plan_ready = false;
    if matches!(r.action.as_str(),"validate-plan"|"create-missions"|"revise-missions"|"release-plan"|"configure-team"|"budget") { root.preparation_project(&mut p,policy,causes)?; }
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
            "UPDATE preparations SET revision=?1,body=?2,work_id=?5 WHERE id=?3 AND revision=?4",
            params![p.revision, body, id, previous,if p.work_id.is_empty(){None}else{Some(&p.work_id)}],
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
    if let Some(witness)=materialization {witness.verify(db)?;}
    if stored(db,&id)?!=p {return Err(E::Refused("preparation-write-readback-mismatch"));}
    Ok(p)
}

impl RootStore {
    /// Create/edit/adopt/answer a draft, materialize a new or bound-work plan or replay
    /// its immutable event receipt on this opened
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
    /// Brief adoption requires its exact hash and unchanged need, copies the
    /// document without rewriting history and clears any verdict. It attests no
    /// named human identity. Missing/blank/mismatched brief and changed need
    /// refuse with stale-document/stale-brief and no mutation; replay precedes
    /// freshness checks. validate-plan requires saved v43 caps, current method/brief/plan hashes and
    /// a strict answered ActionPlan. It records a structural verdict, not named
    /// human acceptance or execution permission. Reads project hash freshness;
    /// later policy changes require new validation before any future conversion.
    /// Consumed document hashes must match stored text. Mismatches refuse with
    /// preparation-document-digest-mismatch; public save/adopt operations may
    /// replace damaged targets, without implicit repair or datastore authenticity.
    /// create-missions requires a fresh exact plan and no existing conversion.
    /// Zero target revision creates a Work; a positive revision binds an existing
    /// target by CAS, preserving old tasks, plans, settings and history while
    /// appending new tasks under unreleased locks in the same transaction. release-plan explicitly clears those locks for
    /// exactly the conversion task subset at the current Work revision, preserving edited task
    /// contracts, draft documents and the verdict. The conversion snapshot hash
    /// selects the task IDs, never a newer draft. It can unblock an authorized
    /// conductor but performs no synchronous dispatch or budget/autonomy change.
    /// configure-team requires the original locked task contracts and a current
    /// verdict. Its explicit CLI/human configuration preserves locks/budgets and
    /// snapshots reviewer defaults; existing history, teams and release refuse.
    /// revise-missions revalidates the changed draft at both revisions, preserves
    /// histories and consumption, invalidates impacted results, relocks only the
    /// new conversion and pauses cockpit/planning in this transaction. Active
    /// agents/reviews/decisions or incompatible controls/dependencies refuse.
    /// It grants no start; another release is explicit. Automatic initial team
    /// modes remain unavailable. All failures roll back jointly
    /// or retain explicit unknown/known publication semantics as described above.
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
                let changed = apply(&WriteContext{root:self,policy:&read,settings:&settings.values,at:&at,document:settings}, &tx, &r, method, &mut causes,environment).and_then(|p| {
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
