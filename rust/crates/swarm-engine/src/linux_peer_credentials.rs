//! Linux connection credentials with raw integer representation until validation.
//! Not per-message attribution, human identity, a grant or confinement evidence.
use nix::{
    errno::Errno,
    sys::socket::{getsockopt, sockopt::PeerCredentials, UnixCredentials},
};
use std::{error::Error as StdError, fmt, os::fd::AsFd};
/// Safe kernel-read/representation refusal. No private PID/UID/path is formatted.
pub(crate) enum Error {
    Read(Errno),
    Invalid,
}
impl Error {
    /// Static diagnostic only; invalid observations never become a chosen PID.
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::Read(_) => "linux-peer-credentials-read",
            Self::Invalid => "linux-peer-credentials-invalid",
        }
    }
    /// Original OS errno only for a real failed syscall; invalid data is not errno0.
    pub(crate) fn os_errno(&self) -> Option<i32> {
        match self {
            Self::Read(e) => Some(*e as i32),
            Self::Invalid => None,
        }
    }
}
impl fmt::Display for Error {
    /// Omit credential values and nested error text from ordinary diagnostics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for Error {
    /// Same safe category as Display, preserving typed read causes separately.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Error {
    /// Actual failed syscall cause; invalid raw representation has no OS failure.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Read(e) => Some(e),
            Self::Invalid => None,
        }
    }
}
/// Read SO_PEERCRED through Nix's libc::ucred raw integers, then reject invisible/
/// absent/nonpositive PID or invalid UID/GID sentinel before any strong PID type,
/// grant read or Root operation. Borrow the actual socket FD, open/close nothing.
/// Valid connection credentials stay data only: caller still verifies expected
/// parent/source/UID/PDEATH/current rights. This is not SCM_CREDENTIALS/SCM_PIDFD,
/// a process lifetime/image guarantee or a fallback to a PID from a request.
pub(crate) fn read<F: AsFd>(socket: &F) -> Result<UnixCredentials, Error> {
    let credentials = getsockopt(socket, PeerCredentials).map_err(Error::Read)?;
    if credentials.pid() <= 0 || credentials.uid() == u32::MAX || credentials.gid() == u32::MAX {
        return Err(Error::Invalid);
    }
    Ok(credentials)
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, os::unix::net::UnixStream};
    /// Actual connected witness, no untrusted PID or identity supplied.
    fn valid() {
        let (a, _b) = UnixStream::pair().unwrap();
        let c = read(&a).unwrap();
        assert_eq!(c.pid() as u32, std::process::id());
        assert_eq!(c.uid(), rustix::process::geteuid().as_raw());
        assert_eq!(c.gid(), rustix::process::getegid().as_raw());
    }
    /// Real connected credentials match current process, without granting authority.
    #[test]
    fn connected_peer_raw_credentials_are_validated_without_reinterpretation() {
        valid()
    }
    /// Linux returns raw PID0 for a fresh unconnected Unix socket. Safe Nix fields
    /// can represent this; the common boundary rejects it before a strong PID.
    #[test]
    fn unconnected_pid_zero_refuses_without_errno_zero_then_connected_recovers() {
        use nix::sys::socket::{socket, AddressFamily, SockFlag, SockType};
        valid();
        let s = socket(
            AddressFamily::Unix,
            SockType::Stream,
            SockFlag::SOCK_CLOEXEC,
            None,
        )
        .unwrap();
        let raw = getsockopt(&s, PeerCredentials).unwrap();
        assert_eq!(raw.pid(), 0);
        let e = read(&s).unwrap_err();
        assert_eq!(e.code(), "linux-peer-credentials-invalid");
        assert_eq!(e.os_errno(), None);
        assert!(e.source().is_none());
        nix::unistd::close(s).unwrap();
        valid();
    }
    /// Wrong FD preserves the actual ENOTSOCK cause and safe formatting, then a
    /// connected witness still succeeds; no arbitrary descriptor is closed.
    #[test]
    fn non_socket_refusal_keeps_actual_errno_and_does_not_change_owner() {
        valid();
        let f = File::open("/dev/null").unwrap();
        let e = read(&f).unwrap_err();
        assert_eq!(e.os_errno(), Some(Errno::ENOTSOCK as i32));
        assert!(e.source().is_some());
        assert_eq!(format!("{e:?}"), "linux-peer-credentials-read");
        assert!(f.metadata().is_ok());
        nix::unistd::close(f).unwrap();
        valid();
    }
    /// Safe diagnostic traversal must retain invalid-data classification without
    /// errno0, and the actual ENOTSOCK errno through the custom IO wrapper.
    #[test]
    fn peer_diagnostics_keep_raw_invalid_separate_from_syscall_failure() {
        valid();
        let direct = crate::native_error_links(&Error::Invalid);
        assert!(direct
            .labels
            .iter()
            .any(|label| label == "linux-peer-credentials-invalid"));
        assert_eq!(direct.io_errno, None);
        let wrapped = std::io::Error::other(Error::Read(Errno::ENOTSOCK));
        let outer = crate::native_error_links(&wrapped);
        assert!(!outer.children.is_empty());
        let found = outer
            .children
            .iter()
            .map(|(_, child)| crate::native_error_links(*child))
            .any(|link| {
                link.io_errno == Some(Errno::ENOTSOCK as i32)
                    && link
                        .labels
                        .iter()
                        .any(|label| label == "linux-peer-credentials-read")
            });
        assert!(found, "custom IO must retain actual credential read errno");
        valid();
    }
}
