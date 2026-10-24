//! Read-only automatic dispatch projection for ordinary and initial planned todo tasks.
//! Mirrors Go missionDispatchState then assistCanStart/startBlockReason; no
//! admission, provider probe, lease, reservation, authentication or launch.
use super::RootStoreError as E;
use crate::mission_status_tasks::*;

/// Complete borrowed observations: validated saved slots, selected Work's full
/// Agent history and global active cwd rows, never the display-window subset.
/// Budget is already strictly read from persisted ledgers; None is unlimited.
/// Caller owns freshness/coherence checks and Root close/output error handling.
pub(super) struct AutomaticInputs<'a> {
    pub work: &'a crate::Work,
    pub task: &'a crate::Task,
    pub slots: i64,
    pub budget: Option<&'a crate::WorkBudgetState>,
    pub occupied: &'a [String],
    pub agents: &'a [crate::Agent],
    pub root: &'a std::path::Path,
}

/// Go workspaceOverlap on recorded strings, removing exactly one trailing slash.
/// No canonicalization/IO or ownership inference; even empty/absolute inputs keep
/// their historical lexical behavior. True is overlap metadata, never a lease.
fn overlaps(a: &str, b: &str) -> bool {
    let a = a.strip_suffix('/').unwrap_or(a);
    let b = b.strip_suffix('/').unwrap_or(b);
    a == b || a.starts_with(&format!("{b}/")) || b.starts_with(&format!("{a}/"))
}

/// Match Go startBlockReason's raw SQL root/cwd containment expression.
/// Unlike the dispatcher helper this removes no trailing slash. It explains an
/// already held assist check; it does not replace the effective profile workspace.
fn root_overlaps(root: &str, cwd: &str) -> bool {
    cwd == root || cwd.starts_with(&format!("{root}/")) || root.starts_with(&format!("{cwd}/"))
}

