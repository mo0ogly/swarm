#![cfg(target_os = "linux")]
use std::{fs, path::PathBuf};
use swarm_engine::{
    run_validation_control, EngineSettingsDocument, EngineSettingsValues, SettingCap, SettingCount,
    ValidationControl, ValidationRunError,
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(swarm_engine::generate_identifier("swarm-control-").unwrap());
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn settings() -> EngineSettingsDocument {
    EngineSettingsDocument::initial(
        EngineSettingsValues::installation_defaults().unwrap(),
        "fixture/uid:1".into(),
        "2026-10-18T00:00:00Z".into(),
    )
    .unwrap()
}
fn control(script: &str) -> ValidationControl {
    ValidationControl {
        id: "control".into(),
        command: Some(vec!["python3".into(), "-c".into(), script.into()]),
        criteria: Some(vec![1]),
        justification: "Reproducible fixture check".into(),
        timeout_seconds: 60,
        review_output: true,
        ..Default::default()
    }
}
fn run(
    f: &Fixture,
    c: &ValidationControl,
    d: &EngineSettingsDocument,
) -> Result<swarm_engine::ValidationRun, ValidationRunError> {
    run_validation_control(&f.0, c, d)
}
#[test]
fn actual_process_retains_configured_bytes_counts_streams_and_reports_exit_without_shell() {
    let f = Fixture::new();
    let d = settings();
    let c=control("import sys; sys.stdout.buffer.write(b'a'*131072); sys.stdout.flush(); sys.stderr.buffer.write(b'END');sys.stderr.flush()");
    let r = run(&f, &c, &d).unwrap();
    assert!(r.result.executed && r.result.passed);
    assert_eq!(r.result.exit_code, 0);
    assert_eq!(r.output.total(), 131075);
    assert_eq!(r.output.retained(), vec![b'a'; 65536]);
    assert!(r.output.truncated());
    assert_eq!(r.result.output_bytes, 131075);
    assert!(r.result.review_output_truncated);
    assert_eq!(r.result.review_output.len(), 65536);
    assert!(r.result.wall_duration_ms.is_some());
    assert!(r.result.cpu_duration_ms.is_some());
    assert_eq!(r.settings_revision, d.revision);
    assert_eq!(
        r.result.output_sha256,
        format!("{:x}", sha2::Sha256::digest(r.output.retained()))
    );
    let mut failed = control("import sys; print('ordinary failure');sys.exit(7)");
    failed.environment_exit_codes = Some(vec![7]);
    let r = run(&f, &failed, &d).unwrap();
    assert!(r.result.executed && !r.result.passed && r.result.environment_failure);
    assert_eq!(r.result.exit_code, 7);
    assert!(!r.timed_out);
    assert!(!f.0.join("shell-side-effect").exists());
    let c = control("import sys; sys.stdout.write(sys.argv[1])");
    let mut c = c;
    c.command
        .as_mut()
        .unwrap()
        .push("; touch shell-side-effect".into());
    assert_eq!(
        run(&f, &c, &d).unwrap().output.retained(),
        b"; touch shell-side-effect"
    );
    assert!(!f.0.join("shell-side-effect").exists());
}
use sha2::Digest;
#[test]
fn zero_unlimited_and_invalid_utf8_review_use_the_configured_consumer() {
    let f = Fixture::new();
    for (cap, expected) in [
        (Some(0), b"".as_slice()),
        (Some(4), b"a\xF0\x9F\x92".as_slice()),
        (None, b"a\xF0\x9F\x92\xA1\xFFz".as_slice()),
    ] {
        let mut values = EngineSettingsValues::installation_defaults().unwrap();
        values.validation_max_output_bytes = Some(SettingCap(cap.map(SettingCount)));
        let d = EngineSettingsDocument::initial(
            values,
            "operator".into(),
            "2026-10-18T00:00:00Z".into(),
        )
        .unwrap();
        let r=run(&f,&control("import sys;sys.stdout.buffer.write(bytes([97,240,159,146,161,255,122]));sys.stdout.flush()"),&d).unwrap();
        assert!(r.result.passed);
        assert_eq!(r.output.retained(), expected);
        assert_eq!(r.output.total(), 7);
        assert_eq!(
            r.result.review_output,
            if cap == Some(0) {
                ""
            } else if cap == Some(4) {
                "a"
            } else {
                "a💡z"
            }
        );
        assert!(r.result.review_output_truncated); // invalid byte was dropped even without retention truncation
    }
}
// Obtain the descendant's stable handle while it is demonstrably alive,
// release its parent, then observe kernel termination. No sleep/instant /proc
// snapshot is used as a substitute for the lifecycle event.
fn observed_descendant_run(
    f: &Fixture,
    d: &EngineSettingsDocument,
    deadline: bool,
) -> swarm_engine::ValidationRun {
    use rustix::{
        event::{poll, PollFd, PollFlags, Timespec},
        process::{pidfd_open, pidfd_send_signal, Pid, PidfdFlags, Signal},
    };
    use std::{
        io::{BufRead, Write},
        os::unix::net::{UnixListener, UnixStream},
    };
    let listener = UnixListener::bind(f.0.join("observer.sock")).unwrap();
    let (done_reader, done_writer) = UnixStream::pair().unwrap();
    let mut c=control("import subprocess,socket,signal; p=subprocess.Popen(['python3','-c','import signal;signal.pause()']); s=socket.socket(socket.AF_UNIX);s.connect('observer.sock');s.sendall((str(p.pid)+'\\n').encode());s.recv(1);s.close()" );
    if deadline {
        c.timeout_seconds = 1;
        c.command.as_mut().unwrap()[2].push_str(";signal.pause()")
    }
    std::thread::scope(|scope| {
        let task = scope.spawn(|| {
            let result = run(f, &c, d);
            drop(done_writer);
            result
        });
        let mut pending = [
            PollFd::new(&listener, PollFlags::IN),
            PollFd::new(&done_reader, PollFlags::IN),
        ];
        poll(&mut pending, None).unwrap();
        assert!(
            pending[0].revents().contains(PollFlags::IN),
            "control finished before lifecycle observation became ready"
        );
        let (mut peer, _) = listener.accept().unwrap();
        let mut line = String::new();
        std::io::BufReader::new(&mut peer)
            .read_line(&mut line)
            .unwrap();
        let id = line.trim().parse::<u32>().unwrap();
        let pid = Pid::from_raw(i32::try_from(id).unwrap()).unwrap();
        let fd = pidfd_open(pid, PidfdFlags::empty()).unwrap();
        let mut alive = [PollFd::new(&fd, PollFlags::IN)];
        assert_eq!(
            poll(
                &mut alive,
                Some(&Timespec {
                    tv_sec: 0,
                    tv_nsec: 0
                })
            )
            .unwrap(),
            0,
            "valid descendant must be alive before parent release"
        );
        peer.write_all(b"continue").unwrap();
        drop(peer);
        let result = task.join().unwrap();
        if result.is_err() {
            pidfd_send_signal(&fd, Signal::KILL).unwrap();
        }
        let mut exited = [PollFd::new(&fd, PollFlags::IN)];
        poll(&mut exited, None).unwrap();
        assert!(exited[0].revents().contains(PollFlags::IN));
        nix::unistd::close(fd).unwrap();
        result.unwrap()
    })
}
#[test]
fn declared_deadline_kills_the_owned_group_and_does_not_leak_descendants() {
    let f = Fixture::new();
    let d = settings();
    let r = observed_descendant_run(&f, &d, true);
    assert!(r.timed_out && r.result.executed && !r.result.passed && r.group_signal_delivered);
    assert_eq!(r.result.exit_code, -1);
    assert!(**r.result.wall_duration_ms.as_ref().unwrap() >= 1000);
    assert!(
        run(&f, &control("print('recovered')"), &d)
            .unwrap()
            .result
            .passed
    );
}
#[test]
fn path_policy_and_missing_program_refuse_then_recover_without_private_error_echo() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let outside = Fixture::new();
    let d = settings();
    fs::write(f.0.join("ordinary"), b"x").unwrap();
    symlink(&outside.0, f.0.join("escape")).unwrap();
    let c = control("from pathlib import Path;p=Path('marker');p.write_text('executed')");
    for dir in ["escape", "missing", "ordinary", "../"] {
        assert!(run(&f, &c, &d).unwrap().result.passed);
        let before = fs::read(f.0.join("marker")).unwrap();
        let mut bad = c.clone();
        bad.dir = dir.into();
        let error = run(&f, &bad, &d).unwrap_err();
        assert_eq!(error.operation(), "directory");
        assert!(!error.to_string().contains(f.0.to_str().unwrap()));
        assert_eq!(fs::read(f.0.join("marker")).unwrap(), before);
        assert!(!outside.0.join("marker").exists());
        assert!(run(&f, &c, &d).unwrap().result.passed);
    }
    let mut values = d.values.clone();
    values
        .validation_allowed_programs
        .insert("swarm-private-missing-fixture-program".into());
    let d =
        EngineSettingsDocument::initial(values, "operator".into(), "2026-10-18T00:00:00Z".into())
            .unwrap();
    let mut missing = c.clone();
    missing.command = Some(vec![
        "swarm-private-missing-fixture-program".into(),
        "PRIVATE_ARG".into(),
    ]);
    let e = run(&f, &missing, &d).unwrap_err();
    assert_eq!(e.operation(), "spawn");
    assert!(std::error::Error::source(&e).is_some());
    assert!(!format!("{e:?}").contains("PRIVATE_ARG"));
    assert!(run(&f, &c, &d).unwrap().result.passed);
    let mut unconfigured = d.clone();
    unconfigured.values.validation_max_output_bytes = None;
    assert!(run(&f, &c, &unconfigured).is_err());
    let mut invalid = c.clone();
    invalid.timeout_seconds = i64::MAX;
    let mut values = d.values.clone();
    values.validation_max_timeout_seconds = None;
    values.validation_max_total_timeout_seconds = None;
    let d =
        EngineSettingsDocument::initial(values, "operator".into(), "2026-10-18T00:00:00Z".into())
            .unwrap();
    assert!(run(&f, &invalid, &d).is_err());
}

