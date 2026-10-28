//! Real kernel gates with synthetic dispatch callbacks, never a separate-UID
//! business service. Cleanup aggregation uses an actual typed ENOTSOCK cause as
//! a synthetic late-close value; it does not inject a failing OS close.
use super::*;
use crate::{control, personal_service::OutcomeData, service_request::Operation};
use nix::sys::socket::{send, socketpair, AddressFamily, MsgFlags, SockFlag, SockType};
use std::{
    cell::Cell,
    os::fd::{AsRawFd, OwnedFd},
};
use zeroize::Zeroizing;
const PIN: &str = "abababababababababababababababababababababababababababababababab";
/// Actual sender is current UID. No public service controller override exists.
fn packet(raw: &[u8]) -> (OwnedFd, OwnedFd, Packet) {
    let (tx, rx) = socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    )
    .unwrap();
    let mut c = channel::configure(&rx).unwrap();
    send(tx.as_raw_fd(), raw, MsgFlags::empty()).unwrap();
    let p = c.receive(Zeroizing::new(vec![0; raw.len() + 8])).unwrap();
    (tx, rx, p)
}
/// Canonical no-secret source packet, no lifecycle/business effect.
fn raw() -> Zeroizing<Vec<u8>> {
    service_request::encode(PIN, &[58; 32], &Operation::Initialize).unwrap()
}
/// Synthetic callback metadata copies exact correlation and public digest only.
fn reply(r: Request<'_>) -> Reply {
    Reply {
        challenge: *r.challenge(),
        operation: r.operation().code(),
        request_digest: r.public_digest(),
        outcome: OutcomeData::Initialized(Ok(())),
    }
}
/// An actual kernel/private allocation witness before each negative operation.
fn valid() {
    let (_a, _b, p) = packet(&raw());
    let expected = p.namespaces().unwrap();
    let calls = Cell::new(0);
    let result = run(p.credentials().1, &p, &expected, |r| {
        calls.set(calls.get() + 1);
        assert_eq!(r.target_pin(), PIN);
        Ok(reply(r))
    })
    .unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(result.challenge, [58; 32]);
    assert_eq!(result.operation(), 1);
    expected.close().unwrap();
    p.close().unwrap();
}
/// UID refusal precedes even malformed payload decoding; no callback/known Reply.
#[test]
fn controller_packet_uid_before_decode_and_no_dispatch_then_valid_recovers() {
    valid();
    let (_a, _b, p) = packet(b"invalid-secret-packet");
    let expected = p.namespaces().unwrap();
    let e = run(!p.credentials().1, &p, &expected, |_| {
        panic!("UID must refuse before callback")
    })
    .err()
    .unwrap();
    assert_eq!(e.code(), "service-controller-uid");
    assert!(e.reply.is_none() && e.cleanup.is_none());
    assert!(!format!("{e:?}").contains("secret"));
    expected.close().unwrap();
    p.close().unwrap();
    valid();
}
/// Matched actual domain permits exactly one decode/callback, malformed data
/// refuses before callback and borrows neither a new buffer nor synthetic PID.
#[test]
fn controller_packet_exact_domain_decode_refusal_and_callback_once() {
    valid();
    let (_a, _b, p) = packet(b"malformed-private-body");
    let expected = p.namespaces().unwrap();
    let e = run(p.credentials().1, &p, &expected, |_| {
        panic!("decode must refuse")
    })
    .err()
    .unwrap();
    assert_eq!(e.code(), "service-request-format");
    assert!(e.reply.is_none());
    expected.close().unwrap();
    p.close().unwrap();
    valid();
}
/// Target dispatch error survives; a business error inside Reply stays inside it.
/// This tests a synthetic callback only, not actual UID-separated authority use.
#[test]
fn controller_packet_dispatch_refusal_and_known_business_outcome_survive() {
    valid();
    let (_a, _b, p) = packet(&raw());
    let expected = p.namespaces().unwrap();
    let e = run(p.credentials().1, &p, &expected, |_| {
        Err(DispatchError::Target)
    })
    .err()
    .unwrap();
    assert_eq!(e.code(), "service-target");
    assert!(e.reply.is_none());
    let r = run(p.credentials().1, &p, &expected, |r| {
        let mut out = reply(r);
        out.outcome = OutcomeData::Control(Ok(Err(control::Error::Permission)));
        Ok(out)
    })
    .unwrap();
    assert!(matches!(
        r.outcome,
        OutcomeData::Control(Ok(Err(control::Error::Permission)))
    ));
    expected.close().unwrap();
    p.close().unwrap();
    valid();
}
/// Actual typed syscall cause used solely to test the pure aggregation contract.
fn cleanup_cause() -> channel::Error {
    channel::configure(&std::fs::File::open("/dev/null").unwrap())
        .err()
        .unwrap()
}
/// Preserve primary + independent cleanup or entire known Reply + cleanup.
/// No actual close failure or after-effect business operation is claimed here.
#[test]
fn controller_packet_late_cleanup_keeps_reply_business_error_and_original_cause() {
    valid();
    let raw = raw();
    let mut known = reply(service_request::decode(&raw).unwrap());
    known.outcome = OutcomeData::Control(Ok(Err(control::Error::Permission)));
    let e = finish(Ok(known), Some(cleanup_cause())).err().unwrap();
    assert_eq!(e.code(), "service-controller-close");
    assert!(e.cause.is_none());
    assert!(matches!(
        e.reply.as_ref().unwrap().outcome,
        OutcomeData::Control(Ok(Err(control::Error::Permission)))
    ));
    assert_eq!(e.error_branches().len(), 2);
    assert!(e.cleanup.as_ref().unwrap().causes()[0]
        .1
        .raw_os_error()
        .is_some());
    assert!(!format!("{e:?}").contains(PIN));
    let e = finish(
        Err(Cause::Dispatch(DispatchError::Target)),
        Some(cleanup_cause()),
    )
    .err()
    .unwrap();
    assert_eq!(e.code(), "service-target");
    assert!(e.reply.is_none());
    assert_eq!(e.error_branches().len(), 2);
    valid();
}

