//! New-work materialization of an explicitly validated preparation plan.
//! The caller owns the immediate transaction, Root binding, receipt and commit.
//! No organization, existing-work update, release or provider is authorized here.
use super::{preparation_read::cause, RootStoreError as E};
use crate::{Preparation, PreparationCommand, EngineSettingsDocument, Work, WorkEnvironment};
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Go-compatible depth-first postorder: visit each authored task, its ordered
/// dependencies, then the task. An explicit stack avoids an implicit recursion
/// depth policy. Unknown/duplicate IDs or cycles refuse without mutation; valid
/// inputs preserve parallel-branch order in task IDs and the immutable receipt.
fn ordered(tasks:&[crate::PlanMission])->Result<Vec<&crate::PlanMission>,E>{
 let by:BTreeMap<_,_>=tasks.iter().map(|t|(t.id.as_str(),t)).collect();
 if by.len()!=tasks.len(){return Err(E::Refused("preparation-materialization-duplicate"));}
 let mut state=BTreeMap::new();let mut result=Vec::new();
 for task in tasks {
  let mut stack=vec![(task.id.as_str(),false)];
  while let Some((id,exit))=stack.pop(){
   let task=by.get(id).ok_or(E::Refused("preparation-materialization-dependency"))?;
   if exit {state.insert(id,2);result.push(*task);continue;}
   match state.get(id){Some(2)=>continue,Some(1)=>return Err(E::Refused("preparation-materialization-cycle")),_=>{}}
   state.insert(id,1);stack.push((id,true));
   for dep in task.depends.iter().flatten().rev(){stack.push((dep.as_str(),false));}
  }
 }
 Ok(result)
}

