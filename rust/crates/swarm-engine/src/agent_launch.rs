//! Engine-owned manual or lease-bound automatic admission. No provider starts here.
#![cfg(target_os = "linux")]
use crate::agent_launch_error::finish;
use crate::agent_launch_preflight::{self, hash, HostPreparation};
use crate::local_storage::LocalDirectory;
use crate::{
    Agent, AgentLaunchError, AgentLaunchPolicy, EngineSettingsDocument, Launch, Provider, Work,
    WorkEnvironment,
};
use rusqlite::{types::ValueRef, Connection, Params, Rows, Transaction};
use serde::Serialize;
use std::{error::Error, path::Path};

/// A stored/previewed intention only. It is not an execution or identity permit.
#[derive(Serialize)]
pub struct AgentLaunchOutcome {
    pub agent: Agent,
    pub created: bool,
    pub preview_only: bool,
}
/// Parameterized query with explicit statement finalization and original cleanup
/// causes. Missing rows are interpreted only by each caller's explicit contract.
pub(crate) fn query<T>(
    db: &Connection,
    sql: &str,
    params: impl Params,
    read: impl FnOnce(&mut Rows<'_>) -> Result<T, AgentLaunchError>,
) -> Result<T, AgentLaunchError> {
    let mut statement = db
        .prepare(sql)
        .map_err(|e| AgentLaunchError::caused("agent-launch-storage", e))?;
    let result = match statement.query(params) {
        Ok(mut rows) => read(&mut rows),
        Err(e) => Err(AgentLaunchError::caused("agent-launch-storage", e)),
    };
    finish(
        result,
        statement
            .finalize()
            .map_err(|e| AgentLaunchError::caused("agent-launch-statement-close", e)),
    )
}
impl From<rusqlite::Error> for AgentLaunchError {
    fn from(e: rusqlite::Error) -> Self {
        Self::caused("agent-launch-storage", e)
    }
}
/// Read exact canonical request/Work bytes by operation identity. This shared
/// consumer neither decodes untrusted data into authority nor interprets absence
/// as permission; callers own transaction, provenance, namespace and cleanup.
fn recorded_request(db: &Connection, event: &str) -> Result<Option<(Vec<u8>, String)>, AgentLaunchError> {
    recorded_request_observed(db, event, || {}, |_| Ok(()))
}
/// Notify presence before decoding/finalizing can fail. The dispatcher preserves
/// this fact across shared storage retries so disappearance never becomes absence.
fn recorded_request_observed(db: &Connection, event: &str, mut seen: impl FnMut(),
    mut decoded: impl FnMut(&(Vec<u8>, String)) -> Result<(), AgentLaunchError>,
) -> Result<Option<(Vec<u8>, String)>, AgentLaunchError> {
    query(
        db,
        "SELECT request,work_id FROM main.agents WHERE id=?",
        [event],
        |rows| {
            let Some(row) = rows.next()? else {
                return Ok(None);
            };
            seen();
            let request = match row.get_ref(0)? {
                ValueRef::Blob(v) | ValueRef::Text(v) => v.to_vec(),
                _ => {
                    return Err(AgentLaunchError::Refused(
                        "agent-launch-recorded-request-invalid",
                    ))
                }
            };
            let original = (request, row.get::<_, String>(1)?);
            decoded(&original)?;
            Ok(Some(original))
        },
    )
}
/// Canonical request bytes bind duplicate identities, before policy normalization.
/// A retained row must agree with its SQL keys and requested Work/task. Replay
/// creates no new attempt/event/reservation and requires no fresh provider call.
fn existing(
    db: &Connection,
    work: &str,
    r: &Launch,
    raw: &[u8],
) -> Result<Option<Agent>, AgentLaunchError> {
    let epoch: i64 = db.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if epoch != 37 {
        return Err(AgentLaunchError::Refused("agent-launch-storage-epoch"));
    }
    let recorded = recorded_request(db, &r.event_id)?;
    let Some((request, recorded_work)) = recorded else {
        return Ok(None);
    };
    if request != raw || recorded_work != work {
        return Err(AgentLaunchError::Refused("agent-launch-event-conflict"));
    }
    let agent = crate::read_agent(db, &r.event_id)
        .map_err(|e| AgentLaunchError::caused("agent-launch-recorded-agent-invalid", e))?;
    crate::agent_store::verify_agent_ownership(db, &agent)
        .map_err(|e| AgentLaunchError::caused("agent-launch-recorded-owner-invalid", e))?;
    if agent.id != r.event_id || agent.work_id != work || agent.task_id != r.task_id {
        return Err(AgentLaunchError::Refused("agent-launch-event-conflict"));
    }
    Ok(Some(agent))
}
/// Reject currently unported launch branches explicitly, never interpreting an
/// omission as a grant. Current entry is an operator-selected manual or scoped
/// worker, without dialogue,
/// delegation, selected skills/memory or recovery. Model routing needs its own
/// explicit provider policy and checked source preparation.
pub(crate) fn supported(r: &Launch) -> Result<(), AgentLaunchError> {
    supported_branch(r,None)
}
/// Accept conductor fields only when an engine-owned fence or verified canonical
/// mission binding supplies the exact expected conductor. Labels alone never do.
pub(crate) fn supported_branch(r:&Launch,conductor:Option<&str>)->Result<(),AgentLaunchError>{
    let origin_matches=match conductor {
        Some(id)=>!id.is_empty() && r.origin=="conducteur" && r.conductor_id==id,
        None=>matches!(r.origin.as_str(),""|"operateur") && r.conductor_id.is_empty(),
    };
    if r.schema_version != 1
        || [&r.event_id, &r.task_id, &r.provider]
            .iter()
            .any(|v| !swarm_identity_contract::name(v.as_bytes()))
    {
        return Err(AgentLaunchError::Refused(
            "agent-launch-identifiers-invalid",
        ));
    }
    if r.brainstorm
        || !r.mode.is_empty()
        || !matches!(r.role.as_str(), "" | "worker")
        || !origin_matches
        || !r.parent.is_empty()
        || !r.previous.is_empty()
        || r.summary_selection.is_some()
        || r.dialogue_restore.is_some()
        || r.prepared_skill_refresh.is_some()
        || r.skills.as_ref().is_some_and(|v| !v.is_empty())
        || r.references.as_ref().is_some_and(|v| !v.is_empty())
        || !r.precondition_evidence.is_empty()
        || !r.plan_brief_hash.is_empty()
    {
        return Err(AgentLaunchError::Refused(
            "agent-launch-request-branch-unported",
        ));
    }
    Ok(())
}
/// Validate persisted manual or scoped operator-worker branches before transition.
/// Hierarchy requires structural organization/reviewer capacity. Managed manual
/// copies require separate attribution at admission and every execution boundary;
/// versioned memory and conversation recovery branches remain refused. Historical
/// Work.memory paths require the separate current memory_files check. Actual provider availability and
/// fresh prerequisite review/gates are separate mandatory caller checks. A
/// blocked/submitted worker cannot bypass the persisted scoped retry gate;
/// only native current changes_requested with the retry signal reaches the
/// separate full original/decision lifecycle check. Other review/recovery
/// branches remain unsupported. This
/// grants no automatic dispatch, parent delegation or task acceptance.
pub(crate) fn supported_work(w: &Work, index: usize) -> Result<(), AgentLaunchError> {
    if let Some(planning) = w.planning.as_deref() {
        crate::validate_mission_organization(w)
            .map_err(|error| AgentLaunchError::caused(error.code(), error))?;
        crate::validate_mission_reviewer_capacity(planning)
            .map_err(|error| AgentLaunchError::caused(error.code(), error))?;
    }
    let task = &w
        .tasks
        .as_deref()
        .ok_or(AgentLaunchError::Refused("agent-launch-task-missing"))?[index];
    if w.planning.is_some() {
        crate::task_lifecycle::require_scoped_worker_restart(task)
            .map_err(|e| AgentLaunchError::caused(e.reason(), e))?;
    }
    if (w.planning.is_none() && w.planning_brief.is_some())
        || w.project_memory.as_ref().is_some_and(|v| !v.is_empty())
        || task
            .memory_selections
            .as_ref()
            .is_some_and(|v| !v.is_empty())
        || task.memory_decision.is_some()
        || task.independent_review.as_deref().is_some_and(|review| !(w.planning.is_some()
            && task.status == "blocked" && task.planning_retry && review.state == "changes_requested"
            && crate::review_evidence::native_receipt_reference(&review.receipt)))
        || task.corrective_recovery.is_some()
        || task.recovered_result.is_some()
        || task.review_coordination.is_some()
        || task.batch_review_resume.is_some()
        || task.provider_relay_decision.is_some()
        || task.brainstorm
        || !task.plan_brief_hash.is_empty()
    {
        return Err(AgentLaunchError::Refused(
            "agent-launch-work-branch-unported",
        ));
    }
    if task.launch_held {
        return Err(AgentLaunchError::Refused("agent-launch-held"));
    }
    if task.plan_max_attempts < 0 || task.plan_tool_limit < 0 {
        return Err(AgentLaunchError::Refused(
            "agent-launch-plan-limits-invalid",
        ));
    }
    Ok(())
}
/// Observe provider/model/host/workspace readiness before a standing FIFO claim,
/// deliberately without slots or workspace occupancy. Uses actual persisted
/// policy and the same provider/preflight consumers as admission; no provider or
/// SQL write runs. This observation is not a launch permit and must be repeated
/// by the real factory. Missing policy/files, unsupported route and host errors
/// retain their causes; canonical UTF-8 workspace is returned only on success.
/// Managed initial tasks return a validated future-copy path without host probing
/// or creating it. This is a standing intention only: the real fenced factory
/// creates and host-checks the copy before admission and execution permission.
pub(crate) fn candidate_workspace(
    root: &Path,
    directory: &LocalDirectory,
    work: &Work,
    task: &str,
    profile: &crate::LaunchProfile,
    settings: &EngineSettingsDocument,
) -> Result<String, AgentLaunchError> {
    let policy = settings.values.agent_launch_policy()
        .map_err(|cause| AgentLaunchError::caused(cause.reason(), cause))?;
    let request = Launch {
        schema_version: 1, task_id: task.into(), provider: profile.provider.clone(),
        role: profile.role.clone(), workspace: profile.workspace.clone(),
        instruction: profile.instruction.clone(), level: profile.level.clone(),
        timeout_seconds: if profile.timeout_seconds == 0 { policy.timeout } else { profile.timeout_seconds },
        capture_output: profile.capture_output, limits: profile.limits.clone(),
        skills: profile.skills.clone(), ..Default::default()
    };
    if request.timeout_seconds < 1 || policy.timeout_cap.is_some_and(|cap| request.timeout_seconds > cap) {
        return Err(AgentLaunchError::Refused("agent-launch-timeout-outside-saved-policy"));
    }
    if policy.instruction_bytes.is_some_and(|cap| request.instruction.len() > cap) {
        return Err(AgentLaunchError::Refused("agent-launch-instruction-outside-saved-policy"));
    }
    field_guide(root)?;
    let (provider, _) = provider(directory, &request, &policy)?;
    cooldown(root, &request.provider)?;
    let model = crate::provider_model_route::resolve(&provider, &request, &settings.values, &policy)?;
    crate::task_model::check_resolved(work, task, model.route())?;
    if work.planning.as_deref().is_some_and(|p| p.repository.is_some()) {
        return crate::managed_repository::candidate_worker(root, work, task, settings);
    }
    let host = agent_launch_preflight::observe(root, &request.provider, &model, &request, &policy)?;
    host.workspace.to_str().map(str::to_owned)
        .ok_or(AgentLaunchError::Refused("agent-workspace-not-utf8"))
}
/// Read configured CLI and API-connection namespace sources under the same saved
/// per-file cap. Validate active collisions/disabled semantics; selected APIs
/// refuse until native execution exists. Return the actual CLI provider and a
/// combined source fingerprint for rereads, never raw API secrets or permission.
/// Read/type/schema/close/digest/conflict errors refuse without a mutation.
pub(crate) fn provider(
    directory: &LocalDirectory,
    r: &Launch,
    policy: &AgentLaunchPolicy,
) -> Result<(Provider, Vec<u8>), AgentLaunchError> {
    let connections = match directory.read_capped("ai-connections.json", policy.provider_bytes) {
        Ok(raw) => Some(raw),
        Err(e) if absent(&e) => None,
        Err(e) => return Err(AgentLaunchError::caused("agent-provider-catalogue-read", e)),
    };
    let raw = directory
        .read_capped("providers.json", policy.provider_bytes)
        .map_err(|e| AgentLaunchError::caused("agent-provider-catalogue-read", e))?;
    let all = crate::decode_providers(&raw)
        .map_err(|e| AgentLaunchError::caused("agent-provider-catalogue-invalid", e))?;
    if all.schema_version != 1 {
        return Err(AgentLaunchError::Refused(
            "agent-provider-catalogue-schema-invalid",
        ));
    }
    let connection_source =
        crate::provider_connection_catalogue::guard(connections.as_deref(), &all, &r.provider)?;
    let sources = crate::encode_model(&serde_json::json!({
        "providers_file_sha256":hash(&raw),
        "api_connections_file_sha256":connection_source
    }))
    .map_err(|e| AgentLaunchError::caused("agent-provider-sources-encoding", e))?;
    let p = all
        .providers
        .and_then(|mut providers| providers.remove(&r.provider))
        .ok_or(AgentLaunchError::Refused("agent-provider-not-configured"))?;
    if !r.provider_digest.is_empty()
        && hash(
            &crate::encode_model(&p)
                .map_err(|e| AgentLaunchError::caused("agent-provider-encoding", e))?,
        ) != r.provider_digest
    {
        return Err(AgentLaunchError::Refused("agent-provider-digest-changed"));
    }
    Ok((p, sources))
}
/// Refuse field-guide context until its complete native loader is ported. This
/// guard is checked initially and immediately before storing the candidate;
/// unreadable metadata is never treated as absence or silently omitted context.
pub(crate) fn field_guide(root: &Path) -> Result<(), AgentLaunchError> {
    match std::fs::symlink_metadata(root.join(".claude/field-guide/index.md")) {
        Ok(_) => {
            return Err(AgentLaunchError::Refused(
                "agent-field-guide-launch-context-unported",
            ))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(AgentLaunchError::caused("agent-field-guide-metadata", e)),
    }
    Ok(())
}
/// Recognize only actual missing owned files, never permission/format failures.
fn absent(error: &crate::local_storage::LocalStorageError) -> bool {
    error
        .source()
        .and_then(|e| e.downcast_ref::<rustix::io::Errno>())
        .is_some_and(|e| *e == rustix::io::Errno::NOENT)
}
/// Read and validate a retained provider cooldown without granting admission.
/// Missing directory/record returns None. Other path, decode, shape and timestamp
/// failures retain their original typed cause. The continuation runs before the
/// mandatory close, so its refusal and a close failure both survive via finish.
/// Reads only; the caller chooses its clock and interpretation of validated data.
/// Descriptor checks are inherited from LocalDirectory, not an atomic snapshot.
pub(crate) fn with_provider_cooldown<T>(
    root: &Path,
    name: &str,
    observe: impl FnOnce(crate::ProviderCooldown) -> Result<T, AgentLaunchError>,
) -> Result<Option<T>, AgentLaunchError> {
    let path = root.join(".swarm/provider-cooldowns");
    let dir = match LocalDirectory::open(&path) {
        Ok(v) => v,
        Err(e) if absent(&e) => return Ok(None),
        Err(e) => return Err(AgentLaunchError::caused("agent-provider-cooldown-read", e)),
    };
    let result = (|| {
        let raw = match dir.read_capped(&format!("{name}.json"), None) {
            Ok(v) => v,
            Err(e) if absent(&e) => return Ok(None),
            Err(e) => return Err(AgentLaunchError::caused("agent-provider-cooldown-read", e)),
        };
        let value: crate::ProviderCooldown =
            crate::work::decode_mode(&raw, crate::agent_models::shape_provider_cooldown(), true)
                .map_err(|e| AgentLaunchError::caused("agent-provider-cooldown-invalid", e))?;
        if value.provider != name
            || value.observed_at.is_empty()
            || value.source.is_empty()
            || value.signal.is_empty()
            || value.resets_at < 0
        {
            return Err(AgentLaunchError::Refused("agent-provider-cooldown-invalid"));
        }
        // The canonical Go writer uses RFC3339Nano. A nonempty malformed
        // clearance must not turn an active refusal into successful admission.
        // This checks representation only, adding no age/future/skew policy.
        for (code, stamp) in [
            (
                "agent-provider-cooldown-observed-time-invalid",
                value.observed_at.as_str(),
            ),
            (
                "agent-provider-cooldown-clear-time-invalid",
                value.cleared_at.as_str(),
            ),
        ] {
            if !stamp.is_empty() {
                swarm_wire_json::go_rfc3339::parse(stamp)
                    .map_err(|e| AgentLaunchError::caused(code, e))?;
            }
        }
        observe(value).map(Some)
    })();
    finish(
        result,
        dir.close()
            .map_err(|e| AgentLaunchError::caused("agent-provider-cooldown-close", e)),
    )
}
/// Honor a persisted provider refusal before and inside admission. Expiry or
/// explicit clearance makes it inactive; no reset or permission is inferred.
/// Clock/validation/IO and close errors retain their typed original causes.
/// The active refusal is decided before directory close, preserving both errors.
pub(crate) fn cooldown(root: &Path, name: &str) -> Result<(), AgentLaunchError> {
    with_provider_cooldown(root, name, |value| {
        let now = crate::system_timestamp()
            .map_err(|e| AgentLaunchError::caused("agent-launch-clock", e))?;
        let now = swarm_wire_json::go_rfc3339::parse(&now)
            .map_err(|e| AgentLaunchError::caused("agent-launch-clock", e))?;
        if value.cleared_at.is_empty()
            && (value.resets_at == 0 || now.unix_timestamp() < value.resets_at)
        {
            return Err(AgentLaunchError::Refused("provider_cooldown"));
        }
        Ok(())
    }).map(|_| ())
}
/// Common admission guards are SQL authoritative, within the reserved writer.
/// Archive, unreadable pause, preparation lock, overlapping agent or integration
/// refuse with no new attempt or estimate. Limits/counters are never reset.
fn guards(
    tx: &Connection,
    work: &str,
    r: &Launch,
    workspace: &str,
) -> Result<(), AgentLaunchError> {
    guards_owned(tx, work, r, workspace, None)
}
/// Recheck the common guards for this actual existing intention. Only its own
/// verified SQL row may be excluded from the overlap query; all other active
/// agents, pause/archive/preparation/integration and epoch checks remain active.
/// This consistency is not identity authentication or execution permission.
pub(crate) fn execution_guards(
    tx: &Connection,
    work: &str,
    r: &Launch,
    agent: &Agent,
) -> Result<(), AgentLaunchError> {
    crate::agent_store::verify_agent_ownership(tx, agent)
        .map_err(|e| AgentLaunchError::caused("agent-execution-owner-invalid", e))?;
    let actual = crate::read_agent(tx, &agent.id)
        .map_err(|e| AgentLaunchError::caused("agent-execution-agent-invalid", e))?;
    if actual != *agent
        || agent.id != r.event_id
        || agent.work_id != work
        || agent.task_id != r.task_id
    {
        return Err(AgentLaunchError::Refused(
            "agent-execution-identity-conflict",
        ));
    }
    guards_owned(tx, work, r, &agent.workspace, Some(&agent.id))
}
/// One shared guard implementation; None excludes no row, including malformed
/// empty IDs. The existing-owner path must validate that row before exclusion.
/// Canonical Unix path boundaries trim trailing separators before adding one,
/// preserving filesystem-root ancestry. Agent and running-integration guards
/// share that rule. Rows are resource observations, never process/actor authority;
/// this reads only and retains SQL/finalize causes through the common query helper.
fn guards_owned(
    tx: &Connection,
    work: &str,
    r: &Launch,
    workspace: &str,
    owner: Option<&str>,
) -> Result<(), AgentLaunchError> {
    let epoch = query(tx, "PRAGMA main.user_version", [], |rows| {
        Ok(rows
            .next()?
            .ok_or(AgentLaunchError::Refused("agent-storage-version-missing"))?
            .get::<_, i64>(0)?)
    })?;
    if epoch != crate::RECORDED_COMMAND_SCHEMA_VERSION {
        return Err(AgentLaunchError::Refused("storage_version_conflict"));
    }
    let archived = query(
        tx,
        "SELECT count(*) FROM main.mission_lifecycle WHERE work_id=?",
        [work],
        |rows| {
            Ok(rows
                .next()?
                .ok_or(AgentLaunchError::Refused("agent-launch-archive-read"))?
                .get::<_, i64>(0)?)
        },
    )?;
    if archived != 0 {
        return Err(AgentLaunchError::Refused("mission_archived"));
    }
    let paused = query(
        tx,
        "SELECT paused FROM main.cockpit_controls WHERE work_id=?",
        [work],
        |rows| match rows.next()? {
            None => Ok(false),
            Some(row) => crate::sqlite_value::boolean(row.get_ref(0)?, 0).map_err(Into::into),
        },
    )?;
    if paused {
        return Err(AgentLaunchError::Refused("agent-launch-mission-paused"));
    }
    let held=query(tx,"SELECT count(*) FROM main.preparation_launch_locks WHERE work_id=? AND task_id=? AND released=0",rusqlite::params![work,r.task_id],|rows|Ok(rows.next()?.ok_or(AgentLaunchError::Refused("agent-launch-preparation-read"))?.get::<_,i64>(0)?))?;
    if held != 0 {
        return Err(AgentLaunchError::Refused(
            "agent-launch-preparation-not-authorized",
        ));
    }
    let busy=query(tx,"SELECT id FROM main.agents WHERE status IN ('queued','starting','running','stopping') AND (? IS NULL OR id!=?) AND (cwd=? OR instr(cwd,rtrim(?,'/')||'/')=1 OR instr(?,rtrim(cwd,'/')||'/')=1) LIMIT 1",rusqlite::params![owner,owner,workspace,workspace,workspace],|rows|Ok(rows.next()?.is_some()))?;
    if busy {
        return Err(AgentLaunchError::Refused("agent-launch-workspace-busy"));
    }
    let busy=query(tx,"SELECT id FROM main.workspace_integrations WHERE state='running' AND (canonical_workspace=? OR instr(canonical_workspace,rtrim(?,'/')||'/')=1 OR instr(?,rtrim(canonical_workspace,'/')||'/')=1) LIMIT 1",rusqlite::params![workspace,workspace,workspace],|rows|Ok(rows.next()?.is_some()))?;
    if busy {
        return Err(AgentLaunchError::Refused("agent-launch-integration-busy"));
    }
    Ok(())
}
/// Stage one actual financial estimate using all four current ledgers. Missing/
/// zero envelope creates no fictitious zero-cost row. Exact insertion/readback
/// detects silent SQL loss; caller must roll back every failure and own admission.
fn reserve(
    tx: &Connection,
    work: &str,
    id: &str,
    budget: Option<crate::Budget>,
) -> Result<(), AgentLaunchError> {
    let Some(budget) = budget.filter(|b| b.limit_usd.0 > 0.0) else {
        return Ok(());
    };
    let inserted=query(tx,"INSERT INTO main.reservations(agent_id,work_id,amount,state) VALUES(?,?,?,'reserved') RETURNING agent_id",rusqlite::params![id,work,budget.reserve_per_launch_usd.0],|rows|Ok(rows.next()?.is_some()))?;
    if !inserted {
        return Err(AgentLaunchError::Refused(
            "agent-launch-reservation-not-recorded",
        ));
    }
    let matches = query(
        tx,
        "SELECT work_id,amount,state FROM main.reservations WHERE agent_id=?",
        [id],
        |rows| {
            let Some(row) = rows.next()? else {
                return Ok(false);
            };
            Ok(row.get::<_, String>(0)? == work
                && row.get::<_, f64>(1)? == budget.reserve_per_launch_usd.0
                && row.get::<_, String>(2)? == "reserved")
        },
    )?;
    if !matches {
        return Err(AgentLaunchError::Refused(
            "agent-launch-reservation-not-preserved",
        ));
    }
    Ok(())
}
/// Compose currently supported manual context and a byte-bound manifest. Existing
/// workflow/project rules and historical recovery-file references are included.
/// No versioned selected memory, dialogue or unported
/// authority is silently dropped. Prompt digests prove bytes only, not receipt.
struct PromptContext<'a> {
    request: &'a Launch,
    workflow: &'a crate::AgentWorkflowObservation,
    policy: &'a AgentLaunchPolicy,
    limits: &'a crate::RunLimits,
    workspace: &'a str,
    original_next: &'a str,
    guard_snapshot: &'a crate::AgentGuardSnapshot,
    model: &'a crate::provider_model_route::ModelRoutePreparation,
}
/// Compose from the post-transition candidate and its actual new attempt. The
/// original next action remains the brief, not the lifecycle's handoff status.
/// Current identity/own-plan and frozen detection policy precede final cap/hash validation;
/// encoding/mapping failures refuse the enclosing transaction without a child.
fn prompt(w: &Work, index: usize, context: PromptContext<'_>) -> Result<String, AgentLaunchError> {
    let PromptContext {
        request: r,
        workflow,
        policy,
        limits,
        workspace,
        original_next,
        guard_snapshot,
        model,
    } = context;
    let task = &w
        .tasks
        .as_deref()
        .ok_or(AgentLaunchError::Refused("agent-launch-task-missing"))?[index];
    let mut instruction = r.instruction.clone();
    if let Some(profile) = &w.launch_profile {
        if !profile.instruction.trim().is_empty() && profile.instruction != instruction {
            instruction = format!(
                "Consignes communes de la mission :\n{}\nConsignes propres à cette tâche :\n{}",
                profile.instruction, instruction
            );
        }
    }
    let (header, checkpoint) = if w.planning.is_some() {
        let chain = crate::planning_scope_ancestors(w, &task.scope_id)
            .map_err(|e| AgentLaunchError::caused("agent-launch-scope-invalid", e))?;
        let own = chain
            .first()
            .ok_or(AgentLaunchError::Refused("agent-launch-scope-missing"))?;
        (
            format!(
                "Périmètre affecté: {}\nObjectif du périmètre: {}\n",
                own.id, own.objective
            ),
            String::new(),
        )
    } else {
        (
            format!(
                "Travail: {}\nObjectif: {}\nPérimètre: {}\n",
                w.title, w.objective, w.scope
            ),
            format!("Checkpoint: {}\n", w.summary),
        )
    };
    let mut text=format!("{}{}Rôle: worker\nTâche {}: {}\nLivrable: {}\nCritères: {}\nProchaine action: {}\n{}Instructions complémentaires: {}\nRacine de cette tentative: {}\nLimites figées: silence={}s, outil={}s, appels={}, répétitions={}, erreurs={}, observation={}\nLa durée seule ne déclenche aucun arrêt. Respecter les droits natifs du fournisseur. Ne pas modifier .swarm/state.db, ne pas accepter la tâche. Produire un handoff avec changements, tests, preuves, risques et prochaine action.\n",workflow.prompt,header,task.id,task.title,task.deliverable,task.criteria.as_deref().unwrap_or_default().join("; "),original_next,checkpoint,instruction,workspace,limits.silence_seconds,limits.tool_seconds,limits.max_tool_calls,limits.max_repeated_calls,limits.max_consecutive_errors,limits.observation_mode);
    text.push_str(&crate::agent_worker_context::current(
        w,
        index,
        &r.event_id,
    )?);
    text.push_str(&crate::memory_files::prompt(w)?);
    text.push_str(
        &guard_snapshot
            .context()
            .map_err(|e| AgentLaunchError::caused(e.reason(), e))?,
    );
    if let Some(route) = model.route() {
        text.push_str(&format!("\nRequested model route (unverified): level={}, model={}, effort={}, billing={}, policy_sha256={}. This request establishes no actual model, account right, price or execution authority.\n",route.level,route.model,route.effort,route.billing,route.policy_hash));
    }
    if policy.context_bytes.is_some_and(|cap| text.len() > cap) {
        return Err(AgentLaunchError::Refused(
            "agent-launch-context-outside-saved-policy",
        ));
    }
    Ok(text)
}
/// Stage agent, matching attempt/Work event, command log and estimate together.
/// Private entry receives engine-observed provider/context; no supplied Agent can
/// reach it. Readbacks detect ignored/reverted writes. Caller owns final commit.
struct Admission<'a> {
    root: &'a Path,
    dir: &'a LocalDirectory,
    work: &'a str,
    r: &'a Launch,
    raw: &'a [u8],
    provider: &'a Provider,
    provider_raw: &'a [u8],
    host: &'a HostPreparation,
    settings: &'a EngineSettingsDocument,
    policy: &'a AgentLaunchPolicy,
    workflow: &'a crate::AgentWorkflowObservation,
    guard_snapshot: &'a crate::AgentGuardSnapshot,
    model: &'a crate::provider_model_route::ModelRoutePreparation,
    planning: Option<&'a crate::PlanningState>,
    fence: Option<&'a dyn crate::work_engine::MutationFence>,
}
impl Admission<'_> {
    /// Reobserve external scope immediately before persistence and after all
    /// environment callbacks. No future lease or provider capability is inferred.
    fn recheck(&self) -> Result<(), AgentLaunchError> {
        let Self {
            root,
            dir,
            r,
            provider: _,
            provider_raw,
            host,
            settings,
            policy,
            workflow,
            model,
            ..
        } = *self;
        cooldown(root, &r.provider)?;
        crate::agent_reviewer_preflight::check(root, dir, self.planning, policy)?;
        model.recheck(r, &settings.values, policy)?;
        agent_launch_preflight::recheck(root, model, r, policy, host)?;
        field_guide(root)?;
        let (_, current) = self::provider(dir, r, policy)?;
        if current != provider_raw {
            return Err(AgentLaunchError::Refused(
                "agent-provider-catalogue-changed",
            ));
        }
        let context_policy = settings
            .values
            .context_policy_limits()
            .map_err(|e| AgentLaunchError::caused(e.reason(), e))?;
        let actual = crate::read_agent_workflow(
            root,
            "worker",
            settings
                .values
                .agent_workflow_body_limit()
                .map_err(|e| AgentLaunchError::caused(e.reason(), e))?,
            &context_policy,
        )
        .map_err(|e| AgentLaunchError::caused(e.code(), e))?;
        if actual.prompt_sha256 != workflow.prompt_sha256 {
            return Err(AgentLaunchError::Refused("agent-launch-workflow-changed"));
        }
        Ok(())
    }
}
/// Automatic fences preserve configured parent/task profiles and bind the generated event to
/// native mission provenance; manual calls retain their original request semantics.
/// Stage one engine-observed admission; reject any final SQL side effect changed
/// by a later trigger. The caller retains commit/rollback and output ownership.
/// Manual task/Work profiles retain the exact bound requested level;
/// an empty level remains intentional inheritance, not an inferred provider tier.
/// Recording it grants neither execution nor model identity and changes no budget.
/// Existing published profiles are not retroactively reconstructed or rewritten.
fn stage<E: WorkEnvironment>(
    tx: &mut Transaction<'_>,
    a: &Admission<'_>,
    environment: &mut E,
) -> Result<AgentLaunchOutcome, AgentLaunchError> {
    let Admission {
        root,
        dir: _,
        work,
        r,
        raw,
        provider,
        provider_raw: _,
        host,
        settings,
        policy,
        workflow,
        guard_snapshot,
        model,
        fence,
        // Recheck consumes the frozen original planning; staging reloads Work.
        planning: _,
    } = *a;
    if let Some(agent) = existing(tx, work, r, raw)? {
        return Ok(AgentLaunchOutcome {
            agent,
            created: false,
            preview_only: false,
        });
    }
    let workspace = host
        .workspace
        .to_str()
        .ok_or(AgentLaunchError::Refused("agent-workspace-utf8-required"))?;
    guards(tx, work, r, workspace)?;
    let mut w = crate::read_recorded_work(tx, work)
        .map_err(|e| AgentLaunchError::caused("agent-launch-work-invalid", e))?;
    if w.revision != r.expected_revision {
        return Err(AgentLaunchError::Refused("revision_conflict"));
    }
    let index = w
        .tasks
        .as_deref()
        .unwrap_or_default()
        .iter()
        .position(|t| t.id == r.task_id)
        .ok_or(AgentLaunchError::Refused("agent-launch-task-missing"))?;
    crate::root_store::require_validation_idle(tx,work,&r.task_id)?;
    let mut actual_request = r.clone();
    crate::task_model::bind_launch(&w, &mut actual_request)?;
    if actual_request.level != r.level || actual_request.model_policy_hash != r.model_policy_hash {
        return Err(AgentLaunchError::Refused("task-model-launch-mismatch"));
    }
    crate::task_model::check_resolved(&w, &r.task_id, model.route())?;
    supported_work(&w, index)?;
    crate::memory_files::check(root, tx, &w)?;
    let runtime_policy = settings
        .values
        .runtime_limit_policy()
        .map_err(|e| AgentLaunchError::caused(e.reason(), e))?;
    let admin = crate::run_limits::read_configured_runtime_limits(
        tx,
        &runtime_policy,
        work,
        "worker",
        &r.task_id,
    )
    .map_err(|e| AgentLaunchError::caused(e.code(), e))?;
    let empty = crate::RunLimits::default();
    let mut limits = crate::resolve_run_limits(
        &provider.limits,
        r.limits.as_deref().unwrap_or(&empty),
        &admin,
        None,
        &runtime_policy,
    )
    .map_err(|e| AgentLaunchError::caused(e.code(), e))?;
    let task = &w
        .tasks
        .as_ref()
        .ok_or(AgentLaunchError::Refused("agent-launch-task-missing"))?[index];
    if task.status == "running" {
        return Err(AgentLaunchError::Refused(
            "agent-launch-task-already-running",
        ));
    }
    if task.plan_max_attempts > 0 && limits.observation_mode != 1 {
        let count = query(
            tx,
            "SELECT count(*) FROM main.agents WHERE work_id=? AND task_id=?",
            rusqlite::params![work, r.task_id],
            |rows| {
                Ok(rows
                    .next()?
                    .ok_or(AgentLaunchError::Refused("agent-launch-history-read"))?
                    .get::<_, i64>(0)?)
            },
        )?;
        if count >= task.plan_max_attempts {
            return Err(AgentLaunchError::Refused(
                "agent-launch-plan-attempts-exhausted",
            ));
        }
    }
    if task.plan_tool_limit > 0 && task.plan_tool_limit < limits.max_tool_calls {
        limits.max_tool_calls = task.plan_tool_limit;
    }
    let original_next = task.next.clone();
    a.recheck()?;
    let budget = crate::financial_budget::admission_budget(tx, work)
        .map_err(|e| AgentLaunchError::caused(e.code(), e))?;
    let mutation = settings
        .values
        .mutation_limits()
        .map_err(|e| AgentLaunchError::caused(e.reason(), e))?;
    let quality = settings
        .quality_policy_snapshot()
        .map_err(|e| AgentLaunchError::caused(e.reason(), e))?;
    let context = crate::TaskLifecycleContext {
        root,
        quality_policy: Some(&quality),
        task_limits: &mutation.task,
        validation_limits: &mutation.policy,
    };
    let request = crate::Request {
        schema_version: 1,
        id: r.task_id.clone(),
        status: "running".into(),
        owner: r.provider.clone(),
        origin: "conducteur".into(),
        // Preserve a retry's canonical correction until the actual grant. The
        // post-admission preflight must compare the same persisted instruction,
        // not a generic progress message or an invented prompt reconstruction.
        next: if task.planning_retry { task.next.clone() } else { "Examiner le handoff et les preuves après exécution".into() },
        ..Default::default()
    };
    let mut next = if w.planning.is_some() {
        crate::transition_scoped_worker_task(&w, &request, &context, settings, environment)
    } else {
        crate::transition_manual_task(&w, &request, &context, environment)
    }
    .map_err(|e| AgentLaunchError::caused(e.code(), e))?;
    let attempt_id = format!(
        "a-{}",
        &hash(format!("{work}|{}", r.event_id).as_bytes())[..24]
    );
    next.attempts
        .as_mut()
        .and_then(|v| v.last_mut())
        .ok_or(AgentLaunchError::Refused(
            "agent-launch-attempt-not-created",
        ))?
        .id = attempt_id.clone();
    if let Some(revisions) = next.instruction_revisions.as_mut() {
        for revision in revisions {
            if revision.state == "applied" && revision.used_by_attempt.is_empty() {
                revision.used_by_attempt = attempt_id.clone();
            }
        }
    }
    let at = environment
        .timestamp()
        .map_err(|e| AgentLaunchError::caused("agent-launch-clock", e))?;
    swarm_wire_json::go_rfc3339::parse(&at)
        .map_err(|e| AgentLaunchError::caused("agent-launch-clock", e))?;
    let profile = crate::LaunchProfile {
        provider: r.provider.clone(),
        role: "worker".into(),
        workspace: workspace.into(),
        instruction: r.instruction.clone(),
        level: r.level.clone(),
        timeout_seconds: r.timeout_seconds,
        capture_output: r.capture_output,
        limits: r.limits.clone(),
        updated: at.clone(),
        actor: if fence.and_then(|f|f.mission_conductor()).is_some() {"conducteur"} else {"operateur"}.into(),
        ..Default::default()
    };
    // Automatic admission hashes the configured task profile. Preserve that
    // precondition rather than turning this attempt's derived request into a
    // saved override; the actual request remains in Agent and canonical events.
    next.launch_profile = if fence.and_then(|f| f.mission_profile()).is_some() {
        task.launch_profile.clone()
    } else {
        Some(Box::new(profile.clone()))
    };
    next.planning_retry = false;
    let mut common = profile;
    common.skills = None;
    common.limits = w.launch_profile.as_ref().and_then(|v| v.limits.clone());
    common.timeout_seconds = w.launch_profile.as_ref().map_or(0, |v| v.timeout_seconds);
    common.instruction = w
        .launch_profile
        .as_ref()
        .map_or_else(String::new, |v| v.instruction.clone());
    if fence.and_then(|f|f.mission_profile()).is_none() {
        w.launch_profile = Some(Box::new(common));
    }
    w.tasks
        .as_mut()
        .ok_or(AgentLaunchError::Refused("agent-launch-task-missing"))?[index] = next;
    w.revision = w
        .revision
        .checked_add(1)
        .ok_or(AgentLaunchError::Refused("agent-launch-revision-overflow"))?;
    w.updated = at.clone();
    let body = prompt(
        &w,
        index,
        PromptContext {
            request: r,
            workflow,
            policy,
            limits: &limits,
            workspace,
            original_next: &original_next,
            guard_snapshot,
            model,
        },
    )?;
    if !r.context_hash.is_empty() && r.context_hash != hash(body.as_bytes()) {
        return Err(AgentLaunchError::Refused(
            "agent-launch-context-hash-changed",
        ));
    }
    let mut agent = Agent::default();
    agent.native_tool_guard_policy = Some(Box::new(guard_snapshot.clone()));
    agent.model_route = model.route().cloned().map(Box::new);
    agent.id = r.event_id.clone();
    agent.work_id = work.into();
    agent.task_id = r.task_id.clone();
    agent.attempt_id = attempt_id;
    agent.origin = if fence.and_then(|f|f.mission_conductor()).is_some() {"conducteur"} else {"operateur"}.into();
    agent.provider = r.provider.clone();
    agent.role = "worker".into();
    agent.workspace = workspace.into();
    agent.status = "queued".into();
    agent.activity = "Lancement demandé ; processus non confirmé".into();
    agent.started = at.clone();
    agent.host = crate::SupervisorIdentity::current()
        .map_err(|e| AgentLaunchError::caused(e.code(), e))?
        .host()
        .into();
    agent.timeout_seconds = r.timeout_seconds;
    agent.capture_output = r.capture_output;
    agent.prompt = body;
    agent.command = provider.command.clone();
    agent.args = provider.args.clone();
    agent.env_allow = Some(host.environment_names.clone());
    agent.limits = limits;
    agent.workflow = Some(Box::new(workflow.workflow.clone()));
    agent.preflight = Some(Box::new(host.preflight.clone()));
    agent.context = Some(Box::new(crate::ContextManifest {
        sha256: hash(agent.prompt.as_bytes()),
        bytes: i64::try_from(agent.prompt.len())
            .map_err(|e| AgentLaunchError::caused("agent-launch-context-length", e))?,
        included: Some(vec![r.task_id.clone()]),
        paths: w.memory.clone(),
        ..Default::default()
    }));
    let event = environment.identifier("launch-")
        .map_err(|e|AgentLaunchError::caused("agent-launch-event-identity",e))?;
    if let Some(fence)=fence {
        agent.native_mission=fence.mission_binding(tx,&agent,crate::work_engine::MutationFenceReceipt{work,event:&event,kind:"agent.start",work_revision:w.revision,command_settings:settings})
            .map_err(|e|AgentLaunchError::caused("agent-launch-mission-binding",e))?;
    }
    let encoded = crate::encode_model(&agent)
        .map_err(|e| AgentLaunchError::caused("agent-launch-agent-encoding", e))?;
    let inserted=query(tx,"INSERT INTO main.agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,'queued',?,?) RETURNING id",rusqlite::params![agent.id,work,agent.task_id,workspace,encoded,raw],|rows|Ok(rows.next()?.is_some()))?;
    if !inserted {
        return Err(AgentLaunchError::Refused("agent-launch-agent-not-recorded"));
    }
    if w.planning.as_deref().is_some_and(|p|p.repository.is_some()) {
        crate::managed_repository::verify_worker(root,tx,&w,&agent,settings,crate::managed_repository::WorkerCheck::Record)?;
    }
    a.recheck()?;
    // Reobserve paths after callbacks before publishing this intention. Source
    // versions are read on the same transaction snapshot, not a second Root.
    crate::memory_files::check(root, tx, &w)?;
    let outcome = crate::stage_recorded_change(
        tx,
        crate::RecordedChange {
            work: &w,
            expected_revision: r.expected_revision,
            event: &event,
            kind: "agent.start",
            request: raw,
        },
    )
    .map_err(|e| AgentLaunchError::caused("agent-launch-work-event", e))?;
    if !matches!(outcome, crate::RecordedChangeOutcome::Staged) {
        return Err(AgentLaunchError::Refused("agent-launch-event-conflict"));
    }
    reserve(tx, work, &agent.id, budget.clone())?;
    let launch_message=if agent.native_mission.is_some() {"Lancement automatique demandé sous bail de mission"} else {"Lancement demandé par opérateur local"};
    let sequence:i64=query(tx,"INSERT INTO main.agent_logs(agent_id,at,kind,message) VALUES(?,?,'command',?) RETURNING seq",rusqlite::params![agent.id,at,launch_message],|rows|Ok(rows.next()?.ok_or(AgentLaunchError::Refused("agent-launch-log-not-recorded"))?.get(0)?))?;
    let valid = query(
        tx,
        "SELECT at,kind,message FROM main.agent_logs WHERE seq=? AND agent_id=?",
        rusqlite::params![sequence, agent.id],
        |rows| {
            let Some(row) = rows.next()? else {
                return Ok(false);
            };
            Ok(row.get::<_, String>(0)? == at
                && row.get::<_, String>(1)? == "command"
                && row.get::<_, String>(2)? == launch_message)
        },
    )?;
    if !valid {
        return Err(AgentLaunchError::Refused("agent-launch-log-not-preserved"));
    }
    crate::agent_store::verify_agent_ownership(tx, &agent)
        .map_err(|e| AgentLaunchError::caused("agent-launch-row-owner-not-preserved", e))?;
    let observed = query(
        tx,
        "SELECT body,request,status,desired FROM main.agents WHERE id=?",
        [&agent.id],
        |rows| {
            let row = rows.next()?.ok_or(AgentLaunchError::Refused(
                "agent-launch-agent-not-preserved",
            ))?;
            Ok(row.get::<_, Vec<u8>>(0)? == encoded
                && row.get::<_, Vec<u8>>(1)? == raw
                && row.get::<_, String>(2)? == "queued"
                && row.get::<_, String>(3)?.is_empty())
        },
    )?;
    if !observed
        || crate::read_recorded_work(tx, work)
            .map_err(|e| AgentLaunchError::caused("agent-launch-final-work-read", e))?
            != w
    {
        return Err(AgentLaunchError::Refused(
            "agent-launch-state-not-preserved",
        ));
    }
    // The log insert can trigger changes to effects verified earlier. Check the
    // complete final event/estimate after the last SQL mutation, not just INSERT.
    let event_preserved = query(
        tx,
        "SELECT work_id,revision,kind,at,payload,request FROM main.events WHERE id=?",
        [&event],
        |rows| {
            let Some(row) = rows.next()? else {
                return Ok(false);
            };
            Ok(row.get::<_, String>(0)? == work
                && row.get::<_, i64>(1)? == w.revision
                && row.get::<_, String>(2)? == "agent.start"
                && row.get::<_, String>(3)? == at
                && row.get::<_, Vec<u8>>(4)? == raw
                && row.get::<_, Vec<u8>>(5)? == raw)
        },
    )?;
    let reservation_preserved = query(
        tx,
        "SELECT work_id,amount,state FROM main.reservations WHERE agent_id=?",
        [&agent.id],
        |rows| {
            let row = rows.next()?;
            match (budget.as_ref().filter(|b| b.limit_usd.0 > 0.0), row) {
                (None, None) => Ok(true),
                (Some(b), Some(row)) => Ok(row.get::<_, String>(0)? == work
                    && row.get::<_, f64>(1)? == b.reserve_per_launch_usd.0
                    && row.get::<_, String>(2)? == "reserved"),
                _ => Ok(false),
            }
        },
    )?;
    if !event_preserved || !reservation_preserved {
        return Err(AgentLaunchError::Refused(
            "agent-launch-effects-not-preserved",
        ));
    }
    if let Some(fence) = fence {
        fence
            .stage_receipt(
                tx,
                crate::work_engine::MutationFenceReceipt {
                    work,
                    event: &event,
                    kind: "agent.start",
                    work_revision: w.revision,
                    command_settings: settings,
                },
            )
            .map_err(|e| AgentLaunchError::caused("agent-launch-mission-receipt", e))?;
    }
    if w.planning.as_deref().is_some_and(|p|p.repository.is_some()) {
        crate::managed_repository::verify_worker(root,tx,&w,&agent,settings,crate::managed_repository::WorkerCheck::BeforeExecution)?;
    }
    Ok(AgentLaunchOutcome {
        agent,
        created: true,
        preview_only: false,
    })
}
/// Check an optional Root-owned lease inside the caller transaction. The manual
/// route adds no observation. A guarded route observes a fresh supplied time and
/// preserves the original typed lease/storage cause; this grants no execution.
/// Automatic fences also inspect admission under the supplied reserved settings;
/// ordinary fences preserve their manual semantics. No settings or SQL writes.
fn check_fence<E: WorkEnvironment>(
    fence: Option<&dyn crate::work_engine::MutationFence>,
    db: &Connection,
    work: &str,
    settings: &EngineSettingsDocument,
    environment: &mut E,
) -> Result<(), AgentLaunchError> {
    if let Some(fence) = fence {
        let now = environment
            .timestamp()
            .map_err(|e| AgentLaunchError::caused("agent-launch-mission-clock", e))?;
        fence
            .check_agent(db, work, &now,settings)
            .map_err(|e| AgentLaunchError::caused("agent-launch-mission-fence", e))?;
    }
    Ok(())
}
/// Resolve historical replay while owning the SQL writer and checking the live
/// lease before and after reading. Always roll back, even for absence; never
/// refresh provider/preflight or rewrite an original receipt. Missing/SQL/clock/
/// ownership and mandatory rollback causes retain their typed distinction.
/// Automatic replay also revalidates controls/profile/capacity against the current
/// reserved settings; historical existence alone never renews permission.
fn fenced_existing<E: WorkEnvironment>(
    db: &mut Connection,
    work: &str,
    request: &Launch,
    raw: &[u8],
    fence: &dyn crate::work_engine::MutationFence,
    settings: &EngineSettingsDocument,
    environment: &mut E,
) -> Result<Option<Agent>, AgentLaunchError> {
    with_fenced_replay(db, work, fence, settings, environment, |tx, _| {
        existing(tx, work, request, raw)
    })
}

