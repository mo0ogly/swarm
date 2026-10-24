//! OTLP/HTTP JSON ingest for provider telemetry (LIVE-UX-03, TEL-R3..R9, R13).
//!
//! `POST /api/otel/v1/logs`, `/metrics` and `/traces` (Codex spans) are the only routes
//! exempt from the browser cookie/CSRF contract. They authenticate a machine
//! exporter instead:
//! 1. before the body is read (transport head check): exact Host, POST,
//!    `Content-Type: application/json`, no `Content-Encoding`, and an
//!    `Authorization: Bearer` value with the token spelling;
//! 2. in the Root job, before the body is decoded: the token digest must be a
//!    non-revoked token in `.swarm/telemetry.db`, and its agent must still be
//!    active in the mission store (a finished, stopped or abandoned attempt
//!    revokes it implicitly, including across engine restarts).
//!
//! Records are attached to the token's agent, never to body attributes. The
//! body is read only up to the saved `web_http_body_max_bytes` (413 beyond).
//! Limit: an unauthenticated client with a well-formed token spelling can make
//! the server read a bounded body before the digest lookup refuses it.
//!
//! Tokens are issued at worker launch when the saved v30 setting
//! `agent_telemetry_providers` enables the provider and this server's endpoint
//! record is live (`swarm_engine::launch_telemetry_environment`). Not provided:
//! Codex injection, reviewer/planning launches, applied retention and a
//! per-attempt cap. Without an issued token every request is refused.

use super::{close, root, HttpServiceError, NativeHttpService, RouteBody};
use crate::WebAccessError;
use http::{header, Method, Request, StatusCode};
use swarm_engine::{
    normalize_logs, normalize_metrics, valid_token_syntax, RootStore, RootStoreError, StoreLimits, TelemetryStore,
    TelemetryStoreError,
};

pub(super) const LOGS: &str = "/api/otel/v1/logs";
pub(super) const METRICS: &str = "/api/otel/v1/metrics";
pub(super) const TRACES: &str = "/api/otel/v1/traces";

/// Whether `path` is an OTLP ingest route (exact match only).
pub(super) fn owns(path: &str) -> bool {
    matches!(path, LOGS | METRICS | TRACES)
}

/// Bearer secret from one `Authorization` header, if well formed.
fn bearer<B>(request: &Request<B>) -> Option<&str> {
    let mut values = request.headers().get_all(header::AUTHORIZATION).iter();
    let value = values.next()?;
    if values.next().is_some() {
        return None;
    }
    value
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .filter(|s| valid_token_syntax(s))
}

/// Header-only checks done before any body byte is read (see module doc).
/// Host is checked by the caller. Codes: `telemetry-method-invalid`,
/// `telemetry-format-unsupported` (protobuf, other type, any compression),
/// `telemetry-token-invalid`.
pub(super) fn authorize_head<B>(request: &Request<B>) -> Result<(), WebAccessError> {
    if request.method() != Method::POST {
        return Err(WebAccessError::Refused("telemetry-method-invalid"));
    }
    let headers = request.headers();
    if headers.get_all(header::CONTENT_ENCODING).iter().any(|v| v.as_bytes() != b"identity") {
        return Err(WebAccessError::Refused("telemetry-format-unsupported"));
    }
    let json = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("application/json"));
    if !json || headers.get_all(header::CONTENT_TYPE).iter().count() != 1 {
        return Err(WebAccessError::Refused("telemetry-format-unsupported"));
    }
    if bearer(request).is_none() {
        return Err(WebAccessError::Refused("telemetry-token-invalid"));
    }
    Ok(())
}

