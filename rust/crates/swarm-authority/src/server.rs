//! HTTP front of the authority daemon (phase B, slice B5.3).
//!
//! The daemon owns the cockpit address. It serves its own `/login` (account +
//! password) and `/logout`, keeps sessions ([`crate::sessions`]), and relays
//! every other request to the engine's internal loopback address with a fresh
//! signed assertion ([`crate::assertion`]). The engine (v52 `authority` mode)
//! refuses anything without a valid assertion, so reaching the internal port
//! without the daemon's key gives nothing.
//!
//! Browser protections: session cookie `HttpOnly; SameSite=Strict; Path=/`;
//! every non-GET/HEAD request must carry `Origin: http://<own host>`; the
//! client's own `Cookie`, `Host` and assertion headers are never forwarded.
//! Bodies above the configured `relay_body_max_bytes` refuse with 413.
//! Before relaying, the access decision ([`crate::policy::decide`]) runs on
//! the session's current account; a refusal answers 403
//! `authority-policy-denied` with the action family and reason code.
//! B8/B9: `/api/v1/authority/*` is served by the daemon itself
//! ([`crate::admin`]); `Authorization: Bearer sak1-…` API keys
//! ([`crate::keys`]) authenticate without cookie or Origin, capped by their
//! scope and refused on that administration API; every non-read request is
//! audited ([`crate::audit`]) with an intent line before execution (refusal
//! 503 if it cannot be written) and an outcome line after. A session or key
//! resolution failure other than a plain refusal answers 503 with its code,
//! never a disguised sign-out. The sign-in page uses `Referrer-Policy:
//! same-origin` so the browser sends a real `Origin` with the form.
//! Blocking work (SQLite journal, Argon2) runs on the blocking pool.
use crate::{accounts::{AccountView, Accounts}, admin, assertion, audit, config::AuthorityConfig, policy, sessions::Sessions};
use ed25519_dalek::SigningKey;
use http::{header, HeaderValue, Method, Request, Response, StatusCode};
use http_body_util::{combinators::BoxBody, BodyExt, Full, Limited};
use hyper::body::{Bytes, Incoming};
use std::{net::SocketAddr, path::PathBuf, sync::{Arc, RwLock}};
use swarm_enrollment_state::Context;

/// Cookie holding the daemon session token.
pub const COOKIE: &str = "swarm_authority_session";
pub(crate) type Body = BoxBody<Bytes, hyper::Error>;

/// Shared state of one running front.
pub struct Front {
    pub accounts: Accounts,
    pub sessions: Sessions,
    pub signer: SigningKey,
    /// Live configuration (Admin-editable through [`crate::admin`]).
    pub config: RwLock<AuthorityConfig>,
    /// Daemon configuration directory holding `policy.json`/`authority.json`.
    pub dir: PathBuf,
    /// Exact `host:port` browsers use (Host and Origin are checked against it).
    pub own_host: String,
    /// Engine internal loopback address (v52 authority mode).
    pub engine: SocketAddr,
    /// Access policy evaluated before every relay (B6); Admin-editable (B8).
    pub policy: RwLock<policy::Policy>,
    /// Access audit (B8 attribution): sign-in/out and every non-read request.
    pub audit: audit::Audit,
}

pub(crate) fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub(crate) fn full(status: StatusCode, kind: &'static str, body: impl Into<Bytes>) -> Response<Body> {
    let mut r = Response::new(Full::new(body.into()).map_err(|never| match never {}).boxed());
    *r.status_mut() = status;
    let h = r.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static(kind));
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    h.insert(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    h.insert("x-content-type-options", HeaderValue::from_static("nosniff"));
    r
}

fn redirect(to: &str, cookie: Option<String>) -> Response<Body> {
    let mut r = full(StatusCode::SEE_OTHER, "text/plain; charset=utf-8", "");
    if let Ok(v) = HeaderValue::from_str(to) {
        r.headers_mut().insert(header::LOCATION, v);
    }
    if let Some(c) = cookie.and_then(|c| HeaderValue::from_str(&c).ok()) {
        r.headers_mut().insert(header::SET_COOKIE, c);
    }
    r
}

