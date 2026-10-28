//! Native enrollment challenge authentication, compatible with Go's proof wire format.
//!
//! A statement authenticates bytes, not enrollment, freshness or business rights.
//! Its public key must come from independent trusted bootstrap. Process-local
//! single use is neither a durable reservation nor a process-start capability.
#![forbid(unsafe_code)]

use swarm_wire_json as wire_json;

/// Differential fixture hook, absent from default builds. No authority is created.
#[cfg(feature = "test-fixtures")]
pub fn fixture_json_string(raw: &[u8]) -> Result<Vec<u8>, Error> {
    let value: String = wire_json::decode(raw).map_err(|_| Error::Response)?;
    wire_json::encode(&value).map_err(|_| Error::Response)
}

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier as _, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::sync::{Arc, Mutex};
use swarm_identity_contract::digest;

pub const PROTOCOL: &str = "swarm-enrollment-statement-v1";

#[derive(Debug)]
pub enum Error {
    Key,
    Challenge,
    Spent,
    Response,
    Binding,
    Signature,
    Entropy(getrandom::Error),
    Io(std::io::Error),
    State,
}

impl Error {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Key => "key",
            Self::Challenge => "challenge",
            Self::Spent => "spent",
            Self::Response => "response",
            Self::Binding => "binding",
            Self::Signature => "signature",
            Self::Entropy(_) | Self::Io(_) => "entropy",
            Self::State => "state",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Key => "independent pinned key required",
            Self::Challenge => "challenge does not belong to this verifier",
            Self::Spent => "challenge already consumed",
            Self::Response => "malformed response",
            Self::Binding => "response belongs to another challenge",
            Self::Signature => "invalid signature",
            Self::Entropy(error) => return write!(output, "enrollment proof entropy: {error}"),
            Self::Io(error) => return write!(output, "enrollment proof entropy: {error}"),
            Self::State => "challenge state unavailable",
        };
        write!(output, "enrollment proof: {message}")
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Entropy(error) => Some(error),
            _ => None,
        }
    }
}

/// Public request data; none of these declared fields conveys authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub protocol: String,
    #[serde(rename = "authority_key_id")]
    pub key_id: String,
    #[serde(rename = "project_context_digest")]
    pub project_digest: String,
    pub request_digest: String,
    pub nonce: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Response {
    protocol: String,
    challenge_digest: String,
    payload: String,
    signature: String,
}

struct VerifierState {
    key: [u8; 32],
    project: [u8; 32],
}

/// Cloning retains the same verifier identity, as copying a Go verifier pointer.
#[derive(Clone)]
pub struct Verifier {
    state: Arc<VerifierState>,
}

struct ChallengeState {
    owner: Arc<VerifierState>,
    request: Request,
    spent: Mutex<bool>,
}

/// Clones share consumption state. No deserialize or public constructor exists.
#[derive(Clone)]
pub struct Challenge {
    state: Arc<ChallengeState>,
}

impl Challenge {
    pub fn request(&self) -> Request {
        self.state.request.clone()
    }
}

/// Authenticated payload/context only; callers must validate schema and current rights.
#[derive(Clone)]
pub struct Statement {
    payload: Vec<u8>,
    request: Request,
}

impl Statement {
    pub fn payload(&self) -> Vec<u8> {
        self.payload.clone()
    }

    pub fn request(&self) -> Request {
        self.request.clone()
    }
}

fn hash_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn digest_request(request: &Request) -> Result<String, Error> {
    let bytes = wire_json::encode(request).map_err(|_| Error::Response)?;
    Ok(hash_hex(&bytes))
}

fn signed_bytes(response: &Response) -> Result<Vec<u8>, Error> {
    let unsigned = Response {
        protocol: response.protocol.clone(),
        challenge_digest: response.challenge_digest.clone(),
        payload: response.payload.clone(),
        signature: String::new(),
    };
    let mut output = format!("{PROTOCOL}\0response\0").into_bytes();
    output.extend(wire_json::encode(&unsigned).map_err(|_| Error::Response)?);
    Ok(output)
}

impl Verifier {
    pub fn new(key: &[u8], project: [u8; 32]) -> Result<Self, Error> {
        let key = key.try_into().map_err(|_| Error::Key)?;
        Ok(Self {
            state: Arc::new(VerifierState { key, project }),
        })
    }

