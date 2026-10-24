//! Derived Work views only. Display facts never authenticate actors or authorize
//! task execution/acceptance. Root supplies its actual row, settings and reads.
#![cfg(target_os = "linux")]
use crate::{
    Evaluation, GitMetadataObservation, RecordedEvent, Task, Work, WorkValidation, WorkViewLanguage,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, error::Error, fmt, path::Path};

pub enum WorkViewError {
    Refused(&'static str),
    Json(serde_json::Error),
    Projection(crate::WorkProjectionError),
    Settings(crate::EngineSettingsError),
    Request(crate::Error),
    Encode(crate::WorkError),
    Events(crate::RecordedEventError),
    Runtime(crate::RuntimeError),
}
impl WorkViewError {
    /// Safe family, excluding report text, filenames and original process output.
    pub fn code(&self) -> &'static str {
        "work-view"
    }
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Refused(r) => r,
            Self::Json(_) => "view-json",
            Self::Projection(e) => e.reason(),
            Self::Settings(e) => e.reason(),
            Self::Request(_) => "decision-event-invalid",
            Self::Encode(_) => "view-encoding",
            Self::Events(e) => e.code(),
            Self::Runtime(e) => e.code(),
        }
    }
}
impl fmt::Display for WorkViewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.reason())
    }
}
impl fmt::Debug for WorkViewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for WorkViewError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Refused(_) => None,
            Self::Json(e) => Some(e),
            Self::Projection(e) => Some(e),
            Self::Settings(e) => Some(e),
            Self::Request(e) => Some(e),
            Self::Encode(e) => Some(e),
            Self::Events(e) => Some(e),
            Self::Runtime(e) => Some(e),
        }
    }
}
/// A calculable gate is not a fresh acceptance: ship/progress and task evidence
/// remain separate. Original refusal is retained without private serialization.
#[derive(Serialize)]
pub struct CurrentGateObservation {
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evaluation: Option<Evaluation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip)]
    pub cause: Option<crate::GateContractError>,
}
/// Complete local read model. Events retain original raw payload bytes; use
/// encode_work_view rather than converting them through a floating-point JSON map.
pub struct WorkViewObservation {
    pub work: Work,
    pub validation: WorkValidation,
    pub events: Vec<RecordedEvent>,
    pub current_gates: BTreeMap<String, CurrentGateObservation>,
    pub git: GitMetadataObservation,
    pub resume_markdown: String,
}
impl fmt::Debug for WorkViewObservation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WorkViewObservation { data: redacted }")
    }
}
struct Ui {
    language: WorkViewLanguage,
    english: BTreeMap<String, String>,
}
impl Ui {
    fn new(language: WorkViewLanguage) -> Result<Self, WorkViewError> {
        let english = if language == WorkViewLanguage::English {
            serde_json::from_slice(include_bytes!("../../../../locales/en.json"))
                .map_err(WorkViewError::Json)?
        } else {
            BTreeMap::new()
        };
        Ok(Self { language, english })
    }
    fn text<'a>(&'a self, source: &'a str) -> &'a str {
        if self.language == WorkViewLanguage::English {
            self.english
                .get(source)
                .map(String::as_str)
                .unwrap_or(source)
        } else {
            source
        }
    }
    /// Expand only trusted repository templates; user data is never translated
    /// or interpreted as formatting directives. Count mismatch refuses explicitly.
    fn format(&self, source: &str, args: &[String]) -> Result<String, WorkViewError> {
        let mut out = String::new();
        let mut chars = self.text(source).chars();
        let mut next = 0;
        while let Some(c) = chars.next() {
            if c != '%' {
                out.push(c);
                continue;
            }
            match chars.next() {
                Some('%') => out.push('%'),
                Some('s' | 'd' | 't') => {
                    out.push_str(
                        args.get(next)
                            .ok_or(WorkViewError::Refused("view-format-argument-count"))?,
                    );
                    next += 1;
                }
                _ => return Err(WorkViewError::Refused("view-format-unsupported")),
            }
        }
        if next != args.len() {
            return Err(WorkViewError::Refused("view-format-argument-count"));
        }
        Ok(out)
    }
    fn value(&self, value: &str) -> String {
        if crate::text::trim_space(value).is_empty() {
            self.text("non renseigné").into()
        } else {
            clean(value)
        }
    }
    fn status(&self, value: &str) -> &str {
        self.text(match value {
            "todo" => "À FAIRE",
            "running" => "EN COURS (activité non confirmée)",
            "blocked" => "BLOQUÉE",
            "submitted" => "SOUMISE — À VALIDER",
            "accepted" => "ACCEPTÉE",
            "waived" => "ACCEPTÉE PAR DÉROGATION",
            "abandoned" => "ABANDONNÉE",
            _ => "",
        })
    }
}
fn clean(value: &str) -> String {
    value
        .chars()
        .filter(|&c| c == '\n' || c == '\t' || !c.is_control())
        .collect()
}
#[derive(Default, Deserialize)]
#[serde(default)]
struct GatePath {
    path: String,
}
fn gate_label(task: &Task) -> String {
    let Some(g) = task.gate.as_deref() else {
        return String::new();
    };
    let name = crate::text::trim_space(&g.name);
    if !name.is_empty() {
        return name.into();
    }
    let path = crate::work::decode::<GatePath>(
        g.document.0.get().as_bytes(),
        &[("path", crate::model_wire::Kind::String)],
    )
    .ok()
    .map(|v| v.path)
    .unwrap_or_default();
    let path = crate::text::trim_space(&path);
    let label = if !path.is_empty() {
        path.into()
    } else {
        let digest = &g.evaluation.config_digest;
        if digest.len() <= 14 {
            digest.clone()
        } else {
            format!("{}…", String::from_utf8_lossy(&digest.as_bytes()[..12]))
        }
    };
    format!("Gate sans nom — {label}")
}

