//! Managed review invocation protocol assembly for one canonical supplied context.
//!
//! [`build`] packages the exact canonical
//! `ManagedReviewContext` the principal assembled from Work, report, cohort and
//! control data, the complete `CitationCatalog` over the real per-task parser
//! originals, and a descriptive JSON Schema for complete citation-v2 cohort
//! replies. The principal includes the protocol in the public managed inputs
//! and the complete prompt; ordinary outputs are unchanged. The existing native
//! invocation/publication/acceptance guard stays outside this module and is not
//! altered: nothing here performs IO, provider calls, publication, reservation
//! or acceptance, and no packaged data authenticates a reviewer, a worker or
//! any supplied byte's origin, freshness or permission.
//!
//! Saved EngineSettings review_reply_limits govern both the schema's reference
//! enum and reply admission, because
//! `crate::managed_review_reply::parse_managed_review_reply` re-checks every
//! rule against the actual reply. Schema draft and representation constants are
//! protocol, not operator policy. The context, catalog and schema carry private
//! supplied text (reports, diffs, control output), so this type deliberately
//! implements neither `Debug` nor `Display`.

use std::collections::{BTreeSet, HashSet};

use serde::{Deserialize, Serialize};

use crate::managed_review_citations::{build_catalog, CitationCatalog, CitationSource};
use crate::{
    ManagedReviewContext, ManagedReviewTaskContext, ReviewReplyError as E, ReviewReplyLimits,
};

fn refused(message: &'static str) -> E {
    E::Refused(message)
}

/// Complete managed-review invocation protocol for one canonical context.
///
/// `context` is the exact canonical input the v2 parser will re-digest;
/// `context_sha256` is that canonical `encode_model` digest; `catalog` is the
/// complete citation catalog bound to the candidate commit and context digest;
/// `catalog_sha256` is the catalog's canonical JSON digest that every reply
/// must cite; `reply_schema` is descriptive guidance whose consts bind both
/// identities. Serialized into the managed inputs; private text has no `Debug`.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ManagedReviewProtocol {
    pub(crate) context: ManagedReviewContext,
    pub(crate) context_sha256: String,
    pub(crate) catalog: CitationCatalog,
    pub(crate) catalog_sha256: String,
    pub(crate) reply_schema: serde_json::Value,
}

impl ManagedReviewProtocol {
    /// Assemble the protocol for one canonical context the principal built.
    /// The context must be citation-v2, inspection-only at scope version 1
    /// with no accepted baseline (no historical applicability), and must carry
    /// a non-empty cohort of unique, non-empty task IDs; vacuous or ambiguous
    /// cohorts refuse instead of producing a protocol no complete reply could
    /// satisfy. The context digest is the exact canonical `encode_model`
    /// digest the v2 parser recomputes; the catalog binds it and the candidate
    /// commit; the reply schema binds both digests, the exact cohort count and
    /// task IDs, per-task criteria counts and index possibilities, one reason
    /// string, and exclusive pass/fail-unknown evidence forms under the saved
    /// reply limits. When no catalog reference satisfies the saved limits the
    /// schema disallows pass only; fail/unknown remain possible and the
    /// complete catalog is unchanged. The caller must pass the same saved
    /// limits to the parser; any divergence is safe (the parser remains the
    /// syntax/evidence validator and refuses) but may reject schema-allowed refs.
    /// No default, cutoff, retry, timeout, environment setting or grant is
    /// introduced, and no invocation, publication or acceptance occurs.
    ///
    /// # Errors
    /// Returns [`crate::ReviewReplyError`] for wrong contract/scope/baseline shapes,
    /// empty, duplicate or empty-ID cohorts, and typed catalog encoding,
    /// digest, count or limit failures with their causes retained.
    fn assemble(
        context: ManagedReviewContext,
        limits: ReviewReplyLimits,
    ) -> Result<ManagedReviewProtocol, E> {
        if context.citation_contract_version != 2 {
            return Err(refused("managed-review-protocol-contract-version"));
        }
        if !context.inspection_only {
            return Err(refused("managed-review-protocol-inspection-only"));
        }
        if context.inspection_scope_version != 1 {
            return Err(refused("managed-review-protocol-scope-version"));
        }
        if context.accepted_baseline.is_some() {
            return Err(refused("managed-review-protocol-baseline-present"));
        }
        let tasks = match context.tasks.as_deref() {
            Some(tasks) if !tasks.is_empty() => tasks,
            _ => return Err(refused("managed-review-protocol-cohort-empty")),
        };
        let mut seen = BTreeSet::new();
        for task in tasks {
            if task.task.is_empty() {
                return Err(refused("managed-review-protocol-task-id-empty"));
            }
            if !seen.insert(task.task.as_str()) {
                return Err(refused("managed-review-protocol-task-duplicate"));
            }
        }
        let context_sha256 = context_digest(&context)?;
        let originals = collect_originals(&context, tasks)?;
        // Typed propagation: an empty candidate or any encoding failure
        // refuses here under the existing catalog identity policy.
        let catalog = build_catalog(&context.candidate_commit, &context_sha256, &originals)?;
        let catalog_sha256 = catalog.digest()?;
        let resolver =
            catalog.resolver(&context.candidate_commit, &context_sha256, &catalog_sha256)?;
        let admissible = resolver.admissible_references(limits)?;
        let schema = build_reply_schema(&context, tasks, &catalog_sha256, &admissible, limits)?;
        Ok(ManagedReviewProtocol {
            context,
            context_sha256,
            catalog,
            catalog_sha256,
            reply_schema: schema,
        })
    }
}

