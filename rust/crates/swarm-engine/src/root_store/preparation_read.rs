//! Stored preparation documents and current method observations.
//! These reads never adopt a plan, authorize a launch or mutate preparation data.
use super::{RootStore, RootStoreError as E};
use crate::{Preparation, PreparationDocument};
use std::{error::Error, fmt};

/// A safe preparation read/write boundary and its original codec/SQL/IO cause.
/// Display never renders source documents, supplied IDs or private paths.
pub struct PreparationReadError {
    code: &'static str,
    cause: Box<dyn Error + Send + Sync>,
}
impl PreparationReadError {
    /// Static diagnostic family; neither absence nor permission follows from it.
    pub fn code(&self) -> &'static str { self.code }
}
impl fmt::Display for PreparationReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(self.code) }
}
impl fmt::Debug for PreparationReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { fmt::Display::fmt(self, f) }
}
impl Error for PreparationReadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> { Some(self.cause.as_ref()) }
}
/// Retain a typed primary failure; Root's existing Cleanup wrapper owns siblings.
pub(crate) fn cause(code: &'static str, cause: impl Error + Send + Sync + 'static) -> E {
    E::Preparation(Box::new(PreparationReadError { code, cause: Box::new(cause) }))
}
/// Data deliberately separated from method-unavailability causes. Consumers must
/// select the DTO and preserve original causes on later output/close failure.
/// Neither the DTO nor the sidecar is an execution or conversion permission.
pub struct PreparationReadObservation<T> {
    pub value: T,
    pub causes: Vec<E>,
}

