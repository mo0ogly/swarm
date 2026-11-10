//! Synthetic wire observations only; no provider, process, root or authority.
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    io::{self, BufRead, Write},
};
use swarm_engine::{decode_agent, decode_agent_row, decode_launch, decode_providers, encode_model};

#[derive(Deserialize)]
struct Frame {
    kind: String,
    body: Box<serde_json::value::RawValue>,
    #[serde(default)]
    status: String,
    #[serde(default)]
    desired: String,
}
#[derive(Serialize)]
struct Observation {
    code: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    intent: Option<String>,
    unregistered_claim: bool,
}

/// Exercise production codecs; semantic refusal is data, framing/encoding/IO
/// failures are nonzero. Checked output/flush preserves failure rather than
/// emitting a successful empty observation. No automatic retry or timeout.
fn main() -> Result<(), Box<dyn Error>> {
    let mut out = io::stdout().lock();
    for line in io::stdin().lock().lines() {
        let frame: Frame = serde_json::from_str(&line?)?;
        let raw = frame.body.get().as_bytes();
        let mut unregistered = false;
        let encoded = match frame.kind.as_str() {
            "agent" => decode_agent(raw).map(|value| encode_model(&value)),
            "row" => decode_agent_row(raw, &frame.status, &frame.desired).map(|value| {
                unregistered = value.has_unregistered_claim();
                encode_model(&value)
            }),
            "launch" => decode_launch(raw).map(|value| encode_model(&value)),
            "providers" => decode_providers(raw).map(|value| encode_model(&value)),
            _ => {
                return Err(
                    io::Error::new(io::ErrorKind::InvalidInput, "unknown fixture kind").into(),
                )
            }
        };
        let observation = match encoded {
            Ok(encoded) => Observation {
                code: "ok",
                intent: Some(String::from_utf8(encoded?)?),
                unregistered_claim: unregistered,
            },
            Err(_) => Observation {
                code: "refused",
                intent: None,
                unregistered_claim: false,
            },
        };
        serde_json::to_writer(&mut out, &observation)?;
        out.write_all(b"\n")?;
    }
    out.flush()?;
    Ok(())
}
