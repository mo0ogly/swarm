//! Typed owner observation, independent of response bytes and client lifetime.
//! This module alone supplies no TCP wiring, durable journal or business authority.
use super::{HttpReply, HttpServiceError, RecordedConfiguration};
use http::{Response, StatusCode};
use std::{
    any::Any,
    error::Error,
    fmt, io,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{Arc, Mutex},
};

/// Internal response extension, shared only to retain non-cloneable typed causes.
/// Arc copies preserve ownership; neither its presence nor a diagnostic grants
/// authority. Successful DTOs already expose their unavailable observations.
#[derive(Clone)]
pub(super) struct ObservationCauses(pub Vec<Arc<swarm_engine::RootStoreError>>);

/// Trusted service outcome and known state, separated from response ownership.
/// Construct from actual NativeHttpService results; this representation neither
/// authenticates a supplied in-process claim nor proves receipt/acceptance.
#[must_use = "Observe the outcome and preserve known effects before delivering its response"]
pub struct NativeJobOutcome {
    pub status: StatusCode,
    pub failure: Option<HttpServiceError>,
    /// Ancillary diagnostics for a valid degraded read. Owner retains them through
    /// late output failures; they are distinct from a refused business operation.
    pub observation_causes: Vec<Arc<swarm_engine::RootStoreError>>,
    pub recorded_work: Option<(String, i64)>,
    pub configuration_may_be_recorded: bool,
    pub recorded_configuration: Option<RecordedConfiguration>,
}
impl NativeJobOutcome {
    /// Consume a trusted service reply, separating response bytes from retained
    /// cause and known/replayed state. Error metadata is retained even if an
    /// adapter omitted redundant reply metadata. None/false never proves rollback.
    pub fn from_reply(mut reply: HttpReply) -> (Self, Response<Vec<u8>>) {
        let observation_causes = reply.response.extensions_mut().remove::<ObservationCauses>()
            .map_or_else(Vec::new, |causes| causes.0);
        let error_work = reply.failure.as_ref().and_then(|e| {
            e.recorded_work()
                .map(|(id, revision)| (id.to_owned(), revision))
        });
        let recorded_configuration = reply.recorded_configuration.or_else(|| {
            reply
                .failure
                .as_ref()
                .and_then(HttpServiceError::recorded_configuration)
                .cloned()
        });
        let configuration = recorded_configuration.is_some()
            || reply.configuration_may_be_recorded
            || reply
                .failure
                .as_ref()
                .is_some_and(HttpServiceError::configuration_may_be_recorded);
        (
            Self {
                status: reply.response.status(),
                observation_causes,
                failure: reply.failure,
                recorded_work: reply.recorded_work.or(error_work),
                configuration_may_be_recorded: configuration,
                recorded_configuration,
            },
            reply.response,
        )
    }
    /// Preserve a service error that prevented a complete reply, including late
    /// encoding/close failures after known effects. No fallback response, mutation
    /// or automatic retry is performed; transport still owes explicit delivery.
    pub fn from_error(error: HttpServiceError) -> Self {
        Self {
            status: error.status(),
            observation_causes: Vec::new(),
            recorded_work: error
                .recorded_work()
                .map(|(id, revision)| (id.to_owned(), revision)),
            configuration_may_be_recorded: error.configuration_may_be_recorded(),
            recorded_configuration: error.recorded_configuration().cloned(),
            failure: Some(error),
        }
    }
    /// Safe v1 projection for an owned observer: status, stable failure family and
    /// known effects, without request body/headers/credential or raw cause text.
    /// Serializing/writing/flushing this record is caller-owned and proves neither
    /// persistence nor HTTP peer receipt. It is not a signed audit attestation.
    pub fn record(&self) -> NativeOutcomeRecord<'_> {
        NativeOutcomeRecord {
            schema_version: 1,
            status: self.status.as_u16(),
            error: self.failure.as_ref().map(HttpServiceError::code),
            observation_causes: self.observation_causes.iter().map(|cause| cause.code()).collect(),
            recorded_work: self
                .recorded_work
                .as_ref()
                .map(|(work, revision)| RecordedWork {
                    work,
                    revision: *revision,
                }),
            configuration_may_be_recorded: self.configuration_may_be_recorded,
            recorded_configuration: self.recorded_configuration.as_ref(),
        }
    }
    /// Attempt one owner observation, returning the same outcome on acknowledgement.
    /// IO failure or unwinding panic retains the original cause/payload and outcome;
    /// callback side effects/state may be partial and are not rolled back/retried.
    /// Standard panic-hook side effects remain outside this redaction guarantee.
    /// Caller must stop admission and drain/retain other already-admitted outcomes
    /// before closing transport/Root. This helper alone does not implement that loop.
    pub fn observe<F>(self, observer: &mut F) -> Result<Self, NativeObservationFailure>
    where
        F: FnMut(&Self) -> Result<(), io::Error> + ?Sized,
    {
        let cause = match catch_unwind(AssertUnwindSafe(|| observer(&self))) {
            Ok(Ok(())) => return Ok(self),
            Ok(Err(error)) => NativeObservationCause::Io(error),
            Err(payload) => NativeObservationCause::Panic(Mutex::new(payload)),
        };
        Err(NativeObservationFailure {
            cause,
            pending: vec![self],
        })
    }
}
impl fmt::Debug for NativeJobOutcome {
    /// Safe summary; no raw cause, response, credential or Work identifier.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeJobOutcome")
            .field("status", &self.status)
            .field("error", &self.failure.as_ref().map(HttpServiceError::code))
            .field("work_known", &self.recorded_work.is_some())
            .field(
                "configuration_may_be_recorded",
                &self.configuration_may_be_recorded,
            )
            .finish()
    }
}
/// Immutable serializable observation projection, built only from an outcome.
/// Fields are private to prevent editing a borrowed record into a different claim.
/// The underlying in-process outcome still requires trusted service provenance.
#[derive(serde::Serialize)]
pub struct NativeOutcomeRecord<'a> {
    schema_version: u8,
    status: u16,
    error: Option<&'static str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    observation_causes: Vec<&'static str>,
    recorded_work: Option<RecordedWork<'a>>,
    configuration_may_be_recorded: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    recorded_configuration: Option<&'a RecordedConfiguration>,
}
/// Named Work metadata avoids an ambiguous positional tuple on the wire. It is
/// known/replayed state from the service, never a new execution authorization.
#[derive(serde::Serialize)]
struct RecordedWork<'a> {
    work: &'a str,
    revision: i64,
}
/// Original observer failure. Panic payload is retained opaquely, never formatted
/// by this module. Catching it does not restore arbitrary callback-owned state.
/// The mutex preserves Send+Sync for typed IO cause composition; transport never
/// locks or formats the payload. Inspectors own lock/poison handling.
pub enum NativeObservationCause {
    Io(io::Error),
    Panic(Mutex<Box<dyn Any + Send>>),
}
impl fmt::Display for NativeObservationCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Io(_) => "web-observer-io",
            Self::Panic(_) => "web-observer-panic",
        })
    }
}
impl fmt::Debug for NativeObservationCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for NativeObservationCause {
    /// Expose typed IO cause; panic payload remains in the explicit enum field.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Panic(_) => None,
        }
    }
}
/// Failed observation plus every subsequently retained admitted outcome. Caller
/// owns the admission bound and must stop new work; no hidden retention cap drops
/// causes/known state. This is not a persisted incident or supervisor receipt.
pub struct NativeObservationFailure {
    pub cause: NativeObservationCause,
    pub pending: Vec<NativeJobOutcome>,
}
impl NativeObservationFailure {
    /// Stable family; retry remains a supervisor/operator decision after readback.
    pub fn code(&self) -> &'static str {
        "web-outcome-observation-failed"
    }
    /// Retain another already-admitted outcome after the observer failed, without
    /// calling that failed observer again. Owner owes bounded admission/drain and
    /// onward handoff; no result is discarded or automatically replayed here.
    pub fn defer(&mut self, outcome: NativeJobOutcome) {
        self.pending.push(outcome);
    }
    /// Observe all retained configuration markers. False is not absence/rollback.
    pub fn configuration_may_be_recorded(&self) -> bool {
        self.pending.iter().any(|o| o.configuration_may_be_recorded)
    }
    /// First exact known settings entry; inspect pending for all outcomes. None
    /// does not infer revision from a bool marker or current mutable disk state.
    pub fn recorded_configuration(&self) -> Option<&RecordedConfiguration> {
        self.pending
            .iter()
            .find_map(|outcome| outcome.recorded_configuration.as_ref())
    }
    /// Compatibility summary of the first known Work; all other Work/errors remain
    /// available in pending. This does not prove delivery or authorize a retry.
    pub fn recorded_work(&self) -> Option<(&str, i64)> {
        self.pending.iter().find_map(|o| {
            o.recorded_work
                .as_ref()
                .map(|(work, revision)| (work.as_str(), *revision))
        })
    }
}
impl fmt::Display for NativeObservationFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for NativeObservationFailure {
    /// No observer diagnostic text/payload or request-derived identifiers in logs.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeObservationFailure")
            .field("cause", &self.cause)
            .field("pending", &self.pending.len())
            .field(
                "configuration_may_be_recorded",
                &self.configuration_may_be_recorded(),
            )
            .finish()
    }
}
impl Error for NativeObservationFailure {
    /// Original cause remains typed; pending service failures are secondary causes
    /// that the owner must inspect explicitly, not a single linear source chain.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.cause)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    /// Successful degraded200 transfers typed causes without faking failure;
    /// a later observer IO failure retains those same originals in pending.
    #[test]
    fn ancillary_read_causes_survive_reply_and_failed_owner_observation() {
        let mut response = Response::new(Vec::new());
        response.extensions_mut().insert(ObservationCauses(vec![Arc::new(swarm_engine::RootStoreError::Refused("fixture-one")),Arc::new(swarm_engine::RootStoreError::Refused("fixture-two"))]));
        let reply = HttpReply { response, failure: None, recorded_work: None, configuration_may_be_recorded: false, recorded_configuration: None };
        assert_eq!(reply.observation_causes().len(),2);
        let (outcome, response) = NativeJobOutcome::from_reply(reply);
        assert!(response.extensions().get::<ObservationCauses>().is_none());
        assert!(outcome.failure.is_none());
        assert_eq!(outcome.status,StatusCode::OK);
        let record=serde_json::to_value(outcome.record()).unwrap();
        assert!(record["error"].is_null());assert_eq!(record["observation_causes"].as_array().unwrap().len(),2);
        let outcome = outcome.observe(&mut |_| Ok(())).unwrap();
        let failure = outcome.observe(&mut |_| Err(io::Error::new(io::ErrorKind::BrokenPipe,"fixture-owner"))).unwrap_err();
        assert_eq!(failure.pending[0].observation_causes.len(),2);
        assert!(matches!(failure.pending[0].observation_causes[1].as_ref(),swarm_engine::RootStoreError::Refused("fixture-two")));
    }
    /// A valid observer witness precedes panic. Retain payload/effect, then keep
    /// later admitted outcomes without retrying a possibly partially changed observer.
    #[test]
    fn observer_panic_and_deferred_outcomes_preserve_causes_without_retry() {
        let outcome = NativeJobOutcome::from_error(HttpServiceError::Refused("owned-refusal"));
        let outcome = outcome.observe(&mut |_| Ok(())).unwrap();
        assert!(!outcome.configuration_may_be_recorded);
        let calls = std::cell::Cell::new(0);
        let mut failed = outcome
            .observe(&mut |_| {
                calls.set(calls.get() + 1);
                panic!("owned observer panic")
            })
            .unwrap_err();
        assert!(
            matches!(&failed.cause,NativeObservationCause::Panic(p) if p.lock().unwrap().downcast_ref::<&str>()==Some(&"owned observer panic"))
        );
        failed.defer(NativeJobOutcome {
            status: StatusCode::OK,
            failure: None,
            observation_causes: Vec::new(),
            recorded_work: Some(("owned-work".into(), 3)),
            configuration_may_be_recorded: true,
            recorded_configuration: None,
        });
        assert_eq!(calls.get(), 1);
        assert_eq!(failed.pending.len(), 2);
        assert!(failed.configuration_may_be_recorded());
        assert_eq!(failed.recorded_work(), Some(("owned-work", 3)));
        let record = serde_json::to_value(failed.pending[1].record()).unwrap();
        assert_eq!(record["recorded_work"]["work"], "owned-work");
        assert_eq!(record["recorded_work"]["revision"], 3);
        assert!(!format!("{failed:?}").contains("owned observer panic"));
    }
    /// Known late error metadata survives independently of a response. A preceding
    /// ordinary refusal does not become a false configuration-publication claim.
    #[test]
    fn late_service_error_retains_configuration_without_upgrading_refusal() {
        assert!(
            !NativeJobOutcome::from_error(HttpServiceError::Refused("owned-refusal"))
                .configuration_may_be_recorded
        );
        let outcome = NativeJobOutcome::from_error(HttpServiceError::ConfigurationState {
            primary: Box::new(HttpServiceError::Refused("owned-late-failure")),
        });
        assert_eq!(outcome.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(outcome.configuration_may_be_recorded);
        assert_eq!(
            outcome.failure.unwrap().code(),
            "web-configuration-state-preserved"
        );
    }
}

#[cfg(test)]
mod correlation_tests {
    use super::*;
    /// A late HTTP-layer cleanup error keeps metadata captured from the trusted
    /// returned document. An uncertain underlying error without such a document
    /// keeps None rather than manufacturing a revision from a bool or readback.
    #[test]
    fn late_error_correlation_survives_response_encoding_and_owner_failure() {
        let unknown = NativeJobOutcome::from_error(HttpServiceError::ConfigurationState {
            primary: Box::new(HttpServiceError::Refused("owned-uncertain-publication")),
        });
        assert!(unknown.configuration_may_be_recorded);
        assert!(unknown.recorded_configuration.is_none());
        let configuration = RecordedConfiguration {
            event_id: "owned-event".into(),
            event_revision: swarm_engine::SettingCount(2),
            current_revision: swarm_engine::SettingCount(3),
            replayed: true,
        };
        let error = HttpServiceError::ConfigurationRecorded {
            primary: Box::new(HttpServiceError::Refused("owned-late-close")),
            configuration: configuration.clone(),
        };
        let reply = super::super::error_reply(error, false).unwrap();
        let encoded: serde_json::Value = serde_json::from_slice(reply.response.body()).unwrap();
        assert_eq!(encoded["recorded_configuration"]["event_revision"], "2");
        assert_eq!(encoded["recorded_configuration"]["current_revision"], "3");
        let (outcome, _response) = NativeJobOutcome::from_reply(reply);
        assert_eq!(outcome.recorded_configuration, Some(configuration.clone()));
        let failure = outcome
            .observe(&mut |_| {
                Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "owned correlation handoff",
                ))
            })
            .unwrap_err();
        assert_eq!(failure.recorded_configuration(), Some(&configuration));
        assert!(failure.configuration_may_be_recorded());
    }
}

#[cfg(test)]
mod io_compatibility_tests {
    use super::*;
    /// CLI's established custom IO cause chain requires Send+Sync. The full server
    /// error enum must fit that contract, including an opaque panic variant.
    #[test]
    fn native_server_failure_remains_a_typed_send_sync_io_cause() {
        let error = super::super::NativeServerError::Refused("owned-io-cause");
        let io = io::Error::other(error);
        assert_eq!(
            io.get_ref()
                .unwrap()
                .downcast_ref::<super::super::NativeServerError>()
                .unwrap()
                .code(),
            "owned-io-cause"
        );
    }
}
