#![cfg(target_os = "linux")]
#![forbid(unsafe_code)]
//! Real kernel witnesses. No listener, provider, grant, Root or live datastore.
use nix::{
    fcntl::{fcntl, FcntlArg, FdFlag},
    sys::socket::{
        send, sendmsg, setsockopt, socketpair, sockopt, AddressFamily, ControlMessage, MsgFlags,
        SockFlag, SockType, UnixCredentials,
    },
    sys::stat::fstat,
};
use std::{
    fs,
    io::{IoSlice, Write},
    os::{
        fd::{AsRawFd, OwnedFd},
        unix::fs::MetadataExt,
    },
    process::{Command, Stdio},
    sync::Mutex,
};
use swarm_linux_identity_channel::{configure, Channel};
use zeroize::Zeroizing;
static SERIAL: Mutex<()> = Mutex::new(());
/// Count current process descriptors; serialize all tests so another fixture
/// cannot hide a leak. The read_dir iterator is closed before returning.
fn descriptors() -> usize {
    fs::read_dir("/proc/self/fd").unwrap().count()
}
/// Owned private socketpair, CLOEXEC from creation; no listener or shared state.
fn pair() -> (OwnedFd, OwnedFd) {
    socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    )
    .unwrap()
}
/// Same-channel witness/recovery: exact content/subject, pidfd and namespace
/// CLOEXEC, actual namespace inode equality, explicit closes and no FD leak.
fn witness(sender: &OwnedFd, channel: &mut Channel<'_>) {
    let count = descriptors();
    send(sender.as_raw_fd(), b"witness", MsgFlags::empty()).unwrap();
    let packet = channel
        .receive(Zeroizing::new(vec![0; 64]))
        .expect("attributed packet required");
    assert_eq!(packet.data(), b"witness");
    assert_eq!(
        packet.credentials(),
        (
            std::process::id() as i32,
            nix::unistd::geteuid().as_raw(),
            nix::unistd::getegid().as_raw()
        )
    );
    assert_ne!(
        fcntl(packet.process(), FcntlArg::F_GETFD).unwrap() & libc::FD_CLOEXEC,
        0
    );
    let namespaces = packet.namespaces().expect("live subject namespace handles");
    for (handle, path) in [
        (namespaces.mount(), "/proc/self/ns/mnt"),
        (namespaces.pid(), "/proc/self/ns/pid"),
    ] {
        assert_ne!(
            fcntl(handle, FcntlArg::F_GETFD).unwrap() & libc::FD_CLOEXEC,
            0
        );
        let actual = fstat(handle).unwrap();
        let expected = fs::metadata(path).unwrap();
        assert_eq!(
            (actual.st_dev, actual.st_ino),
            (expected.dev(), expected.ino())
        );
    }
    namespaces.close().expect("explicit namespace closes");
    packet.close().expect("explicit pidfd close");
    assert_eq!(descriptors(), count);
}
/// Complete packet witness with kernel credentials and explicit handle cleanup.
#[test]
fn same_process_packet_has_kernel_credentials_and_explicit_close() {
    let _guard = SERIAL.lock().unwrap();
    let (sender, receiver) = pair();
    let mut channel = configure(&receiver).unwrap();
    witness(&sender, &mut channel);
}
/// Wrong socket types and a regular file refuse before configuration/receive;
/// real ENOTSOCK remains available without exposing its text in Display/Debug.
#[test]
fn wrong_socket_and_non_socket_refuse_then_valid_recovers() {
    let _guard = SERIAL.lock().unwrap();
    let (sender, receiver) = pair();
    let mut channel = configure(&receiver).unwrap();
    witness(&sender, &mut channel);
    for kind in [SockType::Stream, SockType::Datagram] {
        let (a, _b) = socketpair(AddressFamily::Unix, kind, None, SockFlag::SOCK_CLOEXEC).unwrap();
        assert!(configure(&a).err().unwrap().causes().is_empty());
    }
    let file = fs::File::open("/dev/null").unwrap();
    let error = configure(&file).err().unwrap();
    assert_eq!(error.causes()[0].1.raw_os_error(), Some(libc::ENOTSOCK));
    assert!(!format!("{error:?}").contains("/dev/null"));
    assert!(file.metadata().is_ok());
    witness(&sender, &mut channel);
}
/// Refuse an empty receive buffer before dequeue; subsequent valid receive must
/// retain the queued exact packet, not silently drop it on pre-read failure.
#[test]
fn empty_buffer_refuses_without_consuming_queued_packet() {
    let _guard = SERIAL.lock().unwrap();
    let (sender, receiver) = pair();
    let mut channel = configure(&receiver).unwrap();
    witness(&sender, &mut channel);
    send(sender.as_raw_fd(), b"queued", MsgFlags::empty()).unwrap();
    let error = channel.receive(Zeroizing::new(Vec::new())).err().unwrap();
    assert_eq!(error.code(), "identity-channel-buffer-required");
    let packet = channel.receive(Zeroizing::new(vec![0; 64])).unwrap();
    assert_eq!(packet.data(), b"queued");
    packet.close().unwrap();
    witness(&sender, &mut channel);
}
/// Payload truncation and an actual empty seqpacket both consume/refuse safely,
/// close received pidfds, and leave the same channel usable by the next packet.
#[test]
fn truncated_payload_and_empty_packet_close_handles_before_recovery() {
    let _guard = SERIAL.lock().unwrap();
    let (sender, receiver) = pair();
    let mut channel = configure(&receiver).unwrap();
    witness(&sender, &mut channel);
    let count = descriptors();
    send(sender.as_raw_fd(), b"too-long", MsgFlags::empty()).unwrap();
    let error = channel.receive(Zeroizing::new(vec![0; 1])).err().unwrap();
    assert_eq!(error.code(), "identity-channel-truncated");
    assert!(error.causes().is_empty());
    assert_eq!(descriptors(), count);
    witness(&sender, &mut channel);
    send(sender.as_raw_fd(), b"", MsgFlags::empty()).unwrap();
    let error = channel.receive(Zeroizing::new(vec![0; 64])).err().unwrap();
    assert_eq!(error.code(), "identity-channel-empty-or-closed");
    assert_eq!(descriptors(), count);
    witness(&sender, &mut channel);
}
/// Unexpected SCM_RIGHTS exceed the exact credentials+pidfd ancillary envelope.
/// MSG_CTRUNC refuses, but all delivered rights and pidfds must still close;
/// kernel-owned excess rights and the original sender file remain distinct.
#[test]
fn unexpected_rights_and_control_truncation_do_not_leak_fds() {
    let _guard = SERIAL.lock().unwrap();
    let (sender, receiver) = pair();
    let mut channel = configure(&receiver).unwrap();
    witness(&sender, &mut channel);
    let file = fs::File::open("/dev/null").unwrap();
    let count = descriptors();
    for amount in [1, 16] {
        let rights = vec![file.as_raw_fd(); amount];
        sendmsg::<()>(
            sender.as_raw_fd(),
            &[IoSlice::new(b"rights")],
            &[ControlMessage::ScmRights(&rights)],
            MsgFlags::empty(),
            None,
        )
        .unwrap();
        let error = channel.receive(Zeroizing::new(vec![0; 64])).err().unwrap();
        assert_eq!(error.code(), "identity-channel-truncated");
        assert!(error.causes().is_empty());
        assert_eq!(descriptors(), count);
        assert!(file.metadata().is_ok());
        witness(&sender, &mut channel);
    }
}
/// An alias disabling a required option must be detected before dequeue. Restore
/// both options explicitly; the queued legitimate packet remains readable.
#[test]
fn option_alias_change_refuses_without_dequeue_then_reconfigures() {
    let _guard = SERIAL.lock().unwrap();
    let (sender, receiver) = pair();
    let mut channel = configure(&receiver).unwrap();
    witness(&sender, &mut channel);
    send(sender.as_raw_fd(), b"queued", MsgFlags::empty()).unwrap();
    setsockopt(&receiver, sockopt::PassCred, &false).unwrap();
    let error = channel.receive(Zeroizing::new(vec![0; 64])).err().unwrap();
    assert_eq!(error.code(), "identity-channel-options-required");
    let mut restored = configure(&receiver).unwrap();
    let packet = restored.receive(Zeroizing::new(vec![0; 64])).unwrap();
    assert_eq!(packet.data(), b"queued");
    packet.close().unwrap();
    witness(&sender, &mut restored);
}
/// Without the host credential-changing capability, a claimed PID1 is rejected
/// by Linux before enqueue. This is not a claim about privileged sender semantics.
#[test]
fn forged_unprivileged_credentials_refuse_before_legitimate_recovery() {
    let _guard = SERIAL.lock().unwrap();
    let (sender, receiver) = pair();
    let mut channel = configure(&receiver).unwrap();
    witness(&sender, &mut channel);
    let forged = UnixCredentials::from(nix::libc::ucred {
        pid: 1,
        uid: nix::unistd::geteuid().as_raw(),
        gid: nix::unistd::getegid().as_raw(),
    });
    let error = sendmsg::<()>(
        sender.as_raw_fd(),
        &[IoSlice::new(b"forged")],
        &[ControlMessage::ScmCredentials(&forged)],
        MsgFlags::empty(),
        None,
    )
    .unwrap_err();
    assert_eq!(error, nix::errno::Errno::EPERM);
    witness(&sender, &mut channel);
}
/// Parent-created connection must identify its actual unprivileged child sender.
/// Hold the child alive via stdin, observe namespaces via pidfd, then reap it and
/// require ESRCH from the same held handle. No numeric PID reopening is performed.
#[test]
fn child_sender_differs_from_connection_creator_and_exit_refuses_namespace() {
    let _guard = SERIAL.lock().unwrap();
    let (sender, receiver) = pair();
    let mut channel = configure(&receiver).unwrap();
    witness(&sender, &mut channel);
    let count = descriptors();
    let original = FdFlag::from_bits_retain(fcntl(&sender, FcntlArg::F_GETFD).unwrap());
    fcntl(&sender, FcntlArg::F_SETFD(original & !FdFlag::FD_CLOEXEC)).unwrap();
    let spawned=Command::new("python3").arg("-c").arg("import socket,sys\nwith socket.socket(fileno=int(sys.argv[1])) as s:\n s.send(b'child')\n sys.stdin.buffer.read(1)\n").arg(sender.as_raw_fd().to_string()).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped()).spawn();
    fcntl(&sender, FcntlArg::F_SETFD(original)).unwrap();
    let mut child = spawned.unwrap();
    let packet = channel.receive(Zeroizing::new(vec![0; 64])).unwrap();
    assert_eq!(packet.data(), b"child");
    assert_eq!(packet.credentials().0, child.id() as i32);
    assert_ne!(packet.credentials().0, std::process::id() as i32);
    packet.namespaces().unwrap().close().unwrap();
    child.stdin.take().unwrap().write_all(b"x").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let error = packet
        .namespaces()
        .err()
        .expect("exited subject must refuse");
    assert_eq!(error.causes()[0].1.raw_os_error(), Some(libc::ESRCH));
    packet.close().unwrap();
    assert_eq!(descriptors(), count);
    witness(&sender, &mut channel);
}