/// Assemble readonly protocol data using the actual saved reply limits. Requires
/// a nonempty citation-v2 inspection cohort with no inferred accepted baseline.
/// Typed codec/catalog/limit errors survive; no IO, provider or acceptance occurs.
pub(crate) fn build(context: ManagedReviewContext, limits: ReviewReplyLimits) -> Result<ManagedReviewProtocol, E> {
    ManagedReviewProtocol::assemble(context, limits)
}

/// Exact canonical context digest the citation-v2 parser recomputes for
/// catalog resolution; no alternate digest policy is introduced.
fn context_digest(context: &ManagedReviewContext) -> Result<String, E> {
    let encoded = crate::encode_model(context).map_err(E::Format)?;
    Ok(crate::provider_configuration::configuration_digest(&encoded))
}

/// Collect the real parser originals for every cohort task via the shared
/// `managed_review_reply::originals` producer, so catalog bytes and parser
/// evidence bytes can never diverge. Identical (name, kind, content) tuples —
/// recurring diff sections, declared sources and shared control encodings —
/// coalesce as representation only; every distinct tuple is preserved in full
/// (including same-name originals with different content), so no supplied
/// source is truncated. Per-task controls encoding failures propagate typed.
fn collect_originals(
    context: &ManagedReviewContext,
    tasks: &[ManagedReviewTaskContext],
) -> Result<Vec<CitationSource>, E> {
    let mut values: Vec<CitationSource> = Vec::new();
    let mut seen: HashSet<(String, String, String)> = HashSet::new();
    for task in tasks {
        for source in crate::managed_review_reply::originals(context, task)? {
            if seen.insert((source.name.clone(), source.kind.clone(), source.content.clone())) {
                values.push(source);
            }
        }
    }
    Ok(values)
}

/// Checked usize to JSON number conversion; cohort and criteria counts refuse
/// rather than wrap when they exceed the representable range.
fn count_value(value: usize) -> Result<serde_json::Value, E> {
    let converted =
        i64::try_from(value).map_err(|_| refused("managed-review-protocol-count-overflow"))?;
    Ok(serde_json::json!(converted))
}

/// One text field. UTF-8 byte rules are described from the saved limits and
/// enforced by the parser; JSON Schema character lengths are never substituted
/// for byte lengths.
fn text_schema(label: &str, limits: ReviewReplyLimits) -> serde_json::Value {
    serde_json::json!({
        "type": "string",
        "description": format!(
            "{label}. Saved reply limits are UTF-8 byte rules: trimmed minimum {} bytes, raw maximum {} bytes (absent means unlimited). JSON Schema lengths count characters, not UTF-8 bytes, so parse_managed_review_reply performs the exact byte checks.",
            limits.min_text_bytes,
            limits
                .max_text_bytes
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".into())
        )
    })
}

