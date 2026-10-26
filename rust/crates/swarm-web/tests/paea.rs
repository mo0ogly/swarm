#![cfg(target_os = "linux")]
//! Native typed route/asset regression sources; not browser, transport or effect proof.
use http::{
    header::{COOKIE, HOST, ORIGIN},
    Request, StatusCode,
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    error::Error,
    fs,
    net::TcpListener,
    path::{Path, PathBuf},
};
use swarm_engine::{RootStore, SystemEnvironment};
use swarm_web::{HttpReply, LocalWebAuthority, NativeHttpService};
const KEY: &str = "session-0123456789abcdef01234567";
struct Fixture {
    root: PathBuf,
    listener: TcpListener,
    service: Option<NativeHttpService>,
}
impl Fixture {
    /// Initialize only an owned Root; no live mission, provider or effect exists here.
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(swarm_engine::generate_identifier("paea-web-").unwrap());
        fs::create_dir(&root).unwrap();
        RootStore::initialize(&root, &mut SystemEnvironment)
            .unwrap()
            .close()
            .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let authority = LocalWebAuthority::from_listener(&listener, KEY).unwrap();
        let service = Some(NativeHttpService::open(&root, authority).unwrap());
        Self {
            root,
            listener,
            service,
        }
    }
    /// Build a protected typed request. Tests remove individual admission headers
    /// only after a successful control, never borrowing the live operator session.
    fn request(&self, method: &str, path: &str, body: Vec<u8>) -> Request<Vec<u8>> {
        let authority = LocalWebAuthority::from_listener(&self.listener, KEY).unwrap();
        let mut request = Request::builder()
            .method(method)
            .uri(path)
            .header(HOST, authority.host())
            .header(COOKIE, format!("{}={KEY}", authority.cookie_name()))
            .header("content-type", "application/json")
            .body(body)
            .unwrap();
        if method != "GET" && method != "HEAD" {
            request.headers_mut().insert(
                ORIGIN,
                format!("http://{}", authority.host()).parse().unwrap(),
            );
            request
                .headers_mut()
                .insert("x-swarm-csrf", KEY.parse().unwrap());
        }
        request
    }
    /// Keep the HTTP failure alongside the response; encoded refusal is never success.
    fn call(&self, request: Request<Vec<u8>>) -> HttpReply {
        self.service.as_ref().unwrap().handle(request).unwrap()
    }
}
impl Drop for Fixture {
    /// Check service release and preserve a failing fixture rather than erasing evidence.
    fn drop(&mut self) {
        if let Some(service) = self.service.take() {
            service.close().unwrap();
        }
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.root).unwrap();
        }
    }
}
/// Retain all fixture file bytes and relative names; a passing request must not
/// create a hidden Trust ledger, policy or effect file in the opened Root.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(base: &Path, directory: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(base, &path, files);
            } else {
                files.insert(
                    path.strip_prefix(base).unwrap().to_owned(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files);
    files
}
/// Build unsigned data from frozen both-domain vectors; identities are precomputed
/// independently, while expected response comparison uses the common pure checker.
fn input(index: usize) -> Value {
    let all: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tests/fixtures/paea/contract-v1.json"
    )))
    .unwrap();
    let vector = &all["vectors"][index];
    json!({"schema_version":1,
        "document":serde_json::to_string(&json!({"proposal":vector["proposal"],"context":vector["context"]})).unwrap(),
        "receipt":serde_json::to_string(&json!({"schema_version":1,"action_digest":vector["action_digest"],
            "intent_id":vector["intent_id"],"sink_id":"sink-1","outcome":"COMMITTED","observation_digest":"d".repeat(64)})).unwrap(),
        "expected_sink":"sink-1","previous_state":"COMMIT_REQUESTED"})
}

/// Transport frozen registry/request as exact strings, never caller authority.
fn payment_input() -> Value {
    json!({"schema_version":1,
        "registry":include_str!("../../../../tests/fixtures/paea/payment-registry-v1.json"),
        "request":include_str!("../../../../tests/fixtures/paea/payment-request-v1.json")})
}

