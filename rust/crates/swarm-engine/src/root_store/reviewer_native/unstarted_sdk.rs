//! Regression code for strict decoding and a genuine expired prelaunch fixture.
//! Ordinary ledger checks use doubles. The separate ignored producer calls the
//! real claim writer in a new public-control fixture; it creates no provider
//! original and never executes the reviewer. Parent departure proof is separate.
use super::{guard, store, unstarted};
use super::store::Phase;
use crate::root_store::{RootStore, RootStoreError as E};
use std::{fs, path::Path};

/// Exercise malformed public inputs through the same decoder before any IO.
#[test]
fn unstarted_request_strict_envelope_refuses_invalid_values() {
    let valid = serde_json::json!({
        "schema_version": 1, "event_id": "retire", "expected_revision": 3, "task_id": "task"
    });
    let encode = |v: &serde_json::Value| serde_json::to_vec(v).unwrap();
    super::super::task_validation::decode(&encode(&valid)).unwrap();
    for (key, value) in [
        ("schema_version", serde_json::json!(2)),
        ("expected_revision", serde_json::json!(0)),
        ("event_id", serde_json::json!("../retire")),
        ("task_id", serde_json::Value::Null),
        ("permission", serde_json::json!(true)),
    ] {
        let mut bad = valid.clone();
        bad[key] = value;
        assert!(super::super::task_validation::decode(&encode(&bad)).is_err());
    }
    super::super::task_validation::decode(&encode(&valid)).unwrap();
}

/// Consume a separately generated, token-checked genuine fixture that stopped
/// after claim and before any helper creation. The fixture owns all paths and
/// has a real expired timestamp and departed owner. Never point this SDK at a
/// historical Root or a live mission. Its independent construction must retain
/// a successful reviewer witness before inducing the prelaunch interruption.
#[test]
#[ignore = "requires separately generated genuine expired unstarted fixture"]
fn unstarted_genuine_claim_refusal_rollback_retirement_and_replay() {
    let spec = fs::read(std::env::var_os("SWARM_UNSTARTED_REVIEW_FIXTURE").unwrap()).unwrap();
    let spec: serde_json::Value = serde_json::from_slice(&spec).unwrap();
    let path = Path::new(spec["root"].as_str().unwrap());
    assert_eq!(
        fs::read_to_string(path.join(".swarm/owned-unstarted-review-fixture")).unwrap(),
        spec["token"].as_str().unwrap()
    );
    let root = RootStore::open(path, true).unwrap();
    let id = spec["review"].as_str().unwrap();
    let job = store::job(&root.db, id).unwrap().unwrap();
    unstarted::unstarted(&job).unwrap();
    guard::departed(&job).unwrap();
    assert!(matches!(guard::original_fresh(&job.preflight_until),
        Err(E::Refused("review-guard-original-preflight-expired"))));
    let before = root.read_work(&job.work_id).unwrap();
    let current = crate::SupervisorIdentity::current().unwrap();
    let mut live = job.clone();
    live.owner_pid = current.pid();
    live.owner_stamp = current.process_stamp().into();
    live.owner_host = current.host().into();
    assert!(matches!(guard::departed(&live),
        Err(E::Refused("review-guard-owner-still-present"))));
    for changed in [
        { let mut x=job.clone(); x.permission=1; x },
        { let mut x=job.clone(); x.child_pid=Some(1); x },
        { let mut x=job.clone(); x.receipt_sha256=Some("a".repeat(64)); x },
        { let mut x=job.clone(); x.published_revision=Some(before.revision); x },
        { let mut x=job.clone(); x.phase=store::Phase::Registered; x },
    ] {
        assert!(unstarted::unstarted(&changed).is_err());
    }
    unstarted::unstarted(&job).unwrap();
    let raw = serde_json::to_vec(&serde_json::json!({
        "schema_version":1, "event_id":"owned-retire-unstarted",
        "expected_revision":before.revision, "task_id":job.task_id
    })).unwrap();
    let finance = || -> Vec<(String, String, String)> {
        crate::agent_launch::query(&root.db,
            "SELECT id,CAST(estimate AS TEXT),state FROM planning_calls WHERE work_id=? ORDER BY id",
            [&job.work_id], |rows| {
                let mut values=vec![];
                while let Some(r)=rows.next()? { values.push((r.get(0)?,r.get(1)?,r.get(2)?)); }
                Ok(values)
            }).unwrap()
    };
    let old_finance=finance();
    let unchanged=|| {
        assert_eq!(root.read_work(&job.work_id).unwrap(),before);
        assert!(store::job(&root.db,id).unwrap().as_ref()==Some(&job));
        assert_eq!(finance(),old_finance);
    };
    let mut stale:serde_json::Value=serde_json::from_slice(&raw).unwrap();
    stale["expected_revision"]=serde_json::json!(before.revision+1);
    assert!(root.retire_unstarted_task_review(id,&serde_json::to_vec(&stale).unwrap()).is_err());
    unchanged();
    let original_path = root.directory.process_path().join(super::journal::name(id));
    fs::write(&original_path, b"owned-negative-existing-file").unwrap();
    assert!(root.retire_unstarted_task_review(id,&raw).is_err());
    unchanged();
    fs::remove_file(&original_path).unwrap();
    root.db.execute_batch("CREATE TEMP TRIGGER owned_retirement_fault BEFORE INSERT ON main.events BEGIN SELECT RAISE(ABORT,'owned-retirement-fault'); END;").unwrap();
    let error=root.retire_unstarted_task_review(id,&raw).err().unwrap();
    assert!(error.recorded_work().is_none());
    unchanged();
    root.db.execute_batch("DROP TRIGGER owned_retirement_fault").unwrap();
    let retired=root.retire_unstarted_task_review(id,&raw).unwrap();
    assert!(!retired.review_recorded && !retired.verdict_applicable && !retired.task_accepted && !retired.replayed);
    assert_eq!(finance(),old_finance);
    assert!(!original_path.exists());
    let task=retired.work.tasks.iter().flatten().find(|t|t.id==job.task_id).unwrap();
    let original_task=before.tasks.iter().flatten().find(|t|t.id==job.task_id).unwrap();
    assert!(task.independent_review.is_none());
    assert_eq!(task.attempts,original_task.attempts);
    assert!(task.automatic_validation==original_task.automatic_validation);
    assert_eq!(task.previous_reviews.as_ref().unwrap().last().unwrap().state,"stale");
    let replay=root.retire_unstarted_task_review(id,&raw).unwrap();
    assert!(replay.replayed && !replay.verdict_applicable);
    assert_eq!(replay.work,retired.work);
    assert_eq!(finance(),old_finance);
    // A genuine fresh review uses the fixture's existing inert configured
    // provider and ordinary host/source/budget guards; no fake result is seeded.
    let new=serde_json::to_vec(&serde_json::json!({
        "schema_version":1, "event_id":"owned-fresh-review-after-retirement",
        "expected_revision":retired.work.revision, "task_id":job.task_id
    })).unwrap();
    let input=path.join("owned-fresh-review-request.json");
    fs::write(&input,&new).unwrap();
    let stdout=run_captured_review(&root,path,&job.work_id,&input);
    let review:serde_json::Value=serde_json::from_slice(&stdout).unwrap();
    assert_eq!(review["review_recorded"],true);
    assert_eq!(review["verdict_applicable"],true);
    assert_eq!(review["task_accepted"],false);
    assert_ne!(review["review_id"].as_str().unwrap(),id);
    let after=root.read_work(&job.work_id).unwrap();
    let calls=finance();
    let final_replay=root.retire_unstarted_task_review(id,&raw).unwrap();
    assert!(final_replay.replayed && !final_replay.verdict_applicable);
    assert_eq!(final_replay.work,after);
    assert_eq!(finance(),calls);
    let reconcile=root.reconcile_task_review(id).unwrap();
    assert!(reconcile.replayed && !reconcile.verdict_applicable);
    assert_eq!(reconcile.work,after);
    root.close().unwrap();
}


