//! Test fixture only. The caller supplies an owned root initialized by Go.
use rusqlite::{Connection, OpenFlags};
use std::{
    error::Error,
    fmt,
    io::{self, Write},
    path::Path,
};
use swarm_engine::{
    read_identity_plan_stamp, register_identity_plan_projection, upgrade_identity_plan_projection,
    IdentityPlanProjection::{V1, V2},
};
type Cause = Box<dyn Error>;
macro_rules! output {
    ($($args:tt)*) => {{
        let mut stdout = io::stdout().lock();
        writeln!(stdout, $($args)*)?;
        stdout.flush()?;
    }};
}
struct Causes(Vec<Cause>);
impl fmt::Display for Causes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "identity plan fixture failed ({} causes)", self.0.len())?;
        for cause in &self.0 {
            if let Some(error) = cause.downcast_ref::<swarm_engine::IdentityPlanUpgradeError>() {
                write!(f, "; {}", error.code())?;
                if let Some(sql) = error
                    .source()
                    .and_then(|v| v.downcast_ref::<rusqlite::Error>())
                {
                    if let Some(code) = sql.sqlite_error_code() {
                        write!(f, " ({code:?})")?;
                    }
                }
            }
        }
        Ok(())
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
fn run(db: &mut Connection, mode: &str) -> Result<(), Cause> {
    db.busy_timeout(std::time::Duration::ZERO)?;
    db.execute_batch("PRAGMA foreign_keys=ON;")?;
    register_identity_plan_projection(db)?;
    if mode == "verify" {
        let stamp = read_identity_plan_stamp(db, "legacy", V2)?;
        output!(
            "{}",
            serde_json::json!({"mode":mode,"generation":stamp.generation,"digest":stamp.digest,"projection":stamp.projection as i64})
        );
        return Ok(());
    }
    let before = read_identity_plan_stamp(db, "legacy", V1)?;
    let mut tx = db.transaction()?;
    let result = upgrade_identity_plan_projection(&mut tx);
    let stamps = match result {
        Ok(v) => v,
        Err(primary) => {
            return match tx.rollback() {
                Ok(()) => Err(primary.into()),
                Err(cleanup) => Err(Box::new(Causes(vec![primary.into(), cleanup.into()]))),
            }
        }
    };
    if stamps.len() != 1 || stamps[0].generation != before.generation + 1 {
        tx.rollback()?;
        return Err("fixture transition differs".into());
    }
    if mode == "commit" {
        tx.commit()?;
    } else {
        tx.rollback()?;
    }
    let after = read_identity_plan_stamp(db, "legacy", if mode == "commit" { V2 } else { V1 })?;
    if mode == "rollback" && after != before {
        return Err("fixture rollback differs".into());
    }
    output!(
        "{}",
        serde_json::json!({"mode":mode,"generation":after.generation,"digest":after.digest,"projection":after.projection as i64,"global_schema_unchanged":db.query_row("PRAGMA user_version",[],|r|r.get::<_,i64>(0))?==37})
    );
    Ok(())
}
fn main() -> Result<(), Cause> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 3 {
        return Err("owned fixture root and mode required".into());
    }
    let root = Path::new(&args[1]);
    let mode = args[2].to_str().ok_or("invalid mode")?;
    if !root.is_absolute() || !matches!(mode, "rollback" | "commit" | "verify") {
        return Err("invalid fixture request".into());
    }
    let marker = root.join(".rust-migration-fixture");
    if !std::fs::symlink_metadata(marker)?.file_type().is_file() {
        return Err("owned fixture marker required".into());
    }
    let mut db = Connection::open_with_flags(
        root.join(".swarm/state.db"),
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    let result = run(&mut db, mode);
    let close = db.close().map_err(|(_, e)| e);
    let mut causes = Vec::<Cause>::new();
    if let Err(e) = result {
        causes.push(e)
    }
    if let Err(e) = close {
        causes.push(e.into())
    }
    if causes.is_empty() {
        Ok(())
    } else {
        Err(Box::new(Causes(causes)))
    }
}
