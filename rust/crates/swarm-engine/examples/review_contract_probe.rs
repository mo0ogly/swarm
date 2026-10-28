//! Pure contract fixture transport, no root/database/provider operation.
use std::{
    error::Error,
    io::{self, Write},
    process::ExitCode,
};
fn run() -> Result<(), Box<dyn Error>> {
    let raw = swarm_engine::read_command_document(&mut io::stdin().lock())?;
    let task = swarm_engine::decode_task(&raw)?;
    let contract = swarm_engine::review_contract(&task)?;
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