/// Own a real IMMEDIATE replay snapshot, checking the supplied actual fence on
/// both sides and rolling back EVERY result, including absence. The closure may
/// only observe an original; it must never call prepare or create an intention.
/// Independent rollback/clock/SQL errors retain their original typed causes.
/// A plain MissionLease checks ownership only, so absence is observed before an
/// automatic check_agent could incorrectly reject capacity before queue sync.
pub(crate) fn with_fenced_replay<E: WorkEnvironment, T>(
    db: &mut Connection,
    work: &str,
    fence: &dyn crate::work_engine::MutationFence,
    settings: &EngineSettingsDocument,
    environment: &mut E,
    observe: impl FnOnce(&Connection, &mut E) -> Result<T, AgentLaunchError>,
) -> Result<T, AgentLaunchError> {
    let mut tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let result = (|| {
        check_fence(Some(fence), &tx, work, settings, environment)?;
        let result = observe(&tx, environment)?;
        check_fence(Some(fence), &tx, work, settings, environment)?;
        Ok(result)
    })();
    tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
    finish(result, tx.rollback().map_err(|e| AgentLaunchError::caused("agent-launch-replay-rollback", e)))
}

/// Observe managed admission resources under the actual automatic fence and
/// settings reservation, rolling back the writer pulse. No Agent is fabricated,
/// copy created or host probe performed. Current Work revision, saved profile,
/// provider/model/candidate, overlap and financial/slot/lease guards are read;
/// actual admission must repeat all checks and observe the physical copy/host.
/// Typed operation/rollback causes survive; observation is no execution grant.
pub(crate) fn preview_managed_resources<E: WorkEnvironment>(
    db: &mut Connection, source: PreparationRoot<'_>, work: &str,
    request: &Launch, environment: &mut E,
) -> Result<(), AgentLaunchError> {
    let PreparationRoot {root,dir,settings,fence} = source;
    let fence = fence.ok_or(AgentLaunchError::Refused("mission-agent-fence-required"))?;
    supported_branch(request, fence.mission_conductor())?;
    let profile = fence.mission_profile().ok_or(AgentLaunchError::Refused("mission-profile-required"))?;
    with_fenced_replay(db, work, fence, settings, environment, |tx, _| {
        let actual = crate::read_recorded_work(tx,work)
            .map_err(|e| AgentLaunchError::caused("agent-launch-work-invalid",e))?;
        if actual.revision != request.expected_revision { return Err(AgentLaunchError::Refused("revision_conflict")); }
        if actual.planning.as_deref().is_none_or(|p| p.repository.is_none()) {
            return Err(AgentLaunchError::Refused("managed-repository-required"));
        }
        let index=actual.tasks.iter().flatten().position(|t|t.id==request.task_id)
            .ok_or(AgentLaunchError::Refused("agent-launch-task-missing"))?;
        supported_work(&actual,index)?;
        let workspace=candidate_workspace(root,dir,&actual,&request.task_id,profile,settings)?;
        guards(tx,work,request,&workspace)?;
        crate::financial_budget::admission_budget(tx,work)
            .map_err(|e| AgentLaunchError::caused(e.code(),e))?;
        Ok(())
    })
}
/// Observe a strict stored original inside the caller's writer snapshot.
/// None requires absence of both Agent and its canonical event namespace; an
/// orphan event/reservation refuses. JSON SQL errors remain errors, not absence.
/// Exact reencoding catches unknown/noncanonical original bytes before use.
/// This read grants nothing and must precede automatic capacity/profile guards.
#[derive(Default)]
pub(crate) struct DispatchReplayObservation {
    seen: bool,
    original: Option<(Vec<u8>, String)>,
    namespace_conflict: bool,
}
/// Read the original while retaining presence AND complete request/Work bytes
/// before statement finalization. Shared SQLite retry cannot replace a previously
/// observed operation by a different task/profile/Work under the same event ID.
pub(crate) fn recorded_dispatch_launch(db: &Connection, work: &str, event: &str, observation: &mut DispatchReplayObservation) -> Result<Option<Launch>, AgentLaunchError> {
    if db.is_autocommit() { return Err(AgentLaunchError::Refused("agent-launch-replay-transaction-required")); }
    if observation.namespace_conflict { return Err(AgentLaunchError::Refused("agent-launch-orphaned-operation")); }
    let epoch = query(db, "PRAGMA main.user_version", [], |rows| {
        rows.next()?.ok_or(AgentLaunchError::Refused("agent-storage-version-missing"))?.get::<_,i64>(0).map_err(Into::into)
    })?;
    if epoch != crate::RECORDED_COMMAND_SCHEMA_VERSION { return Err(AgentLaunchError::Refused("agent-launch-storage-epoch")); }
    let original = &mut observation.original;
    let Some((raw, recorded_work)) = recorded_request_observed(db, event, || observation.seen = true, |value| {
        match original {
            Some(previous) if previous != value => return Err(AgentLaunchError::Refused("agent-launch-replay-original-changed")),
            Some(_) => {},
            None => *original = Some(value.clone()),
        }
        Ok(())
    })? else {
        if observation.seen { return Err(AgentLaunchError::Refused("agent-launch-replay-original-disappeared")); }
        let occupied = query(db,
            "SELECT id FROM main.events WHERE id=? OR (kind='agent.start' AND json_extract(CAST(request AS TEXT),'$.event_id')=?) LIMIT 1",
            rusqlite::params![event,event], |rows| {
                let found = rows.next()?.is_some();
                observation.namespace_conflict |= found;
                Ok(found)
            })?;
        if occupied { return Err(AgentLaunchError::Refused("agent-launch-orphaned-operation")); }
        let reserved = query(db, "SELECT agent_id FROM main.reservations WHERE agent_id=? LIMIT 1", [event], |rows| {
            let found = rows.next()?.is_some();
            observation.namespace_conflict |= found;
            Ok(found)
        })?;
        if reserved { return Err(AgentLaunchError::Refused("agent-launch-orphaned-operation")); }
        return Ok(None);
    };
    if recorded_work != work { return Err(AgentLaunchError::Refused("agent-launch-event-conflict")); }
    let launch = crate::decode_launch(&raw).map_err(|e| AgentLaunchError::caused("agent-launch-recorded-request-invalid", e))?;
    let canonical = crate::encode_model(&launch).map_err(|e| AgentLaunchError::caused("agent-launch-recorded-request-encoding", e))?;
    if canonical != raw || launch.event_id != event || recorded_work.is_empty() {
        return Err(AgentLaunchError::Refused("agent-launch-recorded-request-invalid"));
    }
    let agent = crate::read_agent(db, event).map_err(|e| AgentLaunchError::caused("agent-launch-recorded-agent-invalid", e))?;
    crate::agent_store::verify_agent_ownership(db, &agent).map_err(|e| AgentLaunchError::caused("agent-launch-recorded-owner-invalid", e))?;
    if agent.work_id != recorded_work || agent.task_id != launch.task_id || agent.origin != "conducteur" || launch.origin != "conducteur" {
        return Err(AgentLaunchError::Refused("agent-launch-event-conflict"));
    }
    Ok(Some(launch))
}

