//! Versioned project-memory cycle ported from the retained Go
//! `internal/engine/project_memory.go` (SHA-256 16aa64873f1a26f102a9c1e26d4021425bf063ea7ff1b58c7634a423ec77070d).
//! Pure projections, SQL-row readers and IO-checked source guards ported from
//! the retained Go `internal/engine/project_memory.go` cycle: admission
//! (next-version candidate with verified RETEX source), resolution (latest
//! version with replacement refusal plus fresh-source recheck), worker context
//! attachments and active-use inventory. No operation dispatch, provider call,
//! version admission by itself or authority grant happens here; the mutation
//! engine and RootStore remain the arbiters. An inventory/read error is never
//! turned into an empty catalogue (R3).

use crate::models::{BrainstormAnswer, DialogueRef, MemoryReference, MemoryVersion, ProjectMemory, Retex, Work};
use crate::{TaskLifecycleContext, TaskLifecycleError};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::fmt;
use swarm_identity_contract::name as safe_name;

/// One active worker attempt's selected memory reference, for withdrawal and
/// replacement impact lists. Attribution mirrors Go `MemoryUse`.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MemoryUse {
    pub agent: String,
    pub work: String,
    pub task: String,
    pub status: String,
    pub reference: MemoryReference,
}

/// Versioned-memory cycle refusals. Codes are stable diagnostics; the source
/// chain keeps the underlying SQL/decode cause when one exists. No variant
/// proves or grants authority.
#[derive(Debug)]
pub enum MemoryError {
    /// Stable canonical reason, mirroring the Go message semantics.
    Refused(&'static str),
    /// Stored-row envelope/identity/revision incoherence.
    Row(crate::WorkError),
    /// A revision mismatch that a caller must resolve by re-examining.
    RevisionConflict(&'static str),
    Sql(rusqlite::Error),
    /// A required agent-row field contradicts its SQL columns.
    IdentityMismatch,
    Missing,
}
impl MemoryError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Refused(_) => "project-memory-refused",
            Self::Row(_) => "row-incoherent",
            Self::RevisionConflict(_) => "revision-conflict",
            Self::Sql(_) => "project-memory-storage",
            Self::IdentityMismatch => "project-memory-identity",
            Self::Missing => "project-memory-missing",
        }
    }
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Refused(reason)
            | Self::RevisionConflict(reason) => reason,
            Self::Row(e) => e.code(),
            Self::Sql(_) => "project-memory-sql",
            Self::IdentityMismatch => "project-memory-identity-mismatch",
            Self::Missing => "project-memory-missing",
        }
    }
}
impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.reason())
    }
}
impl std::error::Error for MemoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Row(e) => Some(e),
            Self::Sql(e) => Some(e),
            _ => None,
        }
    }
}
impl From<rusqlite::Error> for MemoryError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sql(e)
    }
}
impl From<crate::WorkError> for MemoryError {
    fn from(e: crate::WorkError) -> Self {
        Self::Row(e)
    }
}

/// Go `memoryVersionSHA`: canonical JSON of the version with its own SHA field
/// cleared. Serialization failure retains its cause; no admission follows.
pub fn memory_version_sha(v: &MemoryVersion) -> Result<String, MemoryError> {
    let mut cleared = v.clone();
    cleared.sha256 = String::new();
    let raw = crate::encode_model(&cleared).map_err(MemoryError::Row)?;
    Ok(format!("{:x}", Sha256::digest(raw)))
}

/// Go `memorySourceSHA`: RETEX identity projection binding status, nature,
/// problem/proposal/criteria/proof, task, dialogue source and the source
/// task's gate record at admission time. Pure over caller-owned data.
pub fn memory_source_sha(work: &Work, retex: &Retex) -> Result<String, MemoryError> {
    // Go gates on the source task's gate record best-effort: a missing task
    // leaves the field nil instead of refusing, matching memorySourceSHA.
    let gate = work
        .tasks
        .as_deref()
        .unwrap_or_default()
        .iter()
        .find(|t| t.id == retex.task)
        .and_then(|t| t.gate.as_deref());
    // Capital keys mirror the Go anonymous struct's exported field names; the
    // digest binds these exact wire names.
    #[derive(serde::Serialize)]
    struct Source<'a> {
        #[serde(rename = "ID")]
        id: &'a str,
        #[serde(rename = "Status")]
        status: &'a str,
        #[serde(rename = "Nature")]
        nature: &'a str,
        #[serde(rename = "Problem")]
        problem: &'a str,
        #[serde(rename = "Proposal")]
        proposal: &'a str,
        #[serde(rename = "Criteria")]
        criteria: &'a str,
        #[serde(rename = "Proof")]
        proof: &'a str,
        #[serde(rename = "Task")]
        task: &'a str,
        #[serde(rename = "Source")]
        source: &'a crate::models::DialogueRef,
        #[serde(rename = "Gate")]
        gate: Option<&'a crate::models::GateRecord>,
    }
    let raw = crate::encode_model(&Source {
        id: &retex.id,
        status: &retex.status,
        nature: &retex.nature,
        problem: &retex.problem,
        proposal: &retex.proposal,
        criteria: &retex.criteria,
        proof: &retex.proof,
        task: &retex.task,
        source: &retex.source,
        gate,
    })
    .map_err(MemoryError::Row)?;
    Ok(format!("{:x}", Sha256::digest(raw)))
}

/// Go `DialogueHit`: one brainstorm answer bound to its question text, answer
/// time and content digest. Pure projection of the caller's Work.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DialogueHit {
    pub ref_: DialogueRef,
    pub question: String,
    pub text: String,
    pub at: String,
}

/// Go `dialogueHits`: every brainstorm answer of the Work with its fallback to
/// the task response when no answers are recorded. The digest binds
/// `question + "\n" + answer.text` over the canonical encoding.
pub fn dialogue_hits(work: &Work) -> Vec<DialogueHit> {
    let mut hits = Vec::new();
    for task in work.tasks.as_deref().unwrap_or_default() {
        if !task.brainstorm {
            continue;
        }
        let answers = match task.answers.as_deref() {
            Some(answers) if !answers.is_empty() => answers.to_vec(),
            _ => vec![BrainstormAnswer {
                attempt: String::new(),
                text: task.response.clone(),
                at: String::new(),
            }],
        };
        for answer in answers {
            // Go digests the raw bytes of question + "\n" + answer text; no
            // JSON encoding wraps these bytes.
            let digest = format!(
                "{:x}",
                Sha256::digest(format!("{}\n{}", task.question, answer.text).as_bytes())
            );
            hits.push(DialogueHit {
                ref_: DialogueRef {
                    task: task.id.clone(),
                    attempt: answer.attempt.clone(),
                    sha256: digest,
                },
                question: task.question.clone(),
                text: answer.text.clone(),
                at: answer.at.clone(),
            });
        }
    }
    hits
}

/// Go `resolveDialogue`: exact `DialogueRef` match over the current hits.
/// A miss means the exchange is absent or modified in this Work.
pub fn resolve_dialogue(
    work: &Work,
    reference: &DialogueRef,
) -> Result<DialogueHit, MemoryError> {
    dialogue_hits(work)
        .into_iter()
        .find(|hit| &hit.ref_ == reference)
        .ok_or(MemoryError::Refused("dialogue-exchange-missing"))
}

/// Go `memoryEntry`: locate one memory id inside the Work. Identifiers keep
/// their identity shape; unknown ids refuse without partial catalogues.
pub fn memory_entry<'a>(work: &'a Work, id: &str) -> Result<&'a ProjectMemory, MemoryError> {
    if !safe_name(id.as_bytes()) {
        return Err(MemoryError::Refused("project-memory-id-invalid"));
    }
    work.project_memory
        .as_deref()
        .unwrap_or_default()
        .iter()
        .find(|entry| entry.id == id)
        .ok_or(MemoryError::Refused("project-memory-unknown"))
}

/// Go `findRetex` equivalent over the caller's Work; shared by admission
/// validation and withdrawal freshness rechecks.
pub fn find_retex<'a>(work: &'a Work, id: &str) -> Result<&'a Retex, MemoryError> {
    work.retex
        .as_deref()
        .unwrap_or_default()
        .iter()
        .find(|r| r.id == id)
        .ok_or(MemoryError::Refused("retex-missing"))
}

