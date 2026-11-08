#![cfg(target_os = "linux")]
//! EP-06 and engine-side attribution with real processes: `swarm-authority
//! serve` in front of a real engine (`SWARM_ENGINE_BINARY`) in authority mode.
//!
//! 1. A relayed write is recorded in the engine Root ledger
//!    (`.swarm/web-attribution.jsonl`) with the account and the same
//!    `assertion:<nonce>` reference as the daemon audit line.
//! 2. Restoring an older copy of the daemon account journal is refused by
//!    the engine (`web-assertion-rollback`) — witness first: the same
//!    restart without restore is accepted.
//! 3. `swarm-rust web-authority-witness reset` (engine stopped) accepts the
//!    restore; the reset is in the ledger.
//!
//! Protected roots: `$SWARM_PROTECTED_FIXTURE_AREA` or
//! `~/.local/state/swarm-protected-fixtures`; engine Root in the repository
//! fixture area. Never /tmp. Servers stopped with SIGINT.
use serde_json::Value;
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};

struct Kill(Child);
impl Drop for Kill {
    fn drop(&mut self) {
        let _ = Command::new("kill").args(["-INT", &self.0.id().to_string()]).status();
        let _ = self.0.wait();
    }
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

fn private_dir(p: &Path) {
    fs::create_dir_all(p).unwrap();
    fs::set_permissions(p, fs::Permissions::from_mode(0o700)).unwrap();
}

fn http(port: u16, method: &str, path: &str, headers: &[(&str, String)], body: &str) -> (u16, Vec<(String, String)>, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    let mut req = format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\nContent-Length: {}\r\n", body.len());
    for (k, v) in headers {
        req.push_str(&format!("{k}: {v}\r\n"));
    }
    req.push_str("\r\n");
    req.push_str(body);
    s.write_all(req.as_bytes()).unwrap();
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).unwrap();
    let raw = String::from_utf8_lossy(&raw).into_owned();
    let (head, body) = raw.split_once("\r\n\r\n").unwrap_or((&raw, ""));
    let mut lines = head.lines();
    let status = lines.next().unwrap().split(' ').nth(1).unwrap().parse().unwrap();
    let headers = lines.filter_map(|l| l.split_once(": ")).map(|(k, v)| (k.to_ascii_lowercase(), v.to_owned())).collect();
    (status, headers, body.to_owned())
}

fn wait_listening(port: u16) {
    for _ in 0..600 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("listener on {port} never came up");
}

fn lines(path: &Path) -> Vec<Value> {
    fs::read_to_string(path).unwrap_or_default().lines().map(|l| serde_json::from_str(l).unwrap()).collect()
}