/// Pure normalized-ledger double. No Root, provider or original is manufactured.
fn ledger_fixture() -> (rusqlite::Connection, store::Job) {
    let db=rusqlite::Connection::open_in_memory().unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE works(id TEXT PRIMARY KEY); INSERT INTO works VALUES('work');").unwrap();
    let h="a".repeat(64);
    let j=store::Job {
        id:"review-old".into(), work_id:"work".into(), task_id:"task".into(), event_id:"old-event".into(),
        request_sha256:h.clone(),producer_id:"worker".into(),attempt_id:"attempt".into(),
        source_sha256:h.clone(),validation_receipt_sha256:h.clone(),contract_sha256:h.clone(),
        provider_sha256:h.clone(),executable_sha256:h.clone(),host_scope:h.clone(),
        workflow_sha256:h.clone(),prompt_sha256:h,work_revision:2,settings_revision:1,
        owner_pid:10,owner_stamp:"owned".into(),owner_host:"owned-host".into(),
        directory_device:"1".into(),directory_inode:"2".into(),preflight_until:"1970-01-01T00:00:00Z".into(),
        child_pid:None,parent_pid:None,group_id:None,child_stamp:None,
        permission:0,phase:store::Phase::Claimed,receipt_sha256:None,published_revision:None
    };
    (db,j)
}

/// Actual store transitions exclude concurrent claims, roll back retirement,
/// then permit a new claim while preserving the old immutable ledger row.
#[test]
fn unstarted_ledger_retirement_rollback_and_new_claim_are_atomic() {
    let (db,job)=ledger_fixture();
    let tx=db.unchecked_transaction().unwrap();
    store::claim(&tx,&job).unwrap();
    tx.commit().unwrap();
    assert!(store::active(&db,"work","task").unwrap().as_ref()==Some(&job));
    let mut next=job.clone();
    next.id="review-new".into();next.event_id="new-event".into();
    let tx=db.unchecked_transaction().unwrap();
    assert!(store::claim(&tx,&next).is_err());
    tx.rollback().unwrap();
    let mut done=job.clone();done.phase=store::Phase::Refused;
    done.receipt_sha256=Some("b".repeat(64));done.published_revision=Some(3);
    let tx=db.unchecked_transaction().unwrap();
    store::change(&tx,&job,&done).unwrap();
    assert!(store::active(&tx,"work","task").unwrap().is_none());
    tx.rollback().unwrap();
    assert!(store::job(&db,&job.id).unwrap().as_ref()==Some(&job));
    let tx=db.unchecked_transaction().unwrap();
    store::change(&tx,&job,&done).unwrap();
    store::claim(&tx,&next).unwrap();
    tx.commit().unwrap();
    assert!(store::job(&db,&job.id).unwrap().as_ref()==Some(&done));
    assert!(store::active(&db,"work","task").unwrap().as_ref()==Some(&next));
    let tx=db.unchecked_transaction().unwrap();
    assert!(store::change(&tx,&job,&done).is_err());
    tx.rollback().unwrap();
    assert!(store::active(&db,"work","task").unwrap().as_ref()==Some(&next));
}

/// Each forbidden prelaunch tuple must refuse while the valid double still
/// passes. These shape checks alone establish no process-departure authority.
#[test]
fn unstarted_shape_refuses_partial_children_permissions_and_terminal_data() {
    verify_partial_capture_failure();
    verify_reader_start_ownership();
    let (_,job)=ledger_fixture();
    unstarted::unstarted(&job).unwrap();
    let mut variants=vec![];
    let mut v=job.clone();v.permission=1;variants.push(v);
    let mut v=job.clone();v.child_pid=Some(1);variants.push(v);
    let mut v=job.clone();v.parent_pid=Some(1);variants.push(v);
    let mut v=job.clone();v.group_id=Some(1);variants.push(v);
    let mut v=job.clone();v.child_stamp=Some("child".into());variants.push(v);
    let mut v=job.clone();v.receipt_sha256=Some("a".repeat(64));variants.push(v);
    let mut v=job.clone();v.published_revision=Some(3);variants.push(v);
    let mut v=job.clone();v.phase=store::Phase::Unresolved;variants.push(v);
    for v in variants { assert!(unstarted::unstarted(&v).is_err()); }
    unstarted::unstarted(&job).unwrap();
}

/// GLM-proposed gates, reviewed against the existing clock and OS contract.
#[test]
fn retire_gates_reject_live_owner_unexpired_and_wrong_shape() {
    let (_, value) = ledger_fixture();
    unstarted::unstarted(&value).unwrap();
    let mut permitted = value.clone();
    permitted.permission = 1;
    assert!(unstarted::unstarted(&permitted).is_err());
    let mut registered = value.clone();
    registered.child_pid = Some(20);
    registered.parent_pid = Some(10);
    registered.group_id = Some(20);
    registered.child_stamp = Some("supplied-child".into());
    assert!(unstarted::unstarted(&registered).is_err());
    let mut receipted = value.clone();
    receipted.receipt_sha256 = Some("b".repeat(64));
    assert!(unstarted::unstarted(&receipted).is_err());
    let mut other = value.clone();
    other.phase = Phase::Registered;
    assert!(unstarted::unstarted(&other).is_err());
    let mut live = value.clone();
    live.preflight_until = "9999-12-31T23:59:59Z".into();
    assert!(matches!(unstarted::expired(&live), Err(E::Refused("review-unstarted-not-expired"))));
    let mut gone = value.clone();
    gone.preflight_until = "1970-01-01T00:00:00Z".into();
    unstarted::expired(&gone).unwrap();
    let current = crate::SupervisorIdentity::current().unwrap();
    let mut present = value.clone();
    present.owner_pid = i64::from(current.pid());
    present.owner_stamp = current.process_stamp().into();
    present.owner_host = current.host().into();
    assert!(matches!(guard::departed(&present), Err(E::Refused("review-guard-owner-still-present"))));
    assert!(matches!(guard::departed(&value), Err(E::Refused("review-guard-owner-host-changed"))));
}


/// Use a real freshly initialized private Root for the physical-instance gate.
/// This tests identity observation only and grants no reviewer authority.
#[test]
fn unstarted_physical_root_refuses_other_directory() {
    let path=crate::fixture_root("review-unstarted-root-");
    let root=RootStore::initialize(&path, &mut crate::SystemEnvironment).unwrap();
    let (device,inode)=root.directory.identity().unwrap();
    let (_,mut job)=ledger_fixture();
    job.directory_device=device.to_string();
    job.directory_inode=inode.to_string();
    unstarted::physical(&root,&job).unwrap();
    job.directory_inode="unrelated".into();
    assert!(matches!(unstarted::physical(&root,&job),
        Err(E::Refused("review-unstarted-root-changed"))));
    root.close().unwrap();
    // Retain the fixture rather than silently ignoring a cleanup failure.
}

/// Private finite fixture capture. Bytes remain in memory even if writing
/// proof fails. Production caps/policies and decoder behavior are unchanged.
#[derive(Default)]
struct CapturedChannel {
    panic_payload: Option<Box<dyn std::any::Any + Send>>,
    bytes: Vec<u8>,
    errors: Vec<String>,
}

