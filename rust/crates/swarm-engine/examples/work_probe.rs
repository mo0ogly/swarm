//! Isolated typed-data comparison transport; executes no SQL or command.
use serde::Deserialize;
use std::io::{self, BufRead, Write};
#[derive(Deserialize)]
struct Input {
    name: String,
    model: String,
    raw: String,
    #[serde(default)]
    row_id: Option<String>,
    #[serde(default)]
    revision: i64,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut output = io::stdout().lock();
    for line in io::stdin().lock().lines() {
        let value: Input = serde_json::from_str(&line?)?;
        if !value.raw.len().is_multiple_of(2) || !value.raw.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("invalid fixture hex".into());
        }
        let raw = (0..value.raw.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&value.raw[i..i + 2], 16))
            .collect::<Result<Vec<_>, _>>()?;
        let result = if let Some(id) = value.row_id {
            swarm_engine::decode_work_row(&id, value.revision, &raw)
                .and_then(|v| swarm_engine::encode_model(&v))
        } else {
            swarm_engine::fixture_model_roundtrip(&value.model, &raw)
        };
        let (code, out) = match result {
            Ok(bytes) => (
                "ok",
                bytes.iter().map(|v| format!("{v:02x}")).collect::<String>(),
            ),
            Err(_) => ("format", String::new()),
        };
        writeln!(
            output,
            "{}",
            serde_json::json!({"name":value.name,"code":code,"output":out})
        )?;
    }
    Ok(())
}
