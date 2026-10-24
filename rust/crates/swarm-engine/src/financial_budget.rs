//! Estimated USD envelope and caller-owned SQL reservations. No provider call,
//! actual cost, authenticated actor or permission to launch is established.
use crate::{
    encode_model,
    model_wire::{Fields, Kind},
    GoFloat, WorkError,
};
use rusqlite::{OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};

/// Go Budget DTO, including preserved estimation provenance. Zero limit is
/// unlimited, not measured zero cost. Submitted actor/time are not authority.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Budget {
    pub limit_usd: GoFloat,
    pub reserve_per_launch_usd: GoFloat,
    pub estimate_source: String,
    pub reference_date: String,
    pub updated: String,
    pub actor: String,
}
/// Complete Go budget.configure intent. Engine must replace actor/time before
/// persistence and exclude submitted metadata from idempotent request identity.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BudgetChange {
    pub schema_version: i64,
    pub event_id: String,
    pub expected_revision: i64,
    pub budget: Budget,
}
/// Typed safe monetary/codec/SQL refusal; sources retain original diagnostics.
pub enum BudgetError {
    Decode(WorkError),
    Sql(rusqlite::Error),
    /// The read failed and its explicit finalization failed too; preserve both.
    Cleanup {
        primary: Box<Self>,
        cleanup: rusqlite::Error,
    },
    Date(time::error::ComponentRange),
    Clock(swarm_wire_json::go_rfc3339::GoTimestampError),
    Refused(&'static str),
    Exhausted,
}
impl BudgetError {
    /// Stable safe family; budget exhaustion is distinct from corrupt inputs.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Exhausted => "budget_exhausted",
            Self::Sql(_) | Self::Cleanup { .. } => "budget-storage",
            _ => "budget-invalid",
        }
    }
    /// Static reason; never render private policy, IDs, source strings or SQL text.
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Decode(_) => "budget-json-invalid",
            Self::Sql(_) => "budget-query-or-write-failed",
            Self::Cleanup { .. } => "budget-statement-cleanup",
            Self::Date(_) => "budget-reference-date-invalid",
            Self::Clock(_) => "budget-reservation-time-invalid",
            Self::Refused(reason) => reason,
            Self::Exhausted => "budget-estimate-insufficient",
        }
    }
}
impl fmt::Display for BudgetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.reason())
    }
}
impl fmt::Debug for BudgetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for BudgetError {
    /// Expose the original typed primary failure; a simultaneous finalization
    /// cause remains in Cleanup rather than replacing the query/refusal cause.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Decode(e) => Some(e),
            Self::Sql(e) => Some(e),
            Self::Cleanup { primary, .. } => Some(primary.as_ref()),
            Self::Date(e) => Some(e),
            Self::Clock(e) => Some(e),
            _ => None,
        }
    }
}
type Result<T> = std::result::Result<T, BudgetError>;
/// Exact actual Go Budget field shapes/order; values carry no policy authority.
fn budget_shape() -> Fields {
    &[
        ("limit_usd", Kind::Float64),
        ("reserve_per_launch_usd", Kind::Float64),
        ("estimate_source", Kind::String),
        ("reference_date", Kind::String),
        ("updated", Kind::String),
        ("actor", Kind::String),
    ]
}
/// Decode strict Go-shaped public intent, preserving Go duplicate/null/case and
/// float representation. This performs no business validation, IO or operation.
/// Unknown fields/type/range/UTF8/trailing errors retain the safe codec cause.
pub fn decode_budget_change(raw: &[u8]) -> Result<BudgetChange> {
    crate::work::decode_mode(
        raw,
        &[
            ("schema_version", Kind::Int64),
            ("event_id", Kind::String),
            ("expected_revision", Kind::Int64),
            ("budget", Kind::Object(budget_shape)),
        ],
        true,
    )
    .map_err(BudgetError::Decode)
}
/// Require finite nonnegative estimates; positive limits require positive reserve,
/// nonblank Go-whitespace source and real YYYY-MM-DD date, matching Go policy.
/// Unlimited zero does not require provenance, but still rejects invalid numbers.
/// No IO, mutation, rounding, permission, spending guarantee or price lookup.
pub fn validate_budget(b: &Budget) -> Result<()> {
    let limit = b.limit_usd.0;
    let reserve = b.reserve_per_launch_usd.0;
    if !limit.is_finite() || !reserve.is_finite() || limit < 0.0 || reserve < 0.0 {
        return Err(BudgetError::Refused(
            "budget-nonnegative-finite-amounts-required",
        ));
    }
    if limit > 0.0 {
        if reserve <= 0.0 || !crate::text::nonempty(&b.estimate_source) {
            return Err(BudgetError::Refused("budget-reserve-and-source-required"));
        }
        let raw = b.reference_date.as_bytes();
        if raw.len() != 10
            || raw[4] != b'-'
            || raw[7] != b'-'
            || !raw
                .iter()
                .enumerate()
                .all(|(i, v)| i == 4 || i == 7 || v.is_ascii_digit())
        {
            return Err(BudgetError::Refused("budget-reference-date-format"));
        }
        let number = |part: &[u8]| part.iter().fold(0i32, |v, d| v * 10 + i32::from(d - b'0'));
        let month = time::Month::try_from(number(&raw[5..7]) as u8).map_err(BudgetError::Date)?;
        time::Date::from_calendar_date(number(&raw[..4]), month, number(&raw[8..]) as u8)
            .map_err(BudgetError::Date)?;
    }
    Ok(())
}
/// Validate external request schema/revision and monetary policy before replay.
/// Submitted metadata remains data; callers must clear actor/time before intent.
pub fn validate_budget_change(change: &BudgetChange) -> Result<()> {
    if change.schema_version != 1 || change.expected_revision < 1 {
        return Err(BudgetError::Refused("budget-schema-or-revision-invalid"));
    }
    validate_budget(&change.budget)
}
/// Read actual stored Go JSON under the caller's transaction. Missing policy is
/// the existing unlimited contract; malformed present policy is never absence.
/// Explicit statement finalization preserves read and cleanup errors together.
pub(crate) fn read_budget(tx: &rusqlite::Connection, work: &str) -> Result<Option<Budget>> {
    let mut statement = tx
        .prepare("SELECT body FROM budgets WHERE work_id=?")
        .map_err(BudgetError::Sql)?;
    let result: Result<Option<Vec<u8>>> = statement
        .query_row([work], |row| row.get(0))
        .optional()
        .map_err(BudgetError::Sql);
    let raw = sql_finished(result, statement.finalize())?;
    raw.map(|raw| {
        let budget =
            crate::work::decode_mode(&raw, budget_shape(), false).map_err(BudgetError::Decode)?;
        validate_budget(&budget)?;
        Ok(budget)
    })
    .transpose()
}
/// Reject corrupt active row values before summing the same ledger using SQLite.
/// Queries are static module-owned statements; only work ID is parameterized.
/// No inactive history is changed; missing schema/query/NULL/type/overflow fail.
fn ledger_sum(tx: &rusqlite::Connection, work: &str, rows: &str, sum: &str) -> Result<f64> {
    let mut statement = tx.prepare(rows).map_err(BudgetError::Sql)?;
    let scanned = (|| {
        let mut values = statement.query([work]).map_err(BudgetError::Sql)?;
        while let Some(row) = values.next().map_err(BudgetError::Sql)? {
            let value: f64 = row.get(0).map_err(BudgetError::Sql)?;
            if !value.is_finite() || value < 0.0 {
                return Err(BudgetError::Refused(
                    "budget-ledger-nonnegative-finite-required",
                ));
            }
        }
        Ok(())
    })();
    sql_finished(scanned, statement.finalize())?;
    let mut statement = tx.prepare(sum).map_err(BudgetError::Sql)?;
    let result = statement
        .query_row([work], |row| row.get::<_, f64>(0))
        .map_err(BudgetError::Sql);
    let result = sql_finished(result, statement.finalize())?;
    if !result.is_finite() || result < 0.0 {
        return Err(BudgetError::Refused("budget-ledger-sum-invalid"));
    }
    Ok(result)
}
/// Join read/step and explicit statement-finalization errors without replacing
/// the primary cause or inferring an empty ledger. This performs no retry/IO.
fn sql_finished<T>(result: Result<T>, cleanup: rusqlite::Result<()>) -> Result<T> {
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(BudgetError::Sql(error)),
        (Err(primary), Err(cleanup)) => Err(BudgetError::Cleanup {
            primary: Box::new(primary),
            cleanup,
        }),
    }
}
/// Read all four active ledgers without adding another estimated allocation.
/// Grouping is the existing Go-compatible monetary contract. Corrupt values,
/// SQL errors and arithmetic overflow refuse; no row or consumption is changed.
pub(crate) fn committed_estimates(tx: &rusqlite::Connection, work: &str) -> Result<f64> {
    let agents=ledger_sum(tx,work,"SELECT amount FROM reservations WHERE work_id=? AND state IN ('reserved','estimated')","SELECT coalesce(sum(amount),0) FROM reservations WHERE work_id=? AND state IN ('reserved','estimated')")?;
    let assist=ledger_sum(tx,work,"SELECT amount FROM assist_reservations WHERE work_id=? AND state IN ('reserved','estimated')","SELECT coalesce(sum(amount),0) FROM assist_reservations WHERE work_id=? AND state IN ('reserved','estimated')")?;
    let planning=ledger_sum(tx,work,"SELECT estimate FROM planning_calls WHERE work_id=? AND state!='released'","SELECT coalesce(sum(estimate),0) FROM planning_calls WHERE work_id=? AND state!='released'")?;
    let prep=ledger_sum(tx,work,"SELECT work_amount FROM preparation_reservations WHERE work_id=? AND state IN ('reserved','estimated')","SELECT coalesce(sum(work_amount),0) FROM preparation_reservations WHERE work_id=? AND state IN ('reserved','estimated')")?;
    let committed = agents + (assist + prep + planning);
    if !committed.is_finite() {
        return Err(BudgetError::Refused("budget-ledger-arithmetic-overflow"));
    }
    Ok(committed)
}
/// With an enforced positive limit, observe all four committed-estimate ledgers
/// before testing the next reserve; unlimited policy needs no ledger projection.
/// Requires an already-open transaction, never a cached or autocommit read.
/// Arithmetic grouping matches Go; no cent rounding or hidden epsilon. Corrupt
/// present rows/overflow refuse; no failed query becomes a zero commitment.
pub(crate) fn admission_budget(tx: &rusqlite::Connection, work: &str) -> Result<Option<Budget>> {
    if tx.is_autocommit() {
        return Err(BudgetError::Refused("budget-admission-transaction-required"));
    }
    let budget = read_budget(tx, work)?;
    let Some(b) = budget.as_ref().filter(|b| b.limit_usd.0 > 0.0) else {
        return Ok(budget);
    };
    let committed = committed_estimates(tx, work)?;
    let needed = committed + b.reserve_per_launch_usd.0;
    if !committed.is_finite() || !needed.is_finite() {
        return Err(BudgetError::Refused("budget-ledger-arithmetic-overflow"));
    }
    if needed > b.limit_usd.0 {
        return Err(BudgetError::Exhausted);
    }
    Ok(budget)
}
/// Check available estimated USD under the caller-owned actual SQL transaction.
/// Missing/zero policy is unlimited; exact remaining reserve fits. With a positive
/// limit, all four active ledgers count. No reservation or launch/actor/identity permission is created.
/// Caller must preserve transaction freshness/serialization and other guards.
pub fn check_launch_budget(tx: &Transaction<'_>, work: &str) -> Result<()> {
    admission_budget(tx, work).map(|_| ())
}
/// Validate an already admitted agent's reservation against current commitments.
/// Exact remaining capacity fits: the own reserved amount counts once among all
/// four ledgers, without adding another launch estimate. Positive policy requires
/// the matching reserved row and exact configured estimate. Missing/zero policy
/// remains unlimited but never makes corrupt present rows valid. This is a pure
/// caller-transaction read: no launch, price, new allocation, refund, counter reset
/// or task acceptance. Callers still owe current ownership/freshness and execution
/// checks; this guard alone grants none. Typed SQL/codec/cleanup causes survive.
pub fn check_reserved_agent_budget(tx: &Transaction<'_>, work: &str, agent: &str) -> Result<()> {
    if [work, agent]
        .iter()
        .any(|v| !swarm_identity_contract::name(v.as_bytes()))
    {
        return Err(BudgetError::Refused(
            "budget-reservation-identifiers-invalid",
        ));
    }
    let budget = read_budget(tx, work)?;
    let mut statement = tx
        .prepare("SELECT work_id,amount,state FROM reservations WHERE agent_id=?")
        .map_err(BudgetError::Sql)?;
    let result = (|| {
        let mut rows = statement.query([agent]).map_err(BudgetError::Sql)?;
        let own = match rows.next().map_err(BudgetError::Sql)? {
            None => None,
            Some(row) => Some((
                row.get::<_, String>(0).map_err(BudgetError::Sql)?,
                row.get::<_, f64>(1).map_err(BudgetError::Sql)?,
                row.get::<_, String>(2).map_err(BudgetError::Sql)?,
            )),
        };
        if rows.next().map_err(BudgetError::Sql)?.is_some() {
            return Err(BudgetError::Refused("budget-reservation-ambiguous"));
        }
        Ok(own)
    })();
    let cleanup = statement.finalize();
    let own = match (result, cleanup) {
        (Ok(value), Ok(())) => value,
        (Err(error), Ok(())) => return Err(error),
        (Ok(_), Err(error)) => return Err(BudgetError::Sql(error)),
        (Err(primary), Err(cleanup)) => {
            return Err(BudgetError::Cleanup {
                primary: Box::new(primary),
                cleanup,
            })
        }
    };
    if let Some((owner, amount, state)) = &own {
        if owner != work || state != "reserved" || !amount.is_finite() || *amount < 0.0 {
            return Err(BudgetError::Refused("budget-existing-reservation-invalid"));
        }
    }
    let committed = committed_estimates(tx, work)?;
    if let Some(policy) = budget.as_ref().filter(|b| b.limit_usd.0 > 0.0) {
        let Some((_, amount, _)) = own else {
            return Err(BudgetError::Refused("budget-existing-reservation-required"));
        };
        if amount != policy.reserve_per_launch_usd.0 {
            return Err(BudgetError::Refused("budget-existing-estimate-changed"));
        }
        if committed > policy.limit_usd.0 {
            return Err(BudgetError::Exhausted);
        }
    }
    Ok(())
}
/// Check existing funding immediately before a planning provider may start.
/// Requires an active caller transaction, the exact work/scope/claim row, finite nonnegative estimate,
/// estimated/unobserved state and all current committed ledgers. Under a finite
/// policy its estimate must still equal the current explicit reserve and total
/// commitment must fit; its own estimate is counted once, never reserved again.
/// Missing/unlimited policy still requires an intact original reservation. SQL,
/// type/codec and close failures retain causes; no row/policy/history is changed.
/// This proves funding only: caller still owes current lease/generation, source,
/// immutable inputs, identity/rights, process registration and a gate before exec.
pub fn check_reserved_planning_start_budget(
    tx: &rusqlite::Connection,
    work: &str,
    scope: &str,
    claim: &str,
) -> Result<()> {
    let estimate = reserved_planning_estimate(tx, work, scope, claim)?;
    let budget = read_budget(tx, work)?;
    let committed = committed_estimates(tx, work)?;
    if let Some(policy) = budget.as_ref().filter(|b| b.limit_usd.0 > 0.0) {
        if estimate != policy.reserve_per_launch_usd.0 {
            return Err(BudgetError::Refused(
                "budget-existing-planning-estimate-changed",
            ));
        }
        if committed > policy.limit_usd.0 {
            return Err(BudgetError::Exhausted);
        }
    }
    Ok(())
}

