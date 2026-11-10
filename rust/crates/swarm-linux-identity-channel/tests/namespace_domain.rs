#![cfg(target_os = "linux")]
#![forbid(unsafe_code)]
use nix::sys::socket::{send, socketpair, AddressFamily, MsgFlags, SockFlag, SockType};
use std::os::fd::{AsRawFd, OwnedFd};
use swarm_linux_identity_channel::{configure, Namespaces, Packet};
use zeroize::Zeroizing;
/// Actual current-process kernel packet and namespaces; no approval/OS isolation.
fn observation() -> (OwnedFd, OwnedFd, Packet, Namespaces) {
    let (tx, rx) = socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    )
    .unwrap();
    let mut c = configure(&rx).unwrap();
    send(tx.as_raw_fd(), b"domain-witness", MsgFlags::empty()).unwrap();
    let p = c.receive(Zeroizing::new(vec![0; 64])).unwrap();
    let n = p.namespaces().unwrap();
    (tx, rx, p, n)
}
/// Retained independently captured descriptors compare exact namespace objects;
/// success is an observation and grants no identity/authority or ownership transfer.
#[test]
fn namespace_domain_same_objects_repeated_observation_and_explicit_closes() {
    let (_a, _b, p, expected) = observation();
    let (_c, _d, q, observed) = observation();
    assert!(observed.same_domain(&expected).unwrap());
    assert!(expected.same_domain(&observed).unwrap());
    observed.close().unwrap();
    q.close().unwrap();
    let next = p.namespaces().unwrap();
    assert!(next.same_domain(&expected).unwrap());
    next.close().unwrap();
    expected.close().unwrap();
    p.close().unwrap();
}

/// Real same-kind mount/PID namespace differences, not only swapped test handles.
/// A held child sends a packet from a private single-UID user/mount/PID namespace.
/// One UID mapping is a transport fixture, never actual separate OS identities.
#[test]
fn namespace_domain_actual_private_namespace_refuses_then_original_recovers() {
    use std::os::fd::AsFd;
    use std::process::{Command, Stdio};
    let (_a, _b, p, expected) = observation();
    let good = p.namespaces().unwrap();
    assert!(good.same_domain(&expected).unwrap());
    good.close().unwrap();
    let (tx, rx) = socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    )
    .unwrap();
    let mut c = configure(&rx).unwrap();
    let program = "import socket; s=socket.socket(fileno=0); s.send(b'private-domain'); s.recv(1)";
    let child = Command::new("/usr/bin/unshare")
        .args([
            "--user",
            "--map-root-user",
            "--mount",
            "--pid",
            "--fork",
            "/usr/bin/python3",
            "-c",
            program,
        ])
        .stdin(Stdio::from(tx.as_fd().try_clone_to_owned().unwrap()))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(tx);
    let received = c.receive(Zeroizing::new(vec![0; 64]));
    // Release the child on every observed result before asserting. Its namespace
    // query is performed while it still waits, so exit cannot mask the domain case.
    // Preserve original receive, namespace, comparison and independent close
    // results until child release; do not replace typed causes with code strings.
    let domain = match &received {
        Ok(q) => Some(q.namespaces().map(|n| {
            let same = n.same_domain(&expected);
            let close = n.close();
            (same, close)
        })),
        Err(_) => None, // `received.expect` below reports the original typed error.
    };
    let release = send(rx.as_raw_fd(), b"A", MsgFlags::MSG_NOSIGNAL);
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "private namespace child refused: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    release.unwrap();
    let received = received.expect("actual attributed child packet");
    let (same, close) = domain.unwrap().expect("actual child namespace handles");
    assert!(
        same.is_ok() && close.is_ok(),
        "comparison={same:?}; cleanup={close:?}"
    );
    assert!(!same.unwrap());
    close.unwrap();
    received.close().unwrap();
    let good = p.namespaces().unwrap();
    assert!(good.same_domain(&expected).unwrap());
    good.close().unwrap();
    expected.close().unwrap();
    p.close().unwrap();
}
