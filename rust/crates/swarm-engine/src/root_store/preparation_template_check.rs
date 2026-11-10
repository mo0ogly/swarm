//! Pure completeness check and work-plan projection for existing preparation
//! templates.
//!
//! Port of the Go `checkPreparationTemplate` and `projectPreparationWorkPlan`
//! helpers over the sibling `preparation_templates` model. The owner resolves
//! the requested template (historical id aliases included) and loads it
//! before calling [`check`]; this module is pure projection:
//!
//! - it never creates agents, storage, receipts or histories, never adopts
//!   the projected plan and never grants readiness: `launch_authorized` is
//!   the constant `false`, missing questions stay permitted in the draft,
//!   and any final plan still passes the existing writer's full product
//!   validation;
//! - every byte budget comes from the supplied policies (`None` is
//!   unlimited, `0` blocks nonempty values); no Go-era 1 000/16 000 literal
//!   or hidden cap is embedded;
//! - plan structure is never re-validated here: the adapted plan is checked
//!   once through the authoritative `super::preparation_plan::validate_draft`
//!   draft validator (partial answers permitted, `require_answers = false`);
//! - diagnostics are static strings under the `preparation-request-invalid`
//!   family for request rejections Go surfaced to its callers and the
//!   `preparation-invalid-template` family for catalogue shapes Go could
//!   only reach by panicking or through zero values; encoding causes keep
//!   the `preparation-encode` family; no supplied text is ever copied into
//!   an error;
//! - catalogue text is data: labels, question lines, guidance, increment
//!   titles and decisions are copied verbatim into the projected document.
//!
//! Registered by the owner as a `root_store` submodule so `super::` resolves.

#![forbid(unsafe_code)]

use super::preparation_read::cause;
use super::RootStoreError as E;
use super::preparation_templates::{Role, Template};
use crate::text::trim_space;
use crate::{
    ActionPlan, PlanQuestion, PreparationPlanPolicy, PreparationTemplatePolicy,
    TaskDefinitionLimits,
};
use std::collections::{BTreeMap, BTreeSet};

/// Supplied answers for one template check (Go `PreparationTemplateAnswers`).
///
/// Missing JSON fields decode to Go zero values; `answers` decodes JSON
/// `null` and an absent field to `None`, which behaves exactly like Go's nil
/// answer map: no key is iterated and every question lookup reads as empty.
#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Answers {
    /// Requested work-model increment; empty selects the first when present.
    #[serde(skip_serializing_if="String::is_empty")]
    pub increment:String,
    /// Template selected by the owner; historical aliases resolve there.
    pub template_id:String,
    /// Empty defaults to fr; exact fr/en are accepted.
    pub language:String,
    /// Raw Go answer map; per-answer bytes are counted before trimming.
    pub answers:Option<BTreeMap<String,String>>,
    /// Requested intervention; empty selects the first when present.
    #[serde(skip_serializing_if="String::is_empty")]
    pub intervention:String,
}

/// Pure projection result (Go `PreparationTemplateCheck`).
///
/// Field names and order reproduce the Go JSON tags. `work_plan`,
/// `increment` and `intervention` stay absent/empty when not selected and
/// `missing` always serializes as `[]` (Go non-nil slice).
/// `launch_authorized` is the constant `false`: a draft check never grants
/// execution.
#[derive(Clone, serde::Serialize)]
pub struct Check {
    /// Adapted draft plan, present only when the template has a work model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_plan: Option<ActionPlan>,
    /// Selected increment id; empty when the template has no work model.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub increment: String,
    pub template_id: String,
    pub version: i64,
    pub title: String,
    pub need: String,
    pub missing: Vec<String>,
    pub answered: i64,
    pub total: i64,
    pub need_complete: bool,
    /// Invariant `false`: draft completeness never authorizes a launch.
    pub launch_authorized: bool,
    /// Frozen team of the template as loaded, workflow included.
    pub proposed_team: Vec<Role>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub intervention: String,
    pub recommended_method: String,
}

/// Static refusal for everything Go surfaced to its caller as a request
/// error; the family and message are static and no supplied text joins them.
fn refuse(message: &'static str) -> E {
    cause("preparation-request-invalid", E::Refused(message))
}

/// Static refusal for catalogue shapes Go could only reach by panicking or
/// through zero values; the owner's loader normally prevents them.
fn catalog(message: &'static str) -> E {
    cause("preparation-invalid-template", E::Refused(message))
}

/// Policy cap: `None` is unlimited, `Some(cap)` rejects `len` above it, so a
/// cap of zero refuses any nonempty value.
fn over(cap: Option<usize>, len: usize) -> bool {
    match cap {
        None => false,
        Some(cap) => len > cap,
    }
}

