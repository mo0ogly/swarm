//! Linux packet transport with kernel credential subjects and owned process handles.
//!
//! No listener, admission, mandate, human authentication or confinement is provided.
//! Privileged SCM_CREDENTIALS semantics still apply: admission must verify the
//! expected security domain and rights, not infer authority from this transport.
//! Only `kernel` contains explicit, reviewed Linux FFI. Other Swarm crates keep
//! their existing `forbid(unsafe_code)`; this crate denies unsafe outside that file.
#![cfg(target_os = "linux")]
#![deny(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]
#[allow(unsafe_code)]
mod kernel;
use std::{
    error::Error as StdError,
    fmt, io,
    os::fd::{AsFd, BorrowedFd, OwnedFd},
};
use zeroize::Zeroizing;

/// Refusal with all syscall/cleanup causes. Display/Debug omit private values and
/// IO text. `source` exposes the first cause; inspect `causes()` for every close
/// failure. No error proves rollback, peer receipt, admission or a business effect.
pub struct Error {
    code: &'static str,
    causes: Vec<(&'static str, io::Error)>,
}
impl Error {
    /// Static classification, safe for ordinary diagnostics.
    pub fn code(&self) -> &'static str {
        self.code
    }
    /// Complete original IO causes, including independent cleanup failures.
    pub fn causes(&self) -> &[(&'static str, io::Error)] {
        &self.causes
    }
    /// Build a refusal without inventing a syscall failure or errno zero.
    fn refused(code: &'static str) -> Self {
        Self {
            code,
            causes: Vec::new(),
        }
    }
    /// Preserve an actual syscall cause under a static, nonprivate phase label.
    fn syscall(code: &'static str, error: io::Error) -> Self {
        Self {
            code,
            causes: vec![(code, error)],
        }
    }
    /// Preserve each explicit close cause. Never retry close after EINTR on Linux.
    fn cleanup(&mut self, fd: OwnedFd, phase: &'static str) {
        if let Err(error) = nix::unistd::close(fd) {
            self.causes.push((phase, io::Error::from(error)))
        }
    }
}
impl fmt::Display for Error {
    /// Category only; detailed causes are explicitly available to trusted callers.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code)
    }
}
impl fmt::Debug for Error {
    /// Do not format arbitrary IO text, packet bytes, descriptor or credential data.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChannelError")
            .field("code", &self.code)
            .field("cause_count", &self.causes.len())
            .finish()
    }
}
impl StdError for Error {
    /// First original cause only; `causes` retains additional cleanup failures.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.causes
            .first()
            .map(|(_, e)| e as &(dyn StdError + 'static))
    }
}
/// Failed one-packet send. attempted records the native call, not delivery;
/// accepted_len retains an anomalous nonnegative short return only. None means
/// no accepted count observed, never proof of rollback or no business effect.
/// Caller retains socket and bytes; no retry, close or peer acknowledgement.
pub struct SendFailure {
    pub cause: Error,
    pub attempted: bool,
    pub accepted_len: Option<usize>,
}
impl fmt::Display for SendFailure {
    /// Static class/state only, never packet content, size, FD or private IO text.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}; attempted={}; accepted_count_known={}",
            self.cause.code(),
            self.attempted,
            self.accepted_len.is_some()
        )
    }
}
impl fmt::Debug for SendFailure {
    /// Same private-data redaction as Display; original typed cause stays owned.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for SendFailure {
    /// Original channel error; its causes retain every independent cleanup cause.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.cause)
    }
}
/// Configured borrowed socket. Aliases can change options; each IO rechecks them.
/// Socket lifetime/closure remains the caller's responsibility.
pub struct Channel<'a> {
    fd: BorrowedFd<'a>,
}
/// Complete guarded packet and a uniquely owned kernel credential-subject pidfd.
/// Not Clone/Deserialize and no constructor from caller-selected raw FD numbers.
/// Drop provides emergency FD cleanup only; call `close` to observe close errors.
pub struct Packet {
    buffer: Zeroizing<Vec<u8>>,
    used: usize,
    pid: i32,
    uid: u32,
    gid: u32,
    process: OwnedFd,
}
/// Owned namespace handles obtained through the held pidfd, without PID reopening.
/// They are observations, not proof of approved mounts or confinement.
pub struct Namespaces {
    mount: OwnedFd,
    pid: OwnedFd,
}
impl Packet {
    /// Exact received bytes; backing initialized capacity remains guarded to drop.
    pub fn data(&self) -> &[u8] {
        &self.buffer[..self.used]
    }
    /// Borrow the exact received allocation with its Zeroizing owner retained.
    /// Unused initialized receive bytes were wiped and trimmed without reallocating;
    /// no spare bytes become a password tail. The packet/pidfd remain alive while
    /// this borrow is used. Possession is not admission, and dropping a view does
    /// not erase the retained packet or any deliberate caller-created copies.
    pub fn guarded_data(&self) -> &Zeroizing<Vec<u8>> {
        &self.buffer
    }
    /// Kernel credential subject in the reader namespace, never a lookup key.
    pub fn credentials(&self) -> (i32, u32, u32) {
        (self.pid, self.uid, self.gid)
    }
    /// Borrow the held kernel process handle. No authority or current-liveness claim.
    pub fn process(&self) -> BorrowedFd<'_> {
        self.process.as_fd()
    }
    /// Query live subject namespaces by pidfd. Exited subject/unsupported syscall/
    /// permission refusals preserve cause and partial-handle cleanup errors.
    pub fn namespaces(&self) -> Result<Namespaces, Error> {
        kernel::namespaces(&self.process)
    }
    /// Consume packet, wipe guarded bytes and explicitly close the received pidfd.
    /// Linux close is never retried; success is not a peer or effect acknowledgement.
    pub fn close(self) -> Result<(), Error> {
        let Self {
            buffer, process, ..
        } = self;
        drop(buffer);
        let mut error = Error::refused("identity-channel-close");
        error.cleanup(process, "close-process");
        if error.causes.is_empty() {
            Ok(())
        } else {
            Err(error)
        }
    }
}
impl Namespaces {
    /// Compare exact mount and PID namespace objects while both owners retain
    /// their descriptors. No PID/path reopening, ownership transfer, fallback or
    /// mutation. `expected` must come from an independently approved protected
    /// source; this comparison never approves a first caller, image, confinement,
    /// human or current rights. False means a different domain; fstat failures
    /// retain their original cause and never become an empty successful match.
    /// Namespace handles retain the observed objects, not a freeze of the peer:
    /// the peer may subsequently exit or change namespaces. No currentness lease
    /// or prevention of setns/unshare is established by this point-in-time check.
    pub fn same_domain(&self, expected: &Namespaces) -> Result<bool, Error> {
        /// Compare kernel object identity on an already held descriptor only.
        /// Original fstat errors remain typed; no FD is closed or reopened.
        fn object(fd: BorrowedFd<'_>) -> Result<(u64, u64), Error> {
            nix::sys::stat::fstat(fd)
                .map(|s| (s.st_dev, s.st_ino))
                .map_err(|e| Error::syscall("identity-channel-namespace-stat", io::Error::from(e)))
        }
        let mount = object(self.mount())? == object(expected.mount())?;
        let pid = object(self.pid())? == object(expected.pid())?;
        Ok(mount && pid)
    }
    /// Borrow the mount namespace object, not a path chosen by the sender.
    pub fn mount(&self) -> BorrowedFd<'_> {
        self.mount.as_fd()
    }
    /// Borrow the PID namespace object. Namespace equality is not an authorization.
    pub fn pid(&self) -> BorrowedFd<'_> {
        self.pid.as_fd()
    }
    /// Consume both handles and preserve each close failure independently.
    pub fn close(self) -> Result<(), Error> {
        let mut error = Error::refused("identity-channel-close-namespaces");
        error.cleanup(self.mount, "close-mount-namespace");
        error.cleanup(self.pid, "close-pid-namespace");
        if error.causes.is_empty() {
            Ok(())
        } else {
            Err(error)
        }
    }
}
/// Require AF_UNIX/SOCK_SEQPACKET, enable both kernel credential options and verify
/// their effective values before returning. May partially change socket options
/// on failure; no configured token is returned. Caller must configure before
/// releasing a sender. No listener, timeout, retry or admission is created.
pub fn configure(fd: &impl AsFd) -> Result<Channel<'_>, Error> {
    kernel::configure(fd.as_fd())?;
    Ok(Channel { fd: fd.as_fd() })
}
impl Channel<'_> {
    /// Send one nonempty record with a single native send and per-call NOSIGNAL.
    /// Recheck socket kind/options before invocation; never attach caller controls,
    /// fragment, retry, flush, close, alter process signal policy or copy bytes.
    /// Blocking follows the supplied socket's effective flags; caller configures
    /// its own cancellation/limits before use. Kernel transport limits/errors stay
    /// distinct from operator policy. Success means local full-byte acceptance,
    /// not peer receipt, admission or business success. Failure retains invocation
    /// and any observed short accepted count; caller retains guarded input/socket.
    pub fn send_packet(&mut self, bytes: &[u8]) -> Result<(), SendFailure> {
        kernel::send::packet(self.fd, bytes)
    }

    /// Consume one packet into an initialized guarded caller-owned buffer. Its
    /// length is the transport capacity chosen by the caller, not a hidden policy.
    /// Empty/truncated/unattributed/unexpected controls refuse after explicit FD
    /// cleanup. Option/pre-read errors leave the queue untouched; read failures or
    /// post-read refusals may consume a packet. Never retry the request implicitly.
    pub fn receive(&mut self, buffer: Zeroizing<Vec<u8>>) -> Result<Packet, Error> {
        kernel::receive(self.fd, buffer)
    }
}

