//! Supplied-Work candidate/file-binding fixture; no Git or authority verification.
use std::{
    error::Error,
    io::{self, Write},
    path::Path,
    process::ExitCode,
};
/// Read supplied fixture input, perform data-only checks and write the JSON result.
/// Returns argument, input/codec, evidence or output errors; no acceptance or
/// provider execution is performed. Root access is restricted to marked fixtures.
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
    let work = swarm_engine::decode_work(&swarm_engine::read_command_document(
        &mut io::stdin().lock(),
    )?)?;
    let tasks = work.tasks.as_deref().unwrap_or_default();
    if tasks.len() != 1 {
        return Err("one fixture task required".into());
    }
    swarm_engine::inspect_managed_review_anchor(root, &work, &tasks[0].id)?;
    let raw = swarm_engine::encode_model(&serde_json::json!({"bindings_consistent":true}))?;
    let mut out = io::stdout().lock();
    out.write_all(&raw)?;
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