/// AfterUse retains authority plus failed business outcome through direct, boxed
/// and IO-custom wrappers. These are constructor tests, not a distributed effect.
#[test]
fn controller_packet_after_use_and_custom_io_preserve_all_known_error_branches() {
    use crate::personal::coordinator::UseError;
    let raw = raw();
    let mut known = reply(service_request::decode(&raw).unwrap());
    known.outcome = OutcomeData::Control(Err(UseError::AfterUse {
        cause: swarm_enrollment_state::Error::Canceled,
        outcome: Err(control::Error::Permission),
    }));
    let e = finish(Ok(known), Some(cleanup_cause())).err().unwrap();
    assert_eq!(e.error_branches().len(), 3);
    assert!(e
        .error_branches()
        .iter()
        .any(|e| e.downcast_ref::<control::Error>().is_some()));
    let boxed: Box<dyn StdError + Send + Sync> = Box::new(e);
    let custom = std::io::Error::other(boxed);
    let retained = custom.get_ref().unwrap().downcast_ref::<Error>().unwrap();
    assert_eq!(retained.error_branches().len(), 3);
    assert!(retained.reply.is_some());
    assert!(!format!("{custom}").contains(PIN));
}

/// A real other mount/PID domain refuses before decoding its malformed payload
/// or invoking the callback. Current UID is test input to the private seam only;
/// the public Service still takes its role exclusively from pinned startup.
#[test]
fn controller_packet_actual_other_domain_before_decode_and_no_dispatch() {
    use std::os::fd::AsFd;
    use std::process::{Command, Stdio};
    valid();
    let (_a, _b, p) = packet(&raw());
    let expected = p.namespaces().unwrap();
    let (tx, rx) = socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    )
    .unwrap();
    let mut c = channel::configure(&rx).unwrap();
    let program =
        "import socket; s=socket.socket(fileno=0); s.send(b'malformed-other-domain'); s.recv(1)";
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
    let observed = c.receive(Zeroizing::new(vec![0; 64]));
    let outcome = match &observed {
        Ok(q) => Some(run(q.credentials().1, q, &expected, |_| {
            panic!("wrong domain must refuse before dispatch")
        })),
        Err(_) => None, // Original receive remains owned until explicit assertion.
    };
    // Keep child alive for the actual pidfd query, then release on all outcomes.
    let release = send(rx.as_raw_fd(), b"A", MsgFlags::MSG_NOSIGNAL);
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "namespace fixture refused: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    release.unwrap();
    let observed = observed.expect("actual attributed controller packet");
    let e = outcome.unwrap().err().unwrap();
    assert_eq!(e.code(), "service-controller-domain");
    assert!(e.reply.is_none() && e.cleanup.is_none());
    observed.close().unwrap();
    expected.close().unwrap();
    p.close().unwrap();
    valid();
}

