//! Supplied-data complete-candidate opinions, never Root acceptance authority.
use crate::{
    managed_review_citations::{CitationCatalog, CitationSource},
    model_wire::{Fields, Kind},
    ManagedReviewContext, ManagedReviewTaskContext, ManagedTaskReview, ReviewCriterion,
    ReviewQuoteOriginal, ReviewReplyError as E, ReviewReplyLimits, ReviewReplyState,
};
use serde::{
    de::{MapAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use serde_json::value::RawValue;
use std::{collections::BTreeSet, fmt};

/// Complete parsed opinion data. Criterion fail/unknown remains changes_requested;
/// syntax and quote validity prove neither reviewer identity nor criterion truth.
pub(crate) struct ParsedManagedReview {
    pub state: ReviewReplyState,
    pub tasks: Vec<ManagedTaskReview>,
}
#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
struct TaskReply {
    task: String,
    reason: String,
    criteria: Option<Vec<ReviewCriterion>>,
}
#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
struct LiteralReply {
    candidate_commit: String,
    tasks: Option<Vec<TaskReply>>,
}
fn task_shape() -> Fields {
    &[
        ("task", Kind::String),
        ("reason", Kind::String),
        (
            "criteria",
            Kind::Array(&Kind::Object(crate::models::shape_review_criterion)),
        ),
    ]
}
fn literal_shape() -> Fields {
    &[
        ("candidate_commit", Kind::String),
        ("tasks", Kind::Array(&Kind::Object(task_shape))),
    ]
}
#[derive(Default, Deserialize)]
#[serde(default)]
struct ReferencedCriterion {
    index: i64,
    verdict: String,
    evidence: Option<String>,
    evidence_ref: Option<i64>,
}
#[derive(Default, Deserialize)]
#[serde(default)]
struct ReferencedTask {
    task: String,
    reason: String,
    criteria: Option<Vec<ReferencedCriterion>>,
}
#[derive(Default, Deserialize)]
#[serde(default)]
struct ReferencedReply {
    catalog_sha256: String,
    candidate_commit: String,
    tasks: Option<Vec<ReferencedTask>>,
}
fn referenced_criterion_shape() -> Fields {
    &[
        ("index", Kind::Int64),
        ("verdict", Kind::String),
        ("evidence", Kind::Pointer(&Kind::String)),
        ("evidence_ref", Kind::Pointer(&Kind::Int64)),
    ]
}
fn referenced_task_shape() -> Fields {
    &[
        ("task", Kind::String),
        ("reason", Kind::String),
        (
            "criteria",
            Kind::Array(&Kind::Object(referenced_criterion_shape)),
        ),
    ]
}
fn referenced_shape() -> Fields {
    &[
        ("catalog_sha256", Kind::String),
        ("candidate_commit", Kind::String),
        ("tasks", Kind::Array(&Kind::Object(referenced_task_shape))),
    ]
}

/// Preserve raw values while rejecting duplicate decoded object names. The error
/// is static; it never renders a supplied key, quote or original source.
struct UniqueObject(Vec<(String, Box<RawValue>)>);
impl<'de> Deserialize<'de> for UniqueObject {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ObjectVisitor;
        impl<'de> Visitor<'de> for ObjectVisitor {
            type Value = UniqueObject;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("unique JSON object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut seen = BTreeSet::new();
                let mut entries = Vec::new();
                while let Some((key, value)) = map.next_entry::<String, Box<RawValue>>()? {
                    if !seen.insert(key.clone()) {
                        return Err(serde::de::Error::custom("duplicate JSON field"));
                    }
                    entries.push((key, value));
                }
                Ok(UniqueObject(entries))
            }
        }
        deserializer.deserialize_map(ObjectVisitor)
    }
}
/// Validate every nested object iteratively, preserving duplicate-key refusals
/// before Go case/null projection. No new depth or reply-volume cap is introduced;
/// caller supplies the saved transport bound. Codec causes remain typed/private.
fn unique_json(raw: &[u8]) -> Result<(), E> {
    std::str::from_utf8(raw).map_err(|e| E::Format(crate::WorkError::Utf8(e)))?;
    let initial: Box<RawValue> =
        swarm_wire_json::decode(raw).map_err(|e| E::Format(crate::WorkError::Json(e)))?;
    let mut pending = vec![initial];
    while let Some(value) = pending.pop() {
        match value.get().trim_start().as_bytes().first() {
            Some(b'{') => {
                let object: UniqueObject = swarm_wire_json::decode(value.get().as_bytes())
                    .map_err(|e| E::Format(crate::WorkError::Json(e)))?;
                if object
                    .0
                    .iter()
                    .any(|(key, _)| key.eq_ignore_ascii_case("verdict"))
                {
                    let criterion: ReferencedCriterion = crate::work::decode_mode(
                        value.get().as_bytes(),
                        referenced_criterion_shape(),
                        true,
                    )
                    .map_err(E::Format)?;
                    let text = object.0.iter().any(|(key, _)| key == "evidence");
                    let reference = object.0.iter().any(|(key, _)| key == "evidence_ref");
                    // Go's v2 presence check is exact-key and distinguishes an
                    // explicitly null field from an absent exclusive alternative.
                    if criterion.verdict == "pass" && (text || !reference)
                        || criterion.verdict != "pass" && (!text || reference)
                    {
                        return Err(E::Refused("managed-review-evidence-form-invalid"));
                    }
                }
                pending.extend(object.0.into_iter().map(|(_, v)| v));
            }
            Some(b'[') => {
                let values: Vec<Box<RawValue>> = swarm_wire_json::decode(value.get().as_bytes())
                    .map_err(|e| E::Format(crate::WorkError::Json(e)))?;
                pending.extend(values);
            }
            _ => {}
        }
    }
    Ok(())
}

/// Preserve actual original kinds and per-task report/control boundaries. A task
/// contract or engine receipt is not silently relabeled implementation evidence.
/// Complete diff sections and declared sources remain available across criteria.
/// Enumerate every quote original used by the task parser. Protocol assembly
/// shares this producer so catalogue references and actual task membership use
/// identical evidence semantics. Reads supplied data only; typed encoding errors
/// survive and no actor, file, freshness or acceptance is authenticated.
pub(crate) fn originals(
    context: &ManagedReviewContext,
    task: &ManagedReviewTaskContext,
) -> Result<Vec<CitationSource>, E> {
    let controls = crate::encode_model(&task.controls).map_err(E::Format)?;
    let controls =
        String::from_utf8(controls).map_err(|_| E::Refused("managed-review-controls-encoding"))?;
    let mut values = vec![
        CitationSource {
            name: if task.binding.report.is_empty() {
                "report.md".into()
            } else {
                task.binding.report.clone()
            },
            kind: "source".into(),
            content: task.report.clone(),
        },
        CitationSource {
            name: "controls.json".into(),
            kind: "context".into(),
            content: controls,
        },
    ];
    for control in task
        .controls
        .iter()
        .flatten()
        .filter(|c| !c.review_output.is_empty())
    {
        values.push(CitationSource {
            name: format!("controls/{}/output.txt", control.id),
            kind: "control_output".into(),
            content: control.review_output.clone(),
        });
    }
    if !context.diff.is_empty() {
        for (index, section) in context.diff.split("\ndiff --git ").enumerate() {
            let content = if index == 0 {
                section.into()
            } else {
                format!("diff --git {section}")
            };
            values.push(CitationSource {
                name: content.split('\n').next().unwrap_or_default().into(),
                kind: "diff".into(),
                content,
            });
        }
    }
    for source in context.sources.iter().flatten() {
        values.push(CitationSource {
            name: source.path.clone(),
            kind: "source".into(),
            content: source.content.clone(),
        });
    }
    Ok(values)
}

/// Parse a complete supplied opinion for the exact candidate and every task.
///
/// The caller supplies an already checked context, saved reply/transport limits
/// and, for citation v2, the actual complete catalogue generated for that context.
/// V2 rejects duplicate JSON names and resolves references only after catalogue,
/// candidate, context digest and reconstructed originals agree. Pass still needs
/// a literal/permitted quote in that task's actual original evidence. A contract
/// declaration alone cannot prove implementation. V0/1 preserve literal Go wire
/// semantics; unsupported versions refuse, not fall back to a weaker protocol.
///
/// Returns all original task bindings plus parsed criteria in response order;
/// fail/unknown is ChangesRequested. Unknown/missing/duplicate tasks, bad indices,
/// stale bindings, invalid field forms, quotes or limits refuse without partial
/// results. Causes are retained safely. No IO, model, acceptance, independent
/// identity, current Root authority or publication is established by this parser.
pub(crate) fn parse_managed_review_reply(
    raw: &[u8],
    context: &ManagedReviewContext,
    catalog: Option<&CitationCatalog>,
    limits: ReviewReplyLimits,
    transport_max_bytes: Option<usize>,
) -> Result<ParsedManagedReview, E> {
    if limits
        .max_text_bytes
        .is_some_and(|max| max < limits.min_text_bytes)
    {
        return Err(E::Refused("review-reply-limits-inconsistent"));
    }
    if transport_max_bytes.is_some_and(|cap| raw.len() > cap) {
        return Err(E::Refused("managed-review-reply-outside-policy"));
    }
    let reply = match context.citation_contract_version {
        0 | 1 => crate::work::decode_mode::<LiteralReply>(raw, literal_shape(), true)
            .map_err(E::Format)?,
        2 => {
            unique_json(raw)?;
            let response: ReferencedReply =
                crate::work::decode_mode(raw, referenced_shape(), true).map_err(E::Format)?;
            let catalog = catalog.ok_or(E::Refused("managed-review-catalog-required"))?;
            let context_digest = crate::provider_configuration::configuration_digest(
                &crate::encode_model(context).map_err(E::Format)?,
            );
            let resolver = catalog.resolver(
                &context.candidate_commit,
                &context_digest,
                &response.catalog_sha256,
            )?;
            let mut tasks = Vec::new();
            for task in response.tasks.into_iter().flatten() {
                let mut criteria = Vec::new();
                for criterion in task.criteria.into_iter().flatten() {
                    let evidence = match criterion.verdict.as_str() {
                        "pass"
                            if criterion.evidence.is_none() && criterion.evidence_ref.is_some() =>
                        {
                            resolver.quote(criterion.evidence_ref.unwrap(), limits)?
                        }
                        "fail" | "unknown"
                            if criterion.evidence_ref.is_none() && criterion.evidence.is_some() =>
                        {
                            criterion.evidence.unwrap()
                        }
                        _ => return Err(E::Refused("managed-review-evidence-form-invalid")),
                    };
                    criteria.push(ReviewCriterion {
                        index: criterion.index,
                        verdict: criterion.verdict,
                        evidence,
                    });
                }
                tasks.push(TaskReply {
                    task: task.task,
                    reason: task.reason,
                    criteria: Some(criteria),
                });
            }
            LiteralReply {
                candidate_commit: response.candidate_commit,
                tasks: Some(tasks),
            }
        }
        _ => return Err(E::Refused("managed-review-citation-version-unsupported")),
    };
    let expected = context.tasks.as_deref().unwrap_or_default();
    if reply.candidate_commit != context.candidate_commit
        || reply.tasks.as_ref().map_or(0, Vec::len) != expected.len()
    {
        return Err(E::Refused("managed-review-candidate-or-cohort-incomplete"));
    }
    let mut seen = BTreeSet::new();
    let mut tasks = Vec::new();
    let mut state = ReviewReplyState::Passed;
    for task in reply.tasks.into_iter().flatten() {
        let source = expected
            .iter()
            .find(|t| t.task == task.task)
            .ok_or(E::Refused("managed-review-task-unknown"))?;
        if !seen.insert(task.task.clone()) {
            return Err(E::Refused("managed-review-task-duplicate"));
        }
        let sources = originals(context, source)?;
        let quoted: Vec<_> = sources
            .iter()
            .map(|s| ReviewQuoteOriginal {
                name: &s.name,
                kind: &s.kind,
                content: &s.content,
            })
            .collect();
        let body = crate::encode_model(
            &serde_json::json!({"reason":task.reason,"criteria":task.criteria}),
        )
        .map_err(E::Format)?;
        let parsed = crate::parse_review_reply(
            &body,
            source.criteria.as_ref().map_or(0, Vec::len),
            &quoted,
            limits,
        )?;
        if parsed.state != ReviewReplyState::Passed {
            state = ReviewReplyState::ChangesRequested;
        }
        let mut binding = source.binding.clone();
        binding.reason = parsed.reason;
        binding.criteria = parsed.criteria;
        tasks.push(binding);
    }
    Ok(ParsedManagedReview { state, tasks })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managed_review_citations::build_catalog;

    /// Explicit data-only fixture limits, not installation/operator defaults.
    fn limits() -> ReviewReplyLimits {
        ReviewReplyLimits {
            min_text_bytes: 8,
            max_text_bytes: Some(1000),
            min_normalized_quote_bytes: 8,
        }
    }
    /// Full two-task cohort data; it grants no Root/reviewer/worker authority.
    fn context(version: i64) -> ManagedReviewContext {
        ManagedReviewContext {
            citation_contract_version: version,
            candidate_commit: "a".repeat(40),
            tasks: Some(
                ["current", "accepted"]
                    .into_iter()
                    .map(|id| ManagedReviewTaskContext {
                        task: id.into(),
                        criteria: Some(vec!["required behavior".into()]),
                        report: format!("{id}: observed implementation evidence is complete"),
                        binding: ManagedTaskReview {
                            task: id.into(),
                            attempt: format!("attempt-{id}"),
                            report: format!("docs/{id}.md"),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .collect(),
            ),
            ..Default::default()
        }
    }
    /// Supply report originals with the exact context digest for parser fixtures.
    /// The production context/catalogue assembler remains a separate obligation.
    fn catalog(context: &ManagedReviewContext) -> CitationCatalog {
        let sources: Vec<_> = context
            .tasks
            .iter()
            .flatten()
            .map(|t| CitationSource {
                name: t.binding.report.clone(),
                kind: "source".into(),
                content: t.report.clone(),
            })
            .collect();
        let digest = crate::provider_configuration::configuration_digest(
            &crate::encode_model(context).unwrap(),
        );
        build_catalog(&context.candidate_commit, &digest, &sources).unwrap()
    }
    fn reply(
        context: &ManagedReviewContext,
        catalog: Option<&CitationCatalog>,
        verdict: &str,
    ) -> serde_json::Value {
        let tasks:Vec<_>=context.tasks.iter().flatten().map(|task| {
            let evidence=if verdict=="pass"{task.report.clone()}else{"missing independent execution evidence".into()};
            let mut criterion=serde_json::json!({"index":1,"verdict":verdict});
            if verdict=="pass" && catalog.is_some() {
                let index=catalog.unwrap().quotes.iter().position(|q|q.text==evidence).unwrap();
                criterion["evidence_ref"]=serde_json::json!(index);
            }else{criterion["evidence"]=serde_json::json!(evidence);}
            serde_json::json!({"task":task.task,"reason":"Complete candidate evidence examined","criteria":[criterion]})
        }).collect();
        let mut result =
            serde_json::json!({"candidate_commit":context.candidate_commit,"tasks":tasks});
        if let Some(catalog) = catalog {
            result["catalog_sha256"] = serde_json::json!(catalog.digest().unwrap());
        }
        result
    }
    #[test]
    fn managed_reply_literal_control_requires_whole_cohort_and_exact_candidate() {
        let context = context(1);
        let valid = reply(&context, None, "pass");
        let parsed = parse_managed_review_reply(
            &crate::encode_model(&valid).unwrap(),
            &context,
            None,
            limits(),
            None,
        )
        .unwrap();
        assert_eq!(parsed.state, ReviewReplyState::Passed);
        assert_eq!(parsed.tasks.len(), 2);
        assert_eq!(parsed.tasks[1].attempt, "attempt-accepted");
        let mut bad = valid.clone();
        bad["candidate_commit"] = serde_json::json!("b".repeat(40));
        assert!(parse_managed_review_reply(
            &crate::encode_model(&bad).unwrap(),
            &context,
            None,
            limits(),
            None
        )
        .is_err());
        let mut bad = valid.clone();
        bad["tasks"].as_array_mut().unwrap().pop();
        assert!(parse_managed_review_reply(
            &crate::encode_model(&bad).unwrap(),
            &context,
            None,
            limits(),
            None
        )
        .is_err());
        let mut bad = valid;
        bad["tasks"][1] = bad["tasks"][0].clone();
        assert!(parse_managed_review_reply(
            &crate::encode_model(&bad).unwrap(),
            &context,
            None,
            limits(),
            None
        )
        .is_err());
    }
    #[test]
    fn managed_reply_referenced_control_refuses_stale_duplicate_and_null_alternatives() {
        let context = context(2);
        let catalog = catalog(&context);
        let valid = reply(&context, Some(&catalog), "pass");
        let raw = crate::encode_model(&valid).unwrap();
        let parsed =
            parse_managed_review_reply(&raw, &context, Some(&catalog), limits(), None).unwrap();
        assert_eq!(parsed.state, ReviewReplyState::Passed);
        assert_eq!(parsed.tasks.len(), 2);
        for replacement in [
            serde_json::Value::Null,
            serde_json::json!("forbidden redundant text"),
        ] {
            let mut bad = valid.clone();
            bad["tasks"][0]["criteria"][0]["evidence"] = replacement;
            assert!(parse_managed_review_reply(
                &crate::encode_model(&bad).unwrap(),
                &context,
                Some(&catalog),
                limits(),
                None
            )
            .is_err());
        }
        let mut bad = valid.clone();
        bad["catalog_sha256"] = serde_json::json!("f".repeat(64));
        assert!(parse_managed_review_reply(
            &crate::encode_model(&bad).unwrap(),
            &context,
            Some(&catalog),
            limits(),
            None
        )
        .is_err());
        let mut duplicate = String::from_utf8(raw.clone()).unwrap();
        duplicate.insert_str(1, "\"catalog_sha256\":\"duplicate\",");
        assert!(parse_managed_review_reply(
            duplicate.as_bytes(),
            &context,
            Some(&catalog),
            limits(),
            None
        )
        .is_err());
        assert!(parse_managed_review_reply(&raw, &context, None, limits(), None).is_err());
        assert!(parse_managed_review_reply(
            &raw,
            &context,
            Some(&catalog),
            limits(),
            Some(raw.len() - 1)
        )
        .is_err());
    }
    #[test]
    fn managed_reply_unknown_remains_changes_and_contract_alone_cannot_prove_implementation() {
        let context = context(2);
        let catalog = catalog(&context);
        let unknown = reply(&context, Some(&catalog), "unknown");
        let parsed = parse_managed_review_reply(
            &crate::encode_model(&unknown).unwrap(),
            &context,
            Some(&catalog),
            limits(),
            None,
        )
        .unwrap();
        assert_eq!(parsed.state, ReviewReplyState::ChangesRequested);
        let mut positive = reply(&context, Some(&catalog), "pass");
        let digest = crate::provider_configuration::configuration_digest(
            &crate::encode_model(&context).unwrap(),
        );
        let misleading = build_catalog(
            &context.candidate_commit,
            &digest,
            &[CitationSource {
                name: "task-contract.json".into(),
                kind: "context".into(),
                content: "Declared specification has no executed implementation".into(),
            }],
        )
        .unwrap();
        positive["catalog_sha256"] = serde_json::json!(misleading.digest().unwrap());
        for task in positive["tasks"].as_array_mut().unwrap() {
            task["criteria"][0]["evidence_ref"] = serde_json::json!(0);
        }
        assert!(parse_managed_review_reply(
            &crate::encode_model(&positive).unwrap(),
            &context,
            Some(&misleading),
            limits(),
            None
        )
        .is_err());
    }
}
