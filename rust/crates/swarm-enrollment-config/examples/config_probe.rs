//! Fixture transport only. Never opens storage or authenticates an actor.
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{self, BufRead};
use swarm_enrollment_config as cfg;
#[derive(Deserialize)]
struct Vector {
    name: String,
    mode: String,
    input: String,
}
fn hex(raw: &[u8]) -> String {
    raw.iter().map(|b| format!("{b:02x}")).collect()
}
fn unhex(raw: &str) -> Vec<u8> {
    (0..raw.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&raw[i..i + 2], 16).unwrap())
        .collect()
}
fn snapshot_value(v: &mut Value) {
    if v["history"].is_null() {
        v["history"] = json!([])
    }
}
fn evaluate(v: &Vector) -> Result<Vec<u8>, cfg::Error> {
    let raw = unhex(&v.input);
    if v.mode == "decode" {
        return cfg::encode(&cfg::decode(&raw)?);
    }
    if v.mode == "decode-change" {
        return cfg::encode_change(&cfg::decode_change(&raw)?);
    }
    if v.mode == "clock" || v.mode == "clock-utc" {
        #[derive(Deserialize)]
        struct Input {
            seconds: i64,
            nanos: u32,
            offset: i32,
        }
        let input: Input = serde_json::from_slice(&raw).unwrap();
        let value = time::OffsetDateTime::from_unix_timestamp(input.seconds).unwrap()
            + time::Duration::nanoseconds(i64::from(input.nanos));
        let value = value.to_offset(time::UtcOffset::from_whole_seconds(input.offset).unwrap());
        #[derive(serde::Serialize)]
        struct Local(
            #[serde(serialize_with = "swarm_wire_json::timestamp::serialize_preserving_offset")]
            time::OffsetDateTime,
        );
        #[derive(serde::Serialize)]
        struct Utc(
            #[serde(serialize_with = "swarm_wire_json::timestamp::serialize")] time::OffsetDateTime,
        );
        return if v.mode == "clock" {
            swarm_wire_json::encode(&Local(value))
        } else {
            swarm_wire_json::encode(&Utc(value))
        }
        .map_err(cfg::Error::Frame);
    }
    let mut value: Value = serde_json::from_slice(&raw).map_err(cfg::Error::Frame)?;
    // Go nil slices have no distinct Rust Vec constructor; normalize only this
    // typed fixture transport, never a production Decode input.
    match v.mode.as_str() {
        "encode" | "head" => snapshot_value(&mut value),
        "view" => snapshot_value(&mut value["snapshot"]),
        "projection" => snapshot_value(&mut value["proposed"]["snapshot"]),
        _ => (),
    }
    match v.mode.as_str() {
        "encode" => cfg::encode(&serde_json::from_value(value).map_err(cfg::Error::Frame)?),
        "change" => cfg::encode_change(&serde_json::from_value(value).map_err(cfg::Error::Frame)?),
        "head" => swarm_wire_json::encode(&cfg::snapshot_view(
            &serde_json::from_value(value).map_err(cfg::Error::Frame)?,
        )?)
        .map_err(cfg::Error::Frame),
        "view" => {
            cfg::validate_view(&serde_json::from_value(value).map_err(cfg::Error::Frame)?)?;
            Ok(vec![])
        }
        "projection" => {
            cfg::validate_projection(&serde_json::from_value(value).map_err(cfg::Error::Frame)?)?;
            Ok(vec![])
        }
        _ => panic!("unknown fixture mode"),
    }
}
fn main() {
    for line in io::stdin().lock().lines() {
        let v: Vector = serde_json::from_str(&line.unwrap()).unwrap();
        let (code, output) = match evaluate(&v) {
            Ok(raw) => ("ok", hex(&raw)),
            Err(e) => (e.code(), String::new()),
        };
        println!("{}", json!({"name":v.name,"code":code,"output":output}));
    }
}
