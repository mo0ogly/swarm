//! Pure mission explanations from the Go status/guidance producers at
//! 553593a2ffa3823d16784a6fedc291eebf390837. Inputs must already contain the real
//! dispatcher/result/runtime observations; these projections read no SQL, clock,
//! process or filesystem and grant no execution or acceptance authority.
use crate::mission_status_contract::{
    MissionCoordinationPhase, MissionGuidance, MissionPrimaryAction, MissionStatus, MissionTask,
    MissionUnderstanding,
};
use crate::{MissionStatusExchange, Work};
use std::time::Duration;
use time::OffsetDateTime;

/// Explicit Go agent classifications consumed by task explanations, in the
/// caller's agent order. `active`, `observed_state`, fallback diagnostic limit and
/// environment recovery must come from their actual classifiers, never guessed
/// from PID/status strings or supplied as missing-data defaults.
#[derive(Clone, Copy, Debug)]
pub struct MissionAgentInsight<'a> {
    pub task_id: &'a str,
    pub status: &'a str,
    pub active: bool,
    pub observed_state: &'a str,
    pub fallback_diagnostic_limit_reached: bool,
    pub environment_recovery: bool,
}

/// Construct the five explicit explanation fields without deriving authority.
fn understanding(
    what: impl Into<String>,
    next: impl Into<String>,
    actor: &str,
    actor_kind: &str,
    situation: &str,
) -> MissionUnderstanding {
    MissionUnderstanding {
        what: what.into(),
        next_step: next.into(),
        actor: actor.into(),
        actor_kind: actor_kind.into(),
        situation: situation.into(),
    }
}

/// Construct a navigation action. Empty task is Go's omitted non-task target,
/// not a fabricated task identifier or an execution request.
fn action(kind: &str, label: &str, effect: &str, tone: &str) -> MissionPrimaryAction {
    MissionPrimaryAction {
        kind: kind.into(),
        label: label.into(),
        effect: effect.into(),
        task: String::new(),
        tone: tone.into(),
    }
}

/// Project a task's primary navigation with Go's ordered precedence. Inputs are
/// actual mission-task projections; output opens a view/confirmation and grants
/// no launch, recovery or validation. This changes no task or evidence.
pub fn mission_task_primary(task: &MissionTask) -> MissionPrimaryAction {
    let mut primary = action(
        "task",
        &task.label,
        "Ouvre les preuves et les actions autorisées ; aucune reprise immédiate.",
        "attention",
    );
    primary.task = task.id.clone();
    if task.attempt_limit_reached {
        primary.label = "Examiner les tentatives et les refus".into();
    } else if task.state == "review" {
        primary.label = "Examiner le résultat".into();
        if task.validation_mode == "automatic" {
            primary.label = "Voir les contrôles en cours".into();
            primary.tone = "info".into();
        }
    } else if task.action == "prepare" {
        primary.label = "Autoriser le plan dans Préparer".into();
    } else if task.action == "configure" {
        primary.label = "Configurer le lancement".into();
        primary.tone = "info".into();
        primary.effect = "Ouvre les réglages ; aucun agent ne part avant confirmation.".into();
    } else if task.action == "start" {
        primary.effect =
            "Ouvre la confirmation ; le moteur revérifie les conditions avant le départ.".into();
        primary.tone = "info".into();
    } else if task.action == "inspect" && task.state == "intervention" {
        primary.label = "Diagnostiquer et préparer la reprise".into();
    }
    primary
}

/// Planning failures own the next step only when no projected execution,
/// lifecycle or actionable task already owns it. Nil Go slices iterate empty;
/// this does not manufacture a non-nil serialized task collection.
pub fn mission_planning_owns_next_step(status: &MissionStatus) -> bool {
    status.running <= 0
        && status.active_agents <= 0
        && status.review <= 0
        && !status.tasks.as_deref().unwrap_or(&[]).iter().any(|task| {
            matches!(
                task.state.as_str(),
                "running" | "review" | "intervention" | "configure" | "ready" | "manual"
            )
        })
}

/// A missing root inside present planning remains open exactly as Go; absent
/// planning is false. Closed responsibility retains diagnostics as history.
pub fn mission_planning_open(work: &Work) -> bool {
    work.planning.as_ref().is_some_and(|planning| {
        planning
            .scopes
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .find(|scope| scope.id == "root")
            .is_none_or(|scope| scope.state != "closed")
    })
}