/// Evidence forms for one criterion index. Pass cites an admissible catalog
/// reference exclusively; fail/unknown carry reason text exclusively. When no
/// reference satisfies the saved limits the pass form is omitted entirely and
/// fail/unknown remain possible; the complete catalog is unchanged.
fn criterion_forms(
    index: serde_json::Value,
    admissible: &[i64],
    limits: ReviewReplyLimits,
) -> Vec<serde_json::Value> {
    let mut forms = Vec::new();
    if !admissible.is_empty() {
        forms.push(serde_json::json!({
            "type": "object",
            "description": "pass cites one catalog reference from the saved-limit admissible enum; parse_managed_review_reply revalidates resolution and this task's original evidence membership.",
            "properties": {
                "index": {"const": index},
                "verdict": {"const": "pass"},
                "evidence_ref": {"enum": admissible}
            },
            "required": ["index", "verdict", "evidence_ref"],
            "not": {"required": ["evidence"]},
            "additionalProperties": false
        }));
    }
    for verdict in ["fail", "unknown"] {
        forms.push(serde_json::json!({
            "type": "object",
            "description": "fail/unknown carry reason text and never a catalog reference; byte limits are enforced by parse_managed_review_reply.",
            "properties": {
                "index": {"const": index},
                "verdict": {"const": verdict},
                "evidence": text_schema("Evidence reason text", limits)
            },
            "required": ["index", "verdict", "evidence"],
            "not": {"required": ["evidence_ref"]},
            "additionalProperties": false
        }));
    }
    forms
}

/// Per-criterion oneOf over every possible index (1..=count) and verdict form.
/// JSON Schema cannot enforce that each index appears exactly once; the parser
/// refuses duplicates, omissions and out-of-range indexes.
fn criterion_items(
    count: usize,
    admissible: &[i64],
    limits: ReviewReplyLimits,
) -> Result<serde_json::Value, E> {
    let mut branches = Vec::new();
    for index in 1..=count {
        branches.extend(criterion_forms(count_value(index)?, admissible, limits));
    }
    if branches.is_empty() {
        // A zero-criterion task admits only an empty criteria array; the
        // items schema is never applied to any element.
        return Ok(serde_json::Value::Bool(false));
    }
    Ok(serde_json::json!({ "oneOf": branches }))
}

/// Explain exact coverage and absent admissible references without dropping data.
fn criteria_description(count: usize, admissible: &[i64]) -> String {
    let mut description = format!(
        "Exactly {count} criteria and every declared index 1..={count} must appear exactly once. Index uniqueness and full coverage are enforced by parse_managed_review_reply, not by JSON Schema."
    );
    if admissible.is_empty() {
        description.push_str(" No catalog reference satisfies the saved reply limits, so this schema disallows 'pass' only; 'fail' and 'unknown' remain possible and the complete catalog is unchanged.");
    }
    description
}

/// Describe one exact task and criterion count; the parser separately checks
/// uniqueness, byte limits and quote membership. No data or policy is mutated.
fn build_task_schema(
    task: &ManagedReviewTaskContext,
    admissible: &[i64],
    limits: ReviewReplyLimits,
) -> Result<serde_json::Value, E> {
    let count = task.criteria.as_ref().map_or(0, Vec::len);
    let criteria_count = count_value(count)?;
    Ok(serde_json::json!({
        "type": "object",
        "properties": {
            "task": {"const": task.task},
            "reason": text_schema("One reason string", limits),
            "criteria": {
                "type": "array",
                "description": criteria_description(count, admissible),
                "minItems": criteria_count,
                "maxItems": criteria_count,
                "items": criterion_items(count, admissible, limits)?
            }
        },
        "required": ["task", "reason", "criteria"],
        "additionalProperties": false
    }))
}

