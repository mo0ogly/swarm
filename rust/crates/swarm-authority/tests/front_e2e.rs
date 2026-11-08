#![cfg(target_os = "linux")]
//! End-to-end B5.3 with real processes: `swarm-authority serve` in front of a
//! real engine (`SWARM_ENGINE_BINARY`, a `swarm-rust` with settings v52) in
//! `authority` mode. Sign-in by account/password, relayed request accepted by
//! the engine, direct engine access refused, logout ends the session.
//!
//! Protected roots (declaration, configuration) need non-group-writable
//! ancestors: `$SWARM_PROTECTED_FIXTURE_AREA` or
//! `~/.local/state/swarm-protected-fixtures`. Engine Root in the repository
//! fixture area. Never /tmp. Servers are stopped with SIGINT.
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

/// Raw HTTP/1.1 request; returns (status, headers lowercased, body).
fn http(port: u16, method: &str, path: &str, headers: &[(&str, String)], body: &str) -> (u16, Vec<(String, String)>, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    let mut req = format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\nContent-Length: {}\r\n", body.len());
    for (k, v) in headers {
        req.push_str(&format!("{k}: {v}\r\n"));
    }
    req.push_str("\r\n");
    req.push_str(body);
    s.write_all(req.as_bytes()).unwrap();
    let mut raw = String::new();
    s.read_to_string(&mut raw).unwrap();
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

#[test]
fn daemon_front_signs_in_relays_and_engine_refuses_direct_access() {
    let engine_bin = PathBuf::from(std::env::var("SWARM_ENGINE_BINARY").expect("SWARM_ENGINE_BINARY = swarm-rust with settings v52"));
    let daemon_bin = env!("CARGO_BIN_EXE_swarm-authority");
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let protected = std::env::var_os("SWARM_PROTECTED_FIXTURE_AREA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap()).join(".local/state/swarm-protected-fixtures"));
    private_dir(&protected);
    let p = protected.canonicalize().unwrap().join(format!("front-{}-{nanos}", std::process::id()));
    private_dir(&p);
    // Real install path: `prepare` (as the service account) creates the
    // private dirs, the signing seed and the declaration pinning key + admin.
    let prep = Command::new(daemon_bin)
        .args(["prepare", "--dir", p.to_str().unwrap(), "--controller-uid", "4242", "--admin", "ana", "--json"])
        .output().unwrap();
    assert!(prep.status.success(), "{}", String::from_utf8_lossy(&prep.stderr));
    let prepared: serde_json::Value = serde_json::from_slice(&prep.stdout).unwrap();
    let pin = prepared["pin"].as_str().unwrap().to_owned();
    let decl = PathBuf::from(prepared["declaration"].as_str().unwrap());
    let again = Command::new(daemon_bin)
        .args(["prepare", "--dir", p.to_str().unwrap(), "--controller-uid", "4242", "--admin", "ana", "--json"])
        .output().unwrap();
    assert_eq!(serde_json::from_slice::<serde_json::Value>(&again.stderr).unwrap()["failure"]["code"], "authority-declaration-exists");
    // Fast Argon2 for the fixture: operator config file, not a code default.
    fs::write(
        p.join("configuration/authority.json"),
        r#"{"schema_version":1,"session_absolute_seconds":600,"session_idle_seconds":300,"argon2":{"memory_kib":8,"passes":1,"parallelism":1},"relay_body_max_bytes":1048576}"#,
    )
    .unwrap();
    let base = ["--declaration", decl.to_str().unwrap(), "--pin", &pin];
    // `init` creates the first user (the pinned admin), password on stdin.
    let with_stdin = |args: &[&str], input: &[u8]| {
        let mut c = Command::new(daemon_bin).args(args).args(base).arg("--json")
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        c.stdin.take().unwrap().write_all(input).unwrap();
        c.wait_with_output().unwrap()
    };
    let code = |o: &std::process::Output| serde_json::from_slice::<serde_json::Value>(&o.stderr).unwrap()["failure"]["code"].as_str().unwrap().to_owned();
    assert_eq!(code(&with_stdin(&["init", "--admin", "eve", "--display", "Eve"], b"pw\n")), "authority-admin-not-pinned");
    assert_eq!(code(&with_stdin(&["init", "--admin", "ana", "--display", "Ana"], b"\n")), "authority-password-empty");
    let init = with_stdin(&["init", "--admin", "ana", "--display", "Ana"], b"correct horse\n");
    assert!(init.status.success(), "{}", String::from_utf8_lossy(&init.stderr));
    let initialized: serde_json::Value = serde_json::from_slice(&init.stdout).unwrap();
    assert_eq!(initialized["admin"], "ana");
    let public_key = initialized["public_key"].as_str().unwrap().to_owned();
    assert_eq!(public_key, prepared["public_key"].as_str().unwrap());
    assert_eq!(code(&with_stdin(&["init", "--admin", "ana", "--display", "Ana"], b"other\n")), "authority-already-initialized");
    // A replaced seed cannot sign for this installation.
    let seed = p.join("configuration/authority-signing-seed");
    let original = fs::read(&seed).unwrap();
    fs::write(&seed, [9u8; 32]).unwrap();
    assert_eq!(code(&with_stdin(&["add-account", "mal", "--roles", "admin", "--integrity", "3", "--display", "Mal"], b"x\n")), "authority-key-not-pinned");
    fs::write(&seed, &original).unwrap();
    let added = with_stdin(&["add-account", "bob", "--roles", "viewer", "--integrity", "1", "--display", "Bob"], b"bob secret\n");
    assert!(added.status.success(), "{}", String::from_utf8_lossy(&added.stderr));

    // Engine Root in the repository fixture area, switched to authority mode.
    let area = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../.swarm-fixtures");
    fs::create_dir_all(&area).unwrap();
    let e = area.canonicalize().unwrap().join(format!("front-engine-{}-{nanos}", std::process::id()));
    private_dir(&e);
    let run = |args: &[&str], input: Option<&[u8]>| {
        let mut c = Command::new(&engine_bin).arg("--root").arg(&e).args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        if let Some(i) = input { c.stdin.take().unwrap().write_all(i).unwrap(); }
        let o = c.wait_with_output().unwrap();
        assert!(o.status.success(), "{args:?}: {}", String::from_utf8_lossy(&o.stderr));
        serde_json::from_slice::<serde_json::Value>(&o.stdout).unwrap()
    };
    run(&["init", "--json"], None);
    let shown = run(&["engine-settings", "show", "--json"], None);
    let mut values = shown["values"].clone();
    values["web_access_mode"] = "authority".into();
    values["web_authority_public_key"] = public_key.into();
    let mut change = serde_json::json!({"schema_version": shown["schema_version"], "event_id": "front-e2e", "expected_revision": shown["revision"], "values": values, "reason": "Authority front e2e fixture", "preview_token": ""});
    let input = e.join("change.json");
    fs::write(&input, serde_json::to_vec(&change).unwrap()).unwrap();
    let preview = run(&["engine-settings", "preview", "--input", input.to_str().unwrap(), "--json"], None);
    change["preview_token"] = preview["preview_token"].clone();
    fs::write(&input, serde_json::to_vec(&change).unwrap()).unwrap();
    run(&["engine-settings", "apply", "--input", input.to_str().unwrap(), "--json"], None);

    let (engine_port, front_port) = (free_port(), free_port());
    let _engine = Kill(Command::new(&engine_bin).arg("--root").arg(&e).args(["web", &format!("127.0.0.1:{engine_port}")]).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap());
    wait_listening(engine_port);
    let mut daemon = Command::new(daemon_bin)
        .arg("serve").args(base)
        .args(["--listen", &format!("127.0.0.1:{front_port}"), "--engine", &format!("127.0.0.1:{engine_port}")])
        .stdout(Stdio::null()).stderr(Stdio::piped()).spawn().unwrap();
    let mut first = String::new();
    BufReader::new(daemon.stderr.take().unwrap()).read_line(&mut first).unwrap();
    assert!(first.contains("listening"), "{first}");
    let _daemon = Kill(daemon);
    wait_listening(front_port);

    let origin = ("Origin", format!("http://127.0.0.1:{front_port}"));
    // No session: API 401, browser navigation redirected to /login.
    assert_eq!(http(front_port, "GET", "/api/v1/native-version", &[], "").0, 401);
    let nav = http(front_port, "GET", "/native-runtime.html", &[("Accept", "text/html".into())], "");
    assert_eq!(nav.0, 303);
    assert!(nav.1.iter().any(|(k, v)| k == "location" && v.starts_with("/login?next=")));
    // Login page, wrong password, missing Origin.
    let page = http(front_port, "GET", "/login?lang=en", &[], "");
    assert!(page.2.contains("Sign in to Swarm"));
    // Regression: `no-referrer` made browsers send `Origin: null` on the form POST.
    assert!(page.1.iter().any(|(k, v)| k == "referrer-policy" && v == "same-origin"), "{:?}", page.1);
    let form = "application/x-www-form-urlencoded".to_owned();
    assert_eq!(http(front_port, "POST", "/login", &[origin.clone(), ("Content-Type", form.clone())], "subject=ana&password=wrong&next=%2F").0, 401);
    assert_eq!(http(front_port, "POST", "/login", &[("Content-Type", form.clone())], "subject=ana&password=correct+horse&next=%2F").0, 403);
    // Successful sign-in.
    let login = http(front_port, "POST", "/login", &[origin.clone(), ("Content-Type", form)], "subject=ana&password=correct+horse&next=%2Fnative-runtime.html");
    assert_eq!(login.0, 303);
    let cookie = login.1.iter().find(|(k, _)| k == "set-cookie").map(|(_, v)| v.split(';').next().unwrap().to_owned()).unwrap();
    assert!(login.1.iter().any(|(k, v)| k == "set-cookie" && v.contains("HttpOnly") && v.contains("SameSite=Strict")));
    // Relayed protected request accepted by the engine (assertion verified there).
    let ok = http(front_port, "GET", "/api/v1/native-version", &[("Cookie", cookie.clone())], "");
    assert_eq!(ok.0, 200, "{}", ok.2);
    assert!(ok.2.contains("binary"));
    // Regression: a browser loads many resources at once; concurrent session
    // resolutions used to refuse with journal `sqlite` busy (seen as 401).
    let parallel: Vec<u16> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..20).map(|_| scope.spawn(|| http(front_port, "GET", "/api/v1/native-version", &[("Cookie", cookie.clone())], "").0)).collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    assert!(parallel.iter().all(|s| *s == 200), "{parallel:?}");
    // Regression: `/` behind the daemon is the cockpit, not the engine's
    // owner-key sign-in page (root_login is owner-key mode only).
    let home = http(front_port, "GET", "/", &[("Cookie", cookie.clone()), ("Accept", "text/html".into())], "");
    assert_eq!(home.0, 200);
    assert!(!home.2.contains("access-key") && home.2.contains("Cockpit"), "{}", &home.2[..home.2.len().min(300)]);
    // The engine itself refuses direct access without an assertion.
    let direct = http(engine_port, "GET", "/api/v1/native-version", &[], "");
    assert_eq!(direct.0, 403);
    // A further user created with add-account can sign in too.
    let bob = http(front_port, "POST", "/login", &[origin.clone(), ("Content-Type", "application/x-www-form-urlencoded".into())], "subject=bob&password=bob+secret&next=%2F");
    assert_eq!(bob.0, 303);
    // B6: the viewer reads through the relay but a write is refused by the
    // daemon (Biba floor: integrity 1 < 2) before reaching the engine, while
    // the admin's same request passes the policy (engine answers on its own).
    let bob_cookie = bob.1.iter().find(|(k, _)| k == "set-cookie").map(|(_, v)| v.split(';').next().unwrap().to_owned()).unwrap();
    assert_eq!(http(front_port, "GET", "/api/v1/native-version", &[("Cookie", bob_cookie.clone())], "").0, 200);
    let json = ("Content-Type", "application/json".to_owned());
    let denied = http(front_port, "POST", "/api/v1/mission", &[("Cookie", bob_cookie), origin.clone(), json.clone()], "{}");
    assert_eq!(denied.0, 403, "{}", denied.2);
    let denied: serde_json::Value = serde_json::from_str(&denied.2).unwrap();
    assert_eq!((denied["error"].as_str(), denied["action"].as_str(), denied["reason"].as_str()), (Some("authority-policy-denied"), Some("write"), Some("policy-integrity-floor")));
    let admin_write = http(front_port, "POST", "/api/v1/mission", &[("Cookie", cookie.clone()), origin.clone(), json.clone()], "{}");
    assert!(!admin_write.2.contains("authority-policy-denied"), "{}", admin_write.2);
    // Decision preview (no mutation) through the CLI, saved/default policy.
    let check = |args: &[&str]| {
        let o = Command::new(daemon_bin).arg("policy-check").args(base).args(args).arg("--json").output().unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        serde_json::from_slice::<serde_json::Value>(&o.stdout).unwrap()["decision"].clone()
    };
    assert_eq!(check(&["--method", "POST", "--path", "/api/v1/quotas/apply", "--integrity", "3", "--roles", "admin"])["allowed"], true);
    let viewer = check(&["--method", "POST", "--path", "/api/v1/quotas/apply", "--integrity", "3", "--roles", "viewer"]);
    assert_eq!((viewer["allowed"].as_bool(), viewer["reason"].as_str()), (Some(false), Some("policy-no-grant")));
    // B9 personal API keys: created from the session (secret once), used as
    // `Authorization: Bearer` without cookie or Origin, capped by scope.
    let mk = |scope: &str, secs: i64| {
        let r = http(front_port, "POST", "/api/v1/authority/me/keys", &[("Cookie", cookie.clone()), origin.clone(), json.clone()],
            &serde_json::json!({"name": format!("script {scope}"), "scope": scope, "expires_in_seconds": secs}).to_string());
        assert_eq!(r.0, 200, "{}", r.2);
        let v: serde_json::Value = serde_json::from_str(&r.2).unwrap();
        (v["secret"].as_str().unwrap().to_owned(), v["key"]["id"].as_str().unwrap().to_owned())
    };
    let (read_key, read_id) = mk("read", 600);
    let bearer = |k: &str| ("Authorization", format!("Bearer {k}"));
    assert_eq!(http(front_port, "GET", "/api/v1/native-version", &[bearer(&read_key)], "").0, 200, "read key reads through the relay");
    let capped: serde_json::Value = serde_json::from_str(&http(front_port, "POST", "/api/v1/mission", &[bearer(&read_key), json.clone()], "{}").2).unwrap();
    assert_eq!((capped["error"].as_str(), capped["reason"].as_str()), (Some("authority-policy-denied"), Some("key-scope")));
    let (write_key, write_id) = mk("write", 600);
    let written = http(front_port, "POST", "/api/v1/mission", &[bearer(&write_key), json.clone()], "{}");
    assert!(!written.2.contains("authority-policy-denied") && written.0 != 401, "{} {}", written.0, written.2);
    let scoped = http(front_port, "GET", "/api/v1/authority/me", &[bearer(&write_key)], "");
    assert_eq!((scoped.0, scoped.2.as_str()), (403, r#"{"error":"authority-key-scope"}"#), "keys never reach daemon administration");
    let listed = http(front_port, "GET", "/api/v1/authority/me/keys", &[("Cookie", cookie.clone())], "");
    assert!(listed.2.contains(&read_id) && !listed.2.contains(&read_key[18..]), "listed without secret");
    // Malformed, tampered, revoked and expired keys: 401, no redirect.
    assert_eq!(http(front_port, "GET", "/api/v1/native-version", &[bearer("sak1-zz.nope")], "").0, 401);
    let tampered = format!("{}x", &read_key[..read_key.len() - 1]);
    assert_eq!(http(front_port, "GET", "/api/v1/native-version", &[bearer(&tampered)], "").0, 401);
    let revoke = http(front_port, "POST", &format!("/api/v1/authority/me/keys/{read_id}/revoke"), &[("Cookie", cookie.clone()), origin.clone()], "");
    assert_eq!(revoke.0, 200, "{}", revoke.2);
    assert_eq!(http(front_port, "GET", "/api/v1/native-version", &[bearer(&read_key)], "").0, 401, "revoked");
    assert_eq!(http(front_port, "POST", &format!("/api/v1/authority/me/keys/{read_id}/revoke"), &[("Cookie", cookie.clone()), origin.clone()], "").0, 404);
    let (short, _) = mk("read", 1);
    std::thread::sleep(std::time::Duration::from_millis(2100));
    assert_eq!(http(front_port, "GET", "/api/v1/native-version", &[bearer(&short)], "").0, 401, "expired");
    let zero = http(front_port, "POST", "/api/v1/authority/me/keys", &[("Cookie", cookie.clone()), origin.clone(), json.clone()], r#"{"name":"x","scope":"read","expires_in_seconds":0}"#);
    assert_eq!(zero.0, 400);
    // Attribution: the audit names the key used for the write.
    let audit = http(front_port, "GET", "/api/v1/authority/audit", &[("Cookie", cookie.clone())], "").2;
    assert!(audit.contains(&format!("key:{write_id}")), "{audit}");
    // Cockpit sign-out path ends the daemon session.
    let out = http(front_port, "POST", "/api/v1/logout", &[("Cookie", cookie.clone()), origin.clone()], "");
    assert_eq!((out.0, out.2.as_str()), (200, r#"{"redirect":"/login"}"#));
    assert_eq!(http(front_port, "GET", "/api/v1/native-version", &[("Cookie", cookie)], "").0, 401);
    drop(_daemon);
    fs::remove_dir_all(&p).unwrap();
}
