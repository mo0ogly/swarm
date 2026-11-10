//! Gated owned-fixture persistence probe. Not a product command or authority.
use rusqlite::{types::ValueRef, Connection};
use serde::Deserialize;
use std::{
    error::Error,
    io::{self, BufRead, Write},
};
type Cause = Box<dyn Error>;
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Frame {
    name: String,
    existing: String,
    candidate: String,
    expected: i64,
    event: String,
    kind: String,
    request: String,
    fault: String,
    #[serde(default)]
    hook: String,
    #[serde(default)]
    rollback: bool,
}
fn unhex(s: &str) -> Result<Vec<u8>, Cause> {
    if !s.len().is_multiple_of(2) || !s.bytes().all(|v| v.is_ascii_hexdigit()) {
        return Err("invalid fixture hex".into());
    }
    Ok((0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect::<Result<_, _>>()?)
}
fn hex(b: &[u8]) -> String {
    b.iter().map(|v| format!("{v:02x}")).collect()
}
fn faults(name: &str) -> Result<&'static str, Cause> {
    Ok(match name{
 ""=>"",
 "event-abort"=>"CREATE TRIGGER injected BEFORE INSERT ON events BEGIN SELECT RAISE(ABORT,'fixture');END",
 "event-rollback"=>"CREATE TRIGGER injected BEFORE INSERT ON events BEGIN SELECT RAISE(ROLLBACK,'fixture');END",
 "event-ignore"=>"CREATE TRIGGER injected BEFORE INSERT ON events BEGIN SELECT RAISE(IGNORE);END",
 "work-ignore"=>"CREATE TRIGGER injected BEFORE INSERT ON works BEGIN SELECT RAISE(IGNORE);END",
 "event-tamper"=>"CREATE TRIGGER injected AFTER INSERT ON events BEGIN UPDATE events SET payload=x'7b7d' WHERE id=NEW.id;END",
 "work-tamper"=>"CREATE TRIGGER injected AFTER INSERT ON events BEGIN UPDATE works SET body=x'7b7d' WHERE id=NEW.work_id;END",
 _=>return Err("unknown fixture fault".into())})
}
fn initialize(db: &Connection, f: &Frame) -> Result<(), Cause> {
    db.execute_batch("PRAGMA user_version=37;CREATE TABLE works(id TEXT PRIMARY KEY,revision INTEGER NOT NULL,body BLOB NOT NULL);CREATE TABLE events(id TEXT PRIMARY KEY,work_id TEXT NOT NULL REFERENCES works(id),revision INTEGER NOT NULL,kind TEXT NOT NULL,at TEXT NOT NULL,payload BLOB NOT NULL,request BLOB NOT NULL,UNIQUE(work_id,revision))")?;
    if !f.existing.is_empty() {
        let w = swarm_engine::decode_work(&unhex(&f.existing)?)?;
        let raw = swarm_engine::encode_model(&w)?;
        db.execute(
            "INSERT INTO works VALUES(?,?,?)",
            rusqlite::params![w.id, w.revision, raw],
        )?;
        db.execute(
            "INSERT INTO events VALUES('seed',?,?,'seed',?,x'7b7d',x'7b7d')",
            rusqlite::params![w.id, w.revision, w.updated],
        )?;
    }
    db.execute_batch(faults(&f.fault)?)?;
    if !f.hook.is_empty() {
        db.execute_batch("CREATE TABLE hook_notes(value TEXT)")?;
    }
    Ok(())
}
fn snapshot(db: &Connection) -> Result<serde_json::Value, Cause> {
    let mut output = serde_json::json!({});
    for (key,query)in[("works","SELECT hex(id),revision,hex(body) FROM works ORDER BY id"),("events","SELECT hex(id),hex(work_id),revision,hex(kind),hex(at),hex(payload),hex(request) FROM events ORDER BY id")]{
  let mut stmt=db.prepare(query)?;let count=stmt.column_count();let result=(||->Result<Vec<serde_json::Value>,Cause>{let mut rows=stmt.query([])?;let mut out=Vec::new();while let Some(row)=rows.next()?{let mut cols=Vec::<serde_json::Value>::new();for column in 0..count{cols.push(match row.get_ref(column)?{ValueRef::Integer(v)=>v.into(),ValueRef::Text(v)=>std::str::from_utf8(v)?.into(),_=>return Err("unexpected fixture snapshot type".into())});}out.push(cols.into());}Ok(out)})();
  let closed=stmt.finalize();let data=match(result,closed){(Ok(v),Ok(()))=>v,(Err(e),Ok(()))=>return Err(e),(Ok(_),Err(e))=>return Err(e.into()),(Err(a),Err(b))=>return Err(Box::new(Causes(vec![a,b.into()]))) };output[key]=data.into();
 }
    Ok(output)
}
fn apply(db: &mut Connection, f: &Frame) -> Result<serde_json::Value, Cause> {
    let work = swarm_engine::decode_work(&unhex(&f.candidate)?)?;
    let request = unhex(&f.request)?;
    let mut tx = db.transaction()?;
    let result = swarm_engine::stage_recorded_change_with_hook(
        &mut tx,
        swarm_engine::RecordedChange {
            work: &work,
            expected_revision: f.expected,
            event: &f.event,
            kind: &f.kind,
            request: &request,
        },
        |db, w| {
            if f.hook.is_empty() {
                return Ok(());
            }
            let revision: i64 =
                db.query_row("SELECT revision FROM works WHERE id=?", [&w.id], |r| {
                    r.get(0)
                })?;
            let count: i64 =
                db.query_row("SELECT count(*) FROM events WHERE id=?", [&f.event], |r| {
                    r.get(0)
                })?;
            if revision != w.revision || count != 0 {
                return Err(io::Error::other("hook order mismatch").into());
            }
            db.execute(
                "INSERT INTO hook_notes VALUES(?)",
                [format!("{}:{revision}:{count}", w.id)],
            )?;
            match f.hook.as_str() {
                "observe" => {}
                "refuse" => return Err(io::Error::other("private hook refusal").into()),
                "sql-refuse" => {
                    db.execute("INSERT INTO missing_hook_table VALUES(1)", [])?;
                }
                "work-tamper" => {
                    db.execute("UPDATE works SET body=x'7b7d' WHERE id=?", [&w.id])?;
                }
                "version-tamper" => db.execute_batch("PRAGMA user_version=99")?,
                "commit" => db.execute_batch("COMMIT")?,
                "rollback" => db.execute_batch("ROLLBACK")?,
                _ => return Err(io::Error::other("unknown hook fixture").into()),
            }
            Ok(())
        },
    );
    let (code, replayed) = match result {
        Ok(swarm_engine::RecordedChangeOutcome::Staged) if f.rollback => {
            tx.rollback()?;
            ("staged", None)
        }
        Ok(swarm_engine::RecordedChangeOutcome::Staged) => match tx.commit() {
            Ok(()) => ("ok", None),
            Err(e) => {
                writeln!(io::stderr().lock(), "fixture outer commit refused: {e}")?;
                ("reject", None)
            }
        },
        Ok(swarm_engine::RecordedChangeOutcome::Replayed(w)) => {
            tx.rollback()?;
            ("replay", Some(w))
        }
        Err(e) => {
            let lost = e.outer_transaction_lost();
            writeln!(
                io::stderr().lock(),
                "fixture change refused: {e}; outer_lost={lost}"
            )?;
            if lost {
                drop(tx)
            } else if let Err(cleanup) = tx.rollback() {
                return Err(Box::new(Causes(vec![Box::new(e), cleanup.into()])));
            };
            ("reject", None)
        }
    };
    let mut output = snapshot(db)?;
    output["name"] = f.name.clone().into();
    output["code"] = code.into();
    if !f.hook.is_empty() {
        let mut statement = db.prepare("SELECT value FROM hook_notes ORDER BY rowid")?;
        let notes = statement
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        statement.finalize()?;
        output["hook_notes"] = notes.into();
        output["version"] = db
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))?
            .into();
    }
    if let Some(w) = replayed {
        output["current"] = hex(&swarm_engine::encode_model(&w)?).into()
    }
    Ok(output)
}
#[derive(Debug)]
struct Causes(Vec<Cause>);
impl std::fmt::Display for Causes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "change fixture failed ({} causes)", self.0.len())
    }
}
impl Error for Causes {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.first().map(|v| v.as_ref())
    }
}
fn one(f: &Frame, root: Option<&std::path::Path>) -> Result<serde_json::Value, Cause> {
    let mut db = match root {
        None => Connection::open_in_memory()?,
        Some(root) => Connection::open_with_flags(
            root.join(".swarm/state.db"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?,
    };
    let result = (|| {
        db.busy_timeout(std::time::Duration::ZERO)?;
        db.pragma_update(None, "foreign_keys", true)?;
        swarm_engine::register_identity_plan_projection(&db)?;
        if root.is_none() {
            initialize(&db, f)?
        }
        apply(&mut db, f)
    })();
    let closed = db.close().map_err(|(_, e)| e);
    match (result, closed) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(e.into()),
        (Err(a), Err(b)) => Err(Box::new(Causes(vec![a, b.into()]))),
    }
}
fn main() -> Result<(), Cause> {
    let args: Vec<_> = std::env::args_os().collect();
    let root = match args.len() {
        1 => None,
        2 => {
            let root = std::path::Path::new(&args[1]);
            if !root.is_absolute()
                || !std::fs::symlink_metadata(root.join(".rust-migration-fixture"))?
                    .file_type()
                    .is_file()
            {
                return Err("owned fixture marker required".into());
            }
            Some(root)
        }
        _ => return Err("optional owned fixture root required".into()),
    };
    let mut out = io::stdout().lock();
    for line in io::stdin().lock().lines() {
        let frame: Frame = serde_json::from_str(&line?)?;
        let value = one(&frame, root)?;
        writeln!(out, "{value}")?;
    }
    out.flush()?;
    Ok(())
}
