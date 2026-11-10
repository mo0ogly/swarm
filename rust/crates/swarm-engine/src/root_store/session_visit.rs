//! Session consultation bookkeeping, never Work mutation or acceptance.
use super::{
    finished, material_path, native_transaction, RootStore, RootStoreError as E, SqlEffect,
};
use crate::{AuxiliaryEffect, AuxiliaryEffectState, SessionVisit, VisitMarkInput, WorkEnvironment};
use rusqlite::{Connection, OptionalExtension};

/// Native readback plus Go visit fields. Exact decimal revisions survive browser
/// JSON parsing; exists=false is a missing bookmark, not revision-zero consent.
#[derive(Clone, serde::Serialize)]
pub struct SessionVisitView {
    pub schema_version: u8,
    pub work_id: String,
    pub work_revision: String,
    pub exists: bool,
    pub visit: SessionVisit,
    pub exact_visit_revision: String,
}
/// A verified bookkeeping result and its typed owner observation. The effect is
/// retained in-process through late errors, not reconstructed from response JSON.
/// saved never means Work/task acceptance or a new write rather than known state.
#[derive(serde::Serialize)]
pub struct SessionVisitMutation {
    pub saved: bool,
    pub state: SessionVisitView,
    #[serde(skip)]
    pub effect: AuxiliaryEffect,
}
impl SessionVisitView {
    /// Trusted observation only, never proof of new bytes, human identity or retry
    /// rights. Uncertain is an attempted receipt that must be read back.
    fn effect(&self, state: AuxiliaryEffectState) -> AuxiliaryEffect {
        AuxiliaryEffect::SessionVisit {
            state,
            work_id: self.work_id.clone(),
            visit: self.visit.clone(),
        }
    }
}
/// Read one SQL-text field with the established Go per-invalid-byte replacement.
/// NULL/other storage classes remain errors, not empty successful bookmarks.
fn text(row: &rusqlite::Row<'_>, index: usize) -> Result<String, rusqlite::Error> {
    match row.get_ref(index)? {
        rusqlite::types::ValueRef::Text(raw) | rusqlite::types::ValueRef::Blob(raw) => {
            Ok(crate::text::go_utf8(raw))
        }
        other => Err(rusqlite::Error::InvalidColumnType(
            index,
            "visit time".into(),
            other.data_type(),
        )),
    }
}
/// Read exact operator-keyed row within the caller's SQL snapshot. Missing is
/// distinct from an invalid/unreadable row. Negative recorded revisions refuse
/// before a successful view; statement finalize is checked on every outcome.
fn read(db: &Connection, work: &str, operator: &str) -> Result<Option<SessionVisit>, E> {
    let mut statement = db
        .prepare("SELECT revision,at FROM session_visits WHERE work_id=?1 AND operator=?2")
        .map_err(E::Sql)?;
    let result = statement
        .query_row([work, operator], |r| {
            Ok(SessionVisit {
                revision: r.get(0)?,
                at: text(r, 1)?,
                operator: operator.into(),
            })
        })
        .optional()
        .map_err(E::Sql)
        .and_then(|visit| {
            if visit.as_ref().is_some_and(|value| value.revision < 0) {
                return Err(E::Refused("visit-recorded-revision-invalid"));
            }
            Ok(visit)
        });
    finished(result, statement.finalize().map_err(E::Sql))
}
/// Materialize native exact metadata from one actual Work revision and visit.
fn view(
    work: &str,
    revision: i64,
    visit: Option<SessionVisit>,
    operator: &str,
) -> SessionVisitView {
    let exists = visit.is_some();
    let visit = visit.unwrap_or_else(|| SessionVisit {
        revision: 0,
        at: String::new(),
        operator: operator.into(),
    });
    SessionVisitView {
        schema_version: 1,
        work_id: work.into(),
        work_revision: revision.to_string(),
        exists,
        exact_visit_revision: visit.revision.to_string(),
        visit,
    }
}
/// Keep an observed auxiliary receipt beside an actual later failure. It is not
/// Work/config/Agent publication; caller still owes public readback/recovery.
fn preserved(error: E, effect: AuxiliaryEffect) -> E {
    E::AuxiliaryRecorded {
        primary: Box::new(error),
        effect: Box::new(effect),
    }
}
impl RootStore {
    /// Read this server/operator namespace and Work in one readonly SQL snapshot
    /// under material association/settings reservation. Operator comes from the
    /// trusted environment, not request JSON. Host/UID is not verified human
    /// identity. No visit, Work, history, usage, grant or provider is changed;
    /// original read/statement/rollback/post-check/release errors propagate.
    pub fn session_visit<Ev: WorkEnvironment>(
        &self,
        work: &str,
        environment: &mut Ev,
    ) -> Result<SessionVisitView, E> {
        if !swarm_identity_contract::name(work.as_bytes()) {
            return Err(E::Refused("visit-work-invalid"));
        }
        let operator = environment.operator().map_err(E::Runtime)?;
        if operator.is_empty() {
            return Err(E::Refused("visit-operator-unavailable"));
        }
        material_path::with_material_read(self, || {
            let mut tx = self.db.unchecked_transaction().map_err(E::Sql)?;
            tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
            let result = (|| {
                let document = crate::read_recorded_work(&tx, work).map_err(E::Work)?;
                Ok(view(
                    work,
                    document.revision,
                    read(&tx, work, &operator)?,
                    &operator,
                ))
            })();
            finished(result, tx.rollback().map_err(E::Sql))
        })
    }
    /// Mark current Work revision for this local operator only. Optional exact
    /// native expected_revision refuses stale UI state; absent preserves Go
    /// current-at-call semantics. IMMEDIATE SQL serializes Work read/visit upsert;
    /// one affected row and exact readback are required before commit. No Work,
    /// task, budget, acceptance, provider or process statement is issued here.
    /// Uncertain commit/rollback keeps an attempted receipt; known committed state
    /// survives late association/release failures. Neither Err nor receipt means
    /// rollback, new bytes, durability, peer receipt or retry permission. Caller
    /// must preserve the effect through Root close/encoding/output/flush errors.
    pub fn mark_session_visit<Ev: WorkEnvironment>(
        &self,
        work: &str,
        input: &VisitMarkInput,
        environment: &mut Ev,
    ) -> Result<SessionVisitMutation, E> {
        if !self.writable {
            return Err(E::Refused("root-read-only"));
        }
        if !swarm_identity_contract::name(work.as_bytes()) {
            return Err(E::Refused("visit-work-invalid"));
        }
        let expected = input
            .expected_revision()
            .map_err(|e| E::Launch(Box::new(e)))?;
        let operator = environment.operator().map_err(E::Runtime)?;
        if operator.is_empty() {
            return Err(E::Refused("visit-operator-unavailable"));
        }
        let at = environment.timestamp().map_err(E::Runtime)?;
        time::OffsetDateTime::parse(&at, &time::format_description::well_known::Rfc3339)
            .map_err(|e| E::Runtime(crate::RuntimeError::new(crate::RuntimeOperation::Clock, e)))?;
        let mut recorded = None;
        let result = material_path::with_material_operation(self, || {
            let mut attempted = None;
            let result = native_transaction::apply_observed(self, |tx| {
                let document = crate::read_recorded_work(tx, work).map_err(E::Work)?;
                if expected.is_some_and(|revision| revision != document.revision) {
                    return Err(E::Refused("visit-revision-conflict"));
                }
                let visit = SessionVisit {
                    revision: document.revision,
                    at: at.clone(),
                    operator: operator.clone(),
                };
                let state = view(work, document.revision, Some(visit.clone()), &operator);
                attempted = Some(state.clone());
                let count=tx.execute("INSERT INTO session_visits(work_id,operator,revision,at) VALUES(?1,?2,?3,?4) ON CONFLICT(work_id,operator) DO UPDATE SET revision=excluded.revision,at=excluded.at",rusqlite::params![work,operator,visit.revision,at]).map_err(E::Sql)?;
                if count != 1 {
                    return Err(E::Refused("visit-write-not-recorded"));
                }
                if read(tx, work, &operator)?.as_ref() != Some(&visit) {
                    return Err(E::Refused("visit-write-readback-mismatch"));
                }
                Ok(native_transaction::Change::Mutate(state))
            });
            match result {
                Ok(state) => {
                    recorded = Some(state.effect(AuxiliaryEffectState::Recorded));
                    Ok(state)
                }
                Err(failure) => {
                    let uncertain = failure.effect() == SqlEffect::Uncertain;
                    let error = failure.into_root();
                    Err(match (uncertain, attempted) {
                        (true, Some(state)) => {
                            preserved(error, state.effect(AuxiliaryEffectState::Uncertain))
                        }
                        _ => error,
                    })
                }
            }
        });
        match (result, recorded) {
            (Err(error), Some(effect)) => Err(preserved(error, effect)),
            (Ok(state), _) => Ok(SessionVisitMutation {
                saved: true,
                effect: state.effect(AuxiliaryEffectState::Recorded),
                state,
            }),
            (Err(error), _) => Err(error),
        }
    }
}
