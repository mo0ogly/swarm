#![cfg(target_os = "linux")]
//! B8 end to end with the real binary: the daemon-served administration API
//! (`/api/v1/authority/*`) — self-service `me`, accounts, sessions, policy
//! and configuration — including refusals (policy denial of admin reads,
//! self-lockout, stale revision, invalid documents) and their recovery.
//! No engine is needed: these routes are never relayed (the `--engine`
//! port is unused and closed).
//!
//! Protected roots need non-group-writable ancestors:
//! `$SWARM_PROTECTED_FIXTURE_AREA` or `~/.local/state/swarm-protected-fixtures`.
//! Never /tmp. The daemon is stopped with SIGINT.
use serde_json::{json, Value};
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

struct Client {
    port: u16,
}

struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}
impl Reply {
    fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or_else(|_| panic!("not JSON ({}): {}", self.status, self.body))
    }
    fn cookie(&self) -> String {
        self.headers.iter().find(|(k, _)| k == "set-cookie").map(|(_, v)| v.split(';').next().unwrap().to_owned()).expect("set-cookie")
    }
}

impl Client {
    fn send(&self, method: &str, path: &str, cookie: Option<&str>, body: Option<Value>, origin: bool) -> Reply {
        let mut s = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        let body = body.map(|b| b.to_string()).unwrap_or_default();
        let mut req = format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n", self.port, body.len());
        if origin {
            req.push_str(&format!("Origin: http://127.0.0.1:{}\r\n", self.port));
        }
        if let Some(c) = cookie {
            req.push_str(&format!("Cookie: {c}\r\n"));
        }
        req.push_str("\r\n");
        req.push_str(&body);
        s.write_all(req.as_bytes()).unwrap();
        let mut bytes = Vec::new();
        s.read_to_end(&mut bytes).unwrap();
        let raw = String::from_utf8_lossy(&bytes); // binary bodies (logo) stay readable
        let (head, body) = raw.split_once("\r\n\r\n").unwrap_or((&raw, ""));
        let mut lines = head.lines();
        let status = lines.next().unwrap().split(' ').nth(1).unwrap().parse().unwrap();
        let headers = lines.filter_map(|l| l.split_once(": ")).map(|(k, v)| (k.to_ascii_lowercase(), v.to_owned())).collect();
        Reply { status, headers, body: body.to_owned() }
    }
    fn get(&self, path: &str, cookie: &str) -> Reply {
        self.send("GET", path, Some(cookie), None, false)
    }
    fn post(&self, path: &str, cookie: &str, body: Value) -> Reply {
        self.send("POST", path, Some(cookie), Some(body), true)
    }
    fn login(&self, subject: &str, password: &str) -> Reply {
        let mut s = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        let form = format!("subject={subject}&password={}&next=%2F", password.replace(' ', "+"));
        let req = format!(
            "POST /login HTTP/1.1\r\nHost: 127.0.0.1:{p}\r\nOrigin: http://127.0.0.1:{p}\r\nConnection: close\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{form}",
            form.len(),
            p = self.port
        );
        s.write_all(req.as_bytes()).unwrap();
        let mut raw = String::new();
        s.read_to_string(&mut raw).unwrap();
        let (head, body) = raw.split_once("\r\n\r\n").unwrap_or((&raw, ""));
        let mut lines = head.lines();
        let status = lines.next().unwrap().split(' ').nth(1).unwrap().parse().unwrap();
        let headers = lines.filter_map(|l| l.split_once(": ")).map(|(k, v)| (k.to_ascii_lowercase(), v.to_owned())).collect();
        Reply { status, headers, body: body.to_owned() }
    }
}

fn error(r: &Reply) -> String {
    r.json()["error"].as_str().unwrap_or("").to_owned()
}

