//! Retained-domain binding for one real kernel packet, not a listener/approver.
use super::{ControllerError, Error as DispatchError, Reply};
use crate::service_request::{self, Request};
use std::{error::Error as StdError, fmt};
use swarm_linux_identity_channel::{self as channel, Namespaces, Packet};

/// Earliest observed pre-dispatch or dispatch refusal. Namespace equality is an
/// observation against caller-approved handles, never approval of that source.
pub enum Cause {
    Controller(ControllerError),
    Process(channel::Error),
    ProcessMismatch,
    Domain(channel::Error),
    DomainMismatch,
    Request(service_request::Error),
    Dispatch(DispatchError),
}
impl Cause {
    /// Static safe class; no packet, UID, pin, namespace, path or private IO text.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Controller(e) => e.code(),
            Self::Process(e) | Self::Domain(e) => e.code(),
            Self::ProcessMismatch => "service-controller-process",
            Self::DomainMismatch => "service-controller-domain",
            Self::Request(e) => e.code(),
            Self::Dispatch(e) => e.code(),
        }
    }
}
impl fmt::Display for Cause {
    /// Redact original private diagnostics; typed causes remain inspectable.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for Cause {
    /// Same private-data redaction as Display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Cause {
    /// Original primary; channel causes also retain independent partial closes.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Controller(e) => Some(e),
            Self::Process(e) | Self::Domain(e) => Some(e),
            Self::Request(e) => Some(e),
            Self::Dispatch(e) => Some(e),
            Self::ProcessMismatch | Self::DomainMismatch => None,
        }
    }
}
/// Failure retains primary refusal, independent namespace cleanup and complete
/// known Reply after dispatch. None reply means no reply produced, not rollback.
/// Caller still owns Packet/expected-anchor closes; they are not attempted here.
pub struct Error {
    pub cause: Option<Cause>,
    pub cleanup: Option<channel::Error>,
    pub reply: Option<Reply>,
}
impl Error {
    /// All immediate failure branches, including independent cleanup and known
    /// operation errors retained in Reply. Callers follow each source/typed
    /// cleanup list and avoid logging private cause text. AfterUse's inner error
    /// is retained alongside its authority error, never inferred from source().
    pub fn error_branches(&self) -> Vec<&(dyn StdError + 'static)> {
        let mut out: Vec<&(dyn StdError + 'static)> = Vec::new();
        if let Some(e) = &self.cause {
            out.push(e)
        }
        if let Some(e) = &self.cleanup {
            out.push(e)
        }
        if let Some(reply) = &self.reply {
            out.extend(reply.error_branches());
        }
        out
    }
    /// Static primary class, or explicit late namespace-close failure.
    pub fn code(&self) -> &'static str {
        self.cause
            .as_ref()
            .map(Cause::code)
            .unwrap_or("service-controller-close")
    }
}
impl fmt::Display for Error {
    /// Only safe phase facts; known Reply and private cause text are never printed.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}; reply_known={}; namespace_close_failed={}",
            self.code(),
            self.reply.is_some(),
            self.cleanup.is_some()
        )
    }
}
impl fmt::Debug for Error {
    /// Same redaction as Display, including operation-after-use failures.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Error {
    /// Primary or cleanup only; inspect cause, cleanup and reply independently.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        if let Some(e) = &self.cause {
            Some(e)
        } else {
            self.cleanup.as_ref().map(|e| e as _)
        }
    }
}
/// Merge known outcome with explicit namespace close without erasing either.
/// A business failure can already be inside Reply; that Reply is retained intact.
fn finish(result: Result<Reply, Cause>, cleanup: Option<channel::Error>) -> Result<Reply, Error> {
    match (result, cleanup) {
        (Ok(reply), None) => Ok(reply),
        (Ok(reply), cleanup) => Err(Error {
            cause: None,
            cleanup,
            reply: Some(reply),
        }),
        (Err(cause), cleanup) => Err(Error {
            cause: Some(cause),
            cleanup,
            reply: None,
        }),
    }
}
/// Sole production caller passes its protected startup UID and real dispatcher.
/// Private callback seam tests ordering only; no configurable production bypass.
pub(super) fn run<'a>(
    uid: u32,
    packet: &'a Packet,
    expected: &Namespaces,
    dispatch: impl FnOnce(Request<'a>) -> Result<Reply, DispatchError>,
) -> Result<Reply, Error> {
    if packet.credentials().1 != uid {
        return Err(Error {
            cause: Some(Cause::Controller(ControllerError::Uid)),
            cleanup: None,
            reply: None,
        });
    }
    let observed = packet.namespaces().map_err(|e| Error {
        cause: Some(Cause::Controller(ControllerError::Namespaces(e))),
        cleanup: None,
        reply: None,
    })?;
    let result = (|| {
        if !observed.same_domain(expected).map_err(Cause::Domain)? {
            return Err(Cause::DomainMismatch);
        }
        let request = service_request::decode(packet.guarded_data()).map_err(Cause::Request)?;
        dispatch(request).map_err(Cause::Dispatch)
    })();
    finish(result, observed.close().err())
}

/// Strict additional process binding, with the bootstrap UID checked before
/// touching even an expired expected process. The expected handle must come
/// from an independently approved owned-child capture, not this request. Borrow
/// both anchor owners and Packet through use; caller closes them after return.
/// Matching does not bypass ANY namespace/decode/target/current-authority step
/// of run(), nor grant image/human/confinement rights. No new retry or listener.
pub(super) fn run_spawned<'a>(
    uid: u32,
    packet: &'a Packet,
    process: &channel::ProcessAnchor,
    expected: &Namespaces,
    dispatch: impl FnOnce(Request<'a>) -> Result<Reply, DispatchError>,
) -> Result<Reply, Error> {
    if packet.credentials().1 != uid {
        return Err(Error {
            cause: Some(Cause::Controller(ControllerError::Uid)),
            cleanup: None,
            reply: None,
        });
    }
    let matched = process.matches(packet).map_err(|e| Error {
        cause: Some(Cause::Process(e)),
        cleanup: None,
        reply: None,
    })?;
    if !matched {
        return Err(Error {
            cause: Some(Cause::ProcessMismatch),
            cleanup: None,
            reply: None,
        });
    }
    // The repeated immutable UID comparison in run() is deliberate: this wrapper
    // retains the entire existing namespace/decode/dispatch/cleanup path unchanged.
    run(uid, packet, expected, dispatch)
}

/// Only the atomic owner type enters this route; no PID capture or conditional
/// anchor fallback. Retain its borrow while run_spawned enforces every existing
/// UID/process/namespace/decode/target/current operation and known-error step.
/// External launch authorization/credentials/policy remain preconditions.
#[cfg(target_env = "gnu")]
pub(super) fn run_atomic<'a>(
    uid: u32,
    packet: &'a Packet,
    controller: &channel::atomic_spawn::Spawned,
    expected: &Namespaces,
    dispatch: impl FnOnce(Request<'a>) -> Result<Reply, DispatchError>,
) -> Result<Reply, Error> {
    run_spawned(uid, packet, controller.anchor(), expected, dispatch)
}

#[cfg(test)]
mod tests;
