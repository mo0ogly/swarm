//! Go-compatible browser waiting policy, never an agent/provider cancellation.
//! Shared saved configuration bytes and checked private atomic publication.
use super::{absent, finished, RootStore, RootStoreError as E};
use crate::local_storage::{AtomicStorageFault, PublicationState};
use crate::provider_configuration::configuration_digest as hash;
use crate::{
    ApiWebRequestChange as Request, WebRequestConfig as Config, WebRequestRevision, WorkEnvironment,
};
use serde::Serialize;
const DEFAULTS: &[u8] = include_bytes!("../../../../config/web-requests.json");
/// JSON/JavaScript exact integer bound; no waiting duration or business cutoff.
const MAX_SAFE_INTEGER: i64 = crate::web_request_policy::WEB_REQUEST_MAX_SAFE_INTEGER_MS;
/// Current compatible config and actual source/cap. Only config is persisted;
/// decimal projection protects browser fidelity, not authority or identity.
#[derive(Serialize)]
pub struct WebRequestView {
    #[serde(flatten)]
    pub config: Config,
    pub source: &'static str,
    pub digest: String,
    pub exact: serde_json::Value,
    pub bounds: WebRequestBounds,
    pub configuration_max_bytes: Option<String>,
}
/// Technical representation bounds shared with the browser, zero disables its
/// cancellation only. Provider/work execution remains independently governed.
#[derive(Serialize)]
pub struct WebRequestBounds {
    pub min_ms: i64,
    pub max_safe_integer_ms: i64,
}
/// Preview or known application/replay. Token is absent from persisted Change;
/// preview metadata is an uncommitted observation, real output/close can fail later.
#[derive(Serialize)]
pub struct WebRequestMutation {
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
    view: WebRequestView,
}
struct Proposal {
    state: State,
    result: WebRequestMutation,
}
/// Preserve concrete encoding/codec errors without exposing private file text.
fn encoded(value: &impl Serialize) -> Result<Vec<u8>, E> {
    crate::encode_model(value).map_err(|e| {
        E::Launch(Box::new(crate::AgentLaunchError::caused(
            "web-request-policy-encode",
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
            "web-request-policy-projection",
            e,
        )))
    })?;
    convert(&mut v);
    Ok(v)
}
impl RootStore {
    /// Actual saved shared provider/local-policy file cap, not a literal Go1MiB
    /// fallback. Missing required settings refuse, null explicitly has no cap.
    fn web_request_byte_cap(&self) -> Result<Option<usize>, E> {
        self.read_settings()?
            .values
            .agent_launch_policy()
            .map(|p| p.provider_bytes)
            .map_err(E::Settings)
    }
    /// Read owned source, or only genuinely absent source's versioned defaults.
    /// Bad history/JSON/ownership and checked file close remain source errors.
    fn web_request_state(&self) -> Result<State, E> {
        let cap = self.web_request_byte_cap()?;
        let raw = match self.directory.read_capped("web-requests.json", cap) {
            Ok(raw) => Some(raw),
            Err(e) if absent(&e) => None,
            Err(e) => return Err(E::Storage(e)),
        };
        let config = (|| {
            if let Some(raw) = raw.as_deref() {
                crate::decode_web_request_config(raw)
            } else {
                Ok(Config {
                    schema_version: 1,
                    revision: 0,
                    values: crate::decode_web_request_defaults(DEFAULTS)?,
                    history: Some(vec![]),
                })
            }
        })()
        .map_err(|e| {
            E::Launch(Box::new(crate::AgentLaunchError::caused(
                "web-request-policy-source-invalid",
                e,
            )))
        })?;
        let view = WebRequestView {
            source: if raw.is_some() {
                "stored_file"
            } else {
                "versioned_defaults"
            },
            digest: hash(raw.as_deref().unwrap_or(DEFAULTS)),
            exact: exact(&config)?,
            config,
            bounds: WebRequestBounds {
                min_ms: 0,
                max_safe_integer_ms: MAX_SAFE_INTEGER,
            },
            configuration_max_bytes: cap.map(|n| n.to_string()),
        };
        Ok(State { raw, view })
    }
    /// Read browser-only timing policy/history under settings custody. No lock
    /// file creation, implicit adoption, model, mission or provider operation.
    pub fn web_request_policy(&self) -> Result<WebRequestView, E> {
        let lock = self.directory.reserve(false, false).map_err(E::Storage)?;
        finished(
            self.web_request_state().map(|s| s.view),
            lock.close().map_err(E::Storage),
        )
    }
    /// Build current source/settings/instance/request binding. Exact replay uses
    /// the first matching Go Change and returns CURRENT config, no new row.
    fn web_request_proposal(&self, r: &Request) -> Result<Proposal, E> {
        let change = r.recorded_change();
        crate::validate_web_request_change(&change).map_err(|e| E::Launch(Box::new(e)))?;
        let state = self.web_request_state()?;
        let mut config = state.view.config.clone();
        let replayed = if let Some(row) = config
            .history
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .find(|row| row.change.event_id == change.event_id)
        {
            if row.change != change {
                return Err(E::Refused("web-request-policy-event-conflict"));
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
                .ok_or(E::Refused("web-request-policy-revision-overflow"))?;
            config.values = r.values;
            let mut environment = crate::SystemEnvironment;
            config
                .history
                .get_or_insert_with(Vec::new)
                .push(WebRequestRevision {
                    revision: config.revision,
                    values: r.values,
                    actor: environment.operator().map_err(E::Runtime)?,
                    at: environment.timestamp().map_err(E::Runtime)?,
                    change: change.clone(),
                });
            let projected = encoded(&config)?;
            if self
                .web_request_byte_cap()?
                .is_some_and(|n| projected.len() > n)
            {
                return Err(E::Refused("web-request-policy-file-outside-saved-policy"));
            }
        }
        let settings = self.read_settings()?;
        let binding = encoded(
            &serde_json::json!({"schema_version":1,"source":state.view.source,"digest":state.view.digest,"change":change,"settings_revision":settings.revision,"settings":settings.values,"instance":self.instance()?}),
        )?;
        let result = WebRequestMutation {
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
    /// Read-only preview without creating Go's policy lock. Actor/time are local
    /// uncommitted observations included in the full serialized-size check, like
    /// Go; they can change before apply and never establish identity or commit.
    pub fn preview_web_request_policy(&self, r: &Request) -> Result<WebRequestMutation, E> {
        let lock = self.directory.reserve(false, false).map_err(E::Storage)?;
        finished(
            self.web_request_proposal(r).map(|p| p.result),
            lock.close().map_err(E::Storage),
        )
    }
    /// Explicit Go CAS or stronger supplied native token; no silent policy cutoff.
    /// Settings + Go lock protect checked backup and atomic publication. Exact
    /// replay synchronizes existing bytes; known/uncertain effects survive close.
    /// Already armed browser requests/agents are never interrupted or refrozen.
    pub fn apply_web_request_policy(&self, r: &Request) -> Result<WebRequestMutation, E> {
        if !self.writable {
            return Err(E::Refused("root-read-only"));
        }
        let lock = self.directory.reserve(false, false).map_err(E::Storage)?;
        let mut visible = false;
        let result = (|| {
            let policy_lock = self.directory.reserve_web_requests().map_err(E::Storage)?;
            let result = (|| {
                let mut p = self.web_request_proposal(r)?;
                if !p.result.replayed
                    && !r.preview_token().is_empty()
                    && r.preview_token() != p.result.preview_token
                {
                    return Err(E::Refused("web-request-policy-preview-stale"));
                }
                if p.result.replayed {
                    visible = true;
                    self.directory
                        .synchronize_existing("web-requests.json")
                        .map_err(E::Storage)?;
                    p.result.after_digest = Some(p.state.view.digest.clone());
                } else {
                    crate::validate_web_request_config(&p.result.config)
                        .map_err(|e| E::Launch(Box::new(e)))?;
                    let bytes = encoded(&p.result.config)?;
                    if self
                        .web_request_byte_cap()?
                        .is_some_and(|cap| bytes.len() > cap)
                    {
                        return Err(E::Refused("web-request-policy-file-outside-saved-policy"));
                    }
                    if let Some(before) = p.state.raw.as_deref() {
                        let id = crate::generate_identifier("snapshot-").map_err(E::Runtime)?;
                        let name = format!("web-requests-before-{}-{id}.json", p.state.view.digest);
                        self.directory
                            .publish_new_with_fault(&name, before, AtomicStorageFault::None)
                            .map_err(E::Storage)?;
                        p.result.backup_reference = Some(name);
                    }
                    let fresh = self.web_request_proposal(r)?;
                    if fresh.result.preview_token != p.result.preview_token
                        || fresh.state.raw != p.state.raw
                    {
                        return Err(E::Refused("web-request-policy-preview-stale"));
                    }
                    let published = if p.state.raw.is_none() {
                        self.directory.publish_new_with_fault(
                            "web-requests.json",
                            &bytes,
                            AtomicStorageFault::None,
                        )
                    } else {
                        self.directory.publish("web-requests.json", &bytes)
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
