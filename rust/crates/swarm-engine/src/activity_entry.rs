//! Go activity presentation from recorded bytes; origin is inferred, not identity.
use crate::{activity_rank, ActivityEntry, ActivitySource, AgentLaunchError as E};
use serde::Deserialize;
use swarm_wire_json::projection::{self, Kind};
/// Preserve Go byte-wise invalid-UTF8 replacement for stored SQL text. No trim,
/// Unicode normalization, path access, default actor or permission is introduced.
pub(crate) fn text(raw: &[u8]) -> String { crate::text::go_utf8(raw) }
/// Go event-name labels are protocol presentation, not configurable stop policy.
/// Unknown recorded kinds remain literal data, never proof of autonomous action.
fn label(kind: &str) -> String {
    match kind {
        "assist-action" => "Action conseillée par l’IA",
        "mission.start" => "Configuration de la mission",
        "mission-policy" => "Autorisation de la mission",
        "dispatch" => "Ordonnancement",
        "conductor" => "Conduite",
        "pause" => "Départs",
        "autonomy" => "Niveau d'autonomie",
        "budget" => "Budget",
        "priority" => "Priorité",
        "decision" => "Décision",
        "decision.moteur" => "Décision du moteur",
        "launch.refused" => "Lancement refusé",
        "assistant" => "Assistant",
        "task.update" => "Tâche",
        "gate" => "Gate",
        "agent.start" => "Tentative",
        "ooda" => "Boucle OODA",
        "checkpoint" => "Point d'étape",
        "work.create" => "Travail",
        "task.add" => "Tâche ajoutée",
        "task.submit" => "Rapport soumis",
        "task.override" => "Dérogation",
        "plan.adopt" => "Plan adopté",
        "brief.adopt" => "Brief adopté",
        v if v.starts_with("retex-") => "Retour d'expérience",
        v => v,
    }
    .into()
}
/// Reproduce Go kind/payload attribution as an explicitly inferred display value.
/// External mutations sanitize origin at their writer; this reader authenticates
/// no actor and cannot use the returned string for business authorization.
fn origin(kind: &str, payload_origin: &str) -> &'static str {
    if kind == "task.update" {
        return if payload_origin == "conducteur" {
            "moteur"
        } else {
            "vous"
        };
    }
    if matches!(
        kind,
        "assist-action"
            | "mission.start"
            | "mission-policy"
            | "pause"
            | "autonomy"
            | "priority"
            | "budget"
            | "decision"
            | "assistant"
            | "ooda"
            | "checkpoint"
            | "gate"
            | "work.create"
            | "task.add"
            | "task.submit"
            | "task.override"
            | "plan.adopt"
            | "brief.adopt"
    ) || kind.starts_with("retex-")
    {
        "vous"
    } else {
        "moteur"
    }
}
/// Same recorded status vocabulary as Go; no live state or validation inferred.
fn status(value: &str) -> &str {
    if value.starts_with("unknown/") {
        return "À vérifier";
    }
    match value {
        "todo" => "À faire",
        "running" => "En cours",
        "blocked" => "Bloquée",
        "submitted" => "À vérifier",
        "waived" => "Dérogation",
        "accepted" => "Terminée et validée",
        "completed" => "Terminé",
        "interrupted" => "Interrompu",
        "failed" => "Échec",
        "stopping" => "Arrêt…",
        "queued" => "En attente",
        "starting" => "Démarrage",
        v => v,
    }
}
/// Add a recorded task identifier only when present; absence stays unassigned.
fn prefix(task: &str, message: &str) -> String {
    if task.is_empty() {
        message.into()
    } else {
        format!("{task} : {message}")
    }
}
/// Decode a typed Go object without judging current business/schema authority.
/// Unknown fields follow Go; malformed known types preserve safe typed causes.
fn decoded<T: serde::de::DeserializeOwned>(raw: &[u8], shape: projection::Fields) -> Result<T, E> {
    let v = projection::project(raw, shape, false)
        .map_err(|e| E::caused("activity-source-json-invalid", e))?;
    serde_json::from_value(v).map_err(|e| E::caused("activity-source-json-invalid", e))
}
/// Project one historical event's display inputs, regardless of its producer.
/// Work mutations, Launch and review results have different payload schemas;
/// fields not consumed by this reader are not current Request assertions.
/// The eight displayed/attribution strings retain Go field/null/case semantics.
/// Malformed consumed types still refuse; this grants no producer authenticity,
/// command validity, role, mutation permission or business acceptance.
pub(crate) fn event(
    sequence: i64,
    at: String,
    kind: String,
    raw: &[u8],
) -> Result<ActivityEntry, E> {
    #[derive(Default, Deserialize)]
    #[serde(default)]
    struct DisplayPayload {
        id: String,
        task_id: String,
        title: String,
        status: String,
        blocker: String,
        origin: String,
        decision: String,
        summary: String,
    }
    let p: DisplayPayload = decoded(
        raw,
        &[
            ("id", Kind::String),
            ("task_id", Kind::String),
            ("title", Kind::String),
            ("status", Kind::String),
            ("blocker", Kind::String),
            ("origin", Kind::String),
            ("decision", Kind::String),
            ("summary", Kind::String),
        ],
    )?;
    let task = if p.id.is_empty() {
        p.task_id.clone()
    } else {
        p.id.clone()
    };
    let message = match kind.as_str() {
        "task.update" => {
            let mut m = prefix(
                &task,
                if p.status.is_empty() {
                    "mise à jour"
                } else {
                    status(&p.status)
                },
            );
            if !p.blocker.is_empty() {
                m.push_str(" · ");
                m.push_str(&p.blocker)
            }
            m
        }
        "gate" => prefix(&task, "évaluation enregistrée"),
        "agent.start" => prefix(&task, "tentative lancée"),
        "ooda" => format!("décision : {}", p.decision),
        "checkpoint" => p.summary.clone(),
        _ if !p.title.is_empty() => p.title.clone(),
        _ => label(&kind),
    };
    Ok(ActivityEntry {
        at,
        rank: activity_rank(ActivitySource::WorkEvent, sequence),
        origin: origin(&kind, &p.origin).into(),
        label: label(&kind),
        kind,
        task_id: task,
        message,
    })
}
/// Project one control row. Receipt/policy JSON is typed, never authenticated.
/// Bad JSON/types refuse rather than Go's fallback sentence hiding its cause.
/// Other messages stay literal; no current mission permission is inferred.
pub(crate) fn control(
    sequence: i64,
    at: String,
    kind: String,
    message: String,
) -> Result<ActivityEntry, E> {
    let message = if kind == "assist-action" {
        #[derive(Default, Deserialize)]
        #[serde(default)]
        struct Receipt {
            message: String,
        }
        let r: Receipt = decoded(
            message.as_bytes(),
            &[
                ("id", Kind::String),
                ("turn", Kind::String),
                ("step", Kind::Int64),
                ("status", Kind::String),
                ("message", Kind::String),
            ],
        )?;
        r.message
    } else if kind == "mission-policy" {
        #[derive(Default, Deserialize)]
        #[serde(default)]
        struct Policy {
            enabled: bool,
        }
        let p: Policy = decoded(
            message.as_bytes(),
            &[
                ("enabled", Kind::Bool),
                ("actor", Kind::String),
                ("at", Kind::String),
            ],
        )?;
        if p.enabled {
            "Mission continue autorisée ; les tâches prêtes seront enchaînées".into()
        } else {
            "Mission continue arrêtée ; aucun nouveau départ autorisé".into()
        }
    } else {
        message
    };
    Ok(ActivityEntry {
        at,
        rank: activity_rank(ActivitySource::Cockpit, sequence),
        origin: origin(&kind, "").into(),
        label: label(&kind),
        kind,
        task_id: String::new(),
        message,
    })
}