/// Project automatic readiness after caller's dependency/cost/profile/pause gates.
/// Requires an initial todo without attempts or matching Agent, saved positive
/// slots, and a real profile (task takes precedence). Managed copies and
/// history refuse rather than inventing producers. Counts full active Work Agents
/// for slots and global recorded cwd for workspace before assist checks, as Go.
/// Brief mismatch and insufficient estimated budget hold assistance; the pure
/// task consumer turns that into intervention. Invalid inputs/count/UTF8 refuse.
/// No persisted state or external process changes; ready remains classification.
pub(super) fn observe(input: AutomaticInputs<'_>) -> Result<ReadinessObservation, E> {
    let AutomaticInputs { work, task, slots, budget, occupied, agents, root } = input;
    if let Some(planning) = work.planning.as_deref() {
        if planning.repository.is_some() {
            return Err(E::Refused("mission-status-managed-automatic-readiness-unported"));
        }
        crate::validate_mission_organization(work).map_err(|cause| {
            E::Mutation(crate::WorkEngineError::Mission(Box::new(crate::MissionSupervisionError::Cause {
                code:cause.code(),cause:Box::new(cause),
            })))
        })?;
    }
    if task.status != "todo" || task.attempts.as_ref().is_some_and(|v| !v.is_empty())
        || agents.iter().any(|a| a.task_id == task.id)
    {
        return Err(E::Refused("mission-status-automatic-task-history-unported"));
    }
    if slots < 1 { return Err(E::Refused("mission-status-automatic-slots-invalid")); }
    let profile = task.launch_profile.as_deref().or(work.launch_profile.as_deref())
        .ok_or(E::Refused("mission-status-ready-profile-missing"))?;
    let observed = |disposition, reason: String, assistance, workspace| ReadinessObservation::Observed {
        disposition, reason, profile: ProfileObservation::Configured, assistance, workspace,
    };
    let busy = i64::try_from(agents.iter().filter(|a| a.is_active()).count())
        .map_err(|_| E::Refused("mission-status-count-representation"))?;
    if busy >= slots {
        return Ok(observed(DispatchDisposition::Waiting,
            format!("Aucun départ automatique : {busy} créneau(x) occupé(s) sur {slots}"),
            AssistObservation::NotObserved, WorkspaceObservation::NotObserved));
    }
    if occupied.iter().any(|cwd| overlaps(cwd, &profile.workspace)) {
        return Ok(observed(DispatchDisposition::Waiting,
            format!("{} : espace de travail déjà occupé par une tentative active", task.id),
            AssistObservation::NotObserved, WorkspaceObservation::Occupied));
    }
    let held = if !task.plan_brief_hash.is_empty()
        && work.planning_brief.as_ref().is_none_or(|b| b.sha256 != task.plan_brief_hash)
    {
        Some("Brief du plan modifié : relisez le brief avant de lancer.")
    } else if budget.is_some_and(|b| b.limit_usd != 0.0
        && (b.limit_usd - b.committed_usd).max(0.0) < b.reserve_per_launch_usd)
    {
        let root = root.to_str().ok_or(E::Refused("mission-status-root-utf8"))?;
        if occupied.iter().any(|cwd| root_overlaps(root, cwd)) {
            Some("Un agent occupe déjà cet espace de travail.")
        } else { Some("Budget estimatif insuffisant pour un nouveau départ.") }
    } else { None };
    Ok(observed(DispatchDisposition::Ready,"Prête pour un départ automatique".into(),
        held.map_or(AssistObservation::Allowed, |reason| AssistObservation::Held { reason: reason.into() }),
        WorkspaceObservation::Free))
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Real DTO shapes, fixture strings only; no provider, persisted grant or IO.
    fn fixture() -> (crate::Work, crate::Task, crate::WorkBudgetState) {
        (crate::Work { id:"w".into(), launch_profile:Some(Box::new(crate::LaunchProfile {
            workspace:"/root/worker".into(), ..Default::default() })), ..Default::default() },
         crate::Task { id:"t".into(), status:"todo".into(), ..Default::default() },
         crate::WorkBudgetState { work_id:"w".into(),limit_usd:5.0,reserve_per_launch_usd:1.0,
            committed_usd:4.0,reported_usd:0.0,attempts_without_cost:0 })
    }
    /// All owned observations supplied without truncation or synthetic Agent state.
    fn input<'a>(w:&'a crate::Work,t:&'a crate::Task,b:Option<&'a crate::WorkBudgetState>) -> AutomaticInputs<'a> {
        AutomaticInputs { work:w,task:t,slots:1,budget:b,occupied:&[],agents:&[],root:std::path::Path::new("/root") }
    }
    /// Match a successful classification; does not establish launch admission.
    fn allowed(o:ReadinessObservation) -> bool {
        matches!(o,ReadinessObservation::Observed {disposition:DispatchDisposition::Ready,
            assistance:AssistObservation::Allowed,workspace:WorkspaceObservation::Free,..})
    }
    #[test] fn equal_paths_overlap() { assert!(overlaps("/w/a","/w/a")); }
    #[test] fn child_of_other_overlaps() { assert!(overlaps("/w/a","/w/a/b")); }
    #[test] fn siblings_dont_overlap() { assert!(!overlaps("/w/a","/w/ab")); }
    #[test] fn trailing_slash_removed_once() { assert!(overlaps("/w/a/","/w/a")); assert!(overlaps("/w/a//","/w/a")); }
    #[test] fn root_overlaps_exact_no_trim() { assert!(root_overlaps("/w/a","/w/a")); }
    #[test] fn root_overlaps_ancestor_and_descendant() {
        assert!(root_overlaps("/w/a","/w/a/b"));assert!(root_overlaps("/w/a/b","/w/a"));
        assert!(root_overlaps("/w/a/","/w/a"));assert!(!root_overlaps("/w/a","/w/ab"));
        assert!(overlaps("","/x"));
    }
    #[test] fn budget_absent_and_zero_are_unlimited() {
        let (w,t,mut b)=fixture();assert!(allowed(observe(input(&w,&t,None)).unwrap()));
        b.limit_usd=0.0;b.committed_usd=100.0;assert!(allowed(observe(input(&w,&t,Some(&b))).unwrap()));
    }
    #[test] fn budget_equality_allowed_insufficient_held_and_restored() {
        let (w,t,mut b)=fixture();assert!(allowed(observe(input(&w,&t,Some(&b))).unwrap()));
        b.committed_usd=4.5;
        assert!(matches!(observe(input(&w,&t,Some(&b))).unwrap(),ReadinessObservation::Observed {
            assistance:AssistObservation::Held {reason},..} if reason=="Budget estimatif insuffisant pour un nouveau départ."));
        b.committed_usd=4.0;assert!(allowed(observe(input(&w,&t,Some(&b))).unwrap()));
    }
    #[test] fn brief_mismatch_precedes_budget() {
        let (w,mut t,mut b)=fixture();b.committed_usd=10.0;t.plan_brief_hash="saved".into();
        assert!(matches!(observe(input(&w,&t,Some(&b))).unwrap(),ReadinessObservation::Observed {
            assistance:AssistObservation::Held {reason},..} if reason=="Brief du plan modifié : relisez le brief avant de lancer."));
    }
    #[test] fn workspace_wait_and_task_profile_precedence() {
        let (w,mut t,_)=fixture();let occupied=vec!["/root/worker/sub".into()];
        let mut i=input(&w,&t,None);i.occupied=&occupied;
        assert!(matches!(observe(i).unwrap(),ReadinessObservation::Observed {disposition:DispatchDisposition::Waiting,..}));
        t.launch_profile=Some(Box::new(crate::LaunchProfile { workspace:"/disjoint".into(),..Default::default() }));
        let mut i=input(&w,&t,None);i.occupied=&occupied;assert!(allowed(observe(i).unwrap()));
    }
    #[test] fn full_work_active_agents_consume_slots() {
        let (w,t,_)=fixture();let agents=vec![crate::Agent {task_id:"other".into(),status:"queued".into(),..Default::default()}];
        let mut i=input(&w,&t,None);i.agents=&agents;
        assert!(matches!(observe(i).unwrap(),ReadinessObservation::Observed {disposition:DispatchDisposition::Waiting,reason,..}
            if reason=="Aucun départ automatique : 1 créneau(x) occupé(s) sur 1"));
    }
    #[test] fn budget_hold_uses_root_explanation_after_profile_is_free() {
        let (mut w,t,mut b)=fixture();w.launch_profile.as_mut().unwrap().workspace="/disjoint".into();b.committed_usd=9.0;
        let occupied=vec!["/root/other".into()];let mut i=input(&w,&t,Some(&b));i.occupied=&occupied;
        assert!(matches!(observe(i).unwrap(),ReadinessObservation::Observed {assistance:AssistObservation::Held {reason},..}
            if reason=="Un agent occupe déjà cet espace de travail."));
    }
    #[test] fn invalid_slots_and_history_refuse() {
        let (w,mut t,_)=fixture();assert!(allowed(observe(input(&w,&t,None)).unwrap()));
        let mut i=input(&w,&t,None);i.slots=0;assert!(observe(i).is_err());
        t.status="blocked".into();assert!(observe(input(&w,&t,None)).is_err());
    }
}
