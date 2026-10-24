//! Protected development landing and shared Admin assets; no filesystem routing.
use super::{response, HttpReply, HttpServiceError};
use http::{header::CONTENT_TYPE, HeaderValue, Method, Request, StatusCode};
/// Exact private resource paths. Unknown/traversal-shaped paths never become IO.
pub(super) fn owns(path: &str) -> bool {
    matches!(path, "/" | "/native-runtime.html") || asset(path).is_some()
}
/// Read-only local browser responses may wait on existing slots; API/POST/body
/// requests cannot use this classification to queue a Root operation.
pub(super) fn resource<B>(r: &Request<B>) -> bool {
    matches!(*r.method(), Method::GET | Method::HEAD) && owns(r.uri().path())
}
/// Compile canonical shared assets directly; bytes are versioned source inputs,
/// not a public directory, credential-bearing runtime file or external resource.
fn asset(path: &str) -> Option<(&'static str, &'static [u8])> {
    match path {
        "/native-runtime/base.css" => Some((
            "text/css; charset=utf-8",
            include_bytes!("../../../../../web/processes.css"),
        )),
        "/native-runtime/admin.css" => Some((
            "text/css; charset=utf-8",
            include_bytes!("../../../../../frontend/engine-settings/admin.css"),
        )),
        "/native-runtime/app.css" => Some((
            "text/css; charset=utf-8",
            include_bytes!("../../../../../web/native-runtime.css"),
        )),
        "/native-runtime/english.js" => Some((
            "application/javascript; charset=utf-8",
            include_bytes!("../../../../../web/i18n-en.js"),
        )),
        "/native-runtime/i18n.js" => Some((
            "application/javascript; charset=utf-8",
            include_bytes!("../../../../../web/i18n.js"),
        )),
        "/native-runtime/settings.js" => Some((
            "application/javascript; charset=utf-8",
            include_bytes!("../../../../../frontend/engine-settings/admin.js"),
        )),
        "/native-runtime/quotas.js" => Some((
            "application/javascript; charset=utf-8",
            include_bytes!("../../../../../web/native-quotas.js"),
        )),
        "/native-runtime/app.js" => Some((
            "application/javascript; charset=utf-8",
            include_bytes!("../../../../../web/native-runtime.js"),
        )),
        _ => None,
    }
}
/// Render only trusted language/theme choices into one known static tag. No query
/// value becomes HTML. A source-contract mismatch is a server error, not partial UI.
fn page(r: &Request<Vec<u8>>) -> Result<Vec<u8>, HttpServiceError> {
    let template = include_str!("../../../../../web/native-runtime.html");
    let tag = "<html lang=\"fr\" data-theme=\"sombre\">";
    if template.matches(tag).count() != 1 {
        return Err(HttpServiceError::Refused("web-native-template-invalid"));
    }
    let query: Vec<_> = form_urlencoded::parse(r.uri().query().unwrap_or("").as_bytes()).collect();
    let language = if query.iter().any(|(k, v)| k == "lang" && v == "en") {
        "en"
    } else {
        "fr"
    };
    let theme = if query.iter().any(|(k, v)| k == "theme" && v == "etat") {
        "etat"
    } else {
        "sombre"
    };
    Ok(template
        .replacen(
            tag,
            &format!("<html lang=\"{language}\" data-theme=\"{theme}\">"),
            1,
        )
        .into_bytes())
}
/// Respond only after the caller checks the scoped cookie/exact Host and CSRF for
/// non-GET/HEAD methods. Assets cannot mutate Root; unsupported methods return405.
/// Shared CSP/no-store/HEAD behavior applies. The page truthfully hosts only native
/// settings, not full cockpit/provider/workflow parity or installed Go replacement.
pub(super) fn reply(r: &Request<Vec<u8>>) -> Result<HttpReply, HttpServiceError> {
    if !matches!(*r.method(), Method::GET | Method::HEAD) {
        return Err(HttpServiceError::Method("GET, HEAD"));
    }
    let (kind, bytes) = if matches!(r.uri().path(), "/" | "/native-runtime.html") {
        ("text/html; charset=utf-8", page(r)?)
    } else {
        let (kind, bytes) =
            asset(r.uri().path()).ok_or(HttpServiceError::Refused("web-route-unported"))?;
        (kind, bytes.to_vec())
    };
    let mut response = response(StatusCode::OK, bytes, r.method() == Method::HEAD, None);
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(kind));
    Ok(HttpReply {
        response,
        failure: None,
        recorded_work: None,
        configuration_may_be_recorded: false,
        recorded_configuration: None,
    })
}
