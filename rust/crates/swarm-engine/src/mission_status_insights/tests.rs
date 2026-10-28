//! Pure projection witnesses/counterexamples; no SQL, Root, provider or clock IO.
use super::*;
use crate::mission_status_contract::tests::{status, task};
use crate::{PlanningScope, PlanningState, Task};
use time::format_description::well_known::Rfc3339;

/// Set explicit projection facts on the contract's fully populated test DTO.
fn mission(tasks: Vec<MissionTask>) -> MissionStatus {
    let mut value = status();
    value.runtime.state = "ready".into();
    value.organization.ready = true;
    value.enabled = true;
    value.authorized = true;
    value.total = tasks.len() as i64;
    value.tasks = Some(tasks);
    value
}

/// Supply a classified task, with no result/review observation synthesized.
fn classified(id: &str, state: &str, impact: i64) -> MissionTask {
    let mut value = task();
    value.id = id.into();
    value.title = format!("Title {id}");
    value.state = state.into();
    value.reason = format!("Reason {id}");
    value.label = format!("Action {id}");
    value.action = "inspect".into();
    value.impact = impact;
    value.result.state = "fixture-no-result".into();
    value.validation_mode = "human".into();
    value
}

/// Build a stored graph fixture with explicit non-exhausted planning inputs.
fn planned(root_state: &str) -> Work {
    let scope = PlanningScope {
        id: "root".into(),
        state: root_state.into(),
        ..PlanningScope::default()
    };
    let planning = PlanningState {
        scopes: Some(vec![scope]),
        max_activations: 10,
        max_decisions: 10,
        ..PlanningState::default()
    };
    Work {
        planning: Some(Box::new(planning)),
        ..Work::default()
    }
}

/// Retain every actual exchange field explicitly; this is never a send request.
fn exchange(id: &str, state: &str, created_at: &str) -> MissionStatusExchange {
    MissionStatusExchange {
        id: id.into(),
        work_id: "work".into(),
        kind: "handoff".into(),
        source_agent_id: "agent".into(),
        source_task_id: "source".into(),
        source_attempt_id: "attempt".into(),
        recipient_task_id: "recipient".into(),
        recipient_role: "worker".into(),
        recipient_attempt_id: String::new(),
        reply_to: String::new(),
        state: state.into(),
        result_state: String::new(),
        need: String::new(),
        artifacts: Some(vec![]),
        created_at: created_at.into(),
        deadline_at: String::new(),
        acknowledged_at: String::new(),
        acknowledged_by: String::new(),
        acknowledgement: String::new(),
    }
}

/// Canonical timestamp fixture parsing; malformed cases call the projection itself.
fn time(raw: &str) -> OffsetDateTime {
    OffsetDateTime::parse(raw, &Rfc3339).unwrap()
}

#[test]
fn guidance_storage_and_organization_precede_planning_and_tasks() {
    let mut work = planned("open");
    work.planning.as_mut().unwrap().failure = "planner failed".into();
    let mut d = mission(vec![classified("review", "review", 8)]);
    assert_eq!(mission_guidance(&work, &d).primary.task, "review");
    d.organization.ready = false;
    assert_eq!(mission_guidance(&work, &d).primary.kind, "organization");
    d.runtime.state = "blocked".into();
    assert_eq!(mission_guidance(&work, &d).primary.kind, "runtime");
    d.runtime.state = "ready".into();
    d.organization.ready = true;
    assert_eq!(mission_guidance(&work, &d).primary.task, "review");
}

#[test]
fn guidance_keeps_first_equal_impact_then_higher_impact() {
    let work = Work::default();
    let mut d = mission(vec![
        classified("first", "intervention", 5),
        classified("second", "review", 5),
    ]);
    assert_eq!(mission_guidance(&work, &d).primary.task, "first");
    d.tasks.as_mut().unwrap()[1].impact = 6;
    assert_eq!(mission_guidance(&work, &d).primary.task, "second");
    d.tasks.as_mut().unwrap()[1].impact = 5;
    assert_eq!(mission_guidance(&work, &d).primary.task, "first");
}

#[test]
fn planning_failure_cannot_hide_execution_or_actionable_lifecycle() {
    let mut work = planned("open");
    work.planning.as_mut().unwrap().failure = "planner failure".into();
    let mut d = mission(vec![classified("waiting", "waiting", 0)]);
    assert_eq!(mission_guidance(&work, &d).primary.tone, "attention");
    for state in [
        "running",
        "review",
        "intervention",
        "configure",
        "ready",
        "manual",
    ] {
        d.tasks.as_mut().unwrap()[0].state = state.into();
        assert!(!mission_planning_owns_next_step(&d));
        assert_ne!(
            mission_guidance(&work, &d).what,
            "Le responsable n’a pas terminé sa décision."
        );
    }
    d.tasks.as_mut().unwrap()[0].state = "waiting".into();
    d.active_agents = 1;
    assert!(!mission_planning_owns_next_step(&d));
    d.active_agents = 0;
    assert_eq!(
        mission_guidance(&work, &d).what,
        "Le responsable n’a pas terminé sa décision."
    );
}

