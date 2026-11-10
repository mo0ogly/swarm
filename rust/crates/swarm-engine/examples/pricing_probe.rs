//! Finite trusted pricing frames only; no root, provider, identity or expenditure.
use serde::Deserialize;
use serde_json::{json, value::RawValue, Value};
use std::{
    error::Error,
    io::{self, BufRead, Write},
};
use swarm_engine::{
    decode_rate_change, decode_rate_quote, encode_model, validate_rate_change, RateCatalogue,
};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Frame {
    mode: String,
    request: Box<RawValue>,
    catalogue: Box<RawValue>,
}
/// Convert only contract refusals to observation data; framing and encode/IO
/// failures propagate as nonzero execution. No missing-output success is emitted.
fn one(frame: Frame) -> Result<Value, Box<dyn Error>> {
    let refused = json!({"code":"refused"});
    let bytes = match frame.mode.as_str() {
        "change" => {
            let request = match decode_rate_change(frame.request.get().as_bytes()) {
                Ok(r) => r,
                Err(_) => return Ok(refused),
            };
            if validate_rate_change(&request).is_err() {
                return Ok(refused);
            }
            encode_model(&request)?
        }
        "quote-codec" => {
            let request = match decode_rate_quote(frame.request.get().as_bytes()) {
                Ok(r) => r,
                Err(_) => return Ok(refused),
            };
            encode_model(&request)?
        }
        "catalogue" => {
            let c = match RateCatalogue::decode(frame.catalogue.get().as_bytes()) {
                Ok(c) => c,
                Err(_) => return Ok(refused),
            };
            encode_model(&c)?
        }
        "quote" => {
            let r = match decode_rate_quote(frame.request.get().as_bytes()) {
                Ok(r) => r,
                Err(_) => return Ok(refused),
            };
            let c = match RateCatalogue::decode(frame.catalogue.get().as_bytes()) {
                Ok(c) => c,
                Err(_) => return Ok(refused),
            };
            let quote = match c.quote(&r) {
                Ok(q) => q,
                Err(_) => return Ok(refused),
            };
            encode_model(&quote)?
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unsupported pricing fixture operation",
            )
            .into())
        }
    };
    Ok(json!({"code":"ok","intent":String::from_utf8(bytes)?}))
}
/// Check line framing and complete writes/flush; failures cannot become empty PASS.
fn run() -> Result<(), Box<dyn Error>> {
    let mut output = io::BufWriter::new(io::stdout().lock());
    for line in io::stdin().lock().lines() {
        let frame: Frame = serde_json::from_str(&line?)?;
        output.write_all(&serde_json::to_vec(&one(frame)?)?)?;
        output.write_all(b"\n")?;
    }
    output.flush()?;
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("pricing-fixture: {error}");
        std::process::exit(1)
    }
}
