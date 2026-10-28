//! Test-only qualification component for the private Root-origin grant transport.
//! No production gate calls it yet. A transported binding is physical ownership,
//! never job permission, peer authentication, a serialized grant or OS confinement.
#![forbid(unsafe_code)]

use super::{finished, RootBinding, RootStore, RootStoreError as E};
use rustix::net::{RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags,
    SendAncillaryBuffer, SendAncillaryMessage, SendFlags};
use std::{error::Error, fmt, io::{IoSlice, IoSliceMut}, mem::MaybeUninit,
    os::{fd::OwnedFd, unix::net::UnixStream}, path::Path};

/// Wire version2 distinguishes both mixed-version directions. Two descriptors
/// represent directory and DB witness, not an operator concurrency/size policy.
const GRANT: u8 = 2;
const WITNESSES: usize = 2;

/// Known transport exposure, never execution/adoption or permission evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GrantExposure {
    NotAttempted,
    SendAttemptedUnknown,
    Queued,
    ReceiveAttemptedUnknown,
    Eof,
    Received,
}
/// Failure boundary before any product gate is wired to this component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Operation { Send, Receive, Adopt, Context }

/// Primary typed failure plus every explicitly observed close failure. Display
/// excludes Root paths, descriptor values and private runtime content. source()
/// exposes primary only; future diagnostic wiring MUST also traverse cleanup.
pub(super) struct GateRootChannelError {
    pub(super) operation: Operation,
    pub(super) exposure: GrantExposure,
    pub(super) primary: E,
    pub(super) cleanup: Vec<E>,
}
impl GateRootChannelError {
    /// Retain a refusal without inventing rollback, no-send or child adoption.
    fn new(operation: Operation, exposure: GrantExposure, primary: E) -> Self {
        Self { operation, exposure, primary, cleanup: Vec::new() }
    }
    /// Append an independent checked close; never discard the original error.
    fn closed(mut self, result: Result<(), E>) -> Self {
        if let Err(error) = result { self.cleanup.push(error); }
        self
    }
}
impl fmt::Display for GateRootChannelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "gate-root-channel:{:?}:{:?}", self.operation, self.exposure)
    }
}
impl fmt::Debug for GateRootChannelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { fmt::Display::fmt(self, f) }
}
impl Error for GateRootChannelError {
    fn source(&self) -> Option<&(dyn Error + 'static)> { Some(&self.primary) }
}
/// Retain an OS cause under a static safe family; no formatting of the cause.
fn caused(code: &'static str, error: impl Error + Send + Sync + 'static) -> E {
    E::Launch(Box::new(crate::AgentLaunchError::caused(code, error)))
}
/// Explicitly close every received descriptor, retaining each failure in order.
/// Receive/adopt is intended BEFORE any SQLite child open: a refused foreign
/// data FD must not close after that helper acquires POSIX SQLite locks.
fn close_received(mut error: GateRootChannelError, descriptors: Vec<OwnedFd>) -> GateRootChannelError {
    for descriptor in descriptors {
        error = error.closed(nix::unistd::close(descriptor)
            .map_err(|e| caused("gate-root-received-close", e)));
    }
    error
}

/// Queue version2 plus borrowed real directory/DB O_PATH witnesses on the private
/// Unix socket. Caller retains the binding through return and separately owes
/// actual job/peer/permission checks. Queued is NOT adopted or executed. Any
/// attempted syscall failure is conservatively unknown exposure, never retried.
/// No descriptor flags or ownership are changed and SIGPIPE is suppressed.
pub(super) fn send(channel: &UnixStream, binding: &RootBinding)
    -> Result<GrantExposure, GateRootChannelError>
{
    let descriptors = binding.gate_descriptors();
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(2))];
    let mut ancillary = SendAncillaryBuffer::new(&mut space);
    if !ancillary.push(SendAncillaryMessage::ScmRights(&descriptors)) {
        return Err(GateRootChannelError::new(Operation::Send, GrantExposure::NotAttempted,
            E::Refused("gate-root-ancillary-buffer")));
    }
    let payload = [GRANT];
    match rustix::net::sendmsg(channel, &[IoSlice::new(&payload)], &mut ancillary, SendFlags::NOSIGNAL) {
        Ok(1) => Ok(GrantExposure::Queued),
        Ok(_) => Err(GateRootChannelError::new(Operation::Send, GrantExposure::SendAttemptedUnknown,
            E::Refused("gate-root-send-incomplete"))),
        Err(error) => Err(GateRootChannelError::new(Operation::Send, GrantExposure::SendAttemptedUnknown,
            caused("gate-root-send", error))),
    }
}

