//! Initial explicit CLI team configuration, never release or provider dispatch.
use super::{preparation_conversion::{populate,Witness},preparation_read::cause,RootStoreError as E};
use crate::{Preparation,PreparationCommand,EngineSettingsDocument,Work,WorkEnvironment};
use rusqlite::{Connection,params};
use sha2::{Digest,Sha256};

/// Native operator selection only. Human review/CLI providers are the initial
/// supported scope; no plan/model output can supply an organization implicitly.
#[derive(Default,serde::Deserialize,serde::Serialize)]
#[serde(default)]
pub(super) struct Configuration {
 pub worker_provider:String,pub provider:String,
 #[serde(skip_serializing_if="String::is_empty")] pub level:String,
 #[serde(skip_serializing_if="String::is_empty")] pub model_policy_hash:String,
 pub workspace:String,pub validation:String,
 pub max_tasks:i64,pub max_calls:i64,
}
/// Decode exact fields with the existing Go-shaped wire rules, refusing missing
/// selections/budgets, unknown fields and unsupported mode before any side effect.
/// Positive budgets are protocol preconditions, not implicit execution ceilings.
pub(super) fn configuration(raw:&serde_json::Value)->Result<Configuration,E>{
 let bytes=serde_json::to_vec(raw).map_err(|e|cause("preparation-organization-input",e))?;
 let r:Configuration=crate::work::decode_mode(&bytes,crate::preparation_models::shape_preparation_organization(),true).map_err(|e|cause("preparation-request-invalid",e))?;
 if r.validation!="human" {return Err(E::Refused("preparation-organization-mode-unported"));}
 if !swarm_identity_contract::name(r.provider.as_bytes()) || !swarm_identity_contract::name(r.worker_provider.as_bytes())
  || !matches!(r.level.as_str(),""|"auto"|"simple"|"standard"|"exigeant") || r.workspace.trim().is_empty()
  || r.max_tasks<1 || r.max_calls<1 || (!r.model_policy_hash.is_empty() && !crate::provider_configuration::configuration_digest_valid(&r.model_policy_hash)) {return Err(E::Refused("preparation-request-invalid"));}
 Ok(r)
}

/// Inspect an explicitly selected worker under the real provider reservation.
/// Uses current saved source/model policy and contained workspace rules; releases
/// descriptors with primary and cleanup causes retained. No process/cooldown
/// consumption or future provider identity/availability promise follows.
fn worker(root:&std::path::Path,c:&Configuration,settings:&EngineSettingsDocument)->Result<String,crate::AgentLaunchError>{
 use crate::{AgentLaunchError as A,agent_launch_error::finish,local_storage::LocalDirectory};
 let directory=LocalDirectory::open(&root.join(".swarm")).map_err(|e|A::caused("preparation-team-directory",e))?;
 let result=(||{
  let reservation=directory.reserve_provider_configuration().map_err(|e|A::caused("preparation-team-reservation",e))?;
  let selected=(||{
   let policy=settings.values.agent_launch_policy().map_err(|e|A::caused(e.reason(),e))?;
   let launch=crate::Launch{provider:c.worker_provider.clone(),level:c.level.clone(),role:"worker".into(),..Default::default()};
   let (p,_)=crate::agent_launch::provider(&directory,&launch,&policy)?;
   crate::provider_model_route::resolve_configuration_metadata(&p,&launch,&settings.values,&policy)?;
   let workspace=crate::agent_launch_preflight::workspace(root,&c.workspace)?;
   workspace.to_str().map(str::to_owned).ok_or(A::Refused("preparation-workspace-utf8"))
  })();
  finish(selected,reservation.close().map_err(|e|A::caused("preparation-team-reservation-close",e)))
 })();
 finish(result,directory.close().map_err(|e|A::caused("preparation-team-directory-close",e)))
}

