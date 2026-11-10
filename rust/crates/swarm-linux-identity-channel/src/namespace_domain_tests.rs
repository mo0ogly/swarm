//! Private adversarial construction uses real kernel namespace handles only.
use super::*;
use nix::sys::socket::{send, socketpair, AddressFamily, MsgFlags, SockFlag, SockType};
use std::os::fd::AsRawFd;
/// Capture two real typed handles; the temporary packet and socket owners close.
fn observed() -> Namespaces {
    let (tx, rx) = socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    )
    .unwrap();
    let mut channel = configure(&rx).unwrap();
    send(tx.as_raw_fd(), b"namespace", MsgFlags::empty()).unwrap();
    let p = channel.receive(Zeroizing::new(vec![0; 32])).unwrap();
    let n = p.namespaces().unwrap();
    p.close().unwrap();
    n
}
/// A correct control precedes each independent mount/PID object substitution.
/// The invalid pair is private test data, unavailable through the public API.
#[test]
fn namespace_domain_wrong_mount_or_pid_refuses_then_original_recovers() {
    let expected = observed();
    let good = observed();
    assert!(good.same_domain(&expected).unwrap());
    good.close().unwrap();
    for field in ["mount", "pid"] {
        let good = observed();
        assert!(good.same_domain(&expected).unwrap());
        good.close().unwrap();
        let one = observed();
        let two = observed();
        let (bad, spare) = if field == "mount" {
            (
                Namespaces {
                    mount: one.pid,
                    pid: two.pid,
                },
                Namespaces {
                    mount: one.mount,
                    pid: two.mount,
                },
            )
        } else {
            (
                Namespaces {
                    mount: one.mount,
                    pid: two.mount,
                },
                Namespaces {
                    mount: one.pid,
                    pid: two.pid,
                },
            )
        };
        assert!(!bad.same_domain(&expected).unwrap());
        bad.close().unwrap();
        spare.close().unwrap();
        let good = observed();
        assert!(good.same_domain(&expected).unwrap());
        good.close().unwrap();
    }
    expected.close().unwrap();
}
