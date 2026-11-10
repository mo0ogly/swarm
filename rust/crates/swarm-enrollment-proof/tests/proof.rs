use ed25519_dalek::SigningKey;
#[cfg(feature = "test-fixtures")]
use std::io::{self, Cursor, Read};
use std::sync::{Arc, Barrier};
use swarm_enrollment_proof::{sign_response, Error, Verifier};

fn key() -> SigningKey {
    SigningKey::from_bytes(&[7; 32])
}

fn verifier() -> Verifier {
    Verifier::new(key().verifying_key().as_bytes(), [11; 32]).unwrap()
}

#[test]
fn actual_entropy_creates_distinct_bound_requests() {
    let verifier = verifier();
    let a = verifier.new_challenge([13; 32]).unwrap();
    let b = verifier.new_challenge([13; 32]).unwrap();
    assert_ne!(a.request().nonce, b.request().nonce);
    assert_eq!(a.request().project_digest, "0b".repeat(32));
    assert_eq!(a.request().request_digest, "0d".repeat(32));
    assert!(!a.request().nonce.is_empty());
}

#[test]
fn bad_key_length_never_constructs_a_verifier() {
    for key in [&[][..], &[0; 31][..], &[0; 33][..]] {
        assert!(matches!(Verifier::new(key, [0; 32]), Err(Error::Key)));
    }
    assert!(Verifier::new(&[0; 32], [0; 32]).is_ok());
}

#[test]
fn signature_round_trip_owns_payload_and_request_and_rejects_replay() {
    let verifier = verifier();
    let challenge = verifier.new_challenge([13; 32]).unwrap();
    let raw = sign_response(&challenge.request(), b"payload\0<>&", &key()).unwrap();
    let statement = verifier.verify(&challenge, &raw).unwrap();
    let mut copy = statement.payload();
    copy[0] = b'!';
    assert_eq!(statement.payload(), b"payload\0<>&");
    let mut request = statement.request();
    request.nonce.clear();
    assert!(!statement.request().nonce.is_empty());
    assert_eq!(statement.request(), challenge.request());
    assert!(matches!(
        verifier.verify(&challenge, &raw),
        Err(Error::Spent)
    ));
}

#[test]
fn verifier_public_key_is_owned() {
    let mut public = *key().verifying_key().as_bytes();
    let verifier = Verifier::new(&public, [11; 32]).unwrap();
    public.fill(0);
    let challenge = verifier.new_challenge([13; 32]).unwrap();
    let raw = sign_response(&challenge.request(), b"", &key()).unwrap();
    assert!(verifier
        .verify(&challenge, &raw)
        .unwrap()
        .payload()
        .is_empty());
}

#[test]
fn equal_key_and_context_do_not_imply_equal_verifier_ownership() {
    let a = verifier();
    let b = verifier();
    let challenge = a.new_challenge([13; 32]).unwrap();
    let raw = sign_response(&challenge.request(), b"bound", &key()).unwrap();
    assert!(matches!(b.verify(&challenge, &raw), Err(Error::Challenge)));
    assert!(a.verify(&challenge, &raw).is_ok());
}

#[test]
fn malformed_responses_do_not_consume_valid_challenges() {
    let verifier = verifier();
    for bad in [
        b"".as_slice(),
        b"null",
        b"[]",
        b"{}",
        b"{\"protocol\":null}",
        b"{\"protocol\":1}",
    ] {
        let control = verifier.new_challenge([13; 32]).unwrap();
        let valid = sign_response(&control.request(), b"control", &key()).unwrap();
        assert!(verifier.verify(&control, &valid).is_ok());
        let challenge = verifier.new_challenge([13; 32]).unwrap();
        let valid = sign_response(&challenge.request(), b"control", &key()).unwrap();
        assert!(matches!(
            verifier.verify(&challenge, bad),
            Err(Error::Response)
        ));
        assert!(verifier.verify(&challenge, &valid).is_ok());
    }
}

#[test]
fn challenge_binding_precedes_signature_and_does_not_consume() {
    let verifier = verifier();
    let a = verifier.new_challenge([13; 32]).unwrap();
    let b = verifier.new_challenge([13; 32]).unwrap();
    let raw = sign_response(&a.request(), b"bound", &key()).unwrap();
    assert!(matches!(verifier.verify(&b, &raw), Err(Error::Binding)));
    let valid = sign_response(&b.request(), b"bound", &key()).unwrap();
    assert!(verifier.verify(&b, &valid).is_ok());
}

