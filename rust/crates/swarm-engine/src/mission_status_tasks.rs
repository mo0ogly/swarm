//! Pure Go mission-task/result projections (HEAD 553593a2ffa3823d16784a6fedc291eebf390837).
//!
//! Producers owe the actual ordered Agent history, gate/review/report evidence,
//! dependency freshness, readiness and execution observations. Unobserved facts
//! refuse when consumed; no missing observation means false, empty or permission.
//! These values are snapshots, not grants. No SQL/filesystem/clock/provider IO,
//! policy default, report search limit, delay, retry or mutation is introduced.
//! Result completion remains separate from acceptance. Pure presentation strings
//! and branch order follow result_presentation.go and mission_status.go.
//! The caller retains read/cleanup causes before invoking this module and must
//! handle Root close/publication/flush failures afterwards. No raw Debug is derived.
//!
//! Remaining producer contracts: ReviewObservation follows Go current/managed
//! review (result_presentation.go:227/275): attempt and producer identity, report
//! digest, review contract/artifacts or previous candidate must be observed before
//! Presented/NotApplicable is supplied. Readiness follows missionDispatchState
//! (mission_status.go:975); structural candidates are not that classification.
//! Prepared/execution/assist/global occupancy reads remain outside this module.
//! This increment therefore supplies pure components, not a wired full status.
//!
//! Review axes: literals are Go protocol codes/presentation, reduction identities
//! and count representation, never operator policy. Existing human/empty-text
//! branches are explicit Go outcomes, not recovery from missing observations.
//! Public/nontrivial functions document inputs, errors, effects and limitations.
//! All new errors are static RootStoreError::Refused for absent/incoherent input
//! or count representation; no typed IO cause enters here to be discarded. Tests
//! are pure controls only and do not establish real freshness or runtime parity.
use crate::mission_status_contract::{
    AttemptDiagnostic, MissionAttemptProjection, MissionPrimaryAction, MissionStatus, MissionTask,
    MissionUnderstanding, MissionWait, RecoveryPreview, ResultPresentation,
};
use crate::mission_status_insights::{
    mission_task_primary, task_understanding, MissionAgentInsight,
};
use crate::{Agent, RootStoreError as E, Task, Work};
use std::collections::BTreeSet;