#[cfg(test)]
mod namespace_domain_tests;

/// A retained process object from atomic spawn or a conditional existing Child.
/// Non-Clone/non-Serialize; not a grant, image check, UID role, human or namespace
/// freeze. The caller independently approves the launch and prevents external
/// reaping during capture. Holding this FD never kills, waits for or restarts it.
pub struct ProcessAnchor {
    process: OwnedFd,
}
impl fmt::Debug for ProcessAnchor {
    /// Omit PID, inode, FD and launch details from ordinary diagnostics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ProcessAnchor { <private> }")
    }
}
impl ProcessAnchor {
    /// Capture a live, caller-owned Child before receiving an untrusted request.
    /// `try_wait` may observe/reap an ended child and refuses it. Caller must
    /// prevent SIGCHLD auto-reap and other threads/process APIs from reaping this
    /// child until pidfd_open completes; std Child alone is not OS confinement.
    /// No caller-selected/request PID is accepted. Reject old anonymous pidfds
    /// and incomplete32bit inode representation; no PID/path fallback or retry.
    /// All post-open refusals attempt explicit FD close and preserve its cause.
    /// The child remains caller-owned, with no signal/kill/restart by this API.
    pub fn capture_child(child: &mut std::process::Child) -> Result<Self, Error> {
        let process = kernel::child_process(child)?;
        let checked = kernel::process_identity(process.as_fd())
            .and_then(|_| kernel::process_live(process.as_fd()));
        if let Err(mut error) = checked {
            error.cleanup(process, "close-rejected-process-anchor");
            return Err(error);
        }
        Ok(Self { process })
    }
    /// Compare held pidfs objects to the actual kernel credential-subject pidfd,
    /// requiring the expected process not yet observed exited. No namespace or
    /// ptrace lookup, packet decode, FD transfer or payload copy. Same UID or
    /// connection creator never substitutes for object equality. Root/privileged
    /// SCM_CREDENTIALS semantics still apply; no current rights are granted.
    /// Exit/type/stat/poll errors preserve cause; false is a different subject.
    /// This point-in-time observation does not freeze exec, exit or namespaces.
    pub fn matches(&self, packet: &Packet) -> Result<bool, Error> {
        let expected = kernel::process_identity(self.process.as_fd())?;
        kernel::process_live(self.process.as_fd())?;
        let actual = kernel::process_identity(packet.process())?;
        Ok(expected == actual)
    }
    /// Borrow the retained pidfd, never a reusable numeric PID. Trusted callers
    /// own any additional transfer and approval; possession is not authorization.
    pub fn process(&self) -> BorrowedFd<'_> {
        self.process.as_fd()
    }
    /// Consume only the retained descriptor; never reap/kill the owned Child.
    /// Explicit close errors remain inspectable and are not retried on Linux.
    pub fn close(self) -> Result<(), Error> {
        let mut error = Error::refused("identity-channel-close-process-anchor");
        error.cleanup(self.process, "close-process-anchor");
        if error.causes.is_empty() {
            Ok(())
        } else {
            Err(error)
        }
    }
}

#[cfg(test)]
mod process_anchor_tests;

/// GNU atomic process descriptor creation; approval and confinement remain external.
#[cfg(target_env = "gnu")]
pub mod atomic_spawn;