/// Go `verifiedRetex` (retex.go): the RETEX fact must point at a task that
/// exists, is APEX-accepted and whose acceptance evidence is fresh right now
/// (gate document, independent review, dependencies). Reads real originals
/// under `root`; performs no mutation, launch or acceptance.
///
/// # Errors
///
/// [`MemoryError::Refused`] with the Go message semantics when the fact is
/// unsourced, the task is unknown or its acceptance is not fresh;
/// [`MemoryError::Row`] on engine failures while computing freshness.
pub fn verified_retex(
    work: &Work,
    retex: &Retex,
    context: &TaskLifecycleContext<'_>,
    settings: Option<&crate::EngineSettingsDocument>,
) -> Result<(), MemoryError> {
    let fresh = |id: &str| -> bool {
        // Manual (non-planning) works take the hierarchy-free freshness walk;
        // planning works additionally require explicit current settings.
        let result = if work.planning.is_none() {
            crate::check_manual_task_freshness(work, id, context)
        } else {
            match settings {
                Some(settings) => {
                    crate::task_lifecycle::check_scoped_accepted_freshness(
                        work, id, context, settings,
                    )
                }
                None => Err(TaskLifecycleError::PolicyMissing),
            }
        };
        result.is_ok()
    };
    let task_ok = work
        .tasks
        .as_deref()
        .unwrap_or_default()
        .iter()
        .find(|t| t.id == retex.task)
        .is_some_and(|task| {
            task.status == "accepted"
                && crate::text::nonempty(&retex.proof)
                && fresh(&task.id)
        });
    if !task_ok {
        return Err(MemoryError::Refused(
            "project-memory-source-unfresh",
        ));
    }
    Ok(())
}

/// Go `verifiedMemorySource`: the RETEX fact passes `verified_retex`, then its
/// source binds either the acceptance receipt (fresh accepted task whose
/// recomputed `acceptance_source` matches the recorded `DialogueRef`) or a
/// dialogue exchange resolved exactly in this Work. Reads real originals; no
/// mutation, launch or acceptance follows.
///
/// # Errors
///
/// [`MemoryError::Refused`] when the source is absent or modified; the cause
/// chain of the failing branch is preserved in the refusal's provenance tests.
pub fn verified_memory_source(
    work: &Work,
    retex: &Retex,
    context: &TaskLifecycleContext<'_>,
    settings: Option<&crate::EngineSettingsDocument>,
) -> Result<(), MemoryError> {
    verified_retex(work, retex, context, settings)?;
    if retex.source_kind == "acceptance" {
        let binds = work
            .tasks
            .as_deref()
            .unwrap_or_default()
            .iter()
            .find(|t| t.id == retex.source.task)
            .is_some_and(|task| {
                task.status == "accepted"
                    && crate::root_store::cockpit_post_delivery::acceptance_source(work, task)
                        .map(|source| source == retex.source)
                        .unwrap_or(false)
            });
        if !binds {
            return Err(MemoryError::Refused(
                "project-memory-acceptance-source-changed",
            ));
        }
        return Ok(());
    }
    resolve_dialogue(work, &retex.source).map(|_| ())
}

/// Go `validateMemoryVersion`: imported/foreign-project refusal, version digest,
/// withdrawn action, then RETEX source coherence (verified status + exact
/// source digest) and the IO-performing verified-source check. The checker
/// callback (Go `s.verifiedMemorySource`) rechecks the source task's fresh
/// acceptance or the dialogue exchange against live evidence; this function
/// never touches the filesystem itself.
pub fn validate_memory_version(
    work: &Work,
    entry: &ProjectMemory,
    version: &MemoryVersion,
    project_hash: &str,
    verified_source: &dyn Fn(&Work, &Retex) -> Result<(), MemoryError>,
) -> Result<(), MemoryError> {
    if entry.imported || version.project_sha256 != project_hash {
        return Err(MemoryError::Refused("project-memory-imported-or-foreign"));
    }
    if version.sha256 != memory_version_sha(version)? {
        return Err(MemoryError::Refused("project-memory-version-altered"));
    }
    if version.action == "withdraw" {
        return Err(MemoryError::Refused("project-memory-version-withdrawn"));
    }
    let retex = find_retex(work, &version.source_retex)?;
    if retex.status != "verifie"
        || memory_source_sha(work, retex)? != version.source_sha256
    {
        return Err(MemoryError::Refused("project-memory-source-changed"));
    }
    verified_source(work, retex)
}

/// Go `resolveMemory`: read the entry's latest version, refuse a selection that
/// no longer matches it (replacement/withdrawal), then run full validation.
/// This is the withdrawal-blocks-new-uses guard (R3): a replaced version can
/// never be re-resolved by an old reference.
pub fn resolve_memory(
    work: &Work,
    reference: &MemoryReference,
    project_hash: &str,
    verified_source: &dyn Fn(&Work, &Retex) -> Result<(), MemoryError>,
) -> Result<MemoryVersion, MemoryError> {
    let entry = memory_entry(work, &reference.id)?;
    let version = entry
        .versions
        .as_deref()
        .and_then(|versions| versions.last())
        .ok_or(MemoryError::Refused("project-memory-without-version"))?;
    if reference.version != version.version || reference.sha256 != version.sha256 {
        return Err(MemoryError::RevisionConflict(
            "project-memory-version-replaced",
        ));
    }
    validate_memory_version(work, entry, version, project_hash, verified_source)?;
    Ok(version.clone())
}

/// Go `memoryAttachments`: worker-only joined context text for the exact
/// selected versions. Duplicate work/id pairs refuse; every reference resolves
/// under the current Work before any body is emitted. The text is data with an
/// explicit boundary; it carries no permission, budget or acceptance change.
pub fn memory_attachments(
    work: &Work,
    references: &[MemoryReference],
    role: &str,
    project_hash: &str,
    verified_source: &dyn Fn(&Work, &Retex) -> Result<(), MemoryError>,
) -> Result<String, MemoryError> {
    if references.is_empty() {
        return Ok(String::new());
    }
    if role != "worker" {
        return Err(MemoryError::Refused(
            "project-memory-worker-only",
        ));
    }
    let mut text = String::new();
    let mut seen = std::collections::BTreeSet::new();
    for reference in references {
        if !seen.insert(format!("{}/{}", reference.work, reference.id)) {
            return Err(MemoryError::Refused("project-memory-selected-twice"));
        }
        // Go resolves against each reference's own Work row; the caller's
        // single snapshot binds every reference to the same Work here, so a
        // foreign reference.work also refuses.
        if reference.work != work.id {
            return Err(MemoryError::Refused("project-memory-foreign-work"));
        }
        let version = resolve_memory(work, reference, project_hash, verified_source)?;
        text.push_str(&format!(
            "\nMÉMOIRE DE PROJET SÉLECTIONNÉE (donnée sourcée, sans droit supplémentaire)\nTravail : {} ; mémoire : {} ; version : {} ; SHA-256 : {}\nType : {} ; RETEX : {} ; empreinte source : {}\n{}\nConditions à revérifier : {}\nFIN DE LA MÉMOIRE : vérifier le contexte courant ; aucune acceptation, permission ou limite ne change.\n",
            reference.work,
            reference.id,
            reference.version,
            reference.sha256,
            version.kind,
            version.source_retex,
            version.source_sha256,
            version.body,
            version.conditions,
        ));
    }
    Ok(text)
}

/// Go `nextMemory`: build the next version candidate for admit/replace/withdraw.
/// Admission requires a fresh id with no expected digest; revision actions bind
/// the exact current version digest and refuse imports, foreign projects,
/// altered versions and double withdrawals. Content actions validate kind/body/
/// conditions, refuse recognizable secrets and bind the verified RETEX source;
/// the IO-performing verified-source checker runs where Go calls
/// `s.verifiedMemorySource` (after the verified-status check, before field
/// binding). Revision candidates carry the current version's content forward
/// (Go copies the latest version before editing); a withdrawal keeps
/// body/conditions/kind and only rewrites action/reason/actor fields, so the
/// history digest chain stays per-version. The candidate's `at` stays empty and
/// its digest is cleared: the caller stamps `now()` and recomputes the SHA-256
/// under the mutation transaction (Go `v.At = now(); v.SHA256 = …`).
pub fn next_memory(
    work: &Work,
    action: &str,
    request: &ProjectMemoryRequest,
    project_hash: &str,
    operator: &str,
    verified_source: &dyn Fn(&Work, &Retex) -> Result<(), MemoryError>,
) -> Result<MemoryVersion, MemoryError> {
    let mut version = MemoryVersion::default();
    let entry = memory_entry(work, &request.id);
    if action == "admit" {
        // Go only refuses when memoryEntry succeeds; any entry error (unknown
        // or invalid id) falls through to the identity precondition below,
        // which re-covers invalid ids with the admission refusal.
        if entry.is_ok() {
            return Err(MemoryError::Refused("project-memory-already-present"));
        }
        if !safe_name(request.id.as_bytes()) || !request.expected_sha256.is_empty() {
            return Err(MemoryError::Refused("project-memory-admission-invalid"));
        }
    } else {
        let entry = entry?;
        if entry.imported {
            return Err(MemoryError::Refused("project-memory-imported-create-local"));
        }
        let current = entry
            .versions
            .as_deref()
            .and_then(|versions| versions.last())
            .ok_or(MemoryError::Refused("project-memory-without-version"))?;
        if request.expected_sha256.is_empty() || request.expected_sha256 != current.sha256 {
            return Err(MemoryError::RevisionConflict(
                "project-memory-version-changed",
            ));
        }
        if current.project_sha256 != project_hash
            || current.sha256 != memory_version_sha(current)?
        {
            return Err(MemoryError::Refused("project-memory-altered-or-foreign"));
        }
        if current.action == "withdraw" && action == "withdraw" {
            return Err(MemoryError::Refused("project-memory-already-withdrawn"));
        }
        // Go copies the latest version (content included) before editing the
        // revision fields; a withdrawal candidate therefore keeps body/kind.
        version = current.clone();
    }
    version.version += 1;
    version.action = action.to_string();
    version.reason = request.reason.clone();
    version.actor = operator.to_string();
    version.at = String::new();
    if action != "withdraw" {
        if (request.kind != "experience" && request.kind != "procedure")
            || !crate::text::nonempty(&request.body)
            || !crate::text::nonempty(&request.conditions)
        {
            return Err(MemoryError::Refused(
                "project-memory-content-required",
            ));
        }
        if crate::recognizable_project_secret(&format!(
            "{}\n{}",
            request.body, request.conditions
        )) {
            return Err(MemoryError::Refused(
                "project-memory-secret-refused",
            ));
        }
        let retex = find_retex(work, &request.source_retex)?;
        if retex.status != "verifie" {
            return Err(MemoryError::Refused("project-memory-retex-unverified"));
        }
        // Go calls s.verifiedMemorySource here: the source task's fresh
        // acceptance (or the dialogue exchange) is rechecked against live
        // evidence before any binding.
        verified_source(work, retex)?;
        version.kind = request.kind.clone();
        version.body = request.body.clone();
        version.conditions = request.conditions.clone();
        version.source_retex = retex.id.clone();
        version.source_sha256 = memory_source_sha(work, retex)?;
        version.project_sha256 = project_hash.to_string();
    }
    version.sha256 = String::new();
    Ok(version)
}

