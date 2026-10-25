//! Historical financial observations, not invoices or execution permission.
use super::{finished, reported_costs, RootStore, RootStoreError as E};
use crate::{Budget, BudgetChange, GoFloat};
use rusqlite::Connection;
use serde::Serialize;

/// Historical scope statement; protocol explanation, not a spending policy.
const COVERAGE: &str = "Estimated envelope: execution, planning, page assistance and preparation. Independent reviews have a separate call limit; their monetary cost is not included. Other launch conditions still apply. Active agents are not stopped.";

/// Go BudgetView: estimates and declared actual costs deliberately remain separate.
/// Unknown actual cost is null; an explicit reported zero remains zero. Warning
/// uses the persisted Admin setting; it never admits or stops a process.
#[derive(Serialize)]
pub struct HistoricalBudgetView {
    pub actual_cost_scope: &'static str,
    pub budget: Budget,
    pub reserved_usd: GoFloat,
    pub committed_estimate_usd: GoFloat,
    pub remaining_usd: GoFloat,
    pub warning: bool,
    pub actual_cost_usd: Option<GoFloat>,
    pub policy: &'static str,
}
/// Historical show envelope, observed in one SQL read transaction. Its revision
/// and amounts provide no future lease, authenticated actor or launch grant.
#[derive(Serialize)]
pub struct HistoricalBudgetShow {
    pub revision: i64,
    pub budget: HistoricalBudgetView,
    pub coverage: &'static str,
}
/// Historical advisory preview. Application must independently recheck revision,
/// replay and reservations. Proposed actor/time are cleared, never authenticated.
#[derive(Serialize)]
pub struct HistoricalBudgetPreview {
    pub revision: i64,
    pub current: HistoricalBudgetView,
    pub proposed: Budget,
    pub proposed_remaining_usd: GoFloat,
    pub proposed_unlimited: bool,
    pub new_launch_budget_allowed: bool,
    pub coverage: &'static str,
}

