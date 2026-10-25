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
