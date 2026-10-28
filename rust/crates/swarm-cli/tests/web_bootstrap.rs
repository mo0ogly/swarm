#![cfg(target_os = "linux")]
//! Actual dispatcher with owned output doubles; no serving app restarted.
use std::{
    fs,
    io::{self, Write},
    os::unix::fs::{MetadataExt, PermissionsExt},
};
use swarm_cli::run_detailed;
use swarm_engine::{RootStore, SystemEnvironment};
/// Output witness before each write/flush failure; original IO kind stays typed.
struct Output {
    bytes: Vec<u8>,
    fail_write: bool,
    fail_flush: bool,
}
impl Write for Output {
    fn write(&mut self, raw: &[u8]) -> io::Result<usize> {
        if self.fail_write {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "owned bound write",
            ));
        }
        self.bytes.extend_from_slice(raw);
        Ok(raw.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        if self.fail_flush {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "owned bound flush",
            ))
        } else {
            Ok(())
        }
    }
}
/// Public root initialization, no provider/mission reuse; failed roots are retained.
fn root() -> std::path::PathBuf {
    let p = std::env::temp_dir().join(swarm_engine::generate_identifier("cli-web97-").unwrap());
    fs::create_dir(&p).unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
    RootStore::initialize(&p, &mut SystemEnvironment)
        .unwrap()
        .close()
        .unwrap();
    p
}
/// CLI bootstrap output failure after bound session must close the socket, retain
/// cause/session state and never claim runtime readiness or configuration mutation.
#[test]
fn web_cli_bound_write_and_flush_failures_keep_private_session_and_safe_diagnostics() {
    for language in ["fr", "en"] {
        for flush in [false, true] {
            let p = root();
            let before = fs::read(p.join(".swarm/engine-settings.json")).unwrap();
            let mut out = Output {
                bytes: Vec::new(),
                fail_write: false,
                fail_flush: false,
            };
            let mut err = Vec::new();
            let witness = run_detailed(
                &[
                    "engine-settings".into(),
                    "show".into(),
                    "--root".into(),
                    p.to_string_lossy().into(),
                    "--json".into(),
                ],
                language,
                &mut out,
                &mut err,
            );
            assert_eq!(witness.code, 0);
            assert!(serde_json::from_slice::<serde_json::Value>(&out.bytes)
                .unwrap()
                .get("values")
                .is_some());
            out.bytes.clear();
            err.clear();
            out.fail_write = !flush;
            out.fail_flush = flush;
            let result = run_detailed(
                &[
                    "web".into(),
                    "--root".into(),
                    p.to_string_lossy().into(),
                    "--json".into(),
                ],
                language,
                &mut out,
                &mut err,
            );
            assert_eq!(result.code, 2);
            assert!(result.diagnostic_error.is_none());
            let failure: serde_json::Value = serde_json::from_slice(&err).unwrap();
            assert_eq!(failure["failure"]["code"], "web-bound-output");
            assert_eq!(failure["failure"]["retryable"], false);
            assert_eq!(failure["web_session_may_be_recorded"], true);
            assert_eq!(failure["configuration_may_be_recorded"], false);
            let sessions: Vec<_> = fs::read_dir(p.join(".swarm"))
                .unwrap()
                .map(|e| e.unwrap().path())
                .filter(|p| {
                    p.file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with("web-session-")
                })
                .collect();
            assert_eq!(sessions.len(), 1);
            let key = fs::read_to_string(&sessions[0]).unwrap();
            assert_eq!(key.len(), 32);
            assert_eq!(fs::metadata(&sessions[0]).unwrap().mode() & 0o777, 0o600);
            assert!(!String::from_utf8_lossy(&err).contains(&key));
            assert!(!String::from_utf8_lossy(&out.bytes).contains(&key));
            if flush {
                let bound: serde_json::Value = serde_json::from_slice(&out.bytes).unwrap();
                assert_eq!(bound["event"], "web-bound");
                assert_eq!(bound["runtime_ready"], false);
                let address = bound["address"].as_str().unwrap();
                assert!(std::net::TcpStream::connect(address).is_err());
            }
            assert_eq!(
                fs::read(p.join(".swarm/engine-settings.json")).unwrap(),
                before
            );
            let s = RootStore::open(&p, false).unwrap();
            assert!(s.list_works().unwrap().is_empty());
            s.close().unwrap();
            fs::remove_dir_all(p).unwrap();
        }
    }
}
/// Invalid command arguments and non-loopback override refuse before publishing a
/// session; saved settings remain exact and no network compatibility fallback occurs.
#[test]
fn web_cli_invalid_arguments_refuse_without_session_or_saved_policy_change() {
    for extra in [
        vec!["127.0.0.1:0", "extra"],
        vec!["--input", "unused"],
        vec!["--output", "unused"],
        vec!["--server", "http://127.0.0.1:1"],
        vec!["--task", "unused"],
        vec!["0.0.0.0:0"],
    ] {
        let p = root();
        let before = fs::read(p.join(".swarm/engine-settings.json")).unwrap();
        let mut args = vec![
            "web".into(),
            "--root".into(),
            p.to_string_lossy().into(),
            "--json".into(),
        ];
        args.extend(extra.into_iter().map(String::from));
        let mut out = Vec::new();
        let mut err = Vec::new();
        let result = run_detailed(&args, "en", &mut out, &mut err);
        assert_eq!(result.code, 2);
        assert!(!fs::read_dir(p.join(".swarm")).unwrap().any(|e| e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("web-session-")));
        assert_eq!(
            fs::read(p.join(".swarm/engine-settings.json")).unwrap(),
            before
        );
        fs::remove_dir_all(p).unwrap();
    }
}