/// A valid existing receipt control precedes the new route. Require exact shared
/// resolver bytes, full i64 revision and no Root mutation; not browser/effect proof.
#[test]
fn paea_payment_http_exact_resolution_and_integer_bytes_are_data_only() {
    let fixture = Fixture::new();
    assert_eq!(
        fixture
            .call(fixture.request(
                "POST",
                "/api/v1/protection/receipt-report",
                serde_json::to_vec(&input(0)).unwrap()
            ))
            .response
            .status(),
        StatusCode::OK
    );
    let before = snapshot(&fixture.root);
    let mut body = payment_input();
    for revision in [1_i64, i64::MAX] {
        let mut registry: Value = serde_json::from_str(body["registry"].as_str().unwrap()).unwrap();
        let mut request: Value = serde_json::from_str(body["request"].as_str().unwrap()).unwrap();
        registry["revision"] = json!(revision);
        request["expected_registry_revision"] = json!(revision);
        body["registry"] = json!(serde_json::to_string(&registry).unwrap());
        body["request"] = json!(serde_json::to_string(&request).unwrap());
        let expected = swarm_protection_contract::resolve_payment_documents(
            body["registry"].as_str().unwrap().as_bytes(),
            body["request"].as_str().unwrap().as_bytes(),
        )
        .unwrap();
        let reply = fixture.call(fixture.request(
            "POST",
            "/api/v1/protection/resolve-payment",
            serde_json::to_vec(&body).unwrap(),
        ));
        assert_eq!(
            reply.response.status(),
            StatusCode::OK,
            "owned fixture: {}",
            fixture.root.display()
        );
        assert_eq!(
            reply.response.body(),
            &serde_json::to_vec(&expected).unwrap()
        );
        assert_eq!(snapshot(&fixture.root), before);
    }
}