/// Go `memoryPreviewToken`: bind action + request (token/event cleared) + next
/// candidate + resolved context into one digest. Context-only selection calls
/// pass a default candidate, mirroring Go.
pub fn memory_preview_token(
    action: &str,
    request: &ProjectMemoryRequest,
    next: &MemoryVersion,
    context: &str,
) -> Result<String, MemoryError> {
    let mut cleared = request.clone();
    cleared.preview_token = String::new();
    cleared.event_id = String::new();
    // Capital keys mirror the Go anonymous struct's exported field names.
    #[derive(serde::Serialize)]
    struct Token<'a> {
        #[serde(rename = "Action")]
        action: &'a str,
        #[serde(rename = "Request")]
        request: &'a ProjectMemoryRequest,
        #[serde(rename = "Next")]
        next: &'a MemoryVersion,
        #[serde(rename = "Context")]
        context: &'a str,
    }
    let raw = crate::encode_model(&Token {
        action,
        request: &cleared,
        next,
        context,
    })
    .map_err(MemoryError::Row)?;
    Ok(format!("{:x}", Sha256::digest(raw)))
}

/// Go `memoryUses`: active agent rows (queued/starting/running/stopping) with
/// their selected memory references, followed through each row's retained
/// dialogue-restore lineage. Rows decode through the shared envelope;
/// column/identity contradictions abort the complete inventory (never a
/// partial catalogue). Like Go, the row scan finishes before any lineage
/// lookup runs (single-connection readers cannot nest queries during
/// iteration). The caller owns replay of any withdrawal impact.
pub fn memory_uses(connection: &Connection) -> Result<Vec<MemoryUse>, MemoryError> {
    let agents = {
        let mut statement = connection.prepare(
            "SELECT id,work_id,task_id,status,desired,body FROM agents WHERE status IN ('queued','starting','running','stopping')",
        )?;
        let mut rows = statement.query([])?;
        let mut agents = Vec::new();
        while let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            let work: String = row.get(1)?;
            let task: String = row.get(2)?;
            let status: String = row.get(3)?;
            let desired: String = row.get(4)?;
            let body: Vec<u8> = row.get(5)?;
            let agent =
                crate::decode_agent_row(&body, &status, &desired).map_err(MemoryError::Row)?;
            if agent.id != id || agent.work_id != work || agent.task_id != task {
                return Err(MemoryError::IdentityMismatch);
            }
            agents.push(agent);
        }
        agents
    };
    let mut uses = Vec::new();
    for agent in &agents {
        for reference in agent_memory_references(connection, agent)? {
            uses.push(MemoryUse {
                agent: agent.id.clone(),
                work: agent.work_id.clone(),
                task: agent.task_id.clone(),
                status: agent.status.clone(),
                reference,
            });
        }
    }
    Ok(uses)
}

/// Go `dialogueSourceSHA` (dialogue_restore.go:53): SHA-256 of the agent's
/// canonical JSON encoding. Comparability against a stored receipt relies on
/// the shared encoder being byte-stable over a decode/re-encode of the row.
pub fn dialogue_source_sha(agent: &crate::Agent) -> Result<String, MemoryError> {
    let raw = crate::encode_model(agent).map_err(MemoryError::Row)?;
    Ok(format!("{:x}", Sha256::digest(raw)))
}

/// Go `agentMemoryReferences` (project_memory_dialogue.go): the agent's
/// selected memory references, followed through its retained dialogue-restore
/// lineage. A restored provider session keeps every ancestor's received
/// context; the walk reads those contexts without rewriting them and without
/// assuming the latest selection erased earlier knowledge.
///
/// Per visited agent, local selections refuse a duplicate work/id pair and are
/// deduplicated by full reference across ancestors (the same work/id may
/// reappear with a different version/digest and is then reported twice, like
/// Go). Each hop validates the receipt against the current agent (mode
/// `dialogue`, recorded decision, actor/at, previous binding, turn bounds in
/// `[1,20)`), then the decoded source row (identity, same work/task, dialogue
/// mode, provider/role/workspace, attributed context, retained-state digest)
/// and finally the chain bookkeeping (declared root, strictly decreasing
/// prior-turn count).
///
/// Refusals (stable codes): `project-memory-lineage-cyclic` (an agent appears
/// twice in its own chain — defense in depth, a fully validated cycle is not
/// constructible because each digest covers the next), `project-memory-selected-twice`
/// (local duplicate), `project-memory-lineage-invalid` (receipt contradicts the
/// restore contract), `project-memory-lineage-source-inaccessible` (source row
/// missing), `project-memory-lineage-source-modified` (source contradicts the
/// receipt or its digest no longer matches), `project-memory-lineage-turns-not-decreasing`,
/// `project-memory-lineage-root-mismatch`. Go wraps the source-query SQL cause
/// into the inaccessible message; here a missing row yields the refusal and
/// any other SQL failure keeps its cause through [`MemoryError::Sql`] (finer
/// diagnosis, same refusal outcome).
fn agent_memory_references(
    connection: &Connection,
    agent: &crate::Agent,
) -> Result<Vec<MemoryReference>, MemoryError> {
    let mut refs: Vec<MemoryReference> = Vec::new();
    let mut seen_agents = std::collections::HashSet::new();
    // Go passes the agent by value and rebinds it to the decoded prior row.
    let mut current = agent.clone();
    loop {
        if !seen_agents.insert(current.id.clone()) {
            return Err(MemoryError::Refused("project-memory-lineage-cyclic"));
        }
        if let Some(context) = current.context.as_deref() {
            let mut local = std::collections::HashSet::new();
            for reference in context.memory.as_deref().unwrap_or_default() {
                if !local.insert(format!("{}/{}", reference.work, reference.id)) {
                    return Err(MemoryError::Refused("project-memory-selected-twice"));
                }
                if !refs.iter().any(|seen| seen == reference) {
                    refs.push(reference.clone());
                }
            }
        }
        let receipt = match current.dialogue_restore.as_deref() {
            None => return Ok(refs),
            Some(receipt) => receipt,
        };
        if current.mode != "dialogue"
            || !receipt.decision_recorded
            || receipt.actor.is_empty()
            || receipt.at.is_empty()
            || receipt.source_agent.is_empty()
            || receipt.source_agent != current.previous
            || receipt.prior_turns < 1
            || receipt.prior_turns >= 20
        {
            return Err(MemoryError::Refused("project-memory-lineage-invalid"));
        }
        let fetched = connection
            .query_row(
                "SELECT body,status,desired FROM agents WHERE id=?",
                rusqlite::params![receipt.source_agent],
                |row| {
                    Ok((
                        row.get::<_, Vec<u8>>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    MemoryError::Refused("project-memory-lineage-source-inaccessible")
                }
                other => MemoryError::Sql(other),
            })?;
        let prior =
            crate::decode_agent_row(&fetched.0, &fetched.1, &fetched.2).map_err(MemoryError::Row)?;
        // Go compares dialogueSourceSHA(prior) to the receipt in one combined
        // condition; the digest failure and the field contradictions refuse
        // with the same code here.
        let state_matches = dialogue_source_sha(&prior)? == receipt.source_state_sha256;
        if prior.id != receipt.source_agent
            || prior.attempt_id != receipt.source_attempt
            || prior.work_id != current.work_id
            || prior.task_id != current.task_id
            || prior.mode != "dialogue"
            || prior.provider != current.provider
            || prior.role != current.role
            || prior.workspace != current.workspace
            || prior.context.is_none()
            || !state_matches
        {
            return Err(MemoryError::Refused(
                "project-memory-lineage-source-modified",
            ));
        }
        let mut root = prior.id.clone();
        if let Some(prior_receipt) = prior.dialogue_restore.as_deref() {
            root = prior_receipt.root_agent.clone();
            if prior_receipt.prior_turns >= receipt.prior_turns {
                return Err(MemoryError::Refused(
                    "project-memory-lineage-turns-not-decreasing",
                ));
            }
        }
        if root != receipt.root_agent {
            return Err(MemoryError::Refused("project-memory-lineage-root-mismatch"));
        }
        current = prior;
    }
}

/// Go `projectMemoryPreview` result: the proposed next version (default for a
/// selection), the currently latest version (replace/withdraw), the projected
/// worker context (selection), the impact list on active attempts
/// (replace/withdraw) and the token binding them all. `revision` observes the
/// Work at read time; the mutation rechecks it under the transaction.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProjectMemoryPreview {
    pub preview_token: String,
    pub preview: MemoryVersion,
    pub before: Option<MemoryVersion>,
    pub context: String,
    pub revision: i64,
    pub affected_attempts: Vec<MemoryUse>,
}

