use swarm_enrollment_config::{
    decode, decode_change, encode, encode_change, snapshot_view, validate_projection,
    validate_view, Entry, Error, Head, Pins, Projection, Snapshot, LOCAL_SOCKET_PATH_MAX_BYTES,
};
use time::UtcOffset;
mod common;
use common::*;
#[test]
fn canonical_history_owns_data_and_links_every_entry() {
    let s = snapshot();
    let raw = encode(&s).unwrap();
    assert!(String::from_utf8_lossy(&raw).contains("\"revision\":\"0\""));
    let mut d = decode(&raw).unwrap();
    assert_eq!(encode(&d).unwrap(), raw);
    d.history[0].change.reason = "changed".into();
    assert_eq!(s.history[0].change.reason, "explicit fixture choice");
    let mut s = snapshot();
    let base = snapshot_view(&s).unwrap().head;
    s.history.push(Entry {
        change: change("second", base),
        actor: s.pins.principal.clone(),
        at: s.history[0].at,
    });
    let good = encode(&s).unwrap();
    assert_eq!(snapshot_view(&s).unwrap().head.revision, 2);
    s.history[0].change.reason = "tampered".into();
    assert_eq!(encode(&s).unwrap_err().code(), "format");
    assert_eq!(encode(&decode(&good).unwrap()).unwrap(), good);
}
#[test]
fn changes_keep_u64_exact_and_refuse_noncanonical_frames() {
    for revision in [0, 1, 9_007_199_254_740_993, u64::MAX] {
        let head = Head {
            revision,
            digest: if revision == 0 {
                String::new()
            } else {
                "a".repeat(64)
            },
        };
        let c = change("event", head);
        let raw = encode_change(&c).unwrap();
        assert_eq!(decode_change(&raw).unwrap(), c);
    }
    let raw = encode_change(&change("event", Head::default())).unwrap();
    let raw = String::from_utf8(raw).unwrap();
    for bad in [
        format!(" {raw}"),
        raw.replace("\"revision\":\"0\"", "\"revision\":0"),
        raw.replace(
            "\"revision\":\"0\"",
            "\"revision\":\"18446744073709551616\"",
        ),
        raw.replace("\"event_id\"", "\"Event_Id\""),
        format!("{{\"unknown\":true,{}", &raw[1..]),
        format!("{{\"event_id\":\"event\",{}", &raw[1..]),
    ] {
        assert!(decode_change(bad.as_bytes()).is_err(), "{bad}");
    }
    assert_eq!(
        decode_change(
            raw.replace("\"schema_version\":1", "\"schema_version\":2")
                .as_bytes()
        )
        .unwrap_err()
        .code(),
        "version"
    );
    assert_eq!(
        decode_change(b"{\"schema_version\":2,\"unknown\":true}")
            .unwrap_err()
            .code(),
        "version"
    );
    assert_eq!(decode_change(b"{}").unwrap_err().code(), "version");
}
#[test]
fn values_pins_and_history_rejections_do_not_mutate_input() {
    let valid = snapshot();
    assert!(encode(&valid).is_ok());
    let variants: Vec<fn(&mut Snapshot)> = vec![
        |s| s.schema_version = 2,
        |s| s.pins.public_key.push('='),
        |s| s.pins.installation_digest.clear(),
        |s| s.pins.principal.kind = "service".into(),
        |s| s.history.clear(),
        |s| s.history[0].actor.subject = "other".into(),
        |s| s.history[0].at = at("0001-01-01T00:00:00Z"),
        |s| s.history[0].change.expected_head.revision = 1,
        |s| s.history[0].change.reason = " ".into(),
        |s| s.history[0].change.event_id = " ".into(),
        |s| s.history[0].change.values.socket_path = "/run/../a.sock".into(),
        |s| s.history[0].change.values.socket_path = "//a.sock".into(),
        |s| s.history[0].change.values.socket_path = "/a/.".into(),
        |s| s.history[0].change.values.socket_path = "/".into(),
        |s| s.history[0].change.values.socket_path = "relative.sock".into(),
        |s| s.history[0].change.values.socket_path = "/a\0sock".into(),
        |s| s.history[0].change.values.mode = "auto".into(),
        |s| s.history.push(s.history[0].clone()),
    ];
    for mutate in variants {
        let mut bad = valid.clone();
        mutate(&mut bad);
        let original = bad.clone();
        assert!(encode(&bad).is_err());
        assert_eq!(bad, original);
        assert!(encode(&valid).is_ok());
    }
    let mut bad = valid.clone();
    bad.history[0].change.values.mode = "remote".into();
    assert!(matches!(encode(&bad), Err(Error::Mode)));
    let mut s = valid.clone();
    s.history[0].change.values.socket_path =
        format!("/{}", "x".repeat(LOCAL_SOCKET_PATH_MAX_BYTES - 1));
    assert!(encode(&s).is_ok());
    s.history[0].change.values.socket_path.push('x');
    assert!(encode(&s).is_err());
    assert!(matches!(
        Pins {
            public_key: String::new(),
            ..pins()
        }
        .validate(),
        Err(Error::Pins)
    ));
}
#[test]
fn clocks_normalize_on_copy_and_views_refuse_offsets_or_wrong_heads() {
    let s = snapshot();
    let v = snapshot_view(&s).unwrap();
    assert!(validate_view(&v).is_ok());
    let mut offset = s.clone();
    offset.history[0].at = offset.history[0]
        .at
        .to_offset(UtcOffset::from_hms(1, 0, 0).unwrap());
    assert_eq!(encode(&offset).unwrap(), encode(&s).unwrap());
    assert_ne!(offset.history[0].at.offset(), UtcOffset::UTC);
    let mut bad = v.clone();
    bad.snapshot = offset;
    assert!(validate_view(&bad).is_err());
    let mut bad = v.clone();
    bad.head.digest = "b".repeat(64);
    assert!(matches!(validate_view(&bad), Err(Error::Head)));
    for replay in [false, true] {
        let p = Projection {
            base: if replay {
                v.head.clone()
            } else {
                Head::default()
            },
            proposed: v.clone(),
            current_event_replay: replay,
        };
        assert!(validate_projection(&p).is_ok());
    }
    for p in [
        Projection {
            base: v.head.clone(),
            proposed: v.clone(),
            current_event_replay: false,
        },
        Projection {
            base: Head::default(),
            proposed: v,
            current_event_replay: true,
        },
    ] {
        assert!(matches!(validate_projection(&p), Err(Error::Head)));
    }
}
#[test]
fn all_unknown_duplicate_folded_missing_and_null_snapshot_fields_refused() {
    let raw = String::from_utf8(encode(&snapshot()).unwrap()).unwrap();
    for bad in [
        format!(" {raw}"),
        format!("{raw}{{}}"),
        format!("{{\"schema_version\":1,{}", &raw[1..]),
        raw.replace("\"mode\":\"local\"", "\"mode\":\"local\",\"tls_off\":true"),
        raw.replace("\"schema_version\"", "\"Schema_Version\""),
        raw.replace("\"schema_version\":1,", ""),
        raw.replace("\"schema_version\":1", "\"schema_version\":null"),
    ] {
        assert!(decode(bad.as_bytes()).is_err(), "{bad}");
    }
    assert!(matches!(
        decode(b"{\"schema_version\":2,\"unknown\":true}"),
        Err(Error::Version)
    ));
}

