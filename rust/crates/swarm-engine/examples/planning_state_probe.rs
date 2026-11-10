//! Gated data fixture: no public mutation command or authority.
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    error::Error,
    io::{self, BufRead, Write},
};
use swarm_engine::{
    apply_planning_validation_signals, decode_work, encode_model, validate_planning_state,
};
type Cause = Box<dyn Error>;
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Frame {
    name: String,
    mode: String,
    body: String,
    before: BTreeMap<String, String>,
    event: String,
    at: String,
}
fn unhex(s: &str) -> Result<Vec<u8>, Cause> {
    if !s.len().is_multiple_of(2) || !s.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err("invalid fixture hex".into());
    };
    Ok((0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect::<Result<_, _>>()?)
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn one(f: Frame) -> Result<serde_json::Value, Cause> {
    let mut code = "ok";
    let mut body = String::new();
    let mut instant = String::new();
    if f.mode == "clock" {
        match swarm_wire_json::go_rfc3339::parse(&f.at) {
            Ok(t) => instant = format!("{}:{:09}", t.unix_timestamp(), t.nanosecond()),
            Err(_) => code = "format",
        }
    } else {
        let mut w = decode_work(&unhex(&f.body)?)?;
        let result = if matches!(f.mode.as_str(), "signals" | "unsafe-signals") {
            apply_planning_validation_signals(&mut w, &f.before, &f.event, &f.at)
        } else {
            validate_planning_state(&w)
        };
        if result.is_err() {
            code = "format"
        };
        body = hex(&encode_model(&w)?);
    }
    Ok(serde_json::json!({"name":f.name,"code":code,"body":body,"instant":instant}))
}
fn run() -> Result<(), Cause> {
    let mut out = io::BufWriter::new(io::stdout().lock());
    for line in io::stdin().lock().lines() {
        serde_json::to_writer(&mut out, &one(serde_json::from_str(&line?)?)?)?;
        out.write_all(b"\n")?
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
