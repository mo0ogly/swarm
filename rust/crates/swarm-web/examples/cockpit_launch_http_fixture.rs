//! Serve the protected native routes and embedded cockpit on an isolated Root.
//! The fixture owns no mission conductor. It accepts the repository HTTP test
//! harness's local CLI shape and must not be used as an installed server.

use std::{
    error::Error,
    io::{self, Write},
    path::Path,
};
use swarm_web::NativeHttpServer;

/// Emit one test harness framing record without credentials.
fn emit(value: serde_json::Value) -> io::Result<()> {
    let mut out = io::stdout().lock();
    writeln!(out, "{value}")?;
    out.flush()
}

/// Bind an existing private Root and run the production HTTP service until SIGINT.
/// The driver owns the session files and checks the process's exit.
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [command, flag, root, json, lang, en] = args.as_slice() else {
        return Err("expected web --root ROOT --json --lang en".into());
    };
    if (
        command.as_str(),
        flag.as_str(),
        json.as_str(),
        lang.as_str(),
        en.as_str(),
    ) != ("web", "--root", "--json", "--lang", "en")
    {
        return Err("invalid HTTP fixture arguments".into());
    }
    let server = NativeHttpServer::bind(Path::new(root), Some("127.0.0.1:0"))?;
    let address = server.local_addr()?;
    emit(serde_json::json!({
        "schema_version": 1, "event": "web-bound", "runtime_ready": false, "address": address.to_string()
    }))?;
    let mut signal = Ok(());
    let report = server.serve_until(
        async { signal = tokio::signal::ctrl_c().await },
        &mut |_| Ok(()),
    )?;
    signal?;
    emit(serde_json::json!({
        "schema_version": 1, "event": "web-stopped",
        "provider_contact_may_have_occurred": report.provider_contact_may_have_occurred,
        "work_outcomes": report.work_outcomes, "configuration_outcomes": report.configuration_outcomes
    }))?;
    if report.provider_contact_may_have_occurred
        || report.work_outcomes > 0
        || report.configuration_outcomes > 0
    {
        return Err("HTTP fixture recorded an unexpected operation effect".into());
    }
    Ok(())
}
