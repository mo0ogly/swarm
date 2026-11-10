//! Gated data/system observation fixture. No Go runtime or provider invocation.
use serde::Deserialize;
use std::{
    error::Error,
    io::{self, BufRead, Write},
    path::Path,
};
use swarm_engine::{
    encode_model, format_utc_timestamp, go_command_text, identifier_from_entropy, observe_git,
    operator_identity, system_timestamp,
};
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
struct Frame {
    name: String,
    mode: String,
    bytes: String,
    prefix: String,
    seconds: i64,
    nanos: i64,
    root: String,
}
type Cause = Box<dyn Error>;
fn unhex(s: &str) -> Result<Vec<u8>, Cause> {
    if !s.len().is_multiple_of(2) || !s.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err("invalid fixture hex".into());
    }
    Ok((0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect::<Result<_, _>>()?)
}
fn hex(v: &[u8]) -> String {
    v.iter().map(|b| format!("{b:02x}")).collect()
}
fn one(f: Frame) -> Result<serde_json::Value, Cause> {
    let raw = unhex(&f.bytes)?;
    let value = match f.mode.as_str() {
        "text" => go_command_text(&raw)?,
        "id" => identifier_from_entropy(
            &f.prefix,
            &raw.try_into().map_err(|_| "12 entropy bytes required")?,
        ),
        "clock" => format_utc_timestamp(f.seconds, f.nanos)?,
        "now" => system_timestamp()?,
        "operator" => operator_identity()?,
        "git" => {
            let observed = observe_git(Path::new(&f.root))?;
            return Ok(
                serde_json::json!({"name":f.name,"value":hex(&encode_model(&observed.state)?),"issues":observed.issues.len()}),
            );
        }
        _ => return Err("unknown fixture mode".into()),
    };
    Ok(serde_json::json!({"name":f.name,"value":hex(&encode_model(&value)?)}))
}
fn run() -> Result<(), Cause> {
    let mut out = io::BufWriter::new(io::stdout().lock());
    for line in io::stdin().lock().lines() {
        serde_json::to_writer(&mut out, &one(serde_json::from_str(&line?)?)?)?;
        out.write_all(b"\n")?;
    }
    out.flush()?;
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(2)
    }
}