/// Create new private evidence and sync the exact bytes. Any partial artifact
/// remains named for diagnosis; existing evidence is never overwritten.
fn publish_capture(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::{io::Write, os::unix::fs::OpenOptionsExt};
    let mut file=fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// Drain one actual pipe while teeing every observed chunk to the private
/// artifact. Read/write/sync errors are preserved independently. A failed disk
/// write does not discard memory bytes or silently stop draining the child.
fn capture_channel(mut pipe: impl std::io::Read, path: std::path::PathBuf) -> CapturedChannel {
    use std::{io::Write, os::unix::fs::OpenOptionsExt};
    let mut result=CapturedChannel::default();
    let mut target=match fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(path) {
        Ok(file)=>Some(file),
        Err(error)=>{result.errors.push(format!("artifact-create: {error}"));None}
    };
    let mut chunk=[0u8;8192]; // Fixture transfer buffer, never an output cap.
    loop {
        match pipe.read(&mut chunk) {
            Ok(0)=>break,
            Ok(count)=>{
                result.bytes.extend_from_slice(&chunk[..count]);
                if let Some(file)=target.as_mut() {
                    if let Err(error)=file.write_all(&chunk[..count]) {
                        result.errors.push(format!("artifact-write: {error}"));
                        target=None;
                    }
                }
            }
            Err(error) if error.kind()==std::io::ErrorKind::Interrupted=>continue,
            Err(error)=>{result.errors.push(format!("pipe-read: {error}"));break}
        }
    }
    if let Some(file)=target {
        if let Err(error)=file.sync_all() {result.errors.push(format!("artifact-sync: {error}"));}
    }
    result
}

/// Run only the fixture's fresh reviewer CLI, retaining argv, exact candidate
/// and profile hashes, status/signal, raw/partial pipes and independent capture
/// failures before judging success. Unknown wait/spawn and nonzero status imply
/// possible recorded effects and require public state reread by the caller.
/// All output is owned inert fixture data. No inherited environment is logged.
fn run_captured_review(root: &RootStore, path: &Path, work: &str, input: &Path) -> Vec<u8> {
    use std::{os::unix::{fs::DirBuilderExt,process::ExitStatusExt},process::{Command,Stdio}};
    let evidence=std::path::PathBuf::from(std::env::var_os("SWARM_UNSTARTED_REVIEW_EVIDENCE").unwrap());
    fs::DirBuilder::new().mode(0o700).create(&evidence).unwrap();
    let binary=std::path::PathBuf::from(std::env::var_os("SWARM_UNSTARTED_REVIEW_BINARY").unwrap());
    let profile=std::path::PathBuf::from(std::env::var_os("SWARM_UNSTARTED_REVIEW_PROFILE").unwrap());
    let profile_raw=fs::read(&profile).unwrap();
    let profile_value:serde_json::Value=serde_json::from_slice(&profile_raw).unwrap();
    let binary_raw=fs::read(&binary).unwrap();
    let hash=crate::provider_configuration::configuration_digest(&binary_raw);
    assert_eq!(profile_value["sha256"].as_str(),Some(hash.as_str()));
    assert_eq!(profile_value["status"],"VERIFIED_DEFAULT_NATIVE_PROFILE");
    assert_eq!(profile_value["no_fixture_feature"],true);
    assert_eq!(profile_value["all_core_features_empty"],true);
    assert_eq!(profile_value["all_core_test_false"],true);
    let argv=vec![binary.to_string_lossy().into_owned(),"task".into(),"review".into(),work.into(),
        "--root".into(),path.to_string_lossy().into_owned(),"--json".into(),
        "--input".into(),input.to_string_lossy().into_owned()];
    let manifest=serde_json::to_vec(&serde_json::json!({
        "argv":argv,"binary_sha256":hash,
        "profile_path":profile,"profile_sha256":crate::provider_configuration::configuration_digest(&profile_raw),
        "global_watchdog":null,"scope":"pre-spawn-intent-only; not a current launch status"
    })).unwrap();
    // Failure here prevents spawning. After spawn every issue is aggregated.
    publish_capture(&evidence.join("command.json"),&manifest).unwrap();
    let mut command=Command::new(&binary);
    command.args(&argv[1..]).env("CODEX_HOME",path.join("codex-home"))
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut issues=vec![];
    let mut status=None;
    let mut stdout=CapturedChannel::default();
    let mut stderr=CapturedChannel::default();
    let mut launch="spawn-not-observed";
    let mut child_pid=None;
    match command.spawn() {
        Err(error)=>{
            launch="spawn-outcome-unknown";
            issues.push(format!("spawn-outcome-unknown: {error}"));
        }
        Ok(mut child)=>{
            launch="started";
            child_pid=Some(child.id());
            let launched=serde_json::to_vec(&serde_json::json!({
                "launch":"started","pid":child.id(),"effect":"possible-recorded-state"
            })).unwrap();
            if let Err(error)=publish_capture(&evidence.join("launch.json"),&launched) {
                issues.push(format!("launch-publication: {error}"));
            }
            let out=child.stdout.take();
            let err=child.stderr.take();
            let out_reader=out.map(|pipe|start_reader(pipe,evidence.join("stdout.raw"),
                |worker|std::thread::Builder::new().name("owned-review-stdout".into()).spawn(worker)));
            let err_reader=err.map(|pipe|start_reader(pipe,evidence.join("stderr.raw"),
                |worker|std::thread::Builder::new().name("owned-review-stderr".into()).spawn(worker)));
            let setup_failed=out_reader.as_ref().is_none_or(|r|r.startup_error.is_some())
                || err_reader.as_ref().is_none_or(|r|r.startup_error.is_some());
            if setup_failed {
                issues.push("reader-setup-refused; owned direct-child cleanup requested".into());
                // Cleanup is explicit for this owned fixture child, not an
                // inference of rollback or a retry of the review operation.
                if let Err(error)=child.kill() {issues.push(format!("owned-child-kill: {error}"));}
            }
            match child.wait() {
                Ok(value)=>status=Some(value),
                Err(error)=>{
                    issues.push(format!("wait-outcome-unknown pid={}: {error}",child.id()));
                    // Keep this secondary cleanup distinct from the failed wait.
                    if let Err(error)=child.kill() {issues.push(format!("owned-child-kill-after-wait-error: {error}"));}
                    match child.wait() {
                        Ok(value)=>status=Some(value),
                        Err(error)=>issues.push(format!("owned-child-reap-unknown: {error}")),
                    }
                }
            }
            // Every created JoinHandle is retained until joined. Refused
            // readers still own their pipes and are drained after child wait.
            if let Some(reader)=out_reader {stdout=finish_reader(reader);}
            else {issues.push("stdout-pipe-missing; extent unknown".into());}
            if let Some(reader)=err_reader {stderr=finish_reader(reader);}
            else {issues.push("stderr-pipe-missing; extent unknown".into());}
        }
    }
    issues.extend(stdout.errors.iter().map(|e|format!("stdout: {e}")));
    issues.extend(stderr.errors.iter().map(|e|format!("stderr: {e}")));
    // Observe actual Work even after nonzero/spawn/wait failure. This does not
    // infer rollback or an absence of provider effects from the exit status.
    match root.read_work(work) {
        Ok(value)=>match crate::encode_model(&value) {
            Ok(raw)=>if let Err(error)=publish_capture(&evidence.join("work-after.json"),&raw) {
                issues.push(format!("work-observation-publication: {error}"));
            },
            Err(error)=>issues.push(format!("work-observation-encode: {error}")),
        },
        Err(error)=>issues.push(format!("work-observation: {:?}",crate::native_diagnostics::project(&error))),
    }
    if let Some(payload)=reader_panic_description(&stdout) {issues.push(format!("stdout-panic: {payload}"));}
    if let Some(payload)=reader_panic_description(&stderr) {issues.push(format!("stderr-panic: {payload}"));}
    let success=status.as_ref().is_some_and(|s|s.success());
    let terminal=serde_json::to_vec(&serde_json::json!({
        "launch":launch,"child_pid":child_pid,"child_wait_confirmed":status.is_some(),"status_code":status.as_ref().and_then(|s|s.code()),
        "signal":status.as_ref().and_then(|s|s.signal()),
        "exit_success":success,"capture_complete":success && issues.is_empty(),"issues":issues,"effect":"possible-recorded-state; public-reread-required",
        "stdout_bytes":stdout.bytes.len(),"stderr_bytes":stderr.bytes.len(),
        "stdout_sha256":crate::provider_configuration::configuration_digest(&stdout.bytes),
        "stderr_sha256":crate::provider_configuration::configuration_digest(&stderr.bytes)
    })).unwrap();
    if let Err(error)=publish_capture(&evidence.join("result.json"),&terminal) {
        issues.push(format!("result-publication: {error}"));
    }
    // Preserve the primary nonzero/unknown outcome together with all capture
    // failures, including memory-retained bytes when disk publication failed.
    if !success || !issues.is_empty() {
        panic!("owned reviewer outcome retained: status={status:?}; issues={issues:?}; stdout={:?}; stderr={:?}",
            stdout.bytes,stderr.bytes);
    }
    stdout.bytes
}


/// Capture guard with a valid tee witness before combined artifact/pipe faults.
/// A fixture IO failure must preserve already-read bytes in memory and retain
/// both independent diagnostics. This is an IO double, not a reviewer run.
fn verify_partial_capture_failure() {
    struct PartialThenError(bool);
    impl std::io::Read for PartialThenError {
        fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
            if self.0 { return Err(std::io::Error::other("owned-pipe-fixture")); }
            self.0=true;
            let bytes=b"partial-owned";
            output[..bytes.len()].copy_from_slice(bytes);
            Ok(bytes.len())
        }
    }
    let directory=crate::fixture_root("review-capture-");
    let witness=directory.join("witness.raw");
    let captured=capture_channel(std::io::Cursor::new(b"valid-owned".to_vec()),witness.clone());
    assert!(captured.errors.is_empty());
    assert_eq!(captured.bytes,b"valid-owned");
    assert_eq!(fs::read(witness).unwrap(),b"valid-owned");
    let refused=capture_channel(PartialThenError(false),directory);
    assert_eq!(refused.bytes,b"partial-owned");
    assert!(refused.errors.iter().any(|e|e.starts_with("artifact-create:")));
    assert!(refused.errors.iter().any(|e|e.starts_with("pipe-read:")));
}

