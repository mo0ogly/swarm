//! Strict `ActionPlan` v1 preparation validation.
//!
//! Port of the Go `validateActionPlan`, `validateProductPlan` and
//! `validatePreparedProduct` checks. Pure data validation: a returned plan is
//! not adopted, launched or authorized here. Byte/list caps come only from the
//! supplied policy, per-task ceilings only from the saved task definition
//! limits; no Go-era literal cutoff is embedded. Errors carry only static
//! diagnostics under the `preparation-invalid-plan` family and never quote
//! supplied plan text; codec/encode causes are preserved.
//!
//! Registered by the owner as a `root_store` submodule so `super::` resolves.

#![forbid(unsafe_code)]

use super::preparation_read::cause;
use super::RootStoreError as E;
use crate::text::nonempty;
use crate::{ActionPlan, PreparationPlanPolicy, TaskDefinitionLimits};
use std::collections::{BTreeMap, BTreeSet};

/// Versioned protocol vocabulary of product phases (Go `productPhase`).
const PRODUCT_PHASES: [&str; 13] = [
    "frame",
    "requirements",
    "stories",
    "story-review",
    "architecture",
    "design-system",
    "research",
    "design",
    "plan",
    "implement",
    "review",
    "deliver",
    "retex",
];

/// Versioned protocol vocabulary of product modes.
const PRODUCT_MODES: [&str; 3] = ["existing", "greenfield", "replacement"];

/// Method whose preparation requires the full product structure.
const PRODUCT_METHOD: &str = "ks-product";

/// The only role an executable mission may carry.
const WORKER_ROLE: &str = "worker";

/// Retain a safe static rule as a typed cause of the public invalid-plan
/// family. No supplied text is copied into diagnostics or discarded on errors.
fn rule(message: &'static str) -> E {
    cause("preparation-invalid-plan", E::Refused(message))
}

/// Policy cap: `None` is unlimited, `Some(cap)` rejects `len` above it, so a
/// cap of zero refuses any nonempty list.
fn over(cap: Option<usize>, len: usize) -> bool {
    match cap {
        None => false,
        Some(cap) => len > cap,
    }
}

/// Reuse the authoritative shared identity shape; per-plan32/64 byte bounds
/// below belong to the versioned local-ID representation, not operator policy.
fn safe_name(id: &str) -> bool {
    swarm_identity_contract::name(id.as_bytes())
}

/// Story identifier shape (Go `storyName`): `s`, at least one ASCII digit,
/// '-', then one or more hyphen-separated segments of lowercase ASCII or
/// digits. Length is checked by the caller, as in Go.
fn story_name(id: &str) -> bool {
    let b = id.as_bytes();
    if b.first() != Some(&b's') {
        return false;
    }
    let mut i = 1;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if i == 1 || i >= b.len() || b[i] != b'-' {
        return false;
    }
    i += 1;
    loop {
        let start = i;
        while i < b.len() && (b[i].is_ascii_lowercase() || b[i].is_ascii_digit()) {
            i += 1;
        }
        if i == start {
            return false; // empty segment; also rejects a trailing '-'
        }
        if i == b.len() {
            return true;
        }
        if b[i] != b'-' {
            return false;
        }
        i += 1;
    }
}

/// Every reference must be known and appear at most once (Go
/// `productReferences`).
fn references_ok(refs: &[String], known: &BTreeSet<&str>) -> bool {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    refs.iter()
        .all(|r| known.contains(r.as_str()) && seen.insert(r.as_str()))
}

