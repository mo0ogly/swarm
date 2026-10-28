//! Raw fixture transport for the native Work library; not a public Work API.
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    io::{self, BufRead, Write},
    path::Path,
};
use swarm_engine::{RootStore, RootStoreError, SystemEnvironment};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Frame {
    work: String,
    kind: String,
    request: Box<serde_json::value::RawValue>,
}
#[derive(Serialize)]
struct Reply<'a> {
    code: &'static str,
    work: &'a swarm_engine::Work,
    replayed: bool,
    settings_revision: swarm_engine::SettingCount,
    storage_attempts: u64,
}
fn one(root: &Path, frame: Frame) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut store = RootStore::open(root, true)?;
    let result = store.execute_external(
        &frame.work,
        &frame.kind,
        frame.request.get().as_bytes(),
        &mut SystemEnvironment,
    );
    let closed = store.close();
    let result = match (result, closed) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(e),
        (Err(primary), Err(cleanup)) => Err(RootStoreError::Cleanup {
            primary: Some(Box::new(primary)),
            cleanup: Box::new(cleanup),
        }),
    };
    let bytes = match result {
        Ok(v) => swarm_engine::encode_model(&Reply {
            code: "ok",
            work: &v.result.work,
            replayed: v.result.replayed,
            settings_revision: v.settings_revision,
            storage_attempts: v.storage_attempts,
        })?,
        Err(e) => swarm_engine::encode_model(&serde_json::json!({"code":e.code()}))?,
    };
    Ok(bytes)
}
fn run() -> Result<(), Box<dyn Error>> {
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
        out.write_all(&one(root, serde_json::from_str(&line?)?)?)?;
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
