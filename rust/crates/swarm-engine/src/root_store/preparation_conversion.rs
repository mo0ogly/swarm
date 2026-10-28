//! New or existing-work materialization of an explicitly validated preparation plan.
//! The caller owns the immediate transaction, Root binding, receipt and commit.
//! No organization, release or provider is authorized here.
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

/// Rebuild the exact locked task contract from a retained plan using current
/// explicit manual gate defaults. Pure ordered construction; returns task IDs
/// and source mapping, without timestamp, SQL, authority or provider execution.
/// Used by creation and before initial team configuration to refuse task/history
/// drift rather than silently adopting edits or attempts.
pub(super) fn populate(work:&mut Work,preparation_id:&str,spec:&crate::ActionPlan,defaults:&crate::ManualGatePhaseDefaults)->Result<(Vec<String>,BTreeMap<String,String>),E>{
 let prefix=format!("plan-{}-",&format!("{:x}",Sha256::digest(preparation_id.as_bytes()))[..10]);
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
   crate::work_definition::apply_task_addition_with_defaults(work,&crate::Request{schema_version:1,id:id.clone(),title:m.title.clone(),deliverable:m.deliverable.clone(),criteria:m.criteria.clone(),depends:Some(m.depends.iter().flatten().map(|d|format!("{prefix}{d}")).collect()),owner:m.role.clone(),next,..Default::default()},defaults).map_err(|e|cause("preparation-task-invalid",e))?;
   let t=work.tasks.as_mut().and_then(|t|t.last_mut()).ok_or(E::Refused("preparation-materialization-incomplete"))?;
   let mut checks=BTreeMap::from([("plan-entry".into(),"entry".into()),("plan-validation".into(),"validation".into()),("plan-delivery".into(),"delivery".into())]);
   for (i,_) in m.criteria.iter().flatten().enumerate(){checks.insert(format!("plan-criterion-{}",i+1),"validation".into());}
   t.plan_checks=Some(checks);t.plan_role=m.role.clone();t.plan_max_attempts=m.max_attempts;t.plan_tool_limit=m.max_tool_calls;t.launch_held=true;
   done.insert(m.id.clone(),id.clone());ids.push(id);
 }
 Ok((ids,done))
}

