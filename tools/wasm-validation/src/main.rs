//! Pure validation command for Swarm's operator-authorized task controls.

#![forbid(unsafe_code)]

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::ExitCode;
use wasmparser::{Encoding, Parser, Payload};
use wasmtime::{Config, Engine, Instance, Module, Store, StoreLimits, StoreLimitsBuilder};

const MAX_MODULE_BYTES: u64 = 65_536;

#[derive(Debug, PartialEq, Eq)]
enum Failure {
    Arguments,
    InputFile,
    ModuleTooLarge,
    InvalidModule,
    UnsupportedProfile,
    Engine,
    Execution,
    InvalidVerdict,
    Output,
}

impl Failure {
    /// Return a stable diagnostic without printing module bytes or a private path.
    fn code(&self) -> &'static str {
        match self {
            Self::Arguments => "wasm-control-arguments",
            Self::InputFile => "wasm-control-input-file",
            Self::ModuleTooLarge => "wasm-control-module-too-large",
            Self::InvalidModule => "wasm-control-invalid-module",
            Self::UnsupportedProfile => "wasm-control-unsupported-profile",
            Self::Engine => "wasm-control-engine",
            Self::Execution => "wasm-control-execution",
            Self::InvalidVerdict => "wasm-control-invalid-verdict",
            Self::Output => "wasm-control-output",
        }
    }
}

/// Read bounded bytes from one opened regular file. A failed read produces no verdict.
fn load(path: &Path) -> Result<Vec<u8>, Failure> {
    let file = File::open(path).map_err(|_| Failure::InputFile)?;
    if !file.metadata().map_err(|_| Failure::InputFile)?.is_file() {
        return Err(Failure::InputFile);
    }
    let mut bytes = Vec::new();
    file.take(MAX_MODULE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Failure::InputFile)?;
    if bytes.len() as u64 > MAX_MODULE_BYTES {
        return Err(Failure::ModuleTooLarge);
    }
    Ok(bytes)
}

/// Run only the closed predicate ABI on these bytes in a fresh fuel-limited Store.
/// Validation and traps return errors. A valid false result is a business refusal.
fn check(bytes: &[u8], input: i32, fuel: u64) -> Result<bool, Failure> {
    for section in Parser::new(0).parse_all(bytes) {
        match section.map_err(|_| Failure::InvalidModule)? {
            Payload::Version {
                encoding: Encoding::Component,
                ..
            }
            | Payload::StartSection { .. } => return Err(Failure::UnsupportedProfile),
            Payload::ImportSection(section) if section.count() != 0 => {
                return Err(Failure::UnsupportedProfile);
            }
            Payload::MemorySection(section) if section.count() != 0 => {
                return Err(Failure::UnsupportedProfile);
            }
            Payload::TableSection(section) if section.count() != 0 => {
                return Err(Failure::UnsupportedProfile);
            }
            _ => {}
        }
    }

    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config).map_err(|_| Failure::Engine)?;
    let module = Module::from_binary(&engine, bytes).map_err(|_| Failure::InvalidModule)?;
    let mut exports = module.exports();
    if !exports
        .next()
        .is_some_and(|export| export.name() == "check")
        || exports.next().is_some()
    {
        return Err(Failure::UnsupportedProfile);
    }
    let limits = StoreLimitsBuilder::new().memories(0).tables(0).build();
    let mut store = Store::new(&engine, limits);
    store.limiter(|limits: &mut StoreLimits| limits);
    store.set_fuel(fuel).map_err(|_| Failure::Engine)?;
    let instance = Instance::new(&mut store, &module, &[]).map_err(|_| Failure::Execution)?;
    let predicate = instance
        .get_typed_func::<i32, i32>(&mut store, "check")
        .map_err(|_| Failure::UnsupportedProfile)?;
    match predicate
        .call(&mut store, input)
        .map_err(|_| Failure::Execution)?
    {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(Failure::InvalidVerdict),
    }
}

/// Decode the operator's path, scalar input and positive fuel budget and run once.
fn run() -> Result<bool, Failure> {
    let mut args = std::env::args_os().skip(1);
    let path = args.next().ok_or(Failure::Arguments)?;
    let input = args
        .next()
        .and_then(|raw| raw.to_str()?.parse().ok())
        .ok_or(Failure::Arguments)?;
    let fuel = args
        .next()
        .and_then(|raw| raw.to_str()?.parse().ok())
        .ok_or(Failure::Arguments)?;
    if fuel == 0 || args.next().is_some() {
        return Err(Failure::Arguments);
    }
    check(&load(Path::new(&path))?, input, fuel)
}

fn main() -> ExitCode {
    let result = run().and_then(|passed| {
        if passed {
            let mut output = io::stdout().lock();
            writeln!(output, "WASM_CONTROL_PASS")
                .and_then(|_| output.flush())
                .map_err(|_| Failure::Output)?;
        }
        Ok(passed)
    });
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => {
            eprintln!("wasm-control-refused");
            ExitCode::FAILURE
        }
        Err(failure) => {
            eprintln!("{}", failure.code());
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn predicate_accepts_and_refuses_distinct_inputs() {
        let bytes = wat::parse_str(
            r#"(module
            (func (export "check") (param i32) (result i32)
                local.get 0 i32.const 7 i32.eq))"#,
        )
        .unwrap();
        assert_eq!(check(&bytes, 7, 100), Ok(true));
        assert_eq!(check(&bytes, 8, 100), Ok(false));
    }

    #[test]
    fn guest_import_is_refused_as_unsupported() {
        let bytes = wat::parse_str(
            r#"(module
            (import "host" "f" (func))
            (func (export "check") (param i32) (result i32) i32.const 1))"#,
        )
        .unwrap();
        assert_eq!(check(&bytes, 7, 100), Err(Failure::UnsupportedProfile));
    }
}