/// The single draft byte budget Go applied to the need document, to the
/// canonical projected plan and to the need document after the work graph:
/// refused when above `limits.draft_max_bytes`.
fn enforce_draft_cap(limits: &PreparationTemplatePolicy, len: usize) -> Result<(), E> {
    if over(limits.draft_max_bytes, len) {
        return Err(refuse("draft exceeds the configured byte budget"));
    }
    Ok(())
}

/// Go map lookup `answers[id]`: a nil map or a missing key reads as `""`.
fn answer_of<'a>(answers: Option<&'a BTreeMap<String, String>>, id: &str) -> &'a str {
    answers
        .and_then(|map| map.get(id))
        .map(String::as_str)
        .unwrap_or("")
}

/// The catalogue's localized "still to be defined" markers, matched anywhere
/// inside a trimmed answer exactly as in Go; such answers are never filled.
fn has_placeholder(answer: &str) -> bool {
    answer.contains("[à préciser") || answer.contains("[define") || answer.contains("[describe")
}

/// Check draft completeness for one request and project the work plan.
///
/// Inputs: `t` is an already-resolved, loader-validated template; `r` the
/// request (an empty language defaults to `fr`, only exact `fr`/`en` pass);
/// `limits` carries the answer and draft byte budgets; `plan_policy` and
/// `task` are forwarded unchanged to the authoritative draft validator.
///
/// Effects: builds the localized need document and fills exactly one
/// occurrence of each answered question's localized line between its first
/// `[` and its last `]`; records required questions whose trimmed answer is
/// blank or still carries a placeholder as missing; appends the selected
/// intervention's title and guidance; clones the selected increment's plan
/// for the language, appends one open question per missing template question
/// and one joined assumption line per supplied answer, forces
/// `product.mode = "existing"` unless the intervention is `create`, appends
/// the intervention guidance, then appends the work graph and the localized
/// decisions. Pure: neither `t` nor `r` is mutated and nothing outside the
/// returned [`Check`] is produced, stored or authorized.
///
/// Errors: a language other than `fr`/`en`, an unknown answer key or an
/// answer above the configured budget (bytes counted before trimming, blank
/// values under unknown keys included), an intervention the template does
/// not offer, a missing work graph, an unknown increment, an invalid
/// question line and any draft above the configured budget refuse under
/// `preparation-request-invalid`; catalogue shapes the loader should have
/// rejected refuse under `preparation-invalid-template`; encoding and
/// draft-validation causes propagate unchanged.
///
/// Limits: budgets come only from the supplied policies (`None` unlimited,
/// `0` blocks nonempty); the question counter is a checked conversion and
/// `answered` stays bounded by it; no Go-era literal is embedded.
pub(super) fn check(
    t: &Template,
    r: &Answers,
    limits: &PreparationTemplatePolicy,
    plan_policy: &PreparationPlanPolicy,
    task: &TaskDefinitionLimits,
) -> Result<Check, E> {
    // Go defaults an empty language to FR; aliases were resolved by the
    // owner before this helper, so only exact fr/en remain.
    let language: &str = if r.language.is_empty() { "fr" } else { r.language.as_str() };
    if language != "fr" && language != "en" {
        return Err(refuse("language must be fr or en"));
    }

    // Every supplied key must be a known question and fit the policy budget;
    // bytes are counted before trimming and a blank value under an unknown
    // key is still refused (Go map semantics).
    let known: BTreeSet<&str> = t.questions.iter().map(|q| q.id.as_str()).collect();
    if let Some(map) = r.answers.as_ref() {
        for (key, answer) in map {
            if !known.contains(key.as_str()) || over(limits.answer_max_bytes, answer.len()) {
                return Err(refuse(
                    "unknown answer key or answer exceeds the configured byte budget",
                ));
            }
        }
    }

    // Checked conversion: the question counter never truncates.
    let total = i64::try_from(t.questions.len())
        .map_err(|err| cause("preparation-invalid-template", err))?;

    let mut v = Check {
        work_plan: None,
        increment: String::new(),
        template_id: t.id.clone(),
        version: t.version,
        title: t.title.get(language).cloned().unwrap_or_default(),
        need: t.need.get(language).cloned().unwrap_or_default(),
        missing: Vec::new(),
        answered: 0,
        total,
        need_complete: false,
        // Invariant: a draft check never authorizes execution.
        launch_authorized: false,
        proposed_team: t.team.clone(),
        intervention: String::new(),
        recommended_method: t.recommended_method.clone(),
    };

    // Subject: default to the first intervention, refuse an unknown request
    // and refuse untranslated or out-of-protocol catalogue interventions
    // (only ks-product/debug exist, as in Go).
    if let Some(subject) = t.subject.as_ref() {
        let mut requested: &str = r.intervention.as_str();
        if requested.is_empty() {
            if let Some(first) = subject.interventions.first() {
                requested = first.id.as_str();
            }
        }
        let mut found = false;
        for intervention in subject.interventions.iter() {
            if intervention.id != requested {
                continue;
            }
            found = true;
            v.intervention = intervention.id.clone();
            v.recommended_method = intervention.recommended_method.clone();
            let title = intervention.title.get(language).cloned().unwrap_or_default();
            let guidance = intervention
                .guidance
                .get(language)
                .cloned()
                .unwrap_or_default();
            if title.is_empty()
                || guidance.is_empty()
                || (v.recommended_method != "ks-product" && v.recommended_method != "debug")
            {
                return Err(refuse("intervention of the preparation template is invalid"));
            }
            v.title.push_str(" — ");
            v.title.push_str(&title);
            v.need.push_str("\n## ");
            v.need.push_str(&title);
            v.need.push('\n');
            v.need.push_str(&guidance);
            v.need.push('\n');
            break;
        }
        if !found {
            return Err(refuse("intervention of the preparation template is invalid"));
        }
    } else if !r.intervention.is_empty() {
        return Err(refuse("this preparation template does not offer an intervention choice"));
    }

    // Fill each answered question line exactly once; blank or placeholder
    // answers of required questions are recorded as missing and skipped.
    for q in t.questions.iter() {
        let answer = trim_space(answer_of(r.answers.as_ref(), &q.id));
        if answer.is_empty() || has_placeholder(answer) {
            if q.required {
                v.missing.push(q.id.clone());
            }
            continue;
        }
        let line = q.line.get(language).map(String::as_str).unwrap_or("");
        // Go slices the localized line between its first '[' and last ']'.
        let (start, end) = match (line.find('['), line.rfind(']')) {
            (Some(start), Some(end)) if end >= start => (start, end),
            _ => return Err(refuse("invalid question line in the preparation template")),
        };
        // The line must occur exactly once in the need built so far.
        if v.need.match_indices(line).count() != 1 {
            return Err(refuse("invalid question line in the preparation template"));
        }
        let filled = format!("{}{}{}", &line[..start], answer, &line[end + 1..]);
        v.need = v.need.replacen(line, &filled, 1);
        // Bounded by `total`, itself a checked i64: cannot overflow.
        v.answered += 1;
    }

    enforce_draft_cap(limits, v.need.len())?;
    v.need_complete = v.missing.is_empty();
    project_work_plan(t, r, language, &mut v, limits, plan_policy, task)?;
    Ok(v)
}

