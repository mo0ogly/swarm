//! One controlled packet exchange, not a listener, peer approval or retry loop.
use super::{controller_packet, Reply};
use crate::service_response::{self as response, PacketOutputCause};
use std::{error::Error as StdError, fmt};
use swarm_linux_identity_channel as channel;
use zeroize::Zeroizing;

/// First refusal and its original owned data. PacketClose is necessarily after
/// dispatch produced a Reply; output failure retains its own Reply/observation.
pub enum Cause {
    Receive(channel::Error),
    Dispatch(controller_packet::Error),
    PacketClose { cause: channel::Error, reply: Reply },
    Output(response::PacketSendError),
}
impl Cause {
    /// Safe static cause/stage only; no packet, identity, path or private IO text.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Receive(e) => e.code(),
            Self::Dispatch(e) => e.code(),
            Self::PacketClose { .. } => "service-exchange-packet-close",
            Self::Output(e) => match &e.cause {
                PacketOutputCause::Encode(_) => "service-response-encoding",
                PacketOutputCause::Send(e) => e.cause.code(),
            },
        }
    }
}
impl fmt::Display for Cause {
    /// Redact original private diagnostics; typed branches remain inspectable.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for Cause {
    /// Same redaction as Display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Cause {
    /// Primary original only; Error::error_branches exposes independent results.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(match self {
            Self::Receive(e) => e,
            Self::Dispatch(e) => e,
            Self::PacketClose { cause, .. } => cause,
            Self::Output(e) => e,
        })
    }
}
/// Retain first refusal plus an independent packet-close failure after dispatch
/// refusal. No clone/serialization or rollback. Caller retains channel/controller/
/// expected namespaces and owns their later cleanup; only received packet closes.
pub struct Error {
    pub cause: Cause,
    pub packet_close: Option<channel::Error>,
}
impl Error {
    /// Static safe code; no frame or nested private diagnostics.
    pub fn code(&self) -> &'static str {
        self.cause.code()
    }
    /// Borrow the known original operation result wherever the failure owns it.
    /// None means no result produced, never proof of no effect or permission to retry.
    pub fn known_reply(&self) -> Option<&Reply> {
        match &self.cause {
            Cause::Dispatch(e) => e.reply.as_ref(),
            Cause::PacketClose { reply, .. } => Some(reply),
            Cause::Output(e) => Some(&e.reply),
            Cause::Receive(_) => None,
        }
    }
    /// Every immediate primary, independent cleanup and known business branch.
    /// Traverse each typed/source chain, including channel cleanup/custom IO; Source
    /// alone never replaces Reply's independent AfterUse inner result.
    pub fn error_branches(&self) -> Vec<&(dyn StdError + 'static)> {
        let mut out: Vec<&(dyn StdError + 'static)> = match &self.cause {
            Cause::Receive(e) => vec![e],
            Cause::Dispatch(e) => e.error_branches(),
            Cause::PacketClose { cause, reply } => {
                let mut v: Vec<&(dyn StdError + 'static)> = vec![cause];
                v.extend(reply.error_branches());
                v
            }
            Cause::Output(e) => {
                let mut v: Vec<&(dyn StdError + 'static)> = vec![&e.cause];
                v.extend(e.reply.error_branches());
                v
            }
        };
        if let Some(e) = &self.packet_close {
            out.push(e)
        }
        out
    }
}
impl fmt::Display for Error {
    /// Safe state flags only; no known response, source identity or nested IO text.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}; known_reply={}; packet_close_failed={}",
            self.code(),
            self.known_reply().is_some(),
            self.packet_close.is_some()
        )
    }
}
impl fmt::Debug for Error {
    /// Same private-data redaction as Display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Error {
    /// First cause only; use error_branches for all independent failures.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.cause)
    }
}
/// Merge complete dispatch and explicit close before any output. Synthetic private
/// tests inject close results here; production passes only actual Packet::close.
/// Never send after either refusal, even when a business result already exists.
fn finish(
    result: Result<Reply, controller_packet::Error>,
    closed: Result<(), channel::Error>,
    send: impl FnOnce(Reply) -> Result<response::Sent, response::PacketSendError>,
) -> Result<response::Sent, Error> {
    let reply = match result {
        Ok(reply) => reply,
        Err(cause) => {
            return Err(Error {
                cause: Cause::Dispatch(cause),
                packet_close: closed.err(),
            })
        }
    };
    if let Err(cause) = closed {
        return Err(Error {
            cause: Cause::PacketClose { cause, reply },
            packet_close: None,
        });
    }
    send(reply).map_err(|cause| Error {
        cause: Cause::Output(cause),
        packet_close: None,
    })
}
/// Receive exactly one guarded frame, run the fixed production dispatch callback,
/// always attempt its Packet close, then merge before one send. Receive refusal
/// never invokes dispatch. Private callback seam is not a public admission bypass.
pub(super) fn run(
    channel: &mut channel::Channel<'_>,
    buffer: Zeroizing<Vec<u8>>,
    dispatch: impl FnOnce(&channel::Packet) -> Result<Reply, controller_packet::Error>,
) -> Result<response::Sent, Error> {
    let packet = channel.receive(buffer).map_err(|cause| Error {
        cause: Cause::Receive(cause),
        packet_close: None,
    })?;
    let result = dispatch(&packet);
    let closed = packet.close();
    finish(result, closed, |reply| {
        response::send_packet(channel, reply)
    })
}

#[cfg(test)]
mod tests;