/// Bind complete cohort and catalogue identity with a structural reply schema.
/// Counts are checked representations; no invocation or acceptance follows.
fn build_reply_schema(
    context: &ManagedReviewContext,
    tasks: &[ManagedReviewTaskContext],
    catalog_sha256: &str,
    admissible: &[i64],
    limits: ReviewReplyLimits,
) -> Result<serde_json::Value, E> {
    let mut per_task = Vec::new();
    for task in tasks {
        per_task.push(build_task_schema(task, admissible, limits)?);
    }
    let cohort_count = count_value(tasks.len())?;
    Ok(serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "ManagedReviewReply",
        "type": "object",
        "description": "Descriptive schema for a complete managed citation-v2 cohort reply. It is guidance only: syntactic and evidence validity are checked by parse_managed_review_reply; task acceptance is outside this protocol with the saved reply limits, which re-checks candidate and catalog identity, cohort coverage, criterion indexes, evidence forms, duplicate JSON names and all UTF-8 byte rules.",
        "properties": {
            "candidate_commit": {"const": context.candidate_commit},
            "catalog_sha256": {"const": catalog_sha256},
            "tasks": {
                "type": "array",
                "description": "One reply object per cohort task in any order. Per-task 'task' consts pin the exact cohort task IDs and minItems/maxItems pin the exact cohort count; JSON Schema cannot enforce unique ID coverage, so parse_managed_review_reply refuses unknown, missing and duplicate task IDs.",
                "minItems": cohort_count,
                "maxItems": cohort_count,
                "items": {"anyOf": per_task}
            }
        },
        "required": ["candidate_commit", "catalog_sha256", "tasks"],
        "additionalProperties": false
    }))
}

