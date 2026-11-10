//! Gated event fixture only: no native Store, root migration or authority.
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
struct Frame {
    name: String,
    work: String,
    rows: Vec<Vec<Cell>>,
    escape_h_t_m_l: bool,
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
fn hex(raw: &[u8]) -> String {
    raw.iter().map(|v| format!("{v:02x}")).collect()
}
fn run(db: &Connection) -> Result<(), Cause> {
    db.execute_batch("CREATE TABLE events(id,work_id,revision,kind,at,payload)")?;
    let mut output = io::stdout().lock();
    for line in io::stdin().lock().lines() {
        let frame: Frame = serde_json::from_str(&line?)?;
        db.execute("DELETE FROM events", [])?;
        for row in frame.rows {
            if row.len() != 6 {
                return Err("six fixture columns required".into());
            }
            let mut sql = String::from("INSERT INTO events VALUES(");
            let mut args = Vec::new();
            for (i, cell) in row.iter().enumerate() {
                if i > 0 {
                    sql.push(',')
                }
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
        let mut result = serde_json::json!({"name":frame.name,"code":"format","events":null,"json_code":"not-run","json_output":""});
        if let Ok(events) = swarm_engine::list_recorded_events(db, &frame.work) {
            result["code"] = "ok".into();
            result["events"]=serde_json::Value::Array(events.iter().map(|e|serde_json::json!({"id":hex(&e.id),"work_id":hex(&e.work_id),"revision":e.revision,"kind":hex(&e.kind),"at":hex(&e.at),"payload":e.payload.as_ref().map(|v|hex(v))})).collect());
            result["json_code"] = "format".into();
            if let Ok(raw) = swarm_engine::encode_recorded_events(&events, frame.escape_h_t_m_l) {
                result["json_code"] = "ok".into();
                result["json_output"] = hex(&raw).into()
            }
        }
        writeln!(output, "{result}")?;
    }
    output.flush()?;
    Ok(())
}
fn file_read(db: &Connection, work: &str) -> Result<(), Cause> {
    let events = swarm_engine::list_recorded_events(db, work)?;
    let raw = swarm_engine::encode_recorded_events(&events, false)?;
    let mut out = io::stdout().lock();
    out.write_all(&raw)?;
    out.flush()?;
    Ok(())
}
#[derive(Debug)]
struct Causes(Vec<Cause>);
impl std::fmt::Display for Causes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "event fixture failed ({} causes)", self.0.len())
    }
}
impl Error for Causes {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.first().map(|v| v.as_ref())
    }
}
fn main() -> Result<(), Cause> {
    let args: Vec<_> = std::env::args_os().collect();
    let (db, work) = if args.len() == 1 {
        (Connection::open_in_memory()?, None)
    } else {
        if args.len() != 3 {
            return Err("owned fixture root and work required".into());
        }
        let root = std::path::Path::new(&args[1]);
        if !root.is_absolute()
            || !std::fs::symlink_metadata(root.join(".rust-migration-fixture"))?
                .file_type()
                .is_file()
        {
            return Err("owned fixture marker required".into());
        }
        (
            Connection::open_with_flags(
                root.join(".swarm/state.db"),
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                    | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )?,
            Some(args[2].to_str().ok_or("invalid work")?),
        )
    };
    db.busy_timeout(std::time::Duration::ZERO)?;
    let result = match work {
        Some(work) => file_read(&db, work),
        None => run(&db),
    };
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
