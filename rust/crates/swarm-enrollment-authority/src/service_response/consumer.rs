//! Point-in-time data validation only, after the caller authenticates its actual
//! peer. No response or locally checked wrapper supplies current effect authority.
use super::{FailureLayer, FilePhase, JournalPhase, Observation};
use crate::{
    control,
    service_request::{Operation, Request},
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use std::{error::Error as StdError, fmt};
use subtle::ConstantTimeEq;
use swarm_enrollment_config as config;

/// Safe decoding/shape/binding/configuration refusal, no private payload text.
pub enum Error {
    Frame(serde_json::Error),
    Shape,
    Binding,
    Configuration(config::Error),
}
impl Error {
    /// Stable static category; nested detailed causes remain trusted-local only.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Frame(_) => "service-response-frame",
            Self::Shape => "service-response-shape",
            Self::Binding => "service-response-binding",
            Self::Configuration(_) => "service-response-configuration",
        }
    }
}
impl fmt::Display for Error {
    /// Do not print parser strings, configuration, identity, binding or paths.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "personal service response: {}", self.code())
    }
}
impl fmt::Debug for Error {
    /// Same safe category as Display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Error {
    /// Actual parser/configuration cause; no authority or effect inferred.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Frame(e) => Some(e),
            Self::Configuration(e) => Some(e),
            _ => None,
        }
    }
}
/// Constructed only after canonical shape, expected request and configuration
/// checks. No Deserialize/Clone or grant; observations can become stale immediately.
pub struct BoundResponse {
    observation: Observation,
}
impl BoundResponse {
    /// Checked response data only. Sender/freshness/current authority remain
    /// independent prerequisites; completed=false must still be honored.
    pub fn observation(&self) -> &Observation {
        &self.observation
    }
}
impl fmt::Debug for BoundResponse {
    /// No checked payload, configuration or request correlation is formatted.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("BoundPersonalServiceResponse { <private> }")
    }
}

