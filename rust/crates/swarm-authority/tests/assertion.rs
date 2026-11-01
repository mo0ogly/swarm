//! B5.1 assertion: valid witness first, then each refusal code.
use ed25519_dalek::SigningKey;
use swarm_authority::assertion::{body_digest, sign, verify, Claims, WindowNonces};

fn claims(now: i64, nonce: &str) -> Claims {
    Claims {
        v: 1,
        subject: "ana".into(),
        roles: vec!["operator".into()],
        integrity: 2,
        method: "POST".into(),
        path: "/api/v1/action?x=1".into(),
        body_sha256: body_digest(b"{}"),
        issued_at: now,
        nonce: nonce.into(),
    }
}

#[test]
fn assertion_binds_request_freshness_and_single_use() {
    let key = SigningKey::from_bytes(&[3; 32]);
    let pk = key.verifying_key();
    let mut nonces = WindowNonces::default();
    let h = sign(&key, &claims(1000, "nonce-aaaaaaaaaaaa")).unwrap();
    let ok = verify(&pk, &h, "POST", "/api/v1/action?x=1", b"{}", 1005, 30, &mut nonces).unwrap();
    assert_eq!(ok.subject, "ana");
    let v = |h: &str, m: &str, p: &str, b: &[u8], now: i64, n: &mut WindowNonces| verify(&pk, h, m, p, b, now, 30, n).unwrap_err();
    assert_eq!(v(&h, "POST", "/api/v1/action?x=1", b"{}", 1005, &mut nonces), "assertion-replayed");
    let h2 = sign(&key, &claims(1000, "nonce-bbbbbbbbbbbb")).unwrap();
    let mut fresh = WindowNonces::default();
    assert_eq!(v(&h2, "GET", "/api/v1/action?x=1", b"{}", 1005, &mut fresh), "assertion-request-mismatch");
    assert_eq!(v(&h2, "POST", "/api/v1/action?x=2", b"{}", 1005, &mut fresh), "assertion-request-mismatch");
    assert_eq!(v(&h2, "POST", "/api/v1/action?x=1", b"{ }", 1005, &mut fresh), "assertion-request-mismatch");
    assert_eq!(v(&h2, "POST", "/api/v1/action?x=1", b"{}", 1031, &mut fresh), "assertion-stale");
    assert_eq!(v(&h2, "POST", "/api/v1/action?x=1", b"{}", 969, &mut fresh), "assertion-stale");
    let other = SigningKey::from_bytes(&[4; 32]);
    let forged = sign(&other, &claims(1000, "nonce-cccccccccccc")).unwrap();
    assert_eq!(v(&forged, "POST", "/api/v1/action?x=1", b"{}", 1000, &mut fresh), "assertion-signature");
    // Tampered claims with the original signature.
    let parts: Vec<&str> = h2.split('.').collect();
    let tampered = sign(&key, &Claims { roles: vec!["admin".into()], ..claims(1000, "nonce-bbbbbbbbbbbb") }).unwrap();
    let swapped = format!("v1.{}.{}", tampered.split('.').nth(1).unwrap(), parts[2]);
    assert_eq!(v(&swapped, "POST", "/api/v1/action?x=1", b"{}", 1000, &mut fresh), "assertion-signature");
    for bad in ["", "v2.a.b", "v1.a", "v1.!!.??", "v1.e30.AAAA"] {
        assert_eq!(v(bad, "POST", "/api/v1/action?x=1", b"{}", 1000, &mut fresh), "assertion-format", "{bad}");
    }
    assert_eq!(verify(&pk, &h2, "POST", "/api/v1/action?x=1", b"{}", 1000, 0, &mut fresh).unwrap_err(), "assertion-window-invalid");
    // Pruning forgets only nonces that can no longer pass freshness.
    nonces.prune(2000, 30);
    let replay_late = verify(&pk, &h, "POST", "/api/v1/action?x=1", b"{}", 2000, 30, &mut nonces).unwrap_err();
    assert_eq!(replay_late, "assertion-stale");
}
