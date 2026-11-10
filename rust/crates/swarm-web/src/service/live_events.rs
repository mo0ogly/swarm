//! Server-sent change notifications for the live views (`/api/v1/live/events`).
//!
//! The stream is owned by its connection future: when the client disconnects or
//! the server aborts connections at shutdown, it is dropped; no task is detached.
//! While idle it holds no Root lane. Each probe is an ordinary Root job for
//! `/api/v1/live/fingerprint`, admitted with `try_acquire` on the same Root lane
//! as every other request (saturation → a `: busy` comment, never a queue), sent
//! to the owner, observed like any job and authorized with the client's own
//! Host/Cookie (an expired session ends the stream with a `refused` event).
//! Events carry only a fingerprint; clients then fetch the regular JSON routes.
use super::transport::{Body, Message, Submission};
use http::{
    header::{CACHE_CONTROL, CONTENT_TYPE, COOKIE, HOST, X_CONTENT_TYPE_OPTIONS},
    HeaderValue, Method, Request, Response, StatusCode,
};
use hyper::body::{Body as HttpBody, Bytes, Frame, SizeHint};
use std::{
    convert::Infallible,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};
use tokio::{
    sync::{mpsc, oneshot, Semaphore},
    time::Sleep,
};

pub(super) const PATH: &str = "/api/v1/live/events";
/// Delay between fingerprint probes. UI freshness cadence (half the 2 s polling
/// it replaces), not an engine execution policy: each probe is one bounded read
/// admitted on the operator-sized Root lane and refused, not queued, when busy.
const PROBE_INTERVAL: Duration = Duration::from_secs(1);
/// EventSource reconnection delay advertised to browsers (Go `/api/v1/events` parity).
const RETRY: &str = "retry: 2000\n\n";

/// Response body: either the ordinary buffered reply or one live event stream.
pub(super) enum WireBody {
    Full(Body),
    Events(Box<LiveEvents>),
}
impl HttpBody for WireBody {
    type Data = Bytes;
    type Error = Infallible;
    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, Infallible>>> {
        match self.get_mut() {
            Self::Full(body) => Pin::new(body).poll_frame(cx),
            Self::Events(events) => events.poll_next(cx).map(|f| f.map(Ok)),
        }
    }
    fn is_end_stream(&self) -> bool {
        match self {
            Self::Full(body) => body.is_end_stream(),
            Self::Events(events) => matches!(events.state, State::Done),
        }
    }
    fn size_hint(&self) -> SizeHint {
        match self {
            Self::Full(body) => body.size_hint(),
            Self::Events(_) => SizeHint::default(),
        }
    }
}

/// Only a plain GET without query or body opens a stream; every other shape
/// takes the ordinary job path, where the route returns its typed refusal.
pub(super) fn accepts<B: HttpBody>(request: &Request<B>) -> bool {
    request.uri().path() == PATH
        && request.method() == Method::GET
        && request.uri().query().is_none_or(str::is_empty)
        && request.body().is_end_stream()
}

/// Open the stream for an already authorized request. `Last-Event-ID` (a
/// 64-hex fingerprint) suppresses a redundant first `change` after reconnect.
pub(super) fn open<B>(
    request: &Request<B>,
    root: Arc<Semaphore>,
    sender: mpsc::UnboundedSender<Message>,
) -> Response<WireBody> {
    let last = request
        .headers()
        .get("last-event-id")
        .and_then(|v| v.to_str().ok())
        .filter(|v| v.len() == 64 && v.bytes().all(|b| b.is_ascii_hexdigit()))
        .map(str::to_owned);
    let mut credentials = Vec::new();
    for name in [HOST, COOKIE] {
        for value in request.headers().get_all(&name) {
            credentials.push((name.clone(), value.clone()));
        }
    }
    let events = LiveEvents {
        state: State::Start,
        last,
        credentials,
        root,
        sender,
    };
    let mut response = Response::new(WireBody::Events(Box::new(events)));
    for (name, value) in [
        (CONTENT_TYPE, "text/event-stream; charset=utf-8"),
        (CACHE_CONTROL, "no-store"),
        (X_CONTENT_TYPE_OPTIONS, "nosniff"),
    ] {
        response
            .headers_mut()
            .insert(name, HeaderValue::from_static(value));
    }
    *response.status_mut() = StatusCode::OK;
    response
}

enum State {
    Start,
    Sleeping(Pin<Box<Sleep>>),
    Probing(oneshot::Receiver<Response<Body>>),
    Done,
}

