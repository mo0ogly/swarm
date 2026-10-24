//! Public browser bootstrap only. Private API routing remains separately gated.
use super::{response, HttpReply, HttpServiceError, NativeHttpService};
use http::{
    header::{ACCEPT, CONTENT_TYPE, LOCATION, SET_COOKIE},
    HeaderValue, Method, Request, StatusCode,
};

/// Exact local asset allowlist shared with Go. No request path becomes a disk path.
/// Only GET/HEAD is public; login POST has an independent exact-Origin precondition.
pub(super) fn public<B>(r: &Request<B>) -> bool {
    (r.uri().path() == "/login" && matches!(*r.method(), Method::GET | Method::HEAD | Method::POST))
        || (matches!(*r.method(), Method::GET | Method::HEAD) && asset(r.uri().path()).is_some())
}
/// Embedded canonical assets, not a file server, remote resource or traversal fallback.
fn asset(path: &str) -> Option<(&'static str, &'static [u8])> {
    match path {
        "/access.css" => Some((
            "text/css; charset=utf-8",
            include_bytes!("../../../../../web/access.css"),
        )),
        "/access.js" => Some((
            "application/javascript; charset=utf-8",
            include_bytes!("../../../../../web/access.js"),
        )),
        "/wattson_themes.css" => Some((
            "text/css; charset=utf-8",
            include_bytes!("../../../../../web/wattson_themes.css"),
        )),
        "/swarm-design.css" => Some((
            "text/css; charset=utf-8",
            include_bytes!("../../../../../web/swarm-design.css"),
        )),
        "/swarm-logo.png" | "/favicon.ico" => Some((
            "image/png",
            include_bytes!("../../../../../web/swarm-logo.png"),
        )),
        _ => None,
    }
}
/// Strict form decoding: reject bad percent escapes/UTF8 and ambiguous key fields.
/// Only representation is validated; length policy belongs to the shared transport.
/// Typed UTF8 cause is retained, never raw credential bytes in Display/Debug.
fn field(raw: &[u8]) -> Result<String, HttpServiceError> {
    let mut decoded = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        match raw[i] {
            b'+' => decoded.push(b' '),
            b'%' => {
                let pair = raw
                    .get(i + 1..i + 3)
                    .ok_or(HttpServiceError::Refused("web-login-form-escape-invalid"))?;
                let hi = (pair[0] as char).to_digit(16);
                let lo = (pair[1] as char).to_digit(16);
                let (Some(hi), Some(lo)) = (hi, lo) else {
                    return Err(HttpServiceError::Refused("web-login-form-escape-invalid"));
                };
                decoded.push((hi * 16 + lo) as u8);
                i += 2;
            }
            c => decoded.push(c),
        }
        i += 1;
    }
    std::str::from_utf8(&decoded)
        .map(str::to_owned)
        .map_err(HttpServiceError::LoginEncoding)
}
/// Unique form fields; query tokens never authenticate. Unknown/duplicate keys
/// refuse rather than selecting a convenient first/last key. Missing next is '/'.
fn form(raw: &[u8]) -> Result<(String, String), HttpServiceError> {
    let mut token = None;
    let mut next = None;
    for pair in raw.split(|c| *c == b'&') {
        if pair.is_empty() {
            continue;
        }
        let Some(eq) = pair.iter().position(|c| *c == b'=') else {
            return Err(HttpServiceError::Refused("web-login-form-invalid"));
        };
        let key = field(&pair[..eq])?;
        let value = field(&pair[eq + 1..])?;
        let slot = match key.as_str() {
            "token" => &mut token,
            "next" => &mut next,
            _ => return Err(HttpServiceError::Refused("web-login-form-field-invalid")),
        };
        if slot.replace(value).is_some() {
            return Err(HttpServiceError::Refused("web-login-form-ambiguous"));
        }
    }
    Ok((
        token.ok_or(HttpServiceError::Refused("web-login-token-required"))?,
        next.unwrap_or_else(|| "/".into()),
    ))
}
/// Keep a relative same-origin redirect. Slash/backslash/control ambiguity refuses
/// to '/'; also inspect percent-decoded controls without decoding the returned URL.
/// This is URL/header safety, not an operator navigation policy or network request.
fn redirect(target: &str) -> &str {
    let safe = |v: &str| {
        v.starts_with('/')
            && !v.starts_with("//")
            && !v.contains('\\')
            && !v.chars().any(char::is_control)
    };
    if safe(target)
        && field(target.as_bytes()).is_ok_and(|v| safe(&v))
        && target.parse::<http::Uri>().is_ok()
    {
        target
    } else {
        "/"
    }
}
/// Escape template values as HTML attribute/text content, never a script or URL.
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
/// Render the canonical Go access template through its explicit finite contract.
/// Unknown/unresolved placeholders fail closed. No untrusted HTML is interpolated;
/// only language/theme selectors and fixed public text enter this template.
fn page(english: bool, light: bool, message: &str) -> Result<Vec<u8>, HttpServiceError> {
    render(
        include_str!("../../../../../web/access.html"),
        english,
        light,
        message,
    )
}
/// Apply the finite embedded-template contract; unknown syntax is a server build
/// fault, not a caller fault. Escaping never evaluates arbitrary template code.
fn render(
    source: &str,
    english: bool,
    light: bool,
    message: &str,
) -> Result<Vec<u8>, HttpServiceError> {
    let mut text = source.replace(
        "{{if not .Message}}hidden{{end}}",
        if message.is_empty() { "hidden" } else { "" },
    );
    let values = if english {
        [("Lang","en"),("Title","Sign in to Swarm"),("Description","Local access to the native Swarm development API."),("Label","Local access key"),("Button","Sign in"),("Help","The local key is stored in the project's private .swarm directory. Engine permissions still apply."),("Dark","Dark"),("Light","Light"),("Offline","Connection interrupted. Check the server and try again."),("Loading","Signing in…")]
    } else {
        [("Lang","fr"),("Title","Connexion à Swarm"),("Description","Accès local à l’API native de développement Swarm."),("Label","Clé d’accès locale"),("Button","Se connecter"),("Help","La clé locale reste dans le dossier privé .swarm du projet. Les droits du moteur continuent de s’appliquer."),("Dark","Sombre"),("Light","État"),("Offline","Connexion interrompue. Vérifiez le serveur puis réessayez."),("Loading","Connexion…")]
    };
    let next = format!(
        "/?lang={}&theme={}",
        if english { "en" } else { "fr" },
        if light { "etat" } else { "sombre" }
    );
    for (key, value) in values.into_iter().chain([
        ("Theme", if light { "etat" } else { "sombre" }),
        ("Next", next.as_str()),
        ("Message", message),
    ]) {
        text = text.replace(&format!("{{{{.{key}}}}}"), &escape(value));
    }
    if text.contains("{{") || text.contains("}}") {
        return Err(HttpServiceError::Refused("web-login-template-invalid"));
    }
    Ok(text.into_bytes())
}
/// Public bootstrap reply with no Root mutation/effect claim. Shared CSP/no-store
/// headers apply to HTML/assets as well as JSON. HEAD omits body but keeps headers.
fn reply(
    status: StatusCode,
    body: Vec<u8>,
    head: bool,
    kind: &'static str,
    failure: Option<HttpServiceError>,
) -> HttpReply {
    let mut r = response(status, body, head, None);
    r.headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(kind));
    HttpReply {
        response: r,
        failure,
        recorded_work: None,
        configuration_may_be_recorded: false,
        recorded_configuration: None,
    }
}
impl NativeHttpService {
    /// Serve only a host/origin-checked public route. Login does not open/mutate Root;
    /// configured cookie age was validated at startup. Auth failure never sets cookie,
    /// never reflects input; its typed validation cause stays with the owner outcome.
    /// Browser-session cookies do not expire server-side bearer files. UI beyond the
    /// bootstrap assets remains unported; a successful login is not business authority.
    pub(super) fn public_reply(&self, r: &Request<Vec<u8>>) -> Result<HttpReply, HttpServiceError> {
        let head = r.method() == Method::HEAD;
        if let Some((kind, bytes)) = asset(r.uri().path()) {
            return Ok(reply(StatusCode::OK, bytes.to_vec(), head, kind, None));
        }
        let query: Vec<_> =
            form_urlencoded::parse(r.uri().query().unwrap_or("").as_bytes()).collect();
        let english = query.iter().any(|(k, v)| k == "lang" && v == "en");
        let light = query.iter().any(|(k, v)| k == "theme" && v == "etat");
        if r.method() != Method::POST {
            return Ok(reply(
                StatusCode::OK,
                page(english, light, "")?,
                head,
                "text/html; charset=utf-8",
                None,
            ));
        }
        let json = r
            .headers()
            .get(ACCEPT)
            .is_some_and(|v| v == "application/json");
        let parsed = if r.headers().get_all(CONTENT_TYPE).iter().count() == 1
            && r.headers()[CONTENT_TYPE]
                .to_str()
                .is_ok_and(|v| v.split(';').next() == Some("application/x-www-form-urlencoded"))
        {
            // Same native representation bound used by other typed service requests.
            swarm_engine::check_command_input_size(r.body())
                .map_err(HttpServiceError::Input)
                .and_then(|()| form(r.body()))
        } else {
            Err(HttpServiceError::Refused("web-login-content-type-invalid"))
        };
        let (token, next) = match parsed {
            Err(error @ HttpServiceError::Input(_)) => return Err(error),
            Ok((token, next)) if self.authority.matches(token.as_bytes()) => (token, next),
            result => {
                let error = match result {
                    Err(error) => error,
                    Ok(_) => HttpServiceError::Refused("web-login-key-refused"),
                };
                let message = if english {
                    "Access key rejected. Try again."
                } else {
                    "Clé d’accès refusée. Réessayez."
                };
                let body = if json {
                    serde_json::to_vec(&serde_json::json!({"error":message}))
                        .map_err(HttpServiceError::Json)?
                } else {
                    page(english, light, message)?
                };
                return Ok(reply(
                    StatusCode::UNAUTHORIZED,
                    body,
                    head,
                    if json {
                        "application/json; charset=utf-8"
                    } else {
                        "text/html; charset=utf-8"
                    },
                    Some(error),
                ));
            }
        };
        let next = redirect(&next);
        let body = if json {
            serde_json::to_vec(&serde_json::json!({"redirect":next}))
                .map_err(HttpServiceError::Json)?
        } else {
            vec![]
        };
        let mut result = reply(
            if json {
                StatusCode::OK
            } else {
                StatusCode::SEE_OTHER
            },
            body,
            head,
            "application/json; charset=utf-8",
            None,
        );
        let mut cookie = cookie::Cookie::build((self.authority.cookie_name().to_owned(), token))
            .path("/")
            .http_only(true)
            .same_site(cookie::SameSite::Strict)
            .build();
        cookie.set_max_age(
            self.cookie_max_age_seconds
                .map(cookie::time::Duration::seconds),
        );
        result.response.headers_mut().insert(
            SET_COOKIE,
            HeaderValue::from_str(&cookie.to_string()).map_err(HttpServiceError::HeaderEncoding)?,
        );
        if !json {
            result.response.headers_mut().insert(
                LOCATION,
                HeaderValue::from_str(next).map_err(HttpServiceError::HeaderEncoding)?,
            );
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// A valid canonical page precedes injected unknown template syntax. Rendering
    /// must fail as server500 while keeping the original typed failure in the reply.
    #[test]
    fn native_login_unknown_template_is_a_server_failure_not_partial_html() {
        let valid = render(
            include_str!("../../../../../web/access.html"),
            true,
            false,
            "<unsafe & \"quoted\">",
        )
        .unwrap();
        let html = std::str::from_utf8(&valid).unwrap();
        assert!(html.contains("&lt;unsafe &amp; &quot;quoted&quot;&gt;"));
        assert!(!html.contains("{{"));
        for unknown in ["{{.Unknown}}", "{{if .Unknown}}", "unbalanced {{"] {
            let failure = match render(unknown, true, false, "") {
                Err(e) => e,
                Ok(_) => panic!("Unknown template was accepted"),
            };
            let reply = super::super::error_reply(failure, false).unwrap();
            assert_eq!(reply.response.status(), StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(
                reply.failure.as_ref().unwrap().code(),
                "web-login-template-invalid"
            );
            assert!(!reply.response.body().is_empty());
        }
    }
}