#[test]
fn clones_share_consumption_with_exactly_one_concurrent_success() {
    let verifier = Arc::new(verifier());
    let challenge = verifier.new_challenge([13; 32]).unwrap();
    let raw = Arc::new(sign_response(&challenge.request(), b"concurrent", &key()).unwrap());
    let barrier = Arc::new(Barrier::new(32));
    let mut workers = Vec::new();
    for _ in 0..32 {
        let (verifier, challenge, raw, barrier) = (
            verifier.clone(),
            challenge.clone(),
            raw.clone(),
            barrier.clone(),
        );
        workers.push(std::thread::spawn(move || {
            barrier.wait();
            match verifier.verify(&challenge, &raw) {
                Ok(_) => true,
                Err(Error::Spent) => false,
                Err(error) => panic!("unexpected error: {error}"),
            }
        }));
    }
    assert_eq!(
        workers
            .into_iter()
            .map(|w| usize::from(w.join().unwrap()))
            .sum::<usize>(),
        1
    );
}

#[test]
fn signing_rejects_wrong_context_and_noncanonical_nonce() {
    let verifier = verifier();
    let challenge = verifier.new_challenge([13; 32]).unwrap();
    for field in ["protocol", "key", "project", "request", "nonce"] {
        let mut request = challenge.request();
        assert!(sign_response(&request, b"control", &key()).is_ok());
        match field {
            "protocol" => request.protocol = "other".into(),
            "key" => request.key_id = "0".repeat(64),
            "project" => request.project_digest = "A".repeat(64),
            "request" => request.request_digest = "g".repeat(64),
            "nonce" => request.nonce.push('='),
            _ => unreachable!(),
        }
        assert!(matches!(
            sign_response(&request, b"control", &key()),
            Err(Error::Challenge)
        ));
    }
}

#[test]
fn correctly_signed_historical_bytes_do_not_attest_current_authority() {
    let verifier = verifier();
    let challenge = verifier.new_challenge([13; 32]).unwrap();
    let raw = sign_response(&challenge.request(), b"opaque historical state", &key()).unwrap();
    assert_eq!(
        verifier.verify(&challenge, &raw).unwrap().payload(),
        b"opaque historical state"
    );
}

#[test]
fn go_json_lossy_string_decoding_preserves_signature_error_and_recovery() {
    let verifier = verifier();
    for replacement in [b"\\ud800".as_slice(), b"\\udfff", &[0xe2, 0x82], &[0xff]] {
        let control = verifier.new_challenge([13; 32]).unwrap();
        let valid = sign_response(&control.request(), b"control", &key()).unwrap();
        assert!(verifier.verify(&control, &valid).is_ok());
        let challenge = verifier.new_challenge([13; 32]).unwrap();
        let valid = sign_response(&challenge.request(), b"control", &key()).unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(&valid).unwrap();
        value["signature"] = "SENTINEL".into();
        let raw = serde_json::to_vec(&value).unwrap();
        let position = raw.windows(8).position(|w| w == b"SENTINEL").unwrap();
        let mut bad = raw[..position].to_vec();
        bad.extend_from_slice(replacement);
        bad.extend_from_slice(&raw[position + 8..]);
        assert!(matches!(
            verifier.verify(&challenge, &bad),
            Err(Error::Signature)
        ));
        assert!(verifier.verify(&challenge, &valid).is_ok());
    }
}

#[cfg(feature = "test-fixtures")]
#[test]
fn entropy_failure_returns_no_challenge_and_next_attempt_is_usable() {
    struct Broken;
    impl Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("fixture entropy failed"))
        }
    }
    let verifier = verifier();
    assert!(matches!(
        verifier.challenge_with_reader([13; 32], &mut Broken),
        Err(Error::Io(_))
    ));
    assert!(matches!(
        verifier.challenge_with_reader([13; 32], &mut Cursor::new([0; 31])),
        Err(Error::Io(_))
    ));
    let challenge = verifier
        .challenge_with_reader([13; 32], &mut Cursor::new([17; 32]))
        .unwrap();
    let raw = sign_response(&challenge.request(), b"recover", &key()).unwrap();
    assert!(verifier.verify(&challenge, &raw).is_ok());
}
