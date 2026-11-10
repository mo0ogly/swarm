//! Dedicated Admin advice envelope. Scalar SQL history/reservations are not
//! provider invoices, execution permission or authenticated human identity.
use super::{finished, RootStore, RootStoreError as R};
use crate::{agent_launch::query, AgentLaunchError as E, SettingCount, WorkEnvironment};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Explicit operator estimate in micro-USD (10^-6 USD), exact signed-64-bit
/// nonnegative representation. Null limit is explicit unlimited; zero admits no
/// call. Estimates require positive reserve and provenance even when unlimited.
/// These values cannot guarantee actual external spending or select a provider.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminBudgetPolicy {
    #[serde(deserialize_with = "required_limit")]
    pub limit_micro_usd: Option<SettingCount>,
    pub reserve_per_call_micro_usd: SettingCount,
    pub estimate_source: String,
}
/// Strict configuration intention. Revision zero means first configuration only.
/// Preview token binds policy and current consumption, not authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminBudgetChange {
    pub schema_version: i64,
    pub event_id: String,
    pub expected_revision: SettingCount,
    pub policy: AdminBudgetPolicy,
    pub reason: String,
    #[serde(default)]
    pub preview_token: String,
}
/// Immutable logical history; actor is a local environment observation. Hashes
/// detect drift/idempotence, not hostile same-UID edits or full database rollback.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AdminBudgetEntry {
    pub revision: SettingCount,
    pub event_id: String,
    pub policy: AdminBudgetPolicy,
    pub reason: String,
    pub actor: String,
    pub at: String,
    pub request_sha256: String,
    pub previous_sha256: String,
    pub entry_sha256: String,
}
/// Saved envelope and sum of retained reservations. Actual billed cost is unknown;
/// absence and explicit unlimited are distinct through configured/policy. A limit
/// below retained usage is allowed to stop calls, never to erase consumption.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AdminBudgetView {
    pub schema_version: i64,
    pub currency: &'static str,
    pub amount_unit: &'static str,
    pub configured: bool,
    pub revision: SettingCount,
    pub policy: Option<AdminBudgetPolicy>,
    pub reserved_estimate_micro_usd: SettingCount,
    pub actual_billed_cost_known: bool,
    pub history: Vec<AdminBudgetEntry>,
}
/// Pure proposal or confirmed/replayed result. A proposal is not saved policy;
/// no response is provider execution permission, even after reservation.
#[derive(Debug, Serialize)]
pub struct AdminBudgetOutcome {
    pub schema_version: i64,
    pub document: AdminBudgetView,
    pub proposed_policy: AdminBudgetPolicy,
    pub preview_token: String,
    pub applied: bool,
    pub replayed: bool,
}
/// Caller-bound accounting intention. A future native gate must independently
/// verify its job/identity/source before any provider. Same id+hash replays the
/// original estimate even after policy changes; conflicting reuse refuses.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminBudgetReservationRequest {
    pub call_id: String,
    pub request_sha256: String,
    pub expected_budget_revision: SettingCount,
}
/// Retained estimate, never proof of a started call or actual zero/nonzero cost.
/// No automatic refund exists: failure/uncertainty cannot manufacture capacity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AdminBudgetReservation {
    pub call_id: String,
    pub request_sha256: String,
    pub budget_revision: SettingCount,
    pub reserved_estimate_micro_usd: SettingCount,
    pub replayed: bool,
    pub execution_authorized: bool,
}
const POLICY: &str = "admin_advice_budget_history";
const CALLS: &str = "admin_advice_budget_reservations";
const POLICY_DDL:&str="CREATE TABLE admin_advice_budget_history(revision INTEGER PRIMARY KEY CHECK(revision>0),event_id TEXT NOT NULL UNIQUE,limit_micro_usd INTEGER CHECK(limit_micro_usd>=0),reserve_micro_usd INTEGER NOT NULL CHECK(reserve_micro_usd>0),estimate_source TEXT NOT NULL,reason TEXT NOT NULL,actor TEXT NOT NULL,at TEXT NOT NULL,request_sha256 TEXT NOT NULL,previous_sha256 TEXT NOT NULL,entry_sha256 TEXT NOT NULL)";
const CALLS_DDL:&str="CREATE TABLE admin_advice_budget_reservations(call_id TEXT PRIMARY KEY,request_sha256 TEXT NOT NULL,budget_revision INTEGER NOT NULL REFERENCES admin_advice_budget_history(revision),reserve_micro_usd INTEGER NOT NULL CHECK(reserve_micro_usd>0),reservation_sha256 TEXT NOT NULL)";
/// Required nullable field: absence cannot silently mean an unlimited budget.
fn required_limit<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Option<SettingCount>, D::Error> {
    Option::<SettingCount>::deserialize(decoder)
}
/// Mark an uncertain SQL commit before the retry/release wrapper can combine
/// failures; original typed commit/rollback causes remain available underneath.
fn root(error: E) -> R {
    let uncertain = matches!(
        &error,
        E::Commit {
            rollback_confirmed: false,
            ..
        }
    );
    let primary = R::Launch(Box::new(error));
    if uncertain {
        R::Published {
            primary: Box::new(primary),
        }
    } else {
        primary
    }
}
/// Hash canonical typed intent bytes. Not a signature or an identity credential.
fn hash(value: &impl Serialize) -> Result<String, E> {
    serde_json::to_vec(value)
        .map(|raw| format!("{:x}", Sha256::digest(raw)))
        .map_err(|e| E::caused("admin-budget-encode", e))
}
/// Require a real caller-owned SQL transaction before any ledger mutation.
fn writing(db: &Connection) -> Result<(), E> {
    if db.is_autocommit() {
        Err(E::Refused("admin-budget-transaction-required"))
    } else {
        Ok(())
    }
}
/// Verify both auxiliary definitions and their exact indexes/triggers. Missing
/// pair is unconfigured; partial/tampered schema refuses. Creates only in TX.
fn schema(db: &Connection, create: bool) -> Result<bool, E> {
    let mut present = Vec::new();
    for name in [POLICY, CALLS] {
        present.push(query(
            db,
            "SELECT type,sql FROM main.sqlite_schema WHERE name=?",
            [name],
            |rows| {
                Ok(rows
                    .next()?
                    .map(|row| {
                        Ok::<_, rusqlite::Error>((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                        ))
                    })
                    .transpose()?)
            },
        )?)
    }
    if present.iter().all(Option::is_none) {
        if !create {
            return Ok(false);
        }
        writing(db)?;
        db.execute_batch(POLICY_DDL)?;
        db.execute_batch(CALLS_DDL)?;
    } else if present[0].as_ref() != Some(&("table".into(), POLICY_DDL.into()))
        || present[1].as_ref() != Some(&("table".into(), CALLS_DDL.into()))
    {
        return Err(E::Refused("admin-budget-schema-invalid"));
    }
    let actual=query(db,"SELECT type,name,sql FROM main.sqlite_schema WHERE tbl_name IN (?,?) AND name NOT IN (?,?) ORDER BY name",params![POLICY,CALLS,POLICY,CALLS],|rows|{let mut found=Vec::new();while let Some(row)=rows.next()?{found.push((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,Option<String>>(2)?))}Ok(found)})?;
    let expected = vec![
        (
            "index".to_string(),
            format!("sqlite_autoindex_{POLICY}_1"),
            None,
        ),
        (
            "index".to_string(),
            format!("sqlite_autoindex_{CALLS}_1"),
            None,
        ),
    ];
    if actual != expected {
        return Err(E::Refused("admin-budget-schema-invalid"));
    }
    Ok(true)
}
/// Validate operator policy, not pricing. Source is explicit unverified prose;
/// no hardcoded price, expiry, budget or reset is inferred.
fn validate(policy: &AdminBudgetPolicy) -> Result<(), E> {
    if policy.limit_micro_usd.is_some_and(|v| v.0 < 0)
        || policy.reserve_per_call_micro_usd.0 <= 0
        || policy.estimate_source.trim().is_empty()
    {
        return Err(E::Refused("admin-budget-policy-invalid"));
    }
    Ok(())
}
/// Strict bounded decoding. Unknown fields, duplicate keys, malformed counts and
/// missing policy data refuse; null is explicit unlimited, never absence.
fn decode(raw: &[u8]) -> Result<AdminBudgetChange, R> {
    crate::check_command_input_size(raw).map_err(R::Input)?;
    let change: AdminBudgetChange = serde_json::from_slice(raw)
        .map_err(|e| root(E::caused("admin-budget-input-invalid", e)))?;
    if change.schema_version != 1
        || change.event_id.trim().is_empty()
        || change.reason.trim().is_empty()
    {
        return Err(root(E::Refused("admin-budget-intent-invalid")));
    }
    validate(&change.policy).map_err(root)?;
    Ok(change)
}
/// Observe all immutable scalar rows and validate contiguous history/references.
/// Missing schema is the only unconfigured case. Sum overflow/corruption refuses
/// rather than inventing zero capacity/consumption. Caller owns snapshot/lock.
fn read(db: &Connection) -> Result<AdminBudgetView, E> {
    let mut view = AdminBudgetView {
        schema_version: 1,
        currency: "USD",
        amount_unit: "micro_usd",
        configured: false,
        revision: SettingCount(0),
        policy: None,
        reserved_estimate_micro_usd: SettingCount(0),
        actual_billed_cost_known: false,
        history: Vec::new(),
    };
    if !schema(db, false)? {
        return Ok(view);
    }
    view.history=query(db,"SELECT revision,event_id,limit_micro_usd,reserve_micro_usd,estimate_source,reason,actor,at,request_sha256,previous_sha256,entry_sha256 FROM admin_advice_budget_history ORDER BY revision",[],|rows|{let mut found=Vec::new();while let Some(row)=rows.next()?{found.push(AdminBudgetEntry{revision:SettingCount(row.get(0)?),event_id:row.get(1)?,policy:AdminBudgetPolicy{limit_micro_usd:row.get::<_,Option<i64>>(2)?.map(SettingCount),reserve_per_call_micro_usd:SettingCount(row.get(3)?),estimate_source:row.get(4)?},reason:row.get(5)?,actor:row.get(6)?,at:row.get(7)?,request_sha256:row.get(8)?,previous_sha256:row.get(9)?,entry_sha256:row.get(10)?})}Ok(found)})?;
    if view.history.is_empty() {
        return Err(E::Refused("admin-budget-history-empty"));
    }
    let mut previous = String::new();
    for entry in &view.history {
        if entry.revision.0
            != view
                .revision
                .0
                .checked_add(1)
                .ok_or(E::Refused("admin-budget-revision-representation"))?
            || entry.event_id.trim().is_empty()
            || entry.reason.trim().is_empty()
            || entry.actor.trim().is_empty()
            || entry.at.trim().is_empty()
            || !digest(&entry.request_sha256)
        {
            return Err(E::Refused("admin-budget-history-invalid"));
        }
        if entry.previous_sha256 != previous || entry.entry_sha256 != entry_hash(entry)? {
            return Err(E::Refused("admin-budget-history-checksum-invalid"));
        }
        previous = entry.entry_sha256.clone();
        validate(&entry.policy)?;
        view.revision = entry.revision;
        view.policy = Some(entry.policy.clone())
    }
    let reservations=query(db,"SELECT r.call_id,r.request_sha256,r.reserve_micro_usd,p.reserve_micro_usd,p.revision,r.reservation_sha256 FROM admin_advice_budget_reservations r LEFT JOIN admin_advice_budget_history p ON p.revision=r.budget_revision",[],|rows|{let mut found=Vec::new();while let Some(row)=rows.next()?{found.push((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,i64>(2)?,row.get::<_,Option<i64>>(3)?,row.get::<_,Option<i64>>(4)?,row.get::<_,String>(5)?))}Ok(found)})?;
    for (id, sha, amount, original, revision, receipt) in reservations {
        if id.trim().is_empty()
            || !digest(&sha)
            || amount <= 0
            || Some(amount) != original
            || revision.is_none()
        {
            return Err(E::Refused("admin-budget-reservation-invalid"));
        }
        if receipt != hash(&(id, sha, revision, amount))? {
            return Err(E::Refused("admin-budget-reservation-checksum-invalid"));
        }
        view.reserved_estimate_micro_usd.0 = view
            .reserved_estimate_micro_usd
            .0
            .checked_add(amount)
            .ok_or(E::Refused("admin-budget-consumption-representation"))?
    }
    view.configured = true;
    Ok(view)
}
/// Checksum the complete typed history entry and its predecessor. Detects drift,
/// not a hostile writer recomputing hashes or rolling back the entire Root.
fn entry_hash(entry: &AdminBudgetEntry) -> Result<String, E> {
    hash(&(
        1,
        entry.revision,
        &entry.event_id,
        &entry.policy,
        &entry.reason,
        &entry.actor,
        &entry.at,
        &entry.request_sha256,
        &entry.previous_sha256,
    ))
}
/// Canonical SHA256 format is a protocol invariant, not an operator cutoff.
fn digest(raw: &str) -> bool {
    raw.len() == 64
        && raw
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
/// Bind typed proposal to saved monetary revision/consumption. A hash is not
/// authentication; current locked state remains authoritative during apply.
fn token(change: &AdminBudgetChange, view: &AdminBudgetView) -> Result<String, E> {
    let mut intent = change.clone();
    intent.preview_token.clear();
    hash(&(intent, view.revision, view.reserved_estimate_micro_usd))
}
/// Verify exact event replay before freshness checks. Does not publish another
/// row, restore old limits or consume another estimate.
fn replay(change: &AdminBudgetChange, view: &AdminBudgetView) -> Result<bool, E> {
    if let Some(entry) = view
        .history
        .iter()
        .find(|entry| entry.event_id == change.event_id)
    {
        if entry.request_sha256 != hash(change)? {
            return Err(E::Refused("admin-budget-event-conflict"));
        }
        return Ok(true);
    }
    Ok(false)
}
/// Persist a new policy in the caller TX. Refusals have no mutation; exact replay
/// returns the current document. Existing reservations are never rewritten.
fn apply(
    db: &Connection,
    change: &AdminBudgetChange,
    actor: &str,
    at: &str,
) -> Result<AdminBudgetOutcome, E> {
    writing(db)?;
    let before = read(db)?;
    if replay(change, &before)? {
        return Ok(AdminBudgetOutcome {
            schema_version: 1,
            document: before,
            proposed_policy: change.policy.clone(),
            preview_token: change.preview_token.clone(),
            applied: true,
            replayed: true,
        });
    }
    if before.revision != change.expected_revision {
        return Err(E::Refused("admin-budget-revision-conflict"));
    }
    if change.preview_token.is_empty() || token(change, &before)? != change.preview_token {
        return Err(E::Refused("admin-budget-preview-stale"));
    }
    if actor.trim().is_empty() || at.trim().is_empty() {
        return Err(E::Refused("admin-budget-author-observation-invalid"));
    }
    let next = before
        .revision
        .0
        .checked_add(1)
        .ok_or(E::Refused("admin-budget-revision-representation"))?;
    schema(db, true)?;
    let mut entry = AdminBudgetEntry {
        revision: SettingCount(next),
        event_id: change.event_id.clone(),
        policy: change.policy.clone(),
        reason: change.reason.clone(),
        actor: actor.into(),
        at: at.into(),
        request_sha256: hash(change)?,
        previous_sha256: before
            .history
            .last()
            .map(|entry| entry.entry_sha256.clone())
            .unwrap_or_default(),
        entry_sha256: String::new(),
    };
    entry.entry_sha256 = entry_hash(&entry)?;
    let n=db.execute("INSERT INTO admin_advice_budget_history(revision,event_id,limit_micro_usd,reserve_micro_usd,estimate_source,reason,actor,at,request_sha256,previous_sha256,entry_sha256) VALUES(?,?,?,?,?,?,?,?,?,?,?)",params![next,change.event_id,change.policy.limit_micro_usd.map(|v|v.0),change.policy.reserve_per_call_micro_usd.0,change.policy.estimate_source,change.reason,actor,at,entry.request_sha256,entry.previous_sha256,entry.entry_sha256])?;
    if n != 1 {
        return Err(E::Refused("admin-budget-policy-not-recorded"));
    }
    Ok(AdminBudgetOutcome {
        schema_version: 1,
        document: read(db)?,
        proposed_policy: change.policy.clone(),
        preview_token: change.preview_token.clone(),
        applied: true,
        replayed: false,
    })
}
/// Atomically reserve against the current global envelope. Historical immutable
/// estimates remain charged on every later policy. Replay grants no fresh exec.
fn reserve(
    db: &Connection,
    request: &AdminBudgetReservationRequest,
) -> Result<AdminBudgetReservation, E> {
    writing(db)?;
    if request.call_id.trim().is_empty() || !digest(&request.request_sha256) {
        return Err(E::Refused("admin-budget-reservation-intent-invalid"));
    }
    let view = read(db)?;
    let policy = view
        .policy
        .as_ref()
        .ok_or(E::Refused("admin-budget-not-configured"))?;
    if let Some(old)=query(db,"SELECT request_sha256,budget_revision,reserve_micro_usd FROM admin_advice_budget_reservations WHERE call_id=?",[&request.call_id],|rows|Ok(rows.next()?.map(|row|Ok::<_,rusqlite::Error>((row.get::<_,String>(0)?,row.get::<_,i64>(1)?,row.get::<_,i64>(2)?))).transpose()?))?{
        if old.0!=request.request_sha256{return Err(E::Refused("admin-budget-reservation-conflict"))}return Ok(AdminBudgetReservation{call_id:request.call_id.clone(),request_sha256:old.0,budget_revision:SettingCount(old.1),reserved_estimate_micro_usd:SettingCount(old.2),replayed:true,execution_authorized:false})
    }
    if request.expected_budget_revision != view.revision {
        return Err(E::Refused("admin-budget-revision-conflict"));
    }
    let proposed = view
        .reserved_estimate_micro_usd
        .0
        .checked_add(policy.reserve_per_call_micro_usd.0)
        .ok_or(E::Refused("admin-budget-consumption-representation"))?;
    if policy
        .limit_micro_usd
        .is_some_and(|limit| proposed > limit.0)
    {
        return Err(E::Refused("admin-budget-exhausted"));
    }
    let n=db.execute("INSERT INTO admin_advice_budget_reservations(call_id,request_sha256,budget_revision,reserve_micro_usd,reservation_sha256) VALUES(?,?,?,?,?)",params![request.call_id,request.request_sha256,view.revision.0,policy.reserve_per_call_micro_usd.0,hash(&(&request.call_id,&request.request_sha256,Some(view.revision.0),policy.reserve_per_call_micro_usd.0))?])?;
    if n != 1 {
        return Err(E::Refused("admin-budget-reservation-not-recorded"));
    }
    let after = read(db)?;
    if after.reserved_estimate_micro_usd.0 != proposed {
        return Err(E::Refused("admin-budget-reservation-readback"));
    }
    Ok(AdminBudgetReservation {
        call_id: request.call_id.clone(),
        request_sha256: request.request_sha256.clone(),
        budget_revision: view.revision,
        reserved_estimate_micro_usd: policy.reserve_per_call_micro_usd,
        replayed: false,
        execution_authorized: false,
    })
}
/// Preserve configuration uncertainty on failed commit and known commit followed
/// by lock release. Original SQL/cleanup causes survive; no retry/refund implied.
fn publication<T>(result: Result<T, R>, known: bool) -> Result<T, R> {
    result.map_err(|error|{let uncertain=matches!(&error,R::Launch(cause) if matches!(cause.as_ref(),E::Commit{rollback_confirmed:false,..}));if known||uncertain{R::Published{primary:Box::new(error)}}else{error}})
}
impl RootStore {
    /// Read the global budget in one explicit SQL snapshot under saved settings
    /// reservation. Absence returns configured:false; corrupt state refuses.
    /// No schema creation, provider call, default adoption or permission occurs.
    pub fn admin_advice_budget(&self) -> Result<AdminBudgetView, R> {
        let lock = self.directory.reserve(false, false).map_err(R::Storage)?;
        let result = (|| {
            self.read_settings()?;
            self.db.execute_batch("BEGIN DEFERRED").map_err(R::Sql)?;
            let result = read(&self.db).map_err(root);
            finished(result, self.db.execute_batch("ROLLBACK").map_err(R::Sql))
        })();
        finished(result, lock.close().map_err(R::Storage))
    }
    /// Preview explicit policy with current consumption/settings and no writes.
    /// Root owns validation. The token is freshness binding, not authorization.
    pub fn preview_admin_advice_budget(&self, raw: &[u8]) -> Result<AdminBudgetOutcome, R> {
        let change = decode(raw)?;
        let lock = self.directory.reserve(false, false).map_err(R::Storage)?;
        let result = (|| {
            self.read_settings()?;
            self.db.execute_batch("BEGIN DEFERRED").map_err(R::Sql)?;
            let result = (|| {
                let view = read(&self.db).map_err(root)?;
                if change.expected_revision != view.revision {
                    return Err(root(E::Refused("admin-budget-revision-conflict")));
                }
                Ok(AdminBudgetOutcome {
                    schema_version: 1,
                    preview_token: token(&change, &view).map_err(root)?,
                    document: view,
                    proposed_policy: change.policy.clone(),
                    applied: false,
                    replayed: false,
                })
            })();
            finished(result, self.db.execute_batch("ROLLBACK").map_err(R::Sql))
        })();
        finished(result, lock.close().map_err(R::Storage))
    }
    /// Publish/replay under saved storage retry and an immediate transaction.
    /// Local actor/time are observed by the supplied environment, not requested
    /// identities. Known/uncertain commit and later release errors stay marked.
    /// No mission, provider process, reset or execution grant is created.
    pub fn apply_admin_advice_budget<W: WorkEnvironment>(
        &mut self,
        raw: &[u8],
        environment: &mut W,
    ) -> Result<AdminBudgetOutcome, R> {
        if !self.writable {
            return Err(R::Refused("root-read-only"));
        }
        let change = decode(raw)?;
        let actor = environment.operator().map_err(R::Runtime)?;
        let at = environment.timestamp().map_err(R::Runtime)?;
        let mut known = false;
        let result = crate::agent_storage_retry::execute(
            &mut self.db,
            &self.directory,
            false,
            |db, _settings| {
                let value = crate::agent_execution_store::transaction(db, |tx| {
                    apply(tx, &change, &actor, &at)
                })
                .map_err(root)?;
                known = true;
                Ok(value)
            },
        );
        publication(result, known)
    }
    /// Account one native advice intention, not an exec capability. Caller must
    /// separately establish protected process identity, prompt/source freshness,
    /// consent and provider admission. Reservation and replay are durable, without
    /// refunds or reset. Unknown/late commit stays preserved-state error.
    pub fn reserve_admin_advice_budget(
        &mut self,
        request: &AdminBudgetReservationRequest,
    ) -> Result<AdminBudgetReservation, R> {
        if !self.writable {
            return Err(R::Refused("root-read-only"));
        }
        let mut known = false;
        let result =
            crate::agent_storage_retry::execute(&mut self.db, &self.directory, false, |db, _| {
                let value =
                    crate::agent_execution_store::transaction(db, |tx| reserve(tx, request))
                        .map_err(root)?;
                known = true;
                Ok(value)
            });
        publication(result, known)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    fn db() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON").unwrap();
        db
    }
    fn intent(revision: i64, event: &str, limit: Option<i64>, reserve: i64) -> AdminBudgetChange {
        AdminBudgetChange {
            schema_version: 1,
            event_id: event.into(),
            expected_revision: SettingCount(revision),
            policy: AdminBudgetPolicy {
                limit_micro_usd: limit.map(SettingCount),
                reserve_per_call_micro_usd: SettingCount(reserve),
                estimate_source: "Explicit fixture estimate, not a provider price".into(),
            },
            reason: "Owned budget fixture".into(),
            preview_token: String::new(),
        }
    }
    fn preview(db: &Connection, mut request: AdminBudgetChange) -> AdminBudgetChange {
        request.preview_token = token(&request, &read(db).unwrap()).unwrap();
        request
    }
    fn publish(db: &mut Connection, request: &AdminBudgetChange) -> Result<AdminBudgetOutcome, E> {
        crate::agent_execution_store::transaction(db, |tx| {
            apply(
                tx,
                request,
                "fixture-local-operator",
                "2026-10-21T12:00:00Z",
            )
        })
    }
    fn reservation(id: &str, revision: i64) -> AdminBudgetReservationRequest {
        AdminBudgetReservationRequest {
            call_id: id.into(),
            request_sha256: "a".repeat(64),
            expected_budget_revision: SettingCount(revision),
        }
    }
    fn charge(db: &mut Connection, id: &str, revision: i64) -> Result<AdminBudgetReservation, E> {
        crate::agent_execution_store::transaction(db, |tx| reserve(tx, &reservation(id, revision)))
    }
    #[test]
    fn strict_budget_wire_requires_explicit_limit_and_exact_amounts() {
        let valid = intent(0, "first", None, 9_007_199_254_740_993);
        let raw = serde_json::to_vec(&valid).unwrap();
        assert_eq!(decode(&raw).unwrap(), valid);
        for mutate in [0, 1, 2, 3, 4, 5] {
            let mut raw = serde_json::to_value(&valid).unwrap();
            match mutate {
                0 => {
                    raw["policy"]
                        .as_object_mut()
                        .unwrap()
                        .remove("limit_micro_usd");
                }
                1 => raw["policy"]["reserve_per_call_micro_usd"] = serde_json::json!(4),
                2 => raw["policy"]["reserve_per_call_micro_usd"] = serde_json::json!("0"),
                3 => raw["policy"]["surprise"] = serde_json::json!(true),
                4 => raw["expected_revision"] = serde_json::json!("01"),
                _ => raw["policy"]["estimate_source"] = serde_json::json!(" "),
            };
            assert!(
                decode(&serde_json::to_vec(&raw).unwrap()).is_err(),
                "invalid mutation {mutate}"
            )
        }
        assert!(decode(br#"{"schema_version":1,"schema_version":1}"#).is_err());
    }
    #[test]
    fn budget_history_replay_conflict_and_reopen_preserve_exact_policy() {
        let mut db = db();
        assert!(!read(&db).unwrap().configured);
        let first = preview(&db, intent(0, "first", Some(9_007_199_254_740_993), 7));
        let saved = publish(&mut db, &first).unwrap();
        assert!(!saved.replayed);
        assert_eq!(saved.document.revision.0, 1);
        assert_eq!(
            saved.document.policy.unwrap().limit_micro_usd.unwrap().0,
            9_007_199_254_740_993
        );
        let second = preview(&db, intent(1, "second", Some(0), 10));
        publish(&mut db, &second).unwrap();
        let replay = publish(&mut db, &first).unwrap();
        assert!(replay.replayed);
        assert_eq!(replay.document.revision.0, 2);
        assert_eq!(
            replay.document.policy.unwrap().limit_micro_usd.unwrap().0,
            0
        );
        let before = read(&db).unwrap();
        let mut wrong = first.clone();
        wrong.reason.push('x');
        assert_eq!(
            publish(&mut db, &wrong).unwrap_err().code(),
            "admin-budget-event-conflict"
        );
        assert_eq!(read(&db).unwrap(), before);
        assert_eq!(before.history.len(), 2);
        assert!(db.is_autocommit());
    }
    #[test]
    fn budget_admission_exhaustion_changes_and_replay_never_reset_consumption() {
        let mut db = db();
        assert_eq!(
            charge(&mut db, "absent", 0).unwrap_err().code(),
            "admin-budget-not-configured"
        );
        let first = preview(&db, intent(0, "first", Some(10), 6));
        publish(&mut db, &first).unwrap();
        let valid = charge(&mut db, "one", 1).unwrap();
        assert_eq!(valid.reserved_estimate_micro_usd.0, 6);
        assert!(!valid.execution_authorized);
        let before = read(&db).unwrap();
        assert_eq!(
            charge(&mut db, "two", 1).unwrap_err().code(),
            "admin-budget-exhausted"
        );
        assert_eq!(read(&db).unwrap(), before);
        let lower = preview(&db, intent(1, "lower", Some(0), 9));
        publish(&mut db, &lower).unwrap();
        let old = charge(&mut db, "one", 1).unwrap();
        assert!(old.replayed);
        assert_eq!(old.reserved_estimate_micro_usd.0, 6);
        assert_eq!(
            charge(&mut db, "two", 2).unwrap_err().code(),
            "admin-budget-exhausted"
        );
        let unlimited = preview(&db, intent(2, "unlimited", None, 9));
        publish(&mut db, &unlimited).unwrap();
        charge(&mut db, "two", 3).unwrap();
        let state = read(&db).unwrap();
        assert_eq!(state.reserved_estimate_micro_usd.0, 15);
        assert!(!state.actual_billed_cost_known);
        let mut mismatch = reservation("two", 3);
        mismatch.request_sha256 = "b".repeat(64);
        assert_eq!(
            crate::agent_execution_store::transaction(&mut db, |tx| reserve(tx, &mismatch))
                .unwrap_err()
                .code(),
            "admin-budget-reservation-conflict"
        );
        assert_eq!(read(&db).unwrap(), state);
    }
    #[test]
    fn budget_stale_consumption_and_sql_failure_refuse_without_partial_publication() {
        let mut db = db();
        let first = preview(&db, intent(0, "first", Some(100), 10));
        publish(&mut db, &first).unwrap();
        let stale = preview(&db, intent(1, "next", Some(200), 20));
        charge(&mut db, "call", 1).unwrap();
        let before = read(&db).unwrap();
        assert_eq!(
            publish(&mut db, &stale).unwrap_err().code(),
            "admin-budget-preview-stale"
        );
        assert_eq!(read(&db).unwrap(), before);
        let fresh = preview(&db, stale);
        db.execute_batch("PRAGMA query_only=ON").unwrap();
        assert!(publish(&mut db, &fresh).is_err());
        assert!(db.is_autocommit());
        assert_eq!(read(&db).unwrap(), before);
        db.execute_batch("PRAGMA query_only=OFF").unwrap();
        publish(&mut db, &fresh).unwrap();
        assert_eq!(read(&db).unwrap().revision.0, 2);
    }
    #[test]
    fn budget_schema_tampering_and_overflow_never_become_empty_success() {
        let mut db = db();
        let first = preview(&db, intent(0, "first", None, i64::MAX));
        publish(&mut db, &first).unwrap();
        charge(&mut db, "one", 1).unwrap();
        assert_eq!(
            charge(&mut db, "two", 1).unwrap_err().code(),
            "admin-budget-consumption-representation"
        );
        let before = read(&db).unwrap();
        db.execute(
            "UPDATE admin_advice_budget_history SET reason=? WHERE revision=1",
            ["Corrupted but nonempty reason"],
        )
        .unwrap();
        assert_eq!(
            read(&db).unwrap_err().code(),
            "admin-budget-history-checksum-invalid"
        );
        db.execute(
            "UPDATE admin_advice_budget_history SET reason=? WHERE revision=1",
            ["Owned budget fixture"],
        )
        .unwrap();
        assert_eq!(read(&db).unwrap(), before);
        db.execute(
            "UPDATE admin_advice_budget_reservations SET request_sha256=? WHERE call_id='one'",
            ["b".repeat(64)],
        )
        .unwrap();
        assert_eq!(
            read(&db).unwrap_err().code(),
            "admin-budget-reservation-checksum-invalid"
        );
        db.execute(
            "UPDATE admin_advice_budget_reservations SET request_sha256=? WHERE call_id='one'",
            ["a".repeat(64)],
        )
        .unwrap();
        assert_eq!(read(&db).unwrap(), before);
        db.execute_batch("CREATE TRIGGER unexpected_admin AFTER INSERT ON admin_advice_budget_reservations BEGIN SELECT 1; END").unwrap();
        assert_eq!(read(&db).unwrap_err().code(), "admin-budget-schema-invalid");
        db.execute_batch("DROP TRIGGER unexpected_admin").unwrap();
        assert_eq!(read(&db).unwrap(), before);
        db.execute_batch("DROP TABLE admin_advice_budget_reservations")
            .unwrap();
        assert_eq!(read(&db).unwrap_err().code(), "admin-budget-schema-invalid");
    }
    #[test]
    fn budget_known_and_uncertain_publications_keep_typed_causes() {
        let error = E::Commit {
            cause: Box::new(E::Refused("fixture-commit-original")),
            rollback_confirmed: false,
            cleanup: None,
        };
        let marked = root(error);
        assert!(marked.configuration_may_be_recorded());
        assert_eq!(
            marked.source().unwrap().to_string(),
            "agent-launch-commit-uncertain"
        );
        let error = publication::<()>(
            Err(R::Cleanup {
                primary: Some(Box::new(marked)),
                cleanup: Box::new(R::Refused("fixture-release-original")),
            }),
            false,
        )
        .unwrap_err();
        assert!(error.configuration_may_be_recorded());
        let error =
            publication::<()>(Err(R::Refused("fixture-release-original")), true).unwrap_err();
        assert!(error.configuration_may_be_recorded());
        assert_eq!(
            error.source().unwrap().to_string(),
            "fixture-release-original"
        );
    }
}
