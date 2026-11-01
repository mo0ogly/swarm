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
//! Blocking work (SQLite journal, Argon2) runs on the blocking pool.
use crate::{accounts::Accounts, assertion, config::AuthorityConfig, sessions::Sessions};
use ed25519_dalek::SigningKey;
use http::{header, HeaderValue, Method, Request, Response, StatusCode};
use http_body_util::{combinators::BoxBody, BodyExt, Full, Limited};
use hyper::body::{Bytes, Incoming};
use std::{net::SocketAddr, sync::Arc};
use swarm_enrollment_state::Context;

/// Cookie holding the daemon session token.
pub const COOKIE: &str = "swarm_authority_session";
type Body = BoxBody<Bytes, hyper::Error>;

/// Shared state of one running front.
pub struct Front {
    pub accounts: Accounts,
    pub sessions: Sessions,
    pub signer: SigningKey,
    pub config: AuthorityConfig,
    /// Exact `host:port` browsers use (Host and Origin are checked against it).
    pub own_host: String,
    /// Engine internal loopback address (v52 authority mode).
    pub engine: SocketAddr,
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn full(status: StatusCode, kind: &'static str, body: impl Into<Bytes>) -> Response<Body> {
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
fn login_page(english: bool, next: &str, message: &str) -> String {
    let (title, user, pass, button, lang) = if english {
        ("Sign in to Swarm", "Account", "Password", "Sign in", "en")
    } else {
        ("Connexion à Swarm", "Compte", "Mot de passe", "Se connecter", "fr")
    };
    let alert = if message.is_empty() { String::new() } else { format!("<p role=\"alert\">{}</p>", escape(message)) };
    format!(
        "<!doctype html><html lang=\"{lang}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{title}</title>\
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

fn same_origin(front: &Front, req: &Request<Incoming>) -> bool {
    let origin_ok = req.headers().get(header::ORIGIN).and_then(|v| v.to_str().ok()) == Some(&format!("http://{}", front.own_host));
    matches!(*req.method(), Method::GET | Method::HEAD) || origin_ok
}

fn host_ok(front: &Front, req: &Request<Incoming>) -> bool {
    req.headers().get(header::HOST).and_then(|v| v.to_str().ok()) == Some(front.own_host.as_str())
}

/// Read the body up to the configured limit (413 beyond it).
async fn read_body(front: &Front, body: Incoming) -> Result<Bytes, Response<Body>> {
    let limit = usize::try_from(front.config.relay_body_max_bytes).unwrap_or(usize::MAX);
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
            Ok(full(StatusCode::OK, "text/html; charset=utf-8", login_page(english, safe_next(next), "")))
        }
        (Method::POST, "/login") => {
            let body = match read_body(&front, req.into_body()).await { Ok(b) => b, Err(r) => return Ok(r) };
            let form: Vec<(String, String)> = form_urlencoded::parse(&body).into_owned().collect();
            let get = |k: &str| form.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone()).unwrap_or_default();
            let (subject, password, next) = (get("subject"), get("password"), get("next"));
            let f = Arc::clone(&front);
            let outcome = tokio::task::spawn_blocking(move || f.sessions.login(&Context::new(), &f.accounts, &subject, password.as_bytes(), now())).await;
            match outcome {
                Ok(Ok((token, _))) => {
                    let c = cookie::Cookie::build((COOKIE, token)).path("/").http_only(true).same_site(cookie::SameSite::Strict).build();
                    Ok(redirect(safe_next(&next), Some(c.to_string())))
                }
                _ => {
                    let msg = if english { "Sign-in refused. Check the account and password." } else { "Connexion refusée. Vérifiez le compte et le mot de passe." };
                    let mut r = full(StatusCode::UNAUTHORIZED, "text/html; charset=utf-8", login_page(english, safe_next(&next), msg));
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
                let _ = front.sessions.logout(&t);
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
        _ => relay(front, req).await,
    }
}

/// Resolve the session, sign an assertion for this exact request and relay
/// it to the engine; stream the engine response back unchanged.
async fn relay(front: Arc<Front>, req: Request<Incoming>) -> Result<Response<Body>, hyper::Error> {
    let path = req.uri().path_and_query().map(|p| p.as_str().to_owned()).unwrap_or_else(|| "/".into());
    let Some(token) = session_token(&req) else {
        return Ok(unauthenticated(&req, &path));
    };
    let f = Arc::clone(&front);
    let resolved = tokio::task::spawn_blocking(move || f.sessions.resolve(&Context::new(), &f.accounts, &token, now())).await;
    let Ok(Ok(view)) = resolved else {
        return Ok(unauthenticated(&req, &path));
    };
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
    match sender.send_request(out).await {
        Ok(resp) => Ok(resp.map(|b| b.boxed())),
        Err(_) => Ok(full(StatusCode::BAD_GATEWAY, "text/plain; charset=utf-8", "authority-engine-unavailable")),
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
