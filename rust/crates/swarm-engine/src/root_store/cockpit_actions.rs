//! Cockpit action oracle: read-only UI projection of the human actions the
//! engine offers for one task (port of Go `engine.taskActions`).
//!
//! Consumed by the web cockpit, the TUI console and the AI assistant. The
//! engine store remains the final arbiter of every transition; this module
//! only mirrors its rules so no operator discovers a refused transition after
//! the fact.
//!
//! Inputs come from the private Root reader: the native [`crate::Task`], the
//! native [`crate::Agent`] rows and a [`Facts`] record of store-derived
//! predicates (start conditions, gate currency, prepared launch, recovery
//! advisories, ...). Every fact is an [`Observation`]: the reader observes
//! what the engine state actually shows and marks the rest
//! [`Observation::Unobserved`]. [`project`] demands a value exactly where the
//! task/agent branch at hand can consume it; a consumed-but-unobserved fact
//! fails with `RootStoreError::Refused("cockpit-action-fact-unobserved")`, so
//! no unknown input is ever read as a guard pass, an empty reason, a silent
//! denial or a successful empty action list. Facts the branch cannot reach
//! (gate currency of a todo task, legacy repair of a todo or blocked task,
//! attempt-cap display facts outside a blocked status, ...) may stay
//! unobserved. The projection stays pure and fallible: no SQL, no filesystem,
//! no process access, no actor authentication, no side effects — decoding
//! these inputs grants no identity and no execution authority.
//!
//! Outputs are preliminary read-only UI facts, never operation admission.
//! [`project`] must be called with the FULL agent slice of the work: activity,
//! stop availability and the last finished attempt are computed from the
//! passed rows only, so a truncated slice silently corrupts every availability
//! and the recommendation. A single `project` result is one task's action
//! list — never present it as a complete top-level cockpit.
//!
//! Documented deviations from the Go original, all channelled through
//! [`Facts`] or computed from the passed rows:
//! - the per-agent desired state is read from the row's `desired` field
//!   instead of a store lookup;
//! - `gateFiles`, `validGate`, `assistCanStart`, `startBlockReason`,
//!   `legacyReportSubmission` (repairable flag), `independentReviewGuard`,
//!   `requiresEnvironmentVerification` (evaluated on the last finished row),
//!   `preparedLaunchForTask`, `attemptExtensionReason` and
//!   `correctiveRecoveryReason` are supplied as facts;
//! - every fact arrives as an [`Observation`]; `project` requires the
//!   observed value only where the current branch consumes it and refuses
//!   with `RootStoreError::Refused("cockpit-action-fact-unobserved")`
//!   otherwise (each [`Facts`] field documents its consumption branch);
//! - the display thresholds `status == "blocked" && PlanMaxAttempts > 0 &&
//!   len(Attempts) >= PlanMaxAttempts` and the nested `PlanMaxAttempts >= 3`
//!   arrive as `show_attempt_extension` / `show_corrective_recovery`; no
//!   threshold constant is reintroduced here;
//! - dependency override reasons come from the Root reader's actual current
//!   accepted/waived-fresh observations; listing a dependency alone is not refusal.

use serde::Serialize;

use super::RootStoreError;

/// Return true only for a false bool, reproducing Go bool `omitempty`.
fn is_false(value: &bool) -> bool {
    !*value
}

/// Return true only for numeric zero, reproducing Go integer `omitempty`.
fn is_zero(value: &i64) -> bool {
    *value == 0
}

/// Champ demandé à l'humain pour une action : tout champ porte un libellé
/// explicite et une aide (décision de cadrage 2026-09-12). Wire-compatible
/// with Go `TaskField`: field order, tags and `omitempty`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct TaskField {
    pub(crate) name: String,
    pub(crate) label: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) aide: String,
    #[serde(skip_serializing_if = "is_false")]
    pub(crate) requis: bool,
}

/// Action proposée sur une tâche : disponibilité, motif d'indisponibilité,
/// statut conseillé pour l'état courant et champs attendus. Wire-compatible
/// with Go `TaskAction`: field order, tags and `omitempty`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct TaskAction {
    #[serde(rename = "prepared_launch", skip_serializing_if = "Option::is_none")]
    pub(crate) prepared: Option<PreparedLaunch>,
    pub(crate) kind: String,
    pub(crate) label: String,
    pub(crate) disponible: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) raison: String,
    #[serde(skip_serializing_if = "is_false")]
    pub(crate) conseillee: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) champs: Option<Vec<TaskField>>,
}

/// Lancement préparé en attente de reprise. Wire-compatible with Go
/// `PreparedLaunch`: field order, tags and `omitempty`.
#[derive(Clone, PartialEq, Serialize)]
pub(crate) struct PreparedLaunch {
    pub(crate) id: String,
    pub(crate) task: String,
    pub(crate) ready: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) reason: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) provider: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) workspace: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) instruction: String,
    #[serde(skip_serializing_if = "is_zero")]
    pub(crate) timeout_seconds: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) limits: Option<crate::models::RunLimits>,
}

impl std::fmt::Debug for PreparedLaunch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreparedLaunch { data: redacted }")
    }
}


/// Valeur de fait fournie par le lecteur Root : soit observée
/// ([`Observation::Observed`]), soit explicitement non observée
/// ([`Observation::Unobserved`]). Pas de `Default`, pas de `Deserialize` :
/// une valeur ne peut être ni devinée ni reconstruite depuis les défauts
/// du fil. [`project`] n'exige la valeur observée que là où la branche
/// courante consomme le fait ; consommer un fait non observé refuse la
/// projection au lieu d'inventer un passe-droit, un motif vide ou un
/// refus silencieux.
#[derive(Clone, Debug)]
pub(crate) enum Observation<T> {
    Observed(T),
    Unobserved,
}

/// Extrait la valeur d'un fait observé, ou refuse explicitement la
/// projection : un fait consommé mais non observé ne peut être ni deviné,
/// ni remplacé par un défaut silencieux. Le message de refus est fixe ;
/// `.into()` couvre le type de charge utile déclaré par le module parent.
fn observed<T>(fact: Observation<T>) -> Result<T, RootStoreError> {
    match fact {
        Observation::Observed(value) => Ok(value),
        Observation::Unobserved => Err(RootStoreError::Refused(
            "cockpit-action-fact-unobserved".into(),
        )),
    }
}