/// Materialize only a fresh standalone preparation into a new Work, in the
/// caller transaction. Current saved plan/task/document/default policies apply.
/// Stable topological order preserves all dependencies, product/story contracts,
/// gates and source hashes. All tasks carry both launch_held and an unreleased
/// SQL lock. Return an engine-owned Witness that the caller must verify after
/// preparation/receipt insertion and before commit; this catches later trigger
/// effects. Errors propagate for joint rollback, never attest commit or no effect. This grants no start or
/// named-human authority; a replay is handled before this function is reached.
pub(super) fn materialize<W: WorkEnvironment>(root:&super::RootStore, db:&Connection,
 p:&mut Preparation,r:&PreparationCommand,settings:&EngineSettingsDocument,
 at:&str,environment:&mut W,causes:&mut Vec<E>)->Result<Witness,E>{
 if p.conversion.is_some(){return Err(E::Refused("preparation-already-created"));}
 if !p.work_id.is_empty(){return Err(E::Refused("preparation-existing-work-unported"));}
 let read=settings.values.preparation_read_policy().map_err(E::Settings)?;
 root.preparation_project(p,&read,causes)?;
 if !p.plan_ready{return Err(E::Refused("preparation-stale-plan"));}
 let plan=p.documents.as_ref().and_then(|d|d.get("plan")).ok_or(E::Refused("preparation-stale-plan"))?;
 if r.sha256!=plan.sha256{return Err(E::Refused("preparation-stale-document"));}
 let edit=settings.values.preparation_write_policy().map_err(E::Settings)?;
 if edit.document_max_bytes.is_some_and(|n|plan.text.len()>n) || edit.title_max_bytes.is_some_and(|n|p.title.len()>n){return Err(E::Refused("preparation-request-invalid"));}
 let limits=settings.values.mutation_limits().map_err(E::Settings)?;
 let spec=super::preparation_plan::validate(plan.text.as_bytes(),&p.method,&settings.values.preparation_plan_policy().map_err(E::Settings)?,&limits.task)?;
 let defaults=limits.task.manual_gate_phases.as_ref().ok_or(E::Refused("task-gate-defaults-not-configured"))?;
 let brief=p.adopted_brief.as_ref().ok_or(E::Refused("preparation-stale-brief"))?;
 let work_id=format!("w-{}",&format!("{:x}",Sha256::digest(p.id.as_bytes()))[..24]);
 let mut work=Work{schema_version:1,id:work_id.clone(),revision:1,created:at.into(),updated:at.into(),tasks:Some(vec![]),..Default::default()};
 let git=environment.git_with_settings(&root.root,settings).map_err(E::Runtime)?;
 causes.extend(git.issues.into_iter().map(|e|cause("preparation-git-observation",e)));
 crate::apply_work_creation(&mut work,&crate::Request{schema_version:1,title:p.title.clone(),objective:spec.objective.clone(),scope:brief.text.clone(),criteria:Some(vec!["Chaque mission satisfait ses critères et sa gate delivery avec des preuves fraîches.".into()]),..Default::default()},git.state).map_err(|e|cause("preparation-work-invalid",e))?;
 let prefix=format!("plan-{}-",&format!("{:x}",Sha256::digest(p.id.as_bytes()))[..10]);
 let tasks=spec.tasks.as_deref().ok_or(E::Refused("preparation-invalid-plan"))?;
 let mut done=BTreeMap::new();let mut ids=Vec::new();
 for m in ordered(tasks)? {
   let id=format!("{prefix}{}",m.id);
   let mut next=format!("Périmètre : {}\nPreuves : {}\nGate entry : {}\nGate validation : {}\nGate delivery : {}\nConditions d’arrêt : {}\nOODA : consigner observation, orientation, décision et résultat sur blocage.\nObjectif du plan : {}\nHypothèses : {}",m.scope,m.proof,m.entry,m.validation,m.delivery,m.stop,spec.objective,spec.assumptions.as_deref().unwrap_or_default().join(" ; "));
   if !m.phase.is_empty(){next.push_str(&format!("\nÉtape produit : {}",m.phase));}
   for story in spec.product.iter().flat_map(|p|p.stories.iter().flatten()).filter(|s|s.task_ids.iter().flatten().any(|t|t==&m.id)){
    next.push_str(&format!("\nStory {} — {} ; utilisateur : {} ; valeur : {} ; critères : {} ; dépendances stories : {}",story.id,story.title,story.user,story.value,story.criteria.as_deref().unwrap_or_default().join(" ; "),story.depends.as_deref().unwrap_or_default().join(", ")));
   }
   for q in spec.questions.iter().flatten(){next.push_str(&format!("\nDécision : {} → {}",q.question,q.answer));}
   next.push_str("\nCadre de gate obligatoire : checks plan-entry (entry), plan-validation (validation), plan-delivery (delivery) et plan-criterion-N pour chaque critère (validation). Tous mandatory=true, avec résultats et preuves réelles. Aucun PASS avant exécution.\n");
   crate::work_definition::apply_task_addition_with_defaults(&mut work,&crate::Request{schema_version:1,id:id.clone(),title:m.title.clone(),deliverable:m.deliverable.clone(),criteria:m.criteria.clone(),depends:Some(m.depends.iter().flatten().map(|d|format!("{prefix}{d}")).collect()),owner:m.role.clone(),next,..Default::default()},defaults).map_err(|e|cause("preparation-task-invalid",e))?;
   let t=work.tasks.as_mut().and_then(|t|t.last_mut()).ok_or(E::Refused("preparation-materialization-incomplete"))?;
   let mut checks=BTreeMap::from([("plan-entry".into(),"entry".into()),("plan-validation".into(),"validation".into()),("plan-delivery".into(),"delivery".into())]);
   for (i,_) in m.criteria.iter().flatten().enumerate(){checks.insert(format!("plan-criterion-{}",i+1),"validation".into());}
   t.plan_checks=Some(checks);t.plan_role=m.role.clone();t.plan_max_attempts=m.max_attempts;t.plan_tool_limit=m.max_tool_calls;t.launch_held=true;
   done.insert(m.id.clone(),id.clone());ids.push(id);
 }
 work.plans=Some(vec![crate::ApprovedPlan{source:p.id.clone(),brief_hash:brief.sha256.clone(),response_hash:plan.sha256.clone(),spec:spec.clone(),task_ids:Some(ids.clone()),task_map:Some(done),at:at.into()}]);
 let body=crate::encode_model(&work).map_err(|e|cause("preparation-work-encode",e))?;
 if db.execute("INSERT INTO works(id,revision,body) VALUES(?1,1,?2)",params![work_id,body]).map_err(E::Sql)?!=1{return Err(E::Refused("preparation-materialization-write-mismatch"));}
 for id in &ids {
  if db.execute("INSERT INTO preparation_launch_locks(work_id,task_id,preparation_id,released) VALUES(?1,?2,?3,0)",params![work_id,id,p.id]).map_err(E::Sql)?!=1{return Err(E::Refused("preparation-lock-write-mismatch"));}
 }
 let raw=crate::encode_model(r).map_err(|e|cause("preparation-event-encode",e))?;
 let event=format!("prep-{}",&format!("{:x}",Sha256::digest(format!("{}/{}",p.id,r.event_id).as_bytes()))[..32]);
 if db.execute("INSERT INTO events(id,work_id,revision,kind,at,payload,request) VALUES(?1,?2,1,'preparation.create-missions',?3,?4,?4)",params![event,work_id,at,raw]).map_err(E::Sql)?!=1{return Err(E::Refused("preparation-event-write-mismatch"));}
 p.conversion=Some(Box::new(crate::PreparationConversion{work_id:work_id.clone(),plan_hash:plan.sha256.clone(),brief_hash:brief.sha256.clone(),method_hash:p.method_hash.clone(),task_ids:Some(ids.clone()),spec,created_at:at.into(),..Default::default()}));p.work_id=work_id.clone();Ok(Witness{work_id,body,ids,event,request:raw,at:at.into(),preparation_id:p.id.clone()})
}

