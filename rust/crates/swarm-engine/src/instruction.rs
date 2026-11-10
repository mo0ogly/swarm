//! Pure instruction composition; attributed learning and original base survive
//! operator edits. No planning decision, event or authority is generated here.
use crate::Task;
use std::fmt;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstructionError(&'static str);
impl InstructionError {
    pub fn reason(&self) -> &'static str {
        self.0
    }
    pub fn code(&self) -> &'static str {
        "format"
    }
}
impl fmt::Display for InstructionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "task instructions: {}", self.0)
    }
}
impl std::error::Error for InstructionError {}
fn operator_base(t: &Task) -> &str {
    t.instruction_operator_base
        .as_deref()
        .map(String::as_str)
        .or_else(|| t.instruction_base.as_deref().map(String::as_str))
        .unwrap_or_default()
}
pub fn composed_task_instructions(t: &Task) -> String {
    if t.instruction_base.is_none() {
        return t.next.clone();
    }
    let mut parts: Vec<&str> = t
        .instruction_revisions
        .as_deref()
        .unwrap_or_default()
        .iter()
        .rev()
        .filter(|r| r.state == "applied")
        .map(|r| r.addition.as_str())
        .collect();
    parts.push(operator_base(t));
    parts.join("\n\n")
}
fn reconcile(t: &mut Task) -> Result<(), InstructionError> {
    let current = composed_task_instructions(t);
    if t.next == current {
        return Ok(());
    }
    if t.instruction_base.is_none() || current.is_empty() || !t.next.ends_with(&current) {
        return Err(InstructionError("instruction-current-context-changed"));
    }
    let prefix = t.next.strip_suffix(&current).unwrap_or_default();
    let base = prefix.to_owned() + operator_base(t);
    t.instruction_operator_base = Some(Box::new(base));
    t.next = composed_task_instructions(t);
    Ok(())
}
pub fn update_task_instructions(t: &mut Task, next: &str) -> Result<(), InstructionError> {
    if t.instruction_base.is_none() || t.status != "todo" {
        t.next = next.into();
        return Ok(());
    }
    let mut candidate = t.clone();
    let previous = candidate.next.clone();
    reconcile(&mut candidate)?;
    let current = candidate.next.clone();
    let mut base = operator_base(&candidate).to_owned();
    if next == previous || next == current {
        *t = candidate;
        return Ok(());
    }
    if let Some(prefix) = next.strip_suffix(&previous) {
        base = prefix.to_owned() + &base;
    } else if let Some(prefix) = next.strip_suffix(&current) {
        base = prefix.to_owned() + &base;
    } else {
        let learning = current.strip_suffix(&base).unwrap_or(&current);
        if !learning.is_empty() && next.starts_with(learning) {
            base = next[learning.len()..].into();
        } else {
            if candidate
                .instruction_revisions
                .iter()
                .flatten()
                .any(|r| r.state == "applied" && next.contains(&r.addition))
            {
                return Err(InstructionError(
                    "instruction-learning-relocation-ambiguous",
                ));
            }
            base = next.into();
        }
    }
    candidate.instruction_operator_base = Some(Box::new(base));
    candidate.next = composed_task_instructions(&candidate);
    *t = candidate;
    Ok(())
}
