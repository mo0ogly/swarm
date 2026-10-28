//! root_store/preparation_call_budget.rs — preparation AI call budget
//! reservation over `preparation_reservations`, executed inside the
//! caller-owned transaction (no `BEGIN`/`COMMIT` here). This establishes no
//! provider contact, actual cost, authenticated intent or launch permission:
//! recorded amounts are reserves only, and a zero limit is unlimited exactly
//! as in Go, so a recorded zero reserve never asserts a measured zero cost.
//! Ledger reads see the transaction's current shared state; no operator
//! totals, retries, time cutoffs or new tables. Refusals are static codes
//! carrying no amounts, identifiers or keys.
//!
//! Caller flow: `BEGIN IMMEDIATE`, insert the turn, validate source/current
//! revision, [`reserve`], all writes/triggers, [`verify`] again, commit.
//! [`mark_attempt`] runs before any possible provider contact; an `estimated`
//! reservation is never automatically released.

use super::preparation_read::cause;
use super::{finished, RootStoreError};
use crate::financial_budget::{ledger_sum, read_budget};
use crate::{read_recorded_work, validate_budget, Preparation};
use rusqlite::{params, Connection};

const TRANSACTION_REQUIRED: &str = "preparation-budget-transaction-required";
const ENVELOPE_INVALID: &str = "preparation-budget-envelope-invalid";
const PREPARATION_EXHAUSTED: &str = "preparation-budget-exhausted";
const WORK_EXHAUSTED: &str = "work-budget-exhausted";
const ARITHMETIC_INVALID: &str = "preparation-budget-arithmetic-invalid";
const TURN_MISSING: &str = "preparation-budget-turn-missing";
const TURN_NOT_RUNNING: &str = "preparation-budget-turn-not-running";
const RESERVATION_STATE: &str = "preparation-budget-reservation-state";
const WITNESS_UNMATCHED: &str = "preparation-budget-witness-unmatched";

// Exact Go preparation reservation grouping, including planning_calls. `?1`
// lets ledger_sum bind both union branches without regrouping floating sums.
const PREPARATION_USED_ROWS: &str = "SELECT amount FROM preparation_reservations WHERE preparation_id=? AND state IN ('reserved','estimated')";
const PREPARATION_USED_SUM: &str = "SELECT coalesce(sum(amount),0) FROM preparation_reservations WHERE preparation_id=? AND state IN ('reserved','estimated')";
const AGENT_ASSIST_ROWS: &str = "SELECT amount FROM reservations WHERE work_id=?1 AND state IN ('reserved','estimated') UNION ALL SELECT amount FROM assist_reservations WHERE work_id=?1 AND state IN ('reserved','estimated')";
const AGENT_ASSIST_SUM: &str = "SELECT coalesce(sum(amount),0) FROM (SELECT amount FROM reservations WHERE work_id=?1 AND state IN ('reserved','estimated') UNION ALL SELECT amount FROM assist_reservations WHERE work_id=?1 AND state IN ('reserved','estimated'))";
const PREPARATION_WORK_ROWS: &str = "SELECT work_amount FROM preparation_reservations WHERE work_id=? AND state IN ('reserved','estimated')";
const PREPARATION_WORK_SUM: &str = "SELECT coalesce(sum(work_amount),0) FROM preparation_reservations WHERE work_id=? AND state IN ('reserved','estimated')";
const PLANNING_ROWS: &str =
    "SELECT estimate FROM planning_calls WHERE work_id=? AND state!='released'";
const PLANNING_SUM: &str =
    "SELECT coalesce(sum(estimate),0) FROM planning_calls WHERE work_id=? AND state!='released'";

/// Witness of the one `reserved` row written by [`reserve`]. Its amounts are
/// recorded reserves, never measured or actual cost, and possession grants no
/// process, provider or permission right. [`verify`] accepts only the exact
/// saved row (all five columns) still in state `reserved` — not mere
/// existence for the turn.
pub(super) struct Witness {
    turn: String,
    prep: String,
    work: String,
    amount: f64,
    work_amount: f64,
}