/// Replay a previously observed original only, inside the SAME owned writer.
/// The current automatic fence repeats actual profile/consent/capacity checks.
/// Missing/replaced original is an error, never a prepare fallback. Canonical
/// command/claim/Agent/FIFO and actual directory/DB instance provenance are
/// checked before returning the real row. Legacy native rows cannot acquire new
/// authority by replay, and the directory must belong to the caller's open Root.
/// No provider refresh, SQL commit, generation rewrite or permission is produced.
pub(crate) fn replay_observed_dispatch<E: WorkEnvironment>(
    db: &Connection, dir: &crate::local_storage::LocalDirectory, work: &str, request: &Launch,
    fence: &dyn crate::work_engine::MutationFence,
    settings: &EngineSettingsDocument, environment: &mut E,
) -> Result<AgentLaunchOutcome, AgentLaunchError> {
    if db.is_autocommit() { return Err(AgentLaunchError::Refused("agent-launch-replay-transaction-required")); }
    supported_branch(request, fence.mission_conductor())?;
    check_fence(Some(fence), db, work, settings, environment)?;
    let raw = crate::encode_model(request).map_err(|e| AgentLaunchError::caused("agent-launch-request-encoding", e))?;
    let agent = existing(db, work, request, &raw)?.ok_or(AgentLaunchError::Refused("agent-launch-replay-original-disappeared"))?;
    crate::root_store::mission_agent::inspect(db, &agent, settings, crate::root_store::mission_agent::Check::BeforeGrant(dir))?
        .ok_or(AgentLaunchError::Refused("mission-agent-binding-required"))?;
    check_fence(Some(fence), db, work, settings, environment)?;
    Ok(AgentLaunchOutcome { agent, created: false, preview_only: false })
}
/// Engine factory for the supported manual path. Idempotent replay precedes any
/// new provider observation; an installed lease fence also checks replay in a
/// writer transaction. Host checks occur outside the writer; admission rechecks
/// scope/config/workflow and stages all database effects atomically. Preview
/// rolls back staging; it may exercise/remove a temporary host write probe.
/// First manual or native-fenced managed admission retains an attributed private Git copy even
/// on later SQL rollback; preview refuses before creating any managed copy.
/// Commit failure returns uncertainty plus rollback cause, never an ownership or
/// execution receipt. No provider subprocess, paid call or automatic acceptance.
pub(crate) struct PreparationRoot<'a> {
    pub root: &'a Path,
    pub dir: &'a LocalDirectory,
    pub settings: &'a EngineSettingsDocument,
    pub fence: Option<&'a dyn crate::work_engine::MutationFence>,
}
/// Construct and stage a manual intention from the original strict request.
/// The caller owns this writable connection/root and holds its settings reserve.
/// Preview rolls SQL back; real admission commits once, replay creates no effects.
/// New intentions freeze the explicitly adopted v17 detection policy/revision;
/// historical replay retains its original metadata, without current defaults.
/// Host checks may create/remove a temporary write probe but start no provider.
/// Refusals preserve typed causes and explicit rollback failures; commit errors
/// distinguish confirmed rollback from uncertainty requiring reread by identity.
/// Optional Root lease checks and new-event receipts share this transaction;
/// preview and guarded replay roll back, including any no-op writer pulse.
/// No identity authentication, automatic consent, execution permission or acceptance
/// is produced.
pub(crate) fn prepare<E: WorkEnvironment>(
    db: &mut Connection,
    source: PreparationRoot<'_>,
    work: &str,
    request: &Launch,
    preview: bool,
    environment: &mut E,
) -> Result<AgentLaunchOutcome, AgentLaunchError> {
    let PreparationRoot {
        root,
        dir,
        settings,
        fence,
    } = source;
    if let Some(conductor)=fence.and_then(|f|f.mission_conductor()) {supported_branch(request,Some(conductor))?;} else {supported(request)?;}
    if !swarm_identity_contract::name(work.as_bytes()) {
        return Err(AgentLaunchError::Refused("agent-launch-work-id-invalid"));
    }
    let raw = crate::encode_model(request)
        .map_err(|e| AgentLaunchError::caused("agent-launch-request-encoding", e))?;
    let replay = match fence {
        Some(fence) => fenced_existing(db, work, request, &raw, fence, settings, environment)?,
        None => existing(db, work, request, &raw)?,
    };
    if let Some(agent) = replay {
        return Ok(AgentLaunchOutcome {
            agent,
            created: false,
            preview_only: preview,
        });
    }
    let policy = settings
        .values
        .agent_launch_policy()
        .map_err(|e| AgentLaunchError::caused(e.reason(), e))?;
    let guard_snapshot = settings
        .values
        .tool_guard_policy()
        .and_then(|policy| policy.freeze(settings.revision))
        .map_err(|e| AgentLaunchError::caused(e.reason(), e))?;
    let mut r = request.clone();
    let chosen_work = crate::read_recorded_work(db, work)
        .map_err(|e| AgentLaunchError::caused("agent-launch-work-invalid", e))?;
    if chosen_work.revision != r.expected_revision {
        return Err(AgentLaunchError::Refused("revision_conflict"));
    }
    crate::task_model::bind_launch(&chosen_work, &mut r)?;
    if r.role.is_empty() {
        r.role = "worker".into();
    }
    if r.timeout_seconds == 0 {
        r.timeout_seconds = policy.timeout;
    }
    if r.timeout_seconds < 1 || policy.timeout_cap.is_some_and(|v| r.timeout_seconds > v) {
        return Err(AgentLaunchError::Refused(
            "agent-launch-timeout-outside-saved-policy",
        ));
    }
    if policy
        .instruction_bytes
        .is_some_and(|cap| r.instruction.len() > cap)
    {
        return Err(AgentLaunchError::Refused(
            "agent-launch-instruction-outside-saved-policy",
        ));
    }
    field_guide(root)?;
    crate::memory_files::check(root, db, &chosen_work)?;
    let (original_provider, provider_raw) = provider(dir, &r, &policy)?;
    cooldown(root, &r.provider)?;
    let model =
        crate::provider_model_route::resolve(&original_provider, &r, &settings.values, &policy)?;
    crate::task_model::check_resolved(&chosen_work, &r.task_id, model.route())?;
    let p = model.provider();
    let managed=chosen_work.planning.as_deref().is_some_and(|p|p.repository.is_some());
    if managed {
        if preview { return Err(AgentLaunchError::Refused("agent-launch-work-branch-unported")); }
        let index=chosen_work.tasks.iter().flatten().position(|t|t.id==r.task_id).ok_or(AgentLaunchError::Refused("agent-launch-task-missing"))?;
        supported_work(&chosen_work,index)?;
        // Revalidate the actual automatic fence before any retained copy effect.
        if fence.and_then(|f| f.mission_conductor()).is_some() {
            let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(|e| AgentLaunchError::caused("managed-worker-fence-writer",e))?;
            let checked = check_fence(fence,&tx,work,settings,environment);
            finish(checked,tx.rollback().map_err(|e| AgentLaunchError::caused("managed-worker-fence-rollback",e)))?;
        }
        r.workspace=crate::managed_repository::prepare_worker(root,&chosen_work,&r,&raw,settings,fence.and_then(|f|f.mission_conductor()))?;
    }
    let host = agent_launch_preflight::observe(root, &r.provider, &model, &r, &policy)?;
    let workflow = crate::read_agent_workflow(
        root,
        "worker",
        settings
            .values
            .agent_workflow_body_limit()
            .map_err(|e| AgentLaunchError::caused(e.reason(), e))?,
        &settings
            .values
            .context_policy_limits()
            .map_err(|e| AgentLaunchError::caused(e.reason(), e))?,
    )
    .map_err(|e| AgentLaunchError::caused(e.code(), e))?;
    let mut tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let result = (|| {
        check_fence(fence, &tx, work, settings, environment)?;
        let outcome = stage(
            &mut tx,
            &Admission {
                root,
                dir,
                work,
                r: &r,
                raw: &raw,
                provider: p,
                provider_raw: &provider_raw,
                host: &host,
                settings,
                policy: &policy,
                workflow: &workflow,
                guard_snapshot: &guard_snapshot,
                model: &model,
                planning: chosen_work.planning.as_deref(),
                fence,
            },
            environment,
        )?;
        check_fence(fence, &tx, work, settings, environment)?;
        Ok(outcome)
    })();
    tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
    match result {
        Ok(mut outcome) if preview || (fence.is_some() && !outcome.created) => {
            finish(
                Ok(()),
                tx.rollback().map_err(|e| {
                    AgentLaunchError::caused(
                        if preview {
                            "agent-launch-preview-rollback"
                        } else {
                            "agent-launch-replay-rollback"
                        },
                        e,
                    )
                }),
            )?;
            outcome.created = false;
            outcome.preview_only = preview;
            Ok(outcome)
        }
        Ok(outcome) => match tx.commit() {
            Ok(()) => Ok(outcome),
            Err(e) => {
                let cause = AgentLaunchError::caused("agent-launch-commit", e);
                let rollback_attempted = !db.is_autocommit();
                let cleanup = if !rollback_attempted {
                    None
                } else {
                    db.execute_batch("ROLLBACK")
                        .err()
                        .map(|e| Box::new(AgentLaunchError::caused("agent-launch-rollback", e)))
                };
                Err(AgentLaunchError::Commit {
                    cause: Box::new(cause),
                    rollback_confirmed: rollback_attempted
                        && cleanup.is_none()
                        && db.is_autocommit(),
                    cleanup,
                })
            }
        },
        Err(error) => finish(
            Err(error),
            tx.rollback()
                .map_err(|e| AgentLaunchError::caused("agent-launch-rollback", e)),
        ),
    }
}