/// Go `projectMemoryPreview`: read-only proposed outcome at the Work's current
/// revision, mirroring the future mutation byte-for-byte (`next_memory` for
/// content actions; selection guards, idle SQL and projected attachments for a
/// selection). Reads run under the caller's settings reservation; no SQL write,
/// admission, actor authorization or future freshness lease occurs. Source
/// changes before apply are independently rechecked by the token and the
/// mutation path.
#[allow(clippy::too_many_arguments)]
pub fn preview(
    connection: &Connection,
    root: &std::path::Path,
    work_id: &str,
    action: &str,
    request: &ProjectMemoryRequest,
    lifecycle: &crate::TaskLifecycleContext,
    settings: Option<&crate::EngineSettingsDocument>,
) -> Result<ProjectMemoryPreview, MemoryError> {
    let work = crate::read_recorded_work(connection, work_id).map_err(|e| match e {
        crate::recorded_work::RecordedWorkError::Sql(sql) => MemoryError::Sql(sql),
        crate::recorded_work::RecordedWorkError::Work(row) => MemoryError::Row(row),
        crate::recorded_work::RecordedWorkError::Missing
        | crate::recorded_work::RecordedWorkError::Finalize { .. } => {
            MemoryError::Refused("project-memory-work-unknown")
        }
    })?;
    if work.revision != request.expected_revision {
        return Err(MemoryError::RevisionConflict(
            "project-memory-work-changed",
        ));
    }
    // Go hash([]byte(s.root)): digest of the root path string.
    let project_hash = crate::agent_launch_preflight::hash(root.to_string_lossy().as_bytes());
    let verified_source = |work: &Work, retex: &Retex| {
        verified_memory_source(work, retex, lifecycle, settings)
    };
    let mut next = MemoryVersion::default();
    let mut context = String::new();
    if action == "select" {
        // Go memorySelection: an executing future task only.
        let task = work
            .tasks
            .as_deref()
            .unwrap_or_default()
            .iter()
            .find(|t| t.id == request.task)
            .ok_or(MemoryError::Refused("project-memory-task-unknown"))?;
        if task.status != "todo" || task.plan_role == "subplanner" || task.brainstorm {
            return Err(MemoryError::Refused(
                "project-memory-selection-not-executing",
            ));
        }
        // Go instructionIdleTx against the same read connection.
        let active: i64 = connection.query_row(
            "SELECT count(*) FROM agents WHERE work_id=? AND task_id=? AND status IN ('queued','starting','running','stopping')",
            rusqlite::params![work.id, request.task],
            |row| row.get(0),
        )?;
        if active != 0 {
            return Err(MemoryError::Refused(
                "project-memory-instructions-reserved",
            ));
        }
        context = memory_attachments(
            &work,
            request.selections.as_deref().unwrap_or_default(),
            "worker",
            &project_hash,
            &verified_source,
        )?;
    } else {
        next = next_memory(
            &work,
            action,
            request,
            &project_hash,
            // Go previews call operatorIdentity() through nextMemory's shared
            // path; the actor is an observation, not an admission.
            &crate::operator_identity()
                .map_err(|_| MemoryError::Refused("project-memory-operator-unknown"))?,
            &verified_source,
        )?;
    }
    let uses = memory_uses(connection)?;
    let mut affected = Vec::new();
    if action == "replace" || action == "withdraw" {
        for use_ in &uses {
            if use_.reference.work == request.work && use_.reference.id == request.id {
                affected.push(use_.clone());
            }
        }
    }
    let mut before = None;
    if action == "replace" || action == "withdraw" {
        let entry = memory_entry(&work, &request.id)?;
        before = entry
            .versions
            .as_deref()
            .and_then(|versions| versions.last())
            .cloned();
    }
    Ok(ProjectMemoryPreview {
        preview_token: memory_preview_token(action, request, &next, &context)?,
        preview: next,
        before,
        context,
        revision: work.revision,
        affected_attempts: affected,
    })
}

/// Go `ProjectMemoryRequest`: admission/revision/selection request DTO.
/// Shape follows the retained Go JSON exactly; the wire projection and schema
/// checks belong to the dispatching caller.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ProjectMemoryRequest {
    #[serde(rename = "schema_version")]
    pub schema_version: i64,
    #[serde(rename = "event_id")]
    pub event_id: String,
    #[serde(rename = "expected_revision")]
    pub expected_revision: i64,
    #[serde(rename = "work")]
    pub work: String,
    #[serde(rename = "id")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub id: String,
    #[serde(rename = "task")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub task: String,
    #[serde(rename = "source_retex")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub source_retex: String,
    #[serde(rename = "kind")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub kind: String,
    #[serde(rename = "body")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub body: String,
    #[serde(rename = "conditions")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub conditions: String,
    #[serde(rename = "reason")]
    pub reason: String,
    #[serde(rename = "expected_sha256")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub expected_sha256: String,
    #[serde(rename = "preview_token")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub preview_token: String,
    #[serde(rename = "selections")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selections: Option<Vec<MemoryReference>>,
}

/// Go transport ceiling: the re-encoded request must stay under 1 MiB
/// (`projectMemoryTransportBytes`, `1 << 20`), mirroring Go's exact bound.
pub const PROJECT_MEMORY_TRANSPORT_BYTES: usize = 1 << 20;

/// Wire fields inherited from Go `ProjectMemoryRequest`; the strict projector
/// refuses unknown fields and invalid types. Selections bind the memory
/// reference object shape. No defaults are created beyond Go's zero values.
fn request_shape() -> crate::model_wire::Fields {
    use crate::model_wire::Kind;
    &[
        ("schema_version", Kind::Int64),
        ("event_id", Kind::String),
        ("expected_revision", Kind::Int64),
        ("work", Kind::String),
        ("id", Kind::String),
        ("task", Kind::String),
        ("source_retex", Kind::String),
        ("kind", Kind::String),
        ("body", Kind::String),
        ("conditions", Kind::String),
        ("reason", Kind::String),
        ("expected_sha256", Kind::String),
        ("preview_token", Kind::String),
        (
            "selections",
            Kind::Array(&Kind::Object(crate::shape_memory_reference)),
        ),
    ]
}

