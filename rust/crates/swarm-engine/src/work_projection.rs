//! Read models derived from actual artifacts and caller-owned SQL. No projection
//! authenticates actors, starts work or grants acceptance/a future freshness lease.
use crate::{AutomaticValidation, EvidenceControl, Task, Work};
use rusqlite::Connection;
use serde::Serialize;
use std::{collections::BTreeMap, error::Error, fmt, path::Path};

/// Explicit presentation policy. Null/all and zero/none never prune stored data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkViewLimits {
    pub longest_measured_count: Option<usize>,
    pub recent_decisions: Option<usize>,
    pub wrap_characters: Option<usize>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Presentation labels only; language never changes validation decisions.
pub enum WorkViewLanguage {
    French,
    English,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct EvidenceReportReview {
    pub state: String,
    pub attempt_id: String,
    pub reviewer: String,
    pub at: String,
    pub limits: Vec<String>,
}
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct EvidenceControls {
    pub state: String,
    pub items: Vec<EvidenceControl>,
    pub history: Vec<EvidenceControl>,
    pub longest_measured: Vec<EvidenceControl>,
}
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct EvidenceAcceptance {
    pub state: String,
    pub revision: String,
    pub at: String,
}
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct TaskEvidence {
    pub attempt_id: String,
    pub revision: i64,
    pub freshness: String,
    pub observed_at: String,
    pub report_review: EvidenceReportReview,
    pub controls: EvidenceControls,
    pub acceptance: EvidenceAcceptance,
    pub limits: Vec<String>,
}
impl fmt::Debug for TaskEvidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TaskEvidence { data: redacted }")
    }
}
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct TaskValidation {
    pub state: String,
    pub fresh: bool,
    pub blockers: Vec<String>,
    pub owner: String,
    pub next: String,
    pub evidence: TaskEvidence,
}
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct WorkValidation {
    pub state: String,
    pub label: String,
    pub validated: usize,
    pub historical: usize,
    pub stale: usize,
    pub tasks: BTreeMap<String, TaskValidation>,
}
impl fmt::Debug for WorkValidation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WorkValidation { data: redacted }")
    }
}