/// Same-origin relative target only; anything else becomes `/`.
fn safe_next(next: &str) -> &str {
    if next.starts_with('/') && !next.starts_with("//") && !next.contains('\\') && !next.chars().any(char::is_control) {
        next
    } else {
        "/"
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;")
}

/// Minimal self-contained sign-in page (FR/EN, both themes via
/// `prefers-color-scheme`); no external resource, no script.
/// Sign-in form response. `Referrer-Policy: same-origin`, not the
/// `no-referrer` of other responses: under `no-referrer` browsers send
/// `Origin: null` with a form POST, which the Origin check refuses (found by
/// the browser check `tests/authority_admin_ui.cjs`). `same-origin` still
/// sends no referrer to other sites.
fn login_response(status: StatusCode, html: String) -> Response<Body> {
    let mut r = full(status, "text/html; charset=utf-8", html);
    r.headers_mut().insert(header::REFERRER_POLICY, HeaderValue::from_static("same-origin"));
    r
}

fn login_page(english: bool, next: &str, message: &str) -> String {
    let (title, user, pass, button, lang) = if english {
        ("Sign in to Swarm", "Account", "Password", "Sign in", "en")
    } else {
        ("Connexion à Swarm", "Compte", "Mot de passe", "Se connecter", "fr")
    };
    let alert = if message.is_empty() { String::new() } else { format!("<p role=\"alert\">{}</p>", escape(message)) };
    format!(
        "<!doctype html><html lang=\"{lang}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><link rel=\"icon\" href=\"data:,\"><title>{title}</title>\
<style>:root{{color-scheme:light dark;--bg:#f7f5ef;--fg:#16233a;--line:#8a93a3;--accent:#0b2b4c}}@media(prefers-color-scheme:dark){{:root{{--bg:#0b1a2a;--fg:#e8edf4;--line:#4b5b70;--accent:#9cc7e8}}}}\
body{{margin:0;min-height:100vh;display:grid;place-items:center;background:var(--bg);color:var(--fg);font:16px system-ui,sans-serif}}main{{width:min(360px,calc(100% - 32px))}}\
label{{display:block;margin-top:14px}}input{{width:100%;box-sizing:border-box;padding:10px;margin-top:4px;border:1px solid var(--line);border-radius:6px;background:transparent;color:inherit}}\
button{{margin-top:18px;width:100%;padding:11px;border:0;border-radius:6px;background:var(--accent);color:var(--bg);font-weight:600}}:focus-visible{{outline:3px solid #e0a800;outline-offset:2px}}[role=alert]{{color:#c0392b}}</style></head>\
<body><main><h1>{title}</h1>{alert}<form method=\"post\" action=\"/login\"><input type=\"hidden\" name=\"next\" value=\"{next}\">\
<label>{user}<input name=\"subject\" autocomplete=\"username\" required autofocus></label><label>{pass}<input name=\"password\" type=\"password\" autocomplete=\"current-password\" required></label>\
<button type=\"submit\">{button}</button></form></main></body></html>",
        next = escape(next)
    )
}

fn session_token(req: &Request<Incoming>) -> Option<String> {
    for v in req.headers().get_all(header::COOKIE) {
        for c in cookie::Cookie::split_parse(v.to_str().ok()?).flatten() {
            if c.name() == COOKIE {
                return Some(c.value().to_owned());
            }
        }
    }
    None
}

/// Personal API key presented as `Authorization: Bearer sak1-…` (B9).
fn bearer_key(req: &Request<Incoming>) -> Option<String> {
    let v = req.headers().get(header::AUTHORIZATION)?.to_str().ok()?;
    v.strip_prefix("Bearer ").filter(|t| t.starts_with("sak1-")).map(str::to_owned)
}

/// Non-GET/HEAD requests need `Origin: http://<own host>` — except a request
/// authenticated only by an API key and carrying no cookie: a bearer header
/// is not an ambient browser credential (cross-site pages cannot set it
/// without a CORS preflight, which the daemon never grants).
fn same_origin(front: &Front, req: &Request<Incoming>) -> bool {
    let origin_ok = req.headers().get(header::ORIGIN).and_then(|v| v.to_str().ok()) == Some(&format!("http://{}", front.own_host));
    let key_only = bearer_key(req).is_some() && req.headers().get(header::COOKIE).is_none();
    matches!(*req.method(), Method::GET | Method::HEAD) || origin_ok || key_only
}

/// How a request was authenticated.
pub(crate) enum Credential {
    /// Browser session (token kept to mark/replace the caller's own session).
    Session(String),
    /// Personal API key: public id and scope cap.
    Key { id: String, scope: policy::Action },
}
impl Credential {
    fn label(&self) -> String {
        match self {
            Self::Session(_) => "session".into(),
            Self::Key { id, .. } => format!("key:{id}"),
        }
    }
}

fn host_ok(front: &Front, req: &Request<Incoming>) -> bool {
    req.headers().get(header::HOST).and_then(|v| v.to_str().ok()) == Some(front.own_host.as_str())
}

/// Read the body up to the configured limit (413 beyond it).
pub(crate) async fn read_body(front: &Front, body: Incoming) -> Result<Bytes, Response<Body>> {
    let max = front.config.read().map(|c| c.relay_body_max_bytes).unwrap_or(1);
    let limit = usize::try_from(max).unwrap_or(usize::MAX);
    match Limited::new(body, limit).collect().await {
        Ok(c) => Ok(c.to_bytes()),
        Err(e) if e.is::<http_body_util::LengthLimitError>() => Err(full(StatusCode::PAYLOAD_TOO_LARGE, "text/plain; charset=utf-8", "authority-body-too-large")),
        Err(_) => Err(full(StatusCode::BAD_REQUEST, "text/plain; charset=utf-8", "authority-body-refused")),
    }
}

/// Route one browser request. Never panics on client input.
pub async fn handle(front: Arc<Front>, req: Request<Incoming>) -> Result<Response<Body>, hyper::Error> {
    if !host_ok(&front, &req) {
        return Ok(full(StatusCode::FORBIDDEN, "text/plain; charset=utf-8", "authority-host-refused"));
    }
    if !same_origin(&front, &req) {
        return Ok(full(StatusCode::FORBIDDEN, "text/plain; charset=utf-8", "authority-origin-refused"));
    }
    let query: Vec<(String, String)> = form_urlencoded::parse(req.uri().query().unwrap_or("").as_bytes()).into_owned().collect();
    let english = query.iter().any(|(k, v)| k == "lang" && v == "en");
    match (req.method().clone(), req.uri().path()) {
        (Method::GET, "/login") => {
            let next = query.iter().find(|(k, _)| k == "next").map(|(_, v)| v.as_str()).unwrap_or("/");
            Ok(login_response(StatusCode::OK, login_page(english, safe_next(next), "")))
        }
        (Method::POST, "/login") => {
            let body = match read_body(&front, req.into_body()).await { Ok(b) => b, Err(r) => return Ok(r) };
            let form: Vec<(String, String)> = form_urlencoded::parse(&body).into_owned().collect();
            let get = |k: &str| form.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone()).unwrap_or_default();
            let (subject, password, next) = (get("subject"), get("password"), get("next"));
            let (f, attempted) = (Arc::clone(&front), subject.clone());
            let outcome = tokio::task::spawn_blocking(move || f.sessions.login(&Context::new(), &f.accounts, &attempted, password.as_bytes(), now())).await;
            let named = swarm_identity_contract::name(subject.as_bytes()).then_some(subject.as_str());
            match outcome {
                Ok(Ok((token, _))) => {
                    // A refused audit line refuses the sign-in (fail closed).
                    if front.audit.record(&audit::Event { at: now(), event: "sign-in", subject: named, ..Default::default() }).is_err() {
                        let _ = front.sessions.logout(&token);
                        return Ok(full(StatusCode::SERVICE_UNAVAILABLE, "text/plain; charset=utf-8", "authority-audit-unavailable"));
                    }
                    let c = cookie::Cookie::build((COOKIE, token)).path("/").http_only(true).same_site(cookie::SameSite::Strict).build();
                    Ok(redirect(safe_next(&next), Some(c.to_string())))
                }
                _ => {
                    if front.audit.record(&audit::Event { at: now(), event: "sign-in-refused", subject: named, ..Default::default() }).is_err() {
                        eprintln!("{}", serde_json::json!({"schema_version": 1, "warning": "authority-audit-unavailable"}));
                    }
                    let msg = if english { "Sign-in refused. Check the account and password." } else { "Connexion refusée. Vérifiez le compte et le mot de passe." };
                    let mut r = login_response(StatusCode::UNAUTHORIZED, login_page(english, safe_next(&next), msg));
                    r.headers_mut().insert("x-authority-failure", HeaderValue::from_static("authority-login-refused"));
                    Ok(r)
                }
            }
        }
        (Method::POST, "/logout") | (Method::POST, "/api/v1/logout") => {
            // Both the form path and the cockpit buttons' API path end the
            // daemon session (the engine's own cookie logout is irrelevant here).
            let api = req.uri().path() == "/api/v1/logout";
            if let Some(t) = session_token(&req) {
                if let Ok(Some(subject)) = front.sessions.logout(&t) {
                    if front.audit.record(&audit::Event { at: now(), event: "sign-out", subject: Some(&subject), ..Default::default() }).is_err() {
                        eprintln!("{}", serde_json::json!({"schema_version": 1, "warning": "authority-audit-unavailable"}));
                    }
                }
            }
            let mut c = cookie::Cookie::build((COOKIE, "")).path("/").http_only(true).same_site(cookie::SameSite::Strict).build();
            c.make_removal();
            if api {
                let mut r = full(StatusCode::OK, "application/json; charset=utf-8", r#"{"redirect":"/login"}"#);
                if let Ok(v) = HeaderValue::from_str(&c.to_string()) {
                    r.headers_mut().insert(header::SET_COOKIE, v);
                }
                Ok(r)
            } else {
                Ok(redirect("/login", Some(c.to_string())))
            }
        }
        _ if policy::under(req.uri().path(), policy::AUTHORITY_PREFIX) => administration(front, req).await,
        _ => relay(front, req).await,
    }
}

/// Resolve the request's session (token + current account view) on the
/// blocking pool; `Err` carries the unauthenticated response.
/// A presented API key wins over a cookie only when no cookie is sent; a
/// refused key answers 401 `authority-key-refused` (never a redirect).
async fn authenticate(front: &Arc<Front>, req: &Request<Incoming>, path: &str) -> Result<(Credential, AccountView), Response<Body>> {
    if let (Some(key), None) = (bearer_key(req), req.headers().get(header::COOKIE)) {
        let f = Arc::clone(front);
        return match tokio::task::spawn_blocking(move || f.accounts.resolve_key(&Context::new(), &key, now())).await {
            Ok(Ok((view, scope, id))) => Ok((Credential::Key { id, scope }, view)),
            Ok(Err(e)) if e.code() == "key-refused" => Err(full(StatusCode::UNAUTHORIZED, "application/json", r#"{"error":"authority-key-refused"}"#)),
            Ok(Err(e)) => Err(full(StatusCode::SERVICE_UNAVAILABLE, "application/json", serde_json::json!({"error": e.code()}).to_string())),
            Err(_) => Err(full(StatusCode::SERVICE_UNAVAILABLE, "application/json", r#"{"error":"authority-state"}"#)),
        };
    }
    let Some(token) = session_token(req) else { return Err(unauthenticated(req, path)) };
    let (f, t) = (Arc::clone(front), token.clone());
    match tokio::task::spawn_blocking(move || f.sessions.resolve(&Context::new(), &f.accounts, &t, now())).await {
        Ok(Ok(view)) => Ok((Credential::Session(token), view)),
        // Only a refused session means "sign in again"; a journal or state
        // failure is the daemon's, kept visible (never turned into a logout).
        Ok(Err(e)) if e.code() == "session-refused" => Err(unauthenticated(req, path)),
        Ok(Err(e)) => Err(full(StatusCode::SERVICE_UNAVAILABLE, "application/json", serde_json::json!({"error": e.code()}).to_string())),
        Err(_) => Err(full(StatusCode::SERVICE_UNAVAILABLE, "application/json", r#"{"error":"authority-state"}"#)),
    }
}

/// Access decision of the live policy for `view`; `Err` is the 403 refusal.
#[allow(clippy::result_large_err)] // the Err is the ready HTTP refusal
fn authorize(front: &Front, view: &AccountView, method: &str, path: &str) -> Result<policy::Action, Response<Body>> {
    let Ok(p) = front.policy.read() else {
        return Err(full(StatusCode::INTERNAL_SERVER_ERROR, "text/plain; charset=utf-8", "authority-state"));
    };
    let decision = policy::decide(&p, &policy::Subject { roles: &view.roles, integrity: view.integrity, attributes: &view.attributes }, method, path);
    if decision.allowed {
        return Ok(decision.action);
    }
    let body = serde_json::json!({"error": "authority-policy-denied", "action": decision.action, "reason": decision.reason});
    Err(full(StatusCode::FORBIDDEN, "application/json", body.to_string()))
}

/// B8: daemon-served administration. `…/me` is self-service for any
/// signed-in account; everything else needs the `administer` decision.
async fn administration(front: Arc<Front>, req: Request<Incoming>) -> Result<Response<Body>, hyper::Error> {
    let path = req.uri().path().to_owned();
    let (credential, view) = match authenticate(&front, &req, &path).await { Ok(v) => v, Err(r) => return Ok(r) };
    let Credential::Session(token) = credential else {
        // B9: keys never manage accounts, sessions, keys, policy or settings.
        return Ok(full(StatusCode::FORBIDDEN, "application/json", r#"{"error":"authority-key-scope"}"#));
    };
    let action = if policy::under(&path, admin::ME) {
        "self"
    } else {
        match authorize(&front, &view, req.method().as_str(), &path) {
            Ok(_) => "administer",
            Err(r) => return Ok(r),
        }
    };
    if req.method() == Method::GET || req.method() == Method::HEAD {
        return Ok(admin::handle(front, req, view, token).await);
    }
    let method = req.method().as_str().to_owned();
    let subject = view.subject.clone();
    if let Err(r) = intent(&front, &subject, "session", &method, &path, action) {
        return Ok(r);
    }
    let response = admin::handle(Arc::clone(&front), req, view, token).await;
    outcome(&front, &subject, "session", &method, &path, action, response.status());
    Ok(response)
}

/// Resolve the session, sign an assertion for this exact request and relay
/// it to the engine; stream the engine response back unchanged.
async fn relay(front: Arc<Front>, req: Request<Incoming>) -> Result<Response<Body>, hyper::Error> {
    let path = req.uri().path_and_query().map(|p| p.as_str().to_owned()).unwrap_or_else(|| "/".into());
    let (credential, view) = match authenticate(&front, &req, &path).await { Ok(v) => v, Err(r) => return Ok(r) };
    // B6: decide before reading the body or touching the engine; a refusal
    // is final for this request and carries only stable codes.
    let action = match authorize(&front, &view, req.method().as_str(), req.uri().path()) {
        Ok(a) => a,
        Err(r) => return Ok(r),
    };
    if let Credential::Key { scope, .. } = &credential {
        if action > *scope {
            let body = serde_json::json!({"error": "authority-policy-denied", "action": action, "reason": "key-scope"});
            return Ok(full(StatusCode::FORBIDDEN, "application/json", body.to_string()));
        }
    }
    let via = credential.label();
    let audited = (action != policy::Action::Read).then(|| (view.subject.clone(), req.method().as_str().to_owned(), req.uri().path().to_owned(), action_name(action)));
    if let Some((subject, method, p, a)) = &audited {
        if let Err(r) = intent(&front, subject, &via, method, p, a) {
            return Ok(r);
        }
    }
    let (parts, body) = req.into_parts();
    let body = match read_body(&front, body).await { Ok(b) => b, Err(r) => return Ok(r) };
    let mut nonce = [0u8; 16];
    if getrandom::fill(&mut nonce).is_err() {
        return Ok(full(StatusCode::SERVICE_UNAVAILABLE, "text/plain; charset=utf-8", "authority-entropy"));
    }
    let claims = assertion::Claims {
        v: 1,
        subject: view.subject,
        roles: view.roles,
        integrity: view.integrity,
        method: parts.method.as_str().to_owned(),
        path: path.clone(),
        body_sha256: assertion::body_digest(&body),
        issued_at: now(),
        nonce: nonce.iter().map(|b| format!("{b:02x}")).collect(),
    };
    let Ok(signed) = assertion::sign(&front.signer, &claims) else {
        return Ok(full(StatusCode::INTERNAL_SERVER_ERROR, "text/plain; charset=utf-8", "authority-sign"));
    };
    let mut out = Request::builder().method(parts.method).uri(path);
    for (name, value) in parts.headers.iter() {
        if ![header::COOKIE, header::HOST, header::ORIGIN].contains(name) && name.as_str() != assertion::HEADER && name.as_str() != "x-swarm-csrf" {
            out = out.header(name, value);
        }
    }
    let out = match out
        .header(header::HOST, front.engine.to_string())
        .header(assertion::HEADER, signed)
        .body(Full::new(body))
    {
        Ok(r) => r,
        Err(_) => return Ok(full(StatusCode::BAD_REQUEST, "text/plain; charset=utf-8", "authority-relay-request")),
    };
    let stream = match tokio::net::TcpStream::connect(front.engine).await {
        Ok(s) => s,
        Err(_) => return Ok(full(StatusCode::BAD_GATEWAY, "text/plain; charset=utf-8", "authority-engine-unavailable")),
    };
    let (mut sender, conn) = match hyper::client::conn::http1::handshake(hyper_util::rt::TokioIo::new(stream)).await {
        Ok(v) => v,
        Err(_) => return Ok(full(StatusCode::BAD_GATEWAY, "text/plain; charset=utf-8", "authority-engine-handshake")),
    };
    tokio::spawn(conn);
    let response = match sender.send_request(out).await {
        Ok(resp) => resp.map(|b| b.boxed()),
        Err(_) => full(StatusCode::BAD_GATEWAY, "text/plain; charset=utf-8", "authority-engine-unavailable"),
    };
    if let Some((subject, method, p, a)) = &audited {
        outcome(&front, subject, &via, method, p, a, response.status());
    }
    Ok(response)
}

fn action_name(a: policy::Action) -> &'static str {
    match a {
        policy::Action::Read => "read",
        policy::Action::Write => "write",
        policy::Action::Approve => "approve",
        policy::Action::Administer => "administer",
    }
}

/// Record the intent of a mutating request before executing it; a failed
/// write refuses the request with 503 `authority-audit-unavailable` (no
/// unattributed effect).
#[allow(clippy::result_large_err)] // the Err is the ready HTTP refusal
fn intent(front: &Front, subject: &str, via: &str, method: &str, path: &str, action: &str) -> Result<(), Response<Body>> {
    front
        .audit
        .record(&audit::Event { at: now(), event: "intent", subject: Some(subject), credential: Some(via), method: Some(method), path: Some(path), action: Some(action), status: None })
        .map(|_| ())
        .map_err(|_| full(StatusCode::SERVICE_UNAVAILABLE, "text/plain; charset=utf-8", "authority-audit-unavailable"))
}

/// Record the outcome status after execution (best effort: the effect has
/// happened; a failure is reported on stderr, the response is unchanged).
#[allow(clippy::too_many_arguments)] // one audit line's fields
fn outcome(front: &Front, subject: &str, via: &str, method: &str, path: &str, action: &str, status: StatusCode) {
    let ev = audit::Event { at: now(), event: "outcome", subject: Some(subject), credential: Some(via), method: Some(method), path: Some(path), action: Some(action), status: Some(status.as_u16()) };
    if front.audit.record(&ev).is_err() {
        eprintln!("{}", serde_json::json!({"schema_version": 1, "warning": "authority-audit-unavailable", "subject": subject, "path": path}));
    }
}

/// No valid session: browsers navigating with GET are sent to `/login`,
/// API calls get 401 (no redirect loop for fetch).
fn unauthenticated(req: &Request<Incoming>, path: &str) -> Response<Body> {
    let wants_html = req.headers().get(header::ACCEPT).and_then(|v| v.to_str().ok()).is_some_and(|a| a.contains("text/html"));
    if req.method() == Method::GET && wants_html {
        let next: String = form_urlencoded::byte_serialize(path.as_bytes()).collect();
        redirect(&format!("/login?next={next}"), None)
    } else {
        full(StatusCode::UNAUTHORIZED, "application/json", r#"{"error":"authority-session-required"}"#)
    }
}

/// Serve `front` on an already bound listener until the task is cancelled.
pub async fn serve(listener: tokio::net::TcpListener, front: Arc<Front>) -> std::io::Result<()> {
    loop {
        let (stream, _) = listener.accept().await?;
        let f = Arc::clone(&front);
        tokio::spawn(async move {
            let service = hyper::service::service_fn(move |r| handle(Arc::clone(&f), r));
            let _ = hyper::server::conn::http1::Builder::new()
                .serve_connection(hyper_util::rt::TokioIo::new(stream), service)
                .await;
        });
    }
}
