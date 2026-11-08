//! Historical cockpit health labels, never process ownership or current authority.
//! Callers provide the real observer result, host comparison, cancellation
//! eligibility and saved effective silence limit; no IO or default policy here.
use serde_json::{json, Value};
use super::RootStoreError as E;

/// Classify the legacy terminal vocabulary, not a new kernel wait observation.
fn terminal(state: &str) -> bool { matches!(state, "completed" | "failed" | "interrupted") }
/// Empty output/tool/vitality values mean protocol-unknown, never inferred work.
fn known(value: &str) -> &str { if value.is_empty() { "unknown" } else { value } }

/// Project Go pilotAgentHealth using actual caller-owned observations.
/// Stored active intent controls the stop label, independently of unconfirmed
/// observed liveness; a terminal intent is not relabelled from a supplied unknown
/// state. Timestamps use the shared Go parser. A malformed last result stays
/// unknown; an invalid snapshot clock with a result or negative saved threshold
/// refuses, preserving the parser cause. No mutation, signal, provider or grant.
/// Terminal/degraded/interactive precedence matches Go. Memory follows input
/// strings. Compatibility completion labels remain recorded data, not a receipt.
pub(super) fn project(agent: &crate::Agent, state: &str, same_host: bool,
    can_cancel_pending: bool, silence_seconds: i64, at: &str) -> Result<Value, E> {
    if silence_seconds < 0 { return Err(E::Refused("cockpit-health-negative-silence")); }
    let mut message = match state {
        "queued/unconfirmed" | "starting/unconfirmed" => "Démarrage non confirmé",
        "unknown/other-host" => "État non vérifiable depuis cette session",
        "unknown/supervisor-lost" => "Superviseur perdu — fin non confirmée",
        "unknown/no-heartbeat" => "Signal absent — fin non confirmée",
        "running" => "Agent démarré", "stopping" => "Arrêt demandé — confirmation attendue",
        "completed" => "Exécution terminée — résultat à examiner", "failed" => "Exécution en échec",
        "interrupted" => "Arrêt confirmé", "queued" => "Démarrage en attente",
        "starting" => "Démarrage en cours", _ => "État non confirmé",
    };
    if agent.desired == "stop" && agent.is_active() { message = "Arrêt demandé — confirmation attendue"; }
    let mut activity = "unknown";
    let mut label = "Aucun résultat public reçu".to_owned();
    let result = swarm_wire_json::go_rfc3339::parse(&agent.progress.last_result_at).ok();
    if let Some(last) = result {
        let now = swarm_wire_json::go_rfc3339::parse(at)
            .map_err(|e| E::Runtime(crate::RuntimeError::new(crate::RuntimeOperation::Platform, e)))?;
        if (now-last).whole_nanoseconds() > i128::from(silence_seconds) * 1_000_000_000 {
            activity = "old"; label = "Dernier résultat ancien — progression non démontrée".into();
        } else { activity = "recent"; label = "Résultat public récent".into(); }
    }
    if terminal(state) {
        if result.is_some() { activity = "recorded"; label = "Résultat public reçu — tentative terminée".into(); }
        else { activity = "unknown"; label = "Tentative terminée — aucun résultat public enregistré".into(); }
    }
    if !agent.progress.degraded.is_empty() {
        activity = "unknown"; label = format!("Flux d'activité incomplet : {}", agent.progress.degraded);
    }
    if agent.mode == "terminal" { activity = "unknown"; label = "Terminal interactif — progression et appels d’outils non mesurés".into(); }
    Ok(json!({"can_cancel_pending":can_cancel_pending,"same_host":same_host,"agent_id":agent.id,
        "task_attempt_id":agent.attempt_id,"process_state":state,"process_label":message,
        "activity_state":activity,"activity_label":label,"last_result_at":agent.progress.last_result_at,
        "output_state":known(&agent.progress.output_state),"last_output_at":agent.progress.last_output_at,
        "tool_state":known(&agent.progress.tool_state),"provider_vitality":known(&agent.progress.provider_vitality),
        "completion_state":if terminal(state){"finished"}else{"open"},"completion_rule":"process_wait_confirmed",
        "threshold_seconds":silence_seconds,"observed_at":at,"heartbeat":agent.heartbeat,
        "supervisor_heartbeat":agent.heartbeat,"stop_requested":agent.desired=="stop"}))
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Valid current snapshot precedes malformed clock/negative-threshold refusal.
    #[test]
    fn health_clock_threshold_and_refusal_preserve_input() {
        let mut a = crate::Agent { status:"running".into(), ..Default::default() };
        a.progress.last_result_at = "2026-10-28T12:00:00Z".into();
        assert_eq!(project(&a,"running",true,false,5,"2026-10-28T12:00:05Z").unwrap()["activity_state"],"recent");
        assert_eq!(project(&a,"running",true,false,5,"2026-10-28T12:00:06Z").unwrap()["activity_state"],"old");
        assert!(project(&a,"running",true,false,-1,"2026-10-28T12:00:05Z").is_err());
        assert!(project(&a,"running",true,false,5,"invalid").is_err());
        assert_eq!(a.progress.last_result_at,"2026-10-28T12:00:00Z");
    }
    /// Terminal results remain recorded, while degraded and PTY modes override.
    #[test]
    fn health_terminal_degraded_and_interactive_precedence() {
        let mut a = crate::Agent { status:"completed".into(), ..Default::default() };
        assert_eq!(project(&a,"completed",false,false,5,"2026-10-28T12:00:05Z").unwrap()["activity_state"],"unknown");
        a.progress.last_result_at="2026-10-28T12:00:00Z".into();
        assert_eq!(project(&a,"completed",false,false,5,"2026-10-28T12:01:05Z").unwrap()["activity_state"],"recorded");
        a.progress.degraded="fixture partial stream".into();
        let v=project(&a,"completed",false,false,5,"2026-10-28T12:01:05Z").unwrap();
        assert_eq!(v["activity_label"],"Flux d'activité incomplet : fixture partial stream");
        a.mode="terminal".into();
        assert_eq!(project(&a,"completed",false,false,5,"2026-10-28T12:01:05Z").unwrap()["activity_label"],"Terminal interactif — progression et appels d’outils non mesurés");
    }
    /// Active stored intent, not the inverse of observed terminal states, owns
    /// the stop label. Host/cancellation facts remain independently supplied.
    #[test]
    fn health_stop_label_uses_active_intent_without_inventing_liveness() {
        let mut a=crate::Agent { status:"running".into(),desired:"stop".into(),..Default::default() };
        let v=project(&a,"unknown/other-host",false,false,5,"2026-10-28T12:00:00Z").unwrap();
        assert_eq!(v["process_label"],"Arrêt demandé — confirmation attendue");
        assert_eq!(v["same_host"],false);assert_eq!(v["can_cancel_pending"],false);
        a.status="completed".into();
        assert_eq!(project(&a,"unknown/other-host",false,false,5,"2026-10-28T12:00:00Z").unwrap()["process_label"],"État non vérifiable depuis cette session");
    }
}