/// Retain the pipe independently of a possibly refused thread start. Builder
/// drops the closure on failure; it owns only an Arc clone, never the sole pipe.
struct ReaderOwner<T> {
    pending: std::sync::Arc<std::sync::Mutex<Option<T>>>,
    thread: Option<std::thread::JoinHandle<CapturedChannel>>,
    startup_error: Option<std::io::Error>,
    path: std::path::PathBuf,
}

/// Fallible reader constructor with an explicit fixture-only factory boundary.
/// A refused factory leaves pending intact, so the controller can terminate and
/// reap its child before draining the still-owned pipe synchronously.
fn start_reader<T: std::io::Read + Send + 'static>(
    pipe: T,
    path: std::path::PathBuf,
    factory: impl FnOnce(Box<dyn FnOnce() -> CapturedChannel + Send>)
        -> std::io::Result<std::thread::JoinHandle<CapturedChannel>>,
) -> ReaderOwner<T> {
    let pending=std::sync::Arc::new(std::sync::Mutex::new(Some(pipe)));
    let worker_pending=std::sync::Arc::clone(&pending);
    let worker_path=path.clone();
    let worker=Box::new(move || {
        let mut errors=vec![];
        let mut slot=match worker_pending.lock() {
            Ok(value)=>value,
            Err(error)=>{errors.push("reader-pipe-lock-poisoned".into());error.into_inner()}
        };
        let pipe=slot.take();
        drop(slot);
        let mut captured=match pipe {
            Some(pipe)=>capture_channel(pipe,worker_path),
            None=>CapturedChannel::default(),
        };
        if captured.bytes.is_empty() && !errors.is_empty() {
            errors.push("reader-extent-unknown".into());
        }
        captured.errors.extend(errors);
        captured
    });
    match factory(worker) {
        Ok(thread)=>ReaderOwner{pending,thread:Some(thread),startup_error:None,path},
        Err(error)=>ReaderOwner{pending,thread:None,startup_error:Some(error),path},
    }
}

/// Join every started reader. On startup refusal only, retain/drain the original
/// pipe after child cleanup. A reader panic retains its actual opaque payload;
/// its worker-local memory prefix remains unknown, as recorded explicitly.
fn finish_reader<T: std::io::Read + Send + 'static>(mut reader: ReaderOwner<T>) -> CapturedChannel {
    if let Some(thread)=reader.thread.take() {
        return match thread.join() {
            Ok(value)=>value,
            Err(payload)=>CapturedChannel{
                bytes:vec![],errors:vec!["reader-panicked; worker memory extent unknown".into()],
                panic_payload:Some(payload),
            }
        };
    }
    let mut errors=vec![];
    if let Some(error)=reader.startup_error.take() {
        errors.push(format!("reader-start: kind={:?} errno={:?}: {error}",error.kind(),error.raw_os_error()));
    }
    let mut slot=match reader.pending.lock() {
        Ok(value)=>value,
        Err(error)=>{errors.push("reader-pipe-lock-poisoned".into());error.into_inner()}
    };
    let pipe=slot.take();
    drop(slot);
    let mut captured=match pipe {
        Some(pipe)=>capture_channel(pipe,reader.path),
        None=>{errors.push("reader-pipe-missing; extent unknown".into());CapturedChannel::default()}
    };
    captured.errors.extend(errors);
    captured
}

/// Describe known string payloads while the original Any payload remains owned.
/// An unknown payload is explicitly opaque and cannot claim a full error chain.
fn reader_panic_description(channel: &CapturedChannel) -> Option<String> {
    channel.panic_payload.as_ref().map(|payload| {
        if let Some(text)=payload.downcast_ref::<String>() {text.clone()}
        else if let Some(text)=payload.downcast_ref::<&'static str>() {(*text).into()}
        else {format!("opaque reader panic payload {:?}",payload.as_ref().type_id())}
    })
}


/// Reader-owner double: a healthy pair precedes each injected first/second
/// startup refusal. This exercises the fallible factory and ownership only;
/// a separate owned-child protocol is required to qualify cancellation/reaping.
fn verify_reader_start_ownership() {
    for refused in [None,Some(0usize),None,Some(1usize)] {
        let directory=crate::fixture_root("review-reader-owner-");
        let start=|index:usize,bytes:Vec<u8>,path:std::path::PathBuf| {
            start_reader(std::io::Cursor::new(bytes),path,move|worker| {
                if refused==Some(index) {
                    Err(std::io::Error::from_raw_os_error(rustix::io::Errno::AGAIN.raw_os_error()))
                } else {
                    std::thread::Builder::new().spawn(worker)
                }
            })
        };
        let stdout=start(0,b"owned-stdout".to_vec(),directory.join("stdout.raw"));
        let stderr=start(1,b"owned-stderr".to_vec(),directory.join("stderr.raw"));
        assert_eq!(stdout.startup_error.is_some(),refused==Some(0));
        assert_eq!(stderr.startup_error.is_some(),refused==Some(1));
        let stdout=finish_reader(stdout);
        let stderr=finish_reader(stderr);
        assert_eq!(stdout.bytes,b"owned-stdout");
        assert_eq!(stderr.bytes,b"owned-stderr");
        assert_eq!(fs::read(directory.join("stdout.raw")).unwrap(),b"owned-stdout");
        assert_eq!(fs::read(directory.join("stderr.raw")).unwrap(),b"owned-stderr");
        assert_eq!(stdout.errors.iter().any(|e|e.starts_with("reader-start:")),refused==Some(0));
        assert_eq!(stderr.errors.iter().any(|e|e.starts_with("reader-start:")),refused==Some(1));
    }
}

