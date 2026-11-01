//! B6/B8 access decision: permission catalog, roles (built-in and custom),
//! Biba floor, ABAC conditions, policy validation (schema 2 and the
//! schema-1 upgrade) and route inventory coverage.
use std::collections::BTreeMap;
use swarm_authority::policy::{classify, classify_permission, decide, Action, Policy, Subject, BUILTIN_ROLES, CATALOG};

fn subject<'a>(roles: &'a [String], integrity: i64, attributes: &'a BTreeMap<String, String>) -> Subject<'a> {
    Subject { roles, integrity, attributes }
}

fn all_permissions() -> String {
    CATALOG.iter().map(|p| format!("\"{}\"", p.key)).collect::<Vec<_>>().join(",")
}

fn doc(extra_roles: &str, extra_grants: &str) -> Vec<u8> {
    format!(
        r#"{{"schema_version":2,"roles":[{{"key":"viewer","label":"L"}},{{"key":"operator","label":"O"}},{{"key":"approver","label":"A"}},{{"key":"admin","label":"Ad"}}{extra_roles}],
        "grants":[{{"role":"admin","permissions":[{}]}}{extra_grants}]}}"#,
        all_permissions()
    )
    .into_bytes()
}

#[test]
fn shipped_default_grants_follow_roles() {
    let p = Policy::installation().expect("default policy valid");
    let none = BTreeMap::new();
    let viewer = vec!["viewer".to_owned()];
    let operator = vec!["operator".to_owned()];
    let admin = vec!["admin".to_owned()];
    assert!(decide(&p, &subject(&viewer, 3, &none), "GET", "/api/v1/works").allowed, "witness: viewer reads");
    let d = decide(&p, &subject(&viewer, 3, &none), "POST", "/api/v1/mission");
    assert_eq!((d.allowed, d.action, d.permission, d.reason), (false, Action::Write, "missions.write", "policy-no-grant"));
    assert!(decide(&p, &subject(&operator, 2, &none), "POST", "/api/v1/mission").allowed);
    let d = decide(&p, &subject(&operator, 3, &none), "POST", "/api/v1/native-settings/apply");
    assert_eq!((d.allowed, d.permission), (false, "settings.write"));
    assert!(decide(&p, &subject(&operator, 1, &none), "POST", "/api/v1/native-settings/preview").allowed, "operator reads settings");
    assert!(!decide(&p, &subject(&viewer, 1, &none), "POST", "/api/v1/native-settings/preview").allowed, "viewer lacks settings.read");
    assert!(decide(&p, &subject(&admin, 3, &none), "POST", "/api/v1/native-settings/apply").allowed);
    assert!(decide(&p, &subject(&admin, 3, &none), "POST", "/api/v1/brand-new").allowed, "admin holds unclassified.write");
    for b in BUILTIN_ROLES {
        assert!(p.has_role(b));
    }
}

