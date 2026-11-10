//! Adapt required binary control observations to the existing gate3 contract.
//! Percentage endpoints describe control conformity, never semantic quality,
//! an authenticated author, independent review or task acceptance.
use crate::{AgentLaunchError as E, EngineSettingsDocument, Task, ValidationControlResult};
use std::{collections::BTreeMap, path::Path};
/// Build the existing delivery-check vocabulary from exactly the stored controls.
/// All configured controls are already mandatory in validation_evidence; 0/100
/// encode no/full binary conformity. One domain has unit normalization, with no
/// weighting decision between domains. Neutral severity does not weaken the
/// mandatory blocker. Saved quality thresholds/generation remain authoritative.
/// None results is a private NON TESTÉ precheck only, never persisted evidence.
/// Empty/foreign/incoherent results refuse rather than manufacturing a score.
fn document(
    task: &Task,
    artifacts: &BTreeMap<String, String>,
    results: Option<&[ValidationControlResult]>,
) -> Result<Vec<u8>, E> {
    let controls = task
        .validation_policy
        .as_deref()
        .and_then(|p| p.controls.as_deref())
        .filter(|c| !c.is_empty())
        .ok_or(E::Refused("task-validation-controls-required"))?;
    if results.is_some_and(|r| r.len() != controls.len()) {
        return Err(E::Refused("task-validation-results-incomplete"));
    }
    let mut checks = Vec::new();
    let mut observations = Vec::new();
    for (index, c) in controls.iter().enumerate() {
        checks.push(serde_json::json!({"id":c.id,"domain":"controls","mandatory":true,"gate":"delivery","severity":"none","penalty":100,"max_penalty":100}));
        let (status, count) = match results.map(|r| &r[index]) {
            None => ("NON TESTÉ", 0),
            Some(r) => {
                if r.id != c.id
                    || r.command != c.command
                    || r.passed && (!r.executed || r.exit_code != 0)
                    || r.started_at.is_empty()
                    || r.finished_at.is_empty()
                {
                    return Err(E::Refused("task-validation-result-source-inconsistent"));
                }
                if r.passed {
                    ("PASS", 0)
                } else {
                    ("FAIL", 1)
                }
            }
        };
        observations.push(serde_json::json!({"id":c.id,"status":status,"count":count,"evidence":artifacts.keys().collect::<Vec<_>>()}));
    }
    serde_json::to_vec(&serde_json::json!({"method_version":"3","scope_id":task.id,"artifacts":artifacts,"domains":{"controls":1},"checks":checks,"results":observations}))
        .map_err(|e|E::caused("task-validation-gate-encoding",e))
}
/// Validate configuration compatibility before any command. The hypothetical
/// untested evaluation is discarded and never written as a measured gate. This
/// preserves different/manual required checks and existing gate configuration.
/// Human mode validates the control shape only and never replaces manual checks
/// with a binary control gate. Policy edits still require reopen/freshness guards.
pub(super) fn precheck(
    task: &Task,
    artifacts: &BTreeMap<String, String>,
    root: &Path,
    settings: &EngineSettingsDocument,
) -> Result<(), E> {
    let raw = document(task, artifacts, None)?;
    // Human controls are auxiliary observations. They cannot replace manually
    // required checks or create a hypothetical completed quality gate.
    if task
        .validation_policy
        .as_deref()
        .is_some_and(|p| p.mode == "human")
    {
        return Ok(());
    }
    let profile = settings
        .quality_policy_snapshot()
        .map_err(|e| E::caused("task-validation-quality-policy", e))?;
    crate::gate_contract::prepare_gate_document(
        task,
        "deterministic-control-conformity",
        "delivery",
        &raw,
        root,
        &profile,
    )
    .map(|_| ())
    .map_err(|e| E::caused("task-validation-gate-configuration", e))
}
/// Prepare a fresh candidate from original observed results and real artifacts.
/// The supplied saved document must be current under the caller's reservation;
/// the Root publisher owns source/identity/receipt guards, SQL and timestamp.
/// Artifact/required-check/config/profile failures retain their typed cause and
/// leave input Task unchanged. Human mode preserves all manual gate/check data
/// and marks human validation required; it never synthesizes a quality PASS.
/// The returned Task is not accepted or reviewed.
pub(super) fn apply(
    task: &Task,
    artifacts: &BTreeMap<String, String>,
    results: &[ValidationControlResult],
    root: &Path,
    settings: &EngineSettingsDocument,
    at: &str,
) -> Result<Task, E> {
    let raw = document(task, artifacts, Some(results))?;
    if task
        .validation_policy
        .as_deref()
        .is_some_and(|p| p.mode == "human")
    {
        let mut next = task.clone();
        next.evidence_stale_reason = "human-validation-required".into();
        return Ok(next);
    }
    let profile = settings
        .quality_policy_snapshot()
        .map_err(|e| E::caused("task-validation-quality-policy", e))?;
    crate::prepare_gate_record(
        task,
        &crate::GateSubmission {
            name: "deterministic-control-conformity",
            phase: "delivery",
            document: &raw,
            at,
        },
        root,
        &profile,
    )
    .map_err(|e| E::caused("task-validation-gate-publication", e))
}
