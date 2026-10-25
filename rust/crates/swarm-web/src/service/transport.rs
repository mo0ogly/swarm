//! Native HTTP/1 jobs remain owned independently of their clients. Each completed
//! outcome is observed before response handoff; observer failure stops admission.
use super::live_events::{self, WireBody};
use super::{
    listener::finish, HttpReply, HttpServiceError, NativeHttpServer, NativeHttpService,
    NativeJobOutcome, NativeServerError,
};
use http::{Method, Request, Response, StatusCode};
use http_body_util::{BodyExt, Full, Limited};
use hyper::{
    body::{Bytes, Incoming},
    server::conn::http1,
    service::service_fn,
};
use hyper_util::rt::{TokioIo, TokioTimer};
use std::{convert::Infallible, future::Future, io, sync::Arc};
use swarm_engine::{WebTransportConstraints, WebTransportPolicy};
use tokio::{
    sync::{mpsc, oneshot, OwnedSemaphorePermit, Semaphore},
    task::JoinSet,
};
pub(super) type Body = Full<Bytes>;
/// A queued refusal keeps its connection reservation until owner consumption.
/// None is the explicitly configured unlimited connection policy.
type ConnectionLease = Arc<Option<OwnedSemaphorePermit>>;
/// Owner callback is required even for reads. Acknowledgement is not durability,
/// HTTP receipt, authenticated identity or acceptance; caller owns those proofs.
pub type NativeOutcomeObserver<'a> = dyn FnMut(&NativeJobOutcome) -> io::Result<()> + 'a;
/// Counters and first parser/request/service failures for one serve cycle. Each
/// completed job additionally reaches the required observer, or remains in the
/// returned observation failure. Counts include replay, not just new mutations.
#[must_use = "Inspect failures and retain observed/pending effects; shutdown is not acceptance"]
#[derive(Default)]
pub struct NativeServerReport {
    pub accepted_connections: u64,
    pub completed_jobs: u64,
    pub rejected_connections: u64,
    pub connection_failures: u64,
    pub first_connection_failure: Option<hyper::Error>,
    pub first_service_failure: Option<HttpServiceError>,
    pub rejected_requests: u64,
    pub disconnected_replies: u64,
    pub first_request_failure: Option<NativeRequestError>,
    pub observed_outcomes: u64,
    pub configuration_outcomes: u64,
    pub work_outcomes: u64,
    pub rejected_queued_jobs: u64,
    /// Last acknowledged known values, for late stop/output failures only. They
    /// do not replace the full observer stream or prove current active state.
    pub last_recorded_work: Option<(String, i64)>,
    pub last_recorded_configuration: Option<super::RecordedConfiguration>,
}
/// Safe request failure family; original parse/access/timeout causes remain
/// accessible through Error::source. No raw request or credential is logged.
pub enum NativeRequestError {
    Access(crate::WebAccessError),
    Body(Box<dyn std::error::Error + Send + Sync>),
    Deadline(tokio::time::error::Elapsed),
    WorkersUnavailable(tokio::sync::TryAcquireError),
    WorkersStopped(tokio::sync::AcquireError),
    Refused(&'static str),
}
impl NativeRequestError {
    /// Stable safe family with typed admission causes preserved separately. A
    /// closed lane differs from capacity pressure; neither grants retry authority.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Access(e) => e.code(),
            Self::Body(_) => "web-body-refused",
            Self::Deadline(_) => "web-body-timeout",
            Self::WorkersUnavailable(tokio::sync::TryAcquireError::Closed)
            | Self::WorkersStopped(_) => "web-workers-stopping",
            Self::WorkersUnavailable(tokio::sync::TryAcquireError::NoPermits) => {
                "web-workers-saturated"
            }
            Self::Refused(code) => code,
        }
    }
}
impl std::fmt::Display for NativeRequestError {
    /// Render only the safe family; never request/body/credential/cause text.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::fmt::Debug for NativeRequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
impl std::error::Error for NativeRequestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Access(e) => Some(e),
            Self::Body(e) => Some(e.as_ref()),
            Self::Deadline(e) => Some(e),
            Self::WorkersUnavailable(e) => Some(e),
            Self::WorkersStopped(e) => Some(e),
            Self::Refused(_) => None,
        }
    }
}