/// Iterative Kahn cycle check. `edges` are `(prerequisite, dependent)` pairs
/// whose endpoints are registered in `nodes`; true when the graph is acyclic.
/// Inconsistent internal edges/count representation return false, never panic.
/// Iterative on purpose: policy-configured unbounded lists must not recurse.
fn acyclic(nodes: &[&str], edges: &[(&str, &str)]) -> bool {
    let mut indegree: BTreeMap<&str, usize> = nodes.iter().map(|&n| (n, 0)).collect();
    let mut dependents: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for &(prereq, dependent) in edges {
        let Some(degree)=indegree.get_mut(dependent) else {return false};
        let Some(next)=degree.checked_add(1) else {return false};
        *degree=next;
        dependents.entry(prereq).or_default().push(dependent);
    }
    let mut ready: Vec<&str> = Vec::new();
    for (id, degree) in indegree.iter() {
        if *degree == 0 {
            ready.push(*id);
        }
    }
    let mut cursor = 0usize;
    while cursor < ready.len() {
        let id = ready[cursor];
        cursor += 1;
        if let Some(later) = dependents.get(id) {
            for &next in later {
                let Some(degree) = indegree.get_mut(next) else {return false};
                let Some(value)=degree.checked_sub(1) else {return false};
                *degree=value;
                if *degree == 0 {
                    ready.push(next);
                }
            }
        }
    }
    ready.len() == nodes.len()
}

/// Validate a strictly decoded `ActionPlan` v1 for preparation storage.
///
/// Mirrors the Go sequence: strict codec decode, normalized byte budget from
/// the policy, cadre requirements (version 1, objective, assumption/question
/// lists present, at least one task within the policy cap), list caps,
/// per-task structure with ceilings from the saved task limits, iterative
/// dependency cycle detection, product phase vocabulary (with or without a
/// product), full product structure when present, and the `ks-product`
/// product requirement. The decoded plan is returned unchanged; no task
/// ordering is produced.
pub(crate) fn validate(
    raw: &[u8],
    method: &str,
    policy: &PreparationPlanPolicy,
    task: &TaskDefinitionLimits,
) -> Result<ActionPlan, crate::RootStoreError> {
    validate_inner(raw, method, policy, task, true)
}

