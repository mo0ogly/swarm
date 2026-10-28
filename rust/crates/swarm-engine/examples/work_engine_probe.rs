//! Gated fixture using the real native WorkEngine and system environment.
//! Fixture root opening is not the product's migration/authority implementation.
use rusqlite::{Connection, OpenFlags};
use serde::Deserialize;
use std::{
    error::Error,
    io::{self, BufRead, Write},
    path::Path,
};
use swarm_engine::{
    encode_model, EngineMutationLimits, SystemEnvironment, TaskDefinitionLimits,
    ValidationPolicyLimits, WorkEngine,
};
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
struct Frame {
    name: String,
    work: String,
    kind: String,
    request: String,
}
type Cause = Box<dyn Error>;
fn unhex(s: &str) -> Result<Vec<u8>, Cause> {
    if !s.len().is_multiple_of(2) || !s.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err("invalid fixture hex".into());
    }
    Ok((0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect::<Result<_, _>>()?)
}
fn one(root: &Path, f: Frame) -> Result<serde_json::Value, Cause> {
    let mut db = Connection::open_with_flags(
        root.join(".swarm/state.db"),
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    db.busy_timeout(std::time::Duration::ZERO)?;
    db.pragma_update(None, "foreign_keys", true)?;
    swarm_engine::register_identity_plan_projection(&db)?;
    let mut environment = SystemEnvironment;
    let limits = EngineMutationLimits {
        task: TaskDefinitionLimits::legacy_go(),
        policy: ValidationPolicyLimits::legacy_go(),
    };
    let outcome = WorkEngine::new(&mut db, root, &mut environment, &limits).execute_external(
        &f.work,
        &f.kind,
        &unhex(&f.request)?,
    );
    let value = match outcome {
        Ok(outcome) => {
            serde_json::json!({"name":f.name,"code":"ok","body":encode_model(&outcome.work)?.iter().map(|b|format!("{b:02x}")).collect::<String>(),"replayed":outcome.replayed,"git_issues":outcome.git_issues.len(),"requires_reconciliation":false})
        }
        Err(e) => {
            serde_json::json!({"name":f.name,"code":e.code(),"body":"","replayed":false,"git_issues":0,"requires_reconciliation":e.requires_reconciliation()})
        }
    };
    db.close().map_err(|(_, e)| e)?;
    Ok(value)
}
fn run() -> Result<(), Cause> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 2 {
        return Err("owned fixture root required".into());
    }
    let root = Path::new(&args[1]);
    if !root.is_absolute()
        || !std::fs::symlink_metadata(root.join(".rust-migration-fixture"))?
            .file_type()
            .is_file()
    {
        return Err("owned fixture marker required".into());
    }
    let mut out = io::BufWriter::new(io::stdout().lock());
    for line in io::stdin().lock().lines() {
        serde_json::to_writer(&mut out, &one(root, serde_json::from_str(&line?)?)?)?;
        out.write_all(b"\n")?;
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
