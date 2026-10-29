//! Cockpit post-delivery projection: a direct Rust port of the Go
//! `postDelivery` + `acceptanceSource` pair in
//! `internal/engine/post_delivery.go`.
//!
//! This phase is an offer after a fresh recipe. It grants no execution
//! budget: the projection is read-only, performs no clock, filesystem, SQL
//! or model access, executes nothing, and confers no rights, identity or
//! authority. The freshness facts are authoritative observations already
//! read by the private Root reader; nothing here deserializes, defaults or
//! re-derives them. Native gate, independent-review and automatic-validation
//! records are only borrowed through the [`Task`] for byte-compatible
//! encoding: their fields are not mirrored and no freshness is inferred from
//! them. Neither the inputs nor the result grant any rights.
//!
//! INCOMPLETE: this helper alone is not the full cockpit agent list. The
//! parent owns integration and Go-oracle comparison, and wires this file
//! itself (no `mod.rs` is assumed).

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    encode_model, review_contract, AutomaticValidation, DialogueRef, GateRecord,
    IndependentReview, Task, Work, WorkError,
};

/// Hex SHA-256 of raw Go-compatible JSON bytes (Go `hash`).
fn sha256_hex(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}

/// Body of Go `acceptanceSource`, under the exact capital key order
/// `Work Task Contract Gate Review Automatic`, with no omission and nulls
/// preserved (`None` encodes as JSON `null`).
///
/// `gate`, `review` and `automatic` are the native records borrowed through
/// the task and serialized in their generated Go-compatible field order.
#[derive(Serialize)]
struct AcceptanceSourceBody<'a> {
    #[serde(rename = "Work")]
    work: &'a str,
    #[serde(rename = "Task")]
    task: &'a str,
    #[serde(rename = "Contract")]
    contract: &'a str,
    #[serde(rename = "Gate")]
    gate: Option<&'a GateRecord>,
    #[serde(rename = "Review")]
    review: Option<&'a IndependentReview>,
    #[serde(rename = "Automatic")]
    automatic: Option<&'a AutomaticValidation>,
}

/// Go `PostDelivery`: the post-delivery offer.
///
/// `sources` keeps the stored Work task order and is never null; an empty
/// projection serializes as `"sources":[]`.
#[derive(Debug, Serialize)]
pub(crate) struct PostDelivery {
    pub(crate) ready: bool,
    pub(crate) sources: Vec<DialogueRef>,
}

/// Narrow private error set; the parent adapts it later.
pub(crate) enum PostDeliveryError {
    /// An accepted, non-RETEX task id has no freshness fact from the Root
    /// reader. Explicit by design: a missing fact is never read as `false`.
    MissingFreshness {
        /// Id of the accepted task whose fact is absent.
        task_id: String,
    },
    /// Go-compatible JSON encoding of an acceptance source body failed.
    Encode(WorkError),
}

impl std::fmt::Debug for PostDeliveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
impl std::fmt::Display for PostDeliveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::MissingFreshness { .. } => "cockpit-post-delivery-freshness-missing",
            Self::Encode(_) => "cockpit-post-delivery-encoding",
        })
    }
}
impl std::error::Error for PostDeliveryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self { Self::Encode(cause) => Some(cause), _ => None }
    }
}

/// Go's stop value: `ready == false`, empty (never null) sources.
fn not_ready() -> PostDelivery {
    PostDelivery {
        ready: false,
        sources: Vec::new(),
    }
}