/// Materialize an unconverted preparation into a new or explicitly bound Work, in the
/// caller transaction. Current saved plan/task/document/default policies apply.
/// Existing targets require exact Work CAS; append tasks/plans without rewriting
/// prior tasks, configuration, consumption or history. Stable order preserves
/// all dependencies, product/story contracts,
/// gates and source hashes. All newly materialized tasks carry both launch_held and an unreleased
/// SQL lock. Return an engine-owned Witness that the caller must verify after
/// preparation/receipt insertion and before commit; this catches later trigger
/// effects. Errors propagate for joint rollback, never attest commit or no effect. This grants no start or
/// named-human authority; a replay is handled before this function is reached.
pub(super) fn materialize<W: WorkEnvironment>(root:&super::RootStore, db:&Connection,
 p:&mut Preparation,r:&PreparationCommand,settings:&EngineSettingsDocument,
 at:&str,environment:&mut W,causes:&mut Vec<E>)->Result<Witness,E>{
 if p.conversion.is_some(){return Err(E::Refused("preparation-already-created"));}
 let create=p.work_id.is_empty();
 if create && r.expected_work_revision!=Some(0){return Err(E::Refused("preparation-work-revision-conflict"));}
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
 let work_id=if create{format!("w-{}",&format!("{:x}",Sha256::digest(p.id.as_bytes()))[..24])}else{p.work_id.clone()};
 let mut work=if create{Work{schema_version:1,id:work_id.clone(),revision:1,created:at.into(),updated:at.into(),tasks:Some(vec![]),..Default::default()}}else{crate::read_recorded_work(db,&work_id).map_err(E::Work)?};
 let previous=work.revision;
 if !create && r.expected_work_revision!=Some(previous){return Err(E::Refused("preparation-work-revision-conflict"));}
 if create {
 let git=environment.git_with_settings(&root.root,settings).map_err(E::Runtime)?;
 causes.extend(git.issues.into_iter().map(|e|cause("preparation-git-observation",e)));
 crate::apply_work_creation(&mut work,&crate::Request{schema_version:1,title:p.title.clone(),objective:spec.objective.clone(),scope:brief.text.clone(),criteria:Some(vec!["Chaque mission satisfait ses critères et sa gate delivery avec des preuves fraîches.".into()]),..Default::default()},git.state).map_err(|e|cause("preparation-work-invalid",e))?;
 }
 let (ids,done)=populate(&mut work,&p.id,&spec,defaults)?;
 let retained_locks=retained_locks(db,&work_id,&ids)?;
 work.plans.get_or_insert_default().push(crate::ApprovedPlan{source:p.id.clone(),brief_hash:brief.sha256.clone(),response_hash:plan.sha256.clone(),spec:spec.clone(),task_ids:Some(ids.clone()),task_map:Some(done),at:at.into()});
 if !create {work.revision=previous.checked_add(1).filter(|n|*n>0).ok_or(E::Refused("preparation-work-revision-overflow"))?;work.updated=at.into();}
 let body=crate::encode_model(&work).map_err(|e|cause("preparation-work-encode",e))?;
 let written=if create{db.execute("INSERT INTO works(id,revision,body) VALUES(?1,?2,?3)",params![work_id,work.revision,body])}else{db.execute("UPDATE works SET revision=?1,body=?2 WHERE id=?3 AND revision=?4",params![work.revision,body,work_id,previous])}.map_err(E::Sql)?;
 if written!=1{return Err(E::Refused("preparation-materialization-write-mismatch"));}
 for id in &ids {
  if db.execute("INSERT INTO preparation_launch_locks(work_id,task_id,preparation_id,released) VALUES(?1,?2,?3,0)",params![work_id,id,p.id]).map_err(E::Sql)?!=1{return Err(E::Refused("preparation-lock-write-mismatch"));}
 }
 let raw=crate::encode_model(r).map_err(|e|cause("preparation-event-encode",e))?;
 let event=format!("prep-{}",&format!("{:x}",Sha256::digest(format!("{}/{}",p.id,r.event_id).as_bytes()))[..32]);
 if db.execute("INSERT INTO events(id,work_id,revision,kind,at,payload,request) VALUES(?1,?2,?3,'preparation.create-missions',?4,?5,?5)",params![event,work_id,work.revision,at,raw]).map_err(E::Sql)?!=1{return Err(E::Refused("preparation-event-write-mismatch"));}
 p.conversion=Some(Box::new(crate::PreparationConversion{work_id:work_id.clone(),plan_hash:plan.sha256.clone(),brief_hash:brief.sha256.clone(),method_hash:p.method_hash.clone(),task_ids:Some(ids.clone()),spec,created_at:at.into(),..Default::default()}));p.work_id=work_id.clone();Ok(Witness{work_id,body,ids,event,request:raw,at:at.into(),preparation_id:p.id.clone(),work_revision:work.revision,released:0,event_kind:"preparation.create-missions",retained_locks,cockpit_paused:None})
}

/// Snapshot all locks outside the selected task IDs in deterministic order,
/// including retired tasks of the SAME preparation. Final readback compares
/// owner and released state unchanged. SQL/read/finalize failures propagate;
/// this transactional observation grants no release or execution permission.
pub(super) fn retained_locks(db:&Connection,work:&str,scope:&[String])->Result<Vec<(String,String,i64)>,E>{
 let scope:std::collections::BTreeSet<_>=scope.iter().map(String::as_str).collect();
 crate::agent_launch::query(db,"SELECT task_id,preparation_id,released FROM preparation_launch_locks WHERE work_id=?1 ORDER BY task_id",[work],|rows|{
  let mut result=Vec::new();while let Some(row)=rows.next()?{let task:String=row.get(0)?;if !scope.contains(task.as_str()){result.push((task,row.get(1)?,row.get(2)?));}}Ok(result)
 }).map_err(|e|E::Launch(Box::new(e)))
}

