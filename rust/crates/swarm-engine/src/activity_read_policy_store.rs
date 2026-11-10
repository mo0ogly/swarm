//! Native activity display policy ledger, never agent execution/completion policy.
//! Shared saved configuration bytes and checked private atomic publication.
use super::{absent, finished, RootStore, RootStoreError as E};
use crate::local_storage::{AtomicStorageFault, PublicationState};
use crate::provider_configuration::configuration_digest as hash;
use crate::{
    ActivityReadConfig as Config, ActivityReadRevision, ApiActivityReadChange as Request,
    WorkEnvironment,
};
use serde::Serialize;
const DEFAULTS: &[u8] = include_bytes!("../../../../config/activity-policy.json");
/// JSON/JavaScript exact integer bound; no waiting duration or business cutoff.
const MAX_SAFE_INTEGER: i64 = crate::activity_read_policy::ACTIVITY_MAX_SAFE_ROWS;
/// Current compatible config and actual source/cap. Only config is persisted;
/// decimal projection protects browser fidelity, not authority or identity.
#[derive(Serialize)]
pub struct ActivityReadPolicyView {
    #[serde(flatten)]
    pub config: Config,
    pub source: &'static str,
    pub digest: String,
    pub exact: serde_json::Value,
    pub bounds: ActivityReadPolicyBounds,
    pub configuration_max_bytes: Option<String>,
}
/// Exact representation and nonempty-page/sentinel preconditions. Source0
/// selects no payload rows, never a claim that no history exists.
#[derive(Serialize)]
pub struct ActivityReadPolicyBounds {
    pub min_page_rows: i64,
    pub min_source_rows: i64,
    pub max_safe_integer_rows: i64,
}
/// Preview or known application/replay. Token is absent from persisted Change;
/// preview metadata is an uncommitted observation, real output/close can fail later.
#[derive(Serialize)]
pub struct ActivityReadPolicyMutation {
    pub applied: bool,
    pub preview: bool,
    pub replayed: bool,
    pub config: Config,
    pub exact: serde_json::Value,
    pub preview_token: String,
    pub before_digest: String,
    pub after_digest: Option<String>,
    pub backup_reference: Option<String>,
}
struct State {
    raw: Option<Vec<u8>>,
    view: ActivityReadPolicyView,
}
struct Proposal {
    state: State,
    result: ActivityReadPolicyMutation,
}
/// Preserve concrete encoding/codec errors without exposing private file text.
fn encoded(value: &impl Serialize) -> Result<Vec<u8>, E> {
    crate::encode_model(value).map_err(|e| {
        E::Launch(Box::new(crate::AgentLaunchError::caused(
            "activity-read-policy-encode",
            e,
        )))
    })
}
/// Exact integers as browser decimals, preserving original metadata/structure.
fn exact(value: &impl Serialize) -> Result<serde_json::Value, E> {
    fn convert(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Number(n) => *v = serde_json::Value::String(n.to_string()),
            serde_json::Value::Array(a) => a.iter_mut().for_each(convert),
            serde_json::Value::Object(o) => o.values_mut().for_each(convert),
            _ => (),
        }
    }
    let mut v = serde_json::to_value(value).map_err(|e| {
        E::Launch(Box::new(crate::AgentLaunchError::caused(
            "activity-read-policy-projection",
            e,
        )))
    })?;
    convert(&mut v);
    Ok(v)
}
impl RootStore {
    /// Actual saved shared provider/local-policy file cap, not a literal legacy1MiB
    /// fallback. Missing required settings refuse, null explicitly has no cap.
    fn activity_read_byte_cap(&self) -> Result<Option<usize>, E> {
        self.read_settings()?
            .values
            .agent_launch_policy()
            .map(|p| p.provider_bytes)
            .map_err(E::Settings)
    }
    /// Read owned source, or only genuinely absent source's versioned defaults.
    /// Bad history/JSON/ownership and checked file close remain source errors.
    fn activity_read_state(&self) -> Result<State, E> {
        let cap = self.activity_read_byte_cap()?;
        let raw = match self.directory.read_capped("activity-policy.json", cap) {
            Ok(raw) => Some(raw),
            Err(e) if absent(&e) => None,
            Err(e) => return Err(E::Storage(e)),
        };
        let config = (|| {
            if let Some(raw) = raw.as_deref() {
                crate::decode_activity_read_config(raw)
            } else {
                Ok(Config {
                    schema_version: 1,
                    revision: 0,
                    values: crate::decode_activity_read_defaults(DEFAULTS)?,
                    history: Some(vec![]),
                })
            }
        })()
        .map_err(|e| {
            E::Launch(Box::new(crate::AgentLaunchError::caused(
                "activity-read-policy-source-invalid",
                e,
            )))
        })?;
        let view = ActivityReadPolicyView {
            source: if raw.is_some() {
                "stored_file"
            } else {
                "versioned_defaults"
            },
            digest: hash(raw.as_deref().unwrap_or(DEFAULTS)),
            exact: exact(&config)?,
            config,
            bounds: ActivityReadPolicyBounds {
                min_page_rows: 1,
                min_source_rows: 0,
                max_safe_integer_rows: MAX_SAFE_INTEGER,
            },
            configuration_max_bytes: cap.map(|n| n.to_string()),
        };
        Ok(State { raw, view })
    }
    /// Read activity read policy/history under settings custody. No lock
    /// file creation, implicit adoption, model, mission or provider operation.
    pub fn activity_read_policy(&self) -> Result<ActivityReadPolicyView, E> {
        let lock = self.directory.reserve(false, false).map_err(E::Storage)?;
        finished(
            self.activity_read_state().map(|s| s.view),
            lock.close().map_err(E::Storage),
        )
    }
    /// Build current source/settings/instance/request binding. Exact replay uses
    /// the first matching recorded Change and returns CURRENT config, no new row.
    fn activity_read_proposal(&self, r: &Request) -> Result<Proposal, E> {
        let change = r.recorded_change();
        crate::validate_activity_read_change(&change).map_err(|e| E::Launch(Box::new(e)))?;
        let state = self.activity_read_state()?;
        let mut config = state.view.config.clone();
        let replayed = if let Some(row) = config
            .history
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .find(|row| row.change.event_id == change.event_id)
        {
            if row.change != change {
                return Err(E::Refused("activity-read-policy-event-conflict"));
            }
            true
        } else {
            false
        };
        if !replayed {
            if config.revision != r.expected_revision {
                return Err(E::Refused("revision_conflict"));
            }
            config.revision = config
                .revision
                .checked_add(1)
                .ok_or(E::Refused("activity-read-policy-revision-overflow"))?;
            config.values = r.values;
            let mut environment = crate::SystemEnvironment;
            config
                .history
                .get_or_insert_with(Vec::new)
                .push(ActivityReadRevision {
                    revision: config.revision,
                    values: r.values,
                    actor: environment.operator().map_err(E::Runtime)?,
                    at: environment.timestamp().map_err(E::Runtime)?,
                    change: change.clone(),
                });
            let projected = encoded(&config)?;
            if self
                .activity_read_byte_cap()?
                .is_some_and(|n| projected.len() > n)
            {
                return Err(E::Refused("activity-read-policy-file-outside-saved-policy"));
            }
        }
        let settings = self.read_settings()?;
        let binding = encoded(
            &serde_json::json!({"schema_version":1,"source":state.view.source,"digest":state.view.digest,"change":change,"settings_revision":settings.revision,"settings":settings.values,"instance":self.instance()?}),
        )?;
        let result = ActivityReadPolicyMutation {
            applied: false,
            preview: true,
            replayed,
            exact: exact(&config)?,
            config,
            preview_token: hash(&binding),
            before_digest: state.view.digest.clone(),
            after_digest: None,
            backup_reference: None,
        };
        Ok(Proposal { state, result })
    }
    /// Read-only preview without creating the policy lock. Actor/time are local
    /// uncommitted observations included in the full serialized-size check; they
    /// can change before apply and never establish identity or commit.
    pub fn preview_activity_read_policy(&self, r: &Request) -> Result<ActivityReadPolicyMutation, E> {
        let lock = self.directory.reserve(false, false).map_err(E::Storage)?;
        finished(
            self.activity_read_proposal(r).map(|p| p.result),
            lock.close().map_err(E::Storage),
        )
    }
    /// Explicit CAS or stronger supplied native token; no silent policy cutoff.
    /// Settings + policy lock protect checked backup and atomic publication. Exact
    /// replay synchronizes existing bytes; known/uncertain effects survive close.
    /// Existing read snapshots/agents are never interrupted or refrozen.
    pub fn apply_activity_read_policy(&self, r: &Request) -> Result<ActivityReadPolicyMutation, E> {
        if !self.writable {
            return Err(E::Refused("root-read-only"));
        }
        let lock = self.directory.reserve(false, false).map_err(E::Storage)?;
        let mut visible = false;
        let result = (|| {
            let policy_lock = self
                .directory
                .reserve_activity_read_policy()
                .map_err(E::Storage)?;
            let result = (|| {
                let mut p = self.activity_read_proposal(r)?;
                if !p.result.replayed
                    && !r.preview_token().is_empty()
                    && r.preview_token() != p.result.preview_token
                {
                    return Err(E::Refused("activity-read-policy-preview-stale"));
                }
                if p.result.replayed {
                    visible = true;
                    self.directory
                        .synchronize_existing("activity-policy.json")
                        .map_err(E::Storage)?;
                    p.result.after_digest = Some(p.state.view.digest.clone());
                } else {
                    crate::validate_activity_read_config(&p.result.config)
                        .map_err(|e| E::Launch(Box::new(e)))?;
                    let bytes = encoded(&p.result.config)?;
                    if self
                        .activity_read_byte_cap()?
                        .is_some_and(|cap| bytes.len() > cap)
                    {
                        return Err(E::Refused("activity-read-policy-file-outside-saved-policy"));
                    }
                    if let Some(before) = p.state.raw.as_deref() {
                        let id = crate::generate_identifier("snapshot-").map_err(E::Runtime)?;
                        let name =
                            format!("activity-policy-before-{}-{id}.json", p.state.view.digest);
                        self.directory
                            .publish_new_with_fault(&name, before, AtomicStorageFault::None)
                            .map_err(E::Storage)?;
                        p.result.backup_reference = Some(name);
                    }
                    let fresh = self.activity_read_proposal(r)?;
                    if fresh.result.preview_token != p.result.preview_token
                        || fresh.state.raw != p.state.raw
                    {
                        return Err(E::Refused("activity-read-policy-preview-stale"));
                    }
                    let published = if p.state.raw.is_none() {
                        self.directory.publish_new_with_fault(
                            "activity-policy.json",
                            &bytes,
                            AtomicStorageFault::None,
                        )
                    } else {
                        self.directory.publish("activity-policy.json", &bytes)
                    };
                    if let Err(e) = published {
                        visible = e.publication == PublicationState::RenamedDurabilityUnknown;
                        return Err(E::Storage(e));
                    }
                    visible = true;
                    p.result.after_digest = Some(hash(&bytes));
                    p.result.exact = exact(&p.result.config)?;
                }
                p.result.preview = false;
                p.result.applied = true;
                Ok(p.result)
            })();
            finished(result, policy_lock.close().map_err(E::Storage))
        })();
        match finished(result, lock.close().map_err(E::Storage)) {
            Err(primary) if visible => Err(E::Published {
                primary: Box::new(primary),
            }),
            other => other,
        }
    }
}
