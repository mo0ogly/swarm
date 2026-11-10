//! Gated task contract fixture with explicit legacy limits and supplied stamps.
use serde::Deserialize;
use std::{
    error::Error,
    io::{self, BufRead, Write},
};
use swarm_engine::{
    apply_task_metadata, decode_request, decode_task, decode_work, encode_model,
    normalize_validation_policy, update_task_definition, update_task_instructions,
    validate_task_dependency_graph, validation_policy_covers_task, DefinitionContext, Request,
    TaskDefinitionLimits, ValidationPolicy, ValidationPolicyLimits,
};
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
struct Frame {
    name: String,
    mode: String,
    body: String,
    request: String,
    policy: String,
    #[serde(rename = "control")]
    _control: bool,
    #[serde(rename = "classification")]
    _classification: String,
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
fn hex(b: &[u8]) -> String {
    b.iter().map(|b| format!("{b:02x}")).collect()
}
fn one(f: Frame) -> Result<serde_json::Value, Cause> {
    let raw = unhex(&f.body)?;
    let request = unhex(&f.request)?;
    let r = if request.is_empty() {
        Request::default()
    } else {
        decode_request(&request)?
    };
    let context = DefinitionContext {
        authorized_at: "supplied-time",
        actor: "supplied-actor",
        task_limits: TaskDefinitionLimits::legacy_go(),
        policy_limits: ValidationPolicyLimits::legacy_go(),
    };
    let (ok, body) = match f.mode.as_str() {
        "definition" | "metadata" | "graph" => {
            let mut w = decode_work(&raw)?;
            let i = w
                .tasks
                .as_deref()
                .unwrap_or_default()
                .iter()
                .position(|t| t.id == r.id);
            let ok = match i {
                None => false,
                Some(i) => match f.mode.as_str() {
                    "definition" => match update_task_definition(
                        &w,
                        &w.tasks.as_ref().unwrap()[i],
                        &r,
                        &context,
                    ) {
                        Ok(t) => {
                            w.tasks.as_mut().unwrap()[i] = t;
                            true
                        }
                        Err(_) => false,
                    },
                    "metadata" => apply_task_metadata(&mut w, &r, &context).is_ok(),
                    _ => {
                        let mut t = w.tasks.as_ref().unwrap()[i].clone();
                        t.depends = r.depends.clone();
                        validate_task_dependency_graph(&w, &t).is_ok()
                    }
                },
            };
            (ok, encode_model(&w)?)
        }
        "instructions" => {
            let mut t = decode_task(&raw)?;
            let ok = update_task_instructions(&mut t, &r.next).is_ok();
            (ok, encode_model(&t)?)
        }
        "policy" => {
            // The policy DTO uses Go projection through Request, not serde's
            // field-presence rules or an invented second policy decoder.
            let wrapped = [b"{\"validation_policy\":".as_slice(), &raw, b"}"].concat();
            let p = decode_request(&wrapped)?
                .validation_policy
                .ok_or("policy required")?;
            match normalize_validation_policy(p.clone(), &context.policy_limits) {
                Ok(p) => (true, encode_model(&p)?),
                Err(_) => (false, encode_model(&p)?),
            }
        }
        "coverage" => {
            let t = decode_task(&raw)?;
            let p: ValidationPolicy = decode_request(
                &[
                    b"{\"validation_policy\":".as_slice(),
                    &unhex(&f.policy)?,
                    b"}",
                ]
                .concat(),
            )?
            .validation_policy
            .ok_or("policy required")?;
            (
                validation_policy_covers_task(&p, &t).is_ok(),
                encode_model(&t)?,
            )
        }
        _ => return Err("unknown fixture mode".into()),
    };
    Ok(serde_json::json!({"name":f.name,"code":if ok{"ok"}else{"format"},"body":hex(&body)}))
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