/// Engine-owned expected bytes, not caller data or a permission. Verify only
/// after all preparation/history/receipt writes, so their triggers cannot damage
/// materialization after an earlier check. SQL/readback errors force rollback.
pub(super) struct Witness {work_id:String,body:Vec<u8>,ids:Vec<String>,event:String,request:Vec<u8>,at:String,preparation_id:String}
impl Witness {
 /// Read only exact work, event and lock values staged in this transaction.
 /// This is neither a commit witness nor authentication of the datastore.
 pub(super) fn verify(&self,db:&Connection)->Result<(),E>{
 let work_id=self.work_id.clone();let body=self.body.clone();let ids=self.ids.clone();let event=self.event.clone();let raw=self.request.clone();let at=self.at.as_str();let preparation_id=self.preparation_id.clone();
 // Exact persisted bytes/columns witness trigger damage before caller commit.
 let stored:(String,i64,Vec<u8>)=crate::agent_launch::query(db,"SELECT id,revision,CAST(body AS BLOB) FROM works WHERE id=?1",[&work_id],|rows|{let row=rows.next()?.ok_or(crate::AgentLaunchError::Refused("preparation-work-missing"))?;Ok((row.get(0)?,row.get(1)?,row.get(2)?))}).map_err(|e|E::Launch(Box::new(e)))?;
 if stored!=(work_id.clone(),1,body){return Err(E::Refused("preparation-materialization-write-mismatch"));}
 let count:i64=crate::agent_launch::query(db,"SELECT count(*) FROM preparation_launch_locks WHERE work_id=?1",[&work_id],|rows|Ok(rows.next()?.ok_or(crate::AgentLaunchError::Refused("preparation-lock-missing"))?.get(0)?)).map_err(|e|E::Launch(Box::new(e)))?;
 if usize::try_from(count).ok()!=Some(ids.len()){return Err(E::Refused("preparation-lock-write-mismatch"));}
 for id in &ids {
  let lock:(String,i64)=crate::agent_launch::query(db,"SELECT preparation_id,released FROM preparation_launch_locks WHERE work_id=?1 AND task_id=?2",params![work_id,id],|rows|{let row=rows.next()?.ok_or(crate::AgentLaunchError::Refused("preparation-lock-missing"))?;Ok((row.get(0)?,row.get(1)?))}).map_err(|e|E::Launch(Box::new(e)))?;
  if lock!=(preparation_id.clone(),0){return Err(E::Refused("preparation-lock-write-mismatch"));}
 }
 let event_back:(String,i64,String,String,Vec<u8>,Vec<u8>)=crate::agent_launch::query(db,"SELECT work_id,revision,kind,at,CAST(payload AS BLOB),CAST(request AS BLOB) FROM events WHERE id=?1",[&event],|rows|{let row=rows.next()?.ok_or(crate::AgentLaunchError::Refused("preparation-event-missing"))?;Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?))}).map_err(|e|E::Launch(Box::new(e)))?;
 if event_back!=(work_id.clone(),1,"preparation.create-missions".into(),at.into(),raw.clone(),raw){return Err(E::Refused("preparation-event-write-mismatch"));}
 Ok(())
 }
}

