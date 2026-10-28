//! Preparation-plan change classification for the plan-revision vertical.
//!
//! Internal mirror of the supplied Go producer `preparationPlanChanges`
//! together with its helpers `productTaskContract`, `productTaskChanged`,
//! and the exact kind-string literals. Kind values, condition order, and
//! authored output order are reproduced exactly; the wire struct and its
//! JSON tags (`id`, `kind`, `title`, serialized in declaration order)
//! match the Go `PreparationPlanChange` struct.
//!
//! Wiring (nothing else changes): declare `mod preparation_plan_changes;`
//! at the crate root next to the plan types referenced as `crate::…`, and
//! optionally re-export `pub(crate) use
//! preparation_plan_changes::PreparationPlanChange;` for callers.
//!
//! Scope: pure in-memory classification. No IO, no SQL, no network, no
//! policy or permission checks, no plan mutation or materialization, and
//! no duplicate-ID validation (the shared plan validator owns that
//! invariant).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{ActionPlan, PlanMission, ProductPlan, UserStory};

/// Exact wire values mirrored character-for-character from the Go source
/// (accents included); comparison and assignment share these constants so
/// the labels can never drift apart.
const KIND_ADDED: &str = "ajoutée";
const KIND_MODIFIED: &str = "modifiée";
const KIND_KEPT: &str = "conservée";
const KIND_REMOVED: &str = "retirée (historique conservé)";
const KIND_RECHECK: &str = "à revérifier (dépendance modifiée)";

/// Wire shape of one preparation-plan task change, mirroring the Go
/// `PreparationPlanChange`: JSON `{"id":…,"kind":…,"title":…}` with fields
/// serialized in that declaration order, as in Go.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparationPlanChange {
    /// Task ID as authored. Not validated here (shared validator's job).
    #[serde(rename = "id")]
    pub id: String,
    /// Classification; one of the `KIND_*` wire values.
    #[serde(rename = "kind")]
    pub kind: String,
    /// Task title from the plan the entry was built from (`next` for
    /// added/modified/kept, `old` for removed).
    #[serde(rename = "title")]
    pub title: String,
}

/// Classifies task-level changes between the previously authored
/// preparation plan (`old`) and the revision (`next`), reproducing the Go
/// `preparationPlanChanges` producer exactly: kind strings, condition
/// order, and authored output order.
///
/// Output order: first every task of `next` in stored order (kinds
/// `KIND_ADDED`, `KIND_MODIFIED`, or `KIND_KEPT`, with kept entries later
/// upgraded in place to `KIND_RECHECK` when they transitively depend — via
/// the OLD plan's edges — on an impacted task), then the tasks of `old`
/// missing from `next` in stored order (`KIND_REMOVED`). The internal
/// `BTreeMap`/`BTreeSet` are used strictly for ID lookups and are never
/// iterated, so the emitted order never depends on them. Identical nonempty plans
/// return one kept entry per task; only zero-task inputs return an empty vector.
///
/// Semantics mirrored from Go:
/// - Any plan-level difference (`objective`, `questions`, `assumptions`,
///   a product-mode difference while both product plans exist, or a
///   story-contract difference for the task's linked stories) marks every
///   otherwise-conserved task of `next` as `KIND_MODIFIED`.
/// - Equality follows Go `reflect.DeepEqual`: `None` and `Some(empty)`
///   are different values for `questions`, `assumptions`, and the option
///   fields of tasks and stories; Rust's derived `Option`/`Vec` equality
///   reproduces this distinction exactly.
/// - The dependency closure scans the OLD plan's tasks and their
///   `depends` edges only; removed tasks participate both as invalidation
///   sources and as dependents. Like the Go loop it has no iteration cap;
///   it terminates because the impacted set only grows within the finite
///   bound of old task IDs.
///
/// Honesty / scope: a pure classification of the two supplied plans. It
/// performs no duplicate-ID validation (the shared plan validator owns
/// that invariant; if duplicates slip through, Go map semantics apply —
/// the last occurrence of an ID wins in the lookup maps — and the output
/// simply carries one entry per authored task occurrence), and it performs
/// no persistence, SQL, network, policy, or permission checks, no plan
/// mutation, and no materialization. Authorization for viewing or acting
/// on the result is the caller's concern.
#[must_use]
pub(crate) fn changes(old: &ActionPlan, next: &ActionPlan) -> Vec<PreparationPlanChange> {
    let old_tasks: &[PlanMission] = old.tasks.as_deref().unwrap_or(&[]);
    let next_tasks: &[PlanMission] = next.tasks.as_deref().unwrap_or(&[]);

    // Go's map[string]PlanMission. Used only for presence/value lookups;
    // like the Go maps, a duplicate ID keeps the last occurrence. Never
    // iterated: output order comes from the authored slices.
    let before: BTreeMap<&str, &PlanMission> =
        old_tasks.iter().map(|m| (m.id.as_str(), m)).collect();
    let after: BTreeMap<&str, &PlanMission> =
        next_tasks.iter().map(|m| (m.id.as_str(), m)).collect();

    // Loop-invariant plan-level differences in the Go condition, hoisted
    // without behavior change.
    let objective_changed = old.objective != next.objective;
    // None vs Some(empty) differs, matching reflect.DeepEqual.
    let questions_changed = old.questions != next.questions;
    let assumptions_changed = old.assumptions != next.assumptions;
    let old_product: Option<&ProductPlan> = old.product.as_deref();
    let next_product: Option<&ProductPlan> = next.product.as_deref();

    let mut out: Vec<PreparationPlanChange> = Vec::new();

    for m in next_tasks {
        let kind = match before.get(m.id.as_str()) {
            None => KIND_ADDED,
            Some(&prior) => {
                if prior != m
                    || product_task_changed(old_product, next_product, &m.id)
                    || objective_changed
                    || questions_changed
                    || assumptions_changed
                {
                    KIND_MODIFIED
                } else {
                    KIND_KEPT
                }
            }
        };
        out.push(PreparationPlanChange {
            id: m.id.clone(),
            kind: kind.to_string(),
            title: m.title.clone(),
        });
    }

    for m in old_tasks {
        if !after.contains_key(m.id.as_str()) {
            out.push(PreparationPlanChange {
                id: m.id.clone(),
                kind: KIND_REMOVED.to_string(),
                title: m.title.clone(),
            });
        }
    }

    // Go: impacted := map[string]bool over the collected output. Owned ID
    // copies mirror Go's copied map keys and keep the set independent of
    // `out`, so the final reclassification can borrow `out` mutably.
    let mut impacted: BTreeSet<String> = BTreeSet::new();
    for change in &out {
        if change.kind != KIND_KEPT {
            impacted.insert(change.id.clone());
        }
    }

    // Fixpoint propagation over the OLD plan's dependency edges, with no
    // iteration cap (matching the Go loop); termination follows from the
    // set growing monotonically within a finite bound.
    let mut again = true;
    while again {
        again = false;
        for task in old_tasks {
            for dep in task.depends.as_deref().unwrap_or(&[]) {
                if impacted.contains(dep.as_str()) && !impacted.contains(task.id.as_str()) {
                    impacted.insert(task.id.clone());
                    again = true;
                }
            }
        }
    }

    for change in &mut out {
        if change.kind == KIND_KEPT && impacted.contains(change.id.as_str()) {
            change.kind = KIND_RECHECK.to_string();
        }
    }

    out
}