/// Proposed process-level guard for reader ownership. Uses only an explicitly
/// hashed isolated Python interpreter and four new finite inert children.
/// No reviewer, Root, SQL, provider or network is involved. A healthy child
/// precedes each factory refusal; all facts are published before assertions.
#[test]
#[ignore = "requires explicit inert interpreter/hash and fresh private evidence"]
fn unstarted_capture_owned_inert_child_before_each_reader_refusal() {
    use std::{io::Write,os::unix::{fs::DirBuilderExt,process::ExitStatusExt},
        process::{Command,Stdio}};
    const PROGRAM:&str = r#"import sys
sys.stdout.buffer.write(b'ready-out\n');sys.stdout.buffer.flush()
sys.stderr.buffer.write(b'ready-err\n');sys.stderr.buffer.flush()
x=sys.stdin.buffer.read(1)
assert x==b'X'
sys.stdout.buffer.write(b'done-out\n');sys.stdout.buffer.flush()
sys.stderr.buffer.write(b'done-err\n');sys.stderr.buffer.flush()
"#;
    let binary=std::path::PathBuf::from(std::env::var_os("SWARM_GI_CAPTURE_PYTHON").unwrap());
    assert!(binary.is_absolute());
    let binary_hash=crate::provider_configuration::configuration_digest(&fs::read(&binary).unwrap());
    assert_eq!(binary_hash,std::env::var("SWARM_GI_CAPTURE_PYTHON_SHA256").unwrap());
    let evidence=std::path::PathBuf::from(std::env::var_os("SWARM_GI_CAPTURE_EVIDENCE").unwrap());
    fs::DirBuilder::new().mode(0o700).create(&evidence).unwrap();
    publish_capture(&evidence.join("inert-child.py"),PROGRAM.as_bytes()).unwrap();
    let mut reports=vec![];
    for (number,refused) in [None,Some(0usize),None,Some(1usize)].into_iter().enumerate() {
        let case=evidence.join(format!("case-{number}"));
        fs::DirBuilder::new().mode(0o700).create(&case).unwrap();
        let argv=vec![binary.to_string_lossy().into_owned(),"-I".into(),"-c".into(),PROGRAM.into()];
        let intent=serde_json::to_vec(&serde_json::json!({
            "argv":argv,"binary_sha256":binary_hash,
            "program_sha256":crate::provider_configuration::configuration_digest(PROGRAM.as_bytes()),
            "refused_reader":refused,"scope":"pre-spawn intent, owned inert child only"
        })).unwrap();
        publish_capture(&case.join("command.json"),&intent).unwrap();
        let mut issues=vec![];
        let mut status=None;
        let mut pid=None;
        let mut launch="spawn-outcome-unknown";
        let mut started_readers=0;
        let mut finished_readers=0;
        let mut out_prefix=vec![];
        let mut err_prefix=vec![];
        let mut stdout=CapturedChannel::default();
        let mut stderr=CapturedChannel::default();
        let spawned=Command::new(&binary).args(&argv[1..])
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
        match spawned {
            Err(error)=>issues.push(format!("spawn-outcome-unknown: {error}")),
            Ok(mut child)=>{
                launch="started";
                pid=Some(child.id());
                let event=serde_json::to_vec(&serde_json::json!({
                    "launch":"started","pid":pid,"scope":"owned inert child, no Work"
                })).unwrap();
                if let Err(error)=publish_capture(&case.join("launch.json"),&event) {
                    issues.push(format!("launch-publication: {error}"));
                }
                let mut out=child.stdout.take();
                let mut err=child.stderr.take();
                // Bounded handshake data (10 bytes per pipe) is fixture input,
                // not a wait deadline or output limit. The child emits both
                // prefixes and then blocks only on our owned release pipe.
                let mut ready=true;
                if let Some(pipe)=out.as_mut() {
                    if let Err(error)=read_ready_prefix(pipe,&mut out_prefix) {
                        ready=false;issues.push(format!("stdout-ready: {error}; observed bytes retained"));
                    }
                } else {ready=false;issues.push("stdout-pipe-missing".into());}
                if let Some(pipe)=err.as_mut() {
                    if let Err(error)=read_ready_prefix(pipe,&mut err_prefix) {
                        ready=false;issues.push(format!("stderr-ready: {error}; observed bytes retained"));
                    }
                } else {ready=false;issues.push("stderr-pipe-missing".into());}
                if out_prefix!=b"ready-out\n" || err_prefix!=b"ready-err\n" {
                    ready=false;issues.push("inert child ready protocol invalid; no factory fault injected".into());
                }
                let out_reader=out.map(|pipe|start_reader(pipe,case.join("stdout.tail.raw"),move|worker| {
                    if ready && refused==Some(0) {
                        Err(std::io::Error::from_raw_os_error(rustix::io::Errno::AGAIN.raw_os_error()))
                    } else {std::thread::Builder::new().name("inert-stdout".into()).spawn(worker)}
                }));
                let err_reader=err.map(|pipe|start_reader(pipe,case.join("stderr.tail.raw"),move|worker| {
                    if ready && refused==Some(1) {
                        Err(std::io::Error::from_raw_os_error(rustix::io::Errno::AGAIN.raw_os_error()))
                    } else {std::thread::Builder::new().name("inert-stderr".into()).spawn(worker)}
                }));
                started_readers=usize::from(out_reader.as_ref().is_some_and(|r|r.thread.is_some()))
                    +usize::from(err_reader.as_ref().is_some_and(|r|r.thread.is_some()));
                let setup_failed=out_reader.as_ref().is_none_or(|r|r.startup_error.is_some())
                    ||err_reader.as_ref().is_none_or(|r|r.startup_error.is_some());
                let mut cleanup=setup_failed||!ready;
                if !cleanup {
                    match child.stdin.take() {
                        Some(mut input)=>{
                            if let Err(error)=input.write_all(b"X") {
                                cleanup=true;issues.push(format!("release-pipe: {error}"));
                            }
                        },
                        None=>{cleanup=true;issues.push("release-pipe-missing".into());}
                    }
                }
                if cleanup {
                    if let Err(error)=child.kill() {issues.push(format!("owned-child-kill: {error}"));}
                }
                match child.wait() {
                    Ok(value)=>status=Some(value),
                    Err(error)=>{
                        issues.push(format!("wait-outcome-unknown: {error}"));
                        if let Err(error)=child.kill() {issues.push(format!("owned-child-kill-after-wait: {error}"));}
                        match child.wait() {
                            Ok(value)=>status=Some(value),
                            Err(error)=>issues.push(format!("reap-outcome-unknown: {error}")),
                        }
                    }
                }
                if let Some(reader)=out_reader {
                    let was_started=reader.thread.is_some();
                    stdout=finish_reader(reader);
                    finished_readers+=usize::from(was_started);
                }
                if let Some(reader)=err_reader {
                    let was_started=reader.thread.is_some();
                    stderr=finish_reader(reader);
                    finished_readers+=usize::from(was_started);
                }
            }
        }
        out_prefix.extend_from_slice(&stdout.bytes);
        err_prefix.extend_from_slice(&stderr.bytes);
        if let Err(error)=publish_capture(&case.join("stdout.raw"),&out_prefix) {
            issues.push(format!("stdout-publication: {error}"));
        }
        if let Err(error)=publish_capture(&case.join("stderr.raw"),&err_prefix) {
            issues.push(format!("stderr-publication: {error}"));
        }
        let expected_reader_error=|channel:&CapturedChannel,index:usize| {
            channel.errors.len()==usize::from(refused==Some(index))
                && channel.errors.iter().all(|e|e.starts_with("reader-start:"))
                && channel.panic_payload.is_none()
        };
        let expected_output=if refused.is_none() {
            out_prefix==b"ready-out\ndone-out\n" && err_prefix==b"ready-err\ndone-err\n"
        } else {
            out_prefix==b"ready-out\n" && err_prefix==b"ready-err\n"
        };
        let expected_exit=status.as_ref().is_some_and(|s| {
            if refused.is_none() {s.success()} else {s.signal()==Some(nix::libc::SIGKILL)}
        });
        let valid=issues.is_empty() && launch=="started" && pid.is_some()
            && expected_output && expected_exit && started_readers==finished_readers
            && expected_reader_error(&stdout,0) && expected_reader_error(&stderr,1);
        let report=serde_json::to_vec(&serde_json::json!({
            "launch":launch,"pid":pid,"wait_confirmed":status.is_some(),
            "exit_code":status.as_ref().and_then(|s|s.code()),
            "signal":status.as_ref().and_then(|s|s.signal()),
            "started_readers":started_readers,"join_returned":finished_readers,
            "refused_reader":refused,"stdout_reader_errors":stdout.errors,
            "stderr_reader_errors":stderr.errors,"issues":issues,
            "scope":"inert child, factory refusal double; not real OS exhaustion",
            "pass":valid,"stdout_bytes":out_prefix.len(),"stderr_bytes":err_prefix.len()
        })).unwrap();
        let published=publish_capture(&case.join("result.json"),&report);
        assert!(published.is_ok() && valid,
            "owned inert capture: status={status:?}; result-publication={published:?}; issues={issues:?}; stdout={out_prefix:?}; stderr={err_prefix:?}");
        reports.push(serde_json::from_slice::<serde_json::Value>(&report).unwrap());
    }
    publish_capture(&evidence.join("result.json"),
        &serde_json::to_vec(&serde_json::json!({"scope":"inert capture only","cases":reports})).unwrap()).unwrap();
}


