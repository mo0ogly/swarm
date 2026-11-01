//! B6 access decision: route classes, default grants, Biba floor, ABAC
//! conditions, policy document validation, and route inventory coverage.
use std::collections::BTreeMap;
use swarm_authority::policy::{classify, decide, Action, Policy, Subject};

fn subject<'a>(roles: &'a [String], integrity: i64, attributes: &'a BTreeMap<String, String>) -> Subject<'a> {
    Subject { roles, integrity, attributes }
}

#[test]
fn shipped_default_grants_follow_roles() {
    let p = Policy::installation().expect("default policy valid");
    let none = BTreeMap::new();
    let viewer = vec!["viewer".to_owned()];
    let operator = vec!["operator".to_owned()];
    let admin = vec!["admin".to_owned()];
    // Witness: the same viewer may read.
    assert!(decide(&p, &subject(&viewer, 3, &none), "GET", "/api/v1/works").allowed);
    let d = decide(&p, &subject(&viewer, 3, &none), "POST", "/api/v1/mission");
    assert_eq!((d.allowed, d.action, d.reason), (false, Action::Write, "policy-no-grant"));
    assert!(decide(&p, &subject(&operator, 2, &none), "POST", "/api/v1/mission").allowed);
    let d = decide(&p, &subject(&operator, 3, &none), "POST", "/api/v1/native-settings/apply");
    assert_eq!((d.allowed, d.action), (false, Action::Administer));
    assert!(decide(&p, &subject(&admin, 3, &none), "POST", "/api/v1/native-settings/apply").allowed);
    // Preview is read-only: a viewer may preview, not apply.
    assert!(decide(&p, &subject(&viewer, 1, &none), "POST", "/api/v1/native-settings/preview").allowed);
}

#[test]
fn permissive_grant_cannot_bypass_biba_floor() {
    let p = Policy::decode(br#"{"schema_version":1,"grants":[{"role":"viewer","actions":["read","write","approve","administer"]}]}"#).unwrap();
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
fn attribute_conditions_restrict_grants() {
    let p = Policy::decode(br#"{"schema_version":1,"grants":[{"role":"operator","actions":["read","write"],"when":{"team":"alpha"}}]}"#).unwrap();
    let operator = vec!["operator".to_owned()];
    let alpha = BTreeMap::from([("team".to_owned(), "alpha".to_owned())]);
    let beta = BTreeMap::from([("team".to_owned(), "beta".to_owned())]);
    assert!(decide(&p, &subject(&operator, 2, &alpha), "POST", "/api/v1/preparations/send").allowed);
    assert!(!decide(&p, &subject(&operator, 2, &beta), "POST", "/api/v1/preparations/send").allowed);
    assert!(!decide(&p, &subject(&operator, 2, &BTreeMap::new()), "GET", "/api/v1/works").allowed);
}

#[test]
fn invalid_policy_documents_refuse() {
    assert!(Policy::decode(br#"{"schema_version":1,"grants":[{"role":"viewer","actions":["read"]}]}"#).is_ok(), "witness");
    for (raw, code) in [
        (&br#"{"schema_version":2,"grants":[{"role":"viewer","actions":["read"]}]}"#[..], "authority-policy-invalid"),
        (br#"{"schema_version":1,"grants":[]}"#, "authority-policy-invalid"),
        (br#"{"schema_version":1,"grants":[{"role":"root","actions":["read"]}]}"#, "authority-policy-invalid"),
        (br#"{"schema_version":1,"grants":[{"role":"viewer","actions":[]}]}"#, "authority-policy-invalid"),
        (br#"{"schema_version":1,"grants":[{"role":"viewer","actions":["delete"]}]}"#, "authority-policy-format"),
        (br#"{"schema_version":1,"grants":[{"role":"viewer","actions":["read"],"extra":1}]}"#, "authority-policy-format"),
    ] {
        assert_eq!(Policy::decode(raw).unwrap_err(), code);
    }
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    assert!(Policy::load(dir.path()).is_ok(), "absent file -> shipped default");
    std::fs::write(dir.path().join("policy.json"), b"{").unwrap();
    assert_eq!(Policy::load(dir.path()).unwrap_err(), "authority-policy-format");
}

#[test]
fn unknown_mutation_is_administer_and_classes_are_explicit() {
    assert_eq!(classify("POST", "/api/v1/brand-new"), Action::Administer);
    assert_eq!(classify("GET", "/api/v1/brand-new"), Action::Read);
    assert_eq!(classify("POST", "/api/v1/paea/git-call"), Action::Write);
    assert_eq!(classify("POST", "/api/v1/paea/git-call/approval"), Action::Approve);
    assert_eq!(classify("POST", "/api/v1/paea/git-call/approval/presentation"), Action::Read);
    assert_eq!(classify("POST", "/api/v1/visit-state"), Action::Write);
    assert_eq!(classify("POST", "/api/v1/pricing"), Action::Administer);
    assert_eq!(classify("POST", "/api/v1/pricing/estimate"), Action::Read);
    // Every engine API route literal is reviewed: none may fall to the
    // unknown-mutation default except the GET-only families listed here.
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../swarm-web/src/service.rs")).unwrap();
    let get_only = [
        "/api/v1/activity", "/api/v1/agent-logs", "/api/v1/agent-transcript", "/api/v1/exchanges", "/api/v1/live/agent",
        "/api/v1/live/alerts", "/api/v1/live/dataset", "/api/v1/live/events", "/api/v1/live/fingerprint", "/api/v1/live/overview",
        "/api/v1/live/run", "/api/v1/logs", "/api/v1/native-capabilities", "/api/v1/native-session", "/api/v1/native-version",
        "/api/v1/planning-history", "/api/v1/runtime-health", "/api/v1/web-requests/script.js", "/api/v1/logout",
    ];
    let mut unreviewed = Vec::new();
    for part in source.split('"').skip(1).step_by(2) {
        if part.starts_with("/api/v1/") && !get_only.contains(&part) && classify("POST", part) == Action::Administer {
            let reviewed_admin = ["native-settings", "-policy", "admin-advice", "graph-performance", "project-profile", "provider", "quotas", "run-limits", "web-requests", "work-budget", "pricing", "reconcile-effects"];
            if !reviewed_admin.iter().any(|k| part.contains(k)) {
                unreviewed.push(part.to_owned());
            }
        }
    }
    unreviewed.sort();
    unreviewed.dedup();
    assert!(unreviewed.is_empty(), "engine routes without explicit class: {unreviewed:?}");
}
