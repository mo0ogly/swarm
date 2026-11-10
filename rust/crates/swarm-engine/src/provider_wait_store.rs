//! Checked compatible provider-wait storage; no network, mission or budget effects.
use super::{absent,finished};
use crate::{RootStore,RootStoreError as E,ProviderWaitConfig as Config,ApiProviderWaitChange as Request,ProviderWaitRevision,WorkEnvironment};
use crate::provider_configuration::configuration_digest as hash;
use crate::local_storage::{AtomicStorageFault,PublicationState};
use serde::Serialize;
const DEFAULTS:&[u8]=include_bytes!("../../../../config/provider-wait.json");
/// Read projection. Exact decimal strings avoid browser rounding; only config
/// is persisted. Source/digest identify actual bytes, not authenticated policy.
#[derive(Serialize)]
pub struct ProviderWaitView {
    #[serde(flatten)] pub config:Config,
    pub source:&'static str,pub digest:String,pub exact:serde_json::Value,
    pub lease_min_seconds:String,pub lease_max_seconds:Option<String>,pub duration_max_seconds:String,
}
/// Preview or recorded application. Replay confirms current stored state without
/// a new revision. Preview has no commit timestamp/digest; token is never stored.
#[derive(Serialize)]
pub struct ProviderWaitMutation {
    pub applied:bool,pub preview:bool,pub replayed:bool,pub config:Config,
    pub exact:serde_json::Value,pub preview_token:String,pub before_digest:String,
    pub after_digest:Option<String>,pub backup_reference:Option<String>,
}
struct State {raw:Option<Vec<u8>>,view:ProviderWaitView}
struct Proposal {state:State,result:ProviderWaitMutation}
/// Turn all typed JSON integers into decimal strings for exact UI consumption.
/// No floating values occur in this DTO; original typed config remains public.
fn exact(config:&Config)->Result<serde_json::Value,E> {
    /// Recursively project this fixed typed DTO, preserving text/null/structure
    /// and rendering integer values as exact decimals; no source IO or mutation.
    fn convert(value:&mut serde_json::Value) {match value {serde_json::Value::Number(n)=>*value=serde_json::Value::String(n.to_string()),serde_json::Value::Array(a)=>a.iter_mut().for_each(convert),serde_json::Value::Object(o)=>o.values_mut().for_each(convert),_=>()}}
    let mut v=serde_json::to_value(config).map_err(|e|E::Launch(Box::new(crate::AgentLaunchError::caused("provider-wait-encode-failed",e))))?;convert(&mut v);Ok(v)
}
/// Validate typed requests again at the authoritative Root boundary, including
/// callers bypassing the wire decoder. Current bounds apply only to NEW rows.
fn intrinsic(r:&Request)->Result<(),E> {
    if r.schema_version!=1 || r.expected_revision<0 || !swarm_identity_contract::name(r.event_id.as_bytes()) || crate::text::trim_space(&r.reason).is_empty() {return Err(E::Refused("api-provider-wait-change-invalid"));}
    crate::provider_wait::validate_stored_values(&r.values).map_err(|e|E::Launch(Box::new(e)))
}
impl RootStore {
    /// Read checked private source with current saved cap; absence uses compiled
    /// versioned defaults. Invalid stored/default data retain a source-specific
    /// cause, never caller-error or empty-success fallback. No file/lock creation.
    fn wait_state(&self)->Result<State,E> {
        let settings=self.read_settings()?;
        let launch=settings.values.agent_launch_policy().map_err(E::Settings)?;
        let lease=settings.values.planning_action_policy().map_err(E::Settings)?;
        let raw=match self.directory.read_capped("provider-wait.json",launch.provider_bytes){Ok(v)=>Some(v),Err(e) if absent(&e)=>None,Err(e)=>return Err(E::Storage(e))};
        let config=(|| {if let Some(bytes)=raw.as_deref(){crate::provider_wait::decode_provider_wait_config(bytes)}else{Ok(Config{schema_version:1,revision:0,values:crate::provider_wait::decode_provider_wait_defaults(DEFAULTS)?,history:Some(vec![])})}})().map_err(|e|E::Launch(Box::new(crate::AgentLaunchError::caused("provider-wait-source-invalid",e))))?;
        let view=ProviderWaitView{digest:hash(raw.as_deref().unwrap_or(DEFAULTS)),source:if raw.is_some(){"stored_file"}else{"versioned_defaults"},exact:exact(&config)?,config,lease_min_seconds:lease.lease_min_seconds.0.to_string(),lease_max_seconds:lease.lease_max_seconds.map(|n|n.0.to_string()),duration_max_seconds:crate::PROVIDER_WAIT_MAX_SECONDS.to_string()};
        Ok(State{raw,view})
    }
    /// Read current policy/history with settings custody. No provider contact,
    /// snapshot, policy lock, adoption or execution authority. Close causes kept.
    pub fn provider_wait(&self)->Result<ProviderWaitView,E> {
        let lock=self.directory.reserve(false,false).map_err(E::Storage)?;
        finished(self.wait_state().map(|s|s.view),lock.close().map_err(E::Storage))
    }
    /// Prepare an exact source/settings/Root/request-bound projection. Replays
    /// preserve the current config, even when later rows exist or bounds changed.
    fn wait_proposal(&self,r:&Request)->Result<Proposal,E> {
        intrinsic(r)?;let state=self.wait_state()?;let mut config=state.view.config.clone();let change=r.recorded_change();
        let replayed=if let Some(row)=config.history().iter().find(|row|row.change.event_id==change.event_id){if row.change!=change{return Err(E::Refused("provider-wait-event-conflict"));}true}else{false};
        let settings=self.read_settings()?;
        if !replayed {
            if r.expected_revision!=config.revision{return Err(E::Refused("revision_conflict"));}
            crate::provider_wait::validate_requested_values(&r.values,&settings.values.planning_action_policy().map_err(E::Settings)?).map_err(|e|E::Launch(Box::new(e)))?;
            config.revision=config.revision.checked_add(1).ok_or(E::Refused("provider-wait-revision-overflow"))?;config.values=r.values;
            config.history.get_or_insert_with(Vec::new).push(ProviderWaitRevision{revision:config.revision,values:r.values,actor:String::new(),at:String::new(),change:change.clone()});
        }
        // Preview rejects an already oversized projected history. Actual commit
        // adds observed timestamp and checks the complete serialized file again.
        let cap=settings.values.agent_launch_policy().map_err(E::Settings)?.provider_bytes;
        if !replayed {let bytes=crate::encode_model(&config).map_err(|e|E::Launch(Box::new(crate::AgentLaunchError::caused("provider-wait-encode-failed",e))))?;if cap.is_some_and(|n|bytes.len()>n){return Err(E::Refused("provider-wait-file-outside-saved-policy"));}}
        let binding=crate::encode_model(&serde_json::json!({"version":1,"source":state.view.source,"digest":state.view.digest,"change":change,"settings_revision":settings.revision,"settings":settings.values,"instance":self.instance()?})).map_err(|e|E::Launch(Box::new(crate::AgentLaunchError::caused("provider-wait-binding-invalid",e))))?;
        let result=ProviderWaitMutation{applied:false,preview:true,replayed,exact:exact(&config)?,config,preview_token:hash(&binding),before_digest:state.view.digest.clone(),after_digest:None,backup_reference:None};
        Ok(Proposal{state,result})
    }
    /// Preview without creating the Go-compatible policy lock or changing bytes.
    /// A projection is not an application; commit metadata is explicitly empty.
    pub fn preview_provider_wait(&self,r:&Request)->Result<ProviderWaitMutation,E> {
        let lock=self.directory.reserve(false,false).map_err(E::Storage)?;
        finished(self.wait_proposal(r).map(|p|p.result),lock.close().map_err(E::Storage))
    }
    /// Apply current preview under settings and Go policy locks. New changes use
    /// exact private backup + checked atomic publication; replay only synchronizes
    /// current bytes. Token excludes volatile timestamp, original Change excludes
    /// token. Known/uncertain effects survive errors/close; reread on recovery.
    /// No authenticated actor, remote cancellation or mission acceptance claimed.
    pub fn apply_provider_wait(&self,r:&Request)->Result<ProviderWaitMutation,E> {
        if !self.writable{return Err(E::Refused("root-read-only"));}
        let lock=self.directory.reserve(false,false).map_err(E::Storage)?;let mut visible=false;
        let result=(|| {
            let first=self.wait_proposal(r)?;
            if !first.result.replayed && (r.preview_token.is_empty() || r.preview_token!=first.result.preview_token){return Err(E::Refused("provider-wait-preview-required"));}
            let policy_lock=self.directory.reserve_provider_wait().map_err(E::Storage)?;
            let result=(|| {
                let mut p=self.wait_proposal(r)?;
                if p.result.replayed {visible=true;self.directory.synchronize_existing("provider-wait.json").map_err(E::Storage)?;p.result.after_digest=Some(p.state.view.digest.clone());}
                else {
                    if r.preview_token!=p.result.preview_token{return Err(E::Refused("provider-wait-preview-required"));}
                    let mut env=crate::SystemEnvironment;
                    let at=env.timestamp().map_err(E::Runtime)?;
                    p.result.config.history.as_mut().and_then(|rows|rows.last_mut()).ok_or(E::Refused("provider-wait-history-invalid"))?.at=at;
                    crate::provider_wait::validate_stored_config(&p.result.config).map_err(|e|E::Launch(Box::new(e)))?;
                    let bytes=crate::encode_model(&p.result.config).map_err(|e|E::Launch(Box::new(crate::AgentLaunchError::caused("provider-wait-encode-failed",e))))?;
                    let cap=self.read_settings()?.values.agent_launch_policy().map_err(E::Settings)?.provider_bytes;
                    if cap.is_some_and(|cap|bytes.len()>cap){return Err(E::Refused("provider-wait-file-outside-saved-policy"));}
                    if let Some(before)=p.state.raw.as_deref(){let id=crate::generate_identifier("snapshot-").map_err(E::Runtime)?;let name=format!("provider-wait-before-{}-{id}.json",p.state.view.digest);self.directory.publish_new_with_fault(&name,before,AtomicStorageFault::None).map_err(E::Storage)?;p.result.backup_reference=Some(name);}
                    let fresh=self.wait_proposal(r)?;
                    if fresh.result.preview_token!=p.result.preview_token || fresh.state.raw!=p.state.raw{return Err(E::Refused("provider-wait-preview-required"));}
                    let published=if p.state.raw.is_none(){self.directory.publish_new_with_fault("provider-wait.json",&bytes,AtomicStorageFault::None)}else{self.directory.publish("provider-wait.json",&bytes)};
                    if let Err(e)=published{visible=e.publication==PublicationState::RenamedDurabilityUnknown;return Err(E::Storage(e));}visible=true;p.result.after_digest=Some(hash(&bytes));p.result.exact=exact(&p.result.config)?;
                }
                p.result.preview=false;p.result.applied=true;Ok(p.result)
            })();finished(result,policy_lock.close().map_err(E::Storage))
        })();
        match finished(result,lock.close().map_err(E::Storage)){Err(primary) if visible=>Err(E::Published{primary:Box::new(primary)}),other=>other}
    }
}
