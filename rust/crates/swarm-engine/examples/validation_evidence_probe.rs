//! Pure supplied-task/private-file comparison only; this is not a public API or identity verifier.
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
    task: Box<serde_json::value::RawValue>,
    max_review_output_bytes: Box<serde_json::value::RawValue>,
}
/// Decode the actual Go-compatible task, require an explicit cap/null and marked
/// absolute fixture, then compare ordinary data without any state mutation.
/// Errors return code2; valid guard refusals remain normalized transport data.
fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 2 {
        return Err("one absolute owned fixture root required".into());
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
    let task = swarm_engine::decode_task(frame.task.get().as_bytes())?;
    let cap: Option<usize> = serde_json::from_str(frame.max_review_output_bytes.get())?;
    let digest = task
        .validation_policy
        .as_deref()
        .map(swarm_engine::validation_policy_digest)
        .transpose()?
        .unwrap_or_default();
    let result = swarm_engine::check_validation_evidence(root, &task, None, cap);
    let value = match result {
        Ok(Some(observation)) => {
            serde_json::json!({"ok":true,"receipt":serde_json::from_str::<serde_json::Value>(&observation.receipt_json)?,"artifacts":observation.artifacts,"observations":observation.observations,"policy_digest":digest})
        }
        Ok(None) => {
            serde_json::json!({"ok":true,"receipt":null,"artifacts":null,"observations":[],"policy_digest":digest})
        }
        Err(_) => {
            serde_json::json!({"ok":false,"receipt":null,"artifacts":null,"observations":[],"policy_digest":digest})
        }
    };
    let mut stdout = io::stdout().lock();
    stdout.write_all(&swarm_engine::encode_model(&value)?)?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(())
}
/// Failed framing, filesystem, codec and output operations never become process success.
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}
