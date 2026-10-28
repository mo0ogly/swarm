//! Dialogue projection, never a process supervisor or provider runner.
use super::{RootStoreError as E,finished,preparation_read::{body,cause}};
use crate::{Preparation,PreparationTurn};
use rusqlite::Connection;

/// Read every retained turn in insertion order inside the caller's read snapshot.
/// The SQL status is authoritative, as in Go. Row identities and known field
/// types must agree; no corrupt row is omitted. Legacy active rows are observations;
/// native active metadata must bind the opened physical Root. Reads perform no
/// inherited15s kill/finish heuristic, state write, release or provider retry.
/// `method` is an observed current hash, None when unavailable. used/stale flags
/// describe current document provenance, not acceptance, agent identity or rights.
/// Private prompt text is dropped and its DTO cannot serialize it. SQL/codec/
/// allocation/finalization failures retain original causes; no truncation/cap.
pub(super) fn read(db:&Connection,p:&Preparation,method:Option<&str>,root:&super::RootInstanceIdentity)->Result<Vec<PreparationTurn>,E>{
    let mut statement=db.prepare("SELECT id,preparation_id,status,body FROM preparation_turns WHERE preparation_id=?1 ORDER BY rowid").map_err(E::Sql)?;
    let result=(||{
        let mut rows=statement.query([&p.id]).map_err(E::Sql)?;
        let mut out=Vec::new();
        while let Some(row)=rows.next().map_err(E::Sql)?{
            let id:String=row.get(0).map_err(E::Sql)?;
            let parent:String=row.get(1).map_err(E::Sql)?;
            let status:String=row.get(2).map_err(E::Sql)?;
            let mut turn:PreparationTurn=crate::work::decode(&body(row,3)?,crate::preparation_turn::shape_preparation_turn()).map_err(|e|cause("preparation-dialogue-document-invalid",e))?;
            if turn.id!=id || turn.preparation_id!=parent || parent!=p.id{return Err(E::Refused("preparation-dialogue-row-identity-mismatch"));}
            turn.status=status;
            if turn.active(){
                super::preparation_call::active_history(db,&id,root)?;
            }
            let document=|kind:&str|p.documents.as_ref().and_then(|d|d.get(kind)).map(|d|d.source_turn.as_str()).unwrap_or("");
            turn.used=document("brief")==turn.id;
            turn.used_in_plan=document("plan")==turn.id;
            turn.stale=turn.revision!=p.revision || method.is_none_or(|hash|hash!=turn.method_hash);
            turn.prompt.clear();
            out.try_reserve(1).map_err(|e|cause("preparation-dialogue-allocation",e))?;
            out.push(turn);
        }
        Ok(out)
    })();
    finished(result,statement.finalize().map_err(E::Sql))
}
