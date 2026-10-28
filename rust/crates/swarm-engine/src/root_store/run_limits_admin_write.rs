//! Append-only scope overrides using the existing saved storage retry/transaction.
//! No Agent/Work/budget update, provider process or business completion.
use super::run_limits_admin_read::{bytes, current, exact, history};
use super::{RootStore, RootStoreError as E};
use crate::agent_launch::query;
use crate::{
    AgentLaunchError as A, ApiRunLimitsConfigChange as Request, EngineSettingsDocument,
    RunLimitsConfigChange as Change, RunLimitsConfigEntry as Entry, WorkEnvironment,
};
use rusqlite::{params, Connection};
use serde::Serialize;

/// Projection of preview/known apply. Replayed entry is CURRENT, not an old
/// rolled-back value. Token never enters history.request; it is not authority.
#[derive(Serialize)]
pub struct RunLimitsMutation {
    pub preview: bool,
    pub applied: bool,
    pub replayed: bool,
    pub revision_ok: bool,
    pub entry: Entry,
    pub current: Option<Entry>,
    pub proposed: crate::RunLimits,
    pub preview_token: String,
    pub exact: serde_json::Value,
}
struct Proposal {
    frame: RunLimitsMutation,
    change: Change,
    body: Vec<u8>,
    request: Vec<u8>,
}
/// Preserve an uncertain SQL commit before settings cleanup/retry traversal.
/// Confirmed rollback stays refused without a false publication marker.
fn root(error: A) -> E {
    let uncertain = matches!(
        &error,
        A::Commit {
            rollback_confirmed: false,
            ..
        }
    );
    let error = E::Launch(Box::new(error));
    if uncertain {
        E::Published {
            primary: Box::new(error),
        }
    } else {
        error
    }
}
/// Read ONLY opaque request bytes for replay, like Go. No new history actor,
/// revision or JSON validation gate; query/finalization failures retain causes.
fn recorded(db: &Connection, r: &Change) -> Result<Option<Vec<u8>>, A> {
    query(db,"SELECT request FROM main.run_limits_history WHERE scope=? AND mission_id=? AND scope_key=? AND event_id=?",params![r.scope,r.mission,r.key,r.event_id],|rows|{
        rows.next()?.map(|row|bytes(row,0)).transpose()
    })
}
/// Build a current-source/settings/instance/intent binding. Rollback selects Go's
/// first matching descending historical row and records it as a NEW change.
/// Positive current policy applies only to new requests; exact replay is opaque.
fn propose(
    db: &Connection,
    settings: &EngineSettingsDocument,
    instance: &impl Serialize,
    r: &Request,
    target: Option<i64>,
    informational: bool,
) -> Result<Proposal, A> {
    let mut change = r.original_change();
    crate::run_limits_admin_codec::valid_run_limits_scope(
        &change.scope,
        &change.mission,
        &change.key,
    )?;
    let target = if let Some(revision) = target {
        let rows = history(db, &change.scope, &change.mission, &change.key)?;
        let row = rows
            .unwrap_or_default()
            .into_iter()
            .find(|row| row.revision == revision)
            .ok_or(A::Refused("run-limits-history-revision-not-found"))?;
        change.values = row.values.clone();
        change.rollback_of = revision;
        if change.reason.is_empty() {
            change.reason = format!("retour à la révision {revision}");
        }
        Some(row)
    } else {
        None
    };
    if !informational {
        crate::run_limits_admin_codec::validate_change_header(&change)?;
    }
    let state = current(db, &change.scope, &change.mission, &change.key)?;
    let request = crate::run_limits_admin_codec::canonical_request_bytes(&change)?;
    let replayed = if !informational {
        if let Some(recorded) = recorded(db, &change)? {
            crate::run_limits_admin_codec::verify_historical_replay(&recorded, &change)?;
            true
        } else {
            false
        }
    } else {
        false
    };
    if !replayed {
        let policy = settings
            .values
            .runtime_limit_policy()
            .map_err(|e| A::caused(e.code(), e))?;
        if informational {
            policy
                .validate_override(&change.values)
                .map_err(|e| A::caused(e.code(), e))?;
        } else {
            crate::run_limits_admin_codec::validate_new_change(&change, &policy)?;
        }
    }
    let mut entry = state.entry.clone();
    if !replayed {
        entry.revision = entry
            .revision
            .checked_add(1)
            .filter(|n| *n > 0)
            .ok_or(A::Refused("run-limits-next-revision-invalid"))?;
        entry.values = change.values.clone();
        entry.actor.clear();
        entry.updated.clear();
        entry.reason = change.reason.clone();
    }
    let body =
        crate::encode_model(&entry.values).map_err(|e| A::caused("run-limits-values-encode", e))?;
    let binding=crate::encode_model(&serde_json::json!({"schema_version":1,"instance":instance,"settings_revision":settings.revision,"settings":settings.values,"current":state.entry,"raw":state.raw,"request":request,"rollback_target":target})).map_err(|e|A::caused("run-limits-binding-encode",e))?;
    let token = if informational {
        String::new()
    } else {
        crate::provider_configuration::configuration_digest(&binding)
    };
    let revision_ok = state.entry.revision == change.revision;
    let exact =
        exact(&serde_json::json!({"entry":&entry,"current":&state.entry,"proposed":&entry.values}))
            .map_err(|e| A::caused("run-limits-projection-invalid", e))?;
    Ok(Proposal {
        frame: RunLimitsMutation {
            preview: true,
            applied: false,
            replayed,
            revision_ok,
            proposed: entry.values.clone(),
            entry,
            current: Some(state.entry),
            preview_token: token,
            exact,
        },
        change,
        body,
        request,
    })
}
/// Preserve Go's explicit revision-CAS write/recovery path. New writes do not
/// decode the old current body/metadata: Go can replace an unreadable value.
/// No false zero-before image is supplied (current=None). A supplied native
/// preview token uses the stronger inspected snapshot path instead.
fn propose_legacy_apply(
    db: &Connection,
    settings: &EngineSettingsDocument,
    r: &Request,
    target: Option<i64>,
) -> Result<Proposal, A> {
    let mut change = r.original_change();
    crate::run_limits_admin_codec::valid_run_limits_scope(
        &change.scope,
        &change.mission,
        &change.key,
    )?;
    if let Some(revision) = target {
        let rows = history(db, &change.scope, &change.mission, &change.key)?;
        let row = rows
            .unwrap_or_default()
            .into_iter()
            .find(|row| row.revision == revision)
            .ok_or(A::Refused("run-limits-history-revision-not-found"))?;
        change.values = row.values;
        change.rollback_of = revision;
        if change.reason.is_empty() {
            change.reason = format!("retour à la révision {revision}");
        }
    }
    crate::run_limits_admin_codec::validate_change_header(&change)?;
    let request = crate::run_limits_admin_codec::canonical_request_bytes(&change)?;
    if let Some(old) = recorded(db, &change)? {
        crate::run_limits_admin_codec::verify_historical_replay(&old, &change)?;
        let state = current(db, &change.scope, &change.mission, &change.key)?;
        let exact=exact(&serde_json::json!({"entry":&state.entry,"current":&state.entry,"proposed":&state.entry.values})).map_err(|e|A::caused("run-limits-projection-invalid",e))?;
        let body = crate::encode_model(&state.entry.values)
            .map_err(|e| A::caused("run-limits-values-encode", e))?;
        return Ok(Proposal {
            frame: RunLimitsMutation {
                preview: true,
                applied: false,
                replayed: true,
                revision_ok: state.entry.revision == change.revision,
                proposed: state.entry.values.clone(),
                entry: state.entry.clone(),
                current: Some(state.entry),
                preview_token: String::new(),
                exact,
            },
            change,
            body,
            request,
        });
    }
    let policy = settings
        .values
        .runtime_limit_policy()
        .map_err(|e| A::caused(e.code(), e))?;
    crate::run_limits_admin_codec::validate_new_change(&change, &policy)?;
    let revision=query(db,"SELECT revision FROM main.run_limits_config WHERE scope=? AND mission_id=? AND scope_key=?",params![change.scope,change.mission,change.key],|rows|{
        Ok(match rows.next()?{Some(row)=>row.get::<_,i64>(0)?,None=>0})
    })?;
    if revision != change.revision {
        return Err(A::Refused("revision_conflict"));
    }
    let next = revision
        .checked_add(1)
        .filter(|n| *n > 0)
        .ok_or(A::Refused("run-limits-next-revision-invalid"))?;
    let entry = Entry {
        scope: change.scope.clone(),
        mission_id: change.mission.clone(),
        scope_key: change.key.clone(),
        values: change.values.clone(),
        revision: next,
        actor: String::new(),
        reason: change.reason.clone(),
        updated: String::new(),
    };
    let body =
        crate::encode_model(&entry.values).map_err(|e| A::caused("run-limits-values-encode", e))?;
    let exact=exact(&serde_json::json!({"entry":&entry,"current":serde_json::Value::Null,"proposed":&entry.values})).map_err(|e|A::caused("run-limits-projection-invalid",e))?;
    Ok(Proposal {
        frame: RunLimitsMutation {
            preview: true,
            applied: false,
            replayed: false,
            revision_ok: true,
            proposed: entry.values.clone(),
            entry,
            current: None,
            preview_token: String::new(),
            exact,
        },
        change,
        body,
        request,
    })
}