/// Queued job owns a lane; queued diagnostic owns its connection reservation.
pub(super) enum Message {
    Job(Box<Submission>),
    Rejected(NativeRequestError, ConnectionLease),
}
/// Body passed access/framing checks; Root is not executed until owner dispatch.
pub(super) struct Submission {
    pub(super) request: Request<Vec<u8>>,
    pub(super) reply: oneshot::Sender<Response<Body>>,
    pub(super) permit: OwnedSemaphorePermit,
}
/// Retain the lane through Root completion AND observation. Dropping a client
/// cannot free active/completed-but-unobserved job capacity.
struct JobCompletion {
    result: Result<HttpReply, HttpServiceError>,
    reply: oneshot::Sender<Response<Body>>,
    permit: OwnedSemaphorePermit,
    head: bool,
}
/// Encounter-order fatal aggregation and one live observer. Once its output fails,
/// retain all later admitted results instead of retrying the same failed output.
struct OutcomeOwner<'a> {
    observer: &'a mut NativeOutcomeObserver<'a>,
    failure: Option<NativeServerError>,
}
impl OutcomeOwner<'_> {
    /// Preserve the first encountered failure and every subsequent cause.
    fn fail(&mut self, error: NativeServerError) {
        self.failure = Some(match self.failure.take() {
            Some(primary) => NativeServerError::Cleanup {
                primary: Box::new(primary),
                cleanup: Box::new(error),
            },
            None => error,
        });
    }
    /// Call a healthy observer once, or retain without retry after its first failure.
    fn observe(&mut self, outcome: NativeJobOutcome) -> Option<NativeJobOutcome> {
        if let Some(failure) = self
            .failure
            .as_mut()
            .and_then(NativeServerError::observation_failure_mut)
        {
            failure.defer(outcome);
            return None;
        }
        match outcome.observe(self.observer) {
            Ok(outcome) => Some(outcome),
            Err(failure) => {
                self.fail(NativeServerError::Observation(Box::new(failure)));
                None
            }
        }
    }
}
/// Use the same operator-saved lane capacity for every admitted job. Empty-body
/// GET/HEAD browser resources may wait cancellably instead of failing a normal
/// parallel asset load. The caller has already checked Host/session; only its
/// exact local-resource classification can choose this path. Waiting holds the
/// existing connection lease, not a worker/body buffer; no third pool, deadline,
/// retry, hidden queue cap or Root mutation is added. Body/API/POST requests keep
/// immediate saturation refusal. Connection shutdown drops pending admission.
async fn acquire_lane(
    lane: Arc<Semaphore>,
    resource: bool,
) -> Result<OwnedSemaphorePermit, NativeRequestError> {
    if resource {
        lane.acquire_owned()
            .await
            .map_err(NativeRequestError::WorkersStopped)
    } else {
        lane.try_acquire_owned()
            .map_err(NativeRequestError::WorkersUnavailable)
    }
}
/// Redacted refusal body. Authentication/parser errors never disclose submitted
/// tokens/paths. Close prevents unread refused bodies becoming another request.
fn refusal(status: StatusCode, code: &'static str) -> Response<Body> {
    let mut response = Response::new(Full::new(Bytes::from_static(code.as_bytes())));
    *response.status_mut() = status;
    response.headers_mut().insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    response.headers_mut().insert(
        http::header::CACHE_CONTROL,
        http::HeaderValue::from_static("no-store"),
    );
    response.headers_mut().insert(
        http::header::CONNECTION,
        http::HeaderValue::from_static("close"),
    );
    response
}
/// Count without wrapping. Exhausting the representation stops admission and
/// drains owned work; it is a technical failure, not an operator cutoff.
fn count(value: &mut u64) -> Result<(), NativeServerError> {
    *value = value
        .checked_add(1)
        .ok_or(NativeServerError::Refused("web-counter-overflow"))?;
    Ok(())
}
/// Route an authorized plain GET of the live event stream to its connection-owned
/// body (no lane held while idle; each probe is an ordinary admitted job). Every
/// other request, including refused stream shapes, takes the ordinary job path.
async fn dispatch(
    request: Request<Incoming>,
    service: Arc<NativeHttpService>,
    policy: Arc<WebTransportPolicy>,
    root: Arc<Semaphore>,
    admin: Arc<Semaphore>,
    sender: mpsc::UnboundedSender<Message>,
    connection: ConnectionLease,
) -> Result<Response<WireBody>, Infallible> {
    if live_events::accepts(&request) && service.authorize_head(&request).is_ok() {
        return Ok(live_events::open(&request, root, sender));
    }
    let response = self::request(request, service, policy, root, admin, sender, connection).await;
    response.map(|r| r.map(WireBody::Full))
}
/// Head authorization precedes body collection/Root dispatch. Saturated lanes
/// refuse rather than enqueue. Library frame limit is separate from saved policy;
/// body deadline starts after headers and never wraps a Root operation.
async fn request(
    request: Request<Incoming>,
    service: Arc<NativeHttpService>,
    policy: Arc<WebTransportPolicy>,
    root: Arc<Semaphore>,
    admin: Arc<Semaphore>,
    sender: mpsc::UnboundedSender<Message>,
    connection: ConnectionLease,
) -> Result<Response<Body>, Infallible> {
    if let Err(error) = service.authorize_head(&request) {
        let (status, code) = super::otel::head_refusal(request.uri().path(), &error)
            .unwrap_or((StatusCode::FORBIDDEN, "web-access-refused"));
        return Ok(reject(
            &sender,
            status,
            code,
            NativeRequestError::Access(error),
            Arc::clone(&connection),
        ));
    }
    let lane = if matches!(
        request.uri().path(),
        "/api/v1/native-settings"
            | "/api/v1/native-settings/describe"
            | "/api/v1/native-settings/preview"
            | "/api/v1/native-settings/apply"
            | "/api/v1/admin-advice-budget"
            | "/api/v1/admin-advice-budget/preview"
            | "/api/v1/admin-advice-budget/apply"
            | "/api/v1/quotas"
            | "/api/v1/quotas/preview"
            | "/api/v1/quotas/apply"
    ) {
        admin
    } else {
        root
    };
    let resource =
        service.browser_resource(&request) && hyper::body::Body::is_end_stream(request.body());
    let permit = match acquire_lane(lane, resource).await {
        Ok(p) => p,
        Err(error) => {
            return Ok(reject(
                &sender,
                StatusCode::SERVICE_UNAVAILABLE,
                error.code(),
                error,
                Arc::clone(&connection),
            ))
        }
    };
    let (parts, body) = request.into_parts();
    let technical = usize::try_from(WebTransportConstraints::current().native_body_max_bytes.0)
        .expect("native input bound fits usize");
    let limit = policy.body_max_bytes.unwrap_or(technical);
    let collect = Limited::new(body, limit).collect();
    let body = match policy.body_timeout {
        Some(deadline) => match tokio::time::timeout(deadline, collect).await {
            Ok(result) => result,
            Err(error) => {
                return Ok(reject(
                    &sender,
                    StatusCode::REQUEST_TIMEOUT,
                    "web-body-timeout",
                    NativeRequestError::Deadline(error),
                    Arc::clone(&connection),
                ))
            }
        },
        None => collect.await,
    };
    let bytes = match body {
        Ok(body) => body.to_bytes().to_vec(),
        Err(error) => {
            let status = if error.is::<http_body_util::LengthLimitError>() {
                StatusCode::PAYLOAD_TOO_LARGE
            } else {
                StatusCode::BAD_REQUEST
            };
            return Ok(reject(
                &sender,
                status,
                "web-body-refused",
                NativeRequestError::Body(error),
                Arc::clone(&connection),
            ));
        }
    };
    let (reply, receive) = oneshot::channel();
    if sender
        .send(Message::Job(Box::new(Submission {
            request: Request::from_parts(parts, bytes),
            reply,
            permit,
        })))
        .is_err()
    {
        return Ok(refusal(
            StatusCode::SERVICE_UNAVAILABLE,
            "web-server-stopping",
        ));
    }
    Ok(match receive.await {
        Ok(response) => response,
        Err(_) => refusal(StatusCode::INTERNAL_SERVER_ERROR, "web-root-job-failed"),
    })
}

