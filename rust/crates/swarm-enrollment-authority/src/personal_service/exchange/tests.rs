//! Composition guards use real kernel packets and a synthetic business callback.
//! They do not establish separate-UID service admission or an atomic spawn gate.
use super::*;
use crate::{
    control,
    personal_service::OutcomeData,
    service_request::{self, Operation},
};
use nix::sys::socket::{recv, send, socketpair, AddressFamily, MsgFlags, SockFlag, SockType};
use std::os::fd::{AsRawFd, OwnedFd};
const PIN: &str = "abababababababababababababababababababababababababababababababab";
/// A private real seqpacket pair; no daemon or shared runtime state.
fn pair() -> (OwnedFd, OwnedFd) {
    socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    )
    .unwrap()
}
/// Public-data response with exact request correlation, no business side effect.
fn reply() -> Reply {
    let raw = service_request::encode(PIN, &[63; 32], &Operation::Initialize).unwrap();
    let request = service_request::decode(&raw).unwrap();
    Reply {
        challenge: *request.challenge(),
        operation: request.operation().code(),
        request_digest: request.public_digest(),
        outcome: OutcomeData::Initialized(Ok(())),
    }
}
/// Actual ENOTSOCK retained solely as synthetic late-close input to pure merge.
fn close_cause() -> channel::Error {
    channel::configure(&std::fs::File::open("/dev/null").unwrap())
        .err()
        .unwrap()
}
/// A successful real packet exchange is the witness before preflight rejection.
#[test]
fn exchange_record_once_then_preflight_refusal_preserves_queue_and_recovers() {
    let (tx, rx) = pair();
    let mut c = channel::configure(&rx).unwrap();
    for preflight in [false, true] {
        send(tx.as_raw_fd(), b"actual-request", MsgFlags::empty()).unwrap();
        if preflight {
            let e = run(&mut c, Zeroizing::new(vec![]), |_| {
                panic!("preflight dispatch")
            })
            .err()
            .unwrap();
            assert!(matches!(e.cause, Cause::Receive(_)));
            assert!(e.known_reply().is_none());
            assert_eq!(e.error_branches().len(), 1);
        }
        let result = run(&mut c, Zeroizing::new(vec![0; 128]), |p| {
            assert_eq!(p.credentials().0 as u32, std::process::id());
            Ok(reply())
        })
        .unwrap();
        let expected = swarm_wire_json::encode(&result.observation).unwrap();
        let mut wire = vec![0; 65536];
        let n = recv(tx.as_raw_fd(), &mut wire, MsgFlags::empty()).unwrap();
        assert_eq!(&wire[..n], expected);
        assert!(matches!(
            result.reply.outcome,
            OutcomeData::Initialized(Ok(()))
        ));
        assert_eq!(
            recv(tx.as_raw_fd(), &mut wire, MsgFlags::MSG_DONTWAIT),
            Err(nix::errno::Errno::EAGAIN)
        );
    }
}
/// Dispatch refusal never publishes a response; another real request can recover.
#[test]
fn exchange_dispatch_refuses_without_send_and_recovers() {
    let (tx, rx) = pair();
    let mut c = channel::configure(&rx).unwrap();
    send(
        tx.as_raw_fd(),
        b"private-invalid-request",
        MsgFlags::empty(),
    )
    .unwrap();
    let e = run(&mut c, Zeroizing::new(vec![0; 128]), |_| {
        Err(controller_packet::Error {
            cause: Some(controller_packet::Cause::Dispatch(
                super::super::Error::Target,
            )),
            cleanup: None,
            reply: None,
        })
    })
    .err()
    .unwrap();
    assert_eq!(e.code(), "service-target");
    assert!(e.known_reply().is_none());
    assert_eq!(e.error_branches().len(), 1);
    assert!(!format!("{e:?}").contains("private"));
    assert_eq!(
        recv(tx.as_raw_fd(), &mut [0; 8], MsgFlags::MSG_DONTWAIT),
        Err(nix::errno::Errno::EAGAIN)
    );
    send(tx.as_raw_fd(), b"recovery", MsgFlags::empty()).unwrap();
    run(&mut c, Zeroizing::new(vec![0; 128]), |_| Ok(reply())).unwrap();
    assert!(recv(tx.as_raw_fd(), &mut [0; 4096], MsgFlags::empty()).unwrap() > 0);
}
/// Dispatch plus independent synthetic close retains both, no send invocation.
#[test]
fn exchange_dispatch_and_close_retains_independent_causes() {
    let e = finish(
        Err(controller_packet::Error {
            cause: Some(controller_packet::Cause::Dispatch(
                super::super::Error::Target,
            )),
            cleanup: Some(close_cause()),
            reply: None,
        }),
        Err(close_cause()),
        |_| panic!("must not send"),
    )
    .err()
    .unwrap();
    assert_eq!(e.code(), "service-target");
    assert!(e.packet_close.is_some());
    assert_eq!(e.error_branches().len(), 3);
}
/// Known AfterUse result survives late close beside both business failure branches.
#[test]
fn exchange_known_after_use_and_close_keeps_all_branches() {
    let mut known = reply();
    known.outcome = OutcomeData::Control(Err(crate::personal::coordinator::UseError::AfterUse {
        cause: swarm_enrollment_state::Error::Canceled,
        outcome: Err(control::Error::Permission),
    }));
    assert_eq!(known.error_branches().len(), 2);
    let e = finish(Ok(known), Err(close_cause()), |_| panic!("must not send"))
        .err()
        .unwrap();
    assert_eq!(e.code(), "service-exchange-packet-close");
    assert!(e.known_reply().is_some());
    assert_eq!(e.error_branches().len(), 3);
    assert!(!format!("{e:?}").contains(PIN));
    assert!(e
        .error_branches()
        .iter()
        .any(|e| e.downcast_ref::<control::Error>().is_some()));
}
/// Real EPIPE retains the original known reply; a fresh pair remains usable.
#[test]
fn exchange_actual_output_failure_keeps_known_reply() {
    let (tx, rx) = pair();
    let mut c = channel::configure(&rx).unwrap();
    send(tx.as_raw_fd(), b"request-before-close", MsgFlags::empty()).unwrap();
    drop(tx);
    let e = run(&mut c, Zeroizing::new(vec![0; 128]), |_| Ok(reply()))
        .err()
        .unwrap();
    assert!(matches!(e.cause, Cause::Output(_)));
    assert!(e.known_reply().is_some());
    assert_eq!(e.error_branches().len(), 1);
    if let Cause::Output(output) = e.cause {
        assert!(matches!(output.cause, PacketOutputCause::Send(_)));
    }
    let (tx, rx) = pair();
    let mut c = channel::configure(&rx).unwrap();
    send(tx.as_raw_fd(), b"fresh", MsgFlags::empty()).unwrap();
    run(&mut c, Zeroizing::new(vec![0; 128]), |_| Ok(reply())).unwrap();
    assert!(recv(tx.as_raw_fd(), &mut [0; 4096], MsgFlags::empty()).unwrap() > 0);
}

