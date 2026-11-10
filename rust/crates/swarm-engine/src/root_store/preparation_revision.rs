//! Atomic revision assembly; caller owns Root custody, transaction and receipt.
use super::{RootStore,RootStoreError as E,preparation_read::cause,preparation_conversion::{Witness,retained_locks},preparation_revision_contract::{revise_prepared_missions,RevisionObservation}};
use crate::{Preparation,PreparationCommand,EngineSettingsDocument,WorkEnvironment};
use rusqlite::{Connection,params};
use sha2::{Digest,Sha256};
use std::collections::BTreeSet;

/// Apply a current validated draft at both observed revisions in the caller's
/// immediate transaction. Preserve current settings, consumption, history and
/// nonselected locks; reject live agents/reviews/decisions and inconsistent old
/// conversion custody. Materialization changes task data, never executes it.
/// Revisions pause cockpit/planning and require a new explicit release. Return
/// a final body/lock/event/pause witness to check AFTER receipt/history writes.
/// Typed IO/codec/SQL/policy/cleanup failures propagate for joint rollback; this
/// function neither commits nor establishes no effect on an uncertain commit.
pub(super) fn revise<W:WorkEnvironment>(root:&RootStore,db:&Connection,p:&mut Preparation,r:&PreparationCommand,settings:&EngineSettingsDocument,at:&str,environment:&mut W,causes:&mut Vec<E>)->Result<Witness,E>{
 if db.is_autocommit(){return Err(E::Refused("preparation-revision-transaction-required"));}
 let conversion=p.conversion.as_ref().ok_or(E::Refused("preparation-stale-plan"))?;
 if p.work_id.is_empty() || conversion.work_id!=p.work_id{return Err(E::Refused("preparation-release-scope-unported"));}
 if r.sha256==conversion.plan_hash{return Err(E::Refused("preparation-already-created"));}
 root.preparation_project(p,&settings.values.preparation_read_policy().map_err(E::Settings)?,causes)?;
 if !p.plan_ready{return Err(E::Refused("preparation-stale-plan"));}
 let plan=p.documents.as_ref().and_then(|d|d.get("plan")).ok_or(E::Refused("preparation-stale-plan"))?;
 if plan.sha256!=r.sha256{return Err(E::Refused("preparation-stale-document"));}
 let edit=settings.values.preparation_write_policy().map_err(E::Settings)?;
 if edit.document_max_bytes.is_some_and(|cap|plan.text.len()>cap){return Err(E::Refused("preparation-request-invalid"));}
 let limits=settings.values.mutation_limits().map_err(E::Settings)?;
 let spec=super::preparation_plan::validate(plan.text.as_bytes(),&p.method,&settings.values.preparation_plan_policy().map_err(E::Settings)?,&limits.task)?;
 let defaults=limits.task.manual_gate_phases.as_ref().ok_or(E::Refused("task-gate-defaults-not-configured"))?;
 let work=crate::read_recorded_work(db,&p.work_id).map_err(E::Work)?;
 if r.expected_work_revision!=Some(work.revision){return Err(E::Refused("preparation-work-revision-conflict"));}
 let active:i64=crate::agent_launch::query(db,"SELECT count(*) FROM agents WHERE work_id=?1 AND status IN ('queued','starting','running','stopping')",[&work.id],|rows|Ok(rows.next()?.ok_or(crate::AgentLaunchError::Refused("preparation-agent-read"))?.get(0)?)).map_err(|e|E::Launch(Box::new(e)))?;
 if active!=0{return Err(E::Refused("preparation-revision-agent-active"));}
 #[cfg(target_os="linux")]
 for task in work.tasks.iter().flatten(){RootStore::require_idle_task_review(db,&work.id,&task.id).map_err(|e|E::Launch(Box::new(e)))?;}
 let conversion=p.conversion.as_ref().ok_or(E::Refused("preparation-stale-plan"))?;
 let old_ids=conversion.task_ids.as_deref().ok_or(E::Refused("preparation-release-scope-unported"))?;
 let unique:BTreeSet<_>=old_ids.iter().collect();
 if old_ids.is_empty() || unique.len()!=old_ids.len(){return Err(E::Refused("preparation-release-scope-unported"));}
 let released=i64::from(!conversion.released_at.is_empty());
 for id in old_ids {
  let mut selected=work.tasks.iter().flatten().filter(|task|&task.id==id);
  let task=selected.next().ok_or(E::Refused("preparation-release-scope-unported"))?;
  if selected.next().is_some() || task.launch_held!=(released==0){return Err(E::Refused("preparation-lock-write-mismatch"));}
  let recorded:(String,i64)=crate::agent_launch::query(db,"SELECT preparation_id,released FROM preparation_launch_locks WHERE work_id=?1 AND task_id=?2",params![work.id,id],|rows|{let row=rows.next()?.ok_or(crate::AgentLaunchError::Refused("preparation-lock-missing"))?;Ok((row.get(0)?,row.get(1)?))}).map_err(|e|E::Launch(Box::new(e)))?;
  if recorded!=(p.id.clone(),released){return Err(E::Refused("preparation-lock-write-mismatch"));}
 }
 let previous=work.revision;
 let actor=if work.planning.is_some(){environment.operator().map_err(E::Runtime)?}else{String::new()};
 let (mut work,conversion)=revise_prepared_missions(work,p,&spec,&RevisionObservation{at,event:&r.event_id,operator:&actor},defaults,&limits.policy,settings.values.planning_requirement_limit().map_err(E::Settings)?)?;
 let ids=conversion.task_ids.clone().ok_or(E::Refused("preparation-materialization-incomplete"))?;
 let retained_locks=retained_locks(db,&work.id,&ids)?;
 work.revision=previous.checked_add(1).filter(|n|*n>0).ok_or(E::Refused("preparation-work-revision-overflow"))?;work.updated=at.into();
 let body=crate::encode_model(&work).map_err(|e|cause("preparation-work-encode",e))?;
 if db.execute("UPDATE works SET revision=?1,body=?2 WHERE id=?3 AND revision=?4",params![work.revision,body,work.id,previous]).map_err(E::Sql)?!=1{return Err(E::Refused("preparation-work-revision-conflict"));}
 for id in &ids {
  if db.execute("INSERT INTO preparation_launch_locks(work_id,task_id,preparation_id,released) VALUES(?1,?2,?3,0) ON CONFLICT(work_id,task_id) DO UPDATE SET released=0 WHERE preparation_launch_locks.preparation_id=excluded.preparation_id",params![work.id,id,p.id]).map_err(E::Sql)?!=1{return Err(E::Refused("preparation-lock-write-mismatch"));}
 }
 if db.execute("INSERT INTO cockpit_controls(work_id,paused) VALUES(?1,1) ON CONFLICT(work_id) DO UPDATE SET paused=1",[&work.id]).map_err(E::Sql)?!=1{return Err(E::Refused("preparation-pause-write-mismatch"));}
 let raw=crate::encode_model(r).map_err(|e|cause("preparation-event-encode",e))?;
 let event=format!("prep-{}",&format!("{:x}",Sha256::digest(format!("{}/{}",p.id,r.event_id).as_bytes()))[..32]);
 if db.execute("INSERT INTO events(id,work_id,revision,kind,at,payload,request) VALUES(?1,?2,?3,'preparation.revise-missions',?4,?5,?5)",params![event,work.id,work.revision,at,raw]).map_err(E::Sql)?!=1{return Err(E::Refused("preparation-event-write-mismatch"));}
 p.conversion=Some(Box::new(conversion));
 Ok(Witness{work_id:work.id,body,ids,event,request:raw,at:at.into(),preparation_id:p.id.clone(),work_revision:work.revision,released:0,event_kind:"preparation.revise-missions",retained_locks,cockpit_paused:Some(true)})
}
