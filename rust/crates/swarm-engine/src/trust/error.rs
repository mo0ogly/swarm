//! Content-free trust failures with original codec causes retained.
use std::{error::Error, fmt};
/// Trust command refusal. This carries no token, raw document or claim of rollback.
pub enum TrustError {
    Invalid(&'static str),
    Authorization,
    Conflict(&'static str),
    Integrity(&'static str),
    Unsupported(&'static str),
    Json(serde_json::Error),
}
impl TrustError {
    /// Stable historical diagnostic family; do not classify by message text.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Authorization => "authorization_required",
            Self::Conflict(code) => code,
            Self::Integrity(_) => "trust-proof-integrity",
            Self::Unsupported(_) => "command_not_ported",
            Self::Invalid(_) | Self::Json(_) => "format",
        }
    }
    /// Safe static reason; private input and serde source messages stay hidden.
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Authorization => "identité non reconnue ou suspendue",
            Self::Invalid(reason)
            | Self::Conflict(reason)
            | Self::Integrity(reason)
            | Self::Unsupported(reason) => reason,
            Self::Json(_) => "document Actions de confiance invalide",
        }
    }
}
impl fmt::Display for TrustError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.reason())
    }
}
impl fmt::Debug for TrustError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for TrustError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Json(e) => Some(e),
            _ => None,
        }
    }
}
impl From<serde_json::Error> for TrustError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}