impl NativeHttpServer {
    /// Serve native HTTP1 with a mandatory owner observer for every completed job,
    /// before response handoff. Saved settings bound IO, separate Root/Admin lanes
    /// and blocking-thread retention; no hidden Root queue/retry/execution deadline.
    /// The callback must retain/route results as needed; Ok is acknowledgement only,
    /// not durable storage, business acceptance or peer receipt. It receives no raw
    /// request, token or body. Observer IO/unwinding panic stops admission, preserves cause and
    /// pending effects and is never retried. Shutdown aborts only connection futures,
    /// refuses not-started queued jobs, drains every started Root job and observes or
    /// retains its result before checked binding/listener close. No agent is stopped.
    /// Hyper/Tokio descriptor drops on accepted sockets/registration failures are not
    /// checked-close proofs. Request/parser first-error summaries remain separate
    /// from complete Root outcome observation; incident persistence/routing is caller-owned.
    /// The observer is synchronous on the IO owner and must return; no watchdog
    /// cancels it. A production sink must supply its own progress-safe integration.
    pub fn serve_until<'a, F: Future<Output = ()>>(
        self,
        shutdown: F,
        observer: &'a mut NativeOutcomeObserver<'a>,
    ) -> Result<NativeServerReport, NativeServerError> {
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .max_blocking_threads(
                self.policy.root_worker_threads + self.policy.admin_worker_threads,
            )
            .thread_keep_alive(self.policy.worker_idle_keep_alive)
            .build()
        {
            Ok(runtime) => runtime,
            Err(cause) => {
                return finish(
                    Err(NativeServerError::Io {
                        phase: "web-runtime-build",
                        cause,
                    }),
                    self.close(),
                )
                .map_err(Self::known_session)
            }
        };
        if let Err(cause) = self.listener.set_nonblocking(true) {
            return finish(
                Err(NativeServerError::Io {
                    phase: "web-listener-nonblocking",
                    cause,
                }),
                self.close(),
            )
            .map_err(Self::known_session);
        }
        // LIVE-UX L1 (settings v33): expired console rows go once at start.
        #[cfg(target_os = "linux")]
        if let Err(error) = self.service.startup_retention() {
            return finish(Err(NativeServerError::Service(error)), self.close()).map_err(Self::known_session);
        }
        // LIVE-UX-03: publish the actually bound loopback address so provider
        // launches can reach the OTLP receiver; retracted below only if still ours.
        #[cfg(target_os = "linux")]
        let endpoint = match self
            .listener
            .local_addr()
            .map_err(|cause| NativeServerError::Io { phase: "web-telemetry-endpoint-address", cause })
            .and_then(|address| {
                swarm_engine::publish_telemetry_endpoint(&self.service.root, address)
                    .map_err(|e| NativeServerError::Service(HttpServiceError::Telemetry(Box::new(e))))
            }) {
            Ok(endpoint) => endpoint,
            Err(error) => return finish(Err(error), self.close()).map_err(Self::known_session),
        };
        let Self {
            listener,
            service,
            policy,
            settings_revision: _,
        } = self;
        #[cfg(target_os = "linux")]
        let root_path = service.root.clone();
        let service = Arc::new(service);
        // LIVE-UX (settings v35): server-side alert evaluation, joined below
        // before the service closes; a start failure refuses the server.
        #[cfg(target_os = "linux")]
        let alert_loop = match service.start_alert_loop() {
            Ok(alert_loop) => alert_loop,
            Err(error) => {
                let cleanup = match Arc::try_unwrap(service) {
                    Ok(service) => service.close().map_err(NativeServerError::Service),
                    Err(_) => Err(NativeServerError::Refused("web-service-still-owned")),
                };
                let cleanup = finish(
                    cleanup,
                    swarm_engine::retract_telemetry_endpoint(&root_path, &endpoint)
                        .map_err(|e| NativeServerError::Service(HttpServiceError::Telemetry(Box::new(e)))),
                );
                return finish(Err(NativeServerError::Service(error)), cleanup).map_err(Self::known_session);
            }
        };
        let result = runtime.block_on(run(
            listener,
            Arc::clone(&service),
            Arc::new(policy),
            shutdown,
            observer,
        ));
        #[cfg(target_os = "linux")]
        let result = finish(
            result,
            alert_loop.map_or(Ok(()), |l| l.stop(&root_path)).map_err(NativeServerError::Service),
        );
        let cleanup = match Arc::try_unwrap(service) {
            Ok(service) => service.close().map_err(NativeServerError::Service),
            Err(_) => Err(NativeServerError::Refused("web-service-still-owned")),
        };
        #[cfg(target_os = "linux")]
        let cleanup = finish(
            cleanup,
            swarm_engine::retract_telemetry_endpoint(&root_path, &endpoint)
                .map_err(|e| NativeServerError::Service(HttpServiceError::Telemetry(Box::new(e)))),
        );
        finish(result, cleanup).map_err(Self::known_session)
    }
}
/// Submit without detaching the job or its capacity. JoinSet owns the handle;
/// returned completion owns the permit until the observer has processed it.
fn spawn_job(
    jobs: &mut JoinSet<JobCompletion>,
    service: Arc<NativeHttpService>,
    message: Submission,
) {
    jobs.spawn_blocking(move || {
        let head = message.request.method() == Method::HEAD;
        let result = service.handle_observed_transport(message.request);
        JobCompletion {
            result,
            reply: message.reply,
            permit: message.permit,
            head,
        }
    });
}
/// One owner for admission, completions, observer state and drain. Root calls are
/// never aborted: after shutdown only queued-not-started operations are refused.
/// Held permits bound queued/running/completed-unobserved results. Connections
/// with operator None remain explicitly unlimited, not silently clamped.
async fn run<'a, F: Future<Output = ()>>(
    listener: std::net::TcpListener,
    service: Arc<NativeHttpService>,
    policy: Arc<WebTransportPolicy>,
    shutdown: F,
    observer: &'a mut NativeOutcomeObserver<'a>,
) -> Result<NativeServerReport, NativeServerError> {
    let listener =
        tokio::net::TcpListener::from_std(listener).map_err(|cause| NativeServerError::Io {
            phase: "web-listener-register",
            cause,
        })?;
    let root = Arc::new(Semaphore::new(policy.root_worker_threads));
    let admin = Arc::new(Semaphore::new(policy.admin_worker_threads));
    let connections = policy
        .max_connections
        .map(|limit| Arc::new(Semaphore::new(limit)));
    let (sender, mut receiver) = mpsc::unbounded_channel::<Message>();
    let mut sockets = JoinSet::new();
    let mut jobs = JoinSet::new();
    let mut report = NativeServerReport::default();
    let mut owner = OutcomeOwner {
        observer,
        failure: None,
    };
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
         _=&mut shutdown=>break,
         accepted=listener.accept()=>{
          let (stream,_)=match accepted{Ok(value)=>value,Err(cause)=>{owner.fail(NativeServerError::Io{phase:"web-listener-accept",cause});break;}};
          if let Err(error)=count(&mut report.accepted_connections){owner.fail(error);break;}
          let permit=match connections.as_ref().map(|c|Arc::clone(c).try_acquire_owned()).transpose(){Ok(p)=>p,Err(_)=>{if let Err(error)=count(&mut report.rejected_connections){owner.fail(error);break;}drop(stream);continue;}};
          let service=Arc::clone(&service);let policy=Arc::clone(&policy);let root=Arc::clone(&root);let admin=Arc::clone(&admin);let sender=sender.clone();
          sockets.spawn(async move{
           let connection=Arc::new(permit);let mut builder=http1::Builder::new();let technical=WebTransportConstraints::current();
           builder.timer(TokioTimer::new()).max_headers(policy.max_headers).max_header_size(policy.header_max_bytes).max_buf_size(policy.header_max_bytes.max(usize::try_from(technical.minimum_connection_buffer_bytes.0).expect("Hyper minimum fits usize"))).header_read_timeout(policy.header_timeout);
           builder.serve_connection(TokioIo::new(stream),service_fn(move|r|dispatch(r,Arc::clone(&service),Arc::clone(&policy),Arc::clone(&root),Arc::clone(&admin),sender.clone(),Arc::clone(&connection)))).await
          });
         },
         Some(message)=receiver.recv()=>match message {
          Message::Job(message)=>spawn_job(&mut jobs,Arc::clone(&service),*message),
          Message::Rejected(error,_lease)=>if let Err(error)=record_rejection(error,&mut report){owner.fail(error);},
         },
         Some(result)=jobs.join_next()=>complete_job(result,&mut report,&mut owner),
         Some(result)=sockets.join_next()=>if let Err(error)=complete_connection(result,&mut report){owner.fail(error);},
        }
        if owner.failure.is_some() {
            break;
        }
    }
    receiver.close();
    drop(sender);
    sockets.abort_all();
    while let Some(result) = sockets.join_next().await {
        if let Err(error) = complete_connection(result, &mut report) {
            owner.fail(error);
        }
    }
    while let Some(message) = receiver.recv().await {
        match message {
            Message::Job(message) => reject_queued(*message, &mut report, &mut owner),
            Message::Rejected(error, _lease) => {
                if let Err(error) = record_rejection(error, &mut report) {
                    owner.fail(error);
                }
            }
        }
    }
    while let Some(result) = jobs.join_next().await {
        complete_job(result, &mut report, &mut owner);
    }
    let close = listener
        .into_std()
        .map_err(|cause| NativeServerError::Io {
            phase: "web-listener-unregister",
            cause,
        })
        .and_then(|socket| nix::unistd::close(socket).map_err(NativeServerError::SocketClose));
    if let Err(error) = close {
        owner.fail(error);
    }
    match owner.failure {
        Some(error) => Err(error),
        None => Ok(report),
    }
}
/// Encode a refusal with known effect metadata before moving outcome into the
/// observer. Serialization failure remains typed alongside the full outcome.
/// A500 does not roll back the business result; no peer receipt is established.
fn wire_failure(
    outcome: &NativeJobOutcome,
    code: &'static str,
    status: StatusCode,
    head: bool,
) -> Result<Response<Body>, serde_json::Error> {
    let mut value = serde_json::json!({"schema_version":1,"error":code,"failure":{"code":code}});
    if let Some((work, revision)) = &outcome.recorded_work {
        value["recorded_work"] = serde_json::json!({"id":work,"revision":revision});
    }
    if let Some(publication)=outcome.trust_publication{value["trust_publication"]=publication.into();value["trust_state_may_be_recorded"]=true.into();}
    if outcome.configuration_may_be_recorded {
        value["configuration_may_be_recorded"] = true.into();
    }
    if let Some(configuration) = &outcome.recorded_configuration {
        value["recorded_configuration"] = serde_json::to_value(configuration)?;
    }
    let raw = serde_json::to_vec(&value)?;
    let mut response =
        super::response(status, raw, head, None).map(|raw| Full::new(Bytes::from(raw)));
    response.headers_mut().insert(
        http::header::CONNECTION,
        http::HeaderValue::from_static("close"),
    );
    Ok(response)
}
/// Observe every completed result before reply. The permit remains held through
/// encoding/observation/send even if the client has disappeared. A failed observer
/// retains full outcome/cause and yields a failed response, never a fabricated200.
fn complete_job(
    result: Result<JobCompletion, tokio::task::JoinError>,
    report: &mut NativeServerReport,
    owner: &mut OutcomeOwner<'_>,
) {
    let completion = match result {
        Ok(value) => value,
        Err(error) => {
            owner.fail(NativeServerError::Job(error));
            return;
        }
    };
    let JobCompletion {
        result,
        reply,
        permit: _permit,
        head,
    } = completion;
    if let Err(error) = count(&mut report.completed_jobs) {
        owner.fail(error);
    }
    let (outcome, original) = match result {
        Ok(reply) => {
            let (outcome, response) = NativeJobOutcome::from_reply(reply);
            (
                outcome,
                Some(response.map(|raw| Full::new(Bytes::from(raw)))),
            )
        }
        Err(error) => (NativeJobOutcome::from_error(error), None),
    };
    let observation_failure = match wire_failure(
        &outcome,
        "web-outcome-observation-failed",
        StatusCode::INTERNAL_SERVER_ERROR,
        head,
    ) {
        Ok(response) => response,
        Err(cause) => {
            owner.fail(NativeServerError::OutcomeEncoding {
                cause,
                outcome: Box::new(outcome),
            });
            return;
        }
    };
    let original = match original {
        Some(response) => response,
        None => match wire_failure(
            &outcome,
            outcome
                .failure
                .as_ref()
                .map(HttpServiceError::code)
                .unwrap_or("web-service-failed"),
            outcome.status,
            head,
        ) {
            Ok(response) => response,
            Err(cause) => {
                owner.fail(NativeServerError::OutcomeEncoding {
                    cause,
                    outcome: Box::new(outcome),
                });
                return;
            }
        },
    };
    let response = match owner.observe(outcome) {
        Some(mut outcome) => {
            if let Err(error) = count(&mut report.observed_outcomes) {
                owner.fail(error);
            }
            if outcome.configuration_may_be_recorded {
                if let Err(error) = count(&mut report.configuration_outcomes) {
                    owner.fail(error);
                }
            }
            if outcome.recorded_work.is_some() {
                if let Err(error) = count(&mut report.work_outcomes) {
                    owner.fail(error);
                }
            }
            if let Some(work) = &outcome.recorded_work {
                report.last_recorded_work = Some(work.clone());
            }
            if let Some(configuration) = &outcome.recorded_configuration {
                report.last_recorded_configuration = Some(configuration.clone());
            }
            if report.first_service_failure.is_none() {
                report.first_service_failure = outcome.failure.take();
            }
            original
        }
        None => observation_failure,
    };
    match reply.send(response) {
        Ok(()) => (),
        Err(_undelivered) => {
            if let Err(error) = count(&mut report.disconnected_replies) {
                owner.fail(error);
            }
        }
    }
}
/// Refuse a queued request after admission stops, without running Root. Retain its
/// refusal in an already-failed observer, or observe it once if output is healthy.
/// Its lane permit is released only after this owner handoff attempt.
fn reject_queued(
    message: Submission,
    report: &mut NativeServerReport,
    owner: &mut OutcomeOwner<'_>,
) {
    if let Err(error) = count(&mut report.rejected_queued_jobs) {
        owner.fail(error);
    }
    let head = message.request.method() == Method::HEAD;
    let outcome = NativeJobOutcome {
        recorded_preparation:None,
        trust_publication:None,
        status: StatusCode::SERVICE_UNAVAILABLE,
        failure: Some(HttpServiceError::Refused("web-server-stopping")),
        observation_causes: Vec::new(),
        recorded_work: None,
        configuration_may_be_recorded: false,
        recorded_configuration: None,
    };
    let response = match wire_failure(
        &outcome,
        "web-server-stopping",
        StatusCode::SERVICE_UNAVAILABLE,
        head,
    ) {
        Ok(response) => response,
        Err(cause) => {
            owner.fail(NativeServerError::OutcomeEncoding {
                cause,
                outcome: Box::new(outcome),
            });
            return;
        }
    };
    if owner.observe(outcome).is_some() {
        if let Err(error) = count(&mut report.observed_outcomes) {
            owner.fail(error);
        }
    }
    match message.reply.send(response) {
        Ok(()) => (),
        Err(_undelivered) => {
            if let Err(error) = count(&mut report.disconnected_replies) {
                owner.fail(error);
            }
        }
    }
}
/// Retain first parser/IO cause and total count; malformed clients do not stop the
/// service. Only owned shutdown cancellation is expected; other job failures remain fatal.
fn complete_connection(
    result: Result<Result<(), hyper::Error>, tokio::task::JoinError>,
    report: &mut NativeServerReport,
) -> Result<(), NativeServerError> {
    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => {
            count(&mut report.connection_failures)?;
            if report.first_connection_failure.is_none() {
                report.first_connection_failure = Some(error);
            }
            Ok(())
        }
        Err(error) if error.is_cancelled() => Ok(()),
        Err(error) => Err(NativeServerError::Job(error)),
    }
}
/// Preserve the first typed rejection and count every received refusal. A closed
/// owner channel means shutdown already owns cleanup; no Root job was submitted.
/// The queued report retains its connection lease across a client-future drop.
fn reject(
    sender: &mpsc::UnboundedSender<Message>,
    status: StatusCode,
    code: &'static str,
    error: NativeRequestError,
    connection: ConnectionLease,
) -> Response<Body> {
    match sender.send(Message::Rejected(error, connection)) {
        Ok(()) => (),
        Err(_closed) => (),
    };
    refusal(status, code)
}
/// Bounded diagnostic retention: keep first cause and total count, never an
/// unbounded history of client failures or request bodies.
fn record_rejection(
    error: NativeRequestError,
    report: &mut NativeServerReport,
) -> Result<(), NativeServerError> {
    count(&mut report.rejected_requests)?;
    if report.first_request_failure.is_none() {
        report.first_request_failure = Some(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    /// A valid slot witness precedes pressure. Resource admission waits on that
    /// same slot; API admission still refuses. Drop cancels only its own waiter.
    #[tokio::test(flavor = "current_thread")]
    async fn browser_resources_wait_for_existing_lane_and_cancel_without_capacity_leak() {
        use std::{
            future::Future,
            task::{Context, Poll, Waker},
        };
        let lane = Arc::new(Semaphore::new(1));
        let occupied = acquire_lane(Arc::clone(&lane), true).await.unwrap();
        let denied = acquire_lane(Arc::clone(&lane), false)
            .await
            .expect_err("API should refuse saturation");
        assert_eq!(denied.code(), "web-workers-saturated");
        assert!(std::error::Error::source(&denied).is_some());
        let mut waiting = Box::pin(acquire_lane(Arc::clone(&lane), true));
        let mut context = Context::from_waker(Waker::noop());
        assert!(matches!(waiting.as_mut().poll(&mut context), Poll::Pending));
        assert_eq!(lane.available_permits(), 0);
        drop(occupied);
        let admitted = waiting.await.unwrap();
        assert_eq!(lane.available_permits(), 0);
        let mut cancelled = Box::pin(acquire_lane(Arc::clone(&lane), true));
        assert!(matches!(
            cancelled.as_mut().poll(&mut context),
            Poll::Pending
        ));
        drop(cancelled);
        assert_eq!(lane.available_permits(), 0);
        drop(admitted);
        assert_eq!(lane.available_permits(), 1);
        drop(acquire_lane(lane, false).await.unwrap());
    }
    /// A healthy admission precedes closed-lane injection; original Tokio causes
    /// remain inspectable for both waiting resource and immediate API admission.
    #[tokio::test(flavor = "current_thread")]
    async fn closed_worker_admission_retains_original_causes_without_saturation_fallback() {
        let lane = Arc::new(Semaphore::new(1));
        drop(acquire_lane(Arc::clone(&lane), false).await.unwrap());
        lane.close();
        for resource in [true, false] {
            let error = acquire_lane(Arc::clone(&lane), resource)
                .await
                .expect_err("Closed lane must refuse");
            assert_eq!(error.code(), "web-workers-stopping");
            assert!(std::error::Error::source(&error).is_some());
        }
    }
    use super::*;
    /// A valid occupied reservation precedes queued refusal/client drop. Capacity
    /// remains owned until the controller consumes the actual queued diagnostic.
    #[test]
    fn queued_diagnostic_retains_connection_reservation_until_owner_consumes_it() {
        let semaphore = Arc::new(Semaphore::new(1));
        let connection = Arc::new(Some(Arc::clone(&semaphore).try_acquire_owned().unwrap()));
        assert!(Arc::clone(&semaphore).try_acquire_owned().is_err());
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let response = reject(
            &sender,
            StatusCode::FORBIDDEN,
            "web-session-required",
            NativeRequestError::Refused("web-session-required"),
            Arc::clone(&connection),
        );
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        drop(connection);
        assert!(Arc::clone(&semaphore).try_acquire_owned().is_err());
        drop(receiver.try_recv().unwrap());
        assert!(Arc::clone(&semaphore).try_acquire_owned().is_ok());
    }
    /// Actual completion helper keeps its permit while observing, retains service
    /// failures, and counts a departed receiver without asserting peer receipt.
    #[test]
    fn departed_read_receiver_is_counted_and_typed_service_failure_retained() {
        let semaphore = Arc::new(Semaphore::new(1));
        let permit = Arc::clone(&semaphore).try_acquire_owned().unwrap();
        let (sender, receiver) = oneshot::channel();
        drop(receiver);
        let mut called = 0;
        let mut callback = |_: &NativeJobOutcome| {
            called += 1;
            assert_eq!(semaphore.available_permits(), 0);
            Ok(())
        };
        let mut owner = OutcomeOwner {
            observer: &mut callback,
            failure: None,
        };
        let mut report = NativeServerReport::default();
        let reply = HttpReply {
            response: Response::new(Vec::new()),
            failure: Some(HttpServiceError::Refused("web-route-unported")),
            recorded_work: None,
            configuration_may_be_recorded: false,
            recorded_configuration: None,
        };
        complete_job(
            Ok(JobCompletion {
                result: Ok(reply),
                reply: sender,
                permit,
                head: false,
            }),
            &mut report,
            &mut owner,
        );
        assert!(owner.failure.is_none());
        assert_eq!(report.completed_jobs, 1);
        assert_eq!(report.observed_outcomes, 1);
        assert_eq!(report.disconnected_replies, 1);
        assert_eq!(
            report.first_service_failure.unwrap().code(),
            "web-route-unported"
        );
        assert_eq!(semaphore.available_permits(), 1);
    }
    /// One failed owner output retains every already-admitted result, including Work
    /// and settings metadata, without repeating the callback or returning200.
    #[test]
    fn failed_observer_retains_all_completed_effects_without_retry_or_early_permit_release() {
        let semaphore = Arc::new(Semaphore::new(2));
        let permits = [
            Arc::clone(&semaphore).try_acquire_owned().unwrap(),
            Arc::clone(&semaphore).try_acquire_owned().unwrap(),
        ];
        let calls = std::cell::Cell::new(0);
        let mut callback = |_: &NativeJobOutcome| {
            calls.set(calls.get() + 1);
            assert_eq!(semaphore.available_permits(), 0);
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "owned observer failure",
            ))
        };
        let mut owner = OutcomeOwner {
            observer: &mut callback,
            failure: None,
        };
        let mut report = NativeServerReport::default();
        for (index, permit) in permits.into_iter().enumerate() {
            let (sender, mut receiver) = oneshot::channel();
            let reply = HttpReply {
                response: Response::new(Vec::new()),
                failure: None,
                recorded_work: (index == 1).then(|| ("owned-work".into(), 3)),
                configuration_may_be_recorded: index == 0,
                recorded_configuration: None,
            };
            complete_job(
                Ok(JobCompletion {
                    result: Ok(reply),
                    reply: sender,
                    permit,
                    head: false,
                }),
                &mut report,
                &mut owner,
            );
            assert_eq!(
                receiver.try_recv().unwrap().status(),
                StatusCode::INTERNAL_SERVER_ERROR
            );
        }
        assert_eq!(calls.get(), 1);
        let failure = owner.failure.unwrap();
        assert!(failure.configuration_may_be_recorded());
        assert_eq!(failure.recorded_work(), Some(("owned-work", 3)));
        assert_eq!(failure.observation_failure().unwrap().pending.len(), 2);
        assert_eq!(semaphore.available_permits(), 2);
    }
}