/// Project the selected increment's catalogue plan into the check result.
///
/// Inputs: `t` is the template, `r` the request, `language` the already
/// validated `fr`/`en` selection and `v` the partially built result whose
/// `missing`, `intervention` and `recommended_method` are final (the
/// effective recommended method is the validator's method context).
///
/// Effects: a template without a work model only refuses a nonempty
/// requested increment; otherwise the increment defaults to the first one,
/// the language plan is cloned (never shared between requests), enriched
/// with one open question per missing template question and one joined
/// assumption line per supplied answer, forced to `product.mode = "existing"`
/// unless the selected intervention is `create`, extended with the
/// intervention guidance, canonically encoded, capped, validated once
/// through `super::preparation_plan::validate_draft` (draft mode: missing
/// answers stay permitted, no adoption) and stored with the increment id;
/// finally the work graph (`id · title ← dependencies`, then one
/// `- decision` line per localized decision) is appended to the need
/// document. The catalogue is never modified.
///
/// Errors: an increment requested from a template without a work model and
/// an unknown increment refuse under `preparation-request-invalid`; a work
/// model without increments, an increment without the language plan or a
/// plan without a product to mark existing refuse under
/// `preparation-invalid-template`; encoding and draft-validation causes
/// propagate unchanged.
///
/// Limits: the canonical plan bytes and the need document after the graph
/// are bounded by `limits.draft_max_bytes` (`None` unlimited, `0` blocks
/// nonempty), exactly as Go bounded both with one constant.
fn project_work_plan(
    t: &Template,
    r: &Answers,
    language: &str,
    v: &mut Check,
    limits: &PreparationTemplatePolicy,
    plan_policy: &PreparationPlanPolicy,
    task: &TaskDefinitionLimits,
) -> Result<(), E> {
    let Some(work_model) = t.work_model.as_ref() else {
        if !r.increment.is_empty() {
            return Err(refuse("this preparation template does not propose a work graph"));
        }
        return Ok(());
    };

    // Go defaults an empty selection to the first increment; a work model
    // without increments cannot occur after loading, so refuse instead of
    // indexing.
    let requested: &str = if r.increment.is_empty() {
        match work_model.increments.first() {
            Some(first) => first.id.as_str(),
            None => return Err(catalog("work model declares no increment")),
        }
    } else {
        r.increment.as_str()
    };
    let Some(inc) = work_model.increments.iter().find(|inc| inc.id == requested) else {
        // Go quoted the requested id; diagnostics stay static here.
        return Err(refuse("unknown work increment"));
    };

    // A missing language plan cannot occur after loading; refuse instead of
    // projecting a zero-valued plan.
    let source = inc
        .plans
        .get(language)
        .ok_or_else(|| catalog("work model increment has no plan for the requested language"))?;
    // Copy before adapting so one request never changes another projection.
    let mut plan: ActionPlan = source.clone();

    // One open question per missing template question and one joined
    // assumption line for every supplied answer, in catalogue order.
    let mut context: Vec<String> = Vec::new();
    for q in t.questions.iter() {
        let answer = trim_space(answer_of(r.answers.as_ref(), &q.id));
        if v.missing.iter().any(|missing| missing == &q.id) {
            plan.questions
                .get_or_insert_with(Vec::new)
                .push(PlanQuestion {
                    question: q.label.get(language).cloned().unwrap_or_default(),
                    answer: String::new(),
                });
        } else if !answer.is_empty() {
            context.push(format!(
                "{}: {}",
                q.label.get(language).cloned().unwrap_or_default(),
                answer
            ));
        }
    }
    if !context.is_empty() {
        plan.assumptions
            .get_or_insert_with(Vec::new)
            .push(context.join("\n"));
    }

    // A missing product is a malformed catalogue: refuse, never panic.
    if v.intervention != "create" {
        let product = plan
            .product
            .as_deref_mut()
            .ok_or_else(|| catalog("work model plan has no product to mark existing"))?;
        product.mode = "existing".to_string();
    }

    // The intervention stays explicit in the actual plan, including the
    // correction reproduction and migration preservation already specified
    // by the catalogue.
    if let Some(subject) = t.subject.as_ref() {
        for intervention in subject.interventions.iter() {
            if intervention.id == v.intervention {
                plan.assumptions
                    .get_or_insert_with(Vec::new)
                    .push(intervention.guidance.get(language).cloned().unwrap_or_default());
            }
        }
    }

    // The canonical Go-compatible encoding is the only byte basis, as in the
    // plan validator.
    let encoded = crate::encode_model(&plan).map_err(|err| cause("preparation-encode", err))?;
    enforce_draft_cap(limits, encoded.len())?;
    // Authoritative draft validation without adoption; the returned decoded
    // copy is discarded and the adapted plan itself is reported.
    let _ = super::preparation_plan::validate_draft(
        &encoded,
        v.recommended_method.as_str(),
        plan_policy,
        task,
    )?;

    // Work graph: exact `taskID · title ← dependencies`, then decisions.
    let mut graph = String::new();
    graph.push_str("\n## ");
    graph.push_str(inc.title.get(language).map(String::as_str).unwrap_or(""));
    graph.push('\n');
    for mission in plan.tasks.as_deref().unwrap_or(&[]) {
        graph.push_str(&mission.id);
        graph.push_str(" · ");
        graph.push_str(&mission.title);
        graph.push_str(" ← ");
        graph.push_str(&mission.depends.as_deref().unwrap_or(&[]).join(", "));
        graph.push('\n');
    }
    for decision in work_model
        .decisions
        .get(language)
        .map(Vec::as_slice)
        .unwrap_or(&[])
    {
        graph.push_str("- ");
        graph.push_str(decision);
        graph.push('\n');
    }

    v.work_plan = Some(plan);
    v.increment = inc.id.clone();
    v.need.push_str(&graph);
    enforce_draft_cap(limits, v.need.len())?;
    Ok(())
}

/// Strict Go-compatible questionnaire decoding, including aliases, duplicate
/// member order, map merging and null strings. Unknown/mistyped fields retain
/// a safe request-invalid cause. No IO, template selection or authority follows.
pub(super) fn decode(raw:&[u8])->Result<Answers,E>{
    use crate::model_wire::{Fields,Kind};
    fn shape()->Fields {&[("template_id",Kind::String),("language",Kind::String),("answers",Kind::Map(&Kind::String)),("intervention",Kind::String),("increment",Kind::String)]}
    crate::work::decode_mode(raw,shape(),true).map_err(|e|cause("preparation-request-invalid",e))
}
