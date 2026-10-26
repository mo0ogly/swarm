//! Typed HTTP dispatcher with a separate owner-observed HTTP/1 transport.
//! Supported Go paths are deliberately partial; unknown operations never fall back.
use crate::{LocalWebAuthority, WebAccessError};
mod listener;
mod live;
mod live_events;
mod otel;
mod login;
mod native_admin;
mod transport;
pub use transport::{NativeOutcomeObserver, NativeRequestError, NativeServerReport};
mod observation;
mod settings;
mod trust;
mod paea;
mod planning;
mod mission;
mod cockpit_read;
mod work_budget;
mod work_resume;
mod agent_logs;
mod agent_transcript;
mod pricing;
#[cfg(target_os = "linux")]
mod admin_budget;
#[cfg(target_os = "linux")]
mod quotas;
use http::{
    header::{ALLOW, CACHE_CONTROL, CONTENT_TYPE},
    HeaderValue, Method, Request, Response, StatusCode,
};
pub use listener::{NativeHttpServer, NativeServerError};
pub use observation::{
    NativeJobOutcome, NativeObservationCause, NativeObservationFailure, NativeOutcomeRecord,
};
use serde::Deserialize;
use settings::SettingsOperation;
use std::{
    error::Error,
    fmt,
    path::{Path, PathBuf},
};
use swarm_engine::{RootBinding, RootStore, RootStoreError, SystemEnvironment};
use swarm_wire_json::projection::{self, Fields, Kind};
/// Known settings-history entry returned by Root apply/replay, captured before
/// response encoding/close. Event revision differs from current revision when an
/// older event is replayed after later changes. This proves neither new bytes,
/// crash durability, authenticated operator identity nor a signed audit receipt.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct RecordedConfiguration {
    pub event_id: String,
    pub event_revision: swarm_engine::SettingCount,
    pub current_revision: swarm_engine::SettingCount,
    pub replayed: bool,
}
/// Safe typed service failures; no raw body/header/credential appears in diagnostics.
/// Original causes and known recorded Work survive response/close aggregation.
pub enum HttpServiceError {
    /// A known preparation receipt preceded a later response/close failure.
    PreparationRecorded { primary: Box<Self>, id: String, revision: i64 },
    ConfigurationRecorded {
        primary: Box<Self>,
        configuration: RecordedConfiguration,
    },
    ConfigurationState {
        primary: Box<Self>,
    },
    Session(swarm_engine::WebSessionError),
    SessionState {
        primary: Box<Self>,
    },
    Access(WebAccessError),
    Input(swarm_engine::CommandInputError),
    /// Pure PAEA input/binding/lifecycle refusal, preserving its typed cause.
    Protection(swarm_protection_contract::Error),
    Projection(projection::Error),
    Json(serde_json::Error),
    LoginEncoding(std::str::Utf8Error),
    HeaderEncoding(http::header::InvalidHeaderValue),
    Model(swarm_engine::WorkError),
    Request(swarm_engine::Error),
    Root {
        operation: &'static str,
        cause: Box<RootStoreError>,
    },
    Close {
        primary: Option<Box<Self>>,
        cleanup: Box<RootStoreError>,
    },
    Recorded {
        primary: Box<Self>,
        work: String,
        revision: i64,
    },
    ResponseEncoding {
        primary: Box<Self>,
        cause: serde_json::Error,
    },
    Method(&'static str),
    Refused(&'static str),
    /// Provider-telemetry store failure (OTLP ingest), cause retained.
    Telemetry(Box<swarm_engine::TelemetryStoreError>),
    /// OTLP body not decodable as the expected signal; cause retained.
    TelemetryInput(swarm_engine::TelemetryInputError),
}
impl HttpServiceError {
    /// Retain trust outcomes through primary/secondary Root close and response
    /// encoding errors. None is unknown, never proof of rollback or retry safety.
    pub fn trust_publication(&self)->Option<&'static str>{match self{
      Self::Root{cause,..}=>cause.trust_publication(),
      Self::Close{primary,cleanup}=>primary.as_deref().and_then(Self::trust_publication).or_else(||cleanup.trust_publication()),
      Self::ResponseEncoding{primary,..}|Self::Recorded{primary,..}|Self::ConfigurationState{primary}|Self::ConfigurationRecorded{primary,..}|Self::SessionState{primary}=>primary.trust_publication(),_=>None,
    }}