/// Receive exactly byte2 with one SCM_RIGHTS message containing the directory
/// and DB witness, atomically CLOEXEC; validate/adopt BEFORE opening Root. Then
/// compare to the actual opened Root context and close that connection on every
/// path. Root's own open may read schema/settings or recover a journal before
/// comparison; no blanket zero-effect promise is made. No Job/material is read.
/// Caller must independently verify kernel peer/PDEATH and current permission.
/// Missing, extra, truncated, wrong-mode or legacy grants never fall back to a
/// pathname-only binding. All observed received descriptors close on refusal.
pub(super) fn receive(channel: &UnixStream, root: &Path)
    -> Result<RootBinding, GateRootChannelError>
{
    let mut bytes = [0u8; 2];
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(2))];
    let mut ancillary = RecvAncillaryBuffer::new(&mut space);
    let result = rustix::net::recvmsg(channel, &mut [IoSliceMut::new(&mut bytes)],
        &mut ancillary, RecvFlags::CMSG_CLOEXEC);
    let mut descriptors = Vec::new();
    let mut messages = 0;
    let mut unexpected = false;
    // Drain even if recvmsg failed: implicit buffer Drop cannot report close errors.
    for message in ancillary.drain() {
        match message {
            RecvAncillaryMessage::ScmRights(rights) => {
                messages += 1;
                descriptors.extend(rights);
            }
            _ => unexpected = true,
        }
    }
    let observed = match result {
        Ok(observed) => observed,
        Err(error) => return Err(close_received(GateRootChannelError::new(Operation::Receive,
            GrantExposure::ReceiveAttemptedUnknown, caused("gate-root-receive", error)), descriptors)),
    };
    let exposure = if observed.bytes == 0 { GrantExposure::Eof } else { GrantExposure::Received };
    if observed.bytes != 1 || bytes[0] != GRANT || messages != 1 || unexpected
        || descriptors.len() != WITNESSES || observed.flags.contains(ReturnFlags::CTRUNC)
    {
        return Err(close_received(GateRootChannelError::new(Operation::Receive,
            exposure, E::Refused("gate-root-grant-shape")), descriptors));
    }
    let pair: [OwnedFd; 2] = match descriptors.try_into() {
        Ok(pair) => pair,
        Err(descriptors) => return Err(close_received(GateRootChannelError::new(Operation::Receive,
            GrantExposure::Received, E::Refused("gate-root-grant-shape")), descriptors)),
    };
    let [directory, database] = pair;
    let binding = RootBinding::adopt_gate_descriptors(directory, database)
        .map_err(|error| GateRootChannelError::new(Operation::Adopt, GrantExposure::Received, error))?;
    let context = match RootStore::open(root, false) {
        Ok(store) => {
            let result = binding.matches(&store).and_then(|same| if same { Ok(()) }
                else { Err(E::Refused("gate-root-instance-changed")) });
            finished(result, store.close())
        }
        Err(error) => Err(error),
    };
    match context {
        Ok(()) => Ok(binding),
        Err(error) => Err(GateRootChannelError::new(Operation::Context,
            GrantExposure::Received, error).closed(binding.close())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, os::{fd::{AsFd, OwnedFd}, unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt}},
        path::PathBuf, process::{Command, Stdio}};

    /// Only a fresh privately owned component Root; preserve files on test panic.
    struct Fixture(PathBuf);
    impl Fixture {
        /// Initialize using the real Root API and close its SQLite connection.
        fn new() -> Self {
            let path = std::env::temp_dir().join(crate::generate_identifier("gate-channel100-").unwrap());
            fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
            RootStore::initialize(&path, &mut crate::SystemEnvironment).unwrap().close().unwrap();
            Self(path)
        }
        /// Capture actual original descriptors; the connection is closed before
        /// transport/type-fault tests, which never substitute SQLite data FDs.
        fn binding(&self) -> RootBinding {
            let store = RootStore::open(&self.0, false).unwrap();
            let binding = store.bind_root().unwrap();
            store.close().unwrap();
            binding
        }
    }
    impl Drop for Fixture {
        /// Remove only this owned fixture on success; retain failing state.
        fn drop(&mut self) {
            if !std::thread::panicking() { fs::remove_dir_all(&self.0).unwrap(); }
        }
    }
    /// Inject only protocol bytes/descriptors into an owned socket, never a Job
    /// or SQL permission. Oversized control space is fixture representation data.
    fn raw_send(socket: &UnixStream, bytes: &[u8], fds: &[std::os::fd::BorrowedFd<'_>]) {
        let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(4))];
        let mut ancillary = SendAncillaryBuffer::new(&mut space);
        if !fds.is_empty() { assert!(ancillary.push(SendAncillaryMessage::ScmRights(fds))); }
        assert_eq!(rustix::net::sendmsg(socket, &[IoSlice::new(bytes)], &mut ancillary,
            SendFlags::NOSIGNAL).unwrap(), bytes.len());
    }
    /// Count descriptors for these unique fixture inodes only. Other parallel
    /// tests' unrelated FDs cannot manufacture a leak/refusal result.
    fn owned_counts(identities: &[(u64, u64)]) -> Vec<usize> {
        let mut counts = vec![0; identities.len()];
        for entry in fs::read_dir("/proc/self/fd").unwrap() {
            match fs::metadata(entry.unwrap().path()) {
                Ok(metadata) => for (index, identity) in identities.iter().enumerate() {
                    if *identity == (metadata.dev(), metadata.ino()) { counts[index] += 1; }
                },
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
                Err(error) => panic!("fixture descriptor observation failed: {error}"),
            }
        }
        counts
    }

    /// Subprocess-only receiver. Marker confirms this is our inert component
    /// fixture, never authorization. It creates no Job, provider or acceptance.
    #[test]
    #[ignore = "selected only by owned gate-channel parent test"]
    fn gate_root_channel_owned_receiver_child() {
        let root = PathBuf::from(std::env::var_os("SWARM_GATE_ROOT_CHANNEL_FIXTURE").unwrap());
        assert_eq!(fs::read(root.join("gate-channel-fixture")).unwrap(), b"owned-inert-root-channel100");
        let socket = UnixStream::from(rustix::io::fcntl_dupfd_cloexec(std::io::stdin(), 3).unwrap());
        let binding = receive(&socket, &root).unwrap();
        let fds = binding.gate_descriptors();
        for fd in fds {
            assert!(rustix::io::fcntl_getfd(fd).unwrap().contains(rustix::io::FdFlags::CLOEXEC));
        }
        binding.close().unwrap();
        nix::unistd::close(socket).unwrap();
        println!("GATE_ROOT_CHANNEL_ACTUAL_CHILD_ADOPTED_AND_CLOSED");
    }

    /// Kernel-queued witnesses survive explicit closure of every parent binding
    /// and connection BEFORE the child even starts. Actual receive/adoption and
    /// checked close are observed from a separately executed test process; no
    /// label or send return alone is treated as adoption. No timing threshold.
    #[test]
    fn gate_root_channel_queued_lifetime_survives_parent_close_before_child() {
        let fixture = Fixture::new();
        fs::write(fixture.0.join("gate-channel-fixture"), b"owned-inert-root-channel100").unwrap();
        let binding = fixture.binding();
        let (sender, receiver) = UnixStream::pair().unwrap();
        assert_eq!(send(&sender, &binding).unwrap(), GrantExposure::Queued);
        binding.close().unwrap();
        nix::unistd::close(sender).unwrap();
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "root_store::gate_root_channel::tests::gate_root_channel_owned_receiver_child",
                "--ignored", "--nocapture"])
            .env("SWARM_GATE_ROOT_CHANNEL_FIXTURE", &fixture.0)
            .stdin(Stdio::from(OwnedFd::from(receiver))).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains("GATE_ROOT_CHANNEL_ACTUAL_CHILD_ADOPTED_AND_CLOSED"));
        assert!(stdout.contains("1 passed; 0 failed"));
    }

    /// Both protocol directions refuse: legacy byte1 cannot enter the new
    /// receiver; the existing old read1 predicate rejects our real byte2. Extra
    /// bytes/FDs, missing witnesses and EOF refuse and retain no received FD.
    #[test]
    fn gate_root_channel_rejects_mixed_versions_shapes_and_eof_without_leaks() {
        let fixture = Fixture::new();
        let binding = fixture.binding();
        let identity = binding.instance().unwrap();
        let identities = [identity.directory, identity.database];
        for (payload, count) in [(vec![1], 0), (vec![1], 2), (vec![2], 0),
            (vec![2], 1), (vec![2], 3), (vec![2, 2], 2)] {
            let before = owned_counts(&identities);
            let fds = binding.gate_descriptors();
            let extra = [fds[0], fds[1], fds[0]];
            let (sender, receiver) = UnixStream::pair().unwrap();
            raw_send(&sender, &payload, &extra[..count]);
            let error = receive(&receiver, &fixture.0).err().expect("shape must refuse");
            assert_eq!(error.operation, Operation::Receive);
            assert_eq!(error.exposure, GrantExposure::Received);
            assert!(error.cleanup.is_empty());
            assert_eq!(owned_counts(&identities), before);
            nix::unistd::close(sender).unwrap();
            nix::unistd::close(receiver).unwrap();
        }
        let (sender, receiver) = UnixStream::pair().unwrap();
        send(&sender, &binding).unwrap();
        let mut byte = [0];
        use std::io::Read;
        (&receiver).read_exact(&mut byte).unwrap();
        assert_ne!(byte, [1], "old consumer must refuse new parent protocol");
        nix::unistd::close(sender).unwrap();
        nix::unistd::close(receiver).unwrap();
        let (sender, receiver) = UnixStream::pair().unwrap();
        nix::unistd::close(sender).unwrap();
        let error = receive(&receiver, &fixture.0).err().unwrap();
        assert_eq!(error.exposure, GrantExposure::Eof);
        nix::unistd::close(receiver).unwrap();
        binding.close().unwrap();
    }

    /// Received ordinary data file and swapped descriptor roles refuse before
    /// any context SQLite open. The invalid file is unrelated to SQLite locks;
    /// every received handle is closed while sender ownership remains intact.
    #[test]
    fn gate_root_channel_rejects_wrong_descriptor_modes_and_retains_sender_ownership() {
        let fixture = Fixture::new();
        let binding = fixture.binding();
        let file = fs::OpenOptions::new().read(true).write(true).create_new(true).mode(0o600)
            .open(fixture.0.join("unrelated-file")).unwrap();
        let metadata = file.metadata().unwrap();
        let instance = binding.instance().unwrap();
        let identities = [instance.directory, instance.database, (metadata.dev(), metadata.ino())];
        for case in [0, 1, 2] {
            let fds = binding.gate_descriptors();
            let wrong = match case {
                0 => [fds[0], file.as_fd()],
                1 => [fds[1], fds[0]],
                _ => [file.as_fd(), fds[1]],
            };
            let before = owned_counts(&identities);
            let (sender, receiver) = UnixStream::pair().unwrap();
            raw_send(&sender, &[GRANT], &wrong);
            let error = receive(&receiver, &fixture.0).err().unwrap();
            assert_eq!(error.operation, Operation::Adopt);
            assert!(std::error::Error::source(&error).is_some());
            assert_eq!(owned_counts(&identities), before);
            nix::unistd::close(sender).unwrap();
            nix::unistd::close(receiver).unwrap();
        }
        nix::unistd::close(file).unwrap();
        binding.close().unwrap();
    }

    /// Control succeeds before a closed, coherently copied DB-only substitution.
    /// Different physical DB refuses, then restoring original inode succeeds;
    /// copy never creates a Job, grant row or SQL authority.
    #[test]
    fn gate_root_channel_refuses_database_replacement_and_recovers() {
        let fixture = Fixture::new();
        let binding = fixture.binding();
        let exchange = || {
            let (sender, receiver) = UnixStream::pair().unwrap();
            send(&sender, &binding).unwrap();
            let result = receive(&receiver, &fixture.0);
            nix::unistd::close(sender).unwrap();
            nix::unistd::close(receiver).unwrap();
            result
        };
        exchange().unwrap().close().unwrap();
        let path = fixture.0.join(".swarm/state.db");
        let copy = fixture.0.join(".swarm/state-copy.db");
        let original = fixture.0.join(".swarm/state-original.db");
        fs::copy(&path, &copy).unwrap();
        fs::rename(&path, &original).unwrap();
        fs::rename(&copy, &path).unwrap();
        let error = exchange().err().expect("different connected DB must refuse");
        assert_eq!(error.operation, Operation::Context);
        assert_eq!(error.primary.code(), "gate-root-instance-changed");
        fs::rename(&path, &copy).unwrap();
        fs::rename(&original, &path).unwrap();
        exchange().unwrap().close().unwrap();
        binding.close().unwrap();
    }

    /// A real closed peer causes typed uncertain-send exposure without SIGPIPE.
    /// The pure aggregation case separately checks primary and secondary close
    /// preservation; it is not described as an injected OS close failure.
    #[test]
    fn gate_root_channel_send_failure_and_cleanup_keep_known_observations() {
        let fixture = Fixture::new();
        let binding = fixture.binding();
        let (sender, receiver) = UnixStream::pair().unwrap();
        nix::unistd::close(receiver).unwrap();
        let error = send(&sender, &binding).err().unwrap();
        assert_eq!(error.operation, Operation::Send);
        assert_eq!(error.exposure, GrantExposure::SendAttemptedUnknown);
        assert!(error.source().is_some());
        assert!(!format!("{error:?}").contains(&fixture.0.to_string_lossy().to_string()));
        let error = GateRootChannelError::new(Operation::Receive, GrantExposure::Received,
            E::Refused("fixture-primary")).closed(Err(E::Refused("fixture-close-one")))
            .closed(Err(E::Refused("fixture-close-two")));
        assert_eq!(error.primary.code(), "fixture-primary");
        assert_eq!(error.cleanup.len(), 2);
        assert_eq!(error.cleanup[0].code(), "fixture-close-one");
        assert_eq!(error.cleanup[1].code(), "fixture-close-two");
        nix::unistd::close(sender).unwrap();
        binding.close().unwrap();
    }

    /// Probe the sender's lock from an independent SQLite process without DML,
    /// timeout sleeps or retries. BUSY must be the actual SQLite numeric code.
    fn independent_lock_probe(path: &Path) -> String {
        let script = "import sqlite3,sys\nc=sqlite3.connect(sys.argv[1],timeout=0)\ntry:\n c.execute('BEGIN IMMEDIATE')\nexcept sqlite3.OperationalError as e:\n assert e.sqlite_errorcode==sqlite3.SQLITE_BUSY,e\n print('busy')\nelse:\n c.rollback()\n print('available')\nc.close()\n";
        let output = Command::new("python3").args(["-c", script]).arg(path).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    /// A sender holds a real SQLite write reservation. A different process
    /// receives/adopts/closes both witnesses and its own read-only context;
    /// independent writers remain BUSY before/after until sender rollback.
    /// This is lock evidence, distinct from queue/adoption protocol observations.
    #[test]
    fn gate_root_channel_transfer_and_close_preserve_sender_sqlite_locks() {
        let fixture = Fixture::new();
        fs::write(fixture.0.join("gate-channel-fixture"), b"owned-inert-root-channel100").unwrap();
        let store = RootStore::open(&fixture.0, true).unwrap();
        let binding = store.bind_root().unwrap();
        store.db.execute_batch("BEGIN IMMEDIATE").unwrap();
        let database = fixture.0.join(".swarm/state.db");
        assert_eq!(independent_lock_probe(&database), "busy");
        println!("GATE_ROOT_CHANNEL_LOCK before=busy");
        let (sender, receiver) = UnixStream::pair().unwrap();
        send(&sender, &binding).unwrap();
        binding.close().unwrap();
        nix::unistd::close(sender).unwrap();
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "root_store::gate_root_channel::tests::gate_root_channel_owned_receiver_child",
                "--ignored", "--nocapture"])
            .env("SWARM_GATE_ROOT_CHANNEL_FIXTURE", &fixture.0)
            .stdin(Stdio::from(OwnedFd::from(receiver))).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert!(String::from_utf8(output.stdout).unwrap().contains("GATE_ROOT_CHANNEL_ACTUAL_CHILD_ADOPTED_AND_CLOSED"));
        assert_eq!(independent_lock_probe(&database), "busy");
        println!("GATE_ROOT_CHANNEL_LOCK after=busy");
        store.db.execute_batch("ROLLBACK").unwrap();
        assert_eq!(independent_lock_probe(&database), "available");
        println!("GATE_ROOT_CHANNEL_LOCK released=available");
        store.close().unwrap();
    }

}
