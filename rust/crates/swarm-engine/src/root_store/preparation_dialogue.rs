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
/// entry → `POLICY_STALE`), then the three-pass recovery: per matched receipt
/// the full
/// producer-written capsule re-verification (`preparation_admission::verify`
/// in its original order: principal from the record, never supplied; record
/// validity, Root instance BEFORE policy currency, every source re-join,
/// output label reconstruction) whose completed typed result is classified
/// once by `classify_verify_result` — `Ok` marks a fully verified CURRENT
/// authority, the exact `Refused("blp-admission-policy-changed")` variant
/// marks an identity-valid but stale receipt, and ANY other error variant is
/// returned unchanged (record invalid, Root changed, source re-join, storage,
/// close wrappers), refusing the whole read even if a current equivalent
/// exists; no pre-validation duplicates verify and no refusal is suppressed.
/// A stale receipt no longer refuses by itself: it is covered only when one
/// fully verified CURRENT admission of the SAME principal, SAME opened Root
/// instance and STRICTLY same recorded output classification carries a
/// byte-identical captured lineage (`equivalent_current_covers`: every
/// captured field plus serialized provenance; a provenance serialization
/// error is not treated as non-coverage — it propagates as its typed Trust
/// cause and refuses the whole read with the original diagnostic;
/// admission_id/event_id/request_digest may legitimately differ on renewal;
/// trust/policy revisions are the staleness itself); any uncovered staleness
/// keeps the declared reader refusal via the exact-typed adapter — no global
/// stale-ignore, no choosing any recent receipt, no mission/date/event-key
/// exception; a current admission from a different principal, capture or
/// label never covers an old one, and a higher-label renewal does not cover
/// a lower-labeled stale admission (strict-equality limitation). Then the
/// reader's clearance via `Policy::check_read` against each matched receipt's
/// recorded output classification — current and covered-stale alike — →
/// `ACCESS_DENIED` otherwise. A refusal returns before any turn byte leaves
/// the call: no partial filtering, no
/// flag, no empty success disclosing that classified turns exist. Zero
/// matched receipts while turns exist is NOT evidence of absence — the
/// control keys on matched admissions only, exactly like the worker
/// transport gate. Matched receipt read/decode failures are fail-closed
/// refusals, never silent authorization. This is a presentation frontier
/// only: no dispatch, sink or OS authority follows; the producer-internal
/// dialogue use (`preparation_call::read`) stays deliberately uncontrolled.
pub(super) fn read_controlled(store:&super::RootStore,db:&Connection,p:&Preparation,method:Option<&str>,root:&super::RootInstanceIdentity,reader_token:Option<&str>)->Result<Vec<PreparationTurn>,E>{
    let turns=read(db,p,method,root)?;
    reader_verdict(store,db,p,&turns,reader_token)?;
    Ok(turns)
}

/// Reader verdict shared by every BF-16 presentation frontier (dialogue, show,
/// history). Same scan-then-decide contract as `read_controlled`'s docstring:
/// the caller must have read its material already but serve NOTHING before this
/// verdict returns `Ok(())`. The scan keys on the turn rows passed in (the
/// dialogue frontier passes its read turns; show/history pass their own
/// recorded turn rows read under the same snapshot) so a receipt matches only
/// an intent actually recorded for THIS preparation. All refusal codes,
/// three-pass recovery and limits are exactly the dialogue frontier's.
pub(super) fn reader_verdict(store:&super::RootStore,db:&Connection,p:&Preparation,turns:&[PreparationTurn],reader_token:Option<&str>)->Result<(),E>{
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
        // Legacy path: no admitted classified intent matches this frontier's
        // recorded turns; the caller serves its already-read material as before.
        return Ok(());
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
    // Three-pass recovery. Pass 1 fully re-verifies EVERY matched receipt via
    // `preparation_admission::verify` in its original order (principal, record
    // validity, Root instance BEFORE policy currency, source re-join, output
    // label) and classifies the completed typed result once — no duplicated
    // pre-validation: any non-staleness refusal (corruption, wrong Root,
    // invalid record, storage, close wrapper) propagates unchanged and refuses
    // the whole read even if a current equivalent exists.
    let mut currents=Vec::new();
    let mut stale=Vec::new();
    for admission in &matched {
        match classify_verify_result(
            super::preparation_admission::verify(store,&state,&admission.principal,admission))?
        {
            true=>currents.push(admission),
            false=>stale.push(admission),
        }
    }
    // Pass 2: staleness alone is no longer refusal — an ordinary-stale receipt
    // is covered only by one fully verified CURRENT equivalent admission (same
    // principal, same Root instance, strictly same output classification,
    // byte-identical captured lineage including provenance); any uncovered
    // staleness keeps the declared reader refusal via the exact-typed adapter.
    // No mission/date/event-key exception, no global stale-ignore, no choice of
    // any recent receipt. The equivalence check returns `Result`: an actual
    // provenance serialization error propagates its typed cause unchanged
    // (`?`) — it is never mapped to non-coverage (which would mask the real
    // cause behind a stale 403) and never to coverage (which would serve
    // without a decision). Explicit checked loops rather than `.any` closures
    // because a closure cannot early-return the enclosing function's error;
    // no hidden retry, no cap, first coverage decision per stale receipt.
    for stale_admission in &stale {
        let mut covered=false;
        for current_admission in &currents {
            if super::preparation_admission::equivalent_current_covers(stale_admission,current_admission)? {
                covered=true;
                break;
            }
        }
        if !covered {
            return Err(reader_stale(E::Refused(super::preparation_admission::POLICY_CHANGED)));
        }
    }
    // Pass 3: every matched admission, current or covered-stale, still passes
    // the reader's clearance against its recorded output classification before
    // any byte is served: no partial filtered output.
    for admission in &matched {
        entry.policy().check_read(&actor.subject,Some(&admission.classification))
            .map_err(|_|E::Refused(ACCESS_DENIED))?;
    }
    Ok(())
}