/// Go list envelope. An explicit null limit is the saved unlimited policy;
/// zero rows is a presentation choice, never deletion or lack of preparations.
#[derive(serde::Serialize)]
pub struct PreparationList {
    pub preparations: Vec<Preparation>,
    pub limit: Option<usize>,
}
/// Read a retained SQLite text/blob body without coercing another SQL type.
/// Finalization belongs to the caller; allocation/SQL causes propagate safely.
fn body(row: &rusqlite::Row<'_>, index: usize) -> Result<Vec<u8>, E> {
    match row.get_ref(index).map_err(E::Sql)? {
        rusqlite::types::ValueRef::Blob(raw) | rusqlite::types::ValueRef::Text(raw) => Ok(raw.to_vec()),
        other => Err(E::Sql(rusqlite::Error::InvalidColumnType(index, "preparation body".into(), other.data_type()))),
    }
}
/// Decode stored Go fields using the shared ordered duplicate/null/type rules,
/// then bind the projection to actual row identity/revision/Work association.
/// Malformed records never become a successful absent preparation.
fn preparation(row: &rusqlite::Row<'_>) -> Result<Preparation, E> {
    let id: String = row.get(0).map_err(E::Sql)?;
    let work: Option<String> = row.get(1).map_err(E::Sql)?;
    let revision: i64 = row.get(2).map_err(E::Sql)?;
    let value: Preparation = crate::work::decode(&body(row, 3)?, crate::preparation_models::shape_preparation())
        .map_err(|e| cause("preparation-document-invalid", e))?;
    if value.id != id || value.work_id != work.unwrap_or_default() || value.revision != revision {
        return Err(E::Refused("preparation-row-identity-mismatch"));
    }
    Ok(value)
}
/// Select one known row with parameterized SQL; no default ID or creation.
pub(super) fn stored(db: &rusqlite::Connection, id: &str) -> Result<Preparation, E> {
    let mut statement = db.prepare("SELECT id,work_id,revision,body FROM preparations WHERE id=?1").map_err(E::Sql)?;
    let result = (|| {
        let mut rows = statement.query([id]).map_err(E::Sql)?;
        let row = rows.next().map_err(E::Sql)?.ok_or(E::Refused("preparation-not-found"))?;
        preparation(row)
    })();
    super::finished(result, statement.finalize().map_err(E::Sql))
}
/// Convert an already validated nullable row bound to SQLite's unlimited -1.
/// This is SQLite vocabulary/representation, never a second operator ceiling.
fn sql_limit(limit: Option<usize>) -> Result<i64, E> {
    limit.map(i64::try_from).transpose().map(|value| value.unwrap_or(-1))
        .map_err(|e| cause("preparation-limit-representation", e))
}
/// Only Go's freshness branch with a verdict, adopted brief and current need
/// consumes method sources. Other rows do not acquire an irrelevant IO failure.
fn needs_method(p: &Preparation) -> bool {
    p.verdict.is_some() && !p.receipt_historical && p.adopted_brief.as_deref().is_some_and(|brief|
        brief.need_hash == p.documents.as_ref().and_then(|d| d.get("besoin")).map(|d| d.sha256.as_str()).unwrap_or(""))
}
#[cfg(target_os = "linux")]
impl RootStore {
    /// Guard read-only SQL/material association, saved policy and explicit rollback.
    /// Retain unavailable-method causes even if an outer path/close check fails;
    /// no snapshot atomicity, policy adoption or execution authority is inferred.
    fn preparation_observed<T>(&self, read: impl FnOnce(&rusqlite::Connection, &crate::PreparationReadPolicy, &mut Vec<E>) -> Result<T, E>) -> Result<PreparationReadObservation<T>, E> {
        let mut causes = Vec::new();
        let result = super::material_path::with_material_read(self, || {
            let policy = self.read_settings()?.values.preparation_read_policy().map_err(E::Settings)?;
            let mut tx = self.db.unchecked_transaction().map_err(E::Sql)?;
            tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
            let result = read(&tx, &policy, &mut causes);
            super::finished(result, tx.rollback().map_err(E::Sql))
        });
        match result {
            Ok(value) => Ok(PreparationReadObservation { value, causes }),
            Err(mut primary) => {
                for cause in causes { primary = E::Cleanup { primary: Some(Box::new(primary)), cleanup: Box::new(cause) }; }
                Err(primary)
            }
        }
    }
    /// Project Go plan_ready from stored hashes and observed current method bytes.
    /// Unavailable methods retain false plus original causes; nothing is accepted.
    pub(super) fn preparation_project(&self, p: &mut Preparation, policy: &crate::PreparationReadPolicy, causes: &mut Vec<E>) -> Result<(), E> {
        if !needs_method(p) { crate::preparation_models::apply_freshness(p, None); return Ok(()); }
        let mut observed = super::preparation_methods::catalogue(&self.root, policy.method_max_bytes)?;
        causes.append(&mut observed.causes);
        let id = if p.method == "audit_pdca" { "audit-pdca" } else { &p.method };
        let hash = observed.value.iter().find(|method| method.id == id && method.available).map(|method| method.sha256.as_str());
        crate::preparation_models::apply_freshness(p, hash); Ok(())
    }
    /// Read a finite supported method catalogue and actual source hashes under
    /// current saved policy. Unavailable entries retain safe reason and causes;
    /// they do not select fallback or become conversion/launch authorization.
    pub fn preparation_methods(&self) -> Result<PreparationReadObservation<Vec<crate::PreparationMethod>>, E> {
        self.preparation_observed(|_, policy, causes| {
            let mut observed = super::preparation_methods::catalogue(&self.root, policy.method_max_bytes)?;
            causes.append(&mut observed.causes); Ok(observed.value)
        })
    }
    /// List newest recorded rows by SQLite rowid, with the persisted row bound
    /// and freshly projected plan_ready. Empty/unlimited/zero remain distinct;
    /// typed SQL/codec/material/close errors refuse, no history is changed.
    pub fn preparation_list(&self) -> Result<PreparationReadObservation<PreparationList>, E> {
        self.preparation_observed(|db, policy, causes| {
            let mut statement = db.prepare("SELECT id,work_id,revision,body FROM preparations ORDER BY rowid DESC LIMIT ?1").map_err(E::Sql)?;
            let result = (|| {
                let mut rows = statement.query([sql_limit(policy.list_limit)?]).map_err(E::Sql)?;
                let mut preparations = Vec::new();
                while let Some(row) = rows.next().map_err(E::Sql)? {
                    let mut p = preparation(row)?; self.preparation_project(&mut p, policy, causes)?; preparations.push(p);
                }
                Ok(PreparationList { preparations, limit: policy.list_limit })
            })();
            super::finished(result, statement.finalize().map_err(E::Sql))
        })
    }
    /// Read one retained preparation, recomputing plan_ready from actual method
    /// observations. ID is a query parameter, never a path or default Work.
    /// Missing/malformed/misbound data and all enclosing IO/close errors refuse.
    pub fn preparation_show(&self, id: &str) -> Result<PreparationReadObservation<Preparation>, E> {
        if !crate::valid_preparation_key(id) { return Err(E::Refused("preparation-id-invalid")); }
        self.preparation_observed(|db, policy, causes| {
            let mut p = stored(db, id)?; self.preparation_project(&mut p, policy, causes)?; Ok(p)
        })
    }
    /// Read earlier document snapshots newest revision first. Positive before
    /// is exclusive; absence uses Go's i64 max sentinel, not a product cutoff.
    /// The saved history row limit is explicit. No visit, write, adoption or launch.
    pub fn preparation_history(&self, id: &str, kind: &str, before: Option<i64>) -> Result<PreparationReadObservation<Vec<PreparationDocument>>, E> {
        if !crate::valid_preparation_key(id) { return Err(E::Refused("preparation-id-invalid")); }
        if !matches!(kind, "besoin" | "brief" | "plan") { return Err(E::Refused("preparation-document-kind-invalid")); }
        if before.is_some_and(|value| value < 1) { return Err(E::Refused("preparation-revision-invalid")); }
        self.preparation_observed(|db, policy, causes| {
            let mut p = stored(db, id)?; self.preparation_project(&mut p, policy, causes)?;
            let mut statement = db.prepare("SELECT kind,revision,body FROM preparation_documents WHERE preparation_id=?1 AND kind=?2 AND revision<?3 ORDER BY revision DESC LIMIT ?4").map_err(E::Sql)?;
            let result = (|| {
                let mut rows = statement.query(rusqlite::params![id, kind, before.unwrap_or(i64::MAX), sql_limit(policy.history_limit)?]).map_err(E::Sql)?;
                let mut documents = Vec::new();
                while let Some(row) = rows.next().map_err(E::Sql)? {
                    let document: PreparationDocument = crate::work::decode(&body(row, 2)?, crate::preparation_models::shape_preparation_document())
                        .map_err(|e| cause("preparation-history-document-invalid", e))?;
                    let stored_kind: String = row.get(0).map_err(E::Sql)?;
                    let revision: i64 = row.get(1).map_err(E::Sql)?;
                    if document.kind != stored_kind || document.revision != revision { return Err(E::Refused("preparation-history-row-mismatch")); }
                    documents.push(document);
                }
                Ok(documents)
            })();
            super::finished(result, statement.finalize().map_err(E::Sql))
        })
    }
}