#[test]
fn engine_attributes_writes_and_refuses_a_restored_daemon_journal() {
    let engine_bin = PathBuf::from(std::env::var("SWARM_ENGINE_BINARY").expect("SWARM_ENGINE_BINARY = swarm-rust with settings v52"));
    let daemon_bin = env!("CARGO_BIN_EXE_swarm-authority");
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let protected = std::env::var_os("SWARM_PROTECTED_FIXTURE_AREA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap()).join(".local/state/swarm-protected-fixtures"));
    private_dir(&protected);
    let p = protected.canonicalize().unwrap().join(format!("witness-{}-{nanos}", std::process::id()));
    private_dir(&p);
    let prep = Command::new(daemon_bin).args(["prepare", "--dir", p.to_str().unwrap(), "--controller-uid", "4242", "--admin", "ana", "--json"]).output().unwrap();
    assert!(prep.status.success(), "{}", String::from_utf8_lossy(&prep.stderr));
    let prepared: Value = serde_json::from_slice(&prep.stdout).unwrap();
    let (pin, decl) = (prepared["pin"].as_str().unwrap().to_owned(), prepared["declaration"].as_str().unwrap().to_owned());
    let conf = p.join("configuration");
    fs::write(conf.join("authority.json"), r#"{"schema_version":1,"session_absolute_seconds":600,"session_idle_seconds":300,"argon2":{"memory_kib":8,"passes":1,"parallelism":1},"relay_body_max_bytes":1048576}"#).unwrap();
    let base = ["--declaration", decl.as_str(), "--pin", pin.as_str()];
    let mut init = Command::new(daemon_bin).args(["init", "--admin", "ana", "--display", "Ana"]).args(base).arg("--json")
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    init.stdin.take().unwrap().write_all(b"ana pass\n").unwrap();
    let init = init.wait_with_output().unwrap();
    assert!(init.status.success(), "{}", String::from_utf8_lossy(&init.stderr));

    let area = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../.swarm-fixtures");
    fs::create_dir_all(&area).unwrap();
    let e = area.canonicalize().unwrap().join(format!("witness-engine-{}-{nanos}", std::process::id()));
    private_dir(&e);
    let run = |args: &[&str]| {
        let o = Command::new(&engine_bin).arg("--root").arg(&e).args(args).output().unwrap();
        assert!(o.status.success(), "{args:?}: {}", String::from_utf8_lossy(&o.stderr));
        serde_json::from_slice::<Value>(&o.stdout).unwrap()
    };
    run(&["init", "--json"]);
    let shown = run(&["engine-settings", "show", "--json"]);
    let mut values = shown["values"].clone();
    values["web_access_mode"] = "authority".into();
    values["web_authority_public_key"] = prepared["public_key"].clone();
    let mut change = serde_json::json!({"schema_version": shown["schema_version"], "event_id": "witness-e2e", "expected_revision": shown["revision"], "values": values, "reason": "Witness e2e fixture", "preview_token": ""});
    let input = e.join("change.json");
    fs::write(&input, serde_json::to_vec(&change).unwrap()).unwrap();
    change["preview_token"] = run(&["engine-settings", "preview", "--input", input.to_str().unwrap(), "--json"])["preview_token"].clone();
    fs::write(&input, serde_json::to_vec(&change).unwrap()).unwrap();
    run(&["engine-settings", "apply", "--input", input.to_str().unwrap(), "--json"]);

    let (engine_port, front_port) = (free_port(), free_port());
    let start_engine = || {
        let k = Kill(Command::new(&engine_bin).arg("--root").arg(&e).args(["web", &format!("127.0.0.1:{engine_port}")]).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap());
        wait_listening(engine_port);
        k
    };
    let start_daemon = || {
        let mut d = Command::new(daemon_bin).arg("serve").args(base)
            .args(["--listen", &format!("127.0.0.1:{front_port}"), "--engine", &format!("127.0.0.1:{engine_port}")])
            .stdout(Stdio::null()).stderr(Stdio::piped()).spawn().unwrap();
        let mut first = String::new();
        BufReader::new(d.stderr.take().unwrap()).read_line(&mut first).unwrap();
        assert!(first.contains("listening"), "{first}");
        let k = Kill(d);
        wait_listening(front_port);
        k
    };
    let origin = ("Origin", format!("http://127.0.0.1:{front_port}"));
    let login = || {
        let r = http(front_port, "POST", "/login", &[origin.clone(), ("Content-Type", "application/x-www-form-urlencoded".into())], "subject=ana&password=ana+pass&next=%2F");
        assert_eq!(r.0, 303, "{}", r.2);
        r.1.iter().find(|(k, _)| k == "set-cookie").map(|(_, v)| v.split(';').next().unwrap().to_owned()).unwrap()
    };
    let json = ("Content-Type", "application/json".to_owned());
    let mut engine = start_engine();
    let mut daemon = start_daemon();
    let cookie = login();

    // 1. Attribution: the engine ledger names the account and links the
    // daemon audit line through the same assertion reference.
    let written = http(front_port, "POST", "/api/v1/mission", &[("Cookie", cookie.clone()), origin.clone(), json.clone()], "{}");
    assert!(!written.2.contains("authority-") && !written.2.contains("web-assertion"), "{} {}", written.0, written.2);
    let ledger = lines(&e.join(".swarm/web-attribution.jsonl"));
    let intent = ledger.iter().find(|l| l["event"] == "intent" && l["path"] == "/api/v1/mission").expect("engine intent line");
    assert_eq!((intent["subject"].as_str(), intent["method"].as_str()), (Some("ana"), Some("POST")));
    let reference = intent["reference"].as_str().unwrap().to_owned();
    assert!(reference.starts_with("assertion:"));
    assert!(ledger.iter().any(|l| l["event"] == "outcome" && l["reference"] == reference.as_str() && l["status"] == written.0));
    let audit = lines(&conf.join("audit.jsonl"));
    assert!(audit.iter().any(|l| l["event"] == "intent" && l["reference"] == reference.as_str() && l["subject"] == "ana"), "same reference in the daemon audit");
    // Reads are not attributed; the witness exists after a verified assertion.
    let before = ledger.len();
    assert_eq!(http(front_port, "GET", "/api/v1/native-version", &[("Cookie", cookie.clone())], "").0, 200);
    assert_eq!(lines(&e.join(".swarm/web-attribution.jsonl")).len(), before);
    assert!(e.join(".swarm/authority-witness.json").exists());

    // 2. Backup of the account journal (daemon stopped), then a newer head.
    drop(daemon);
    let backup = p.join("journal-backup");
    private_dir(&backup);
    let journal: Vec<PathBuf> = fs::read_dir(&conf).unwrap().map(|x| x.unwrap().path()).filter(|x| x.file_name().unwrap().to_string_lossy().starts_with("accounts.sqlite")).collect();
    assert!(!journal.is_empty());
    for f in &journal {
        fs::copy(f, backup.join(f.file_name().unwrap())).unwrap();
    }
    daemon = start_daemon();
    // Witness: a plain daemon restart (no restore) is accepted.
    let cookie = login();
    assert_eq!(http(front_port, "GET", "/api/v1/native-version", &[("Cookie", cookie.clone())], "").0, 200, "restart without restore accepted");
    let created = http(front_port, "POST", "/api/v1/authority/accounts", &[("Cookie", cookie.clone()), origin.clone(), json.clone()],
        r#"{"subject":"bob","display_name":"Bob","roles":["viewer"],"integrity":1,"password":"bob pass"}"#);
    assert_eq!(created.0, 200, "{}", created.2);
    assert_eq!(http(front_port, "GET", "/api/v1/native-version", &[("Cookie", cookie.clone())], "").0, 200, "engine sees the newer journal head");
    // Issued strictly after the newer head was first seen (1 s clock).
    std::thread::sleep(std::time::Duration::from_millis(1100));
    drop(daemon);
    for f in fs::read_dir(&conf).unwrap().map(|x| x.unwrap().path()).filter(|x| x.file_name().unwrap().to_string_lossy().starts_with("accounts.sqlite")) {
        fs::remove_file(f).unwrap();
    }
    for f in &journal {
        fs::copy(backup.join(f.file_name().unwrap()), f).unwrap();
    }
    daemon = start_daemon();
    let cookie = login();
    let refused = http(front_port, "GET", "/api/v1/native-version", &[("Cookie", cookie.clone())], "");
    assert_eq!(refused.0, 403, "{}", refused.2);
    assert!(refused.2.contains("web-assertion-rollback"), "{}", refused.2);

    // 3. Deliberate restore accepted by the operator (engine stopped).
    drop(engine);
    let reset = run(&["web-authority-witness", "reset", "--json"]);
    assert_eq!(reset["existed"], true);
    assert!(lines(&e.join(".swarm/web-attribution.jsonl")).iter().any(|l| l["event"] == "witness-reset"));
    engine = start_engine();
    assert_eq!(http(front_port, "GET", "/api/v1/native-version", &[("Cookie", cookie.clone())], "").0, 200, "recovered after reset");
    drop(daemon);
    drop(engine);
    let _ = fs::remove_dir_all(&p);
    let _ = fs::remove_dir_all(&e);
}
