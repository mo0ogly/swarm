//! Owned-fixture file guard with actual saved policy; supplied review data grants no authority.
use std::{
    error::Error,
    io::{self, Write},
    path::Path,
    process::ExitCode,
};
/// Read a supplied review and actual Root settings, then inspect only owned fixture files.
/// Checked errors return code2. This reads no Work row and verifies no identity or
/// acceptance; the actual stored-Work consumer calls this same guard under its lease.
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
    let raw = swarm_engine::read_command_document(&mut io::stdin().lock())?;
    let work = swarm_engine::decode_work(&raw)?;
    let task = work
        .tasks
        .as_ref()
        .filter(|t| t.len() == 1)
        .and_then(|t| t.first())
        .ok_or("exactly one fixture task required")?;
    let review = task
        .independent_review
        .as_deref()
        .ok_or("fixture review required")?;
    let store = swarm_engine::RootStore::open(root, false)?;
    let settings = store.settings();
    let closed = store.close();
    let settings = match (settings, closed) {
        (Ok(s), Ok(())) => s,
        (Err(e), Ok(())) | (Ok(_), Err(e)) => return Err(e.into()),
        (Err(primary), Err(cleanup)) => {
            return Err(swarm_engine::RootStoreError::Cleanup {
                primary: Some(Box::new(primary)),
                cleanup: Box::new(cleanup),
            }
            .into())
        }
    };
    let (code, value) = match swarm_engine::check_managed_review_files(
        root, review, &settings, false,
    ) {
        Ok(value) => (
            0,
            serde_json::json!({"files_consistent":true,"reviews_checked":value.reviews_checked,"max_depth":value.max_depth,"settings_revision":settings.revision}),
        ),
        Err(e) => (2, serde_json::json!({"code":e.code(),"reason":e.reason()})),
    };
    let mut stdout = io::stdout().lock();
    stdout.write_all(&swarm_engine::encode_model(&value)?)?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(code)
}
/// Exit with the fixture observation; never convert refusal or output failure to success.
fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}
