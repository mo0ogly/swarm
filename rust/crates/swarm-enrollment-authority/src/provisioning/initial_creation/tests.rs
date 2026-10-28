//! Real private FS faults plus counted generator invocation; no live state.
use super::*;
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use swarm_enrollment_config::store::FixtureFault as F;
use zeroize::Zeroizing;
struct Case {
    root: tempfile::TempDir,
    op: PathBuf,
    anchor: PathBuf,
    request: material::Request,
    raw: Vec<u8>,
    pin: String,
}
impl Case {
    /// Synthetic explicit current-controller data, never actual separate accounts.
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let op = root.path().join("operator");
        let a = root.path().join("anchor");
        for d in [&op, &a] {
            fs::create_dir(d).unwrap();
            fs::set_permissions(d, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let mut request: material::Request = serde_json::from_str(include_str!(
            "../../../../../../tests/fixtures/paea_provisioning_request.json"
        ))
        .unwrap();
        let uid = rustix::process::geteuid().as_raw();
        request.controller_uid = uid;
        request.service_uid = if uid == 0 { 1 } else { !uid };
        request.root_directory = root.path().join("absent-service").to_str().unwrap().into();
        let raw = material::encode_request(&request).unwrap();
        let pin = format!("{:x}", Sha256::digest(&raw));
        Self {
            root,
            op,
            anchor: a.join("pin.json"),
            request,
            raw,
            pin,
        }
    }
    /// Real production-owned operation with fixture-configured FS faults and a
    /// counting seam that still calls actual material::prepare. No new keys in logs.
    fn run(
        &self,
        op_faults: &[F],
        anchor_faults: &[F],
        fail_prepare: bool,
        cancel: bool,
    ) -> (Result<checkpoint::Checkpoint, Cause>, Progress, usize) {
        validate(&self.raw, &self.pin, rustix::process::geteuid().as_raw()).unwrap();
        let (op, a) = open_pair(
            &self.op,
            self.anchor.parent().unwrap(),
            rustix::process::geteuid().as_raw(),
        )
        .unwrap();
        op.fixture_faults(op_faults);
        a.fixture_faults(anchor_faults);
        let ctx = Context::new();
        if cancel {
            ctx.cancel();
        }
        let mut progress = Progress::default();
        progress.request_pin_matched = true;
        let mut calls = 0;
        let result = create_owned(
            &self.request,
            &self.raw,
            &self.pin,
            op,
            a,
            "pin.json",
            &ctx,
            &mut progress,
            &mut |r| {
                calls += 1;
                if fail_prepare {
                    Err(material::Error::Request)
                } else {
                    material::prepare(r)
                }
            },
        );
        (result, progress, calls)
    }
    /// Explicit public read reconciliation only, after real handles closed.
    fn recover(&self) -> Result<Receipt, Failure> {
        reconcile(
            &self.raw,
            &self.pin,
            &self.op,
            &self.anchor,
            &Context::new(),
        )
    }
    /// Current private repository singleton, not a discovered service root/head.
    fn package(&self) -> PathBuf {
        self.op.join("initial-provisioning-v1.bin")
    }
}
/// Positive real generator/IO/reconcile witness before every fault injection.
fn valid() {
    let c = Case::new();
    let (result, p, calls) = c.run(&[], &[], false, false);
    assert!(result.is_ok());
    assert_eq!(calls, 1);
    assert_eq!(p.package_publication, Publication::Durable);
    assert!(c.recover().unwrap().progress.package_request_matched);
    assert!(!c.root.path().join("absent-service").exists());
}
/// Intent temp-name/write/sync failure precedes fresh key/proof generation. The
/// shared temporary filename may use entropy; generator counter must stay zero.
#[test]
fn initial_intent_fs_failures_precede_material_and_preserve_slot() {
    for fault in [
        F::Entropy,
        F::Write,
        F::PartialWrite,
        F::FileSync,
        F::FileClose,
        F::DirectorySync,
    ] {
        valid();
        let c = Case::new();
        let (result, p, calls) = c.run(&[], &[fault], false, false);
        assert!(result.is_err());
        assert_eq!(calls, 0);
        assert!(!p.material_generation_attempted);
        assert_eq!(p.package_publication, Publication::NotAttempted);
        assert!(!c.package().exists());
        assert!(!c.anchor.exists());
        if matches!(fault, F::DirectorySync) {
            assert_eq!(p.intent_publication, Publication::Visible);
            let (_, again, repeated) = c.run(&[], &[], false, false);
            assert_eq!(repeated, 0);
            assert!(!again.material_generation_attempted);
        }
    }
}
/// Generation failure after durable intent keeps the slot closed to generation;
/// orphan reconciliation refuses explicitly, never succeeds empty or resets.
#[test]
fn initial_material_failure_retains_intent_and_refuses_regeneration() {
    valid();
    let c = Case::new();
    let (result, p, calls) = c.run(&[], &[], true, false);
    assert!(result.is_err());
    assert_eq!(calls, 1);
    assert!(p.material_generation_attempted && !p.material_generation_completed);
    assert_eq!(p.intent_publication, Publication::Durable);
    assert!(!c.anchor.exists() && !c.package().exists());
    let (_, again, repeated) = c.run(&[], &[], false, false);
    assert_eq!(repeated, 0);
    assert!(!again.material_generation_attempted);
    let e = c.recover().unwrap_err();
    assert!(e.progress.intent_decoded);
    assert!(!e.progress.material_generation_attempted);
    assert!(!c.package().exists());
}
/// Once pin is durably recorded, generated material must be conserved before
/// late anchor cleanup can fail. Failure stays failure; exact recovery is read-only.
#[test]
fn initial_anchor_cleanup_keeps_pinned_material_for_explicit_recovery() {
    valid();
    let c = Case::new();
    let (result, p, calls) = c.run(&[], &[F::Unlock, F::StoreClose], false, false);
    let cause = result.unwrap_err();
    assert_eq!(calls, 1);
    assert_eq!(p.intent_publication, Publication::Durable);
    assert_eq!(p.anchor_publication, Publication::Durable);
    assert_eq!(
        p.package_publication,
        Publication::Durable,
        "durable-pin-but-generated-material-not-conserved"
    );
    assert!(matches!(cause, Cause::Joined { .. }));
    assert!(c.anchor.exists() && c.package().exists());
    let original = Zeroizing::new(fs::read(c.package()).unwrap());
    assert!(c.recover().unwrap().progress.package_request_matched);
    assert!(
        fs::read(c.package()).unwrap().as_slice() == original.as_slice(),
        "recovery-rewrote-package"
    );
    let (_, _, repeated) = c.run(&[], &[], false, false);
    assert_eq!(repeated, 0);
}

/// Package rename followed by sync refusal keeps visible exact bytes and pin;
/// explicit read/ack reconciliation succeeds without replacement or generation.
#[test]
fn initial_package_visible_failure_reconciles_same_material() {
    valid();
    let c = Case::new();
    let (result, p, calls) = c.run(&[F::DirectorySync], &[], false, false);
    assert!(result.is_err());
    assert_eq!(calls, 1);
    assert_eq!(p.intent_publication, Publication::Durable);
    assert_eq!(p.anchor_publication, Publication::Durable);
    assert_eq!(p.package_publication, Publication::Visible);
    let raw = Zeroizing::new(fs::read(c.package()).unwrap());
    let recovered = c.recover().unwrap();
    assert!(!recovered.progress.material_generation_attempted);
    assert_eq!(
        recovered.progress.package_durability,
        ReadDurability::DirectoryAcknowledged
    );
    assert!(
        fs::read(c.package()).unwrap().as_slice() == raw.as_slice(),
        "reconcile-rewrote-material"
    );
    let (_, _, again) = c.run(&[], &[], false, false);
    assert_eq!(again, 0);
}
/// Operator late release/close cannot erase known durable material; both errors
/// stay failure and readable recovery uses the retained intent/anchor/package.
#[test]
fn initial_operator_cleanup_keeps_durable_package_and_all_causes() {
    valid();
    let c = Case::new();
    let (result, p, calls) = c.run(&[F::Unlock, F::StoreClose], &[], false, false);
    let cause = result.unwrap_err();
    assert_eq!(calls, 1);
    assert_eq!(p.package_publication, Publication::Durable);
    assert!(matches!(cause, Cause::Joined { .. }));
    assert!(c.recover().unwrap().progress.package_request_matched);
    let (_, _, again) = c.run(&[], &[], false, false);
    assert_eq!(again, 0);
}
/// Existing explicit cancellation has no timer or retry; no material/intent is
/// attempted, all custodians close and a fresh explicit context can create once.
#[test]
fn initial_cancelled_context_refuses_before_material_and_recovers() {
    valid();
    let c = Case::new();
    let (result, p, calls) = c.run(&[], &[], false, true);
    assert!(result.is_err());
    assert_eq!(calls, 0);
    assert!(!p.material_generation_attempted);
    assert_eq!(p.intent_publication, Publication::NotAttempted);
    assert!(!c.anchor.exists() && !c.package().exists());
    let (result, _, calls) = c.run(&[], &[], false, false);
    assert!(result.is_ok());
    assert_eq!(calls, 1);
}
/// Pure wrapper matrix, including constructed Unknown metadata (not a real OS
/// uncertain rename injection). Custom IO and independent owners remain typed.
#[test]
fn initial_custom_io_cleanup_and_unknown_metadata_keep_every_branch() {
    let cause = Cause::Joined {
        cause: Box::new(Cause::Operator(config::Error::Io {
            class: config::IoClass::Storage,
            cause: std::io::Error::other(config::Error::Joined {
                cause: Box::new(config::Error::State),
                cleanup: Box::new(config::Error::Pins),
            }),
        })),
        cleanup: config::Error::Closed,
        owner: Owner::Anchor,
    };
    let mut progress = Progress::default();
    progress.package_publication = Publication::Unknown;
    let error = Failure { progress, cause };
    let mut pending: Vec<&(dyn StdError + 'static)> = vec![&error];
    let mut seen: Vec<*const dyn StdError> = vec![];
    let (mut state, mut pins, mut closed) = (false, false, false);
    let mut labels = vec![];
    while let Some(e) = pending.pop() {
        let id = e as *const dyn StdError;
        if seen.iter().any(|old| std::ptr::eq(*old, id)) {
            continue;
        }
        seen.push(id);
        state |= matches!(
            e.downcast_ref::<config::Error>(),
            Some(config::Error::State)
        );
        pins |= matches!(e.downcast_ref::<config::Error>(), Some(config::Error::Pins));
        closed |= matches!(
            e.downcast_ref::<config::Error>(),
            Some(config::Error::Closed)
        );
        let (l, c) = error_links(e);
        labels.extend(l);
        pending.extend(c);
        if let Some(source) = e.source() {
            pending.push(source);
        }
    }
    assert!(state && pins && closed);
    assert!(labels
        .iter()
        .any(|s| s == "initial-package-publication:Unknown"));
    assert!(labels.iter().any(|s| s == "initial-cleanup-owner:Anchor"));
    assert!(!format!("{error}").contains(&"ab".repeat(32)));
}