/// Matching controls bracket transport/admission/semantic refusals; preserve
/// safe typed causes, no success/Root mutation and recovery. New assets stay private.
#[test]
fn paea_payment_http_refusals_recover_without_mutation() {
    let fixture = Fixture::new();
    let path = "/api/v1/protection/resolve-payment";
    let raw = serde_json::to_vec(&payment_input()).unwrap();
    let before = snapshot(&fixture.root);
    for fault in [
        "cookie",
        "csrf",
        "host",
        "origin",
        "method",
        "query",
        "type",
        "version",
        "duplicate",
        "unknown",
        "inner-type",
        "inner-duplicate",
        "project",
        "stale",
        "invalid-json",
    ] {
        assert_eq!(
            fixture
                .call(fixture.request("POST", path, raw.clone()))
                .response
                .status(),
            StatusCode::OK
        );
        let mut request = fixture.request("POST", path, raw.clone());
        match fault {
            "cookie" => {
                request.headers_mut().remove(COOKIE);
            }
            "csrf" => {
                request.headers_mut().remove("x-swarm-csrf");
            }
            "host" => {
                request
                    .headers_mut()
                    .insert(HOST, "outside.invalid".parse().unwrap());
            }
            "origin" => {
                request
                    .headers_mut()
                    .insert(ORIGIN, "http://outside.invalid".parse().unwrap());
            }
            "method" => *request.method_mut() = http::Method::GET,
            "query" => *request.uri_mut() = format!("{path}?root=PRIVATE_CANARY").parse().unwrap(),
            "type" => {
                request.headers_mut().remove("content-type");
            }
            "duplicate" => {
                *request.body_mut() = format!(
                    "{{\"schema_version\":1,{}",
                    &String::from_utf8(raw.clone()).unwrap()[1..]
                )
                .into_bytes()
            }
            "invalid-json" => *request.body_mut() = b"PRIVATE_CANARY".to_vec(),
            _ => {
                let mut body = payment_input();
                match fault {
                    "version" => body["schema_version"] = json!(2),
                    "unknown" => body["PRIVATE_CANARY"] = json!(true),
                    "inner-type" => body["registry"] = json!({}),
                    "inner-duplicate" => {
                        body["request"] = json!(body["request"].as_str().unwrap().replace(
                            "\"schema_version\": 1",
                            "\"schema_version\": 1, \"schema_version\": 1"
                        ))
                    }
                    _ => {
                        let mut inner: Value =
                            serde_json::from_str(body["request"].as_str().unwrap()).unwrap();
                        if fault == "project" {
                            inner["project_id"] = json!("PRIVATE_CANARY");
                        } else {
                            inner["expected_registry_revision"] = json!(2);
                        }
                        body["request"] = json!(serde_json::to_string(&inner).unwrap());
                    }
                }
                *request.body_mut() = serde_json::to_vec(&body).unwrap();
            }
        }
        let reply = fixture.call(request);
        assert!(!reply.response.status().is_success(), "{fault}");
        let error = reply.failure.as_ref().unwrap();
        assert!(!error.to_string().contains("PRIVATE_CANARY"));
        assert!(!String::from_utf8_lossy(reply.response.body()).contains("PRIVATE_CANARY"));
        if [
            "duplicate",
            "unknown",
            "inner-type",
            "inner-duplicate",
            "project",
            "stale",
            "invalid-json",
        ]
        .contains(&fault)
        {
            assert!(error.source().is_some(), "{fault}");
        }
        assert_eq!(snapshot(&fixture.root), before);
        assert_eq!(
            fixture
                .call(fixture.request("POST", path, raw.clone()))
                .response
                .status(),
            StatusCode::OK
        );
    }
    let mut private = fixture.request("GET", "/paea-payment.html", Vec::new());
    private.headers_mut().remove(COOKIE);
    assert!(!fixture.call(private).response.status().is_success());
    let page = fixture.call(fixture.request("GET", "/paea-payment.html", Vec::new()));
    assert_eq!(page.response.status(), StatusCode::OK);
    assert_eq!(
        page.response.body(),
        include_bytes!("../../../../web/paea-payment.html")
    );
    let entry = fixture.call(fixture.request("GET", "/paea.html", Vec::new()));
    assert!(String::from_utf8_lossy(entry.response.body()).contains("href=\"/paea-payment.html\""));
    assert_eq!(snapshot(&fixture.root), before);
}
/// Require exact common-library report, explicitly no authority or Root mutation.
#[test]
fn paea_http_exact_both_domains_and_unknown_are_data_only() {
    let fixture = Fixture::new();
    let before = snapshot(&fixture.root);
    for index in [0, 8] {
        for outcome in ["COMMITTED", "UNKNOWN"] {
            let mut body = input(index);
            let mut receipt: Value =
                serde_json::from_str(body["receipt"].as_str().unwrap()).unwrap();
            receipt["outcome"] = outcome.into();
            body["receipt"] = serde_json::to_string(&receipt).unwrap().into();
            let expected = swarm_protection_contract::check_receipt_documents(
                body["document"].as_str().unwrap().as_bytes(),
                body["receipt"].as_str().unwrap().as_bytes(),
                "sink-1",
                swarm_protection_contract::State::CommitRequested,
            )
            .unwrap();
            let reply = fixture.call(fixture.request(
                "POST",
                "/api/v1/protection/receipt-report",
                serde_json::to_vec(&body).unwrap(),
            ));
            assert_eq!(reply.response.status(), StatusCode::OK);
            assert!(reply.failure.is_none());
            assert_eq!(
                reply.response.body(),
                &serde_json::to_vec(&expected).unwrap()
            );
            assert!(reply.recorded_work.is_none());
            assert!(!reply.configuration_may_be_recorded);
            assert_eq!(snapshot(&fixture.root), before);
        }
    }
}
/// Valid control before each framing/binding/auth fault, cause/redaction checks,
/// then recovery. Private assets/menu are protected, exact bytes and HEAD retained.
#[test]
fn paea_http_refusal_recovery_and_private_menu_assets() {
    let fixture = Fixture::new();
    let raw = serde_json::to_vec(&input(0)).unwrap();
    let before = snapshot(&fixture.root);
    for fault in [
        "cookie",
        "csrf",
        "host",
        "origin",
        "method",
        "query",
        "type",
        "version",
        "duplicate",
        "unknown-key",
        "binding",
        "invalid-json",
    ] {
        assert_eq!(
            fixture
                .call(fixture.request("POST", "/api/v1/protection/receipt-report", raw.clone()))
                .response
                .status(),
            StatusCode::OK
        );
        let mut request = fixture.request("POST", "/api/v1/protection/receipt-report", raw.clone());
        match fault {
            "cookie" => {
                request.headers_mut().remove(COOKIE);
            }
            "csrf" => {
                request.headers_mut().remove("x-swarm-csrf");
            }
            "host" => {
                request
                    .headers_mut()
                    .insert(HOST, "outside.invalid".parse().unwrap());
            }
            "origin" => {
                request
                    .headers_mut()
                    .insert(ORIGIN, "http://outside.invalid".parse().unwrap());
            }
            "method" => {
                *request.method_mut() = http::Method::GET;
            }
            "query" => {
                *request.uri_mut() = "/api/v1/protection/receipt-report?root=PRIVATE_CANARY"
                    .parse()
                    .unwrap();
            }
            "type" => {
                request.headers_mut().remove("content-type");
            }
            "version" | "binding" | "unknown-key" => {
                let mut value = input(0);
                match fault {
                    "version" => value["schema_version"] = 2.into(),
                    "binding" => value["expected_sink"] = "wrong-sink".into(),
                    _ => value["PRIVATE_CANARY"] = true.into(),
                }
                *request.body_mut() = serde_json::to_vec(&value).unwrap();
            }
            "duplicate" => {
                let text = String::from_utf8(raw.clone()).unwrap();
                *request.body_mut() = format!("{{\"schema_version\":1,{}", &text[1..]).into_bytes();
            }
            _ => {
                *request.body_mut() = b"PRIVATE_CANARY".to_vec();
            }
        }
        let reply = fixture.call(request);
        assert!(!reply.response.status().is_success(), "{fault}");
        let error = reply.failure.as_ref().unwrap();
        assert!(!error.to_string().contains("PRIVATE_CANARY"));
        assert!(!String::from_utf8_lossy(reply.response.body()).contains("PRIVATE_CANARY"));
        if matches!(
            fault,
            "duplicate" | "unknown-key" | "invalid-json" | "binding"
        ) {
            assert!(error.source().is_some());
        }
        assert_eq!(snapshot(&fixture.root), before);
        assert_eq!(
            fixture
                .call(fixture.request("POST", "/api/v1/protection/receipt-report", raw.clone()))
                .response
                .status(),
            StatusCode::OK
        );
    }
    for (path, bytes) in [
        (
            "/paea.html",
            include_bytes!("../../../../web/paea.html").as_slice(),
        ),
        (
            "/paea.js",
            include_bytes!("../../../../web/paea.js").as_slice(),
        ),
        (
            "/paea.css",
            include_bytes!("../../../../web/paea.css").as_slice(),
        ),
    ] {
        let request = fixture.request("GET", path, vec![]);
        assert_eq!(fixture.call(request).response.body(), bytes);
        let mut request = fixture.request("GET", path, vec![]);
        request.headers_mut().remove(COOKIE);
        assert_eq!(
            fixture.call(request).response.status(),
            StatusCode::FORBIDDEN
        );
        let reply = fixture.call(fixture.request("HEAD", path, vec![]));
        assert_eq!(reply.response.status(), StatusCode::OK);
        assert!(reply.response.body().is_empty());
    }
    let reply = fixture.call(fixture.request("GET", "/", vec![]));
    assert!(String::from_utf8_lossy(reply.response.body()).contains("href=\"/paea.html\""));
    assert_eq!(snapshot(&fixture.root), before);
}

