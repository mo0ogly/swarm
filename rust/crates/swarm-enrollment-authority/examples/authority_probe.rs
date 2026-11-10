//! Deterministic protocol fixture; excluded from builds without test-fixtures.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::SigningKey;
use serde::Deserialize;
use serde_json::json;
use std::io::{self, BufRead, Cursor};
use swarm_enrollment_authority::publication::{self, Publication};
use swarm_enrollment_authority::{
    decode, decode_active, encode, sign_enrollment, Document, Error, Expected,
};
use swarm_enrollment_proof::{Request, Verifier};
use swarm_enrollment_state::Head;
use swarm_identity_contract::{Principal, Scope};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
#[derive(Deserialize)]
struct ExpectedWire {
    #[serde(rename = "Head")]
    head: Head,
    #[serde(rename = "EnrollmentID")]
    id: String,
    #[serde(rename = "Generation")]
    generation: i64,
    #[serde(rename = "ProjectDigest")]
    project: String,
    #[serde(rename = "Principal")]
    principal: Principal,
    #[serde(rename = "Scope")]
    scope: Scope,
    #[serde(rename = "TrustRevision")]
    trust: i64,
    #[serde(rename = "RequestDigest")]
    request: String,
}
impl From<ExpectedWire> for Expected {
    fn from(v: ExpectedWire) -> Self {
        Self {
            head: v.head,
            enrollment_id: v.id,
            generation: v.generation,
            project_digest: v.project,
            principal: v.principal,
            scope: v.scope,
            trust_revision: v.trust,
            request_digest: v.request,
        }
    }
}
#[derive(Deserialize)]
struct Vector {
    name: String,
    mode: String,
    input: String,
    base: Head,
    request: Request,
    response: String,
    expected: ExpectedWire,
    at: String,
}
fn hex(raw: &[u8]) -> String {
    raw.iter().map(|b| format!("{b:02x}")).collect()
}
fn unhex(raw: &str) -> Vec<u8> {
    assert_eq!(raw.len() % 2, 0);
    (0..raw.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&raw[i..i + 2], 16).unwrap())
        .collect()
}
fn fixture_document(raw: &[u8]) -> Result<Document, Error> {
    let mut value: serde_json::Value = serde_json::from_slice(raw).map_err(|_| Error::Document)?;
    // Go's typed nil slices have no distinct Rust Vec constructor. Normalize only
    // this test transport; the production Decode still requires exact canonical [] .
    for field in ["permissions", "delegable"] {
        if value[field].is_null() {
            value[field] = json!([])
        }
    }
    serde_json::from_value(value).map_err(|_| Error::Document)
}
fn evaluate_authority(v: Vector) -> Result<Vec<u8>, Error> {
    match v.mode.as_str() {
        "encode" => encode(&fixture_document(&unhex(&v.input))?),
        "decode" => encode(&decode(&unhex(&v.input))?),
        "sign" => sign_enrollment(
            &SigningKey::from_bytes(&[7; 32]),
            [11; 32],
            &v.base,
            &fixture_document(&unhex(&v.input))?,
        ),
        "active" => {
            let key = SigningKey::from_bytes(&[7; 32]);
            let verifier = Verifier::new(&key.verifying_key().to_bytes(), [11; 32]).unwrap();
            let nonce = URL_SAFE_NO_PAD.decode(&v.request.nonce).unwrap();
            let challenge = verifier
                .challenge_with_reader([13; 32], &mut Cursor::new(nonce))
                .unwrap();
            assert_eq!(challenge.request(), v.request);
            let statement = verifier.verify(&challenge, &unhex(&v.response)).unwrap();
            let at = OffsetDateTime::parse(&v.at, &Rfc3339).unwrap();
            encode(&decode_active(&statement, &v.expected.into(), at)?)
        }
        _ => panic!("unknown fixture mode"),
    }
}
// Error codes are the public comparison surface; original Go causes are retained
// in its native tests and Rust error chaining is covered by native regressions.
fn evaluate(v: Vector) -> Result<(Vec<u8>, String), &'static str> {
    if !v.mode.starts_with("publication-") {
        return evaluate_authority(v)
            .map(|raw| (raw, String::new()))
            .map_err(|e| e.code());
    }
    let raw = unhex(&v.input);
    let result = (|| -> Result<(Vec<u8>, String), publication::Error> {
        let prepared = if v.mode == "publication-decode" {
            publication::decode(&raw)?
        } else {
            let p: Publication =
                serde_json::from_slice(&raw).map_err(|_| publication::Error::Request)?;
            publication::prepare(&p)?
        };
        match v.mode.as_str() {
            "publication-prepare" | "publication-decode" => {
                Ok((prepared.bytes(), hex(&prepared.digest())))
            }
            "publication-bind" => {
                prepared.bind(&v.request)?;
                Ok((Vec::new(), String::new()))
            }
            "publication-active" => {
                let key = SigningKey::from_bytes(&[7; 32]);
                let verifier = Verifier::new(&key.verifying_key().to_bytes(), [11; 32]).unwrap();
                let nonce = URL_SAFE_NO_PAD.decode(&v.request.nonce).unwrap();
                let challenge = verifier
                    .challenge_with_reader(prepared.digest(), &mut Cursor::new(nonce))
                    .unwrap();
                assert_eq!(challenge.request(), v.request);
                let statement = verifier.verify(&challenge, &unhex(&v.response)).unwrap();
                let at = OffsetDateTime::parse(&v.at, &Rfc3339).unwrap();
                let doc = prepared.check_active(&statement, at)?;
                let bytes = encode(&doc).map_err(publication::Error::Authority)?;
                Ok((bytes, String::new()))
            }
            _ => panic!("unknown publication fixture mode"),
        }
    })();
    result.map_err(|e| e.code())
}
fn main() {
    for line in io::stdin().lock().lines() {
        let v: Vector = serde_json::from_str(&line.unwrap()).unwrap();
        let name = v.name.clone();
        let (code, output, digest) = match evaluate(v) {
            Ok((raw, digest)) => ("ok", hex(&raw), digest),
            Err(code) => (code, String::new(), String::new()),
        };
        println!(
            "{}",
            json!({"name":name,"code":code,"output":output,"digest":digest})
        );
    }
}
