//! Native manual graph batches. Definitions are data, never execution authority.
//! Preparation checks current Root-owned rights/activity; WorkEngine alone owns
//! replay, commit and publication. Historical Go graph-draft rows are unchanged.
use crate::{
    AgentLaunchError as E, DefinitionContext, EngineMutationLimits, Request, Task, Work,
    WorkEngineError as W,
};
use rusqlite::{Connection, Params, Rows};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// Editable manual definition only; no status, proof, owner, model or budget.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphTaskDefinition {
    pub id: String,
    pub title: String,
    pub deliverable: String,
    pub criteria: Vec<String>,
    pub depends: Vec<String>,
}
impl GraphTaskDefinition {
    /// Project only editable fields from an observed task. Missing collections
    /// become their semantic empty value; this grants no mutation authority.
    pub fn observed(t: &Task) -> Self {
        Self {
            id: t.id.clone(),
            title: t.title.clone(),
            deliverable: t.deliverable.clone(),
            criteria: t.criteria.clone().unwrap_or_default(),
            depends: t.depends.clone().unwrap_or_default(),
        }
    }
    /// Build the existing definition-validator input, with no injected origin,
    /// lifecycle transition or validation policy. No IO or mutation occurs.
    fn request(&self, event: &str, revision: i64) -> Request {
        Request {
            schema_version: 1,
            event_id: event.into(),
            expected_revision: revision,
            id: self.id.clone(),
            title: self.title.clone(),
            deliverable: self.deliverable.clone(),
            criteria: Some(self.criteria.clone()),
            depends: Some(self.depends.clone()),
            ..Default::default()
        }
    }
}
/// Before/after definitions. A normal batch adds or edits; deletion is reserved
/// for an engine-derived compensation of a newly added, still-unused task.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphTaskChange {
    pub before: Option<GraphTaskDefinition>,
    pub after: Option<GraphTaskDefinition>,
}
/// New native wire contract, deliberately separate from historical Go drafts.
/// Definitions are complete: title/deliverable must contain non-whitespace text;
/// an empty string never means an omitted patch field.
/// `undo_of` selects the last recorded normal batch; changes must then be empty.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphBatchRequest {
    pub schema_version: i64,
    pub event_id: String,
    pub expected_revision: i64,
    pub changes: Vec<GraphTaskChange>,
    #[serde(default)]
    pub undo_of: String,
    #[serde(default)]
    pub preview_digest: String,
}
/// Read-only validated proposal. Revision and digest must be reused at apply;
/// they confer no permission and never represent a successful publication.
#[derive(Serialize)]
pub struct GraphBatchPreview {
    pub work: String,
    pub revision: i64,
    pub changes: Vec<GraphTaskChange>,
    pub preview_digest: String,
    pub no_implicit_launch: bool,
}

fn refuse(reason: &'static str) -> W {
    W::Graph(E::Refused(reason))
}
fn encoded<T: Serialize>(value: &T) -> Result<Vec<u8>, W> {
    crate::encode_model(value).map_err(|e| W::Graph(E::caused("graph-batch-encoding", e)))
}
/// Run a parameterized read and explicitly finalize its statement, preserving
/// primary and cleanup causes through the established error wrapper matrix.
fn query<T>(
    db: &Connection,
    sql: &str,
    params: impl Params,
    read: impl FnOnce(&mut Rows<'_>) -> rusqlite::Result<T>,
) -> Result<T, W> {
    let cause = |e| E::caused("graph-batch-storage", e);
    let mut statement = db.prepare(sql).map_err(|e| W::Graph(cause(e)))?;
    let result = match statement.query(params) {
        Ok(mut rows) => read(&mut rows).map_err(cause),
        Err(e) => Err(cause(e)),
    };
    crate::agent_launch_error::finish(result, statement.finalize().map_err(cause)).map_err(W::Graph)
}

/// Decode the strict native DTO and its envelope. Unknown/duplicate fields,
/// wrong types/schema/IDs/revision, empty or ambiguous batches refuse safely.
/// Root separately applies current transport policy, rights and transactions.
pub fn decode_graph_batch(raw: &[u8]) -> Result<GraphBatchRequest, E> {
    crate::check_command_input_size(raw).map_err(|e| E::caused("graph-batch-input", e))?;
    let r: GraphBatchRequest =
        serde_json::from_slice(raw).map_err(|e| E::caused("graph-batch-input", e))?;
    if r.schema_version != 1
        || r.expected_revision < 1
        || !swarm_identity_contract::name(r.event_id.as_bytes())
        || (!r.undo_of.is_empty() && !swarm_identity_contract::name(r.undo_of.as_bytes()))
        || (r.undo_of.is_empty() == r.changes.is_empty())
    {
        return Err(E::Refused("graph-batch-envelope"));
    }
    if !r.preview_digest.is_empty()
        && (r.preview_digest.len() != 64
            || !r
                .preview_digest
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()))
    {
        return Err(E::Refused("graph-preview-digest-invalid"));
    }
    Ok(r)
}

