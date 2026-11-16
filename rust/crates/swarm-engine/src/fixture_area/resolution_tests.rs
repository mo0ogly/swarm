//! Proposed R1 unit guards; source only, not executed/compiled for this packet.
//! All configuration lives in newly owned fixture directories. These guards do
//! not touch the process-global Context or live .swarm settings. Real wrapper
//! conversion is exercised through resolved_path, not a Display-string double.
use super::resolution::{AccessEvent, Context, FixtureAreaError, Hooks};
use std::{error::Error, fs::{self, File}, io, os::unix::fs::{symlink, DirBuilderExt},
    path::{Path, PathBuf}, sync::{Arc, atomic::{AtomicUsize, Ordering}}};

/// Create an explicitly retained fixture; success/failure never removes its
/// evidence. HOME fallback is the existing allocator contract, not a new policy.
fn fixture(label: &str) -> PathBuf {
    let root = super::fallback_area().join(crate::generate_identifier(label).unwrap());
    fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
    fs::create_dir_all(root.join("rust/crates/swarm-engine/config")).unwrap();
    fs::write(root.join("rust/crates/swarm-engine/config/engine-settings.json"),
        br#"{"runtime_temporary_directory":"default-B"}"#).unwrap();
    fs::create_dir(root.join(".swarm")).unwrap();
    fs::write(root.join(".swarm/engine-settings.json"),
        br#"{"values":{"runtime_temporary_directory":"saved-A"}}"#).unwrap();
    fs::write(root.join("fixture-owner.txt"),b"C-D01/R1 owned inert settings fixture; no service/provider\n").unwrap();
    root
}
/// Verify the same real resolver's valid saved witness before injecting a fault.
fn healthy(root: &Path) {
    let receipt = Context::new(root.to_path_buf()).resolve();
    assert!(receipt.performed_access);
    let resolved = receipt.outcome.unwrap();
    assert_eq!(resolved.path,root.join("saved-A"));
    assert_eq!(resolved.base,root.to_path_buf());
    assert_eq!(resolved.source,"saved");
    assert!(resolved.trace.iter().any(|e|e.phase=="consumed" && e.source=="saved" && e.identity.is_some() && e.consumed_sha256.is_some()));
    assert!(!root.join("saved-A").exists(),"resolution itself must not allocate");
}
/// Write explicit fixture configuration only; no live settings API or adoption.
fn saved(root: &Path, bytes: &[u8]) { fs::write(root.join(".swarm/engine-settings.json"),bytes).unwrap(); }

#[test]
/// Real absence differs from a present missing/null field; recover with fresh
/// context and explicit saved configuration, never a reset of the retained cell.
fn absence_missing_null_and_recovery_are_distinct() {
    let root = fixture("cd01-absence-"); healthy(&root);
    fs::remove_file(root.join(".swarm/engine-settings.json")).unwrap();
    let missing_leaf = Context::new(root.clone()).resolve().outcome.unwrap();
    assert_eq!(missing_leaf.path,root.join("default-B"));
    assert_eq!(missing_leaf.source,"default");
    assert!(missing_leaf.trace.iter().any(|e|e.phase=="absent"));
    fs::remove_dir(root.join(".swarm")).unwrap();
    assert_eq!(Context::new(root.clone()).resolve().outcome.unwrap().source,"default");
    fs::create_dir(root.join(".swarm")).unwrap();
    for bytes in [br#"{"values":{}}"#.as_slice(),br#"{"values":{"runtime_temporary_directory":null}}"#.as_slice(),br#"{}"#.as_slice()] {
        saved(&root,br#"{"values":{"runtime_temporary_directory":"saved-A"}}"#);healthy(&root);
        saved(&root,bytes);
        let context = Context::new(root.clone());
        let error = context.resolve().outcome.unwrap_err();
        assert_eq!(error.stage(),"settings-runtime-temporary-not-configured");
        assert_eq!(fs::read(root.join(".swarm/engine-settings.json")).unwrap(),bytes);
        saved(&root,br#"{"values":{"runtime_temporary_directory":"saved-A"}}"#);
        let cached = context.resolve(); assert!(!cached.performed_access);
        assert!(error.same_resolution(&cached.outcome.unwrap_err()));
        healthy(&root);
    }
    // Broken shipped data after proven absence refuses instead of another fallback.
    fs::remove_file(root.join(".swarm/engine-settings.json")).unwrap();
    fs::write(root.join("rust/crates/swarm-engine/config/engine-settings.json"),b"{").unwrap();
    let error = Context::new(root.clone()).resolve().outcome.unwrap_err();
    assert_eq!(error.origin(),"default");assert_eq!(error.stage(),"settings-json");
    fs::write(root.join("rust/crates/swarm-engine/config/engine-settings.json"),br#"{"runtime_temporary_directory":"default-B"}"#).unwrap();
    assert_eq!(Context::new(root).resolve().outcome.unwrap().source,"default");
}

#[test]
/// Every malformed shape/type has a valid same-access witness first; JSON causes
/// remain typed, refused values do not leak, and no dependent action is dispatched.
fn invalid_data_keeps_original_cause_and_stops_wrapper_consumer() {
    let root=fixture("cd01-shape-");
    for bytes in [b"{".as_slice(),b"[]",br#"{"values":null}"#,br#"{"values":[]}"#,
        br#"{"values":{"runtime_temporary_directory":true}}"#,br#"{"values":{"runtime_temporary_directory":7}}"#,
        br#"{"values":{"runtime_temporary_directory":[]}}"#,br#"{"values":{"runtime_temporary_directory":{}}}"#] {
        saved(&root,br#"{"values":{"runtime_temporary_directory":"saved-A"}}"#);healthy(&root);
        saved(&root,bytes);
        let context=Context::new(root.clone());let error=context.resolve().outcome.unwrap_err();
        assert!(!error.access_observations().any(|(_,source,_)|source=="default"));
        let actions=AtomicUsize::new(0);
        let caught=std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _path=super::resolved_path(context.resolve().outcome);
            actions.fetch_add(1,Ordering::SeqCst);
        })).unwrap_err();
        let payload=caught.downcast::<FixtureAreaError>().expect("actual wrapper must preserve typed payload");
        assert!(payload.same_resolution(&error));assert_eq!(actions.load(Ordering::SeqCst),0);
        if bytes==b"{" {assert!(payload.source().unwrap().downcast_ref::<serde_json::Error>().is_some());}
        saved(&root,br#"{"values":{"runtime_temporary_directory":"saved-A"}}"#);healthy(&root);
    }
}

#[test]
/// Preserve current path grammar, including accepted /tmpx and whitespace; no
/// normalization repairs invalid raw text. Future execution must use safe paths.
fn grammar_is_reference_based_not_normalized() {
    let root=fixture("cd01-grammar-");
    for raw in ["","/","/tmp","/tmp/a","a/","a//b","./a","a/../b","//a","a\0b"] {
        saved(&root,br#"{"values":{"runtime_temporary_directory":"saved-A"}}"#);healthy(&root);
        saved(&root,serde_json::to_vec(&serde_json::json!({"values":{"runtime_temporary_directory":raw}})).unwrap().as_slice());
        let error=Context::new(root.clone()).resolve().outcome.unwrap_err();
        assert_eq!(error.stage(),"settings-path");
        let settings=error.source().unwrap().downcast_ref::<crate::engine_settings::EngineSettingsError>().unwrap();
        assert_eq!(settings.reason(),"settings-runtime-temporary-invalid");
        assert!(!format!("{error:?}").contains(raw) || raw.is_empty());
    }
    for raw in ["relative-A","tmp/x","/tmpx"," "] {
        saved(&root,serde_json::to_vec(&serde_json::json!({"values":{"runtime_temporary_directory":raw}})).unwrap().as_slice());
        let value=Context::new(root.clone()).resolve().outcome.unwrap();
        assert_eq!(value.path,if raw.starts_with('/') {PathBuf::from(raw)} else {root.join(raw)});
    }
    saved(&root,br#"{"values":{"runtime_temporary_directory":"saved-A"}}"#);healthy(&root);
}

#[test]
/// Directory/FIFO/dangling link refuse without reading or selecting default;
/// an existing regular-file symlink remains allowed, without confinement claims.
fn special_entries_refuse_and_valid_symlink_recovers() {
    let root=fixture("cd01-special-");healthy(&root);
    let leaf=root.join(".swarm/engine-settings.json");fs::remove_file(&leaf).unwrap();
    fs::create_dir(&leaf).unwrap();
    assert_eq!(Context::new(root.clone()).resolve().outcome.unwrap_err().stage(),"regular-file-required");
    fs::remove_dir(&leaf).unwrap();
    saved(&root,br#"{"values":{"runtime_temporary_directory":"saved-A"}}"#);healthy(&root);fs::remove_file(&leaf).unwrap();
    rustix::fs::mknodat(rustix::fs::CWD,&leaf,rustix::fs::FileType::Fifo,rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,0).unwrap();
    assert_eq!(Context::new(root.clone()).resolve().outcome.unwrap_err().stage(),"regular-file-required");
    fs::remove_file(&leaf).unwrap();
    saved(&root,br#"{"values":{"runtime_temporary_directory":"saved-A"}}"#);healthy(&root);fs::remove_file(&leaf).unwrap();
    symlink("missing-target",&leaf).unwrap();
    let error=Context::new(root.clone()).resolve().outcome.unwrap_err();
    assert_eq!(error.stage(),"saved-open-entry-present");
    assert!(error.source().unwrap().downcast_ref::<io::Error>().is_some());
    fs::remove_file(&leaf).unwrap();
    let target=root.join("valid.json");fs::write(&target,br#"{"values":{"runtime_temporary_directory":"saved-A"}}"#).unwrap();
    symlink(&target,&leaf).unwrap();healthy(&root);
}

#[test]
/// Replacing the saved parent after it is held cannot redirect the leaf read.
/// This proves only descriptor-relative source selection, not frozen contents.
fn parent_replacement_reads_held_source() {
    struct Replace { root: PathBuf }
    impl Hooks for Replace {
        /// Replace the parent pathname with a different valid B document once.
        fn parent_opened(&mut self,_parent:&File)->io::Result<()> {
            fs::rename(self.root.join(".swarm"),self.root.join("old-parent"))?;
            fs::create_dir(self.root.join(".swarm"))?;
            fs::write(self.root.join(".swarm/engine-settings.json"),br#"{"values":{"runtime_temporary_directory":"new-B"}}"#)
        }
    }
    let root=fixture("cd01-parent-");healthy(&root);
    let value=Context::new(root.clone()).resolve_with(&mut Replace{root:root.clone()}).outcome.unwrap();
    assert_eq!(value.path,root.join("saved-A"));
    assert_eq!(Context::new(root.clone()).resolve().outcome.unwrap().path,root.join("new-B"));
}

#[test]
/// Inject read/close/observer causes after a valid witness, preserve original
/// branches through the real panic wrapper, then recover using a fresh context.
/// Close faults are synthetic after actual release, not kernel-close witnesses.
fn read_close_observer_causes_are_received_and_refusal_is_cached() {
    struct Fault;
    impl Hooks for Fault {
        /// Inject the saved-read error on an actually opened regular file.
        fn before_read(&mut self,source:&'static str,_:&File)->io::Result<()> {
            if source=="saved" {Err(io::Error::from_raw_os_error(13))} else {Ok(())}
        }
        /// Retain a secondary observer error without replacing the read cause.
        fn observe(&mut self,_:&AccessEvent)->io::Result<()> {Err(io::Error::new(io::ErrorKind::Other,"private-observer-text"))}
        /// Add an explicit known-released synthetic close failure.
        fn after_release(&mut self,_:&'static str)->io::Result<()> {Err(io::Error::from_raw_os_error(5))}
    }
    let root=fixture("cd01-causes-");healthy(&root);
    let context=Context::new(root.clone());let error=context.resolve_with(&mut Fault).outcome.unwrap_err();
    assert_eq!(error.source().unwrap().downcast_ref::<io::Error>().unwrap().raw_os_error(),Some(13));
    assert_eq!(error.close_causes().count(),3);
    assert!(error.close_causes().all(|(_,fd,uncertain,cause)|fd>=0 && !uncertain && cause.downcast_ref::<io::Error>().unwrap().raw_os_error()==Some(5)));
    assert!(error.observation_causes().count()>0);
    assert!(!format!("{error:?}").contains("private-observer-text"));
    let caught=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||super::resolved_path(context.resolve().outcome))).unwrap_err();
    assert!(caught.downcast::<FixtureAreaError>().unwrap().same_resolution(&error));
    assert!(!context.resolve().performed_access);
    healthy(&root);
}

#[test]
/// After a healthy control, observer-only and close-only faults independently
/// refuse; neither becomes success because no primary read error exists.
fn secondary_only_failures_are_not_success() {
    struct Observer;
    impl Hooks for Observer {
        /// Fail safe-event publication while allowing the actual read.
        fn observe(&mut self,_:&AccessEvent)->io::Result<()> {Err(io::Error::from_raw_os_error(5))}
    }
    struct Close;
    impl Hooks for Close {
        /// Simulate failure diagnostics only after actual release.
        fn after_release(&mut self,_:&'static str)->io::Result<()> {Err(io::Error::from_raw_os_error(5))}
    }
    let root=fixture("cd01-secondary-");healthy(&root);
    let observer=Context::new(root.clone()).resolve_with(&mut Observer).outcome.unwrap_err();
    assert_eq!(observer.stage(),"resolution-observation");
    assert_eq!(observer.source().unwrap().downcast_ref::<io::Error>().unwrap().raw_os_error(),Some(5));
    healthy(&root);
    let closing=Context::new(root.clone()).resolve_with(&mut Close).outcome.unwrap_err();
    assert_eq!(closing.stage(),"content-close");assert_eq!(closing.close_causes().count(),3);
    healthy(&root);
}

#[test]
/// Concurrent first callers share exactly one completed generation. Configuration
/// replacement affects only a fresh owned context, never the retained success.
fn concurrent_context_initialization_and_new_adoption_are_explicit() {
    let root=fixture("cd01-cache-");healthy(&root);
    let context=Arc::new(Context::new(root.clone()));
    let threads:Vec<_>=(0..4).map(|_| {let context=context.clone();std::thread::spawn(move||context.resolve())}).collect();
    let receipts:Vec<_>=threads.into_iter().map(|t|t.join().unwrap()).collect();
    assert_eq!(receipts.iter().filter(|r|r.performed_access).count(),1);
    let first=receipts[0].outcome.as_ref().unwrap();
    assert!(receipts.iter().all(|r|Arc::ptr_eq(first,r.outcome.as_ref().unwrap())));
    saved(&root,br#"{"values":{"runtime_temporary_directory":"new-B"}}"#);
    let cached=context.resolve();assert!(!cached.performed_access);
    assert_eq!(cached.outcome.unwrap().path,root.join("saved-A"));
    assert_eq!(Context::new(root.clone()).resolve().outcome.unwrap().path,root.join("new-B"));
}

#[test]
/// A refused initial generation is also shared by concurrent first callers;
/// repairing fixture bytes cannot trigger implicit re-access in that context.
fn concurrent_refusal_is_retained_until_new_context() {
    let root=fixture("cd01-refusal-cache-");healthy(&root);
    saved(&root,b"{");
    let context=Arc::new(Context::new(root.clone()));
    let barrier=Arc::new(std::sync::Barrier::new(4));
    let threads:Vec<_>=(0..4).map(|_| {let context=context.clone();let barrier=barrier.clone();
        std::thread::spawn(move|| {barrier.wait();context.resolve()})}).collect();
    let receipts:Vec<_>=threads.into_iter().map(|t|t.join().unwrap()).collect();
    assert_eq!(receipts.iter().filter(|r|r.performed_access).count(),1);
    let first=receipts[0].outcome.as_ref().unwrap_err();
    assert!(receipts.iter().all(|r|first.same_resolution(r.outcome.as_ref().unwrap_err())));
    saved(&root,br#"{"values":{"runtime_temporary_directory":"saved-A"}}"#);
    let cached=context.resolve();assert!(!cached.performed_access);
    assert!(first.same_resolution(&cached.outcome.unwrap_err()));healthy(&root);
}

#[test]
/// Require an effective filesystem permission refusal under the actual test
/// identity after a readable witness; a privileged/non-discriminating identity
/// fails this guard rather than reporting a skipped/PASS EACCES case. Repair
/// only the owned leaf's mode, preserve cached refusal, then use a fresh context.
fn unreadable_saved_source_refuses_without_default_then_recovers() {
    use std::os::unix::fs::PermissionsExt;
    let root=fixture("cd01-permission-");healthy(&root);
    assert!(!nix::unistd::geteuid().is_root(),"EACCES identity prerequisite is not admitted");
    let leaf=root.join(".swarm/engine-settings.json");
    fs::set_permissions(&leaf,fs::Permissions::from_mode(0o000)).unwrap();
    let context=Context::new(root.clone());let error=context.resolve().outcome.unwrap_err();
    assert_eq!(error.source().unwrap().downcast_ref::<io::Error>().unwrap().raw_os_error(),Some(13));
    assert!(!error.access_observations().any(|(_,source,_)|source=="default"));
    fs::set_permissions(&leaf,fs::Permissions::from_mode(0o600)).unwrap();
    let cached=context.resolve();assert!(!cached.performed_access);
    assert!(error.same_resolution(&cached.outcome.unwrap_err()));healthy(&root);
}
