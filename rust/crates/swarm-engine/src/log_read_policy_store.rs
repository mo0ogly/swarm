//! Native log/terminal display policy ledger, never agent execution/completion policy.
//! Shared saved configuration bytes and checked private atomic publication.
use super::{absent, finished, RootStore, RootStoreError as E};
use crate::local_storage::{AtomicStorageFault, PublicationState};
use crate::provider_configuration::configuration_digest as hash;
use crate::{
    ApiLogReadChange as Request, LogReadConfig as Config, LogReadRevision, WorkEnvironment,
};
use serde::Serialize;
const DEFAULTS: &[u8] = include_bytes!("../../../../config/log-read-policy.json");
/// JSON/JavaScript exact integer bound; no waiting duration or business cutoff.
const MAX_SAFE_INTEGER: i64 = crate::log_read_policy::LOG_MAX_SAFE_ROWS;
/// Current compatible config and actual source/cap. Only config is persisted;
/// decimal projection protects browser fidelity, not authority or identity.
#[derive(Serialize)]
pub struct LogReadPolicyView {
    #[serde(flatten)]
    pub config: Config,
    pub source: &'static str,
    pub digest: String,
    pub exact: serde_json::Value,
    pub bounds: LogReadPolicyBounds,
    pub configuration_max_bytes: Option<String>,
    /// False means an old settings group has no file-cap choice yet, not unlimited.
    pub configuration_cap_adopted: bool,
}
/// Exact integer representation and nonempty finite page preconditions.
/// Null explicitly selects all retained rows, with no execution/retention grant.
#[derive(Serialize)]
pub struct LogReadPolicyBounds {
    pub min_page_rows: i64,
    pub max_safe_integer_rows: i64,
}
/// Preview or known application/replay. Token is absent from persisted Change;
/// preview metadata is an uncommitted observation, real output/close can fail later.
#[derive(Serialize)]
pub struct LogReadPolicyMutation {
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
    view: LogReadPolicyView,
}
struct Proposal {
    state: State,
    result: LogReadPolicyMutation,
}
/// Preserve concrete encoding/codec errors without exposing private file text.
fn encoded(value: &impl Serialize) -> Result<Vec<u8>, E> {
    crate::encode_model(value).map_err(|e| {
        E::Launch(Box::new(crate::AgentLaunchError::caused(
            "log-read-policy-encode",
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
            "log-read-policy-projection",
            e,
        )))
    })?;
    convert(&mut v);
    Ok(v)
}
impl RootStore {
    /// Observe only the shared saved file-cap field. Outer None is historical
    /// absence, inner None explicitly unlimited; no launch/environment policy is
    /// required merely to read history. No implicit setting adoption or fallback.
    fn log_read_cap_observation(&self) -> Result<Option<Option<usize>>, E> {
        self.read_settings()?
            .values
            .agent_provider_config_max_bytes
            .map(|crate::SettingCap(value)| {
                value
                    .map(|n| {
                        usize::try_from(n.0).map_err(|e| {
                            E::Launch(Box::new(crate::AgentLaunchError::caused(
                                "log-read-file-cap-representation",
                                e,
                            )))
                        })
                    })
                    .transpose()
            })
            .transpose()
    }
    /// A present private source or any proposed write requires an explicitly
    /// adopted shared file cap. Missing old settings refuse, never mean unlimited.
    fn log_read_byte_cap(&self) -> Result<Option<usize>, E> {
        self.log_read_cap_observation()?
            .ok_or(E::Refused("log-read-file-cap-not-configured"))
    }
    /// Read owned source, or only genuinely absent source's versioned defaults.
    /// Bad history/JSON/ownership and checked file close remain source errors.
    fn log_read_state(&self) -> Result<State, E> {
        let cap = self.log_read_cap_observation()?;
        // O_PATH observes checked private regular-file presence without reading
        // payload bytes or inventing a byte ceiling. Defaults apply only to a
        // genuinely absent source; old settings stay byte-for-byte unadopted.
        let raw = match self.directory.regular_identity("log-read-policy.json") {
            Ok(_) => Some(
                self.directory
                    .read_capped(
                        "log-read-policy.json",
                        cap.ok_or(E::Refused("log-read-file-cap-not-configured"))?,
                    )
                    .map_err(E::Storage)?,
            ),
            Err(e) if absent(&e) => None,
            Err(e) => return Err(E::Storage(e)),
        };
        let config = (|| {
            if let Some(raw) = raw.as_deref() {
                crate::decode_log_read_config(raw)
            } else {
                Ok(Config {
                    schema_version: 1,
                    revision: 0,
                    values: crate::decode_log_read_defaults(DEFAULTS)?,
                    history: Some(vec![]),
                })
            }
        })()
        .map_err(|e| {
            E::Launch(Box::new(crate::AgentLaunchError::caused(
                "log-read-policy-source-invalid",
                e,
            )))
        })?;
        let view = LogReadPolicyView {
            source: if raw.is_some() {
                "stored_file"
            } else {
                "versioned_defaults"
            },
            digest: hash(raw.as_deref().unwrap_or(DEFAULTS)),
            exact: exact(&config)?,
            config,
            bounds: LogReadPolicyBounds {
                min_page_rows: 1,
                max_safe_integer_rows: MAX_SAFE_INTEGER,
            },
            configuration_max_bytes: cap.flatten().map(|n| n.to_string()),
            configuration_cap_adopted: cap.is_some(),
        };
        Ok(State { raw, view })
    }
    /// Read log/terminal read policy/history under settings custody. No lock
    /// file creation, implicit adoption, model, mission or provider operation.
    pub fn log_read_policy(&self) -> Result<LogReadPolicyView, E> {
        let lock = self.directory.reserve(false, false).map_err(E::Storage)?;
        finished(
            self.log_read_state().map(|s| s.view),
            lock.close().map_err(E::Storage),
        )
    }
    /// Build current source/settings/instance/request binding. Exact replay uses
    /// the first matching recorded Change and returns CURRENT config, no new row.
    fn log_read_proposal(&self, r: &Request) -> Result<Proposal, E> {
        let change = r.recorded_change();
        crate::validate_log_read_change(&change).map_err(|e| E::Launch(Box::new(e)))?;
        let state = self.log_read_state()?;
        let mut config = state.view.config.clone();
        let replayed = if let Some(row) = config
            .history
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .find(|row| row.change.event_id == change.event_id)
        {
            if row.change != change {
                return Err(E::Refused("log-read-policy-event-conflict"));
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
                .ok_or(E::Refused("log-read-policy-revision-overflow"))?;
            config.values = r.values;
            let mut environment = crate::SystemEnvironment;
            config
                .history
                .get_or_insert_with(Vec::new)
                .push(LogReadRevision {
                    revision: config.revision,
                    values: r.values,
                    actor: environment.operator().map_err(E::Runtime)?,
                    at: environment.timestamp().map_err(E::Runtime)?,
                    change: change.clone(),
                });
            let projected = encoded(&config)?;
            if self
                .log_read_byte_cap()?
                .is_some_and(|n| projected.len() > n)
            {
                return Err(E::Refused("log-read-policy-file-outside-saved-policy"));
            }
        }
        let settings = self.read_settings()?;
        let binding = encoded(
            &serde_json::json!({"schema_version":1,"source":state.view.source,"digest":state.view.digest,"change":change,"settings_revision":settings.revision,"settings":settings.values,"instance":self.instance()?}),
        )?;
        let result = LogReadPolicyMutation {
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
    pub fn preview_log_read_policy(&self, r: &Request) -> Result<LogReadPolicyMutation, E> {
        let lock = self.directory.reserve(false, false).map_err(E::Storage)?;
        finished(
            self.log_read_proposal(r).map(|p| p.result),
            lock.close().map_err(E::Storage),
        )
    }
    /// Explicit CAS or stronger supplied native token; no silent policy cutoff.
    /// Settings + policy lock protect checked backup and atomic publication. Exact
    /// replay synchronizes existing bytes; known/uncertain effects survive close.
    /// Existing read snapshots/agents are never interrupted or refrozen.
    pub fn apply_log_read_policy(&self, r: &Request) -> Result<LogReadPolicyMutation, E> {
        if !self.writable {
            return Err(E::Refused("root-read-only"));
        }
        let lock = self.directory.reserve(false, false).map_err(E::Storage)?;
        let mut visible = false;
        let result = (|| {
            let policy_lock = self
                .directory
                .reserve_log_read_policy()
                .map_err(E::Storage)?;
            let result = (|| {
                let mut p = self.log_read_proposal(r)?;
                if !p.result.replayed
                    && !r.preview_token().is_empty()
                    && r.preview_token() != p.result.preview_token
                {
                    return Err(E::Refused("log-read-policy-preview-stale"));
                }
                if p.result.replayed {
                    visible = true;
                    self.directory
                        .synchronize_existing("log-read-policy.json")
                        .map_err(E::Storage)?;
                    p.result.after_digest = Some(p.state.view.digest.clone());
                } else {
                    crate::validate_log_read_config(&p.result.config)
                        .map_err(|e| E::Launch(Box::new(e)))?;
                    let bytes = encoded(&p.result.config)?;
                    if self
                        .log_read_byte_cap()?
                        .is_some_and(|cap| bytes.len() > cap)
                    {
                        return Err(E::Refused("log-read-policy-file-outside-saved-policy"));
                    }
                    if let Some(before) = p.state.raw.as_deref() {
                        let id = crate::generate_identifier("snapshot-").map_err(E::Runtime)?;
                        let name =
                            format!("log-read-policy-before-{}-{id}.json", p.state.view.digest);
                        self.directory
                            .publish_new_with_fault(&name, before, AtomicStorageFault::None)
                            .map_err(E::Storage)?;
                        p.result.backup_reference = Some(name);
                    }
                    let fresh = self.log_read_proposal(r)?;
                    if fresh.result.preview_token != p.result.preview_token
                        || fresh.state.raw != p.state.raw
                    {
                        return Err(E::Refused("log-read-policy-preview-stale"));
                    }
                    let published = if p.state.raw.is_none() {
                        self.directory.publish_new_with_fault(
                            "log-read-policy.json",
                            &bytes,
                            AtomicStorageFault::None,
                        )
                    } else {
                        self.directory.publish("log-read-policy.json", &bytes)
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
