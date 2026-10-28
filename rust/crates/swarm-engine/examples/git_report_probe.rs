//! Owned-fixture Git reader using real persisted Root policy; no agent authorization.
use serde::Deserialize;
use std::{
    error::Error,
    io::{self, Write},
    path::Path,
    process::ExitCode,
};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Frame {
    repository: String,
    candidate: String,
    report: String,
    version: i64,
}
/// Read one marked fixture request and return the Root observation or safe reason.
/// Preserves Root/close errors, checks every output write, starts only the fixed
/// local Git blob read and grants no review/acceptance/HTTP authority.
fn run() -> Result<u8, Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 2 {
        return Err("one owned root required".into());
    }
    let root = Path::new(&args[1]);
    if !root.is_absolute()
        || !std::fs::symlink_metadata(root.join(".rust-migration-fixture"))?
            .file_type()
            .is_file()
    {
        return Err("owned fixture marker required".into());
    }
    let frame: Frame = serde_json::from_slice(&swarm_engine::read_command_document(
        &mut io::stdin().lock(),
    )?)?;
    let store = swarm_engine::RootStore::open(root, false)?;
    let result = store.inspect_git_report(
        &frame.repository,
        &frame.candidate,
        &frame.report,
        frame.version,
    );
    let closed = store.close();
    let result = match (result, closed) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(e), Ok(())) | (Ok(_), Err(e)) => Err(e),
        (Err(primary), Err(cleanup)) => Err(swarm_engine::RootStoreError::Cleanup {
            primary: Some(Box::new(primary)),
            cleanup: Box::new(cleanup),
        }),
    };
    let (code, value) = match result {
        Ok(value) => (0, serde_json::to_value(value)?),
        Err(e) => (
            2,
            serde_json::json!({"code":e.code(),"reason":e.to_string()}),
        ),
    };
    let mut stdout = io::stdout().lock();
    stdout.write_all(&swarm_engine::encode_model(&value)?)?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(code)
}
/// Exit with the checked fixture result; malformed transport or output errors use code2.
fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}
