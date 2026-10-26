//! Session-protected PAEA data checks/resolution; no Root or effect IO.
use super::{HttpServiceError, RouteBody};
use http::{header::CONTENT_TYPE, Method, Request};
use serde::Deserialize;
use swarm_protection_contract::{check_receipt_documents, resolve_payment_documents, State};

/// The JSON envelope transports original UTF-8 strings without parsing their
/// numbers. Every field is required; duplicate/unknown keys and invalid states
/// fail decoding. Version 1 describes this envelope, not the v2 report it returns.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptCheck {
    schema_version: u32,
    document: String,
    receipt: String,
    expected_sink: String,
    previous_state: State,
}

/// Closed transport envelope; both inner JSON documents remain exact strings.
/// This version describes transport, never registry provenance or authority.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PaymentCheck {
    schema_version: u32,
    registry: String,
    request: String,
}

/// Check two supplied UTF-8 documents after outer Host/session/CSRF admission.
/// POST JSON only, no query/path selection, using the existing native transport
/// bound. The common pure checker returns v2 input hashes or a typed safe refusal;
/// original JSON/contract causes survive HttpServiceError and no partial report
/// is returned. This performs no filesystem/Root/network IO, policy mutation,
/// authority lookup, receipt authentication or effect/state application. A
/// successful response means structural correspondence only, even for UNKNOWN.
pub(super) fn receipt_report(r: &Request<Vec<u8>>) -> Result<RouteBody, HttpServiceError> {
    require_json(r)?;
    let input: ReceiptCheck = serde_json::from_slice(r.body()).map_err(HttpServiceError::Json)?;
    if input.schema_version != 1 {
        return Err(HttpServiceError::Refused("protection-envelope-version"));
    }
    let report = check_receipt_documents(
        input.document.as_bytes(),
        input.receipt.as_bytes(),
        &input.expected_sink,
        input.previous_state,
    )
    .map_err(HttpServiceError::Protection)?;
    serde_json::to_vec(&report)
        .map(RouteBody::plain)
        .map_err(HttpServiceError::Json)
}

/// Resolve supplied supplier/invoice references with the CLI's pure resolver,
/// after the outer native Host/session/origin/CSRF gate. Return the exact v1
/// report, including raw input hashes and i64 revision; no response coercion.
/// Strict transport/inner decoding and resolver errors preserve typed causes.
/// No Root read/mutation, registry authentication, approval or effect occurs;
/// two matching files remain untrusted and establish no current authority.
pub(super) fn resolve_payment(r: &Request<Vec<u8>>) -> Result<RouteBody, HttpServiceError> {
    require_json(r)?;
    let input: PaymentCheck = serde_json::from_slice(r.body()).map_err(HttpServiceError::Json)?;
    if input.schema_version != 1 {
        return Err(HttpServiceError::Refused("protection-envelope-version"));
    }
    let report = resolve_payment_documents(input.registry.as_bytes(), input.request.as_bytes())
        .map_err(HttpServiceError::Protection)?;
    serde_json::to_vec(&report)
        .map(RouteBody::plain)
        .map_err(HttpServiceError::Json)
}

/// Shared POST/JSON/no-query technical transport contract. Caller must already
/// have passed native admission; refusal preserves its safe code/cause and has
/// no IO or fallback. The existing engine input bound is not an operator policy.
fn require_json(r: &Request<Vec<u8>>) -> Result<(), HttpServiceError> {
    if r.method() != Method::POST {
        return Err(HttpServiceError::Method("POST"));
    }
    if r.uri().query().is_some() {
        return Err(HttpServiceError::Refused("protection-query-refused"));
    }
    if !r
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.split(';')
                .next()
                .is_some_and(|v| v.trim().eq_ignore_ascii_case("application/json"))
        })
    {
        return Err(HttpServiceError::Refused("protection-json-required"));
    }
    swarm_engine::check_command_input_size(r.body()).map_err(HttpServiceError::Input)?;
    Ok(())
}
