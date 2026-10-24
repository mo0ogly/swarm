//! Marked local fixture transport, not actor authentication or HTTP acceptance.
use std::{
    error::Error,
    io::{self, Write},
    path::Path,
    process::ExitCode,
};
/// Observe the actual Root policy/metadata or create a Work through its supported
/// boundary. Private process output is never rendered; checked close precedes
/// output. Failure data retains safe reason/cause presence and returns code2.
fn run() -> Result<u8, Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 3 {
        return Err("mode and marked fixture root required".into());
    }
    let root = Path::new(&args[2]);
    if !root.is_absolute()
        || !std::fs::symlink_metadata(root.join(".rust-migration-fixture"))?
            .file_type()
            .is_file()
    {
        return Err("owned fixture marker required".into());
    }
    if args[1] == "legacy" {
        let o = swarm_engine::observe_git(root)?;
        let value = serde_json::json!({"scope":"legacy-display-data-only","state":o.state,"issues":o.issues.len()});
        let mut out = io::stdout().lock();
        out.write_all(&swarm_engine::encode_model(&value)?)?;
        out.write_all(b"\n")?;
        out.flush()?;
        return Ok(0);
    }
    let mut store = swarm_engine::RootStore::open(root, args[1] == "create")?;
    let result = (|| -> Result<serde_json::Value, swarm_engine::RootStoreError> {
        if args[1] == "read" {
            let o = store.inspect_git_metadata()?;
            return Ok(
                serde_json::json!({"scope":"git-display-data-only","state":o.state,"diagnostics":o.diagnostics,"issues":o.issues.len(),"external_filters_disabled":o.external_filters_disabled,"settings_revision":o.settings_revision}),
            );
        }
        if args[1] == "create" {
            let raw = swarm_engine::read_command_document(&mut io::stdin().lock())
                .map_err(swarm_engine::RootStoreError::Input)?;
            let result = store.execute_external(
                "",
                "work.create",
                &raw,
                &mut swarm_engine::SystemEnvironment,
            )?;
            return Ok(
                serde_json::json!({"work":result.result.work,"issues":result.result.git_issues.len()}),
            );
        }
        Err(swarm_engine::RootStoreError::Refused(
            "known-metadata-fixture-mode-required",
        ))
    })();
    let result = match (result, store.close()) {
        (result, Ok(())) => result,
        (Ok(_), Err(e)) => Err(e),
        (Err(primary), Err(cleanup)) => Err(swarm_engine::RootStoreError::Cleanup {
            primary: Some(Box::new(primary)),
            cleanup: Box::new(cleanup),
        }),
    };
    let (code, value) = match result {
        Ok(v) => (0, v),
        Err(e) => {
            let mut cause: Option<&(dyn Error + 'static)> = Some(&e);
            let mut reason = "metadata-observation-refused";
            while let Some(c) = cause {
                if let Some(g) = c.downcast_ref::<swarm_engine::GitMetadataError>() {
                    reason = g.reason();
                    break;
                }
                if let Some(s) = c.downcast_ref::<swarm_engine::EngineSettingsError>() {
                    reason = s.reason();
                    break;
                }
                cause = c.source();
            }
            (
                2,
                serde_json::json!({"code":e.code(),"reason":reason,"source_present":e.source().is_some()}),
            )
        }
    };
    let mut out = io::stdout().lock();
    out.write_all(&swarm_engine::encode_model(&value)?)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(code)
}
fn main() -> ExitCode {
    match run() {
        Ok(c) => ExitCode::from(c),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}
