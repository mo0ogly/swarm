//! B6/B8 access decision: permission catalog, roles (built-in and custom),
//! Biba floor, ABAC conditions, policy validation (schema 3 and the 1/2
//! upgrades), the view catalog and route inventory coverage.
use std::collections::BTreeMap;
use swarm_authority::policy::{classify, classify_permission, decide, view_permission, Action, Policy, Subject, VIEWS, BUILTIN_ROLES, CATALOG};

fn subject<'a>(roles: &'a [String], integrity: i64, attributes: &'a BTreeMap<String, String>) -> Subject<'a> {
    Subject { roles, integrity, attributes }
}

fn all_permissions() -> String {
    CATALOG.iter().map(|p| format!("\"{}\"", p.key)).chain(VIEWS.iter().map(|v| format!("\"{}\"", v.key))).collect::<Vec<_>>().join(",")
}

fn doc(extra_roles: &str, extra_grants: &str) -> Vec<u8> {
    format!(
        r#"{{"schema_version":3,"roles":[{{"key":"viewer","label":"L"}},{{"key":"operator","label":"O"}},{{"key":"approver","label":"A"}},{{"key":"admin","label":"Ad"}}{extra_roles}],
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
    let no_admin_full = br#"{"schema_version":3,"roles":[{"key":"viewer","label":"L"},{"key":"operator","label":"O"},{"key":"approver","label":"A"},{"key":"admin","label":"Ad"}],"grants":[{"role":"admin","permissions":["cockpit.read"]}]}"#;
    let missing_builtin = br#"{"schema_version":3,"roles":[{"key":"admin","label":"Ad"}],"grants":[]}"#;
    for (raw, code) in [
        (no_admin_full.to_vec(), "authority-policy-invalid"),
        (missing_builtin.to_vec(), "authority-policy-invalid"),
        (doc(r#",{"key":"viewer","label":"dup"}"#, ""), "authority-policy-invalid"),
        (doc("", r#",{"role":"ghost","permissions":["cockpit.read"]}"#), "authority-policy-invalid"),
        (doc("", r#",{"role":"viewer","permissions":["delete.all"]}"#), "authority-policy-invalid"),
        (doc("", r#",{"role":"viewer","permissions":["cockpit.read","cockpit.read"]}"#), "authority-policy-invalid"),
        (doc(r#",{"key":"bad key","label":"x"}"#, ""), "authority-policy-invalid"),
        (br#"{"schema_version":4,"roles":[],"grants":[]}"#.to_vec(), "authority-policy-format"),
        (br#"{"schema_version":1,"grants":[]}"#.to_vec(), "authority-policy-invalid"),
        (br#"{"schema_version":1,"grants":[{"role":"viewer","actions":["delete"]}]}"#.to_vec(), "authority-policy-format"),
        ({ let mut v = doc("", ""); while v.last() != Some(&b'}') { v.pop(); } v.pop(); v.extend_from_slice(br#","extra":1}"#); v }, "authority-policy-format"),
    ] {
        assert_eq!(Policy::decode(&raw).unwrap_err(), code, "{}", String::from_utf8_lossy(&raw));
    }
    // Schema 1 (families per fixed role) is converted; admin gets the whole catalog.
    let v1 = Policy::decode(br#"{"schema_version":1,"grants":[{"role":"viewer","actions":["read"]},{"role":"operator","actions":["read","write"],"when":{"team":"a"}}]}"#).unwrap();
    assert_eq!(v1.schema_version, 3);
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

/// The 13 cockpit menu entries of `web/index.html` (12 buttons; the
/// `authority` entry is injected by `web/cockpit-authority.js`) plus the 3
/// standalone pages must equal the view catalog — and conversely. A menu
/// entry or page without a `view.<id>` key would be ungovernable, a catalog
/// key without a menu entry dead weight.
#[test]
fn views_match_menu() {
    let index = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../web/index.html")).unwrap();
    let mut menu: std::collections::BTreeSet<&str> = index.split("data-view=\"").skip(1).map(|s| s.split('"').next().unwrap()).collect();
    let cockpit_js = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../web/cockpit-authority.js")).unwrap();
    assert!(cockpit_js.contains("data-view=\"authority\""), "cockpit-authority.js injects the authority entry");
    menu.insert("authority");
    let catalog: std::collections::BTreeSet<&str> = VIEWS.iter().map(|v| v.id).collect();
    let pages: std::collections::BTreeSet<&str> = ["live", "trust-native", "native-runtime"].into_iter().collect();
    let mut expected = menu.clone();
    expected.extend(pages);
    assert_eq!(catalog, expected, "menu (data-view + injected) + pages must equal the view catalog");
    for v in VIEWS.iter() {
        if menu.contains(v.id) {
            assert_ne!(v.section, "Pages", "cockpit view {} must not be a page", v.id);
        } else {
            assert_eq!(v.section, "Pages", "standalone page {} must say Pages", v.id);
        }
        assert_eq!(v.key, format!("view.{}", v.id), "key derives from id");
        assert!(!v.label.trim().is_empty());
    }
    // Exclusive routes of one view must not be claimed by another.
    for (i, a) in VIEWS.iter().enumerate() {
        for b in VIEWS.iter().skip(i + 1) {
            for ra in a.routes {
                assert!(!b.routes.iter().any(|rb| rb == ra), "{} and {} both claim {}", a.id, b.id, ra);
            }
        }
    }
}

/// A route exclusively claimed by one view additionally requires that view
/// permission: family held but view missing gives the distinct
/// `policy-no-view` refusal (not a silent pass, not a plain no-grant);
/// with both keys the request passes.
#[test]
fn exclusive_route_requires_view_permission() {
    assert_eq!(view_permission("/api/v1/agent-logs"), Some("view.logs"));
    assert_eq!(view_permission("/api/v1/live/annotations"), Some("view.live"));
    assert_eq!(view_permission("/api/v1/works"), None, "shared routes stay family-gated");
    // Family held (cockpit.read covers both GET routes), views missing.
    let p = Policy::decode(&doc("", r#",{"role":"viewer","permissions":["cockpit.read"]}"#)).unwrap();
    let viewer = vec!["viewer".to_owned()];
    let none = BTreeMap::new();
    let d = decide(&p, &subject(&viewer, 1, &none), "GET", "/api/v1/agent-logs");
    assert_eq!((d.allowed, d.permission, d.view, d.reason), (false, "cockpit.read", Some("view.logs"), "policy-no-view"));
    let d = decide(&p, &subject(&viewer, 1, &none), "GET", "/api/v1/live/overview");
    assert_eq!((d.allowed, d.view, d.reason), (false, Some("view.live"), "policy-no-view"));
    // View granted: passes.
    let with_view = Policy::decode(&doc(
        "",
        r#",{"role":"viewer","permissions":["cockpit.read","view.logs","view.native-runtime"]}"#,
    ))
    .unwrap();
    assert!(decide(&with_view, &subject(&viewer, 1, &none), "GET", "/api/v1/agent-logs").allowed);
    // But the route's own write permission stays required for mutations
    // (I3 subject: past the floor, so the refusal is the missing grant).
    let d = decide(&with_view, &subject(&viewer, 3, &none), "POST", "/api/v1/agent-logs/clear");
    assert_eq!((d.allowed, d.reason), (false, "policy-no-grant"), "view never grants the family");
    // The default policy: viewer reads logs (view + family both granted).
    let inst = Policy::installation().unwrap();
    assert!(decide(&inst, &subject(&viewer, 1, &none), "GET", "/api/v1/agent-logs").allowed);
}

/// The schema 2 → 3 conversion is mechanical and lossless: each role
/// receives the views reachable through its families; every request that
/// was allowed before stays allowed (family key unchanged, view key added);
/// unknown v2 keys still refuse. The shipped v3 default must equal the
/// conversion of the shipped v2 default (archived verbatim below, commit
/// 562953dba3's `config/policy-defaults.json`) — no operator drift.
#[test]
fn v2_conversion_preserves_access() {
    const V2_DEFAULT: &str = r#"{
 "schema_version": 2,
 "roles": [
  {"key": "viewer", "label": "Lecteur", "description": "Consulte le cockpit et les préparations."},
  {"key": "operator", "label": "Opérateur", "description": "Pilote les missions et mène les préparations."},
  {"key": "approver", "label": "Approbateur", "description": "Consulte et approuve les actions de confiance."},
  {"key": "admin", "label": "Administrateur", "description": "Toutes les habilitations (non modifiable)."}
 ],
 "grants": [
  {"role": "viewer", "permissions": ["cockpit.read", "preparations.read"]},
  {"role": "operator", "permissions": ["cockpit.read", "missions.write", "preparations.read", "preparations.write", "settings.read", "providers.read", "budgets.read"]},
  {"role": "approver", "permissions": ["cockpit.read", "preparations.read", "approvals.approve"]},
  {"role": "admin", "permissions": ["cockpit.read", "missions.write", "preparations.read", "preparations.write", "approvals.approve", "settings.read", "settings.write", "providers.read", "providers.write", "budgets.read", "budgets.write", "accounts.manage", "audit.read", "unclassified.write"]}
 ]
}"#;
    let v2 = Policy::decode(V2_DEFAULT.as_bytes()).unwrap();
    assert_eq!(v2.schema_version, 3, "decoded as the converted schema 3");
    let shipped = Policy::installation().unwrap();
    assert_eq!(v2, shipped, "shipped v3 default must be the conversion image of the archived v2 default");
    let none = BTreeMap::new();
    let viewer = vec!["viewer".to_owned()];
    // Every previously allowed request stays allowed (spot checks across
    // the viewer's own v2 reach: cockpit reads, exclusive-page data).
    for (method, path) in [("GET", "/api/v1/works"), ("GET", "/api/v1/agent-logs"), ("GET", "/api/v1/live/overview"), ("GET", "/api/v1/exchanges")] {
        let d = decide(&v2, &subject(&viewer, 3, &none), method, path);
        assert!(d.allowed, "{method} {path} must survive conversion: {d:?}");
    }
    // And never more: the viewer v2 had no budgets.read, so view.budget was
    // not converted in (a view is granted only through its families).
    let d = decide(&v2, &subject(&viewer, 3, &none), "GET", "/api/v1/work-budget");
    assert_eq!((d.allowed, d.reason), (false, "policy-no-grant"));
    // A v2 document with an unknown permission still refuses.
    let bad = br#"{"schema_version":2,"roles":[{"key":"viewer","label":"L"},{"key":"operator","label":"O"},{"key":"approver","label":"A"},{"key":"admin","label":"Ad"}],"grants":[{"role":"admin","permissions":["delete.all"]}]}"#;
    assert_eq!(Policy::decode(bad).unwrap_err(), "authority-policy-invalid");
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