/// Probe state machine polled by the connection; see module docs.
pub(super) struct LiveEvents {
    state: State,
    last: Option<String>,
    credentials: Vec<(http::HeaderName, HeaderValue)>,
    root: Arc<Semaphore>,
    sender: mpsc::UnboundedSender<Message>,
}
impl LiveEvents {
    fn frame(text: String) -> Option<Frame<Bytes>> {
        Some(Frame::data(Bytes::from(text)))
    }
    /// End the stream after one final, explicit event.
    fn finish(&mut self, event: &str, data: serde_json::Value) -> Option<Frame<Bytes>> {
        self.state = State::Done;
        Self::frame(format!("event: {event}\ndata: {data}\n\n"))
    }
    fn sleep(&mut self) {
        self.state = State::Sleeping(Box::pin(tokio::time::sleep(PROBE_INTERVAL)));
    }
    /// Submit one fingerprint job; capacity is never waited for.
    fn probe(&mut self) -> Option<Frame<Bytes>> {
        let permit = match Arc::clone(&self.root).try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                self.sleep();
                return Self::frame(": busy\n\n".into());
            }
        };
        let mut builder = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/live/fingerprint");
        for (name, value) in &self.credentials {
            builder = builder.header(name, value);
        }
        let request = match builder.body(Vec::new()) {
            Ok(request) => request,
            Err(_) => {
                return self.finish("refused", serde_json::json!({"error":"live-probe-invalid"}))
            }
        };
        let (reply, receive) = oneshot::channel();
        let job = Message::Job(Box::new(Submission {
            request,
            reply,
            permit,
        }));
        if self.sender.send(job).is_err() {
            return self.finish("closed", serde_json::json!({"error":"web-server-stopping"}));
        }
        self.state = State::Probing(receive);
        None
    }
    /// Turn a completed probe into at most one event. Non-200 replies end the
    /// stream with their status and code, never a silent unchanged result.
    fn received(&mut self, response: Response<Body>, cx: &mut Context<'_>) -> Option<Frame<Bytes>> {
        let status = response.status();
        let mut body = response.into_body();
        let bytes = match Pin::new(&mut body).poll_frame(cx) {
            Poll::Ready(Some(Ok(frame))) => frame.into_data().unwrap_or_default(),
            _ => Bytes::new(),
        };
        let parsed: Option<serde_json::Value> = serde_json::from_slice(&bytes).ok();
        if status != StatusCode::OK {
            let error = parsed
                .as_ref()
                .and_then(|v| v.get("error")?.as_str().map(str::to_owned))
                .unwrap_or_else(|| String::from_utf8_lossy(&bytes).into_owned());
            return self.finish(
                "refused",
                serde_json::json!({"status": status.as_u16(), "error": error}),
            );
        }
        let Some(fingerprint) = parsed
            .as_ref()
            .and_then(|v| v.get("fingerprint")?.as_str().map(str::to_owned))
        else {
            return self.finish("refused", serde_json::json!({"error":"live-probe-invalid"}));
        };
        self.sleep();
        if self.last.as_deref() == Some(fingerprint.as_str()) {
            return None;
        }
        let data = serde_json::json!({ "fingerprint": fingerprint });
        let frame = Self::frame(format!("id: {fingerprint}\nevent: change\ndata: {data}\n\n"));
        self.last = Some(fingerprint);
        frame
    }
    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<Frame<Bytes>>> {
        loop {
            let frame = match &mut self.state {
                State::Done => return Poll::Ready(None),
                State::Start => {
                    // The retry hint precedes the first, immediate probe.
                    self.state = State::Sleeping(Box::pin(tokio::time::sleep(Duration::ZERO)));
                    return Poll::Ready(Self::frame(RETRY.into()));
                }
                State::Sleeping(sleep) => match sleep.as_mut().poll(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(()) => self.probe(),
                },
                State::Probing(receive) => match Pin::new(receive).poll(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(Ok(response)) => self.received(response, cx),
                    Poll::Ready(Err(_)) => {
                        self.finish("refused", serde_json::json!({"error":"web-root-job-failed"}))
                    }
                },
            };
            if frame.is_some() {
                return Poll::Ready(frame);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::Full;

    fn events(permits: usize) -> (LiveEvents, mpsc::UnboundedReceiver<Message>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        let events = LiveEvents {
            state: State::Start,
            last: None,
            credentials: vec![(HOST, HeaderValue::from_static("127.0.0.1:1"))],
            root: Arc::new(Semaphore::new(permits)),
            sender,
        };
        (events, receiver)
    }
    async fn next(e: &mut LiveEvents) -> Option<String> {
        std::future::poll_fn(|cx| e.poll_next(cx))
            .await
            .map(|f| String::from_utf8(f.into_data().unwrap().to_vec()).unwrap())
    }
    fn reply(status: StatusCode, body: &'static str) -> Response<Body> {
        let mut r = Response::new(Full::new(Bytes::from_static(body.as_bytes())));
        *r.status_mut() = status;
        r
    }
    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
    }

    #[test]
    fn witness_change_then_saturated_lane_is_a_busy_comment_not_a_queue() {
        runtime().block_on(async {
            let (mut e, mut jobs) = events(1);
            assert_eq!(next(&mut e).await.unwrap(), RETRY);
            let probe = tokio::spawn(async move {
                let Some(Message::Job(job)) = jobs.recv().await else { panic!() };
                assert_eq!(job.request.uri().path(), "/api/v1/live/fingerprint");
                assert_eq!(job.request.headers()[HOST], "127.0.0.1:1");
                job.reply.send(reply(StatusCode::OK, r#"{"fingerprint":"ab"}"#)).ok();
                jobs
            });
            let change = next(&mut e).await.unwrap();
            assert!(change.contains("id: ab\nevent: change\n"), "{change}");
            let _jobs = probe.await.unwrap();
            let held = Arc::clone(&e.root).try_acquire_owned().unwrap();
            assert_eq!(next(&mut e).await.unwrap(), ": busy\n\n");
            drop(held);
        });
    }

    #[test]
    fn refused_probe_and_stopped_owner_end_the_stream_explicitly() {
        runtime().block_on(async {
            let (mut e, mut jobs) = events(1);
            next(&mut e).await;
            let owner = tokio::spawn(async move {
                let Some(Message::Job(job)) = jobs.recv().await else { panic!() };
                job.reply.send(reply(StatusCode::FORBIDDEN, "web-access-refused")).ok();
            });
            let refused = next(&mut e).await.unwrap();
            assert!(refused.starts_with("event: refused\n"), "{refused}");
            assert!(refused.contains("403") && refused.contains("web-access-refused"));
            owner.await.unwrap();
            assert!(next(&mut e).await.is_none());
            let (mut e, jobs) = events(1);
            drop(jobs);
            next(&mut e).await;
            let closed = next(&mut e).await.unwrap();
            assert!(closed.starts_with("event: closed\n"), "{closed}");
            assert!(next(&mut e).await.is_none());
        });
    }
}
