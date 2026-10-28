//! Default-profile native manual preparation, actual CLI and output failures.
#![cfg(target_os = "linux")]
use std::{
    fs,
    io::{self, Write},
    path::Path,
    process::{Command, Output},
};
use swarm_engine::{RootStore, SystemEnvironment};
/// Invoke only this test's owned root through the actual compiled CLI binary.
fn call(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .args(args)
        .arg("--root")
        .arg(root)
        .output()
        .unwrap()
}
/// Public root, Work/task mutations and strict native request; local executable
/// is observed only. Admission starts no provider and performs no paid call.
fn fixture() -> (tempfile::TempDir, String, std::path::PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let mut store = RootStore::initialize(root.path(), &mut SystemEnvironment).unwrap();
    let work=store.execute_external("","work.create",br#"{"schema_version":1,"event_id":"create","expected_revision":0,"title":"Title","objective":"Goal","scope":"Scope","criteria":["Proof"]}"#,&mut SystemEnvironment).unwrap().result.work.id;
    store.execute_external(&work,"task.add",br#"{"schema_version":1,"event_id":"add","expected_revision":1,"id":"task","title":"Task","deliverable":"Report","criteria":["Proof"]}"#,&mut SystemEnvironment).unwrap();
    store.close().unwrap();
    fs::write(
        root.path().join(".swarm/providers.json"),
        br#"{"schema_version":1,"providers":{"fixture":{"command":"/bin/cat","args":[]}}}"#,
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(
        root.path().join(".swarm/providers.json"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let input = root.path().join("launch.json");
    fs::write(&input,br#"{"schema_version":1,"event_id":"session","expected_revision":2,"task_id":"task","provider":"fixture"}"#).unwrap();
    (root, work, input)
}
/// Preview leaves no intention, real preparation records once, show/reconcile
/// consume its actual output. No fake PID, process or acceptance is inferred.
#[test]
fn agent_cli_manual_prepare_preview_replay_show_and_cancel() {
    let (root, work, input) = fixture();
    for operation in ["preview", "prepare", "prepare"] {
        let o = call(
            root.path(),
            &[
                "agent",
                operation,
                &work,
                "--input",
                input.to_str().unwrap(),
                "--json",
            ],
        );
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
        assert_eq!(v["preview_only"], operation == "preview");
        assert_eq!(v["process_started"], false);
        assert!(v["agent"].get("child_pid").is_none());
        assert_eq!(v["agent"]["status"], "queued");
        assert!(v["agent"]["prompt"].as_str().unwrap().contains(&format!(
            "mission {work} ; tâche task ; agent session ; tentative {}",
            v["agent"]["attempt_id"].as_str().unwrap()
        )));
        assert!(v["agent"]["prompt"]
            .as_str()
            .unwrap()
            .contains("révision du travail 3"));
    }
    let show = call(root.path(), &["agent", "show", "session", "--json"]);
    assert!(show.status.success());
    let v: serde_json::Value = serde_json::from_slice(&show.stdout).unwrap();
    assert_eq!(v["agent"]["task_id"], "task");
    let cancel = call(root.path(), &["agent", "reconcile", "session", "--json"]);
    assert!(
        cancel.status.success(),
        "{}",
        String::from_utf8_lossy(&cancel.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&cancel.stdout).unwrap();
    assert_eq!(v["cancelled_before_start"], true);
    assert_eq!(v["agent"]["status"], "interrupted");
}
/// Both write and flush loss stay failures after durable preparation; reread and
/// idempotent replay recover without creating a second attempt or launching.
#[test]
fn agent_cli_prepared_intention_survives_write_and_flush_failure() {
    struct Broken {
        flush: bool,
    }
    impl Write for Broken {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            if self.flush {
                Ok(b.len())
            } else {
                Err(io::Error::new(io::ErrorKind::BrokenPipe, "PRIVATE WRITE"))
            }
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "PRIVATE FLUSH"))
        }
    }
    for flush in [false, true] {
        let (root, work, input) = fixture();
        let args = vec![
            "agent".into(),
            "prepare".into(),
            work.clone(),
            "--root".into(),
            root.path().to_string_lossy().into_owned(),
            "--input".into(),
            input.to_string_lossy().into_owned(),
            "--json".into(),
            "--lang".into(),
            "en".into(),
        ];
        let mut err = Vec::new();
        let outcome = swarm_cli::run_detailed(&args, "en", &mut Broken { flush }, &mut err);
        assert_eq!(outcome.code, 2);
        assert!(matches!(
            outcome.error,
            Some(swarm_cli::CliError::AgentLaunchPreserved { .. })
        ));
        let message = String::from_utf8(err).unwrap();
        assert!(message.contains("Launch intention is recorded"));
        assert!(!message.contains("PRIVATE"));
        let store = RootStore::open(root.path(), false).unwrap();
        assert_eq!(store.read_agent("session").unwrap().status, "queued");
        assert_eq!(store.list_agents(&work).unwrap().len(), 1);
        store.close().unwrap();
        let replay = call(
            root.path(),
            &[
                "agent",
                "prepare",
                &work,
                "--input",
                input.to_str().unwrap(),
                "--json",
            ],
        );
        assert!(replay.status.success());
        let v: serde_json::Value = serde_json::from_slice(&replay.stdout).unwrap();
        assert_eq!(v["created"], false);
    }
}

/// Actual default-profile CLI diagnostic consumes the stored frozen snapshot,
/// refuses malformed events safely and never changes the retained queued agent.
#[test]
fn agent_cli_guard_replays_declared_events_without_mutation() {
    let (root, work, input) = fixture();
    assert!(call(
        root.path(),
        &[
            "agent",
            "prepare",
            &work,
            "--input",
            input.to_str().unwrap(),
            "--json"
        ]
    )
    .status
    .success());
    let show = || call(root.path(), &["agent", "show", "session", "--json"]).stdout;
    let before = show();
    let trace = root.path().join("trace.json");
    fs::write(&trace,br#"{"schema_version":1,"events":[{"call":{"at_ms":0,"id":"A","name":"Read","input":{"PRIVATE":true}}},{"result":{"at_ms":1,"id":"A","failed":true,"message":"PRIVATE permission denied"}}],"final_at_ms":1}"#).unwrap();
    let output = call(
        root.path(),
        &[
            "agent",
            "guard",
            "session",
            "--input",
            trace.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["observation"]["total_errors"], 1);
    assert_eq!(report["preview_only"], true);
    assert_eq!(report["process_started"], false);
    assert_eq!(report["event_source"], "caller_declared_unverified");
    assert!(!String::from_utf8(output.stdout)
        .unwrap()
        .contains("PRIVATE"));
    assert_eq!(show(), before);
    fs::write(
        &trace,
        br#"{"schema_version":1,"events":[{"result":{"at_ms":0,"id":"PRIVATE"}}],"final_at_ms":0}"#,
    )
    .unwrap();
    let output = call(
        root.path(),
        &[
            "agent",
            "guard",
            "session",
            "--input",
            trace.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(!output.status.success());
    assert!(!String::from_utf8(output.stderr)
        .unwrap()
        .contains("PRIVATE"));
    assert_eq!(show(), before);
}

/// Actual CLI exposes safe retry exhaustion, not success/secret request data.
/// A real held writer is released explicitly before admission/replay recovery;
/// no elapsed deadline, paid provider or extra retry budget is introduced.
#[test]
fn agent_cli_saved_storage_policy_refuses_then_recovers_without_duplicate() {
    use swarm_engine::{EngineSettingsChange, SettingCount};
    let (root, work, input) = fixture();
    let mut request: serde_json::Value =
        serde_json::from_slice(&fs::read(&input).unwrap()).unwrap();
    request["instruction"] = "PRIVATE_NEVER_LOG".into();
    fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
    assert!(call(
        root.path(),
        &[
            "agent",
            "preview",
            &work,
            "--input",
            input.to_str().unwrap(),
            "--json"
        ]
    )
    .status
    .success());
    let store = RootStore::open(root.path(), true).unwrap();
    let doc = store.settings().unwrap();
    let mut values = doc.values;
    values.storage_busy_timeout_ms = SettingCount(0);
    values.storage_busy_retries = Some(SettingCount(1));
    values.storage_busy_retry_delay_ms = SettingCount(0);
    let mut change = EngineSettingsChange {
        schema_version: doc.schema_version,
        event_id: "cli-storage-policy".into(),
        expected_revision: doc.revision,
        values,
        reason: "Private actual CLI held writer".into(),
        preview_token: String::new(),
    };
    change.preview_token = store.preview_settings(&change).unwrap().preview_token;
    store
        .apply_settings(&change, &mut SystemEnvironment)
        .unwrap();
    store.close().unwrap();
    let mut writer = rusqlite::Connection::open(root.path().join(".swarm/state.db")).unwrap();
    let counts = |db: &rusqlite::Connection| -> Vec<i64> {
        ["agents", "reservations", "events", "agent_logs"]
            .into_iter()
            .map(|table| {
                db.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                    .unwrap()
            })
            .collect()
    };
    let before = counts(&writer);
    let held = writer
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .unwrap();
    let output = call(
        root.path(),
        &[
            "agent",
            "prepare",
            &work,
            "--input",
            input.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(counts(&held), before);
    let wire: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(wire["failure"]["code"], "agent-launch-storage-busy");
    assert_eq!(wire["failure"]["retryable"], false);
    let message = wire["failure"]["message"].as_str().unwrap();
    assert!(message.contains("storage_attempts=2"));
    assert!(message.contains("settings_revision=2"));
    assert!(!String::from_utf8(output.stderr)
        .unwrap()
        .contains("PRIVATE_NEVER_LOG"));
    held.rollback().unwrap();
    writer.close().unwrap();
    for created in [true, false] {
        let output = call(
            root.path(),
            &[
                "agent",
                "prepare",
                &work,
                "--input",
                input.to_str().unwrap(),
                "--json",
            ],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let wire: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(wire["created"], created);
        assert_eq!(wire["process_started"], false);
    }
    let store = RootStore::open(root.path(), false).unwrap();
    assert_eq!(store.list_agents(&work).unwrap().len(), 1);
    store.close().unwrap();
}

/// Actual default-feature CLI must run a configured local Python fixture, not
/// merely prepare metadata. The provider independently records PID/PPID and the
/// complete stdin digest. Successful exit blocks the task for handoff; replay
/// never starts a second provider. No paid service, sleep or global watchdog.
#[test]
fn native_manual_agent_run_has_real_input_kernel_identity_and_no_task_acceptance() {
    use sha2::{Digest, Sha256};
    let (root, work, input) = fixture();
    let proof = root.path().join("provider-proof.json");
    let script="import os,sys,json,hashlib; raw=sys.stdin.buffer.read(); p=sys.argv[1]; prior=json.load(open(p))['runs'] if os.path.exists(p) else 0; json.dump({'pid':os.getpid(),'ppid':os.getppid(),'input_bytes':len(raw),'input_sha256':hashlib.sha256(raw).hexdigest(),'runs':prior+1},open(p,'w')); sys.stdout.buffer.write(raw)";
    let providers = serde_json::json!({"schema_version":1,"providers":{"fixture":{"command":"/usr/bin/python3","args":["-c",script,proof.to_str().unwrap()]}}});
    fs::write(
        root.path().join(".swarm/providers.json"),
        serde_json::to_vec(&providers).unwrap(),
    )
    .unwrap();
    let prepared = call(
        root.path(),
        &[
            "agent",
            "prepare",
            &work,
            "--input",
            input.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(
        prepared.status.success(),
        "{}",
        String::from_utf8_lossy(&prepared.stderr)
    );
    let prepared: serde_json::Value = serde_json::from_slice(&prepared.stdout).unwrap();
    let prompt = prepared["agent"]["prompt"].as_str().unwrap();
    let output = call(root.path(), &["agent", "run", "session", "--json"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let outcome: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(outcome["task_accepted"], false);
    assert_eq!(outcome["agent"]["status"], "completed");
    assert_eq!(outcome["agent"]["exit_code"], 0);
    let witness: serde_json::Value = serde_json::from_slice(&fs::read(&proof).unwrap()).unwrap();
    assert_eq!(witness["input_bytes"], prompt.len());
    assert_eq!(
        witness["input_sha256"],
        format!("{:x}", Sha256::digest(prompt.as_bytes()))
    );
    assert_eq!(witness["pid"], outcome["agent"]["child_pid"]);
    assert_eq!(witness["ppid"], outcome["agent"]["supervisor_pid"]);
    assert_ne!(witness["pid"], witness["ppid"]);
    assert_eq!(witness["runs"], 1);
    let shown = call(root.path(), &["work", "show", &work, "--json"]);
    assert!(shown.status.success());
    let shown: serde_json::Value = serde_json::from_slice(&shown.stdout).unwrap();
    let task = &shown["work"]["tasks"][0];
    assert_eq!(task["status"], "blocked");
    assert_eq!(task["attempts"][0]["status"], "completed");
    let again = call(root.path(), &["agent", "run", "session", "--json"]);
    assert!(!again.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&fs::read(&proof).unwrap()).unwrap()["runs"],
        1
    );
}

/// Public settings preview/apply/reopen; values are explicit fixture choices,
/// not hidden product policies or replacement test watchdogs.
fn supervision_settings(
    root: &Path,
    event: &str,
    change: impl Fn(&mut swarm_engine::EngineSettingsValues),
) {
    loop {
        let store = RootStore::open(root, true).unwrap();
        let current = store.settings().unwrap();
        let mut values = current.values;
        change(&mut values);
        let mut request = swarm_engine::EngineSettingsChange {
            schema_version: values.schema_version(),
            event_id: event.into(),
            expected_revision: current.revision,
            values,
            reason: "Native supervision isolated regression fixture".into(),
            preview_token: String::new(),
        };
        request.preview_token = store.preview_settings(&request).unwrap().preview_token;
        let result = store.apply_settings(&request, &mut SystemEnvironment);
        store.close().unwrap();
        match result {
            Ok(_) => return,
            Err(swarm_engine::RootStoreError::Storage(error))
                if error.operation == swarm_engine::LocalStorageOperation::Lock
                    && std::error::Error::source(&error)
                        .and_then(|e| e.downcast_ref::<rustix::io::Errno>())
                        .is_some_and(|e| *e == rustix::io::Errno::AGAIN) =>
            {
                // This fixture synchronizes on actual kernel lock release after
                // a real API contention refusal. It adds no product retry or
                // deadline and never treats the refused apply as successful.
                let lock = fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(root.join(".swarm/.engine-settings.lock"))
                    .unwrap();
                rustix::fs::flock(&lock, rustix::fs::FlockOperation::LockExclusive).unwrap();
                rustix::fs::flock(&lock, rustix::fs::FlockOperation::Unlock).unwrap();
                drop(lock);
            }
            Err(e) => panic!("settings apply failed: {e}"),
        }
    }
}
/// Actual independent local provider writes READY over its private test socket.
/// Poll both listener and CLI pidfd, with no arbitrary global deadline, so a
/// startup refusal cannot masquerade as readiness or leave a blocked accept.
fn ready_provider(
    listener: &std::os::unix::net::UnixListener,
    child: &std::process::Child,
) -> std::os::unix::net::UnixStream {
    use rustix::{
        event::{poll, PollFd, PollFlags},
        process::{pidfd_open, Pid, PidfdFlags},
    };
    use std::io::Read;
    listener.set_nonblocking(true).unwrap();
    let fd = pidfd_open(
        Pid::from_raw(i32::try_from(child.id()).unwrap()).unwrap(),
        PidfdFlags::empty(),
    )
    .unwrap();
    loop {
        match listener.accept() {
            Ok((mut socket, _)) => {
                let mut marker = [0];
                socket.read_exact(&mut marker).unwrap();
                assert_eq!(marker, [b'R']);
                return socket;
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("provider handshake: {e}"),
        }
        let mut fds = [
            PollFd::new(listener, PollFlags::IN),
            PollFd::new(&fd, PollFlags::IN),
        ];
        match poll(&mut fds, None) {
            Err(rustix::io::Errno::INTR) => continue,
            Ok(_) => {}
            Err(e) => panic!("provider readiness poll: {e}"),
        }
        if fds[0].revents().contains(PollFlags::IN) {
            continue;
        }
        assert!(
            fds[1].revents().is_empty(),
            "native CLI exited before actual provider READY"
        );
    }
}
/// Real provider death, ownership, concurrent duplicate refusal and public stop
/// are observed for TERM and immediate configured KILL escalation. A settings
/// update while running proves the startup lock is released; frozen policy stays
/// unchanged. Signal exit0 never accepts or completes an interrupted task.
#[test]
fn native_agent_run_public_stop_term_kill_and_admin_edit_are_owned() {
    use std::{os::unix::net::UnixListener, process::Stdio};
    for ignore_term in [false, true] {
        let (root, work, input) = fixture();
        supervision_settings(root.path(), "configure-supervision", |v| {
            v.agent_supervision_poll_interval_ms = Some(swarm_engine::SettingCount(1));
            if ignore_term {
                v.agent_supervision_term_grace_ms = Some(swarm_engine::SettingCount(0));
            }
        });
        let socket_path = root.path().join("ready.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();
        let script=format!("import os,sys,socket,signal; signal.signal(signal.SIGTERM,{}); s=socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); s.sendall(b'R'); s.recv(1)",if ignore_term {"signal.SIG_IGN"}else{"lambda *_:(os.write(1,b'TERM observed\\n'),os._exit(0))"});
        fs::write(root.path().join(".swarm/providers.json"),serde_json::to_vec(&serde_json::json!({"schema_version":1,"providers":{"fixture":{"command":"/usr/bin/python3","args":["-c",script,socket_path.to_str().unwrap()]}}})).unwrap()).unwrap();
        let prepared = call(
            root.path(),
            &[
                "agent",
                "prepare",
                &work,
                "--input",
                input.to_str().unwrap(),
                "--json",
            ],
        );
        assert!(
            prepared.status.success(),
            "{}",
            String::from_utf8_lossy(&prepared.stderr)
        );
        let output_path = root.path().join("run.stdout");
        let error_path = root.path().join("run.stderr");
        let mut child = Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
            .args(["agent", "run", "session", "--json", "--root"])
            .arg(root.path())
            .stdout(Stdio::from(fs::File::create(&output_path).unwrap()))
            .stderr(Stdio::from(fs::File::create(&error_path).unwrap()))
            .spawn()
            .unwrap();
        let socket = ready_provider(&listener, &child);
        let duplicate = call(root.path(), &["agent", "run", "session", "--json"]);
        assert!(!duplicate.status.success());
        supervision_settings(root.path(), "edit-while-running", |v| {
            v.agent_supervision_heartbeat_stale_ms = Some(swarm_engine::SettingCount(1))
        });
        let shown = call(root.path(), &["agent", "show", "session", "--json"]);
        assert!(shown.status.success());
        let shown: serde_json::Value = serde_json::from_slice(&shown.stdout).unwrap();
        assert_eq!(shown["agent"]["status"], "running");
        assert_eq!(shown["supervision_heartbeat"]["stale_after_ms"], "10000");
        let stopped = call(root.path(), &["agent", "stop", "session", "--json"]);
        assert!(
            stopped.status.success(),
            "{}",
            String::from_utf8_lossy(&stopped.stderr)
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&stopped.stdout).unwrap()
                ["termination_confirmed"],
            false
        );
        let status = child.wait().unwrap();
        if !status.success() {
            let store = RootStore::open(root.path(), false).unwrap();
            let failed = store.read_agent("session").unwrap();
            store.close().unwrap();
            eprintln!("native stop diagnostic ignore_term={ignore_term}; status={}; desired={}; native_result={:?}",failed.status,failed.desired,failed.native_execution_result.as_ref().map(|v|v.0.get()));
            let retained = root.keep();
            eprintln!("isolated failure root retained at {}", retained.display());
            panic!(
                "native run failed: {}",
                String::from_utf8_lossy(&fs::read(&error_path).unwrap())
            );
        }
        drop(socket);
        let v: serde_json::Value =
            serde_json::from_slice(&fs::read(&output_path).unwrap()).unwrap();
        assert_eq!(v["task_accepted"], false);
        assert_eq!(v["agent"]["status"], "interrupted");
        assert_eq!(v["agent"]["stop_kind"], "operateur");
        assert_eq!(v["agent"]["native_execution_result"]["kernel_reaped"], true);
        assert_eq!(
            v["agent"]["native_execution"]["policy"]["heartbeat_stale_ms"],
            "10000"
        );
        if ignore_term {
            assert!(v["agent"].get("exit_code").is_none());
        } else {
            assert_eq!(v["agent"]["exit_code"], 0);
        }
        let store = RootStore::open(root.path(), false).unwrap();
        let a = store.read_agent("session").unwrap();
        assert!(swarm_engine::observe_linux_process(u32::try_from(a.child_pid).unwrap()).is_err());
        store.close().unwrap();
        let shown = call(root.path(), &["work", "show", &work, "--json"]);
        let shown: serde_json::Value = serde_json::from_slice(&shown.stdout).unwrap();
        assert_eq!(shown["work"]["tasks"][0]["status"], "blocked");
        assert_eq!(
            shown["work"]["tasks"][0]["attempts"][0]["status"],
            "interrupted"
        );
    }
}
/// A valid prepared control precedes a saved stop; run refuses before claiming
/// any child and preserves the intention exactly. Reconcile remains the explicit
/// cancellation operation, not an implicit refund hidden in run's refusal.
#[test]
fn native_agent_run_refuses_prior_stop_without_process_or_mutation() {
    let (root, work, input) = fixture();
    let p = call(
        root.path(),
        &[
            "agent",
            "prepare",
            &work,
            "--input",
            input.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(p.status.success());
    let mut store = RootStore::open(root.path(), true).unwrap();
    store.stop_agent("session", &mut SystemEnvironment).unwrap();
    let before = store.read_agent("session").unwrap();
    store.close().unwrap();
    let run = call(root.path(), &["agent", "run", "session", "--json"]);
    assert!(!run.status.success());
    let store = RootStore::open(root.path(), false).unwrap();
    let after = store.read_agent("session").unwrap();
    assert_eq!(after, before);
    assert_eq!(after.child_pid, 0);
    assert!(after.native_execution.is_none());
    store.close().unwrap();
}
/// Exit7 settles the actual manual attempt as failed and conservatively consumes
/// its reserved estimate. Nonzero process exit creates no accepted business task.
#[test]
fn native_agent_nonzero_exit_settles_failed_attempt_without_inventing_estimate() {
    let (root, work, input) = fixture();
    fs::write(root.path().join(".swarm/providers.json"),br#"{"schema_version":1,"providers":{"fixture":{"command":"/bin/sh","args":["-c","exit 7"]}}}"#).unwrap();
    let p = call(
        root.path(),
        &[
            "agent",
            "prepare",
            &work,
            "--input",
            input.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(p.status.success());
    let run = call(root.path(), &["agent", "run", "session", "--json"]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&run.stdout).unwrap();
    assert_eq!(v["agent"]["status"], "failed");
    assert_eq!(v["agent"]["exit_code"], 7);
    assert_eq!(v["task_accepted"], false);
    let store = RootStore::open(root.path(), false).unwrap();
    let a = store.read_agent("session").unwrap();
    store.close().unwrap();
    assert!(a
        .native_execution_result
        .unwrap()
        .0
        .get()
        .contains("provider_may_have_started\":true"));
    let db = rusqlite::Connection::open(root.path().join(".swarm/state.db")).unwrap();
    let count: i64 = db
        .query_row(
            "SELECT count(*) FROM reservations WHERE agent_id='session'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
    db.close().unwrap();
}
/// A real ENOEXEC after complete preflight uses the private helper channel,
/// settles failed with a retained numeric cause and releases the existing
/// estimate. A valid executable control runs first; no error-text whitelist.
#[test]
fn native_agent_pre_exec_refusal_releases_only_confirmed_unstarted_estimate() {
    use std::os::unix::fs::PermissionsExt;
    for invalid in [false, true] {
        let (root, work, input) = fixture();
        let mut store = RootStore::open(root.path(), true).unwrap();
        store.execute_external(&work,"budget.configure",br#"{"schema_version":1,"event_id":"budget","expected_revision":2,"budget":{"limit_usd":10,"reserve_per_launch_usd":1,"estimate_source":"Private fixture","reference_date":"2026-10-19"}}"#,&mut SystemEnvironment).unwrap();
        store.close().unwrap();
        fs::write(&input,br#"{"schema_version":1,"event_id":"session","expected_revision":3,"task_id":"task","provider":"fixture"}"#).unwrap();
        let executable = if invalid {
            let file = root.path().join("invalid-executable");
            fs::write(
                &file,
                format!(
                    "printf unexpected-shell > '{}'\n",
                    root.path().join("unexpected-shell").display()
                ),
            )
            .unwrap();
            fs::set_permissions(&file, fs::Permissions::from_mode(0o700)).unwrap();
            file
        } else {
            std::path::PathBuf::from("/bin/true")
        };
        fs::write(root.path().join(".swarm/providers.json"),serde_json::to_vec(&serde_json::json!({"schema_version":1,"providers":{"fixture":{"command":executable,"args":[]}}})).unwrap()).unwrap();
        let p = call(
            root.path(),
            &[
                "agent",
                "prepare",
                &work,
                "--input",
                input.to_str().unwrap(),
                "--json",
            ],
        );
        assert!(p.status.success(), "{}", String::from_utf8_lossy(&p.stderr));
        let run = call(root.path(), &["agent", "run", "session", "--json"]);
        assert!(
            run.status.success(),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
        if invalid && root.path().join("unexpected-shell").exists() {
            let retained = root.keep();
            eprintln!("implicit-shell fixture retained at {}", retained.display());
            panic!("provider implicitly executed through a shell");
        }
        let result: serde_json::Value = serde_json::from_slice(&run.stdout).unwrap();
        let facts = &result["agent"]["native_execution_result"];
        assert_eq!(
            result["agent"]["status"],
            if invalid { "failed" } else { "completed" }
        );
        assert_eq!(result["task_accepted"], false);
        assert_eq!(facts["provider_may_have_started"], !invalid);
        if invalid {
            assert_eq!(facts["pre_exec_failure"]["code"], "agent-provider-exec");
            assert_eq!(facts["pre_exec_failure"]["io_errno"], 8);
            assert!(result["agent"].get("exit_code").is_none());
            assert_eq!(facts["owned_process_exit_code"], 2);
        } else {
            assert!(facts["pre_exec_failure"].is_null());
        }
        let db = rusqlite::Connection::open(root.path().join(".swarm/state.db")).unwrap();
        let (amount, state): (f64, String) = db
            .query_row(
                "SELECT amount,state FROM reservations WHERE agent_id='session'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(amount, 1.0);
        assert_eq!(state, if invalid { "released" } else { "estimated" });
        db.close().unwrap();
    }
}
/// Actual provider emits tool declarations or survives its saved one-second
/// observation threshold. Only a tool guard or an explicit public operator stop
/// signals the owned group. Zero retention never erases cumulative tool facts.
#[test]
fn native_agent_guard_stops_but_elapsed_threshold_waits_for_operator_decision() {
    use std::{os::unix::net::UnixListener, process::Stdio};
    for timeout in [false, true] {
        let (root, work, input) = fixture();
        supervision_settings(root.path(), "retention-policy", |v| {
            v.agent_capture_max_bytes = Some(swarm_engine::SettingCap(Some(
                swarm_engine::SettingCount(0),
            )));
            v.agent_capture_max_lines = Some(swarm_engine::SettingCap(Some(
                swarm_engine::SettingCount(0),
            )));
            v.agent_public_message_max_bytes = Some(swarm_engine::SettingCap(Some(
                swarm_engine::SettingCount(0),
            )));
            v.agent_log_message_max_bytes = Some(swarm_engine::SettingCap(Some(
                swarm_engine::SettingCount(0),
            )));
            if timeout {
                v.agent_supervision_term_grace_ms = Some(swarm_engine::SettingCount(0));
            }
        });
        let mut request: serde_json::Value =
            serde_json::from_slice(&fs::read(&input).unwrap()).unwrap();
        request["capture_output"] = true.into();
        if timeout {
            request["timeout_seconds"] = 1.into();
        } else {
            request["limits"] = serde_json::json!({"silence_seconds":180,"tool_seconds":300,"max_tool_calls":1,"max_repeated_calls":4,"max_consecutive_errors":3,"observation_mode":0});
        }
        fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
        let socket_path = root.path().join("ready.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();
        let tool_events="{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"tool_use\",\"id\":\"A\",\"name\":\"Read\",\"input\":{}},{\"type\":\"text\",\"text\":\"PUBLIC omitted\"}]}}\n{\"type\":\"user\",\"message\":{\"content\":[{\"type\":\"tool_result\",\"tool_use_id\":\"A\",\"is_error\":false}]}}\n";
        let payload = serde_json::to_string(if timeout { "" } else { tool_events }).unwrap();
        let script=format!("import os,sys,socket,signal; signal.signal(signal.SIGTERM,{}); s=socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); s.sendall(b'R'); os.write(1,({payload}).encode()); s.recv(1)",if timeout{"signal.SIG_IGN"}else{"lambda *_:os._exit(0)"});
        fs::write(root.path().join(".swarm/providers.json"),serde_json::to_vec(&serde_json::json!({"schema_version":1,"providers":{"fixture":{"command":"/usr/bin/python3","args":["-c",script,socket_path.to_str().unwrap()]}}})).unwrap()).unwrap();
        let p = call(
            root.path(),
            &[
                "agent",
                "prepare",
                &work,
                "--input",
                input.to_str().unwrap(),
                "--json",
            ],
        );
        assert!(p.status.success(), "{}", String::from_utf8_lossy(&p.stderr));
        let output_path = root.path().join("run.stdout");
        let error_path = root.path().join("run.stderr");
        let mut child = Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
            .args(["agent", "run", "session", "--json", "--root"])
            .arg(root.path())
            .stdout(Stdio::from(fs::File::create(&output_path).unwrap()))
            .stderr(Stdio::from(fs::File::create(&error_path).unwrap()))
            .spawn()
            .unwrap();
        let socket = ready_provider(&listener, &child);
        if timeout {
            loop {
                let shown = call(root.path(), &["agent", "show", "session", "--json"]);
                assert!(
                    shown.status.success(),
                    "{}",
                    String::from_utf8_lossy(&shown.stderr)
                );
                let shown: serde_json::Value = serde_json::from_slice(&shown.stdout).unwrap();
                assert_eq!(
                    shown["agent"]["status"], "running",
                    "elapsed threshold must not stop the owned provider"
                );
                if shown["agent"]["progress"]["time_threshold_exceeded"] == true {
                    break;
                }
            }
            let stop = call(root.path(), &["agent", "stop", "session", "--json"]);
            assert!(stop.status.success());
        }
        let status = child.wait().unwrap();
        drop(socket);
        assert!(
            status.success(),
            "{}",
            String::from_utf8_lossy(&fs::read(&error_path).unwrap())
        );
        let result: serde_json::Value =
            serde_json::from_slice(&fs::read(&output_path).unwrap()).unwrap();
        let a = &result["agent"];
        let facts = &a["native_execution_result"];
        assert_eq!(result["task_accepted"], false);
        assert_eq!(a["status"], "interrupted");
        assert_eq!(a["stop_kind"], if timeout { "operateur" } else { "garde" });
        assert_eq!(facts["capture"], "");
        assert!(a.get("reply").is_none());
        if !timeout {
            assert_eq!(a["progress"]["tool_calls"], 1);
            assert_eq!(a["progress"]["tool_results"], 1);
            assert_eq!(facts["observation"]["stop_reason"], "tool_calls");
            assert_eq!(facts["capture_truncated"], true);
            assert_eq!(facts["public_messages_truncated"], true);
        }
        let db = rusqlite::Connection::open(root.path().join(".swarm/state.db")).unwrap();
        let (kind,message):(String,String)=db.query_row("SELECT kind,message FROM agent_logs WHERE agent_id='session' ORDER BY seq DESC LIMIT 1",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
        assert_eq!(kind, "lifecycle-truncated");
        assert_eq!(message, "");
        db.close().unwrap();
    }
}
/// A controlled descendant deliberately leaves the provider group and holds its
/// output FD. The valid control closes those FDs before leader exit. A real drain
/// refusal must preserve the active workspace/estimate, not falsely settle it.
/// Cleanup uses only this fixture's private socket and pidfd observation, no
/// numeric-PID signal or arbitrary verification watchdog.
#[test]
fn native_agent_incomplete_drain_keeps_active_lease_and_reserved_estimate() {
    use std::{io::Read, os::unix::net::UnixListener, process::Stdio};
    for hold_output in [false, true] {
        let (root, work, input) = fixture();
        supervision_settings(root.path(), "drain-policy", |v| {
            v.agent_supervision_output_drain_ms = Some(swarm_engine::SettingCap(Some(
                swarm_engine::SettingCount(0),
            )))
        });
        let mut store = RootStore::open(root.path(), true).unwrap();
        store.execute_external(&work,"budget.configure",br#"{"schema_version":1,"event_id":"budget","expected_revision":2,"budget":{"limit_usd":10,"reserve_per_launch_usd":1,"estimate_source":"Drain fixture","reference_date":"2026-10-19"}}"#,&mut SystemEnvironment).unwrap();
        store.close().unwrap();
        fs::write(&input,br#"{"schema_version":1,"event_id":"session","expected_revision":3,"task_id":"task","provider":"fixture"}"#).unwrap();
        let socket_path = root.path().join("drain.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();
        let script = format!(
            r#"import os,sys,socket,struct
r,w=os.pipe()
pid=os.fork()
if pid:
 os.close(w)
 os.read(r,1)
 os._exit(0)
os.close(r)
os.setsid()
s=socket.socket(socket.AF_UNIX)
s.connect(sys.argv[1])
s.sendall(b'R'+struct.pack('!I',os.getpid()))
if not {}:
 os.close(1)
 os.close(2)
os.write(w,b'R')
os.close(w)
s.recv(1)
os._exit(0)
"#,
            if hold_output { "True" } else { "False" }
        );
        fs::write(root.path().join(".swarm/providers.json"),serde_json::to_vec(&serde_json::json!({"schema_version":1,"providers":{"fixture":{"command":"/usr/bin/python3","args":["-c",script,socket_path.to_str().unwrap()]}}})).unwrap()).unwrap();
        let p = call(
            root.path(),
            &[
                "agent",
                "prepare",
                &work,
                "--input",
                input.to_str().unwrap(),
                "--json",
            ],
        );
        assert!(p.status.success(), "{}", String::from_utf8_lossy(&p.stderr));
        let output = root.path().join("run.stdout");
        let error = root.path().join("run.stderr");
        let mut child = Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
            .args(["agent", "run", "session", "--json", "--root"])
            .arg(root.path())
            .stdout(Stdio::from(fs::File::create(&output).unwrap()))
            .stderr(Stdio::from(fs::File::create(&error).unwrap()))
            .spawn()
            .unwrap();
        let mut socket = ready_provider(&listener, &child);
        let mut pid = [0; 4];
        socket.read_exact(&mut pid).unwrap();
        let pid = u32::from_be_bytes(pid);
        let observation = swarm_engine::observe_linux_process(pid).unwrap();
        assert_eq!(observation.process_group_id(), pid);
        let pidfd = rustix::process::pidfd_open(
            rustix::process::Pid::from_raw(i32::try_from(pid).unwrap()).unwrap(),
            rustix::process::PidfdFlags::empty(),
        )
        .unwrap();
        let status = child.wait().unwrap();
        let store = RootStore::open(root.path(), false).unwrap();
        let agent = store.read_agent("session").unwrap();
        store.close().unwrap();
        let db = rusqlite::Connection::open(root.path().join(".swarm/state.db")).unwrap();
        let state: String = db
            .query_row(
                "SELECT state FROM reservations WHERE agent_id='session'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        db.close().unwrap();
        // Complete owned fixture cleanup before any assertion can panic.
        socket.write_all(b"X").unwrap();
        let mut rest = Vec::new();
        socket.read_to_end(&mut rest).unwrap();
        drop(socket);
        let mut fds = [rustix::event::PollFd::new(
            &pidfd,
            rustix::event::PollFlags::IN,
        )];
        loop {
            match rustix::event::poll(&mut fds, None) {
                Err(rustix::io::Errno::INTR) => continue,
                Ok(_) => break,
                Err(e) => panic!("fixture exit observation: {e}"),
            }
        }
        if hold_output {
            if !agent.is_active() || state != "reserved" {
                let retained = root.keep();
                eprintln!("incomplete-drain fixture retained at {}; status={}; reservation={state}; CLI={status}",retained.display(),agent.status);
                panic!("incomplete drain must not release the active lease or estimate");
            }
            assert!(!status.success());
            assert!(agent.ended.is_empty());
            let facts: serde_json::Value =
                serde_json::from_str(agent.native_execution_result.as_ref().unwrap().0.get())
                    .unwrap();
            assert_eq!(facts["kernel_reaped"], true);
            assert_eq!(facts["owned_process_exit_code"], 0);
            assert_eq!(facts["output_complete"], false);
            assert_eq!(facts["workspace_release_confirmed"], false);
            assert_eq!(facts["incident"], "output_drain_incomplete");
        } else {
            assert!(
                status.success(),
                "{}",
                String::from_utf8_lossy(&fs::read(&error).unwrap())
            );
            assert_eq!(agent.status, "completed");
            assert_eq!(state, "estimated");
        }
    }
}

/// Real local-provider declarations are observations, never a requested route or
/// invoice. Valid control precedes malformed metadata; a later valid event
/// replaces, never sums, earlier token counts. Reopen preserves canonical facts,
/// blocked task and the original estimate even when declared cost exceeds it.
/// Tool counters continue through metadata errors; no paid provider or timeout.
#[test]
fn native_agent_declared_usage_model_invalid_and_recovery_preserve_budget() {
    for variant in ["valid", "invalid", "recovered"] {
        let (root, work, input) = fixture();
        let verdict = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut store = RootStore::open(root.path(), true).unwrap();
            store.execute_external(&work,"budget.configure",br#"{"schema_version":1,"event_id":"budget","expected_revision":2,"budget":{"limit_usd":10,"reserve_per_launch_usd":1,"estimate_source":"Metadata fixture","reference_date":"2026-10-19"}}"#,&mut SystemEnvironment).unwrap();
            store.close().unwrap();
            fs::write(&input,br#"{"schema_version":1,"event_id":"session","expected_revision":3,"task_id":"task","provider":"fixture"}"#).unwrap();
            let mut events = vec![
                serde_json::json!({"type":"system","subtype":"init","model":" asserted-initial ","at":"1900-01-01T00:00:00Z"}),
                serde_json::json!({"type":"turn.completed","usage":{"input_tokens":7,"output_tokens":3,"cached_input_tokens":0},"total_cost_usd":0.01}),
            ];
            if variant != "valid" {
                events.push(serde_json::json!({"type":"assistant","message":{"model":{"PRIVATE":"must-not-leak"},"content":[{"type":"tool_use","id":"metadata-tool","name":"Read","input":{"file":"proof"}}]}}));
                events.push(serde_json::json!({"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"metadata-tool"}]}}));
                events.push(
                serde_json::json!({"type":"result","usage":{"input_tokens":-1,"output_tokens":3}}),
            );
            }
            if variant == "recovered" {
                events.push(
                    serde_json::json!({"type":"system","subtype":"init","model":"asserted-final"}),
                );
                events.push(serde_json::json!({"type":"result","usage":{"input_tokens":0,"output_tokens":1,"cache_read_input_tokens":0,"cache_creation_input_tokens":2,"cached_input_tokens":0},"total_cost_usd":25}));
            }
            let wire = events
                .iter()
                .map(|v| serde_json::to_string(v).unwrap())
                .collect::<Vec<_>>()
                .join("\n");
            // Payload belongs to provider output, not public configured argv.
            // Raw capture is disabled so only diagnostic projection is tested.
            let events_file = root.path().join("provider-events.jsonl");
            fs::write(&events_file, wire).unwrap();
            let script = "import sys,pathlib; sys.stdin.buffer.read(); sys.stdout.buffer.write(pathlib.Path(sys.argv[1]).read_bytes())";
            fs::write(root.path().join(".swarm/providers.json"),serde_json::to_vec(&serde_json::json!({"schema_version":1,"providers":{"fixture":{"command":"/usr/bin/python3","args":["-c",script,events_file.to_str().unwrap()]}}})).unwrap()).unwrap();
            let p = call(
                root.path(),
                &[
                    "agent",
                    "prepare",
                    &work,
                    "--input",
                    input.to_str().unwrap(),
                    "--json",
                ],
            );
            assert!(p.status.success(), "{}", String::from_utf8_lossy(&p.stderr));
            let output = call(root.path(), &["agent", "run", "session", "--json"]);
            fs::write(root.path().join("metadata-run.stdout"), &output.stdout).unwrap();
            fs::write(root.path().join("metadata-run.stderr"), &output.stderr).unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            let a = &result["agent"];
            let f = &a["native_execution_result"]["provider_facts"];
            assert_eq!(a["native_provider_facts"], *f);
            // Existing valid lifecycle control must pass before metadata assertions.
            assert_eq!(a["status"], "completed");
            assert_eq!(a["native_execution_result"]["output_complete"], true);
            assert_eq!(result["task_accepted"], false);
            if variant == "invalid" {
                assert!(a.get("usage").is_none());
                assert!(a.get("reported_model").is_none());
                assert_eq!(f["usage_state"], "invalid");
                assert_eq!(f["model_state"], "invalid");
                assert_eq!(f["last_valid_usage"]["input_tokens"], 7);
                assert_eq!(f["last_valid_reported_model"]["model"], "asserted-initial");
                assert_eq!(a["progress"]["tool_calls"], 1);
                assert_eq!(a["progress"]["tool_results"], 1);
                assert!(!String::from_utf8_lossy(&output.stdout).contains("must-not-leak"));
            } else {
                assert_eq!(
                    a["usage"]["input_tokens"],
                    if variant == "valid" { 7 } else { 0 }
                );
                assert_eq!(
                    a["usage"]["output_tokens"],
                    if variant == "valid" { 3 } else { 1 }
                );
                assert_eq!(a["usage"]["cached_input_tokens"], 0);
                assert_eq!(
                    a["reported_model"]["model"],
                    if variant == "valid" {
                        "asserted-initial"
                    } else {
                        "asserted-final"
                    }
                );
                assert_ne!(a["reported_model"]["at"], "1900-01-01T00:00:00Z");
                assert_eq!(f["usage_state"], "observed");
                assert_eq!(f["model_state"], "observed");
                if variant == "recovered" {
                    assert_eq!(
                        a["usage"]["provider_reported_cost_usd"].as_f64(),
                        Some(25.0)
                    );
                    assert_eq!(a["usage"]["cache_read_input_tokens"], 0);
                    assert_eq!(a["usage"]["cache_creation_input_tokens"], 2);
                }
            }
            assert_eq!(f["source"], "provider_declared_unverified");
            let observed = call(root.path(), &["agent", "show", "session", "--json"]);
            assert!(observed.status.success());
            let shown: serde_json::Value = serde_json::from_slice(&observed.stdout).unwrap();
            assert_eq!(shown["agent"], *a);
            let db = rusqlite::Connection::open(root.path().join(".swarm/state.db")).unwrap();
            let reservation: (String, f64) = db
                .query_row(
                    "SELECT state,amount FROM reservations WHERE agent_id='session'",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .unwrap();
            db.close().unwrap();
            assert_eq!(reservation, ("estimated".into(), 1.0));
            let state = call(root.path(), &["work", "show", &work, "--json"]);
            assert!(state.status.success());
            let state: serde_json::Value = serde_json::from_slice(&state.stdout).unwrap();
            assert_eq!(state["work"]["tasks"][0]["status"], "blocked");
        }));
        if let Err(cause) = verdict {
            let retained = root.keep();
            eprintln!(
                "metadata fixture retained: {}; variant={variant}",
                retained.display()
            );
            std::panic::resume_unwind(cause);
        }
    }
}

/// Public progress is the phase witness: each provider phase ends with a unique
/// successful tool result. Once that count is persisted, its earlier metadata
/// must be in the same heartbeat, including invalid states while still running.
/// Socket release/EOF and real CLI wait precede assertions; no global watchdog,
/// paid call, guessed model, budget reset or tool counter reset is introduced.
#[test]
fn native_agent_running_facts_refuse_then_recover_in_same_owned_process() {
    use std::{io::Read, os::unix::net::UnixListener, process::Stdio};
    let (root, work, input) = fixture();
    let verdict = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        supervision_settings(root.path(), "metadata-heartbeat", |v| {
            v.agent_supervision_poll_interval_ms = Some(swarm_engine::SettingCount(10))
        });
        let phase = |model: serde_json::Value, usage: serde_json::Value, id: &str| {
            [serde_json::json!({"type":"system","subtype":"init","model":model}),serde_json::json!({"type":"result","usage":usage}),serde_json::json!({"type":"assistant","message":{"content":[{"type":"tool_use","id":id,"name":"Read","input":{"file":id}}]}}),serde_json::json!({"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":id}]}})].iter().map(|v|serde_json::to_string(v).unwrap()+"\n").collect::<String>()
        };
        let initial = root.path().join("initial.jsonl");
        let invalid = root.path().join("invalid.jsonl");
        let recovered = root.path().join("recovered.jsonl");
        fs::write(
            &initial,
            phase(
                serde_json::json!("live-initial"),
                serde_json::json!({"input_tokens":7,"output_tokens":3}),
                "first",
            ),
        )
        .unwrap();
        fs::write(
            &invalid,
            phase(
                serde_json::json!({"PRIVATE":"raw-model-metadata-witness-unpublished"}),
                serde_json::json!({"input_tokens":-1,"output_tokens":0}),
                "second",
            ),
        )
        .unwrap();
        fs::write(
            &recovered,
            phase(
                serde_json::json!("live-final"),
                serde_json::json!({"input_tokens":0,"output_tokens":1}),
                "third",
            ),
        )
        .unwrap();
        let socket_path = root.path().join("metadata.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();
        let script="import os,sys,socket,pathlib\nsys.stdin.buffer.read()\ndef emit(p):\n sys.stdout.buffer.write(pathlib.Path(p).read_bytes()); sys.stdout.buffer.flush()\nemit(sys.argv[2])\ns=socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); s.sendall(b'R')\nfor p,marker in [(sys.argv[3],b'B'),(sys.argv[4],b'C')]:\n if s.recv(1)!=marker: sys.exit(0)\n emit(p); s.sendall(marker)\ns.recv(1)\n";
        fs::write(root.path().join(".swarm/providers.json"),serde_json::to_vec(&serde_json::json!({"schema_version":1,"providers":{"fixture":{"command":"/usr/bin/python3","args":["-c",script,socket_path.to_str().unwrap(),initial.to_str().unwrap(),invalid.to_str().unwrap(),recovered.to_str().unwrap()]}}})).unwrap()).unwrap();
        let p = call(
            root.path(),
            &[
                "agent",
                "prepare",
                &work,
                "--input",
                input.to_str().unwrap(),
                "--json",
            ],
        );
        assert!(p.status.success(), "{}", String::from_utf8_lossy(&p.stderr));
        let stdout = root.path().join("active.stdout");
        let stderr = root.path().join("active.stderr");
        let mut child = Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
            .args(["agent", "run", "session", "--json", "--root"])
            .arg(root.path())
            .stdout(Stdio::from(fs::File::create(&stdout).unwrap()))
            .stderr(Stdio::from(fs::File::create(&stderr).unwrap()))
            .spawn()
            .unwrap();
        let mut socket = ready_provider(&listener, &child);
        let mut observations = Vec::new();
        let mut issues = Vec::new();
        'phases: for (index, command) in [None, Some(b'B'), Some(b'C')].into_iter().enumerate() {
            if let Some(command) = command {
                if let Err(e) = socket.write_all(&[command]) {
                    issues.push(format!("fixture phase send: {e}"));
                    break;
                }
                let mut marker = [0u8; 1];
                if let Err(e) = socket.read_exact(&mut marker) {
                    issues.push(format!("fixture phase response: {e}"));
                    break;
                }
                if marker != [command] {
                    issues.push("fixture phase marker mismatch".into());
                    break;
                }
            }
            loop {
                let shown = call(root.path(), &["agent", "show", "session", "--json"]);
                if !shown.status.success() {
                    issues.push(format!(
                        "public show refused: {}",
                        String::from_utf8_lossy(&shown.stderr)
                    ));
                    break 'phases;
                }
                let value: serde_json::Value = match serde_json::from_slice(&shown.stdout) {
                    Ok(v) => v,
                    Err(e) => {
                        issues.push(format!("public show codec: {e}"));
                        break 'phases;
                    }
                };
                let a = &value["agent"];
                let count = a["progress"]["tool_results"].as_i64().unwrap_or(0);
                let degraded = a["progress"]["degraded"].as_str().unwrap_or_default();
                let status = a["status"].as_str().unwrap_or_default();
                if count >= i64::try_from(index + 1).unwrap()
                    || !degraded.is_empty()
                    || !matches!(status, "starting" | "running")
                {
                    observations.push(value);
                    break;
                }
                match child.try_wait() {
                    Ok(None) => {}
                    Ok(Some(_)) => {
                        issues.push("native CLI exited before phase progress".into());
                        break 'phases;
                    }
                    Err(e) => {
                        issues.push(format!("fixture child observation: {e}"));
                        break 'phases;
                    }
                }
                std::thread::yield_now();
            }
        }
        // Complete the owned fixture before assertions, even after a refusal.
        if let Err(e) = socket.write_all(b"X") {
            issues.push(format!("fixture release: {e}"));
        }
        let mut end = Vec::new();
        if let Err(e) = socket.read_to_end(&mut end) {
            issues.push(format!("fixture socket EOF: {e}"));
        }
        drop(socket);
        let status = child.wait().unwrap();
        fs::write(
            root.path().join("active-observations.json"),
            serde_json::to_vec_pretty(&observations).unwrap(),
        )
        .unwrap();
        assert!(
            status.success(),
            "{}",
            String::from_utf8_lossy(&fs::read(&stderr).unwrap())
        );
        assert!(issues.is_empty(), "{issues:?}");
        assert_eq!(observations.len(), 3);
        for (index, value) in observations.iter().enumerate() {
            let a = &value["agent"];
            let facts = &a["native_provider_facts"];
            assert_eq!(a["status"], "running");
            assert!(a.get("native_execution_result").is_none());
            assert_eq!(a["progress"]["tool_results"], index + 1);
            assert_eq!(facts["source"], "provider_declared_unverified");
            if index == 1 {
                assert!(a.get("usage").is_none() && a.get("reported_model").is_none());
                assert_eq!(facts["usage_state"], "invalid");
                assert_eq!(facts["model_state"], "invalid");
                assert_eq!(facts["last_valid_usage"]["input_tokens"], 7);
                assert!(
                    facts["first_usage_error"].is_string()
                        && facts["first_model_error"].is_string()
                );
            } else {
                assert_eq!(facts["usage_state"], "observed");
                assert_eq!(facts["model_state"], "observed");
                assert_eq!(a["usage"]["input_tokens"], if index == 0 { 7 } else { 0 });
                assert_eq!(
                    a["reported_model"]["model"],
                    if index == 0 {
                        "live-initial"
                    } else {
                        "live-final"
                    }
                );
            }
            assert!(!serde_json::to_string(value)
                .unwrap()
                .contains("raw-model-metadata-witness-unpublished"));
        }
        let final_result: serde_json::Value =
            serde_json::from_slice(&fs::read(&stdout).unwrap()).unwrap();
        assert_eq!(final_result["agent"]["status"], "completed");
        assert_eq!(final_result["agent"]["usage"]["input_tokens"], 0);
        assert_eq!(
            final_result["agent"]["reported_model"]["model"],
            "live-final"
        );
        assert_eq!(final_result["task_accepted"], false);
    }));
    if let Err(cause) = verdict {
        let retained = root.keep();
        eprintln!("active metadata fixture retained: {}", retained.display());
        std::panic::resume_unwind(cause);
    }
}