#[test]
fn primary_attempt_limit_precedes_automatic_review() {
    let mut t = classified("task", "review", 0);
    t.validation_mode = "automatic".into();
    assert_eq!(
        mission_task_primary(&t).label,
        "Voir les contrôles en cours"
    );
    t.attempt_limit_reached = true;
    assert_eq!(
        mission_task_primary(&t).label,
        "Examiner les tentatives et les refus"
    );
    assert_eq!(mission_task_primary(&t).tone, "attention");
    t.attempt_limit_reached = false;
    t.state = "ready".into();
    t.action = "start".into();
    assert_eq!(
        mission_task_primary(&t).effect,
        "Ouvre la confirmation ; le moteur revérifie les conditions avant le départ."
    );
}

#[test]
fn review_result_precedes_terminal_task_explanation() {
    let mut t = classified("task", "validated", 0);
    let d = mission(vec![t.clone()]);
    assert_eq!(task_understanding(&t, &d, &[]).situation, "termine");
    t.result.state = "review_blocked".into();
    t.result.reason = "review proof refused".into();
    t.result.next_step = "inspect review".into();
    let value = task_understanding(&t, &d, &[]);
    assert_eq!(value.what, "review proof refused");
    assert_eq!(value.actor_kind, "user");
    assert_eq!(value.situation, "blocage");
    t.result.state = "review_running".into();
    assert_eq!(task_understanding(&t, &d, &[]).actor_kind, "supervisor");
}

#[test]
fn automatic_review_requires_actual_authorization_and_supervisor() {
    let mut t = classified("task", "review", 0);
    t.validation_mode = "automatic".into();
    let mut d = mission(vec![t.clone()]);
    assert_eq!(task_understanding(&t, &d, &[]).actor_kind, "supervisor");
    d.enabled = false;
    assert_eq!(
        task_understanding(&t, &d, &[]).situation,
        "information_manquante"
    );
    d.paused = true;
    assert_eq!(
        task_understanding(&t, &d, &[]).next_step,
        "Reprenez ou autorisez la mission pour permettre les contrôles."
    );
    d.paused = false;
    d.enabled = true;
    assert_eq!(task_understanding(&t, &d, &[]).actor_kind, "supervisor");
}

#[test]
fn running_explanation_uses_first_active_observation_without_claiming_liveness() {
    let t = classified("task", "running", 0);
    let d = mission(vec![t.clone()]);
    let mut observed = MissionAgentInsight {
        task_id: "task",
        status: "running",
        active: true,
        observed_state: "running",
        fallback_diagnostic_limit_reached: false,
        environment_recovery: false,
    };
    assert_eq!(
        task_understanding(&t, &d, &[observed]).situation,
        "en_cours"
    );
    for state in ["queued", "starting", "stopping"] {
        observed.observed_state = state;
        assert_eq!(
            task_understanding(&t, &d, &[observed]).what,
            "Une tentative est enregistrée, mais aucun travail en cours n’est affirmé."
        );
    }
    observed.observed_state = "unknown";
    assert_eq!(
        task_understanding(&t, &d, &[observed]).situation,
        "information_manquante"
    );
    observed.active = false;
    assert_eq!(
        task_understanding(&t, &d, &[observed]).what,
        "La tâche est marquée en cours, mais aucun agent actif n’est visible."
    );
}

#[test]
fn environment_refusal_respects_first_matching_agent_and_terminal_limit() {
    let mut t = classified("task", "intervention", 0);
    let d = mission(vec![t.clone()]);
    let mut first = MissionAgentInsight {
        task_id: "task",
        status: "failed",
        active: false,
        observed_state: "failed",
        fallback_diagnostic_limit_reached: false,
        environment_recovery: true,
    };
    let text = task_understanding(&t, &d, &[first]).what;
    assert!(text.starts_with("L’environnement"));
    first.fallback_diagnostic_limit_reached = true;
    assert_eq!(task_understanding(&t, &d, &[first]).what, t.reason);
    first.fallback_diagnostic_limit_reached = false;
    t.attempt_limit_reached = true;
    t.attempts_used = 3;
    t.attempts_allowed = 3;
    assert_eq!(
        task_understanding(&t, &d, &[first]).situation,
        "limite_tentatives"
    );
    t.attempt_limit_reached = false;
    let earlier = MissionAgentInsight {
        environment_recovery: false,
        ..first
    };
    assert_eq!(task_understanding(&t, &d, &[earlier, first]).what, t.reason);
}