/// Refuse anything outside the caller's open immediate transaction.
fn require_transaction(db: &Connection) -> Result<(), RootStoreError> {
    if db.is_autocommit() {
        Err(RootStoreError::Refused(TRANSACTION_REQUIRED))
    } else {
        Ok(())
    }
}

/// Nonnegative-finite-checked projection addition; inputs are already
/// validated nonnegative-finite, so only overflow can fail.
fn checked_add(left: f64, right: f64) -> Result<f64, RootStoreError> {
    let sum = left + right;
    if sum.is_finite() {
        Ok(sum)
    } else {
        Err(RootStoreError::Refused(ARITHMETIC_INVALID))
    }
}

/// Transcribes Go `reservePreparationBudget` exactly. The optional
/// preparation budget is validated even when unlimited; a limit above zero
/// compares the preparation's `reserved`/`estimated` `amount` sum plus its
/// reserve and records that reserve. When `work_id` is set, the linked Work
/// is read via `read_recorded_work` and the saved envelope via `read_budget`;
/// a limit above zero counts, in Go order, the agent+assist union sum, the
/// preparation `work_amount` sum, the planning estimate sum and the work
/// reserve (`financial_budget::committed_estimates` is deliberately not used:
/// its agent-launch grouping differs). Zero/absent limits record zero.
/// Inserts the `reserved` row and returns its [`Witness`]; performs no
/// commit, process call, provider contact or permission check.
pub(super) fn reserve(
    db: &Connection,
    p: &Preparation,
    turn: &str,
) -> Result<Witness, RootStoreError> {
    require_transaction(db)?;
    if let Some(budget) = &p.budget {
        validate_budget(budget).map_err(|e| cause(ENVELOPE_INVALID, e))?;
    }
    let mut amount = 0.0_f64;
    let mut work_amount = 0.0_f64;
    if let Some(budget) = &p.budget {
        let limit = budget.limit_usd.0;
        if limit > 0.0 {
            let used = ledger_sum(db, &p.id, PREPARATION_USED_ROWS, PREPARATION_USED_SUM)
                .map_err(|e| cause(ENVELOPE_INVALID, e))?;
            let reserve = budget.reserve_per_launch_usd.0;
            if checked_add(used, reserve)? > limit {
                return Err(RootStoreError::Refused(PREPARATION_EXHAUSTED));
            }
            amount = reserve;
        }
    }
    if !p.work_id.is_empty() {
        read_recorded_work(db, &p.work_id).map_err(RootStoreError::Work)?;
        if let Some(work_budget) =
            read_budget(db, &p.work_id).map_err(|e| cause(ENVELOPE_INVALID, e))?
        {
            let limit = work_budget.limit_usd.0;
            if limit > 0.0 {
                let agents = ledger_sum(db, &p.work_id, AGENT_ASSIST_ROWS, AGENT_ASSIST_SUM)
                    .map_err(|e| cause(ENVELOPE_INVALID, e))?;
                let preparation =
                    ledger_sum(db, &p.work_id, PREPARATION_WORK_ROWS, PREPARATION_WORK_SUM)
                        .map_err(|e| cause(ENVELOPE_INVALID, e))?;
                let planning = ledger_sum(db, &p.work_id, PLANNING_ROWS, PLANNING_SUM)
                    .map_err(|e| cause(ENVELOPE_INVALID, e))?;
                let reserve = work_budget.reserve_per_launch_usd.0;
                let projected = checked_add(
                    checked_add(checked_add(agents, preparation)?, planning)?,
                    reserve,
                )?;
                if projected > limit {
                    return Err(RootStoreError::Refused(WORK_EXHAUSTED));
                }
                work_amount = reserve;
            }
        }
    }
    let mut statement = db
        .prepare("INSERT INTO preparation_reservations(turn_id,preparation_id,work_id,amount,work_amount,state) VALUES(?,?,?,?,?,'reserved')")
        .map_err(RootStoreError::Sql)?;
    let inserted = statement
        .execute(params![turn, p.id, p.work_id, amount, work_amount])
        .map_err(RootStoreError::Sql);
    let inserted = finished(inserted, statement.finalize().map_err(RootStoreError::Sql))?;
    if inserted != 1 {
        return Err(RootStoreError::Refused(WITNESS_UNMATCHED));
    }
    Ok(Witness {
        turn: turn.to_string(),
        prep: p.id.clone(),
        work: p.work_id.clone(),
        amount,
        work_amount,
    })
}

