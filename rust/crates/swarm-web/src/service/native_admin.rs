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
        "/paea.html"=>Some(("text/html; charset=utf-8",include_bytes!("../../../../../web/paea.html"))),
        "/paea-payment.html"=>Some(("text/html; charset=utf-8",include_bytes!("../../../../../web/paea-payment.html"))),
        "/paea.js"=>Some(("application/javascript; charset=utf-8",include_bytes!("../../../../../web/paea.js"))),
        "/paea.css"=>Some(("text/css; charset=utf-8",include_bytes!("../../../../../web/paea.css"))),
        "/trust-actions.html"=>Some(("text/html; charset=utf-8",include_bytes!("../../../../../web/trust-native.html"))),
        "/trust-native.js"=>Some(("application/javascript; charset=utf-8",include_bytes!("../../../../../web/trust-native.js"))),
        "/trust-native.css"=>Some(("text/css; charset=utf-8",include_bytes!("../../../../../web/trust-native.css"))),

        "/cockpit.css" => Some(("text/css; charset=utf-8", include_bytes!("../../../../../web/cockpit.css"))),
        "/cockpit-ux.css" => Some(("text/css; charset=utf-8", include_bytes!("../../../../../web/cockpit-ux.css"))),
        "/agent-terminal.css" => Some(("text/css; charset=utf-8", include_bytes!("../../../../../web/agent-terminal.css"))),
        "/log-console.css" => Some(("text/css; charset=utf-8", include_bytes!("../../../../../web/log-console.css"))),
        "/product-views.css" => Some(("text/css; charset=utf-8", include_bytes!("../../../../../web/product-views.css"))),
        "/mission.css" => Some(("text/css; charset=utf-8", include_bytes!("../../../../../web/mission.css"))),
        "/pilotage.css" => Some(("text/css; charset=utf-8", include_bytes!("../../../../../web/pilotage.css"))),
        "/conduite.css" => Some(("text/css; charset=utf-8", include_bytes!("../../../../../web/conduite.css"))),
        "/graph.css" => Some(("text/css; charset=utf-8", include_bytes!("../../../../../web/graph.css"))),
        "/plan.css" => Some(("text/css; charset=utf-8", include_bytes!("../../../../../web/plan.css"))),
        "/assistant.css" => Some(("text/css; charset=utf-8", include_bytes!("../../../../../web/assistant.css"))),
        "/project-profile-settings.css" => Some(("text/css; charset=utf-8", include_bytes!("../../../../../web/project-profile-settings.css"))),
        "/providers.css" => Some(("text/css; charset=utf-8", include_bytes!("../../../../../web/providers.css"))),
        "/preparation-timeout-admin.css" => Some(("text/css; charset=utf-8", include_bytes!("../../../../../web/preparation-timeout-admin.css"))),
        "/context-help.css" => Some(("text/css; charset=utf-8", include_bytes!("../../../../../web/context-help.css"))),
        "/i18n-en.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../../../../../web/i18n-en.js"))),
        "/i18n.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../../../../../web/i18n.js"))),
        "/swarm-shell.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../../../../../web/swarm-shell.js"))),
        "/cockpit-native-read.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../../../../../web/cockpit-native-read.js"))),
        "/cockpit-native-instructions.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../../../../../web/cockpit-native-instructions.js"))),
        "/cockpit-native-panels.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../../../../../web/cockpit-native-panels.js"))),
        "/lib/dagre.min.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../../../../../web/lib/dagre.min.js"))),
        "/graph.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../../../../../web/graph.js"))),
        "/cockpit-native-graph.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../../../../../web/cockpit-native-graph.js"))),
        "/cockpit-native-budget.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../../../../../web/cockpit-native-budget.js"))),
        "/cockpit-native-logs.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../../../../../web/cockpit-native-logs.js"))),
        "/cockpit-native-resume.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../../../../../web/cockpit-native-resume.js"))),
        "/runtime-health.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../../../../../web/runtime-health.js"))),
        "/cockpit-native-version.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../../../../../web/cockpit-native-version.js"))),
        "/pricing.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../../../../../web/pricing.js"))),
        "/cockpit-native-pricing.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../../../../../web/cockpit-native-pricing.js"))),
        "/cockpit-native-quotas.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../../../../../web/cockpit-native-quotas.js"))),
        "/cockpit-native.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../../../../../web/cockpit-native.js"))),
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
        "/native-runtime/cockpit.js" => Some((
            "application/javascript; charset=utf-8",
            include_bytes!("../../../../../web/native-cockpit.js"),
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
/// Reopen the canonical product HTML, retaining its existing structure/styles.
/// Legacy scripts require the full Go snapshot and mutation contracts; select
/// explicit native adapters instead, never feed them fabricated snapshots.
/// Only trusted source tags are parsed. Missing/unterminated tags refuse the
/// entire page; no filesystem lookup, user HTML or command is involved.
fn cockpit_page(r: &Request<Vec<u8>>) -> Result<Vec<u8>, HttpServiceError> {
    let mut source = include_str!("../../../../../web/index.html").to_owned();
    while let Some(start) = source.find("<script") {
        let end = source[start..].find("</script>")
            .ok_or(HttpServiceError::Refused("web-cockpit-template-invalid"))?;
        source.replace_range(start..start + end + "</script>".len(), "");
    }
    if source.matches("</head>").count() != 1 {
        return Err(HttpServiceError::Refused("web-cockpit-template-invalid"));
    }
    source=source.replacen("</nav>","<a href=\"/trust-actions.html\" data-i18n>Biba — Actions de confiance</a><a href=\"/paea.html\" data-i18n>Vérifications PAEA</a></nav>",1);
    let scripts = "<link rel=\"stylesheet\" href=\"/native-runtime/admin.css\"><script src=\"/i18n-en.js\" defer></script><script src=\"/i18n.js\" defer></script><script src=\"/swarm-shell.js\" defer></script><script src=\"/lib/dagre.min.js\" defer></script><script src=\"/graph.js\" defer></script><script src=\"/cockpit-native-graph.js\" defer></script><script src=\"/cockpit-native-read.js\" defer></script><script src=\"/cockpit-native-panels.js\" defer></script><script src=\"/cockpit-native-budget.js\" defer></script><script src=\"/native-runtime/quotas.js\" defer></script><script src=\"/cockpit-native-quotas.js\" defer></script><script src=\"/cockpit-native-pricing.js\" defer></script><script src=\"/pricing.js\" defer></script><script src=\"/cockpit-native-version.js\" defer></script><script src=\"/runtime-health.js\" defer></script><script src=\"/cockpit-native-resume.js\" defer></script><script src=\"/cockpit-native-logs.js\" defer></script><script src=\"/cockpit-native-instructions.js\" defer></script><script src=\"/cockpit-native.js\" defer></script></head>";
    source = source.replacen("</head>", scripts, 1);
    let tag = "<html lang=\"fr\" data-theme=\"etat\">";
    if source.matches(tag).count() != 1 {
        return Err(HttpServiceError::Refused("web-cockpit-template-invalid"));
    }
    let query: Vec<_> = form_urlencoded::parse(r.uri().query().unwrap_or("").as_bytes()).collect();
    let language = if query.iter().any(|(k,v)| k=="lang" && v=="en") { "en" } else { "fr" };
    let theme = if query.iter().any(|(k,v)| k=="theme" && v=="sombre") { "sombre" } else { "etat" };
    Ok(source.replacen(tag, &format!("<html lang=\"{language}\" data-theme=\"{theme}\">"), 1).into_bytes())
}

/// Respond only after the caller checks the scoped cookie/exact Host and CSRF for
/// non-GET/HEAD methods. Assets cannot mutate Root; unsupported methods return405.
/// Shared CSP/no-store/HEAD behavior applies. The page truthfully hosts only native
/// mission reads, settings and unused-instruction rejection, not complete parity.
/// Explicit writes reuse protected Root APIs and their revision/ownership checks;
/// asset delivery itself grants no mutation or complete workflow authority.
pub(super) fn reply(r: &Request<Vec<u8>>) -> Result<HttpReply, HttpServiceError> {
    if !matches!(*r.method(), Method::GET | Method::HEAD) {
        return Err(HttpServiceError::Method("GET, HEAD"));
    }
    let (kind, bytes) = if r.uri().path() == "/" {
        ("text/html; charset=utf-8", cockpit_page(r)?)
    } else if r.uri().path() == "/native-runtime.html" {
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