    fn challenge_from_nonce(&self, request_digest: [u8; 32], nonce: [u8; 32]) -> Challenge {
        Challenge {
            state: Arc::new(ChallengeState {
                owner: self.state.clone(),
                request: Request {
                    protocol: PROTOCOL.into(),
                    key_id: hash_hex(&self.state.key),
                    project_digest: self
                        .state
                        .project
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect(),
                    request_digest: request_digest.iter().map(|b| format!("{b:02x}")).collect(),
                    nonce: URL_SAFE_NO_PAD.encode(nonce),
                },
                spent: Mutex::new(false),
            }),
        }
    }

    pub fn new_challenge(&self, request_digest: [u8; 32]) -> Result<Challenge, Error> {
        let mut nonce = [0; 32];
        getrandom::getrandom(&mut nonce).map_err(Error::Entropy)?;
        Ok(self.challenge_from_nonce(request_digest, nonce))
    }

    /// Deterministic entropy injection is absent from the default build.
    #[cfg(feature = "test-fixtures")]
    pub fn challenge_with_reader(
        &self,
        request_digest: [u8; 32],
        reader: &mut impl std::io::Read,
    ) -> Result<Challenge, Error> {
        let mut nonce = [0; 32];
        reader.read_exact(&mut nonce).map_err(Error::Io)?;
        Ok(self.challenge_from_nonce(request_digest, nonce))
    }

    pub fn verify(&self, challenge: &Challenge, raw: &[u8]) -> Result<Statement, Error> {
        if !Arc::ptr_eq(&self.state, &challenge.state.owner) {
            return Err(Error::Challenge);
        }
        let mut spent = challenge.state.spent.lock().map_err(|_| Error::State)?;
        if *spent {
            return Err(Error::Spent);
        }
        let response: Response = wire_json::decode(raw).map_err(|_| Error::Response)?;
        if response.protocol != PROTOCOL || !digest(response.challenge_digest.as_bytes()) {
            return Err(Error::Response);
        }
        if response.challenge_digest != digest_request(&challenge.state.request)? {
            return Err(Error::Binding);
        }
        let signature = URL_SAFE_NO_PAD
            .decode(&response.signature)
            .map_err(|_| Error::Signature)?;
        if URL_SAFE_NO_PAD.encode(&signature) != response.signature {
            return Err(Error::Signature);
        }
        let signature = Signature::from_slice(&signature).map_err(|_| Error::Signature)?;
        let key = VerifyingKey::from_bytes(&self.state.key).map_err(|_| Error::Signature)?;
        // Match Go's non-batch ed25519.Verify. Never silently switch acceptance
        // rules to verify_strict during a wire-compatibility migration.
        key.verify(&signed_bytes(&response)?, &signature)
            .map_err(|_| Error::Signature)?;
        let payload = URL_SAFE_NO_PAD
            .decode(&response.payload)
            .map_err(|_| Error::Response)?;
        if URL_SAFE_NO_PAD.encode(&payload) != response.payload {
            return Err(Error::Response);
        }
        *spent = true;
        Ok(Statement {
            payload,
            request: challenge.state.request.clone(),
        })
    }
}

/// Signs explicitly supplied payload; this makes no enrollment/authorization decision.
/// Key import/provisioning is outside this typed API, as is current-state validation.
pub fn sign_response(
    request: &Request,
    payload: &[u8],
    key: &SigningKey,
) -> Result<Vec<u8>, Error> {
    if request.protocol != PROTOCOL
        || request.key_id != hash_hex(key.verifying_key().as_bytes())
        || !digest(request.project_digest.as_bytes())
        || !digest(request.request_digest.as_bytes())
    {
        return Err(Error::Challenge);
    }
    let nonce = URL_SAFE_NO_PAD
        .decode(&request.nonce)
        .map_err(|_| Error::Challenge)?;
    if nonce.len() != 32 || URL_SAFE_NO_PAD.encode(nonce) != request.nonce {
        return Err(Error::Challenge);
    }
    let mut response = Response {
        protocol: PROTOCOL.into(),
        challenge_digest: digest_request(request)?,
        payload: URL_SAFE_NO_PAD.encode(payload),
        signature: String::new(),
    };
    response.signature = URL_SAFE_NO_PAD.encode(key.sign(&signed_bytes(&response)?).to_bytes());
    wire_json::encode(&response).map_err(|_| Error::Response)
}
