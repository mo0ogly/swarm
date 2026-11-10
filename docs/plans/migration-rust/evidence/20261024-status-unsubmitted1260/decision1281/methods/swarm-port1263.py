from pathlib import Path
import sys,json
sys.path.insert(0,'.cache/rust-git-port');from editlib import atomic
p=Path('rust/crates/swarm-engine/src/root_store/mission_status_reports.rs');s=p.read_text()
def replace(old,new):
 global s
 assert s.count(old)==1,(old[:80],s.count(old));s=s.replace(old,new)
anchor='/// Go operationText masking and terminal-cell clipping for fallback traces.'
helper='''/// Observe Go provenReport over Root docs, not the worker's workspace. The
/// supplied nonnegative margin comes from persisted settings; it is not a local
/// fallback. Nonempty regular candidates at/after start-minus-margin are fresh;
/// exactly one is attributable, many/old/empty candidates retain Go's reasons.
/// Invalid start retains its typed cause beside the explicit unavailable result.
/// Metadata/containment/scan/close errors propagate, never become empty success.
/// No content, authorship, quality, mutation or permission is established; these
/// sequential filesystem observations are not an atomic snapshot or an ABA fence.
fn proven_report(root: &Path, agent: &Agent, skew: i64, causes: &mut Vec<E>)
    -> Result<crate::mission_status_tasks::ReportAttribution, E> {
    use crate::mission_status_tasks::ReportAttribution::{Attributed, Unavailable};
    if skew < 0 { return Err(E::Refused("mission-status-report-clock-skew-invalid")); }
    let started = match swarm_wire_json::go_rfc3339::parse(&agent.started) {
        Ok(value) => value.unix_timestamp_nanos(),
        Err(error) => {
            causes.push(cause("mission-status-report-start-invalid", error));
            return Ok(Unavailable { reason: "date de départ de la tentative illisible".into() });
        }
    };
    // i64 seconds and the parsed timestamp fit i128 nanoseconds; this is a unit
    // conversion, never a business cutoff or configurable second policy ceiling.
    let since = started - i128::from(skew) * 1_000_000_000;
    let mut fresh = Vec::new();
    let mut stale = 0usize;
    for name in task_reports(root, &agent.task_id)? {
        let (_, metadata) = crate::gate_evaluation::artifact_metadata(root, &name)
            .map_err(|e| E::Gate(crate::GateContractError::Evaluation(e)))?;
        if metadata.len() == 0 { continue; }
        let modified = i128::from(metadata.mtime()) * 1_000_000_000 + i128::from(metadata.mtime_nsec());
        if modified < since { stale += 1; } else { fresh.push(name); }
    }
    match fresh.as_slice() {
        [name] => Ok(Attributed(name.clone())),
        [] => Ok(Unavailable { reason: if stale > 0 {
            format!("aucun rapport produit par cette tentative ; {stale} rapport(s) antérieur(s) ignoré(s)")
        } else { "aucun rapport lisible et non vide sous docs/ pour cette tâche".into() } }),
        _ => Ok(Unavailable { reason: format!("plusieurs rapports candidats pour cette tentative ({})", fresh.join(", ")) }),
    }
}
/// Narrow ordinary completion shape whose attribution producer is wired here.
/// Incomplete-delivery and current-review branches keep their separate evidence
/// requirements. This read-only shape is not validation of the original process.
fn unsubmitted_pair(task: &crate::Task, agent: &Agent) -> bool {
    task.id == agent.task_id && task.status == "blocked" && agent.status == "completed"
        && !task.blocker.starts_with("Livraison incomplète :")
}
'''
replace(anchor,helper+anchor)
replace('(agent.status=="completed" && matching_review(task,agent))','(agent.status=="completed" && (matching_review(task,agent) || unsubmitted_pair(task,agent)))')
replace('(task.status=="blocked" && agent.is_some_and(|a|matching_review(task,a)))','(task.status=="blocked" && agent.is_some_and(|a|matching_review(task,a) || (work.planning.is_some() && unsubmitted_pair(task,a))))')
replace('''    result_inputs_with_review(root,task,agent,facts.freshness,facts.gate,review)
''','''    let attribution = match agent {
        Some(agent) if unsubmitted_pair(task, agent)
            && matches!(&review, Some(crate::mission_status_tasks::ReviewObservation::NotApplicable)) => {
                Some(proven_report(root, agent, settings.values.report_clock_skew_seconds()
                    .map_err(E::Settings)?, causes)?)
            }
        _ => None,
    };
    result_inputs_with_review(root,task,agent,facts.freshness,facts.gate,review,attribution)
''')
replace('result_inputs_with_review(root,task,agent,freshness,gate,None)','result_inputs_with_review(root,task,agent,freshness,gate,None,None)')
replace('''    observed_review: Option<crate::mission_status_tasks::ReviewObservation>,
''','''    observed_review: Option<crate::mission_status_tasks::ReviewObservation>,
    attribution: Option<crate::mission_status_tasks::ReportAttribution>,
''')
replace('''&& !(task.status=="blocked" && matches!(&observed_review,Some(ReviewObservation::Presented(_))))''','''&& !(task.status=="blocked" && matches!(&observed_review,Some(ReviewObservation::Presented(_))))
            && !(unsubmitted_pair(task, agent) && attribution.is_some())''')
replace('''        attribution: ReportAttribution::NotObserved,
''','''        attribution: attribution.unwrap_or(ReportAttribution::NotObserved),
''')
replace('''/// Completed but unsubmitted without a current review, recovery/managed execution
/// and closed scopes still refuse at this port boundary, not a quota.''','''/// Completed blocked nonmanaged planned tasks may display actual Root-scoped
/// attribution without a current review. Incomplete delivery, other unsubmitted
/// shapes, recovery/managed execution and closed scopes still refuse here.''')
replace('''/// there. Display-window absence''','''/// there, including completed/blocked without incomplete delivery when actual
/// Root-scoped attribution was supplied. Display-window absence''')
replace('''/// No new authority, defaults, launch or mutation follows.''','''/// Completed/blocked fallthrough obtains attribution from Root docs and the
/// saved clock margin only after an observed NotApplicable review. Other branches
/// do not scan for attribution. No new authority, defaults, launch or mutation follows.''')
# Existing pure test adapter has no settings and intentionally cannot invent this observation.
atomic(p,s)
p=Path('version_history.json');d=json.loads(p.read_bytes());d['local_changes']['items'].insert(0,dict(fr='Candidat Rust : statut historique des tâches planifiées bloquées avec processus terminé ; attribution du rapport depuis docs/ et marge persistée, sans soumission ni validation implicite. Qualification en cours.',en='Rust candidate: historical status for blocked planned tasks with completed processes; report attribution from Root docs and saved margin, without implicit submission or validation. Qualification pending.'));atomic(p,json.dumps(d,ensure_ascii=False,indent=2)+'\n')
