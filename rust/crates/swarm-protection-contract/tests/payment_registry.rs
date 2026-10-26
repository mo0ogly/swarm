//! Debt resolution data tests. Neither snapshots nor test contexts are authorities.
use serde_json::{json, Value};
use swarm_protection_contract::{canonicalize, decode_context, resolve_payment_documents};

const REGISTRY: &[u8] = include_bytes!("../../../../tests/fixtures/paea/payment-registry-v1.json");
const REQUEST: &[u8] = include_bytes!("../../../../tests/fixtures/paea/payment-request-v1.json");

/// Parse owned copies of frozen inputs; mutation never changes retained fixtures.
fn inputs() -> (Value, Value) {
    (
        serde_json::from_slice(REGISTRY).unwrap(),
        serde_json::from_slice(REQUEST).unwrap(),
    )
}

/// Resolve supplied test data without any IO or fallback after refusal.
fn resolve(registry: &Value, request: &Value) -> swarm_protection_contract::PaymentResolution {
    resolve_payment_documents(
        &serde_json::to_vec(registry).unwrap(),
        &serde_json::to_vec(request).unwrap(),
    )
    .unwrap()
}

/// Obtain a valid data-only context with the fixture project; no identity lookup.
fn context() -> swarm_protection_contract::AdmissionContext {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/paea/contract-v1.json"
    ))
    .unwrap();
    let mut context = vectors["vectors"][0]["context"].clone();
    context["project_id"] = json!("project-alpha");
    decode_context(&serde_json::to_vec(&context).unwrap()).unwrap()
}

/// Exact aliases converge, while supplier-scoped identical invoice references do
/// not merge distinct debts. All amounts/accounts originate in the registry.
#[test]
fn aliases_converge_and_supplier_references_remain_distinct() {
    let (registry, mut request) = inputs();
    let first = resolve(&registry, &request);
    let a = canonicalize(&first.proposal, &context()).unwrap();
    request["request_id"] = json!("attempt-2");
    request["invoice_reference"] = json!("legacy-12");
    let alias = resolve(&registry, &request);
    let b = canonicalize(&alias.proposal, &context()).unwrap();
    assert_eq!(a, b);
    assert_eq!(
        serde_json::to_value(&alias.proposal).unwrap()["action"]["invoice_id"],
        "debt-17"
    );
    request["invoice_reference"] = json!("F-12");
    request["supplier_id"] = json!("supplier-2");
    let other = canonicalize(&resolve(&registry, &request).proposal, &context()).unwrap();
    assert_ne!(a.intent_id(), other.intent_id());
    assert_ne!(a.digest(), other.digest());
}

/// Corrected terms keep business identity and change exact approval bytes; no
/// continuity between independently supplied revisions is claimed by the resolver.
#[test]
fn corrected_terms_preserve_intent_and_change_content() {
    let (mut registry, mut request) = inputs();
    let a = canonicalize(&resolve(&registry, &request).proposal, &context()).unwrap();
    registry["revision"] = json!(2);
    registry["debts"][0]["invoice_version"] = json!("v8");
    registry["debts"][0]["amount_minor"] = json!("420001");
    registry["debts"][0]["beneficiary_account_id"] = json!("account-corrected");
    request["expected_registry_revision"] = json!(2);
    request["expected_invoice_version"] = json!("v8");
    let b = canonicalize(&resolve(&registry, &request).proposal, &context()).unwrap();
    assert_eq!(a.intent_id(), b.intent_id());
    assert_ne!(a.digest(), b.digest());
}

/// A valid control precedes every mutation, including ambiguous unrelated rows.
/// The entire snapshot must be valid even when the selected row is unambiguous.
#[test]
fn ambiguous_ids_and_supplier_aliases_refuse_then_recover() {
    for field in ["id", "cross-row-alias", "same-row-alias", "unrelated-row"] {
        let (mut registry, request) = inputs();
        resolve(&registry, &request);
        match field {
            "id" => registry["debts"][1]["debt_id"] = json!("debt-17"),
            "cross-row-alias" => registry["debts"][1]["supplier_id"] = json!("supplier-1"),
            "same-row-alias" => {
                registry["debts"][0]["invoice_references"] = json!(["F-12", "F-12"])
            }
            _ => {
                let duplicate = registry["debts"][1].clone();
                registry["debts"].as_array_mut().unwrap().push(duplicate);
            }
        }
        let error = resolve_payment_documents(&serde_json::to_vec(&registry).unwrap(), REQUEST)
            .unwrap_err();
        assert_eq!(error.code(), "protection-registry-ambiguous", "{field}");
        resolve_payment_documents(REGISTRY, REQUEST).unwrap();
    }
}

