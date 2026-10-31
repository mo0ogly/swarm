//! Protected single-intent HTTP approval channel routed to the PROJECT root
//! held by the prepared PAEA Git call. The existing `/api/v1/trust-actions`
//! route serves the SERVICE root, so a business bearer is refused there before
//! any business identity lookup; this route is the explicit, typed channel for
//! exactly the opposite case: one exact business command (typically `approve`,
//! also `request`/`mandate-create`/`mandate-revoke`) executed on the separate
//! project root under the operator's already-held enrolled authority.
//! Nothing is selectable by the request: the root is the trusted startup
//! project path, the operation and command bytes are frozen into the bound
//! approval object at startup, and no JSON field, cookie or bearer can mint an
//! identity, choose a root or rewrite a term. The producers are the existing
//! engine Trust chain (`trust_enrollment_request` + `call_trust_as_enrolled`);
//! there is no second approval system and no Git execution here.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
use swarm_engine::{check_command_input_size, RootStore, RootStoreError};

impl super::PaeaGitCallAuthority {
    /// Hold the frozen approval source beside the already-held authority; the
    /// approval route below is the only consumer and nothing is selectable.
    /// The trust-action witness is prepared against ITS OWN project-root path,
    /// a different digest domain from the git-call's transfer-roots witness, so
    /// digest comparison cannot decide the bind: the engine's physical-witness
    /// check against the actually opened project root does. On refusal the
    /// supplied witness is closed explicitly and EVERY cause is retained typed:
    /// a failed open keeps the witness's own close error; a failed check keeps
    /// the check_root or project-close cause as primary beside the witness
    /// close outcome (no cause is downgraded to a bare refusal code).
    pub(super) fn bind_approval(
        &mut self,
        source: super::PaeaGitCallApprovalSource,
    ) -> Result<(), RootStoreError> {
        let project = match RootStore::open(self.project_path(), false) {
            Ok(project) => project,
            Err(open_cause) => {
                // Never touched the witness: close it explicitly and retain its
                // own outcome beside the open cause.
                return match source.close_prepared() {
                    Ok(()) => Err(open_cause),
                    Err(cleanup) => Err(RootStoreError::Cleanup {
                        primary: Some(Box::new(open_cause)),
                        cleanup: Box::new(cleanup),
                    }),
                };
            }
        };
        let check = source.engine_request().check_root(&project);
        let closed = project.close();
        let check = match (check, closed) {
            (Ok(()), Ok(())) => Ok(()),
            (Ok(()), Err(cleanup)) => Err(cleanup),
            (Err(cause), Ok(())) => Err(cause),
            (Err(cause), Err(cleanup)) => Err(RootStoreError::Cleanup {
                primary: Some(Box::new(cause)),
                cleanup: Box::new(cleanup),
            }),
        };
        match check {
            Ok(()) => {
                self.attach_approval(source);
                Ok(())
            }
            Err(cause) => {
                // The typed check/close cause is primary; the witness close
                // outcome is retained independently beside it.
                match source.close_prepared() {
                    Ok(()) => Err(cause),
                    Err(cleanup) => Err(RootStoreError::Cleanup {
                        primary: Some(Box::new(cause)),
                        cleanup: Box::new(cleanup),
                    }),
                }
            }
        }
    }
    /// Public capability of the bound approval: operation and exact command
    /// digest only. This authenticates nobody and grants nothing.
    pub(super) fn approval_capability(&self) -> serde_json::Value {
        self.bound_approval()
            .map(|a| {
                serde_json::json!({
                    "scope":"single_prepared_git_call_approval",
                    "operation":a.operation().as_str(),
                    "request_digest":a.request().request_digest(),
                })
            })
            .unwrap_or(serde_json::Value::Null)
    }
    /// Execute the frozen exact command on the PROJECT root under the held
    /// enrolled authority. The engine consumer recompares the physical witness,
    /// the current SQL identity plan and the current Trust revision, resolves
    /// the enrolled human principal and enforces the existing approve guard
    /// (role, separation of duties, exact terms). The password authenticates
    /// the held personal account; no request data reaches this call. A failed
    /// project close pairs inside the joined wrapper so no outcome is
    /// downgraded or hidden by cleanup alone. This performs no Git operation.
    pub(super) fn call_approval(&self, password: &[u8]) -> Result<RouteBody, E> {
        let Some(approval) = self.bound_approval() else {
            return Err(E::Refused("paea-git-call-approval-unavailable"));
        };
        let (authority, head) = self.approval_parts();
        // The project root is opened writable exactly like the consumption
        // route; the engine consumer rechecks both witnesses under authority.
        let project = RootStore::open(self.source().project_path(), true)
            .map_err(|cause| root("paea-git-call-approval-open", cause))?;
        let result = (|| {
            let reply = project
                .call_trust_as_enrolled(
                    approval.engine_request(),
                    authority,
                    &swarm_enrollment_authority::authenticated_control::Context::new(),
                    head,
                    password,
                    approval.command(),
                )
                .map_err(|e| {
                    root(
                        "paea-git-call-approval",
                        RootStoreError::TrustEnrollment(Box::new(e)),
                    )
                })?;
            let mut body = RouteBody::plain(reply.bytes);
            body.trust_publication = reply.publication;
            Ok(body)
        })();
        let closed = project.close().map_err(|e| match reply_publication(&result) {
            Some(publication) => RootStoreError::TrustPublished {
                primary: Box::new(e),
                publication,
            },
            None => e,
        });
        close(result, closed)
    }
}

