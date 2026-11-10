//! Trusted finite fixture frames only; no root DB, provider, actor or commit.
use serde::Deserialize;
use serde_json::{value::RawValue, Value};
use std::{
    error::Error,
    io::{self, BufRead, Write},
};
use swarm_engine::{
    check_launch_budget, decode_budget_change, encode_model, reserve_planning_call,
    validate_budget_change, BudgetError,
};
#[derive(Deserialize)]
struct Row {
    amount: f64,
    state: String,
}
#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Frame {
    request: Box<RawValue>,
    agents: Vec<Row>,
    assist: Vec<Row>,
    prep: Vec<Row>,
    planning: Vec<Row>,
    absent: bool,
    reserve: bool,
}
impl Default for Frame {
    fn default() -> Self {
        Self {
            request: RawValue::from_string("null".into()).expect("literal JSON"),
            agents: vec![],
            assist: vec![],
            prep: vec![],
            planning: vec![],
            absent: false,
            reserve: false,
        }
    }
}
/// Retain primary and cleanup causes on fixture failure without claiming PASS.
struct Cleanup {
    primary: Option<Box<dyn Error>>,
    cleanup: rusqlite::Error,
}
impl std::fmt::Debug for Cleanup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
impl std::fmt::Display for Cleanup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("financial-fixture-cleanup-failed")
    }
}
impl Error for Cleanup {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.primary.as_deref().or(Some(&self.cleanup))
    }
}
/// Preserve primary error and checked rollback/close errors without fallback data.
fn finish(
    result: Result<Value, Box<dyn Error>>,
    cleanup: Option<rusqlite::Error>,
) -> Result<Value, Box<dyn Error>> {
    match (result, cleanup) {
        (value, None) => value,
        (value, Some(error)) => Err(Box::new(Cleanup {
            primary: value.err(),
            cleanup: error,
        })),
    }
}
/// Call real native codecs/admission/reservation on isolated memory SQL; expected
/// policy refusals are data. Unexpected codec/SQL/rollback/close failures propagate.
/// Caller framing is trusted fixture data, never a public transport or authority.
fn one(frame: Frame) -> Result<Value, Box<dyn Error>> {
    let mut change = match decode_budget_change(frame.request.get().as_bytes()) {
        Ok(c) => c,
        Err(_) => return Ok(serde_json::json!({"code":"refused"})),
    };
    if validate_budget_change(&change).is_err() {
        return Ok(serde_json::json!({"code":"refused"}));
    }
    change.budget.actor.clear();
    change.budget.updated.clear();
    let intent = String::from_utf8(encode_model(&change)?)?;
    let mut db = rusqlite::Connection::open_in_memory()?;
    let observed = (|| -> Result<Value, Box<dyn Error>> {
        db.execute_batch("CREATE TABLE budgets(work_id TEXT PRIMARY KEY,body BLOB NOT NULL);CREATE TABLE reservations(agent_id TEXT PRIMARY KEY,work_id TEXT,amount REAL,state TEXT);CREATE TABLE assist_reservations(turn_id TEXT PRIMARY KEY,work_id TEXT,amount REAL,state TEXT);CREATE TABLE preparation_reservations(turn_id TEXT PRIMARY KEY,work_id TEXT,work_amount REAL,state TEXT);CREATE TABLE planning_calls(id TEXT PRIMARY KEY,work_id TEXT,scope_id TEXT,estimate REAL,usage BLOB,state TEXT,created_at TEXT);")?;
        if !frame.absent {
            db.execute(
                "INSERT INTO budgets VALUES('work',?)",
                [encode_model(&change.budget)?],
            )?;
        }
        for (query, rows) in [
            (
                "INSERT INTO reservations VALUES(?,'work',?,?)",
                frame.agents,
            ),
            (
                "INSERT INTO assist_reservations VALUES(?,'work',?,?)",
                frame.assist,
            ),
            (
                "INSERT INTO preparation_reservations VALUES(?,'work',?,?)",
                frame.prep,
            ),
            (
                "INSERT INTO planning_calls(id,work_id,estimate,state) VALUES(?,'work',?,?)",
                frame.planning,
            ),
        ] {
            for (i, row) in rows.into_iter().enumerate() {
                db.execute(
                    query,
                    rusqlite::params![format!("row-{i}"), row.amount, row.state],
                )?;
            }
        }
        let tx = db.transaction()?;
        let observed = (|| -> Result<Value, Box<dyn Error>> {
            let code = match check_launch_budget(&tx, "work") {
                Ok(()) => {
                    if frame.reserve {
                        reserve_planning_call(&tx, "work", "root", "call", "2026-10-19T00:00:00Z")?;
                    }
                    "ok"
                }
                Err(BudgetError::Exhausted) => "refused",
                Err(e) => return Err(e.into()),
            };
            let (count, estimate): (i64, f64) = tx.query_row(
                "SELECT count(*),coalesce(sum(estimate),0) FROM planning_calls",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            Ok(
                serde_json::json!({"code":code,"intent":intent,"planning_count":count,"planning_estimate":estimate}),
            )
        })();
        finish(observed, tx.rollback().err())
    })();
    finish(observed, db.close().err().map(|(_, error)| error))
}
/// Preserve framing/IO errors and checked complete output/flush; never empty PASS.
fn run() -> Result<(), Box<dyn Error>> {
    let mut out = io::BufWriter::new(io::stdout().lock());
    for line in io::stdin().lock().lines() {
        let frame = serde_json::from_str(&line?)?;
        out.write_all(&serde_json::to_vec(&one(frame)?)?)?;
        out.write_all(b"\n")?;
    }
    out.flush()?;
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("financial-budget-fixture: {error}");
        std::process::exit(1)
    }
}