/// Pre-commit re-check: accepts only the exact witnessed row — every recorded
/// column equal and state still `reserved` — counting matches, so missing,
/// drifted or duplicated rows are refused rather than mere existence.
pub(super) fn verify(db: &Connection, witness: &Witness) -> Result<(), RootStoreError> {
    verify_state(db, witness, "reserved")
}

/// Recheck every recorded identity/amount and exact ledger state after triggers.
fn verify_state(db: &Connection, witness: &Witness, state: &str) -> Result<(), RootStoreError> {
    require_transaction(db)?;
    let mut statement = db
        .prepare("SELECT count(*) FROM preparation_reservations WHERE turn_id=? AND preparation_id=? AND work_id=? AND amount=? AND work_amount=? AND state=?")
        .map_err(RootStoreError::Sql)?;
    let matched = statement
        .query_row(
            params![
                witness.turn,
                witness.prep,
                witness.work,
                witness.amount,
                witness.work_amount,
                state
            ],
            |row| row.get::<_, i64>(0),
        )
        .map_err(RootStoreError::Sql);
    let matched = finished(matched, statement.finalize().map_err(RootStoreError::Sql))?;
    if matched != 1 {
        return Err(RootStoreError::Refused(WITNESS_UNMATCHED));
    }
    Ok(())
}

/// Moves the witnessed reservation from `reserved` to `estimated`; the caller
/// must invoke it before any possible provider contact. This function itself
/// performs no network, process, permission or provider action. Refuses when
/// the turn row is missing or not `running` (Go: "interrupted") and when the
/// exact reserved witness row does not change exactly one row — already
/// `estimated` or `released`, missing, or drifted. Afterwards the reservation
/// is never automatically released, even without provider usage.
pub(super) fn mark_attempt(db: &Connection, witness: &Witness) -> Result<(), RootStoreError> {
    require_transaction(db)?;
    let mut statement = db
        .prepare("SELECT status FROM preparation_turns WHERE id=?")
        .map_err(RootStoreError::Sql)?;
    let status = statement
        .query_row(params![witness.turn], |row| row.get::<_, String>(0))
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => RootStoreError::Refused(TURN_MISSING),
            other => RootStoreError::Sql(other),
        });
    let status = finished(status, statement.finalize().map_err(RootStoreError::Sql))?;
    if status != "running" {
        return Err(RootStoreError::Refused(TURN_NOT_RUNNING));
    }
    let mut statement = db
        .prepare("UPDATE preparation_reservations SET state='estimated' WHERE turn_id=? AND preparation_id=? AND work_id=? AND amount=? AND work_amount=? AND state='reserved'")
        .map_err(RootStoreError::Sql)?;
    let changed = statement
        .execute(params![
            witness.turn,
            witness.prep,
            witness.work,
            witness.amount,
            witness.work_amount
        ])
        .map_err(RootStoreError::Sql);
    let changed = finished(changed, statement.finalize().map_err(RootStoreError::Sql))?;
    if changed != 1 {
        return Err(RootStoreError::Refused(RESERVATION_STATE));
    }
    verify_state(db, witness, "estimated")
}

