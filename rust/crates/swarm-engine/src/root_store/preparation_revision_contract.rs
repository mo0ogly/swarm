//! root_store/preparation_revision_contract.rs — pure owned-value revision of an
//! already-converted preparation plan.
//!
//! Mirrors the supplied Go `revisePreparedMissions` body between the caller's
//! freshness / plan-hash / active-agent SQL / plan-validation checks and the
//! final cockpit pause SQL. No SQL, transaction, CAS, lock row, event row,
//! clock, RNG or provider call exists here; nothing is started and no model,
//! provider or named-human authority is created or attested. The caller owns
//! the transaction and must persist the returned Work body under its own
//! revision CAS, write the revision event and any launch-lock rows, assign the
//! returned conversion into the stored preparation, and record the cockpit
//! `paused` witness transactionally with this result. Binding between the
//! preparation and the Work is caller-owned, as in the Go store.
//!
//! Refusals are total and carry safe static reasons (`RootStoreError::Refused`);
//! typed engine errors are wrapped through `super::preparation_read::cause`
//! (static reason + typed cause). Nothing is swallowed. The operator, timestamp
//! and event inputs are caller observations recorded as provenance (Go
//! `operatorIdentity()` / `now()` / `r.Event`); they authenticate nothing and
//! imply no named-human attestation or model authority.
//!
//! Deliberate divergences from Go, all refusing instead of silently
//! continuing: an unchanged plan hash refuses instead of Go's silent no-op
//! (the caller must filter that case first, as the Go store does); malformed
//! or out-of-range `req-N` requirement identifiers refuse instead of Go's
//! `fmt.Sscanf` zero-value skip; a missing root scope and a missing approved
//! task refuse instead of Go's ignored-error nil dereference. Go's hardcoded
//! `PlanMaxAttempts == 4` recovery check is generalized, not copied — see
//! `revise_prepared_missions`. Dynamic Go error text (task titles/IDs) is
//! intentionally replaced by static reasons; `Refused` carries no dynamic data.

use super::preparation_read::cause;
use super::RootStoreError as E;
use crate::{
    ActionPlan, ApprovedPlan, ManualGatePhaseDefaults, PlanningEvent, Preparation,
    PreparationConversion, Task, ValidationPolicy, ValidationPolicyLimits, Work,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// Exact wire literal of the shared classifier's private "kept" constant.
/// Only exact equality with this value counts as unimpacted; every other kind
/// — including any future label — is treated as impacted, as in Go.
const KIND_KEPT: &str = "conservée";

/// Caller-supplied current engine observations, recorded as provenance only.
#[derive(Clone, Copy)]
pub(super) struct RevisionObservation<'a> {
    /// Current engine timestamp (Go `now()`).
    pub(super) at: &'a str,
    /// Current request event identifier (Go `r.Event`).
    pub(super) event: &'a str,
    /// Current operator observation (Go `operatorIdentity()`).
    pub(super) operator: &'a str,
}

/// Strictly parse a `req-N` planning requirement identifier. Go's
/// `fmt.Sscanf` accepts trailing garbage and silently yields zero for
/// malformed input; this port is checked: empty, non-numeric or overflowing
/// identifiers yield `None`, and the caller refuses instead of silently
/// skipping a criterion update.
fn parse_requirement_index(raw: &str) -> Option<usize> {
    let digits = raw.strip_prefix("req-")?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse::<usize>().ok()
}

/// Refuse when one more planning requirement would exceed the persisted limit.
/// Checked arithmetic only; no invented cap exists when the limit is `None`.
fn requirement_slot(
    criteria_len: usize,
    planning_requirement_limit: Option<usize>,
) -> Result<usize, E> {
    let next_len = criteria_len
        .checked_add(1)
        .ok_or(E::Refused("preparation-revision-requirement-overflow"))?;
    if planning_requirement_limit.is_some_and(|max| next_len > max) {
        return Err(E::Refused("preparation-revision-requirement-limit"));
    }
    Ok(next_len)
}

