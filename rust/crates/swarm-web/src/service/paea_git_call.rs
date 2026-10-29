//! Trusted single-intent HTTP consumer of the guarded PAEA Git transfer and
//! promote calls. One exact prepared call, provisioned outside HTTP, is
//! consumed at most once through one authenticated POST; no request selects a
//! root, proposal, operation, declaration, pin or password. Both root paths
//! are trusted startup inputs, never request data, and both prepared physical
//! witnesses are checked against the actually opened roots before any session
//! publication or consumption.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
use std::path::PathBuf;
use swarm_engine::{check_command_input_size, RootStore, RootStoreError};
use swarm_enrollment_authority::{
    authenticated_control::{AuthenticatedControl, CloseError, Context as UseContext},
    authenticated_startup, paea_action::Request as PaeaRequest,
};
use swarm_enrollment_state::Head;

/// One exact prepared PAEA Git call with the trusted startup paths of BOTH
/// roots it binds (service and separate project), held outside HTTP between
/// preparation and single consumption. The prepared witnesses keep both
/// physical roots alive inside this process; the operator provisions the
/// exact permission out of band and opens the authority from the provisioned
/// declaration. This is a trusted startup input, not a queue, an identity
/// provider or a grant issuer: no account is created, no permission is minted
/// and no second run exists after the one-shot call.
pub struct PaeaGitCallSource {
    prepared: Prepared,
    /// Trusted startup path of the service root (the web Root itself); never
    /// request data. The path designates nothing by itself; the prepared
    /// physical witness comparison at bind and consumption is the contract.
    service_path: PathBuf,
    /// Trusted startup path of the separate project root; never request data.
    project_path: PathBuf,
}
/// The guarded native preparation itself performs no reference effect; only
/// the engine consumers below publish or read durable state under authority.
enum Prepared {
    /// Pack import observation (one-shot transfer job, no ref effect).
    Transfer(swarm_engine::PaeaGitTransferCall),
    /// Promotion with OLD-fenced commit or the explicit negative outcome.
    Promote(swarm_engine::PaeaGitPromoteCall),
}
impl PaeaGitCallSource {
    /// Trusted startup construction of a prepared transfer call source; both
    /// paths must be the same canonical roots the preparation witnesses bound.
    /// Nothing here performs IO or grants authority.
    pub fn new_transfer(
        prepared: swarm_engine::PaeaGitTransferCall,
        service_path: PathBuf,
        project_path: PathBuf,
    ) -> Self {
        Self {
            prepared: Prepared::Transfer(prepared),
            service_path,
            project_path,
        }
    }
    /// Trusted startup construction of a prepared promote call source; both
    /// paths must be the same canonical roots the preparation witnesses bound.
    /// Nothing here performs IO or grants authority.
    pub fn new_promote(
        prepared: swarm_engine::PaeaGitPromoteCall,
        service_path: PathBuf,
        project_path: PathBuf,
    ) -> Self {
        Self {
            prepared: Prepared::Promote(prepared),
            service_path,
            project_path,
        }
    }
    /// Stable protocol query spelling; both bind Service::GitPromotion.
    pub fn kind(&self) -> &'static str {
        match &self.prepared {
            Prepared::Transfer(_) => "transfer",
            Prepared::Promote(_) => "promote",
        }
    }
    /// Exact permission data to provision separately, never an issued grant.
    pub fn request(&self) -> &PaeaRequest {
        match &self.prepared {
            Prepared::Transfer(prepared) => prepared.request(),
            Prepared::Promote(prepared) => prepared.request(),
        }
    }
    /// Explicitly release both held storage witnesses on every refusal path.
    fn close_prepared(self) -> Result<(), RootStoreError> {
        match self.prepared {
            Prepared::Transfer(prepared) => prepared.close(),
            Prepared::Promote(prepared) => prepared.close(),
        }
    }
}
/// Startup/close causes are independent; a close failure is never consumption.
/// No path, pin, password, proposal or private nested diagnostic is formatted.
pub struct PaeaGitCallSourceError {
    pub startup: Option<authenticated_startup::Error>,
    pub authority_close: Option<CloseError>,
    pub prepared_close: Option<RootStoreError>,
}
impl PaeaGitCallSourceError {
    /// Complete retained causes; source() alone cannot enumerate cleanup errors.
    pub fn branches(&self) -> Vec<&(dyn std::error::Error + 'static)> {
        let mut causes: Vec<&(dyn std::error::Error + 'static)> = Vec::new();
        if let Some(e) = &self.startup {
            causes.push(e);
        }
        if let Some(e) = &self.authority_close {
            causes.push(e);
        }
        if let Some(e) = &self.prepared_close {
            causes.push(e);
        }
        causes
    }
}
impl std::fmt::Display for PaeaGitCallSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("paea-git-call-source-failed")
    }
}
impl std::fmt::Debug for PaeaGitCallSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
impl std::error::Error for PaeaGitCallSourceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.branches().into_iter().next()
    }
}
/// One exact prepared call plus its current held authority, kept alive until
/// the single authenticated consumption or an explicit source close.
pub struct PaeaGitCallAuthority {
    source: PaeaGitCallSource,
    authority: AuthenticatedControl,
    head: Head,
}
impl PaeaGitCallSource {
    /// Open only an independently provisioned protected declaration/pin for
    /// this exact operator after the owner provisioned the prepared
    /// permission. On startup refusal the owned prepared call is closed and
    /// its cause retained beside the fixed refusal; no account is created, no
    /// head adopted and no grant issued.
    pub fn open(
        self,
        declaration: &str,
        pin: &str,
    ) -> Result<PaeaGitCallAuthority, PaeaGitCallSourceError> {
        match authenticated_startup::open(&swarm_enrollment_state::Context::new(), declaration, pin)
        {
            Ok((authority, head)) => Ok(PaeaGitCallAuthority {
                source: self,
                authority,
                head,
            }),
            Err(startup) => Err(PaeaGitCallSourceError {
                startup: Some(startup),
                authority_close: None,
                prepared_close: self.close_prepared().err(),
            }),
        }
    }
}
impl PaeaGitCallAuthority {
    /// Public startup observation only; this does not authenticate the next
    /// call or promise that the held grant remains current. No secret body.
    pub(super) fn capability(&self) -> serde_json::Value {
        serde_json::json!({
            "scope":"single_prepared_git_call",
            "reason":"trusted-single-intent-git-call-bound",
            "call":self.source.kind(),
            "service":self.source.request().service().as_str(),
            "operation":self.source.request().operation().as_str(),
            "root_digest":self.source.request().root_digest(),
            "action_digest":self.source.request().action_digest(),
        })
    }
    /// Compare the prepared physical witnesses against the actually opened
    /// service and project roots, before transport publication. Both bindings
    /// must match; this precedes and never replaces the engine-side recheck
    /// under the held authority inside the guarded call itself.
    pub(super) fn check_roots(
        &self,
        service: &RootStore,
        project: &RootStore,
    ) -> Result<(), RootStoreError> {
        let bound = match &self.source.prepared {
            Prepared::Transfer(prepared) => prepared.binding_matches(project, service)?,
            Prepared::Promote(prepared) => prepared.binding_matches(project, service)?,
        };
        if bound {
            Ok(())
        } else {
            Err(RootStoreError::Refused("paea-git-call-root-binding"))
        }
    }
    /// The trusted startup path of the separate project root; never request
    /// data. The listener opens it read-only for the bind-time witness check.
    pub(super) fn project_path(&self) -> &std::path::Path {
        &self.source.project_path
    }
    /// Run only the configured exact call. The body must be the exact request
    /// file bytes (canonical digest comparison) and the kind must match the
    /// prepared call; both roots are opened writable here — the engine
    /// consumers recompare both physical witnesses under the held authority
    /// and enforce separated roots. Authority AfterUse and business causes
    /// stay typed through the engine wrappers; no second run exists inside
    /// one prepared call. A failed project close pairs inside the service
    /// close pairing so no outcome is downgraded or hidden by cleanup alone.
    pub fn call(&self, kind: &str, raw: &[u8], password: &[u8]) -> Result<serde_json::Value, E> {
        use sha2::{Digest, Sha256};
        if kind != self.source.kind() {
            return Err(E::Refused("paea-git-call-kind"));
        }
        let digest = Sha256::digest(raw)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        if digest != self.source.request().action_digest() {
            return Err(E::Refused("paea-git-call-request"));
        }
        let service = RootStore::open(&self.source.service_path, true)
            .map_err(|cause| root("paea-git-call-open", cause))?;
        let result = (|| {
            let project = RootStore::open(&self.source.project_path, true)
                .map_err(|cause| root("paea-git-call-open", cause))?;
            let reply: Result<serde_json::Value, E> = (|| {
                self.check_roots(&service, &project)
                    .map_err(|cause| root("paea-git-call-root", cause))?;
                match &self.source.prepared {
                    Prepared::Transfer(prepared) => {
                        let receipt = service
                            .call_paea_git_transfer(
                                &project,
                                prepared,
                                &self.authority,
                                &UseContext::new(),
                                &self.head,
                                password,
                            )
                            .map_err(|e| {
                                root(
                                    "paea-git-call",
                                    RootStoreError::PaeaGitTransferCall(Box::new(e)),
                                )
                            })?;
                        Ok(serde_json::json!({
                            "schema_version":1,
                            "event":"paea-git-call-result",
                            "call":"transfer",
                            "outcome":"prepared",
                            "candidate":receipt.candidate,
                            "pack_bytes":receipt.pack_bytes,
                            "pack_sha256":receipt.pack_sha256,
                        }))
                    }
                    Prepared::Promote(prepared) => {
                        let outcome = service
                            .call_paea_git_promote(
                                &project,
                                prepared,
                                &self.authority,
                                &UseContext::new(),
                                &self.head,
                                password,
                            )
                            .map_err(|e| {
                                root(
                                    "paea-git-call",
                                    RootStoreError::PaeaGitPromoteCall(Box::new(e)),
                                )
                            })?;
                        Ok(match &outcome {
                            swarm_engine::PaeaGitPromoteOutcome::Recorded {
                                intent_id,
                                action_digest,
                                target_ref,
                                new_oid,
                                replayed,
                            } => serde_json::json!({
                                "schema_version":1,
                                "event":"paea-git-call-result",
                                "call":"promote",
                                "outcome":"recorded",
                                "intent_id":intent_id,
                                "action_digest":action_digest,
                                "target_ref":target_ref,
                                "new_oid":new_oid,
                                "replayed":replayed,
                            }),
                            // The explicit negative reconciliation is a
                            // structured outcome, never an empty success.
                            swarm_engine::PaeaGitPromoteOutcome::ReconciledNotCommitted {
                                intent_id,
                                target_ref,
                            } => serde_json::json!({
                                "schema_version":1,
                                "event":"paea-git-call-result",
                                "call":"promote",
                                "outcome":"reconciled_not_committed",
                                "intent_id":intent_id,
                                "target_ref":target_ref,
                            }),
                        })
                    }
                }
            })();
            let closed = project.close();
            close(reply, closed)
        })();
        let closed = service.close();
        close(result, closed)
    }
    /// Explicitly release the held authority and the prepared witnesses after
    /// all requests drain or on any refusal; all independent causes retained.
    /// This neither revokes the signed grant nor removes any persisted data.
    pub fn close(self) -> Result<(), PaeaGitCallSourceError> {
        let authority_close = self.authority.close().err();
        let prepared_close = self.source.close_prepared().err();
        if authority_close.is_some() || prepared_close.is_some() {
            Err(PaeaGitCallSourceError {
                startup: None,
                authority_close,
                prepared_close,
            })
        } else {
            Ok(())
        }
    }
}
impl NativeHttpService {
    /// Read-only startup observation behind the ordinary session/Host/CSRF
    /// gate: `GET /api/v1/paea/git-call` reports whether a prepared call is
    /// bound, with its public capability and no consumption, secret or Root
    /// IO. An explicit `bound:false` distinguishes "no source bound" from a
    /// refused request; the operator never has to consume the one-shot call
    /// to observe its state (configured/applied/observed stay distinct).
    pub(super) fn paea_git_call_status(&self) -> Result<RouteBody, E> {
        let reply = serde_json::json!({
            "schema_version":1,
            "event":"paea-git-call-status",
            "bound":self.paea_git_call.is_some(),
            "capability":self.paea_git_call.as_ref().map(|a|a.capability()),
        });
        Ok(RouteBody::plain(
            serde_json::to_vec(&reply).map_err(E::ResponseJson)?,
        ))
    }
    /// Trusted single-intent consumption route behind the ordinary
    /// session/Host/CSRF gate: `POST /api/v1/paea/git-call?call=transfer|promote`
    /// with header `Authorization: Enrollment <base64(password)>` and the
    /// exact request-file bytes as body. Any other method/kind/credential
    /// shape refuses before any Root open; a missing or already closed source
    /// refuses without consuming anything.
    pub(super) fn paea_git_call(&self, r: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        if r.method() == Method::GET || r.method() == Method::HEAD {
            return self.paea_git_call_status();
        }
        if r.method() != Method::POST {
            return Err(E::Method("POST"));
        }
        let kind = r
            .uri()
            .query()
            .and_then(|q| q.strip_prefix("call="))
            .filter(|k| matches!(*k, "transfer" | "promote"))
            .ok_or(E::Refused("paea-git-call-kind-required"))?;
        check_command_input_size(r.body()).map_err(E::Input)?;
        if r.headers().get_all(http::header::AUTHORIZATION).iter().count() > 1 {
            return Err(E::Refused("trust-authentication-header"));
        }
        let source = self
            .paea_git_call
            .as_ref()
            .ok_or(E::Refused("paea-git-call-source-unavailable"))?;
        let encoded = r
            .headers()
            .get(http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Enrollment "))
            .ok_or(E::Refused("paea-git-call-credential"))?;
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        let mut password = zeroize::Zeroizing::new(Vec::new());
        URL_SAFE_NO_PAD
            .decode_vec(encoded, &mut password)
            .map_err(|_| E::Refused("paea-git-call-credential"))?;
        if password.is_empty() {
            return Err(E::Refused("paea-git-call-credential"));
        }
        let reply = source.call(kind, r.body(), password.as_slice())?;
        Ok(RouteBody::plain(
            serde_json::to_vec(&reply).map_err(E::ResponseJson)?,
        ))
    }
}
