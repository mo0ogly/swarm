//! Immutable native claim inputs. Hashes bind data, never caller authority.
#![cfg(target_os = "linux")]
use crate::{AgentLaunchError as E, PlanningRequest, Work};
use rusqlite::{params, Connection, Transaction};
use sha2::{Digest, Sha256};
/// Auxiliary v1 layout: operational ownership in columns, input documents as
/// immutable UTF8 evidence. No TTL, purge policy or new core-schema version.
const TABLE:&str="CREATE TABLE native_planning_claims(claim_id TEXT PRIMARY KEY REFERENCES events(id),work_id TEXT NOT NULL REFERENCES works(id),scope_id TEXT NOT NULL,generation INTEGER NOT NULL CHECK(generation>0),scope_revision INTEGER NOT NULL CHECK(scope_revision>0),claim_revision INTEGER NOT NULL CHECK(claim_revision>0),holder TEXT NOT NULL,context_json TEXT NOT NULL,context_sha256 TEXT NOT NULL,workflow_json TEXT NOT NULL,workflow_prompt TEXT NOT NULL,workflow_prompt_sha256 TEXT NOT NULL,model_json TEXT NOT NULL,recorded_at TEXT NOT NULL,UNIQUE(work_id,scope_id,generation))";
/// Hash complete bytes; equality establishes neither producer identity nor health.
fn hash(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}
/// Read or create exactly this auxiliary layout in the caller transaction. A
/// reserved-name collision, altered DDL, extra index/trigger or SQL/close error
/// refuses. Read-only mode never creates; creation is an internal phase, not an
/// operator-selectable bypass. Caller owns rollback/commit and source authority.
pub(crate) fn schema(db: &Connection, create: bool) -> Result<bool, E> {
    let found = crate::agent_launch::query(
        db,
        "SELECT type,sql FROM main.sqlite_schema WHERE name=?",
        ["native_planning_claims"],
        |rows| {
            let found = match rows.next()? {
                None => None,
                Some(r) => Some((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            };
            if rows.next()?.is_some() {
                return Err(E::Refused("planning-claim-input-schema-collision"));
            }
            Ok(found)
        },
    )?;
    match found {
        None if !create => return Ok(false),
        None => db
            .execute_batch(TABLE)
            .map_err(|e| E::caused("planning-claim-input-schema-create", e))?,
        Some((kind, sql)) if kind == "table" && sql == TABLE => {}
        Some(_) => return Err(E::Refused("planning-claim-input-schema-invalid")),
    }
    let extras:i64=crate::agent_launch::query(db,"SELECT count(*) FROM main.sqlite_schema WHERE tbl_name=? AND type!='table' AND substr(name,1,7)!='sqlite_'",["native_planning_claims"],|rows|Ok(rows.next()?.ok_or(E::Refused("planning-claim-input-schema-observation"))?.get(0)?))?;
    if extras != 0 {
        return Err(E::Refused("planning-claim-input-schema-invalid"));
    }
    Ok(true)
}
/// Persist one append-only binding after the matching event has been staged.
/// Verify complete documents/digests, current scope and actual event correlation;
/// any refusal or later transaction failure must roll back Work, reserve and
/// binding together. No process/permission/receipt, invoice or crypto identity
/// is established. Existing bindings cannot be overwritten, including on replay.
pub(crate) fn stage(
    tx: &Transaction<'_>,
    work: &Work,
    r: &PlanningRequest,
    input: &crate::planning_activation::ClaimInputs,
) -> Result<(), E> {
    schema(tx, true)?;
    let scope = work
        .planning
        .as_deref()
        .and_then(|p| p.scopes.as_deref())
        .and_then(|s| s.iter().find(|s| s.id == r.scope))
        .ok_or(E::Refused("planning-claim-input-scope-missing"))?;
    let delivery = scope
        .delivery
        .as_deref()
        .ok_or(E::Refused("planning-claim-input-delivery-missing"))?;
    let workflow = scope
        .workflow
        .as_deref()
        .ok_or(E::Refused("planning-claim-input-workflow-missing"))?;
    let model = scope
        .last_model
        .as_deref()
        .ok_or(E::Refused("planning-claim-input-model-missing"))?;
    if scope.generation < 1
        || scope.revision != r.scope_revision
        || scope.holder != r.holder
        || delivery.context_sha256 != hash(&input.context)
        || input.workflow_prompt_sha256 != hash(input.workflow_prompt.as_bytes())
    {
        return Err(E::Refused("planning-claim-input-correlation-invalid"));
    }
    let context = std::str::from_utf8(&input.context)
        .map_err(|e| E::caused("planning-claim-input-utf8-invalid", e))?;
    let workflow = String::from_utf8(
        crate::encode_model(workflow)
            .map_err(|e| E::caused("planning-claim-input-workflow-encoding", e))?,
    )
    .map_err(|e| E::caused("planning-claim-input-workflow-utf8", e))?;
    let model = String::from_utf8(
        crate::encode_model(model)
            .map_err(|e| E::caused("planning-claim-input-model-encoding", e))?,
    )
    .map_err(|e| E::caused("planning-claim-input-model-utf8", e))?;
    let event: (String, i64, String) = crate::agent_launch::query(
        tx,
        "SELECT work_id,revision,kind FROM main.events WHERE id=?",
        [&r.event_id],
        |rows| {
            let row = rows
                .next()?
                .ok_or(E::Refused("planning-claim-input-event-missing"))?;
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        },
    )?;
    if event != (work.id.clone(), work.revision, "planning.claim".into()) {
        return Err(E::Refused("planning-claim-input-event-mismatch"));
    }
    let changed = tx
        .execute(
            "INSERT INTO native_planning_claims VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            params![
                r.event_id,
                work.id,
                r.scope,
                scope.generation,
                scope.revision,
                work.revision,
                r.holder,
                context,
                delivery.context_sha256,
                workflow,
                input.workflow_prompt,
                input.workflow_prompt_sha256,
                model,
                work.updated
            ],
        )
        .map_err(|e| E::caused("planning-claim-input-write", e))?;
    if changed != 1 {
        return Err(E::Refused("planning-claim-input-not-inserted"));
    }
    let stored:(String,String,String,String,i64)=crate::agent_launch::query(tx,"SELECT context_json,workflow_json,workflow_prompt,model_json,generation FROM native_planning_claims WHERE claim_id=?",[&r.event_id],|rows|{let row=rows.next()?.ok_or(E::Refused("planning-claim-input-readback-missing"))?;Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?))})?;
    if stored
        != (
            context.into(),
            workflow,
            input.workflow_prompt.clone(),
            model,
            scope.generation,
        )
    {
        return Err(E::Refused("planning-claim-input-readback-mismatch"));
    }
    Ok(())
}