pub enum WorkProjectionError {
    Refused(&'static str),
    Settings(crate::EngineSettingsError),
    Review(crate::ReviewEvidenceError),
    History(crate::ControlHistoryError),
}
impl WorkProjectionError {
    /// Stable safe error family, never containing private artifact/receipt bytes.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Refused(
                "hierarchical-projection-not-ported"
                | "managed-hierarchical-projection-not-ported"
                | "managed-review-batch-protocol-not-ported"
                | "managed-review-fragment-protocol-not-ported"
                | "managed-review-platform-not-ported",
            ) => "command_not_ported",
            Self::Refused(_) => "work-projection-invalid",
            Self::Settings(e) => e.code(),
            Self::Review(e) => e.code(),
            Self::History(e) => e.code(),
        }
    }
    /// Safe discriminant for recovery; the original typed cause remains available.
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Refused(r) => r,
            Self::Settings(e) => e.reason(),
            Self::Review(e) => e.reason(),
            Self::History(e) => e.code(),
        }
    }
}
impl fmt::Display for WorkProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.reason())
    }
}
impl fmt::Debug for WorkProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for WorkProjectionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Refused(_) => None,
            Self::Settings(e) => Some(e),
            Self::Review(e) => Some(e),
            Self::History(e) => Some(e),
        }
    }
}
fn known(value: &str) -> String {
    if crate::text::trim_space(value).is_empty() {
        "unknown".into()
    } else {
        value.into()
    }
}
fn latest(task: &Task) -> String {
    task.attempts
        .as_ref()
        .and_then(|v| v.last())
        .map(|v| known(&v.id))
        .unwrap_or_else(|| "unknown".into())
}
fn unknown_control(id: &str, freshness: &str) -> EvidenceControl {
    EvidenceControl{id:known(id),attempt_id:"unknown".into(),execution:"unknown".into(),revision:"unknown".into(),candidate_sha:"unknown".into(),command:Some(vec![]),exit_code:None,started_at:"unknown".into(),finished_at:"unknown".into(),wall_duration_ms:None,cpu_duration_ms:None,cost:"unknown".into(),tokens:"unknown".into(),freshness:freshness.into(),result:"unknown".into(),limits:vec!["Aucun reçu d’exécution moteur : une citation du rapport ou un statut de gate ne démontre ni la commande ni son code de sortie.".into()]}
}
/// Preserve Go's exact result keys/IDs; malformed gate data supplies no executed controls.
fn gate_result_ids(task: &Task) -> Vec<String> {
    use crate::gate_json::{Document, Node};
    let Some(g) = task.gate.as_deref() else {
        return vec![];
    };
    let Ok(d) = Document::parse(g.document.0.get().as_bytes()) else {
        return vec![];
    };
    let Some(Node::Array(rows)) = d.get(0, "results").and_then(|i| d.nodes.get(i)) else {
        return vec![];
    };
    rows.iter()
        .filter_map(|&row| d.get(row, "id").and_then(|i| d.text(i)))
        .filter(|id| !crate::text::trim_space(id).is_empty() && id != "deliverable")
        .collect()
}
/// Project current controls using the actual gate observation; unknown and an empty
/// list never default to PASS. Historical data is supplied separately below.
struct EvidenceContext<'a> {
    db: &'a Connection,
    root: &'a Path,
    work: &'a Work,
    output_cap: Option<usize>,
    limits: &'a WorkViewLimits,
    settings: &'a crate::EngineSettingsDocument,
}
/// Build the task evidence DTO from this read's private context and derived
/// freshness. Receipt/gate claims remain distinct from executed/current facts.
/// Malformed or unavailable SQL history refuses with cause; a human-control
/// mismatch is visibly stale with a safe reason, never hidden as empty success.
fn evidence(
    c: &EvidenceContext<'_>,
    task: &Task,
    accepted: bool,
    gate_fresh: bool,
) -> Result<TaskEvidence, WorkProjectionError> {
    let EvidenceContext {
        db,
        root,
        work,
        output_cap,
        limits,
        settings,
    } = *c;
    let attempt = latest(task);
    let mut e = TaskEvidence {
        attempt_id: attempt.clone(),
        revision: work.revision,
        freshness: "unknown".into(),
        observed_at: "unknown".into(),
        report_review: EvidenceReportReview {
            state: "not_configured".into(),
            attempt_id: attempt.clone(),
            reviewer: "unknown".into(),
            at: "unknown".into(),
            limits: vec!["Aucune revue indépendante du rapport n’est configurée.".into()],
        },
        controls: EvidenceControls {
            state: "unknown".into(),
            items: vec![],
            history: vec![],
            longest_measured: vec![],
        },
        acceptance: EvidenceAcceptance {
            state: "not_accepted".into(),
            revision: "unknown".into(),
            at: "unknown".into(),
        },
        limits: vec!["Les valeurs unknown ne constituent pas un succès.".into()],
    };
    if let Some(r) = task.independent_review.as_deref() {
        e.report_review=EvidenceReportReview{state:r.state.clone(),attempt_id:known(&r.attempt),reviewer:known(&r.reviewer),at:known(if r.finished.is_empty(){&r.started}else{&r.finished}),limits:vec!["La revue porte sur le rapport et ses citations ; elle n’exécute aucun contrôle et ne vaut pas acceptation.".into()]};
    } else if let Some(reviewer) = work.planning.as_deref().and_then(|p| p.reviewer.as_deref()) {
        e.report_review = EvidenceReportReview { state: "pending".into(), attempt_id: attempt.clone(), reviewer: format!("reviewer://{}", reviewer.provider), at: "unknown".into(), limits: vec!["Vérificateur indépendant configuré pour ce travail ; aucune revue n’a encore produit de verdict pour cette tentative.".into()] };
    }
    if task.independent_review.is_some()
        && work
            .planning
            .as_deref()
            .is_some_and(|p| p.repository.is_some())
    {
        e.report_review.limits = vec!["La revue examine le candidat, les sources fournies et les reçus de contrôles ; elle n’exécute pas elle-même les commandes et ne vaut pas acceptation.".into()];
    }
    if let Some(g) = task.gate.as_deref() {
        e.observed_at = known(&g.at);
        e.freshness = if gate_fresh { "fresh" } else { "stale" }.into();
    }
    // A receipt fallback is display data only: do not inject it into Work or the
    // independent-review freshness guard, which requires the original automatic record.
    let receipt = if task.automatic_validation.is_none()
        && work
            .planning
            .as_deref()
            .is_some_and(|p| p.repository.is_some())
    {
        crate::read_managed_receipt(root, work, &task.id, settings)
            .map_err(WorkProjectionError::Review)?
    } else {
        None
    };
    let current: Option<&AutomaticValidation> =
        task.automatic_validation.as_deref().or(receipt.as_ref());
    if let Some(a) = current {
        let mut control_fresh = gate_fresh;
        if task.validation_policy.as_deref().is_some_and(|p| {
            p.mode == "human" && p.controls.as_ref().is_some_and(|c| !c.is_empty())
        }) {
            match crate::check_validation_evidence(root, task, None, output_cap) {
                Ok(_) => control_fresh = true,
                Err(error) => {
                    control_fresh = false;
                    e.limits
                        .push(format!("Contrôles non courants : {}.", error));
                }
            }
            e.freshness = if control_fresh { "fresh" } else { "stale" }.into();
        }
        e.attempt_id = known(&a.attempt_id);
        e.observed_at = known(&a.at);
        let (mut failed, mut unknown, mut passed) = (false, false, false);
        for r in a.controls.iter().flatten() {
            let (result, execution) = if r.executed {
                (if r.passed { "passed" } else { "failed" }, "executed")
            } else if r.started_at.is_empty() {
                ("unknown", "unknown")
            } else {
                ("failed", "not_executed")
            };
            match result {
                "failed" => failed = true,
                "unknown" => unknown = true,
                _ => passed = true,
            }
            let mut command = r.command.as_ref().filter(|v| !v.is_empty()).cloned();
            if command.is_none() {
                command = a
                    .policy
                    .controls
                    .iter()
                    .flatten()
                    .find(|c| c.id == r.id)
                    .and_then(|c| c.command.as_ref().filter(|v| !v.is_empty()).cloned());
            }
            let retention=output_cap.map_or_else(||"Sortie non exposée ; seule son empreinte SHA-256 est conservée. Sans plafond de rétention.".into(),|cap|format!("Sortie non exposée ; seule son empreinte SHA-256 est conservée. Plafond {cap} octets."));
            e.controls.items.push(EvidenceControl {
                id: r.id.clone(),
                attempt_id: known(&a.attempt_id),
                execution: execution.into(),
                revision: if a.revision > 0 {
                    a.revision.to_string()
                } else {
                    "unknown".into()
                },
                candidate_sha: known(&a.candidate_sha),
                command,
                exit_code: if execution == "unknown" {
                    None
                } else {
                    Some(r.exit_code)
                },
                started_at: known(&r.started_at),
                finished_at: known(&r.finished_at),
                wall_duration_ms: r.wall_duration_ms.as_deref().copied(),
                cpu_duration_ms: r.cpu_duration_ms.as_deref().copied(),
                cost: "unknown".into(),
                tokens: "unknown".into(),
                freshness: if control_fresh && (attempt == "unknown" || a.attempt_id == attempt) {
                    "fresh"
                } else {
                    "stale"
                }
                .into(),
                result: result.into(),
                limits: vec![retention],
            });
        }
        e.controls.state = if failed {
            "failed"
        } else if unknown {
            "unknown"
        } else if passed {
            "passed"
        } else {
            "unknown"
        }
        .into();
        e.limits.push(if a.revision > 0 {
            format!(
                "Contrôles exécutés sur la révision {} ; révision de lecture {}.",
                a.revision, work.revision
            )
        } else {
            "Révision d’exécution inconnue pour cet ancien reçu.".into()
        });
    } else if task.gate.is_some() {
        for id in gate_result_ids(task) {
            e.controls.items.push(unknown_control(&id, &e.freshness));
        }
        if e.controls.items.is_empty() {
            e.controls
                .items
                .push(unknown_control("unknown", &e.freshness));
        }
        e.limits.push("Gate enregistrée sans reçu moteur : les contrôles effectivement exécutés restent unknown.".into());
    } else {
        e.controls.items.push(unknown_control("unknown", "unknown"));
    }
    e.controls.history = crate::read_validation_control_history(db, &work.id, task, current)
        .map_err(WorkProjectionError::History)?;
    let mut measured: Vec<_> = e
        .controls
        .items
        .iter()
        .chain(&e.controls.history)
        .filter(|c| c.wall_duration_ms.is_some())
        .cloned()
        .collect();
    measured.sort_by_key(|entry| std::cmp::Reverse(entry.wall_duration_ms));
    if let Some(cap) = limits.longest_measured_count {
        measured.truncate(cap);
    }
    e.controls.longest_measured = measured;
    e.acceptance.state = match task.status.as_str() {
        "accepted" if accepted => "accepted",
        "accepted" => "stale",
        "waived" => "waived",
        "submitted" if gate_fresh => "pending",
        _ => "not_accepted",
    }
    .into();
    if task.status == "waived" {
        if let Some(o) = task.manual_override.as_deref() {
            e.acceptance.at = known(&o.at);
        }
    }
    Ok(e)
}