/// Validate the authoritative historical reservation for a known original result,
/// not a future start. Require a caller transaction, exact work/scope/call owner,
/// finite nonnegative stored estimate and estimated/unobserved state. Current
/// reserve tariff/limit apply to future spending and do not reprice this row.
/// This does not bind a separately signed historical price or settle usage;
/// caller must verify the actual original/wait/permission and source as needed.
/// SQL/type/cleanup errors remain failures; no row, policy, budget, history,
/// execution right, refund or additional reservation is created or changed.
pub(crate) fn check_reserved_planning_result_budget(
    tx: &rusqlite::Connection,
    work: &str,
    scope: &str,
    claim: &str,
) -> Result<()> {
    reserved_planning_estimate(tx, work, scope, claim).map(|_| ())
}
/// Read and validate exactly one canonical stored planning reservation under the
/// caller's transaction. The returned amount is ledger data, not a new invoice,
/// permission or proof of a signed original price. Legacy not-startable diagnostic
/// describes invalid ownership/state for both start and result consumers. Preserve
/// query/finalization causes and distinguish missing, malformed and observed rows.
fn reserved_planning_estimate(
    tx: &rusqlite::Connection,
    work: &str,
    scope: &str,
    claim: &str,
) -> Result<f64> {
    if tx.is_autocommit() {
        return Err(BudgetError::Refused("budget-transaction-required"));
    }
    if [work, scope, claim]
        .iter()
        .any(|v| !swarm_identity_contract::name(v.as_bytes()))
    {
        return Err(BudgetError::Refused(
            "budget-reservation-identifiers-invalid",
        ));
    }
    let mut statement=tx.prepare("SELECT work_id,scope_id,estimate,state,usage IS NOT NULL FROM planning_calls WHERE id=?").map_err(BudgetError::Sql)?;
    let result = (|| {
        let mut rows = statement.query([claim]).map_err(BudgetError::Sql)?;
        let own = match rows.next().map_err(BudgetError::Sql)? {
            None => None,
            Some(row) => Some((
                row.get::<_, String>(0).map_err(BudgetError::Sql)?,
                row.get::<_, String>(1).map_err(BudgetError::Sql)?,
                row.get::<_, f64>(2).map_err(BudgetError::Sql)?,
                row.get::<_, String>(3).map_err(BudgetError::Sql)?,
                row.get::<_, bool>(4).map_err(BudgetError::Sql)?,
            )),
        };
        if rows.next().map_err(BudgetError::Sql)?.is_some() {
            return Err(BudgetError::Refused(
                "budget-planning-reservation-ambiguous",
            ));
        }
        Ok(own)
    })();
    let cleanup = statement.finalize();
    let own = match (result, cleanup) {
        (Ok(value), Ok(())) => value,
        (Err(e), Ok(())) => return Err(e),
        (Ok(_), Err(e)) => return Err(BudgetError::Sql(e)),
        (Err(primary), Err(cleanup)) => {
            return Err(BudgetError::Cleanup {
                primary: Box::new(primary),
                cleanup,
            })
        }
    };
    let Some((owner, owner_scope, estimate, state, observed)) = own else {
        return Err(BudgetError::Refused("budget-planning-reservation-required"));
    };
    if owner != work
        || owner_scope != scope
        || !estimate.is_finite()
        || estimate < 0.0
        || state != "estimated"
        || observed
    {
        return Err(BudgetError::Refused(
            "budget-planning-reservation-not-startable",
        ));
    }
    Ok(estimate)
}

