//! Session-protected read-only preview of the exact prepared PAEA action held
//! by the engine, addressed to a future approver. This is a presentation of
//! the bound preparation only: no consent is captured, no grant is issued, no
//! identity is created, no one-shot call is consumed and no human presence is
//! claimed. Absence of a bound source and stale prerequisites are explicit
//! observed states, never an empty success.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
use swarm_engine::RootStore;

impl NativeHttpService {
    /// GET/HEAD, empty body/query, behind the ordinary session/Host/CSRF gate.
    /// When a prepared call is bound at startup, present the exact data that
    /// preparation holds — declared proposal and admission context verbatim,
    /// the canonical identity recomputed here from those same declared fields
    /// with the preparation's own canonicalization and compared to the bound
    /// action digest, the current physical witness state of both roots, and
    /// the current observed Trust revision of the project root beside the
    /// declared one. Every prerequisite is reported as an explicit field; a
    /// stale one never becomes a success and no prerequisite is inferred.
    /// Without a bound source the reply stays an explicit `available:false`
    /// 200 so an operator observes the state without any consumption, grant
    /// or consent. No endpoint executes, no password is requested and no
    /// approval binding is created or checked here: this route neither
    /// replaces nor captures the future human approval.
    pub(super) fn paea_git_call_preview(&self, r: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        if !matches!(*r.method(), Method::GET | Method::HEAD) {
            return Err(E::Method("GET, HEAD"));
        }
        if !r.body().is_empty() {
            return Err(E::Refused("paea-git-call-preview-body-invalid"));
        }
        if r.uri().query().is_some_and(|v| !v.is_empty()) {
            return Err(E::Refused("paea-git-call-preview-query-invalid"));
        }
        let Some(authority) = self.paea_git_call.as_ref() else {
            let reply = serde_json::json!({
                "schema_version":1,
                "event":"paea-git-call-preview",
                "available":false,
                "reason":"paea-git-call-preview-source-unavailable",
            });
            return Ok(RouteBody::plain(
                serde_json::to_vec(&reply).map_err(E::ResponseJson)?,
            ));
        };
        let source = authority.source();
        let request = source.request();
        // Recompute the canonical identity from the declared proposal and
        // context with the preparation's own canonicalization; a mismatch with
        // the bound digest is reported explicitly, never normalized away.
        let canonical = swarm_protection_contract::canonicalize(source.proposal(), source.context())
            .map_err(|_| E::Refused("paea-git-call-preview-canonical"))?;
        let digest_matches = canonical.digest() == request.action_digest();
        // Current physical witness observation of both held roots, read-only:
        // the same comparison bind and consumption recheck, without IO beyond
        // the witnesses' own inode observation and no writable open.
        let service = RootStore::open(&self.root, false).map_err(|e| root("open", e))?;
        let result = (|| -> Result<RouteBody, E> {
            self.check_binding(&service)?;
            let project =
                RootStore::open(source.project_path(), false).map_err(|e| root("open", e))?;
            let result = (|| -> Result<RouteBody, E> {
                let roots_bound = source
                    .binding_matches(&project, &service)
                    .map_err(|e| root("paea-git-call-preview-root", e))?;
                // Current Trust revision of the project root beside the
                // declared one; an absent journal is an explicit null, never a
                // fabricated revision or an inferred prerequisite success.
                let trust_revision = project
                    .trust_revision()
                    .map_err(|e| root("paea-git-call-preview-trust", e))?;
                let reply = serde_json::json!({
                    "schema_version":1,
                    "event":"paea-git-call-preview",
                    "available":true,
                    "capability":authority.capability(),
                    "proposal":source.proposal(),
                    "context":source.context(),
                    "canonical":{
                        "schema":"paea-json-v1",
                        "action_digest":canonical.digest(),
                        "intent_id":canonical.intent_id(),
                        "policy_digest":source.context().policy_digest,
                        "digest_matches":digest_matches,
                    },
                    "current":{
                        "roots_bound":roots_bound,
                        "trust_revision":trust_revision,
                        "declared_trust_revision":source.context().policy_revision,
                        "trust_revision_current":trust_revision
                            .is_some_and(|revision| revision == source.context().policy_revision),
                    },
                    // Engine-derived exact approval terms for this prepared
                    // action (same fixed recipe the captured-agreement guard
                    // matches on), computed from the same recomputed canonical
                    // bytes already compared to the bound digest. Presentation
                    // only: the approver still goes through the real Trust
                    // producers; an absent derivation (P1-only binding) is an
                    // explicit null, never a synthesized fallback.
                    "agreement_terms": project
                        .paea_git_agreement_terms(canonical.bytes())
                        .map_err(|e| root("paea-git-call-preview-terms", e))?,
                    "limits":{
                        "consent":"none-captured",
                        "consumption":"none",
                        "human_presence":"not-established",
                    },
                });
                Ok(RouteBody::plain(
                    serde_json::to_vec(&reply).map_err(E::ResponseJson)?,
                ))
            })();
            let closed = project.close();
            close(result, closed)
        })();
        let closed = service.close();
        close(result, closed)
    }
}
