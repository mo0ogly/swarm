//! Personal-secret derivation and exact-record knowledge checking, without storage,
//! bootstrap consumption, sessions or business authority. Independent provisioning
//! and current/revocation checks remain mandatory at the protected consumer.
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error as StdError, fmt};
use subtle::ConstantTimeEq;
use swarm_identity_contract::{digest, Principal};
use swarm_wire_json as wire;
use zeroize::Zeroizing;

/// Explicit operator KDF costs, persisted in each canonical credential. No Swarm
/// defaults or second policy ceiling: validate with Argon2's actual parameter
/// contract. A service must manage the same values through its settings before
/// exposing authentication. Low algorithm-valid costs need not be secure policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub memory_kib: u32,
    pub passes: u32,
    pub parallelism: u32,
}
impl Parameters {
    /// Validate representation/algorithm bounds only, without allocation or work.
    /// This is not a security recommendation or approval of these operator costs.
    pub fn validate(self) -> Result<(), Error> {
        self.params().map(|_| ())
    }
    /// Convert the persisted costs with a fixed 32-byte protocol output length.
    /// Invalid combinations retain the concrete Argon2 parameter cause.
    fn params(self) -> Result<Params, Error> {
        Params::new(self.memory_kib, self.passes, self.parallelism, Some(32)).map_err(Error::Crypto)
    }
}
/// Private verifier material, not a password or an authenticated identity.
/// Only exact canonical encoding is supported; protect records and their pins
/// outside the agent channel. Debug deliberately excludes salt and derived value.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub schema_version: i64,
    pub algorithm: String,
    pub installation_digest: String,
    pub principal: Principal,
    pub generation: i64,
    pub parameters: Parameters,
    pub salt: String,
    pub derived: String,
}
impl fmt::Debug for Record {
    /// Never print verifier material, even in error/debug diagnostics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PersonalCredential { <private> }")
    }
}
/// Safe refusal category with original typed causes; no password/verifier/path
/// is formatted by Debug/Display. Callers must not log private source text.
pub enum Error {
    Record,
    Binding,
    Password,
    Empty,
    Crypto(argon2::Error),
    Allocation(std::collections::TryReserveError),
    Random(getrandom::Error),
    Frame(serde_json::Error),
    Encoding(base64::DecodeError),
}
impl Error {
    /// Stable safe category; mismatch is distinct from malformed data/failed work.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Record => "credential-record",
            Self::Binding => "credential-binding",
            Self::Password => "password",
            Self::Empty => "empty-password",
            Self::Crypto(_) => "credential-crypto",
            Self::Allocation(_) => "credential-allocation",
            Self::Random(_) => "credential-random",
            Self::Frame(_) => "credential-json",
            Self::Encoding(_) => "credential-encoding",
        }
    }
}
impl fmt::Display for Error {
    /// No private input or derived material appears in this public diagnostic.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "personal credential: {}", self.code())
    }
}
impl fmt::Debug for Error {
    /// Debug uses the same redacted category as Display, preserving causes typed.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Error {
    /// Preserve actual algorithm, entropy or framing failures for safe traversal.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Crypto(e) => Some(e),
            Self::Allocation(e) => Some(e),
            Self::Random(e) => Some(e),
            Self::Frame(e) => Some(e),
            Self::Encoding(e) => Some(e),
            _ => None,
        }
    }
}
/// Knowledge of one exact credential, borrowed only after successful comparison.
/// Not Clone/Serialize and has no public constructor. This is not a session,
/// identity enrollment, proof of actual human presence, grant or transferable
/// action token. Consumers must still hold current protected account authority.
pub struct Proof<'a> {
    record: &'a Record,
    pin: String,
}
impl fmt::Debug for Proof<'_> {
    /// Do not disclose identity or verifier data from a successful comparison.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PersonalCredentialKnowledge { <private> }")
    }
}
impl Proof<'_> {
    /// Installation bound by this exact verified record. This is knowledge data,
    /// not current protected account authority or a transferable permit. A trusted
    /// consumer must still hold its configured account source during business use.
    pub fn installation_digest(&self) -> &str {
        &self.record.installation_digest
    }

    /// Principal bound by the independently selected record, not request labels.
    pub fn principal(&self) -> &Principal {
        &self.record.principal
    }
    /// Exact credential generation; current/revoked status is not implied.
    pub fn generation(&self) -> i64 {
        self.record.generation
    }
    /// Domain-separated digest that was independently expected by verification.
    pub fn record_digest(&self) -> &str {
        &self.pin
    }
}
/// Decode exactly 32 canonical base64url bytes. Length is a cryptographic wire
/// invariant, never an execution/transport policy; no permissive fallback.
fn bytes(value: &str) -> Result<[u8; 32], Error> {
    let raw = URL_SAFE_NO_PAD.decode(value).map_err(Error::Encoding)?;
    if URL_SAFE_NO_PAD.encode(&raw) != value {
        return Err(Error::Record);
    }
    raw.try_into().map_err(|_| Error::Record)
}
/// Validate all record fields before serialization or any password derivation.
/// Schema/algorithm/key lengths are protocol invariants; costs remain explicit.
fn validate(record: &Record) -> Result<(), Error> {
    if record.schema_version != 1
        || record.algorithm != "argon2id-v19"
        || !digest(record.installation_digest.as_bytes())
        || !record.principal.valid()
        || record.principal.kind != "human"
        || record.generation <= 0
    {
        return Err(Error::Record);
    }
    record.parameters.validate()?;
    bytes(&record.salt)?;
    bytes(&record.derived)?;
    Ok(())
}
/// Encode canonical private verifier data without storage, authority or defaults.
/// Invalid fields/parameters refuse, preserving typed framing/algorithm causes.
pub fn encode(record: &Record) -> Result<Vec<u8>, Error> {
    validate(record)?;
    wire::encode(record).map_err(Error::Frame)
}
/// Require exact canonical closed-schema JSON (including nested objects). Reject
/// ambiguous/unknown fields, nulls, trailing data and algorithm substitutions.
pub fn decode(raw: &[u8]) -> Result<Record, Error> {
    let record: Record = wire::decode(raw).map_err(Error::Frame)?;
    if encode(&record)? != raw {
        return Err(Error::Record);
    }
    Ok(record)
}
/// SHA256(domain NUL canonical record) for an independently retained expectation.
/// Computing this from an untrusted incoming record does not establish its trust.
pub fn fingerprint(record: &Record) -> Result<String, Error> {
    let mut h = Sha256::new();
    h.update(b"swarm-personal-credential-v1\0");
    h.update(encode(record)?);
    Ok(format!("{:x}", h.finalize()))
}
/// Derive Argon2id v19 with explicit costs and protocol-sized salt/output. The
/// output and owned working allocation have zeroizing guards on success/error;
/// caller owns/wipes input bytes. This does not guarantee erasure of every stack,
/// compiler/OS copy, allocator metadata, crash dump or borrowed password.
/// No password normalization, silent truncation or alternate algorithm is used.
fn derive(
    parameters: Parameters,
    salt: &[u8; 32],
    password: &[u8],
) -> Result<Zeroizing<[u8; 32]>, Error> {
    if password.is_empty() {
        return Err(Error::Empty);
    }
    let mut result = Zeroizing::new([0u8; 32]);
    let parameters = parameters.params()?;
    let mut memory = Vec::new();
    memory
        .try_reserve_exact(parameters.block_count())
        .map_err(Error::Allocation)?;
    memory.resize(parameters.block_count(), argon2::Block::new());
    let mut memory = Zeroizing::new(memory);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, parameters)
        .hash_password_into_with_memory(password, salt, result.as_mut(), memory.as_mut_slice())
        .map_err(Error::Crypto)?;
    Ok(result)
}
/// Create only verifier material for an independently provisioned human binding.
/// Fresh OS entropy supplies the 32-byte salt. Passwords are borrowed, never
/// retained/serialized/logged; the caller must wipe its own input. This function
/// neither creates accounts nor consumes installation proof nor approves costs.
/// Invalid inputs, entropy/allocation/algorithm errors refuse without storage IO.
pub fn create(
    installation: &str,
    principal: &Principal,
    generation: i64,
    parameters: Parameters,
    password: &[u8],
) -> Result<Record, Error> {
    if !digest(installation.as_bytes())
        || !principal.valid()
        || principal.kind != "human"
        || generation <= 0
    {
        return Err(Error::Binding);
    }
    parameters.validate()?;
    if password.is_empty() {
        return Err(Error::Empty);
    }
    let mut salt = [0u8; 32];
    getrandom::fill(&mut salt).map_err(Error::Random)?;
    let derived = derive(parameters, &salt, password)?;
    let record = Record {
        schema_version: 1,
        algorithm: "argon2id-v19".into(),
        installation_digest: installation.into(),
        principal: principal.clone(),
        generation,
        parameters,
        salt: URL_SAFE_NO_PAD.encode(salt),
        derived: URL_SAFE_NO_PAD.encode(derived.as_ref()),
    };
    validate(&record)?;
    Ok(record)
}
/// Verify exact independent installation/principal/generation/record expectation
/// before Argon2 work, then compare the derived bytes in constant time. A wrong
/// password is not an infrastructure error or empty successful identity. The
/// resulting borrow proves only knowledge of that record's secret at this call;
/// current account authority/revocation, actual human presence, bootstrap, session
/// lifetime, transport and OS isolation are outside this function's contract.
pub fn verify<'a>(
    installation: &str,
    principal: &Principal,
    generation: i64,
    expected_digest: &str,
    record: &'a Record,
    password: &[u8],
) -> Result<Proof<'a>, Error> {
    let pin = fingerprint(record)?;
    if !digest(expected_digest.as_bytes())
        || installation != record.installation_digest
        || principal != &record.principal
        || generation != record.generation
        || !bool::from(pin.as_bytes().ct_eq(expected_digest.as_bytes()))
    {
        return Err(Error::Binding);
    }
    let actual = derive(record.parameters, &bytes(&record.salt)?, password)?;
    let expected = Zeroizing::new(bytes(&record.derived)?);
    if !bool::from(actual.as_ref().ct_eq(expected.as_ref())) {
        return Err(Error::Password);
    }
    Ok(Proof { record, pin })
}
