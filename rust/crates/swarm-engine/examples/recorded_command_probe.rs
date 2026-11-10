//! Gated prefix fixture. No product command, full mutation or authority.
use rusqlite::{types::Value, Connection};
use serde::Deserialize;
use std::{
    error::Error,
    io::{self, BufRead, Write},
};
type Cause = Box<dyn Error>;
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Cell {
    kind: String,
    data: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Row {
    #[serde(rename = "ID")]
    id: String,
    revision: i64,
    raw: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Frame {
    name: String,
    #[serde(default)]
    version: i64,
    #[serde(rename = "ID")]
    id: String,
    kind: String,
    event: String,
    request: String,
    #[serde(default)]
    works: Vec<Row>,
    #[serde(default)]
    events: Vec<Vec<Cell>>,
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
fn prepare(db: &Connection, f: &Frame) -> Result<(), Cause> {
    db.execute_batch("DELETE FROM works;DELETE FROM events")?;
    db.pragma_update(None, "user_version", f.version)?;
    for row in &f.works {
        db.execute(
            "INSERT INTO works VALUES(?,?,?)",
            rusqlite::params![row.id, row.revision, unhex(&row.raw)?],
        )?;
    }
    for row in &f.events {
        if row.len() != 4 {
            return Err("four fixture columns required".into());
        }
        let mut sql = String::from("INSERT INTO events VALUES(");
        let mut args = Vec::new();
        for (i, cell) in row.iter().enumerate() {
            if i > 0 {
                sql.push(',')
            };
            sql.push_str(if cell.kind == "text" {
                "CAST(? AS TEXT)"
            } else {
                "?"
            });
            args.push(match cell.kind.as_str() {
                "null" => Value::Null,
                "blob" | "text" => Value::Blob(unhex(&cell.data)?),
                "integer" => Value::Integer(cell.data.parse()?),
                "real" => Value::Real(cell.data.parse()?),
                _ => return Err("unknown fixture type".into()),
            });
        }
        sql.push(')');
        db.execute(&sql, rusqlite::params_from_iter(args))?;
    }
    Ok(())
}
fn inspect(db: &mut Connection, f: &Frame) -> Result<serde_json::Value, Cause> {
    let request = unhex(&f.request)?;
    let tx = db.transaction()?;
    let result = match swarm_engine::reserve_recorded_command(
        &tx, &f.id, &f.kind, &f.event, &request,
    ) {
        Ok(None) => serde_json::json!({"name":f.name,"code":"new","output":""}),
        Ok(Some(w)) => {
            serde_json::json!({"name":f.name,"code":"replay","output":hex(&swarm_engine::encode_model(&w)?)})
        }
        Err(e) => {
            serde_json::json!({"name":f.name,"code":match e.code(){"storage_version_conflict"=>"storage_version_conflict","event_conflict"=>"event_conflict",_=>"format"},"output":""})
        }
    };
    tx.rollback()?;
    Ok(result)
}
fn run(db: &mut Connection, file: bool) -> Result<(), Cause> {
    if !file {
        db.execute_batch(
            "CREATE TABLE works(id,revision,body);CREATE TABLE events(id,work_id,kind,request)",
        )?;
    }
    let mut out = io::stdout().lock();
    for line in io::stdin().lock().lines() {
        let frame: Frame = serde_json::from_str(&line?)?;
        if !file {
            prepare(db, &frame)?
        }
        let result = inspect(db, &frame)?;
        writeln!(out, "{result}")?;
    }
    out.flush()?;
    Ok(())
}
#[derive(Debug)]
struct Causes(Vec<Cause>);
impl std::fmt::Display for Causes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "command fixture failed ({} causes)", self.0.len())
    }
}
impl Error for Causes {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.first().map(|v| v.as_ref())
    }
}
fn main() -> Result<(), Cause> {
    let args: Vec<_> = std::env::args_os().collect();
    let file = args.len() != 1;
    let mut db = if file {
        if args.len() != 2 {
            return Err("owned fixture root required".into());
        }
        let root = std::path::Path::new(&args[1]);
        if !root.is_absolute()
            || !std::fs::symlink_metadata(root.join(".rust-migration-fixture"))?
                .file_type()
                .is_file()
        {
            return Err("owned fixture marker required".into());
        }
        Connection::open_with_flags(
            root.join(".swarm/state.db"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?
    } else {
        Connection::open_in_memory()?
    };
    db.busy_timeout(std::time::Duration::ZERO)?;
    db.pragma_update(None, "foreign_keys", true)?;
    swarm_engine::register_identity_plan_projection(&db)?;
    let result = run(&mut db, file);
    let closed = db.close().map_err(|(_, e)| e);
    let mut errors = Vec::<Cause>::new();
    if let Err(e) = result {
        errors.push(e)
    }
    if let Err(e) = closed {
        errors.push(e.into())
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(Box::new(Causes(errors)))
    }
}