#[test]
fn permissive_grant_cannot_bypass_biba_floor() {
    let p = Policy::decode(&doc("", &format!(r#",{{"role":"viewer","permissions":[{}]}}"#, all_permissions()))).unwrap();
    let none = BTreeMap::new();
    let viewer = vec!["viewer".to_owned()];
    assert!(decide(&p, &subject(&viewer, 3, &none), "POST", "/api/v1/run-limits/apply").allowed, "witness at I3");
    for (method, path, floor) in [("POST", "/api/v1/mission", 2), ("POST", "/api/v1/trust-actions", 3), ("POST", "/api/v1/quotas/apply", 3)] {
        let d = decide(&p, &subject(&viewer, floor - 1, &none), method, path);
        assert_eq!((d.allowed, d.reason), (false, "policy-integrity-floor"), "{path}");
    }
    assert!(!decide(&p, &subject(&viewer, 0, &none), "GET", "/live.html").allowed);
}

#[test]
fn custom_roles_and_attribute_conditions() {
    let p = Policy::decode(&doc(
        r#",{"key":"auditor","label":"Auditeur","description":"Lit le journal"}"#,
        r#",{"role":"operator","permissions":["cockpit.read","preparations.write"],"when":{"team":"alpha"}},{"role":"auditor","permissions":["cockpit.read","audit.read"]}"#,
    ))
    .unwrap();
    let operator = vec!["operator".to_owned()];
    let alpha = BTreeMap::from([("team".to_owned(), "alpha".to_owned())]);
    let beta = BTreeMap::from([("team".to_owned(), "beta".to_owned())]);
    assert!(decide(&p, &subject(&operator, 2, &alpha), "POST", "/api/v1/preparations/send").allowed);
    assert!(!decide(&p, &subject(&operator, 2, &beta), "POST", "/api/v1/preparations/send").allowed);
    assert!(!decide(&p, &subject(&operator, 2, &BTreeMap::new()), "GET", "/api/v1/works").allowed);
    let auditor = vec!["auditor".to_owned()];
    let none = BTreeMap::new();
    assert!(decide(&p, &subject(&auditor, 3, &none), "GET", "/api/v1/authority/audit").allowed, "custom role reads the audit");
    assert!(!decide(&p, &subject(&auditor, 3, &none), "GET", "/api/v1/authority/accounts").allowed, "but cannot manage accounts");
    assert!(p.has_role("auditor"));
}

#[test]
fn invalid_policy_documents_refuse_and_v1_upgrades() {
    assert!(Policy::decode(&doc("", "")).is_ok(), "witness");
    let no_admin_full = br#"{"schema_version":2,"roles":[{"key":"viewer","label":"L"},{"key":"operator","label":"O"},{"key":"approver","label":"A"},{"key":"admin","label":"Ad"}],"grants":[{"role":"admin","permissions":["cockpit.read"]}]}"#;
    let missing_builtin = br#"{"schema_version":2,"roles":[{"key":"admin","label":"Ad"}],"grants":[]}"#;
    for (raw, code) in [
        (no_admin_full.to_vec(), "authority-policy-invalid"),
        (missing_builtin.to_vec(), "authority-policy-invalid"),
        (doc(r#",{"key":"viewer","label":"dup"}"#, ""), "authority-policy-invalid"),
        (doc("", r#",{"role":"ghost","permissions":["cockpit.read"]}"#), "authority-policy-invalid"),
        (doc("", r#",{"role":"viewer","permissions":["delete.all"]}"#), "authority-policy-invalid"),
        (doc("", r#",{"role":"viewer","permissions":["cockpit.read","cockpit.read"]}"#), "authority-policy-invalid"),
        (doc(r#",{"key":"bad key","label":"x"}"#, ""), "authority-policy-invalid"),
        (br#"{"schema_version":3,"roles":[],"grants":[]}"#.to_vec(), "authority-policy-format"),
        (br#"{"schema_version":1,"grants":[]}"#.to_vec(), "authority-policy-invalid"),
        (br#"{"schema_version":1,"grants":[{"role":"viewer","actions":["delete"]}]}"#.to_vec(), "authority-policy-format"),
        ({ let mut v = doc("", ""); while v.last() != Some(&b'}') { v.pop(); } v.pop(); v.extend_from_slice(br#","extra":1}"#); v }, "authority-policy-format"),
    ] {
        assert_eq!(Policy::decode(&raw).unwrap_err(), code, "{}", String::from_utf8_lossy(&raw));
    }
    // Schema 1 (families per fixed role) is converted; admin gets the whole catalog.
    let v1 = Policy::decode(br#"{"schema_version":1,"grants":[{"role":"viewer","actions":["read"]},{"role":"operator","actions":["read","write"],"when":{"team":"a"}}]}"#).unwrap();
    assert_eq!(v1.schema_version, 2);
    let none = BTreeMap::new();
    let viewer = vec!["viewer".to_owned()];
    assert!(decide(&v1, &subject(&viewer, 1, &none), "GET", "/api/v1/quotas").allowed, "read family -> every read permission");
    assert!(!decide(&v1, &subject(&viewer, 3, &none), "POST", "/api/v1/mission").allowed);
    assert!(decide(&v1, &subject(&["admin".to_owned()], 3, &none), "POST", "/api/v1/authority/policy").allowed);
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    assert!(Policy::load(dir.path()).is_ok(), "absent file -> shipped default");
    std::fs::write(dir.path().join("policy.json"), b"{").unwrap();
    assert_eq!(Policy::load(dir.path()).unwrap_err(), "authority-policy-format");
}

#[test]
fn unknown_mutation_is_unclassified_and_routes_are_reviewed() {
    assert_eq!(classify_permission("POST", "/api/v1/brand-new"), "unclassified.write");
    assert_eq!(classify("POST", "/api/v1/brand-new"), Action::Administer);
    assert_eq!(classify_permission("GET", "/api/v1/brand-new"), "cockpit.read");
    for (method, path, key) in [
        ("POST", "/api/v1/paea/git-call", "missions.write"),
        ("POST", "/api/v1/paea/git-call/approval", "approvals.approve"),
        ("POST", "/api/v1/paea/git-call/approval/presentation", "cockpit.read"),
        ("POST", "/api/v1/visit-state", "missions.write"),
        ("POST", "/api/v1/graph-batch/apply", "missions.write"),
        ("POST", "/api/v1/graph-batch/preview", "cockpit.read"),
        ("GET", "/api/v1/authority/accounts", "accounts.manage"),
        ("GET", "/api/v1/authority/audit", "audit.read"),
        ("POST", "/api/v1/pricing", "providers.write"),
        ("POST", "/api/v1/pricing/estimate", "providers.read"),
        ("GET", "/api/v1/quotas", "budgets.read"),
        ("POST", "/api/v1/preparations/send", "preparations.write"),
        ("GET", "/api/v1/preparations", "preparations.read"),
        ("POST", "/api/v1/native-settings/apply", "settings.write"),
    ] {
        assert_eq!(classify_permission(method, path), key, "{method} {path}");
    }
    // Every engine API route literal is reviewed: none may fall to the
    // unclassified default except the GET-only families listed here.
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../swarm-web/src/service.rs")).unwrap();
    let get_only = [
        "/api/v1/activity", "/api/v1/agent-logs", "/api/v1/agent-transcript", "/api/v1/exchanges", "/api/v1/live/agent",
        "/api/v1/live/alerts", "/api/v1/live/dataset", "/api/v1/live/events", "/api/v1/live/fingerprint", "/api/v1/live/overview",
        "/api/v1/live/run", "/api/v1/logs", "/api/v1/native-capabilities", "/api/v1/native-session", "/api/v1/native-version",
        "/api/v1/planning-history", "/api/v1/runtime-health", "/api/v1/web-requests/script.js", "/api/v1/logout",
    ];
    let mut unreviewed: Vec<String> = source
        .split('"')
        .skip(1)
        .step_by(2)
        .filter(|p| p.starts_with("/api/v1/") && !get_only.contains(p) && classify_permission("POST", p) == "unclassified.write")
        .map(str::to_owned)
        .collect();
    unreviewed.sort();
    unreviewed.dedup();
    assert!(unreviewed.is_empty(), "engine routes without explicit permission: {unreviewed:?}");
}