#[cfg(test)]
mod shutdown_tests {
    use super::*;
    /// Already-admitted framing can contain an undecoded business body. Stop must
    /// refuse it503 before Root parsing/execution, and acknowledge the refusal once
    /// while its lane is held. This models the actual queued-stop helper, not OS IO.
    #[test]
    fn queued_job_is_refused_and_observed_before_permit_release_without_root_execution() {
        let semaphore = Arc::new(Semaphore::new(1));
        let permit = Arc::clone(&semaphore).try_acquire_owned().unwrap();
        let (sender, mut receiver) = oneshot::channel();
        let mut records = Vec::new();
        let mut callback = |outcome: &NativeJobOutcome| {
            assert_eq!(semaphore.available_permits(), 0);
            assert!(!outcome.configuration_may_be_recorded);
            assert!(outcome.recorded_work.is_none());
            records.push(serde_json::to_value(outcome.record()).map_err(io::Error::other)?);
            Ok(())
        };
        let mut owner = OutcomeOwner {
            observer: &mut callback,
            failure: None,
        };
        let mut report = NativeServerReport::default();
        reject_queued(
            Submission {
                request: Request::builder()
                    .method("POST")
                    .uri("/api/v1/native-settings/apply")
                    .body(b"{}".to_vec())
                    .unwrap(),
                reply: sender,
                permit,
            },
            &mut report,
            &mut owner,
        );
        assert!(owner.failure.is_none());
        assert_eq!(
            receiver.try_recv().unwrap().status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(report.completed_jobs, 0);
        assert_eq!(report.rejected_queued_jobs, 1);
        assert_eq!(report.observed_outcomes, 1);
        assert_eq!(semaphore.available_permits(), 1);
    }
}
