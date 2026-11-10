//! Read-only owned SQL fixture, not a stored-Work/identity/acceptance verifier.
use serde::Deserialize;
use std::{
    error::Error,
    fmt,
    io::{self, Write},
    path::Path,
    process::ExitCode,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Frame {
    work: String,
    task: Box<serde_json::value::RawValue>,
    current: Option<Box<serde_json::value::RawValue>>,
}

/// Preserve both a history error and a later connection-close error without private output.
#[derive(Debug)]
struct CloseError {
    primary: Option<swarm_engine::ControlHistoryError>,
    close: rusqlite::Error,
}
impl fmt::Display for CloseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("history-fixture-close")
    }
}
impl Error for CloseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.primary
            .as_ref()
            .map(|p| p as &dyn Error)
            .or(Some(&self.close))
    }
}
/// Decode supplied task/current data through the actual Go-compatible model shapes.
/// Read only a marked fixture DB; errors return code2, after checked close/output.
fn run() -> Result<u8, Box<dyn Error>> {
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
    let raw = swarm_engine::read_command_document(&mut io::stdin().lock())?;
    let frame: Frame = serde_json::from_slice(&raw)?;
    let task = swarm_engine::decode_task(frame.task.get().as_bytes())?;
    let current = frame
        .current
        .map(|raw| {
            swarm_engine::fixture_model_roundtrip("AutomaticValidation", raw.get().as_bytes())
                .and_then(|canonical| {
                    serde_json::from_slice::<swarm_engine::AutomaticValidation>(&canonical)
                        .map_err(swarm_engine::WorkError::Json)
                })
        })
        .transpose()?;
    let db = rusqlite::Connection::open_with_flags(
        root.join("history.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    let result =
        swarm_engine::read_validation_control_history(&db, &frame.work, &task, current.as_ref());
    if let Err((_, close)) = db.close() {
        return Err(Box::new(CloseError {
            primary: result.err(),
            close,
        }));
    }
    let (code, value) = match result {
        Ok(history) => (0, serde_json::json!({"history":history})),
        Err(error) => (
            2,
            serde_json::json!({"code":error.code(),"source_present":error.source().is_some()}),
        ),
    };
    let mut stdout = io::stdout().lock();
    stdout.write_all(&swarm_engine::encode_model(&value)?)?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(code)
}
/// Refusals and output/cleanup errors never become process success.
fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}