/// Engine-owned expected bytes, not caller data or a permission. Verify only
/// after all preparation/history/receipt writes, so their triggers cannot damage
/// materialization or release after an earlier check. The engine selects exact
/// revision, event kind and lock state; SQL/readback errors force rollback.
pub(super) struct Witness {pub(super) work_id:String,pub(super) body:Vec<u8>,pub(super) ids:Vec<String>,pub(super) event:String,pub(super) request:Vec<u8>,pub(super) at:String,pub(super) preparation_id:String,pub(super) work_revision:i64,pub(super) released:i64,pub(super) event_kind:&'static str,pub(super) retained_locks:Vec<(String,String,i64)>,pub(super) cockpit_paused:Option<bool>}
impl Witness {
 /// Read only exact work, event, scoped/retained locks and requested cockpit
 /// pause staged in this transaction.
 /// This is neither a commit witness nor authentication of the datastore.
 pub(super) fn verify(&self,db:&Connection)->Result<(),E>{
 let work_id=self.work_id.clone();let body=self.body.clone();let ids=self.ids.clone();let event=self.event.clone();let raw=self.request.clone();let at=self.at.as_str();let preparation_id=self.preparation_id.clone();
 // Exact persisted bytes/columns witness trigger damage before caller commit.
 let stored:(String,i64,Vec<u8>)=crate::agent_launch::query(db,"SELECT id,revision,CAST(body AS BLOB) FROM works WHERE id=?1",[&work_id],|rows|{let row=rows.next()?.ok_or(crate::AgentLaunchError::Refused("preparation-work-missing"))?;Ok((row.get(0)?,row.get(1)?,row.get(2)?))}).map_err(|e|E::Launch(Box::new(e)))?;
 if stored!=(work_id.clone(),self.work_revision,body){return Err(E::Refused("preparation-materialization-write-mismatch"));}
 if let Some(expected)=self.cockpit_paused {
  let actual:i64=crate::agent_launch::query(db,"SELECT paused FROM cockpit_controls WHERE work_id=?1",[&work_id],|rows|Ok(rows.next()?.ok_or(crate::AgentLaunchError::Refused("preparation-pause-missing"))?.get(0)?)).map_err(|e|E::Launch(Box::new(e)))?;
  if actual!=i64::from(expected){return Err(E::Refused("preparation-pause-write-mismatch"));}
 }
 let count:i64=crate::agent_launch::query(db,"SELECT count(*) FROM preparation_launch_locks WHERE work_id=?1",[&work_id],|rows|Ok(rows.next()?.ok_or(crate::AgentLaunchError::Refused("preparation-lock-missing"))?.get(0)?)).map_err(|e|E::Launch(Box::new(e)))?;
 if usize::try_from(count).ok()!=Some(ids.len()+self.retained_locks.len()){return Err(E::Refused("preparation-lock-write-mismatch"));}
 if retained_locks(db,&work_id,&ids)?!=self.retained_locks{return Err(E::Refused("preparation-lock-write-mismatch"));}
 for id in &ids {
  let lock:(String,i64)=crate::agent_launch::query(db,"SELECT preparation_id,released FROM preparation_launch_locks WHERE work_id=?1 AND task_id=?2",params![work_id,id],|rows|{let row=rows.next()?.ok_or(crate::AgentLaunchError::Refused("preparation-lock-missing"))?;Ok((row.get(0)?,row.get(1)?))}).map_err(|e|E::Launch(Box::new(e)))?;
  if lock!=(preparation_id.clone(),self.released){return Err(E::Refused("preparation-lock-write-mismatch"));}
 }
 let event_back:(String,i64,String,String,Vec<u8>,Vec<u8>)=crate::agent_launch::query(db,"SELECT work_id,revision,kind,at,CAST(payload AS BLOB),CAST(request AS BLOB) FROM events WHERE id=?1",[&event],|rows|{let row=rows.next()?.ok_or(crate::AgentLaunchError::Refused("preparation-event-missing"))?;Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?))}).map_err(|e|E::Launch(Box::new(e)))?;
 if event_back!=(work_id.clone(),self.work_revision,self.event_kind.into(),at.into(),raw.clone(),raw){return Err(E::Refused("preparation-event-write-mismatch"));}
 Ok(())
 }
}