#[cfg(test)]
mod tests {
 use super::*;
 /// Minimal transaction fixture exercises final readback after a receipt's SQL
 /// trigger. Public Root opening separately rejects such added schema objects;
 /// this injection is not a public exploitation claim or a production feature.
 #[test]
 fn late_receipt_trigger_rolls_back_and_recovers(){
  let mut db=Connection::open_in_memory().unwrap();
  db.execute_batch("CREATE TABLE works(id TEXT,revision INTEGER,body BLOB);CREATE TABLE events(id TEXT,work_id TEXT,revision INTEGER,kind TEXT,at TEXT,payload BLOB,request BLOB);CREATE TABLE preparation_launch_locks(work_id TEXT,task_id TEXT,preparation_id TEXT,released INTEGER);CREATE TABLE fixture_receipts(id TEXT);").unwrap();
  let witness=Witness{work_id:"w".into(),body:b"body".to_vec(),ids:vec!["t".into()],event:"e".into(),request:b"request".to_vec(),at:"time".into(),preparation_id:"p".into()};
  fn stage(db:&Connection){db.execute_batch("INSERT INTO works VALUES('w',1,CAST('body' AS BLOB));INSERT INTO events VALUES('e','w',1,'preparation.create-missions','time',CAST('request' AS BLOB),CAST('request' AS BLOB));INSERT INTO preparation_launch_locks VALUES('w','t','p',0);").unwrap();}
  // Valid witness with the same prerequisites precedes every fault variant.
  for (name,sql) in [("lock","UPDATE preparation_launch_locks SET released=1;"),("work","UPDATE works SET body=CAST('corrupt' AS BLOB);"),("event","UPDATE events SET request=CAST('corrupt' AS BLOB);")]{
   let tx=db.transaction().unwrap();stage(&tx);witness.verify(&tx).unwrap();
   tx.execute_batch(&format!("CREATE TRIGGER fault AFTER INSERT ON fixture_receipts BEGIN {sql} END;INSERT INTO fixture_receipts VALUES('recorded');")).unwrap();
   let error=witness.verify(&tx).unwrap_err();assert!(matches!(error,E::Refused(_)),"{name}: {error}");tx.rollback().unwrap();
   for table in ["works","events","preparation_launch_locks","fixture_receipts"]{let count:i64=db.query_row(&format!("SELECT count(*) FROM {table}"),[],|r|r.get(0)).unwrap();assert_eq!(count,0,"{name}/{table}");}
  }
  let tx=db.transaction().unwrap();stage(&tx);tx.execute("INSERT INTO fixture_receipts VALUES('recorded')",[]).unwrap();witness.verify(&tx).unwrap();tx.commit().unwrap();witness.verify(&db).unwrap();
 }
}
