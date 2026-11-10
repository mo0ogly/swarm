//! Explicit release of a converted whole-Work snapshot; never provider dispatch.
use super::{preparation_conversion::Witness, preparation_read::cause, RootStoreError as E};
use crate::{Preparation, PreparationCommand};
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// Release exactly the converted task IDs at the requested current Work revision.
/// The caller owns the immediate transaction, settings reservation and receipt.
/// Draft edits do not replace the conversion snapshot. Public task edits are
/// retained (Go semantics); reviewers must inspect the current Work, not infer
/// unchanged instructions from the snapshot hash. Conversion IDs must be a
/// nonempty unique subset of current task IDs; unrelated tasks and locks remain
/// unchanged, with the latter included in final transactional readback.
/// Conditional lock/Work updates and the returned final readback witness ensure
/// joint rollback on failure. Release may unblock a configured conductor but
/// changes neither autonomy nor budget and makes no synchronous provider call.
pub(super) fn release(db: &Connection, p: &mut Preparation, r: &PreparationCommand, at: &str) -> Result<Witness,E> {
    let c=p.conversion.as_mut().ok_or(E::Refused("preparation-stale-plan"))?;
    if r.sha256!=c.plan_hash {return Err(E::Refused("preparation-stale-plan"));}
    if !c.released_at.is_empty() {return Err(E::Refused("preparation-already-released"));}
    if p.work_id.is_empty() || p.work_id!=c.work_id {return Err(E::Refused("preparation-release-scope-unported"));}
    let mut w=crate::read_recorded_work(db,&p.work_id).map_err(E::Work)?;
    if r.expected_work_revision!=Some(w.revision) {return Err(E::Refused("preparation-work-revision-conflict"));}
    let ids=c.task_ids.clone().ok_or(E::Refused("preparation-release-scope-unported"))?;
    let expected:BTreeSet<_>=ids.iter().collect();
    let tasks=w.tasks.as_mut().ok_or(E::Refused("preparation-release-scope-unported"))?;
    if ids.is_empty() || expected.len()!=ids.len()
        || tasks.iter().map(|t|&t.id).collect::<BTreeSet<_>>().len()!=tasks.len()
        || !expected.is_subset(&tasks.iter().map(|t|&t.id).collect::<BTreeSet<_>>()) {
        return Err(E::Refused("preparation-release-scope-unported"));
    }
    let retained_locks=super::preparation_conversion::retained_locks(db,&w.id,&ids)?;
    let previous=w.revision;
    w.revision=previous.checked_add(1).filter(|n|*n>0).ok_or(E::Refused("preparation-work-revision-overflow"))?;
    for task in tasks.iter_mut().filter(|task|expected.contains(&task.id)) {
        if !task.launch_held {return Err(E::Refused("preparation-lock-write-mismatch"));}
        if db.execute("UPDATE preparation_launch_locks SET released=1 WHERE work_id=?1 AND task_id=?2 AND preparation_id=?3 AND released=0",params![w.id,task.id,p.id]).map_err(E::Sql)?!=1 {
            return Err(E::Refused("preparation-lock-write-mismatch"));
        }
        task.launch_held=false;
    }
    if let Some(planning)=w.planning.as_mut(){planning.paused=false;}
    w.updated=at.into();
    let body=crate::encode_model(&w).map_err(|e|cause("preparation-work-encode",e))?;
    if db.execute("UPDATE works SET revision=?1,body=?2 WHERE id=?3 AND revision=?4",params![w.revision,body,w.id,previous]).map_err(E::Sql)?!=1 {return Err(E::Refused("preparation-work-revision-conflict"));}
    let raw=crate::encode_model(r).map_err(|e|cause("preparation-event-encode",e))?;
    let event=format!("prep-{}",&format!("{:x}",Sha256::digest(format!("{}/{}",p.id,r.event_id).as_bytes()))[..32]);
    if db.execute("INSERT INTO events(id,work_id,revision,kind,at,payload,request) VALUES(?1,?2,?3,'preparation.release-plan',?4,?5,?5)",params![event,w.id,w.revision,at,raw]).map_err(E::Sql)?!=1 {return Err(E::Refused("preparation-event-write-mismatch"));}
    c.released_at=at.into();
    Ok(Witness{work_id:w.id,body,ids,event,request:raw,at:at.into(),preparation_id:p.id.clone(),work_revision:w.revision,released:1,event_kind:"preparation.release-plan",retained_locks,cockpit_paused:None})
}