/// Current evidence classification, not stored acceptance or a cached permit.
/// NotCurrent carries the real ordered blockers, possibly empty as in Go;
/// NotObserved must be resolved by the reader before a consuming branch succeeds.
#[derive(Clone)]
pub enum FreshnessObservation {
    NotObserved,
    Current,
    NotCurrent { blockers: Vec<String> },
}
/// Gate observation from current originals, independent of the stored Gate field.
#[derive(Clone, Copy)]
pub enum GateObservation {
    NotObserved,
    Absent,
    Current,
    NotCurrent,
}
/// A completed report scan with producer order, or an explicitly missing scan.
/// An observed empty result is distinct from no evidence of a scan.
#[derive(Clone)]
pub enum ReportListing {
    NotObserved,
    Observed(Vec<String>),
    /// The selected producer cannot publish reports yet (managed Git results).
    /// This is not an empty scan or evidence that no report exists.
    PublicationUnavailable,
}
/// A report attributable to this attempt, or the actual refusal explanation.
/// The pure consumer performs no mtime/skew check or search of its own.
#[derive(Clone)]
pub enum ReportAttribution {
    NotObserved,
    Attributed(String),
    Unavailable { reason: String },
}
/// Optional incomplete delivery report, after actual observation, not a fallback.
#[derive(Clone)]
pub enum IncompleteReport {
    NotObserved,
    Observed(Option<String>),
}
/// Causal review-only fields from currentReviewPresentation/managed counterpart.
/// This cannot overwrite process identity or manufacture an accepted result.
#[derive(Clone)]
pub struct ReviewPresentation {
    pub state: String,
    pub label: String,
    pub validation_state: String,
    pub reason: String,
    pub next_step: String,
    pub report_id: String,
    pub receipt_id: String,
    pub report_state: String,
}
/// Actual applicable review projection. NotApplicable requires the producer's
/// observed branch prerequisites, including Go’s passed-opinion bypass; it never
/// means a positive verdict. Presented requires actual contract/artifact checks
/// with stale causes retained by the caller. No IO is performed here.
#[derive(Clone)]
pub enum ReviewObservation {
    NotObserved,
    NotApplicable,
    /// Current controls are semantically missing/stale before first review.
    /// Preserve the report/receipt fields already observed by the result branch.
    ControlsStale { reason: String },
    Presented(ReviewPresentation),
}
/// Mandatory result inputs. Fields may remain explicitly NotObserved only when
/// Go's ordered result branch never consumes them (e.g. no report scan on fresh acceptance).
#[derive(Clone)]
pub struct ResultObservation {
    pub freshness: FreshnessObservation,
    pub gate: GateObservation,
    pub reports: ReportListing,
    pub attribution: ReportAttribution,
    pub incomplete_report: IncompleteReport,
    pub review: ReviewObservation,
}
/// One latest-first Agent plus its actual activity classification and diagnostic.
/// Active membership is read from Agent.status; observed_state is not inferred
/// from a PID or current defaults here. Environment classification must be real.
pub struct AgentObservation<'a> {
    pub agent: &'a Agent,
    pub observed_state: &'a str,
    pub fallback_diagnostic: &'a AttemptDiagnostic,
    pub environment_recovery: bool,
}
/// A direct dependency classification in Task.depends order. Unknown evidence is
/// separate from an observed stale/missing prerequisite and must not be defaulted.
#[derive(Clone, Copy)]
pub enum DependencyFreshness {
    NotObserved,
    Current,
    NotCurrent,
}
/// Actual dependency ID and freshness; missing plan entries may be NotCurrent.
#[derive(Clone)]
pub struct DependencyObservation {
    pub task_id: String,
    pub freshness: DependencyFreshness,
}
/// Read-only missionDispatchState result. None of these values grants a departure.
#[derive(Clone, Copy)]
pub enum DispatchDisposition {
    Ready,
    Waiting,
    Manual,
    Intervention,
}
/// Whether the effective profile was actually resolved, independently of readiness.
#[derive(Clone, Copy)]
pub enum ProfileObservation {
    NotObserved,
    Missing,
    Configured,
}
/// Current assist/start guard, not a user approval or an executable capability.
#[derive(Clone)]
pub enum AssistObservation {
    NotObserved,
    Allowed,
    Held { reason: String },
}
/// Actual global workspace observation; no zero count or isolation is inferred.
#[derive(Clone, Copy)]
pub enum WorkspaceObservation {
    NotObserved,
    Free,
    Occupied,
}
/// Required readiness facts from the read-only dispatcher and related guards.
/// NotObserved is explicit so the applicable branch can refuse, not guess ready.
#[derive(Clone)]
pub enum ReadinessObservation {
    NotObserved,
    Observed {
        disposition: DispatchDisposition,
        reason: String,
        profile: ProfileObservation,
        assistance: AssistObservation,
        workspace: WorkspaceObservation,
    },
}
/// Result of managed prepared-launch inspection; not a new prepared launch.
#[derive(Clone)]
pub enum PreparedLaunchObservation {
    NotObserved,
    Absent,
    Present { reason: String },
}
/// Actual executionObserved classification, not Agent membership or PID liveness.
#[derive(Clone, Copy)]
pub enum ExecutionObservation {
    NotObserved,
    Observed,
    NotPresent,
}
/// All task-specific inputs. Readers must preserve causes rather than constructing
/// absence on read error. No currentReview/dispatcher/managed read is performed here.
#[derive(Clone)]
pub struct TaskObservation {
    pub task_id: String,
    pub result: ResultObservation,
    pub dependencies: Vec<DependencyObservation>,
    pub readiness: ReadinessObservation,
    pub prepared_launch: PreparedLaunchObservation,
    pub execution: ExecutionObservation,
}
/// Complete local projection, committed to no storage. Counters reflect result
/// branches before later task overrides, matching Go; task state alone is not a count.
pub struct TaskProjection {
    pub tasks: Vec<MissionTask>,
    pub validated: i64,
    pub review: i64,
    pub running: i64,
}

