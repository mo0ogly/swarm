//! Private supplied-data fixture; actual Root transaction proof is separate.
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
    mode: String,
    #[serde(default)]
    action: String,
    #[serde(default)]
    at: String,
    request: Box<serde_json::value::RawValue>,
    #[serde(default)]
    work: Option<Box<serde_json::value::RawValue>>,
}
/// Read bounded explicit fixture input, decode/apply supplied data with actual
/// saved settings, close Root and flush safe JSON. No Store write/provider or
/// identity/acceptance occurs. Codec/controls report safe categories; setup and
/// checked-close/output errors refuse rather than manufacturing an empty result.
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
    let frame: Frame = serde_json::from_slice(&swarm_engine::read_command_document(
        &mut io::stdin().lock(),
    )?)?;
    let result = match swarm_engine::decode_planning_request(frame.request.get().as_bytes()) {
        Err(error) => serde_json::json!({"code":error.code()}),
        Ok(request) if frame.mode == "codec" => serde_json::json!({"code":"ok","request":request}),
        Ok(request) if frame.mode == "apply" => {
            let action = match frame.action.as_str() {
                "enable" => swarm_engine::PlanningControlAction::Enable,
                "pause" => swarm_engine::PlanningControlAction::Pause,
                "resume" => swarm_engine::PlanningControlAction::Resume,
                _ => return Err("enable/pause/resume required".into()),
            };
            let work =
                swarm_engine::decode_work(frame.work.ok_or("work required")?.get().as_bytes())?;
            let store = swarm_engine::RootStore::open(root, false)?;
            let settings = store.settings();
            let closed = store.close();
            let settings = match (settings, closed) {
                (Ok(doc), Ok(())) => doc,
                (Err(e), Ok(())) | (Ok(_), Err(e)) => return Err(Box::new(e)),
                (Err(primary), Err(cleanup)) => {
                    return Err(Box::new(Combined { primary, cleanup }))
                }
            };
            match swarm_engine::apply_planning_control(
                &work, action, &request, &frame.at, &settings,
            ) {
                Ok(work) => serde_json::json!({"code":"ok","work":work}),
                Err(error) => serde_json::json!({"code":error.code(),"reason":error.reason()}),
            }
        }
        Ok(_) => return Err("codec/apply required".into()),
    };
    let raw = swarm_engine::encode_public_model(&result)?;
    let mut out = io::stdout().lock();
    out.write_all(&raw)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(ExitCode::SUCCESS)
}
/// Retain both settings observation and release causes without private rendering.
struct Combined {
    primary: swarm_engine::RootStoreError,
    cleanup: swarm_engine::RootStoreError,
}
impl std::fmt::Debug for Combined {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "planning-fixture-cleanup: {}", self.cleanup.code())
    }
}
impl std::fmt::Display for Combined {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}
impl Error for Combined {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.primary)
    }
}
fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}
