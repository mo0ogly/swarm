#![forbid(unsafe_code)]
use std::{io, process::ExitCode};
fn main() -> ExitCode {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    #[cfg(target_os = "linux")]
    if args.first().is_some_and(|a| a == "_native_provider_proxy") {
        return match swarm_engine::execute_native_provider_proxy(&args[1..]) {
            Ok(code) => ExitCode::from(code),
            Err(error) => {
                let diagnostic = swarm_engine::native_proxy_diagnostic(&error);
                // A failed diagnostic write is not successful delivery; the
                // process remains failed and the parent retains output loss.
                if serde_json::to_writer(&mut io::stderr().lock(), &diagnostic).is_ok() {
                    use std::io::Write;
                    if io::stderr().lock().write_all(b"\n").is_err() {
                        return ExitCode::FAILURE;
                    }
                }
                ExitCode::FAILURE
            }
        };
    }
    let result = swarm_cli::run_os_detailed(
        &args,
        &std::env::var("SWARM_LANG").unwrap_or_default(),
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    );
    ExitCode::from(result.code)
}