/// Retain a known Trust publication through the joined close pairing without
/// reading private bytes: the route wrapper owns the publication label.
fn reply_publication(result: &Result<RouteBody, E>) -> Option<&'static str> {
    match result {
        Ok(body) => body.trust_publication,
        Err(error) => error.trust_publication(),
    }
}

impl NativeHttpService {
    /// Read-only startup observation behind the ordinary session/Host/CSRF
    /// gate: `GET /api/v1/paea/git-call/approval` reports whether an approval
    /// command is bound, with its public capability and no IO, secret or
    /// consumption. An explicit `bound:false` distinguishes "no approval
    /// prepared" from a refused request.
    pub(super) fn paea_git_call_approval_status(&self) -> Result<RouteBody, E> {
        let bound = self
            .paea_git_call
            .as_ref()
            .is_some_and(|a| a.bound_approval().is_some());
        let capability = self
            .paea_git_call
            .as_ref()
            .map(|a| a.approval_capability())
            .unwrap_or(serde_json::Value::Null);
        let reply = serde_json::json!({
            "schema_version":1,
            "event":"paea-git-call-approval-status",
            "bound":bound,
            "capability":capability,
        });
        Ok(RouteBody::plain(
            serde_json::to_vec(&reply).map_err(E::ResponseJson)?,
        ))
    }
    /// Protected presentation of the single bound approval behind the
    /// ordinary session/Host/CSRF gate: `GET
    /// /api/v1/paea/git-call/approval/presentation` serves only the explicitly
    /// authorized projection — the public approval capability (operation +
    /// request digest), the recomputed canonical identity of the underlying
    /// prepared Git action, the engine-derived agreement terms for it, and the
    /// current observed revision/witness state. The frozen command bytes and
    /// their parsed fields (action_id, mandate_id, content_digest) are NEVER
    /// served: the enrolled approver holds no Biba read right over that I2
    /// object (the engine already redacts its reply to a status
    /// acknowledgement), and the terms served here derive from the same public
    /// canonical proposal bytes the preview route serves verbatim — no new
    /// disclosure class. The confirmation is therefore connected to the
    /// digest and command held SERVER-SIDE: the browser never transports
    /// command bytes, never invents JSON and never reaches the engine with
    /// request data; the POST route consumes the held command on the bare
    /// credential. GET/HEAD only with an empty body and query; every other
    /// shape refuses typed before any Root open. Without a bound source or
    /// bound approval the reply stays an explicit `available:false` 200 with a
    /// reason, so absence is observable without any consumption, grant or
    /// consent. Effects: read-only opens of the service and project roots with
    /// paired closes; no secret is served, no password is requested and no
    /// approval is captured: the explicit Confirmation stays the form's own
    /// act, consumed only by the POST route under the held enrolled authority.
    /// Limits: `current` is an observation re-read from the live roots, not a
    /// promise; the engine consumer alone decides currency and terms at
    /// consumption, and a stale or altered credential still refuses there.
    pub(super) fn paea_git_call_approval_presentation(
        &self,
        r: &Request<Vec<u8>>,
    ) -> Result<RouteBody, E> {
        if !matches!(*r.method(), Method::GET | Method::HEAD) {
            return Err(E::Method("GET, HEAD"));
        }
        if !r.body().is_empty() {
            return Err(E::Refused("paea-git-call-approval-presentation-body-invalid"));
        }
        if r.uri().query().is_some_and(|v| !v.is_empty()) {
            return Err(E::Refused("paea-git-call-approval-presentation-query-invalid"));
        }
        let unavailable = |reason: &str| {
            serde_json::json!({
                "schema_version":1,
                "event":"paea-git-call-approval-presentation",
                "available":false,
                "reason":reason,
            })
        };
        let reply = |value: &serde_json::Value| {
            Ok(RouteBody::plain(
                serde_json::to_vec(value).map_err(E::ResponseJson)?,
            ))
        };
        let Some(authority) = self.paea_git_call.as_ref() else {
            return reply(&unavailable("paea-git-call-approval-source-unavailable"));
        };
        let Some(approval) = authority.bound_approval() else {
            return reply(&unavailable("paea-git-call-approval-unavailable"));
        };
        // Same recomputed canonical identity as the preview route: the approval
        // always rides one exact prepared Git action held by the same authority.
        let source = authority.source();
        let canonical = swarm_protection_contract::canonicalize(source.proposal(), source.context())
            .map_err(|_| E::Refused("paea-git-call-approval-presentation-canonical"))?;
        let digest_matches = canonical.digest() == source.request().action_digest();
        // Read-only observation of the live roots, mirroring the preview: no
        // writable open, no consumption, no consent.
        let service = RootStore::open(&self.root, false).map_err(|e| root("open", e))?;
        let result = (|| -> Result<RouteBody, E> {
            self.check_binding(&service)?;
            let project =
                RootStore::open(source.project_path(), false).map_err(|e| root("open", e))?;
            let result = (|| -> Result<RouteBody, E> {
                let roots_bound = source
                    .binding_matches(&project, &service)
                    .map_err(|e| root("paea-git-call-approval-presentation-root", e))?;
                let trust_revision = project
                    .trust_revision()
                    .map_err(|e| root("paea-git-call-approval-presentation-trust", e))?;
                let body = serde_json::json!({
                    "schema_version":1,
                    "event":"paea-git-call-approval-presentation",
                    "available":true,
                    "operation":approval.operation().as_str(),
                    "request_digest":approval.request().request_digest(),
                    "canonical":{
                        "schema":"paea-json-v1",
                        "action_digest":canonical.digest(),
                        "intent_id":canonical.intent_id(),
                        "policy_digest":source.context().policy_digest,
                        "digest_matches":digest_matches,
                    },
                    // Engine-derived recipe terms for the underlying canonical
                    // action; an absent derivation (P1-only binding) is an
                    // explicit null, never a synthesized fallback.
                    "agreement_terms": project
                        .paea_git_agreement_terms(canonical.bytes())
                        .map_err(|e| root("paea-git-call-approval-presentation-terms", e))?,
                    "current":{
                        "roots_bound":roots_bound,
                        "trust_revision":trust_revision,
                        "declared_trust_revision":source.context().policy_revision,
                        "trust_revision_current":trust_revision
                            .is_some_and(|revision| revision == source.context().policy_revision),
                    },
                    "limits":{
                        "consent":"none-captured",
                        "consumption":"none",
                        "human_presence":"not-established",
                    },
                });
                reply(&body)
            })();
            let closed = project.close();
            close(result, closed)
        })();
        let closed = service.close();
        close(result, closed)
    }
    /// Protected single-intent approval route behind the ordinary
    /// session/Host/CSRF gate: `POST /api/v1/paea/git-call/approval` with
    /// header `Authorization: Enrollment <base64(password)>` and an EMPTY
    /// body. The command is never transported: consumption is connected to
    /// the digest and command held server-side (the bound approval), so the
    /// browser transports only the credential over the ordinary session gate.
    /// Any other method/credential shape or non-empty body refuses before any
    /// Root open; a missing or closed source refuses without touching any
    /// store. The engine chain then enforces the enrolled human principal,
    /// current plan/revision, physical witnesses and the existing approve
    /// guard; wrong root/role/terms/revision all refuse typed without any Git
    /// effect, and a replay stays a legitimate retained reply of the same
    /// original, never a second effect.
    pub(super) fn paea_git_call_approval(&self, r: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        if r.method() == Method::GET || r.method() == Method::HEAD {
            return self.paea_git_call_approval_status();
        }
        if r.method() != Method::POST {
            return Err(E::Method("POST"));
        }
        if !r.uri().query().unwrap_or("").is_empty() {
            return Err(E::Refused("paea-git-call-approval-query-invalid"));
        }
        check_command_input_size(r.body()).map_err(E::Input)?;
        if r.headers().get_all(http::header::AUTHORIZATION).iter().count() > 1 {
            return Err(E::Refused("trust-authentication-header"));
        }
        // The body must be EMPTY: the command is held server-side (bound
        // approval), so nothing transportable can drift. No request data —
        // term, action or JSON field — ever reaches the engine chain. Refused
        // before any Root open.
        if !r.body().is_empty() {
            return Err(E::Refused("paea-git-call-approval-request"));
        }
        // A bound git-call source is the precondition; without it there is no
        // project root to approve on and no held authority to approve with.
        let authority = self
            .paea_git_call
            .as_ref()
            .ok_or(E::Refused("paea-git-call-approval-source-unavailable"))?;
        if authority.bound_approval().is_none() {
            return Err(E::Refused("paea-git-call-approval-unavailable"));
        }
        let encoded = r
            .headers()
            .get(http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Enrollment "))
            .ok_or(E::Refused("paea-git-call-approval-credential"))?;
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        let mut password = zeroize::Zeroizing::new(Vec::new());
        URL_SAFE_NO_PAD
            .decode_vec(encoded, &mut password)
            .map_err(|_| E::Refused("paea-git-call-approval-credential"))?;
        if password.is_empty() {
            return Err(E::Refused("paea-git-call-approval-credential"));
        }
        authority.call_approval(password.as_slice())
    }
}