/// Borrow the actual blockers; no missing freshness is treated as a passing gate.
fn blockers(value: &FreshnessObservation) -> Result<&[String], E> {
    match value {
        FreshnessObservation::Current => Ok(&[]),
        FreshnessObservation::NotCurrent { blockers } => Ok(blockers),
        FreshnessObservation::NotObserved => Err(E::Refused("mission-status-freshness-unobserved")),
    }
}
/// Read an observed list in producer order. PublicationUnavailable deliberately
/// yields no candidate; its consumer must retain report_state=unobserved. This
/// represents an unsupported producer, never an empty successful scan.
fn first_report(value: &ReportListing) -> Result<Option<&str>, E> {
    match value {
        ReportListing::NotObserved => Err(E::Refused("mission-status-reports-unobserved")),
        ReportListing::PublicationUnavailable => Ok(None),
        ReportListing::Observed(paths) => {
            if paths.iter().any(String::is_empty) {
                return Err(E::Refused("mission-status-report-empty"));
            }
            Ok(paths.first().map(String::as_str))
        }
    }
}
/// Apply a supplied review's narrow causal fields. Only Go's review result family
/// is admitted; no accepted/validated or process-state overwrite is representable.
fn apply_review(p: &mut ResultPresentation, observation: &ReviewObservation) -> Result<bool, E> {
    match observation {
        ReviewObservation::NotObserved => Err(E::Refused("mission-status-review-unobserved")),
        ReviewObservation::NotApplicable => Ok(false),
        ReviewObservation::ControlsStale { reason } => {
            p.state="review_blocked".into();p.label="Preuves à renouveler".into();
            p.validation_state="failed_or_stale".into();p.reason=reason.clone();
            p.next_step="Renouveler les preuves et rejouer les contrôles préautorisés avant de demander la revue.".into();
            Ok(true)
        }
        ReviewObservation::Presented(r) => {
            if !matches!(
                r.state.as_str(),
                "review_blocked" | "review_in_progress" | "review_awaiting_publication"
            ) {
                return Err(E::Refused("mission-status-review-result-invalid"));
            }
            p.state = r.state.clone();
            p.label = r.label.clone();
            p.validation_state = r.validation_state.clone();
            p.reason = r.reason.clone();
            p.next_step = r.next_step.clone();
            p.report_id = r.report_id.clone();
            p.receipt_id = r.receipt_id.clone();
            p.report_state = r.report_state.clone();
            Ok(true)
        }
    }
}
/// Finish all three human labels from the actual classified facts, as Go's defer.
fn finish_result(mut p: ResultPresentation) -> ResultPresentation {
    p.process_label = fact_label(&p.process_state).into();
    p.report_label = fact_label(&p.report_state).into();
    p.validation_label = fact_label(&p.validation_state).into();
    p
}
/// Go resultFactLabel vocabulary, including explicit unknown state presentation.
/// This string fallback is protocol presentation, not a success or policy fallback.
fn fact_label(state: &str) -> &'static str {
    match state {
        "pending_review" => "Revue indépendante en cours",
        "pending_publication" => "Publication non confirmée",
        "review_unvalidated" => "Avis indépendant non obtenu",
        "review_changes_requested" => "Avis indépendant défavorable ou incomplet",
        "not_started" => "Pas encore commencé",
        "absent" => "Aucun rapport soumis",
        "unknown" => "Exécution non documentée",
        "running" => "Agent actif",
        "completed" => "Processus terminé",
        "failed" => "Processus en échec",
        "interrupted" => "Processus interrompu",
        "queued" => "Démarrage en attente",
        "starting" => "Démarrage en cours",
        "stopping" => "Arrêt en cours",
        "submitted" => "Rapport soumis, contenu à vérifier",
        "fresh" => "Preuves actuelles",
        "stale" => "Preuves à renouveler",
        "waived" => "Dérogation, contrôles non validés",
        "abandoned" => "Abandonné",
        "early_unverified" => "Rapport détecté, peut-être incomplet",
        "pending_process" => "En attente de la fin du processus",
        "not_validated" => "Non validé",
        "partial_possible" => "Rapport présent, peut-être partiel",
        "missing_gate" => "Contrôles de validation non enregistrés",
        "failed_or_stale" => "Contrôles en échec ou preuves périmées",
        "ready_for_decision" => "Contrôles actuels, décision à enregistrer",
        "attributable_unsubmitted" => "Rapport détecté, à soumettre et vérifier",
        "absent_or_unattributed" => "Rapport absent ou non attribuable",
        _ => "État non confirmé",
    }
}