/// Build the complete view from one Root observation. Actual files are checked
/// for referenced memory paths; malformed selected OODA events refuse instead of
/// becoming empty decisions. Presentation limits are the supplied saved v8 group.
/// No command, mutation, actor authentication or acceptance is performed here.
pub(crate) fn render_work_view(
    root: &Path,
    view: &WorkViewObservation,
    settings: &crate::EngineSettingsDocument,
    language: WorkViewLanguage,
) -> Result<String, WorkViewError> {
    let ui = Ui::new(language)?;
    let limits = settings
        .values
        .work_view_limits()
        .map_err(WorkViewError::Settings)?;
    let w = &view.work;
    let tasks = w.tasks.as_deref().unwrap_or_default();
    let accepted = view.validation.tasks.values().filter(|t| t.fresh).count();
    let mut out = String::new();
    macro_rules! line{($source:expr $(,$arg:expr)* $(,)?)=>{out.push_str(&ui.format($source,&[$(($arg).to_string()),*])?);}}
    line!("# %s\n\nTravail : %s | révision %d | %s\nDernière activité : %s\n\n## Objectif et résultat attendu\n%s\nPérimètre : %s\n",w.title,w.id,w.revision,view.validation.label,w.updated,w.objective,w.scope);
    for criterion in w.criteria.iter().flatten() {
        out.push_str(&format!("- {criterion}\n"));
    }
    line!(
        "\n## Situation actuelle\n%s\nHEAD enregistré : %s\nBranche enregistrée : %s\n",
        ui.value(&w.summary),
        ui.value(&w.git.head),
        ui.value(&w.git.branch)
    );
    if view.git.state != w.git {
        out.push_str(ui.text("ATTENTION : contexte Git modifié depuis le checkpoint.\n"));
    }
    line!(
        "HEAD courant : %s\nBranche courante : %s\n",
        ui.value(&view.git.state.head),
        ui.value(&view.git.state.branch)
    );
    for d in &view.git.diagnostics {
        let field = match d.field {
            crate::GitMetadataField::Head => "HEAD",
            crate::GitMetadataField::Branch => "branch",
            crate::GitMetadataField::Changes => "changes",
        };
        line!("Métadonnée Git indisponible (%s) : %s.\n", field, d.reason);
    }
    if !view.git.state.changes.is_empty() {
        out.push_str(ui.text("Modifications locales présentes ; consulter git status.\n"));
    }
    out.push_str(ui.text("\n## Tâches actives et résultats attendus\n"));
    let mut active = 0;
    for t in tasks {
        if matches!(t.status.as_str(), "running" | "submitted") {
            active += 1;
            line!(
                "- %s : %s — %s\n  Responsable : %s\n  Livrable : %s\n  Prochaine action : %s\n",
                t.id,
                t.title,
                ui.status(&t.status),
                ui.value(&t.owner),
                t.deliverable,
                ui.value(&t.next)
            );
        }
    }
    if active == 0 {
        out.push_str(ui.text("Aucune tâche active déclarée.\n"));
    }
    line!("\n## Travail validé\nTâches acceptées (preuves ou dérogation) : %d/%d.\nCe ratio est distinct de la progression des critères d’une gate.\n",accepted,tasks.len());
    for t in tasks {
        let validation = view
            .validation
            .tasks
            .get(&t.id)
            .ok_or(WorkViewError::Refused("view-task-validation-missing"))?;
        if validation.fresh {
            line!("- %s : %s\n", t.id, t.title);
            if let Some(o) = t.manual_override.as_deref() {
                line!("  DÉROGATION : %s — %s — %s\n", o.reason, o.actor, o.at);
            }
            if let Some(g) = t.gate.as_deref() {
                for name in g
                    .evaluation
                    .artifacts
                    .as_ref()
                    .into_iter()
                    .flat_map(|a| a.keys())
                {
                    line!("  Preuve/artefact : %s\n", name);
                }
            }
        }
    }
    out.push_str(ui.text("\n## Travail restant et blocages\n"));
    for t in tasks {
        let validation = &view.validation.tasks[&t.id];
        if validation.fresh {
            continue;
        }
        line!(
            "- %s : %s — %s\n  Livrable : %s\n  Dépendances : %s\n  Blocage : %s\n",
            t.id,
            t.title,
            ui.status(&t.status),
            t.deliverable,
            ui.value(&t.depends.as_deref().unwrap_or_default().join(", ")),
            ui.value(&t.blocker)
        );
        if matches!(t.status.as_str(), "accepted" | "waived") {
            out.push_str(
                ui.text("  Acceptation à revalider : tâche ou dépendance périmée/rouverte.\n"),
            );
        }
        for c in t.criteria.iter().flatten() {
            line!("  Critère : %s\n", c);
        }
        for a in t.attempts.iter().flatten() {
            line!("  Tentative %s : %s\n", a.id, a.status);
        }
    }
    out.push_str(ui.text("\n## Gates, qualité et progression vérifiée\n"));
    for t in tasks {
        let e = &view.validation.tasks[&t.id].evidence;
        line!("- %s · preuve structurée : tentative=%s ; révision=%d ; fraîcheur=%s\n  Revue du rapport=%s ; contrôles exécutés=%s ; acceptation=%s\n",t.id,e.attempt_id,e.revision,e.freshness,e.report_review.state,e.controls.state,e.acceptance.state);
        for c in &e.controls.items {
            let command = c
                .command
                .as_ref()
                .filter(|v| !v.is_empty())
                .map(|v| v.join(" "))
                .unwrap_or_else(|| "unknown".into());
            let exit = c
                .exit_code
                .map(|v| v.to_string())
                .unwrap_or_else(|| "unknown".into());
            line!("  Contrôle %s : tentative=%s ; exécution=%s ; révision=%s ; sha_candidat=%s ; commande=%s ; code=%s ; début=%s ; fin=%s ; fraîcheur=%s\n",c.id,c.attempt_id,c.execution,c.revision,c.candidate_sha,command,exit,c.started_at,c.finished_at,c.freshness);
        }
        if t.gate.is_none() {
            line!("- %s : gate non renseignée.\n", t.id);
            continue;
        }
        let current = view
            .current_gates
            .get(&t.id)
            .ok_or(WorkViewError::Refused("view-current-gate-missing"))?;
        if let Some(error) = &current.error {
            line!("- %s : PREUVES PÉRIMÉES OU INVALIDES — %s\n", t.id, error);
            continue;
        }
        let ev = current
            .evaluation
            .as_ref()
            .ok_or(WorkViewError::Refused("view-current-evaluation-missing"))?;
        let quality = ev
            .quality
            .as_deref()
            .map(|v| format!("{:.2}/100", v.0))
            .unwrap_or_else(|| "indisponible".into());
        let pct = ev
            .progress
            .percent
            .as_deref()
            .map(|v| format!("{:.2}%", v.0))
            .unwrap_or_else(|| "indisponible".into());
        line!("- %s [%s] : gate « %s », qualité %s ; provisoire=%t\n  Critères PASS : %d/%d (%s) ; livraison permise par gate=%t\n  Blocages : %s\n",t.id,ev.phase,gate_label(t),quality,ev.provisional,ev.progress.passed,ev.progress.applicable,pct,ev.ship_allowed,ui.value(&ev.blockers.as_deref().unwrap_or_default().join(", ")));
    }
    let next = if w.next.is_empty() {
        ui.text("à définir")
    } else {
        &w.next
    };
    line!(
        "\n## Prochaine action du travail\n%s\n\n## Décisions OODA et mémoire utile\n",
        next
    );
    for event in crate::recent_work_decisions(&view.events, limits.recent_decisions) {
        let raw = event
            .payload
            .as_deref()
            .ok_or(WorkViewError::Refused("decision-payload-required"))?;
        let projected =
            swarm_wire_json::projection::project(raw, crate::request::FIELDS_REQUEST, false)
                .map_err(|e| {
                    WorkViewError::Request(match e {
                        swarm_wire_json::projection::Error::Json(e) => crate::Error::Frame(e),
                        e => crate::Error::Projection(e),
                    })
                })?;
        let r: crate::Request = serde_json::from_value(projected).map_err(WorkViewError::Json)?;
        let at = crate::environment::go_text(&event.at).map_err(WorkViewError::Runtime)?;
        line!("- %s : observation=%s ; orientation=%s ; décision=%s\n  Responsable=%s ; action=%s ; résultat=%s\n",at,r.observation,r.orientation,r.decision,r.owner,r.next,ui.value(&r.result));
    }
    if w.memory.as_ref().is_none_or(|m| m.is_empty()) {
        out.push_str(ui.text("Mémoire spécifique : non renseignée.\n"));
    }
    for name in w.memory.iter().flatten() {
        out.push_str(&format!("- {name}"));
        if crate::local_file(root, name).is_err() {
            out.push_str(ui.text(" (indisponible)"));
        }
        out.push('\n');
    }
    out.push_str(ui.text("\nLes tentatives enregistrées ne prouvent pas qu’un processus tourne encore.\nUne gate ne remplace pas une autorisation de publication.\n"));
    crate::wrap_work_text(&out, limits.wrap_characters).map_err(WorkViewError::Projection)
}