/// Classify one completed `preparation_admission::verify` typed result for the
/// dialogue reader's three-pass recovery. `Ok(_)` means the receipt is a fully
/// verified CURRENT authority; the exact `Refused(POLICY_CHANGED)` variant —
/// verify's ordinary staleness block, reached only AFTER record validity and
/// Root-instance validation — means the receipt's identity is valid but its
/// currency is stale; ANY other error propagates unchanged (wrong Root,
/// invalid record, source re-join, storage, close wrappers): no error is
/// suppressed and no success is synthesized. Typed enum-variant matching, not
/// string parsing.
fn classify_verify_result(result:Result<usize,E>)->Result<bool,E>{
    match result{
        Ok(_)=>Ok(true),
        Err(E::Refused(super::preparation_admission::POLICY_CHANGED))=>Ok(false),
        Err(other)=>Err(other),
    }
}

/// Typed result adaptation for staleness left UNCOVERED after the three-pass
/// recovery in `read_controlled`: an ordinary-stale receipt covered by one
/// fully verified current equivalent admission (same principal, same opened
/// Root instance, strictly same recorded output classification, byte-identical
/// captured lineage including serialized provenance) is accepted instead, so
/// the supported public renewal recovery works; only staleness with NO current
/// equivalent reaches this adapter. ONLY the exact stable
/// `RootStoreError::Refused(super::preparation_admission::POLICY_CHANGED)` —
/// verify's ordinary staleness block: recorded trust revision, policy
/// revision or policy digest no longer current — is converted to the declared
/// reader code `POLICY_STALE`, so the internal admission code never reaches
/// the HTTP layer. Every other variant is returned unchanged: neighbor
/// refusals (`blp-admission-not-found`, `blp-admission-record-invalid`,
/// `preparation-root-instance-changed`) keep their typed codes, source
/// re-join `Trust` errors, `Storage`/`Sql` failures and `TrustPublished`
/// close wrappers retain their original cause. By the time this adapter sees
/// a result, verify's full validation order has already run, so staleness
/// cannot mask corruption, a wrong Root or an invalid record combined with
/// it; conversely no refusal is suppressed, no success is synthesized and no
/// free-form error wording is parsed — matching the stable enum-variant
/// static code is typed contract classification, not string parsing.
/// Production-used: the uncovered-staleness return in `read_controlled`
/// routes through this adapter so the internal admission code never crosses
/// the reader guard, exactly as before the recovery.
fn reader_stale(result:E)->E{
    match result{
        E::Refused(super::preparation_admission::POLICY_CHANGED)=>E::Refused(POLICY_STALE),
        other=>other,
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    // Nested module: `super` here is `preparation_dialogue`, not `root_store`,
    // so the sibling admission module is imported by its real crate path
    // (E0433 namespace fix; no visibility widened, production path untouched).
    use crate::root_store::preparation_admission;
    use crate::root_store::preparation_prompt::assemble;
    use swarm_confidentiality::Classification;

    /// One producer-shaped admission record for the pure recovery tests, built
    /// through the real `assemble` producer so the captured provenance is
    /// valid over its prompt. `classification` and `captured` fields are the
    /// same fixture values across records unless a test deliberately changes
    /// one to prove non-coverage; no IO, no store, no authorization.
    fn fixture_admission(
        principal: &str,
        classification: Classification,
        captured: super::super::preparation_call::AdmissionCapture,
    ) -> preparation_admission::Admission {
        // Rust format strings do not support inline field access (compiler
        // exit 101 observed at this exact line, raw unit.stderr retained):
        // the intended ID value is bound to a local first, producing the
        // same fixture bytes.
        let source_revision=captured.source_revision;
        preparation_admission::Admission{
            schema_version:1,
            admission_id:format!("ba-{principal}-{source_revision}"),
            event_id:"fixture-event".into(),
            principal:principal.into(),
            root:super::super::RootInstanceIdentity{directory:(1,2),database:(3,4)},
            trust_revision:1,policy_revision:1,
            policy_digest:"fixture-policy-digest".into(),
            request_digest:"0".repeat(64),
            classification,captured,
        }
    }

    /// One captured lineage through the real producer: same values as the
    /// renewal flow would recapture, byte-identical across two calls because
    /// `assemble` is deterministic over identical inputs.
    fn fixture_capture() -> super::super::preparation_call::AdmissionCapture {
        let data=serde_json::json!({"documents":{"besoin":"échéance"},"sources":[{"text":"source"}],
            "adopted_brief":null,"method":"apex","method_sources":["instructions"],
            "history":[{"question":"Q","answer":"A"}],"history_mode":"full",
            "earlier_exchanges_not_sent":0,"user_message":"courant"});
        let (prompt,provenance)=assemble(None,"instructions\n",None,&data).unwrap();
        super::super::preparation_call::AdmissionCapture{
            preparation_id:"prep-1".into(),source_revision:1,
            provider:"fixture-provider".into(),provider_source:"fixture-source".into(),
            request_level:"native".into(),target:"fixture-target".into(),
            context_mode:"fixture-mode".into(),request_hash:"fixture-request-hash".into(),
            settings_digest:"fixture-settings-digest".into(),prompt,provenance,
        }
    }
    /// Targeted unit for the recovery cover relation
    /// `equivalent_current_covers`: a current admission with the SAME
    /// principal, Root instance, strictly same classification and byte-equal
    /// captured lineage (including serialized provenance) covers the stale
    /// one; a different principal, a different opened Root identity, a
    /// different captured lineage (provider, request hash, prompt) or a
    /// different recorded output classification never covers it. The helper
    /// returns `Result<bool,E>`: every fixture here is a valid producer
    /// representation that always serializes, so each assertion observes an
    /// `Ok` decision (`unwrap` on a non-coverage case would panic on the
    /// typed error, i.e. an Err is never silently accepted as false); the
    /// typed `Err` branch is contract-preserved in the helper but NOT
    /// fault-branch-tested here, because a valid `Manifest` cannot fail to
    /// serialize. The wrong-Root variant mutates the owned fixture record's
    /// root tuple fields (pure fixture values, no live OS identity claim).
    /// Pure inputs, no IO; does not replace the public end-to-end recovery
    /// proof, which is the red flow
    /// `authorized-recovery-after-policy-renewal` of
    /// `tests/blp_dialogue_reader_public.py`.
    #[test]
    fn equivalent_current_covers_only_same_identity_and_lineage(){
        let stale=fixture_admission("alice",Classification{level:"I3".into(),compartments:vec![]},fixture_capture());
        // Same lineage recaptured through the real producer (byte-equal) → covered.
        let current=fixture_admission("alice",Classification{level:"I3".into(),compartments:vec![]},fixture_capture());
        assert!(matches!(preparation_admission::equivalent_current_covers(&stale,&current),Ok(true)));
        // Unrelated principal → never covers.
        let other_principal=fixture_admission("bob",Classification{level:"I3".into(),compartments:vec![]},fixture_capture());
        assert!(!preparation_admission::equivalent_current_covers(&stale,&other_principal).unwrap());
        // Different opened Root identity (owned pure fixture tuples in the
        // record, no OS identity claim) → never covers.
        let mut other_root=fixture_admission("alice",Classification{level:"I3".into(),compartments:vec![]},fixture_capture());
        other_root.root=super::super::RootInstanceIdentity{directory:(9,8),database:(7,6)};
        assert!(!preparation_admission::equivalent_current_covers(&stale,&other_root).unwrap());
        // Changed capture lineage (provider and request hash) → never covers.
        let mut changed=fixture_capture();
        changed.provider="other-provider".into();
        changed.request_hash="other-request-hash".into();
        let other_capture=fixture_admission("alice",Classification{level:"I3".into(),compartments:vec![]},changed);
        assert!(!preparation_admission::equivalent_current_covers(&stale,&other_capture).unwrap());
        // Changed prompt bytes with valid recomputed provenance → never covers.
        let mut retrained=fixture_capture();
        let new_data=serde_json::json!({"documents":{"besoin":"échéance"},"sources":[{"text":"source"}],
            "adopted_brief":null,"method":"apex","method_sources":["instructions"],
            "history":[{"question":"Q","answer":"A"}],"history_mode":"full",
            "earlier_exchanges_not_sent":0,"user_message":"courant modifié"});
        let (prompt,provenance)=assemble(None,"instructions\n",None,&new_data).unwrap();
        retrained.prompt=prompt; retrained.provenance=provenance;
        let other_prompt=fixture_admission("alice",Classification{level:"I3".into(),compartments:vec![]},retrained);
        assert!(!preparation_admission::equivalent_current_covers(&stale,&other_prompt).unwrap());
        // Different recorded output classification (no higher-label coverage) → never covers.
        let higher=fixture_admission("alice",Classification{level:"I4".into(),compartments:vec![]},fixture_capture());
        assert!(!preparation_admission::equivalent_current_covers(&stale,&higher).unwrap());
    }
    /// Targeted unit for `classify_verify_result`: only the exact ordinary-
    /// staleness refusal is classified stale; every neighbor refusal —
    /// invalid record, wrong Root instance, not-found — propagates unchanged
    /// even though a current equivalent could otherwise exist, which is the
    /// typed-level half of "invalid Root + stale remains a failure" (the
    /// store-level half stays the public red-flow contract). Positive results
    /// classify current. Pure inputs, no IO, no store.
    #[test]
    fn classify_verify_result_propagates_every_non_stale_refusal(){
        let ok:Result<usize,E>=Ok(2);
        assert!(matches!(classify_verify_result(ok),Ok(true)));
        let stale:Result<usize,E>=Err(E::Refused(preparation_admission::POLICY_CHANGED));
        assert!(matches!(classify_verify_result(stale),Ok(false)));
        for (input,expected) in [
            (E::Refused("blp-admission-record-invalid"),"blp-admission-record-invalid"),
            (E::Refused("preparation-root-instance-changed"),"preparation-root-instance-changed"),
            (E::Refused("blp-admission-not-found"),"blp-admission-not-found"),
        ]{
            match classify_verify_result(Err(input)){
                Err(E::Refused(code))=>assert_eq!(code,expected),
                _=>panic!("non-staleness refusal must propagate unchanged"),
            }
        }
    }
    /// Focused unit regression for the after-verify result adapter
    /// `reader_stale` only. It must convert the exact ordinary-staleness
    /// refusal of `verify` to the declared reader code and preserve every
    /// neighbor refusal with its original typed code, preserve the same code
    /// inside a `TrustPublished` close wrapper and pass a positive `verify`
    /// result through unchanged. The inputs are constructed directly; these
    /// tests perform no IO, no store access and no authorization, and they do
    /// not replace the integration red proof of the public renewal boundary.
    #[test]
    fn reader_stale_adapts_only_exact_stale_refusal(){
        // Exact ordinary-staleness refusal → declared reader code.
        match reader_stale(E::Refused(preparation_admission::POLICY_CHANGED)){
            E::Refused(code)=>assert_eq!(code,POLICY_STALE),
            _=>panic!("exact staleness refusal must become the reader staleness refusal"),
        }
        // Neighbor refusals of verify keep their original typed codes.
        for (input,expected) in [
            (E::Refused("blp-admission-not-found"),"blp-admission-not-found"),
            (E::Refused("blp-admission-record-invalid"),"blp-admission-record-invalid"),
            (E::Refused("preparation-root-instance-changed"),"preparation-root-instance-changed"),
        ]{
            match reader_stale(input){
                E::Refused(code)=>assert_eq!(code,expected),
                _=>panic!("neighbor refusal must stay a refusal with its original code"),
            }
        }
        // The exact stale code inside a close wrapper keeps its typed shape.
        let published=E::TrustPublished{primary:Box::new(E::Refused(preparation_admission::POLICY_CHANGED)),publication:"admission-replayed"};
        match reader_stale(published){
            E::TrustPublished{primary,publication}=>{
                assert_eq!(publication,"admission-replayed");
                match primary.as_ref(){
                    E::Refused(code)=>assert_eq!(*code,preparation_admission::POLICY_CHANGED),
                    _=>panic!("wrapper primary must keep its original typed code"),
                }
            }
            _=>panic!("close wrapper must keep its typed shape"),
        }
        // A positive verify result passes through unchanged.
        let ok:Result<usize,E>=Ok(3);
        match ok.map_err(reader_stale){
            Ok(count)=>assert_eq!(count,3),
            Err(_)=>panic!("positive verify result must pass through unchanged"),
        }
    }
}
