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
        ) {
            return Err(HttpServiceError::Refused("trust-action-unknown"));
        }
        swarm_engine::check_command_input_size(r.body()).map_err(HttpServiceError::Input)?;
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
        if op == "verify" {
            return swarm_engine::verify_trust(r.body())
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
            "show" | "export" | "anchor" | "config-preview" | "read-object" | "read-original"
        );
        let store = RootStore::open(&self.root, !readonly).map_err(|e| root("trust-open", e))?;
        let mut publication = None;
        let result = (|| {
            self.check_binding(&store)?;
            let token = r
                .headers()
                .get(http::header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.strip_prefix("Bearer "))
                .unwrap_or("");
            let reply = store
                .trust(op, r.body(), token)
                .map_err(|e| root("trust-command", e))?;
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