#[test]
fn exited_leader_signals_group_and_descendant_termination_is_observed_separately() {
    let f = Fixture::new();
    let d = settings();
    let r = observed_descendant_run(&f, &d, false);
    assert!(
        r.result.passed && !r.timed_out && !r.output_drain_timed_out && r.group_signal_delivered
    );
}

#[test]
fn measured_cpu_is_owned_by_each_child_with_concurrency_and_prior_waited_children() {
    let f = Fixture::new();
    let d = settings();
    let mut heavy=control("import resource,time,json; start=time.process_time_ns();exec('while time.process_time_ns()-start<200000000: pass');r=resource.getrusage(resource.RUSAGE_SELF);print(json.dumps({'self_us':int((r.ru_utime+r.ru_stime)*1000000)}))");
    heavy.timeout_seconds = 60;
    let light=control("import resource,json;r=resource.getrusage(resource.RUSAGE_SELF);print(json.dumps({'self_us':int((r.ru_utime+r.ru_stime)*1000000)}))");
    let (heavy, first_light) = std::thread::scope(|s| {
        let a = s.spawn(|| run(&f, &heavy, &d));
        let b = s.spawn(|| run(&f, &light, &d));
        (a.join().unwrap().unwrap(), b.join().unwrap().unwrap())
    });
    let after_heavy = run(&f, &light, &d).unwrap();
    for r in [&heavy, &first_light, &after_heavy] {
        assert!(r.result.passed);
        let own: serde_json::Value = serde_json::from_slice(r.output.retained()).unwrap();
        let cpu = **r
            .result
            .cpu_duration_ms
            .as_ref()
            .expect("per-child CPU must be measured");
        assert!(cpu >= own["self_us"].as_i64().unwrap() / 1000);
        // These fixtures have one CPU thread and no children: their own CPU
        // cannot exceed elapsed wall time. No arbitrary performance bound.
        assert!(cpu <= **r.result.wall_duration_ms.as_ref().unwrap());
    }
    assert!(
        heavy.result.cpu_duration_ms > after_heavy.result.cpu_duration_ms,
        "prior children must not inflate the light child's CPU"
    );
}
