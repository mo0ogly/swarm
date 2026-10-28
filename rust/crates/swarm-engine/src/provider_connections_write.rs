//! Private API connection publication; no provider/contact or mission mutation.
use super::{absent,finished,RootStore,RootStoreError as E};
use crate::{ApiConnectionChange as Change,ApiConnectionMutation as Mutation,local_storage::{AtomicStorageFault,PublicationState}};
/// In-memory private bytes, never serialized or logged. Outcome exposes hashes
/// only; exact source is used for private snapshot and source recheck.
struct Proposal { before:Option<Vec<u8>>,after:Vec<u8>,result:Mutation }
/// Merge one already representation-validated draft with its actual retained
/// credential. False explicitly ignores submitted key bytes; changed destination
/// requires replacement when a retained key exists. Retained header failures keep
/// their stored-source cause and allow explicit replacement/removal. Pure: no IO,
/// namespace/default adoption, network or serialization. Shared by save and probe.
pub(super) fn effective_connection(mut c:crate::provider_connection_catalogue::ConnectionState,replace_key:bool,old:Option<&crate::provider_connection_catalogue::ConnectionState>)->Result<crate::provider_connection_catalogue::ConnectionState,E> {
    if !replace_key {
        if let Some(old)=old {
            if !old.key.is_empty() && old.base_url!=c.base_url{return Err(E::Refused("api-connection-key-destination-replacement-required"));}
            c.key=old.key.clone();
        }else{c.key.clear();}
        c_http::HeaderValue::from_bytes(c.key.as_bytes()).map_err(|e|E::Launch(Box::new(crate::AgentLaunchError::caused("api-connection-stored-key-invalid",e))))?;
    }
    Ok(c)
}
impl RootStore {
    /// Read the optional private source under caller-owned settings custody.
    /// Absence alone maps to None; all other IO/close failures retain causes.
    fn connection_source(&self,cap:Option<usize>)->Result<Option<Vec<u8>>,E> {
        match self.directory.read_capped("ai-connections.json",cap) {Ok(raw)=>Ok(Some(raw)),Err(e) if absent(&e)=>Ok(None),Err(e)=>Err(E::Storage(e))}
    }
    /// Build an actual source/CLI namespace/settings/Root-instance-bound proposal.
    /// Retained credentials also require valid header representation; malformed
    /// stored keys retain server causes and permit explicit replacement/removal.
    /// Explicit key retention only for unchanged endpoint; replacement/removal
    /// requires replace_key. No public raw keys, vendor defaults, count/byte cap
    /// beyond saved policy, file write or provider contact. Stored errors refuse.
    fn connection_proposal(&self,r:&Change)->Result<Proposal,E> {
        use crate::provider_configuration::configuration_digest as hash;
        let mut c=crate::provider_connection_change::validated(r).map_err(|e|E::Launch(Box::new(e)))?;
        let settings=self.read_settings()?;
        let policy=settings.values.agent_launch_policy().map_err(E::Settings)?;
        let before=self.connection_source(policy.provider_bytes)?;
        let before_digest=hash(before.as_deref().unwrap_or_default());
        if before_digest!=r.expected_digest {return Err(E::Refused("revision_conflict"));}
        // Preserve authoritative inventory identity semantics before editing.
        crate::provider_connection_catalogue::public(before.as_deref()).map_err(|e|E::Launch(Box::new(e)))?;
        let mut entries=crate::provider_connection_catalogue::records(before.as_deref()).map_err(|e|E::Launch(Box::new(e)))?;
        let old=entries.get(&c.id);
        c=effective_connection(c,r.replace_key,old)?;
        let cli=self.provider_configuration()?;
        if cli.configuration.providers.as_ref().is_some_and(|p|p.contains_key(&format!("api-{}",c.id))) {return Err(E::Refused("api-connection-cli-name-conflict"));}
        entries.insert(c.id.clone(),c);
        // Intentional storage serialization only, never a public connection DTO.
        let stored:std::collections::BTreeMap<_,_>=entries.iter().map(|(id,c)|(id,serde_json::json!({"id":c.id,"label":c.label,"base_url":c.base_url,"model":c.model,"key":c.key,"disabled":c.disabled}))).collect();
        let after=crate::encode_model(&stored).map_err(|e|E::Launch(Box::new(crate::AgentLaunchError::caused("api-connection-encode-failed",e))))?;
        if policy.provider_bytes.is_some_and(|cap|after.len()>cap) {return Err(E::Refused("api-connection-file-outside-saved-policy"));}
        let after_digest=hash(&after);
        let binding=crate::encode_model(&serde_json::json!({"version":1,"before":before_digest,"after":after_digest,"cli_source":cli.digest,"root_instance":self.instance()?,"settings_revision":settings.revision,"settings":settings.values})).map_err(|e|E::Launch(Box::new(crate::AgentLaunchError::caused("api-connection-binding-encode-failed",e))))?;
        let token=hash(&binding);
        Ok(Proposal{before,after,result:Mutation{version:1,preview:true,applied:false,saved:false,changed:before_digest!=after_digest,before_digest,after_digest,preview_token:token,backup_reference:None}})
    }
    /// Preview checked private configuration without creating lock/snapshot/file.
    /// Token binds observed sources/policy/instance, not identity, authorization
    /// or enduring custody. All read/validation/close causes remain original.
    pub fn preview_api_connection(&self,r:&Change)->Result<Mutation,E> {
        let lock=self.directory.reserve(false,false).map_err(E::Storage)?;
        let result=self.connection_proposal(r).map(|p|p.result);
        finished(result,lock.close().map_err(E::Storage))
    }
    /// Apply a current preview under shared settings and Go-compatible provider
    /// coordination lock. Exact private originals are snapshotted; absent source
    /// uses atomic no-replace. Reread all bindings after snapshot before publish.
    /// No-op explicitly syncs existing bytes. Visible/uncertain publication and
    /// late close errors preserve state; caller must reread, never assume rollback.
    /// No key returned, provider contact, budget/Agent/Work mutation or audit grant.
    pub fn apply_api_connection(&self,r:&Change)->Result<Mutation,E> {
        if !self.writable {return Err(E::Refused("root-read-only"));}
        if r.preview {return Err(E::Refused("api-connection-preview-not-application"));}
        let settings_lock=self.directory.reserve(false,false).map_err(E::Storage)?;
        let mut visible=false;
        let result=(|| {
            let first=self.connection_proposal(r)?;
            if r.preview_token.is_empty() || r.preview_token!=first.result.preview_token {return Err(E::Refused("api-connection-preview-required"));}
            let provider_lock=self.directory.reserve_provider_configuration().map_err(E::Storage)?;
            let result=(|| {
                let mut proposal=self.connection_proposal(r)?;
                if r.preview_token!=proposal.result.preview_token {return Err(E::Refused("api-connection-preview-required"));}
                if proposal.result.changed {
                    if let Some(before)=proposal.before.as_deref() {
                        let id=crate::generate_identifier("snapshot-").map_err(E::Runtime)?;
                        let reference=format!("api-connections-before-{}-{id}.json",proposal.result.before_digest);
                        self.directory.publish_new_with_fault(&reference,before,AtomicStorageFault::None).map_err(E::Storage)?;
                        proposal.result.backup_reference=Some(reference);
                    }
                    // A snapshot is history, not authority: recheck current inputs.
                    let fresh=self.connection_proposal(r)?;
                    if fresh.result.preview_token!=proposal.result.preview_token || fresh.before!=proposal.before || fresh.after!=proposal.after {return Err(E::Refused("api-connection-preview-required"));}
                    let published=if proposal.before.is_none() {
                        self.directory.publish_new_with_fault("ai-connections.json",&proposal.after,AtomicStorageFault::None)
                    } else {self.directory.publish("ai-connections.json",&proposal.after)};
                    if let Err(e)=published {visible=e.publication==PublicationState::RenamedDurabilityUnknown;return Err(E::Storage(e));}
                    visible=true;
                } else {
                    visible=true;self.directory.synchronize_existing("ai-connections.json").map_err(E::Storage)?;
                }
                proposal.result.preview=false;proposal.result.applied=true;proposal.result.saved=true;
                Ok(proposal.result)
            })();
            finished(result,provider_lock.close().map_err(E::Storage))
        })();
        let result=finished(result,settings_lock.close().map_err(E::Storage));
        match result {Err(primary) if visible=>Err(E::Published{primary:Box::new(primary)}),result=>result}
    }
}
