//! Private service request data only. Decoding neither admits a peer nor grants
//! permission. The transport must own a guarded complete packet and authenticate
//! its actual controller before dispatch; this module opens no descriptor.
use crate::control;
use sha2::{Digest, Sha256};
use std::{collections::TryReserveError, error::Error as StdError, fmt};
use swarm_enrollment_config as config;
use zeroize::Zeroizing;

/// Distinct binary protocol domain, never an alias for Go installation-control-v2.
pub const HEADER: &[u8] = b"swarm-personal-service-request-v1\n";

/// Closed explicit operations. Proof/password borrow the guarded packet, never
/// a normalized string. No actor, signing key, grant, path or authority head is
/// request-selectable; configuration CAS remains inside the existing Change.
pub enum Operation<'a> {
    Initialize,
    Enroll {
        proof: &'a [u8; 32],
        password: &'a [u8],
    },
    Recover,
    Control {
        password: &'a [u8],
        request: control::Request,
    },
}
impl Operation<'_> {
    /// Protocol opcode only, not a permission or execution policy.
    pub(crate) fn code(&self) -> u8 {
        match self {
            Self::Initialize => 1,
            Self::Enroll { .. } => 2,
            Self::Recover => 3,
            Self::Control { request, .. } => match request {
                control::Request::Read => 4,
                control::Request::Preview(_) => 5,
                control::Request::Create(_) => 6,
                control::Request::Apply(_) => 7,
            },
        }
    }
    /// Borrow exact secret parts without copies; no secrets for init/recover.
    fn secrets(&self) -> (Option<&[u8; 32]>, &[u8]) {
        match self {
            Self::Enroll { proof, password } => (Some(proof), password),
            Self::Control { password, .. } => (None, password),
            _ => (None, &[]),
        }
    }
}
impl fmt::Debug for Operation<'_> {
    /// Never format secrets, configuration, sizes, target or identity.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PersonalServiceOperation { <private> }")
    }
}

/// Safe framing/allocation/configuration refusal. Nested causes stay typed;
/// ordinary diagnostics never print request content or private parser text.
pub enum Error {
    Format,
    Allocation(TryReserveError),
    Configuration(config::Error),
}
impl Error {
    /// Static category only; no secret digest or input text.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Format => "service-request-format",
            Self::Allocation(_) => "service-request-allocation",
            Self::Configuration(_) => "service-request-configuration",
        }
    }
}
impl fmt::Display for Error {
    /// Keep private nested causes out of normal diagnostics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "personal service request: {}", self.code())
    }
}
impl fmt::Debug for Error {
    /// Same redaction as Display, including malformed secret-bearing packets.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Error {
    /// Preserve original allocation/configuration cause for trusted traversal.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Allocation(e) => Some(e),
            Self::Configuration(e) => Some(e),
            Self::Format => None,
        }
    }
}

/// Decoded data bound to the lifetime of its guarded input. Private fields prevent
/// construction bypassing shape validation. No Clone/Serialize or authority token.
pub struct Request<'a> {
    pin: &'a str,
    challenge: [u8; 32],
    operation: Operation<'a>,
    public_prefix: &'a [u8],
}
impl fmt::Debug for Request<'_> {
    /// No credentials, target, challenge, lengths or configuration in diagnostics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PersonalServiceRequest { <private> }")
    }
}
impl<'a> Request<'a> {
    /// Borrow validated exact operation data; possession is not authorization.
    pub fn operation(&self) -> &Operation<'a> {
        &self.operation
    }
    /// Expected independently pinned service declaration, not a learned server pin.
    pub fn target_pin(&self) -> &str {
        self.pin
    }
    /// Caller correlation bytes only; neither nonce freshness nor replay protection
    /// is established. Never replace this with a public hash of the password.
    pub fn challenge(&self) -> &[u8; 32] {
        &self.challenge
    }
    /// Domain-separated digest of only the exact validated public prefix. Proof
    /// and password are excluded; this binds action/target/correlation, never
    /// secret knowledge, packet freshness, sender admission or authorization.
    pub fn public_digest(&self) -> String {
        let mut hash = Sha256::new();
        hash.update(b"swarm-personal-service-public-request-v1\0");
        hash.update(self.public_prefix);
        format!("{:x}", hash.finalize())
    }
    /// Consume validated request data for the trusted service owner only.
    pub(crate) fn into_operation(self) -> Operation<'a> {
        self.operation
    }
}

/// Consume an exact slice prefix with checked framing, no allocation or IO.
fn take<'a>(raw: &mut &'a [u8], count: usize) -> Result<&'a [u8], Error> {
    if count > raw.len() {
        return Err(Error::Format);
    }
    let (value, rest) = raw.split_at(count);
    *raw = rest;
    Ok(value)
}