/// Derive all dependency freshness in one read-local traversal. Gate booleans
/// and review booleans come only from current module checks, never from Root callers.
/// Missing dependencies/cycles propagate false without recursion or a durable cache.
fn dependency_freshness<'a>(
    tasks: &'a [Task],
    gates: &BTreeMap<&str, bool>,
    reviews: &BTreeMap<&str, bool>,
) -> Result<BTreeMap<&'a str, bool>, WorkProjectionError> {
    let mut index = BTreeMap::new();
    for task in tasks {
        if task.id.is_empty() || index.insert(task.id.as_str(), task).is_some() {
            return Err(WorkProjectionError::Refused("task-id-required-unique"));
        }
    }
    let mut memo = BTreeMap::new();
    for task in tasks {
        if memo.contains_key(task.id.as_str()) {
            continue;
        }
        let mut visiting = std::collections::BTreeSet::new();
        let mut pending = vec![(task.id.as_str(), false)];
        while let Some((id, exit)) = pending.pop() {
            if exit {
                visiting.remove(id);
                let t = index[id];
                let fresh = memo.get(id).copied().unwrap_or(true)
                    && t.depends
                        .iter()
                        .flatten()
                        .all(|d| memo.get(d.as_str()).copied().unwrap_or(false));
                memo.insert(id, fresh);
                continue;
            }
            if memo.contains_key(id) {
                continue;
            }
            if visiting.contains(id) {
                memo.insert(id, false);
                continue;
            }
            let Some(t) = index.get(id).copied() else {
                memo.insert(id, false);
                continue;
            };
            let base = reviews.get(id).copied().unwrap_or(false)
                && match t.status.as_str() {
                    "accepted" => gates.get(id).copied().unwrap_or(false),
                    "waived" => t
                        .manual_override
                        .as_deref()
                        .is_some_and(|o| !crate::text::trim_space(&o.reason).is_empty()),
                    _ => false,
                };
            if !base {
                memo.insert(id, false);
                continue;
            }
            visiting.insert(id);
            pending.push((id, true));
            for dep in t.depends.iter().flatten().rev() {
                pending.push((dep.as_str(), false));
            }
        }
    }
    memo.retain(|id, _| index.contains_key(id));
    Ok(memo)
}
/// Derive manual, ordinary and simple managed read data from files/settings/history.
/// Hierarchical freshness requires the original current review, including waivers;
/// a receipt fallback supplies display controls only and never repairs that guard.
/// Managed batches/fragments refuse in every task state; managed Git is Linux-only.
/// The fallback requires explicit receipt policy and complete digest-bound bytes.
/// No identity, authority or acceptance follows from this read. SQL/codec
/// history failure refuses the whole observation; stale gate/control facts stay
/// visibly stale. No caller-supplied freshness flag, mutation or acceptance.
pub fn project_work_validation(
    db: &Connection,
    root: &Path,
    work: &Work,
    settings: &crate::EngineSettingsDocument,
    language: WorkViewLanguage,
) -> Result<WorkValidation, WorkProjectionError> {
    use WorkProjectionError as E;
    if work
        .planning
        .as_deref()
        .is_some_and(|p| p.repository.is_some())
    {
        #[cfg(not(target_os = "linux"))]
        return Err(E::Refused("managed-review-platform-not-ported"));
        #[cfg(target_os = "linux")]
        for review in work
            .tasks
            .iter()
            .flatten()
            .filter_map(|t| t.independent_review.as_deref())
        {
            if review.fragment_journal.is_some() {
                return Err(E::Refused("managed-review-fragment-protocol-not-ported"));
            }
            if review.batches.as_ref().is_some_and(|b| !b.is_empty()) {
                return Err(E::Refused("managed-review-batch-protocol-not-ported"));
            }
        }
    }
    let limits = settings.values.work_view_limits().map_err(E::Settings)?;
    let output = settings
        .values
        .validation_output_limit()
        .map_err(E::Settings)?;
    let quality = settings.quality_policy_snapshot().map_err(E::Settings)?;
    let tasks = work.tasks.as_deref().unwrap_or_default();
    let mut gate_freshness = BTreeMap::new();
    for task in tasks {
        if task.id.is_empty()
            || gate_freshness
                .insert(
                    task.id.as_str(),
                    task.gate.is_some() && crate::check_gate_evidence(task, root, &quality).is_ok(),
                )
                .is_some()
        {
            return Err(E::Refused("task-id-required-unique"));
        }
    }
    let mut review_freshness = BTreeMap::new();
    for task in tasks {
        let current = if work.planning.is_none() {
            true
        } else if !matches!(task.status.as_str(), "accepted" | "waived") {
            false
        } else {
            match crate::check_current_review_evidence(root, work, &task.id, settings) {
                Ok(Some(_)) => true,
                Ok(None) => return Err(E::Refused("review-observation-incomplete")),
                Err(crate::ReviewEvidenceError::Refused(_)) => false,
                Err(crate::ReviewEvidenceError::Context(e)) if e.is_stale() => false,
                Err(e) => return Err(E::Review(e)),
            }
        };
        review_freshness.insert(task.id.as_str(), current);
    }
    let freshness = dependency_freshness(tasks, &gate_freshness, &review_freshness)?;
    let accepted = freshness.values().filter(|&&f| f).count();
    let abandoned = tasks.iter().filter(|t| t.status == "abandoned").count();
    let blocked = tasks.iter().any(|t| {
        t.status == "blocked"
            || matches!(t.status.as_str(), "accepted" | "waived") && !freshness[t.id.as_str()]
    });
    let code = if !tasks.is_empty() && accepted == tasks.len() {
        if tasks.iter().any(|t| t.status == "waived") {
            "waived"
        } else {
            "validated"
        }
    } else if blocked {
        "blocked"
    } else if !tasks.is_empty() && abandoned == tasks.len() {
        "abandoned"
    } else if abandoned > 0 && accepted + abandoned == tasks.len() {
        "partial"
    } else {
        "open"
    };
    let label = match (code, language) {
        ("validated", WorkViewLanguage::French) => "VALIDÉ",
        ("waived", WorkViewLanguage::French) => "ACCEPTÉ AVEC DÉROGATION",
        ("blocked", WorkViewLanguage::French) => "BLOQUÉ",
        ("abandoned", WorkViewLanguage::French) => "ABANDONNÉ",
        ("partial", WorkViewLanguage::French) => "PARTIEL",
        ("open", WorkViewLanguage::French) => "OUVERT",
        ("validated", _) => "VALIDATED",
        ("waived", _) => "ACCEPTED WITH WAIVER",
        ("blocked", _) => "BLOCKED",
        ("abandoned", _) => "ABANDONED",
        ("partial", _) => "PARTIAL",
        _ => "OPEN",
    };
    let mut result = WorkValidation {
        state: code.into(),
        label: label.into(),
        validated: 0,
        historical: 0,
        stale: 0,
        tasks: BTreeMap::new(),
    };
    for t in tasks {
        let fresh = freshness[t.id.as_str()];
        let gate_fresh = gate_freshness[t.id.as_str()];
        let mut x = TaskValidation {
            state: t.status.clone(),
            fresh,
            blockers: vec![],
            owner: if t.owner.is_empty() {
                "Responsable à désigner".into()
            } else {
                t.owner.clone()
            },
            next: String::new(),
            evidence: evidence(
                &EvidenceContext {
                    db,
                    root,
                    work,
                    output_cap: output,
                    limits: &limits,
                    settings,
                },
                t,
                fresh,
                gate_fresh,
            )?,
        };
        if matches!(t.status.as_str(), "accepted" | "waived") {
            result.historical += 1;
            if fresh && t.status == "accepted" {
                result.validated += 1;
            }
            if !fresh {
                x.state = "stale".into();
                result.stale += 1;
            }
        }
        if let Some(g) = t.gate.as_deref().filter(|_| !gate_fresh) {
            for (name, expected) in g.evaluation.artifacts.iter().flatten() {
                match crate::local_file(root, name) {
                    Err(_) => x
                        .blockers
                        .push(format!("Preuve absente ou inaccessible : {name}")),
                    Ok(_) => {
                        if !crate::gate_evaluation::artifact_hash(root, name)
                            .is_ok_and(|(_, actual)| &actual == expected)
                        {
                            x.blockers.push(format!("Preuve modifiée : {name}"));
                        }
                    }
                }
            }
            if x.blockers.is_empty() {
                x.blockers
                    .push("Gate absente, refusée ou contrôles obligatoires non conformes".into());
            }
        } else if t.status == "accepted" && t.gate.is_none() {
            x.blockers.push("Gate manquante".into());
        }
        for id in t.depends.iter().flatten() {
            if !freshness.get(id.as_str()).copied().unwrap_or(false) {
                x.blockers
                    .push(format!("Dépendance non validée actuellement : {id}"));
            }
        }
        if !x.blockers.is_empty() {
            x.next="Rouvrir la tâche si elle est acceptée ; exécuter les contrôles affectés, soumettre un nouveau rapport et ses preuves, enregistrer la gate puis accepter après revue. Traiter les dépendances en premier.".into();
        }
        result.tasks.insert(t.id.clone(), x);
    }
    Ok(result)
}

