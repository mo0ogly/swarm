//! Supplied-data codec probe; not a file/protocol/identity or acceptance verifier.
use std::{
    error::Error,
    io::{self, Write},
    process::ExitCode,
};
/// Decode one bounded transport document under the requested schema mode and write its exact model.
/// Checked argument/input/codec/output errors return code2; no state is changed.
fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 2 {
        return Err("one codec mode required".into());
    }
    let strict = match args[1].to_str() {
        Some("strict") => true,
        Some("compatible") => false,
        _ => return Err("strict or compatible required".into()),
    };
    let raw = swarm_engine::read_command_document(&mut io::stdin().lock())?;
    let context = swarm_engine::decode_managed_review_context(&raw, strict)?;
    let mut stdout = io::stdout().lock();
    stdout.write_all(&swarm_engine::encode_model(&context)?)?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(())
}
/// Preserve fixture errors on stderr with code2; success grants no execution authority.
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}