/// Shared structural check. `require_answers=false` permits partial decisions,
/// but never creates a verdict; task/product rules and saved caps still apply.
/// Only final validation requires the method-specific complete product contract.
fn validate_inner(
    raw: &[u8], method: &str, policy: &PreparationPlanPolicy,
    task: &TaskDefinitionLimits, require_answers: bool,
) -> Result<ActionPlan, E> {
    let plan =
        crate::work::decode_mode::<ActionPlan>(raw, crate::models::shape_action_plan(), true)
            .map_err(|err| cause("preparation-invalid-plan", err))?;

    // The normalized Go-compatible encoding is the only length basis (Go
    // marshals the plan before every other check). `None` is unlimited, so no
    // length is computed at all.
    if let Some(max_bytes) = policy.max_bytes {
        let encoded =
            crate::encode_model(&plan).map_err(|err| cause("preparation-invalid-plan", err))?;
        if encoded.len() > max_bytes {
            return Err(rule("plan exceeds the configured byte budget"));
        }
    }

    let tasks = plan.tasks.as_deref().unwrap_or(&[]);
    if plan.version != 1
        || !nonempty(&plan.objective)
        || plan.assumptions.is_none()
        || plan.questions.is_none()
        || tasks.is_empty()
        || over(policy.max_tasks, tasks.len())
    {
        return Err(rule(
            "plan requires version 1, an objective, assumption and question lists, and at least one task",
        ));
    }

    let assumptions = plan.assumptions.as_deref().unwrap_or(&[]);
    if over(policy.max_assumptions, assumptions.len()) {
        return Err(rule("too many assumptions"));
    }
    for a in assumptions {
        if !nonempty(a) {
            return Err(rule("empty assumption"));
        }
    }

    let questions = plan.questions.as_deref().unwrap_or(&[]);
    if over(policy.max_questions, questions.len()) {
        return Err(rule("too many questions"));
    }
    for q in questions {
        // Saving answers allows incomplete decisions; verdicts do not.
        if !nonempty(&q.question) || (require_answers && !nonempty(&q.answer)) {
            return Err(rule("unresolved decision: question and answer are required"));
        }
    }

    let mut by: BTreeMap<&str, _> = BTreeMap::new();
    for t in tasks {
        if !safe_name(&t.id) || t.id.len() > 32 {
            return Err(rule("invalid mission identifier"));
        }
        if by.contains_key(t.id.as_str()) {
            return Err(rule("duplicate mission identifier"));
        }
        if t.role != WORKER_ROLE {
            return Err(rule("mission role must be worker"));
        }
        if !nonempty(&t.title)
            || !nonempty(&t.scope)
            || !nonempty(&t.deliverable)
            || !nonempty(&t.proof)
            || !nonempty(&t.entry)
            || !nonempty(&t.validation)
            || !nonempty(&t.delivery)
            || !nonempty(&t.stop)
        {
            return Err(rule(
                "mission requires title, scope, deliverable, proof, gate entry/validation/delivery and stop",
            ));
        }
        let criteria = t.criteria.as_deref().unwrap_or(&[]);
        if criteria.is_empty()
            || criteria.len() > task.max_criteria
            || t.depends.is_none()
            || t.max_attempts < 1
            || t.max_attempts > task.max_attempts
            || t.max_tool_calls < 1
            || t.max_tool_calls > task.max_tool_calls
        {
            return Err(rule(
                "mission requires criteria, a dependency list and configured attempt/tool ceilings",
            ));
        }
        for c in criteria {
            if !nonempty(c) {
                return Err(rule("empty mission criterion"));
            }
        }
        by.insert(t.id.as_str(), t);
    }

    // Dependencies: reject repeats and unknown targets, then detect cycles.
    let mut task_edges: Vec<(&str, &str)> = Vec::new();
    for t in tasks {
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for d in t.depends.as_deref().unwrap_or(&[]) {
            if !seen.insert(d.as_str()) {
                return Err(rule("repeated mission dependency"));
            }
            if !by.contains_key(d.as_str()) {
                return Err(rule("unknown mission dependency"));
            }
            task_edges.push((d.as_str(), t.id.as_str()));
        }
    }
    let task_nodes: Vec<&str> = by.keys().copied().collect();
    if !acyclic(&task_nodes, &task_edges) {
        return Err(rule("mission dependency cycle"));
    }

    // Product phase vocabulary applies whether or not a product is present.
    for t in tasks {
        if !t.phase.is_empty() && !PRODUCT_PHASES.contains(&t.phase.as_str()) {
            return Err(rule("unknown product phase"));
        }
    }

    let product = match plan.product.as_deref() {
        Some(product) => product,
        None => {
            if require_answers && method == PRODUCT_METHOD {
                return Err(rule(
                    "the full application journey method requires the product structure",
                ));
            }
            return Ok(plan);
        }
    };

    let stories_list = product.stories.as_deref().unwrap_or(&[]);
    let journeys_list = product.journeys.as_deref().unwrap_or(&[]);
    if !PRODUCT_MODES.contains(&product.mode.as_str())
        || journeys_list.is_empty()
        || stories_list.is_empty()
    {
        return Err(rule(
            "product requires mode existing/greenfield/replacement, journeys and stories",
        ));
    }

    let task_ids: BTreeSet<&str> = by.keys().copied().collect();
    let mut stories: BTreeMap<&str, _> = BTreeMap::new();
    for s in stories_list {
        if !story_name(&s.id)
            || s.id.len() > 64
            || !nonempty(&s.title)
            || !nonempty(&s.user)
            || !nonempty(&s.value)
            || s.criteria.as_deref().map_or(true, |c| c.is_empty())
            || s.depends.is_none()
            || s.task_ids.is_none()
        {
            return Err(rule(
                "story requires an sN-slug identifier, title, user, value, criteria and lists",
            ));
        }
        if stories.contains_key(s.id.as_str())
            || s.complexity < 1
            || s.complexity > 4
            || (s.complexity == 4 && !nonempty(&s.risk))
        {
            return Err(rule(
                "duplicate story or invalid complexity; complexity 4 requires an explicit risk",
            ));
        }
        for c in s.criteria.as_deref().unwrap_or(&[]) {
            if !nonempty(c) {
                return Err(rule("empty story criterion"));
            }
        }
        if !references_ok(s.task_ids.as_deref().unwrap_or(&[]), &task_ids) {
            return Err(rule("story task reference unknown or repeated"));
        }
        stories.insert(s.id.as_str(), s);
    }

    let known_stories: BTreeSet<&str> = stories.keys().copied().collect();
    let mut journey_ids: BTreeSet<&str> = BTreeSet::new();
    let mut covered: BTreeSet<&str> = BTreeSet::new();
    for j in journeys_list {
        if !safe_name(&j.id)
            || j.id.len() > 64
            || journey_ids.contains(j.id.as_str())
            || !nonempty(&j.title)
            || !nonempty(&j.goal)
            || j.story_ids.as_deref().map_or(true, |v| v.is_empty())
        {
            return Err(rule(
                "journey requires a unique identifier, title, goal and stories",
            ));
        }
        journey_ids.insert(j.id.as_str());
        let story_refs = j.story_ids.as_deref().unwrap_or(&[]);
        if !references_ok(story_refs, &known_stories) {
            return Err(rule("journey story reference unknown or repeated"));
        }
        covered.extend(story_refs.iter().map(|id| id.as_str()));
    }

    // Stories: journey coverage, then dependency shape, then acyclicity.
    let mut story_edges: Vec<(&str, &str)> = Vec::new();
    for s in stories_list {
        if !covered.contains(s.id.as_str()) {
            return Err(rule("story is not covered by any journey"));
        }
        let deps = s.depends.as_deref().unwrap_or(&[]); // None already rejected
        if !references_ok(deps, &known_stories) {
            return Err(rule("story dependency unknown or repeated"));
        }
        for d in deps {
            story_edges.push((d.as_str(), s.id.as_str()));
        }
    }
    let story_nodes: Vec<&str> = stories.keys().copied().collect();
    if !acyclic(&story_nodes, &story_edges) {
        return Err(rule("story dependency cycle"));
    }

    Ok(plan)
}