/// Preserve the existing budget failure category and its full typed cause.
fn budget_error(error: crate::BudgetError) -> E {
    E::Mutation(crate::WorkEngineError::Budget(error))
}
/// Reject invalid retained numbers rather than treating corrupt data as zero.
/// No rounding, epsilon, ceiling or operator policy is introduced.
fn amount(value: f64) -> Result<f64, E> {
    if value.is_finite() && value >= 0.0 { Ok(value) }
    else { Err(E::Refused("budget-history-amount-invalid")) }
}
/// Validate all selected row amounts before using SQLite's own aggregate/grouping.
/// Queries are module-owned static statements with the Work as bound data. Caller
/// owns the SQL snapshot. Preserve step and explicit finalization failures; no IO
/// error or malformed row is converted to an empty successful ledger.
fn totals(db: &Connection, work: &str, scan: &str, aggregate: &str) -> Result<(f64, f64), E> {
    let mut statement = db.prepare(scan).map_err(E::Sql)?;
    let result = (|| {
        let mut rows = statement.query([work]).map_err(E::Sql)?;
        while let Some(row) = rows.next().map_err(E::Sql)? {
            amount(row.get::<_, f64>(0).map_err(E::Sql)?)?;
        }
        Ok(())
    })();
    finished(result, statement.finalize().map_err(E::Sql))?;
    let mut statement = db.prepare(aggregate).map_err(E::Sql)?;
    let result = (|| {
        let pair = statement.query_row([work], |r| Ok((r.get::<_, f64>(0)?, r.get::<_, f64>(1)?))).map_err(E::Sql)?;
        Ok((amount(pair.0)?, amount(pair.1)?))
    })();
    finished(result, statement.finalize().map_err(E::Sql))
}
/// Sum declared planner costs, including released calls, without inventing prices.
/// SQL NULL/JSON null/missing cost remains unknown. Validate numeric types before
/// SQLite SUM so strings cannot silently become zero. Malformed JSON, SQL,
/// negative/nonfinite values and finalization failures retain their cause.
fn planning_actual(db: &Connection, work: &str) -> Result<Option<f64>, E> {
    let mut statement = db.prepare("SELECT usage,json_extract(usage,'$.provider_reported_cost_usd'),json_type(usage,'$.provider_reported_cost_usd') FROM planning_calls WHERE work_id=?").map_err(E::Sql)?;
    let result = (|| {
        let mut rows = statement.query([work]).map_err(E::Sql)?;
        while let Some(row) = rows.next().map_err(E::Sql)? {
            // SQLite accepts some non-JSON blobs as JSONB. Stored usage is the
            // Go JSON-text contract; validate its actual bytes before trusting
            // a missing extracted field as an unknown measurement.
            match row.get_ref(0).map_err(E::Sql)? {
                rusqlite::types::ValueRef::Null => {},
                rusqlite::types::ValueRef::Text(raw) | rusqlite::types::ValueRef::Blob(raw) => {
                    serde_json::from_slice::<&serde_json::value::RawValue>(raw)
                        .map_err(|e| E::Encode(crate::WorkError::Json(e)))?;
                },
                _ => return Err(E::Refused("budget-history-planning-usage-type-invalid")),
            }
            let kind: Option<String> = row.get(2).map_err(E::Sql)?;
            match kind.as_deref() {
                None | Some("null") => {},
                Some("integer" | "real") => { amount(row.get::<_, f64>(1).map_err(E::Sql)?)?; },
                _ => return Err(E::Refused("budget-history-planning-cost-invalid")),
            }
        }
        Ok(())
    })();
    finished(result, statement.finalize().map_err(E::Sql))?;
    let mut statement = db.prepare("SELECT sum(json_extract(usage,'$.provider_reported_cost_usd')) FROM planning_calls WHERE work_id=?").map_err(E::Sql)?;
    let result = statement.query_row([work], |r| r.get::<_, Option<f64>>(0)).map_err(E::Sql)
        .and_then(|value| value.map(amount).transpose());
    finished(result, statement.finalize().map_err(E::Sql))
}
/// Read all four estimate ledgers with Go arithmetic grouping and no UI window.
/// Caller owns the snapshot; active corrupt amounts or sum overflow refuse.
/// Released amounts do not consume the envelope. No rows or counters change.
fn estimates(db: &Connection, work: &str) -> Result<(f64, f64), E> {
    let agent = totals(db, work, "SELECT amount FROM reservations WHERE work_id=? AND state IN ('reserved','estimated')", "SELECT coalesce(sum(CASE WHEN state='reserved' THEN amount ELSE 0 END),0),coalesce(sum(CASE WHEN state='estimated' THEN amount ELSE 0 END),0) FROM reservations WHERE work_id=?")?;
    let assist = totals(db, work, "SELECT amount FROM assist_reservations WHERE work_id=? AND state IN ('reserved','estimated')", "SELECT coalesce(sum(CASE WHEN state='reserved' THEN amount ELSE 0 END),0),coalesce(sum(CASE WHEN state='estimated' THEN amount ELSE 0 END),0) FROM assist_reservations WHERE work_id=?")?;
    let preparation = totals(db, work, "SELECT work_amount FROM preparation_reservations WHERE work_id=? AND state IN ('reserved','estimated')", "SELECT coalesce(sum(CASE WHEN state='reserved' THEN work_amount ELSE 0 END),0),coalesce(sum(CASE WHEN state='estimated' THEN work_amount ELSE 0 END),0) FROM preparation_reservations WHERE work_id=?")?;
    let planning = totals(db, work, "SELECT estimate FROM planning_calls WHERE work_id=? AND state!='released'", "SELECT 0.0,coalesce(sum(estimate),0) FROM planning_calls WHERE work_id=? AND state!='released'")?;
    let reserved = amount(amount(agent.0 + preparation.0)? + assist.0)?;
    let estimated = amount(amount(amount(agent.1 + preparation.1)? + assist.1)? + planning.1)?;
    Ok((reserved, estimated))
}
/// Build the exact financial shape using all four estimate ledgers and both
/// actual-cost sources in the caller's snapshot. Missing saved budget has the
/// historical unlimited wire representation; malformed policy still refuses.
/// Sum grouping matches Go. Provider declarations remain unauthenticated, partial
/// and separate from atomic launch admission. No state or reservations change.
fn view(db: &Connection, work: &str, warning_percent: Option<i64>) -> Result<HistoricalBudgetView, E> {
    let budget = crate::financial_budget::read_budget(db, work).map_err(budget_error)?.unwrap_or_default();
    let (reserved, estimated) = estimates(db, work)?;
    let committed = amount(reserved + estimated)?;
    let worker = reported_costs::read(db, work)?;
    let actual = match (planning_actual(db, work)?, worker.total.attempts_with_cost > 0) {
        (Some(planner), true) => Some(amount(planner + worker.total.reported_usd.0)?),
        (Some(planner), false) => Some(planner),
        (None, true) => Some(worker.total.reported_usd.0),
        (None, false) => None,
    };
    Ok(HistoricalBudgetView {
        actual_cost_scope: "Coûts rapportés par les agents et planificateurs ; les valeurs absentes restent inconnues. Estimations partagées entre exécution, planification, assistance et préparation.",
        remaining_usd: GoFloat((budget.limit_usd.0 - reserved - estimated).max(0.0)),
        warning: budget.limit_usd.0 > 0.0 && warning_percent.is_some_and(|p| committed >= (p as f64 / 100.0) * budget.limit_usd.0),
        budget, reserved_usd: GoFloat(reserved), committed_estimate_usd: GoFloat(estimated),
        actual_cost_usd: actual.map(GoFloat),
        policy: "Budget estimatif non garanti. Réservations atomiques avant départ ; les agents actifs continuent. Le réservé n'est pas une dépense : le coût rapporté est indiqué à part.",
    })
}
impl RootStore {
    /// Read historical show JSON in one SQL snapshot, using the explicitly saved
    /// Admin warning percentage (null disables). Absent old settings refuse rather
    /// than adopting a hidden 80%. Settings are observed before SQL, not atomically
    /// with that external file. Work/ledger/cost/read/rollback errors propagate.
    /// This read grants no permission, provider execution, visit or budget mutation.
    pub fn historical_budget_show(&self, work: &str) -> Result<HistoricalBudgetShow, E> {
        let settings = self.settings()?;
        if settings.values.work_budget_warning_percent.is_none() { return Err(E::Refused("budget-warning-policy-missing")); }
        let mut tx = self.db.unchecked_transaction().map_err(E::Sql)?;
        tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
        let result = (|| {
            let current = crate::read_recorded_work(&tx, work).map_err(E::Work)?;
            Ok(HistoricalBudgetShow { revision: current.revision,
                budget: view(&tx, work, settings.values.budget_warning_percent())?, coverage: COVERAGE })
        })();
        finished(result, tx.rollback().map_err(E::Sql))
    }
    /// Decode/validate an advisory financial preview before reads, rejecting stale
    /// revision and stripping submitted actor/time. Reuse the coherent show view;
    /// its snapshot is not a lease for later apply. No reservation/history/counter
    /// is changed and allowance is monetary only. Original causes propagate.
    pub fn preview_historical_budget(&self, work: &str, raw: &[u8]) -> Result<HistoricalBudgetPreview, E> {
        let mut change: BudgetChange = crate::decode_budget_change(raw).map_err(budget_error)?;
        crate::validate_budget_change(&change).map_err(budget_error)?;
        change.budget.actor.clear(); change.budget.updated.clear();
        let current = self.historical_budget_show(work)?;
        if current.revision != change.expected_revision { return Err(E::Refused("budget-revision-conflict")); }
        let remaining = change.budget.limit_usd.0 - current.budget.reserved_usd.0 - current.budget.committed_estimate_usd.0;
        Ok(HistoricalBudgetPreview { revision: current.revision, current: current.budget,
            proposed_remaining_usd: GoFloat(remaining.max(0.0)), proposed_unlimited: change.budget.limit_usd.0 == 0.0,
            new_launch_budget_allowed: change.budget.limit_usd.0 == 0.0 || remaining >= change.budget.reserve_per_launch_usd.0,
            proposed: change.budget, coverage: COVERAGE })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Isolated helper-schema fixture only; not a Root writer or launch proof.
    fn ledgers() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE reservations(work_id TEXT,amount REAL,state TEXT);
            CREATE TABLE assist_reservations(work_id TEXT,amount REAL,state TEXT);
            CREATE TABLE preparation_reservations(work_id TEXT,amount REAL,work_amount REAL,state TEXT);
            CREATE TABLE planning_calls(work_id TEXT,estimate REAL,usage BLOB,state TEXT);
            INSERT INTO reservations VALUES('w',2,'reserved'),('w',7,'estimated'),('other',900,'reserved'),('w',999,'released');
            INSERT INTO assist_reservations VALUES('w',3,'reserved'),('w',13,'estimated');
            INSERT INTO preparation_reservations VALUES('w',999,5,'reserved'),('w',999,11,'estimated');
            INSERT INTO planning_calls VALUES('w',17,NULL,'recorded'),('w',999,NULL,'released');").unwrap();
        db
    }
    /// All four actual query shapes, attribution, released exclusions and amount
    /// selection precede active corruption/refusal, then restored exact sums.
    #[test]
    fn historical_budget_all_ledgers_refuse_corruption_and_recover() {
        let db = ledgers();
        assert_eq!(estimates(&db, "w").unwrap(), (10.0, 48.0));
        assert_eq!(estimates(&db, "absent").unwrap(), (0.0, 0.0));
        for table in ["reservations", "assist_reservations", "preparation_reservations", "planning_calls"] {
            let column = if table == "planning_calls" { "estimate" } else if table == "preparation_reservations" { "work_amount" } else { "amount" };
            db.execute_batch("SAVEPOINT fault").unwrap();
            db.execute(&format!("UPDATE {table} SET {column}=-1 WHERE work_id='w' AND state!='released'"), []).unwrap();
            assert!(estimates(&db, "w").is_err(), "{table}");
            db.execute_batch("ROLLBACK TO fault; RELEASE fault").unwrap();
            assert_eq!(estimates(&db, "w").unwrap(), (10.0, 48.0));
        }
        db.execute("UPDATE planning_calls SET estimate=1e308 WHERE state!='released'", []).unwrap();
        db.execute("INSERT INTO planning_calls VALUES('w',1e308,NULL,'recorded')", []).unwrap();
        assert!(estimates(&db, "w").is_err());
    }
    /// Missing, explicit zero and positive declared costs remain distinct;
    /// malformed JSONB-looking blobs/strings/negative values refuse and recover.
    #[test]
    fn historical_planning_costs_validate_json_bytes_before_sql_jsonb() {
        let db = ledgers();
        assert_eq!(planning_actual(&db, "w").unwrap(), None);
        for (raw, expected) in [(br#"{"provider_reported_cost_usd":0}"#.as_slice(), Some(0.0)),
            (br#"{"provider_reported_cost_usd":0.75}"#.as_slice(), Some(0.75)),
            (b"null".as_slice(), None)] {
            db.execute("UPDATE planning_calls SET usage=? WHERE state='recorded'", [raw]).unwrap();
            assert_eq!(planning_actual(&db, "w").unwrap(), expected);
        }
        for raw in [b"{invalid".as_slice(), br#"{"provider_reported_cost_usd":"0.75"}"#.as_slice(),
            br#"{"provider_reported_cost_usd":-1}"#.as_slice(), br#"{"provider_reported_cost_usd":1e999}"#.as_slice()] {
            db.execute("UPDATE planning_calls SET usage=? WHERE state='recorded'", [raw]).unwrap();
            assert!(planning_actual(&db, "w").is_err());
            db.execute("UPDATE planning_calls SET usage=NULL WHERE state='recorded'", []).unwrap();
            assert_eq!(planning_actual(&db, "w").unwrap(), None);
        }
    }
}