/// Explicit project/revision/version checks and exact lookup refuse stale or
/// absent data, including empty snapshots and case changes, then recover.
#[test]
fn project_staleness_unknown_and_empty_registry_refuse() {
    for (field, value, code) in [
        (
            "project_id",
            json!("other-project"),
            "protection-registry-project-mismatch",
        ),
        (
            "expected_registry_revision",
            json!(2),
            "protection-registry-stale",
        ),
        (
            "expected_invoice_version",
            json!("v8"),
            "protection-invoice-stale",
        ),
        (
            "invoice_reference",
            json!("f-12"),
            "protection-registry-reference-unknown",
        ),
        (
            "supplier_id",
            json!("unknown"),
            "protection-registry-reference-unknown",
        ),
    ] {
        let (registry, mut request) = inputs();
        resolve(&registry, &request);
        request[field] = value;
        let error = resolve_payment_documents(
            &serde_json::to_vec(&registry).unwrap(),
            &serde_json::to_vec(&request).unwrap(),
        )
        .unwrap_err();
        assert_eq!(error.code(), code);
        resolve_payment_documents(REGISTRY, REQUEST).unwrap();
    }
    let (mut registry, request) = inputs();
    registry["debts"] = json!([]);
    assert_eq!(
        resolve_payment_documents(
            &serde_json::to_vec(&registry).unwrap(),
            &serde_json::to_vec(&request).unwrap()
        )
        .unwrap_err()
        .code(),
        "protection-registry-reference-unknown"
    );
}

/// Invalid unrelated terms and representations cannot be hidden by a good row.
#[test]
fn invalid_registry_and_request_values_never_produce_a_proposal() {
    for (field, value) in [
        ("amount_minor", json!("01")),
        ("currency", json!("eur")),
        ("invoice_references", json!([])),
        ("debt_id", json!("../escape")),
        ("supplier_id", json!("")),
    ] {
        let (mut registry, request) = inputs();
        resolve(&registry, &request);
        registry["debts"][1][field] = value;
        assert!(
            resolve_payment_documents(&serde_json::to_vec(&registry).unwrap(), REQUEST).is_err()
        );
        resolve_payment_documents(REGISTRY, REQUEST).unwrap();
    }
    for (field, value) in [
        ("schema_version", json!(2)),
        ("expected_registry_revision", json!(0)),
        ("request_id", json!("")),
        ("expected_invoice_version", json!("")),
        ("amount_minor", json!("1")),
    ] {
        let (registry, mut request) = inputs();
        resolve(&registry, &request);
        request[field] = value;
        assert!(
            resolve_payment_documents(REGISTRY, &serde_json::to_vec(&request).unwrap()).is_err()
        );
    }
}

/// Strict nested duplicates and number syntax retain typed JSON causes. Input
/// reformatting only changes raw hashes; it cannot silently change a proposal.
#[test]
fn strict_raw_bytes_and_non_authority_markers_are_preserved() {
    use sha2::{Digest, Sha256};
    use std::error::Error as _;
    let report = resolve_payment_documents(REGISTRY, REQUEST).unwrap();
    assert_eq!(
        report.registry_sha256,
        format!("{:x}", Sha256::digest(REGISTRY))
    );
    assert_eq!(
        report.request_sha256,
        format!("{:x}", Sha256::digest(REQUEST))
    );
    assert_eq!(report.authority, "not-established");
    assert_eq!(report.registry_authenticity, "not-established");
    assert_eq!(report.effect, "not-established");
    for (raw, is_registry) in [
        (
            String::from_utf8(REGISTRY.to_vec()).unwrap().replace(
                "\"debt_id\": \"debt-17\"",
                "\"debt_id\": \"debt-17\", \"debt_id\": \"other\"",
            ),
            true,
        ),
        (
            String::from_utf8(REQUEST.to_vec()).unwrap().replace(
                "\"schema_version\": 1",
                "\"schema_version\": 1, \"schema_version\": 1",
            ),
            false,
        ),
        (
            String::from_utf8(REQUEST.to_vec()).unwrap().replace(
                "\"expected_registry_revision\": 1",
                "\"expected_registry_revision\": 1.0",
            ),
            false,
        ),
    ] {
        resolve_payment_documents(REGISTRY, REQUEST).unwrap();
        let error = if is_registry {
            resolve_payment_documents(raw.as_bytes(), REQUEST)
        } else {
            resolve_payment_documents(REGISTRY, raw.as_bytes())
        }
        .unwrap_err();
        assert!(error.source().is_some());
    }
    let (registry, request) = inputs();
    let compact = resolve(&registry, &request);
    assert_eq!(compact.proposal, report.proposal);
    assert_ne!(compact.registry_sha256, report.registry_sha256);
    assert_ne!(compact.request_sha256, report.request_sha256);
}