/// Port resultPresentation's ordered branches without IO. The Agent must be the
/// latest applicable row selected from the supplied history. Freshness, review,
/// reports and gate state are actual observations, never authoritative by being
/// supplied here. Missing consumed evidence refuses; serialization grants nothing.
pub fn result_presentation(
    task: &Task,
    agent: Option<&AgentObservation<'_>>,
    facts: &ResultObservation,
) -> Result<ResultPresentation, E> {
    if agent.is_some_and(|a| a.agent.task_id != task.id || a.observed_state.is_empty()) {
        return Err(E::Refused("mission-status-agent-observation-mismatch"));
    }
    let mut p = ResultPresentation {
        process_label: String::new(),
        report_label: String::new(),
        validation_label: String::new(),
        state: "not_started".into(),
        label: "Pas encore commencé".into(),
        process_state: "not_started".into(),
        report_state: if matches!(facts.reports, ReportListing::PublicationUnavailable) {
            "unobserved".into()
        } else { "absent".into() },
        validation_state: "not_started".into(),
        reason: "Aucune tentative terminée ni validation actuelle n’est établie.".into(),
        next_step: "Lancer la tâche lorsque ses conditions sont réunies.".into(),
        attempt_id: String::new(),
        report_id: String::new(),
        gate_id: String::new(),
        receipt_id: String::new(),
    };
    if let Some(a) = agent {
        p.attempt_id = if a.agent.attempt_id.is_empty() {
            a.agent.id.clone()
        } else {
            a.agent.attempt_id.clone()
        };
        p.process_state = a.observed_state.into();
    }
    if let Some(r) = task
        .revalidation
        .as_deref()
        .map(|r| crate::text::trim_space(&r.report))
        .filter(|r| !r.is_empty())
    {
        p.report_id = r.into();
    } else if let Some(at) = task.next.rfind("handoff : ") {
        p.report_id = crate::text::trim_space(&task.next[at + "handoff : ".len()..]).into();
    }
    if !p.report_id.is_empty() || matches!(task.status.as_str(), "submitted" | "accepted") {
        p.report_state = "submitted".into();
    }
    if let Some(g) = &task.gate {
        p.gate_id = if crate::text::trim_space(&g.name).is_empty() {
            g.evaluation.scope_id.clone()
        } else {
            crate::text::trim_space(&g.name).into()
        };
    }
    if let Some(v) = &task.automatic_validation {
        p.receipt_id = v.receipt.clone();
    }
    if task.status == "accepted" {
        match &facts.freshness {
            FreshnessObservation::NotObserved => {
                return Err(E::Refused("mission-status-freshness-unobserved"))
            }
            FreshnessObservation::Current => {
                if !matches!(facts.gate, GateObservation::Current) {
                    return Err(E::Refused("mission-status-current-gate-unobserved"));
                }
                p.state = "validated".into();
                p.label = "Validé".into();
                p.validation_state = "fresh".into();
                p.reason = "Gate delivery et preuves actuelles ; la tâche est acceptée.".into();
                p.next_step = "Consulter le résultat ; revalider si une preuve change.".into();
            }
            FreshnessObservation::NotCurrent { blockers } => {
                p.state = "validation_stale".into();
                p.label = "Validation à refaire".into();
                p.validation_state = "stale".into();
                p.reason = blockers.first().cloned().unwrap_or_else(|| {
                    "L’acceptation historique n’est plus fondée sur des preuves actuelles.".into()
                });
                p.next_step="Rouvrir la tâche, renouveler les preuves et enregistrer une nouvelle revue de validation.".into();
            }
        }
        return Ok(finish_result(p));
    }
    if task.status == "waived" {
        p.state = "waived".into();
        p.label = "Dérogation — non validé".into();
        p.validation_state = "waived".into();
        p.reason =
            "Une décision explicite permet la suite sans transformer les contrôles en PASS.".into();
        p.next_step = "Consulter le motif et la portée de la dérogation.".into();
        return Ok(finish_result(p));
    }
    if task.status == "abandoned" {
        p.state = "abandoned".into();
        p.label = "Abandonné — non validé".into();
        p.validation_state = "abandoned".into();
        p.reason = "La tâche est abandonnée ; aucun résultat validé n’est revendiqué.".into();
        p.next_step = "Consulter la décision d’abandon.".into();
        return Ok(finish_result(p));
    }
    if let Some(a) = agent {
        if a.agent.is_active() {
            if p.process_state == "running" {
                p.state = "in_progress".into();
                p.label = "En cours".into();
                p.reason =
                    "Le processus est actif ; aucun résultat final n’est encore affirmé.".into();
                p.next_step = "Voir la session ou attendre la fin de la tentative.".into();
            } else {
                p.state = "execution_unconfirmed".into();
                p.label = "État à vérifier".into();
                p.reason =
                    "La tentative est enregistrée, mais son exécution n’est pas confirmée.".into();
                p.next_step = "Réconcilier la tentative avant toute relance.".into();
            }
            if let Some(report) = first_report(&facts.reports)? {
                p.report_state = "early_unverified".into();
                p.report_id = report.into();
                p.reason
                    .push_str(" Un rapport existe déjà, mais il peut être précoce ou incomplet.");
            }
            p.validation_state = "pending_process".into();
            return Ok(finish_result(p));
        }
        if matches!(a.agent.status.as_str(), "failed" | "interrupted") {
            if apply_review(&mut p, &facts.review)? {
                return Ok(finish_result(p));
            }
            p.state = "stopped_early".into();
            p.label = "Arrêté avant la fin".into();
            p.validation_state = "not_validated".into();
            p.reason = crate::text::trim_space(&a.agent.activity).into();
            if p.reason.is_empty() {
                p.reason = if a.agent.status == "failed" {
                    "Le processus s’est terminé en échec."
                } else {
                    "Le processus a été interrompu."
                }
                .into();
            }
            if let Some(report) = first_report(&facts.reports)? {
                if p.report_state == "absent" {
                    p.report_state = "partial_possible".into();
                    p.report_id = report.into();
                    p.reason.push_str(" Un rapport existe, mais l’arrêt ne permet pas d’en affirmer la complétude.");
                }
            }
            p.next_step="Examiner le motif, les traces et le rapport éventuel avant de décider d’une reprise.".into();
            return Ok(finish_result(p));
        }
        if a.agent.status == "completed"
            && task.status == "blocked"
            && task.blocker.starts_with("Livraison incomplète :")
        {
            p.state = "delivery_incomplete".into();
            p.label = "Résultat à compléter".into();
            p.validation_state = "not_validated".into();
            p.reason = task.blocker.clone();
            p.next_step="Le responsable doit examiner les critères manquants et préparer une correction dans les limites autorisées.".into();
            match &facts.incomplete_report {
                IncompleteReport::NotObserved => {
                    return Err(E::Refused("mission-status-incomplete-report-unobserved"))
                }
                IncompleteReport::Observed(Some(report)) if !report.is_empty() => {
                    p.report_id = report.clone();
                    p.report_state = "partial_possible".into();
                }
                IncompleteReport::Observed(_) => {}
            }
            return Ok(finish_result(p));
        }
        if a.agent.status == "completed" && apply_review(&mut p, &facts.review)? {
            return Ok(finish_result(p));
        }
    }
    if task.status == "submitted" {
        if agent.is_none() {
            p.process_state = "unknown".into();
        }
        p.validation_state = "missing_gate".into();
        p.state = "result_to_review".into();
        p.label = "Résultat à vérifier".into();
        p.reason="Un rapport est soumis ; sa complétude et les critères restent à examiner. Aucune revue de validation n’est enregistrée.".into();
        p.next_step =
            "Lire le rapport, vérifier les critères et enregistrer la revue de validation.".into();
        if task.gate.is_some() {
            let valid = match facts.gate {
                GateObservation::Current => true,
                GateObservation::NotCurrent => false,
                _ => return Err(E::Refused("mission-status-gate-unobserved")),
            };
            let blockers = blockers(&facts.freshness)?;
            if !valid || !blockers.is_empty() {
                p.state = "validation_withheld".into();
                p.label = "Validation retenue".into();
                p.validation_state = "failed_or_stale".into();
                p.reason = blockers.first().cloned().unwrap_or_else(|| {
                    "La revue de validation n’autorise pas une validation actuelle.".into()
                });
                p.next_step="Corriger ou renouveler les preuves, puis enregistrer une revue de validation actuelle.".into();
            } else {
                p.validation_state = "ready_for_decision".into();
                p.reason="La revue de validation et ses preuves sont actuelles ; la décision d’acceptation reste à enregistrer.".into();
                p.next_step =
                    "Examiner le rapport puis enregistrer la décision de validation.".into();
            }
        }
        return Ok(finish_result(p));
    }
    if agent.is_some_and(|a| a.agent.status == "completed") {
        p.process_state = "completed".into();
        p.validation_state = "not_started".into();
        match &facts.attribution {
            ReportAttribution::NotObserved => {
                return Err(E::Refused("mission-status-report-attribution-unobserved"))
            }
            ReportAttribution::Attributed(report) => {
                if report.is_empty() {
                    return Err(E::Refused("mission-status-report-empty"));
                }
                p.state = "result_to_review".into();
                p.label = "Résultat à vérifier".into();
                p.report_state = "attributable_unsubmitted".into();
                p.report_id = report.clone();
                p.reason="Le processus est terminé et un rapport lui est attribuable ; sa complétude n’est pas démontrée.".into();
                p.next_step="Soumettre le rapport, vérifier les critères et enregistrer la revue de validation.".into();
            }
            ReportAttribution::Unavailable { reason } => {
                p.state = "completed_unproven".into();
                p.label = "Terminé, résultat non démontré".into();
                p.report_state = "absent_or_unattributed".into();
                p.reason = format!("Le processus est terminé normalement, mais {reason}.");
                if let Some(a) = agent {
                    if !crate::text::trim_space(&a.agent.relay).is_empty() {
                        p.reason = a.agent.relay.clone();
                    }
                }
                p.next_step =
                    "Désigner ou compléter un rapport avant toute validation ou relance.".into();
            }
        }
        return Ok(finish_result(p));
    }
    if task.gate.is_some() {
        match facts.gate {
            GateObservation::Current => {}
            GateObservation::NotCurrent => p.validation_state = "failed_or_stale".into(),
            _ => return Err(E::Refused("mission-status-gate-unobserved")),
        }
    }
    Ok(finish_result(p))
}