/// Store-derived inputs for one projection call, supplied by the private Root
/// reader. Every field is an [`Observation`]: the reader observes what the
/// engine state actually shows and leaves everything else
/// [`Observation::Unobserved`]. [`project`] requires a value only where the
/// branch at hand consumes it and refuses with [`RootStoreError::Refused`]
/// otherwise. No `Default` and no `Deserialize` on purpose — no value may be
/// reconstructed from wire defaults, and an absent observation is never read
/// as a guard pass, an empty reason or a denial.
pub(crate) struct Facts {
    /// Engine mirror of `assistCanStart` for this task. Consumed as soon as
    /// the start reason is still empty, a retry reaches its start-conditions
    /// branch, or a pending prepared launch is rendered — always the case
    /// for an initial todo task with no agents.
    pub(crate) can_start: Observation<bool>,
    /// Engine mirror of `startBlockReason`. Consumed only when a real refusal
    /// reason must be rendered: refused start (empty reason with `can_start`
    /// false), held launch, retry blocked on start conditions, or a pending
    /// launch that cannot start. Otherwise it may stay unobserved.
    pub(crate) start_block_reason: Observation<String>,
    /// `len(gateFiles(task)) > 0`: a gate document is listed for the task.
    /// Consumed only by the "submitted" recommendation (gate vs report).
    pub(crate) gate_files_present: Observation<bool>,
    /// `validGate(task)`: the recorded gate delivery is still current.
    /// Consumed only by the submitted acceptance guard (the Go recommendation
    /// table never reads `acceptedFresh`).
    pub(crate) gate_current: Observation<bool>,
    /// `independentReviewGuard` refusal text, when the guard refuses.
    /// Consumed only for a "submitted" task.
    pub(crate) independent_review_reason: Observation<Option<String>>,
    /// First actual unresolved/nonfresh dependency reason from the Root reader.
    /// None means every listed dependency was checked current, not inferred empty.
    /// Consumed unless a running/active attempt or a waived/abandoned status
    /// short-circuits the override guard — for an initial todo task it is
    /// consumed.
    pub(crate) override_dependency_reason: Observation<Option<String>>,
    /// Whether `legacyReportSubmission` finds a repairable legacy submission.
    /// Consumed only when no attempt is active and the status is neither todo
    /// nor blocked (the Go submit exemption).
    pub(crate) legacy_submission_repairable: Observation<bool>,
    /// `requiresEnvironmentVerification` evaluated on the last finished
    /// attempt row; consumed only when such a row exists in `agents` and the
    /// status is not "running" (that branch rewrites the start reason anyway).
    pub(crate) last_finished_requires_environment_verification: Observation<bool>,
    /// `preparedLaunchForTask` result while a launch is pending for the task.
    /// Consumed whenever no attempt is active; `Observed(None)` must be proven
    /// by the Root reader (e.g. `Work.planning` absent), never inferred.
    pub(crate) prepared: Observation<Option<PreparedLaunch>>,
    /// Display predicate `status == "blocked" && PlanMaxAttempts > 0 &&
    /// len(Attempts) >= PlanMaxAttempts` (Go inline threshold, not re-hardcoded).
    /// Consumed only for a blocked task: the Go predicate begins with the
    /// status test, so elsewhere the fact cannot change the result.
    pub(crate) show_attempt_extension: Observation<bool>,
    /// Nested display predicate `PlanMaxAttempts >= 3` (Go inline threshold,
    /// not re-hardcoded). Consumed only for a blocked task whose extension
    /// display predicate was observed true.
    pub(crate) show_corrective_recovery: Observation<bool>,
    /// `attemptExtensionReason` result; empty means the extension is offerable.
    /// Consumed only for a blocked task (extend-attempt availability and the
    /// blocked recommendation).
    pub(crate) attempt_extension_reason: Observation<String>,
    /// `correctiveRecoveryReason` result; empty means the recovery is offerable.
    /// Consumed only for a blocked task: per the supplied Go source the reason
    /// is never empty for a non-blocked status, so the recommendation switch
    /// cannot fire elsewhere.
    pub(crate) corrective_recovery_reason: Observation<String>,
}

/// Construit un champ demandé (miroir du helper Go `champ`).
fn champ(name: &str, label: &str, aide: &str, requis: bool) -> TaskField {
    TaskField {
        name: name.to_string(),
        label: label.to_string(),
        aide: aide.to_string(),
        requis,
    }
}

/// Construit une action (miroir du helper Go `action`).
fn action(kind: &str, label: &str, disponible: bool, raison: &str) -> TaskAction {
    TaskAction {
        prepared: None,
        kind: kind.to_string(),
        label: label.to_string(),
        disponible,
        raison: raison.to_string(),
        conseillee: false,
        champs: None,
    }
}

/// Statut actif d'une tentative : miroir de l'ensemble utilisé par les gardes
/// SQL du store (`queued`, `starting`, `running`, `stopping`) ; évalué sur la
/// ligne fournie, rien n'est déduit au-delà.
fn active_agent(agent: &crate::Agent) -> bool {
    matches!(
        agent.status.as_str(),
        "queued" | "starting" | "running" | "stopping"
    )
}

/// Table de vérité du design : l'action pertinente pour l'état courant
/// (miroir de `conseilleePour`). Le paramètre Go `acceptedFresh` n'y est
/// jamais lu — une acceptation périmée conseille déjà la réouverture — et
/// n'est donc pas transporté ici : `gate_current` n'est consommé que par la
/// garde d'acceptation d'une tâche « Soumise ».
fn conseillee_pour(status: &str, task_active: bool, gates: bool) -> &'static str {
    match status {
        "todo" => "start",
        "running" => {
            if task_active {
                "stop"
            } else {
                "reconcile"
            }
        }
        "submitted" => {
            if gates {
                "gate"
            } else {
                "report"
            }
        }
        "accepted" | "waived" | "abandoned" => "reopen",
        "blocked" => "start",
        _ => "",
    }
}

/// Champs demandés par action ; `None` reproduit l'absence de clé dans la map
/// Go (aucun champ pour accepted, reopen, extend-attempt, authorize-recovery
/// et resume-launch : la décision se prend après lecture de l'aperçu).
fn champs_for(kind: &str) -> Option<Vec<TaskField>> {
    match kind {
        "start" => Some(vec![
            champ("provider", "Fournisseur", "Programme agent à lancer (défini dans .swarm/providers.json).", true),
            champ("role", "Rôle", "Worker réalise la tâche ; planner et subplanner planifient sans coder.", false),
            champ("workspace", "Espace de travail", "Répertoire dans lequel l'agent travaillera.", true),
            champ("instruction", "Consigne pour l'agent", "Transmise telle quelle ; préremplie avec la prochaine action prévue.", true),
            champ("capture", "Capture détaillée des sorties", "Désactivée par défaut : activité structurée seulement.", false),
        ]),
        "retry" => Some(vec![
            champ("agent", "Tentative à relancer", "Reprend le fournisseur et l'espace de travail de cette tentative.", true),
            champ("precondition_evidence", "Vérification nouvelle des préconditions", "Obligatoire après un échec d’environnement : contrôle observé des droits, du montage ou de la ressource. Ne désactivez aucune protection.", false),
            champ("instruction", "Consigne pour l'agent", "Ajoutée au handoff de la tentative précédente.", false),
            champ("capture", "Capture détaillée des sorties", "Désactivée par défaut : activité structurée seulement.", false),
        ]),
        "stop" => Some(vec![champ(
            "agent",
            "Tentative à arrêter",
            "Demande d'arrêt ; vérifiez ensuite l'état observé.",
            true,
        )]),
        "reconcile" => Some(vec![champ(
            "agent",
            "Tentative à réconcilier",
            "Rapproche l'état enregistré du processus réellement observé.",
            true,
        )]),
        "report" => Some(vec![champ(
            "path",
            "Rapport à lire",
            "Fichier listé parmi les rapports détectés.",
            true,
        )]),
        "submit" => Some(vec![champ(
            "path",
            "Chemin du rapport soumis",
            "Fichier produit par l'agent, listé parmi les rapports détectés.",
            true,
        )]),
        "gate" => Some(vec![
            champ("path", "Document de gate (preuve d'évaluation)", "Fichier .evidence.json calculé par le moteur ; aperçu avant enregistrement.", true),
            champ("name", "Nom de la gate", "Nom humain qui identifie cette évaluation dans la console et l'assistant.", true),
        ]),
        "override" => Some(vec![champ(
            "note",
            "Motif de la dérogation",
            "Conservé avec votre compte local et la date ; les contrôles restent non PASS.",
            true,
        )]),
        "assign" => Some(vec![champ(
            "owner",
            "Responsable",
            "Humain ou fournisseur chargé du prochain geste sur la tâche.",
            false,
        )]),
        _ => None,
    }
}

