//! Policy data normalization. It authorizes or executes no command. Limits are
//! explicit inputs; the eventual Store must load persisted operator settings.
use crate::{text::trim_space, Task, ValidationPolicy};
use std::{
    collections::BTreeSet,
    fmt,
    path::{Component, Path},
};

#[derive(Clone, Debug)]
pub struct ValidationPolicyLimits {
    pub allowed_programs: BTreeSet<String>,
    pub max_controls: usize,
    pub max_arguments: usize,
    pub max_inputs: usize,
    pub min_justification_chars: usize,
    pub max_justification_chars: usize,
    pub default_timeout_seconds: i64,
    pub max_timeout_seconds: i64,
    pub max_total_timeout_seconds: i64,
}
impl ValidationPolicyLimits {
    /// Explicit compatibility fixture, not an implicit runtime fallback.
    pub fn legacy_go() -> Self {
        Self {
            allowed_programs: ["go", "git", "node", "npm", "python", "python3", "pytest"]
                .into_iter()
                .map(String::from)
                .collect(),
            max_controls: 8,
            max_arguments: 32,
            max_inputs: 64,
            min_justification_chars: 8,
            max_justification_chars: 500,
            default_timeout_seconds: 60,
            max_timeout_seconds: 300,
            max_total_timeout_seconds: 300,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValidationPolicyError(&'static str);
impl ValidationPolicyError {
    pub fn reason(&self) -> &'static str {
        self.0
    }
    pub fn code(&self) -> &'static str {
        "format"
    }
}
impl fmt::Display for ValidationPolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "validation policy: {}", self.0)
    }
}
impl std::error::Error for ValidationPolicyError {}
fn fail(reason: &'static str) -> ValidationPolicyError {
    ValidationPolicyError(reason)
}

// Go filepath.Clean then containment of relative input, without touching disk.
fn relative_input(name: &str) -> bool {
    if name.is_empty() || Path::new(name).is_absolute() {
        return false;
    }
    let mut depth = 0usize;
    for c in Path::new(name).components() {
        match c {
            Component::Normal(_) => depth += 1,
            Component::ParentDir => {
                if depth == 0 {
                    return false;
                }
                depth -= 1;
            }
            Component::CurDir => {}
            _ => return false,
        }
    }
    true
}
pub fn normalize_validation_policy(
    mut p: ValidationPolicy,
    limits: &ValidationPolicyLimits,
) -> Result<ValidationPolicy, ValidationPolicyError> {
    if limits.min_justification_chars > limits.max_justification_chars
        || limits.default_timeout_seconds < 1
        || limits.max_timeout_seconds < 1
        || limits.default_timeout_seconds > limits.max_timeout_seconds
        || limits.max_total_timeout_seconds < 1
    {
        return Err(fail("policy-limits-invalid"));
    }
    p.mode = trim_space(&p.mode).into();
    if p.mode != "human" && p.mode != "automatic" {
        return Err(fail("policy-mode-invalid"));
    }
    let count = p.controls.as_ref().map_or(0, Vec::len);
    if count > limits.max_controls || (p.mode == "automatic" && count == 0) {
        return Err(fail("policy-control-count-invalid"));
    }
    let mut ids = BTreeSet::new();
    let mut total = 0i64;
    for c in p.controls.iter_mut().flatten() {
        let mut exits = BTreeSet::new();
        for code in c.environment_exit_codes.iter().flatten() {
            // OS process exit representation, not an operator retry ceiling.
            if !(1..=255).contains(code) || !exits.insert(*code) {
                return Err(fail("policy-exit-codes-invalid"));
            }
        }
        if c.inputs.as_ref().map_or(0, Vec::len) > limits.max_inputs {
            return Err(fail("policy-input-count-invalid"));
        }
        for name in c.inputs.iter().flatten() {
            if !relative_input(name) {
                return Err(fail("policy-input-outside-project"));
            }
        }
        c.id = trim_space(&c.id).into();
        if !swarm_identity_contract::name(c.id.as_bytes()) || !ids.insert(c.id.clone()) {
            return Err(fail("policy-control-id-invalid"));
        }
        if c.id == "deliverable" {
            return Err(fail("policy-control-id-reserved"));
        }
        let args = c.command.as_deref().unwrap_or_default();
        if args.is_empty()
            || args.len() > limits.max_arguments
            || !limits.allowed_programs.contains(&args[0])
        {
            return Err(fail("policy-program-or-arguments-invalid"));
        }
        if c.criteria.as_ref().is_none_or(Vec::is_empty) {
            return Err(fail("policy-covered-criteria-required"));
        }
        c.justification = trim_space(&c.justification).into();
        let chars = c.justification.chars().count();
        if chars < limits.min_justification_chars || chars > limits.max_justification_chars {
            return Err(fail("policy-justification-invalid"));
        }
        let mut criteria = BTreeSet::new();
        for index in c.criteria.iter().flatten() {
            if *index < 1 || !criteria.insert(*index) {
                return Err(fail("policy-criterion-index-invalid"));
            }
        }
        for arg in args {
            if trim_space(arg).is_empty() || arg.contains('\0') {
                return Err(fail("policy-argument-invalid"));
            }
        }
        // Go only checks absolute Dir here. Physical containment is the runner's
        // separate responsibility, not established by this normalized DTO.
        if Path::new(&c.dir).is_absolute() {
            return Err(fail("policy-directory-must-be-relative"));
        }
        if c.timeout_seconds == 0 {
            c.timeout_seconds = limits.default_timeout_seconds;
        }
        if c.timeout_seconds < 1 || c.timeout_seconds > limits.max_timeout_seconds {
            return Err(fail("policy-timeout-invalid"));
        }
        total = total
            .checked_add(c.timeout_seconds)
            .ok_or_else(|| fail("policy-total-timeout-overflow"))?;
    }
    if total > limits.max_total_timeout_seconds {
        return Err(fail("policy-total-timeout-invalid"));
    }
    p.authorized_at.clear();
    p.actor.clear();
    Ok(p)
}
pub fn validation_policy_covers_task(
    p: &ValidationPolicy,
    t: &Task,
) -> Result<(), ValidationPolicyError> {
    let mut indices = BTreeSet::new();
    let mut controls = BTreeSet::new();
    let count = t.criteria.as_ref().map_or(0, Vec::len);
    for c in p.controls.iter().flatten() {
        controls.insert(c.id.as_str());
        for index in c.criteria.iter().flatten() {
            if *index > count as i64 {
                return Err(fail("policy-covered-criterion-unknown"));
            }
            indices.insert(*index);
        }
    }
    if p.mode == "human" {
        return Ok(());
    }
    for index in 1..=count {
        if !indices.contains(&(index as i64)) {
            return Err(fail("policy-criterion-uncovered"));
        }
    }
    for id in t.plan_checks.iter().flat_map(|m| m.keys()) {
        if !controls.contains(id.as_str()) {
            return Err(fail("policy-plan-check-unauthorized"));
        }
    }
    Ok(())
}