/// Clean Unicode controls like Go (except newline/tab), expand tabs, then wrap
/// by Unicode scalar count. None disables wrapping; zero refuses to avoid an
/// unproductive loop. This is presentation only; it mutates/truncates no history.
pub fn wrap_work_text(text: &str, width: Option<usize>) -> Result<String, WorkProjectionError> {
    if width == Some(0) {
        return Err(WorkProjectionError::Refused("work-view-wrap-positive"));
    }
    let clean: String = text
        .chars()
        .filter(|&c| c == '\n' || c == '\t' || !c.is_control())
        .collect();
    let clean = clean.replace('\t', "    ");
    let Some(width) = width else { return Ok(clean) };
    let mut output = Vec::new();
    for line in clean.split('\n') {
        let chars: Vec<_> = line.chars().collect();
        let mut start = 0;
        while chars.len() - start > width {
            let mut cut = width;
            while cut > width / 2 && chars[start + cut] != ' ' {
                cut -= 1;
            }
            if cut <= width / 2 {
                cut = width;
            }
            output.push(chars[start..start + cut].iter().collect::<String>());
            start += cut;
            while start < chars.len() && chars[start] == ' ' {
                start += 1;
            }
        }
        output.push(chars[start..].iter().collect());
    }
    Ok(output.join("\n"))
}