/// Apply an explicitly validated plan revision to an owned Work and return the
/// replacement conversion. Ordering, conditions and refusals mirror the Go
/// mutation body from the temporary materialization through the organization
/// guard; see the module docs for caller duties, honesty boundaries and the
/// documented divergences (no-op, req-N parsing, root lookups, hardcoded 4).
pub(super) fn revise_prepared_missions(
    mut work: Work,
    preparation: &Preparation,
    spec: &ActionPlan,
    observation: &RevisionObservation<'_>,
    defaults: &ManualGatePhaseDefaults,
    limits: &ValidationPolicyLimits,
    planning_requirement_limit: Option<usize>,
) -> Result<(Work, PreparationConversion), E> {
    // Structural prerequisites re-verified with static refusals (Go would
    // either fail upstream, silently no-op, or nil-dereference).
    let conversion = preparation
        .conversion
        .as_deref()
        .ok_or(E::Refused("preparation-revision-missing-conversion"))?;
    if !preparation.plan_ready {
        return Err(E::Refused("preparation-stale-plan"));
    }
    let plan_doc = preparation
        .documents
        .as_ref()
        .and_then(|d| d.get("plan"))
        .ok_or(E::Refused("preparation-stale-plan"))?;
    // Go returns a silent success no-op here; the caller must filter it first,
    // so a pure transform reaching this point refuses rather than duplicate
    // the approved plan and counters.
    if plan_doc.sha256 == conversion.plan_hash {
        return Err(E::Refused("preparation-revision-unchanged-plan"));
    }
    let brief = preparation
        .adopted_brief
        .as_deref()
        .ok_or(E::Refused("preparation-stale-brief"))?;

    let task_ids:BTreeSet<_>=work.tasks.iter().flatten().map(|task|task.id.as_str()).collect();
    if task_ids.len()!=work.tasks.as_ref().map_or(0,Vec::len){return Err(E::Refused("preparation-revision-task-id-ambiguous"));}

    // Go scope guard: nothing held, exactly one scope, and that scope is root
    // (Go would nil-dereference later when the root lookup fails).
    if let Some(planning) = work.planning.as_deref() {
        let scopes = planning.scopes.as_deref().unwrap_or(&[]);
        if scopes.iter().any(|scope| !scope.holder.is_empty()) {
            return Err(E::Refused("preparation-revision-scope-held"));
        }
        if scopes.len() != 1 {
            return Err(E::Refused("preparation-revision-scope-branching"));
        }
        if scopes[0].id != "root" {
            return Err(E::Refused("preparation-revision-root-missing"));
        }
    }

    // Temporary materialization through the shared conversion path. The temp
    // Work carries the target identity fields so task addition behaves exactly
    // as on the reviewed creation path; nothing else leaks into the result.
    let mut temp = Work {
        schema_version: work.schema_version,
        id: work.id.clone(),
        revision: work.revision,
        created: work.created.clone(),
        updated: work.updated.clone(),
        tasks: Some(Vec::new()),
        ..Default::default()
    };
    let (ids, done) =
        super::preparation_conversion::populate(&mut temp, &preparation.id, spec, defaults)?;
    let temp_tasks = temp.tasks.take().unwrap_or_default();
    // Preserve populate's authored (dependency-ordered) task order; the lookup
    // map is never iterated.
    let temp_order: Vec<String> = temp_tasks.iter().map(|t| t.id.clone()).collect();
    let next_by: BTreeMap<String, Task> =
        temp_tasks.into_iter().map(|t| (t.id.clone(), t)).collect();
    let old_ids: BTreeSet<&str> = conversion
        .task_ids
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(String::as_str)
        .collect();
    // Same prefix rule as Go and as populate (`plan-` + hash(p.ID)[:10] + `-`).
    let prefix =
        format!("plan-{}-", &format!("{:x}", Sha256::digest(preparation.id.as_bytes()))[..10]);
    let mut changed: BTreeSet<String> =
        crate::preparation_plan_changes::changes(&conversion.spec, spec)
            .into_iter()
            .filter(|change| change.kind != KIND_KEPT)
            .map(|change| format!("{prefix}{}", change.id))
            .collect();

    let tasks = work.tasks.get_or_insert_with(Vec::new);
    let mut planning = work.planning.as_deref_mut();

    // Revalidation propagates through every dependent, including tasks outside
    // this plan (external dependents of the old graph are impacted too).
    let mut again = true;
    while again {
        again = false;
        for task in tasks.iter() {
            if changed.contains(task.id.as_str()) {
                continue;
            }
            for dep in task.depends.iter().flatten() {
                if changed.contains(dep.as_str()) {
                    changed.insert(task.id.clone());
                    again = true;
                    break;
                }
            }
        }
    }

    for index in 0..tasks.len() {
        let task = &mut tasks[index];
        if !changed.contains(task.id.as_str()) {
            continue;
        }
        if task.status == "running"
            || task.independent_review.as_ref().is_some_and(|r| r.state == "running")
        {
            return Err(E::Refused("preparation-revision-task-running"));
        }
        if planning.is_some()
            && old_ids.contains(task.id.as_str())
            && !next_by.contains_key(task.id.as_str())
        {
            return Err(E::Refused("preparation-revision-retires-requirement"));
        }
        // Prior authored mission for this locked task, matched by the same
        // prefixed-ID rule as Go (`prefix+prior.ID == t.ID`); validated plans
        // carry unique authored IDs.
        let prior_mission = conversion
            .spec
            .tasks
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .find(|m| task.id.strip_prefix(prefix.as_str()) == Some(m.id.as_str()));
        if let Some(nt) = next_by.get(task.id.as_str()).cloned() {
            let has_controls = task
                .validation_policy
                .as_ref()
                .is_some_and(|p| p.controls.as_ref().is_some_and(|c| !c.is_empty()));
            if has_controls {
                if let Some(prior) = prior_mission {
                    if let Some(next_mission) =
                        spec.tasks.as_deref().unwrap_or(&[]).iter().find(|m| m.id == prior.id)
                    {
                        if prior.entry != next_mission.entry
                            || prior.validation != next_mission.validation
                            || prior.delivery != next_mission.delivery
                            || prior.proof != next_mission.proof
                            || prior.scope != next_mission.scope
                            || prior.deliverable != next_mission.deliverable
                        {
                            return Err(E::Refused("preparation-revision-controls-sense"));
                        }
                    }
                }
                if task.criteria != nt.criteria {
                    return Err(E::Refused("preparation-revision-controls-criteria"));
                }
            }
            if planning.is_some()
                && nt.criteria.as_ref().map_or(0, Vec::len)
                    < task.criteria.as_ref().map_or(0, Vec::len)
            {
                return Err(E::Refused("preparation-revision-criteria-shrink"));
            }
            task.title = nt.title.clone();
            task.deliverable = nt.deliverable.clone();
            task.criteria = nt.criteria.clone();
            task.depends = nt.depends.clone();
            task.next = nt.next.clone();
            task.plan_role = nt.plan_role.clone();
            task.plan_checks = nt.plan_checks.clone();
            // Preserve the stored, separately adjusted limit when the authored
            // plan limit is unchanged. This retains data; it grants no extra
            // reservation/call or identity. Explicit authored changes replace it.
            let preserve_recovery = task.corrective_recovery.is_some()
                && prior_mission.is_some_and(|prior| prior.max_attempts == nt.plan_max_attempts);
            if !preserve_recovery {
                task.plan_max_attempts = nt.plan_max_attempts;
            }
            task.plan_tool_limit = nt.plan_tool_limit;
            if let Some(pl) = planning.as_deref_mut() {
                if !pl.human_review_authorized.is_empty() {
                    let root = pl
                        .scopes
                        .as_deref_mut()
                        .and_then(|scopes| scopes.iter_mut().find(|scope| scope.id == "root"))
                        .ok_or(E::Refused("preparation-revision-root-missing"))?;
                    let criteria = work.criteria.get_or_insert_with(Vec::new);
                    for (n, criterion) in
                        task.criteria.clone().unwrap_or_default().into_iter().enumerate()
                    {
                        if n < task.requirements.as_ref().map_or(0, Vec::len) {
                            let raw = task
                                .requirements
                                .as_ref()
                                .and_then(|r| r.get(n))
                                .map(String::as_str)
                                .unwrap_or_default();
                            let requirement = parse_requirement_index(raw).ok_or(E::Refused(
                                "preparation-revision-requirement-id-invalid",
                            ))?;
                            let slot = requirement
                                .checked_sub(1)
                                .filter(|&slot| slot < criteria.len())
                                .ok_or(E::Refused(
                                    "preparation-revision-requirement-out-of-range",
                                ))?;
                            criteria[slot] = criterion;
                        } else {
                            let next_len =
                                requirement_slot(criteria.len(), planning_requirement_limit)?;
                            criteria.push(criterion);
                            let req = format!("req-{next_len}");
                            task.requirements.get_or_insert_with(Vec::new).push(req.clone());
                            root.requirements.get_or_insert_with(Vec::new).push(req);
                        }
                    }
                }
            }
        }
        // Every impacted task needs a fresh review, including dependents whose
        // contract text is unchanged. The verdict and spent calls stay in
        // history; nothing else about the task's consumption is rewritten.
        if let Some(review) = task.independent_review.take() {
            task.previous_reviews.get_or_insert_with(Vec::new).push(*review);
        }
        task.status = "todo".into();
        task.gate = None;
        task.automatic_validation = None;
        task.manual_override = None;
        task.revalidation = None;
        task.blocker = String::new();
        task.planning_retry = false;
        if old_ids.contains(task.id.as_str()) && !next_by.contains_key(task.id.as_str()) {
            task.status = "abandoned".into();
            task.blocker = "Retirée par la révision du plan ; historique conservé.".into();
        }
    }

    for id in &temp_order {
        let mut nt = next_by
            .get(id).cloned()
            .ok_or(E::Refused("preparation-revision-materialization-invisible"))?;
        if tasks.iter().any(|t| t.id == nt.id) {
            if !old_ids.contains(nt.id.as_str()) {
                return Err(E::Refused("preparation-revision-task-id-collide"));
            }
            continue;
        }
        if let Some(pl) = planning.as_deref_mut() {
            if pl.human_review_authorized.is_empty() {
                return Err(E::Refused("preparation-revision-new-task-unauthorized"));
            }
            let root = pl
                .scopes
                .as_deref_mut()
                .and_then(|scopes| scopes.iter_mut().find(|scope| scope.id == "root"))
                .ok_or(E::Refused("preparation-revision-root-missing"))?;
            nt.scope_id = "root".into();
            let criteria = work.criteria.get_or_insert_with(Vec::new);
            for criterion in nt.criteria.clone().unwrap_or_default() {
                let next_len = requirement_slot(criteria.len(), planning_requirement_limit)?;
                criteria.push(criterion);
                let req = format!("req-{next_len}");
                nt.requirements.get_or_insert_with(Vec::new).push(req.clone());
                root.requirements.get_or_insert_with(Vec::new).push(req);
            }
            // New human policy: normalized against the caller's current limits,
            // then stamped with the caller observation. Go writes
            // mode/authorized/actor directly; normalization here validates the
            // policy contract instead of bypassing it with a literal default.
            // Coverage of the new task is enforced by the organization
            // validation below, exactly where the Go guard enforces it.
            let mut policy = crate::normalize_validation_policy(
                ValidationPolicy {
                    mode: "human".into(),
                    controls: None,
                    authorized_at: String::new(),
                    actor: String::new(),
                },
                limits,
            )
            .map_err(|e| cause("preparation-revision-policy", e))?;
            policy.authorized_at = observation.at.to_string();
            policy.actor = observation.operator.to_string();
            nt.validation_policy = Some(Box::new(policy));
        }
        tasks.push(nt);
    }

    // External dependents of retired tasks cannot be silently made runnable.
    for task in tasks.iter() {
        if task.status == "abandoned" {
            continue;
        }
        for dep in task.depends.iter().flatten() {
            if old_ids.contains(dep.as_str()) && !next_by.contains_key(dep.as_str()) {
                return Err(E::Refused("preparation-revision-retired-dependency"));
            }
        }
    }

    work.objective = spec.objective.clone();
    work.scope = brief.text.clone();
    work.plans.get_or_insert_with(Vec::new).push(ApprovedPlan {
        source: preparation.id.clone(),
        brief_hash: brief.sha256.clone(),
        response_hash: plan_doc.sha256.clone(),
        spec: spec.clone(),
        task_ids: Some(ids.clone()),
        task_map: Some(done),
        at: observation.at.to_string(),
    });
    for id in &ids {
        let task = tasks
            .iter_mut()
            .find(|t| &t.id == id)
            .ok_or(E::Refused("preparation-revision-launch-task-missing"))?;
        task.launch_held = true;
    }

    let conversion_next = PreparationConversion {
        work_id: work.id.clone(),
        plan_hash: plan_doc.sha256.clone(),
        brief_hash: brief.sha256.clone(),
        method_hash: preparation.method_hash.clone(),
        task_ids: Some(ids),
        spec: spec.clone(),
        created_at: observation.at.to_string(),
        ..Default::default()
    };

    if work.planning.is_some() {
        if let Some(pl) = work.planning.as_deref_mut() {
            let root = pl
                .scopes
                .as_deref_mut()
                .and_then(|scopes| scopes.iter_mut().find(|scope| scope.id == "root"))
                .ok_or(E::Refused("preparation-revision-root-missing"))?;
            root.objective = spec.objective.clone();
            root.revision = root
                .revision
                .checked_add(1)
                .ok_or(E::Refused("preparation-revision-scope-counter-overflow"))?;
            root.generation = root
                .generation
                .checked_add(1)
                .ok_or(E::Refused("preparation-revision-scope-counter-overflow"))?;
            root.state = "waiting".into();
            pl.paused = true;
            let decision = format!("revision-{}", observation.event);
            for event in pl.inbox.iter_mut().flatten() {
                if event.decision.is_empty() {
                    event.decision = decision.clone();
                }
            }
            pl.inbox.get_or_insert_with(Vec::new).push(PlanningEvent {
                id: decision,
                scope: "root".into(),
                kind: "plan_revision".into(),
                message: "Plan révisé par l’opérateur. Conserver les missions et leurs critères ; examiner les retours avant toute nouvelle tâche.".into(),
                at: observation.at.to_string(),
                ..Default::default()
            });
        }
        // Same control semantics and criteria as the Go organization guard;
        // also enforces policy/task coverage for every task, including the
        // freshly stamped new human policies.
        crate::validate_mission_organization(&work)
            .map_err(|e| cause("preparation-revision-organization", e))?;
    }

    Ok((work, conversion_next))
}

