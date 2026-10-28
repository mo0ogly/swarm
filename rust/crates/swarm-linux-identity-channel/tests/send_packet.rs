#![cfg(target_os = "linux")]
#![forbid(unsafe_code)]
//! Actual local kernel sends, no admission/daemon/live datastore/provider.
use nix::sys::socket::{
    recv, setsockopt, socketpair, sockopt, AddressFamily, MsgFlags, SockFlag, SockType,
};
use std::os::fd::{AsRawFd, OwnedFd};
use swarm_linux_identity_channel::{configure, Channel};
use zeroize::Zeroizing;
/// Private CLOEXEC seqpacket pair with no listener or shared fixture root.
fn pair() -> (OwnedFd, OwnedFd) {
    socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    )
    .unwrap()
}
/// Require exact content and kernel subject, then observe explicit pidfd close.
fn received(channel: &mut Channel<'_>, expected: &[u8]) {
    let p = channel.receive(Zeroizing::new(vec![0; 128])).unwrap();
    assert_eq!(p.data(), expected);
    assert_eq!(p.credentials().0, std::process::id() as i32);
    p.close().unwrap();
}
/// Queue success preserves two distinct records and caller-owned guarded bytes.
#[test]
fn packet_send_exact_records_and_guarded_input() {
    let (a, b) = pair();
    let mut out = configure(&a).unwrap();
    let mut input = configure(&b).unwrap();
    let raw = Zeroizing::new(b"private-fixture-record".to_vec());
    out.send_packet(&raw).unwrap();
    out.send_packet(b"second").unwrap();
    assert_eq!(&raw[..], b"private-fixture-record");
    received(&mut input, &raw);
    received(&mut input, b"second");
    input.send_packet(b"response").unwrap();
    received(&mut out, b"response");
}
/// A valid control precedes each preflight refusal; no packet queued, recovery exact.
#[test]
fn packet_send_empty_or_alias_options_refuses_before_native_then_recovers() {
    let (a, b) = pair();
    let mut out = configure(&a).unwrap();
    let mut input = configure(&b).unwrap();
    out.send_packet(b"control").unwrap();
    received(&mut input, b"control");
    let e = out.send_packet(b"").unwrap_err();
    assert!(!e.attempted);
    assert_eq!(e.accepted_len, None);
    assert_eq!(e.cause.code(), "identity-channel-send-empty");
    let mut tmp = [0; 128];
    assert_eq!(
        recv(b.as_raw_fd(), &mut tmp, MsgFlags::MSG_DONTWAIT).unwrap_err(),
        nix::errno::Errno::EAGAIN
    );
    out.send_packet(b"before-alias").unwrap();
    received(&mut input, b"before-alias");
    setsockopt(&a, sockopt::PassCred, &false).unwrap();
    let e = out.send_packet(b"not-queued").unwrap_err();
    assert!(!e.attempted);
    assert_eq!(e.cause.code(), "identity-channel-options-required");
    assert_eq!(
        recv(b.as_raw_fd(), &mut tmp, MsgFlags::MSG_DONTWAIT).unwrap_err(),
        nix::errno::Errno::EAGAIN
    );
    let mut out = configure(&a).unwrap();
    out.send_packet(b"recovery").unwrap();
    received(&mut input, b"recovery");
}
/// Closed peer keeps actual errno and invocation state; new pair is fresh recovery.
#[test]
fn packet_send_closed_peer_keeps_original_cause_and_fresh_recovery() {
    let (a, b) = pair();
    let mut out = configure(&a).unwrap();
    let mut input = configure(&b).unwrap();
    out.send_packet(b"control").unwrap();
    received(&mut input, b"control");
    drop(input);
    drop(b);
    let secret = Zeroizing::new(b"private-send-fixture".to_vec());
    let e = out.send_packet(&secret).unwrap_err();
    assert!(e.attempted);
    assert_eq!(e.accepted_len, None);
    assert_eq!(e.cause.causes()[0].1.raw_os_error(), Some(libc::EPIPE));
    assert!(!format!("{e:?}").contains("private-send-fixture"));
    assert_eq!(&secret[..], b"private-send-fixture");
    let (c, d) = pair();
    let mut out = configure(&c).unwrap();
    let mut input = configure(&d).unwrap();
    out.send_packet(b"fresh-control").unwrap();
    received(&mut input, b"fresh-control");
}

/// Use the observed kernel send-buffer extent, not an invented product ceiling.
/// Valid control precedes native EMSGSIZE; refusal leaves receive queue empty.
#[test]
fn packet_send_actual_kernel_size_refusal_and_recovery() {
    use nix::sys::socket::getsockopt;
    let (a, b) = pair();
    let mut out = configure(&a).unwrap();
    let mut input = configure(&b).unwrap();
    out.send_packet(b"control").unwrap();
    received(&mut input, b"control");
    let capacity = getsockopt(&a, sockopt::SndBuf).unwrap();
    assert!(capacity > 0);
    let n = capacity.checked_add(1).unwrap();
    let mut raw = Zeroizing::new(Vec::new());
    raw.try_reserve_exact(n).unwrap();
    raw.resize(n, 0x61);
    let e = out.send_packet(&raw).unwrap_err();
    assert!(e.attempted);
    assert_eq!(e.accepted_len, None);
    assert_eq!(e.cause.causes()[0].1.raw_os_error(), Some(libc::EMSGSIZE));
    let mut buf = [0; 128];
    assert_eq!(
        recv(b.as_raw_fd(), &mut buf, MsgFlags::MSG_DONTWAIT).unwrap_err(),
        nix::errno::Errno::EAGAIN
    );
    out.send_packet(b"recovery").unwrap();
    received(&mut input, b"recovery");
    println!("SEND_KERNEL_CAPACITY bytes={capacity}");
}
/// Real nonblocking saturation, stopping only at kernel EAGAIN, then exact drain.
/// Finite observed SO_SNDBUF and nonblocking flags provide the fixture condition;
/// no arbitrary attempt/deadline/volume guard or production policy is introduced.
#[test]
fn packet_send_nonblocking_full_queue_refuses_then_drain_recovers() {
    use nix::{
        fcntl::{fcntl, FcntlArg, OFlag},
        sys::socket::getsockopt,
    };
    let (a, b) = socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC | SockFlag::SOCK_NONBLOCK,
    )
    .unwrap();
    assert_ne!(
        fcntl(&a, FcntlArg::F_GETFL).unwrap() & OFlag::O_NONBLOCK.bits(),
        0
    );
    let capacity = getsockopt(&a, sockopt::SndBuf).unwrap();
    assert!(capacity > 0);
    let mut out = configure(&a).unwrap();
    let mut input = configure(&b).unwrap();
    out.send_packet(b"control").unwrap();
    received(&mut input, b"control");
    let mut queued = 0usize;
    let failure = loop {
        match out.send_packet(b"x") {
            Ok(()) => queued = queued.checked_add(1).unwrap(),
            Err(e) => break e,
        }
    };
    assert!(queued > 0);
    assert!(failure.attempted);
    assert_eq!(failure.accepted_len, None);
    assert_eq!(
        failure.cause.causes()[0].1.raw_os_error(),
        Some(libc::EAGAIN)
    );
    for _ in 0..queued {
        received(&mut input, b"x")
    }
    out.send_packet(b"recovery").unwrap();
    received(&mut input, b"recovery");
    println!("SEND_QUEUE_RECOVERY records={queued} capacity={capacity}");
}