/// Caller-selected nonblocking mode preserves actual EAGAIN without dequeue or
/// FD loss, then the same channel recovers. Peer shutdown produces an explicit
/// empty/closed refusal while borrowed socket ownership remains intact.
#[test]
fn nonblocking_empty_and_peer_shutdown_keep_causes_and_descriptor_owner() {
    let _guard = SERIAL.lock().unwrap();
    let (sender, receiver) = pair();
    let mut channel = configure(&receiver).unwrap();
    witness(&sender, &mut channel);
    let count = descriptors();
    let flags = nix::fcntl::OFlag::from_bits_retain(fcntl(&receiver, FcntlArg::F_GETFL).unwrap());
    fcntl(
        &receiver,
        FcntlArg::F_SETFL(flags | nix::fcntl::OFlag::O_NONBLOCK),
    )
    .unwrap();
    let error = channel.receive(Zeroizing::new(vec![0; 64])).err().unwrap();
    assert_eq!(error.code(), "identity-channel-receive");
    assert_eq!(error.causes()[0].1.raw_os_error(), Some(libc::EAGAIN));
    assert_eq!(descriptors(), count);
    witness(&sender, &mut channel);
    nix::sys::socket::shutdown(sender.as_raw_fd(), nix::sys::socket::Shutdown::Write).unwrap();
    let error = channel.receive(Zeroizing::new(vec![0; 64])).err().unwrap();
    assert_eq!(error.code(), "identity-channel-empty-or-closed");
    assert!(error.causes().is_empty());
    assert_eq!(descriptors(), count);
    assert!(fstat(&receiver).is_ok());
    assert!(fstat(&sender).is_ok());
    let (a, b) = pair();
    let mut recovery = configure(&b).unwrap();
    witness(&a, &mut recovery);
}
