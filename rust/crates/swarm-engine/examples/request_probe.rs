//! Typed fixture transport only; it executes no command or SQL mutation.
use serde::Deserialize;
use serde_json::json;
use std::io::{self, BufRead};
#[derive(Deserialize)]
struct Input {
    name: String,
    mode: String,
    raw: String,
}
fn unhex(raw: &str) -> Vec<u8> {
    (0..raw.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&raw[i..i + 2], 16).unwrap())
        .collect()
}
fn hex(raw: &[u8]) -> String {
    raw.iter().map(|v| format!("{v:02x}")).collect()
}
fn main() {
    for line in io::stdin().lock().lines() {
        let v: Input = serde_json::from_str(&line.unwrap()).unwrap();
        let raw = unhex(&v.raw);
        let result = match v.mode.as_str() {
            "config-change" => swarm_enrollment_config::decode_change(&raw)
                .and_then(|d| swarm_enrollment_config::encode_change(&d))
                .map_err(|e| e.code().to_owned()),
            "reply" => swarm_enrollment_state::decode_active_reply(&raw)
                .map_err(|e| e.code().to_owned())
                .and_then(|d| swarm_wire_json::encode(&d).map_err(|_| "reply".into())),
            "external" => swarm_engine::decode_external_request(&raw)
                .and_then(|d| swarm_engine::encode_request(&d))
                .map_err(|e| e.code().to_owned()),
            "request" => swarm_engine::decode_request(&raw)
                .and_then(|d| swarm_engine::encode_request(&d))
                .map_err(|e| e.code().to_owned()),
            _ => panic!("unknown fixture mode"),
        };
        let (code, output) = match result {
            Ok(v) => ("ok".into(), hex(&v)),
            Err(code) => (code, String::new()),
        };
        println!("{}", json!({"name":v.name,"code":code,"output":output}));
    }
}