/// Distinguish JSON decoding from admission failures on both public data routes.
/// A valid control precedes each canary/type or conflicting-version mutation;
/// public Display/Debug/body redact input while the original typed cause survives.
/// Recovery preserves the exact response and all Root bytes; no effect is tested.
#[test]
fn paea_envelope_type_and_conflicting_duplicates_are_redacted_and_recover() {
    let fixture = Fixture::new();
    let before = snapshot(&fixture.root);
    const CANARY: &str = "GLM_PAEA_PRIVATE_CANARY";
    for (path, body) in [
        ("/api/v1/protection/receipt-report", input(0)),
        ("/api/v1/protection/resolve-payment", payment_input()),
    ] {
        let raw = serde_json::to_vec(&body).unwrap();
        let text = std::str::from_utf8(&raw).unwrap();
        let mut wrong_type = body.clone();
        wrong_type["schema_version"] = json!(CANARY);
        let mut unknown = body.clone();
        unknown[CANARY] = json!(true);
        let invalid_version = text.replacen("\"schema_version\":1", "\"schema_version\":2", 1);
        assert_ne!(invalid_version, text);
        for (fault, changed, reflected_cause) in [
            ("type", serde_json::to_vec(&wrong_type).unwrap(), true),
            ("unknown", serde_json::to_vec(&unknown).unwrap(), true),
            (
                "invalid-first",
                format!("{{\"schema_version\":2,{}", &text[1..]).into_bytes(),
                false,
            ),
            (
                "invalid-last",
                format!("{{\"schema_version\":1,{}", &invalid_version[1..]).into_bytes(),
                false,
            ),
            (
                "escaped-duplicate",
                format!("{{\"\\u0073chema_version\":2,{}", &text[1..]).into_bytes(),
                false,
            ),
        ] {
            let control = fixture.call(fixture.request("POST", path, raw.clone()));
            assert_eq!(control.response.status(), StatusCode::OK, "{path} {fault}");
            assert!(control.failure.is_none());
            let refused = fixture.call(fixture.request("POST", path, changed));
            assert_eq!(
                refused.response.status(),
                StatusCode::BAD_REQUEST,
                "{path} {fault}"
            );
            let error = refused.failure.as_ref().unwrap();
            assert!(
                matches!(error, swarm_web::HttpServiceError::Json(_)),
                "{path} {fault}"
            );
            assert_eq!(error.code(), "web-document-invalid");
            let cause = error
                .source()
                .unwrap()
                .downcast_ref::<serde_json::Error>()
                .unwrap();
            assert!(cause.is_data(), "{path} {fault}");
            if reflected_cause {
                assert!(cause.to_string().contains(CANARY), "{path} {fault}");
            } else {
                assert!(
                    cause.to_string().contains("duplicate field"),
                    "{path} {fault}"
                );
            }
            assert!(!error.to_string().contains(CANARY));
            assert!(!format!("{error:?}").contains(CANARY));
            assert!(!String::from_utf8_lossy(refused.response.body()).contains(CANARY));
            assert!(!String::from_utf8_lossy(refused.response.body()).contains("duplicate field"));
            assert_eq!(snapshot(&fixture.root), before);
            let recovery = fixture.call(fixture.request("POST", path, raw.clone()));
            assert_eq!(recovery.response.status(), StatusCode::OK);
            assert!(recovery.failure.is_none());
            assert_eq!(recovery.response.body(), control.response.body());
            assert_eq!(snapshot(&fixture.root), before);
        }
    }
}