/// Read the known ten-byte inert handshake without discarding a partial read.
/// Ten is the fixture record size, not a runtime output cap or deadline.
fn read_ready_prefix(pipe: &mut impl std::io::Read, bytes: &mut Vec<u8>) -> std::io::Result<()> {
    let mut buffer=[0u8;10];
    while bytes.len()<buffer.len() {
        let remaining=buffer.len()-bytes.len();
        match pipe.read(&mut buffer[..remaining]) {
            Ok(0)=>return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof,"inert handshake incomplete")),
            Ok(count)=>bytes.extend_from_slice(&buffer[..count]),
            Err(error) if error.kind()==std::io::ErrorKind::Interrupted=>continue,
            Err(error)=>return Err(error),
        }
    }
    Ok(())
}

/// Proposed genuine prelaunch producer. Public worker/control originals must
/// already exist in a NEW token-bound synthetic Root. This cfg(test)-only entry
/// uses the actual claim writer, proves freshness before and after its commit,
/// then waits for that unchanged original expiry without running any helper.
/// The parent must wait for this whole libtest process and verify owner departure.
/// It creates no provider original, verdict, payment, refund or replacement root.
/// Known Work identity/revision and typed primary/publication/close causes survive
/// failures. No recorded metadata means unknown after claim invocation, not rollback.
/// Observation is published before close and finalization afterward; failed proof
/// publication remains an error. Neither output proves owner departure.
#[test]
#[ignore = "requires new public source/control fixture; no historical Root"]
fn unstarted_expired_claim_producer() -> Result<(),Box<dyn std::error::Error>> {
    run_unstarted_claim_producer().map_err(producer_terminal_error)
}

