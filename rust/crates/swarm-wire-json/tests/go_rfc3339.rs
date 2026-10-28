#![cfg(feature = "timestamp")]
use swarm_wire_json::go_rfc3339::parse;
#[test]
fn go_date_spellings_keep_instants_and_refuse_bad_calendar_without_echo() {
    let expected = parse("2026-10-18T01:02:03.123456789Z").unwrap();
    for spelling in [
        "2026-10-18T1:02:03,123456789Z",
        "2026-10-18T01:02:03.123456789999Z",
    ] {
        assert_eq!(parse(spelling).unwrap(), expected);
    }
    assert!(parse("0000-01-01T00:00:00+24:60").is_ok());
    assert!(parse("9999-12-31T23:59:59-24:60").is_ok());
    for raw in [
        "private",
        "2023-02-29T00:00:00Z",
        "2026-10-18T24:00:00Z",
        "2026-10-18T00:00:00+25:00",
    ] {
        let e = parse(raw).unwrap_err();
        assert!(!e.to_string().contains(raw));
    }
}