/// Select the exact Go navigation priority from real status facts. Stable impact
/// order preserves the first review/intervention on ties. Runtime/organization
/// outrank planning; active/review/manual-ready work prevents a planning failure
/// from stealing its next action. This is pure presentation, never permission.
pub fn mission_guidance(work: &Work, status: &MissionStatus) -> MissionGuidance {
    let mut guidance = MissionGuidance {
        what: status.understanding.what.clone(),
        next: status.understanding.next_step.clone(),
        actor: status.understanding.actor.clone(),
        primary: action("", "", "", ""),
    };
    let tasks = status.tasks.as_deref().unwrap_or(&[]);
    if status.runtime.state == "blocked" {
        guidance.primary = action(
            "runtime",
            "Diagnostic du stockage",
            "Ouvre le diagnostic de la machine ; aucun départ n’est autorisé.",
            "attention",
        );
    } else if !status.organization.ready {
        guidance.primary = action(
            "organization",
            "Préparer l’organisation",
            "Affiche les rôles et les conditions manquantes.",
            "attention",
        );
    } else if mission_planning_open(work)
        && work
            .planning
            .as_ref()
            .is_some_and(|p| !p.failure.is_empty())
        && mission_planning_owns_next_step(status)
    {
        guidance.what = "Le responsable n’a pas terminé sa décision.".into();
        guidance.next =
            "Examinez le diagnostic du responsable avant de reprendre la planification.".into();
        guidance.primary = action(
            "planning",
            "Voir les décisions",
            "Ouvre les décisions et les retours du responsable.",
            "attention",
        );
    } else if mission_planning_open(work) && work.planning.as_ref().is_some_and(|p| p.paused) {
        guidance.primary = action(
            "planning-resume",
            "Reprendre la planification",
            "Demande la reprise des décisions dans les limites déjà autorisées.",
            "attention",
        );
    } else {
        // max_by returns the last tie; Go's stable sort returns the first.
        let mut need: Option<&MissionTask> = None;
        for task in tasks
            .iter()
            .filter(|task| matches!(task.state.as_str(), "review" | "intervention"))
        {
            if need.is_none_or(|first| task.impact > first.impact) {
                need = Some(task);
            }
        }
        if let Some(task) = need {
            guidance.primary = mission_task_primary(task);
            return guidance;
        }
        if let Some(task) = tasks.iter().find(|task| task.state == "configure") {
            guidance.primary = mission_task_primary(task);
            return guidance;
        }
        if let Some(task) = tasks.iter().find(|task| task.state == "running") {
            guidance.primary = action(
                "follow",
                "Suivre cette tâche",
                "Ouvre le journal de cette tâche ; l’agent poursuit son travail.",
                "info",
            );
            guidance.primary.task = task.id.clone();
            return guidance;
        }
        if work.planning.as_ref().is_some_and(|p| {
            p.scopes
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .any(|scope| scope.state != "closed")
        }) {
            guidance.primary = if !status.authorized {
                action(
                    "start",
                    "Lancer la mission",
                    "Ouvre l’aperçu et la confirmation du lancement.",
                    "info",
                )
            } else {
                action(
                    "planning",
                    "Voir les décisions",
                    "Ouvre les décisions et les retours du responsable.",
                    "info",
                )
            };
            return guidance;
        }
        let closed = status.total > 0
            && tasks
                .iter()
                .all(|task| matches!(task.state.as_str(), "validated" | "waived" | "abandoned"));
        guidance.primary = if closed {
            action(
                "results",
                "Voir les résultats",
                "Affiche les résultats et leur validation actuelle.",
                if status.validated == status.total {
                    "succes"
                } else {
                    "attention"
                },
            )
        } else if status.total == 0 {
            action(
                "prepare",
                "Préparer les tâches",
                "Ouvre la préparation du besoin ; aucun agent n’est lancé.",
                "info",
            )
        } else if status.paused {
            action(
                "resume",
                "Reprendre la mission",
                "Autorise les prochains départs dans les limites existantes.",
                "info",
            )
        } else if !status.authorized {
            action(
                "start",
                "Lancer la mission",
                "Ouvre l’aperçu et la confirmation du lancement.",
                "info",
            )
        } else if !status.enabled {
            action(
                "supervision",
                "Comprendre la reprise",
                "Affiche l’état du conducteur et les conditions de reprise.",
                "attention",
            )
        } else {
            action(
                "results",
                "Voir ce qui attend",
                "Affiche les dépendances et les conditions du prochain départ.",
                "info",
            )
        };
    }
    guidance
}

