#![forbid(unsafe_code)]
use std::{io, process::ExitCode};
fn main() -> ExitCode {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let result = swarm_cli::run_os_detailed(
        &args,
        &std::env::var("SWARM_LANG").unwrap_or_default(),
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    );
    ExitCode::from(result.code)
}