/// Mirrors Go `productTaskChanged`.
///
/// A product-mode difference counts only when both plans carry a product
/// plan (`before != nil && after != nil` in the Go source); a plan
/// appearing, disappearing, or changing its linked stories is caught by
/// the story-contract comparison instead. Contracts are compared under Go
/// `reflect.DeepEqual` semantics — a nil contract (no product plan, or no
/// story linked to `task`) differs from any non-nil contract — which the
/// `Option` equality below reproduces exactly. Journeys are deliberately
/// not consulted here.
#[must_use]
fn product_task_changed(
    before: Option<&ProductPlan>,
    after: Option<&ProductPlan>,
    task: &str,
) -> bool {
    if let (Some(before), Some(after)) = (before, after) {
        if before.mode != after.mode {
            return true;
        }
    }
    product_task_contract(before, task) != product_task_contract(after, task)
}

/// Mirrors Go `productTaskContract`.
///
/// A changed story contract changes the work required of its linked tasks;
/// presentation changes to journey labels do not invalidate execution
/// evidence, so `ProductPlan.journeys` is never read here.
///
/// Returns the stories of `product` whose `task_ids` contain `task`, in
/// authored story order. The Go original returns a nil slice both when the
/// plan pointer is nil and when no story matches, so an empty selection
/// yields the same `None` representation regardless of product-plan
/// presence; `Some` is produced only for a non-empty match list.
/// (`reflect.DeepEqual` at the Go call site distinguishes nil from
/// non-nil results; `Option` equality reproduces that.)
#[must_use]
fn product_task_contract(product: Option<&ProductPlan>, task: &str) -> Option<Vec<UserStory>> {
    let product = product?;
    let mut matched: Vec<UserStory> = Vec::new();
    for story in product.stories.as_deref().unwrap_or(&[]) {
        if contains_string(story.task_ids.as_deref().unwrap_or(&[]), task) {
            matched.push(story.clone());
        }
    }
    if matched.is_empty() {
        None
    } else {
        Some(matched)
    }
}

