//! Session-protected data-only PAEA correspondence; no Root or effect IO.
use super::{HttpServiceError, RouteBody};
use http::{header::CONTENT_TYPE, Method, Request};
use serde::Deserialize;
use swarm_protection_contract::{check_receipt_documents, State};

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

/// Check two supplied UTF-8 documents after outer Host/session/CSRF admission.
/// POST JSON only, no query/path selection, using the existing native transport
/// bound. The common pure checker returns v2 input hashes or a typed safe refusal;
/// original JSON/contract causes survive HttpServiceError and no partial report
/// is returned. This performs no filesystem/Root/network IO, policy mutation,
/// authority lookup, receipt authentication or effect/state application. A
/// successful response means structural correspondence only, even for UNKNOWN.
pub(super) fn receipt_report(r: &Request<Vec<u8>>) -> Result<RouteBody, HttpServiceError> {
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
