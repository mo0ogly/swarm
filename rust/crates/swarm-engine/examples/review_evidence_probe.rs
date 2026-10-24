//! Supplied-Work data fixture with actual Root settings; no row/authority proof.
use std::{
    error::Error,
    io::{self, Write},
    path::Path,
    process::ExitCode,
};
use swarm_engine::{RootStore, RootStoreError};
fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 3 && args.len() != 4 {
        return Err("owned root and fixture task required".into());
    }
    let current = args.len() == 4;
    if current && args[3] != "current" {
        return Err("explicit current fixture mode required".into());
    }
    let root = Path::new(&args[1]);
    if !root.is_absolute()
        || !std::fs::symlink_metadata(root.join(".rust-migration-fixture"))?
            .file_type()
            .is_file()
    {
        return Err("owned fixture marker required".into());
    }
    let task = args[2].to_str().ok_or("UTF8 task required")?;
    let work = swarm_engine::decode_work(&swarm_engine::read_command_document(
        &mut io::stdin().lock(),
    )?)?;
    let store = RootStore::open(root, false)?;
    let result = store.settings();
    let closed = store.close();
    let settings = match (result, closed) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), Ok(())) | (Ok(_), Err(e)) => Err(e),
        (Err(primary), Err(cleanup)) => Err(RootStoreError::Cleanup {
            primary: Some(Box::new(primary)),
            cleanup: Box::new(cleanup),
        }),
    }?;
    let limits = if work
        .planning
        .as_deref()
        .is_some_and(|p| p.reviewer.is_some())
    {
        Some(settings.values.context_policy_limits()?)
    } else {
        None
    };
    let observed = if current {
        swarm_engine::check_current_review_evidence(root, &work, task, &settings)?
    } else {
        swarm_engine::check_review_evidence(root, &work, task, limits.as_ref())?
    };
    let bytes = swarm_engine::encode_model(
        &serde_json::json!({"consistent":true,"configured":observed.is_some()}),
    )?;
    let mut out = io::stdout().lock();
    out.write_all(&bytes)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(())
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}