/// Build the Go `acceptanceSource` dialogue reference for one task.
///
/// Binds Work id, task id, the [`review_contract`] digest and the task's
/// native `gate`, `independent_review` and `automatic_validation` values
/// under the exact capital keys `Work Task Contract Gate Review Automatic`,
/// encoded with `encode_model` (Go field order, HTML escaping, nulls
/// preserved) and digested with SHA-256. The attempt is the latest
/// `task.attempts` id, empty when there are no attempts.
///
/// The digest proves binding to the encoded bytes only: no execution,
/// acceptance truth, identity or freshness.
///
/// # Errors
///
/// [`PostDeliveryError::Encode`] if [`review_contract`] or `encode_model`
/// fail.
pub(crate) fn acceptance_source(
    work: &Work,
    task: &Task,
) -> Result<DialogueRef, PostDeliveryError> {
    let contract = review_contract(task).map_err(PostDeliveryError::Encode)?;
    let body = AcceptanceSourceBody {
        work: &work.id,
        task: &task.id,
        contract: &contract,
        gate: task.gate.as_deref(),
        review: task.independent_review.as_deref(),
        automatic: task.automatic_validation.as_deref(),
    };
    let raw = encode_model(&body).map_err(PostDeliveryError::Encode)?;
    let attempt = task
        .attempts
        .as_deref()
        .and_then(|attempts| attempts.last())
        .map(|attempt| attempt.id.clone())
        .unwrap_or_default();
    Ok(DialogueRef {
        task: task.id.clone(),
        attempt,
        sha256: sha256_hex(&raw),
    })
}

