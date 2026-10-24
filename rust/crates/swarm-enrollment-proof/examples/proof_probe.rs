//! Fixture-only probe: test-features required, never a product entry point.
#![forbid(unsafe_code)]
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{self, BufRead, Cursor, Write};
use swarm_enrollment_proof::{fixture_json_string, sign_response, Request, Statement, Verifier};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Vector {
    name: String,
    mode: String,
    seed: String,
    key: String,
    project: String,
    request: Request,
    payload: String,
    valid: String,
    input: String,
    expected: String,
    recovery: String,
    replay: String,
}

#[derive(Serialize)]
struct ResultLine {
    name: String,
    code: String,
    recovery: String,
    replay: String,
    signature_equal: bool,
    payload_equal: bool,
    request_equal: bool,
}

fn decode_hex(value: &str) -> Result<Vec<u8>, String> {
    if !value.len().is_multiple_of(2) || !value.is_ascii() {
        return Err("invalid fixture hex".into());
    }
    (0..value.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&value[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

fn code(result: &Result<Statement, swarm_enrollment_proof::Error>) -> String {
    match result {
        Ok(_) => "ok",
        Err(error) => error.code(),
    }
    .into()
}

fn check(vector: Vector) -> Result<ResultLine, String> {
    let seed: [u8; 32] = decode_hex(&vector.seed)?
        .try_into()
        .map_err(|_| "seed size")?;
    let key = SigningKey::from_bytes(&seed);
    let payload = decode_hex(&vector.payload)?;
    let signed = sign_response(&vector.request, &payload, &key);
    if vector.mode == "sign" {
        let expected_bytes = decode_hex(&vector.valid)?;
        return Ok(ResultLine {
            name: vector.name,
            code: signed
                .as_ref()
                .map(|_| "ok")
                .unwrap_or_else(|e| e.code())
                .into(),
            recovery: String::new(),
            replay: String::new(),
            signature_equal: signed
                .as_ref()
                .map(|bytes| bytes == &expected_bytes)
                .unwrap_or_else(|_| vector.valid.is_empty()),
            payload_equal: true,
            request_equal: true,
        });
    }
    if vector.mode != "verify" {
        return Err("unknown fixture operation".into());
    }
    let project = decode_hex(&vector.project)?
        .try_into()
        .map_err(|_| "project size")?;
    let verifier = Verifier::new(&decode_hex(&vector.key)?, project).map_err(|e| e.to_string())?;
    let request_digest = decode_hex(&vector.request.request_digest)?
        .try_into()
        .map_err(|_| "request digest size")?;
    let nonce = URL_SAFE_NO_PAD
        .decode(&vector.request.nonce)
        .map_err(|e| e.to_string())?;
    let valid = decode_hex(&vector.valid)?;
    let control = verifier
        .challenge_with_reader(request_digest, &mut Cursor::new(&nonce))
        .map_err(|e| e.to_string())?;
    if control.request() != vector.request {
        return Err("fixture request did not reconstruct exactly".into());
    }
    verifier
        .verify(&control, &valid)
        .map_err(|e| format!("invalid positive control {}: {e}", vector.name))?;
    let challenge = verifier
        .challenge_with_reader(request_digest, &mut Cursor::new(&nonce))
        .map_err(|e| e.to_string())?;
    let first = verifier.verify(&challenge, &decode_hex(&vector.input)?);
    let recovery = verifier.verify(&challenge, &valid);
    let replay = verifier.verify(&challenge, &valid);
    let first_code = code(&first);
    let recovery_code = code(&recovery);
    let replay_code = code(&replay);
    let statement = first.as_ref().ok().or_else(|| recovery.as_ref().ok());
    let result = ResultLine {
        name: vector.name,
        code: first_code,
        recovery: recovery_code,
        replay: replay_code,
        signature_equal: signed.map(|bytes| bytes == valid).unwrap_or(false),
        payload_equal: statement.map(|s| s.payload() == payload).unwrap_or(false),
        request_equal: statement
            .map(|s| s.request() == vector.request)
            .unwrap_or(false),
    };
    // Expected codes are compared by the Python driver, preserving every mismatch.
    let _ = (vector.expected, vector.recovery, vector.replay);
    Ok(result)
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--json-table"] {
        return json_table();
    }
    if !args.is_empty() {
        return Err("unknown fixture argument".into());
    }
    let stdout = io::stdout();
    let mut output = io::BufWriter::new(stdout.lock());
    for line in io::stdin().lock().lines() {
        let vector =
            serde_json::from_str(&line.map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        let result = check(vector)?;
        serde_json::to_writer(&mut output, &result).map_err(|e| e.to_string())?;
        output.write_all(b"\n").map_err(|e| e.to_string())?;
    }
    output.flush().map_err(|e| e.to_string())
}

fn json_table() -> Result<(), String> {
    let mut hash = Sha256::new();
    let mut count = 0_u64;
    let mut check = |raw: &[u8]| {
        count += 1;
        match fixture_json_string(raw) {
            Ok(value) => {
                hash.update(b"1");
                hash.update(value);
                hash.update(b"\n");
            }
            Err(_) => hash.update(b"0\n"),
        }
    };
    for value in (0..=0x10ffff).filter_map(char::from_u32) {
        let raw = format!("\"{value}\"");
        check(raw.as_bytes());
    }
    for a in 0..=255 {
        for b in 0..=255 {
            check(&[b'"', a, b, b'"']);
        }
    }
    for unit in 0..=0xffff {
        check(format!("\"\\u{unit:04x}\"").as_bytes());
    }
    serde_json::to_writer(
        io::stdout(),
        &serde_json::json!({"string_cases":count,"sha256":format!("{:x}", hash.finalize())}),
    )
    .map_err(|e| e.to_string())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}