#[test]
fn administration_api_manages_accounts_sessions_policy_and_config() {
    let bin = env!("CARGO_BIN_EXE_swarm-authority");
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let protected = std::env::var_os("SWARM_PROTECTED_FIXTURE_AREA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap()).join(".local/state/swarm-protected-fixtures"));
    private_dir(&protected);
    let p = protected.canonicalize().unwrap().join(format!("admin-{}-{nanos}", std::process::id()));
    private_dir(&p);
    let prep = Command::new(bin).args(["prepare", "--dir", p.to_str().unwrap(), "--controller-uid", "4242", "--admin", "ana", "--json"]).output().unwrap();
    assert!(prep.status.success(), "{}", String::from_utf8_lossy(&prep.stderr));
    let prepared: Value = serde_json::from_slice(&prep.stdout).unwrap();
    let (pin, decl) = (prepared["pin"].as_str().unwrap().to_owned(), prepared["declaration"].as_str().unwrap().to_owned());
    let conf = p.join("configuration");
    // Fast Argon2 for the fixture: operator config file, not a code default.
    fs::write(
        conf.join("authority.json"),
        r#"{"schema_version":1,"session_absolute_seconds":600,"session_idle_seconds":300,"argon2":{"memory_kib":8,"passes":1,"parallelism":1},"relay_body_max_bytes":1048576}"#,
    )
    .unwrap();
    let base = ["--declaration", decl.as_str(), "--pin", pin.as_str()];
    let mut init = Command::new(bin).args(["init", "--admin", "ana", "--display", "Ana"]).args(base).arg("--json")
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    init.stdin.take().unwrap().write_all(b"ana pass\n").unwrap();
    let init = init.wait_with_output().unwrap();
    assert!(init.status.success(), "{}", String::from_utf8_lossy(&init.stderr));

    let port = free_port();
    let mut daemon = Command::new(bin).arg("serve").args(base)
        .args(["--listen", &format!("127.0.0.1:{port}"), "--engine", &format!("127.0.0.1:{}", free_port())])
        .stdout(Stdio::null()).stderr(Stdio::piped()).spawn().unwrap();
    let mut first = String::new();
    BufReader::new(daemon.stderr.take().unwrap()).read_line(&mut first).unwrap();
    assert!(first.contains("listening"), "{first}");
    let _daemon = Kill(daemon);
    let c = Client { port };

    // No session: 401 on the administration API too.
    assert_eq!(c.send("GET", "/api/v1/authority/me", None, None, false).status, 401);
    let ana = c.login("ana", "ana pass").cookie();
    let me = c.get("/api/v1/authority/me", &ana);
    assert_eq!(me.status, 200, "{}", me.body);
    let me = me.json();
    assert_eq!((me["account"]["subject"].as_str(), me["administer"].as_bool()), (Some("ana"), Some(true)));
    assert!(me["account"].get("credential").is_none(), "no verifier material exposed");
    let ana_session = me["session"].as_str().unwrap().to_owned();

    // Accounts: create (witness), duplicate and invalid refusals, Origin required.
    let bob = json!({"subject":"bob","display_name":"Bob","roles":["viewer"],"integrity":1,"attributes":{"team":"blue"},"password":"bob pass"});
    assert_eq!(c.send("POST", "/api/v1/authority/accounts", Some(&ana), Some(bob.clone()), false).status, 403, "Origin required");
    let created = c.post("/api/v1/authority/accounts", &ana, bob.clone());
    assert_eq!(created.status, 200, "{}", created.body);
    assert_eq!(created.json()["account"]["attributes"]["team"], "blue");
    assert!(!created.body.contains("bob pass"));
    assert_eq!(error(&c.post("/api/v1/authority/accounts", &ana, bob)), "account-exists");
    let bad = c.post("/api/v1/authority/accounts", &ana, json!({"subject":"zed","display_name":"Z","roles":["root"],"integrity":1,"password":"x"}));
    assert_eq!((bad.status, error(&bad).as_str()), (400, "account-roles-invalid"));
    assert_eq!(error(&c.post("/api/v1/authority/accounts", &ana, json!({"subject":"zed","extra":1}))), "authority-request-invalid");
    let carol = json!({"subject":"carol","display_name":"Carol","roles":["approver"],"integrity":3,"password":"carol pass"});
    assert_eq!(c.post("/api/v1/authority/accounts", &ana, carol).status, 200);
    let list = c.get("/api/v1/authority/accounts", &ana).json();
    let subjects: Vec<&str> = list["accounts"].as_array().unwrap().iter().map(|a| a["subject"].as_str().unwrap()).collect();
    assert_eq!(subjects, ["ana", "bob", "carol"]);

    // Public sign-in assets and forgotten-password requests (no session).
    let logo = c.send("GET", "/login/logo.png", None, None, false);
    assert!(logo.status == 200 && logo.headers.iter().any(|(k, v)| k == "content-type" && v == "image/png"));
    assert_eq!(c.send("GET", "/login/icon.svg", None, None, false).status, 200);
    let login_page = c.send("GET", "/login", None, None, false);
    assert!(login_page.body.contains("/login/logo.png") && login_page.body.contains("/login/forgot"));
    assert!(c.send("GET", "/login/forgot?lang=en", None, None, false).body.contains("Ask for a reset"));
    let forgot = |who: &str| {
        let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let form = format!("subject={who}&next=%2F");
        s.write_all(format!("POST /login/forgot HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: http://127.0.0.1:{port}\r\nConnection: close\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{form}", form.len()).as_bytes()).unwrap();
        let mut raw = String::new();
        s.read_to_string(&mut raw).unwrap();
        raw.split_once("\r\n\r\n").map(|(h, b)| (h.lines().next().unwrap().to_owned(), b.to_owned())).unwrap()
    };
    let (known, unknown, invalid) = (forgot("bob"), forgot("nobody"), forgot("..%2Fx"));
    assert_eq!((known.0.as_str(), &known.1), ("HTTP/1.1 200 OK", &unknown.1), "no account oracle");
    assert_eq!(known.1, invalid.1);
    forgot("bob");
    let resets = c.get("/api/v1/authority/accounts", &ana).json()["reset_requests"].clone();
    assert_eq!(resets.as_object().unwrap().keys().collect::<Vec<_>>(), ["bob"]);

    // Non-admins: `me` works, every admin route (GET included) is denied.
    let bob_cookie = c.login("bob", "bob pass").cookie();
    assert_eq!(c.get("/api/v1/authority/me", &bob_cookie).json()["administer"], false);
    let denied = c.get("/api/v1/authority/accounts", &bob_cookie).json();
    assert_eq!((denied["error"].as_str(), denied["reason"].as_str()), (Some("authority-policy-denied"), Some("policy-integrity-floor")));
    let carol_cookie = c.login("carol", "carol pass").cookie();
    let denied = c.get("/api/v1/authority/sessions", &carol_cookie).json();
    assert_eq!((denied["action"].as_str(), denied["reason"].as_str()), (Some("administer"), Some("policy-no-grant")));

    // Profile change keeps the session alive with the new attributes.
    let changed = c.post("/api/v1/authority/accounts/bob/profile", &ana, json!({"display_name":"Bob B","roles":["operator"],"integrity":2,"attributes":{}}));
    assert_eq!(changed.status, 200, "{}", changed.body);
    let bob_me = c.get("/api/v1/authority/me", &bob_cookie).json();
    assert_eq!((bob_me["account"]["roles"][0].as_str(), bob_me["account"]["integrity"].as_i64()), (Some("operator"), Some(2)));
    assert_eq!(error(&c.post("/api/v1/authority/accounts/nobody/profile", &ana, json!({"display_name":"N","roles":["viewer"],"integrity":1}))), "account-missing");

    // Self-lockout guards: no self-demotion, no self-disable.
    assert_eq!(error(&c.post("/api/v1/authority/accounts/ana/profile", &ana, json!({"display_name":"Ana","roles":["viewer"],"integrity":1}))), "authority-self-lockout");
    assert_eq!(error(&c.post("/api/v1/authority/accounts/ana/disable", &ana, json!({}))), "authority-self-lockout");
    assert_eq!(c.get("/api/v1/authority/me", &ana).json()["administer"], true, "still admin after refusals");

    // Sessions: listed without tokens; revoking one ends exactly that session.
    let sessions = c.get("/api/v1/authority/sessions", &ana).json();
    assert_eq!(sessions["current"].as_str(), Some(ana_session.as_str()));
    let rows = sessions["sessions"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    assert!(!sessions.to_string().contains("as1-"), "no token in the listing");
    let carol_id = rows.iter().find(|r| r["subject"] == "carol").unwrap()["id"].as_str().unwrap().to_owned();
    assert_eq!(c.post(&format!("/api/v1/authority/sessions/{carol_id}/revoke"), &ana, json!({})).status, 200);
    assert_eq!(c.get("/api/v1/authority/me", &carol_cookie).status, 401);
    assert_eq!(c.post(&format!("/api/v1/authority/sessions/{carol_id}/revoke"), &ana, json!({})).status, 404);

    // Admin password reset ends the target's sessions; disable ends sign-in.
    let reset = c.post("/api/v1/authority/accounts/bob/password", &ana, json!({"password":"bob new"}));
    assert_eq!((reset.status, reset.json()["sessions_ended"].as_u64()), (200, Some(1)));
    assert!(c.get("/api/v1/authority/accounts", &ana).json()["reset_requests"].as_object().unwrap().is_empty(), "request answered by the reset");
    assert_eq!(c.get("/api/v1/authority/me", &bob_cookie).status, 401);
    let bob_cookie = c.login("bob", "bob new").cookie();
    assert_eq!(c.post("/api/v1/authority/accounts/bob/disable", &ana, json!({})).json()["disabled"], true);
    assert_eq!(c.get("/api/v1/authority/me", &bob_cookie).status, 401);
    assert_eq!(c.login("bob", "bob new").status, 401);

    // Policy: read, preview (current vs proposed), refusals, apply, persisted.
    let pol = c.get("/api/v1/authority/policy", &ana).json();
    assert_eq!((pol["source"].as_str(), pol["floors"]["write"].as_i64()), (Some("installation"), Some(2)));
    let revision = pol["revision"].as_str().unwrap().to_owned();
    let mut locked = pol["policy"].clone();
    locked["grants"].as_array_mut().unwrap().retain(|g| g["role"] != "admin");
    let preview = c.post("/api/v1/authority/policy/preview", &ana, json!({"policy": locked, "method":"POST","path":"/api/v1/mission","roles":["admin"],"integrity":3}));
    let preview = preview.json();
    assert_eq!((preview["current"]["allowed"].as_bool(), preview["proposed"]["allowed"].as_bool(), preview["keeps_your_administration"].as_bool()), (Some(true), Some(false), Some(false)));
    assert_eq!(error(&c.post("/api/v1/authority/policy", &ana, json!({"policy": locked, "revision": revision}))), "authority-self-lockout");
    let mut wider = pol["policy"].clone();
    wider["grants"].as_array_mut().unwrap().push(json!({"role":"approver","actions":["write"],"when":{"team":"red"}}));
    assert_eq!(error(&c.post("/api/v1/authority/policy", &ana, json!({"policy": wider, "revision": "0"}))), "authority-revision-stale");
    assert_eq!(error(&c.post("/api/v1/authority/policy", &ana, json!({"policy": {"schema_version":1,"grants":[]}, "revision": revision}))), "authority-policy-invalid");
    assert!(!conf.join("policy.json").exists(), "refusals wrote nothing");
    let applied = c.post("/api/v1/authority/policy", &ana, json!({"policy": wider, "revision": revision}));
    assert_eq!(applied.status, 200, "{}", applied.body);
    let saved: Value = serde_json::from_slice(&fs::read(conf.join("policy.json")).unwrap()).unwrap();
    assert_eq!(saved, wider);
    assert_eq!(fs::metadata(conf.join("policy.json")).unwrap().permissions().mode() & 0o777, 0o600);
    let after = c.get("/api/v1/authority/policy", &ana).json();
    assert_eq!((after["source"].as_str(), after["revision"].as_str()), (Some("saved"), applied.json()["revision"].as_str()));

    // Configuration: preview, stale refusal, apply persisted and live.
    let cfg = c.get("/api/v1/authority/config", &ana).json();
    let mut next = cfg["config"].clone();
    next["session_idle_seconds"] = json!(120);
    assert_eq!(c.post("/api/v1/authority/config/preview", &ana, json!({"config": next})).json()["valid"], true);
    let mut invalid = next.clone();
    invalid["session_idle_seconds"] = json!(999_999);
    assert_eq!(error(&c.post("/api/v1/authority/config/preview", &ana, json!({"config": invalid}))), "authority-config-invalid");
    assert_eq!(error(&c.post("/api/v1/authority/config", &ana, json!({"config": next, "revision": "stale"}))), "authority-revision-stale");
    let applied = c.post("/api/v1/authority/config", &ana, json!({"config": next, "revision": cfg["revision"]}));
    assert_eq!(applied.status, 200, "{}", applied.body);
    let saved: Value = serde_json::from_slice(&fs::read(conf.join("authority.json")).unwrap()).unwrap();
    assert_eq!(saved["session_idle_seconds"], 120);
    assert_eq!(c.get("/api/v1/authority/config", &ana).json()["config"]["session_idle_seconds"], 120);
    assert!(fs::read_dir(&conf).unwrap().all(|e| !e.unwrap().file_name().to_string_lossy().ends_with(".tmp")), "no temporary left");

    // Own password: wrong current refused; success swaps the cookie.
    assert_eq!(error(&c.post("/api/v1/authority/me/password", &ana, json!({"current":"nope","new":"ana 2"}))), "account-authentication-refused");
    let own = c.post("/api/v1/authority/me/password", &ana, json!({"current":"ana pass","new":"ana 2"}));
    assert_eq!(own.status, 200, "{}", own.body);
    let fresh = own.cookie();
    assert_eq!(c.get("/api/v1/authority/me", &ana).status, 401, "old session ended");
    assert_eq!(c.get("/api/v1/authority/me", &fresh).status, 200);
    assert_eq!(c.login("ana", "ana pass").status, 401);
    assert_eq!(c.get("/api/v1/authority/unknown", &fresh).status, 404);

    // Audit: attribution of sign-ins and of every mutating request, chained.
    let audit = c.get("/api/v1/authority/audit", &fresh).json();
    assert_eq!(audit["chain_valid"], true, "witness: intact chain");
    let entries = audit["entries"].as_array().unwrap();
    let has = |event: &str, subject: &str, path: Option<&str>| entries.iter().any(|e| e["event"] == event && e["subject"] == subject && path.is_none_or(|p| e["path"] == p));
    assert!(has("sign-in", "ana", None) && has("sign-in-refused", "bob", None));
    assert!(has("intent", "ana", Some("/api/v1/authority/accounts")) && has("outcome", "ana", Some("/api/v1/authority/policy")));
    assert!(has("intent", "ana", Some("/api/v1/authority/me/password")));
    assert!(entries.iter().all(|e| e["event"] != "intent" || e["method"] == "POST"), "reads are not audited");
    assert!(!audit.to_string().contains("ana pass") && !audit.to_string().contains("as1-"), "no secret in the audit");
    let seqs: Vec<u64> = entries.iter().map(|e| e["seq"].as_u64().unwrap()).collect();
    assert_eq!(seqs, (1..=seqs.len() as u64).collect::<Vec<_>>());
    // Fail closed: an unwritable audit refuses the mutation, with no effect.
    let log = conf.join("audit.jsonl");
    fs::set_permissions(&log, fs::Permissions::from_mode(0o400)).unwrap();
    let dave = json!({"subject":"dave","display_name":"Dave","roles":["viewer"],"integrity":1,"password":"dave pass"});
    let refused = c.post("/api/v1/authority/accounts", &fresh, dave.clone());
    assert_eq!((refused.status, refused.body.as_str()), (503, "authority-audit-unavailable"));
    assert_eq!(c.get("/api/v1/authority/me", &fresh).status, 200, "reads still served");
    fs::set_permissions(&log, fs::Permissions::from_mode(0o600)).unwrap();
    let list = c.get("/api/v1/authority/accounts", &fresh).json();
    assert!(list["accounts"].as_array().unwrap().iter().all(|a| a["subject"] != "dave"), "no effect while refused");
    assert_eq!(c.post("/api/v1/authority/accounts", &fresh, dave).status, 200, "recovered");
    assert_eq!(c.get("/api/v1/authority/audit", &fresh).json()["chain_valid"], true);
    let requested = c.get("/api/v1/authority/audit", &fresh).json()["entries"].as_array().unwrap().iter().filter(|e| e["event"] == "reset-requested").count();
    assert_eq!(requested, 1, "repeated and unknown requests write nothing");
    // Break-glass `set-password` (sudo path) while the daemon runs.
    let mut cmd = Command::new(bin).args(["set-password", "ana"]).args(base).arg("--json")
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    cmd.stdin.take().unwrap().write_all(b"ana 3\n").unwrap();
    let out = cmd.wait_with_output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(c.get("/api/v1/authority/me", &fresh).status, 401, "sessions end with the new generation");
    let fresh = c.login("ana", "ana 3").cookie();
    // Tampering is detected (not prevented).
    let text = fs::read_to_string(&log).unwrap().replacen("\"sign-in\"", "\"sign-out\"", 1);
    fs::write(&log, text).unwrap();
    assert_eq!(c.get("/api/v1/authority/audit", &fresh).json()["chain_valid"], false);
    let _ = fs::remove_dir_all(&p);
}
