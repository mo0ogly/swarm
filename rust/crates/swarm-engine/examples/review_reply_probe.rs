//! Owned supplied-text parsing fixture; no authenticated reviewer or task acceptance.
use serde::Deserialize;
use std::{
    error::Error,
    io::{self, Write},
    path::Path,
    process::ExitCode,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Original {
    name: String,
    kind: String,
    content: String,
}
/// Explicit pure-function policy is fixture data, never a Root override.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pure {
    raw: String,
    criterion_count: usize,
    originals: Vec<Original>,
    limits: swarm_engine::ReviewReplyLimits,
}
/// Root mode receives neither a Work object, criterion count nor policy override.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    raw: String,
    work: String,
    task: String,
    originals: Vec<Original>,
}
/// Borrow UTF-8 supplied text without granting provenance to declared name/kind.
fn originals(values: &[Original]) -> Vec<swarm_engine::ReviewQuoteOriginal<'_>> {
    values
        .iter()
        .map(|v| swarm_engine::ReviewQuoteOriginal {
            name: &v.name,
            kind: &v.kind,
            content: &v.content,
        })
        .collect()
}
/// Decode pure data or an owned Root request, retaining checked close/output failures.
/// Business refusals emit safe JSON and code2; no database or journal is mutated.
fn run() -> Result<u8, Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    let raw = swarm_engine::read_command_document(&mut io::stdin().lock())?;
    let (code, value) = match args.as_slice() {
        [_, mode] if mode == "pure" => {
            let f: Pure = serde_json::from_slice(&raw)?;
            match swarm_engine::parse_review_reply(
                f.raw.as_bytes(),
                f.criterion_count,
                &originals(&f.originals),
                f.limits,
            ) {
                Ok(parsed) => (0, serde_json::json!({"valid":true,"parsed":parsed})),
                Err(e) => (
                    2,
                    serde_json::json!({"valid":false,"code":e.code(),"reason":e.reason()}),
                ),
            }
        }
        [_, mode, path] if mode == "root" => {
            let root = Path::new(path);
            if !root.is_absolute()
                || !std::fs::symlink_metadata(root.join(".rust-migration-fixture"))?
                    .file_type()
                    .is_file()
            {
                return Err("owned fixture marker required".into());
            }
            let f: Stored = serde_json::from_slice(&raw)?;
            let store = swarm_engine::RootStore::open(root, false)?;
            let result = store.inspect_review_reply(
                &f.work,
                &f.task,
                f.raw.as_bytes(),
                &originals(&f.originals),
            );
            let result = match (result, store.close()) {
                (value, Ok(())) => value,
                (value, Err(cleanup)) => Err(swarm_engine::RootStoreError::Cleanup {
                    primary: value.err().map(Box::new),
                    cleanup: Box::new(cleanup),
                }),
            };
            match result {
                Ok(observation) => (
                    0,
                    serde_json::json!({"valid":true,"observation":observation}),
                ),
                Err(e) => (
                    2,
                    serde_json::json!({"valid":false,"code":e.code(),"reason":e.to_string()}),
                ),
            }
        }
        _ => return Err("pure or root ABSOLUTE_OWNED_ROOT required".into()),
    };
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, &value)?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(code)
}
/// Propagate refusals and transport errors; process success is no acceptance evidence.
fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}