#[cfg(test)]
mod workspace_root_tests {
    use super::*;
    /// Read-only SQL guard projections with canonical path data, not a Root,
    /// authentic integration, owning process or permission. A disjoint witness
    /// precedes overlap cases on each real query branch. Actual native admission
    /// refusal/recovery has its own permanent fixture outside this minimal schema.
    #[test]
    fn canonical_root_overlap_and_sibling_boundaries_cover_both_guards() {
        let db=Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE mission_lifecycle(work_id TEXT); CREATE TABLE cockpit_controls(work_id TEXT,paused INTEGER); CREATE TABLE preparation_launch_locks(work_id TEXT,task_id TEXT,released INTEGER); CREATE TABLE agents(id TEXT,cwd TEXT,status TEXT); CREATE TABLE workspace_integrations(id TEXT,canonical_workspace TEXT,state TEXT);").unwrap();
        db.pragma_update(None,"user_version",crate::RECORDED_COMMAND_SCHEMA_VERSION).unwrap();
        let request=Launch {task_id:"task".into(),..Default::default()};
        let cases=[("/project/a","/project/b",false),("/project/a","/project/ab",false),("/project","/project/child",true),("/project/child","/project",true),("/project","/project",true),("/","/project",true),("/project","/",true),("/","/",true)];
        for integration in [false,true] {
            let (insert,update,remove,expected)=if integration {
                ("INSERT INTO workspace_integrations VALUES('fixture',?,'running')","UPDATE workspace_integrations SET canonical_workspace=?","DELETE FROM workspace_integrations","agent-launch-integration-busy")
            } else {
                ("INSERT INTO agents VALUES('fixture',?,'queued')","UPDATE agents SET cwd=?","DELETE FROM agents","agent-launch-workspace-busy")
            };
            db.execute(insert,["/project/b"]).unwrap();
            guards(&db,"work",&request,"/project/a").unwrap();
            for (stored,candidate,overlap) in cases {
                db.execute(update,[stored]).unwrap();
                let actual=guards(&db,"work",&request,candidate);
                if overlap {assert_eq!(actual.unwrap_err().code(),expected);} else {actual.unwrap();}
            }
            db.execute(remove,[]).unwrap();
            guards(&db,"work",&request,"/project/a").unwrap();
        }
    }
}

