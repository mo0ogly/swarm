//! SQL column/read fixture only. No project root, provider or runtime is opened.
use rusqlite::{types::Value, Connection};
use serde::Deserialize;
use std::{
    error::Error,
    fmt,
    io::{self, BufRead, Write},
};
type Cause = Box<dyn Error>;
#[derive(Deserialize)]
struct Row {
    #[serde(rename = "ID")]
    id: String,
    #[serde(rename = "IDKind", default)]
    id_kind: String,
    #[serde(rename = "Revision")]
    revision: String,
    #[serde(rename = "RevisionKind")]
    revision_kind: String,
    #[serde(rename = "Raw")]
    raw: String,
}
#[derive(Deserialize)]
struct Frame {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Operation")]
    operation: String,
    #[serde(rename = "ID", default)]
    id: String,
    #[serde(rename = "Rows")]
    rows: Vec<Row>,
}
fn unhex(raw: &str) -> Result<Vec<u8>, Cause> {
    if !raw.len().is_multiple_of(2) || !raw.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("invalid fixture hex".into());
    }
    Ok((0..raw.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&raw[i..i + 2], 16))
        .collect::<Result<_, _>>()?)
}
fn hex(raw: &[u8]) -> String {
    raw.iter().map(|v| format!("{v:02x}")).collect()
}
fn run(db: &Connection) -> Result<(), Cause> {
    db.busy_timeout(std::time::Duration::ZERO)?;
    db.execute_batch("CREATE TABLE works(id,revision,body);")?;
    let mut stdout = io::stdout().lock();
    for line in io::stdin().lock().lines() {
        let frame: Frame = serde_json::from_str(&line?)?;
        db.execute("DELETE FROM main.works", [])?;
        for row in frame.rows {
            let revision = match row.revision_kind.as_str() {
                "integer" => Value::Integer(row.revision.parse()?),
                "real" => Value::Real(row.revision.parse()?),
                "text" => Value::Text(row.revision),
                "blob" => Value::Blob(unhex(&row.revision)?),
                "null" => Value::Null,
                _ => return Err("unknown fixture revision kind".into()),
            };
            let sql = match row.id_kind.as_str() {
                "" | "text" => "INSERT INTO main.works VALUES(CAST(? AS TEXT),?,?)",
                "blob" => "INSERT INTO main.works VALUES(?,?,?)",
                _ => return Err("unknown fixture id kind".into()),
            };
            db.execute(
                sql,
                rusqlite::params![unhex(&row.id)?, revision, unhex(&row.raw)?],
            )?;
        }
        let result = match frame.operation.as_str() {
            "read" => swarm_engine::read_recorded_work(db, &frame.id)
                .map(|v| swarm_engine::encode_model(&v)),
            "list" => swarm_engine::list_recorded_works(db).map(|v| swarm_engine::encode_model(&v)),
            _ => return Err("unknown fixture operation".into()),
        };
        let (code, output) = match result {
            Ok(encoded) => ("ok", hex(&encoded?)),
            Err(_) => ("format", String::new()),
        };
        writeln!(
            stdout,
            "{}",
            serde_json::json!({"name":frame.name,"code":code,"output":output})
        )?;
    }
    stdout.flush()?;
    Ok(())
}
struct Causes(Vec<Cause>);
impl fmt::Display for Causes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SQL work fixture failed ({} causes)", self.0.len())
    }
}
impl fmt::Debug for Causes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for Causes {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.first().map(|v| v.as_ref())
    }
}
fn file_read(db: &Connection, operation: &str) -> Result<(), Cause> {
    db.busy_timeout(std::time::Duration::ZERO)?;
    let raw = match operation {
        "read" => swarm_engine::encode_model(&swarm_engine::read_recorded_work(db, "legacy")?)?,
        "list" => swarm_engine::encode_model(&swarm_engine::list_recorded_works(db)?)?,
        _ => return Err("unknown fixture operation".into()),
    };
    let mut stdout = io::stdout().lock();
    stdout.write_all(&raw)?;
    stdout.flush()?;
    Ok(())
}
fn main() -> Result<(), Cause> {
    let args: Vec<_> = std::env::args_os().collect();
    let (db, operation) = if args.len() == 1 {
        (Connection::open_in_memory()?, None)
    } else {
        if args.len() != 3 {
            return Err("owned fixture root and read/list required".into());
        }
        let root = std::path::Path::new(&args[1]);
        if !root.is_absolute()
            || !std::fs::symlink_metadata(root.join(".rust-migration-fixture"))?
                .file_type()
                .is_file()
        {
            return Err("owned fixture marker required".into());
        }
        let mode = args[2].to_str().ok_or("invalid fixture operation")?;
        if !matches!(mode, "read" | "list") {
            return Err("unknown fixture operation".into());
        }
        (
            Connection::open_with_flags(
                root.join(".swarm/state.db"),
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                    | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )?,
            Some(mode),
        )
    };
    let result = match operation {
        None => run(&db),
        Some(mode) => file_read(&db, mode),
    };
    let closed = db.close().map_err(|(_, e)| e);
    let mut causes = Vec::<Cause>::new();
    if let Err(e) = result {
        causes.push(e)
    }
    if let Err(e) = closed {
        causes.push(e.into())
    }
    if causes.is_empty() {
        Ok(())
    } else {
        Err(Box::new(Causes(causes)))
    }
}
