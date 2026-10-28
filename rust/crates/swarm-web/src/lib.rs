//! Shared native HTTP boundary under construction. Typed protected Root routes
//! and persisted sessions exist. Native HTTP/1 requires an explicit outcome observer; Go replacement is incomplete.
//! Protected-request consistency is not agent/model/business authority.
#![forbid(unsafe_code)]
mod access;
mod error;
#[cfg(target_os = "linux")]
mod service;
pub use access::LocalWebAuthority;
pub use error::WebAccessError;
#[cfg(target_os = "linux")]
pub use service::{
    HttpReply, HttpServiceError, NativeHttpServer, NativeHttpService, NativeJobOutcome,
    NativeObservationCause, NativeObservationFailure, NativeOutcomeObserver, NativeOutcomeRecord,
    NativeRequestError, NativeServerError, NativeServerReport, RecordedConfiguration,
    TrustEnrollmentSource, TrustEnrollmentSourceError,
};