/// Replace only answers in an exact ordered question list. Strict-decode both
/// plan and decisions, enforce current plan/task/document caps, and emit Go-shaped
/// indented JSON with HTML/JS escaping. Partial answers remain valid drafts.
/// This pure helper changes no stored provenance, receipt, history or verdict.
/// Malformed inputs retain safe causes; changed questions refuse as stale data.
pub(super) fn answer_questions(
    raw: &[u8], decisions: &[serde_json::Value], method: &str,
    policy: &PreparationPlanPolicy, task: &TaskDefinitionLimits,
    document_cap: Option<usize>,
) -> Result<String, E> {
    use serde::Serialize;
    let mut plan=validate_inner(raw,method,policy,task,false)?;
    let questions=plan.questions.as_mut().ok_or(E::Refused("preparation-invalid-plan"))?;
    if decisions.len()!=questions.len() {return Err(E::Refused("preparation-request-invalid"));}
    for (question,decision) in questions.iter_mut().zip(decisions) {
        if !decision.is_object() {return Err(E::Refused("preparation-request-invalid"));}
        let encoded=crate::encode_model(decision).map_err(|e|cause("preparation-request-invalid",e))?;
        let supplied=crate::work::decode_mode::<crate::PlanQuestion>(
            &encoded,crate::models::shape_plan_question(),true)
            .map_err(|e|cause("preparation-request-invalid",e))?;
        if supplied.question!=question.question {return Err(E::Refused("preparation-stale-document"));}
        question.answer=supplied.answer;
    }
    let mut pretty=Vec::new();
    plan.serialize(serde_stacker::Serializer::new(&mut serde_json::Serializer::pretty(&mut pretty)))
        .map_err(|e|cause("preparation-encode",e))?;
    // RawValue keeps typed field order/pretty whitespace while encode_model
    // applies the established Go string escaping, including U+2028/U+2029.
    let pretty=String::from_utf8(pretty).map_err(|e|cause("preparation-encode",e))?;
    let value=serde_json::value::RawValue::from_string(pretty)
        .map_err(|e|cause("preparation-encode",e))?;
    let encoded=crate::encode_model(&value).map_err(|e|cause("preparation-encode",e))?;
    if document_cap.is_some_and(|cap|encoded.len()>cap) {
        return Err(E::Refused("preparation-request-invalid"));
    }
    validate_inner(&encoded,method,policy,task,false)?;
    String::from_utf8(encoded).map_err(|e|cause("preparation-encode",e))
}