    /// Stable family only; it grants neither retry nor any business authority.
    pub fn code(&self) -> &'static str {
        match self {
            Self::ConfigurationState { .. } | Self::ConfigurationRecorded { .. } => {
                "web-configuration-state-preserved"
            }
            Self::LoginEncoding(_) => "web-login-form-utf8-invalid",
            Self::HeaderEncoding(_) => "web-response-header-invalid",
            Self::Session(e) => e.code(),
            Self::SessionState { .. } => "web-session-state-preserved",
            Self::Access(e) => e.code(),
            Self::Input(e) => e.code(),
            Self::Protection(e) => e.code(),
            Self::Projection(_) | Self::Json(_) => "web-document-invalid",
            Self::Model(_) => "web-model-encoding",
            Self::Request(_) => "web-request-encoding",
            Self::Root { cause, .. } => cause.code(),
            Self::Close { .. } => "web-root-close",
            Self::Recorded { .. } => "work-state-preserved",
            Self::PreparationRecorded { .. } => "preparation-state-preserved",
            Self::ResponseEncoding { .. } => "web-response-encoding",
            Self::Method(_) => "web-method-refused",
            Self::Refused(code) => code,
            Self::Telemetry(e) => e.code(),
            Self::TelemetryInput(e) => e.code(),
        }
    }
    /// Whether a failed startup retained a successfully read/published session,
    /// or publication became visible with uncertain directory durability. False
    /// does not establish file absence/rollback; no secret is exposed or removed.
    pub fn session_state_may_be_recorded(&self) -> bool {
        match self {
            Self::Session(e) => e.publication_uncertain(),
            Self::SessionState { .. } => true,
            Self::Close { primary, .. } => primary
                .as_deref()
                .is_some_and(Self::session_state_may_be_recorded),
            Self::Recorded { primary, .. }
            | Self::ResponseEncoding { primary, .. }
            | Self::ConfigurationState { primary }
            | Self::ConfigurationRecorded { primary, .. } => {
                primary.session_state_may_be_recorded()
            }
            _ => false,
        }
    }
    /// Observe known/replayed settings or visible publication with uncertain
    /// durability, including secondary Root-close causes. False is unknown or
    /// no known publication, never proof that data is absent or rolled back.
    pub fn configuration_may_be_recorded(&self) -> bool {
        match self {
            Self::ConfigurationState { .. } | Self::ConfigurationRecorded { .. } => true,
            Self::Root { cause, .. } => cause.configuration_may_be_recorded(),
            Self::Close { primary, cleanup } => {
                primary
                    .as_deref()
                    .is_some_and(Self::configuration_may_be_recorded)
                    || cleanup.configuration_may_be_recorded()
            }
            Self::Recorded { primary, .. }
            | Self::ResponseEncoding { primary, .. }
            | Self::SessionState { primary } => primary.configuration_may_be_recorded(),
            _ => false,
        }
    }
    /// Exact entry only when Root returned a validated document before a later
    /// HTTP-layer failure. None retains unknown precision; no JSON parsing or
    /// concurrent readback is used to manufacture a revision for uncertain IO.
    pub fn recorded_configuration(&self) -> Option<&RecordedConfiguration> {
        match self {
            Self::ConfigurationRecorded { configuration, .. } => Some(configuration),
            Self::Close { primary, .. } => {
                primary.as_deref().and_then(Self::recorded_configuration)
            }
            Self::Recorded { primary, .. }
            | Self::ResponseEncoding { primary, .. }
            | Self::SessionState { primary }
            | Self::ConfigurationState { primary } => primary.recorded_configuration(),
            _ => None,
        }
    }
    /// Known preparation receipt through original/cleanup errors, independently
    /// of Work/configuration. None is unknown, never a rollback guarantee.
    pub fn recorded_preparation(&self) -> Option<(&str, i64)> {
        match self {
            Self::PreparationRecorded { id, revision, .. } => Some((id, *revision)),
            Self::Root { cause, .. } => cause.recorded_preparation(),
            Self::Close { primary, cleanup } => primary
                .as_deref()
                .and_then(Self::recorded_preparation)
                .or_else(|| cleanup.recorded_preparation()),
            Self::Recorded { primary, .. } | Self::ResponseEncoding { primary, .. }
            | Self::SessionState { primary }
            | Self::ConfigurationState { primary }
            | Self::ConfigurationRecorded { primary, .. } => primary.recorded_preparation(),
            _ => None,
        }
    }
    /// Observe known Work state only; None does not prove rollback after an
    /// uncertain commit. No reread, refund, mutation or automatic retry occurs.
    pub fn recorded_work(&self) -> Option<(&str, i64)> {
        match self {
            Self::Recorded { work, revision, .. } => Some((work, *revision)),
            Self::Root { cause, .. } => cause.recorded_work(),
            Self::Close { primary, cleanup } => primary
                .as_deref()
                .and_then(Self::recorded_work)
                .or_else(|| cleanup.recorded_work()),
            Self::ResponseEncoding { primary, .. }
            | Self::SessionState { primary }
            | Self::ConfigurationState { primary }
            | Self::ConfigurationRecorded { primary, .. } => primary.recorded_work(),
            _ => None,
        }
    }
    /// Distinguish protocol refusals, stale conflicts and late server failure;
    /// an HTTP response never changes the underlying Root result into success.
    /// Classify known contract refusals without converting internal faults into
    /// client errors. A known/uncertain publication dominates with500; quota
    /// revision/owners use409, invalid proposals400 and unsupported windows501.
    /// No retry, rollback or authentication decision is inferred from this code.
    fn status(&self) -> StatusCode {
        if self.trust_publication().is_some(){return StatusCode::INTERNAL_SERVER_ERROR;}
        if let Self::Root{cause,..}=self {if cause.trust_publication().is_some(){return StatusCode::INTERNAL_SERVER_ERROR;}
        if let Some(e)=cause.trust_failure(){return match e{swarm_engine::TrustError::Authorization=>StatusCode::FORBIDDEN,swarm_engine::TrustError::Conflict(_)|swarm_engine::TrustError::Integrity(_)=>StatusCode::CONFLICT,swarm_engine::TrustError::Unsupported(_)=>StatusCode::NOT_IMPLEMENTED,_=>StatusCode::BAD_REQUEST};}}

        if self.recorded_preparation().is_some() || self.recorded_work().is_some() || self.configuration_may_be_recorded() {
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
        match self {
            Self::ConfigurationState { .. }
            | Self::ConfigurationRecorded { .. }
            | Self::Session(_)
            | Self::SessionState { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Access(_) => StatusCode::FORBIDDEN,
            Self::Input(e) if e.failure == swarm_engine::CommandInputFailure::TooLarge => {
                StatusCode::PAYLOAD_TOO_LARGE
            }
            Self::Method(_) => StatusCode::METHOD_NOT_ALLOWED,
            Self::Refused(code) if otel::status(code).is_some() => {
                otel::status(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
            }
            Self::Telemetry(e) => {
                otel::status(e.code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
            }
            Self::TelemetryInput(_) => StatusCode::BAD_REQUEST,
            Self::Protection(_) => StatusCode::BAD_REQUEST,
            Self::Refused(
                "web-login-template-invalid"
                | "web-native-template-invalid"
                | "live-nonce-unavailable"
                | "live-clock-invalid",
            ) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::Refused("web-root-binding-changed" | "budget-revision-conflict") => StatusCode::CONFLICT,
            Self::Refused("web-route-unported" | "live-agent-unknown" | "live-run-unknown" | "live-task-unknown") => StatusCode::NOT_FOUND,
            Self::Refused(
                "web-operation-unported"
                | "web-mutation-transport-pending"
                | "live-events-transport-required",
            ) => {
                StatusCode::NOT_IMPLEMENTED
            }
            Self::Root { operation, cause } => match cause.code() {
                "settings-revision-conflict"
                | "settings-event-conflict"
                | "settings-preview-stale-or-missing"
                | "admin-budget-revision-conflict"
                | "admin-budget-event-conflict"
                | "admin-budget-preview-stale"
                | "admin-budget-exhausted"
                | "revision_conflict"
                | "preview_stale"
                | "event_conflict"
                | "active_scope_conflict"
                | "draft_conflict"
                | "already_applied"
                | "quota-revision-conflict"
                | "quota-planner-owner-active"
                | "quota-reviewer-running" => StatusCode::CONFLICT,
                "quota-client-window-unported" => StatusCode::NOT_IMPLEMENTED,
                "quota-request-invalid"
                | "quota-reason-outside-policy"
                | "quota-below-consumption"
                | "quota-below-hierarchy"
                | "quota-child-exceeds-parent"
                | "quota-judge-below-consumption"
                | "quota-judge-required"
                | "quota-planning-required"
                | "quota-reviewer-below-consumption"
                | "quota-reviewer-budget-required"
                | "quota-reviewer-context-invalid"
                | "quota-reviewer-required"
                | "quota-scope-below-consumption"
                | "quota-scope-missing"
                | "quota-review-timeout-invalid"
                | "quota-review-timeout-unrepresentable" => StatusCode::BAD_REQUEST,
                // Only typed caller budget validation is a 400; stored corrupt
                // policy, SQL/cleanup and clock/runtime failures remain server errors.
                "budget-invalid" if matches!(*operation,"budget-input"|"budget-apply")
                    && matches!(cause.as_ref(),RootStoreError::Mutation(swarm_engine::WorkEngineError::Budget(
                        swarm_engine::BudgetError::Decode(_) | swarm_engine::BudgetError::Date(_) | swarm_engine::BudgetError::Refused(_)))) => StatusCode::BAD_REQUEST,
                "command_not_ported" if *operation == "resume-read" && matches!(cause.as_ref(), RootStoreError::Projection(_)) => StatusCode::NOT_IMPLEMENTED,
                "pricing-version-unknown" if *operation == "pricing-quote" => StatusCode::NOT_FOUND,
                _ if matches!(*operation,"pricing-input" | "pricing-quote-input" | "pricing-quote")
                    && matches!(cause.as_ref(), RootStoreError::Pricing(swarm_engine::PricingError::Decode(_) | swarm_engine::PricingError::Date(_) | swarm_engine::PricingError::Refused(_))) => StatusCode::BAD_REQUEST,
                "preparation-request-invalid" | "preparation-method-unavailable" | "preparation-invalid-plan" => StatusCode::BAD_REQUEST,
                "preparation-event-conflict" | "preparation-revision-conflict"
                | "preparation-stale-document" | "preparation-stale-brief" | "preparation-stale-method" | "preparation-stale-plan" => StatusCode::CONFLICT,
                "preparation-write-disabled" => StatusCode::FORBIDDEN,
                "preparation-not-found" | "work-missing" | "unknown_work" | "unknown_task" => StatusCode::NOT_FOUND,
                "authorization_required" => StatusCode::FORBIDDEN,
                "admin-budget-input-invalid"|"admin-budget-policy-invalid"|"admin-budget-intent-invalid"=>StatusCode::BAD_REQUEST,
                "storage_unavailable" => StatusCode::INSUFFICIENT_STORAGE,
                _ if matches!(*operation, "mutate" | "settings-preview" | "settings-apply" | "mission-input")
                    && !matches!(
                        cause.as_ref(),
                        RootStoreError::Storage(_)
                            | RootStoreError::Runtime(_)
                            | RootStoreError::Encode(_)
                    ) =>
                {
                    StatusCode::BAD_REQUEST
                }
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            },
            Self::HeaderEncoding(_)
            | Self::Model(_)
            | Self::Request(_)
            | Self::Close { .. }
            | Self::Recorded { .. }
            | Self::ResponseEncoding { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::BAD_REQUEST,
        }
    }
}
impl fmt::Display for HttpServiceError {
    /// Only the stable family is public, never Root paths/SQL/body/header text.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for HttpServiceError {
    /// The same redaction applies to debug logs and transport diagnostics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for HttpServiceError {
    /// Preserve primary typed causes. Compound variants retain secondary close/
    /// encoding causes in their fields for the owning transport's diagnostics.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ConfigurationState { primary } | Self::ConfigurationRecorded { primary, .. } => {
                Some(primary.as_ref())
            }
            Self::PreparationRecorded { primary, .. } => Some(primary.as_ref()),
            Self::Session(e) => Some(e),
            Self::SessionState { primary } => Some(primary.as_ref()),
            Self::Access(e) => Some(e),
            Self::Input(e) => Some(e),
            Self::Protection(e) => Some(e),
            Self::Projection(e) => Some(e),
            Self::Json(e) => Some(e),
            Self::LoginEncoding(e) => Some(e),
            Self::HeaderEncoding(e) => Some(e),
            Self::Model(e) => Some(e),
            Self::Request(e) => Some(e),
            Self::Root { cause, .. } => Some(cause.as_ref()),
            Self::Telemetry(e) => Some(e.as_ref()),
            Self::TelemetryInput(e) => Some(e),
            Self::Close {
                primary: Some(primary),
                ..
            } => Some(primary.as_ref()),
            Self::Close { cleanup, .. } => Some(cleanup.as_ref()),
            Self::Recorded { primary, .. } | Self::ResponseEncoding { primary, .. } => {
                Some(primary.as_ref())
            }
            _ => None,
        }
    }
}
/// Complete typed HTTP reply plus original action failure. A successfully built
/// 4xx/5xx response is still a refused/failed operation; consumers must retain
/// failure/known state and deliver/flush the response. Known state survives even
/// when delivery fails; no network delivery is established here. Debug/Serialize absent.
#[must_use = "Deliver the response and preserve diagnostics; Work/settings may already be recorded"]
pub struct HttpReply {
    pub response: Response<Vec<u8>>,
    pub failure: Option<HttpServiceError>,
    /// Known Work from this operation/replay, retained independently of body/HEAD.
    /// Consumers must carry it into late transmission errors; None is not rollback.
    pub recorded_work: Option<(String, i64)>,
    /// Known/replayed or visibly uncertain settings publication, not new bytes or
    /// durable success. Carry into late write/flush failure without reparsing JSON.
    pub configuration_may_be_recorded: bool,
    /// Exact known apply/replay entry independent of response bytes. None is not
    /// rollback and does not supply metadata absent from underlying Root errors.
    pub recorded_configuration: Option<RecordedConfiguration>,
}
impl HttpReply {
    /// Borrow ancillary read diagnostics already represented in a successful DTO.
    /// No HTTP failure/commit/permission is inferred. The actual owner must retain
    /// these typed causes if later response/observer output fails; raw messages
    /// may be private. NativeJobOutcome::from_reply transfers them automatically.
    pub fn observation_causes(&self) -> &[std::sync::Arc<RootStoreError>] {
        self.response.extensions().get::<observation::ObservationCauses>()
            .map_or(&[], |causes| causes.0.as_slice())
    }
}
/// Internal encoded route result and known effects. Plain data has no claimed
/// operation effect; it never establishes rollback after an unknown failure.
struct RouteBody {
    trust_publication:Option<&'static str>,
    preparation_receipt: Option<(String,i64)>,
    observation_causes: Vec<RootStoreError>,
    bytes: Vec<u8>,
    recorded_work: Option<(String, i64)>,
    configuration_recorded: bool,
    recorded_configuration: Option<RecordedConfiguration>,
}
impl RouteBody {
    /// Wrap successful observational bytes without inventing a recorded mutation.
    fn plain(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            trust_publication:None,
            preparation_receipt:None,
            observation_causes: Vec::new(),
            recorded_work: None,
            configuration_recorded: false,
            recorded_configuration: None,
        }
    }
}
/// Implementation profiles describe adapters, never operator policy or business
/// authority. The HTTP1 adapter requires an explicit owner observer before writes.
#[derive(Clone, Copy)]
enum DispatchProfile {
    Typed,
    ObservedHttp1,
}
impl DispatchProfile {
    /// Exact implemented capability DTO. Framing remains the shared native limit;
    /// an advertised method is not authenticated agent identity or peer delivery.
    fn capabilities(self) -> serde_json::Value {
        match self {
            Self::Typed => {
                serde_json::json!({"schema_version":1,"profile":"rust-native-in-construction","actions":["task"],"trust":{"biba":"strict-with-named-endorsement-v1","scope":"trust-actions-only","simulation":true},"planning":planning::capabilities(),"mission":mission::capabilities(),"settings":["describe","show","preview","apply"],"preparations":["list","methods","show","history","create","save","method","adopt-brief","validate-plan","answer-questions"],"admin_advice_budget":if cfg!(target_os="linux"){vec!["show","preview","apply"]}else{vec![]},"command_input_transport":swarm_engine::CommandInputTransport::current()})
            }
            Self::ObservedHttp1 => {
                serde_json::json!({"schema_version":1,"profile":"rust-native-http1-observed","actions":["task"],"trust":{"biba":"strict-with-named-endorsement-v1","scope":"trust-actions-only","simulation":true},"planning":planning::capabilities(),"mission":mission::capabilities(),"settings":["describe","show","preview","apply"],"preparations":["list","methods","show","history","create","save","method","adopt-brief","validate-plan","answer-questions"],"admin_advice_budget":if cfg!(target_os="linux"){vec!["show","preview","apply"]}else{vec![]},"command_input_transport":swarm_engine::CommandInputTransport::current()})
            }
        }
    }
}
/// Trusted service configuration: private Root path and bound-address capability.
/// Request data cannot replace either. No Root is initialized implicitly; each
/// supported operation opens/checks/closes the configured Root through its API,
/// and compares it with an owned startup descriptor binding.
/// Persisted session and local browser bootstrap are available. The separate HTTP1
/// adapter owns its listener loop; bearer expiry/rotation, worker launch and full
/// cockpit are not supplied by this dispatcher.
pub struct NativeHttpService {
    root: PathBuf,
    authority: LocalWebAuthority,
    binding: RootBinding,
    cookie_max_age_seconds: Option<i64>,
}
impl NativeHttpService {
    /// Bootstrap the native service and persisted private session from one trusted
    /// writable Root and the real bound loopback listener. NOREPLACE publication
    /// reuses Go-compatible complete bytes; invalid state refuses without repair.
    /// Binds the datastore before closing startup Root; all failures retain typed
    /// causes and checked binding/store cleanup. A successfully read/published
    /// credential survives later failure, marked explicitly without exposing it.
    /// No accept loop, expiry/rotation or business authority follows. Public local
    /// login uses the same saved cookie policy; full cockpit routes remain unported.
    pub fn open_local(
        root_path: &Path,
        listener: &std::net::TcpListener,
    ) -> Result<Self, HttpServiceError> {
        let store = RootStore::open(root_path, true).map_err(|cause| root("open", cause))?;
        let cookie_max_age_seconds = match Self::cookie_age(&store) {
            Ok(age) => age,
            Err(error) => return close(Err(error), store.close()),
        };
        Self::from_open_store(root_path, store, listener, cookie_max_age_seconds)
    }
    /// Consume the already-open trusted Root supplying listener policy. Session,
    /// binding and checked store close share that descriptor; no second path open
    /// can substitute a different Root between policy read and credential setup.
    /// Cookie age is supplied from the same validated policy/revision snapshot
    /// as the caller transport; never re-read newer settings during construction.
    fn from_open_store(
        root_path: &Path,
        store: RootStore,
        listener: &std::net::TcpListener,
        cookie_max_age_seconds: Option<i64>,
    ) -> Result<Self, HttpServiceError> {
        let prepared = match store.web_session(listener) {
            Err(error) => Err(HttpServiceError::Session(error)),
            Ok(session) => LocalWebAuthority::from_session(listener, &session)
                .map_err(HttpServiceError::Access)
                .and_then(|authority| {
                    store
                        .bind_root()
                        .map(|binding| (authority, binding))
                        .map_err(|cause| root("bind", cause))
                })
                .map_err(|primary| HttpServiceError::SessionState {
                    primary: Box::new(primary),
                }),
        };
        match (prepared, store.close()) {
            (Ok((authority, binding)), Ok(())) => Ok(Self {
                root: root_path.to_path_buf(),
                authority,
                binding,
                cookie_max_age_seconds,
            }),
            (Err(primary), cleanup) => close(Err(primary), cleanup),
            (Ok((_, binding)), Err(cause)) => {
                close(Err(root("bootstrap-close", cause)), binding.close()).map_err(|primary| {
                    HttpServiceError::SessionState {
                        primary: Box::new(primary),
                    }
                })
            }
        }
    }
    /// Bootstrap trusted service configuration by checking Root once and retaining
    /// an owned datastore descriptor binding, then explicitly closing that store.
    /// Root path/authority never come from request fields. Subsequent protected
    /// operations recheck binding before business reads/mutations; constructor IO
    /// is trusted startup, not an unauthenticated request. Preserve setup/close
    /// errors and release a created binding if later startup cleanup fails.
    /// Caller owes persisted credential/session policy, actual bounded transport
    /// and explicit service close; this is not OS/project-file confinement.
    pub fn open(root_path: &Path, authority: LocalWebAuthority) -> Result<Self, HttpServiceError> {
        let store = RootStore::open(root_path, false).map_err(|cause| root("open", cause))?;
        let cookie_max_age_seconds = match Self::cookie_age(&store) {
            Ok(age) => age,
            Err(error) => return close(Err(error), store.close()),
        };
        match store.bind_root() {
            Err(cause) => close(Err(root("bind", cause)), store.close()),
            Ok(binding) => match store.close() {
                Ok(()) => Ok(Self {
                    root: root_path.to_path_buf(),
                    authority,
                    binding,
                    cookie_max_age_seconds,
                }),
                Err(cause) => close(Err(root("bootstrap-close", cause)), binding.close()),
            },
        }
    }
    /// Snapshot saved v20 cookie policy through authoritative settings validation.
    /// No fallback/legacy file read, mutation, expiry/revocation or runtime hot reload.
    /// The older Go web-session.json still requires migration/arbitration before cutover.
    fn cookie_age(store: &RootStore) -> Result<Option<i64>, HttpServiceError> {
        let document = store.settings().map_err(|cause| root("settings", cause))?;
        document
            .values
            .web_transport_policy()
            .map(|p| p.cookie_max_age_seconds)
            .map_err(|cause| root("settings", RootStoreError::Settings(cause)))
    }
    /// Exact local browser resources contain no datastore operation. The caller
    /// must additionally check an empty incoming body and authorize before using
    /// cancellable lane waiting; POST/API/unknown routes never gain this behavior.
    fn browser_resource<B>(&self, request: &Request<B>) -> bool {
        matches!(*request.method(), Method::GET | Method::HEAD)
            && (login::public(request) || native_admin::resource(request) || live::resource(request))
    }
    /// Like Go's public landing, a GET/HEAD / without a usable session renders
    /// login only. Malformed/ambiguous headers and other access failures keep
    /// their typed refusal; no API/private asset receives this exception.
    fn root_login<B>(&self, request: &Request<B>) -> bool {
        request.uri().path() == "/"
            && matches!(*request.method(), Method::GET | Method::HEAD)
            && matches!(
                self.authority.authorize(request),
                Err(WebAccessError::Refused("web-session-required"))
            )
    }
    /// Gate the exact public bootstrap allowlist by Host and login POST Origin;
    /// every other route keeps the scoped session/CSRF contract before body/Root IO.
    fn authorize_head<B>(&self, request: &Request<B>) -> Result<(), WebAccessError> {
        if otel::owns(request.uri().path()) {
            // Machine exporters: Host + header checks, token verified in the job.
            self.authority.authorize_host(request)?;
            return otel::authorize_head(request);
        }
        if login::public(request) || self.root_login(request) {
            self.authority.authorize_login(request)
        } else {
            self.authority.authorize(request)
        }
    }
    /// Explicitly close the datastore binding after the owner has drained all
    /// requests. Errors remain typed; Drop alone does not prove checked release.
    /// No project data, credential or agent is removed/stopped.
    pub fn close(self) -> Result<(), HttpServiceError> {
        self.binding
            .close()
            .map_err(|cause| root("binding-close", cause))
    }
    /// Gate every protected route before body decode/per-request Root open, then dispatch the
    /// supported Go /works list and /action kind=task subset. Unsupported routes/
    /// methods/actions fail visibly; business transitions remain Root-owned.
    /// JSON framing uses the existing native command transport bound, disclosed
    /// by /native-capabilities; no extra hidden64KiB or timeout/retry is invented.
    /// Return response plus typed failure and independently retained known state;
    /// the transport must carry that state after a failed write/flush. Serialization of an error response
    /// itself returns a compound Err, never an empty successful fallback.
    /// HEAD keeps status/headers but omits body. This handles typed requests only,
    /// not TCP parsing/draining, credential creation, expiry or output flushing.
    pub fn handle(&self, request: Request<Vec<u8>>) -> Result<HttpReply, HttpServiceError> {
        self.handle_profile(request, DispatchProfile::Typed)
    }
    /// Dispatch the observed HTTP1 profile. The owning transport must retain every
    /// Root job/result and observe its effects before handing off response bytes.
    /// This method alone supplies no observation, peer receipt or business rights.
    fn handle_observed_transport(
        &self,
        request: Request<Vec<u8>>,
    ) -> Result<HttpReply, HttpServiceError> {
        self.handle_profile(request, DispatchProfile::ObservedHttp1)
    }
    /// Shared public-bootstrap/protected authorization and response/error/state encoding.
    /// The finite public allowlist grants no API access. Profile changes capability
    /// reporting only after protected access; it grants no business rights.
    fn handle_profile(
        &self,
        request: Request<Vec<u8>>,
        profile: DispatchProfile,
    ) -> Result<HttpReply, HttpServiceError> {
        let head = request.method() == Method::HEAD;
        if login::public(&request) || self.root_login(&request) {
            return self
                .authorize_head(&request)
                .map_err(HttpServiceError::Access)
                .and_then(|()| self.public_reply(&request))
                .or_else(|error| error_reply(error, head));
        }
        if otel::owns(request.uri().path()) {
            return self
                .authority
                .authorize_host(&request)
                .and_then(|()| otel::authorize_head(&request))
                .map_err(HttpServiceError::Access)
                .and_then(|()| self.otel_ingest(&request))
                .map(|body| HttpReply {
                    response: response(StatusCode::OK, body.bytes, head, None),
                    failure: None,
                    recorded_work: None,
                    configuration_may_be_recorded: false,
                    recorded_configuration: None,
                })
                .or_else(|error| error_reply(error, head));
        }
        if live::owns(request.uri().path()) {
            return self
                .authority
                .authorize(&request)
                .map_err(HttpServiceError::Access)
                .and_then(|()| live::reply(&request))
                .or_else(|error| error_reply(error, head));
        }
        if native_admin::owns(request.uri().path()) {
            return self
                .authority
                .authorize(&request)
                .map_err(HttpServiceError::Access)
                .and_then(|()| native_admin::reply(&request))
                .or_else(|error| error_reply(error, head));
        }
        let result = self
            .authority
            .authorize(&request)
            .map_err(HttpServiceError::Access)
            .and_then(|()| self.route(&request, profile));
        match result {
            Ok(body) => {
                let mut response = response(StatusCode::OK, body.bytes, head, None);
                if let Some(publication)=body.trust_publication{response.extensions_mut().insert(observation::TrustPublication(publication));}
                if !body.observation_causes.is_empty() {
                    response.extensions_mut().insert(observation::ObservationCauses(
                        body.observation_causes.into_iter().map(std::sync::Arc::new).collect()
                    ));
                }
                if let Some(receipt)=body.preparation_receipt { response.extensions_mut().insert(observation::PreparationReceipt(receipt)); }
                Ok(HttpReply { response, failure: None, recorded_work: body.recorded_work,
                    configuration_may_be_recorded: body.configuration_recorded,
                    recorded_configuration: body.recorded_configuration })
            },
            Err(error) => error_reply(error, head),
        }
    }
    /// Exact limited route dispatch; no client path becomes a file/Root or shell
    /// command. GET/HEAD remain observational; POST action/settings apply can mutate.
    /// Native version returns compiled identity and embedded release notes only;
    /// malformed declarations preserve typed causes without borrowing live Git.
    /// Shared session/Host admission still precedes this read-only route.
    fn route(
        &self,
        request: &Request<Vec<u8>>,
        profile: DispatchProfile,
    ) -> Result<RouteBody, HttpServiceError> {
        match request.uri().path() {
            "/api/v1/native-version" => {
                if request.method() != Method::GET && request.method() != Method::HEAD {
                    return Err(HttpServiceError::Method("GET, HEAD"));
                }
                // Compiled declarations and embedded notes; no source HEAD is borrowed.
                let binary = swarm_engine::binary_version().map_err(|cause| root(
                    "version", swarm_engine::RootStoreError::Runtime(swarm_engine::RuntimeError::new(
                        swarm_engine::RuntimeOperation::Platform, cause))))?;
                let history: serde_json::Value = serde_json::from_slice(include_bytes!("../../../../version_history.json"))
                    .map_err(HttpServiceError::Json)?;
                serde_json::to_vec(&serde_json::json!({"schema_version":1,"binary":binary,"history":history}))
                    .map(RouteBody::plain).map_err(HttpServiceError::Json)
            }
            "/api/v1/native-session" => {
                if request.method() != Method::GET && request.method() != Method::HEAD {
                    return Err(HttpServiceError::Method("GET, HEAD"));
                }
                serde_json::to_vec(&serde_json::json!({"schema_version":1,"csrf_token":self.authority.csrf_token(),"csrf_format":"hmac-sha256-v1","legacy_csrf_header":true,"host":self.authority.host()})).map(RouteBody::plain).map_err(HttpServiceError::Json)
            }
            "/api/v1/native-capabilities" => {
                if request.method() != Method::GET && request.method() != Method::HEAD {
                    return Err(HttpServiceError::Method("GET, HEAD"));
                }
                serde_json::to_vec(&profile.capabilities())
                    .map(RouteBody::plain)
                    .map_err(HttpServiceError::Json)
            }
            "/api/v1/preparations/command" => self.preparation_command(request),
            "/api/v1/preparations" | "/api/v1/preparations/methods" | "/api/v1/preparations/show" | "/api/v1/preparations/history" => self.preparations(request),
            "/api/v1/works" => {
                if request.method() != Method::GET && request.method() != Method::HEAD {
                    return Err(HttpServiceError::Method("GET, HEAD"));
                }
                self.list().map(RouteBody::plain)
            }
            "/api/v1/live/overview" => self.live_overview(request),
            "/api/v1/live/agent" => self.live_agent(request),
            "/api/v1/live/run" => self.live_run(request),
            "/api/v1/live/annotations" => self.live_annotations(request),
            "/api/v1/live/dataset" => self.live_dataset(request),
            "/api/v1/live/scenario" => self.live_scenario(request),
            "/api/v1/live/fingerprint" => self.live_fingerprint(request),
            "/api/v1/live/alerts" => self.live_alerts(request),
            "/api/v1/live/alerts/evaluate" => self.live_alerts_evaluate(request),
            // Streams exist only on the owned HTTP/1 transport; a typed/in-process
            // call or a refused shape gets an explicit code, never an empty stream.
            "/api/v1/live/events" => {
                if request.method() != Method::GET {
                    return Err(HttpServiceError::Method("GET"));
                }
                if request.uri().query().is_some_and(|q| !q.is_empty()) {
                    return Err(HttpServiceError::Refused("live-query-invalid"));
                }
                Err(HttpServiceError::Refused("live-events-transport-required"))
            }
            "/api/v1/planning" => self.planning(request),
            "/api/v1/mission" => self.mission(request),
            "/api/v1/pricing" => self.pricing_route(if request.method() == Method::POST { pricing::PricingOperation::Save } else { pricing::PricingOperation::Show }, request),
            "/api/v1/pricing/estimate" => self.pricing_route(pricing::PricingOperation::Estimate, request),
            "/api/v1/work-resume" => self.work_resume(work_resume::Operation::Calculated, request),
            "/api/v1/work-recorded-resume" => self.work_resume(work_resume::Operation::Recorded, request),
            "/api/v1/agent-transcript" => self.agent_transcript(request),
            "/api/v1/agent-logs" => self.agent_logs(request),
            "/api/v1/cockpit-read" => self.cockpit_read(request),
            "/api/v1/work-budget" => self.work_budget_route(work_budget::Operation::Show,request),
            "/api/v1/work-budget/preview" => self.work_budget_route(work_budget::Operation::Preview,request),
            "/api/v1/work-budget/apply" => self.work_budget_route(work_budget::Operation::Apply,request),
            "/api/v1/action" => {
                if request.method() != Method::POST {
                    return Err(HttpServiceError::Method("POST"));
                }
                let action = task_action(request.body())?;
                self.task(action)
            }
            #[cfg(target_os = "linux")]
            "/api/v1/admin-advice-budget"=>self.admin_budget(admin_budget::Operation::Show,request),
            #[cfg(target_os = "linux")]
            "/api/v1/admin-advice-budget/preview"=>self.admin_budget(admin_budget::Operation::Preview,request),
            #[cfg(target_os = "linux")]
            "/api/v1/admin-advice-budget/apply"=>self.admin_budget(admin_budget::Operation::Apply,request),
            #[cfg(target_os = "linux")]
            "/api/v1/quotas" => self.quotas(quotas::Operation::Show,request),
            #[cfg(target_os = "linux")]
            "/api/v1/quotas/preview" => self.quotas(quotas::Operation::Preview,request),
            #[cfg(target_os = "linux")]
            "/api/v1/quotas/apply" => self.quotas(quotas::Operation::Apply,request),
            "/api/v1/trust-actions" => self.trust_actions(request),
            "/api/v1/protection/receipt-report" => paea::receipt_report(request),
            "/api/v1/protection/resolve-payment" => paea::resolve_payment(request),
            "/api/v1/protection/compare" => paea::compare(request),
            "/api/v1/protection/inspect" => paea::inspect(request),
            "/api/v1/native-settings" => self.engine_settings(SettingsOperation::Show, request),
            "/api/v1/native-settings/describe" => {
                self.engine_settings(SettingsOperation::Describe, request)
            }
            "/api/v1/native-settings/preview" => {
                self.engine_settings(SettingsOperation::Preview, request)
            }
            "/api/v1/native-settings/apply" => {
                self.engine_settings(SettingsOperation::Apply, request)
            }
            _ => Err(HttpServiceError::Refused("web-route-unported")),
        }
    }
    /// Read the full actual Work list under Root ownership/schema checks; a
    /// failed read/close cannot be converted to an empty list. No SQL mutation.
    fn list(&self) -> Result<Vec<u8>, HttpServiceError> {
        let store = RootStore::open(&self.root, false).map_err(|cause| root("open", cause))?;
        let result = self
            .check_binding(&store)
            .and_then(|()| store.list_works().map_err(|cause| root("read", cause)))
            .and_then(|works| swarm_engine::encode_model(&works).map_err(HttpServiceError::Model));
        close(result, store.close())
    }
    /// Refuse a different opened datastore before business reads/mutations.
    /// The retained FD prevents inode reuse; it does not lock relative project
    /// files or defend against arbitrary same-UID filesystem/database writes.
    fn check_binding(&self, store: &RootStore) -> Result<(), HttpServiceError> {
        if !self
            .binding
            .matches(store)
            .map_err(|cause| root("binding", cause))?
        {
            return Err(HttpServiceError::Refused("web-root-binding-changed"));
        }
        Ok(())
    }
    /// Apply server-derived outer fields through Root's common external boundary;
    /// no Request origin can impersonate engine/conductor authority. Known Work
    /// survives encoding/close failure; Root late-release metadata also propagates.
    fn task(&self, mut action: TaskAction) -> Result<RouteBody, HttpServiceError> {
        action.request.schema_version = 1;
        action.request.event_id = action.event_id;
        action.request.expected_revision = action.expected_revision;
        action.request.id = action.task;
        action.request.origin.clear();
        let raw =
            swarm_engine::encode_request(&action.request).map_err(HttpServiceError::Request)?;
        let mut store = RootStore::open(&self.root, true).map_err(|cause| root("open", cause))?;
        let mut recorded = None;
        let result = (|| {
            self.check_binding(&store)?;
            let outcome = store
                .execute_external(&action.work, "task.update", &raw, &mut SystemEnvironment)
                .map_err(|cause| root("mutate", cause))?;
            let work = outcome.result.work;
            recorded = Some((work.id.clone(), work.revision));
            swarm_engine::encode_model(&work)
                .map(|bytes| RouteBody { trust_publication:None, preparation_receipt:None, observation_causes: Vec::new(),
                    bytes,
                    recorded_work: recorded.clone(),
                    configuration_recorded: false,
                    recorded_configuration: None,
                })
                .map_err(HttpServiceError::Model)
        })();
        let result = close(result, store.close());
        match (result, recorded) {
            (Err(primary), Some((work, revision))) => Err(HttpServiceError::Recorded {
                primary: Box::new(primary),
                work,
                revision,
            }),
            (result, _) => result,
        }
    }
}
/// Build an explicit failed HTTP reply while preserving original/secondary
/// causes and known recorded Work metadata. Failed JSON encoding returns compound
/// Err; it never manufactures empty success or rolls back a known transaction.
fn error_reply(error: HttpServiceError, head: bool) -> Result<HttpReply, HttpServiceError> {
    let mut document = serde_json::json!({"schema_version":1,"error":error.code(),"failure":{"code":error.code()}});
    if let Some((work, revision)) = error.recorded_work() {
        document["recorded_work"] = serde_json::json!({"id":work,"revision":revision});
    }
    if let Some(publication)=error.trust_publication(){document["trust_publication"]=publication.into();document["trust_state_may_be_recorded"]=true.into();}
    let configuration_may_be_recorded = error.configuration_may_be_recorded();
    if configuration_may_be_recorded {
        document["configuration_may_be_recorded"] = true.into();
    }
    let recorded_work = error
        .recorded_work()
        .map(|(id, revision)| (id.to_owned(), revision));
    let preparation_receipt=error.recorded_preparation().map(|(id,revision)|(id.to_owned(),revision));
    if let Some((id,revision))=&preparation_receipt {document["recorded_preparation"]=serde_json::json!({"id":id,"revision":revision});}
    let recorded_configuration = error.recorded_configuration().cloned();
    if let Some(configuration) = &recorded_configuration {
        match serde_json::to_value(configuration) {
            Ok(value) => document["recorded_configuration"] = value,
            Err(cause) => {
                return Err(HttpServiceError::ResponseEncoding {
                    primary: Box::new(error),
                    cause,
                })
            }
        }
    }
    let body = match serde_json::to_vec(&document) {
        Ok(body) => body,
        Err(cause) => {
            return Err(HttpServiceError::ResponseEncoding {
                primary: Box::new(error),
                cause,
            })
        }
    };
    let allow = match &error {
        HttpServiceError::Method(allow) => Some(*allow),
        _ => None,
    };
    let mut response=response(error.status(),body,head,allow);
    if let Some(receipt)=preparation_receipt {response.extensions_mut().insert(observation::PreparationReceipt(receipt));}
    Ok(HttpReply {
        response,
        failure: Some(error),
        recorded_work,
        configuration_may_be_recorded,
        recorded_configuration,
    })
}
/// Existing Go task-action outer fields only. A partial native adapter refuses
/// unsupported fields/operations, never silently ignores an execution request.
#[derive(Default, Deserialize)]
#[serde(default)]
struct TaskAction {
    kind: String,
    work: String,
    task: String,
    event_id: String,
    expected_revision: i64,
    request: swarm_engine::Request,
}
/// Decode the common Go null/case/merge shape using the actual generated Request
/// descriptor. Require task/event/work identities and positive revision before
/// Root open; current business/revision/origin rules remain authoritative there.
fn task_action(raw: &[u8]) -> Result<TaskAction, HttpServiceError> {
    swarm_engine::check_command_input_size(raw).map_err(HttpServiceError::Input)?;
    const FIELDS: Fields = &[
        ("kind", Kind::String),
        ("work", Kind::String),
        ("task", Kind::String),
        ("event_id", Kind::String),
        ("expected_revision", Kind::Int64),
        ("request", Kind::Object(swarm_engine::request_fields())),
    ];
    let value = projection::project(raw, FIELDS, true).map_err(HttpServiceError::Projection)?;
    let action: TaskAction = serde_json::from_value(value).map_err(HttpServiceError::Json)?;
    if action.kind != "task" {
        return Err(HttpServiceError::Refused("web-operation-unported"));
    }
    if [&action.work, &action.task, &action.event_id]
        .iter()
        .any(|s| !swarm_identity_contract::name(s.as_bytes()))
        || action.expected_revision < 1
    {
        return Err(HttpServiceError::Refused("web-action-invalid"));
    }
    Ok(action)
}
/// Retain the exact Root operation/cause; static operation data is diagnostic,
/// never client-granted permission or an implicit retry decision.
fn root(operation: &'static str, cause: RootStoreError) -> HttpServiceError {
    HttpServiceError::Root {
        operation,
        cause: Box::new(cause),
    }
}
/// Join primary result and explicit Root close, retaining both failures. No
/// successful empty body, rollback or retry is inferred from a cleanup failure.
fn close<T>(
    result: Result<T, HttpServiceError>,
    cleanup: Result<(), RootStoreError>,
) -> Result<T, HttpServiceError> {
    match (result, cleanup) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(cleanup)) => Err(HttpServiceError::Close {
            primary: None,
            cleanup: Box::new(cleanup),
        }),
        (Err(primary), Err(cleanup)) => Err(HttpServiceError::Close {
            primary: Some(Box::new(primary)),
            cleanup: Box::new(cleanup),
        }),
    }
}
/// Fixed protocol/security response headers; these are not operator execution
/// policies. HTTP error bodies remain JSON/no-store and HEAD emits no content.
fn response(
    status: StatusCode,
    body: Vec<u8>,
    head: bool,
    allow: Option<&'static str>,
) -> Response<Vec<u8>> {
    let mut r = Response::new(if head { vec![] } else { body });
    *r.status_mut() = status;
    for (name,value) in [(CONTENT_TYPE,"application/json; charset=utf-8"),(CACHE_CONTROL,"no-store"),(http::header::X_CONTENT_TYPE_OPTIONS,"nosniff"),(http::header::REFERRER_POLICY,"no-referrer"),(http::header::CONTENT_SECURITY_POLICY,"default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self' data:; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'")]{r.headers_mut().insert(name,HeaderValue::from_static(value));}
    if let Some(allow) = allow {
        r.headers_mut()
            .insert(ALLOW, HeaderValue::from_static(allow));
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Quota validation refusals are client/state/capability responses. Corrupt
    /// history, encoding, clock and settings failures stay server errors; no
    /// prefix-based catch-all may hide an internal fault as invalid client data.
    #[test]
    fn quota_status_preserves_client_conflict_and_internal_failures() {
        for (code,status) in [
            ("quota-request-invalid",StatusCode::BAD_REQUEST),
            ("quota-reason-outside-policy",StatusCode::BAD_REQUEST),
            ("quota-below-consumption",StatusCode::BAD_REQUEST),
            ("quota-review-timeout-invalid",StatusCode::BAD_REQUEST),
            ("quota-review-timeout-unrepresentable",StatusCode::BAD_REQUEST),
            ("quota-planner-owner-active",StatusCode::CONFLICT),
            ("quota-reviewer-running",StatusCode::CONFLICT),
            ("quota-revision-conflict",StatusCode::CONFLICT),
            ("quota-client-window-unported",StatusCode::NOT_IMPLEMENTED),
            ("quota-hierarchy-invalid",StatusCode::INTERNAL_SERVER_ERROR),
            ("quota-consumption-invalid",StatusCode::INTERNAL_SERVER_ERROR),
            ("quota-request-encoding",StatusCode::INTERNAL_SERVER_ERROR),
            ("quota-time-invalid",StatusCode::INTERNAL_SERVER_ERROR),
            ("settings-quota-reason-not-configured",StatusCode::INTERNAL_SERVER_ERROR),
        ] {
            let error=root("quotas-preview",RootStoreError::Launch(Box::new(swarm_engine::AgentLaunchError::Refused(code))));
            assert_eq!(error.status(),status,"{code}");
        }
    }

    /// Reproduce a saved settings update between server-policy read and service
    /// construction. The cookie must use the same validated snapshot as transport,
    /// not silently re-read a later revision. No scheduling delay or provider used.
    #[test]
    fn native_cookie_policy_keeps_the_server_snapshot_when_saved_values_change() {
        use std::{fs, net::TcpListener, os::unix::fs::PermissionsExt};
        let root = std::env::temp_dir()
            .join(swarm_engine::generate_identifier("cookie-snapshot98-").unwrap());
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let store = RootStore::initialize(&root, &mut SystemEnvironment).unwrap();
        let document = store.settings().unwrap();
        let policy = document.values.web_transport_policy().unwrap();
        let mut values = document.values;
        values.web_cookie_max_age_seconds = Some(swarm_engine::SettingCap(Some(
            swarm_engine::SettingCount(7200),
        )));
        let mut change = swarm_engine::EngineSettingsChange {
            schema_version: document.schema_version,
            event_id: "concurrent-cookie-change".into(),
            expected_revision: document.revision,
            values,
            reason: "Snapshot regression fixture".into(),
            preview_token: String::new(),
        };
        change.preview_token = store.preview_settings(&change).unwrap().preview_token;
        store
            .apply_settings(&change, &mut SystemEnvironment)
            .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let service = NativeHttpService::from_open_store(
            &root,
            store,
            &listener,
            policy.cookie_max_age_seconds,
        )
        .unwrap();
        assert_eq!(
            service.cookie_max_age_seconds,
            policy.cookie_max_age_seconds
        );
        service.close().unwrap();
        let restarted = NativeHttpService::open_local(&root, &listener).unwrap();
        assert_eq!(restarted.cookie_max_age_seconds, Some(7200));
        restarted.close().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    /// Typed classification from the retained pre93 counterexample; internal
    /// runtime failure is server-side, not a caller refusal. No OS fault implied.
    #[test]
    fn root_runtime_failure_is_server_error() {
        let witness = root(
            "mutate",
            RootStoreError::Refused("fixture-business-refusal"),
        );
        assert_eq!(witness.status(), StatusCode::BAD_REQUEST);
        let error = root(
            "mutate",
            RootStoreError::Runtime(swarm_engine::RuntimeError::new(
                swarm_engine::RuntimeOperation::Entropy,
                std::io::Error::other("fixture runtime unavailable"),
            )),
        );
        assert_eq!(error.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(error.source().is_some());
    }
    /// Known publication, visible uncertain publication and secondary cleanup
    /// survive HTTP JSON plus independent metadata. This is a typed aggregation
    /// model, not an actual OS close fault or network transmission proof.
    #[test]
    fn settings_recorded_state_survives_http_close_json_and_delivery_metadata() {
        let witness = error_reply(
            root(
                "settings-apply",
                RootStoreError::Refused("fixture-before-write"),
            ),
            false,
        )
        .unwrap();
        assert!(!witness.configuration_may_be_recorded);
        for primary in [
            HttpServiceError::ConfigurationState {
                primary: Box::new(root(
                    "settings-apply",
                    RootStoreError::Refused("fixture-after-save"),
                )),
            },
            root(
                "settings-apply",
                RootStoreError::Published {
                    primary: Box::new(RootStoreError::Refused(
                        "fixture-visible-durability-unknown",
                    )),
                },
            ),
        ] {
            let error = close::<Vec<u8>>(
                Err(primary),
                Err(RootStoreError::Refused("fixture-extra-close")),
            )
            .unwrap_err();
            assert!(error.configuration_may_be_recorded());
            let reply = error_reply(error, false).unwrap();
            assert_eq!(reply.response.status(), StatusCode::INTERNAL_SERVER_ERROR);
            assert!(reply.configuration_may_be_recorded);
            assert!(reply.recorded_work.is_none());
            let body: serde_json::Value = serde_json::from_slice(reply.response.body()).unwrap();
            assert_eq!(body["configuration_may_be_recorded"], true);
            let error = reply.failure.unwrap();
            assert!(error.source().is_some());
            match error {
                HttpServiceError::Close {
                    primary: Some(primary),
                    cleanup,
                } => {
                    assert!(primary.configuration_may_be_recorded());
                    assert_eq!(cleanup.code(), "fixture-extra-close");
                }
                _ => panic!("lost original causes"),
            }
        }
        let reply = error_reply(
            HttpServiceError::Close {
                primary: None,
                cleanup: Box::new(RootStoreError::Published {
                    primary: Box::new(RootStoreError::Refused("fixture-secondary-visible")),
                }),
            },
            true,
        )
        .unwrap();
        assert!(reply.configuration_may_be_recorded);
        assert!(reply.response.body().is_empty());
        assert_eq!(reply.response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
    /// Startup aggregation model, not an injected OS failure: both primary and
    /// cleanup causes remain available, with known session state and safe JSON.
    #[test]
    fn persisted_session_marker_survives_startup_close_and_error_rendering() {
        let witness = HttpServiceError::Refused("fixture-before-session");
        assert!(!witness.session_state_may_be_recorded());
        let primary = HttpServiceError::SessionState {
            primary: Box::new(root("bind", RootStoreError::Refused("fixture-bind"))),
        };
        let error = close::<Vec<u8>>(Err(primary), Err(RootStoreError::Refused("fixture-close")))
            .unwrap_err();
        assert!(error.session_state_may_be_recorded());
        assert!(error.source().unwrap().source().is_some());
        let reply = error_reply(error, false).unwrap();
        assert_eq!(reply.response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(reply
            .failure
            .as_ref()
            .unwrap()
            .session_state_may_be_recorded());
        assert!(
            serde_json::from_slice::<serde_json::Value>(reply.response.body())
                .unwrap()
                .get("recorded_work")
                .is_none()
        );
        match reply.failure.unwrap() {
            HttpServiceError::Close {
                primary: Some(primary),
                cleanup,
            } => {
                assert_eq!(primary.code(), "web-session-state-preserved");
                assert_eq!(cleanup.code(), "fixture-close");
            }
            _ => panic!("lost startup causes"),
        }
    }
    /// Aggregation model only: Root/second-close failure remains HTTP500 with
    /// known Work metadata and both typed causes, never an actual OS-fault claim.
    #[test]
    fn root_recorded_state_survives_http_close_and_json_response() {
        let recorded = RootStoreError::WorkRecorded {
            primary: Box::new(RootStoreError::Refused("fixture-release")),
            work: "w-fixture".into(),
            revision: 3,
        };
        let error = close::<Vec<u8>>(
            Err(root("mutate", recorded)),
            Err(RootStoreError::Refused("fixture-extra-close")),
        )
        .unwrap_err();
        assert_eq!(error.recorded_work(), Some(("w-fixture", 3)));
        let reply = error_reply(error, false).unwrap();
        assert_eq!(reply.response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(reply.recorded_work, Some(("w-fixture".into(), 3)));
        assert!(!reply.configuration_may_be_recorded);
        let value: serde_json::Value = serde_json::from_slice(reply.response.body()).unwrap();
        assert_eq!(value["recorded_work"]["id"], "w-fixture");
        assert_eq!(value["recorded_work"]["revision"], 3);
        match reply.failure.unwrap() {
            HttpServiceError::Close {
                primary: Some(primary),
                cleanup,
            } => {
                assert_eq!(primary.code(), "work-state-preserved");
                assert_eq!(cleanup.code(), "fixture-extra-close");
                assert!(primary.source().is_some());
            }
            _ => panic!("lost compound error"),
        }
        let reply = error_reply(HttpServiceError::Refused("fixture-before-commit"), false).unwrap();
        let value: serde_json::Value = serde_json::from_slice(reply.response.body()).unwrap();
        assert!(value.get("recorded_work").is_none());
    }
}

mod preparations;