/// Project the latest applicable attempt and model/cost attribution exactly from
/// retained rows. Absence uses Task.attempts history, never current model settings.
fn attempt(task: &Task, agents: &[AgentObservation<'_>]) -> MissionAttemptProjection {
    let mut p = MissionAttemptProjection {
        agent_id: String::new(),
        attempt_id: String::new(),
        role: String::new(),
        process_state: String::new(),
        activity: String::new(),
        validation: task.status.clone(),
        requested_model: String::new(),
        observed_model: String::new(),
        cost_state: "unknown".into(),
    };
    if let Some(a) = agents.iter().find(|a| a.agent.task_id == task.id) {
        p.agent_id = a.agent.id.clone();
        p.attempt_id = a.agent.attempt_id.clone();
        p.role = a.agent.role.clone();
        p.process_state = a.agent.status.clone();
        p.activity = a.agent.activity.clone();
        if let Some(model) = &a.agent.model_route {
            p.requested_model = model.model.clone();
        }
        if let Some(model) = &a.agent.reported_model {
            p.observed_model = model.model.clone();
        }
        if a.agent
            .usage
            .as_ref()
            .is_some_and(|u| u.provider_reported_cost_usd.is_some())
        {
            p.cost_state = "reported".into();
        }
    } else if let Some(last) = task.attempts.as_ref().and_then(|a| a.last()) {
        p.attempt_id = last.id.clone();
    }
    p
}
/// Count transitive dependants with a visited set as Go descendantCount does.
/// This is a display impact, not graph validation, ordering or scheduling.
fn impact(work: &Work, id: &str) -> Result<i64, E> {
    let mut seen = BTreeSet::from([id]);
    loop {
        let old = seen.len();
        for task in work.tasks.iter().flatten() {
            if task
                .depends
                .iter()
                .flatten()
                .any(|d| seen.contains(d.as_str()))
            {
                seen.insert(task.id.as_str());
            }
        }
        if old == seen.len() {
            break;
        }
    }
    i64::try_from(seen.len() - 1).map_err(|_| E::Refused("mission-status-count-representation"))
}
/// Validate exact dependency coordinates before reading freshness; every real
/// dependency is observed even for accepted tasks because taskWaits consumes it.
fn dependencies<'a>(
    task: &Task,
    facts: &'a [DependencyObservation],
) -> Result<&'a [DependencyObservation], E> {
    let ids = task.depends.as_deref().unwrap_or(&[]);
    if ids.len() != facts.len() || ids.iter().zip(facts).any(|(id, f)| id != &f.task_id) {
        return Err(E::Refused("mission-status-dependencies-mismatch"));
    }
    if facts
        .iter()
        .any(|f| matches!(f.freshness, DependencyFreshness::NotObserved))
    {
        return Err(E::Refused("mission-status-dependency-unobserved"));
    }
    Ok(facts)
}
/// Read-only dependency explanations in stored order. Missing/stale facts were
/// classified by the reader; no acceptance or filesystem checks occur here.
fn waits(work: &Work, facts: &[DependencyObservation]) -> Vec<MissionWait> {
    facts.iter().filter(|f|matches!(f.freshness,DependencyFreshness::NotCurrent)).map(|f|{
        match work.tasks.iter().flatten().find(|t|t.id==f.task_id){
            None=>MissionWait {task:f.task_id.clone(),title:f.task_id.clone(),state:"missing".into(),reason:"Prérequis absent du plan.".into()},
            Some(parent)=>MissionWait {task:f.task_id.clone(),title:parent.title.clone(),state:if parent.status=="accepted"{"stale".into()}else{parent.status.clone()},reason:if parent.status=="accepted"{"Validation ancienne : les preuves actuelles doivent être vérifiées à nouveau."}else{"Ce résultat n’est pas encore validé."}.into()}
        }
    }).collect()
}
/// Pure recoveryPreviewFor text only. No dialogue/receipt inspection or recovery
/// consent is implied; optional advanced preview fields remain nil as Go does.
fn recovery(work: &Work, task: &Task, agent: Option<&AgentObservation<'_>>) -> RecoveryPreview {
    let mut p=RecoveryPreview {dialogue:None,work:work.id.clone(),task:task.id.clone(),revision:work.revision,agent:String::new(),kept:Some(vec!["Historique des tentatives, rapports et avis précédents.".into(),"Critères et dépendances du plan.".into()]),redone:Some(vec!["Vérification des conditions de lancement et du candidat courant.".into(),"Contrôles et revue requis pour la nouvelle tentative.".into()]),correction:task.next.clone(),criteria:Some(task.criteria.as_deref().unwrap_or(&[]).to_vec()),limits:"Aucun budget ni plafond n’est augmenté. Ce résumé n’autorise aucun départ et ne valide aucun résultat.".into(),since_refusal:None,evidence:None,remaining:None,refusal_note:String::new(),evidence_note:String::new()};
    if let Some(a) = agent {
        p.agent = a.agent.id.clone();
        if let Some(kept) = &mut p.kept {
            kept.push("Fournisseur et espace de la tentative sélectionnée, sous réserve des règles du lancement.".into());
        }
        if let Some(redone) = &mut p.redone {
            redone.push(
                "Une nouvelle tentative d’agent ; l’ancienne reste dans l’historique.".into(),
            );
        }
    }
    if let Some(r) = &task.corrective_recovery {
        if !r.instruction.is_empty() {
            p.correction = r.instruction.clone();
        }
    }
    if work
        .planning
        .as_ref()
        .is_some_and(|p| p.repository.is_some())
    {
        if let Some(kept) = &mut p.kept {
            kept.push("Résultat précédent réutilisable seulement après contrôle du dossier de reprise ; ancienne copie conservée.".into());
        }
    }
    p
}
/// Apply only a producer-observed dispatch disposition. Structural eligibility,
/// slot/pause checks and recovery classification are not reimplemented or guessed.
fn disposition(observed: &ReadinessObservation) -> Result<(&'static str, &str), E> {
    match observed {
        ReadinessObservation::NotObserved => Err(E::Refused("mission-status-readiness-unobserved")),
        ReadinessObservation::Observed {
            disposition,
            reason,
            ..
        } => Ok((
            match disposition {
                DispatchDisposition::Ready => "ready",
                DispatchDisposition::Waiting => "waiting",
                DispatchDisposition::Manual => "manual",
                DispatchDisposition::Intervention => "intervention",
            },
            reason,
        )),
    }
}
/// Configure default task-state branches from exact dependency/readiness inputs.
/// The repeated dispatch observation used for corrective retries grants nothing.
fn initial_task_state(
    x: &mut MissionTask,
    work: &Work,
    task: &Task,
    context: &MissionStatus,
    facts: &TaskObservation,
) -> Result<(), E> {
    x.action = "inspect".into();
    x.label = "Comprendre et résoudre".into();
    x.reason = task.blocker.clone();
    for dependency in &facts.dependencies {
        if matches!(dependency.freshness, DependencyFreshness::NotCurrent) {
            x.state = "waiting".into();
            x.target = dependency.task_id.clone();
            x.reason = work
                .tasks
                .iter()
                .flatten()
                .find(|t| t.id == dependency.task_id)
                .map(|p| format!("Attend {}", p.title))
                .unwrap_or_else(|| format!("Attend la validation de {}", dependency.task_id));
            x.label = "Examiner le prérequis".into();
            break;
        }
    }
    if x.state == "waiting" {
        return Ok(());
    }
    let (state, reason) = disposition(&facts.readiness)?;
    x.state = state.into();
    x.reason = reason.into();
    let ReadinessObservation::Observed {
        profile,
        assistance,
        workspace,
        ..
    } = &facts.readiness
    else {
        return Err(E::Refused("mission-status-readiness-unobserved"));
    };
    if matches!(profile, ProfileObservation::NotObserved) {
        return Err(E::Refused("mission-status-profile-unobserved"));
    }
    if x.state == "ready" {
        match assistance {
            AssistObservation::NotObserved => {
                return Err(E::Refused("mission-status-assist-unobserved"))
            }
            AssistObservation::Held { reason } => {
                x.state = "intervention".into();
                x.reason = reason.clone();
            }
            AssistObservation::Allowed => {
                if matches!(profile, ProfileObservation::Missing) {
                    return Err(E::Refused("mission-status-ready-profile-missing"));
                }
                match workspace {
                    WorkspaceObservation::NotObserved => {
                        return Err(E::Refused("mission-status-workspace-unobserved"))
                    }
                    WorkspaceObservation::Occupied => {
                        x.state = "waiting".into();
                        x.reason =
                            "Espace occupé par une tentative ; reprise après sa libération".into();
                    }
                    WorkspaceObservation::Free => {}
                }
                if x.state == "ready" {
                    x.action = "start".into();
                    x.label = "Lancer cette tâche".into();
                    if !context.authorized {
                        x.state = "manual".into();
                        x.reason =
                            "Lancement manuel possible ; mission continue non autorisée".into();
                    } else if !context.enabled {
                        x.state = "waiting".into();
                        x.action = "supervise".into();
                        x.label = "Voir comment reprendre".into();
                        x.reason = if context.supervision.state == "error" {
                            "Conducteur présent, mais sa dernière vérification est en erreur"
                        } else {
                            "Autorisation enregistrée ; aucun conducteur actif observé"
                        }
                        .into();
                    }
                }
            }
        }
    }
    if matches!(profile, ProfileObservation::Missing) && x.state == "intervention" {
        x.state = "configure".into();
        x.reason = "Configuration initiale à préparer avant le premier départ".into();
        x.action = "configure".into();
        x.label = "Préparer le lancement".into();
    }
    if task.launch_held {
        x.state = "intervention".into();
        x.action = "prepare".into();
        x.label = "Autoriser le plan dans Préparer".into();
    }
    Ok(())
}