#[test]
fn mission_understanding_preserves_empty_terminal_and_intervention_priority() {
    let mut d = mission(vec![]);
    assert_eq!(
        mission_understanding(&d).what,
        "Cette mission ne contient aucune tâche."
    );
    d.total = 1;
    d.validated = 1;
    assert_eq!(mission_understanding(&d).situation, "termine");
    d.runtime.state = "blocked".into();
    d.runtime.message = "actual storage problem".into();
    assert_eq!(mission_understanding(&d).what, "actual storage problem");
    d.runtime.state = "ready".into();
    d.validated = 0;
    let mut t = classified("task", "intervention", 0);
    t.understanding = understanding("agent environment", "verify", "Vous", "user", "blocage");
    d.tasks = Some(vec![t]);
    assert_eq!(
        mission_understanding(&d).what,
        "Title task — agent environment"
    );
    d.tasks.as_mut().unwrap()[0].attempt_limit_reached = true;
    assert_eq!(mission_understanding(&d).situation, "limite_tentatives");
    d.active_agents = 1;
    assert_eq!(
        mission_understanding(&d).what,
        "Title task — agent environment"
    );
}

#[test]
fn planning_overrides_retain_closed_history_and_pending_root_responsibility() {
    let mut work = planned("open");
    let mut d = mission(vec![classified("done", "validated", 0)]);
    d.validated = 1;
    d.understanding = mission_understanding(&d);
    assert_eq!(d.understanding.situation, "termine");
    apply_planning_understanding(&work, &mut d);
    assert_eq!(d.understanding.actor, "Le planificateur");
    assert_eq!(d.next, d.understanding.next_step);
    work.planning.as_mut().unwrap().failure = "retained diagnostic".into();
    work.planning.as_mut().unwrap().scopes.as_mut().unwrap()[0].state = "closed".into();
    d.understanding = mission_understanding(&d);
    d.next = "terminal next".into();
    apply_planning_understanding(&work, &mut d);
    assert_eq!(d.understanding.situation, "termine");
    assert_eq!(d.next, "terminal next");
    work.planning.as_mut().unwrap().scopes.as_mut().unwrap()[0].state = "open".into();
    d.validated = 0;
    d.tasks.as_mut().unwrap()[0].state = "waiting".into();
    apply_planning_understanding(&work, &mut d);
    assert_eq!(d.understanding.what, "retained diagnostic");
}

#[test]
fn evidence_stage_requires_positive_validated_count_and_first_root_closed() {
    let mut work = planned("open");
    let unverified = mission_evidence_stage(&work, 1, 1);
    assert!(unverified.starts_with("Parcours"));
    work.planning.as_mut().unwrap().scopes.as_mut().unwrap()[0].state = "closed".into();
    assert!(mission_evidence_stage(&work, 1, 1).starts_with("Résultats validés"));
    assert_eq!(mission_evidence_stage(&work, 0, 0), unverified);
    assert_eq!(mission_evidence_stage(&work, 2, 1), unverified);
    let duplicate = PlanningScope {
        id: "root".into(),
        state: "closed".into(),
        ..PlanningScope::default()
    };
    let scopes = work.planning.as_mut().unwrap().scopes.as_mut().unwrap();
    scopes[0].state = "open".into();
    scopes.push(duplicate);
    assert_eq!(mission_evidence_stage(&work, 1, 1), unverified);
}

#[test]
fn coordination_keeps_four_phases_even_for_nil_task_inputs() {
    let work = Work::default();
    let mut d = mission(vec![]);
    d.tasks = None;
    let phases = mission_coordination_phases(
        &work,
        &d,
        &[],
        time("2026-10-23T12:00:00Z"),
        Duration::from_secs(2),
    );
    assert_eq!(
        phases.iter().map(|p| p.kind.as_str()).collect::<Vec<_>>(),
        vec!["work", "wait", "exchange", "validation"]
    );
    assert_eq!(phases[2].summary, "Aucun échange structuré enregistré.");
    assert!(phases[2].at.is_empty() && phases[2].relative.is_empty());
}

#[test]
fn coordination_prefers_latest_unresolved_and_retains_missing_task_id() {
    let mut work = Work::default();
    let source = Task {
        id: "source".into(),
        title: "Source title".into(),
        ..Task::default()
    };
    work.tasks = Some(vec![source]);
    let mut pending = exchange("first", "pending", "2026-10-23T11:58:00Z");
    pending.need = "actual need".into();
    let exchanges = vec![
        pending,
        exchange("last", "consumed", "2026-10-23T11:59:00Z"),
    ];
    let before = exchanges.clone();
    let d = mission(vec![]);
    let phases = mission_coordination_phases(
        &work,
        &d,
        &exchanges,
        time("2026-10-23T12:00:00Z"),
        Duration::from_secs(2),
    );
    assert_eq!(
        phases[2].summary,
        "Source title remet un résultat à recipient · à traiter. actual need"
    );
    assert_eq!(phases[2].at, "2026-10-23T11:58:00Z");
    assert_eq!(phases[2].relative, "il y a environ 2 min");
    assert!(exchanges == before);
}