#[test]
fn go_clock_spellings_keep_error_priority_without_accepting_noncanonical_data() {
    let raw = String::from_utf8(encode(&snapshot()).unwrap()).unwrap();
    for spelling in [
        "2026-10-17T1:00:00Z",
        "2026-10-17T12:00:00,123Z",
        "2026-10-17T12:00:00.123456789012Z",
        "2026-10-17T12:00:00+24:00",
        "2026-10-17T12:00:00+01:60",
        "2026-10-17T12:00:00+24:60",
        "0000-01-01T00:00:00+24:60",
        "9999-12-31T23:59:59-24:60",
    ] {
        assert!(decode(raw.as_bytes()).is_ok());
        let alternate = raw.replace("2026-10-17T12:00:00Z", spelling);
        assert_eq!(decode(alternate.as_bytes()).unwrap_err().code(), "format");
        let future = alternate.replacen("\"schema_version\":1", "\"schema_version\":2", 1);
        assert_eq!(
            decode(future.as_bytes()).unwrap_err().code(),
            "version",
            "{spelling}"
        );
    }
    for spelling in [
        "2026-10-17t12:00:00Z",
        "2026-10-17T12:00:60Z",
        "2026-10-17T12:00:00+25:00",
        "2026-10-17T12:00:00+01:61",
        r"2026-10-17\u005412:00:00Z",
    ] {
        let future = raw.replace("2026-10-17T12:00:00Z", spelling).replacen(
            "\"schema_version\":1",
            "\"schema_version\":2",
            1,
        );
        assert_eq!(
            decode(future.as_bytes()).unwrap_err().code(),
            "format",
            "{spelling}"
        );
        assert!(decode(raw.as_bytes()).is_ok());
    }
}

#[test]
fn go_json_nesting_limit_precedes_schema_version_classification() {
    assert_eq!(
        decode(b"{\"schema_version\":2,\"unknown\":true}")
            .unwrap_err()
            .code(),
        "version"
    );
    for (depth, expected) in [(10000, "version"), (10001, "format")] {
        let raw = format!(
            "{{\"schema_version\":2,\"unknown\":{}0{}}}",
            "[".repeat(depth - 1),
            "]".repeat(depth - 1)
        );
        assert_eq!(
            decode(raw.as_bytes()).unwrap_err().code(),
            expected,
            "depth {depth}"
        );
    }
}

#[test]
fn public_diagnostics_preserve_causes_without_echoing_input_or_callback_secrets() {
    let marker = "fixture-private-credential-should-not-be-logged";
    let raw = format!("{{\"schema_version\":\"{marker}\"}}");
    let error = decode_change(raw.as_bytes()).unwrap_err();
    assert_eq!(error.code(), "format");
    assert!(std::error::Error::source(&error)
        .unwrap()
        .to_string()
        .contains(marker));
    assert!(!error.to_string().contains(marker));
    assert!(!format!("{error:?}").contains(marker));
    let error = Error::Io {
        class: swarm_enrollment_config::IoClass::Storage,
        cause: std::io::Error::other(marker),
    };
    let wrapped = Error::Authorization(Box::new(error));
    assert_eq!(wrapped.code(), "unauthorized");
    assert!(std::error::Error::source(&wrapped).is_some());
    assert!(!wrapped.to_string().contains(marker));
    assert!(!format!("{wrapped:?}").contains(marker));
}

#[test]
fn signed_negative_zero_reaches_version_validation_instead_of_a_type_error() {
    assert_eq!(
        swarm_enrollment_config::decode_change(br#"{"schema_version":-0}"#)
            .unwrap_err()
            .code(),
        "version"
    );
    assert_eq!(
        swarm_enrollment_config::decode(br#"{"schema_version":-0}"#)
            .unwrap_err()
            .code(),
        "version"
    );
}
