//! Raw-byte fixture transport for Root gate; not an installed/public gate API.
use serde::Serialize;
use std::{
    error::Error,
    io::{self, Write},
    path::Path,
};
use swarm_engine::{RootStore, RootStoreError, SystemEnvironment};
#[derive(Serialize)]
struct Reply<'a> {
    code: &'static str,
    work: &'a swarm_engine::Work,
    replayed: bool,
    settings_revision: swarm_engine::SettingCount,
}
fn finish<T>(
    result: Result<T, RootStoreError>,
    closed: Result<(), RootStoreError>,
) -> Result<T, RootStoreError> {
    match (result, closed) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(e),
        (Err(primary), Err(cleanup)) => Err(RootStoreError::Cleanup {
            primary: Some(Box::new(primary)),
            cleanup: Box::new(cleanup),
        }),
    }
}
fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    if !(args.len() == 3 || args.len() == 4) {
        return Err("owned fixture root/work and optional inspection task required".into());
    }
    let root = Path::new(&args[1]);
    if !root.is_absolute()
        || !std::fs::symlink_metadata(root.join(".rust-migration-fixture"))?
            .file_type()
            .is_file()
    {
        return Err("owned fixture marker required".into());
    }
    let work = args[2].to_str().ok_or("UTF8 work identifier required")?;
    let task = if args.len() == 4 {
        Some(args[3].to_str().ok_or("UTF8 task identifier required")?)
    } else {
        None
    };
    let raw = if args.len() == 3 {
        Some(swarm_engine::read_command_document(
            &mut io::stdin().lock(),
        )?)
    } else {
        None
    };
    let mut store = RootStore::open(root, args.len() == 3)?;
    let result = if let Some(raw) = raw {
        store
            .execute_external(work, "gate", &raw, &mut SystemEnvironment)
            .and_then(|result| {
                swarm_engine::encode_model(&Reply {
                    code: "ok",
                    work: &result.result.work,
                    replayed: result.result.replayed,
                    settings_revision: result.settings_revision,
                })
                .map_err(RootStoreError::Encode)
            })
    } else if let Some(task) = task {
        store
            .inspect_gate_evidence(work, task)
            .and_then(|result| swarm_engine::encode_model(&result).map_err(RootStoreError::Encode))
    } else {
        Err(RootStoreError::Refused("fixture-operation-invalid"))
    };
    let closed = store.close();
    let bytes = match finish(result, closed) {
        Ok(bytes) => bytes,
        Err(error) => swarm_engine::encode_model(&serde_json::json!({"code":error.code()}))?,
    };
    let mut out = io::stdout().lock();
    out.write_all(&bytes)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2)
    }
}
