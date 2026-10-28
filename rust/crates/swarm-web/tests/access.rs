//! Protected request fixtures only: no live server, account or agent authority.
use http::{
    header::{COOKIE, HOST, ORIGIN},
    HeaderValue, Request,
};
use std::{error::Error, net::TcpListener};
use swarm_web::{LocalWebAuthority, WebAccessError};
const KEY: &str = "session-0123456789abcdef01234567";
/// Actual private loopback listener supplies an observed assigned port; no fixed port.
fn fixture() -> (TcpListener, LocalWebAuthority) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let authority = LocalWebAuthority::from_listener(&listener, KEY).unwrap();
    (listener, authority)
}
/// Valid protected request witness; fixtures never exercise a mutation on GET.
fn request(a: &LocalWebAuthority, method: &str) -> Request<Vec<u8>> {
    let mut r = Request::builder()
        .method(method)
        .uri("/api/v1/action")
        .header(HOST, a.host())
        .header(COOKIE, format!("{}={KEY}", a.cookie_name()))
        .body(b"unconsumed fixture body".to_vec())
        .unwrap();
    if method != "GET" && method != "HEAD" {
        r.headers_mut()
            .insert(ORIGIN, format!("http://{}", a.host()).parse().unwrap());
        r.headers_mut().insert("x-swarm-csrf", KEY.parse().unwrap());
    }
    r
}
/// Stable error family and safe diagnostics, never a reflected credential/header.
fn refused(r: Result<(), WebAccessError>, code: &str) {
    let e = r.unwrap_err();
    assert_eq!(e.code(), code);
    assert!(!format!("{e:?} {e}").contains(KEY));
}
/// Real listener address is observed, rather than trusting a claimed Host/forwarded header.
#[test]
fn listener_scope_and_credential_format_are_explicit() {
    let (_l, a) = fixture();
    assert!(a.host().starts_with("127.0.0.1:"));
    assert!(a.cookie_name().starts_with("swarm_session_"));
    assert!(!format!("{a:?}").contains(KEY));
    for addr in ["0.0.0.0:0", "127.0.0.2:0"] {
        let l = TcpListener::bind(addr).unwrap();
        refused(
            LocalWebAuthority::from_listener(&l, KEY).map(|_| ()),
            "web-listener-loopback-required",
        );
    }
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    for key in [
        "",
        "PRIVATE fake",
        "session-short",
        "not-ses-0123456789abcdef01234567",
    ] {
        refused(
            LocalWebAuthority::from_listener(&l, key).map(|_| ()),
            "web-session-credential-invalid",
        );
    }
}
/// All protected methods use cookies; only GET/HEAD omit mutation Origin/CSRF.
#[test]
fn valid_protected_methods_and_quoted_legacy_cookie_are_compatible() {
    let (_l, a) = fixture();
    for method in ["GET", "HEAD", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"] {
        a.authorize(&request(&a, method)).unwrap();
    }
    for cookie in [
        format!("swarm_session={KEY}"),
        format!("unrelated=okay; swarm_session=\"{KEY}\""),
    ] {
        let mut r = request(&a, "GET");
        r.headers_mut().insert(COOKIE, cookie.parse().unwrap());
        a.authorize(&r).unwrap();
    }
}
/// A positive witness precedes rebinding and proxy-target faults; forwarded values grant nothing.
#[test]
fn host_and_absolute_target_cannot_redirect_authority() {
    let (_l, a) = fixture();
    let valid = request(&a, "POST");
    a.authorize(&valid).unwrap();
    let mut r = request(&a, "POST");
    r.headers_mut().insert(HOST, "evil.test".parse().unwrap());
    r.headers_mut()
        .insert("x-forwarded-host", a.host().parse().unwrap());
    refused(a.authorize(&r), "web-host-refused");
    let mut r = request(&a, "POST");
    r.headers_mut().remove(HOST);
    refused(a.authorize(&r), "web-host-refused");
    for uri in [
        "http://evil.test/api/v1/action".to_string(),
        format!("https://{}/api/v1/action", a.host()),
    ] {
        let mut r = request(&a, "POST");
        *r.uri_mut() = uri.parse().unwrap();
        refused(a.authorize(&r), "web-target-refused");
    }
    let mut r = request(&a, "POST");
    *r.uri_mut() = format!("http://{}/api/v1/action", a.host())
        .parse()
        .unwrap();
    a.authorize(&r).unwrap();
}
/// Missing/wrong scoped credentials cannot fall back to a valid legacy cookie.
#[test]
fn session_priority_and_cookie_ambiguity_fail_closed() {
    let (_l, a) = fixture();
    a.authorize(&request(&a, "GET")).unwrap();
    let mut r = request(&a, "GET");
    r.headers_mut().remove(COOKIE);
    refused(a.authorize(&r), "web-session-required");
    let mut r = request(&a, "GET");
    r.headers_mut().insert(
        COOKIE,
        format!("{}=wrong; swarm_session={KEY}", a.cookie_name())
            .parse()
            .unwrap(),
    );
    refused(a.authorize(&r), "web-session-required");
    let mut r = request(&a, "GET");
    r.headers_mut().append(
        COOKIE,
        format!("{}={KEY}", a.cookie_name()).parse().unwrap(),
    );
    refused(a.authorize(&r), "web-cookie-ambiguous");
    let mut r = request(&a, "GET");
    r.headers_mut().insert(
        COOKIE,
        format!("swarm_session={KEY}; swarm_session={KEY}")
            .parse()
            .unwrap(),
    );
    refused(a.authorize(&r), "web-cookie-ambiguous");
    let mut r = request(&a, "GET");
    r.headers_mut()
        .append(COOKIE, "other=valid".parse().unwrap());
    a.authorize(&r).unwrap();
}
/// No absent/wrong/multiple mutation headers; the same valid request remains usable afterwards.
#[test]
fn origin_csrf_and_duplicate_sensitive_headers_are_checked() {
    let (_l, a) = fixture();
    a.authorize(&request(&a, "POST")).unwrap();
    for field in ["origin", "x-swarm-csrf"] {
        let expected = if field == "origin" {
            "web-origin-refused"
        } else {
            "web-csrf-refused"
        };
        let mut r = request(&a, "POST");
        r.headers_mut().remove(field);
        refused(a.authorize(&r), expected);
        let mut r = request(&a, "POST");
        r.headers_mut()
            .insert(field, "PRIVATE wrong".parse().unwrap());
        refused(a.authorize(&r), expected);
        let mut r = request(&a, "POST");
        let v = r.headers()[field].clone();
        r.headers_mut().append(field, v);
        refused(a.authorize(&r), "web-header-ambiguous");
    }
    let mut r = request(&a, "GET");
    r.headers_mut().append(HOST, a.host().parse().unwrap());
    refused(a.authorize(&r), "web-header-ambiguous");
    a.authorize(&request(&a, "POST")).unwrap();
}
/// Parser/encoding causes survive but private values do not escape Display/Debug.
#[test]
fn malformed_cookie_and_non_ascii_header_keep_safe_causes() {
    let (_l, a) = fixture();
    a.authorize(&request(&a, "GET")).unwrap();
    let mut r = request(&a, "GET");
    r.headers_mut()
        .insert(COOKIE, HeaderValue::from_bytes(b"\xffPRIVATE").unwrap());
    let e = a.authorize(&r).unwrap_err();
    assert_eq!(e.code(), "web-header-invalid");
    assert!(e.source().is_some());
    assert!(!format!("{e:?} {e}").contains("PRIVATE"));
    let mut r = request(&a, "GET");
    r.headers_mut().insert(
        COOKIE,
        format!("PRIVATE-malformed; {}={KEY}", a.cookie_name())
            .parse()
            .unwrap(),
    );
    let e = a.authorize(&r).unwrap_err();
    assert_eq!(e.code(), "web-cookie-invalid");
    assert!(e.source().is_some());
    assert!(!format!("{e:?} {e}").contains("PRIVATE"));
}
/// Request-head authorization performs no body IO, network/Root mutation or acceptance.
#[test]
fn protected_gate_does_not_consume_or_replace_the_body() {
    let (_l, a) = fixture();
    let r = request(&a, "POST");
    let r = r.map(std::io::Cursor::new);
    assert_eq!(r.body().position(), 0);
    a.authorize(&r).unwrap();
    assert_eq!(r.body().position(), 0);
    assert_eq!(r.body().get_ref(), b"unconsumed fixture body");
}