/// Project the Go `postDelivery` offer for a whole [`Work`].
///
/// # Preconditions
///
/// * `accepted_fresh` holds the actual freshness facts read by the private
///   Root reader for every task that is both `"accepted"` and not RETEX
///   excluded; Root supplies all of them. The map is authoritative input: it
///   is neither deserialized nor defaulted here, and no clock, filesystem,
///   SQL or model access happens in this function.
/// * `work.tasks` is the stored Work order; it is preserved in `sources`.
///
/// # Behavior (Go oracle)
///
/// * A retained RETEX task belongs to the next increment, not the recipe
///   source: ids named by a non-empty `work.retex[].task` are excluded.
/// * Tasks are visited in stored order. The first task whose status is not
///   `"accepted"`, or whose supplied freshness fact is `false`, stops the
///   projection with `ready == false` and empty sources; earlier sources are
///   discarded (Go's `||` short-circuit order is preserved, so a
///   non-accepted task never triggers a freshness lookup).
/// * A missing fact for an accepted, non-excluded task is an explicit
///   [`PostDeliveryError::MissingFreshness`], never a silent `false`.
/// * With at least one source and `work.planning` present, the scope with
///   `id == "root"` must exist and have `state == "closed"`; otherwise the
///   sources are discarded (`ready == false`, empty sources). Planning
///   absent, or sources empty, skips this check.
/// * Empty or absent tasks yield `ready == false` and `[]`, never null.
///
/// # Authority
///
/// Read-only offer projection: it executes nothing, grants no execution
/// budget, introduces no limits, defaults or retries, and neither the Work,
/// the facts nor the result confer rights. Native review/gate records are
/// borrowed for encoding only; freshness is never inferred from them.
///
/// # Errors
///
/// * [`PostDeliveryError::MissingFreshness`] — accepted, non-RETEX task id
///   absent from `accepted_fresh`.
/// * [`PostDeliveryError::Encode`] — Go-compatible JSON encoding failed.
pub(crate) fn cockpit_post_delivery(
    work: &Work,
    accepted_fresh: &BTreeMap<String, bool>,
) -> Result<PostDelivery, PostDeliveryError> {
    // A retained RETEX task belongs to the next increment, not the recipe source.
    let excluded: BTreeSet<&str> = work
        .retex
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|retex| retex.task.as_str())
        .filter(|task_id| !task_id.is_empty())
        .collect();

    let mut sources = Vec::new();
    for task in work.tasks.as_deref().unwrap_or(&[]) {
        if excluded.contains(task.id.as_str()) {
            continue;
        }
        if task.status != "accepted" {
            return Ok(not_ready());
        }
        let fresh = accepted_fresh
            .get(&task.id)
            .copied()
            .ok_or_else(|| PostDeliveryError::MissingFreshness {
                task_id: task.id.clone(),
            })?;
        if !fresh {
            return Ok(not_ready());
        }
        sources.push(acceptance_source(work, task)?);
    }

    if sources.is_empty() {
        return Ok(not_ready());
    }
    if let Some(planning) = work.planning.as_deref() {
        let root_closed = planning
            .scopes
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .find(|scope| scope.id == "root")
            .is_some_and(|root| root.state == "closed");
        if !root_closed {
            return Ok(not_ready());
        }
    }
    Ok(PostDelivery {
        ready: true,
        sources,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PlanningScope, PlanningState, Retex, Attempt};

    fn accepted_task(id: &str) -> Task {
        Task {
            id: id.to_string(),
            status: "accepted".to_string(),
            ..Task::default()
        }
    }

    fn work_with(id: &str, tasks: Vec<Task>) -> Work {
        Work {
            id: id.to_string(),
            tasks: Some(tasks),
            ..Work::default()
        }
    }

    fn facts(values: &[(&str, bool)]) -> BTreeMap<String, bool> {
        values
            .iter()
            .map(|(task_id, fresh)| ((*task_id).to_string(), *fresh))
            .collect()
    }

    fn planning_with_root(state: &str) -> Option<Box<PlanningState>> {
        Some(Box::new(PlanningState {
            scopes: Some(vec![PlanningScope {
                id: "root".to_string(),
                state: state.to_string(),
                ..PlanningScope::default()
            }]),
            ..PlanningState::default()
        }))
    }

    #[test]
    fn non_accepted_task_stops_not_ready_without_needing_a_fact() {
        let task = Task {
            id: "t1".to_string(),
            status: "delivered".to_string(),
            ..Task::default()
        };
        let work = work_with("w1", vec![task]);
        // Empty fact map: the status check must short-circuit before the
        // freshness lookup, exactly like Go's `||`.
        let out = cockpit_post_delivery(&work, &BTreeMap::new()).unwrap();
        assert!(!out.ready);
        assert!(out.sources.is_empty());
    }

    #[test]
    fn stale_accepted_task_is_not_ready_with_empty_sources() {
        let work = work_with("w1", vec![accepted_task("t1")]);
        let out = cockpit_post_delivery(&work, &facts(&[("t1", false)])).unwrap();
        assert!(!out.ready);
        assert!(out.sources.is_empty());
    }

    #[test]
    fn first_failing_task_discards_earlier_sources() {
        let work = work_with("w1", vec![accepted_task("t1"), accepted_task("t2")]);
        let out = cockpit_post_delivery(&work, &facts(&[("t1", true), ("t2", false)])).unwrap();
        assert!(!out.ready);
        assert!(out.sources.is_empty());
    }

    #[test]
    fn excluded_retex_task_needs_no_fact_and_keeps_remaining_order() {
        let mut work = work_with(
            "w1",
            vec![accepted_task("t1"), accepted_task("t2"), accepted_task("t3")],
        );
        work.retex = Some(vec![Retex {
            task: "t1".to_string(),
            ..Retex::default()
        }]);
        // No fact for the excluded t1: exclusion precedes the freshness read.
        let out = cockpit_post_delivery(&work, &facts(&[("t2", true), ("t3", true)])).unwrap();
        assert!(out.ready);
        let ids: Vec<&str> = out.sources.iter().map(|source| source.task.as_str()).collect();
        assert_eq!(ids, ["t2", "t3"]);
    }

    #[test]
    fn retex_excluded_only_task_is_not_ready_without_error() {
        let mut work = work_with("w1", vec![accepted_task("t1")]);
        work.retex = Some(vec![Retex {
            task: "t1".to_string(),
            ..Retex::default()
        }]);
        let out = cockpit_post_delivery(&work, &BTreeMap::new()).unwrap();
        assert!(!out.ready);
        assert!(out.sources.is_empty());
    }

    #[test]
    fn missing_fact_for_accepted_task_is_explicit_error() {
        let work = work_with("w1", vec![accepted_task("t1"), accepted_task("t2")]);
        let err = cockpit_post_delivery(&work, &facts(&[("t1", true)])).unwrap_err();
        match err {
            PostDeliveryError::MissingFreshness { task_id } => assert_eq!(task_id, "t2"),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn open_root_scope_discards_sources_and_is_not_ready() {
        let mut work = work_with("w1", vec![accepted_task("t1")]);
        work.planning = planning_with_root("open");
        let out = cockpit_post_delivery(&work, &facts(&[("t1", true)])).unwrap();
        assert!(!out.ready);
        // Go discards the computed sources when the root scope is not closed.
        assert!(out.sources.is_empty());
    }

    #[test]
    fn missing_root_scope_and_absent_scopes_are_not_ready() {
        let mut work = work_with("w1", vec![accepted_task("t1")]);
        work.planning = Some(Box::new(PlanningState {
            scopes: Some(vec![PlanningScope {
                id: "aux".to_string(),
                state: "closed".to_string(),
                ..PlanningScope::default()
            }]),
            ..PlanningState::default()
        }));
        let out = cockpit_post_delivery(&work, &facts(&[("t1", true)])).unwrap();
        assert!(!out.ready);
        assert!(out.sources.is_empty());

        let mut work = work_with("w1", vec![accepted_task("t1")]);
        work.planning = Some(Box::new(PlanningState {
            scopes: None,
            ..PlanningState::default()
        }));
        let out = cockpit_post_delivery(&work, &facts(&[("t1", true)])).unwrap();
        assert!(!out.ready);
        assert!(out.sources.is_empty());
    }

    #[test]
    fn closed_root_scope_is_ready() {
        let mut work = work_with("w1", vec![accepted_task("t1")]);
        work.planning = planning_with_root("closed");
        let out = cockpit_post_delivery(&work, &facts(&[("t1", true)])).unwrap();
        assert!(out.ready);
        assert_eq!(out.sources.len(), 1);
    }

    #[test]
    fn fresh_ready_preserves_task_order_and_latest_attempt() {
        let with_attempts = Task {
            id: "t2".to_string(),
            status: "accepted".to_string(),
            attempts: Some(vec![
                Attempt {
                    id: "a1".to_string(),
                    ..Attempt::default()
                },
                Attempt {
                    id: "a2".to_string(),
                    ..Attempt::default()
                },
            ]),
            ..Task::default()
        };
        // Stored Work order (t2 before t1) must win.
        let work = work_with("w1", vec![with_attempts, accepted_task("t1")]);
        let out = cockpit_post_delivery(&work, &facts(&[("t1", true), ("t2", true)])).unwrap();
        assert!(out.ready);
        assert_eq!(out.sources.len(), 2);
        assert_eq!(out.sources[0].task, "t2");
        assert_eq!(out.sources[0].attempt, "a2");
        assert_eq!(out.sources[1].task, "t1");
        assert_eq!(out.sources[1].attempt, "");
    }

    #[test]
    fn empty_or_absent_tasks_are_false_empty_even_with_open_planning() {
        let mut work = work_with("w1", vec![]);
        work.planning = planning_with_root("open");
        let out = cockpit_post_delivery(&work, &BTreeMap::new()).unwrap();
        assert!(!out.ready);
        assert!(out.sources.is_empty());

        let work = Work {
            id: "w1".to_string(),
            tasks: None,
            planning: planning_with_root("open"),
            ..Work::default()
        };
        let out = cockpit_post_delivery(&work, &BTreeMap::new()).unwrap();
        assert!(!out.ready);
        assert!(out.sources.is_empty());
    }

    #[test]
    fn empty_projection_serializes_false_and_empty_array_never_null() {
        let work = work_with("w1", vec![]);
        let out = cockpit_post_delivery(&work, &BTreeMap::new()).unwrap();
        let raw = encode_model(&out).unwrap();
        assert_eq!(raw.as_slice(), br#"{"ready":false,"sources":[]}"#.as_slice());
    }

    #[test]
    fn acceptance_body_serializes_go_capital_key_order_with_nulls() {
        let body = AcceptanceSourceBody {
            work: "w1",
            task: "t1",
            contract: "abc",
            gate: None,
            review: None,
            automatic: None,
        };
        let raw = encode_model(&body).unwrap();
        let expected =
            br#"{"Work":"w1","Task":"t1","Contract":"abc","Gate":null,"Review":null,"Automatic":null}"#;
        assert_eq!(raw.as_slice(), expected.as_slice());
    }

    #[test]
    fn source_digest_matches_explicit_go_json_hashed_with_sha2() {
        let task = accepted_task("t1");
        let contract = review_contract(&task).unwrap();
        // Expected body stated explicitly; the digest is computed here with
        // sha2, so no cryptographic constant is hardcoded.
        let expected_body = format!(
            r#"{{"Work":"w1","Task":"t1","Contract":"{contract}","Gate":null,"Review":null,"Automatic":null}}"#
        );
        let expected_digest = sha256_hex(expected_body.as_bytes());

        let work = work_with("w1", vec![task]);
        let out = cockpit_post_delivery(&work, &facts(&[("t1", true)])).unwrap();
        assert!(out.ready);
        assert_eq!(out.sources[0].task, "t1");
        assert_eq!(out.sources[0].attempt, "");
        assert_eq!(out.sources[0].sha256, expected_digest);
    }

    #[test]
    fn source_digest_escapes_html_like_go_json() {
        let task = Task {
            id: "t&1<x>".to_string(),
            status: "accepted".to_string(),
            ..Task::default()
        };
        let contract = review_contract(&task).unwrap();
        // Go escapes `<`, `>` and `&` in strings; the expected body states
        // those escapes literally and is hashed in-test with sha2.
        let expected_body = format!(
            r#"{{"Work":"w\u003cy\u003e\u0026","Task":"t\u00261\u003cx\u003e","Contract":"{contract}","Gate":null,"Review":null,"Automatic":null}}"#
        );
        let expected_digest = sha256_hex(expected_body.as_bytes());

        let work = work_with("w<y>&", vec![task]);
        let out = cockpit_post_delivery(&work, &facts(&[("t&1<x>", true)])).unwrap();
        assert!(out.ready);
        assert_eq!(out.sources[0].sha256, expected_digest);
    }
    /// Actual retained Go producer outputs, not expected hashes recomputed by
    /// the Rust implementation. Raw task JSON preserves embedded gate ordering.
    /// These fixture values grant no freshness/identity, provider or acceptance.
    #[test]
    fn retained_go_oracle_matches_sources_and_cost_text() {
        use serde::Deserialize;
        #[derive(Deserialize)]
        struct Cost { reported_usd:f64, attempts_with_cost:i64, attempts_without_cost:i64 }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Case {
            name:String, kind:String, work_id:Option<String>,
            task_json:Option<String>, expected:Option<DialogueRef>,
            cost:Option<Cost>, expected_text:Option<String>,
        }
        #[derive(Deserialize)]
        struct Oracle { schema_version:i64, cases:Vec<Case> }
        let fixture=std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../tools/rust-migration/fixtures/cockpit-post-delivery-go.json");
        let oracle:Oracle=serde_json::from_str(&std::fs::read_to_string(fixture).unwrap()).unwrap();
        assert_eq!(oracle.schema_version,2);
        let required=["null-records","empty-criteria","empty-dependencies","html-and-separators",
            "latest-attempt","gate-present","review-present","automatic-present","combined-records",
            "raw-number-decimal","raw-number-exponent","cost-empty","cost-unreported",
            "cost-zero-reported","cost-rounding-down","cost-partial","cost-rounding-even",
            "cost-large","cost-negative-zero"];
        let names:std::collections::BTreeSet<_>=oracle.cases.iter().map(|c|c.name.as_str()).collect();
        assert_eq!(names.len(),oracle.cases.len(),"duplicate corpus case");
        assert!(required.iter().all(|name|names.contains(name)),"required oracle coverage missing");
        let (mut sources,mut costs)=(0,0);
        for case in oracle.cases {
            match case.kind.as_str() {
                "source" => {
                    let raw=case.task_json.expect("source input missing");
                    let task:Task=serde_json::from_str(&raw).unwrap();
                    let work=Work{id:case.work_id.expect("source work missing"),..Default::default()};
                    assert_eq!(acceptance_source(&work,&task).unwrap(),case.expected.expect("Go reference missing"),"{}",case.name);
                    println!("PASS GO_COCKPIT_SOURCE {}",case.name);sources+=1;
                }
                "cost" => {
                    let cost=case.cost.expect("cost input missing");
                    let total=super::super::reported_costs::CostTotal {
                        reported_usd:crate::GoFloat(cost.reported_usd),
                        attempts_with_cost:cost.attempts_with_cost,
                        attempts_without_cost:cost.attempts_without_cost,
                    };
                    assert_eq!(total.text(),case.expected_text.expect("Go cost text missing"),"{}",case.name);
                    println!("PASS GO_COCKPIT_COST {}",case.name);costs+=1;
                }
                _ => panic!("unknown retained oracle kind"),
            }
        }
        assert!(sources>=11 && costs>=8);
    }

}
