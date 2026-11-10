#![cfg(target_os = "linux")]
#![forbid(unsafe_code)]
//! Actual child/process sender identity, no account, approval or confinement.
use nix::sys::socket::{send, socketpair, AddressFamily, MsgFlags, SockFlag, SockType};
use std::{
    io::{BufRead, BufReader},
    os::fd::{AsFd, AsRawFd},
    process::{Command, Stdio},
    sync::Mutex,
};
use swarm_linux_identity_channel::{configure, ProcessAnchor};
use zeroize::Zeroizing;
static SERIAL: Mutex<()> = Mutex::new(());
/// Current-process FD count; consuming ReadDir closes its temporary descriptor.
/// All tests in this target serialize, so unrelated test fixture FDs cannot hide a leak.
fn descriptors() -> usize {
    std::fs::read_dir("/proc/self/fd").unwrap().count()
}
/// Positive child, same-domain parent refusal and child recovery, then dead anchor.
/// Handshake stdout EOF exposes an early child error before socket receive.
fn roundtrip() {
    let (tx, rx) = socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    )
    .unwrap();
    let mut c = configure(&rx).unwrap();
    let code="import socket; s=socket.socket(fileno=0); s.send(b'first'); print('ready',flush=True); s.recv(1); s.send(b'again'); s.recv(1)";
    let mut child = Command::new("/usr/bin/python3")
        .args(["-c", code])
        .stdin(Stdio::from(tx.as_fd().try_clone_to_owned().unwrap()))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut ready = String::new();
    let read = BufReader::new(child.stdout.take().unwrap()).read_line(&mut ready);
    if read.is_err() || ready != "ready\n" {
        let kill = child.kill();
        let ended = child.wait_with_output();
        panic!("private child handshake failed: read={read:?} kill={kill:?} ended={ended:?}")
    }
    // Capture from the owned Child, not any received credentials or selected PID.
    let anchor = ProcessAnchor::capture_child(&mut child);
    let first = c.receive(Zeroizing::new(vec![0; 64])).unwrap();
    let before = anchor.as_ref().map(|a| a.matches(&first));
    send(tx.as_raw_fd(), b"parent", MsgFlags::empty()).unwrap();
    let other = c.receive(Zeroizing::new(vec![0; 64])).unwrap();
    let refused = anchor.as_ref().map(|a| a.matches(&other));
    send(rx.as_raw_fd(), b"A", MsgFlags::MSG_NOSIGNAL).unwrap();
    let again = c.receive(Zeroizing::new(vec![0; 64])).unwrap();
    let recovered = anchor.as_ref().map(|a| a.matches(&again));
    send(rx.as_raw_fd(), b"B", MsgFlags::MSG_NOSIGNAL).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "private child: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(before.unwrap().unwrap());
    assert!(!refused.unwrap().unwrap());
    assert!(recovered.unwrap().unwrap());
    assert_ne!(first.credentials().0, other.credentials().0);
    assert_eq!(first.credentials().1, other.credentials().1);
    let anchor = anchor.unwrap();
    assert_eq!(
        anchor.matches(&first).err().unwrap().code(),
        "identity-channel-process-exited"
    );
    assert_eq!(format!("{anchor:?}"), "ProcessAnchor { <private> }");
    anchor.close().unwrap();
    first.close().unwrap();
    other.close().unwrap();
    again.close().unwrap();
}
/// Kernel subject differs from connection creator and later sender of same UID.
#[test]
fn spawned_process_anchor_child_then_same_uid_parent_refusal_and_recovery() {
    let _g = SERIAL.lock().unwrap();
    let before = descriptors();
    roundtrip();
    assert_eq!(descriptors(), before);
}
/// Capturing an already reaped child refuses; no PID lookup or child restart.
#[test]
fn spawned_process_anchor_reaped_child_refuses_then_fresh_control() {
    let _g = SERIAL.lock().unwrap();
    let before = descriptors();
    roundtrip();
    let mut ended = Command::new("/usr/bin/true").spawn().unwrap();
    let status = ended.wait().unwrap();
    assert!(status.success());
    let e = ProcessAnchor::capture_child(&mut ended).err().unwrap();
    assert_eq!(e.code(), "identity-channel-process-exited");
    assert_eq!(ended.wait().unwrap(), status);
    drop(ended);
    roundtrip();
    assert_eq!(descriptors(), before);
}