/// Reuse a frozen valid action/context document as exact comparison strings.
/// Inner numbers remain JSON integer text; no Root path or authority is supplied.
fn comparison_input(index: usize) -> Value {
    let document = input(index)["document"].as_str().unwrap().to_owned();
    json!({"schema_version":1,"before":document,"after":document})
}

/// Existing receipt admission controls the fixture before the new comparison
/// route. Both domains cover equality, transport-only, terms, context and formatting;
/// require exact shared v1/CLI envelope, raw hashes and no Root mutation/effect.
#[test]
fn paea_comparison_http_matches_shared_report_and_preserves_root() {
    let fixture = Fixture::new();
    let control = fixture.call(fixture.request(
        "POST",
        "/api/v1/protection/receipt-report",
        serde_json::to_vec(&input(0)).unwrap(),
    ));
    assert_eq!(control.response.status(), StatusCode::OK);
    assert!(control.failure.is_none());
    println!("VALID_RECEIPT_CONTROL_BEFORE_COMPARISON");
    let before_root = snapshot(&fixture.root);
    for index in [0, 8] {
        for variation in ["same", "request", "terms", "context", "formatting"] {
            let mut body = comparison_input(index);
            let mut before: Value = serde_json::from_str(body["before"].as_str().unwrap()).unwrap();
            before["context"]["mandate_revision"] = json!(i64::MAX);
            let mut after = before.clone();
            match variation {
                "request" => after["proposal"]["request_id"] = json!("comparison-attempt-2"),
                "terms" if index == 0 => {
                    after["proposal"]["action"]["amount_minor"] = json!("420001")
                }
                "terms" => {
                    let oid = after["proposal"]["action"]["candidate_oid"]
                        .as_str()
                        .unwrap();
                    let replacement = if oid.chars().all(|c| c == 'e') {
                        "d"
                    } else {
                        "e"
                    }
                    .repeat(oid.len());
                    after["proposal"]["action"]["candidate_oid"] = json!(replacement);
                }
                "context" => after["context"]["policy_revision"] = json!(17),
                _ => {}
            }
            body["before"] = json!(serde_json::to_string(&before).unwrap());
            body["after"] = json!(if variation == "formatting" {
                serde_json::to_string_pretty(&after).unwrap()
            } else {
                serde_json::to_string(&after).unwrap()
            });
            let comparison = swarm_protection_contract::compare_documents(
                body["before"].as_str().unwrap().as_bytes(),
                body["after"].as_str().unwrap().as_bytes(),
            )
            .unwrap();
            assert_eq!(
                comparison.canonical_content_changed,
                matches!(variation, "terms" | "context")
            );
            assert_eq!(comparison.transport_request_changed, variation == "request");
            assert!(!comparison.intent_changed);
            let expected = serde_json::to_vec(
                &json!({"schema_version":1,"scope":"offline-data-only","comparison":comparison}),
            )
            .unwrap();
            let reply = fixture.call(fixture.request(
                "POST",
                "/api/v1/protection/compare",
                serde_json::to_vec(&body).unwrap(),
            ));
            assert_eq!(
                reply.response.status(),
                StatusCode::OK,
                "{index} {variation}; fixture {}",
                fixture.root.display()
            );
            assert!(reply.failure.is_none());
            assert_eq!(reply.response.body(), &expected);
            assert_eq!(snapshot(&fixture.root), before_root);
        }
    }
}

