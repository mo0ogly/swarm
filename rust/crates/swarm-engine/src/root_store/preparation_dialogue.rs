//! Dialogue projection, never a process supervisor or provider runner.
use super::{RootStoreError as E,finished,preparation_read::{body,cause}};
use crate::{Preparation,PreparationTurn};
use rusqlite::Connection;

/// Stable reader-guard refusal codes for the BF-16a dialogue frontier, carried
/// to the HTTP layer's explicit 403 mapping. A bare session alone is never
/// authorization; an absent trust journal cannot hold any policy or identity.
pub(crate) const AUTHENTICATION_REQUIRED: &str = "blp-dialogue-authentication-required";
pub(crate) const POLICY_REQUIRED: &str = "blp-dialogue-policy-required";
pub(crate) const POLICY_STALE: &str = "blp-dialogue-policy-stale";
pub(crate) const ACCESS_DENIED: &str = "blp-dialogue-access-denied";

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

/// Read every retained turn of one preparation for a public reader, inside the
/// caller's already-held read snapshot, under the BF-16a domain-inheritance
/// control. `store` supplies the lock-free reader observation (no reservation
/// beside the held material read); `p` is the already-stored preparation; `db`
/// is the read snapshot. Decision order: (1) the uncontrolled SQL read runs
/// first but its result is only a decision input — nothing is served before
/// the verdict; (2) admit only names with the exact admission receipt grammar
/// and match each receipt's (provider, request_hash) against this
/// preparation's recorded turns, reading receipt bytes fail-closed; (3) zero
/// matched receipts is the legacy path — the read is served exactly as
/// before, without reading any Trust state, so bare sessions keep working on
/// preparations with no admitted classified intent; (4) at least one matched
/// receipt proves admitted classified output for this exact preparation:
/// identity first (bare session → `AUTHENTICATION_REQUIRED`; absent journal
/// → `POLICY_REQUIRED`; unknown/disabled token → typed
/// `TrustError::Authorization`), then policy currency (non-current recorded
/// entry → `POLICY_STALE`), then the full producer-written capsule
/// re-verification per receipt (`preparation_admission::verify`: principal
/// from the record, never supplied; instance, revisions, policy digest,
/// every source re-join, output label reconstruction), then the reader's
/// clearance via `Policy::check_read` against each matched receipt's
/// reconstructed output classification → `ACCESS_DENIED` otherwise. A refusal
/// returns before any turn byte leaves the call: no partial filtering, no
/// flag, no empty success disclosing that classified turns exist. Zero
/// matched receipts while turns exist is NOT evidence of absence — the
/// control keys on matched admissions only, exactly like the worker
/// transport gate. Matched receipt read/decode failures are fail-closed
/// refusals, never silent authorization. This is a presentation frontier
/// only: no dispatch, sink or OS authority follows; the producer-internal
/// dialogue use (`preparation_call::read`) stays deliberately uncontrolled.
pub(super) fn read_controlled(store:&super::RootStore,db:&Connection,p:&Preparation,method:Option<&str>,root:&super::RootInstanceIdentity,reader_token:Option<&str>)->Result<Vec<PreparationTurn>,E>{
    let turns=read(db,p,method,root)?;
    // Admission scan keyed on this preparation's turns (BF-16a frontier):
    // only exact grammar names are candidates; anything else in the
    // directory is not an admission receipt and stays unread.
    let mut matched=Vec::new();
    for name in store.directory.names().map_err(E::Storage)? {
        let hash=std::str::from_utf8(&name)
            .ok()
            .and_then(|n|n.strip_suffix(".receipt"))
            .and_then(|n|n.strip_prefix("blp-admission-"));
        let Some(hash)=hash else {continue};
        if hash.len()!=64 || !hash.bytes().all(|c|c.is_ascii_digit() || (b'a'..=b'f').contains(&c)) {continue;}
        let name=format!("blp-admission-{hash}.receipt");
        let bytes=store.directory.read_native_receipt(&name)
            .map_err(|e|if super::absent(&e){E::Refused("blp-admission-not-found")}else{E::Storage(e)})?;
        let admission:super::preparation_admission::Admission=serde_json::from_slice(&bytes)
            .map_err(|e|cause("blp-admission-record-invalid",e))?;
        if admission.captured.preparation_id!=p.id {continue;}
        // The receipt must describe an intent actually recorded for THIS
        // preparation: provider and per-event request hash from the turn rows.
        if turns.iter().any(|t|t.provider==admission.captured.provider && t.request_hash==admission.captured.request_hash) {
            matched.push(admission);
        }
    }
    if matched.is_empty() {
        // Legacy path: no admitted classified intent matches this dialogue.
        return Ok(turns);
    }
    // Classified dialogue path: decide the reader BEFORE returning any byte.
    let (state,actor)=match store.reader_identity_state(reader_token)? {
        super::reader_guard::ReaderIdentityState::BareSession=>return Err(E::Refused(AUTHENTICATION_REQUIRED)),
        super::reader_guard::ReaderIdentityState::AbsentJournal=>return Err(E::Refused(POLICY_REQUIRED)),
        super::reader_guard::ReaderIdentityState::Current{state,actor}=>(state,actor),
    };
    let entry=state.confidentiality.as_ref().ok_or(E::Refused(POLICY_REQUIRED))?
        .current().map_err(|e|E::Trust(Box::new(e)))?;
    if entry.trust_revision()!=state.revision {return Err(E::Refused(POLICY_STALE));}
    for admission in &matched {
        super::preparation_admission::verify(store,&state,&admission.principal,admission)?;
        entry.policy().check_read(&actor.subject,Some(&admission.classification))
            .map_err(|_|E::Refused(ACCESS_DENIED))?;
    }
    Ok(turns)
}