/// Actual atomic process binding and canonical response on the same channel.
/// Namespace handles originate in an independent parent packet, not admission by
/// the first incoming request. Business callback is synthetic and same-UID only.
#[test]
fn exchange_atomic_child_actual_binding_and_response() {
    use channel::atomic_spawn::{self, Spec};
    use nix::sys::signal::SigSet;
    use std::{
        ffi::CString,
        os::fd::AsFd,
        panic::{catch_unwind, AssertUnwindSafe},
    };
    let (a, b) = pair();
    let mut independent = channel::configure(&b).unwrap();
    send(
        a.as_raw_fd(),
        b"independent-parent-domain",
        MsgFlags::MSG_NOSIGNAL,
    )
    .unwrap();
    let packet = independent.receive(Zeroizing::new(vec![0; 128])).unwrap();
    let uid = packet.credentials().1;
    let expected = packet.namespaces().unwrap();
    packet.close().unwrap();
    let (tx, rx) = pair();
    let mut c = channel::configure(&rx).unwrap();
    let program = CString::new("/usr/bin/python3").unwrap();
    let code="import socket; s=socket.socket(fileno=0); raw=s.recv(65536); s.send(raw); response=s.recv(65536); s.send(response) if response else None; s.recv(2) if response else None";
    let args = [
        CString::new("python3").unwrap(),
        CString::new("-c").unwrap(),
        CString::new(code).unwrap(),
    ];
    let directory = CString::new("/").unwrap();
    let null = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/null")
        .unwrap();
    let stderr = std::io::stderr();
    let mask = SigSet::empty();
    let defaults = SigSet::all();
    let launched = atomic_spawn::launch(&Spec {
        program: &program,
        arguments: &args,
        environment: &[],
        directory: &directory,
        stdin: tx.as_fd(),
        stdout: null.as_fd(),
        stderr: stderr.as_fd(),
        signal_mask: &mask,
        signal_defaults: &defaults,
    });
    let mut child = match launched {
        Ok(c) => c,
        Err(mut e) => {
            drop(c);
            drop(rx);
            drop(tx);
            if let Some(mut child) = e.child.take() {
                let waited = child.wait();
                let closed = child.close();
                panic!("launch refused: {e:?}; cleanup={waited:?}/{closed:?}")
            }
            panic!("launch refused: {e:?}")
        }
    };
    drop(tx);
    let result = catch_unwind(AssertUnwindSafe(|| {
        let raw = service_request::encode(PIN, &[63; 32], &Operation::Initialize).unwrap();
        c.send_packet(&raw).unwrap();
        let sent = run(&mut c, Zeroizing::new(vec![0; 65536]), |packet| {
            controller_packet::run_atomic(uid, packet, &child, &expected, |request| {
                assert_eq!(request.target_pin(), PIN);
                Ok(Reply {
                    challenge: *request.challenge(),
                    operation: request.operation().code(),
                    request_digest: request.public_digest(),
                    outcome: OutcomeData::Initialized(Ok(())),
                })
            })
        })
        .unwrap();
        let reflected = c.receive(Zeroizing::new(vec![0; 65536])).unwrap();
        let expected = swarm_wire_json::encode(&sent.observation).unwrap();
        assert_eq!(reflected.data(), expected.as_slice());
        reflected.close().unwrap();
        c.send_packet(b"go").unwrap();
    }));
    drop(c);
    drop(rx);
    let waited = child.wait();
    let closed = child.close();
    let domains = expected.close();
    if let Err(panic) = result {
        assert!(
            waited.is_ok() && closed.is_ok() && domains.is_ok(),
            "fixture cleanup refused"
        );
        std::panic::resume_unwind(panic)
    }
    assert!(waited.unwrap().success());
    closed.unwrap();
    domains.unwrap();
}
