//! B2.2 account registry on a real signed journal. Fixture directory in the
//! configured fixture area (never /tmp). Argon2 costs here are the minimum
//! valid values for speed; production costs come from an operator setting.
use ed25519_dalek::SigningKey;
use std::collections::BTreeMap;
use swarm_authority::accounts::{Accounts, Profile};
use swarm_enrollment_authority::credentials::Parameters;
use swarm_enrollment_state::Context;

const P: Parameters = Parameters { memory_kib: 8, passes: 1, parallelism: 1 };

fn profile(roles: &[&str], integrity: i64) -> Profile {
    Profile { display_name: "Ana Example".into(), roles: roles.iter().map(|r| r.to_string()).collect(), integrity, attributes: BTreeMap::from([("team".into(), "core".into())]) }
}

fn open(dir: &std::path::Path) -> Accounts {
    Accounts::open(&Context::new(), &dir.join("accounts.sqlite"), SigningKey::from_bytes(&[7; 32]), [11; 32], &"ab".repeat(32)).unwrap()
}

#[test]
fn create_verify_change_disable_and_refusals() {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.swarm-fixtures");
    std::fs::create_dir_all(&base).unwrap();
    let dir = tempfile::tempdir_in(base).unwrap();
    let ctx = Context::new();
    let a = open(dir.path());
    a.create(&ctx, "ana", &profile(&["operator"], 2), b"first secret", P).unwrap();
    let v = a.verify(&ctx, "ana", b"first secret").unwrap();
    assert_eq!((v.roles.clone(), v.integrity, v.generation), (vec!["operator".to_owned()], 2, 1));
    for (subject, pw) in [("ana", &b"wrong"[..]), ("nobody", &b"first secret"[..]), ("bad/subject", &b"x"[..])] {
        assert_eq!(a.verify(&ctx, subject, pw).unwrap_err().code(), "account-authentication-refused");
    }
    assert_eq!(a.create(&ctx, "ana", &profile(&["viewer"], 1), b"x", P).unwrap_err().code(), "account-exists");
    let before = a.list(&ctx).unwrap();
    for (subject, p, code) in [
        ("ana b", profile(&["viewer"], 1), "account-subject-invalid"),
        ("bob", profile(&["bad role"], 1), "account-roles-invalid"), // existence is the policy's check
        ("bob", profile(&["viewer", "viewer"], 1), "account-roles-invalid"),
        ("bob", profile(&[], 1), "account-roles-invalid"),
        ("bob", profile(&["viewer"], 0), "account-integrity-invalid"),
        ("bob", profile(&["viewer"], 4), "account-integrity-invalid"),
    ] {
        assert_eq!(a.create(&ctx, subject, &p, b"pw", P).unwrap_err().code(), code);
    }
    assert_eq!(a.list(&ctx).unwrap(), before, "refusals write nothing");

    a.set_password(&ctx, "ana", b"second secret", P).unwrap();
    assert_eq!(a.verify(&ctx, "ana", b"first secret").unwrap_err().code(), "account-authentication-refused");
    assert_eq!(a.verify(&ctx, "ana", b"second secret").unwrap().generation, 2);
    let listed = a.list(&ctx).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].attributes.get("team").map(String::as_str), Some("core"));
    assert!(!serde_json::to_string(&listed).unwrap().contains("derived"), "no verifier in views");

    a.create(&ctx, "zoe", &profile(&["admin", "approver"], 3), b"zoe secret", P).unwrap();
    assert_eq!(a.list(&ctx).unwrap().iter().map(|v| v.subject.as_str()).collect::<Vec<_>>(), ["ana", "zoe"]);
    a.disable(&ctx, "ana").unwrap();
    assert_eq!(a.verify(&ctx, "ana", b"second secret").unwrap_err().code(), "account-authentication-refused");
    assert_eq!(a.disable(&ctx, "ana").unwrap_err().code(), "account-missing");
    a.create(&ctx, "ana", &profile(&["viewer"], 1), b"third secret", P).unwrap();
    assert_eq!(a.verify(&ctx, "ana", b"third secret").unwrap().generation, 1);

    // Persistence: a reopened registry sees the same signed state.
    a.close().unwrap();
    let b = open(dir.path());
    assert_eq!(b.verify(&ctx, "zoe", b"zoe secret").unwrap().roles, ["admin", "approver"]);
}
