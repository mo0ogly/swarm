//! Explicit historical preparation recovery; no inherited process signalling.
use super::{cause, finished, verify_record, E};
use crate::PreparationTurn;
use rusqlite::{params, Connection};

/// Read the registered supervisor on this same host/boot/namespace. Missing or
/// malformed registration is unknown, never proof of death. A different kernel
/// birth stamp or confirmed absence establishes only that the recorded process
/// is gone; descendants and remote expenditure may survive. No signal or wait.
fn dead(t: &PreparationTurn) -> Result<bool, E> {
    if t.supervisor_pid <= 0 || t.supervisor_stamp.parse::<u64>().ok().filter(|v| *v > 0).is_none() {
        return Err(E::Refused("preparation-legacy-worker-identity-missing"));
    }
    if t.host != crate::SupervisorIdentity::current().map_err(|e| cause("preparation-worker-host", e))?.host() {
        return Err(E::Refused("preparation-worker-host-mismatch"));
    }
    let pid = u32::try_from(t.supervisor_pid).map_err(|e| cause("preparation-worker-pid", e))?;
    match crate::observe_linux_process(pid) {
        Ok(p) => Ok(p.process_stamp() != t.supervisor_stamp),
        Err(e) if e.confirms_absence() => Ok(true),
        Err(e) => Err(cause("preparation-worker-observe", e)),
    }
}

/// Append authoritative Go-ordered fields to the validated historical JSON
/// object, preserving every original byte/unknown/private field before its final
/// brace. At most two transitions append fields (running→stopping→interrupted);
/// idempotent reads append nothing. This is not a native metadata conversion.
/// Serialization/invalid-body errors propagate before any SQL write.
fn amended(raw: &[u8], t: &PreparationTurn) -> Result<Vec<u8>, E> {
    let end = raw.iter().rposition(|b| !b.is_ascii_whitespace()).filter(|i| raw[*i] == b'}')
        .ok_or(E::Refused("preparation-dialogue-document-invalid"))?;
    let suffix = serde_json::to_vec(&serde_json::json!({"status":t.status,"ended_at":t.ended_at,"error":t.error}))
        .map_err(|e| cause("preparation-call-encode", e))?;
    let mut out = raw[..end].to_vec();
    // A decoded turn with matching nonempty ID cannot be an empty object.
    out.push(b','); out.extend_from_slice(&suffix[1..]); out.extend_from_slice(&raw[end+1..]);
    Ok(out)
}

/// Stop an already decoded legacy row within the bound Root's immediate write
/// transaction. Pending CAS fences Go's claim before launch; live running becomes
/// stopping, leaving cancellation to its original worker. Repeated stopping can
/// settle only after confirmed supervisor death. Unknown identity refuses without
/// mutation. The existing ledger releases reserved only, never estimated; absent
/// or corrupt ledgers roll back. Original JSON/prompt/unknown fields survive.
/// No native envelope, timeout, dispatch, raw-PID signal or cost guarantee.
pub(super) fn stop(db: &Connection, mut t: PreparationTurn, at: &str) -> Result<PreparationTurn, E> {
    if db.is_autocommit() { return Err(E::Refused("preparation-call-transaction-required")); }
    if !t.active() { return Ok(t); }
    let previous = t.status.clone();
    let terminal = previous == "pending" || dead(&t)?;
    if !terminal && previous == "stopping" { return Ok(t); }
    let mut statement = db.prepare("SELECT body FROM preparation_turns WHERE id=?1 AND preparation_id=?2 AND status=?3").map_err(E::Sql)?;
    let raw = statement.query_row(params![t.id,t.preparation_id,previous], |row| Ok(row.get_ref(0)?.as_bytes()?.to_vec())).map_err(E::Sql);
    let raw = finished(raw, statement.finalize().map_err(E::Sql))?;
    if terminal {
        t.status = "interrupted".into(); t.ended_at = at.into();
        if t.error.is_empty() { t.error = "preparation-call-interrupted".into(); }
    } else { t.status = "stopping".into(); }
    let next = amended(&raw, &t)?;
    let mut statement = db.prepare("UPDATE preparation_turns SET status=?4,body=?5 WHERE id=?1 AND preparation_id=?2 AND status=?3 AND CAST(body AS BLOB)=?6").map_err(E::Sql)?;
    let changed = statement.execute(params![t.id,t.preparation_id,previous,t.status,next,raw]).map_err(E::Sql);
    if finished(changed, statement.finalize().map_err(E::Sql))? != 1 { return Err(E::Refused("preparation-call-row-unmatched")); }
    verify_record(db,&t,&next)?;
    if terminal {
        let settlement = super::super::preparation_call_budget::release_unstarted(db,&t.id)?;
        verify_record(db,&t,&next)?;
        super::super::preparation_call_budget::verify_settlement(db,&settlement)?;
    }
    Ok(t)
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Actual Go-ordered decoder, including duplicate/alias/unknown fields.
    #[test]
    fn amended_preserves_private_unknown_and_ordered_fields() {
        let raw = br#"{ "id":"turn","preparation_id":"prep","status":"running","STATUS":"running","prompt":"secret","future":{"x":2} }  "#;
        let t = PreparationTurn { id:"turn".into(),preparation_id:"prep".into(),status:"interrupted".into(),ended_at:"now".into(),error:"safe".into(),..Default::default() };
        let saved = amended(raw,&t).unwrap();
        assert!(saved.starts_with(&raw[..raw.iter().rposition(|b| *b==b'}').unwrap()]));
        let observed:PreparationTurn=crate::work::decode(&saved,crate::preparation_turn::shape_preparation_turn()).unwrap();
        assert_eq!(observed.status,"interrupted"); assert_eq!(observed.prompt,"secret");
        assert_eq!(observed.ended_at,"now"); assert_eq!(observed.error,"safe");
        let value:serde_json::Value=serde_json::from_slice(&saved).unwrap();
        assert_eq!(value["future"]["x"],2); assert!(value.get("_native").is_none());
    }
    /// Missing metadata remains historical; explicit null never bypasses native
    /// validation into the legacy recovery path. No IO or process action.
    #[test]
    fn explicit_null_native_refuses_legacy_fallback() {
        let db=Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE preparation_turns(id TEXT,preparation_id TEXT,status TEXT,body BLOB);").unwrap();
        for (value,valid) in [(r#"{"id":"t","preparation_id":"p"}"#,true),(r#"{"id":"t","preparation_id":"p","_native":null}"#,false)] {
            db.execute("DELETE FROM preparation_turns",[]).unwrap();
            db.execute("INSERT INTO preparation_turns VALUES('t','p','pending',?1)",[value.as_bytes()]).unwrap();
            assert_eq!(super::super::record(&db,"t").is_ok(),valid);
        }
    }
}
