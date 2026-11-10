//! Read-only SQL administration of future-launch overrides. No attempt changes.
use super::{finished, RootStore, RootStoreError as E};
use crate::agent_launch::query;
use crate::{
    AgentLaunchError as A, EngineSettingsDocument, RunLimits, RunLimitsConfigEntry as Entry,
    RunLimitsHistoryEntry as History,
};
use rusqlite::{Connection, Row};
use serde::Serialize;

/// Exact protocol field order, not operator bounds or defaults.
pub(super) const LIMIT_FIELDS: [&str; 5] = [
    "silence_seconds",
    "tool_seconds",
    "max_tool_calls",
    "max_repeated_calls",
    "max_consecutive_errors",
];
/// One saved nullable definition ceiling; zero in an override always inherits.
#[derive(Serialize)]
pub struct RunLimitsFieldBound {
    pub name: &'static str,
    pub unit: &'static str,
    pub max: Option<i64>,
}
/// Checked data view with independently retained policy diagnostics. Entry/history
/// are recorded metadata, not proof of authenticated actor or execution readiness.
#[derive(Serialize)]
pub struct RunLimitsAdminView {
    pub scope: String,
    pub mission_id: String,
    pub scope_key: String,
    pub source: &'static str,
    pub entry: Entry,
    pub history: Option<Vec<History>>,
    pub bounds: Option<Vec<RunLimitsFieldBound>>,
    pub defaults: Option<RunLimits>,
    pub settings_revision: String,
    pub issues: Vec<&'static str>,
    pub exact: serde_json::Value,
    #[serde(skip)]
    pub observation_causes: Vec<E>,
}
/// Physical current-row presence/body are kept for future freshness bindings.
pub(super) struct Current {
    pub entry: Entry,
    pub raw: Option<Vec<u8>>,
}
/// Preserve original decoder/SQL/cleanup causes; never format private data.
pub(super) fn root(error: A) -> E {
    E::Launch(Box::new(error))
}
/// Scope names/field combinations are wire syntax, not existence/role authority.
pub(super) fn scope(scope: &str, mission: &str, key: &str) -> Result<(), E> {
    crate::run_limits_admin_codec::valid_run_limits_scope(scope, mission, key).map_err(root)
}
/// Go scans body bytes from TEXT/BLOB. Unsupported kinds remain source failures.
pub(super) fn bytes(row: &Row<'_>, index: usize) -> Result<Vec<u8>, A> {
    match row.get_ref(index)? {
        rusqlite::types::ValueRef::Text(v) | rusqlite::types::ValueRef::Blob(v) => Ok(v.to_vec()),
        _ => Err(A::Refused("run-limits-source-body-kind")),
    }
}
/// Current Go reader does not rejudge historical actor/reason/values under new
/// policy. Unknown JSON value fields are ignored like json.Unmarshal; null/case/
/// duplicate semantics use the common Go projection. Absence is an actual row miss.
pub(super) fn current(db: &Connection, sc: &str, mission: &str, key: &str) -> Result<Current, A> {
    query(db,"SELECT revision,body,actor,reason,updated FROM main.run_limits_config WHERE scope=? AND mission_id=? AND scope_key=?",[sc,mission,key],|rows|{
        let mut entry=Entry{scope:sc.into(),mission_id:mission.into(),scope_key:key.into(),..Default::default()};
        let Some(row)=rows.next()? else{return Ok(Current{entry,raw:None})};
        let raw=bytes(row,1)?;entry.revision=row.get(0)?;entry.actor=row.get(2)?;entry.reason=row.get(3)?;entry.updated=row.get(4)?;
        entry.values=crate::work::decode_mode(&raw,crate::models::shape_run_limits(),false).map_err(|e|A::caused("run-limits-source-values-invalid",e))?;
        Ok(Current{entry,raw:Some(raw)})
    })
}
/// Append-only history in Go's descending revision order. No current-policy,
/// metadata authority or row-count gate. Empty Go slice remains JSON null.
/// Statement finalization and original row/decoder failures are retained.
pub(super) fn history(
    db: &Connection,
    sc: &str,
    mission: &str,
    key: &str,
) -> Result<Option<Vec<History>>, A> {
    query(db,"SELECT revision,body,actor,reason,at,coalesce(rollback_of,0) FROM main.run_limits_history WHERE scope=? AND mission_id=? AND scope_key=? ORDER BY revision DESC",[sc,mission,key],|rows|{
        let mut result=Vec::new();
        while let Some(row)=rows.next()?{
            let raw=bytes(row,1)?;
            let values=crate::work::decode_mode(&raw,crate::models::shape_run_limits(),false).map_err(|e|A::caused("run-limits-history-values-invalid",e))?;
            result.try_reserve(1).map_err(|e|A::caused("run-limits-history-allocation",e))?;
            result.push(History{scope:sc.into(),mission_id:mission.into(),scope_key:key.into(),revision:row.get(0)?,values,actor:row.get(2)?,reason:row.get(3)?,at:row.get(4)?,rollback_of:row.get(5)?});
        }
        Ok(if result.is_empty(){None}else{Some(result)})
    })
}
/// Exact decimal read projection for browser fields; no float, fallback, extra
/// source or authorization claim. Text/null/list structure remain unchanged.
pub(super) fn exact(value: &impl Serialize) -> Result<serde_json::Value, E> {
    fn convert(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Number(n) => *value = serde_json::Value::String(n.to_string()),
            serde_json::Value::Array(v) => v.iter_mut().for_each(convert),
            serde_json::Value::Object(v) => {
                v.values_mut().for_each(convert);
                if LIMIT_FIELDS.iter().all(|key| v.contains_key(*key)) {
                    v.entry("observation_mode")
                        .or_insert_with(|| serde_json::Value::String("0".to_owned()));
                }
            }
            _ => (),
        }
    }
    let mut value = serde_json::to_value(value)
        .map_err(|e| root(A::caused("run-limits-projection-invalid", e)))?;
    convert(&mut value);
    Ok(value)
}
impl RootStore {
    /// Read settings/current/history in one same-connection deferred snapshot
    /// under shared settings custody. Explicit rollback/close retain both causes.
    /// No data/schema adoption, provider call, timer or frozen-attempt mutation.
    pub(super) fn with_run_limits_read<T>(
        &self,
        read: impl FnOnce(&Connection, &EngineSettingsDocument) -> Result<T, E>,
    ) -> Result<T, E> {
        let lock = self.directory.reserve(false, false).map_err(E::Storage)?;
        let result = (|| {
            let settings = self.read_settings()?;
            let tx = self.db.unchecked_transaction().map_err(E::Sql)?;
            let result = read(&tx, &settings);
            finished(result, tx.rollback().map_err(E::Sql))
        })();
        finished(result, lock.close().map_err(E::Storage))
    }
    /// Read one current scope, preserving Go defaults only for an absent row.
    /// Syntax is validated; no mission-existence, actor or new-policy gate. The
    /// values are not provider-specific launch permission or frozen state.
    pub fn run_limits_entry(&self, sc: &str, mission: &str, key: &str) -> Result<Entry, E> {
        scope(sc, mission, key)?;
        self.with_run_limits_read(|db, _| {
            current(db, sc, mission, key).map(|s| s.entry).map_err(root)
        })
    }
    /// Read all retained scope revisions; empty is None/Go null, never a failed
    /// query fallback. Original metadata is unverified; no history mutation.
    pub fn run_limits_history(
        &self,
        sc: &str,
        mission: &str,
        key: &str,
    ) -> Result<Option<Vec<History>>, E> {
        scope(sc, mission, key)?;
        self.with_run_limits_read(|db, _| history(db, sc, mission, key).map_err(root))
    }
    /// Inspect recorded current/history plus actual saved bounds. Missing or
    /// incompatible policy becomes an explicit degraded observation with original
    /// causes, not substituted bounds or an empty config. Write/launch still refuse.
    pub fn run_limits_view(
        &self,
        sc: &str,
        mission: &str,
        key: &str,
    ) -> Result<RunLimitsAdminView, E> {
        scope(sc, mission, key)?;
        self.with_run_limits_read(|db,settings|{
            let state=current(db,sc,mission,key).map_err(root)?;let history=history(db,sc,mission,key).map_err(root)?;
            let mut issues=Vec::new();let mut causes=Vec::new();
            let (bounds,defaults)=match settings.values.runtime_limit_policy(){
                Ok(policy)=>{
                    if let Err(e)=policy.validate_override(&state.entry.values){issues.push(e.code());causes.push(E::RunLimits(e));}
                    let bounds=LIMIT_FIELDS.iter().copied().zip(policy.definition_ceilings()).enumerate().map(|(index,(name,max))|RunLimitsFieldBound{name,max,unit:if index<2{"seconds"}else{"count"}}).collect();
                    (Some(bounds),Some(policy.normalize(&RunLimits::default()).map_err(E::RunLimits)?))
                }
                Err(e)=>{issues.push(e.code());causes.push(E::Settings(e));(None,None)}
            };
            let exact=exact(&serde_json::json!({"entry":&state.entry,"history":&history,"bounds":&bounds,"defaults":&defaults}))?;
            Ok(RunLimitsAdminView{scope:sc.into(),mission_id:mission.into(),scope_key:key.into(),source:if state.raw.is_some(){"stored_sql"}else{"absent"},entry:state.entry,history,bounds,defaults,settings_revision:settings.revision.0.to_string(),issues,exact,observation_causes:causes})
        })
    }
    /// Resolve administrative hierarchy under current saved defaults/ceilings,
    /// not a provider-specific or already-frozen attempt. Use agent preview for
    /// actual launch admission/provider/mission/previous-attempt constraints.
    /// Invalid tuple, missing policy or incompatible stored overrides refuse.
    pub fn run_limits_effective(
        &self,
        mission: &str,
        role: &str,
        task: &str,
    ) -> Result<RunLimits, E> {
        if (mission.is_empty() && (!role.is_empty() || !task.is_empty()))
            || [mission, role, task]
                .iter()
                .any(|v| !v.is_empty() && !swarm_identity_contract::name(v.as_bytes()))
        {
            return Err(E::Refused("run-limits-effective-scope-invalid"));
        }
        self.with_run_limits_read(|db, settings| {
            let policy = settings
                .values
                .runtime_limit_policy()
                .map_err(E::Settings)?;
            let admin = crate::run_limits::read_configured_runtime_limits(
                db, &policy, mission, role, task,
            )?;
            policy.normalize(&admin).map_err(E::RunLimits)
        })
    }
}
