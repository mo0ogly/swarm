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
    assert_eq!((bad.status, error(&bad).as_str()), (400, "account-role-unknown"));
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
    assert_eq!(pol["catalog"].as_array().unwrap().len(), 14);
    assert_eq!(pol["builtin_roles"], json!(["viewer", "operator", "approver", "admin"]));
    // Removing admin's full grant is an invalid policy (no lockout by editing).
    let mut locked = pol["policy"].clone();
    locked["grants"].as_array_mut().unwrap().retain(|g| g["role"] != "admin");
    assert_eq!(error(&c.post("/api/v1/authority/policy/preview", &ana, json!({"policy": locked, "method":"POST","path":"/api/v1/mission","roles":["admin"],"integrity":3}))), "authority-policy-invalid");
    assert_eq!(error(&c.post("/api/v1/authority/policy", &ana, json!({"policy": locked, "revision": revision}))), "authority-policy-invalid");
    // Custom role "auditor": preview shows the proposed effect.
    let mut wider = pol["policy"].clone();
    wider["roles"].as_array_mut().unwrap().push(json!({"key":"auditor","label":"Auditeur","description":"Lit le journal"}));
    wider["grants"].as_array_mut().unwrap().push(json!({"role":"auditor","permissions":["cockpit.read","audit.read"],"when":{"team":"red"}}));
    let preview = c.post("/api/v1/authority/policy/preview", &ana, json!({"policy": wider, "method":"GET","path":"/api/v1/authority/audit","roles":["auditor"],"integrity":3,"attributes":{"team":"red"}})).json();
    assert_eq!((preview["current"]["allowed"].as_bool(), preview["proposed"]["allowed"].as_bool(), preview["proposed"]["permission"].as_str(), preview["keeps_your_administration"].as_bool()), (Some(false), Some(true), Some("audit.read"), Some(true)));
    assert_eq!(error(&c.post("/api/v1/authority/policy", &ana, json!({"policy": wider, "revision": "0"}))), "authority-revision-stale");
    assert_eq!(error(&c.post("/api/v1/authority/policy", &ana, json!({"policy": {"schema_version":2,"roles":[],"grants":[]}, "revision": revision}))), "authority-policy-invalid");
    assert!(!conf.join("policy.json").exists(), "refusals wrote nothing");
    // A role must exist before an account can hold it.
    let dan = json!({"subject":"dan","display_name":"Dan","roles":["auditor"],"integrity":3,"attributes":{"team":"red"},"password":"dan pass"});
    assert_eq!(error(&c.post("/api/v1/authority/accounts", &ana, dan.clone())), "account-role-unknown");
    let applied = c.post("/api/v1/authority/policy", &ana, json!({"policy": wider, "revision": revision}));
    assert_eq!(applied.status, 200, "{}", applied.body);
    let saved: Value = serde_json::from_slice(&fs::read(conf.join("policy.json")).unwrap()).unwrap();
    assert_eq!(saved, wider);
    assert_eq!(fs::metadata(conf.join("policy.json")).unwrap().permissions().mode() & 0o777, 0o600);
    let after = c.get("/api/v1/authority/policy", &ana).json();
    assert_eq!((after["source"].as_str(), after["revision"].as_str()), (Some("saved"), applied.json()["revision"].as_str()));
    assert_eq!(c.post("/api/v1/authority/accounts", &ana, dan).status, 200, "custom role assignable now");
    let dan_cookie = c.login("dan", "dan pass").cookie();
    assert_eq!(c.get("/api/v1/authority/audit", &dan_cookie).status, 200, "auditor reads the audit");
    assert_eq!(c.get("/api/v1/authority/accounts", &dan_cookie).status, 403, "but does not manage accounts");
    // A role still held by an account cannot be deleted.
    let mut without = after["policy"].clone();
    without["roles"].as_array_mut().unwrap().retain(|r| r["key"] != "auditor");
    without["grants"].as_array_mut().unwrap().retain(|g| g["role"] != "auditor");
    let in_use = c.post("/api/v1/authority/policy", &ana, json!({"policy": without, "revision": after["revision"]})).json();
    assert_eq!((in_use["error"].as_str(), in_use["subject"].as_str()), (Some("authority-role-in-use"), Some("dan")));

    // Groups (ADM-01): CRUD on the signed journal, effective roles = own ∪
    // group roles seen by sessions, `/me` and policy decisions.
    assert_eq!(error(&c.post("/api/v1/authority/groups", &ana, json!({"key":"ops","label":"Ops","roles":["root"],"members":[]}))), "account-role-unknown");
    assert_eq!(c.post("/api/v1/authority/groups", &ana, json!({"key":"ops","label":"Ops","roles":["operator"],"members":["dan"]})).status, 200, "create with roles and members");
    assert_eq!(error(&c.post("/api/v1/authority/groups", &ana, json!({"key":"ops","label":"Ops","roles":[],"members":[]}))), "group-exists");
    let dan_me = c.get("/api/v1/authority/me", &dan_cookie).json();
    assert!(dan_me["account"]["roles"].as_array().unwrap().iter().any(|r| r == "operator"), "dan sees the group role");
    assert!(dan_me["permissions"].as_array().unwrap().iter().any(|p| p == "missions.write"), "effective right granted through the group");
    // The group alone gives administration-grade rights? No — operator is not admin; check the refusal stays.
    assert_eq!(c.get("/api/v1/authority/accounts", &dan_cookie).status, 403);
    // Group grants administration to dan through a custom admin-grade role.
    let pol = c.get("/api/v1/authority/policy", &ana).json();
    let revision = pol["revision"].as_str().unwrap().to_owned();
    let mut wider2 = pol["policy"].clone();
    wider2["roles"].as_array_mut().unwrap().push(json!({"key":"superop","label":"Super op","description":"Administers the daemon"}));
    wider2["grants"].as_array_mut().unwrap().push(json!({"role":"superop","permissions":["accounts.manage"]}));
    let applied2 = c.post("/api/v1/authority/policy", &ana, json!({"policy": wider2, "revision": revision}));
    assert_eq!(applied2.status, 200, "{}", applied2.body);
    let g2 = c.post("/api/v1/authority/groups", &ana, json!({"key":"ops2","label":"Ops2","roles":["superop"],"members":["dan"]}));
    assert_eq!(g2.status, 200, "{}", g2.body);
    let dan_me = c.get("/api/v1/authority/me", &dan_cookie).json();
    assert_eq!(dan_me["administer"], true, "dan administers through the group");
    assert_eq!(c.get("/api/v1/authority/accounts", &dan_cookie).status, 200);
    // Stripping the group's admin role down while dan still holds nothing
    // else: candidate state leaves ana administering — allowed (200).
    let strip = c.post("/api/v1/authority/groups/ops2/profile", &ana, json!({"label":"Ops2","roles":["viewer"],"members":["dan"]}));
    assert_eq!(strip.status, 200, "{}", strip.body);
    assert_eq!(c.get("/api/v1/authority/me", &dan_cookie).json()["administer"], false, "role retrait effective immediately");
    // Restore the admin-grade role, then delete the group as if dan were the
    // only administrator: refused `authority-last-admin`.
    assert_eq!(c.post("/api/v1/authority/groups/ops2/profile", &ana, json!({"label":"Ops2","roles":["superop"],"members":["dan"]})).status, 200);
    // dan revokes... actually dan now administers: self-lockout check via
    // dan editing his own admin group is the second guard below.
    assert_eq!(
        error(&c.post("/api/v1/authority/groups/ops2/delete", &dan_cookie, json!({}))),
        "authority-self-lockout",
        "dan cannot delete his own source of administration"
    );
    assert_eq!(c.post("/api/v1/authority/groups/ops2/delete", &ana, json!({})).status, 200, "ana remains administrator, delete allowed");
    assert_eq!(c.get("/api/v1/authority/me", &dan_cookie).json()["administer"], false, "group deletion ends the borrowed rights");
    assert_eq!(error(&c.post("/api/v1/authority/groups/ops2/delete", &ana, json!({}))), "group-missing");
    // Rename keeps members/roles under the new key; the old key is free.
    assert_eq!(c.post("/api/v1/authority/groups/ops/rename", &ana, json!({"key":"ops-team"})).status, 200);
    let groups = c.get("/api/v1/authority/groups", &ana).json()["groups"].as_array().unwrap().to_owned();
    assert!(groups.iter().any(|g| g["key"] == "ops-team" && g["members"].as_array().unwrap().iter().any(|m| m == "dan")), "rename kept members");
    assert!(!groups.iter().any(|g| g["key"] == "ops"), "old key gone");
    assert_eq!(c.post("/api/v1/authority/groups", &ana, json!({"key":"ops","label":"Ops again","roles":[],"members":[]})).status, 200, "old key reusable");
    // Duplicate copies label/roles/members under a new key.
    assert_eq!(c.post("/api/v1/authority/groups/ops-team/duplicate", &ana, json!({"key":"ops-copy","label":"Ops copy","roles":[],"members":[]})).status, 200);
    let dup = c.get("/api/v1/authority/groups", &ana).json()["groups"].as_array().unwrap().iter().find(|g| g["key"] == "ops-copy").unwrap().to_owned();
    assert_eq!((dup["roles"][0].as_str(), dup["members"][0].as_str()), (Some("operator"), Some("dan")), "duplicate copied roles and members");
    assert_eq!(error(&c.post("/api/v1/authority/groups/ops-copy/rename", &ana, json!({"key":"ops-team"}))), "group-exists", "rename onto a live key refused");
    assert_eq!(error(&c.post("/api/v1/authority/groups/nobody/profile", &ana, json!({"label":"X","roles":[],"members":[]}))), "group-missing");
    // Journal chain still valid with the group records.
    assert_eq!(c.get("/api/v1/authority/audit", &ana).json()["chain_valid"], true);

    // A browser navigation (GET with Accept: text/html) is a consultation;
    // the engine is absent here, the note is written all the same.
    let nav = {
        let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
        s.write_all(format!("GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nCookie: {ana}\r\nConnection: close\r\nAccept: text/html\r\n\r\n").as_bytes()).unwrap();
        let mut raw = String::new();
        s.read_to_string(&mut raw).unwrap();
        raw.lines().next().unwrap().to_owned()
    };
    assert!(nav.starts_with("HTTP/1.1 5"), "engine absent: {nav}");

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
    // ADM-05 enrichment: outcome lines carry duration/client/UA/view; intents
    // carry the readable detail; consultations name their view.
    let outcome = entries.iter().find(|e| e["event"] == "outcome" && e["path"] == "/api/v1/authority/accounts").expect("outcome");
    assert!(outcome["duration_ms"].as_i64().is_some(), "duration_ms present");
    assert_eq!(outcome["client"], "browser");
    assert_eq!(outcome["view"], "view.authority");
    let intent = entries.iter().find(|e| e["event"] == "intent" && e["path"] == "/api/v1/authority/accounts").expect("intent");
    assert_eq!(intent["detail"], "account created", "readable detail from path only (subject is in the body)");
    assert_eq!(intent["client"], "browser");
    assert!(entries.iter().all(|e| e.get("user_agent").and_then(|u| u.as_str()).map(|u| u.len() <= 120).unwrap_or(true)), "UA truncated to 120");
    let consult = entries.iter().find(|e| e["event"] == "consultation" && e["subject"] == "ana").expect("consultation");
    assert_eq!(consult["view"], "none", "engine pages are not exclusive views");
    assert_eq!(consult["client"], "browser");
    // Connections per session, session detail, dashboard (audit.read).
    let conns = c.get("/api/v1/authority/audit/sessions", &fresh).json();
    let rows = conns["sessions"].as_array().unwrap();
    assert!(rows.iter().all(|r| r["credential"].as_str().unwrap().starts_with("session:") || r["credential"].as_str().unwrap().starts_with("key:")));
    let carol_row = rows.iter().find(|r| r["subject"] == "carol").expect("carol session");
    assert_eq!(carol_row["refusals"], 1, "policy refusal recorded");
    let detail = c.get(&format!("/api/v1/authority/audit/sessions/{}", carol_row["credential"].as_str().unwrap()), &fresh).json();
    assert!(detail["entries"].as_array().unwrap().iter().any(|e| e["event"] == "refused" && e["path"] == "/api/v1/authority/sessions"));
    assert_eq!(c.get("/api/v1/authority/audit/sessions?subject=bob", &fresh).json()["sessions"].as_array().unwrap().iter().filter(|r| r["subject"] != "bob").count(), 0);
    // ADM-06: enrichment columns, filters, sort, pagination, export.
    let carol = carol_row.as_object().unwrap();
    assert_eq!(carol["clients"][0], "browser", "access means recorded");
    assert!(carol["views"].as_array().unwrap().iter().any(|v| v == "view.authority"), "views touched by the session");
    assert_eq!(c.get("/api/v1/authority/audit/sessions?client=api-key", &fresh).json()["sessions"].as_array().unwrap().len(), 0, "no key used in this run");
    assert_eq!(c.get("/api/v1/authority/audit/sessions?view=view.authority", &fresh).json()["sessions"].as_array().unwrap().iter().all(|r| r["views"].as_array().unwrap().iter().any(|v| v == "view.authority")), true);
    let no_admin_reply = c.get("/api/v1/authority/audit/sessions?no_admin=1", &fresh).json();
    let no_admin = no_admin_reply["sessions"].as_array().unwrap();
    assert!(no_admin.iter().all(|r| r["subject"] != "ana"), "ana administers; her sessions are filtered out");
    assert!(no_admin.iter().any(|r| r["subject"] == "bob"), "bob has no administration");
    assert_eq!(c.get("/api/v1/authority/audit/sessions?status=active", &fresh).json()["sessions"].as_array().unwrap().iter().all(|r| r["signed_out"] == false), true);
    assert_eq!(c.get("/api/v1/authority/audit/sessions?status=refused", &fresh).json()["sessions"].as_array().unwrap().iter().all(|r| r["refusals"].as_u64().unwrap() > 0), true);
    assert_eq!(error(&c.get("/api/v1/authority/audit/sessions?status=nonsense", &fresh)), "authority-request-invalid");
    assert_eq!(c.get("/api/v1/authority/audit/sessions?q=CAR", &fresh).json()["sessions"].as_array().unwrap().len(), 1, "case-insensitive search");
    let sorted_reply = c.get("/api/v1/authority/audit/sessions?sort=subject&dir=asc", &fresh).json();
    let sorted = sorted_reply["sessions"].as_array().unwrap();
    let subs: Vec<&str> = sorted.iter().map(|r| r["subject"].as_str().unwrap_or("")).collect();
    let mut want = subs.clone();
    want.sort();
    assert_eq!(subs, want, "subject sort asc");
    let page1 = c.get("/api/v1/authority/audit/sessions?per_page=2&page=1", &fresh).json();
    assert_eq!(page1["sessions"].as_array().unwrap().len(), 2);
    let total = page1["total"].as_u64().unwrap();
    let page2 = c.get("/api/v1/authority/audit/sessions?per_page=2&page=2", &fresh).json();
    assert_eq!(page2["sessions"].as_array().unwrap().len(), (total as usize - 2).min(2), "second page holds the rest");
    assert_ne!(page1["sessions"][0]["credential"], page2["sessions"][0]["credential"], "pages are disjoint");
    let csv = c.get("/api/v1/authority/audit/sessions?format=csv", &fresh);
    assert_eq!(csv.headers.iter().find(|(k, _)| k == "content-type").map(|(_, v)| v.as_str()), Some("text/csv; charset=utf-8"));
    assert!(csv.body.starts_with("credential,subject,first_at,"), "CSV header");
    assert_eq!(csv.body.lines().count() as u64, total + 1, "export is not paginated");
    let exported = c.get("/api/v1/authority/audit/sessions?format=json", &fresh).json();
    assert_eq!(exported["sessions"].as_array().unwrap().len() as u64, total, "JSON export not paginated either");
    let dash = c.get("/api/v1/authority/audit/dashboard", &fresh).json();
    assert!(!dash["days"].as_array().unwrap().is_empty() && dash["accounts"].as_array().unwrap().iter().any(|a| a["subject"] == "ana"));
    // ADM-07: KPIs, per-view usage, 20 most recent lines.
    assert!(dash["kpis"]["sign_ins"].as_u64().unwrap() >= 3, "sign-in KPI");
    assert!(dash["kpis"]["refusals"].as_u64().unwrap() >= 1, "refusal KPI");
    let views = dash["views"].as_array().unwrap();
    assert!(views.iter().any(|v| v["view"] == "view.authority" && v["actions"].as_u64().unwrap() > 0), "authority usage charted");
    assert!(views.iter().any(|v| v["view"] == "none"), "engine consultations grouped under none");
    let recent = dash["recent"].as_array().unwrap();
    assert!(!recent.is_empty() && recent.len() <= 20);
    let recents: Vec<i64> = recent.iter().map(|e| e["at"].as_i64().unwrap()).collect();
    let mut want_recent = recents.clone();
    want_recent.sort_unstable_by_key(|t| std::cmp::Reverse(*t));
    assert_eq!(recents, want_recent, "recent lines are the newest first");
    assert_eq!(error(&c.post("/api/v1/authority/audit/purge", &fresh, json!({"older_than_days": 0}))), "authority-purge-days-invalid");
    assert_eq!(error(&c.post("/api/v1/authority/audit/purge", &fresh, json!({}))), "authority-request-invalid", "no default retention");
    let purge = c.post("/api/v1/authority/audit/purge", &fresh, json!({"older_than_days": 1})).json();
    assert_eq!(purge["removed"], 0, "everything is recent");
    assert_eq!(c.get("/api/v1/authority/audit", &fresh).json()["chain_valid"], true);
    // ADM-08: manual purge is rate-limited to 5 calls per rolling minute.
    // The purge above consumed one slot; four more are accepted, the 6th
    // call (5 slots used) is refused with 429 + Retry-After.
    let statuses: Vec<u16> = (0..5)
        .map(|_| c.post("/api/v1/authority/audit/purge", &fresh, json!({"older_than_days": 1})).status)
        .collect();
    assert_eq!(&statuses[..4], &[200, 200, 200, 200], "slots 2-5 accepted");
    assert_eq!(statuses[4], 429, "6th call in the minute refused");
    // ADM-09: the daemon's diagnostics file is readable, bounded and level-filtered.
    let log = c.get("/api/v1/authority/log", &fresh).json();
    assert_eq!(log["level"], "info");
    assert!(log["lines"].as_array().unwrap().len() <= 200);
    // Configuration gains the retention and level; invalid values refused.
    // The fixture's own authority.json predates both fields: absent here
    // (None) and defaulted to "info" — the shipped 365-day default is
    // asserted in tests/sessions.rs.
    let cfg = c.get("/api/v1/authority/config", &fresh).json();
    assert_eq!(cfg["config"]["log_level"], "info");
    assert!(cfg["config"].get("audit_retention_days").is_none_or(|v| v.is_null()), "legacy document: no automatic purge");
    assert_eq!(cfg["log_levels"].as_array().unwrap().len(), 4);
    let mut next = cfg["config"].clone();
    next["log_level"] = json!("loud");
    assert_eq!(error(&c.post("/api/v1/authority/config/preview", &fresh, json!({"config": next}))), "authority-config-invalid");
    let mut next = cfg["config"].clone();
    next["audit_retention_days"] = json!(0);
    assert_eq!(error(&c.post("/api/v1/authority/config/preview", &fresh, json!({"config": next}))), "authority-config-invalid");
    let mut next = cfg["config"].clone();
    next["audit_retention_days"] = json!(null);
    next["log_level"] = json!("debug");
    let applied = c.post("/api/v1/authority/config", &fresh, json!({"config": next, "revision": cfg["revision"]}));
    assert_eq!(applied.status, 200, "{}", applied.body);
    assert_eq!(c.get("/api/v1/authority/log", &fresh).json()["level"], "debug", "level applied live");
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