/// Calcule les actions disponibles pour une tâche, leur motif et l'action
/// conseillée (miroir de `taskActions`). Ordre de sortie Go : start, retry,
/// stop, reconcile, report, submit, gate, accepted, override, reopen, assign,
/// puis extend-attempt / authorize-recovery quand affichés, puis
/// resume-launch quand un lancement est préparé.
///
/// Projection pure, faillible et sans effet de bord : aucune entrée/sortie,
/// aucune authentification, aucun droit — décoder ces entrées n'admet aucune
/// opération et le store reste l'arbitre final de chaque transition. Un fait
/// [`Observation::Unobserved`] que la branche courante consomme fait échouer
/// l'appel avec [`RootStoreError::Refused`] : aucun défaut substitué, aucune
/// action vide présentée comme un succès.
///
/// `agents` doit contenir toutes les lignes de la tâche (slice complète
/// possédée par l'appelant) : l'activité, la disponibilité de l'arrêt et la
/// dernière tentative terminée sont calculés uniquement à partir des lignes
/// passées ; une slice tronquée corrompt silencieusement chaque
/// disponibilité et la recommandation. Le résultat reste une liste
/// préliminaire en lecture seule pour UNE tâche, jamais un cockpit complet.
pub(crate) fn project(
    task: &crate::Task,
    agents: &[crate::Agent],
    facts: Facts,
) -> Result<Vec<TaskAction>, RootStoreError> {
    let Facts {
        can_start,
        start_block_reason,
        gate_files_present,
        gate_current,
        independent_review_reason,
        override_dependency_reason,
        legacy_submission_repairable,
        last_finished_requires_environment_verification,
        prepared,
        show_attempt_extension,
        show_corrective_recovery,
        attempt_extension_reason,
        corrective_recovery_reason,
    } = facts;

    // Activité et dernière tentative terminée : calculées sur les lignes
    // passées, rien d'autre.
    let mut task_active = false;
    let mut stop_available = false;
    for agent in agents {
        if agent.task_id == task.id && active_agent(agent) {
            task_active = true;
            if agent.desired != "stop" {
                stop_available = true;
            }
        }
    }
    let last_finished = agents
        .iter()
        .find(|agent| agent.task_id == task.id && !active_agent(agent));
    let has_finished_attempt = last_finished.is_some();

    let accepted = task.status == "accepted";

    // Drapeau d'environnement : consommé seulement si une tentative terminée
    // existe et que la branche « running » ne réécrit pas ensuite le motif ;
    // le repli n'est jamais lu hors de cette condition.
    let last_finished_environment_failure = if has_finished_attempt && task.status != "running" {
        observed(last_finished_requires_environment_verification)?
    } else {
        false
    };

    let mut start_raison = match task.status.as_str() {
        "accepted" => "Tâche acceptée : rouvrez-la (action conseillée) avant de relancer un agent.",
        "waived" => "Tâche acceptée par dérogation : rouvrez-la avant de relancer un agent.",
        "abandoned" => "Tâche abandonnée : rouvrez-la avant de relancer un agent.",
        _ => "",
    }
    .to_string();
    if has_finished_attempt && last_finished_environment_failure {
        start_raison = "Échec d’environnement identifié : utilisez la reprise et décrivez une vérification nouvelle des préconditions.".to_string();
    }
    if task.status == "running" {
        start_raison = if task_active {
            "Un agent est actif sur cette tâche : attendre sa fin ou l'arrêter avant de relancer."
        } else {
            "Aucun agent actif : réconciliez l'état observé avant de relancer."
        }
        .to_string();
    }

    // Lancement préparé : consommé dès qu'aucune tentative n'est active (Go
    // consulte preparedLaunchForTask dans ce cas, quel que soit le statut) ;
    // Observed(None) doit être prouvé par le lecteur Root, jamais déduit.
    let pending_prepared: Option<PreparedLaunch> = if !task_active {
        observed(prepared)?
    } else {
        None
    };

    // Branche « relance » qui consulte les conditions de départ.
    let retry_checks_start = !task.launch_held && !task_active && has_finished_attempt;

    // can_start : requis dès qu'un motif de départ vide, une relance
    // conditionnée ou une reprise de lancement peut en dépendre ; le repli
    // `false` n'est lu que dans des positions déjà déterminées sans ce fait,
    // jamais comme un passe-droit.
    let can_start = if start_raison.is_empty()
        || retry_checks_start
        || pending_prepared
            .as_ref()
            .map_or(false, |pending| pending.ready || pending.reason.is_empty())
    {
        observed(can_start)?
    } else {
        false
    };

    // Raison de blocage : requise seulement quand un motif de refus réel doit
    // être restitué (départ refusé, lancement tenu, relance conditionnée,
    // reprise non prête) ; une valeur non observée n'est jamais déguisée en
    // chaîne vide silencieuse.
    let start_block_needed = task.launch_held
        || (start_raison.is_empty() && !can_start)
        || (retry_checks_start && !can_start)
        || pending_prepared
            .as_ref()
            .map_or(false, |pending| pending.reason.is_empty() && !can_start);
    let start_block_reason = if start_block_needed {
        observed(start_block_reason)?
    } else {
        String::new()
    };
    if start_raison.is_empty() && !can_start {
        start_raison = start_block_reason.clone();
    }

    let mut retry_dispo = false;
    let mut retry_raison = String::new();
    if task.launch_held {
        retry_raison = start_block_reason.clone();
    } else if task_active {
        retry_raison = "Une tentative est en cours : attendez sa fin ou arrêtez-la.".to_string();
    } else if !has_finished_attempt {
        retry_raison = "Aucune tentative à relancer : choisir Lancer un agent.".to_string();
    } else if !can_start {
        retry_raison = start_block_reason.clone();
    } else {
        retry_dispo = true;
    }

    let stop_raison = if !task_active {
        "Aucune tentative active sur cette tâche."
    } else if !stop_available {
        "Arrêt déjà demandé ; confirmation du superviseur attendue."
    } else {
        ""
    };

    let reconcile_raison = if !task_active {
        "Aucune tentative active sur cette tâche."
    } else {
        ""
    };

    // Réparation legacy : consommée seulement sans tentative active et hors
    // exemption Go « todo »/« bloquée » de la soumission.
    let legacy_submission_repairable = if !task_active
        && task.status != "todo"
        && task.status != "blocked"
    {
        observed(legacy_submission_repairable)?
    } else {
        false
    };

    let mut submit_raison = String::new();
    if task_active {
        submit_raison = "Une tentative est en cours : attendez sa fin avant de soumettre un rapport.".to_string();
    } else if task.status != "todo" && task.status != "blocked" && !legacy_submission_repairable {
        submit_raison = "Un rapport se soumet depuis une tâche « À faire » ou « Bloquée » qui vient de produire son livrable.".to_string();
    }

    let mut gate_raison = String::new();
    if task.status == "accepted" || task.status == "waived" || task.status == "abandoned" {
        gate_raison = "Tâche validée : rouvrez-la avant une nouvelle évaluation.".to_string();
    } else if task.revalidation.is_some() && task.status != "submitted" {
        gate_raison = "Revalidation en attente : soumettez un nouveau rapport avant la gate.".to_string();
    }

    // Gate et revue indépendante : consommées seulement pour une tâche
    // « Soumise » (conseil gate/rapport et garde d'acceptation) ; ailleurs
    // elles ne peuvent pas changer le résultat.
    let gate_files_present = if task.status == "submitted" {
        observed(gate_files_present)?
    } else {
        false
    };
    let gate_current = if task.status == "submitted" {
        observed(gate_current)?
    } else {
        false
    };
    let independent_review_reason = if task.status == "submitted" {
        observed(independent_review_reason)?
    } else {
        None
    };

    let mut accept_raison = String::new();
    if task.status != "submitted" {
        accept_raison = "Un rapport doit être soumis avant l'acceptation.".to_string();
    } else if !gate_current {
        accept_raison = "Examinez puis enregistrez une gate delivery avant d'accepter.".to_string();
    }
    if task.status == "submitted" {
        if let Some(reason) = independent_review_reason {
            accept_raison = reason;
        }
    }

    // Raison de dérogation : consommée sauf si une tentative active/running
    // ou un statut déjà dérogé/abandonné court-circuite la garde Go.
    let override_dependency_reason = if !(task_active || task.status == "running")
        && task.status != "waived"
        && task.status != "abandoned"
    {
        observed(override_dependency_reason)?
    } else {
        None
    };

    let mut override_raison = String::new();
    if task_active || task.status == "running" {
        override_raison = "Arrêtez et réconciliez la tentative avant dérogation.".to_string();
    } else if task.status == "waived" {
        override_raison = "Dérogation déjà enregistrée : rouvrez la tâche pour la modifier.".to_string();
    } else if task.status == "abandoned" {
        override_raison = "Rouvrez la tâche abandonnée avant dérogation.".to_string();
    } else if let Some(reason) = override_dependency_reason {
        override_raison = reason;
    }

    let reopen_raison = if !accepted && task.status != "waived" && task.status != "abandoned" {
        "Seule une tâche acceptée, dérogée ou abandonnée peut être rouverte."
    } else {
        ""
    };

    // Faits d'affichage extension/récupération : le prédicat Go encapsulé
    // commence par `status == "blocked"`, donc hors blocage ils ne peuvent
    // pas changer le résultat et peuvent rester non observés ; pour une
    // tâche bloquée chaque fait consommé est exigé. Les drapeaux restent des
    // observations fournies : aucun seuil n'est réintroduit ici.
    let (show_attempt_extension, show_corrective_recovery, attempt_extension_reason, corrective_recovery_reason) =
        if task.status == "blocked" {
            let show_attempt_extension = observed(show_attempt_extension)?;
            let show_corrective_recovery = if show_attempt_extension {
                observed(show_corrective_recovery)?
            } else {
                false
            };
            (
                show_attempt_extension,
                show_corrective_recovery,
                observed(attempt_extension_reason)?,
                observed(corrective_recovery_reason)?,
            )
        } else {
            (false, false, String::new(), String::new())
        };

    let mut actions = vec![
        action("start", "Lancer un agent", can_start && start_raison.is_empty(), &start_raison),
        action("retry", "Relancer une tentative", retry_dispo, &retry_raison),
        action("stop", "Arrêter la tentative", stop_available, stop_raison),
        action("reconcile", "Réconcilier l'état observé", task_active, reconcile_raison),
        action("report", "Lire le rapport", true, ""),
        action("submit", "Soumettre le rapport", submit_raison.is_empty(), &submit_raison),
        action("gate", "Examiner et enregistrer une gate", gate_raison.is_empty(), &gate_raison),
        action("accepted", "Accepter après revue", accept_raison.is_empty(), &accept_raison),
        action("override", "Accepter par dérogation motivée", override_raison.is_empty(), &override_raison),
        action("reopen", "Rouvrir la tâche", reopen_raison.is_empty(), reopen_raison),
        action("assign", "Changer le responsable", true, ""),
    ];

    // Seuils d'affichage fournis (aucune constante réintroduite ici).
    if show_attempt_extension {
        actions.push(action(
            "extend-attempt",
            "Autoriser une tentative supplémentaire",
            attempt_extension_reason.is_empty(),
            &attempt_extension_reason,
        ));
        if show_corrective_recovery {
            actions.push(action(
                "authorize-recovery",
                "Préparer un essai correctif",
                corrective_recovery_reason.is_empty(),
                &corrective_recovery_reason,
            ));
        }
    }

    for entry in actions.iter_mut() {
        entry.champs = champs_for(&entry.kind);
    }

    let mut conseillee = conseillee_pour(&task.status, task_active, gate_files_present);
    if !task_active {
        if let Some(pending) = pending_prepared {
            let mut resume = action(
                "resume-launch",
                "Reprendre le lancement préparé",
                pending.ready && can_start,
                &pending.reason,
            );
            resume.prepared = Some(pending);
            if resume.raison.is_empty() && !can_start {
                resume.raison = start_block_reason.clone();
            }
            actions.push(resume);
            for entry in actions.iter_mut() {
                if entry.kind == "start" || entry.kind == "retry" {
                    entry.disponible = false;
                    entry.raison = "Un lancement est déjà préparé. Reprendre cette opération conserve sa copie et évite un doublon.".to_string();
                }
            }
            conseillee = "resume-launch";
        }
    }
    if task.status == "blocked" && attempt_extension_reason.is_empty() {
        conseillee = "extend-attempt";
    }
    // Pour une tâche non bloquée, le correctiveRecoveryReason Go fourni en
    // source retourne toujours un motif non vide (contrôle immédiat
    // `t.Status != "blocked"`), donc la bascule de conseil ne peut pas s'y
    // déclencher : le fait n'est consulté que pour une tâche bloquée.
    if task.status == "blocked" && corrective_recovery_reason.is_empty() {
        conseillee = "authorize-recovery";
    }
    for entry in actions.iter_mut() {
        entry.conseillee = entry.kind == conseillee && entry.disponible;
        if entry.conseillee {
            entry.raison.clear();
        }
    }
    Ok(actions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Owned minimal native data; irrelevant optional fields use model defaults,
    /// never operator policy or synthesized admission observations.
    fn task_fixture(id: &str, status: &str) -> crate::Task {
        crate::Task { id: id.into(), status: status.into(), ..Default::default() }
    }

    fn agent_fixture(task_id: &str, status: &str, desired: &str) -> crate::Agent {
        crate::Agent {
            task_id: task_id.to_string(),
            status: status.to_string(),
            desired: desired.to_string(),
            ..Default::default()
        }
    }

    /// Faits de base : raisons sentinelles non vides pour garder les
    /// conseils extension/récupération inactifs hors de leurs tests dédiés
    /// (une raison vide s'accapare la recommandation, comme en Go).
    fn base_facts() -> Facts {
        Facts {
            can_start: Observation::Observed(false),
            start_block_reason: Observation::Observed(String::new()),
            gate_files_present: Observation::Observed(false),
            gate_current: Observation::Observed(false),
            independent_review_reason: Observation::Observed(None),
            override_dependency_reason: Observation::Observed(None),
            legacy_submission_repairable: Observation::Observed(false),
            last_finished_requires_environment_verification: Observation::Observed(false),
            prepared: Observation::Observed(None),
            show_attempt_extension: Observation::Observed(false),
            show_corrective_recovery: Observation::Observed(false),
            attempt_extension_reason: Observation::Observed("Aucune tentative supplémentaire applicable.".to_string()),
            corrective_recovery_reason: Observation::Observed("Aucun essai correctif applicable.".to_string()),
        }
    }

    /// Témoin initial épars valide : tâche « À faire », aucune ligne d'agent.
    /// Seuls les faits consommés par cette branche sont observés — can_start,
    /// prepared None prouvé par le lecteur Root (Work.planning absent) et la
    /// raison de dépendance réellement vérifiée. Gate, revue indépendante,
    /// réparation legacy, drapeau d'environnement et faits d'affichage du
    /// blocage restent non observés sans jamais devenir un passe-droit.
    fn sparse_todo_facts() -> Facts {
        Facts {
            can_start: Observation::Observed(true),
            start_block_reason: Observation::Unobserved,
            gate_files_present: Observation::Unobserved,
            gate_current: Observation::Unobserved,
            independent_review_reason: Observation::Unobserved,
            override_dependency_reason: Observation::Observed(None),
            legacy_submission_repairable: Observation::Unobserved,
            last_finished_requires_environment_verification: Observation::Unobserved,
            prepared: Observation::Observed(None),
            show_attempt_extension: Observation::Unobserved,
            show_corrective_recovery: Observation::Unobserved,
            attempt_extension_reason: Observation::Unobserved,
            corrective_recovery_reason: Observation::Unobserved,
        }
    }

    /// Vérifie le refus explicite d'un fait consommé non observé : ni liste
    /// vide réussie, ni motif fabriqué, ni garde PASS inventée.
    fn expect_unobserved_refusal(result: Result<Vec<TaskAction>, RootStoreError>) {
        match result {
            Ok(_) => panic!("la projection doit refuser un fait consommé non observé"),
            Err(RootStoreError::Refused(reason)) => {
                assert_eq!(reason, "cockpit-action-fact-unobserved");
            }
            #[allow(unreachable_patterns)]
            Err(_) => panic!("le refus attendu est RootStoreError::Refused"),
        }
    }

    fn find_entry<'a>(value: &'a serde_json::Value, kind: &str) -> Option<&'a serde_json::Value> {
        value
            .as_array()
            .expect("action array")
            .iter()
            .find(|entry| entry.get("kind").and_then(serde_json::Value::as_str) == Some(kind))
    }

    fn by_kind<'a>(value: &'a serde_json::Value, kind: &str) -> &'a serde_json::Value {
        find_entry(value, kind).unwrap_or_else(|| panic!("missing action {kind}"))
    }

    fn conseillee_kinds(value: &serde_json::Value) -> Vec<&str> {
        value
            .as_array()
            .expect("action array")
            .iter()
            .filter(|entry| entry.get("conseillee").and_then(serde_json::Value::as_bool) == Some(true))
            .filter_map(|entry| entry.get("kind").and_then(serde_json::Value::as_str))
            .collect()
    }

    #[test]
    fn initial_todo_accepts_sparse_facts_matching_full_observation() {
        let task = task_fixture("task-1", "todo");
        let actions = project(&task, &[], sparse_todo_facts()).unwrap();
        assert_eq!(actions.len(), 11);
        assert_eq!(actions[0].kind, "start");
        assert!(actions[0].disponible);
        assert!(actions[0].conseillee);
        // L'éparsité ne change rien : la projection pleinement observée
        // produit exactement les mêmes actions, motifs et champs.
        let fully_observed = project(
            &task,
            &[],
            Facts { can_start: Observation::Observed(true), ..base_facts() },
        )
        .unwrap();
        assert_eq!(actions, fully_observed);
        // Aucun fait non observé n'a été transformé en garde PASS : la relance
        // garde son motif Go, la dérogation reste disponible (aucune
        // dépendance non courante observée).
        let value = serde_json::to_value(&actions).unwrap();
        assert_eq!(
            by_kind(&value, "retry")["raison"],
            json!("Aucune tentative à relancer : choisir Lancer un agent.")
        );
        assert_eq!(by_kind(&value, "override")["disponible"], json!(true));
        assert_eq!(conseillee_kinds(&value), ["start"]);
    }

    #[test]
    fn initial_todo_refuses_each_missing_consumed_fact() {
        let task = task_fixture("task-1", "todo");

        // can_start est consommé dès l'état « À faire » sans agent.
        expect_unobserved_refusal(project(
            &task,
            &[],
            Facts { can_start: Observation::Unobserved, ..sparse_todo_facts() },
        ));

        // prepared est consommé hors tentative active : Observed(None) doit
        // être prouvé par le lecteur Root, jamais déduit.
        expect_unobserved_refusal(project(
            &task,
            &[],
            Facts { prepared: Observation::Unobserved, ..sparse_todo_facts() },
        ));

        // Départ refusé : la raison de blocage devient nécessaire au motif ;
        // sans elle, aucun silence ne peut passer pour un succès.
        expect_unobserved_refusal(project(
            &task,
            &[],
            Facts {
                can_start: Observation::Observed(false),
                start_block_reason: Observation::Unobserved,
                ..sparse_todo_facts()
            },
        ));

        // Dérogation : la raison de dépendance est consommée pour un
        // « À faire » sans tentative active.
        expect_unobserved_refusal(project(
            &task,
            &[],
            Facts {
                override_dependency_reason: Observation::Unobserved,
                ..sparse_todo_facts()
            },
        ));

        // Avec la raison de blocage observée, le même can_start faux projette
        // le motif moteur fourni, sans rien inventer.
        let value = serde_json::to_value(project(
            &task,
            &[],
            Facts {
                can_start: Observation::Observed(false),
                start_block_reason: Observation::Observed(
                    "Conditions de départ non réunies (dépendances, budget ou espace de travail)."
                        .to_string(),
                ),
                ..sparse_todo_facts()
            },
        )
        .unwrap())
        .unwrap();
        assert_eq!(
            by_kind(&value, "start")["raison"],
            json!("Conditions de départ non réunies (dépendances, budget ou espace de travail).")
        );
        assert_eq!(by_kind(&value, "start")["disponible"], json!(false));
    }

    #[test]
    fn todo_ready_project_keeps_go_order_and_exact_wire_shape() {
        let task = task_fixture("task-1", "todo");
        let actions = project(&task, &[], Facts { can_start: Observation::Observed(true), ..base_facts() }).unwrap();
        assert_eq!(actions.len(), 11);
        let kinds: Vec<&str> = actions.iter().map(|a| a.kind.as_str()).collect();
        assert_eq!(
            kinds,
            ["start", "retry", "stop", "reconcile", "report", "submit", "gate", "accepted", "override", "reopen", "assign"]
        );
        // Forme wire exacte : ordre des champs, omitempty, champs demandés.
        let expected_start = r#"{"kind":"start","label":"Lancer un agent","disponible":true,"conseillee":true,"champs":[{"name":"provider","label":"Fournisseur","aide":"Programme agent à lancer (défini dans .swarm/providers.json).","requis":true},{"name":"role","label":"Rôle","aide":"Worker réalise la tâche ; planner et subplanner planifient sans coder."},{"name":"workspace","label":"Espace de travail","aide":"Répertoire dans lequel l'agent travaillera.","requis":true},{"name":"instruction","label":"Consigne pour l'agent","aide":"Transmise telle quelle ; préremplie avec la prochaine action prévue.","requis":true},{"name":"capture","label":"Capture détaillée des sorties","aide":"Désactivée par défaut : activité structurée seulement."}]}"#;
        assert_eq!(serde_json::to_string(&actions[0]).unwrap(), expected_start);

        let value = serde_json::to_value(&actions).unwrap();
        let retry = by_kind(&value, "retry");
        assert_eq!(retry["disponible"], json!(false));
        assert_eq!(retry["raison"], json!("Aucune tentative à relancer : choisir Lancer un agent."));
        assert_eq!(retry["champs"][0]["name"], json!("agent"));
        let stop = by_kind(&value, "stop");
        assert_eq!(stop["disponible"], json!(false));
        assert_eq!(stop["raison"], json!("Aucune tentative active sur cette tâche."));
        let reconcile = by_kind(&value, "reconcile");
        assert_eq!(reconcile["raison"], json!("Aucune tentative active sur cette tâche."));
        let report = by_kind(&value, "report");
        assert_eq!(report["disponible"], json!(true));
        assert!(report.get("raison").is_none());
        assert_eq!(by_kind(&value, "submit")["disponible"], json!(true));
        assert_eq!(
            by_kind(&value, "accepted")["raison"],
            json!("Un rapport doit être soumis avant l'acceptation.")
        );
        assert_eq!(by_kind(&value, "override")["disponible"], json!(true));
        assert_eq!(
            by_kind(&value, "reopen")["raison"],
            json!("Seule une tâche acceptée, dérogée ou abandonnée peut être rouverte.")
        );
        assert_eq!(by_kind(&value, "assign")["disponible"], json!(true));
        assert_eq!(conseillee_kinds(&value), ["start"]);
    }

    #[test]
    fn running_with_active_agent_conseille_stop() {
        let task = task_fixture("task-1", "running");
        let agents = vec![
            agent_fixture("task-9", "running", ""), // autre tâche : ignoré
            agent_fixture("task-1", "running", ""),
        ];
        let value = serde_json::to_value(project(&task, &agents, base_facts()).unwrap()).unwrap();
        assert_eq!(value.as_array().unwrap().len(), 11);
        assert_eq!(
            by_kind(&value, "start")["raison"],
            json!("Un agent est actif sur cette tâche : attendre sa fin ou l'arrêter avant de relancer.")
        );
        assert_eq!(
            by_kind(&value, "retry")["raison"],
            json!("Une tentative est en cours : attendez sa fin ou arrêtez-la.")
        );
        let stop = by_kind(&value, "stop");
        assert_eq!(stop["disponible"], json!(true));
        assert_eq!(stop["champs"][0]["name"], json!("agent"));
        assert_eq!(stop["champs"][0]["label"], json!("Tentative à arrêter"));
        let reconcile = by_kind(&value, "reconcile");
        assert_eq!(reconcile["disponible"], json!(true));
        assert!(reconcile.get("raison").is_none());
        assert_eq!(
            by_kind(&value, "submit")["raison"],
            json!("Une tentative est en cours : attendez sa fin avant de soumettre un rapport.")
        );
        assert_eq!(
            by_kind(&value, "override")["raison"],
            json!("Arrêtez et réconciliez la tentative avant dérogation.")
        );
        assert_eq!(conseillee_kinds(&value), ["stop"]);
    }

    #[test]
    fn running_without_confirmed_stop_or_activity_leaves_no_conseillee() {
        let task = task_fixture("task-1", "running");
        // (a) arrêt déjà demandé : « stop » conseillé mais indisponible.
        let agents = vec![agent_fixture("task-1", "running", "stop")];
        let value = serde_json::to_value(project(&task, &agents, base_facts()).unwrap()).unwrap();
        let stop = by_kind(&value, "stop");
        assert_eq!(stop["disponible"], json!(false));
        assert_eq!(stop["raison"], json!("Arrêt déjà demandé ; confirmation du superviseur attendue."));
        assert!(conseillee_kinds(&value).is_empty());

        // (b) aucune ligne : réconcilié conseillé mais indisponible.
        let value = serde_json::to_value(project(&task, &[], base_facts()).unwrap()).unwrap();
        assert_eq!(
            by_kind(&value, "start")["raison"],
            json!("Aucun agent actif : réconciliez l'état observé avant de relancer.")
        );
        let reconcile = by_kind(&value, "reconcile");
        assert_eq!(reconcile["disponible"], json!(false));
        assert_eq!(reconcile["raison"], json!("Aucune tentative active sur cette tâche."));
        assert!(conseillee_kinds(&value).is_empty());
    }

    #[test]
    fn submitted_gate_truth_table() {
        let task = task_fixture("task-1", "submitted");

        // (a) gate fraîche + rapport legacy réparable.
        let facts = Facts {
            can_start: Observation::Observed(true),
            gate_files_present: Observation::Observed(true),
            gate_current: Observation::Observed(true),
            legacy_submission_repairable: Observation::Observed(true),
            ..base_facts()
        };
        let value = serde_json::to_value(project(&task, &[], facts).unwrap()).unwrap();
        assert_eq!(by_kind(&value, "gate")["conseillee"], json!(true));
        assert_eq!(by_kind(&value, "accepted")["disponible"], json!(true));
        assert_eq!(by_kind(&value, "submit")["disponible"], json!(true));
        assert_eq!(by_kind(&value, "start")["disponible"], json!(true));
        assert_eq!(conseillee_kinds(&value), ["gate"]);

        // (b) gate absente : rapport d'abord, soumission et acceptation bloquées.
        let facts = Facts {
            start_block_reason: Observation::Observed("Conditions de départ non réunies (dépendances, budget ou espace de travail).".to_string()),
            ..base_facts()
        };
        let value = serde_json::to_value(project(&task, &[], facts).unwrap()).unwrap();
        assert_eq!(by_kind(&value, "report")["conseillee"], json!(true));
        assert!(by_kind(&value, "gate").get("raison").is_none());
        assert_eq!(
            by_kind(&value, "accepted")["raison"],
            json!("Examinez puis enregistrez une gate delivery avant d'accepter.")
        );
        assert_eq!(
            by_kind(&value, "submit")["raison"],
            json!("Un rapport se soumet depuis une tâche « À faire » ou « Bloquée » qui vient de produire son livrable.")
        );
        assert_eq!(
            by_kind(&value, "start")["raison"],
            json!("Conditions de départ non réunies (dépendances, budget ou espace de travail).")
        );

        // (c) garde de revue indépendante : le motif d'acceptation est remplacé.
        let facts = Facts {
            gate_files_present: Observation::Observed(true),
            gate_current: Observation::Observed(true),
            independent_review_reason: Observation::Observed(Some("Revue indépendante requise avant acceptation.".to_string())),
            ..base_facts()
        };
        let value = serde_json::to_value(project(&task, &[], facts).unwrap()).unwrap();
        assert_eq!(
            by_kind(&value, "accepted")["raison"],
            json!("Revue indépendante requise avant acceptation.")
        );
        assert_eq!(conseillee_kinds(&value), ["gate"]);
    }

    /// Témoin « Soumise » épars valide : gate listée et courante, revue sans
    /// refus, réparation legacy possible ; la raison de départ reste non
    /// observée (can_start vrai) et les faits du blocage hors branche.
    fn submitted_witness() -> Facts {
        Facts {
            can_start: Observation::Observed(true),
            start_block_reason: Observation::Unobserved,
            gate_files_present: Observation::Observed(true),
            gate_current: Observation::Observed(true),
            independent_review_reason: Observation::Observed(None),
            override_dependency_reason: Observation::Observed(None),
            legacy_submission_repairable: Observation::Observed(true),
            last_finished_requires_environment_verification: Observation::Unobserved,
            prepared: Observation::Observed(None),
            show_attempt_extension: Observation::Unobserved,
            show_corrective_recovery: Observation::Unobserved,
            attempt_extension_reason: Observation::Unobserved,
            corrective_recovery_reason: Observation::Unobserved,
        }
    }

    #[test]
    fn submitted_witness_consumes_gate_review_and_start_facts() {
        let task = task_fixture("task-1", "submitted");
        let value = serde_json::to_value(project(&task, &[], submitted_witness()).unwrap()).unwrap();
        assert_eq!(conseillee_kinds(&value), ["gate"]);
        assert_eq!(by_kind(&value, "accepted")["disponible"], json!(true));
        assert!(by_kind(&value, "report").get("conseillee").is_none());

        // Chaque fait consommé par cette branche refuse explicitement s'il
        // manque — y compris ceux que le témoin « À faire » tolérait
        // non observés.
        expect_unobserved_refusal(project(&task, &[], Facts {
            gate_files_present: Observation::Unobserved,
            ..submitted_witness()
        }));
        expect_unobserved_refusal(project(&task, &[], Facts {
            gate_current: Observation::Unobserved,
            ..submitted_witness()
        }));
        expect_unobserved_refusal(project(&task, &[], Facts {
            independent_review_reason: Observation::Unobserved,
            ..submitted_witness()
        }));
        expect_unobserved_refusal(project(&task, &[], Facts {
            legacy_submission_repairable: Observation::Unobserved,
            ..submitted_witness()
        }));
        expect_unobserved_refusal(project(&task, &[], Facts {
            can_start: Observation::Unobserved,
            ..submitted_witness()
        }));
        expect_unobserved_refusal(project(&task, &[], Facts {
            override_dependency_reason: Observation::Unobserved,
            ..submitted_witness()
        }));
        expect_unobserved_refusal(project(&task, &[], Facts {
            prepared: Observation::Unobserved,
            ..submitted_witness()
        }));
        // can_start faux : la raison de blocage devient consommée.
        expect_unobserved_refusal(project(&task, &[], Facts {
            can_start: Observation::Observed(false),
            start_block_reason: Observation::Unobserved,
            ..submitted_witness()
        }));
    }

    #[test]
    fn finished_attempt_with_environment_failure_blocks_start_with_recovery_wording() {
        let task = task_fixture("task-1", "accepted");
        let agents = vec![agent_fixture("task-1", "done", "")];
        let facts = Facts {
            can_start: Observation::Observed(false),
            last_finished_requires_environment_verification: Observation::Observed(true),
            start_block_reason: Observation::Observed("Conditions de départ non réunies (dépendances, budget ou espace de travail).".to_string()),
            ..base_facts()
        };
        let value = serde_json::to_value(project(&task, &agents, facts).unwrap()).unwrap();
        assert_eq!(
            by_kind(&value, "start")["raison"],
            json!("Échec d’environnement identifié : utilisez la reprise et décrivez une vérification nouvelle des préconditions.")
        );
        assert_eq!(by_kind(&value, "start")["disponible"], json!(false));
        // retry : tentative terminée présente, départ refusé -> motif moteur fourni.
        assert_eq!(
            by_kind(&value, "retry")["raison"],
            json!("Conditions de départ non réunies (dépendances, budget ou espace de travail).")
        );
        assert_eq!(by_kind(&value, "retry")["champs"][1]["name"], json!("precondition_evidence"));
        assert_eq!(
            by_kind(&value, "gate")["raison"],
            json!("Tâche validée : rouvrez-la avant une nouvelle évaluation.")
        );
        assert_eq!(conseillee_kinds(&value), ["reopen"]);
    }

    #[test]
    fn waived_and_abandoned_statuses() {
        let task = task_fixture("task-1", "waived");
        let value = serde_json::to_value(project(&task, &[], base_facts()).unwrap()).unwrap();
        assert_eq!(
            by_kind(&value, "start")["raison"],
            json!("Tâche acceptée par dérogation : rouvrez-la avant de relancer un agent.")
        );
        assert_eq!(
            by_kind(&value, "override")["raison"],
            json!("Dérogation déjà enregistrée : rouvrez la tâche pour la modifier.")
        );
        assert_eq!(
            by_kind(&value, "gate")["raison"],
            json!("Tâche validée : rouvrez-la avant une nouvelle évaluation.")
        );
        assert_eq!(conseillee_kinds(&value), ["reopen"]);

        let task = task_fixture("task-1", "abandoned");
        let value = serde_json::to_value(project(&task, &[], base_facts()).unwrap()).unwrap();
        assert_eq!(
            by_kind(&value, "start")["raison"],
            json!("Tâche abandonnée : rouvrez-la avant de relancer un agent.")
        );
        assert_eq!(
            by_kind(&value, "override")["raison"],
            json!("Rouvrez la tâche abandonnée avant dérogation.")
        );
        assert_eq!(conseillee_kinds(&value), ["reopen"]);
    }

    /// Témoin « Bloquée » au plafond : faits d'affichage et motifs observés,
    /// can_start faux avec sa raison moteur ; gate, revue et réparation
    /// legacy restent hors branche (non observés).
    fn blocked_witness() -> Facts {
        Facts {
            can_start: Observation::Observed(false),
            start_block_reason: Observation::Observed("Plafond de tentatives du plan atteint.".to_string()),
            gate_files_present: Observation::Unobserved,
            gate_current: Observation::Unobserved,
            independent_review_reason: Observation::Unobserved,
            override_dependency_reason: Observation::Observed(None),
            legacy_submission_repairable: Observation::Unobserved,
            last_finished_requires_environment_verification: Observation::Unobserved,
            prepared: Observation::Observed(None),
            show_attempt_extension: Observation::Observed(true),
            show_corrective_recovery: Observation::Observed(true),
            attempt_extension_reason: Observation::Observed(String::new()),
            corrective_recovery_reason: Observation::Observed(String::new()),
        }
    }

    #[test]
    fn blocked_witness_consumes_display_and_start_facts() {
        let task = task_fixture("task-1", "blocked");
        let actions = project(&task, &[], blocked_witness()).unwrap();
        assert_eq!(actions.len(), 13);
        assert_eq!(actions[11].kind, "extend-attempt");
        assert_eq!(actions[12].kind, "authorize-recovery");
        let value = serde_json::to_value(&actions).unwrap();
        assert_eq!(conseillee_kinds(&value), ["authorize-recovery"]);

        // Chaque fait consommé par une tâche bloquée refuse s'il manque.
        expect_unobserved_refusal(project(&task, &[], Facts {
            show_attempt_extension: Observation::Unobserved,
            ..blocked_witness()
        }));
        expect_unobserved_refusal(project(&task, &[], Facts {
            show_corrective_recovery: Observation::Unobserved,
            ..blocked_witness()
        }));
        expect_unobserved_refusal(project(&task, &[], Facts {
            attempt_extension_reason: Observation::Unobserved,
            ..blocked_witness()
        }));
        expect_unobserved_refusal(project(&task, &[], Facts {
            corrective_recovery_reason: Observation::Unobserved,
            ..blocked_witness()
        }));
        expect_unobserved_refusal(project(&task, &[], Facts {
            can_start: Observation::Unobserved,
            ..blocked_witness()
        }));
        expect_unobserved_refusal(project(&task, &[], Facts {
            start_block_reason: Observation::Unobserved,
            ..blocked_witness()
        }));
        expect_unobserved_refusal(project(&task, &[], Facts {
            prepared: Observation::Unobserved,
            ..blocked_witness()
        }));
        expect_unobserved_refusal(project(&task, &[], Facts {
            override_dependency_reason: Observation::Unobserved,
            ..blocked_witness()
        }));

        // Le drapeau nested n'est consommé que si l'extension est affichée :
        // non observé, il est accepté et n'invente aucune action.
        let value = serde_json::to_value(project(
            &task,
            &[],
            Facts {
                show_attempt_extension: Observation::Observed(false),
                show_corrective_recovery: Observation::Unobserved,
                attempt_extension_reason: Observation::Observed(
                    "Un essai correctif est déjà en cours.".to_string(),
                ),
                corrective_recovery_reason: Observation::Observed(
                    "La reprise corrective exige une tâche bloquée après trois tentatives consommées."
                        .to_string(),
                ),
                ..blocked_witness()
            },
        )
        .unwrap())
        .unwrap();
        assert_eq!(value.as_array().unwrap().len(), 11);
        assert!(find_entry(&value, "authorize-recovery").is_none());
        assert!(find_entry(&value, "extend-attempt").is_none());
        assert!(conseillee_kinds(&value).is_empty());
    }

    #[test]
    fn prepared_launch_suppresses_start_and_retry_and_conseille_resume() {
        let task = task_fixture("task-1", "todo");
        let facts = Facts {
            can_start: Observation::Observed(true),
            prepared: Observation::Observed(Some(PreparedLaunch {
                id: "prep-1".to_string(),
                task: "task-1".to_string(),
                ready: true,
                reason: String::new(),
                provider: "cli".to_string(),
                workspace: "/tmp/w".to_string(),
                instruction: "Relance contrôlée".to_string(),
                timeout_seconds: 600,
                limits: None,
            })),
            ..base_facts()
        };
        let actions = project(&task, &[], facts).unwrap();
        assert_eq!(actions.len(), 12);
        let resume = actions.iter().find(|a| a.kind == "resume-launch").unwrap();
        assert_eq!(
            serde_json::to_string(resume).unwrap(),
            r#"{"prepared_launch":{"id":"prep-1","task":"task-1","ready":true,"provider":"cli","workspace":"/tmp/w","instruction":"Relance contrôlée","timeout_seconds":600},"kind":"resume-launch","label":"Reprendre le lancement préparé","disponible":true,"conseillee":true}"#
        );
        let value = serde_json::to_value(&actions).unwrap();
        for kind in ["start", "retry"] {
            let entry = by_kind(&value, kind);
            assert_eq!(entry["disponible"], json!(false));
            assert_eq!(
                entry["raison"],
                json!("Un lancement est déjà préparé. Reprendre cette opération conserve sa copie et évite un doublon.")
            );
        }
        assert_eq!(conseillee_kinds(&value), ["resume-launch"]);

        // Non prêt et départ refusé : repli sur le motif moteur fourni.
        let facts = Facts {
            can_start: Observation::Observed(false),
            start_block_reason: Observation::Observed("Départs suspendus : réautoriez les départs depuis l'en-tête du cockpit.".to_string()),
            prepared: Observation::Observed(Some(PreparedLaunch {
                id: "prep-2".to_string(),
                task: "task-1".to_string(),
                ready: false,
                reason: String::new(),
                provider: String::new(),
                workspace: String::new(),
                instruction: String::new(),
                timeout_seconds: 0,
                limits: None,
            })),
            ..base_facts()
        };
        let value = serde_json::to_value(project(&task, &[], facts).unwrap()).unwrap();
        let resume = by_kind(&value, "resume-launch");
        assert_eq!(resume["disponible"], json!(false));
        assert_eq!(
            resume["raison"],
            json!("Départs suspendus : réautoriez les départs depuis l'en-tête du cockpit.")
        );
        assert!(conseillee_kinds(&value).is_empty());

        // Raison du préparé présente : elle gagne sur le motif moteur.
        let facts = Facts {
            can_start: Observation::Observed(true),
            prepared: Observation::Observed(Some(PreparedLaunch {
                id: "prep-3".to_string(),
                task: "task-1".to_string(),
                ready: false,
                reason: "Espace de travail non confirmé.".to_string(),
                provider: String::new(),
                workspace: String::new(),
                instruction: String::new(),
                timeout_seconds: 0,
                limits: None,
            })),
            ..base_facts()
        };
        let value = serde_json::to_value(project(&task, &[], facts).unwrap()).unwrap();
        let resume = by_kind(&value, "resume-launch");
        assert_eq!(resume["disponible"], json!(false));
        assert_eq!(resume["raison"], json!("Espace de travail non confirmé."));
        assert!(conseillee_kinds(&value).is_empty());
    }

    #[test]
    fn blocked_at_cap_orders_extension_then_recovery() {
        let task = task_fixture("task-1", "blocked");
        let facts = Facts {
            show_attempt_extension: Observation::Observed(true),
            show_corrective_recovery: Observation::Observed(true),
            attempt_extension_reason: Observation::Observed(String::new()),
            corrective_recovery_reason: Observation::Observed(String::new()),
            can_start: Observation::Observed(false),
            start_block_reason: Observation::Observed("Plafond de tentatives du plan atteint.".to_string()),
            ..base_facts()
        };
        let actions = project(&task, &[], facts).unwrap();
        assert_eq!(actions.len(), 13);
        assert_eq!(actions[11].kind, "extend-attempt");
        assert_eq!(actions[11].label, "Autoriser une tentative supplémentaire");
        assert_eq!(actions[12].kind, "authorize-recovery");
        assert_eq!(actions[12].label, "Préparer un essai correctif");
        let value = serde_json::to_value(&actions).unwrap();
        let extend = by_kind(&value, "extend-attempt");
        assert!(extend.get("raison").is_none());
        assert!(extend.get("champs").is_none());
        assert_eq!(by_kind(&value, "authorize-recovery")["conseillee"], json!(true));
        assert_eq!(by_kind(&value, "start")["raison"], json!("Plafond de tentatives du plan atteint."));
        assert_eq!(conseillee_kinds(&value), ["authorize-recovery"]);

        // Sans le seuil d'affichage récupération, l'action disparaît ; une
        // raison d'extension non vide laisse le conseil sur start.
        let facts = Facts {
            show_attempt_extension: Observation::Observed(true),
            attempt_extension_reason: Observation::Observed("Un essai correctif est déjà en cours.".to_string()),
            can_start: Observation::Observed(false),
            start_block_reason: Observation::Observed("Plafond de tentatives du plan atteint.".to_string()),
            ..base_facts()
        };
        let value = serde_json::to_value(project(&task, &[], facts).unwrap()).unwrap();
        assert_eq!(value.as_array().unwrap().len(), 12);
        let extend = by_kind(&value, "extend-attempt");
        assert_eq!(extend["disponible"], json!(false));
        assert_eq!(extend["raison"], json!("Un essai correctif est déjà en cours."));
        assert!(find_entry(&value, "authorize-recovery").is_none());
        assert!(conseillee_kinds(&value).is_empty());
    }

    #[test]
    fn override_dependency_guard_uses_current_facts_and_launch_held_reason() {
        let mut task = task_fixture("task-1", "todo");
        task.depends = Some(vec!["dep-7".to_string()]);
        let ready = serde_json::to_value(project(&task, &[], base_facts()).unwrap()).unwrap();
        assert_eq!(by_kind(&ready, "override")["disponible"], json!(true));
        let value = serde_json::to_value(project(&task, &[], Facts {
            override_dependency_reason: Observation::Observed(Some("Acceptez d'abord la dépendance dep-7 (revue ou dérogation).".into())),
            ..base_facts()
        }).unwrap()).unwrap();
        let override_action = by_kind(&value, "override");
        assert_eq!(override_action["disponible"], json!(false));
        assert_eq!(
            override_action["raison"],
            json!("Acceptez d'abord la dépendance dep-7 (revue ou dérogation).")
        );

        let mut task = task_fixture("task-1", "todo");
        task.launch_held = true;
        let facts = Facts {
            can_start: Observation::Observed(false),
            start_block_reason: Observation::Observed("Démarrage non autorisé : ouvrir la préparation puis autoriser les missions de ce plan.".to_string()),
            ..base_facts()
        };
        let value = serde_json::to_value(project(&task, &[], facts).unwrap()).unwrap();
        assert_eq!(
            by_kind(&value, "start")["raison"],
            json!("Démarrage non autorisé : ouvrir la préparation puis autoriser les missions de ce plan.")
        );
        let retry = by_kind(&value, "retry");
        assert_eq!(retry["disponible"], json!(false));
        assert_eq!(
            retry["raison"],
            json!("Démarrage non autorisé : ouvrir la préparation puis autoriser les missions de ce plan.")
        );
    }

    #[test]
    fn retry_requires_a_finished_attempt_and_start_conditions() {
        let task = task_fixture("task-1", "todo");
        // Aucune tentative terminée : relance indisponible même si start passe.
        let value = serde_json::to_value(project(&task, &[], Facts { can_start: Observation::Observed(true), ..base_facts() }).unwrap()).unwrap();
        let retry = by_kind(&value, "retry");
        assert_eq!(retry["disponible"], json!(false));
        assert_eq!(retry["raison"], json!("Aucune tentative à relancer : choisir Lancer un agent."));

        // Tentative terminée + conditions de départ : relance disponible.
        let agents = vec![agent_fixture("task-1", "done", "")];
        let value = serde_json::to_value(project(&task, &agents, Facts { can_start: Observation::Observed(true), ..base_facts() }).unwrap()).unwrap();
        let retry = by_kind(&value, "retry");
        assert_eq!(retry["disponible"], json!(true));
        assert!(retry.get("raison").is_none());

        // Tentative active : motif dédié, avant toute autre condition.
        let agents = vec![agent_fixture("task-1", "running", "")];
        let value = serde_json::to_value(project(&task, &agents, base_facts()).unwrap()).unwrap();
        assert_eq!(
            by_kind(&value, "retry")["raison"],
            json!("Une tentative est en cours : attendez sa fin ou arrêtez-la.")
        );
    }
}