/// Derive compensation exclusively from the exact last event at the current
/// Work revision. Never trust browser-provided inverses or revive proof data.
fn changes(db: &Connection, w: &Work, r: &GraphBatchRequest) -> Result<Vec<GraphTaskChange>, W> {
    if r.undo_of.is_empty() {
        return Ok(r.changes.clone());
    }
    let previous: Option<(String, i64, String, Vec<u8>)> = query(
        db,
        "SELECT work_id,revision,kind,request FROM main.events WHERE id=?",
        [&r.undo_of],
        |rows| {
            rows.next()?
                .map(|row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
                .transpose()
        },
    )?;
    let (work, revision, kind, raw) = previous.ok_or_else(|| refuse("graph-undo-event-missing"))?;
    if work != w.id || revision != w.revision || kind != "graph.batch" {
        return Err(refuse("graph-undo-not-last-batch"));
    }
    let previous = decode_graph_batch(&raw).map_err(W::Graph)?;
    if !previous.undo_of.is_empty() {
        return Err(refuse("graph-undo-already-compensated"));
    }
    Ok(previous
        .changes
        .into_iter()
        .rev()
        .map(|c| GraphTaskChange {
            before: c.after,
            after: c.before,
        })
        .collect())
}

/// Check the saved Go-compatible graph rights at this boundary. Revocation
/// also applies to replay; scopes constrain changed task IDs. No mutation.
fn authorize(
    db: &Connection,
    work: &str,
    actor: &str,
    applying: bool,
    ids: &BTreeSet<String>,
) -> Result<(i64, Vec<String>), W> {
    let rights:Option<(i64,i64,i64,Vec<u8>)>=query(db,"SELECT revision,prepare,apply,scopes FROM main.graph_draft_authorizations WHERE work_id=? AND actor=?",[work,actor],|rows|rows.next()?.map(|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).transpose())?;
    let result = if let Some((revision, prepare, apply, raw)) = rights {
        if revision < 1 || !matches!(prepare, 0 | 1) || !matches!(apply, 0 | 1) {
            return Err(refuse("graph-authorization-invalid"));
        }
        if (applying && apply != 1) || (!applying && prepare != 1) {
            return Err(refuse("graph-authorization-required"));
        }
        let scopes: Vec<String> = serde_json::from_slice::<Option<Vec<String>>>(&raw)
            .map_err(|e| W::Graph(E::caused("graph-authorization-invalid", e)))?
            .unwrap_or_default();
        if scopes
            .iter()
            .any(|id| !swarm_identity_contract::name(id.as_bytes()))
        {
            return Err(refuse("graph-authorization-invalid"));
        }
        if !scopes.is_empty() && ids.iter().any(|id| !scopes.contains(id)) {
            return Err(refuse("graph-authorization-scope"));
        }
        (revision, scopes)
    } else {
        if actor != crate::environment::operator_identity().map_err(W::Runtime)? {
            return Err(refuse("graph-authorization-required"));
        }
        (0, vec![])
    };
    Ok(result)
}
/// Recheck current graph rights before returning a recorded replay. An undo
/// resolves its historical source without requiring it still be the latest;
/// recorded-command identity has already established this exact replay.
pub(crate) fn authorize_replay(
    db: &Connection,
    work: &str,
    r: &GraphBatchRequest,
    actor: &str,
) -> Result<(), W> {
    let source = if r.undo_of.is_empty() {
        r.clone()
    } else {
        let raw: Option<Vec<u8>> = query(
            db,
            "SELECT request FROM main.events WHERE id=? AND work_id=? AND kind='graph.batch'",
            [&r.undo_of, work],
            |rows| rows.next()?.map(|row| row.get(0)).transpose(),
        )?;
        decode_graph_batch(&raw.ok_or_else(|| refuse("graph-undo-event-missing"))?)
            .map_err(W::Graph)?
    };
    let ids = source
        .changes
        .iter()
        .filter_map(|c| c.after.as_ref().or(c.before.as_ref()))
        .map(|t| t.id.clone())
        .collect();
    authorize(db, work, actor, true, &ids).map(|_| ())
}

/// Validate/prepare an entire graph on a private candidate. Caller holds one
/// snapshot transaction and current settings reservation. Rights rows retain
/// prepare/apply/scopes; default local operator behavior matches Go, not a remote
/// identity claim. All activity guards are rechecked at apply. No SQL writes,
/// event reservation, provider launch or revision increment occurs here.
pub(crate) fn prepare(
    db: &Connection,
    w: &Work,
    r: &GraphBatchRequest,
    limits: &EngineMutationLimits,
    actor: &str,
    applying: bool,
) -> Result<(Work, GraphBatchPreview), W> {
    if w.revision != r.expected_revision {
        return Err(W::Guard(crate::MutationGuardError::Revision {
            expected: r.expected_revision,
            observed: Some(w.revision),
        }));
    }
    if w.planning.is_some() {
        return Err(refuse("graph-batch-hierarchical-work"));
    }
    let mut identities = BTreeSet::new();
    if w.tasks.iter().flatten().any(|t| !identities.insert(&t.id)) {
        return Err(refuse("graph-recorded-task-duplicate"));
    }
    let changes = changes(db, w, r)?;
    let mut seen = BTreeSet::new();
    for c in &changes {
        let task = c
            .after
            .as_ref()
            .or(c.before.as_ref())
            .ok_or_else(|| refuse("graph-change-empty"))?;
        if !swarm_identity_contract::name(task.id.as_bytes())
            || !seen.insert(task.id.clone())
            || c.before == c.after
            || c.before.as_ref().is_some_and(|b| b.id != task.id)
        {
            return Err(refuse("graph-change-invalid"));
        }
        if r.undo_of.is_empty() && c.after.is_none() {
            return Err(refuse("graph-delete-requires-compensation"));
        }
        // The shared task updater accepts partial patches. Graph definitions
        // are complete, so reject empty fields before entering that adapter;
        // otherwise a preview/event could claim a value never actually applied.
        if let Some(after) = &c.after {
            for (value, reason) in [
                (&after.title, "task-title-blank"),
                (&after.deliverable, "task-deliverable-blank"),
            ] {
                if !crate::text::nonempty(value) {
                    return Err(W::Task(crate::TaskDefinitionError::Refused(reason)));
                }
            }
        }
        let existing = w.tasks.iter().flatten().find(|t| t.id == task.id);
        if existing.map(GraphTaskDefinition::observed) != c.before {
            return Err(refuse("graph-definition-stale"));
        }
        if existing.is_some_and(|t| {
            !matches!(t.status.as_str(), "todo" | "blocked")
                || t.independent_review
                    .as_ref()
                    .is_some_and(|v| v.state == "running")
        }) {
            return Err(refuse("graph-task-not-editable"));
        }
    }
    let (rights_revision, scopes) = authorize(db, &w.id, actor, applying, &seen)?;
    let active:i64=query(db,"SELECT count(*) FROM main.agents WHERE work_id=? AND status IN ('queued','starting','running','stopping')",[&w.id],|rows|rows.next()?.ok_or(rusqlite::Error::QueryReturnedNoRows)?.get(0))?;
    if active != 0 || w.tasks.iter().flatten().any(|t| t.status == "running") {
        return Err(refuse("graph-batch-active-work"));
    }
    #[cfg(target_os = "linux")]
    for task in w.tasks.iter().flatten() {
        crate::work_engine::require_native_task_idle(
            db,
            &w.id,
            &task.id,
            "graph-batch-active-work",
        )?
    }
    #[cfg(not(target_os = "linux"))]
    return Err(refuse("graph-batch-native-platform-unavailable"));
    let mut next = w.clone();
    for c in &changes {
        if c.before.is_none() {
            let after = c
                .after
                .as_ref()
                .ok_or_else(|| refuse("graph-change-empty"))?;
            if after.criteria.len() > limits.task.max_criteria {
                return Err(refuse("graph-criteria-invalid"));
            }
            let mut input = after.request(&r.event_id, r.expected_revision);
            input.depends = Some(vec![]);
            crate::work_definition::apply_task_addition_with_defaults(
                &mut next,
                &input,
                limits
                    .task
                    .manual_gate_phases
                    .as_ref()
                    .ok_or_else(|| refuse("task-gate-defaults-not-configured"))?,
            )
            .map_err(W::Definition)?
        }
        if c.after.is_none() {
            let before = c
                .before
                .as_ref()
                .ok_or_else(|| refuse("graph-change-empty"))?;
            let task = next
                .tasks
                .iter()
                .flatten()
                .find(|t| t.id == before.id)
                .ok_or_else(|| refuse("graph-task-missing"))?;
            if task.status != "todo"
                || task.attempts.as_ref().is_some_and(|a| !a.is_empty())
                || task.independent_review.is_some()
            {
                return Err(refuse("graph-undo-task-used"));
            }
            let count: i64 = query(
                db,
                "SELECT count(*) FROM main.agents WHERE work_id=? AND task_id=?",
                [&w.id, &before.id],
                |rows| {
                    rows.next()?
                        .ok_or(rusqlite::Error::QueryReturnedNoRows)?
                        .get(0)
                },
            )?;
            if count != 0 {
                return Err(refuse("graph-undo-task-used"));
            }
            let mut fresh = Work::default();
            let mut input = before.request(&r.event_id, r.expected_revision);
            input.depends = Some(vec![]);
            crate::work_definition::apply_task_addition_with_defaults(
                &mut fresh,
                &input,
                limits
                    .task
                    .manual_gate_phases
                    .as_ref()
                    .ok_or_else(|| refuse("task-gate-defaults-not-configured"))?,
            )
            .map_err(W::Definition)?;
            let expected = fresh
                .tasks
                .as_mut()
                .and_then(|v| v.first_mut())
                .ok_or_else(|| refuse("graph-task-missing"))?;
            expected.depends = Some(before.depends.clone());
            if encoded(task)? != encoded(expected)? {
                return Err(refuse("graph-undo-task-used"));
            }
            next.tasks
                .get_or_insert_default()
                .retain(|t| t.id != before.id)
        }
    }
    // Install the final definition shell before per-task validation so valid
    // forward references and edge swaps never fail on an intermediate graph.
    let mut shell = next.clone();
    for c in &changes {
        if let Some(after) = &c.after {
            let task = shell
                .tasks
                .iter_mut()
                .flatten()
                .find(|t| t.id == after.id)
                .ok_or_else(|| refuse("graph-task-missing"))?;
            task.title = after.title.clone();
            task.deliverable = after.deliverable.clone();
            task.criteria = Some(after.criteria.clone());
            task.depends = Some(after.depends.clone())
        }
    }
    let context = DefinitionContext {
        authorized_at: "",
        actor,
        task_limits: limits.task.clone(),
        policy_limits: limits.policy.clone(),
    };
    for c in &changes {
        if let Some(after) = &c.after {
            let task = next
                .tasks
                .as_mut()
                .and_then(|v| v.iter_mut().find(|t| t.id == after.id))
                .ok_or_else(|| refuse("graph-task-missing"))?;
            *task = crate::update_task_definition(
                &shell,
                task,
                &after.request(&r.event_id, r.expected_revision),
                &context,
            )
            .map_err(W::Task)?
        }
    }
    // Refuse any future adapter drift instead of publishing an event/preview
    // whose full definitions differ from the actual candidate. No SQL writes
    // have occurred at this boundary; this is an internal consistency failure.
    for change in &changes {
        let id = &change
            .after
            .as_ref()
            .or(change.before.as_ref())
            .ok_or_else(|| refuse("graph-change-empty"))?
            .id;
        let actual = next
            .tasks
            .iter()
            .flatten()
            .find(|task| &task.id == id)
            .map(GraphTaskDefinition::observed);
        if actual != change.after {
            return Err(refuse("graph-definition-unapplied"));
        }
    }
    if let Some(task) = next.tasks.iter().flatten().next() {
        crate::validate_task_dependency_graph(&next, task).map_err(W::Task)?
    }
    let mut canonical = r.clone();
    canonical.event_id.clear();
    canonical.preview_digest.clear();
    let digest = format!(
        "{:x}",
        Sha256::digest(encoded(&(&canonical, w, &next, rights_revision, &scopes))?)
    );
    if applying && (r.preview_digest.is_empty() || r.preview_digest != digest) {
        return Err(refuse("graph-preview-stale"));
    }
    Ok((
        next,
        GraphBatchPreview {
            work: w.id.clone(),
            revision: w.revision,
            changes,
            preview_digest: digest,
            no_implicit_launch: true,
        },
    ))
}