/// Each actual comparison refusal follows a matching admission control and is
/// followed by exact recovery. Preserve safe diagnostics and original typed causes,
/// all Root bytes and the private embedded-page boundary; no live state/effect.
#[test]
fn paea_comparison_http_refusals_recover_without_root_mutation() {
    let fixture = Fixture::new();
    let path = "/api/v1/protection/compare";
    let raw = serde_json::to_vec(&comparison_input(0)).unwrap();
    let before = snapshot(&fixture.root);
    for fault in [
        "cookie",
        "csrf",
        "host",
        "origin",
        "method",
        "query",
        "type",
        "version",
        "duplicate",
        "unknown",
        "wrong-type",
        "invalid-before",
        "invalid-after",
        "inner-duplicate",
        "inner-float",
    ] {
        let control = fixture.call(fixture.request("POST", path, raw.clone()));
        assert_eq!(control.response.status(), StatusCode::OK, "control {fault}");
        assert!(control.failure.is_none());
        let mut request = fixture.request("POST", path, raw.clone());
        match fault {
            "cookie" => {
                request.headers_mut().remove(COOKIE);
            }
            "csrf" => {
                request.headers_mut().remove("x-swarm-csrf");
            }
            "host" => {
                request
                    .headers_mut()
                    .insert(HOST, "outside.invalid".parse().unwrap());
            }
            "origin" => {
                request
                    .headers_mut()
                    .insert(ORIGIN, "http://outside.invalid".parse().unwrap());
            }
            "method" => *request.method_mut() = http::Method::GET,
            "query" => {
                *request.uri_mut() = format!("{path}?root=PRIVATE_COMPARISON_CANARY")
                    .parse()
                    .unwrap()
            }
            "type" => {
                request.headers_mut().remove("content-type");
            }
            "duplicate" => {
                *request.body_mut() = format!(
                    "{{\"schema_version\":2,{}",
                    &std::str::from_utf8(&raw).unwrap()[1..]
                )
                .into_bytes()
            }
            _ => {
                let mut body = comparison_input(0);
                match fault {
                    "version" => body["schema_version"] = json!(2),
                    "unknown" => body["PRIVATE_COMPARISON_CANARY"] = json!(true),
                    "wrong-type" => body["schema_version"] = json!("PRIVATE_COMPARISON_CANARY"),
                    "invalid-before" => body["before"] = json!("PRIVATE_COMPARISON_CANARY"),
                    "invalid-after" => body["after"] = json!("PRIVATE_COMPARISON_CANARY"),
                    "inner-duplicate" => {
                        let value = body["before"].as_str().unwrap();
                        let changed = value.replacen(
                            "\"schema_version\":1",
                            "\"schema_version\":1,\"schema_version\":2",
                            1,
                        );
                        assert_ne!(value, changed);
                        body["before"] = json!(changed);
                    }
                    "inner-float" => {
                        let mut value: Value =
                            serde_json::from_str(body["after"].as_str().unwrap()).unwrap();
                        value["context"]["policy_revision"] = json!(1.5);
                        body["after"] = json!(serde_json::to_string(&value).unwrap());
                    }
                    _ => unreachable!(),
                }
                *request.body_mut() = serde_json::to_vec(&body).unwrap();
            }
        }
        let reply = fixture.call(request);
        assert!(!reply.response.status().is_success(), "{fault}");
        let error = reply.failure.as_ref().unwrap();
        assert!(!error.to_string().contains("PRIVATE_COMPARISON_CANARY"));
        assert!(!format!("{error:?}").contains("PRIVATE_COMPARISON_CANARY"));
        assert!(
            !String::from_utf8_lossy(reply.response.body()).contains("PRIVATE_COMPARISON_CANARY")
        );
        if [
            "duplicate",
            "unknown",
            "wrong-type",
            "invalid-before",
            "invalid-after",
            "inner-duplicate",
            "inner-float",
        ]
        .contains(&fault)
        {
            assert!(error.source().is_some(), "{fault}");
        }
        assert_eq!(snapshot(&fixture.root), before);
        let recovery = fixture.call(fixture.request("POST", path, raw.clone()));
        assert_eq!(recovery.response.status(), StatusCode::OK);
        assert_eq!(recovery.response.body(), control.response.body());
        assert_eq!(snapshot(&fixture.root), before);
    }
    let mut denied = fixture.request("GET", "/paea-compare.html", Vec::new());
    denied.headers_mut().remove(COOKIE);
    assert!(!fixture.call(denied).response.status().is_success());
    let page = fixture.call(fixture.request("GET", "/paea-compare.html", Vec::new()));
    assert_eq!(page.response.status(), StatusCode::OK);
    assert_eq!(
        page.response.body(),
        include_bytes!("../../../../web/paea-compare.html")
    );
    let menu = fixture.call(fixture.request("GET", "/paea.html", Vec::new()));
    assert!(String::from_utf8_lossy(menu.response.body()).contains("href=\"/paea-compare.html\""));
    assert_eq!(snapshot(&fixture.root), before);
}


