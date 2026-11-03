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
    /// Bounded diagnostics file (ADM-09), level set in the Admin.
    pub diag: crate::diag::Diag,
    /// Timestamps of recent manual purges (rate limit, ADM-08).
    pub purge_slots: std::sync::Mutex<Vec<i64>>,
    /// Pending forgotten-password requests shown to administrators.
    pub reset_requests: crate::sessions::ResetRequests,
}

/// Run one automatic audit purge according to `audit_retention_days`
/// (ADM-08): a best-effort daemon task, never a request's concern.
/// Returns the number of lines removed (`Ok(0)` when no retention is set,
/// the chain is broken or the write fails — the manual purge reports those).
pub fn auto_purge(front: &Front) -> usize {
    let Some(days) = front.config.read().ok().and_then(|c| c.audit_retention_days) else { return 0 };
    let before = now().saturating_sub(days.saturating_mul(86_400));
    match front.audit.purge(before, "automatic", now()) {
        Ok(n) => n,
        Err(code) => {
            front.diag.log("warn", "audit auto purge skipped", serde_json::json!({"code": code}));
            0
        }
    }
}

/// Spawn the daemon's background loop: automatic audit purge (every 6 hours
/// and once at start-up) under the configured retention.
pub fn spawn_maintenance(front: Arc<Front>) {
    tokio::spawn(async move {
        loop {
            auto_purge(&front);
            if tokio::time::timeout(std::time::Duration::from_secs(6 * 3600), tokio::signal::ctrl_c()).await.is_ok() {
                break;
            }
        }
    });
}

/// Rate limit of the manual purge endpoint: 5 calls per rolling minute
/// (the response carries `Retry-After`; the window is best effort).
pub const PURGE_RATE: usize = 5;