/// Decode one complete guarded binary packet. Fixed header, ASCII SHA256 target,
/// raw32 challenge, opcode, u64BE public-Change length, canonical Change, then
/// raw32 proof for enroll and exact nonempty password through packet end for
/// enroll/control. Init/recover require an empty tail. No secret allocation,
/// normalization, transport size policy, IO, replay admission or journal effect.
/// The caller must prevent unguarded transport copies and impose its actual
/// technical packet bound; dropping this view does not erase a retained packet.
pub fn decode(packet: &Zeroizing<Vec<u8>>) -> Result<Request<'_>, Error> {
    let mut raw = packet.as_slice();
    if take(&mut raw, HEADER.len())? != HEADER {
        return Err(Error::Format);
    }
    let pin_raw = take(&mut raw, 64)?;
    if !swarm_identity_contract::digest(pin_raw) {
        return Err(Error::Format);
    }
    let pin = std::str::from_utf8(pin_raw).map_err(|_| Error::Format)?;
    let challenge = take(&mut raw, 32)?.try_into().map_err(|_| Error::Format)?;
    let code = take(&mut raw, 1)?[0];
    if !(1..=7).contains(&code) {
        return Err(Error::Format);
    }
    let size = u64::from_be_bytes(take(&mut raw, 8)?.try_into().map_err(|_| Error::Format)?);
    let size = usize::try_from(size).map_err(|_| Error::Format)?;
    let change = take(&mut raw, size)?;
    let public_prefix = &packet[..packet.len() - raw.len()];
    if (code >= 5) != !change.is_empty() {
        return Err(Error::Format);
    }
    let request = match code {
        4 => Some(control::Request::Read),
        5..=7 => {
            let c = config::decode_change(change).map_err(Error::Configuration)?;
            Some(match code {
                5 => control::Request::Preview(c),
                6 => control::Request::Create(c),
                _ => control::Request::Apply(c),
            })
        }
        _ => None,
    };
    let operation = match code {
        1 | 3 if raw.is_empty() => {
            if code == 1 {
                Operation::Initialize
            } else {
                Operation::Recover
            }
        }
        2 => {
            let proof = take(&mut raw, 32)?.try_into().map_err(|_| Error::Format)?;
            if raw.is_empty() {
                return Err(Error::Format);
            }
            Operation::Enroll {
                proof,
                password: raw,
            }
        }
        4..=7 if !raw.is_empty() => Operation::Control {
            password: raw,
            request: request.ok_or(Error::Format)?,
        },
        _ => return Err(Error::Format),
    };
    Ok(Request {
        pin,
        challenge,
        operation,
        public_prefix,
    })
}

/// Encode exact data into one pre-reserved guarded allocation before any secret
/// copy. Public Change serialization reuses its authoritative canonical codec.
/// Invalid target/empty required password, length overflow and allocation errors
/// refuse; no secret-bearing growth, hashing, IO, authority or implicit entropy.
/// Caller owns original borrowed secrets and transport copies/abrupt-exit limits.
pub fn encode(
    pin: &str,
    challenge: &[u8; 32],
    operation: &Operation<'_>,
) -> Result<Zeroizing<Vec<u8>>, Error> {
    if !swarm_identity_contract::digest(pin.as_bytes()) {
        return Err(Error::Format);
    }
    let (proof, password) = operation.secrets();
    if matches!(
        operation,
        Operation::Enroll { .. } | Operation::Control { .. }
    ) && password.is_empty()
    {
        return Err(Error::Format);
    }
    let change = match operation {
        Operation::Control {
            request:
                control::Request::Preview(c) | control::Request::Create(c) | control::Request::Apply(c),
            ..
        } => config::encode_change(c).map_err(Error::Configuration)?,
        _ => Vec::new(),
    };
    let length = u64::try_from(change.len()).map_err(|_| Error::Format)?;
    let mut total = HEADER.len();
    for n in [
        64,
        32,
        1,
        8,
        change.len(),
        if proof.is_some() { 32 } else { 0 },
        password.len(),
    ] {
        total = total.checked_add(n).ok_or(Error::Format)?;
    }
    let mut packet = Zeroizing::new(Vec::new());
    packet.try_reserve_exact(total).map_err(Error::Allocation)?;
    packet.extend_from_slice(HEADER);
    packet.extend_from_slice(pin.as_bytes());
    packet.extend_from_slice(challenge);
    packet.push(operation.code());
    packet.extend_from_slice(&length.to_be_bytes());
    packet.extend_from_slice(&change);
    if let Some(proof) = proof {
        packet.extend_from_slice(proof);
    }
    packet.extend_from_slice(password);
    Ok(packet)
}