/// Serve only versioned fictional source assets after normal native admission.
/// A valid receipt precedes exact GET, denied-session, HEAD and method/recovery
/// checks; all Root bytes remain unchanged. Then consume downloaded bytes through
/// the actual shared receipt/comparison/resolver, without authority or effect.
#[test]
fn paea_example_assets_are_private_exact_and_semantically_usable() {
    let fixture = Fixture::new();
    let baseline = snapshot(&fixture.root);
    let control = fixture.call(fixture.request("POST", "/api/v1/protection/receipt-report", serde_json::to_vec(&input(0)).unwrap()));
    assert_eq!(control.response.status(), StatusCode::OK);
    let mut assets = BTreeMap::new();
    for (path, bytes) in [
        ("/paea-examples/v1/git-after.json", include_bytes!("../../../../web/paea-examples/v1/git-after.json").as_slice()),
        ("/paea-examples/v1/git-document.json", include_bytes!("../../../../web/paea-examples/v1/git-document.json").as_slice()),
        ("/paea-examples/v1/git-receipt.json", include_bytes!("../../../../web/paea-examples/v1/git-receipt.json").as_slice()),
        ("/paea-examples/v1/payment-after.json", include_bytes!("../../../../web/paea-examples/v1/payment-after.json").as_slice()),
        ("/paea-examples/v1/payment-document.json", include_bytes!("../../../../web/paea-examples/v1/payment-document.json").as_slice()),
        ("/paea-examples/v1/payment-receipt.json", include_bytes!("../../../../web/paea-examples/v1/payment-receipt.json").as_slice()),
        ("/paea-examples/v1/payment-registry.json", include_bytes!("../../../../web/paea-examples/v1/payment-registry.json").as_slice()),
        ("/paea-examples/v1/payment-request.json", include_bytes!("../../../../web/paea-examples/v1/payment-request.json").as_slice()),
    ] {
        let valid = fixture.call(fixture.request("GET", path, vec![]));
        assert_eq!(valid.response.status(), StatusCode::OK, "{path}");
        assert_eq!(valid.response.body(), bytes);
        assert_eq!(valid.response.headers()["content-type"], "application/json; charset=utf-8");
        assets.insert(path.rsplit('/').next().unwrap(), bytes);
        let mut denied = fixture.request("GET", path, vec![]);
        denied.headers_mut().remove(COOKIE);
        assert_eq!(fixture.call(denied).response.status(), StatusCode::FORBIDDEN);
        let head = fixture.call(fixture.request("HEAD", path, vec![]));
        assert_eq!(head.response.status(), StatusCode::OK);
        assert!(head.response.body().is_empty());
        assert!(!fixture.call(fixture.request("POST", path, vec![])).response.status().is_success());
        assert_eq!(snapshot(&fixture.root), baseline);
        assert_eq!(fixture.call(fixture.request("GET", path, vec![])).response.body(), bytes);
    }
    for prefix in ["payment", "git"] {
        let document = assets[format!("{prefix}-document.json").as_str()];
        let receipt = assets[format!("{prefix}-receipt.json").as_str()];
        let report = swarm_protection_contract::check_receipt_documents(document, receipt, "sink-paea-example", swarm_protection_contract::State::CommitRequested).unwrap();
        assert_eq!(report.effect, "not-established");
        let comparison = swarm_protection_contract::compare_documents(document, assets[format!("{prefix}-after.json").as_str()]).unwrap();
        assert!(comparison.canonical_content_changed);
        assert!(!comparison.intent_changed);
        assert!(!comparison.transport_request_changed);
        assert_eq!(comparison.approval_validity, "not-established");
    }
    let payment = swarm_protection_contract::resolve_payment_documents(assets["payment-registry.json"], assets["payment-request.json"]).unwrap();
    assert_eq!(payment.effect, "not-established");
    assert_eq!(snapshot(&fixture.root), baseline);
    println!("PAEA_EIGHT_PRIVATE_EXAMPLES_USABLE");
}