// ---------------------------------------------------------------------------
// Tests: pure in-memory data frozen from any Root live state; no fixture is
// accepted as authority. Tests exercise data parsing, never runtime acceptance.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        managed_review_reply::{parse_managed_review_reply, ParsedManagedReview},
        ManagedTaskReview, ReviewReplyState,
    };

    /// Explicit fixture limits copied from test data only; no installation
    /// default and no Root policy is read or set.
    fn limits() -> ReviewReplyLimits {
        ReviewReplyLimits {
            min_text_bytes: 8,
            max_text_bytes: Some(1000),
            min_normalized_quote_bytes: 8,
        }
    }

    fn task(id: &str) -> ManagedReviewTaskContext {
        ManagedReviewTaskContext {
            task: id.into(),
            title: format!("{id} supplied title"),
            deliverable: format!("docs/{id}.md"),
            criteria: Some(vec![
                "supplied criterion one".into(),
                "supplied criterion two".into(),
            ]),
            report: format!("{id}: supplied complete implementation report text"),
            binding: ManagedTaskReview {
                task: id.into(),
                attempt: format!("attempt-{id}"),
                report: format!("docs/{id}-report.md"),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn context() -> ManagedReviewContext {
        ManagedReviewContext {
            citation_contract_version: 2,
            inspection_scope_version: 1,
            inspection_only: true,
            candidate_commit: "a".repeat(40),
            previous_candidate: "b".repeat(40),
            diff: "diff --git a/docs/alpha.md b/docs/alpha.md\nindex 1111111..2222222 100644\n--- a/docs/alpha.md\n+++ b/docs/alpha.md\n@@ -1 +1 @@\n+supplied implementation line\n".into(),
            tasks: Some(vec![task("alpha"), task("beta")]),
            ..Default::default()
        }
    }

    /// Complete referenced pass reply citing each task's actual report quote.
    fn pass_reply(protocol: &ManagedReviewProtocol) -> serde_json::Value {
        let tasks: Vec<serde_json::Value> = protocol
            .context
            .tasks
            .iter()
            .flatten()
            .map(|task| {
                let criteria: Vec<serde_json::Value> = (1..=task.criteria.as_ref().map_or(0, Vec::len))
                    .map(|index| {
                        let reference = protocol
                            .catalog
                            .quotes
                            .iter()
                            .find(|quote| quote.text == task.report)
                            .expect("fixture report quote present")
                            .id;
                        serde_json::json!({
                            "index": index,
                            "verdict": "pass",
                            "evidence_ref": reference
                        })
                    })
                    .collect();
                serde_json::json!({
                    "task": task.task,
                    "reason": "Supplied complete cohort reason",
                    "criteria": criteria
                })
            })
            .collect();
        serde_json::json!({
            "candidate_commit": protocol.context.candidate_commit,
            "catalog_sha256": protocol.catalog_sha256,
            "tasks": tasks
        })
    }

    fn canonical(value: &serde_json::Value) -> Vec<u8> {
        crate::encode_model(value).unwrap()
    }

    fn parse_reply(
        raw: &[u8],
        context: &ManagedReviewContext,
        catalog: &CitationCatalog,
        limits: ReviewReplyLimits,
    ) -> Result<ParsedManagedReview, E> {
        parse_managed_review_reply(raw, context, Some(catalog), limits, None)
    }

    /// ManagedReviewProtocol deliberately has no Debug, so refusals are
    /// extracted without unwrap_err.
    fn refusal(result: Result<ManagedReviewProtocol, E>) -> &'static str {
        match result {
            Err(error) => error.reason(),
            Ok(_) => panic!("expected managed review protocol refusal"),
        }
    }

    fn first_pass_enum(schema: &serde_json::Value) -> Vec<i64> {
        let task = &schema["properties"]["tasks"]["items"]["anyOf"][0];
        let branches = task["properties"]["criteria"]["items"]["oneOf"]
            .as_array()
            .unwrap();
        let pass = branches
            .iter()
            .find(|branch| branch["properties"]["verdict"]["const"] == "pass")
            .expect("pass branch present");
        pass["properties"]["evidence_ref"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_i64().unwrap())
            .collect()
    }

    #[test]
    fn managed_protocol_build_refuses_invalid_context_shapes() {
        let mut value = context();
        value.citation_contract_version = 1;
        assert_eq!(
            refusal(build(value, limits())),
            "managed-review-protocol-contract-version"
        );
        let mut value = context();
        value.inspection_only = false;
        assert_eq!(
            refusal(build(value, limits())),
            "managed-review-protocol-inspection-only"
        );
        let mut value = context();
        value.inspection_scope_version = 2;
        assert_eq!(
            refusal(build(value, limits())),
            "managed-review-protocol-scope-version"
        );
        let mut value = context();
        value.accepted_baseline = Some(Box::default());
        assert_eq!(
            refusal(build(value, limits())),
            "managed-review-protocol-baseline-present"
        );
        let mut value = context();
        value.tasks = None;
        assert_eq!(
            refusal(build(value, limits())),
            "managed-review-protocol-cohort-empty"
        );
        let mut value = context();
        value.tasks = Some(Vec::new());
        assert_eq!(
            refusal(build(value, limits())),
            "managed-review-protocol-cohort-empty"
        );
        let mut value = context();
        value.tasks.as_mut().unwrap()[1].task = "alpha".into();
        assert_eq!(
            refusal(build(value, limits())),
            "managed-review-protocol-task-duplicate"
        );
        let mut value = context();
        value.tasks.as_mut().unwrap()[0].task = String::new();
        assert_eq!(
            refusal(build(value, limits())),
            "managed-review-protocol-task-id-empty"
        );
        // The existing catalog identity policy propagates typed, unborrowed.
        let mut value = context();
        value.candidate_commit = String::new();
        assert_eq!(
            refusal(build(value, limits())),
            "invalid evidence catalog identity"
        );
    }

    #[test]
    fn managed_protocol_yields_parseable_full_cohort_pass_references() {
        let protocol = build(context(), limits()).unwrap();
        // Exact canonical context digest binding and catalog self-digest.
        assert_eq!(
            protocol.context_sha256,
            crate::provider_configuration::configuration_digest(
                &crate::encode_model(&protocol.context).unwrap()
            )
        );
        assert_eq!(protocol.catalog_sha256, protocol.catalog.digest().unwrap());
        // Only identical (name, kind, content) tuples coalesced: two reports,
        // one shared control encoding, one shared diff section.
        assert_eq!(protocol.catalog.originals.len(), 4);
        assert_eq!(protocol.catalog.quotes.len(), 4);
        // Schema binds identity consts and exact cohort count.
        assert_eq!(
            protocol.reply_schema["properties"]["candidate_commit"]["const"],
            serde_json::json!(protocol.context.candidate_commit)
        );
        assert_eq!(
            protocol.reply_schema["properties"]["catalog_sha256"]["const"],
            serde_json::json!(protocol.catalog_sha256)
        );
        assert_eq!(
            protocol.reply_schema["properties"]["tasks"]["minItems"],
            serde_json::json!(2)
        );
        assert_eq!(
            protocol.reply_schema["properties"]["tasks"]["maxItems"],
            serde_json::json!(2)
        );
        assert_eq!(
            protocol.reply_schema["additionalProperties"],
            serde_json::json!(false)
        );
        assert_eq!(
            protocol.reply_schema["properties"]["tasks"]["items"]["anyOf"][0]
                ["additionalProperties"],
            serde_json::json!(false)
        );
        // The evidence_ref enum equals the saved-limit admissible set exactly.
        let resolver = protocol
            .catalog
            .resolver(
                &protocol.context.candidate_commit,
                &protocol.context_sha256,
                &protocol.catalog_sha256,
            )
            .unwrap();
        let expected = resolver.admissible_references(limits()).unwrap();
        assert_eq!(expected.len(), 3);
        assert_eq!(first_pass_enum(&protocol.reply_schema), expected);
        // The real parser accepts the complete referenced cohort reply.
        let parsed = parse_reply(
            &canonical(&pass_reply(&protocol)),
            &protocol.context,
            &protocol.catalog,
            limits(),
        )
        .unwrap();
        assert_eq!(parsed.state, ReviewReplyState::Passed);
        assert_eq!(parsed.tasks.len(), 2);
        assert_eq!(parsed.tasks[1].attempt, "attempt-beta");
    }

    #[test]
    fn managed_protocol_reply_task_and_criterion_gaps_refuse() {
        let protocol = build(context(), limits()).unwrap();
        let valid = pass_reply(&protocol);
        assert!(parse_reply(
            &canonical(&valid),
            &protocol.context,
            &protocol.catalog,
            limits()
        )
        .is_ok());
        // The parser, not the schema, enforces unique task coverage.
        let mut omitted = valid.clone();
        omitted["tasks"].as_array_mut().unwrap().pop();
        assert!(parse_reply(
            &canonical(&omitted),
            &protocol.context,
            &protocol.catalog,
            limits()
        )
        .is_err());
        let mut duplicate = valid.clone();
        {
            let tasks = duplicate["tasks"].as_array_mut().unwrap();
            tasks[1] = tasks[0].clone();
        }
        assert!(parse_reply(
            &canonical(&duplicate),
            &protocol.context,
            &protocol.catalog,
            limits()
        )
        .is_err());
        // The parser, not the schema, enforces unique full index coverage.
        let mut criterion_duplicate = valid.clone();
        {
            let criteria = criterion_duplicate["tasks"][0]["criteria"].as_array_mut().unwrap();
            criteria[1] = criteria[0].clone();
        }
        assert!(parse_reply(
            &canonical(&criterion_duplicate),
            &protocol.context,
            &protocol.catalog,
            limits()
        )
        .is_err());
        let mut out_of_range = valid.clone();
        out_of_range["tasks"][0]["criteria"][0]["index"] = serde_json::json!(3);
        assert!(parse_reply(
            &canonical(&out_of_range),
            &protocol.context,
            &protocol.catalog,
            limits()
        )
        .is_err());
        let mut missing_criterion = valid;
        missing_criterion["tasks"][0]["criteria"]
            .as_array_mut()
            .unwrap()
            .pop();
        assert!(parse_reply(
            &canonical(&missing_criterion),
            &protocol.context,
            &protocol.catalog,
            limits()
        )
        .is_err());
    }

    #[test]
    fn managed_protocol_refuses_context_and_catalog_digest_drift() {
        let protocol = build(context(), limits()).unwrap();
        let valid = canonical(&pass_reply(&protocol));
        // A reply citing any other catalog digest refuses.
        let mut stale = pass_reply(&protocol);
        stale["catalog_sha256"] = serde_json::json!("f".repeat(64));
        assert!(parse_reply(
            &canonical(&stale),
            &protocol.context,
            &protocol.catalog,
            limits()
        )
        .is_err());
        // Context drift after assembly: the same catalog no longer resolves.
        let mut drifted = context();
        drifted.tasks.as_mut().unwrap()[0].report =
            "drifted supplied report after protocol assembly".into();
        assert!(parse_reply(&valid, &drifted, &protocol.catalog, limits()).is_err());
        // Mutating the assembled protocol's own context breaks its digest.
        let mut mutated = protocol;
        mutated.context.tasks.as_mut().unwrap()[1].report =
            "late drift of assembled protocol context".into();
        assert!(parse_reply(&valid, &mutated.context, &mutated.catalog, limits()).is_err());
    }

    #[test]
    fn managed_protocol_without_admissible_references_keeps_fail_unknown_possible() {
        let restrictive = ReviewReplyLimits {
            min_text_bytes: 10_000,
            max_text_bytes: None,
            min_normalized_quote_bytes: 0,
        };
        let protocol = build(context(), restrictive).unwrap();
        // The complete catalog is unchanged even though nothing is admissible.
        assert_eq!(protocol.catalog.quotes.len(), 4);
        // No pass branch exists anywhere; every declared index keeps
        // fail/unknown possibilities only.
        let any_of = protocol.reply_schema["properties"]["tasks"]["items"]["anyOf"]
            .as_array()
            .unwrap();
        for task_schema in any_of {
            let branches = task_schema["properties"]["criteria"]["items"]["oneOf"]
                .as_array()
                .unwrap();
            assert!(branches
                .iter()
                .all(|branch| branch["properties"]["verdict"]["const"] != "pass"));
            let indexes: BTreeSet<i64> = branches
                .iter()
                .map(|branch| branch["properties"]["index"]["const"].as_i64().unwrap())
                .collect();
            assert_eq!(indexes, BTreeSet::from([1, 2]));
            let verdicts: BTreeSet<&str> = branches
                .iter()
                .map(|branch| branch["properties"]["verdict"]["const"].as_str().unwrap())
                .collect();
            assert_eq!(verdicts, BTreeSet::from(["fail", "unknown"]));
        }
        // The real parser still accepts a complete unknown reply.
        let reason = "r".repeat(10_001);
        let evidence = "e".repeat(10_001);
        let tasks: Vec<serde_json::Value> = protocol
            .context
            .tasks
            .iter()
            .flatten()
            .map(|task| {
                serde_json::json!({
                    "task": task.task,
                    "reason": reason,
                    "criteria": [
                        {"index": 1, "verdict": "unknown", "evidence": evidence},
                        {"index": 2, "verdict": "unknown", "evidence": evidence}
                    ]
                })
            })
            .collect();
        let reply = serde_json::json!({
            "candidate_commit": protocol.context.candidate_commit,
            "catalog_sha256": protocol.catalog_sha256,
            "tasks": tasks
        });
        let parsed = parse_reply(&canonical(&reply), &protocol.context, &protocol.catalog, restrictive)
            .unwrap();
        assert_eq!(parsed.state, ReviewReplyState::ChangesRequested);
        // A pass-shaped reply is refused by the parser under the same limits.
        assert!(parse_reply(
            &canonical(&pass_reply(&protocol)),
            &protocol.context,
            &protocol.catalog,
            restrictive
        )
        .is_err());
    }
    /// Actual public serialization must preserve opaque receipt bytes and both
    /// context/catalogue digests before the real reply parser sees the data.
    #[test]
    fn managed_protocol_survives_public_json_roundtrip() {
        let mut input=context();
        input.receipt=crate::model_wire::persisted_raw(&serde_json::json!({
            "actual_metadata":{"unicode":"<tag>&\nélève\u{2028}","integer":17},"observations":[true,null,"quoted\"data"]
        })).unwrap();
        let protocol=build(input,limits()).unwrap();
        let reply=canonical(&pass_reply(&protocol));
        let projected=serde_json::to_value(&protocol).unwrap();
        let public=crate::encode_model(&projected).unwrap();
        let observed:serde_json::Value=serde_json::from_slice(&public).unwrap();
        let decoded=crate::decode_managed_review_context(&crate::encode_model(&observed["context"]).unwrap(),true).unwrap();
        let catalogue:CitationCatalog=serde_json::from_value(observed["catalog"].clone()).unwrap();
        assert_eq!(context_digest(&decoded).unwrap(),protocol.context_sha256);
        assert_eq!(catalogue.digest().unwrap(),protocol.catalog_sha256);
        assert_eq!(parse_reply(&reply,&decoded,&catalogue,limits()).unwrap().state,ReviewReplyState::Passed);
    }

    /// Saved normalization minimum governs Markdown fallback only; every
    /// catalogue reference resolves exact literal original bytes first.
    #[test]
    fn managed_protocol_literal_quotes_do_not_use_normalization_fallback() {
        let mut saved=limits();
        let reference=build(context(),saved).unwrap();
        saved.min_normalized_quote_bytes=reference.catalog.quotes.iter().map(|quote|quote.text.len()).max().unwrap()+1;
        let protocol=build(context(),saved).unwrap();
        assert_eq!(first_pass_enum(&protocol.reply_schema),first_pass_enum(&reference.reply_schema));
        assert_eq!(parse_reply(&canonical(&pass_reply(&protocol)),&protocol.context,&protocol.catalog,saved).unwrap().state,ReviewReplyState::Passed);
    }

}