/// Configure one pristine whole-Work conversion in the caller transaction. The
/// exact plan/verdict and task bodies must still match creation; attempts, edits,
/// existing configuration, released/foreign locks and stale CAS refuse. Sources
/// are observed, not leased until launch. Human validation authority records the
/// local engine actor, not authenticated named-human acceptance. Tasks stay held.
/// The returned engine witness must run AFTER receipt insertion, before commit;
/// errors join shared rollback/commit/close handling without inferring no effect.
pub(super) fn configure<W:WorkEnvironment>(root:&super::RootStore,db:&Connection,p:&mut Preparation,r:&PreparationCommand,settings:&EngineSettingsDocument,at:&str,env:&mut W,causes:&mut Vec<E>)->Result<Witness,E>{
 let c=configuration(r.organization.as_ref().ok_or(E::Refused("preparation-request-invalid"))?)?;
 root.preparation_project(p,&settings.values.preparation_read_policy().map_err(E::Settings)?,causes)?;
 let conversion=p.conversion.as_ref().ok_or(E::Refused("preparation-stale-plan"))?;
 if !p.plan_ready || r.sha256!=conversion.plan_hash || p.documents.as_ref().and_then(|d|d.get("plan")).is_none_or(|d|d.sha256!=r.sha256) {return Err(E::Refused("preparation-stale-plan"));}
 if !conversion.released_at.is_empty() || !conversion.organization_sha256.is_empty() {return Err(E::Refused("preparation-organization-already-configured"));}
 if p.work_id!=conversion.work_id {return Err(E::Refused("preparation-organization-target-invalid"));}
 let mut w=crate::read_recorded_work(db,&p.work_id).map_err(E::Work)?;
 if r.expected_work_revision!=Some(w.revision){return Err(E::Refused("preparation-work-revision-conflict"));}
 if w.launch_profile.is_some() || w.planning.is_some() {return Err(E::Refused("preparation-organization-already-configured"));}
 let count:i64=crate::agent_launch::query(db,"SELECT count(*) FROM agents WHERE work_id=?1",[&w.id],|rows|Ok(rows.next()?.ok_or(crate::AgentLaunchError::Refused("preparation-agent-read"))?.get(0)?)).map_err(|e|E::Launch(Box::new(e)))?;
 if count!=0 {return Err(E::Refused("preparation-organization-history-present"));}
 let limits=settings.values.mutation_limits().map_err(E::Settings)?;
 let defaults=limits.task.manual_gate_phases.as_ref().ok_or(E::Refused("task-gate-defaults-not-configured"))?;
 let mut expected=Work{schema_version:1,id:w.id.clone(),tasks:Some(vec![]),..Default::default()};
 let (ids,_)=populate(&mut expected,&p.id,&conversion.spec,defaults)?;
 let brief=p.adopted_brief.as_ref().ok_or(E::Refused("preparation-stale-brief"))?;
 if conversion.task_ids.as_ref()!=Some(&ids) || expected.tasks!=w.tasks || w.objective!=conversion.spec.objective || w.scope!=brief.text
  || w.criteria!=Some(vec!["Chaque mission satisfait ses critères et sa gate delivery avec des preuves fraîches.".into()]) {return Err(E::Refused("preparation-organization-contract-changed"));}
 let retained_locks=super::preparation_conversion::retained_locks(db,&w.id,&ids)?;
 let initial=Witness{work_id:w.id.clone(),body:crate::encode_model(&w).map_err(|e|cause("preparation-work-encode",e))?,ids:ids.clone(),event:format!("prep-{}",&format!("{:x}",Sha256::digest(format!("{}/{}",p.id,r.event_id).as_bytes()))[..32]),request:vec![],at:at.into(),preparation_id:p.id.clone(),work_revision:w.revision,released:0,event_kind:"preparation.configure-team",retained_locks,cockpit_paused:None};
 // No writes yet: lock identity/count must be checked independently of event.
 for id in &ids {
  let held:i64=crate::agent_launch::query(db,"SELECT count(*) FROM preparation_launch_locks WHERE work_id=?1 AND task_id=?2 AND preparation_id=?3 AND released=0",params![w.id,id,p.id],|rows|Ok(rows.next()?.ok_or(crate::AgentLaunchError::Refused("preparation-lock-read"))?.get(0)?)).map_err(|e|E::Launch(Box::new(e)))?;
  if held!=1{return Err(E::Refused("preparation-lock-write-mismatch"));}
 }
 if usize::try_from(c.max_tasks).ok().is_none_or(|n|n<ids.len()){return Err(E::Refused("preparation-organization-task-budget"));}
 if w.tasks.iter().flatten().any(|t|t.plan_role!="worker"){return Err(E::Refused("preparation-organization-worker-role-required"));}
 let canonical=crate::encode_model(&c).map_err(|e|cause("preparation-organization-encode",e))?;
 let prepared=crate::reviewer_bootstrap::prepare(&root.root,&crate::PlanningRequest{provider:c.provider.clone(),level:c.level.clone(),model_policy_hash:c.model_policy_hash.clone(),max_review_calls:c.max_calls,max_activations:c.max_calls,..Default::default()},at,settings).map_err(|e|E::Launch(Box::new(e)))?;
 let workspace=worker(&root.root,&c,settings).map_err(|e|E::Launch(Box::new(e)))?;
 let actor=env.operator().map_err(E::Runtime)?;
 let mut criteria=Vec::new();let mut requirements=Vec::new();
 for task in w.tasks.iter_mut().flatten(){
  if task.plan_role!="worker"{return Err(E::Refused("preparation-organization-worker-role-required"));}
  let mut policy=crate::normalize_validation_policy(crate::ValidationPolicy{mode:"human".into(),..Default::default()},&limits.policy).map_err(|e|cause("preparation-organization-validation",e))?;
  crate::validation_policy_covers_task(&policy,task).map_err(|e|cause("preparation-organization-validation",e))?;
  policy.authorized_at=at.into();policy.actor=actor.clone();task.validation_policy=Some(Box::new(policy));task.scope_id="root".into();let mut task_requirements=Vec::new();
  for criterion in task.criteria.iter().flatten(){criteria.push(criterion.clone());let id=format!("req-{}",criteria.len());requirements.push(id.clone());task_requirements.push(id);}
  task.requirements=Some(task_requirements);
 }
 if settings.values.planning_requirement_limit().map_err(E::Settings)?.is_some_and(|cap|requirements.len()>cap){return Err(E::Refused("planning-enable-requirement-limit"));}
 w.criteria=Some(criteria);w.planning=Some(Box::new(crate::PlanningState{version:1,provider:prepared.provider,provider_digest:prepared.provider_digest,model_route:prepared.route,reviewer:Some(prepared.reviewer),reviewer_required:true,human_review_authorized:at.into(),max_tasks:c.max_tasks,max_decisions:c.max_calls,max_activations:c.max_calls,scopes:Some(vec![crate::PlanningScope{id:"root".into(),objective:w.objective.clone(),revision:1,state:"waiting".into(),requirements:Some(requirements),..Default::default()}]),checks:Some(Default::default()),inbox:Some(vec![]),..Default::default()}));
 w.launch_profile=Some(Box::new(crate::LaunchProfile{provider:c.worker_provider,level:c.level,role:"worker".into(),workspace,updated:at.into(),actor:"operator".into(),..Default::default()}));
 crate::validate_mission_organization(&w).map_err(|e|cause("preparation-organization-invalid",e))?;
 let previous=w.revision;w.revision=previous.checked_add(1).ok_or(E::Refused("preparation-work-revision-overflow"))?;w.updated=at.into();
 let raw=crate::encode_model(r).map_err(|e|cause("preparation-event-encode",e))?;let body=crate::encode_model(&w).map_err(|e|cause("preparation-work-encode",e))?;
 if db.execute("UPDATE works SET revision=?1,body=?2 WHERE id=?3 AND revision=?4",params![w.revision,body,w.id,previous]).map_err(E::Sql)?!=1{return Err(E::Refused("preparation-work-revision-conflict"));}
 if db.execute("INSERT INTO events(id,work_id,revision,kind,at,payload,request) VALUES(?1,?2,?3,'preparation.configure-team',?4,?5,?5)",params![initial.event,w.id,w.revision,at,raw]).map_err(E::Sql)?!=1{return Err(E::Refused("preparation-event-write-mismatch"));}
 p.conversion.as_mut().ok_or(E::Refused("preparation-stale-plan"))?.organization_sha256=format!("{:x}",Sha256::digest(canonical));
 Ok(Witness{body,work_revision:w.revision,request:raw,..initial})
}