#[test]
fn coordination_completed_mission_keeps_exchange_stamp_without_action() {
    let work = Work::default();
    let mut d = mission(vec![]);
    d.understanding.situation = "termine".into();
    let exchanges = vec![exchange(
        "history",
        "escalated",
        "invalid historical timestamp",
    )];
    let phases = mission_coordination_phases(
        &work,
        &d,
        &exchanges,
        time("2026-10-23T12:00:00Z"),
        Duration::from_secs(2),
    );
    assert_eq!(
        phases[2].summary,
        "Échanges conservés dans l’historique de la mission terminée."
    );
    assert_eq!(phases[2].actor, "Personne pour ces échanges");
    assert_eq!(phases[2].at, "invalid historical timestamp");
    assert!(phases[2].relative.is_empty());
}

#[test]
fn coordination_automatic_validation_precedes_human_review_and_waiting_order() {
    let work = Work::default();
    let mut automatic = classified("review", "review", 0);
    automatic.validation_mode = "automatic".into();
    let mut waiting = classified("wait", "waiting", 0);
    waiting.understanding = understanding(
        "wait",
        "actual next",
        "Actual actor",
        "user",
        "attente_normale",
    );
    let ready = classified("ready", "ready", 0);
    let mut d = mission(vec![automatic, waiting, ready]);
    d.review = 2;
    let phases = mission_coordination_phases(
        &work,
        &d,
        &[],
        time("2026-10-23T12:00:00Z"),
        Duration::from_secs(2),
    );
    assert_eq!(
        phases[1].summary,
        "Title wait — Reason wait · 1 autre(s) attente(s)"
    );
    assert_eq!(phases[1].actor, "Actual actor");
    assert_eq!(phases[1].next_step, "actual next");
    assert_eq!(
        phases[3].summary,
        "1 résultat(s) attendent leurs contrôles automatiques autorisés."
    );
    d.tasks.as_mut().unwrap()[0].validation_mode = "human".into();
    assert_eq!(
        mission_coordination_phases(
            &work,
            &d,
            &[],
            time("2026-10-23T12:00:00Z"),
            Duration::from_secs(2)
        )[3]
        .summary,
        "2 résultat(s) attendent une revue humaine."
    );
}

#[test]
fn relative_time_rounding_future_boundary_and_go_duration_saturation() {
    let at = time("2026-10-23T12:00:00Z");
    let tolerance = Duration::from_secs(2);
    for (age, expected) in [
        (0, "à l’instant"),
        (60, "il y a environ 1 min"),
        (90, "il y a environ 2 min"),
        (3599, "il y a environ 60 min"),
        (3600, "il y a environ 1 h"),
        (5400, "il y a environ 2 h"),
    ] {
        assert_eq!(
            past_relative(at, at - time::Duration::seconds(age), tolerance),
            expected
        );
    }
    assert_eq!(
        past_relative(at, at + time::Duration::seconds(2), tolerance),
        "à l’instant"
    );
    assert_eq!(
        past_relative(
            at,
            at + time::Duration::seconds(2) + time::Duration::nanoseconds(1),
            tolerance
        ),
        "horloge incohérente"
    );
    assert_eq!(
        past_relative(at, time("0001-01-01T00:00:00Z"), tolerance),
        "il y a environ 2562048 h"
    );
}

/// The retained spelling remains byte-exact while Go-compatible parsing derives
/// relative text. No clock is read, deadline changed or activity inferred.
#[test]
fn mission_status_coordination_accepts_actual_go_timestamp_grammar() {
    let work = Work::default();
    let status = mission(vec![]);
    let at = time("2026-10-23T00:01:00Z");
    for stamp in ["2026-10-23T00:00:00.1Z", "2026-10-23T00:00:00,1Z"] {
        let phases = mission_coordination_phases(
            &work,
            &status,
            &[exchange("history", "pending", stamp)],
            at,
            Duration::ZERO,
        );
        assert_eq!(phases[2].at, stamp);
        assert_eq!(phases[2].relative, "à l’instant");
    }
    let invalid = mission_coordination_phases(
        &work,
        &status,
        &[exchange("history", "pending", "invalid")],
        at,
        Duration::ZERO,
    );
    assert!(invalid[2].relative.is_empty());
}
