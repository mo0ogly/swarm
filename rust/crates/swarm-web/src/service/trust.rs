//! Native-session protected trust routes; bearer identity is a separate authority.
use super::{close, root, HttpServiceError, NativeHttpService, RouteBody};
use http::{Method, Request};
use swarm_engine::RootStore;
impl NativeHttpService {
    /// Decode only fixed operations behind Host/session/CSRF admission. No input
    /// selects a root, executable, URL or integrity authority. Close failures
    /// preserve the exact known trust publication, never a successful empty body.
    pub(super) fn trust_actions(
        &self,
        r: &Request<Vec<u8>>,
    ) -> Result<RouteBody, HttpServiceError> {
        let op = if matches!(*r.method(), Method::GET | Method::HEAD) {
            "show"
        } else if r.method() == Method::POST {
            r.uri()
                .query()
                .and_then(|q| q.strip_prefix("action="))
                .ok_or(HttpServiceError::Refused("trust-action-required"))?
        } else {
            return Err(HttpServiceError::Method("GET, HEAD, POST"));
        };
        if !matches!(
            op,
            "show"
                | "export"
                | "anchor"
                | "verify"
                | "config-preview"
                | "config-apply"
                | "direct-write"
                | "elevate"
                | "elevate-integrity"
                | "mandate-create"
                | "mandate-revoke"
                | "request"
                | "approve"
                | "commit"
                | "reconcile"
                | "read-object"
                | "read-original"
                | "audit-transaction"
                | "public-anchor" | "public-verify" | "evidence-key-prepare" | "evidence-key-activate" | "evidence-key-revoke"
        ) {
            return Err(HttpServiceError::Refused("trust-action-unknown"));
        }
        swarm_engine::check_command_input_size(r.body()).map_err(HttpServiceError::Input)?;
        if r.headers().get_all(http::header::AUTHORIZATION).iter().count()>1 { return Err(HttpServiceError::Refused("trust-authentication-header")); }
        if matches!(*r.method(), Method::GET | Method::HEAD)
            && (r.uri().query().is_some() || !r.body().is_empty())
        {
            return Err(HttpServiceError::Refused("trust-read-query-or-body"));
        }
        if r.method() == Method::POST && matches!(op, "show" | "export" | "anchor") {
            let value: serde_json::Value =
                serde_json::from_slice(r.body()).map_err(HttpServiceError::Json)?;
            if !value.as_object().is_some_and(|v| v.is_empty()) {
                return Err(HttpServiceError::Refused("trust-read-body-invalid"));
            }
        }
        if matches!(op,"verify"|"public-verify") {
            return (if op=="verify"{swarm_engine::verify_trust(r.body())}else{swarm_engine::verify_public_trust(r.body())})
                .map(RouteBody::plain)
                .map_err(|e| {
                    root(
                        "trust-verify",
                        swarm_engine::RootStoreError::Trust(Box::new(e)),
                    )
                });
        }
        let readonly = matches!(
            op,
            "show" | "export" | "anchor" | "config-preview" | "read-object" | "read-original" | "public-anchor"
        );
        let store = RootStore::open(&self.root, !readonly).map_err(|e| root("trust-open", e))?;
        let mut publication = None;
        let result = (|| {
            self.check_binding(&store)?;
            let authorization = r
                .headers()
                .get(http::header::AUTHORIZATION)
                .map(|v| v.to_str().map_err(|_|HttpServiceError::Refused("trust-authentication-header")))
                .transpose()?.unwrap_or("");
            let mut reply = if op=="show" {
                store.trust_as_principal(op,r.body(),"").map_err(|e|root("trust-show",e))?
            } else if let Some(encoded)=authorization.strip_prefix("Enrollment ") {
                use base64::{Engine,engine::general_purpose::URL_SAFE_NO_PAD};
                let source=self.trust_enrollment.as_ref().ok_or(HttpServiceError::Refused("trust-enrollment-source-unavailable"))?;
                let mut password=zeroize::Zeroizing::new(Vec::new());
                URL_SAFE_NO_PAD.decode_vec(encoded, &mut password).map_err(|_|HttpServiceError::Refused("trust-enrollment-credential"))?;
                if password.is_empty() { return Err(HttpServiceError::Refused("trust-enrollment-credential")); }
                source.call(&store,op,r.body(),password.as_slice()).map_err(|e|root("trust-enrollment-command",e))?
            } else if authorization.is_empty() || authorization.starts_with("Bearer ") {
                store.trust_as_principal(op, r.body(), authorization.strip_prefix("Bearer ").unwrap_or(""))
                    .map_err(|e| root("trust-command", e))?
            } else { return Err(HttpServiceError::Refused("trust-authentication-scheme")); };
            if let Some(source)=self.trust_enrollment.as_ref().filter(|_|op=="show") {
                let mut view:serde_json::Value=serde_json::from_slice(&reply.bytes).map_err(HttpServiceError::ResponseJson)?;
                view["identity_authentication"]["held_enrollment"]=source.capability();
                reply.bytes=serde_json::to_vec(&view).map_err(HttpServiceError::ResponseJson)?;
            }
            publication = reply.publication;
            let mut body = RouteBody::plain(reply.bytes);
            body.trust_publication = publication;
            Ok(body)
        })();
        let closed = store.close().map_err(|e| match publication {
            Some(publication) => swarm_engine::RootStoreError::TrustPublished {
                primary: Box::new(e),
                publication,
            },
            None => e,
        });
        close(result, closed)
    }
}