/// Positive sequence only with an acknowledged/observed journal. Closed protocol
/// shape, not an operator sequence ceiling or proof that data is still current.
fn failed_personal(o: &Observation) -> bool {
    let journal = match o.journal {
        Some(j) => j,
        None => return false,
    };
    o.intent.is_some()
        && o.checkpoint.is_some()
        && match journal {
            JournalPhase::Acknowledged | JournalPhase::Observed => {
                o.sequence.is_some_and(|n| n > 0)
            }
            JournalPhase::NotAttempted | JournalPhase::Unknown => o.sequence.is_none(),
            JournalPhase::CurrentVerified => false,
        }
}
/// Verify exact personal operation phases and absence of configuration results.
/// Failure progress remains observed, never promoted to successful recovery.
fn personal_shape(o: &Observation) -> bool {
    if o.view.is_some() || o.projection.is_some() || o.configuration_effect_unknown {
        return false;
    }
    if !o.completed {
        return o.failure_layers == [FailureLayer::Personal]
            && failed_personal(o)
            && (o.operation != 1
                || (o.intent == Some(FilePhase::NotAttempted)
                    && o.journal == Some(JournalPhase::NotAttempted)
                    && o.sequence.is_none()));
    }
    if o.checkpoint != Some(FilePhase::Durable) {
        return false;
    }
    match o.operation {
        1 => {
            o.intent == Some(FilePhase::NotAttempted)
                && o.journal == Some(JournalPhase::NotAttempted)
                && o.sequence.is_none()
        }
        2 => {
            o.intent == Some(FilePhase::Durable)
                && o.journal == Some(JournalPhase::Acknowledged)
                && o.sequence.is_some_and(|n| n > 0)
        }
        3 => {
            o.intent.is_none()
                && o.journal == Some(JournalPhase::CurrentVerified)
                && o.sequence.is_some_and(|n| n > 0)
        }
        _ => false,
    }
}
/// Closed ordered error-layer shapes follow the actual two-source composition;
/// no arbitrary max-length/attempt policy is introduced by this shape check.
fn control_layers(layers: &[FailureLayer]) -> bool {
    use FailureLayer::*;
    matches!(
        layers,
        [] | [Personal]
            | [Business]
            | [BusinessAfterStorage]
            | [PersonalAfterUse]
            | [PersonalAfterUse, Business]
            | [PersonalAfterUse, BusinessAfterStorage]
    )
}
/// Check result exclusivity and known results after nested faults. Unreported
/// data is never absent-effect evidence; an empty success always refuses.
fn control_shape(o: &Observation) -> bool {
    if o.intent.is_some()
        || o.journal.is_some()
        || o.checkpoint.is_some()
        || o.sequence.is_some()
        || !control_layers(&o.failure_layers)
    {
        return false;
    }
    let retained = o.view.is_some() || o.projection.is_some();
    if o.view.is_some() && o.projection.is_some() {
        return false;
    }
    if !retained && (o.completed || o.failure_layers == [FailureLayer::PersonalAfterUse]) {
        return false;
    }
    if retained {
        if !o.completed
            && !o
                .failure_layers
                .contains(&FailureLayer::BusinessAfterStorage)
            && o.failure_layers != [FailureLayer::PersonalAfterUse]
        {
            return false;
        }
        if (o.operation == 5) != o.projection.is_some() {
            return false;
        }
    }
    if o.configuration_effect_unknown
        && (!(o.operation == 6 || o.operation == 7)
            || !(o.failure_layers.contains(&FailureLayer::Business)
                || o.failure_layers
                    .contains(&FailureLayer::BusinessAfterStorage)))
    {
        return false;
    }
    true
}
/// Validate complete history/head/pins with the existing authoritative codec,
/// then bind returned last Change and projection base/replay to this request.
/// Expected pins are independently protected caller inputs, never learned here.
fn configuration(o: &Observation, request: &Request<'_>, pins: &config::Pins) -> Result<(), Error> {
    pins.validate().map_err(Error::Configuration)?;
    let view = if let Some(p) = &o.projection {
        config::validate_projection(p).map_err(Error::Configuration)?;
        Some(&p.proposed)
    } else if let Some(v) = &o.view {
        config::validate_view(v).map_err(Error::Configuration)?;
        Some(v)
    } else {
        None
    };
    let Some(view) = view else { return Ok(()) };
    if &view.snapshot.pins != pins {
        return Err(Error::Binding);
    }
    let change = match request.operation() {
        Operation::Control {
            request:
                control::Request::Preview(c) | control::Request::Create(c) | control::Request::Apply(c),
            ..
        } => Some(c),
        Operation::Control {
            request: control::Request::Read,
            ..
        } => None,
        _ => return Err(Error::Shape),
    };
    if let Some(change) = change {
        if view.snapshot.history.last().map(|entry| &entry.change) != Some(change) {
            return Err(Error::Binding);
        }
        if let Some(p) = &o.projection {
            let base = if p.current_event_replay {
                &p.proposed.head
            } else {
                &change.expected_head
            };
            if &p.base != base {
                return Err(Error::Binding);
            }
        }
    }
    Ok(())
}
/// Decode after actual peer admission, reject noncanonical/duplicate/unknown
/// frames and compare exact expected public request digest/opcode/challenge.
/// Check operation/verdict/phase/result coherence and actual complete View or
/// Projection with independent configuration pins and exact requested Change.
/// Returns a private-constructor wrapper, never sender identity, freshness,
/// password knowledge, current mandate, observed effect or replay permission.
/// Pure data operation: no IO, mutation, retry, transport-size policy or secret
/// copies. Caller owns expected request/pins custody and its original packet.
pub fn decode(
    raw: &[u8],
    request: &Request<'_>,
    expected_pins: &config::Pins,
) -> Result<BoundResponse, Error> {
    let o: Observation = swarm_wire_json::decode(raw).map_err(Error::Frame)?;
    if swarm_wire_json::encode(&o).map_err(Error::Frame)? != raw {
        return Err(Error::Shape);
    }
    if o.schema_version != 1
        || o.protocol != "swarm-personal-service-response-v1"
        || o.completed != o.failure_layers.is_empty()
    {
        return Err(Error::Shape);
    }
    if o.operation != request.operation().code()
        || !bool::from(
            o.request_digest
                .as_bytes()
                .ct_eq(request.public_digest().as_bytes()),
        )
        || o.challenge != URL_SAFE_NO_PAD.encode(request.challenge())
    {
        return Err(Error::Binding);
    }
    if if o.operation <= 3 {
        !personal_shape(&o)
    } else {
        !control_shape(&o)
    } {
        return Err(Error::Shape);
    }
    configuration(&o, request, expected_pins)?;
    Ok(BoundResponse { observation: o })
}
