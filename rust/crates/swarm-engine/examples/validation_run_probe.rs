//! Owned-fixture transport, not a public execution or authorization endpoint.
use serde::Deserialize;
use std::{
    error::Error,
    io::{self, BufRead, Write},
    path::Path,
};
use swarm_engine::{run_validation_control, RootStore, ValidationControl};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Frame {
    control: ValidationControl,
}
fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 2 {
        return Err("fixture root required".into());
    }
    let root = Path::new(&args[1]);
    if !root.is_absolute()
        || !std::fs::symlink_metadata(root.join(".rust-migration-fixture"))?
            .file_type()
            .is_file()
    {
        return Err("fixture marker required".into());
    }
    let mut out = io::BufWriter::new(io::stdout().lock());
    for line in io::stdin().lock().lines() {
        let frame: Frame = serde_json::from_str(&line?)?;
        let store = RootStore::open(root, false)?;
        let settings = store.settings();
        store.close()?;
        let settings = settings?;
        let value = match run_validation_control(root, &frame.control, &settings) {
            Ok(run) => {
                serde_json::json!({"code":"ok","result":run.result,"retained_hex":run.output.retained().iter().map(|b|format!("{b:02x}")).collect::<String>(),"total":run.output.total(),"settings_revision":run.settings_revision,"timed_out":run.timed_out,"group_signal_delivered":run.group_signal_delivered,"output_drain_timed_out":run.output_drain_timed_out})
            }
            Err(e) => {
                if let Some(id) = e.ownership_lost_process_id() {
                    return Err(format!("fixture child ownership lost for diagnostic PID {id}; no signal capability: {e}").into());
                }
                if let Some(id) = e.unresolved_process_id() {
                    return Err(format!("unresolved owned fixture group {id}; operator recovery required; no cleanup claim: {e}").into());
                }
                serde_json::json!({"code":e.code(),"operation":e.operation(),"cleanup_count":e.cleanup().len()})
            }
        };
        serde_json::to_writer(&mut out, &value)?;
        out.write_all(b"\n")?;
        out.flush()?;
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(2)
    }
}
