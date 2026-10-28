//! Internal entropy/sink refusals only; no public deterministic source is added.
use super::*;
use std::{cell::Cell, io};
/// Synthetic explicit input only, not an operator preset or real storage path.
fn request() -> Request {
    serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/paea_provisioning_request.json"
    ))
    .unwrap()
}
/// One reproducible valid byte source for precondition/fault discrimination only.
fn fill(bytes: &mut [u8]) -> Result<(), Error> {
    bytes[..32].fill(1);
    bytes[32..64].fill(2);
    bytes[64..].fill(3);
    Ok(())
}
/// Establish an unchanged valid witness before every internal injected fault.
fn valid() {
    prepare_with_entropy(&request(), fill).unwrap();
}
/// Invalid operator input is rejected before the injected entropy source runs.
#[test]
fn provisioning_validation_precedes_entropy_for_all_operator_contracts() {
    let changes: Vec<fn(&mut Request)> = vec![
        |r| r.parameters.memory_kib = 0,
        |r| r.controller_uid = r.service_uid,
        |r| r.grant.generation = 2,
        |r| r.root_directory = "/private/..".into(),
        |r| r.personal_project_context_digest = r.grant.project_digest.clone(),
    ];
    for change in changes {
        valid();
        let mut bad = request();
        change(&mut bad);
        let called = Cell::new(false);
        assert!(prepare_with_entropy(&bad, |bytes| {
            called.set(true);
            fill(bytes)
        })
        .is_err());
        assert!(!called.get());
        valid();
    }
}
/// Entropy may fail after a prefix; guarded bytes drop with no retry/fallback.
#[test]
fn provisioning_partial_entropy_error_preserves_cause_without_retry() {
    valid();
    let calls = Cell::new(0);
    let e = prepare_with_entropy(&request(), |bytes| {
        calls.set(calls.get() + 1);
        bytes[..7].fill(42);
        Err(Error::Entropy(getrandom::Error::UNSUPPORTED))
    })
    .unwrap_err();
    assert_eq!(calls.get(), 1);
    assert_eq!(e.code(), "provisioning-entropy");
    assert!(e.source().unwrap().is::<getrandom::Error>());
    assert_eq!(
        format!("{e:?}"),
        "initial provisioning: provisioning-entropy"
    );
    valid();
}
/// Distinct keys are a provisioning invariant; collisions refuse, never redraw.
#[test]
fn provisioning_equal_keys_refuse_without_automatic_replacement() {
    valid();
    let calls = Cell::new(0);
    let e = prepare_with_entropy(&request(), |bytes| {
        calls.set(calls.get() + 1);
        bytes.fill(4);
        Ok(())
    })
    .unwrap_err();
    assert_eq!(e.code(), "provisioning-request");
    assert_eq!(calls.get(), 1);
    valid();
}
struct Sink {
    bytes: Vec<u8>,
    flush: bool,
}
impl Write for Sink {
    /// Preserve seven actual delivered bytes before the next write refuses.
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        if self.flush {
            self.bytes.extend_from_slice(b);
            return Ok(b.len());
        }
        if self.bytes.is_empty() {
            let n = b.len().min(7);
            self.bytes.extend_from_slice(&b[..n]);
            Ok(n)
        } else {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "PRIVATE_PARTIAL_SINK",
            ))
        }
    }
    /// A late refusal after all bytes were written must remain a typed failure.
    fn flush(&mut self) -> io::Result<()> {
        if self.flush {
            Err(io::Error::new(io::ErrorKind::Other, "PRIVATE_FLUSH_SINK"))
        } else {
            Ok(())
        }
    }
}
/// Both private outputs preserve partial/complete delivery on write/flush error.
#[test]
fn provisioning_private_partial_and_flush_errors_do_not_claim_no_effect() {
    valid();
    let p = prepare_with_entropy(&request(), fill).unwrap();
    for flush in [false, true] {
        for proof in [false, true] {
            let mut sink = Sink {
                bytes: vec![],
                flush,
            };
            let e = if proof {
                p.write_installation_proof(&mut sink)
            } else {
                p.write_personal_seed(&mut sink)
            }
            .unwrap_err();
            assert_eq!(sink.bytes.len(), if flush { 32 } else { 7 });
            assert_eq!(e.code(), "provisioning-output");
            assert_eq!(
                e.output_phase(),
                Some(if flush {
                    OutputPhase::WrittenFlushFailed
                } else {
                    OutputPhase::WriteUnknown
                })
            );
            assert!(e.source().unwrap().is::<io::Error>());
            assert!(!format!("{e:?} {e}").contains("PRIVATE_"));
        }
    }
}

/// Valid material witness, then reserved UID must refuse before the entropy
/// callback. No file or service account is created; values are synthetic data.
fn reserved_uid_refuses_before_entropy(service: bool) {
    valid();
    let mut bad = request();
    if service {
        bad.service_uid = u32::MAX;
    } else {
        bad.controller_uid = u32::MAX;
    }
    let called = Cell::new(false);
    let result = prepare_with_entropy(&bad, |bytes| {
        called.set(true);
        fill(bytes)
    });
    assert!(result.is_err(), "reserved UID reached material generation");
    assert!(!called.get(), "reserved UID consumed entropy");
    valid();
}
/// Service role sentinel is not an assignable Linux identity.
#[test]
fn provisioning_reserved_service_uid_refuses_before_entropy() {
    reserved_uid_refuses_before_entropy(true)
}
/// Controller role follows the same pre-entropy validation contract.
#[test]
fn provisioning_reserved_controller_uid_refuses_before_entropy() {
    reserved_uid_refuses_before_entropy(false)
}
