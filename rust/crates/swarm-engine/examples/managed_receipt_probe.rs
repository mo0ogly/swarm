//! Marked fixture transport for the actual Root receipt observer; no public API.
use std::{
    error::Error,
    io::{self, Write},
    path::Path,
    process::ExitCode,
};
/// Decode bounded transport input, observe actual stored row/settings and close
/// before emitting safe JSON. Root and output errors retain causes; raw receipt
/// content is never emitted on failure. No provider, acceptance or identity proof.
fn run() -> Result<ExitCode, Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 2 {
        return Err("one owned fixture root required".into());
    }
    let root = Path::new(&args[1]);
    if !root.is_absolute()
        || !std::fs::symlink_metadata(root.join(".rust-migration-fixture"))?
            .file_type()
            .is_file()
    {
        return Err("owned fixture marker required".into());
    }
    let frame: serde_json::Value = serde_json::from_slice(&swarm_engine::read_command_document(
        &mut io::stdin().lock(),
    )?)?;
    let work = frame["work"].as_str().ok_or("work required")?;
    let task = frame["task"].as_str().ok_or("task required")?;
    let store = swarm_engine::RootStore::open(root, false)?;
    let result = store.inspect_managed_receipt(work, task);
    let closed = store.close();
    let (value, exit) = match result {
        Ok(observation) => {
            closed?;
            (
                serde_json::json!({"observation": observation}),
                ExitCode::SUCCESS,
            )
        }
        Err(error) => {
            let cleanup = closed.err().map(|e| e.to_string());
            (
                serde_json::json!({"reason":error.to_string(),"cleanup":cleanup}),
                ExitCode::from(2),
            )
        }
    };
    let raw = swarm_engine::encode_model(&value)?;
    let mut output = io::stdout().lock();
    output.write_all(&raw)?;
    output.write_all(b"\n")?;
    output.flush()?;
    Ok(exit)
}
fn main() -> ExitCode {
    match run() {
        Ok(exit) => exit,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}