/// Stage both rows and verify the actual bytes/metadata. An ignored/reverted
/// write or changed request refuses and rolls back; no false successful no-op.
fn stage(db: &Connection, p: &mut Proposal, actor: &str, at: &str) -> Result<(), A> {
    let e = &mut p.frame.entry;
    e.actor = actor.into();
    e.updated = at.into();
    let n=db.execute("INSERT INTO main.run_limits_config(scope,mission_id,scope_key,revision,body,actor,reason,updated) VALUES(?,?,?,?,?,?,?,?) ON CONFLICT(scope,mission_id,scope_key) DO UPDATE SET revision=excluded.revision,body=excluded.body,actor=excluded.actor,reason=excluded.reason,updated=excluded.updated",params![e.scope,e.mission_id,e.scope_key,e.revision,p.body,e.actor,e.reason,e.updated])?;
    if n != 1 {
        return Err(A::Refused("run-limits-write-not-recorded"));
    }
    let rollback = if p.change.rollback_of == 0 {
        None
    } else {
        Some(p.change.rollback_of)
    };
    let n=db.execute("INSERT INTO main.run_limits_history(scope,mission_id,scope_key,revision,body,request,actor,reason,at,event_id,rollback_of) VALUES(?,?,?,?,?,?,?,?,?,?,?)",params![e.scope,e.mission_id,e.scope_key,e.revision,p.body,p.request,e.actor,e.reason,e.updated,p.change.event_id,rollback])?;
    if n != 1 {
        return Err(A::Refused("run-limits-history-not-recorded"));
    }
    let actual = current(db, &e.scope, &e.mission_id, &e.scope_key)?;
    if actual.entry != *e || actual.raw.as_deref() != Some(p.body.as_slice()) {
        return Err(A::Refused("run-limits-write-not-preserved"));
    }
    let valid=query(db,"SELECT revision,body,request,actor,reason,at,rollback_of FROM main.run_limits_history WHERE scope=? AND mission_id=? AND scope_key=? AND event_id=?",params![e.scope,e.mission_id,e.scope_key,p.change.event_id],|rows|{
        let Some(row)=rows.next()? else{return Ok(false)};
        Ok(row.get::<_,i64>(0)?==e.revision && bytes(row,1)?==p.body && bytes(row,2)?==p.request && row.get::<_,String>(3)?==e.actor && row.get::<_,String>(4)?==e.reason && row.get::<_,String>(5)?==e.updated && row.get::<_,Option<i64>>(6)?==rollback)
    })?;
    if !valid {
        return Err(A::Refused("run-limits-history-not-preserved"));
    }
    p.frame.exact=exact(&serde_json::json!({"entry":&p.frame.entry,"current":&p.frame.current,"proposed":&p.frame.proposed})).map_err(|e|A::caused("run-limits-projection-invalid",e))?;
    Ok(())
}
impl RootStore {
    /// Preview current scope/values without SQL mutation. Full native intent gets
    /// a request/settings/instance token. Legacy Go informational previews with
    /// no event id never authorize later application; stale revision is explicit.
    pub fn preview_run_limits(&self, r: &Request) -> Result<RunLimitsMutation, E> {
        self.preview_run_limits_using(r, None)
    }
    /// Preview a historical rollback as a NEW append-only revision. Target data
    /// is read under the same SQL/settings snapshot; old rows remain untouched.
    pub fn preview_run_limits_rollback(
        &self,
        r: &Request,
        to_revision: i64,
    ) -> Result<RunLimitsMutation, E> {
        self.preview_run_limits_using(r, Some(to_revision))
    }
    /// Common read-only preview; no actor/time observation, schema or file write.
    fn preview_run_limits_using(
        &self,
        r: &Request,
        target: Option<i64>,
    ) -> Result<RunLimitsMutation, E> {
        let instance = self.instance()?;
        self.with_run_limits_read(|db, settings| {
            propose(db, settings, &instance, r, target, r.event_id.is_empty())
                .map(|p| p.frame)
                .map_err(root)
        })
    }
    /// Apply or acknowledge exact replay under saved storage retry and a single
    /// same-connection transaction. Explicit revision-CAS preserves Go recovery;
    /// a supplied token additionally binds the inspected source and saved policy.
    /// Existing limits frozen in Agent metadata/counters are never modified.
    /// Actor/time are local observations, not authenticated caller identities.
    /// Commit uncertainty and later settings-close/output failure retain state.
    pub fn apply_run_limits<W: WorkEnvironment>(
        &mut self,
        r: &Request,
        environment: &mut W,
    ) -> Result<RunLimitsMutation, E> {
        self.apply_run_limits_using(r, None, environment)
    }
    /// Restore retained values by appending a new revision, preserving all
    /// intermediate history. Replayed rollback returns current scope state.
    pub fn rollback_run_limits<W: WorkEnvironment>(
        &mut self,
        r: &Request,
        to_revision: i64,
        environment: &mut W,
    ) -> Result<RunLimitsMutation, E> {
        self.apply_run_limits_using(r, Some(to_revision), environment)
    }
    /// Mutation kernel. No independent DB handle, live attempt or provider IO.
    fn apply_run_limits_using<W: WorkEnvironment>(
        &mut self,
        r: &Request,
        target: Option<i64>,
        environment: &mut W,
    ) -> Result<RunLimitsMutation, E> {
        if !self.writable {
            return Err(E::Refused("root-read-only"));
        }
        let instance = self.instance()?;
        let mut known = false;
        let result = crate::agent_storage_retry::execute(
            &mut self.db,
            &self.directory,
            false,
            |db, settings| {
                let result = crate::agent_execution_store::transaction(db, |tx| {
                    let mut p = if r.preview_token().is_empty() {
                        propose_legacy_apply(tx, settings, r, target)?
                    } else {
                        propose(tx, settings, &instance, r, target, false)?
                    };
                    if p.frame.replayed {
                        known = true;
                    } else {
                        if !p.frame.revision_ok {
                            return Err(A::Refused("revision_conflict"));
                        }
                        if !r.preview_token().is_empty()
                            && r.preview_token() != p.frame.preview_token
                        {
                            return Err(A::Refused("run-limits-preview-required"));
                        }
                        let actor = environment
                            .operator()
                            .map_err(|e| A::caused("run-limits-actor-observation", e))?;
                        let at = environment
                            .timestamp()
                            .map_err(|e| A::caused("run-limits-time-observation", e))?;
                        stage(tx, &mut p, &actor, &at)?;
                    }
                    p.frame.preview = false;
                    p.frame.applied = true;
                    Ok(p.frame)
                })
                .map_err(root);
                if result.is_ok() {
                    known = true;
                }
                result
            },
        );
        result.map_err(|error| {
            if known {
                E::Published {
                    primary: Box::new(error),
                }
            } else {
                error
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Actual Go accepts explicit revision-CAS apply without a token and can
    /// repair a malformed current body. Native must not invent a read prerequisite.
    #[test]
    fn legacy_cas_apply_can_repair_unreadable_current_values() {
        use std::{fs, os::unix::fs::PermissionsExt};
        let path =
            std::env::temp_dir().join(crate::generate_identifier("run-limits-repair-").unwrap());
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        let mut environment = crate::SystemEnvironment;
        let mut store = RootStore::initialize(&path, &mut environment).unwrap();
        let request=crate::decode_api_run_limits_config_change(br#"{"schema_version":1,"event_id":"valid-cas","scope":"project","expected_revision":0,"values":{},"reason":"Owned native control"}"#).unwrap();
        let first = store.apply_run_limits(&request, &mut environment).unwrap();
        assert_eq!(first.entry.revision, 1);
        assert_eq!(
            store.run_limits_entry("project", "", "").unwrap().revision,
            1
        );
        store
            .db
            .execute(
                "UPDATE run_limits_config SET body=? WHERE scope='project'",
                [b"not-json".as_slice()],
            )
            .unwrap();
        assert!(store.run_limits_entry("project", "", "").is_err());
        let request=crate::decode_api_run_limits_config_change(br#"{"schema_version":1,"event_id":"repair-cas","scope":"project","expected_revision":1,"values":{"silence_seconds":2},"reason":"Owned native recovery"}"#).unwrap();
        let second = store.apply_run_limits(&request, &mut environment).unwrap();
        assert_eq!(second.entry.revision, 2);
        assert_eq!(second.entry.values.silence_seconds, 2);
        assert_eq!(
            store
                .run_limits_history("project", "", "")
                .unwrap()
                .unwrap()
                .len(),
            2
        );
        store.close().unwrap();
        fs::remove_dir_all(path).unwrap();
    }
    /// A valid private control precedes a trigger that silently drops history.
    /// The shared transaction must reject readback and undo the current-row write.
    #[test]
    fn ignored_history_insert_rolls_back_current_and_recovers() {
        use std::{fs, os::unix::fs::PermissionsExt};
        let path =
            std::env::temp_dir().join(crate::generate_identifier("run-limits-atomic-").unwrap());
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        let mut env = crate::SystemEnvironment;
        let mut store = RootStore::initialize(&path, &mut env).unwrap();
        let one=crate::decode_api_run_limits_config_change(br#"{"schema_version":1,"event_id":"control","scope":"project","expected_revision":0,"values":{"silence_seconds":2},"reason":"Valid control"}"#).unwrap();
        assert_eq!(
            store
                .apply_run_limits(&one, &mut env)
                .unwrap()
                .entry
                .revision,
            1
        );
        assert_eq!(
            store
                .run_limits_history("project", "", "")
                .unwrap()
                .unwrap()
                .len(),
            1
        );
        store.db.execute_batch("CREATE TEMP TRIGGER drop_owned_history BEFORE INSERT ON main.run_limits_history BEGIN SELECT RAISE(IGNORE); END;").unwrap();
        let two=crate::decode_api_run_limits_config_change(br#"{"schema_version":1,"event_id":"second","scope":"project","expected_revision":1,"values":{"silence_seconds":7},"reason":"No silent success"}"#).unwrap();
        assert!(store.apply_run_limits(&two, &mut env).is_err());
        assert_eq!(
            store.run_limits_entry("project", "", "").unwrap().revision,
            1
        );
        assert_eq!(
            store
                .run_limits_entry("project", "", "")
                .unwrap()
                .values
                .silence_seconds,
            2
        );
        assert_eq!(
            store
                .run_limits_history("project", "", "")
                .unwrap()
                .unwrap()
                .len(),
            1
        );
        store
            .db
            .execute_batch("DROP TRIGGER temp.drop_owned_history;")
            .unwrap();
        assert_eq!(
            store
                .apply_run_limits(&two, &mut env)
                .unwrap()
                .entry
                .revision,
            2
        );
        store.close().unwrap();
        fs::remove_dir_all(path).unwrap();
    }
}
