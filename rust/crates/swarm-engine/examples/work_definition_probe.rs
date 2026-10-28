//! Gated definition/intent fixture; no clock, Git process or authority.
use serde::Deserialize;
use std::{
    error::Error,
    io::{self, BufRead, Write},
};
use swarm_engine::{
    apply_ooda, apply_task_addition, apply_work_creation, apply_work_definition, decode_request,
    decode_work, encode_model, validate_work_request, GitState,
};
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Frame {
    name: String,
    kind: String,
    body: String,
    request: String,
    git: FixtureGit,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureGit {
    head: String,
    branch: String,
    changes: String,
}
type Cause = Box<dyn Error>;
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
    let mut w = decode_work(&unhex(&f.body)?)?;
    let r = decode_request(&unhex(&f.request)?)?;
    let result = validate_work_request(&f.kind, &r).and_then(|()| match f.kind.as_str() {
        "work.create" => apply_work_creation(
            &mut w,
            &r,
            GitState {
                head: f.git.head,
                branch: f.git.branch,
                changes: f.git.changes,
            },
        ),
        "work.update" => apply_work_definition(&mut w, &r),
        "task.add" => apply_task_addition(&mut w, &r),
        "ooda" => apply_ooda(&mut w, &r),
        _ => Err(swarm_engine::WorkDefinitionError::Refused(
            swarm_engine::DefinitionRefusal::Command,
        )),
    });
    let code = result.as_ref().err().map_or("ok", |e| e.code());
    Ok(serde_json::json!({"name":f.name,"code":code,"body":hex(&encode_model(&w)?)}))
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
