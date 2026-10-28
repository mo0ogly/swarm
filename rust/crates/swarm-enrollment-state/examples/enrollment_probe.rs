//! Isolated migration test driver; never a production authority service.
use ed25519_dalek::SigningKey;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    io::{self, BufRead},
    path::Path,
};
use swarm_enrollment_proof::Request;
use swarm_enrollment_state::{decode_active_reply, sign_change, Context, Error, Head, Journal};
#[derive(Deserialize)]
struct Command {
    op: String,
    #[serde(default)]
    file: String,
    #[serde(default)]
    base: Head,
    #[serde(default)]
    raw: String,
    #[serde(default)]
    operation: String,
    #[serde(default)]
    id: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    cancel: bool,
    request: Option<Request>,
}
fn hex(raw: &[u8]) -> String {
    raw.iter().map(|b| format!("{b:02x}")).collect()
}
fn unhex(raw: &str) -> Vec<u8> {
    assert_eq!(raw.len() % 2, 0);
    (0..raw.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&raw[i..i + 2], 16).unwrap())
        .collect()
}
fn execute(root: &Path, c: Command) -> Result<Value, Error> {
    let key = SigningKey::from_bytes(&[7; 32]);
    let ctx = Context::new();
    if c.cancel {
        ctx.cancel()
    }
    match c.op.as_str() {
        "sign" => Ok(
            json!({"code":"ok","raw":hex(&sign_change(&key,[11;32],&c.base,&c.operation,&c.id,&unhex(&c.body))?)}),
        ),
        "decode-reply" => Ok(json!({"code":"ok","reply":decode_active_reply(&unhex(&c.raw))?})),
        _ => {
            assert!(
                !c.file.is_empty()
                    && Path::new(&c.file)
                        .file_name()
                        .is_some_and(|s| s == c.file.as_str())
                    && c.file != "."
                    && c.file != ".."
            );
            let j = Journal::open(
                &ctx,
                root.join(&c.file),
                &key.verifying_key().to_bytes(),
                [11; 32],
            )?;
            match c.op.as_str() {
                "open" => Ok(json!({"code":"ok"})),
                "apply" => Ok(json!({"code":"ok","head":j.apply(&ctx,&c.base,&unhex(&c.raw))?})),
                "read" => j.with_active(&ctx, &c.base, &c.id, |record| {
                    Ok(json!({"code":"ok","record":record}))
                }),
                "sign-active" => Ok(
                    json!({"code":"ok","raw":hex(&j.sign_active(&ctx,&c.base,&c.id,c.request.expect("fixture request"),&key)?)}),
                ),
                _ => panic!("unknown fixture operation"),
            }
        }
    }
}
fn main() {
    let root = std::env::args()
        .nth(1)
        .expect("explicit isolated fixture root");
    assert!(Path::new(&root).is_dir());
    for line in io::stdin().lock().lines() {
        let c: Command = serde_json::from_str(&line.expect("stdin")).expect("fixture JSON");
        let out = execute(Path::new(&root), c).unwrap_or_else(|e| json!({"code":e.code()}));
        println!("{out}")
    }
}