/// Reserve one planning-call estimate after rechecking the common envelope in
/// this same transaction. Unlimited/absent policy inserts estimate0, not real cost.
/// IDs/time must be valid protocol data, never supplied authority. Checked SQL
/// insertion, duplicate/invalid/codec/query errors retain causes. Caller owns
/// rollback/commit and lease/capacity/identity guards; this function never commits.
pub fn reserve_planning_call(
    tx: &Transaction<'_>,
    work: &str,
    scope: &str,
    id: &str,
    at: &str,
) -> Result<()> {
    if [work, scope, id]
        .iter()
        .any(|v| !swarm_identity_contract::name(v.as_bytes()))
    {
        return Err(BudgetError::Refused(
            "budget-reservation-identifiers-invalid",
        ));
    }
    swarm_wire_json::go_rfc3339::parse(at).map_err(BudgetError::Clock)?;
    let budget = admission_budget(tx, work)?;
    let estimate = budget
        .filter(|b| b.limit_usd.0 > 0.0)
        .map_or(0.0, |b| b.reserve_per_launch_usd.0);
    let changed=tx.execute("INSERT INTO planning_calls(id,work_id,scope_id,estimate,state,created_at) VALUES(?,?,?,?,'estimated',?)",rusqlite::params![id,work,scope,estimate,at]).map_err(BudgetError::Sql)?;
    if changed != 1 {
        return Err(BudgetError::Refused("budget-reservation-not-inserted"));
    }
    Ok(())
}
/// Stage operator-observed policy and its cockpit event atomically in caller TX.
/// Does not reset reservations or commit; actor/time come from the engine owner,
/// not request metadata. Codec/query/clock/empty actor/write failures propagate.
pub(crate) fn write_budget(
    tx: &Transaction<'_>,
    work: &str,
    budget: &Budget,
    actor: &str,
    at: &str,
) -> Result<()> {
    validate_budget(budget)?;
    if !crate::text::nonempty(actor) {
        return Err(BudgetError::Refused("budget-operator-required"));
    }
    swarm_wire_json::go_rfc3339::parse(at).map_err(BudgetError::Clock)?;
    let mut saved = budget.clone();
    saved.actor = actor.into();
    saved.updated = at.into();
    let raw = encode_model(&saved).map_err(BudgetError::Decode)?;
    let policy_rows = tx.execute("INSERT INTO budgets(work_id,body) VALUES(?,?) ON CONFLICT(work_id) DO UPDATE SET body=excluded.body",rusqlite::params![work,raw]).map_err(BudgetError::Sql)?;
    let event_rows = tx
        .execute(
            "INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,'budget',?)",
            rusqlite::params![
                work,
                at,
                format!(
                    "{} : limite estimative {:.2} USD ; réservation {:.2} USD",
                    actor, saved.limit_usd.0, saved.reserve_per_launch_usd.0
                )
            ],
        )
        .map_err(BudgetError::Sql)?;
    if policy_rows != 1 || event_rows != 1 {
        return Err(BudgetError::Refused("budget-policy-or-event-not-written"));
    }
    Ok(())
}