/// Encode the Go public envelope without converting RawMessage event payloads
/// through f64 or dropping original invalid string bytes. Availability failures
/// add explicit diagnostics; no private Git output/cause is serialized. Syntax,
/// serialization and event payload failures refuse the whole buffer before output.
pub fn encode_work_view(view: &WorkViewObservation) -> Result<Vec<u8>, WorkViewError> {
    let mut out = b"{\"work\":".to_vec();
    out.extend(crate::encode_public_model(&view.work).map_err(WorkViewError::Encode)?);
    out.extend(b",\"validation\":");
    out.extend(crate::encode_public_model(&view.validation).map_err(WorkViewError::Encode)?);
    out.extend(b",\"events\":");
    out.extend(crate::encode_recorded_events(&view.events, false).map_err(WorkViewError::Events)?);
    out.extend(b",\"current_gates\":");
    out.extend(crate::encode_public_model(&view.current_gates).map_err(WorkViewError::Encode)?);
    out.extend(b",\"current_git\":");
    out.extend(crate::encode_public_model(&view.git.state).map_err(WorkViewError::Encode)?);
    out.extend(b",\"resume_markdown\":");
    out.extend(crate::encode_public_model(&view.resume_markdown).map_err(WorkViewError::Encode)?);
    if !view.git.diagnostics.is_empty() {
        out.extend(b",\"git_diagnostics\":");
        out.extend(
            crate::encode_public_model(&view.git.diagnostics).map_err(WorkViewError::Encode)?,
        );
    }
    out.push(b'}');
    crate::pretty_public_json(&out).map_err(WorkViewError::Encode)
}

