//! UTF-8 supplied-text quote fixture; no Root policy, provenance or acceptance check.
use serde::Deserialize;
use std::{
    error::Error,
    io::{self, Write},
    process::ExitCode,
};

/// All inputs are explicit; the minimum is fixture data, not an operator default.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Frame {
    name: String,
    kind: String,
    content: String,
    evidence: String,
    minimum_normalized_bytes: usize,
}

/// Match one bounded transport document and write checked JSON. Argument, input,
/// allocation and output errors propagate; no engine state or file is changed.
fn run() -> Result<(), Box<dyn Error>> {
    if std::env::args_os().count() != 1 {
        return Err("no arguments allowed".into());
    }
    let raw = swarm_engine::read_command_document(&mut io::stdin().lock())?;
    let f: Frame = serde_json::from_slice(&raw)?;
    let present = swarm_engine::review_evidence_present(
        &f.name,
        &f.kind,
        &f.content,
        &f.evidence,
        f.minimum_normalized_bytes,
    )?;
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, &serde_json::json!({"present": present}))?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(())
}

/// Report fixture failure with code 2; a successful match grants no authority.
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}