/// Run the owned producer without erasing its original early or late errors.
/// Only the libtest entry wraps this result for terminal rendering. Input/Root
/// errors as well as the post-claim proof/close path pass through that wrapper.
/// No helper/provider is executed; all existing freshness/budget guards apply.
fn run_unstarted_claim_producer() -> Result<(),Box<dyn std::error::Error>> {
    use std::os::unix::fs::DirBuilderExt;
    let input_path=std::path::PathBuf::from(std::env::var_os("SWARM_UNSTARTED_PRODUCER_INPUT").unwrap());
    let evidence=std::path::PathBuf::from(std::env::var_os("SWARM_UNSTARTED_PRODUCER_EVIDENCE").unwrap());
    fs::DirBuilder::new().mode(0o700).create(&evidence)?;
    let input_raw=fs::read(&input_path)?;
    publish_capture(&evidence.join("input.json"),&input_raw)?;
    let input:serde_json::Value=serde_json::from_slice(&input_raw)?;
    let path=Path::new(input["root"].as_str().ok_or("fixture Root required")?);
    let token=input["token"].as_str().ok_or("fixture token required")?;
    let work_id=input["work"].as_str().ok_or("fixture Work required")?;
    if fs::read_to_string(path.join(".swarm/owned-unstarted-review-fixture"))?!=token {
        return Err("fixture token mismatch".into());
    }
    let request_raw=crate::encode_model(&input["request"])?;
    let request=super::super::task_validation::decode(&request_raw)?;
    let root=RootStore::open(path,true)?;
    let mut known_claim=None;
    let mut known_work=None;
    let mut claim_attempted=false;
    let mut replay_observed=false;
    let run=(||->Result<serde_json::Value,E>{
        let settings=root.read_settings()?;
        let before=root.read_work(work_id)?;
        if before.revision!=request.expected_revision {
            return Err(E::Refused("unstarted-producer-revision-changed"));
        }
        // Healthy source/host observation precedes claim. No provider permit
        // or child is created by material preparation. Host probe IO is real.
        let tx=root.db.unchecked_transaction().map_err(E::Sql)?;
        let prepared=super::material::prepare(&root,&tx,settings.clone(),work_id,&request);
        let observed=prepared.and_then(|m|guard::original_fresh(&m.host.preflight.valid_until));
        let rollback=tx.rollback().map_err(E::Sql);
        match (observed,rollback) {
            (Ok(()),Ok(()))=>{},
            (Err(primary),Ok(()))=>return Err(primary),
            (Ok(()),Err(cleanup))=>return Err(cleanup),
            (Err(primary),Err(cleanup))=>return Err(E::Cleanup{primary:Some(Box::new(primary)),cleanup:Box::new(cleanup)}),
        }
        claim_attempted=true;
        let staged=match super::claim::record(&root,work_id,&request)? {
            super::claim::Recorded::New(value)=>*value,
            super::claim::Recorded::Replay(job)=>{
                replay_observed=true;
                known_claim=Some(*job);
                return Err(E::Refused("unstarted-producer-new-claim-required"));
            },
        };
        known_work=Some((staged.work.id.clone(),staged.work.revision));
        known_claim=Some(staged.job.clone());
        let job=&staged.job;
        let initial=store::job(&root.db,&job.id).map_err(|e|E::Launch(Box::new(e)))?
            .ok_or(E::Refused("unstarted-producer-job-required"))?;
        if &initial!=job {return Err(E::Refused("unstarted-producer-claim-changed"));}
        unstarted::unstarted(job)?;
        guard::owner(job)?;
        let tx=root.db.unchecked_transaction().map_err(E::Sql)?;
        let fresh=guard::fresh(&root,&tx,settings,job).map(|_|());
        let rollback=tx.rollback().map_err(E::Sql);
        match (fresh,rollback) {
            (Ok(()),Ok(()))=>{},
            (Err(primary),Ok(()))=>return Err(primary),
            (Ok(()),Err(cleanup))=>return Err(cleanup),
            (Err(primary),Err(cleanup))=>return Err(E::Cleanup{primary:Some(Box::new(primary)),cleanup:Box::new(cleanup)}),
        }
        let after=root.read_work(work_id)?;
        let previous_calls=before.planning.as_deref().and_then(|p|p.reviewer.as_ref())
            .ok_or(E::Refused("unstarted-producer-reviewer-required"))?.calls;
        let current_calls=after.planning.as_deref().and_then(|p|p.reviewer.as_ref())
            .ok_or(E::Refused("unstarted-producer-reviewer-required"))?.calls;
        if previous_calls.checked_add(1)!=Some(current_calls) {
            return Err(E::Refused("unstarted-producer-call-count"));
        }
        let finance=||->Result<(String,String),E>{
            crate::agent_launch::query(&root.db,
                "SELECT CAST(estimate AS TEXT),state FROM planning_calls WHERE id=? AND work_id=?",
                rusqlite::params![job.id,work_id],|rows|{
                    let r=rows.next()?.ok_or(crate::AgentLaunchError::Refused("unstarted-producer-funding-required"))?;
                    let value=(r.get(0)?,r.get(1)?);
                    if rows.next()?.is_some(){return Err(crate::AgentLaunchError::Refused("unstarted-producer-funding-ambiguous"));}
                    Ok(value)
                }).map_err(|e|E::Launch(Box::new(e)))
        };
        let funding=finance()?;
        let witness=crate::encode_model(&serde_json::json!({
            "scope":"fresh real claim, not reviewer execution",
            "job":job,"work":after,"funding":funding,
            "original_expiry":job.preflight_until,"owner_alive":true
        })).map_err(E::Encode)?;
        publish_capture(&evidence.join("fresh-witness.json"),&witness)
            .map_err(|e|E::Launch(Box::new(crate::AgentLaunchError::caused("unstarted-producer-witness",e))))?;
        let deadline=swarm_wire_json::go_rfc3339::parse(&job.preflight_until)
            .map_err(|e|E::Launch(Box::new(crate::AgentLaunchError::caused("unstarted-producer-clock",e))))?;
        let mut sequence=0usize;
        loop {
            let stamp=crate::system_timestamp().map_err(E::Runtime)?;
            let now=swarm_wire_json::go_rfc3339::parse(&stamp)
                .map_err(|e|E::Launch(Box::new(crate::AgentLaunchError::caused("unstarted-producer-clock",e))))?;
            let observed=crate::encode_model(&serde_json::json!({
                "now":stamp,"original_expiry":job.preflight_until,"job":job.id
            })).map_err(E::Encode)?;
            publish_capture(&evidence.join(format!("clock-{sequence}.json")),&observed)
                .map_err(|e|E::Launch(Box::new(crate::AgentLaunchError::caused("unstarted-producer-clock-proof",e))))?;
            if now>=deadline {break;}
            let remaining=std::time::Duration::try_from(deadline-now)
                .map_err(|e|E::Launch(Box::new(crate::AgentLaunchError::caused("unstarted-producer-wait",e))))?;
            std::thread::sleep(remaining);
            sequence=sequence.checked_add(1).ok_or(E::Refused("unstarted-producer-sequence-overflow"))?;
        }
        unstarted::expired(job)?;
        let final_job=store::job(&root.db,&job.id).map_err(|e|E::Launch(Box::new(e)))?
            .ok_or(E::Refused("unstarted-producer-job-required"))?;
        if final_job!=*job || root.read_work(work_id)?!=after || finance()?!=funding {
            return Err(E::Refused("unstarted-producer-history-changed"));
        }
        let receipt=root.directory.process_path().join(super::journal::name(&job.id));
        match fs::symlink_metadata(receipt) {
            Err(error) if error.kind()==std::io::ErrorKind::NotFound=>{},
            Err(error)=>return Err(E::Launch(Box::new(crate::AgentLaunchError::caused("unstarted-producer-original-observation",error)))),
            Ok(_)=>return Err(E::Refused("unstarted-producer-unexpected-original")),
        }
        Ok(serde_json::json!({
            "root":path,"work":work_id,"token":token,"review":job.id,
            "job":job,"original_expiry":job.preflight_until,"funding":funding,
            "scope":"expired before helper; parent must observe process exit and departure",
            "owner_departure_verified":false,"provider_started":false,"verdict":false
        }))
    })();
    // Capture recorded metadata BEFORE safe projection, publication or close.
    // record may have committed and then failed release without returning a Job.
    let known_work=known_work.or_else(||run.as_ref().err().and_then(|error|
        error.recorded_work().map(|(work,revision)|(work.to_owned(),revision))));
    let claim_outcome=if replay_observed {"existing-job-observed"}
        else if known_work.is_some() {"recorded-work-known"}
        else if claim_attempted {"unknown"} else {"not-attempted"};
    let recorded=known_work.as_ref().map(|(work,revision)|
        serde_json::json!({"work":work,"revision":revision}));
    let observed=serde_json::json!({
        "result":run.as_ref().ok(),
        "failure":run.as_ref().err().map(|error|crate::native_diagnostics::project(error)),
        "claim_attempted":claim_attempted,"claim_outcome":claim_outcome,
        "known_recorded_work":recorded,"known_claim":known_claim,
        "scope":"observation before Root close; no retry, refund or acceptance",
        "owner_departure_verified":false
    });
    let publication=publish_producer_report(&evidence.join("producer-observation.json"),&observed);
    let close=root.close();
    let outcome=finish_unstarted_producer(run.map(|_|()),publication,close,&known_work);
    let final_report=serde_json::json!({
        "complete":outcome.is_ok(),"claim_outcome":claim_outcome,
        "known_recorded_work":recorded,
        "failure":outcome.as_ref().err().map(|error|crate::native_diagnostics::project(error)),
        "scope":"after checked Root close; final report publication is a separate fallible step",
        "owner_departure_verified":false
    });
    let final_publication=publish_producer_report(&evidence.join("producer-finalization.json"),&final_report);
    // A report that was visible before fsync/close failed never overrides Err.
    finish_unstarted_producer(outcome,final_publication,Ok(()),&known_work)
        .map_err(|error|Box::new(error) as Box<dyn std::error::Error>)
}

/// Encode and publish one private fixture observation without flattening causes.
/// The caller owns the fresh output path. Encode/IO failures remain typed; a
/// visible file alone proves neither successful publication nor producer success.
fn publish_producer_report(path:&Path,value:&serde_json::Value)->Result<(),E> {
    let raw=crate::encode_model(value).map_err(E::Encode)?;
    publish_capture(path,&raw).map_err(|error|E::Launch(Box::new(
        crate::AgentLaunchError::caused("unstarted-producer-proof-publication",error))))
}

/// Combine actual producer, proof-publication and checked Root-close outcomes.
/// Known Work metadata comes only from a returned Staged Work or recorded_work(),
/// never from the request or an invented Job. All original typed branches remain
/// reachable through existing Cleanup/WorkRecorded envelopes. None means unknown,
/// not rollback; successful cleanup grants no retry, refund or acceptance. No IO.
fn finish_unstarted_producer(
    run:Result<(),E>,publication:Result<(),E>,close:Result<(),E>,
    known_work:&Option<(String,i64)>,
)->Result<(),E> {
    let result=crate::root_store::finished(crate::root_store::finished(run,publication),close);
    result.map_err(|primary|match known_work {
        Some((work,revision))=>E::WorkRecorded{
            primary:Box::new(primary),work:work.clone(),revision:*revision},
        None=>primary,
    })
}

