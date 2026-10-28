//! Read-only claim-input readiness. A verified snapshot is not an exec permit.
#![cfg(target_os = "linux")]
use crate::{AgentLaunchError as E, WorkEnvironment};
use rusqlite::Transaction;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
/// Required native data fields. Root access and matching holder declarations
/// are not authenticated agent identity; no supplied clock/actor is accepted.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlanningCallPreviewRequest {
    pub schema_version: i64,
    pub expected_revision: i64,
    pub claim_id: String,
    pub scope: String,
    pub scope_revision: i64,
    pub holder: String,
    pub generation: i64,
}
/// Observed inputs only, with native foreground entry availability. This is no
/// permission or guarantee of execution readiness: run rechecks current policy,
/// host and actual identity before its private gate. Prompt
/// and context plaintext are not serialized to CLI output. All facts expire as
/// soon as inputs/policy/lease change; actual execution still needs a fresh gate.
#[derive(Serialize)]
pub struct PlanningCallPreview {
    pub schema_version: i64,
    pub work_id: String,
    pub scope_id: String,
    pub claim_id: String,
    pub work_revision: i64,
    pub original_claim_revision: i64,
    pub generation: i64,
    pub context_sha256: String,
    pub workflow_sha256: String,
    pub workflow_prompt_sha256: String,
    pub context_bytes: usize,
    pub workflow_bytes: usize,
    pub provider: String,
    pub model_route: Option<Box<crate::ModelRoute>>,
    pub historical_acknowledgement_only: bool,
    pub observed_at: String,
    pub lease_until: String,
    pub prepared_inputs_verified: bool,
    pub process_started: bool,
    pub execution_available: bool,
}
/// Decode only the required v1 native fields. Unknown/duplicate/null/invalid
/// types refuse with original typed cause; no request data is displayed or used
/// as authority. Transport volume belongs to the checked CLI/API reader.
pub fn decode(raw: &[u8]) -> Result<PlanningCallPreviewRequest, E> {
    let r: PlanningCallPreviewRequest =
        serde_json::from_slice(raw).map_err(|e| E::caused("planning-call-request-format", e))?;
    if r.schema_version != 1
        || r.expected_revision < 1
        || r.scope_revision < 1
        || r.generation < 1
        || [&r.claim_id, &r.scope, &r.holder]
            .iter()
            .any(|v| !swarm_identity_contract::name(v.as_bytes()))
    {
        return Err(E::Refused("planning-call-request-invalid"));
    }
    Ok(r)
}
/// Complete normalized claim row and immutable evidence documents. Named
/// columns prevent positional contract swaps; none is a default/authority grant.
struct Binding {
    owner: String,
    scope_id: String,
    generation: i64,
    scope_revision: i64,
    claim_revision: i64,
    holder: String,
    context: String,
    context_sha: String,
    workflow_json: String,
    workflow_prompt: String,
    workflow_prompt_sha: String,
    model_json: String,
    recorded_at: String,
}
/// Read bound evidence after exact auxiliary schema validation. Missing or
/// ambiguous records and type/SQL/close errors refuse without mutation.
fn binding(tx: &rusqlite::Connection, claim: &str) -> Result<Binding, E> {
    if !crate::planning_call_inputs::schema(tx, false)? {
        return Err(E::Refused("planning-call-binding-required"));
    }
    crate::agent_launch::query(tx,"SELECT work_id,scope_id,generation,scope_revision,claim_revision,holder,context_json,context_sha256,workflow_json,workflow_prompt,workflow_prompt_sha256,model_json,recorded_at FROM native_planning_claims WHERE claim_id=?",[claim],|rows|{
        let r=rows.next()?.ok_or(E::Refused("planning-call-binding-required"))?;
        let value=Binding{owner:r.get(0)?,scope_id:r.get(1)?,generation:r.get(2)?,scope_revision:r.get(3)?,claim_revision:r.get(4)?,holder:r.get(5)?,context:r.get(6)?,context_sha:r.get(7)?,workflow_json:r.get(8)?,workflow_prompt:r.get(9)?,workflow_prompt_sha:r.get(10)?,model_json:r.get(11)?,recorded_at:r.get(12)?};
        if rows.next()?.is_some(){return Err(E::Refused("planning-call-binding-ambiguous"));}Ok(value)
    })
}
/// Private complete verified inputs retained for an actual owner/gate. These
/// values are observations, not a permission or a claim that a provider ran.
/// Original lease duration comes from the recorded claim, never a current default.
pub(crate) struct VerifiedInputs {
    pub preview: PlanningCallPreview,
    pub context: String,
    pub workflow_prompt: String,
    pub model: crate::RoleModel,
    pub lease_seconds: i64,
}
/// Read the same live verified input bundle and return only public metadata.
/// Complete prompt/model bytes stay private and are dropped for this pure view.
/// All freshness/funding/format errors propagate; no provider or SQL mutation.
pub(crate) fn inspect(
    tx: &Transaction<'_>,
    root: &std::path::Path,
    settings: &crate::EngineSettingsDocument,
    work_id: &str,
    r: &PlanningCallPreviewRequest,
) -> Result<PlanningCallPreview, E> {
    verified(tx, root, settings, work_id, r).map(|inputs| inputs.preview)
}
/// Verify live fences, original event/binding, complete hashes/workflow/source,
/// pending input identity and report bytes, plus current reserved funding. All
/// reads require an active caller transaction/settings reservation. A final clock check
/// rejects expiry during preparation; no global deadline, process, SQL mutation,
/// invoice or authenticated identity is inferred. Current unrelated Work
/// revisions are allowed when these owned inputs/fences remain intact.
pub(crate) fn verified(
    tx: &rusqlite::Connection,
    root: &std::path::Path,
    settings: &crate::EngineSettingsDocument,
    work_id: &str,
    r: &PlanningCallPreviewRequest,
) -> Result<VerifiedInputs, E> {
    verified_with_funding(tx, root, settings, work_id, r, FundingBoundary::Prospective)
}
/// Reobserve a known original result under the same complete lease/source/action
/// fences as live preview, using its authoritative historical reservation rather
/// than repricing a new start. Caller must verify actual original receipt/wait,
/// process permission, current recorded row and owner boundary separately. The
/// returned input metadata describes this reobservation, not a new launch permit
/// or proof that the original never ran. All nonfinancial checks remain common;
/// SQL/IO/fence errors retain cause. No grant, mutation, refund or settlement.
pub(crate) fn verified_result(
    tx: &rusqlite::Connection,
    root: &std::path::Path,
    settings: &crate::EngineSettingsDocument,
    work_id: &str,
    r: &PlanningCallPreviewRequest,
) -> Result<VerifiedInputs, E> {
    verified_with_funding(tx, root, settings, work_id, r, FundingBoundary::KnownResult)
}
/// Engine-selected lifecycle semantics, never a public request or operator knob.
#[derive(Clone, Copy)]
enum FundingBoundary {
    Prospective,
    KnownResult,
}
/// Shared verifier keeps every canonical input and live lease fence identical;
/// only the requested internal lifecycle funding predicate differs. No implicit
/// authority/default, process, acceptance or mutation; propagate typed failures.
fn verified_with_funding(
    tx: &rusqlite::Connection,
    root: &std::path::Path,
    settings: &crate::EngineSettingsDocument,
    work_id: &str,
    r: &PlanningCallPreviewRequest,
    funding: FundingBoundary,
) -> Result<VerifiedInputs, E> {
    if tx.is_autocommit() {
        return Err(E::Refused("planning-call-transaction-required"));
    }
    let w = crate::read_recorded_work(tx, work_id)
        .map_err(|e| E::caused("planning-call-work-invalid", e))?;
    if w.revision != r.expected_revision {
        return Err(E::Refused("revision_conflict"));
    }
    crate::validate_planning_state(&w).map_err(|e| E::caused("planning-call-state-invalid", e))?;
    crate::root_store::require_planning_mission_active(tx, work_id)
        .map_err(|e| E::caused(e.code(), e))?;
    let fence = crate::PlanningRequest {
        schema_version: 1,
        scope: r.scope.clone(),
        scope_revision: r.scope_revision,
        holder: r.holder.clone(),
        generation: r.generation,
        ..Default::default()
    };
    let at = crate::SystemEnvironment
        .timestamp()
        .map_err(|e| E::caused("planning-call-clock", e))?;
    crate::planning_decision::fence(&w, &fence, &at)?;
    let scope = w
        .planning
        .as_deref()
        .and_then(|p| p.scopes.as_deref())
        .and_then(|s| s.iter().find(|s| s.id == r.scope))
        .ok_or(E::Refused("planning-scope-not-found"))?;
    let Binding {
        owner,
        scope_id,
        generation,
        scope_revision,
        claim_revision,
        holder,
        context,
        context_sha,
        workflow_json,
        workflow_prompt,
        workflow_prompt_sha,
        model_json,
        recorded_at,
    } = binding(tx, &r.claim_id)?;
    if owner != work_id
        || scope_id != r.scope
        || generation != r.generation
        || scope_revision != r.scope_revision
        || holder != r.holder
    {
        return Err(E::Refused("planning-call-binding-mismatch"));
    }
    let event: (String, i64, String, String, Vec<u8>) = crate::agent_launch::query(
        tx,
        "SELECT work_id,revision,kind,at,request FROM main.events WHERE id=?",
        [&r.claim_id],
        |rows| {
            let row = rows
                .next()?
                .ok_or(E::Refused("planning-call-event-missing"))?;
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        },
    )?;
    let original = crate::decode_planning_request(&event.4)
        .map_err(|e| E::caused("planning-call-event-invalid", e))?;
    if (event.0, event.1, event.2, event.3)
        != (owner, claim_revision, "planning.claim".into(), recorded_at)
        || original.scope != r.scope
        || original.holder != r.holder
        || original.scope_revision != r.scope_revision
    {
        return Err(E::Refused("planning-call-event-mismatch"));
    }
    let hash = |b: &[u8]| format!("{:x}", Sha256::digest(b));
    let delivery = scope
        .delivery
        .as_deref()
        .ok_or(E::Refused("planning-call-delivery-missing"))?;
    if !delivery.decision.is_empty()
        || context_sha != delivery.context_sha256
        || hash(context.as_bytes()) != context_sha
        || hash(workflow_prompt.as_bytes()) != workflow_prompt_sha
    {
        return Err(E::Refused("planning-call-input-hash-mismatch"));
    }
    let workflow: crate::AgentWorkflow = crate::work::decode_mode(
        workflow_json.as_bytes(),
        crate::models::shape_agent_workflow(),
        true,
    )
    .map_err(|e| E::caused("planning-call-workflow-invalid", e))?;
    let current = crate::read_agent_workflow(
        root,
        if scope.parent.is_empty() {
            "planner"
        } else {
            "subplanner"
        },
        settings
            .values
            .agent_workflow_body_limit()
            .map_err(|e| E::caused(e.reason(), e))?,
        &settings
            .values
            .context_policy_limits()
            .map_err(|e| E::caused(e.reason(), e))?,
    )
    .map_err(|e| E::caused("planning-call-workflow-read", e))?;
    if scope.workflow.as_deref() != Some(&workflow)
        || current.workflow != workflow
        || current.prompt_sha256 != workflow_prompt_sha
    {
        return Err(E::Refused("planning-call-workflow-changed"));
    }
    let model: crate::RoleModel = crate::work::decode_mode(
        model_json.as_bytes(),
        crate::models::shape_role_model(),
        true,
    )
    .map_err(|e| E::caused("planning-call-model-invalid", e))?;
    if scope.last_model.as_deref() != Some(&model) {
        return Err(E::Refused("planning-call-model-binding-changed"));
    }
    if model.provider.is_empty() {
        return Err(E::Refused("planning-call-provider-required"));
    }
    let mut observed =
        crate::planning_activation::observe_model(root, &w, &r.scope, settings, &at)?;
    observed.at = model.at.clone();
    observed.actor = model.actor.clone();
    if observed != model {
        return Err(E::Refused("planning-call-model-changed"));
    }
    let value: serde_json::Value =
        serde_json::from_str(&context).map_err(|e| E::caused("planning-call-context-format", e))?;
    if value.get("work").and_then(|v| v.as_str()) != Some(work_id)
        || value
            .get("scope")
            .and_then(|v| v.get("id"))
            .and_then(|v| v.as_str())
            != Some(r.scope.as_str())
    {
        return Err(E::Refused("planning-call-context-owner-mismatch"));
    }
    let events = value
        .get("events")
        .and_then(|v| v.as_array())
        .ok_or(E::Refused("planning-call-context-events-invalid"))?;
    let ids: Option<Vec<String>> = events
        .iter()
        .map(|v| v.get("id").and_then(|v| v.as_str()).map(str::to_owned))
        .collect();
    let ids = ids.ok_or(E::Refused("planning-call-context-events-invalid"))?;
    if ids.is_empty() || delivery.events.as_ref() != Some(&ids) {
        return Err(E::Refused("planning-call-context-events-mismatch"));
    }
    let report_limit = settings
        .values
        .planning_report_limit()
        .map_err(|e| E::caused(e.reason(), e))?;
    for (id, saved) in ids.iter().zip(events) {
        let event = w
            .planning
            .as_deref()
            .and_then(|p| p.inbox.as_deref())
            .and_then(|es| {
                es.iter()
                    .find(|e| e.id == *id && e.scope == r.scope && e.decision.is_empty())
            })
            .ok_or(E::Refused("planning-call-input-not-pending"))?;
        if serde_json::to_value(event).map_err(|e| E::caused("planning-call-event-encoding", e))?
            != *saved
        {
            return Err(E::Refused("planning-call-input-changed"));
        }
        if !delivery.historical_only
            && !event.task.is_empty()
            && w.tasks
                .iter()
                .flatten()
                .find(|t| t.id == event.task && t.scope_id == r.scope)
                .is_none_or(|t| {
                    !event.attempt.is_empty()
                        && t.attempts
                            .as_deref()
                            .unwrap_or_default()
                            .last()
                            .is_none_or(|a| a.id != event.attempt)
                })
        {
            return Err(E::Refused("planning-call-input-stale-attempt"));
        }
        crate::read_planning_report(root, event, report_limit)
            .map_err(|e| E::caused("planning-call-report-invalid", e))?;
    }
    let policy = settings
        .values
        .planning_action_policy()
        .map_err(|e| E::caused(e.reason(), e))?;
    if policy.context_max_bytes.is_some_and(|n| context.len() > n)
        || policy.context_max_events.is_some_and(|n| ids.len() > n)
    {
        return Err(E::Refused("planning-call-current-context-policy"));
    }
    // The immutable input hash alone does not establish current task contracts.
    // Compare the same complete owned projection used to build initial context;
    // unrelated events/revisions can advance without rewriting the delivered batch.
    // Managed candidate metadata is compared too, and its original objects are
    // reobserved before permission; this never grants a managed worker launch.
    #[cfg(target_os="linux")]
    if w.planning.as_deref().is_some_and(|p|p.repository.is_some()) {
        crate::managed_repository::observe(root,&w,settings)?;
    }
    let current_contract =
        serde_json::to_value(crate::planning_scope_context::contract(&w, scope)?)
            .map_err(|e| E::caused("planning-call-contract-encoding", e))?;
    for key in ["work", "scope", "requirements", "tasks", "judge", "repository"] {
        if value.get(key) != current_contract.get(key) {
            return Err(E::Refused("planning-call-scope-contract-changed"));
        }
    }
    let combined = context
        .len()
        .checked_add(workflow_prompt.len())
        .ok_or(E::Refused("planning-prompt-size-representation"))?;
    if settings
        .values
        .agent_launch_policy()
        .map_err(|e| E::caused(e.reason(), e))?
        .context_bytes
        .is_some_and(|n| combined > n)
    {
        return Err(E::Refused("planning-prompt-byte-limit-exceeded"));
    }
    match funding {
        FundingBoundary::Prospective => {
            crate::check_reserved_planning_start_budget(tx, work_id, &r.scope, &r.claim_id)
        }
        FundingBoundary::KnownResult => {
            crate::financial_budget::check_reserved_planning_result_budget(
                tx,
                work_id,
                &r.scope,
                &r.claim_id,
            )
        }
    }
    .map_err(|e| E::caused(e.reason(), e))?;
    let now = crate::SystemEnvironment
        .timestamp()
        .map_err(|e| E::caused("planning-call-clock", e))?;
    crate::planning_decision::fence(&w, &fence, &now)?;
    let preview = PlanningCallPreview {
        schema_version: 1,
        work_id: work_id.into(),
        scope_id: r.scope.clone(),
        claim_id: r.claim_id.clone(),
        work_revision: w.revision,
        original_claim_revision: claim_revision,
        generation: r.generation,
        context_sha256: context_sha,
        workflow_sha256: workflow.sha256,
        workflow_prompt_sha256: workflow_prompt_sha,
        context_bytes: context.len(),
        workflow_bytes: workflow_prompt.len(),
        provider: model.provider.clone(),
        model_route: model.route.clone(),
        historical_acknowledgement_only: delivery.historical_only,
        observed_at: now,
        lease_until: scope.lease_until.clone(),
        prepared_inputs_verified: true,
        process_started: false,
        execution_available: true,
    };
    if original.lease_seconds < 1 {
        return Err(E::Refused("planning-call-original-lease-invalid"));
    }
    Ok(VerifiedInputs {
        preview,
        context,
        workflow_prompt,
        model,
        lease_seconds: original.lease_seconds,
    })
}
