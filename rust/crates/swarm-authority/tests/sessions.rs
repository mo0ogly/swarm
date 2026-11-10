//! B4 sessions and daemon configuration, on a real signed journal in the
//! configured fixture area (never /tmp). `now` is explicit, so expiry is
//! exercised without sleeping.
use ed25519_dalek::SigningKey;
use std::collections::BTreeMap;
use swarm_authority::{accounts::{Accounts, Profile}, config::AuthorityConfig, sessions::Sessions};
use swarm_enrollment_authority::credentials::Parameters;
use swarm_enrollment_state::Context;

const P: Parameters = Parameters { memory_kib: 8, passes: 1, parallelism: 1 };

fn dir() -> tempfile::TempDir {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.swarm-fixtures");
    std::fs::create_dir_all(&base).unwrap();
    tempfile::tempdir_in(base).unwrap()
}

#[test]
fn config_defaults_override_and_refusals() {
    let d = AuthorityConfig::installation().unwrap();
    assert!(d.session_idle_seconds <= d.session_absolute_seconds);
    assert_eq!(d.audit_retention_days, Some(365), "shipped retention");
    assert_eq!(d.log_level, "info");
    let t = dir();
    assert_eq!(AuthorityConfig::load(t.path()).unwrap(), d, "absent file = shipped default");
    let mut custom = d;
    custom.session_idle_seconds = 60;
    std::fs::write(t.path().join("authority.json"), serde_json::to_vec(&custom).unwrap()).unwrap();
    assert_eq!(AuthorityConfig::load(t.path()).unwrap(), custom);
    // ADM-08/09 fields: default when absent (old documents decode), no
    // retention means `None`, unknown level and sub-day retention refused.
    let legacy = r#"{"schema_version":1,"session_absolute_seconds":10,"session_idle_seconds":5,"argon2":{"memory_kib":8,"passes":1,"parallelism":1},"relay_body_max_bytes":1}"#;
    let decoded = AuthorityConfig::decode(legacy.as_bytes()).unwrap();
    assert_eq!(decoded.audit_retention_days, None, "no default retention");
    assert_eq!(decoded.log_level, "info");
    for bad in [
        r#"{"schema_version":1}"#,
        r#"{"schema_version":1,"session_absolute_seconds":0,"session_idle_seconds":0,"argon2":{"memory_kib":8,"passes":1,"parallelism":1},"relay_body_max_bytes":1}"#,
        r#"{"schema_version":1,"session_absolute_seconds":10,"session_idle_seconds":5,"argon2":{"memory_kib":1,"passes":1,"parallelism":1},"relay_body_max_bytes":1}"#,
        r#"{"schema_version":1,"session_absolute_seconds":10,"session_idle_seconds":5,"argon2":{"memory_kib":8,"passes":1,"parallelism":1},"relay_body_max_bytes":1,"extra":1}"#,
        r#"{"schema_version":1,"session_absolute_seconds":10,"session_idle_seconds":5,"argon2":{"memory_kib":8,"passes":1,"parallelism":1},"relay_body_max_bytes":1,"audit_retention_days":0}"#,
        r#"{"schema_version":1,"session_absolute_seconds":10,"session_idle_seconds":5,"argon2":{"memory_kib":8,"passes":1,"parallelism":1},"relay_body_max_bytes":1,"log_level":"loud"}"#,
    ] {
        std::fs::write(t.path().join("authority.json"), bad).unwrap();
        assert!(AuthorityConfig::load(t.path()).is_err(), "{bad}");
    }
}

#[test]
fn sessions_expire_and_follow_account_state() {
    let t = dir();
    let ctx = Context::new();
    let a = Accounts::open(&ctx, &t.path().join("a.sqlite"), SigningKey::from_bytes(&[7; 32]), [11; 32], &"ab".repeat(32)).unwrap();
    let profile = Profile { display_name: "Ana".into(), roles: vec!["operator".into()], integrity: 2, attributes: BTreeMap::new() };
    a.create(&ctx, "ana", &profile, b"pw-one", P).unwrap();
    a.create(&ctx, "bob", &profile, b"pw-bob", P).unwrap();
    let mut cfg = AuthorityConfig::installation().unwrap();
    cfg.session_absolute_seconds = 100;
    cfg.session_idle_seconds = 30;
    let s = Sessions::new(cfg);

    let (tok, view) = s.login(&ctx, &a, "ana", b"pw-one", 1000).unwrap();
    assert!(tok.starts_with("as1-") && view.subject == "ana");
    assert_eq!(s.resolve(&ctx, &a, &tok, 1020).unwrap().roles, ["operator"]);
    assert_eq!(s.login(&ctx, &a, "ana", b"bad", 1000).unwrap_err().code(), "account-authentication-refused");
    for bad in ["", "as1-unknown", "garbage"] {
        assert_eq!(s.resolve(&ctx, &a, bad, 1000).unwrap_err().code(), "session-refused");
    }
    // idle: 1020 -> 1050 is 30 s of inactivity = expired
    assert_eq!(s.resolve(&ctx, &a, &tok, 1050).unwrap_err().code(), "session-refused");
    // absolute: kept active every 20 s, still ends at created + 100
    let (tok, _) = s.login(&ctx, &a, "ana", b"pw-one", 2000).unwrap();
    for now in [2020, 2040, 2060, 2080] { s.resolve(&ctx, &a, &tok, now).unwrap(); }
    assert_eq!(s.resolve(&ctx, &a, &tok, 2100).unwrap_err().code(), "session-refused");
    // password change ends the session at the next request
    let (tok, _) = s.login(&ctx, &a, "ana", b"pw-one", 3000).unwrap();
    a.set_password(&ctx, "ana", b"pw-two", P).unwrap();
    assert_eq!(s.resolve(&ctx, &a, &tok, 3001).unwrap_err().code(), "session-refused");
    // disable ends it too; logout and revoke_subject
    let (tok_b, _) = s.login(&ctx, &a, "bob", b"pw-bob", 4000).unwrap();
    let (tok_b2, _) = s.login(&ctx, &a, "bob", b"pw-bob", 4000).unwrap();
    s.logout(&tok_b).unwrap();
    assert!(s.resolve(&ctx, &a, &tok_b, 4001).is_err());
    s.resolve(&ctx, &a, &tok_b2, 4001).unwrap();
    assert_eq!(s.revoke_subject("bob").unwrap(), 1);
    assert!(s.resolve(&ctx, &a, &tok_b2, 4002).is_err());
    let (tok_a, _) = s.login(&ctx, &a, "ana", b"pw-two", 5000).unwrap();
    a.disable(&ctx, "ana").unwrap();
    assert_eq!(s.resolve(&ctx, &a, &tok_a, 5001).unwrap_err().code(), "session-refused");
}