/// Typed counterexamples only: no real transaction, Root or child is created.
/// A healthy finalization precedes each synthetic failure; preserve known Work,
/// unknown commit cause and independent publication/close branches. This is not
/// an executed claim witness or SQL fault-injection qualification.
#[test]
fn unstarted_producer_finalization_preserves_known_and_unknown_effects() {
    let known=Some(("fixture-work".to_owned(),3));
    for case in 0..5 {
        finish_unstarted_producer(Ok(()),Ok(()),Ok(()),&known).unwrap();
        let release=||E::Refused("fixture-claim-release");
        let recorded=||E::WorkRecorded{primary:Box::new(release()),work:"fixture-work".into(),revision:3};
        let io=|code,error|E::Launch(Box::new(crate::AgentLaunchError::caused(
            code,std::io::Error::from_raw_os_error(error))));
        let (run,publication,close,metadata)=match case {
            0=>(Err(E::Refused("fixture-before-claim")),Ok(()),Ok(()),None),
            // Exact06 counterexample: record returns Err after known effect;
            // no Staged/Job was returned and caller metadata is still None.
            1=>(Err(recorded()),Ok(()),Ok(()),None),
            2=>(Err(E::Launch(Box::new(crate::AgentLaunchError::Commit{
                cause:Box::new(crate::AgentLaunchError::caused("fixture-commit",std::io::Error::from_raw_os_error(5))),
                rollback_confirmed:false,cleanup:None}))),Ok(()),Ok(()),None),
            3=>(Err(recorded()),Err(io("fixture-proof",28)),Err(io("fixture-close",13)),known.clone()),
            _=>(Ok(()),Ok(()),Err(io("fixture-close",13)),known.clone()),
        };
        let error=finish_unstarted_producer(run,publication,close,&metadata).unwrap_err();
        assert_eq!(error.recorded_work(),if matches!(case,1|3|4){Some(("fixture-work",3))}else{None});
        let diagnostics=crate::native_diagnostics::project(&error);
        if case==2 {
            assert!(diagnostics.iter().any(|v|v.labels.iter().any(|s|s=="agent-launch-commit-uncertain")));
            assert!(diagnostics.iter().any(|v|v.io_errno==Some(5)));
        }
        if case==3 {
            for errno in [28,13] {assert!(diagnostics.iter().any(|v|v.io_errno==Some(errno)));}
            let E::WorkRecorded{primary,..}=error else {panic!("known state lost")};
            let E::Cleanup{primary:Some(primary),cleanup}=*primary else {panic!("close branch lost")};
            assert!(matches!(*cleanup,E::Launch(_)));
            let E::Cleanup{primary:Some(primary),cleanup}=*primary else {panic!("publication branch lost")};
            assert!(matches!(*primary,E::WorkRecorded{..}) && matches!(*cleanup,E::Launch(_)));
        }
    }
}


/// Own the original error through the real libtest Result/Termination boundary.
/// Debug exposes only the common safe diagnostic graph and recorded Work metadata,
/// never Debug/Display of the private cause. The parent must retain stderr; this
/// is no durable-output guarantee, permission, retry or successful report file.
struct ProducerTerminalFailure {
    cause:Box<dyn std::error::Error>,
}
impl std::fmt::Debug for ProducerTerminalFailure {
    /// Render the safe branches even when producer-finalization could not be
    /// published. Unknown metadata stays None. The original cause remains owned.
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {
        f.debug_struct("ProducerTerminalFailure")
            .field("known_recorded_work",&producer_recorded_work(self.cause.as_ref()))
            .field("causes",&crate::native_diagnostics::project(self.cause.as_ref()))
            .finish()
    }
}
impl std::fmt::Display for ProducerTerminalFailure {
    /// Keep Display as safe as the terminal Debug representation, without
    /// flattening the owned original or printing any free-form private message.
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {
        std::fmt::Debug::fmt(self,f)
    }
}
impl std::error::Error for ProducerTerminalFailure {
    /// Retain the original typed early/Root failure until Termination consumes
    /// this owner. Compound cleanup branches remain in that original graph.
    fn source(&self)->Option<&(dyn std::error::Error+'static)> {Some(self.cause.as_ref())}
}

/// Wrap every producer failure, including input/Root-open errors before claim.
/// No projection replaces the original typed object and no fallible proof file
/// is required to construct this terminal owner. No IO or authority transition.
fn producer_terminal_error(cause:Box<dyn std::error::Error>)->Box<dyn std::error::Error> {
    Box::new(ProducerTerminalFailure{cause})
}

/// Find recorded Work in the existing typed error graph without private text.
/// Root::recorded_work retains its own known-identity/revision semantics. Known
/// box/custom-IO/cleanup branches are traversed; unknown wrappers follow source.
/// Cycles terminate by typed pointer identity, without a policy depth cutoff.
/// None means unknown, never no effect; this observation performs no IO.
fn producer_recorded_work(error:&(dyn std::error::Error+'static))->Option<(String,i64)> {
    let mut pending=vec![error];
    let mut seen:Vec<*const dyn std::error::Error>=vec![];
    while let Some(error)=pending.pop() {
        let pointer=error as *const dyn std::error::Error;
        if seen.iter().any(|prior|std::ptr::eq(*prior,pointer)){continue;}
        seen.push(pointer);
        if let Some(root)=error.downcast_ref::<E>() {
            if let Some((work,revision))=root.recorded_work(){return Some((work.to_owned(),revision));}
        }
        for (_,child) in crate::native_error_links(error).children.into_iter().rev(){pending.push(child);}
    }
    None
}

/// Exercise the actual boxed terminal representation and stable Termination
/// trait with synthetic errors only. A successful Result control precedes each
/// refusal. No Root, SQL fault, proof write, child or provider is created. The
/// formatted bytes are checked before report consumes the owner; actual parent
/// stderr capture, OS/profile and durable evidence still require qualification.
#[test]
fn unstarted_producer_terminal_retains_safe_branches_without_final_file() {
    use std::process::{ExitCode,Termination};
    for case in 0..3 {
        let healthy:Result<(),Box<dyn std::error::Error>>=Ok(());
        assert_eq!(healthy.report(),ExitCode::SUCCESS);
        let io=|code,errno|E::Launch(Box::new(crate::AgentLaunchError::caused(
            code,std::io::Error::from_raw_os_error(errno))));
        let cause:Box<dyn std::error::Error>=match case {
            0=>{
                let known=Some(("fixture-work".into(),3));
                let release=E::WorkRecorded{work:"fixture-work".into(),revision:3,
                    primary:Box::new(E::Launch(Box::new(crate::AgentLaunchError::caused(
                        "fixture-release",std::io::Error::other("PRIVATE release detail")))))};
                let closed=finish_unstarted_producer(Err(release),Ok(()),Err(io("fixture-close",13)),&known);
                Box::new(finish_unstarted_producer(closed,Err(io("fixture-final-publication",28)),Ok(()),&known).unwrap_err())
            },
            1=>Box::new(E::Launch(Box::new(crate::AgentLaunchError::Commit{
                cause:Box::new(crate::AgentLaunchError::caused("fixture-commit",std::io::Error::from_raw_os_error(5))),
                rollback_confirmed:false,cleanup:None}))),
            _=>Box::new(std::io::Error::other("PRIVATE early-input detail")),
        };
        // Same boxed type and wrapping function used by the real test entry.
        let terminal=producer_terminal_error(cause);
        assert!(terminal.source().is_some());
        let rendered=format!("Error: {terminal:?}");
        assert!(rendered.contains("ProducerTerminalFailure") && !rendered.contains("PRIVATE"));
        if case==0 {
            assert_eq!(producer_recorded_work(terminal.as_ref()),Some(("fixture-work".into(),3)));
            assert!(rendered.contains("fixture-work") && rendered.contains("Some(13)") && rendered.contains("Some(28)"));
            assert!(terminal.source().unwrap().downcast_ref::<E>().is_some());
        } else {
            assert_eq!(producer_recorded_work(terminal.as_ref()),None);
            if case==1 {assert!(rendered.contains("agent-launch-commit-uncertain") && rendered.contains("Some(5)"));}
            else {assert!(terminal.source().unwrap().downcast_ref::<std::io::Error>().is_some());}
        }
        let returned:Result<(),Box<dyn std::error::Error>>=Err(terminal);
        assert_eq!(returned.report(),ExitCode::FAILURE);
    }
}