#[cfg(test)]
mod tests {
 use super::*;
 use crate::{Attempt,IndependentReview,CorrectiveRecovery,PreparationDocument,EngineSettingsValues,EngineMutationLimits,GateRecord,PlanningState,PlanningScope,ReviewerConfig};
 /// Complete owned data fixture, not recorded execution, review or authority.
 /// Uses shipped configuration only as explicit TEST input; production uses
 /// the caller's saved settings. Both plans are assumed validated by the caller.
 fn fixture(limit:i64)->(Work,Preparation,ActionPlan,EngineMutationLimits){
  let limits=EngineSettingsValues::installation_defaults().unwrap().mutation_limits().unwrap();
  let mission=|id:&str,depends:Vec<String>|crate::PlanMission{id:id.into(),title:id.into(),role:"worker".into(),scope:"scope".into(),deliverable:"artifact".into(),depends:Some(depends),criteria:Some(vec![format!("{id} evidence")]),proof:"proof".into(),entry:"entry".into(),validation:"validation".into(),delivery:"delivery".into(),stop:"stop".into(),max_attempts:limit,max_tool_calls:1,..Default::default()};
  let old=ActionPlan{version:1,objective:"objective".into(),assumptions:Some(vec![]),questions:Some(vec![]),tasks:Some(vec![mission("a",vec![]),mission("b",vec!["a".into()]),mission("c",vec![])]),..Default::default()};
  let mut work=Work{schema_version:1,id:"w-revision-unit".into(),revision:2,tasks:Some(vec![]),criteria:Some(vec!["initial".into()]),..Default::default()};
  let (ids,map)=super::super::preparation_conversion::populate(&mut work,"p-revision-unit",&old,limits.task.manual_gate_phases.as_ref().unwrap()).unwrap();
  work.plans=Some(vec![ApprovedPlan{source:"p-revision-unit".into(),spec:old.clone(),task_ids:Some(ids.clone()),task_map:Some(map),..Default::default()}]);
  let mut next=old.clone();next.tasks.as_mut().unwrap()[0].title="changed a".into();next.tasks.as_mut().unwrap().pop();next.tasks.as_mut().unwrap().push(mission("d",vec!["b".into()]));
  let preparation=Preparation{id:"p-revision-unit".into(),work_id:work.id.clone(),plan_ready:true,adopted_brief:Some(Box::new(PreparationDocument{sha256:"brief-fixture".into(),text:"brief".into(),..Default::default()})),documents:Some(BTreeMap::from([("plan".into(),PreparationDocument{sha256:"next-fixture".into(),..Default::default()})])),conversion:Some(Box::new(PreparationConversion{work_id:work.id.clone(),plan_hash:"old-fixture".into(),task_ids:Some(ids),spec:old,..Default::default()})),..Default::default()};
  (work,preparation,next,limits)
 }
 /// Apply only a supplied data transform; no provider, Root or acceptance.
 fn apply(work:Work,p:&Preparation,spec:&ActionPlan,limits:&EngineMutationLimits)->Result<(Work,PreparationConversion),E>{
  revise_prepared_missions(work,p,spec,&RevisionObservation{at:"2026-10-27T00:00:00Z",event:"unit-revision",operator:"fixture-observation"},limits.task.manual_gate_phases.as_ref().unwrap(),&limits.policy,None)
 }
 /// Explicit synthetic planning metadata, verified structurally before faults.
 /// Consumption/budgets are fixture inputs, never a grant to launch a model.
 fn planning(work:&mut Work){
  let mut criteria=Vec::new();let mut requirements=Vec::new();
  for task in work.tasks.iter_mut().flatten(){task.scope_id="root".into();task.validation_policy=Some(Box::new(ValidationPolicy{mode:"human".into(),authorized_at:"fixture".into(),actor:"fixture".into(),..Default::default()}));let mut selected=Vec::new();for c in task.criteria.iter().flatten(){criteria.push(c.clone());let id=format!("req-{}",criteria.len());requirements.push(id.clone());selected.push(id);}task.requirements=Some(selected);}
  work.criteria=Some(criteria);work.planning=Some(Box::new(PlanningState{version:1,provider:"fixture".into(),human_review_authorized:"fixture".into(),max_tasks:8,max_decisions:9,max_activations:10,activations:2,decisions:3,reviewer:Some(Box::new(ReviewerConfig{provider:"fixture".into(),authorized:"fixture".into(),max_calls:11,calls:4,..Default::default()})),scopes:Some(vec![PlanningScope{id:"root".into(),revision:3,generation:2,state:"waiting".into(),requirements:Some(requirements),..Default::default()}]),..Default::default()}));
  crate::validate_mission_organization(work).unwrap();
 }
 #[test]
 fn preserves_history_recovery_and_dependency_membership(){
  for stored in [4,6]{
   let (mut work,p,spec,limits)=fixture(1);let task=&mut work.tasks.as_mut().unwrap()[0];task.status="done".into();task.plan_max_attempts=stored;task.corrective_recovery=Some(Box::new(CorrectiveRecovery::default()));task.attempts=Some(vec![Attempt{id:"old-attempt".into(),status:"done".into(),..Default::default()}]);task.independent_review=Some(Box::new(IndependentReview{state:"passed".into(),..Default::default()}));task.gate=Some(Box::new(GateRecord::default()));let attempts=task.attempts.clone();
   let (revised,c)=apply(work.clone(),&p,&spec,&limits).unwrap();let tasks=revised.tasks.as_ref().unwrap();assert_eq!(tasks[0].plan_max_attempts,stored);assert_eq!(tasks[0].attempts,attempts);assert_eq!(tasks[0].previous_reviews.as_ref().unwrap().len(),1);assert!(tasks[0].independent_review.is_none()&&tasks[0].gate.is_none());assert_eq!(tasks[2].status,"abandoned");assert_eq!(tasks[3].depends.as_ref().unwrap(),&vec![tasks[1].id.clone()]);assert_eq!(c.task_ids.as_ref().unwrap().len(),3);assert_eq!(revised.plans.as_ref().unwrap()[0],work.plans.as_ref().unwrap()[0]);assert_eq!(revised.revision,work.revision);
  }
  let (mut work,p,mut spec,limits)=fixture(2);let t=&mut work.tasks.as_mut().unwrap()[0];t.plan_max_attempts=4;t.corrective_recovery=Some(Box::new(CorrectiveRecovery::default()));spec.tasks.as_mut().unwrap()[0].max_attempts=1;let (revised,_)=apply(work,&p,&spec,&limits).unwrap();assert_eq!(revised.tasks.as_ref().unwrap()[0].plan_max_attempts,1);
 }
 #[test]
 fn external_dependents_recheck_but_retired_dependency_refuses_then_recovers(){
  let (mut work,p,spec,limits)=fixture(1);let base=work.tasks.as_ref().unwrap()[1].clone();let mut outside=base.clone();outside.id="outside".into();outside.depends=Some(vec![base.id.clone()]);outside.status="done".into();work.tasks.as_mut().unwrap().push(outside.clone());let (revised,_)=apply(work.clone(),&p,&spec,&limits).unwrap();let external=revised.tasks.as_ref().unwrap().iter().find(|t|t.id=="outside").unwrap();assert_eq!(external.status,"todo");assert_eq!(external.next,outside.next);
  let retired=work.tasks.as_ref().unwrap()[2].id.clone();work.tasks.as_mut().unwrap().last_mut().unwrap().depends=Some(vec![retired]);assert!(matches!(apply(work.clone(),&p,&spec,&limits),Err(E::Refused("preparation-revision-retired-dependency"))));work.tasks.as_mut().unwrap().last_mut().unwrap().depends=Some(vec![base.id]);apply(work,&p,&spec,&limits).unwrap();
 }
 #[test]
 fn planning_preserves_consumption_and_checks_scope_requirements_and_limit(){
  let (mut work,p,mut spec,limits)=fixture(1);planning(&mut work);
  // Planning retirement refuses: valid original witness immediately precedes.
  let mut same=p.conversion.as_ref().unwrap().spec.clone();same.tasks.as_mut().unwrap()[0].title="new title".into();let (valid,_)=apply(work.clone(),&p,&same,&limits).unwrap();assert!(valid.planning.as_ref().unwrap().paused);
  assert!(matches!(apply(work.clone(),&p,&spec,&limits),Err(E::Refused("preparation-revision-retires-requirement"))));
  spec.tasks.as_mut().unwrap().insert(2,p.conversion.as_ref().unwrap().spec.tasks.as_ref().unwrap()[2].clone());let (revised,_)=apply(work.clone(),&p,&spec,&limits).unwrap();let before=work.planning.as_ref().unwrap();let after=revised.planning.as_ref().unwrap();assert_eq!((after.activations,after.decisions,after.max_tasks,after.max_decisions,after.max_activations),(before.activations,before.decisions,before.max_tasks,before.max_decisions,before.max_activations));assert_eq!(after.reviewer,before.reviewer);assert_eq!(after.scopes.as_ref().unwrap()[0].generation,3);assert_eq!(revised.criteria.as_ref().unwrap().len(),4);assert_eq!(after.scopes.as_ref().unwrap()[0].requirements.as_ref().unwrap().len(),4);
  let mut held=work.clone();held.planning.as_mut().unwrap().scopes.as_mut().unwrap()[0].holder="live-owner".into();assert!(matches!(apply(held,&p,&same,&limits),Err(E::Refused("preparation-revision-scope-held"))));
  let mut broken=work.clone();broken.tasks.as_mut().unwrap()[0].requirements=Some(vec!["req-invalid".into()]);assert!(matches!(apply(broken,&p,&same,&limits),Err(E::Refused("preparation-revision-requirement-id-invalid"))));
  let error=revise_prepared_missions(work.clone(),&p,&spec,&RevisionObservation{at:"fixture",event:"unit-limit",operator:"fixture"},limits.task.manual_gate_phases.as_ref().unwrap(),&limits.policy,Some(3)).err().expect("current cap rejects new requirement");assert!(matches!(error,E::Refused("preparation-revision-requirement-limit")));apply(work,&p,&spec,&limits).unwrap();
 }
 #[test]
 fn active_task_and_review_refuse_with_valid_control_and_recovery(){
  let (work,p,spec,limits)=fixture(1);apply(work.clone(),&p,&spec,&limits).unwrap();
  let mut running=work.clone();running.tasks.as_mut().unwrap()[0].status="running".into();assert!(matches!(apply(running,&p,&spec,&limits),Err(E::Refused("preparation-revision-task-running"))));
  let mut review=work.clone();review.tasks.as_mut().unwrap()[0].independent_review=Some(Box::new(IndependentReview{state:"running".into(),..Default::default()}));assert!(matches!(apply(review,&p,&spec,&limits),Err(E::Refused("preparation-revision-task-running"))));apply(work,&p,&spec,&limits).unwrap();
 }

}