/// Project all tasks in Work order, returning a complete new value or an error.
/// Inputs must cover every task exactly once and every dependency in stored order;
/// Agent facts retain the producer's latest-first history window. No partial
/// projection is returned on missing proof, coordinate mismatch or numeric error.
/// Counts follow Go's result branches before prepared/recovery task overrides.
/// Existing insight functions produce explanations/actions; no fresh grant follows.
/// Current/managed review and readiness classifiers remain mandatory external
/// producers: this function does not implement or silently approximate them.
pub fn project_tasks(
    work: &Work,
    context: &MissionStatus,
    observations: &[TaskObservation],
    agents: &[AgentObservation<'_>],
) -> Result<TaskProjection, E> {
    let tasks = work.tasks.as_deref().unwrap_or(&[]);
    if tasks.len() != observations.len()
        || tasks
            .iter()
            .zip(observations)
            .any(|(t, o)| t.id != o.task_id)
    {
        return Err(E::Refused("mission-status-task-observations-mismatch"));
    }
    let mut context = context.clone();
    context.validated = 0;
    context.review = 0;
    context.running = 0;
    context.total = i64::try_from(tasks.len())
        .map_err(|_| E::Refused("mission-status-count-representation"))?;
    let insight_agents: Vec<_> = agents
        .iter()
        .map(|a| MissionAgentInsight {
            task_id: &a.agent.task_id,
            status: &a.agent.status,
            active: a.agent.is_active(),
            observed_state: a.observed_state,
            fallback_diagnostic_limit_reached: a.fallback_diagnostic.limit_reached,
            environment_recovery: a.environment_recovery,
        })
        .collect();
    let mut result = TaskProjection {
        tasks: Vec::with_capacity(tasks.len()),
        validated: 0,
        review: 0,
        running: 0,
    };
    for (task, facts) in tasks.iter().zip(observations) {
        let deps = dependencies(task, &facts.dependencies)?;
        let agent = agents.iter().find(|a| a.agent.task_id == task.id);
        let result_facts = result_presentation(task, agent, &facts.result)?;
        let mut x = MissionTask {
            waits: Some(waits(work, deps)),
            recovery_preview: recovery(work, task, agent),
            primary: MissionPrimaryAction {
                kind: String::new(),
                label: String::new(),
                effect: String::new(),
                task: String::new(),
                tone: String::new(),
            },
            attempts_used: i64::try_from(task.attempts.as_ref().map_or(0, Vec::len))
                .map_err(|_| E::Refused("mission-status-count-representation"))?,
            attempts_allowed: task.plan_max_attempts,
            attempt_limit_reached: false,
            id: task.id.clone(),
            title: task.title.clone(),
            state: task.status.clone(),
            reason: String::new(),
            action: String::new(),
            label: String::new(),
            target: task.id.clone(),
            impact: impact(work, &task.id)?,
            deliverable: task.deliverable.clone(),
            validation_mode: task
                .validation_policy
                .as_ref()
                .map_or_else(|| "human".into(), |v| v.mode.clone()),
            validation_receipt: task
                .automatic_validation
                .as_ref()
                .map_or_else(String::new, |v| v.receipt.clone()),
            result: result_facts,
            understanding: MissionUnderstanding {
                what: String::new(),
                next_step: String::new(),
                actor: String::new(),
                actor_kind: String::new(),
                situation: String::new(),
            },
            diagnostic: None,
            attempt: attempt(task, agents),
        };
        let count = |field: &mut i64| -> Result<(), E> {
            *field = field
                .checked_add(1)
                .ok_or(E::Refused("mission-status-count-representation"))?;
            Ok(())
        };
        match x.result.state.as_str() {
            "validated" => {
                count(&mut result.validated)?;
                x.state = "validated".into();
                x.reason = x.result.reason.clone();
                x.action = "report".into();
                x.label = "Voir le résultat".into();
            }
            "validation_stale" => {
                x.state = "intervention".into();
                x.reason = x.result.reason.clone();
                x.action = "reopen".into();
                x.label = "Revalider les preuves".into();
            }
            "waived" | "abandoned" => {
                x.state = x.result.state.clone();
                x.reason = x.result.reason.clone();
                x.action = "inspect".into();
                x.label = if x.state == "waived" {
                    "Voir la dérogation"
                } else {
                    "Voir la décision"
                }
                .into();
            }
            "review_blocked" | "delivery_incomplete" => {
                x.state = "intervention".into();
                x.reason = x.result.reason.clone();
                x.action = "inspect".into();
                x.label = "Examiner le résultat".into();
            }
            "review_in_progress" | "review_awaiting_publication" => {
                count(&mut result.review)?;
                x.state = "review".into();
                x.reason = x.result.reason.clone();
                x.action = "report".into();
                x.label = "Examiner le résultat".into();
            }
            "result_to_review" | "validation_withheld" => {
                count(&mut result.review)?;
                x.state = "review".into();
                x.reason = x.result.reason.clone();
                x.action = "report".into();
                x.label = "Examiner le résultat".into();
                if x.validation_mode == "automatic" && x.result.validation_state == "missing_gate" {
                    x.reason="Contrôles automatiques préautorisés retenus ; vérifier pause, autorisation et preuves".into();
                }
                if x.result.validation_state == "ready_for_decision" {
                    x.action = "accepted".into();
                    x.label = "Valider après revue".into();
                }
            }
            "in_progress" | "execution_unconfirmed" => {
                count(&mut result.running)?;
                x.state = "running".into();
                x.reason = x.result.reason.clone();
                x.action = "inspect".into();
                x.label = "Suivre l’agent".into();
            }
            "stopped_early" | "completed_unproven" => {
                x.state = "intervention".into();
                x.reason = x.result.reason.clone();
                x.action = "inspect".into();
                x.label = "Examiner avant reprise".into();
            }
            _ => initial_task_state(&mut x, work, task, &context, facts)?,
        }
        match &facts.prepared_launch {
            PreparedLaunchObservation::NotObserved => {
                return Err(E::Refused("mission-status-prepared-launch-unobserved"))
            }
            PreparedLaunchObservation::Absent => {}
            PreparedLaunchObservation::Present { reason } => {
                x.state = "intervention".into();
                x.action = "inspect".into();
                x.label = "Reprendre le lancement préparé".into();
                x.reason = if reason.is_empty() {
                    "La copie et les réglages sont conservés. Confirmez la reprise du lancement depuis cette tâche.".into()
                } else {
                    reason.clone()
                };
            }
        }
        if task.status == "blocked"
            && task.plan_max_attempts > 0
            && x.attempts_used >= task.plan_max_attempts
            && x.state != "review"
        {
            x.attempt_limit_reached = match facts.execution {
                ExecutionObservation::NotObserved => {
                    return Err(E::Refused("mission-status-execution-unobserved"))
                }
                ExecutionObservation::Observed => false,
                ExecutionObservation::NotPresent => true,
            };
        }
        if task
            .independent_review
            .as_ref()
            .is_some_and(|r| r.state == "running")
            || agents
                .iter()
                .any(|a| a.agent.task_id == task.id && a.agent.is_active())
        {
            x.attempt_limit_reached = false;
        }
        if x.attempt_limit_reached {
            x.state = "intervention".into();
            x.action = "recovery".into();
            x.label = "Examiner les tentatives et les refus".into();
            if let Some(r) = &task.independent_review {
                if r.state == "changes_requested" {
                    x.reason = r.reason.clone();
                }
            }
        }
        if task.corrective_recovery.is_some()
            && task.planning_retry
            && task.status == "blocked"
            && x.attempts_used < task.plan_max_attempts
        {
            let (state, reason) = disposition(&facts.readiness)?;
            x.state = state.into();
            x.reason = format!("Essai correctif autorisé. {reason}");
            x.action = "supervise".into();
            x.label = "Suivre la reprise".into();
        }
        context.validated = result.validated;
        context.review = result.review;
        context.running = result.running;
        x.understanding = task_understanding(&x, &context, &insight_agents);
        if let Some(a) = agent {
            if a.fallback_diagnostic
                .items
                .as_ref()
                .is_some_and(|v| !v.is_empty())
            {
                x.diagnostic = Some(Box::new(a.fallback_diagnostic.clone()));
            }
        }
        x.primary = mission_task_primary(&x);
        result.tasks.push(x);
    }
    Ok(result)
}

#[cfg(test)]
mod tests;