/// Status and public code for a head refusal on an OTLP route; None keeps the
/// transport's generic browser refusal. OTLP exporters do not retry 4xx.
pub(super) fn head_refusal(path: &str, error: &WebAccessError) -> Option<(StatusCode, &'static str)> {
    if !owns(path) {
        return None;
    }
    Some(match error.code() {
        "telemetry-method-invalid" => (StatusCode::METHOD_NOT_ALLOWED, "telemetry-method-invalid"),
        "telemetry-format-unsupported" => (StatusCode::UNSUPPORTED_MEDIA_TYPE, "telemetry-format-unsupported"),
        "telemetry-token-invalid" => (StatusCode::UNAUTHORIZED, "telemetry-token-invalid"),
        _ => (StatusCode::FORBIDDEN, "web-access-refused"),
    })
}

/// HTTP status of a telemetry refusal raised inside the Root job.
pub(super) fn status(code: &str) -> Option<StatusCode> {
    Some(match code {
        "telemetry-token-invalid" => StatusCode::UNAUTHORIZED,
        "telemetry-method-invalid" => StatusCode::METHOD_NOT_ALLOWED,
        "telemetry-format-unsupported" => StatusCode::UNSUPPORTED_MEDIA_TYPE,
        "telemetry-body-invalid" => StatusCode::BAD_REQUEST,
        // Exporters retry 503: contention or IO must never be a silent loss.
        "telemetry-store-unavailable" | "telemetry-store-busy" => StatusCode::SERVICE_UNAVAILABLE,
        "telemetry-store-unsafe" | "telemetry-store-version-unsupported" => StatusCode::INTERNAL_SERVER_ERROR,
        // LangWatch L10 operator annotations (console route, same store).
        "annotation-revision-conflict" | "annotation-agent-active" | "annotation-policy-missing" | "scenario-revision-conflict" => StatusCode::CONFLICT,
        "scenario-request-invalid" | "scenario-version-unsupported" | "scenario-empty" | "scenario-value-invalid" => StatusCode::BAD_REQUEST,
        "scenario-stored-invalid" => StatusCode::INTERNAL_SERVER_ERROR,
        "annotation-verdict-invalid" | "annotation-note-too-long" | "annotation-note-invalid" | "annotation-request-invalid" => StatusCode::BAD_REQUEST,
        _ => return None,
    })
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

fn store_error(cause: TelemetryStoreError) -> HttpServiceError {
    HttpServiceError::Telemetry(Box::new(cause))
}

impl NativeHttpService {
    /// Authenticate the exporter (step 2 of the module doc), normalize the OTLP
    /// JSON body and store it for the token's agent. Returns the OTLP response:
    /// `{}` or `{"partialSuccess": {"rejected…": n, "errorMessage": …}}`.
    /// Errors: `telemetry-token-invalid` 401 (unknown, revoked, inactive or
    /// missing subject — agent, reviewer or planning run — or no store:
    /// indistinguishable on purpose),
    /// `telemetry-body-invalid` 400, `telemetry-store-busy` /
    /// `telemetry-store-unavailable` 503 (nothing kept; retry is safe because
    /// records are idempotent). Saved settings v33 apply in the same store
    /// transaction: the per-attempt cap (excess reported in `partialSuccess`
    /// with `telemetry-attempt-cap-reached`) and the retention purge.
    pub(super) fn otel_ingest(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, HttpServiceError> {
        let path = request.uri().path();
        let secret = bearer(request).ok_or(HttpServiceError::Refused("telemetry-token-invalid"))?;
        let mut telemetry = match TelemetryStore::open(&self.root, false) {
            Ok(store) => store,
            Err(TelemetryStoreError::Refused("telemetry-store-missing")) => {
                return Err(HttpServiceError::Refused("telemetry-token-invalid"))
            }
            Err(cause) => return Err(store_error(cause)),
        };
        let result = (|| {
            let agent_id = telemetry
                .token_agent(secret)
                .map_err(store_error)?
                .ok_or(HttpServiceError::Refused("telemetry-token-invalid"))?;
            let mission = RootStore::open(&self.root, false).map_err(|cause| root("open", cause))?;
            let active = (|| {
                self.check_binding(&mission)?;
                // Agent, `review:<id>` or `planning:<claim>` subject still running.
                let active = mission.telemetry_subject_active(&agent_id).map_err(|cause| root("read", cause))?;
                // Saved v33 bounds; none before v33 (no purge, no cap).
                let settings = mission.settings().map_err(|cause| root("settings", cause))?;
                let limits = match settings.values.telemetry_retention_policy() {
                    Ok(Some((days, cap))) => StoreLimits { retention_days: days, max_records_per_agent: cap },
                    Ok(None) => StoreLimits::default(),
                    Err(cause) => return Err(root("settings", RootStoreError::Settings(cause))),
                };
                Ok((active, limits))
            })();
            let (active, limits) = close(active, mission.close())?;
            if !active {
                return Err(HttpServiceError::Refused("telemetry-token-invalid"));
            }
            let received = now_ms();
            let batch = match path {
                LOGS => normalize_logs(request.body(), received),
                TRACES => swarm_engine::normalize_traces(request.body(), received),
                _ => normalize_metrics(request.body(), received),
            }
            .map_err(HttpServiceError::TelemetryInput)?;
            if limits != StoreLimits::default() {
                telemetry.ensure_current().map_err(store_error)?;
            }
            let outcome = telemetry.record(&agent_id, &batch, received, limits).map_err(store_error)?;
            // Malformed items and points beyond the attempt cap are both
            // reported to the exporter as rejected (permanent, not retried).
            let rejected = batch.rejected + outcome.capped;
            let document = if rejected == 0 {
                serde_json::json!({})
            } else {
                let field = match path {
                    LOGS => "rejectedLogRecords",
                    TRACES => "rejectedSpans",
                    _ => "rejectedDataPoints",
                };
                let message = if outcome.capped > 0 { "telemetry-attempt-cap-reached" } else { "telemetry-items-rejected" };
                serde_json::json!({"partialSuccess": {
                    field: rejected.to_string(),
                    "errorMessage": message,
                }})
            };
            serde_json::to_vec(&document).map_err(HttpServiceError::Json)
        })();
        let closed = telemetry.close().map_err(store_error);
        match (result, closed) {
            (Ok(bytes), Ok(())) => Ok(RouteBody::plain(bytes)),
            (Err(error), _) | (Ok(_), Err(error)) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(headers: &[(&str, &str)], method: Method) -> Request<()> {
        let mut builder = Request::builder().method(method).uri(LOGS);
        for (k, v) in headers {
            builder = builder.header(*k, *v);
        }
        builder.body(()).unwrap()
    }

    #[test]
    fn head_checks_accept_a_witness_then_refuse_each_variant() {
        let token = format!("Bearer {}", "a".repeat(64));
        let json = ("content-type", "application/json; charset=utf-8");
        assert!(authorize_head(&request(&[json, ("authorization", &token)], Method::POST)).is_ok());
        type Case<'a> = (&'a [(&'a str, &'a str)], Method, &'a str);
        let cases: [Case; 6] = [
            (&[json, ("authorization", &token)], Method::GET, "telemetry-method-invalid"),
            (&[("content-type", "application/x-protobuf"), ("authorization", &token)], Method::POST, "telemetry-format-unsupported"),
            (&[json, ("content-encoding", "gzip"), ("authorization", &token)], Method::POST, "telemetry-format-unsupported"),
            (&[json], Method::POST, "telemetry-token-invalid"),
            (&[json, ("authorization", "Bearer short")], Method::POST, "telemetry-token-invalid"),
            (&[json, ("authorization", &token), ("authorization", &token)], Method::POST, "telemetry-token-invalid"),
        ];
        for (headers, method, code) in cases {
            let error = authorize_head(&request(headers, method)).unwrap_err();
            assert_eq!(error.code(), code);
            assert!(head_refusal(LOGS, &error).is_some());
            assert_eq!(head_refusal("/api/v1/works", &error), None);
        }
        assert_eq!(head_refusal(METRICS, &WebAccessError::Refused("web-host-refused")), Some((StatusCode::FORBIDDEN, "web-access-refused")));
        assert!(owns(LOGS) && owns(METRICS) && owns(TRACES) && !owns("/api/otel/v1/logs/"));
    }
}
