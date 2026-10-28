//! Descriptor-bound trust transactions; no provider, process launch or hidden retry.
use super::{absent, finished, RootStore, RootStoreError as R};
use crate::{
    trust::{self, state::State, TrustError as E},
    TrustReply,
};
use time::OffsetDateTime;
use crate::trust::authentication::Source;
pub(super) const STATE: &str = "trust-actions.json";
const KEY: &str = "trust-actions-proof.key";
pub(super) const MAX_STATE: usize = 8 * 1024 * 1024;
fn failure(e: E) -> R {
    R::Trust(Box::new(e))
}
fn retained(e: R, publication: Option<&'static str>) -> R {
    match publication {
        Some(publication) => R::TrustPublished {
            primary: Box::new(e),
            publication,
        },
        None => e,
    }
}
impl RootStore {
    /// Execute the trust protocol against this opened root under its historical
    /// flock. Local configuration requires operator ownership; business actions
    /// authenticate their own stored token. Late errors retain publication state.
    /// Unsupported active policies refuse, never fall back to local identities.
    pub fn trust(&self, operation: &str, raw: &[u8], token: &str) -> Result<TrustReply, R> {
        self.trust_scoped(operation, raw, Source::Local(token), false)
    }
    /// Run a native-session business request. Configuration bootstrap is allowed
    /// only on an unconfigured root; afterwards preview/apply and their replays
    /// require a current local I3 administrator under the same state reservation.
    /// A session cookie alone grants no configuration authority after bootstrap.
    /// Local operator recovery remains the separate `trust` entry point.
    pub fn trust_as_principal(&self, operation: &str, raw: &[u8], token: &str) -> Result<TrustReply, R> {
        self.trust_scoped(operation, raw, Source::Local(token), true)
    }
    /// Serialize one request; the fixed caller context selects authentication,
    /// never an input field or operator policy. Preserve publication on errors.
    pub(super) fn trust_scoped(&self, operation: &str, raw: &[u8], token: Source<'_>, principal: bool) -> Result<TrustReply, R> {
        let readonly = matches!(
            operation,
            "show" | "export" | "anchor" | "config-preview" | "read-object" | "read-original" | "public-anchor"
        );
        if !readonly && !self.writable {
            return Err(R::Refused("read-only-root"));
        }
        let initial = self.instance()?;
        let reservation = self
            .directory
            .reserve_trust(!readonly)
            .map_err(R::Storage)?;
        let mut publication = None;
        let result = (|| {
            if self.instance()? != initial {
                return Err(R::Refused("root-instance-changed"));
            }
            let mut st = match self.directory.read_bounded(STATE, MAX_STATE) {
                Ok(bytes) => State::decode(&bytes).map_err(failure)?,
                Err(e) if absent(&e) => State::initial(),
                Err(e) => return Err(R::Storage(e)),
            };
            st.validate().map_err(failure)?;
            if !readonly {
                match self
                    .directory
                    .read_bounded("trust-actions.json.initializing", MAX_STATE)
                {
                    Err(e) if absent(&e) => {}
                    Ok(_) => return Err(failure(E::Conflict("trust-initialization-pending"))),
                    Err(e) => return Err(R::Storage(e)),
                }
            }
            let now = OffsetDateTime::now_utc();
            let operator = if principal && (operation=="audit-transaction" || matches!(operation, "config-preview" | "config-apply") && st.revision != 0) {
                st.require_supported().map_err(failure)?;
                let actor = trust::operations::authenticated(&st, token).map_err(failure)?;
                if actor.integrity != 3 || !actor.roles.iter().any(|r| r == "admin") {
                    return Err(failure(E::Authorization));
                }
                actor.subject.clone()
            } else { "local-operator".to_owned() };
            let mut planned = None;
            let bytes = match operation {
                #[cfg(target_os="linux")]
                "public-anchor"=>super::trust_evidence_runtime::anchor(self,&st,raw,token,now)?,
                #[cfg(target_os="linux")]
                "evidence-key-prepare"|"evidence-key-activate"|"evidence-key-revoke"=>{
                    let document:serde_json::Value=serde_json::from_slice(raw).map_err(|e|failure(e.into()))?;
                    let replayed=document["event_id"].as_str().is_some_and(|id|st.event_digests.contains_key(id));
                    let bytes=super::trust_evidence_runtime::change(self,&mut st,operation,raw,token,now)?;
                    if !replayed{publication=Some("signing-capability-changed");}
                    planned=Some(if replayed{"replayed"}else{"recorded"});bytes
                }
                "audit-transaction"=>{
                    st.require_supported().map_err(failure)?;
                    let actor=trust::operations::authenticated(&st,token).map_err(failure)?;
                    if actor.integrity!=3||!actor.roles.iter().any(|r|r=="admin"){return Err(failure(E::Authorization));}
                    let report=trust::audit::transaction(&st,raw).map_err(failure)?;
                    let mut value:serde_json::Value=serde_json::from_slice(&report).map_err(|e|failure(e.into()))?;
                    let id=value["action_id"].as_str().ok_or_else(||failure(E::Integrity("trust-audit-action-missing")))?.to_owned();
                    let mut command:trust::command::Command=swarm_wire_json::decode(b"{}").map_err(|e|failure(e.into()))?;
                    command.action_id=id.clone();
                    trust::operations::append(&mut st,"audit-access",&command,&actor,&id,&trust::hash(&report),"read","consultation de transaction autorisée",raw,now,None).map_err(failure)?;
                    let access=st.evidence.last().ok_or_else(||failure(E::Integrity("trust-audit-event-missing")))?;
                    value["audit_access"]=serde_json::json!({"sequence":access.sequence.to_string(),"hash":access.hash,"actor":access.actor,"report_digest":access.content_digest});
                    value["access_recorded"]=true.into();planned=Some("recorded");
                    use base64::Engine;
                    value["report_base64"]=base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&report).into();
                    trust::encoded(&value).map_err(failure)?
                }
                "show" => {
                    let mut view=st.public().map_err(failure)?;
                    #[cfg(target_os="linux")]
                    {view["public_evidence_runtime"]=super::trust_evidence_runtime::view(self,&st)?;}
                    trust::encoded(&view).map_err(failure)?
                }
                "export" => {
                    trust::encoded(&trust::proof::export(&st).map_err(failure)?).map_err(failure)?
                }
                "anchor" => {
                    let key = self.trust_key()?;
                    trust::encoded(
                        &trust::proof::anchor(&trust::proof::export(&st).map_err(failure)?, &key)
                            .map_err(failure)?,
                    )
                    .map_err(failure)?
                }
                "config-preview" => {
                    trust::config::change(&mut st, raw, true, now, &operator)
                        .map_err(failure)?
                }
                "config-apply" => {
                    planned = Some(
                        if st.event_replies.contains_key(
                            &serde_json::from_slice::<serde_json::Value>(raw)
                                .map_err(|e| failure(e.into()))?["event_id"]
                                .as_str()
                                .unwrap_or("")
                                .to_owned(),
                        ) {
                            "replayed"
                        } else {
                            "recorded"
                        },
                    );
                    trust::config::change(&mut st, raw, false, now, &operator)
                        .map_err(failure)?
                }
                _ => {
                    let outcome = trust::operations::execute(&mut st, operation, raw, token, now)
                        .map_err(failure)?;
                    planned = outcome.publication;
                    outcome.bytes
                }
            };
            if !readonly && planned == Some("recorded") {
                st.validate().map_err(failure)?;
                trust::proof::verify_chain(&trust::proof::export(&st).map_err(failure)?)
                    .map_err(failure)?;
                if self.instance()? != initial {
                    return Err(R::Refused("root-instance-changed"));
                }
                match self.trust_key() {
                    Ok(_) => {}
                    Err(e)
                        if operation == "config-apply"
                            && st.revision == 1
                            && matches!(&e,R::Storage(s) if absent(s)) =>
                    {
                        let mut key = [0u8; 32];
                        getrandom::getrandom(&mut key)
                            .map_err(|_| R::Refused("trust-entropy-unavailable"))?;
                        let mut bytes = key
                            .iter()
                            .map(|b| format!("{b:02x}"))
                            .collect::<String>()
                            .into_bytes();
                        bytes.push(b'\n');
                        if let Err(e)=self.directory.publish_new_with_fault(KEY,&bytes,crate::local_storage::AtomicStorageFault::None){
                            if e.publication==crate::PublicationState::RenamedDurabilityUnknown{publication=Some("key-durability-unknown");}
                            return Err(R::Storage(e));
                        }
                        publication = Some("key-created");
                    }
                    Err(e) => return Err(e),
                }
                let mut saved = serde_json::to_vec_pretty(&st).map_err(|e| failure(e.into()))?;
                saved.push(b'\n');
                if saved.len() > MAX_STATE {
                    return Err(failure(E::Invalid("trust-state-format-bound")));
                }
                if let Err(e) = self.directory.publish(STATE, &saved) {
                    if e.publication == crate::PublicationState::RenamedDurabilityUnknown {
                        publication = Some("durability-unknown");
                    }
                    return Err(R::Storage(e));
                }
                publication = Some("recorded");
            } else {
                publication = planned;
            }
            Ok(TrustReply { bytes, publication })
        })();
        // Transition errors before publication cannot claim a committed mutation.
        // A replay describes an older retained result, never a new effect.
        let cleanup = reservation.close().map_err(R::Storage);
        finished(result, cleanup).map_err(|e| retained(e, publication))
    }
    /// Read the separately retained symmetric proof key. Loss/malformed material
    /// refuses; it is never regenerated for an existing configured history.
    fn trust_key(&self) -> Result<Vec<u8>, R> {
        let bytes = self.directory.read_bounded(KEY, 65).map_err(R::Storage)?;
        let raw =
            std::str::from_utf8(&bytes).map_err(|_| failure(E::Integrity("trust-key-encoding")))?;
        trust::proof::decode_key(raw).map_err(failure)
    }
}