/// Exact, case-sensitive equality scan. Verified against the actual Go helper
/// in agent_exchange.go; performs no trimming or normalization.
#[must_use]
fn contains_string(haystack: &[String], needle: &str) -> bool {
    haystack.iter().any(|item| item.as_str() == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mission(id: &str, title: &str) -> PlanMission {
        PlanMission {
            id: id.to_string(),
            title: title.to_string(),
            ..Default::default()
        }
    }

    fn mission_with_deps(id: &str, title: &str, depends: &[&str]) -> PlanMission {
        PlanMission {
            depends: Some(depends.iter().map(|d| d.to_string()).collect()),
            ..mission(id, title)
        }
    }

    fn mission_with_proof(id: &str, title: &str, proof: &str) -> PlanMission {
        PlanMission {
            proof: proof.to_string(),
            ..mission(id, title)
        }
    }

    fn story(id: &str, task_ids: &[&str]) -> UserStory {
        UserStory {
            id: id.to_string(),
            task_ids: Some(task_ids.iter().map(|t| t.to_string()).collect()),
            ..Default::default()
        }
    }

    fn plan(product: Option<ProductPlan>, tasks: Vec<PlanMission>) -> ActionPlan {
        ActionPlan {
            product: product.map(Box::new),
            objective: "ship the revision".to_string(),
            tasks: Some(tasks),
            ..Default::default()
        }
    }

    fn entry(id: &str, kind: &str, title: &str) -> PreparationPlanChange {
        PreparationPlanChange {
            id: id.to_string(),
            kind: kind.to_string(),
            title: title.to_string(),
        }
    }

    #[test]
    fn journey_only_change_keeps_task_but_story_contract_or_mode_change_marks_it_modified() {
        let old_product = ProductPlan {
            mode: "draft".to_string(),
            journeys: Some(Vec::new()),
            stories: Some(vec![story("s1", &["t1"])]),
        };
        let old = plan(Some(old_product.clone()), vec![mission("t1", "Task one")]);

        // Only `journeys` differs (Some(empty) → None); mode, stories,
        // tasks, objective, questions, and assumptions are identical, so
        // the linked task stays conserved: journey presentation changes do
        // not invalidate execution evidence.
        let next = plan(
            Some(ProductPlan {
                journeys: None,
                ..old_product.clone()
            }),
            vec![mission("t1", "Task one")],
        );
        assert_eq!(changes(&old, &next), vec![entry("t1", KIND_KEPT, "Task one")]);

        // The linked story's contract (acceptance criteria) changed while
        // everything else is identical → modified.
        let next = plan(
            Some(ProductPlan {
                stories: Some(vec![UserStory {
                    criteria: Some(vec!["signed off".to_string()]),
                    ..story("s1", &["t1"])
                }]),
                ..old_product.clone()
            }),
            vec![mission("t1", "Task one")],
        );
        assert_eq!(
            changes(&old, &next),
            vec![entry("t1", KIND_MODIFIED, "Task one")]
        );

        // Mode differs while both product plans exist → modified even
        // though the story selection is unchanged.
        let next = plan(
            Some(ProductPlan {
                mode: "review".to_string(),
                ..old_product
            }),
            vec![mission("t1", "Task one")],
        );
        assert_eq!(
            changes(&old, &next),
            vec![entry("t1", KIND_MODIFIED, "Task one")]
        );
    }

    #[test]
    fn transitive_old_dependents_are_rechecked_and_removals_are_preserved() {
        // Old edges: b→a, c→b (transitively c→b→a), w→d.
        let old = plan(
            None,
            vec![
                mission_with_proof("a", "Alpha", "old-proof"),
                mission_with_deps("b", "Beta", &["a"]),
                mission_with_deps("c", "Gamma", &["b"]),
                mission_with_deps("w", "Watcher", &["d"]),
                mission("d", "Delta"),
            ],
        );
        // Next: a modified, b/c/w conserved, d removed, e added. The
        // closure runs over the OLD plan, so c is flagged transitively
        // through b, and w through the removed d.
        let next = plan(
            None,
            vec![
                mission_with_proof("a", "Alpha", "new-proof"),
                mission_with_deps("b", "Beta", &["a"]),
                mission_with_deps("c", "Gamma", &["b"]),
                mission_with_deps("w", "Watcher", &["d"]),
                mission("e", "Epsilon"),
            ],
        );

        assert_eq!(
            changes(&old, &next),
            vec![
                entry("a", KIND_MODIFIED, "Alpha"),
                entry("b", KIND_RECHECK, "Beta"),
                entry("c", KIND_RECHECK, "Gamma"),
                entry("w", KIND_RECHECK, "Watcher"),
                entry("e", KIND_ADDED, "Epsilon"),
                entry("d", KIND_REMOVED, "Delta"),
            ]
        );
    }

    #[test]
    fn nil_and_empty_questions_are_distinguished() {
        // None vs Some(empty): reflect.DeepEqual treats a nil slice and an
        // empty slice as different, so the otherwise-identical plan marks
        // every conserved task modified.
        let old = plan(None, vec![mission("t1", "Task one")]);
        let mut next = plan(None, vec![mission("t1", "Task one")]);
        next.questions = Some(Vec::new());
        assert_eq!(
            changes(&old, &next),
            vec![entry("t1", KIND_MODIFIED, "Task one")]
        );

        // Some(empty) vs Some(empty) is equal: the task stays conserved.
        let mut old = plan(None, vec![mission("t1", "Task one")]);
        let mut next = plan(None, vec![mission("t1", "Task one")]);
        old.questions = Some(Vec::new());
        next.questions = Some(Vec::new());
        assert_eq!(changes(&old, &next), vec![entry("t1", KIND_KEPT, "Task one")]);
    }
}