impl Front {
    /// Consume one purge slot; `false` when the last minute already saw
    /// [`PURGE_RATE`] calls (the caller refuses with 429).
    pub fn consume_purge_slot(&self) -> bool {
        let mut q = self.purge_slots.lock().unwrap_or_else(|e| e.into_inner());
        let cut = now().saturating_sub(60);
        q.retain(|t| *t >= cut);
        if q.len() >= PURGE_RATE {
            return false;
        }
        q.push(now());
        true
    }
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

/// Logo and tab icon served by the daemon itself, without a session (the
/// engine's copies sit behind the relay and need one).
const LOGO: &[u8] = include_bytes!("../../../../web/swarm-logo.png");
const ICON: &[u8] = include_bytes!("../../../../web/favicon.svg");

/// Shared shell of the daemon's public pages (sign-in, forgotten password):
/// logo, title, FR/EN switch keeping `next`; colors follow the system theme.
fn page(english: bool, title: &str, body: &str, here: &str, next: &str) -> String {
    let lang = if english { "en" } else { "fr" };
    let next_q: String = form_urlencoded::byte_serialize(next.as_bytes()).collect();
    format!(
        "<!doctype html><html lang=\"{lang}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><link rel=\"icon\" href=\"/login/icon.svg\" type=\"image/svg+xml\"><title>{title}</title>\
<style>:root{{color-scheme:light dark;--bg:#f7f5ef;--fg:#16233a;--muted:#4b5566;--line:#8a93a3;--accent:#0b2b4c}}@media(prefers-color-scheme:dark){{:root{{--bg:#0b1a2a;--fg:#e8edf4;--muted:#b7c2d0;--line:#4b5b70;--accent:#9cc7e8}}}}\
body{{margin:0;min-height:100vh;display:grid;place-items:center;background:var(--bg);color:var(--fg);font:16px system-ui,sans-serif}}main{{width:min(380px,calc(100% - 32px));padding:24px 0}}\
header{{text-align:center}}header img{{width:96px;height:96px}}h1{{font-size:1.5rem;margin:.5rem 0 1rem}}p{{color:var(--muted);line-height:1.45}}\
label{{display:block;margin-top:14px}}input{{width:100%;box-sizing:border-box;min-height:44px;padding:10px;margin-top:4px;border:1px solid var(--line);border-radius:6px;background:transparent;color:inherit;font-size:16px}}\
button{{margin-top:18px;width:100%;min-height:44px;padding:11px;border:0;border-radius:6px;background:var(--accent);color:var(--bg);font-weight:600;font-size:16px;cursor:pointer}}\
a{{color:var(--accent)}}.links{{display:flex;justify-content:space-between;gap:12px;flex-wrap:wrap;margin-top:18px}}.links a{{display:inline-block;min-height:44px;line-height:44px}}\
:focus-visible{{outline:3px solid #e0a800;outline-offset:2px}}[role=alert]{{color:#c0392b}}[role=status]{{color:var(--fg)}}</style></head>\
<body><main><header><img src=\"/login/logo.png\" alt=\"Swarm\" width=\"96\" height=\"96\"><h1>{title}</h1></header>{body}\
<nav class=\"links\" aria-label=\"{langs}\"><a href=\"{here}?lang=fr&amp;next={next_q}\" lang=\"fr\">Français</a><a href=\"{here}?lang=en&amp;next={next_q}\" lang=\"en\">English</a></nav></main></body></html>",
        title = escape(title),
        langs = if english { "Language" } else { "Langue" },
    )
}

fn login_page(english: bool, next: &str, message: &str) -> String {
    let (title, user, pass, button, forgot) = if english {
        ("Sign in to Swarm", "Account", "Password", "Sign in", "Forgot your password?")
    } else {
        ("Connexion à Swarm", "Compte", "Mot de passe", "Se connecter", "Mot de passe oublié ?")
    };
    let lang = if english { "en" } else { "fr" };
    let alert = if message.is_empty() { String::new() } else { format!("<p role=\"alert\">{}</p>", escape(message)) };
    let next_q: String = form_urlencoded::byte_serialize(next.as_bytes()).collect();
    let body = format!(
        "{alert}<form method=\"post\" action=\"/login?lang={lang}\"><input type=\"hidden\" name=\"next\" value=\"{next}\">\
<label>{user}<input name=\"subject\" autocomplete=\"username\" required autofocus></label><label>{pass}<input name=\"password\" type=\"password\" autocomplete=\"current-password\" required></label>\
<button type=\"submit\">{button}</button></form><p><a href=\"/login/forgot?lang={lang}&amp;next={next_q}\">{forgot}</a></p>",
        next = escape(next)
    );
    page(english, title, &body, "/login", next)
}

/// Forgotten-password page. `sent` shows the confirmation, identical for
/// every identifier (no account oracle).
fn forgot_page(english: bool, next: &str, sent: bool) -> String {
    let lang = if english { "en" } else { "fr" };
    let next_q: String = form_urlencoded::byte_serialize(next.as_bytes()).collect();
    let back = format!("<p><a href=\"/login?lang={lang}&amp;next={next_q}\">{}</a></p>", if english { "Back to sign-in" } else { "Retour à la connexion" });
    let body = if sent {
        format!("<p role=\"status\">{}</p>{back}", if english {
            "If this account exists, your request has been passed to the administrators. An administrator will give you a new password; your current one stays valid until then."
        } else {
            "Si ce compte existe, votre demande a été transmise aux administrateurs. Un administrateur vous communiquera un nouveau mot de passe ; l’actuel reste valable d’ici là."
        })
    } else {
        let (intro, user, button, last) = if english {
            ("Swarm runs locally without email: an administrator resets your password from Admin → Accounts and access.", "Account", "Ask for a reset",
             "Only administrator and forgot your own password? On this machine, run <code>./swarm.sh authority reset-password LOGIN</code> (sudo).")
        } else {
            ("Swarm fonctionne en local, sans e-mail : un administrateur réinitialise votre mot de passe depuis Admin → Comptes et accès.", "Compte", "Demander une réinitialisation",
             "Seul administrateur et mot de passe oublié ? Sur cette machine, lancez <code>./swarm.sh authority reset-password LOGIN</code> (sudo).")
        };
        format!("<p>{intro}</p><form method=\"post\" action=\"/login/forgot?lang={lang}\"><input type=\"hidden\" name=\"next\" value=\"{}\">\
<label>{user}<input name=\"subject\" autocomplete=\"username\" required autofocus></label><button type=\"submit\">{button}</button></form><p>{last}</p>{back}", escape(next))
    };
    page(english, if english { "Forgot your password" } else { "Mot de passe oublié" }, &body, "/login/forgot", next)
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
            Self::Session(token) => format!("session:{}", crate::sessions::token_id(token)),
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

/// Request provenance captured once per request for the audit's enriched
/// columns (ADM-05): truncated user agent, `browser`/`api-key` client and
/// the arrival instant (for `duration_ms` on outcome lines).
struct Meta {
    user_agent: Option<String>,
    client: &'static str,
    started: std::time::Instant,
}

impl Meta {
    fn from(req: &Request<Incoming>, client: &'static str) -> Self {
        let ua = req.headers().get(header::USER_AGENT).and_then(|v| v.to_str().ok()).map(|s| s.chars().take(120).collect::<String>());
        Self { user_agent: ua, client, started: std::time::Instant::now() }
    }
    /// Milliseconds elapsed since the request arrived.
    fn duration_ms(&self) -> i64 {
        self.started.elapsed().as_millis().try_into().unwrap_or(i64::MAX)
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
        (Method::GET, "/login/logo.png") => {
            let mut r = full(StatusCode::OK, "image/png", Bytes::from_static(LOGO));
            r.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=86400"));
            Ok(r)
        }
        (Method::GET, "/login/icon.svg") => {
            let mut r = full(StatusCode::OK, "image/svg+xml", Bytes::from_static(ICON));
            r.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=86400"));
            Ok(r)
        }
        (Method::GET, "/login/forgot") => {
            let next = query.iter().find(|(k, _)| k == "next").map(|(_, v)| v.as_str()).unwrap_or("/");
            Ok(login_response(StatusCode::OK, forgot_page(english, safe_next(next), false)))
        }
        (Method::POST, "/login/forgot") => {
            let meta = Meta::from(&req, "browser");
            let body = match read_body(&front, req.into_body()).await { Ok(b) => b, Err(r) => return Ok(r) };
            let form: Vec<(String, String)> = form_urlencoded::parse(&body).into_owned().collect();
            let get = |k: &str| form.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone()).unwrap_or_default();
            let (subject, next) = (get("subject"), get("next"));
            if swarm_identity_contract::name(subject.as_bytes()) {
                let (f, s) = (Arc::clone(&front), subject.clone());
                // Same answer whatever happens below (no account oracle);
                // only a new request of an existing account is recorded.
                if let Ok(Ok(Some(view))) = tokio::task::spawn_blocking(move || f.accounts.current(&Context::new(), &s)).await {
                    if front.reset_requests.request(&view.subject, view.generation, now()) {
                        let ev = audit::Event { at: now(), event: "reset-requested", subject: Some(&view.subject),
                            client: Some(meta.client), user_agent: meta.user_agent.as_deref(), ..Default::default() };
                        if front.audit.record(&ev).is_err() {
                            eprintln!("{}", serde_json::json!({"schema_version": 1, "warning": "authority-audit-unavailable"}));
                        }
                    }
                }
            }
            Ok(login_response(StatusCode::OK, forgot_page(english, safe_next(&next), true)))
        }
        (Method::GET, "/login") => {
            let next = query.iter().find(|(k, _)| k == "next").map(|(_, v)| v.as_str()).unwrap_or("/");
            Ok(login_response(StatusCode::OK, login_page(english, safe_next(next), "")))
        }
        (Method::POST, "/login") => {
            let meta = Meta::from(&req, "browser");
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
                    let via = format!("session:{}", crate::sessions::token_id(&token));
                    if front.audit.record(&audit::Event { at: now(), event: "sign-in", subject: named, credential: Some(&via),
                        client: Some(meta.client), user_agent: meta.user_agent.as_deref(), ..Default::default() }).is_err() {
                        let _ = front.sessions.logout(&token);
                        return Ok(full(StatusCode::SERVICE_UNAVAILABLE, "text/plain; charset=utf-8", "authority-audit-unavailable"));
                    }
                    let c = cookie::Cookie::build((COOKIE, token)).path("/").http_only(true).same_site(cookie::SameSite::Strict).build();
                    Ok(redirect(safe_next(&next), Some(c.to_string())))
                }
                _ => {
                    if front.audit.record(&audit::Event { at: now(), event: "sign-in-refused", subject: named,
                        client: Some(meta.client), user_agent: meta.user_agent.as_deref(), ..Default::default() }).is_err() {
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
            let meta = Meta::from(&req, "browser");
            if let Some(t) = session_token(&req) {
                if let Ok(Some(subject)) = front.sessions.logout(&t) {
                    let via = format!("session:{}", crate::sessions::token_id(&t));
                    if front.audit.record(&audit::Event { at: now(), event: "sign-out", subject: Some(&subject), credential: Some(&via),
                        client: Some(meta.client), user_agent: meta.user_agent.as_deref(), ..Default::default() }).is_err() {
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
    let meta = Meta::from(&req, "browser");
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
            Err(r) => {
                note(&front, &meta, "refused", &view.subject, &format!("session:{}", crate::sessions::token_id(&token)), req.method().as_str(), &path, Some(403));
                return Ok(r);
            }
        }
    };
    if req.method() == Method::GET || req.method() == Method::HEAD {
        return Ok(admin::handle(front, req, view, token).await);
    }
    let method = req.method().as_str().to_owned();
    let subject = view.subject.clone();
    let via = format!("session:{}", crate::sessions::token_id(&token));
    let detail = audit::admin_detail(&method, &path);
    if let Err(r) = intent(&front, &meta, &subject, &via, None, &method, &path, action, detail.as_deref()) {
        return Ok(r);
    }
    let response = admin::handle(Arc::clone(&front), req, view, token).await;
    outcome(&front, &meta, &subject, &via, None, &method, &path, action, response.status());
    Ok(response)
}

/// Resolve the session, sign an assertion for this exact request and relay
/// it to the engine; stream the engine response back unchanged.
async fn relay(front: Arc<Front>, req: Request<Incoming>) -> Result<Response<Body>, hyper::Error> {
    let path = req.uri().path_and_query().map(|p| p.as_str().to_owned()).unwrap_or_else(|| "/".into());
    let (credential, view) = match authenticate(&front, &req, &path).await { Ok(v) => v, Err(r) => return Ok(r) };
    let meta = Meta::from(&req, if matches!(credential, Credential::Key { .. }) { "api-key" } else { "browser" });
    // B6: decide before reading the body or touching the engine; a refusal
    // is final for this request and carries only stable codes.
    let action = match authorize(&front, &view, req.method().as_str(), req.uri().path()) {
        Ok(a) => a,
        Err(r) => {
            note(&front, &meta, "refused", &view.subject, &credential.label(), req.method().as_str(), req.uri().path(), Some(403));
            return Ok(r);
        }
    };
    // Page views (browser navigations) are recorded as consultations, best
    // effort; API reads are not, to keep the audit at human scale.
    let consultation = req.method() == Method::GET
        && req.headers().get(header::ACCEPT).and_then(|v| v.to_str().ok()).is_some_and(|a| a.contains("text/html"));
    if let Credential::Key { scope, .. } = &credential {
        if action > *scope {
            let body = serde_json::json!({"error": "authority-policy-denied", "action": action, "reason": "key-scope"});
            return Ok(full(StatusCode::FORBIDDEN, "application/json", body.to_string()));
        }
    }
    let mut nonce = [0u8; 16];
    if getrandom::fill(&mut nonce).is_err() {
        return Ok(full(StatusCode::SERVICE_UNAVAILABLE, "text/plain; charset=utf-8", "authority-entropy"));
    }
    let nonce: String = nonce.iter().map(|b| format!("{b:02x}")).collect();
    let reference = format!("assertion:{nonce}");
    let via = credential.label();
    let (view_subject, consulted) = (view.subject.clone(), req.uri().path().to_owned());
    let audited = (action != policy::Action::Read).then(|| (view.subject.clone(), req.method().as_str().to_owned(), req.uri().path().to_owned(), action_name(action)));
    if let Some((subject, method, p, a)) = &audited {
        if let Err(r) = intent(&front, &meta, subject, &via, Some(&reference), method, p, a, None) {
            return Ok(r);
        }
    }
    let (parts, body) = req.into_parts();
    let body = match read_body(&front, body).await { Ok(b) => b, Err(r) => return Ok(r) };
    // EP-06 witness: heads after the intent line, so a write's own intent is
    // covered. A head read failure refuses (no unwitnessed assertion).
    let f = Arc::clone(&front);
    let witness = match tokio::task::spawn_blocking(move || f.accounts.head(&Context::new())).await {
        Ok(Ok(head)) => match front.audit.tail() {
            Ok((audit_seq, audit_digest)) => assertion::Witness { journal_seq: head.sequence, journal_digest: head.digest, audit_seq, audit_digest },
            Err(code) => return Ok(full(StatusCode::SERVICE_UNAVAILABLE, "text/plain; charset=utf-8", code)),
        },
        Ok(Err(e)) => return Ok(full(StatusCode::SERVICE_UNAVAILABLE, "application/json", serde_json::json!({"error": e.code()}).to_string())),
        Err(_) => return Ok(full(StatusCode::SERVICE_UNAVAILABLE, "text/plain; charset=utf-8", "authority-state")),
    };
    let claims = assertion::Claims {
        v: 2,
        subject: view.subject,
        roles: view.roles,
        integrity: view.integrity,
        method: parts.method.as_str().to_owned(),
        path: path.clone(),
        body_sha256: assertion::body_digest(&body),
        issued_at: now(),
        nonce,
        witness,
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
    // Consultations (page views) are recorded even when the engine is
    // unreachable: the visit happened, whatever the engine's answer.
    let note_consultation = |status: u16| {
        if consultation {
            note(&front, &meta, "consultation", &view_subject, &via, "GET", &consulted, Some(status));
        }
    };
    let stream = match tokio::net::TcpStream::connect(front.engine).await {
        Ok(s) => s,
        Err(_) => {
            note_consultation(502);
            return Ok(full(StatusCode::BAD_GATEWAY, "text/plain; charset=utf-8", "authority-engine-unavailable"));
        }
    };
    let (mut sender, conn) = match hyper::client::conn::http1::handshake(hyper_util::rt::TokioIo::new(stream)).await {
        Ok(v) => v,
        Err(_) => {
            note_consultation(502);
            return Ok(full(StatusCode::BAD_GATEWAY, "text/plain; charset=utf-8", "authority-engine-handshake"));
        }
    };
    tokio::spawn(conn);
    let response = match sender.send_request(out).await {
        Ok(resp) => resp.map(|b| b.boxed()),
        Err(_) => full(StatusCode::BAD_GATEWAY, "text/plain; charset=utf-8", "authority-engine-unavailable"),
    };
    if let Some((subject, method, p, a)) = &audited {
        outcome(&front, &meta, subject, &via, Some(&reference), method, p, a, response.status());
    } else if consultation {
        note(&front, &meta, "consultation", &view_subject, &via, "GET", &consulted, Some(response.status().as_u16()));
    }
    Ok(response)
}

/// Best-effort informational line (consultation, refusal): a write failure
/// is reported on stderr and never changes the response.
fn note(front: &Front, meta: &Meta, event: &str, subject: &str, via: &str, method: &str, path: &str, status: Option<u16>) {
    let ev = audit::Event { at: now(), event, subject: Some(subject), credential: Some(via), method: Some(method), path: Some(path), status,
        view: Some(audit::view_of(path)), client: Some(meta.client), user_agent: meta.user_agent.as_deref(), ..Default::default() };
    if front.audit.record(&ev).is_err() {
        eprintln!("{}", serde_json::json!({"schema_version": 1, "warning": "authority-audit-unavailable"}));
    }
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
/// `detail` is a human-readable description derived from the path only.
#[allow(clippy::result_large_err)] // the Err is the ready HTTP refusal
#[allow(clippy::too_many_arguments)] // one audit line's fields
fn intent(front: &Front, meta: &Meta, subject: &str, via: &str, reference: Option<&str>, method: &str, path: &str, action: &str, detail: Option<&str>) -> Result<(), Response<Body>> {
    front
        .audit
        .record(&audit::Event { at: now(), event: "intent", subject: Some(subject), credential: Some(via), reference, method: Some(method), path: Some(path), action: Some(action), status: None,
            view: Some(audit::view_of(path)), client: Some(meta.client), user_agent: meta.user_agent.as_deref(), detail: detail.map(|d| d.to_string()).as_deref(), ..Default::default() })
        .map(|_| ())
        .map_err(|_| full(StatusCode::SERVICE_UNAVAILABLE, "text/plain; charset=utf-8", "authority-audit-unavailable"))
}

/// Record the outcome status after execution (best effort: the effect has
/// happened; a failure is reported on stderr, the response is unchanged).
/// Carries the request duration measured from the `Meta` capture instant.
#[allow(clippy::too_many_arguments)] // one audit line's fields
fn outcome(front: &Front, meta: &Meta, subject: &str, via: &str, reference: Option<&str>, method: &str, path: &str, action: &str, status: StatusCode) {
    let ev = audit::Event { at: now(), event: "outcome", subject: Some(subject), credential: Some(via), reference, method: Some(method), path: Some(path), action: Some(action), status: Some(status.as_u16()),
        duration_ms: Some(meta.duration_ms()), view: Some(audit::view_of(path)), client: Some(meta.client), user_agent: meta.user_agent.as_deref(), ..Default::default() };
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
