//! Gated isolated guard fixture, not a product dispatcher or authority boundary.
use rusqlite::{params, Connection, OpenFlags};
use serde::Deserialize;
use std::{
    error::Error,
    io::{self, BufRead, Write},
    path::Path,
};
use swarm_engine::{
    decode_work, inspect_recorded_mutation, task_review_contract, MutationInspection,
};
type Cause = Box<dyn Error>;
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Frame {
    name: String,
    body: String,
    #[serde(rename = "ID")]
    id: String,
    kind: String,
    event: String,
    request: String,
    expected: i64,
    #[serde(rename = "SQL")]
    sql: String,
}
fn unhex(s: &str) -> Result<Vec<u8>, Cause> {
    if !s.len().is_multiple_of(2) || !s.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err("invalid fixture hex".into());
    };
    Ok((0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect::<Result<_, _>>()?)
}
fn one(f: Frame, root: Option<&Path>) -> Result<serde_json::Value, Cause> {
    let mut db = if let Some(root) = root {
        if !root.is_absolute()
            || !root.join(".rust-migration-fixture").is_file()
            || !f.body.is_empty()
            || !f.sql.is_empty()
        {
            return Err("owned absolute fixture root required; frame SQL/body forbidden".into());
        }
        let db = Connection::open_with_flags(
            root.join(".swarm/state.db"),
            OpenFlags::SQLITE_OPEN_READ_WRITE,
        )?;
        swarm_engine::register_identity_plan_projection(&db)?;
        db
    } else {
        Connection::open_in_memory()?
    };
    db.busy_timeout(std::time::Duration::ZERO)?;
    if root.is_none() {
        db.execute_batch("PRAGMA user_version=37;CREATE TABLE works(id TEXT PRIMARY KEY,revision INTEGER,body BLOB);CREATE TABLE events(id TEXT PRIMARY KEY,work_id TEXT,kind TEXT,request BLOB);CREATE TABLE agents(work_id TEXT,task_id TEXT,status TEXT);CREATE TABLE cockpit_controls(work_id TEXT,paused);CREATE TABLE preparation_launch_locks(work_id TEXT,task_id TEXT,released INTEGER)")?;
    }
    let mut contract = String::new();
    if !f.body.is_empty() {
        let bytes = unhex(&f.body)?;
        let w = decode_work(&bytes)?;
        if let Some(t) = w.tasks.as_deref().and_then(|t| t.first()) {
            contract = task_review_contract(t)?
        };
        db.execute(
            "INSERT INTO works VALUES(?,?,?)",
            params![w.id, w.revision, bytes],
        )?;
    }
    db.execute_batch(&f.sql)?;
    let request = unhex(&f.request)?;
    let tx = db.transaction()?;
    let code = match inspect_recorded_mutation(&tx, &f.id, &f.kind, &f.event, f.expected, &request)
    {
        Ok(MutationInspection::Create) => "create",
        Ok(MutationInspection::Existing(_)) => "existing",
        Ok(MutationInspection::Replayed(_)) => "replay",
        Err(e) => match e.code() {
            "event_conflict" => "event_conflict",
            "revision_conflict" => "revision_conflict",
            "active_agent" => "active_agent",
            "storage_version_conflict" => "storage_version_conflict",
            _ => "format",
        },
    };
    tx.rollback()?;
    db.close().map_err(|(_, e)| e)?;
    Ok(serde_json::json!({"name":f.name,"code":code,"contract":contract}))
}
fn run() -> Result<(), Cause> {
    let root = std::env::args_os().nth(1).map(std::path::PathBuf::from);
    let mut out = io::BufWriter::new(io::stdout().lock());
    for line in io::stdin().lock().lines() {
        let f = serde_json::from_str(&line?)?;
        serde_json::to_writer(&mut out, &one(f, root.as_deref())?)?;
        out.write_all(b"\n")?
    }
    out.flush()?;
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(2)
    }
}
