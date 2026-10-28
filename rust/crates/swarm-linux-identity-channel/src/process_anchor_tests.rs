//! Invalid object test, with actual live child/packet witness first.
use super::*;
use nix::sys::socket::{send, socketpair, AddressFamily, MsgFlags, SockFlag, SockType};
use std::{
    io::{BufRead, BufReader},
    os::fd::AsRawFd,
    process::{Command, Stdio},
};
/// Wrong filesystem object cannot match even the same live subject. Child remains
/// explicitly released/reaped; no unsafe FD closure or fabricated raw descriptor.
#[test]
fn process_anchor_non_pidfs_refuses_after_live_child_control() {
    let (tx, rx) = socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    )
    .unwrap();
    let mut c = configure(&rx).unwrap();
    let code="import socket; s=socket.socket(fileno=0); s.send(b'live'); print('ready',flush=True); s.recv(1)";
    let mut child = Command::new("/usr/bin/python3")
        .args(["-c", code])
        .stdin(Stdio::from(tx.as_fd().try_clone_to_owned().unwrap()))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut ready = String::new();
    let r = BufReader::new(child.stdout.take().unwrap()).read_line(&mut ready);
    if r.is_err() || ready != "ready\n" {
        let kill = child.kill();
        let ended = child.wait_with_output();
        panic!("handshake: {r:?} {kill:?} {ended:?}")
    }
    let valid = ProcessAnchor::capture_child(&mut child);
    let p = c.receive(Zeroizing::new(vec![0; 32])).unwrap();
    let witness = valid.as_ref().map(|a| a.matches(&p));
    let invalid = ProcessAnchor {
        process: std::fs::File::open("/dev/null").unwrap().into(),
    };
    let refused = invalid.matches(&p);
    send(rx.as_raw_fd(), b"A", MsgFlags::MSG_NOSIGNAL).unwrap();
    let end = child.wait_with_output().unwrap();
    assert!(end.status.success());
    assert!(witness.unwrap().unwrap());
    assert_eq!(
        refused.err().unwrap().code(),
        "identity-channel-process-pidfs-required"
    );
    valid.unwrap().close().unwrap();
    invalid.close().unwrap();
    p.close().unwrap();
}
