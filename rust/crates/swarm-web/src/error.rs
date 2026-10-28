//! Safe access error families; preserve parser/OS causes without reflecting credentials.
use std::{error::Error, fmt, io};
/// A refused request has no authorization or mutation effect. Raw header/cookie
/// contents are never included in Display/Debug; original typed causes survive.
pub enum WebAccessError {
    Csrf(hmac::digest::InvalidLength),
    Listener(io::Error),
    Header(http::header::ToStrError),
    Cookie(cookie::ParseError),
    Refused(&'static str),
}
impl WebAccessError {
    /// Stable public family only; no raw credential, target or header text.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Csrf(_) => "web-csrf-derive",
            Self::Listener(_) => "web-listener-inspect",
            Self::Header(_) => "web-header-invalid",
            Self::Cookie(_) => "web-cookie-invalid",
            Self::Refused(code) => code,
        }
    }
}
impl fmt::Display for WebAccessError {
    /// Public diagnostics contain only the safe family, not private request data.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for WebAccessError {
    /// Debug follows the same redaction contract as Display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for WebAccessError {
    /// Preserve the original OS/encoding/parser cause for typed internal recovery.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Csrf(e) => Some(e),
            Self::Listener(e) => Some(e),
            Self::Header(e) => Some(e),
            Self::Cookie(e) => Some(e),
            Self::Refused(_) => None,
        }
    }
}