/// Go `projectMemoryOperation` head: strict decode, schema/identity/motif
/// refusal, known-action refusal, then the 1 MiB transport ceiling over the
/// re-encoded request. Validation per action (shape-only, no Work read)
/// mirrors Go's exact order: select takes references only, revision actions
/// refuse task selections, withdraw takes the expected digest and motif only.
///
/// # Errors
///
/// `MemoryError::Refused` with Go's message semantics on schema, identity,
/// unknown action, transport overflow or per-action shape violations;
/// `MemoryError::Row` retains the decode cause.
pub fn decode_project_memory_request(
    raw: &[u8],
    action: &str,
) -> Result<ProjectMemoryRequest, MemoryError> {
    let request: ProjectMemoryRequest =
        crate::work::decode_mode(raw, request_shape(), true).map_err(MemoryError::Row)?;
    if request.schema_version != 1
        || !crate::text::nonempty(&request.work)
        || !safe_name(request.work.as_bytes())
        || !crate::text::nonempty(&request.reason)
    // Go's utf8.ValidString(q.Reason) is structurally satisfied here: a Rust
    // String is valid UTF-8 by construction and the strict JSON decode refuses
    // invalid byte sequences before this point.
    {
        return Err(MemoryError::Refused(
            "project-memory-request-schema-invalid",
        ));
    }
    match action {
        "admit" | "replace" | "withdraw" | "select" => {}
        _ => return Err(MemoryError::Refused("project-memory-action-unknown")),
    }
    let encoded =
        crate::encode_model(&request).map_err(MemoryError::Row)?;
    if encoded.len() > PROJECT_MEMORY_TRANSPORT_BYTES {
        return Err(MemoryError::Refused("project-memory-transport-exceeded"));
    }
    if action == "select" {
        if !request.id.is_empty()
            || !request.source_retex.is_empty()
            || !request.kind.is_empty()
            || !request.body.is_empty()
            || !request.conditions.is_empty()
            || !request.expected_sha256.is_empty()
        {
            return Err(MemoryError::Refused(
                "project-memory-select-fields-refused",
            ));
        }
    } else {
        // Go len(q.Selections) > 0: an explicitly empty array passes decode.
        if !request.task.is_empty()
            || request
                .selections
                .as_deref()
                .is_some_and(|s| !s.is_empty())
        {
            return Err(MemoryError::Refused(
                "project-memory-selection-implied",
            ));
        }
        if action == "withdraw"
            && (!request.body.is_empty()
                || !request.conditions.is_empty()
                || !request.kind.is_empty()
                || !request.source_retex.is_empty())
        {
            return Err(MemoryError::Refused(
                "project-memory-withdraw-fields-refused",
            ));
        }
    }
    Ok(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DialogueRef;

    const PROJECT: &str = "project-hash-fixture";
    const OPERATOR: &str = "operator-fixture";

    /// One verified RETEX fact with a nonempty proof; the caller owns the
    /// fresh-acceptance gate check, so tests exercise the pure projection.
    fn retex() -> Retex {
        Retex {
            id: "retex-1".into(),
            status: "verifie".into(),
            nature: "fait".into(),
            problem: "problème".into(),
            proposal: "proposition".into(),
            criteria: "critère".into(),
            proof: "preuve".into(),
            task: "task-1".into(),
            source: DialogueRef {
                task: "task-1".into(),
                attempt: "attempt-1".into(),
                sha256: "abc".into(),
            },
            ..Default::default()
        }
    }

    fn fixture_work() -> Work {
        Work {
            schema_version: 1,
            revision: 1,
            id: "work-1".into(),
            retex: Some(vec![retex()]),
            ..Default::default()
        }
    }

    fn admit_request() -> ProjectMemoryRequest {
        ProjectMemoryRequest {
            schema_version: 1,
            reason: "motif".into(),
            work: "work-1".into(),
            id: "mem-1".into(),
            source_retex: "retex-1".into(),
            kind: "experience".into(),
            body: "contenu".into(),
            conditions: "conditions".into(),
            ..Default::default()
        }
    }

    /// Pure-projection checker: tests bind a no-op source check so the
    /// projection logic stays the subject; IO helpers are exercised below.
    fn noop<'a>() -> impl Fn(&Work, &Retex) -> Result<(), MemoryError> + 'a {
        |_, _| Ok(())
    }

    fn committed(admit: &ProjectMemoryRequest) -> (Work, MemoryVersion) {
        let mut work = fixture_work();
        let mut version = next_memory(&work, "admit", admit, PROJECT, OPERATOR, &noop()).unwrap();
        version.at = "now".into();
        version.actor = OPERATOR.into();
        version.sha256 = memory_version_sha(&version).unwrap();
        work.project_memory = Some(vec![ProjectMemory {
            id: admit.id.clone(),
            imported: false,
            versions: Some(vec![version.clone()]),
        }]);
        (work, version)
    }

    fn reference(version: &MemoryVersion) -> MemoryReference {
        MemoryReference {
            work: "work-1".into(),
            id: "mem-1".into(),
            version: version.version,
            sha256: version.sha256.clone(),
        }
    }

    /// Admission candidate: digests bind, id rules hold, unknown RETEX refuses,
    /// secrets refuse, unverified RETEX refuses, second admission refuses.
    #[test]
    fn admission_binds_source_and_refuses_duplicates_and_secrets() {
        let work = fixture_work();
        let request = admit_request();
        let candidate = next_memory(&work, "admit", &request, PROJECT, OPERATOR, &noop()).unwrap();
        assert_eq!(candidate.version, 1);
        assert_eq!(candidate.action, "admit");
        assert_eq!(candidate.source_retex, "retex-1");
        assert_eq!(candidate.project_sha256, PROJECT);
        assert_eq!(candidate.actor, OPERATOR);
        assert!(candidate.sha256.is_empty() && candidate.at.is_empty());
        let expected_source = memory_source_sha(&work, &work.retex.as_deref().unwrap()[0]).unwrap();
        assert_eq!(candidate.source_sha256, expected_source);

        // Duplicate id refuses explicit replacement hint.
        let with_memory = committed(&request).0;
        let error = next_memory(&with_memory, "admit", &request, PROJECT, OPERATOR, &noop())
            .unwrap_err();
        assert_eq!(error.reason(), "project-memory-already-present");

        // Foreign expected digest on admission refuses.
        let mut wrong = request.clone();
        wrong.expected_sha256 = "stale".into();
        assert_eq!(
            next_memory(&work, "admit", &wrong, PROJECT, OPERATOR, &noop())
                .unwrap_err()
                .reason(),
            "project-memory-admission-invalid"
        );

        // Unknown RETEX refuses.
        let mut unknown = request.clone();
        unknown.source_retex = "retex-unknown".into();
        assert_eq!(
            next_memory(&work, "admit", &unknown, PROJECT, OPERATOR, &noop())
                .unwrap_err()
                .reason(),
            "retex-missing"
        );

        // Unverified RETEX refuses.
        let mut unverified = fixture_work();
        unverified.retex.as_mut().unwrap()[0].status = "propose".into();
        assert_eq!(
            next_memory(&unverified, "admit", &request, PROJECT, OPERATOR, &noop())
                .unwrap_err()
                .reason(),
            "project-memory-retex-unverified"
        );

        // Recognizable secret refuses before any body is stored.
        let mut secret = request.clone();
        secret.body = "contact api-key sk-live-abcdef0123456789".into();
        let error = next_memory(&work, "admit", &secret, PROJECT, OPERATOR, &noop()).unwrap_err();
        assert_eq!(error.reason(), "project-memory-secret-refused");

        // Invalid kind/content refuses.
        let mut kind = request.clone();
        kind.kind = "opinion".into();
        assert_eq!(
            next_memory(&work, "admit", &kind, PROJECT, OPERATOR, &noop())
                .unwrap_err()
                .reason(),
            "project-memory-content-required"
        );
        let mut empty = request.clone();
        empty.conditions = "  ".into();
        assert_eq!(
            next_memory(&work, "admit", &empty, PROJECT, OPERATOR, &noop())
                .unwrap_err()
                .reason(),
            "project-memory-content-required"
        );

        // Invalid admission id falls through to the identity precondition,
        // matching Go's admission branch (memoryEntry error is not surfaced).
        let mut invalid = request.clone();
        invalid.id = "invalid id!".into();
        assert_eq!(
            next_memory(&work, "admit", &invalid, PROJECT, OPERATOR, &noop())
                .unwrap_err()
                .reason(),
            "project-memory-admission-invalid"
        );

        // A refusing checker propagates its own refusal at the exact Go call
        // site (after the verified-status check, before field binding).
        let refused =
            |_work: &Work, _retex: &Retex| -> Result<(), MemoryError> {
                Err(MemoryError::Refused("project-memory-source-unfresh"))
            };
        assert_eq!(
            next_memory(&work, "admit", &request, PROJECT, OPERATOR, &refused)
                .unwrap_err()
                .reason(),
            "project-memory-source-unfresh"
        );
    }

    /// Withdrawal guard (R3): the exact current version digest is required,
    /// the withdrawn action blocks resolution of new uses, history stays,
    /// and replacement refuses the stale digest.
    #[test]
    fn withdrawal_blocks_new_uses_and_replacement_binds_digest() {
        let request = admit_request();
        let (work, version) = committed(&request);

        // Resolve a fresh selection before withdrawal.
        let resolved =
            resolve_memory(&work, &reference(&version), PROJECT, &noop()).unwrap();
        assert_eq!(resolved.body, "contenu");

        // Withdrawal without the exact expected digest refuses.
        let mut wrong = request.clone();
        wrong.reason = "retrait".into();
        wrong.expected_sha256 = "not-the-version".into();
        assert_eq!(
            next_memory(&work, "withdraw", &wrong, PROJECT, OPERATOR, &noop())
                .unwrap_err()
                .reason(),
            "project-memory-version-changed"
        );

        // Correct digest produces the withdrawal candidate.
        wrong.expected_sha256 = version.sha256.clone();
        let mut withdrawn =
            next_memory(&work, "withdraw", &wrong, PROJECT, OPERATOR, &noop()).unwrap();
        assert_eq!(withdrawn.version, version.version + 1);
        assert_eq!(withdrawn.action, "withdraw");
        // Go copies the current version before rewriting the revision fields,
        // so the candidate keeps body/kind/source bindings.
        assert_eq!(withdrawn.body, "contenu");
        assert_eq!(withdrawn.kind, "experience");
        assert_eq!(withdrawn.source_retex, "retex-1");
        assert_eq!(withdrawn.project_sha256, PROJECT);

        // Commit stamping mirrors Go: timestamp then digest, in transaction.
        withdrawn.at = "2026-11-03T12:00:00Z".into();
        withdrawn.sha256 = memory_version_sha(&withdrawn).unwrap();

        // Persist the withdrawal: newest version wins; history is conserved.
        let mut withdrawn_work = work.clone();
        withdrawn_work.project_memory.as_mut().unwrap()[0]
            .versions
            .as_mut()
            .unwrap()
            .push(withdrawn);
        let error =
            resolve_memory(&withdrawn_work, &reference(&version), PROJECT, &noop())
                .unwrap_err();
        assert_eq!(error.reason(), "project-memory-version-replaced");
        // Direct validation of the latest withdrawn version refuses too.
        let latest = withdrawn_work.project_memory.as_mut().unwrap()[0]
            .versions
            .as_mut()
            .unwrap()
            .last()
            .unwrap()
            .clone();
        assert_eq!(
            validate_memory_version(
                &withdrawn_work,
                &withdrawn_work.project_memory.as_deref().unwrap()[0],
                &latest,
                PROJECT,
                &noop(),
            )
            .unwrap_err()
            .reason(),
            "project-memory-version-withdrawn"
        );
        // History remains attached (both versions stay in the entry).
        assert_eq!(
            withdrawn_work.project_memory.as_deref().unwrap()[0]
                .versions
                .as_deref()
                .unwrap()
                .len(),
            2
        );

        // Replacement must bind the current (withdrawn) version's digest.
        let mut replace = request.clone();
        replace.reason = "remplacement".into();
        replace.expected_sha256 = "stale-again".into();
        assert_eq!(
            next_memory(&withdrawn_work, "replace", &replace, PROJECT, OPERATOR, &noop())
                .unwrap_err()
                .reason(),
            "project-memory-version-changed"
        );

        // Double withdrawal refuses.
        let mut again = wrong.clone();
        again.expected_sha256 = latest.sha256.clone();
        assert_eq!(
            next_memory(&withdrawn_work, "withdraw", &again, PROJECT, OPERATOR, &noop())
                .unwrap_err()
                .reason(),
            "project-memory-already-withdrawn"
        );
    }

    /// Attachments: worker-only, duplicate and foreign refs refuse, boundary
    /// framing present, and a selection resolved before a change refuses after.
    #[test]
    fn attachments_frame_worker_context_and_refuse_stale_selection() {
        let request = admit_request();
        let (work, version) = committed(&request);
        let reference = reference(&version);

        // Non-worker roles refuse.
        assert_eq!(
            memory_attachments(&work, &[reference.clone()], "planner", PROJECT, &noop())
                .unwrap_err()
                .reason(),
            "project-memory-worker-only"
        );

        // Empty selection is empty context.
        assert!(memory_attachments(&work, &[], "worker", PROJECT, &noop())
            .unwrap()
            .is_empty());

        // Duplicate work/id refuses.
        assert_eq!(
            memory_attachments(
                &work,
                &[reference.clone(), reference.clone()],
                "worker",
                PROJECT,
                &noop()
            )
            .unwrap_err()
            .reason(),
            "project-memory-selected-twice"
        );

        // Foreign work refuses.
        let mut foreign = reference.clone();
        foreign.work = "work-2".into();
        assert_eq!(
            memory_attachments(&work, &[foreign], "worker", PROJECT, &noop())
                .unwrap_err()
                .reason(),
            "project-memory-foreign-work"
        );

        // Framed context with the boundary sentence and conditions.
        let text =
            memory_attachments(&work, &[reference.clone()], "worker", PROJECT, &noop())
                .unwrap();
        assert!(text.contains("MÉMOIRE DE PROJET SÉLECTIONNÉE"));
        assert!(text.contains("FIN DE LA MÉMOIRE"));
        assert!(text.contains("contenu"));
        assert!(text.contains("conditions"));

        // A stale reference (older version/digest) refuses with the
        // replacement conflict, never an empty catalogue.
        let mut stale = reference.clone();
        stale.version = version.version + 5;
        assert_eq!(
            memory_attachments(&work, &[stale], "worker", PROJECT, &noop())
                .unwrap_err()
                .reason(),
            "project-memory-version-replaced"
        );
    }

    /// Imported entries refuse revision actions; resolution of imported
    /// content refuses validation, matching the local-admission contract.
    #[test]
    fn imported_memory_needs_local_admission() {
        let request = admit_request();
        let (mut work, _) = committed(&request);
        work.project_memory.as_mut().unwrap()[0].imported = true;
        let mut replace = request.clone();
        replace.reason = "remplacement".into();
        replace.expected_sha256 = "any".into();
        assert_eq!(
            next_memory(&work, "replace", &replace, PROJECT, OPERATOR, &noop())
                .unwrap_err()
                .reason(),
            "project-memory-imported-create-local"
        );
    }

    /// Identity shape and unknown ids refuse distinctly; inventory errors are
    /// never turned into a successful empty catalogue. Revision actions keep
    /// the memoryEntry error (unknown id), admission falls through to the
    /// identity precondition, matching Go's branches.
    #[test]
    fn identity_and_unknown_refusals() {
        let work = fixture_work();
        let mut request = admit_request();
        request.id = "invalid id!".into();
        let error = next_memory(&work, "admit", &request, PROJECT, OPERATOR, &noop()).unwrap_err();
        assert_eq!(error.reason(), "project-memory-admission-invalid");

        request.id = "mem-2".into();
        assert_eq!(
            next_memory(&work, "replace", &request, PROJECT, OPERATOR, &noop())
                .unwrap_err()
                .reason(),
            "project-memory-unknown"
        );

        let unknown = MemoryReference {
            work: "work-1".into(),
            id: "mem-none".into(),
            version: 1,
            sha256: "x".into(),
        };
        assert_eq!(
            resolve_memory(&work, &unknown, PROJECT, &noop())
                .unwrap_err()
                .reason(),
            "project-memory-unknown"
        );
    }

    /// Version digest clears only its own field; the preview token binds
    /// action, request without token/event, candidate and context.
    #[test]
    fn version_sha_and_preview_token_binding() {
        let request = admit_request();
        let mut version =
            next_memory(&fixture_work(), "admit", &request, PROJECT, OPERATOR, &noop()).unwrap();
        version.sha256 = memory_version_sha(&version).unwrap();
        assert_eq!(memory_version_sha(&version).unwrap(), version.sha256);

        let token = memory_preview_token("admit", &request, &version, "").unwrap();
        let again = memory_preview_token("admit", &request, &version, "").unwrap();
        assert_eq!(token, again);
        // A different context binds differently.
        let with_context = memory_preview_token("select", &request, &version, "ctx").unwrap();
        assert_ne!(token, with_context);
        // The token itself and the event id are cleared before binding.
        let mut cleared = request.clone();
        cleared.preview_token = token.clone();
        cleared.event_id = "event-1".into();
        assert_eq!(
            memory_preview_token("admit", &cleared, &version, "").unwrap(),
            token
        );
    }

    /// Dialogue-restore lineage walk (R3 impact inventory): the walker reads
    /// ancestor contexts through valid receipts, deduplicates full references,
    /// and refuses invalid receipts, inaccessible or modified sources,
    /// non-decreasing turn counts and root mismatches — each without emitting
    /// a partial or empty catalogue. The cyclic guard itself carries no
    /// fixture: no cycle survives the per-hop digest and turn checks, so only
    /// refusal of the reachable breakages is asserted here.
    #[test]
    fn lineage_walk_reads_ancestors_and_refuses_broken_chains() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE agents(id TEXT PRIMARY KEY,work_id TEXT,task_id TEXT,status TEXT,desired TEXT,body BLOB)",
        )
        .unwrap();

        let memory = |version: i64, sha: &str| MemoryReference {
            work: "work-1".into(),
            id: "mem-1".into(),
            version,
            sha256: sha.into(),
        };
        // Ancestor context lists the same id at v1; the child re-lists it at
        // v2 — same work/id, full-reference dedup still reports both.
        let ancestor_selection = memory(1, "v1");
        let child_selection = memory(2, "v2");

        let ancestor = crate::Agent {
            id: "agent-root".into(),
            work_id: "work-1".into(),
            task_id: "task-1".into(),
            mode: "dialogue".into(),
            provider: "prov".into(),
            role: "worker".into(),
            workspace: "/fixture".into(),
            status: "ended".into(),
            desired: "stop".into(),
            attempt_id: "attempt-root".into(),
            context: Some(Box::new(crate::models::ContextManifest {
                memory: Some(vec![ancestor_selection.clone()]),
                ..Default::default()
            })),
            ..Default::default()
        };
        let ancestor_sha = dialogue_source_sha(&ancestor).unwrap();

        let receipt_for = |prior_turns: i64| crate::DialogueRestoreReceipt {
            source_agent: "agent-root".into(),
            source_attempt: "attempt-root".into(),
            source_state_sha256: ancestor_sha.clone(),
            root_agent: "agent-root".into(),
            prior_turns,
            decision_recorded: true,
            actor: "operator".into(),
            at: "2026-11-05T00:00:00Z".into(),
            ..Default::default()
        };

        let child = |receipt: crate::DialogueRestoreReceipt| crate::Agent {
            id: "agent-child".into(),
            work_id: "work-1".into(),
            task_id: "task-1".into(),
            mode: "dialogue".into(),
            provider: "prov".into(),
            role: "worker".into(),
            workspace: "/fixture".into(),
            status: "running".into(),
            desired: "run".into(),
            attempt_id: "attempt-child".into(),
            previous: "agent-root".into(),
            context: Some(Box::new(crate::models::ContextManifest {
                memory: Some(vec![child_selection.clone()]),
                ..Default::default()
            })),
            dialogue_restore: Some(Box::new(receipt)),
            ..Default::default()
        };

        // Columns mirror the envelope exactly: decode_agent_row overrides
        // status/desired from the columns, so any drift would break the
        // retained-state digest for the wrong reason.
        let insert = |db: &Connection, agent: &crate::Agent, status: &str| {
            let body = crate::encode_model(agent).unwrap();
            db.execute(
                "INSERT INTO agents VALUES(?1,?2,?3,?4,?5,?6)",
                rusqlite::params![agent.id, agent.work_id, agent.task_id, status, agent.desired, body],
            )
            .unwrap();
        };

        // Valid lineage: the child's uses report both its own selection and
        // the ancestor's, deduplicated by full reference. The cyclic guard
        // itself has no fixture: a validated cycle would need a SHA-256
        // fixed point (each hop's retained-state digest covers the next
        // receipt) and the strictly decreasing turn bound excludes cycles
        // independently, so — as in Go, whose "cycle" test case refuses
        // through the modified-source branch — the seen-set is termination
        // and defense insurance only.
        insert(&db, &ancestor, "ended");
        let good_child = child(receipt_for(3));
        insert(&db, &good_child, "running");
        let uses = memory_uses(&db).unwrap();
        // Only the active (running) child is inventoried; its lineage exposes
        // v2 (own) and v1 (ancestor) — same work/id, both reported.
        assert_eq!(uses.len(), 2);
        assert_eq!(uses[0].agent, "agent-child");
        assert_eq!(uses[0].reference.version, 2);
        assert_eq!(uses[1].reference.version, 1);

        // Fresh database for the remaining refusal variants.
        let db2 = Connection::open_in_memory().unwrap();
        db2.execute_batch(
            "CREATE TABLE agents(id TEXT PRIMARY KEY,work_id TEXT,task_id TEXT,status TEXT,desired TEXT,body BLOB)",
        )
        .unwrap();
        let insert2 = |agent: &crate::Agent, status: &str| {
            let body = crate::encode_model(agent).unwrap();
            db2.execute(
                "INSERT INTO agents VALUES(?1,?2,?3,?4,?5,?6)",
                rusqlite::params![agent.id, agent.work_id, agent.task_id, status, "run", body],
            )
            .unwrap();
        };

        // Invalid receipt: unrecorded decision refuses the walk.
        let mut invalid = child(receipt_for(3));
        invalid.dialogue_restore.as_mut().unwrap().decision_recorded = false;
        insert(&db2, &invalid, "running");
        assert_eq!(
            memory_uses(&db2).unwrap_err().reason(),
            "project-memory-lineage-invalid"
        );

        // Modified source: the retained-state digest no longer matches — the
        // ancestor's reply was rewritten after the receipt was recorded, while
        // every column stays faithful to the envelope.
        let db3 = Connection::open_in_memory().unwrap();
        db3.execute_batch(
            "CREATE TABLE agents(id TEXT PRIMARY KEY,work_id TEXT,task_id TEXT,status TEXT,desired TEXT,body BLOB)",
        )
        .unwrap();
        let mut tampered_ancestor = ancestor.clone();
        tampered_ancestor.reply = "modifié après coup".into();
        insert(&db3, &tampered_ancestor, "ended");
        insert(&db3, &child(receipt_for(3)), "running");
        assert_eq!(
            memory_uses(&db3).unwrap_err().reason(),
            "project-memory-lineage-source-modified"
        );

        // Missing source row refuses as inaccessible.
        let db4 = Connection::open_in_memory().unwrap();
        db4.execute_batch(
            "CREATE TABLE agents(id TEXT PRIMARY KEY,work_id TEXT,task_id TEXT,status TEXT,desired TEXT,body BLOB)",
        )
        .unwrap();
        insert(&db4, &child(receipt_for(3)), "running");
        assert_eq!(
            memory_uses(&db4).unwrap_err().reason(),
            "project-memory-lineage-source-inaccessible"
        );

        // Non-decreasing prior turns refuses: the mid receipt claims as many
        // turns as the child's own receipt.
        let db5 = Connection::open_in_memory().unwrap();
        db5.execute_batch(
            "CREATE TABLE agents(id TEXT PRIMARY KEY,work_id TEXT,task_id TEXT,status TEXT,desired TEXT,body BLOB)",
        )
        .unwrap();
        let mut mid = ancestor.clone();
        mid.id = "agent-mid".into();
        mid.attempt_id = "attempt-mid".into();
        mid.previous = "agent-root".into();
        mid.dialogue_restore = Some(Box::new(crate::DialogueRestoreReceipt {
            source_agent: "agent-root".into(),
            source_attempt: "attempt-root".into(),
            source_state_sha256: dialogue_source_sha(&ancestor).unwrap(),
            root_agent: "agent-root".into(),
            prior_turns: 5,
            ..receipt_for(3)
        }));
        insert(&db5, &mid, "ended");
        let flat_child = child(crate::DialogueRestoreReceipt {
            source_agent: "agent-mid".into(),
            source_attempt: "attempt-mid".into(),
            source_state_sha256: dialogue_source_sha(&mid).unwrap(),
            root_agent: "agent-root".into(),
            prior_turns: 5,
            ..receipt_for(3)
        });
        // The receipt must also satisfy SourceAgent == Previous, so the child's
        // previous pointer follows its source for this fixture to reach the
        // turn comparison instead of refusing as invalid earlier.
        let mut flat_child = flat_child;
        flat_child.previous = "agent-mid".into();
        insert(&db5, &flat_child, "running");
        assert_eq!(
            memory_uses(&db5).unwrap_err().reason(),
            "project-memory-lineage-turns-not-decreasing"
        );

        // Root mismatch: the mid receipt declares a different root while its
        // turn count still strictly decreases (6 > 5), so the walk reaches the
        // root comparison instead of refusing earlier on turns.
        let db6 = Connection::open_in_memory().unwrap();
        db6.execute_batch(
            "CREATE TABLE agents(id TEXT PRIMARY KEY,work_id TEXT,task_id TEXT,status TEXT,desired TEXT,body BLOB)",
        )
        .unwrap();
        let mut root_shifted = mid.clone();
        root_shifted.dialogue_restore.as_mut().unwrap().root_agent = "someone-else".into();
        insert(&db6, &root_shifted, "ended");
        let shifted_child = child(crate::DialogueRestoreReceipt {
            source_agent: "agent-mid".into(),
            source_attempt: "attempt-mid".into(),
            source_state_sha256: dialogue_source_sha(&root_shifted).unwrap(),
            root_agent: "agent-root".into(),
            prior_turns: 6,
            ..receipt_for(3)
        });
        // Same SourceAgent == Previous requirement for this fixture to reach
        // the root comparison; the turn bound (6 > 5) is strict so the walk
        // does not refuse earlier on non-decreasing turns.
        let mut shifted_child = shifted_child;
        shifted_child.previous = "agent-mid".into();
        insert(&db6, &shifted_child, "running");
        assert_eq!(
            memory_uses(&db6).unwrap_err().reason(),
            "project-memory-lineage-root-mismatch"
        );
    }

    /// Active-use inventory: rows decode, identity contradictions refuse,
    /// withdrawn-selection uses are still reported (attributed history).
    #[test]
    fn memory_uses_reads_active_rows_and_refuses_identity_mismatch() {
        use crate::models::ContextManifest;
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE agents(id TEXT PRIMARY KEY,work_id TEXT,task_id TEXT,status TEXT,desired TEXT,body BLOB)",
        )
        .unwrap();
        let agent = crate::Agent {
            id: "agent-1".into(),
            work_id: "work-1".into(),
            task_id: "task-1".into(),
            status: "running".into(),
            desired: "run".into(),
            context: Some(Box::new(ContextManifest {
                memory: Some(vec![MemoryReference {
                    work: "work-1".into(),
                    id: "mem-1".into(),
                    version: 1,
                    sha256: "abc".into(),
                }]),
                ..Default::default()
            })),
            ..Default::default()
        };
        let body = crate::encode_model(&agent).unwrap();
        db.execute(
            "INSERT INTO agents VALUES('agent-1','work-1','task-1','running','run',?)",
            [&body],
        )
        .unwrap();
        let uses = memory_uses(&db).unwrap();
        assert_eq!(uses.len(), 1);
        assert_eq!(uses[0].reference.id, "mem-1");
        assert_eq!(uses[0].status, "running");

        // SQL columns contradicting the envelope refuse; never a partial list.
        db.execute(
            "INSERT INTO agents VALUES('agent-2','OTHER','task-1','running','run',?)",
            [&body],
        )
        .unwrap();
        let error = memory_uses(&db).unwrap_err();
        assert_eq!(error.code(), "project-memory-identity");

        // A broken body refuses with its row error; not an empty catalogue.
        let db2 = Connection::open_in_memory().unwrap();
        db2.execute_batch(
            "CREATE TABLE agents(id TEXT PRIMARY KEY,work_id TEXT,task_id TEXT,status TEXT,desired TEXT,body BLOB)",
        )
        .unwrap();
        db2.execute(
            "INSERT INTO agents VALUES('agent-1','work-1','task-1','running','run',?)",
            [&b"{PRIVATE".to_vec()],
        )
        .unwrap();
        let error = memory_uses(&db2).unwrap_err();
        assert_eq!(error.code(), "row-incoherent");
        assert!(std::error::Error::source(&error).is_some());
        assert!(!format!("{error}").contains("PRIVATE"));
    }

    /// Dialogue-hit projection and exact-ref resolution: brainstorm tasks
    /// expose their answers (or the response fallback), digests bind the raw
    /// question/text bytes, and a modified reference refuses without any
    /// catalogue fallback.
    #[test]
    fn dialogue_hits_bind_answers_and_refuse_modifications() {
        use crate::models::Task;
        let mut work = fixture_work();
        work.tasks = Some(vec![Task {
            id: "task-1".into(),
            brainstorm: true,
            question: "question ?".into(),
            response: "réponse de repli".into(),
            answers: None,
            ..Default::default()
        }]);
        let hits = dialogue_hits(&work);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].text, "réponse de repli");
        assert_eq!(hits[0].question, "question ?");
        let expected = format!(
            "{:x}",
            Sha256::digest("question ?\nréponse de repli".as_bytes())
        );
        assert_eq!(hits[0].ref_.sha256, expected);

        // Exact reference resolves.
        let resolved = resolve_dialogue(&work, &hits[0].ref_).unwrap();
        assert_eq!(resolved.text, "réponse de repli");

        // A modified digest refuses.
        let mut modified = hits[0].ref_.clone();
        modified.sha256 = "tampered".into();
        assert_eq!(
            resolve_dialogue(&work, &modified)
                .unwrap_err()
                .reason(),
            "dialogue-exchange-missing"
        );

        // Non-brainstorm tasks contribute nothing.
        work.tasks.as_mut().unwrap()[0].brainstorm = false;
        assert!(dialogue_hits(&work).is_empty());
        assert_eq!(
            resolve_dialogue(&work, &hits[0].ref_).unwrap_err().reason(),
            "dialogue-exchange-missing"
        );
    }

    /// Verified-source IO helpers: `verified_retex` refuses the unsourced,
    /// unknown-task and stale-task variants; `verified_memory_source` resolves
    /// the dialogue branch through the recorded exchange. Fixtures avoid real
    /// filesystem reads by keeping the Work planning-free and answers bound.
    #[test]
    fn verified_source_helpers_refuse_unfresh_or_modified_sources() {
        use crate::models::Task;
        let lifecycle = TaskLifecycleContext {
            root: std::path::Path::new(PROJECT),
            quality_policy: None,
            task_limits: &crate::TaskDefinitionLimits::legacy_go(),
            validation_limits: &crate::ValidationPolicyLimits::legacy_go(),
        };
        let mut work = fixture_work();
        // The source task does not exist: verifiedRetex refuses.
        assert_eq!(
            verified_retex(&work, &retex(), &lifecycle, None)
                .unwrap_err()
                .reason(),
            "project-memory-source-unfresh"
        );

        // A non-accepted source task refuses the same way.
        work.tasks = Some(vec![Task {
            id: "task-1".into(),
            status: "todo".into(),
            ..Default::default()
        }]);
        assert_eq!(
            verified_retex(&work, &retex(), &lifecycle, None)
                .unwrap_err()
                .reason(),
            "project-memory-source-unfresh"
        );

        // Dialogue branch: Go runs verifiedRetex first, so a source task that
        // is not accepted-with-fresh-evidence refuses before the exchange is
        // even resolved; the tampered digest cannot leak through that gate.
        work.tasks = Some(vec![Task {
            id: "task-1".into(),
            brainstorm: true,
            question: "question ?".into(),
            response: "réponse".into(),
            ..Default::default()
        }]);
        let mut dialogue_retex = retex();
        dialogue_retex.source_kind = "dialogue".into();
        let hits = dialogue_hits(&work);
        dialogue_retex.source = hits[0].ref_.clone();
        assert_eq!(
            verified_memory_source(&work, &dialogue_retex, &lifecycle, None)
                .unwrap_err()
                .reason(),
            "project-memory-source-unfresh"
        );
        dialogue_retex.source.sha256 = "tampered".into();
        assert_eq!(
            verified_memory_source(&work, &dialogue_retex, &lifecycle, None)
                .unwrap_err()
                .reason(),
            "project-memory-source-unfresh"
        );
        // The exchange resolution itself is covered by the dialogue_hits test;
        // the dialogue branch of verified_memory_source (after a fresh
        // acceptance) is exercised by the RootStore preview recipe.
    }

    /// Strict request decode: schema/identity/motif refusal, unknown action,
    /// per-action shape refusals in Go's order, and the transport ceiling.
    #[test]
    fn decode_request_refuses_schema_shape_and_overflow() {
        let wire = serde_json::json!({
            "schema_version": 1,
            "event_id": "event-1",
            "expected_revision": 1,
            "work": "work-1",
            "reason": "motif",
        });
        let raw = crate::encode_model(&wire).unwrap();
        // Unknown action refuses before shape checks.
        assert_eq!(
            decode_project_memory_request(&raw, "compact")
                .unwrap_err()
                .reason(),
            "project-memory-action-unknown"
        );

        // Select keeps reference/motif fields only.
        decode_project_memory_request(&raw, "select").unwrap();
        let mut bad_select = wire.clone();
        bad_select["id"] = serde_json::json!("mem-1");
        let raw_bad = crate::encode_model(&bad_select).unwrap();
        assert_eq!(
            decode_project_memory_request(&raw_bad, "select")
                .unwrap_err()
                .reason(),
            "project-memory-select-fields-refused"
        );

        // Revision actions refuse implicit task selections. Go checks
        // len(q.Selections) > 0: an explicitly empty array passes decode.
        let mut with_selection = wire.clone();
        with_selection["selections"] = serde_json::json!([]);
        let raw_selection = crate::encode_model(&with_selection).unwrap();
        decode_project_memory_request(&raw_selection, "admit").unwrap();
        with_selection["selections"] = serde_json::json!([{"work": "w", "id": "m", "version": 1, "sha256": "abc"}]);
        let raw_selection = crate::encode_model(&with_selection).unwrap();
        assert_eq!(
            decode_project_memory_request(&raw_selection, "admit")
                .unwrap_err()
                .reason(),
            "project-memory-selection-implied"
        );

        // Withdraw accepts only the expected digest and motif.
        let mut withdraw = wire.clone();
        withdraw["kind"] = serde_json::json!("experience");
        let raw_withdraw = crate::encode_model(&withdraw).unwrap();
        assert_eq!(
            decode_project_memory_request(&raw_withdraw, "withdraw")
                .unwrap_err()
                .reason(),
            "project-memory-withdraw-fields-refused"
        );

        // Schema/identity/motif refusal precedes the action check.
        let mut bad_schema = wire.clone();
        bad_schema["schema_version"] = serde_json::json!(2);
        let raw_bad_schema = crate::encode_model(&bad_schema).unwrap();
        assert_eq!(
            decode_project_memory_request(&raw_bad_schema, "admit")
                .unwrap_err()
                .reason(),
            "project-memory-request-schema-invalid"
        );
        let mut bad_work = wire.clone();
        bad_work["work"] = serde_json::json!("invalid work!");
        let raw_bad_work = crate::encode_model(&bad_work).unwrap();
        assert_eq!(
            decode_project_memory_request(&raw_bad_work, "admit")
                .unwrap_err()
                .reason(),
            "project-memory-request-schema-invalid"
        );

        // Unknown fields refuse (strict shape).
        let mut unknown = wire.clone();
        unknown["unexpected"] = serde_json::json!(true);
        let raw_unknown = crate::encode_model(&unknown).unwrap();
        assert!(decode_project_memory_request(&raw_unknown, "admit").is_err());

        // Transport ceiling: a body above 1 MiB refuses on re-encode.
        let mut huge = admit_request();
        huge.work = "work-1".into();
        huge.event_id = "event-1".into();
        huge.expected_revision = 1;
        huge.body = "x".repeat(PROJECT_MEMORY_TRANSPORT_BYTES + 16);
        let raw_huge = crate::encode_model(&huge).unwrap();
        assert_eq!(
            decode_project_memory_request(&raw_huge, "admit")
                .unwrap_err()
                .reason(),
            "project-memory-transport-exceeded"
        );
    }
}
