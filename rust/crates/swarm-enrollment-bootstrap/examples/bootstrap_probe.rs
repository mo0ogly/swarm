//! Read-only fixture driver, unavailable without test-fixtures.
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    io::{self, BufRead},
    path::Path,
};
use swarm_enrollment_bootstrap as b;
#[derive(Deserialize)]
struct Input {
    name: String,
    operation: String,
    raw: Option<String>,
    path: Option<String>,
    digest: Option<String>,
    document: Option<b::Document>,
}
fn hex(raw: &[u8]) -> String {
    raw.iter().map(|v| format!("{v:02x}")).collect()
}
fn unhex(raw: &str) -> Vec<u8> {
    (0..raw.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&raw[i..i + 2], 16).unwrap())
        .collect()
}
fn evaluate(root: &Path, in_: Input) -> Value {
    let mut out = json!({"name":in_.name,"code":"ok","zero":true,"raw":"","document":null});
    let result = (|| -> Result<(), b::Error> {
        match in_.operation.as_str() {
            "encode" => out["raw"] = json!(hex(&b::encode(in_.document.as_ref().unwrap())?)),
            "decode" => {
                let d = b::decode(&unhex(in_.raw.as_ref().unwrap()))?;
                out["raw"] = json!(hex(&b::encode(&d)?));
                out["document"] = serde_json::to_value(d).unwrap();
            }
            "load" => {
                let path = in_.path.as_ref().unwrap();
                assert!(!Path::new(path).is_absolute() || Path::new(path).starts_with(root));
                out["document"] =
                    serde_json::to_value(b::load(path, in_.digest.as_ref().unwrap())?).unwrap();
            }
            _ => panic!("unknown fixture operation"),
        };
        Ok(())
    })();
    if let Err(e) = result {
        out["code"] = json!(e.code());
    }
    out
}
fn main() {
    let root = std::env::args()
        .nth(1)
        .expect("private fixture root required");
    let root = Path::new(&root);
    nix::unistd::close(
        swarm_enrollment_config::store::open_private_directory(
            root,
            rustix::process::geteuid().as_raw(),
        )
        .unwrap(),
    )
    .unwrap();
    for line in io::stdin().lock().lines() {
        println!(
            "{}",
            evaluate(root, serde_json::from_str(&line.unwrap()).unwrap())
        );
    }
}