#[cfg(test)]
mod dispatch_replay_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    /// SQL projection fence only. Never used by a public Root admission or a
    /// provider; its side effects demonstrate actual rollback, not authority.
    struct ProjectionFence(AtomicUsize);
    impl crate::work_engine::MutationFence for ProjectionFence {
        fn check(&self, db: &Connection, _: &str, _: &str) -> Result<(), crate::WorkEngineError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            db.execute("INSERT INTO pulses DEFAULT VALUES", []).map_err(crate::WorkEngineError::Sql)?;
            Ok(())
        }
        fn mission_conductor(&self) -> Option<&str> { Some("owner") }
        fn stage_receipt(&self, _: &Connection, _: crate::work_engine::MutationFenceReceipt<'_>) -> Result<(), crate::WorkEngineError> { Ok(()) }
    }
    /// Minimal negative/absence SQL projection, no positive Agent/claim is seeded.
    fn database() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.pragma_update(None, "user_version", crate::RECORDED_COMMAND_SCHEMA_VERSION).unwrap();
        db.execute_batch("CREATE TABLE agents(id TEXT,request BLOB,work_id TEXT); CREATE TABLE events(id TEXT,kind TEXT,request BLOB); CREATE TABLE reservations(agent_id TEXT); CREATE TABLE pulses(seq INTEGER PRIMARY KEY);").unwrap();
        db
    }
    /// Healthy absence precedes orphan canonical-event/reservation and corrupt
    /// JSON variants. Explicit restoration returns absence; no error becomes None.
    #[test]
    fn absent_agent_does_not_hide_canonical_event_namespace_or_corruption() {
        let db = database(); db.execute_batch("BEGIN IMMEDIATE").unwrap();
        assert!(recorded_dispatch_launch(&db, "work", "operation", &mut DispatchReplayObservation::default()).unwrap().is_none());
        for (id, kind, request) in [
            ("operation", "work.create", "{}"),
            ("generated-event", "agent.start", r#"{"event_id":"operation"}"#),
            ("generated-event", "agent.start", "invalid-json"),
        ] {
            db.execute("INSERT INTO events VALUES(?,?,?)", rusqlite::params![id,kind,request]).unwrap();
            assert!(recorded_dispatch_launch(&db, "work", "operation", &mut DispatchReplayObservation::default()).is_err());
            db.execute("DELETE FROM events", []).unwrap();
            assert!(recorded_dispatch_launch(&db, "work", "operation", &mut DispatchReplayObservation::default()).unwrap().is_none());
        }
        db.execute("INSERT INTO reservations VALUES('operation')", []).unwrap();
        assert!(recorded_dispatch_launch(&db, "work", "operation", &mut DispatchReplayObservation::default()).is_err());
        db.execute("DELETE FROM reservations", []).unwrap();
        db.execute_batch("ROLLBACK").unwrap();
        assert!(recorded_dispatch_launch(&db, "work", "operation", &mut DispatchReplayObservation::default()).is_err());
    }
    /// Negative metadata projection: once observed, original bytes survive errors
    /// and cannot disappear/change into a fresh operation across storage retries.
    #[test]
    fn observed_request_bytes_cannot_be_replaced_or_disappear_after_failure() {
        let db = database(); db.execute_batch("BEGIN IMMEDIATE").unwrap();
        let mut observation = DispatchReplayObservation::default();
        assert!(recorded_dispatch_launch(&db, "work", "operation", &mut observation).unwrap().is_none());
        db.execute("INSERT INTO agents VALUES('operation',?,'work')", [b"negative-request-one".as_slice()]).unwrap();
        assert!(recorded_dispatch_launch(&db, "work", "operation", &mut observation).is_err());
        assert_eq!(observation.original.as_ref().unwrap().0, b"negative-request-one");
        db.execute("UPDATE agents SET request=?", [b"negative-request-two".as_slice()]).unwrap();
        assert!(matches!(recorded_dispatch_launch(&db, "work", "operation", &mut observation),
            Err(AgentLaunchError::Refused("agent-launch-replay-original-changed"))));
        db.execute("DELETE FROM agents", []).unwrap();
        assert!(matches!(recorded_dispatch_launch(&db, "work", "operation", &mut observation),
            Err(AgentLaunchError::Refused("agent-launch-replay-original-disappeared"))));
        db.execute_batch("ROLLBACK").unwrap();
    }
    /// Negative namespace data survives deletion across the same retry state.
    /// The flag is set inside the row consumer before query finalization; no
    /// orphan event or reservation is ever treated as positive launch authority.
    #[test]
    fn orphan_namespace_cannot_become_fresh_absence_across_retries() {
        let db = database(); db.execute_batch("BEGIN IMMEDIATE").unwrap();
        for reserved in [false, true] {
            let mut observation = DispatchReplayObservation::default();
            assert!(recorded_dispatch_launch(&db, "work", "operation", &mut observation).unwrap().is_none());
            if reserved { db.execute("INSERT INTO reservations VALUES('operation')", []).unwrap(); }
            else { db.execute("INSERT INTO events VALUES('generated','agent.start',?)", [r#"{"event_id":"operation"}"#]).unwrap(); }
            assert!(recorded_dispatch_launch(&db, "work", "operation", &mut observation).is_err());
            db.execute("DELETE FROM events", []).unwrap();
            db.execute("DELETE FROM reservations", []).unwrap();
            assert!(matches!(recorded_dispatch_launch(&db, "work", "operation", &mut observation),
                Err(AgentLaunchError::Refused("agent-launch-orphaned-operation"))));
        }
        db.execute_batch("ROLLBACK").unwrap();
    }
    /// Real IMMEDIATE/rollback around a data-only fence; both success and failure
    /// preserve the preexisting database. Missing observed original never prepares.
    #[test]
    fn replay_snapshot_rolls_back_and_observed_disappearance_never_prepares() {
        let mut db = database();
        let settings = crate::EngineSettingsDocument::initial(crate::EngineSettingsValues::installation_defaults().unwrap(),
            "fixture".into(), "2026-10-22T10:00:00Z".into()).unwrap();
        let fence = ProjectionFence(AtomicUsize::new(0));
        let result = with_fenced_replay(&mut db, "work", &fence, &settings, &mut crate::SystemEnvironment,
            |tx, _| recorded_dispatch_launch(tx, "work", "operation", &mut DispatchReplayObservation::default()));
        assert!(result.unwrap().is_none()); assert_eq!(fence.0.load(Ordering::SeqCst), 2);
        assert_eq!(db.query_row("SELECT count(*) FROM pulses", [], |r| r.get::<_,i64>(0)).unwrap(), 0);
        let directory_path = std::env::temp_dir().join(crate::generate_identifier("replay-instance-").unwrap());
        std::fs::create_dir(&directory_path).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&directory_path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let directory = crate::local_storage::LocalDirectory::open(&directory_path).unwrap();
        let launch = Launch { schema_version:1,event_id:"operation".into(),task_id:"task".into(),provider:"fixture".into(),
            role:"worker".into(),origin:"conducteur".into(),conductor_id:"owner".into(),expected_revision:1,..Default::default() };
        let result = with_fenced_replay(&mut db, "work", &fence, &settings, &mut crate::SystemEnvironment,
            |tx, env| replay_observed_dispatch(tx, &directory, "work", &launch, &fence, &settings, env));
        assert!(matches!(result, Err(AgentLaunchError::Refused("agent-launch-replay-original-disappeared"))));
        assert!(db.is_autocommit());
        assert_eq!(db.query_row("SELECT count(*) FROM pulses", [], |r| r.get::<_,i64>(0)).unwrap(), 0);
        assert_eq!(db.query_row("SELECT count(*) FROM agents", [], |r| r.get::<_,i64>(0)).unwrap(), 0);
        directory.close().unwrap();
        std::fs::remove_dir(directory_path).unwrap();
    }
}
