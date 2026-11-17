//! Source-only proposal: one shared behavioral probe for the exact adapted old
//! closure and R1a. This module is cfg(test), not a production CLI or authority.
//! Both variants compile in the SAME admitted R1a libtest. Legacy never calls
//! Context. It models old root/cache binding explicitly; it is not an old released
//! public binary. Global-wrapper coverage is a separate, fixed-only process probe.
#[path = "paired_legacy.rs"]
mod legacy;
use super::{resolution::Context, FixtureAreaError};
use serde_json::{json, Value};
use sha2::{Digest,Sha256};
use std::{error::Error, fs::{self, OpenOptions}, io::Write,
    os::unix::fs::PermissionsExt, path::{Path,PathBuf}, sync::{Arc,Barrier}};

/// Common callable contract: return PathBuf or panic, then the same effect
/// consumer. The old closure uses only Legacy; R1a uses the real resolved_path
/// converter. The effect is an inert one-byte journal, not a provider/action.
enum Backend { Legacy(legacy::Legacy), Fixed(Context) }
struct Capture { summary: Value, typed: Option<FixtureAreaError> }
impl Backend {
    /// Construct a fresh owned old/new cache for exactly the same fixture base.
    fn new(variant:&str,root:&Path)->Self {
        match variant { "legacy"=>Self::Legacy(legacy::Legacy::new(root.to_path_buf())),
            "fixed"=>Self::Fixed(Context::new(root.to_path_buf())), _=>panic!("pair-variant-invalid") }
    }
    /// Invoke one selected configurator and common dependent consumer, retaining
    /// typed causes but never arbitrary panic messages. Effect IO errors are
    /// harness/precondition failures, never a behavioral RED. No automatic retry.
    fn call(&self,root:&Path)->Capture {
        let mut performed=None;
        let outcome=std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let path=match self {Self::Legacy(c)=>c.resolve(),Self::Fixed(c)=> {
                let receipt=c.resolve();performed=Some(receipt.performed_access);
                super::resolved_path(receipt.outcome)
            }};
            path
        }));
        // Consumer IO is deliberately OUTSIDE the configurator panic catch.
        // An evidence-volume error cannot manufacture a legacy behavioral RED.
        if outcome.is_ok() {
            let mut file=OpenOptions::new().create(true).append(true).open(root.join("effect-count.bin")).expect("pair-effect-open");
            file.write_all(b"x").expect("pair-effect-write");file.sync_all().expect("pair-effect-sync");
        }
        capture(outcome,performed)
    }
}
/// Normalize the actual panic payload without converting it into an invented
/// typed failure. Opaque legacy panics remain opaque, with no guessed errno.
fn capture(outcome:std::thread::Result<PathBuf>,performed:Option<bool>)->Capture {
    match outcome {
        Ok(path)=>Capture{summary:json!({"kind":"path","path":path,"performed_access":performed}),typed:None},
        Err(payload)=>match payload.downcast::<FixtureAreaError>() {
            Ok(error)=> {
                let errno=error.source().and_then(|e|e.downcast_ref::<std::io::Error>()).and_then(|e|e.raw_os_error());
                let json_cause=error.source().is_some_and(|e|e.downcast_ref::<serde_json::Error>().is_some());
                let summary=json!({"kind":"refusal","typed":true,"stage":error.stage(),"origin":error.origin(),
                    "errno":errno,"json_cause":json_cause,"performed_access":performed,
                    "close_causes":error.close_causes().count(),"observation_causes":error.observation_causes().count()});
                Capture{summary,typed:Some(*error)}
            }
            Err(_)=>Capture{summary:json!({"kind":"refusal","typed":false,"performed_access":performed}),typed:None},
        }
    }
}
/// Require the caller's newly created inert root marker. This is a collision/
/// ownership check, not authentication, containment or permission to use live data.
fn owned_root()->PathBuf {
    let root=PathBuf::from(std::env::var_os("SWARM_CD01_PAIR_ROOT").expect("pair-root-required"));
    assert!(root.is_absolute() && root.is_dir(),"pair-root-not-admitted");
    assert!(!root.is_symlink(),"pair-root-alias");
    let marker=fs::read(root.join(".cd01-pair-owner")).expect("pair-owner-marker");
    assert_eq!(marker,std::env::var("SWARM_CD01_PAIR_TOKEN").expect("pair-token-required").as_bytes());
    root
}
/// Create only owned fixture settings with saved A distinct from default B.
/// No global Context/live settings or compiler input is mutated by this setup.
fn setup(root:&Path) {
    fs::create_dir(root.join(".swarm")).expect("pair-saved-dir-fresh");
    fs::create_dir_all(root.join("rust/crates/swarm-engine/config")).expect("pair-default-dir");
    fs::write(root.join("rust/crates/swarm-engine/config/engine-settings.json"),br#"{"runtime_temporary_directory":"default-B"}"#).expect("pair-default");
    saved(root,br#"{"values":{"runtime_temporary_directory":"saved-A"}}"#);
}
/// Configure the inert owned saved document; no adoption/migration API is used.
fn saved(root:&Path,bytes:&[u8]) { fs::write(root.join(".swarm/engine-settings.json"),bytes).expect("pair-write-saved"); }
/// Read the real dependent journal length; missing means no effect yet.
fn effects(root:&Path)->u64 {match fs::metadata(root.join("effect-count.bin")) {
    Ok(m)=>m.len(),Err(e) if e.kind()==std::io::ErrorKind::NotFound=>0,Err(_)=>panic!("pair-effect-metadata")}}
/// Publish one fresh JSON report. Existing reports refuse; sync failures remain
/// failures. Partial outputs are incomplete, never PASS or retry permission.
fn publish(root:&Path,value:&Value) {
    let mut f=OpenOptions::new().write(true).create_new(true).open(root.join("probe.json")).expect("pair-report-fresh");
    f.write_all(&serde_json::to_vec_pretty(value).expect("pair-report-json")).expect("pair-report-write");
    f.sync_all().expect("pair-report-sync");
}

#[test]
#[ignore = "requires a root-admitted C-D01 fixture and explicit controller"]
/// The single shared named probe. A healthy A witness using the same backend and
/// effect consumer precedes each fault. Probe success means complete observations,
/// NOT that the selected implementation meets the target contract. The Python
/// driver judges the named before/after behavioral relations. No missing Context
/// symbol or failed compilation is a RED; no test is run during source preparation.
fn paired_probe() {
    let root=owned_root();setup(&root);
    let variant=std::env::var("SWARM_CD01_PAIR_VARIANT").expect("pair-variant");
    let case=std::env::var("SWARM_CD01_PAIR_CASE").expect("pair-case");
    assert!(["presence","missing","json","permission","cache","concurrency"].contains(&case.as_str()),"pair-case-invalid");
    let control=Backend::new(&variant,&root).call(&root);
    assert_eq!(control.summary["kind"],"path","pair-control-not-valid");
    assert_eq!(control.summary["path"],json!(root.join("saved-A")),"pair-control-wrong-value");
    let before=effects(&root);assert_eq!(before,1,"pair-control-no-effect");
    let identity=json!({"euid":nix::unistd::geteuid().as_raw(),"egid":nix::unistd::getegid().as_raw(),
        "root_mode":fs::metadata(&root).expect("pair-root-stat").permissions().mode() & 0o7777,
        "saved_mode":fs::metadata(root.join(".swarm/engine-settings.json")).expect("pair-saved-stat").permissions().mode() & 0o7777});
    let readable_control_bytes=fs::read(root.join(".swarm/engine-settings.json")).expect("pair-control-bytes");
    match case.as_str() {
        "missing"=>saved(&root,br#"{"values":{}}"#),
        "json"|"cache"|"concurrency"=>saved(&root,b"{"),
        "permission"=> {
            assert!(!nix::unistd::geteuid().is_root(),"pair-eacces-identity-not-admitted");
            let leaf=root.join(".swarm/engine-settings.json");
            fs::set_permissions(&leaf,fs::Permissions::from_mode(0o000)).expect("pair-chmod");
            // This independent preflight establishes the fault is discriminating;
            // it is not substituted for the configurator's own access/cause proof.
            let errno=fs::read(&leaf).err().and_then(|e|e.raw_os_error());
            if errno!=Some(13) {fs::set_permissions(&leaf,fs::Permissions::from_mode(0o600)).expect("pair-repair-mode");panic!("pair-eacces-not-discriminating");}
        }
        _=>{}
    }
    let fault_bytes=if case=="permission" {readable_control_bytes}
        else {fs::read(root.join(".swarm/engine-settings.json")).expect("pair-fault-bytes")};
    fs::write(root.join("fault-input.bin"),&fault_bytes).expect("pair-fault-input-proof");
    let backend=Arc::new(Backend::new(&variant,&root));
    let calls:Vec<Capture>=if case=="concurrency" {
        let barrier=Arc::new(Barrier::new(4));
        let jobs:Vec<_>=(0..4).map(|_|{let backend=backend.clone();let root=root.clone();let barrier=barrier.clone();
            std::thread::spawn(move||{barrier.wait();backend.call(&root)})}).collect();
        jobs.into_iter().map(|j|j.join().expect("pair-thread-join")).collect()
    } else {vec![backend.call(&root)]};
    let after_fault=effects(&root);
    let same_refusal=calls.first().and_then(|c|c.typed.as_ref()).map(|first|
        calls.iter().all(|c|c.typed.as_ref().is_some_and(|e|first.same_resolution(e))));
    // Change only the known owned prerequisite. Same context must not adopt it.
    if case=="permission" {fs::set_permissions(root.join(".swarm/engine-settings.json"),fs::Permissions::from_mode(0o600)).expect("pair-repair-mode");}
    let after_fault_bytes=fs::read(root.join(".swarm/engine-settings.json")).expect("pair-after-fault-bytes");
    fs::write(root.join("after-fault-input.bin"),&after_fault_bytes).expect("pair-after-fault-input-proof");
    let settings_unchanged=fault_bytes==after_fault_bytes;
    saved(&root,br#"{"values":{"runtime_temporary_directory":"new-B"}}"#);
    let retained=backend.call(&root);let after_retained=effects(&root);
    let same_retained=calls[0].typed.as_ref().and_then(|a|retained.typed.as_ref().map(|b|a.same_resolution(b)));
    let recovery=Backend::new(&variant,&root).call(&root);let after_recovery=effects(&root);
    assert_eq!(recovery.summary["kind"],"path","pair-recovery-refused");
    assert_eq!(recovery.summary["path"],json!(root.join("new-B")),"pair-recovery-wrong-value");
    publish(&root,&json!({"schema_version":1,"scope":"owned-adapted-context-pair","variant":variant,"case":case,
        "identity":identity,"settings_unchanged":settings_unchanged,
        "fault_bytes_sha256":format!("{:x}",Sha256::digest(&fault_bytes)),
        "after_fault_bytes_sha256":format!("{:x}",Sha256::digest(&after_fault_bytes)),
        "control":control.summary,"observed":calls.iter().map(|c|c.summary.clone()).collect::<Vec<_>>(),
        "retained":retained.summary,"recovery":recovery.summary,"same_refusal_generation":same_refusal,
        "same_retained_refusal":same_retained,"effects":{"before_fault":before,"after_fault":after_fault,
        "after_retained":after_retained,"after_recovery":after_recovery},"global_wrapper_exercised":false}));
    println!("CD01_PAIR_PROBE_COMPLETE {variant} {case}");
}

#[test]
#[ignore = "requires a root-admitted C-D01 fixture and explicit controller"]
/// Separate AFTER-only public-wrapper process probe. Requires a root-selected
/// fresh owned compiler snapshot whose actual CARGO_MANIFEST_DIR-derived repo
/// equals the marker root; the driver separately refuses shared/Git checkouts.
/// Modes: healthy control, two wrapper calls around repair retaining the typed
/// refusal, then adoption in another fresh process. No cache reset or guessed
/// repo-root override. Marker ownership is not a confinement/security guarantee.
/// This AFTER-only probe never counts as legacy RED.
fn global_wrapper_probe() {
    let root=owned_root();assert_eq!(root,super::repo_root(),"pair-global-compiler-root-mismatch");
    let case=std::env::var("SWARM_CD01_PAIR_CASE").expect("pair-global-case");
    if case=="adoption" {
        let value=super::configured_area();assert_eq!(value,root.join("new-B"));
        publish(&root,&json!({"schema_version":1,"scope":"fixed-global-wrapper-process","case":case,"path":value,"fresh_process":true}));
    } else {
        assert!(["control","refusal"].contains(&case.as_str()),"pair-global-case-invalid");
        assert!(!root.join(".swarm").exists(),"pair-global-settings-not-fresh");
        // Do NOT call setup: this is the real owned compiler repository and its
        // embedded config is an immutable compiler input, not fixture default B.
        fs::create_dir(root.join(".swarm")).expect("pair-global-saved-fresh");
        saved(&root,br#"{"values":{"runtime_temporary_directory":"saved-A"}}"#);
        if case=="control" {
            let value=super::configured_area();assert_eq!(value,root.join("saved-A"));
            publish(&root,&json!({"schema_version":1,"scope":"fixed-global-wrapper-process","case":case,"path":value}));
        } else {
            saved(&root,b"{");
            let first=capture(std::panic::catch_unwind(super::configured_area),None);
            assert_eq!(first.summary["kind"],"refusal");assert!(first.typed.is_some(),"pair-global-payload-not-typed");
            saved(&root,br#"{"values":{"runtime_temporary_directory":"new-B"}}"#);
            let second=capture(std::panic::catch_unwind(super::configured_area),None);
            assert!(first.typed.as_ref().unwrap().same_resolution(second.typed.as_ref().expect("pair-global-retry-in-same-process")));
            publish(&root,&json!({"schema_version":1,"scope":"fixed-global-wrapper-process","case":case,
                "first":first.summary,"second":second.summary,"same_refusal_generation":true,"no_cache_reset":true}));
        }
    }
    println!("CD01_GLOBAL_WRAPPER_COMPLETE {case}");
}