/// Render the actual reverse-creation list with its already observed validation.
/// Limits are saved presentation policy; no Work, evidence or authority changes.
/// Template/policy failures refuse the complete text instead of partial success.
pub(crate) fn render_work_list(
    works: &[(Work, WorkValidation)],
    settings: &crate::EngineSettingsDocument,
    language: WorkViewLanguage,
) -> Result<String, WorkViewError> {
    let ui = Ui::new(language)?;
    let limits = settings
        .values
        .work_view_limits()
        .map_err(WorkViewError::Settings)?;
    let mut out = ui
        .text("TRAVAUX — choisir explicitement un identifiant\n\n")
        .to_owned();
    if works.is_empty() {
        out.push_str(
            ui.text("Aucun travail enregistré. Créer un travail avec swarm work create.\n"),
        );
    }
    for (w, v) in works {
        out.push_str(&ui.format("%s | %s | %s\n  Tâches acceptées (preuves ou dérogation) : %d/%d | révision %d\n  Dernière activité : %s\n",&[w.id.clone(),v.label.clone(),w.title.clone(),v.tasks.values().filter(|t|t.fresh).count().to_string(),w.tasks.as_ref().map_or(0,Vec::len).to_string(),w.revision.to_string(),w.updated.clone()])?);
        for t in w.tasks.iter().flatten() {
            if !t.blocker.is_empty() {
                out.push_str(
                    &ui.format("  Blocage %s : %s\n", &[t.id.clone(), t.blocker.clone()])?,
                );
            }
        }
    }
    out.push_str(ui.text("\nReprise : swarm resume <identifiant>\nAucune action de tâche n’est lancée par la reprise.\n"));
    crate::wrap_work_text(&out, limits.wrap_characters).map_err(WorkViewError::Projection)
}