/// Exact expected final ledger row, captured before settlement. This is a
/// transactional witness only, never a monetary charge or execution right.
pub(super) struct Settlement {
    witness: Witness,
    state: String,
}
/// Recheck expected final ledger bytes after every later turn write/trigger.
pub(super) fn verify_settlement(db: &Connection, s: &Settlement) -> Result<(), RootStoreError> {
    verify_state(db, &s.witness, &s.state)
}
/// Release only proven unstarted reserved rows. Estimated remains estimated.
/// Missing/corrupt/misbound rows refuse; exact amounts/owner/state are checked
/// after triggers. Caller rechecks returned witness after its remaining writes.
pub(super) fn release_unstarted(db: &Connection, turn: &str) -> Result<Settlement, RootStoreError> {
    require_transaction(db)?;
    let mut statement=db.prepare("SELECT r.preparation_id,r.work_id,r.amount,r.work_amount,r.state FROM preparation_reservations r JOIN preparation_turns t ON t.id=r.turn_id AND t.preparation_id=r.preparation_id WHERE r.turn_id=?1").map_err(RootStoreError::Sql)?;
    let row = statement
        .query_row([turn], |r| {
            Ok((
                Witness {
                    turn: turn.into(),
                    prep: r.get(0)?,
                    work: r.get(1)?,
                    amount: r.get(2)?,
                    work_amount: r.get(3)?,
                },
                r.get::<_, String>(4)?,
            ))
        })
        .map_err(RootStoreError::Sql);
    let (w, state) = finished(row, statement.finalize().map_err(RootStoreError::Sql))?;
    if !w.amount.is_finite()
        || !w.work_amount.is_finite()
        || w.amount < 0.0
        || w.work_amount < 0.0
        || !matches!(state.as_str(), "reserved" | "estimated" | "released")
    {
        return Err(RootStoreError::Refused(RESERVATION_STATE));
    }
    let expected = if state == "reserved" {
        let mut statement=db.prepare("UPDATE preparation_reservations SET state='released' WHERE turn_id=? AND state='reserved'").map_err(RootStoreError::Sql)?;
        let changed = statement.execute([turn]).map_err(RootStoreError::Sql);
        if finished(changed, statement.finalize().map_err(RootStoreError::Sql))? != 1 {
            return Err(RootStoreError::Refused(RESERVATION_STATE));
        }
        "released".to_owned()
    } else {
        state
    };
    let s = Settlement {
        witness: w,
        state: expected,
    };
    verify_settlement(db, &s)?;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Isolated SQL contract only; no provider, Root or production data access.
    fn fixture() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE preparation_turns(id TEXT PRIMARY KEY,status TEXT,preparation_id TEXT);CREATE TABLE preparation_reservations(turn_id TEXT PRIMARY KEY,preparation_id TEXT,work_id TEXT,amount REAL,work_amount REAL,state TEXT);BEGIN IMMEDIATE;INSERT INTO preparation_turns VALUES('turn','running','prep');").unwrap();
        db
    }
    #[test]
    fn unlimited_reserves_zero_and_attempt_is_never_refunded() {
        let db = fixture();
        let p = Preparation {
            id: "prep".into(),
            ..Default::default()
        };
        let w = reserve(&db, &p, "turn").unwrap();
        verify(&db, &w).unwrap();
        mark_attempt(&db, &w).unwrap();
        release_unstarted(&db, "turn").unwrap();
        verify_state(&db, &w, "estimated").unwrap();
        assert!(mark_attempt(&db, &w).is_err());
    }
    #[test]
    fn prep_limit_and_corrupt_ledger_refuse_without_insert() {
        let db = fixture();
        let mut p = Preparation {
            id: "prep".into(),
            budget: Some(Box::new(crate::Budget {
                limit_usd: crate::GoFloat(1.0),
                reserve_per_launch_usd: crate::GoFloat(0.75),
                estimate_source: "Owned fixture".into(),
                reference_date: "2026-10-27".into(),
                ..Default::default()
            })),
            ..Default::default()
        };
        let w = reserve(&db, &p, "turn").unwrap();
        verify(&db, &w).unwrap();
        db.execute(
            "INSERT INTO preparation_turns VALUES('next','running','prep')",
            [],
        )
        .unwrap();
        assert!(reserve(&db, &p, "next").is_err());
        db.execute("UPDATE preparation_reservations SET amount=-1", [])
            .unwrap();
        p.budget.as_mut().unwrap().reserve_per_launch_usd = crate::GoFloat(0.0);
        assert!(reserve(&db, &p, "next").is_err());
        assert_eq!(
            db.query_row("SELECT count(*) FROM preparation_reservations", [], |r| r
                .get::<_, i64>(
                0
            ))
            .unwrap(),
            1
        );
    }
    #[test]
    fn triggers_cannot_replace_exact_state_or_amount() {
        let db = fixture();
        let p = Preparation {
            id: "prep".into(),
            ..Default::default()
        };
        let w = reserve(&db, &p, "turn").unwrap();
        db.execute_batch("CREATE TRIGGER drift AFTER UPDATE OF state ON preparation_reservations BEGIN UPDATE preparation_reservations SET amount=7 WHERE turn_id=NEW.turn_id; END;").unwrap();
        assert!(mark_attempt(&db, &w).is_err());
        db.execute_batch("ROLLBACK").unwrap();
        assert!(reserve(&db, &p, "turn").is_err());
    }
}