/// Real retained child and actual kernel packets, with a synthetic dispatcher.
/// The false/true switch controls a disposable fixture's private mount namespace,
/// not a product policy. No real separate-UID business service is claimed.
fn spawned_case(other_mount: bool) {
    use std::{
        io::{BufRead, BufReader},
        os::fd::AsFd,
        process::{Command, Stdio},
    };
    valid();
    let (_pa, _pb, own) = packet(&raw());
    let expected = own.namespaces().unwrap();
    let (tx, rx) = socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    )
    .unwrap();
    let mut c = channel::configure(&rx).unwrap();
    let body = if other_mount {
        Zeroizing::new(b"malformed-in-other-domain".to_vec())
    } else {
        raw()
    };
    let hex = body.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let code="import socket,sys; s=socket.socket(fileno=0); s.send(bytes.fromhex(sys.argv[1])); print('ready',flush=True); s.recv(1)";
    let mut command = if other_mount {
        let mut c = Command::new("/usr/bin/unshare");
        c.args(["--user", "--map-root-user", "--mount", "/usr/bin/python3"]);
        c
    } else {
        Command::new("/usr/bin/python3")
    };
    let mut child = command
        .args(["-c", code, &hex])
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
        panic!("spawn fixture handshake: {read:?} {kill:?} {ended:?}")
    }
    let anchor = channel::ProcessAnchor::capture_child(&mut child);
    let first = c.receive(Zeroizing::new(vec![0; body.len() + 8])).unwrap();
    let calls = Cell::new(0);
    let before = anchor.as_ref().map(|a| {
        run_spawned(first.credentials().1, &first, a, &expected, |r| {
            calls.set(calls.get() + 1);
            Ok(reply(r))
        })
    });
    // Same-UID parent uses the same connection after the valid child witness.
    send(tx.as_raw_fd(), b"malformed-parent", MsgFlags::empty()).unwrap();
    let parent = c.receive(Zeroizing::new(vec![0; 64])).unwrap();
    let rejected = anchor.as_ref().map(|a| {
        run_spawned(parent.credentials().1, &parent, a, &expected, |_| {
            panic!("different process must refuse")
        })
    });
    let recovery = anchor.as_ref().map(|a| {
        run_spawned(first.credentials().1, &first, a, &expected, |r| {
            calls.set(calls.get() + 1);
            Ok(reply(r))
        })
    });
    send(rx.as_raw_fd(), b"A", MsgFlags::MSG_NOSIGNAL).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "private child: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let before = before.unwrap();
    let recovery = recovery.unwrap();
    if other_mount {
        assert_eq!(before.err().unwrap().code(), "service-controller-domain");
        assert_eq!(recovery.err().unwrap().code(), "service-controller-domain");
        assert_eq!(calls.get(), 0)
    } else {
        assert_eq!(before.unwrap().challenge, [58; 32]);
        assert_eq!(recovery.unwrap().challenge, [58; 32]);
        assert_eq!(calls.get(), 2)
    }
    let error = rejected.unwrap().err().unwrap();
    assert_eq!(error.code(), "service-controller-process");
    assert!(error.reply.is_none() && error.cleanup.is_none());
    let anchor = anchor.unwrap();
    let after = run_spawned(parent.credentials().1, &parent, &anchor, &expected, |_| {
        panic!("expired process must refuse")
    })
    .err()
    .unwrap();
    assert_eq!(after.code(), "identity-channel-process-exited");
    assert_eq!(after.error_branches().len(), 1);
    anchor.close().unwrap();
    first.close().unwrap();
    parent.close().unwrap();
    expected.close().unwrap();
    own.close().unwrap();
    valid();
}
/// Actual child control, same UID/namespace wrong process refused before decode,
/// and original packet remains a binding witness, never business replay authority.
#[test]
fn spawned_packet_subject_before_decode_and_known_reply_recovery() {
    spawned_case(false)
}
/// Matching approved process never skips the existing mount/PID domain gate.
/// A malformed packet from that same child in another mount domain refuses there.
#[test]
fn spawned_packet_matching_process_keeps_namespace_gate() {
    spawned_case(false);
    spawned_case(true);
    spawned_case(false)
}

