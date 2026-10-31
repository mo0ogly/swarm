//! `swarm-authority check --declaration PATH --pin SHA256 --json`
//!
//! Prints the [`swarm_authority::Readiness`] JSON on success (exit 0) or
//! `{"schema_version":1,"failure":{"code":…}}` on stderr (exit 2). Usage
//! errors exit 64. No other command exists yet (slice B2.1).
use std::{io::Write, process::ExitCode};

fn usage() -> ExitCode {
    eprintln!("usage: swarm-authority check --declaration PATH --pin SHA256 --json");
    ExitCode::from(64)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mut declaration, mut pin, mut json) = (None, None, false);
    let mut iter = args.iter();
    match iter.next().map(String::as_str) {
        Some("check") => {}
        _ => return usage(),
    }
    while let Some(a) = iter.next() {
        match a.as_str() {
            "--declaration" if declaration.is_none() => declaration = iter.next(),
            "--pin" if pin.is_none() => pin = iter.next(),
            "--json" if !json => json = true,
            _ => return usage(),
        }
    }
    let (Some(declaration), Some(pin), true) = (declaration, pin, json) else {
        return usage();
    };
    #[cfg(target_os = "linux")]
    {
        match swarm_authority::check(declaration, pin) {
            Ok(r) => {
                let mut out = std::io::stdout().lock();
                if serde_json::to_writer(&mut out, &r).is_err() || writeln!(out).is_err() {
                    return ExitCode::from(74);
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{}", serde_json::json!({"schema_version": 1, "failure": {"code": e.code()}}));
                ExitCode::from(2)
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (declaration, pin);
        eprintln!("{}", serde_json::json!({"schema_version": 1, "failure": {"code": "authority-linux-required"}}));
        ExitCode::from(2)
    }
}
