//! Private enrollment and service-call stdin framing. No account, human, key or grant is
//! authenticated here. All owned byte storage zeroizes; caller/runtime/OS copies
//! and abrupt process termination are outside that erasure claim.
use std::{
    collections::TryReserveError,
    error::Error as StdError,
    fmt,
    io::{self, Read},
};
use swarm_engine::{check_command_input_size, CommandInputError, COMMAND_INPUT_MAX_BYTES};
use zeroize::Zeroizing;
/// CLI binary protocol version, not execution policy or an authorization token.
pub(super) const HEADER: &[u8] = b"swarm-personal-enrollment-v1\n";
/// Safe framing/read/allocation refusal. Typed sources retain original causes,
/// ordinary formatting never includes bytes, paths or nested IO error text.
pub enum Error {
    Format,
    Input(io::Error),
    Transport(CommandInputError),
    Allocation(TryReserveError),
}
impl Error {
    /// Stable public-safe category, no borrowed/owned secret data is formatted.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Format => "personal-frame-invalid",
            Self::Input(_) => "personal-input-io",
            Self::Transport(e) => e.code(),
            Self::Allocation(_) => "personal-input-allocation",
        }
    }
}
impl fmt::Display for Error {
    /// Redact even a private custom IO error while preserving it via source().
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "personal enrollment input: {}", self.code())
    }
}
impl fmt::Debug for Error {
    /// Same safe diagnostic as Display; never debug-print transport input.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Error {
    /// Actual allocation/IO/transport cause; caller must use safe error traversal.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Input(e) => Some(e),
            Self::Transport(e) => Some(e),
            Self::Allocation(e) => Some(e),
            _ => None,
        }
    }
}
/// Owned non-clonable/non-serializable frame. Exactly one allocation is reserved
/// before any secret is read, preventing secret-bearing reallocation leftovers.
/// Borrowed input/stdio buffers remain their owner's erasure responsibility.
pub(super) struct Frame {
    bytes: Zeroizing<Vec<u8>>,
    used: usize,
}
impl fmt::Debug for Frame {
    /// No bytes, size, installation identity or password is exposed.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PersonalEnrollmentFrame { <private> }")
    }
}
impl Frame {
    /// Read HEADER + raw32 installation proof + exact nonempty password through
    /// EOF. Reuses the existing native16MiB command transport bound, including
    /// header/proof bytes; no new password/operator cutoff or normalization.
    /// One sentinel byte detects oversize. Only interrupted byte reads continue
    /// as standard Read::read_to_end does: never restart an enrollment/effect.
    /// No watchdog/implicit close: caller retains its borrowed reader/descriptor.
    /// Allocation/partial-read/format/size refusals drop the guarded allocation.
    pub(super) fn read(reader: &mut dyn Read) -> Result<Self, Error> {
        let (bytes, used) = read_guarded(reader)?;
        if used <= HEADER.len() + 32 || &bytes[..HEADER.len()] != HEADER {
            return Err(Error::Format);
        }
        Ok(Self { bytes, used })
    }
    /// Borrow validated proof/password without another secret allocation/copy.
    /// This is untrusted knowledge input, not proof that installation matches.
    pub(super) fn parts(&self) -> Result<(&[u8; 32], &[u8]), Error> {
        let start = HEADER.len();
        let secret = (&self.bytes[start..start + 32])
            .try_into()
            .map_err(|_| Error::Format)?;
        Ok((secret, &self.bytes[start + 32..self.used]))
    }
}
/// Shared private command reader. Allocate/guard the existing16MiB+sentinel
/// transport buffer before the first read, retain its full initialized length
/// and actual used count. Partial read/size/allocation refusal zeroizes; only
/// standard interrupted reads continue. No business retry, watchdog or FD close.
fn read_guarded(reader: &mut dyn Read) -> Result<(Zeroizing<Vec<u8>>, usize), Error> {
    let capacity = COMMAND_INPUT_MAX_BYTES + 1;
    let mut bytes = Zeroizing::new(Vec::new());
    bytes
        .try_reserve_exact(capacity)
        .map_err(Error::Allocation)?;
    bytes.resize(capacity, 0);
    let mut used = 0;
    loop {
        let count = match reader.read(&mut bytes[used..]) {
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(Error::Input(e)),
        };
        if count == 0 {
            break;
        }
        used += count;
        check_command_input_size(&bytes[..used]).map_err(Error::Transport)?;
    }
    Ok((bytes, used))
}
/// Read a complete guarded service packet with the same existing command bound.
/// Trim only unused initialized zeros, without allocation/copy/growth; the actual
/// bytes remain guarded for service_request::decode. No framing/admission/IO other
/// than the borrowed reader, and its FD remains caller-owned on every outcome.
pub(super) fn read_packet(reader: &mut dyn Read) -> Result<Zeroizing<Vec<u8>>, Error> {
    let (mut bytes, used) = read_guarded(reader)?;
    bytes.truncate(used);
    Ok(bytes)
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    /// Independent fixture encoding of public protocol; bytes are synthetic.
    fn input() -> Vec<u8> {
        let mut p = b"swarm-personal-enrollment-v1\n".to_vec();
        p.extend([42; 32]);
        p.extend(b"exact\0password\n");
        p
    }
    /// Same valid witness before each framing/transport fault, no live secret.
    fn valid() {
        let f = Frame::read(&mut Cursor::new(input())).unwrap();
        let (s, p) = f.parts().unwrap();
        assert_eq!(s, &[42; 32]);
        assert_eq!(p, b"exact\0password\n");
    }
    #[test]
    fn personal_frame_exact_binary_borrow_and_redaction() {
        valid();
        let f = Frame::read(&mut Cursor::new(input())).unwrap();
        assert_eq!(format!("{f:?}"), "PersonalEnrollmentFrame { <private> }");
        assert_eq!(f.bytes.len(), COMMAND_INPUT_MAX_BYTES + 1);
        assert!(f.bytes.capacity() >= f.bytes.len());
        fn zeroizing<T: zeroize::ZeroizeOnDrop>() {}
        zeroizing::<Zeroizing<Vec<u8>>>();
    }
    #[test]
    fn personal_frame_truncated_header_proof_empty_password_refuse_then_recover() {
        let good = input();
        for mut raw in [
            vec![],
            HEADER.to_vec(),
            good[..HEADER.len() + 31].to_vec(),
            good[..HEADER.len() + 32].to_vec(),
            good.clone(),
        ] {
            valid();
            if raw.len() == good.len() {
                raw[0] ^= 1
            }
            let e = Frame::read(&mut Cursor::new(raw)).unwrap_err();
            assert_eq!(e.code(), "personal-frame-invalid");
            valid();
        }
    }
    struct Interrupted {
        first: bool,
        rest: Cursor<Vec<u8>>,
    }
    impl Read for Interrupted {
        /// Interrupt once before returning normal bytes, no operation is retried.
        fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
            if self.first {
                self.first = false;
                Err(io::ErrorKind::Interrupted.into())
            } else {
                self.rest.read(b)
            }
        }
    }
    struct Partial {
        first: bool,
    }
    impl Read for Partial {
        /// Return actual partial bytes, then private IO failure to check redaction.
        fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
            if self.first {
                self.first = false;
                b[..4].copy_from_slice(b"swar");
                Ok(4)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "PRIVATE_FRAME_IO_CONTENT",
                ))
            }
        }
    }
    #[test]
    fn personal_frame_partial_io_cause_and_interrupted_read_are_preserved() {
        valid();
        let e = Frame::read(&mut Partial { first: true }).unwrap_err();
        assert_eq!(e.code(), "personal-input-io");
        assert!(!format!("{e:?} {e}").contains("PRIVATE_FRAME_IO_CONTENT"));
        assert_eq!(
            e.source()
                .unwrap()
                .downcast_ref::<io::Error>()
                .unwrap()
                .kind(),
            io::ErrorKind::PermissionDenied
        );
        let f = Frame::read(&mut Interrupted {
            first: true,
            rest: Cursor::new(input()),
        })
        .unwrap();
        assert_eq!(f.parts().unwrap().1, b"exact\0password\n");
        valid();
    }
    struct Long {
        remaining: usize,
        consumed: usize,
    }
    impl Read for Long {
        /// Generate overlong transport bytes without another large fixture buffer.
        fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
            let n = b.len().min(self.remaining);
            b[..n].fill(42);
            self.remaining -= n;
            self.consumed += n;
            Ok(n)
        }
    }
    #[test]
    fn personal_frame_existing_transport_bound_stops_after_sentinel() {
        valid();
        let mut r = Long {
            remaining: COMMAND_INPUT_MAX_BYTES + 9,
            consumed: 0,
        };
        let e = Frame::read(&mut r).unwrap_err();
        assert_eq!(e.code(), "input-document-too-large");
        assert_eq!(r.consumed, COMMAND_INPUT_MAX_BYTES + 1);
        valid();
    }
}
