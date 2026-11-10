//! Pure contract fixture transport, no root/database/provider operation.
use std::{
    error::Error,
    io::{self, Write},
    process::ExitCode,
};
/// Read Work from stdin and a UTF-8 selected-task argument; write its data digest.
/// Argument, input/codec and output failures are returned, never acceptance.
/// This fixture performs no Root, provider, file mutation or policy operation.
fn run() -> Result<(), Box<dyn Error>> {
    let raw = swarm_engine::read_command_document(&mut io::stdin().lock())?;
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 2 {
        return Err("fixture task identifier required".into());
    }
    let selected = args[1]
        .to_str()
        .ok_or("UTF8 fixture task identifier required")?;
    let work = swarm_engine::decode_work(&raw)?;
    let contract = swarm_engine::managed_review_contract(&work, selected)?;
    let bytes = swarm_engine::encode_model(&serde_json::json!({"contract":contract}))?;
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
