//! Reuse the existing preparation document layout with the native manual writer.
use super::HttpServiceError;
use http::Request;

/// Render the trusted body from prepare.html with native, locally embedded
/// dependencies. Query values select only FR/EN and the two known themes. No
/// provider, filesystem routing or launch authority is introduced. A changed
/// template boundary refuses instead of serving a partly wired legacy page.
pub(super) fn page(request: &Request<Vec<u8>>) -> Result<Vec<u8>, HttpServiceError> {
    let source = include_str!("../../../../../web/prepare.html");
    if source.matches("<body ").count() != 1 {
        return Err(HttpServiceError::Refused("web-native-preparation-template-invalid"));
    }
    let (_, body) = source.split_once("<body ")
        .ok_or(HttpServiceError::Refused("web-native-preparation-template-invalid"))?;
    // IDs are the trusted layout contract; attribute order is not.
    let mut body=body.to_owned();
    for (id,state) in [("create","disabled"),("new-title","readonly"),("new-need","readonly")] {
        let marker=format!("id=\"{id}\"");
        if body.matches(&marker).count()!=1 {
            return Err(HttpServiceError::Refused("web-native-preparation-template-invalid"));
        }
        body=body.replacen(&marker,&format!("{marker} {state}"),1);
    }
    let query: Vec<_> = form_urlencoded::parse(request.uri().query().unwrap_or("").as_bytes()).collect();
    let lang = if query.iter().any(|(k,v)|k=="lang" && v=="en") {"en"} else {"fr"};
    let theme = if query.iter().any(|(k,v)|k=="theme" && v=="etat") {"etat"} else {"sombre"};
    Ok(format!(r#"<!doctype html><html lang="{lang}" data-theme="{theme}">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<link rel="icon" href="/swarm-logo.png" type="image/png"><title>Préparer — Swarm</title>
<link rel="stylesheet" href="/wattson_themes.css"><link rel="stylesheet" href="/prephase.css">
<link rel="stylesheet" href="/swarm-design.css"><script src="/i18n-en.js" defer></script>
<script src="/i18n.js" defer></script><script src="/swarm-shell.js" defer></script>
<script type="module" src="/preparation-native.js"></script></head><body {body}"#).into_bytes())
}
