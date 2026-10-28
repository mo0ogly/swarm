//! Protected HTTP/1 request-head checks before any body read or Root operation.
use crate::WebAccessError as E;
use hmac::{Hmac, KeyInit, Mac};
use http::{
    header::{COOKIE, HOST, ORIGIN},
    HeaderMap, HeaderValue, Method, Request,
};
use sha2::{Digest, Sha256};
use std::{
    fmt,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, TcpListener},
};
use subtle::ConstantTimeEq;
/// Local bearer-capability gate scoped to an observed bound loopback address.
/// Private fields prevent deserializing request data as authority. Construction
/// validates representation only: the caller still owes trusted credential
/// creation/storage, listener lifetime, session policy and business Root checks.
/// This is not an authenticated agent/model identity or a revocation/OS lease.
/// Debug excludes the credential; secure memory erasure is not established.
pub struct LocalWebAuthority {
    host: String,
    cookie_name: String,
    credential: Box<[u8]>,
    csrf: String,
}
impl fmt::Debug for LocalWebAuthority {
    /// Show public address/cookie namespace only; never the bearer credential.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalWebAuthority")
            .field("host", &self.host)
            .field("cookie_name", &self.cookie_name)
            .finish_non_exhaustive()
    }
}
impl LocalWebAuthority {
    /// Bind a persisted credential to the same actual listener address. This
    /// checks address consistency, not Root identity; trusted service bootstrap
    /// must obtain the session from the Root it serves. No credential is logged.
    /// A different port/address refuses; listener IO causes remain typed.
    #[cfg(target_os = "linux")]
    pub fn from_session(
        listener: &TcpListener,
        session: &swarm_engine::WebSessionCredential,
    ) -> Result<Self, E> {
        if listener.local_addr().map_err(E::Listener)? != session.address() {
            return Err(E::Refused("web-session-listener-mismatch"));
        }
        Self::from_listener(listener, session.secret())
    }
    /// Observe a real listener's assigned address, accepting exactly Go's local
    /// 127.0.0.1/::1 bindings. Require Go session-prefix/24-character name format;
    /// it is representation compatibility, never proof of randomness or ownership.
    /// No accept/read/write, Root mutation, timeout, default credential or retry.
    /// Retains local_addr failure; invalid binding/credential refuses safely.
    /// The caller must keep this same listener and trusted credential source alive.
    pub fn from_listener(listener: &TcpListener, credential: &str) -> Result<Self, E> {
        #[cfg(target_os = "linux")]
        let valid = swarm_engine::valid_web_session_credential(credential);
        #[cfg(not(target_os = "linux"))]
        let valid = credential.starts_with("session-")
            && credential.len() == "session-".len() + 24
            && swarm_identity_contract::name(credential.as_bytes());
        if !valid {
            return Err(E::Refused("web-session-credential-invalid"));
        }
        let address = listener.local_addr().map_err(E::Listener)?;
        Self::from_observed_address(address, credential)
    }
    /// Pure construction from listener metadata, kept private to prevent client
    /// address claims from becoming a public authority constructor. Production
    /// calls only after local_addr on the real listener; unit fixtures verify
    /// representation without claiming that the host supports IPv6 sockets.
    fn from_observed_address(address: std::net::SocketAddr, credential: &str) -> Result<Self, E> {
        if !matches!(address.ip(),IpAddr::V4(v) if v==Ipv4Addr::LOCALHOST)
            && !matches!(address.ip(),IpAddr::V6(v) if v==Ipv6Addr::LOCALHOST)
        {
            return Err(E::Refused("web-listener-loopback-required"));
        }
        let host = address.to_string();
        let hash = format!("{:x}", Sha256::digest(host.as_bytes()));
        // Go webSessionCookie's first12 SHA hex bytes name the address-scoped
        // cookie. This is protocol namespacing, not a secret/strength policy.
        let cookie_name = format!("swarm_session_{}", &hash[..12]);
        let mut mac =
            Hmac::<sha2_csrf::Sha256>::new_from_slice(credential.as_bytes()).map_err(E::Csrf)?;
        mac.update(b"swarm-local-csrf-v1\0");
        mac.update(host.as_bytes());
        let output = mac.finalize().into_bytes();
        let mut csrf = String::with_capacity(output.len() * 2);
        // Hexadecimal is protocol encoding, not an operator policy.
        for byte in output {
            csrf.push(char::from(b"0123456789abcdef"[usize::from(byte >> 4)]));
            csrf.push(char::from(b"0123456789abcdef"[usize::from(byte & 15)]));
        }
        Ok(Self {
            host,
            cookie_name,
            csrf,
            credential: credential.as_bytes().into(),
        })
    }
    /// Observed exact Host authority, including assigned port/IPv6 brackets.
    /// Public metadata only; it does not prove the listener is still live.
    pub fn host(&self) -> &str {
        &self.host
    }
    /// Public scoped cookie name compatible with Go; never exposes its value.
    pub fn cookie_name(&self) -> &str {
        &self.cookie_name
    }
    /// Dedicated HMAC-SHA256 v1 token bound to this observed Host/private key.
    /// Expose only to same-origin cookie-authorized clients, never logs. It cannot
    /// replace the bearer cookie/login key and is not agent/human authority.
    /// Legacy bearer-as-CSRF remains accepted only with cookie and exact Origin.
    pub fn csrf_token(&self) -> &str {
        &self.csrf
    }
    /// Check exact Host/absolute target before trusted bootstrap/static handling.
    /// This grants no session/business authority and consumes no body. Protected
    /// routes must additionally use authorize; no address claim is accepted.
    pub(crate) fn authorize_host<B>(&self, request: &Request<B>) -> Result<(), E> {
        let host = one(request.headers(), HOST.as_str())?
            .ok_or(E::Refused("web-host-refused"))?
            .to_str()
            .map_err(E::Header)?;
        if host != self.host {
            return Err(E::Refused("web-host-refused"));
        }
        let uri = request.uri();
        if let Some(authority) = uri.authority() {
            if uri.scheme_str() != Some("http") || authority.as_str() != self.host {
                return Err(E::Refused("web-target-refused"));
            }
        } else if uri.scheme().is_some() {
            return Err(E::Refused("web-target-refused"));
        }
        Ok(())
    }
    /// Authorize a protected request head without consuming/replacing its body.
    /// Require one exact Host and a consistent HTTP absolute target, then valid
    /// scoped cookie (legacy only if scoped is absent). Non-GET/HEAD additionally
    /// require one exact HTTP Origin and a dedicated X-Swarm-CSRF token (legacy
    /// bearer-as-CSRF compatibility also accepted).
    /// Ambiguous sensitive headers/cookies or encoding/parse errors refuse and
    /// retain safe causes. Content equality uses constant-time comparison at the
    /// known public32/64byte representation lengths; length differences are not hidden.
    /// This grants only local request access: routes must restrict methods, keep
    /// business permissions/revision/evidence checks and handle body/Root errors.
    /// No login/static exception, session expiry/rotation, parser/network bound,
    /// listener loop, response/CSP or underlying business action is implemented.
    pub fn authorize<B>(&self, request: &Request<B>) -> Result<(), E> {
        self.authorize_host(request)?;
        self.session(request.headers())?;
        if request.method() != Method::GET && request.method() != Method::HEAD {
            let origin = one(request.headers(), ORIGIN.as_str())?
                .ok_or(E::Refused("web-origin-refused"))?
                .to_str()
                .map_err(E::Header)?;
            if origin != format!("http://{}", self.host) {
                return Err(E::Refused("web-origin-refused"));
            }
            let csrf =
                one(request.headers(), "x-swarm-csrf")?.ok_or(E::Refused("web-csrf-refused"))?;
            let dedicated = bool::from(self.csrf.as_bytes().ct_eq(csrf.as_bytes()));
            let legacy = self.matches(csrf.as_bytes());
            if !dedicated && !legacy {
                return Err(E::Refused("web-csrf-refused"));
            }
        }
        Ok(())
    }
    /// Public login checks exact Host and one exact POST Origin, without cookie
    /// or CSRF (the submitted key is checked only after bounded form decoding).
    /// This grants no protected-route/session/business authority or body access.
    pub(crate) fn authorize_login<B>(&self, request: &Request<B>) -> Result<(), E> {
        self.authorize_host(request)?;
        if request.method() == Method::POST {
            let origin = one(request.headers(), ORIGIN.as_str())?
                .ok_or(E::Refused("web-origin-refused"))?
                .to_str()
                .map_err(E::Header)?;
            if origin != format!("http://{}", self.host) {
                return Err(E::Refused("web-origin-refused"));
            }
        }
        Ok(())
    }
    /// Check every cookie field explicitly; malformed unrelated cookies also
    /// refuse (stricter than Go's skip behavior). Quoted values are unwrapped by
    /// the maintained cookie parser; percent decoding is deliberately absent.
    /// Save comparison observations only, not another copy of credential strings.
    /// Duplicate scoped/legacy names refuse; Some(false) scoped blocks fallback.
    fn session(&self, headers: &HeaderMap) -> Result<(), E> {
        let mut scoped = None;
        let mut legacy = None;
        for field in headers.get_all(COOKIE).iter() {
            let field = field.to_str().map_err(E::Header)?;
            for parsed in cookie::Cookie::split_parse(field) {
                let parsed = parsed.map_err(E::Cookie)?;
                let slot = if parsed.name() == self.cookie_name {
                    &mut scoped
                } else if parsed.name() == "swarm_session" {
                    &mut legacy
                } else {
                    continue;
                };
                if slot.is_some() {
                    return Err(E::Refused("web-cookie-ambiguous"));
                }
                *slot = Some(self.matches(parsed.value_trimmed().as_bytes()));
            }
        }
        if scoped.or(legacy) != Some(true) {
            return Err(E::Refused("web-session-required"));
        }
        Ok(())
    }
    /// Compare content without early byte-position exit. Credential length/format
    /// are public protocol facts; this comparison alone grants no other authority.
    pub(crate) fn matches(&self, value: &[u8]) -> bool {
        bool::from(self.credential.as_ref().ct_eq(value))
    }
}
/// Select at most one sensitive header; no first/last-wins ambiguity or text copy.
/// Absence stays explicit for the caller's safe refusal; duplicate values refuse.
fn one<'a>(headers: &'a HeaderMap, name: &str) -> Result<Option<&'a HeaderValue>, E> {
    let mut values = headers.get_all(name).iter();
    let first = values.next();
    if values.next().is_some() {
        return Err(E::Refused("web-header-ambiguous"));
    }
    Ok(first)
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Synthetic address-unit fixture only. The original actual-socket test failed
    /// with EAFNOSUPPORT and stays an open environment criterion in evidence06;
    /// this verifies formatting/auth logic, never IPv6 listening capability.
    #[test]
    fn ipv6_address_unit_keeps_brackets_cookie_scope_and_protected_origin() {
        let key = "session-0123456789abcdef01234567";
        let a =
            LocalWebAuthority::from_observed_address("[::1]:49152".parse().unwrap(), key).unwrap();
        assert_eq!(a.host(), "[::1]:49152");
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/action")
            .header(HOST, a.host())
            .header(COOKIE, format!("{}={key}", a.cookie_name()))
            .header(ORIGIN, "http://[::1]:49152")
            .header("x-swarm-csrf", key)
            .body(())
            .unwrap();
        a.authorize(&req).unwrap();
        let mut wrong = req;
        wrong
            .headers_mut()
            .insert(ORIGIN, "http://::1:49152".parse().unwrap());
        assert_eq!(
            a.authorize(&wrong).unwrap_err().code(),
            "web-origin-refused"
        );
    }
}

#[cfg(test)]
mod csrf_tests {
    use super::*;
    /// Compare domain-separated derivation against independent Python stdlib;
    /// scope changes change token, and Debug never prints either secret value.
    #[test]
    fn csrf_hmac_matches_independent_vector_and_is_scoped_not_bearer() {
        let key = "session-0123456789abcdef01234567";
        let a = LocalWebAuthority::from_observed_address("127.0.0.1:49152".parse().unwrap(), key)
            .unwrap();
        assert!(
            a.csrf_token() == "8a0d4d56a8d9a60d1627c6df3e8dbe099c0f79f98d3f59e8b5cad2a9865c825f"
        );
        let b = LocalWebAuthority::from_observed_address("127.0.0.1:49153".parse().unwrap(), key)
            .unwrap();
        assert!(a.csrf_token() != b.csrf_token());
        let c = LocalWebAuthority::from_observed_address(
            "127.0.0.1:49152".parse().unwrap(),
            "session-1123456789abcdef01234567",
        )
        .unwrap();
        assert!(a.csrf_token() != c.csrf_token());
        assert!(!a.matches(a.csrf_token().as_bytes()));
        let debug = format!("{a:?}");
        assert!(!debug.contains(key) && !debug.contains(a.csrf_token()));
    }
}
