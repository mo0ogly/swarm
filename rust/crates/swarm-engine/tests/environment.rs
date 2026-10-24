use swarm_engine::{
    format_utc_timestamp, generate_identifier, go_command_text, identifier_from_entropy,
    operator_identity, system_timestamp,
};
#[test]
fn clock_formats_go_nanoseconds_normalization_epoch_leap_dates_and_extended_years() {
    for (sec, nano, expected) in [
        (0, 0, "1970-01-01T00:00:00Z"),
        (0, 1, "1970-01-01T00:00:00.000000001Z"),
        (0, -1, "1969-12-31T23:59:59.999999999Z"),
        (0, 1_200_000_000, "1970-01-01T00:00:01.2Z"),
        (-62167219200, 0, "0000-01-01T00:00:00Z"),
        (951782400, 123_000_000, "2000-02-29T00:00:00.123Z"),
    ] {
        assert_eq!(format_utc_timestamp(sec, nano).unwrap(), expected);
    }
    assert!(format_utc_timestamp(i64::MAX, i64::MAX).is_err());
    let current = system_timestamp().unwrap();
    swarm_wire_json::go_rfc3339::parse(&current).unwrap();
}
#[test]
fn entropy_ids_match_go_representation_and_system_entropy_is_available() {
    assert_eq!(
        identifier_from_entropy("w-", &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 254, 255]),
        "w-00010203040506070809feff"
    );
    let id = generate_identifier("w-").unwrap();
    assert_eq!(id.len(), 26);
    assert!(id[2..]
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
}
#[test]
fn output_text_preserves_go_lossy_bytes_whitespace_and_control_characters() {
    assert_eq!(go_command_text(&[0xe2, 0x82]).unwrap(), "��");
    assert_eq!(go_command_text(b" \x85\n").unwrap(), "�");
    assert_eq!(
        go_command_text("\u{0085}\u{3000}value\u{3000}".as_bytes()).unwrap(),
        "value"
    );
    assert_eq!(go_command_text(b"\"\\\0\x01\n").unwrap(), "\"\\\0\x01");
    assert_eq!(go_command_text("\u{feff}".as_bytes()).unwrap(), "\u{feff}");
}
#[cfg(unix)]
#[test]
fn operator_observation_is_actual_host_and_uid_without_claiming_authentication() {
    let observed = operator_identity().unwrap();
    assert!(observed.ends_with(&format!("/uid:{}", nix::unistd::getuid().as_raw())));
}