#[cfg(test)]
mod tests {
 use super::*;
 /// Minimal transaction fixture exercises final readback after a receipt's SQL
 /// trigger, including another preparation's retained lock. Public Root opening separately rejects such added schema objects;
 /// this injection is not a public exploitation claim or a production feature.
 #[test]
 fn late_receipt_trigger_rolls_back_and_recovers(){
  let mut faults=0;let mut recoveries=0;
  for phase in [0,1,2,3,4] {
  let released=if phase==1{1}else{0};
  let mut db=Connection::open_in_memory().unwrap();
  db.execute_batch("CREATE TABLE works(id TEXT,revision INTEGER,body BLOB);CREATE TABLE events(id TEXT,work_id TEXT,revision INTEGER,kind TEXT,at TEXT,payload BLOB,request BLOB);CREATE TABLE preparation_launch_locks(work_id TEXT,task_id TEXT,preparation_id TEXT,released INTEGER);CREATE TABLE fixture_receipts(id TEXT);CREATE TABLE cockpit_controls(work_id TEXT,paused INTEGER);").unwrap();
  let witness=Witness{work_id:"w".into(),body:b"body".to_vec(),ids:vec!["t".into()],event:"e".into(),request:b"request".to_vec(),at:"time".into(),preparation_id:"p".into(),work_revision:if phase==0{1}else{2},released,event_kind:match phase{0=>"preparation.create-missions",1=>"preparation.release-plan",4=>"preparation.revise-missions",_=>"preparation.configure-team"},retained_locks:vec![("old".into(),if phase>=3{"p".into()}else{"other".into()},1)],cockpit_paused:if phase==4{Some(true)}else{None}};
  fn stage(db:&Connection,w:&Witness){
   db.execute("INSERT INTO works VALUES('w',?1,?2)",params![w.work_revision,w.body]).unwrap();
   db.execute("INSERT INTO events VALUES('e','w',?1,?2,'time',?3,?3)",params![w.work_revision,w.event_kind,w.request]).unwrap();
   db.execute("INSERT INTO preparation_launch_locks VALUES('w','t','p',?1)",[w.released]).unwrap();
   if let Some(paused)=w.cockpit_paused{db.execute("INSERT INTO cockpit_controls VALUES('w',?1)",[i64::from(paused)]).unwrap();}
   for (task,owner,released) in &w.retained_locks{db.execute("INSERT INTO preparation_launch_locks VALUES('w',?1,?2,?3)",params![task,owner,released]).unwrap();}
  }
  // Valid witness with the same prerequisites precedes every fault variant.
  for (name,sql) in [("pause","UPDATE cockpit_controls SET paused=0;"),("foreign-lock","UPDATE preparation_launch_locks SET released=0 WHERE task_id='old';"),("lock","UPDATE preparation_launch_locks SET released=1-released;"),("work","UPDATE works SET body=CAST('corrupt' AS BLOB);"),("event","UPDATE events SET request=CAST('corrupt' AS BLOB);")]{
   if name=="pause" && witness.cockpit_paused.is_none(){continue;}
   let tx=db.transaction().unwrap();stage(&tx,&witness);witness.verify(&tx).unwrap();
   tx.execute_batch(&format!("CREATE TRIGGER fault AFTER INSERT ON fixture_receipts BEGIN {sql} END;INSERT INTO fixture_receipts VALUES('recorded');")).unwrap();
   let error=witness.verify(&tx).unwrap_err();assert!(matches!(error,E::Refused(_)),"{name}: {error}");tx.rollback().unwrap();faults+=1;
   for table in ["works","events","preparation_launch_locks","fixture_receipts","cockpit_controls"]{let count:i64=db.query_row(&format!("SELECT count(*) FROM {table}"),[],|r|r.get(0)).unwrap();assert_eq!(count,0,"{name}/{table}");}
  }
  let tx=db.transaction().unwrap();stage(&tx,&witness);tx.execute("INSERT INTO fixture_receipts VALUES('recorded')",[]).unwrap();witness.verify(&tx).unwrap();tx.commit().unwrap();witness.verify(&db).unwrap();recoveries+=1;
  }
  assert_eq!((faults,recoveries),(21,5));println!("PREPARATION_REVISION_WITNESS_CASES {faults} {recoveries}");
 }
}