/// Actual atomic descriptor/packets with synthetic dispatcher, retaining all
/// domain checks. Fixture bool selects a private mount, never product policy.
#[cfg(target_env = "gnu")]
fn atomic_case(other_mount: bool) {
    use channel::atomic_spawn::{self, Spec};
    use nix::{fcntl::OFlag, sys::signal::SigSet, unistd::pipe2};
    use std::{
        ffi::CString,
        io::{BufRead, BufReader},
        os::fd::AsFd,
    };
    valid();
    let (_pa, _pb, own) = packet(&raw());
    let expected = own.namespaces().unwrap();
    let (tx, rx) = socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    )
    .unwrap();
    let mut c = channel::configure(&rx).unwrap();
    let (reader, writer) = pipe2(OFlag::O_CLOEXEC).unwrap();
    let null = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/null")
        .unwrap();
    let code="import socket,sys; s=socket.socket(fileno=0); s.send(bytes.fromhex(sys.argv[1])); print('ready',flush=True); s.recv(1)";
    let body = if other_mount {
        Zeroizing::new(b"malformed-other-domain".to_vec())
    } else {
        raw()
    };
    let hex = body.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let program = CString::new(if other_mount {
        "/usr/bin/unshare"
    } else {
        "/usr/bin/python3"
    })
    .unwrap();
    let mut args = vec![CString::new(if other_mount { "unshare" } else { "python3" }).unwrap()];
    if other_mount {
        for a in ["--user", "--map-root-user", "--mount", "/usr/bin/python3"] {
            args.push(CString::new(a).unwrap())
        }
    }
    for a in ["-c", code, &hex] {
        args.push(CString::new(a).unwrap())
    }
    let directory = CString::new("/").unwrap();
    let mask = SigSet::empty();
    let defaults = SigSet::all();
    let spec = Spec {
        program: &program,
        arguments: &args,
        environment: &[],
        directory: &directory,
        stdin: tx.as_fd(),
        stdout: writer.as_fd(),
        stderr: null.as_fd(),
        signal_mask: &mask,
        signal_defaults: &defaults,
    };
    let result = atomic_spawn::launch(&spec);
    drop(writer);
    let mut child = match result {
        Ok(c) => c,
        Err(mut e) => {
            if let Some(mut c) = e.child.take() {
                let release = send(rx.as_raw_fd(), b"A", MsgFlags::MSG_NOSIGNAL);
                let ended = c.wait();
                let close = c.close();
                panic!("native cleanup: {e:?} {release:?} {ended:?} {close:?}")
            }
            panic!("native launch: {e:?}")
        }
    };
    let mut ready = String::new();
    let read = BufReader::new(std::fs::File::from(reader)).read_line(&mut ready);
    if read.is_err() || ready != "ready\n" {
        let release = send(rx.as_raw_fd(), b"A", MsgFlags::MSG_NOSIGNAL);
        let ended = child.wait();
        let close = child.close();
        panic!("fixture handshake: {read:?} {ready:?} {release:?} {ended:?} {close:?}")
    }
    let first = c.receive(Zeroizing::new(vec![0; body.len() + 8])).unwrap();
    let calls = Cell::new(0);
    let before = run_atomic(first.credentials().1, &first, &child, &expected, |r| {
        calls.set(calls.get() + 1);
        Ok(reply(r))
    });
    send(tx.as_raw_fd(), b"malformed-parent", MsgFlags::empty()).unwrap();
    let parent = c.receive(Zeroizing::new(vec![0; 64])).unwrap();
    let wrong_calls = Cell::new(0);
    let rejected = run_atomic(parent.credentials().1, &parent, &child, &expected, |r| {
        wrong_calls.set(wrong_calls.get() + 1);
        Ok(reply(r))
    });
    let recovery = run_atomic(first.credentials().1, &first, &child, &expected, |r| {
        calls.set(calls.get() + 1);
        Ok(reply(r))
    });
    send(rx.as_raw_fd(), b"A", MsgFlags::MSG_NOSIGNAL).unwrap();
    assert!(child.wait().unwrap().success());
    assert_eq!(wrong_calls.get(), 0);
    if other_mount {
        assert_eq!(before.err().unwrap().code(), "service-controller-domain");
        assert_eq!(recovery.err().unwrap().code(), "service-controller-domain");
        assert_eq!(calls.get(), 0)
    } else {
        assert_eq!(before.unwrap().challenge, [58; 32]);
        assert_eq!(recovery.unwrap().challenge, [58; 32]);
        assert_eq!(calls.get(), 2)
    }
    let error = rejected.err().unwrap();
    assert_eq!(error.code(), "service-controller-process");
    assert!(error.reply.is_none() && error.cleanup.is_none());
    child.close().unwrap();
    first.close().unwrap();
    parent.close().unwrap();
    expected.close().unwrap();
    own.close().unwrap();
    valid();
}
/// Actual atomically bound process refuses same-UID parent before decode and
/// recovers binding. Synthetic callback repetition grants no business replay.
#[cfg(target_env = "gnu")]
#[test]
fn atomic_packet_subject_binding_keeps_all_existing_stages() {
    atomic_case(false)
}
/// A matching atomic process in another real mount namespace still refuses
/// before malformed payload decoding; approved process is never a domain bypass.
#[cfg(target_env = "gnu")]
#[test]
fn atomic_packet_matching_process_does_not_bypass_domain() {
    atomic_case(false);
    atomic_case(true);
    atomic_case(false)
}