/// Explain one task with Go's branch order and caller-classified agents. First
/// matching agent order is significant. No PID/heartbeat or diagnostic category
/// is inferred here; missing classifications must be resolved by the caller.
pub fn task_understanding(
    task: &MissionTask,
    mission: &MissionStatus,
    agents: &[MissionAgentInsight<'_>],
) -> MissionUnderstanding {
    if task.result.state.starts_with("review_") {
        return understanding(
            &task.result.reason,
            &task.result.next_step,
            if task.result.state == "review_blocked" {
                "Vous"
            } else {
                "Le superviseur"
            },
            if task.result.state == "review_blocked" {
                "user"
            } else {
                "supervisor"
            },
            if task.result.state == "review_blocked" {
                "blocage"
            } else {
                "attente_normale"
            },
        );
    }
    let task_next = || format!("{}.", task.label);
    match task.state.as_str() {
        "validated" => understanding(
            &task.reason,
            "Aucune action requise ; le résultat reste consultable.",
            "Personne pour le moment",
            "none",
            "termine",
        ),
        "waived" | "abandoned" => understanding(
            &task.reason,
            "Consultez la décision si vous devez vérifier sa portée.",
            "Personne pour le moment",
            "none",
            "termine",
        ),
        "review" if task.validation_mode == "automatic" => {
            if mission.paused || !mission.authorized {
                understanding(
                    "Le résultat attend ses contrôles automatiques autorisés.",
                    "Reprenez ou autorisez la mission pour permettre les contrôles.",
                    "Vous",
                    "user",
                    "attente_normale",
                )
            } else if !mission.enabled {
                understanding(
                    "Le résultat attend ses contrôles ; aucun superviseur actif n’est confirmé.",
                    "Vérifiez le superviseur et les conditions de validation.",
                    "Vous",
                    "user",
                    "information_manquante",
                )
            } else {
                understanding("Le résultat relève des contrôles automatiques préautorisés.", "Le superviseur vérifie les conditions avant de contrôler et valider le résultat.", "Le superviseur", "supervisor", "attente_normale")
            }
        }
        "review" => understanding(
            &task.reason,
            task_next(),
            "Vous",
            "user",
            "decision_humaine",
        ),
        "intervention" | "configure" => {
            if task.attempt_limit_reached {
                return understanding(format!("{}/{} tentatives utilisées. {}", task.attempts_used, task.attempts_allowed, task.reason), "Examinez les refus et les preuves conservées. Aucun redémarrage automatique n’est autorisé ; une décision motivée sur la suite est nécessaire.", "Responsable de la mission", "user", "limite_tentatives");
            }
            if task.state == "intervention" {
                if let Some(agent) = agents.iter().find(|agent| agent.task_id == task.id) {
                    if matches!(agent.status, "failed" | "interrupted")
                        && !agent.fallback_diagnostic_limit_reached
                        && agent.environment_recovery
                    {
                        return understanding("L’environnement de l’agent refuse l’accès à une ressource. Cette tâche ne sera pas relancée automatiquement.", "Faire vérifier l’accès signalé dans le diagnostic, puis enregistrer une nouvelle vérification avant de reprendre.", "Vous", "user", "blocage");
                    }
                }
            }
            understanding(
                &task.reason,
                task_next(),
                "Vous",
                "user",
                if matches!(task.action.as_str(), "reopen" | "accepted" | "prepare") {
                    "decision_humaine"
                } else {
                    "blocage"
                },
            )
        }
        "manual" => understanding(
            &task.reason,
            task_next(),
            "Vous",
            "user",
            "decision_humaine",
        ),
        "waiting" => {
            if mission.paused {
                understanding(
                    &task.reason,
                    "Reprenez la mission lorsque vous voulez autoriser la suite.",
                    "Vous",
                    "user",
                    "attente_normale",
                )
            } else if task.action == "supervise" {
                understanding(
                    &task.reason,
                    "Rétablissez le superviseur avant le prochain départ.",
                    "Vous",
                    "user",
                    "information_manquante",
                )
            } else if mission.enabled {
                understanding(
                    &task.reason,
                    "La tâche reprendra automatiquement lorsque la condition sera remplie.",
                    "Le superviseur",
                    "supervisor",
                    "attente_normale",
                )
            } else {
                understanding(
                    &task.reason,
                    "Traitez d’abord le prérequis indiqué, puis revenez à cette tâche.",
                    "Vous",
                    "user",
                    "attente_normale",
                )
            }
        }
        "ready" => understanding(
            &task.reason,
            "Le prochain contrôle lancera la tâche si les conditions restent remplies.",
            "Le superviseur",
            "supervisor",
            "attente_normale",
        ),
        "running" => match agents
            .iter()
            .find(|agent| agent.task_id == task.id && agent.active)
            .map(|agent| agent.observed_state)
        {
            Some("running") => understanding(
                "Un agent travaille sur cette tâche.",
                "Attendez son résultat ou ouvrez sa session pour voir les faits reçus.",
                "L’agent",
                "agent",
                "en_cours",
            ),
            Some("queued" | "starting" | "stopping") => understanding(
                "Une tentative est enregistrée, mais aucun travail en cours n’est affirmé.",
                "Le superviseur vérifie son démarrage ou son arrêt.",
                "Le superviseur",
                "supervisor",
                "attente_normale",
            ),
            Some(_) => understanding(
                "La tâche est marquée en cours, mais l’état de son agent n’est pas confirmé.",
                "Le superviseur doit vérifier la tentative avant toute reprise.",
                "Le superviseur",
                "supervisor",
                "information_manquante",
            ),
            None => understanding(
                "La tâche est marquée en cours, mais aucun agent actif n’est visible.",
                "Le superviseur doit vérifier la tentative avant toute reprise.",
                "Le superviseur",
                "supervisor",
                "information_manquante",
            ),
        },
        _ => understanding(
            &task.reason,
            task_next(),
            "Vous",
            "user",
            "information_manquante",
        ),
    }
}

/// Project base Go mission understanding (before planning/organization/cooldown
/// overrides). Inputs are classified task/result/runtime facts; no execution or
/// financial facts are synthesized. The caller must apply later overrides in
/// missionStatus order and must not confuse a terminal explanation with proof.
pub fn mission_understanding(status: &MissionStatus) -> MissionUnderstanding {
    let tasks = status.tasks.as_deref().unwrap_or(&[]);
    if status.runtime.state == "blocked" {
        return understanding(
            &status.runtime.message,
            &status.runtime.next,
            "Responsable de la machine",
            "user",
            "incident_stockage",
        );
    }
    if status.total == 0 {
        return understanding(
            "Cette mission ne contient aucune tâche.",
            "Préparez les tâches à réaliser.",
            "Vous",
            "user",
            "decision_humaine",
        );
    }
    if status.total == status.validated {
        return understanding(
            "Tous les résultats sont validés sur les preuves actuelles.",
            "Aucune action requise ; les livrables restent consultables.",
            "Personne pour le moment",
            "none",
            "termine",
        );
    }
    if status.review > 0 {
        let mut human = 0_i64;
        let mut automatic = None;
        for task in tasks.iter().filter(|task| task.state == "review") {
            if task.validation_mode == "automatic" {
                automatic = Some(task_understanding(task, status, &[]));
            } else {
                human += 1;
            }
        }
        if human == 0 {
            if let Some(value) = automatic.filter(|value| !value.what.is_empty()) {
                return value;
            }
        }
        return understanding(
            format!("{human} résultat(s) attendent une décision de validation."),
            "Examinez les résultats signalés.",
            "Vous",
            "user",
            "decision_humaine",
        );
    }
    let blocked = tasks
        .iter()
        .filter(|task| task.state == "intervention")
        .count();
    let exhausted = tasks
        .iter()
        .filter(|task| task.attempt_limit_reached)
        .count();
    let configured = tasks
        .iter()
        .filter(|task| task.state == "configure")
        .count();
    if exhausted > 0 && status.running == 0 && status.active_agents == 0 {
        return understanding(format!("{exhausted} tâche(s) ont épuisé leurs tentatives autorisées. Le conducteur ne peut pas les relancer."), "Examinez les tentatives et les refus pour décider de la suite. Attendre ou actualiser ne lève pas cette limite.", "Responsable de la mission", "user", "limite_tentatives");
    }
    if blocked == 1 {
        if let Some(task) = tasks
            .iter()
            .find(|task| task.state == "intervention" && !task.understanding.what.is_empty())
        {
            let mut value = task.understanding.clone();
            value.what = format!("{} — {}", task.title, value.what);
            return value;
        }
    }
    if blocked > 0 {
        return understanding(
            format!("{blocked} tâche(s) ne peuvent pas continuer sans intervention."),
            "Ouvrez les tâches signalées et choisissez la reprise.",
            "Vous",
            "user",
            "blocage",
        );
    }
    if configured > 0 {
        return understanding(
            format!("{configured} tâche(s) attendent leur configuration de lancement."),
            "Préparez la configuration commune.",
            "Vous",
            "user",
            "decision_humaine",
        );
    }
    if status.paused {
        return understanding(
            "La mission est en pause ; aucun nouveau départ automatique n’est autorisé.",
            "Reprenez la mission lorsque vous voulez continuer.",
            "Vous",
            "user",
            "attente_normale",
        );
    }
    if status.running > 0 {
        if status.active_agents > 0 {
            return understanding(
                format!(
                    "{} tâche(s) sont en cours et {} tentative(s) active(s) sont observées.",
                    status.running, status.active_agents
                ),
                "Attendez les résultats ou ouvrez les sessions pour voir les faits reçus.",
                "L’agent",
                "agent",
                "en_cours",
            );
        }
        return understanding(
            format!(
                "{} tâche(s) sont marquées en cours, sans agent actif confirmé.",
                status.running
            ),
            "Le superviseur doit vérifier les tentatives avant toute reprise.",
            "Le superviseur",
            "supervisor",
            "information_manquante",
        );
    }
    if status.enabled {
        return understanding(
            "La mission attend que les conditions du prochain départ soient réunies.",
            "Le superviseur vérifiera et lancera les tâches devenues prêtes.",
            "Le superviseur",
            "supervisor",
            "attente_normale",
        );
    }
    if status.authorized {
        if status.supervision.state == "error" {
            return understanding(
                "La mission est autorisée, mais la dernière vérification du superviseur a échoué.",
                "Consultez l’erreur et la prochaine vérification annoncée.",
                "Le superviseur",
                "supervisor",
                "information_manquante",
            );
        }
        return understanding(
            "La mission est autorisée, mais aucun superviseur actif n’est observé.",
            "Démarrez le serveur web ou la veille de mission.",
            "Vous",
            "user",
            "information_manquante",
        );
    }
    understanding(
        "La suite automatique n’est pas autorisée.",
        "Lancez une tâche ou autorisez la mission continue.",
        "Vous",
        "user",
        "decision_humaine",
    )
}

/// Apply only Go's planning override stage to an already-produced understanding
/// and next explanation, in memory. Planning absence leaves both unchanged.
/// Organization, cooldown and final runtime overrides still belong to the caller
/// afterwards; no policy change, reservation, action or persisted write occurs.
pub fn apply_planning_understanding(work: &Work, status: &mut MissionStatus) {
    let Some(planning) = work.planning.as_ref() else {
        return;
    };
    let root = planning
        .scopes
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .find(|scope| scope.id == "root");
    let open = root.is_none_or(|scope| scope.state != "closed");
    if open && !planning.failure.is_empty() && mission_planning_owns_next_step(status) {
        status.understanding = understanding(
            &planning.failure,
            "Corrigez la cause puis reprenez la planification.",
            "Vous",
            "user",
            "decision_humaine",
        );
    } else if root.is_some_and(|scope| scope.state != "closed")
        && (planning.activations >= planning.max_activations
            || planning.decisions >= planning.max_decisions)
    {
        status.understanding = understanding("Le plafond de planification est atteint.", "Examinez les décisions et préparez une nouvelle mission bornée pour le travail restant.", "Vous", "user", "decision_humaine");
    } else if open && planning.paused {
        status.understanding = understanding(
            "La planification est en pause.",
            "Reprenez lorsque vous êtes prêt.",
            "Vous",
            "user",
            "decision_humaine",
        );
    } else if root.is_some_and(|scope| scope.state != "closed")
        && (status.total == 0 || status.total == status.validated)
    {
        status.understanding = understanding(
            "Les résultats attendent une décision du responsable du périmètre.",
            "Le responsable examine les retours avant de clore ou compléter le plan.",
            "Le planificateur",
            "supervisor",
            "attente_normale",
        );
    }
    if status.understanding.situation != "termine" {
        status.next = status.understanding.next_step.clone();
    }
}

/// Produce the exact two Go evidence-stage statements. Validated tasks alone,
/// zero tasks, missing planning or an open/missing root never assert closure.
/// Counters must be the real status reduction; this text accepts no evidence.
pub fn mission_evidence_stage(work: &Work, total: i64, validated: i64) -> String {
    if work.planning.as_ref().is_some_and(|planning| {
        total > 0
            && total == validated
            && planning
                .scopes
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .find(|scope| scope.id == "root")
                .is_some_and(|scope| scope.state == "closed")
    }) {
        "Résultats validés et responsabilité racine clôturée dans cette mission.".into()
    } else {
        "Parcours de cette mission non encore vérifié jusqu’à la clôture. Une recette isolée ou une installation ne vaut pas cette preuve.".into()
    }
}

/// Build a coordination phase's known Go zero strings before its branch fills
/// summary/actor/next. Empty optional at/relative mean no timestamp was produced,
/// not a fabricated observation. No input or persisted object is changed.
fn phase(kind: &str, label: &str) -> MissionCoordinationPhase {
    MissionCoordinationPhase {
        kind: kind.into(),
        label: label.into(),
        summary: String::new(),
        actor: String::new(),
        next_step: String::new(),
        at: String::new(),
        relative: String::new(),
    }
}

/// Render Go supervisionPastRelative using explicit time/policy input. Minute and
/// hour units/nearest rounding are existing presentation rules, not execution
/// waits. Match Go time.Time.Sub's i64-nanosecond saturation before rounding.
/// Future tolerance must be supplied by the caller's existing policy, never a
/// local default. Invalid exchange timestamps are handled separately by Go's
/// explicit empty-relative branch in the coordination projection.
fn past_relative(at: OffsetDateTime, stamp: OffsetDateTime, future_tolerance: Duration) -> String {
    let age = (at.unix_timestamp_nanos() - stamp.unix_timestamp_nanos())
        .clamp(i128::from(i64::MIN), i128::from(i64::MAX));
    let tolerance = i128::from(future_tolerance.as_secs()) * 1_000_000_000
        + i128::from(future_tolerance.subsec_nanos());
    let minute = 60_000_000_000_i128;
    let hour = 3_600_000_000_000_i128;
    if age < -tolerance {
        "horloge incohérente".into()
    } else if age < minute {
        "à l’instant".into()
    } else if age < hour {
        format!("il y a environ {} min", (age + minute / 2) / minute)
    } else {
        format!("il y a environ {} h", (age + hour / 2) / hour)
    }
}

/// Produce Go's four phases from real projected task facts and exchanges in
/// ascending `(created_at,id)` producer order. Prefer the last unresolved
/// exchange, else the last historical exchange; missing task names preserve the
/// stored ID. The explicit clock/tolerance perform no clock IO or launch grant.
/// The shared Go parser accepts historical spellings while retaining original
/// created_at bytes. An invalid timestamp leaves relative empty as Go does.
/// Completed missions retain exchange history without new recipient instructions.
/// No SQL, mutation, acknowledgement, timeout or automatic acceptance occurs.
pub fn mission_coordination_phases(
    work: &Work,
    status: &MissionStatus,
    exchanges: &[MissionStatusExchange],
    at: OffsetDateTime,
    future_tolerance: Duration,
) -> Vec<MissionCoordinationPhase> {
    let mut phases = vec![
        phase("work", "Travail"),
        phase("wait", "Attente"),
        phase("exchange", "Échanges"),
        phase("validation", "Vérification"),
    ];
    if status.running > 0 {
        phases[0].summary = format!(
            "{} tâche(s) en cours ; {} agent(s) actif(s) confirmé(s).",
            status.running, status.active_agents
        );
        phases[0].actor = "Les agents".into();
        phases[0].next_step = "Attendre le résultat ou ouvrir leur journal.".into();
    } else {
        phases[0].summary = "Aucune tâche n’est actuellement déclarée en cours.".into();
        phases[0].actor = "Personne pour le moment".into();
        phases[0].next_step = "Suivre la prochaine action indiquée.".into();
    }
    let tasks = status.tasks.as_deref().unwrap_or(&[]);
    let waiting: Vec<_> = tasks
        .iter()
        .filter(|task| matches!(task.state.as_str(), "waiting" | "ready"))
        .collect();
    if let Some(first) = waiting.first() {
        phases[1].summary = format!("{} — {}", first.title, first.reason);
        if waiting.len() > 1 {
            phases[1]
                .summary
                .push_str(&format!(" · {} autre(s) attente(s)", waiting.len() - 1));
        }
        phases[1].actor = first.understanding.actor.clone();
        phases[1].next_step = first.understanding.next_step.clone();
    } else {
        phases[1].summary = "Aucune attente normale signalée.".into();
        phases[1].actor = "Personne pour le moment".into();
        phases[1].next_step = "Aucune résolution d’attente requise.".into();
    }
    if let Some(exchange) = exchanges
        .iter()
        .rev()
        .find(|exchange| {
            matches!(
                exchange.state.as_str(),
                "pending" | "acknowledged" | "escalated" | "stale"
            )
        })
        .or_else(|| exchanges.last())
    {
        let title = |id: &str| {
            work.tasks
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .find(|task| task.id == id)
                .map(|task| task.title.clone())
                .unwrap_or_else(|| id.into())
        };
        let source = title(&exchange.source_task_id);
        let recipient = title(&exchange.recipient_task_id);
        let action = match exchange.kind.as_str() {
            "handoff" => "remet un résultat à",
            "help_request" => "demande de l’aide à",
            "help_answer" => "répond à",
            _ => "échange avec",
        };
        let state_label = match exchange.state.as_str() {
            "pending" => "à traiter",
            "acknowledged" => "pris en charge",
            "consumed" => "traité",
            "answered" => "réponse envoyée",
            "escalated" => "délai dépassé",
            "stale" => "résultat périmé",
            _ => "à examiner",
        };
        phases[2].summary = format!("{source} {action} {recipient} · {state_label}.");
        if !exchange.need.is_empty() {
            phases[2].summary.push(' ');
            phases[2].summary.push_str(&exchange.need);
        }
        phases[2].actor = recipient;
        phases[2].next_step = "Le destinataire doit traiter cet échange.".into();
        phases[2].at = exchange.created_at.clone();
        match exchange.state.as_str() {
            "consumed" | "answered" => {
                phases[2].actor = "Personne pour cet échange".into();
                phases[2].next_step = "Échange terminé ; aucune action supplémentaire.".into();
            }
            "acknowledged" => {
                phases[2].next_step =
                    "Le destinataire a pris la demande en charge ; sa réponse est attendue.".into();
            }
            "escalated" => {
                phases[2].actor = "Vous".into();
                phases[2].next_step =
                    "La demande n’a pas reçu de réponse à temps ; examiner le besoin.".into();
            }
            "stale" => {
                phases[2].actor = source;
                phases[2].next_step = "Fournir un résultat à jour avant de poursuivre.".into();
            }
            _ => {}
        }
        if let Ok(stamp) = swarm_wire_json::go_rfc3339::parse(&exchange.created_at) {
            phases[2].relative = past_relative(at, stamp, future_tolerance);
        }
    } else {
        phases[2].summary = "Aucun échange structuré enregistré.".into();
        phases[2].actor = "Personne pour le moment".into();
        phases[2].next_step = "Les remises et demandes d’aide apparaîtront ici.".into();
    }
    if status.understanding.situation == "termine" && !exchanges.is_empty() {
        phases[2].summary = "Échanges conservés dans l’historique de la mission terminée.".into();
        phases[2].actor = "Personne pour ces échanges".into();
        phases[2].next_step =
            "Aucune action requise ; les messages et leurs états restent consultables.".into();
    }
    let checking = tasks
        .iter()
        .filter(|task| task.validation_mode == "automatic" && task.state == "review")
        .count();
    if checking > 0 {
        phases[3].summary =
            format!("{checking} résultat(s) attendent leurs contrôles automatiques autorisés.");
        phases[3].actor = "Le superviseur".into();
        phases[3].next_step =
            "Exécuter les contrôles puis publier une décision avec des preuves fraîches.".into();
    } else if status.review > 0 {
        phases[3].summary = format!("{} résultat(s) attendent une revue humaine.", status.review);
        phases[3].actor = "Vous".into();
        phases[3].next_step = "Examiner le résultat et ses preuves.".into();
    } else {
        phases[3].summary = "Aucun résultat n’attend actuellement de vérification.".into();
        phases[3].actor = "Personne pour le moment".into();
        phases[3].next_step = "Les contrôles apparaîtront après une remise de résultat.".into();
    }
    phases
}

#[cfg(test)]
mod tests;