/// Reconstruct and revalidate this turn's exact reserved ledger against current
/// envelopes before contact. Own reservation is counted once, never debited
/// again. An envelope change or invalid shared ledger refuses, without refund.
pub(super) fn existing(
    db: &Connection,
    p: &Preparation,
    id: &str,
) -> Result<Witness, RootStoreError> {
    require_transaction(db)?;
    let mut s=db.prepare("SELECT preparation_id,work_id,amount,work_amount FROM preparation_reservations WHERE turn_id=? AND state='reserved'").map_err(RootStoreError::Sql)?;
    let w = s
        .query_row([id], |r| {
            Ok(Witness {
                turn: id.into(),
                prep: r.get(0)?,
                work: r.get(1)?,
                amount: r.get(2)?,
                work_amount: r.get(3)?,
            })
        })
        .map_err(RootStoreError::Sql);
    let w = finished(w, s.finalize().map_err(RootStoreError::Sql))?;
    if w.prep != p.id
        || w.work != p.work_id
        || !w.amount.is_finite()
        || !w.work_amount.is_finite()
        || w.amount < 0.0
        || w.work_amount < 0.0
    {
        return Err(RootStoreError::Refused(WITNESS_UNMATCHED));
    }
    if let Some(b) = &p.budget {
        validate_budget(b).map_err(|e| cause(ENVELOPE_INVALID, e))?;
        if b.limit_usd.0 > 0.0
            && (w.amount != b.reserve_per_launch_usd.0
                || ledger_sum(db, &p.id, PREPARATION_USED_ROWS, PREPARATION_USED_SUM)
                    .map_err(|e| cause(ENVELOPE_INVALID, e))?
                    > b.limit_usd.0)
        {
            return Err(RootStoreError::Refused(PREPARATION_EXHAUSTED));
        }
    }
    if !p.work_id.is_empty() {
        read_recorded_work(db, &p.work_id).map_err(RootStoreError::Work)?;
        if let Some(b) = read_budget(db, &p.work_id).map_err(|e| cause(ENVELOPE_INVALID, e))? {
            if b.limit_usd.0 > 0.0 {
                let a = ledger_sum(db, &p.work_id, AGENT_ASSIST_ROWS, AGENT_ASSIST_SUM)
                    .map_err(|e| cause(ENVELOPE_INVALID, e))?;
                let pr = ledger_sum(db, &p.work_id, PREPARATION_WORK_ROWS, PREPARATION_WORK_SUM)
                    .map_err(|e| cause(ENVELOPE_INVALID, e))?;
                let pl = ledger_sum(db, &p.work_id, PLANNING_ROWS, PLANNING_SUM)
                    .map_err(|e| cause(ENVELOPE_INVALID, e))?;
                if w.work_amount != b.reserve_per_launch_usd.0
                    || checked_add(checked_add(a, pr)?, pl)? > b.limit_usd.0
                {
                    return Err(RootStoreError::Refused(WORK_EXHAUSTED));
                }
            }
        }
    }
    verify(db, &w)?;
    Ok(w)
}