/// Select recent OODA events from the caller's revision-ordered history. No
/// decoding, authorization, database access or retention is performed here.
pub fn recent_work_decisions(
    events: &[crate::RecordedEvent],
    count: Option<usize>,
) -> Vec<&crate::RecordedEvent> {
    let selected: Vec<_> = events.iter().filter(|e| e.kind == b"ooda").collect();
    let start = count.map_or(0, |count| selected.len().saturating_sub(count));
    selected.into_iter().skip(start).collect()
}

#[cfg(test)]
mod graph_tests {
    use super::*;
    /// Manual fixtures have no required hierarchical review (explicit true map).
    /// Compare the shared scalar guard on missing/cyclic/transitive dependencies;
    /// a separate observation after reopening a dependency must invalidate again.
    #[test]
    fn read_local_bulk_freshness_matches_scalar_guard_and_invalidates_on_next_read() {
        let mut w=crate::decode_work(br#"{"tasks":[{"id":"a","status":"waived","manual_override":{"reason":"Exception"},"depends":["b"]},{"id":"b","status":"waived","manual_override":{"reason":"Exception"}},{"id":"c","status":"waived","manual_override":{"reason":"Exception"},"depends":["missing"]},{"id":"d","status":"waived","manual_override":{"reason":"Exception"},"depends":["e"]},{"id":"e","status":"waived","manual_override":{"reason":"Exception"},"depends":["d"]},{"id":"f","status":"accepted"}]}"#).unwrap();
        let values = crate::EngineSettingsValues::installation_defaults().unwrap();
        let limits = values.mutation_limits().unwrap();
        let context = crate::TaskLifecycleContext {
            root: Path::new("."),
            quality_policy: None,
            task_limits: &limits.task,
            validation_limits: &limits.policy,
        };
        for reopen in [false, true] {
            if reopen {
                w.tasks.as_mut().unwrap()[1].status = "todo".into();
            }
            let tasks = w.tasks.as_deref().unwrap();
            let gates = tasks.iter().map(|t| (t.id.as_str(), false)).collect();
            let actual = dependency_freshness(
                tasks,
                &gates,
                &tasks.iter().map(|t| (t.id.as_str(), true)).collect(),
            )
            .unwrap();
            for t in tasks {
                assert_eq!(
                    actual[t.id.as_str()],
                    crate::check_manual_task_freshness(&w, &t.id, &context).is_ok(),
                    "{}",
                    t.id
                );
            }
            assert_eq!(actual["a"], !reopen);
            assert!(!actual["c"] && !actual["d"] && !actual["e"] && !actual["f"]);
        }
    }
    /// A long dependency chain stays iterative. Unknown IDs cannot create fresh
    /// nodes and caller-owned data is never mutated by the read-local memo.
    #[test]
    fn long_manual_chain_is_iterative_and_preserves_inputs() {
        let mut tasks = Vec::new();
        for n in 0..10000 {
            tasks.push(Task {
                id: format!("t{n}"),
                status: "waived".into(),
                manual_override: Some(Box::new(crate::ManualOverride {
                    reason: "Exception".into(),
                    ..Default::default()
                })),
                depends: if n < 9999 {
                    Some(vec![format!("t{}", n + 1)])
                } else {
                    None
                },
                ..Default::default()
            });
        }
        let gates = BTreeMap::new();
        let actual = dependency_freshness(
            &tasks,
            &gates,
            &tasks.iter().map(|t| (t.id.as_str(), true)).collect(),
        )
        .unwrap();
        assert_eq!(actual.len(), 10000);
        assert!(actual.values().all(|v| *v));
        assert_eq!(tasks[0].depends.as_ref().unwrap()[0], "t1");
        tasks[9999].depends = Some(vec!["t0".into()]);
        let cycle = dependency_freshness(
            &tasks,
            &gates,
            &tasks.iter().map(|t| (t.id.as_str(), true)).collect(),
        )
        .unwrap();
        assert!(cycle.values().all(|v| !*v));
    }
}
