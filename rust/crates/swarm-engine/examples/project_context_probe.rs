//! Owned-fixture transport for Root context/document reads, not an agent API.
use serde::Deserialize;
use std::{
    error::Error,
    io::{self, Write},
    path::Path,
    process::ExitCode,
};
use swarm_engine::{ProjectContext, RootStore, RootStoreError};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GuardFrame {
    role: String,
    previous: Option<ProjectContext>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeliveryFrame {
    document: String,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Frame {
    Guard(GuardFrame),
    Delivery(DeliveryFrame),
}
fn run() -> Result<u8, Box<dyn Error>> {
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
    let store = RootStore::open(root, false)?;
    let result = match frame {
        Frame::Guard(frame) => store
            .check_project_context(frame.previous.as_ref(), &frame.role)
            .map(|()| serde_json::json!({"fresh":true})),
        Frame::Delivery(frame) => store
            .review_delivery(&frame.document)
            .map(|document| serde_json::json!({"document":document})),
    };
    let closed = store.close();
    let result = match (result, closed) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(e), Ok(())) | (Ok(_), Err(e)) => Err(e),
        (Err(primary), Err(cleanup)) => Err(RootStoreError::Cleanup {
            primary: Some(Box::new(primary)),
            cleanup: Box::new(cleanup),
        }),
    };
    let (code, value) = match result {
        Ok(value) => (0, value),
        Err(e) => (
            2,
            serde_json::json!({"code":e.code(),"reason":e.to_string()}),
        ),
    };
    let bytes = swarm_engine::encode_model(&value)?;
    let mut out = io::stdout().lock();
    out.write_all(&bytes)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(code)
}
fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}
