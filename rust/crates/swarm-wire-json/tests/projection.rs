use swarm_wire_json::projection::{self as p, Kind};
const INNER: p::Fields = &[("count", Kind::Int64), ("label", Kind::String)];
const SHAPE: p::Fields = &[
    ("schema_version", Kind::Int64),
    ("flag", Kind::Bool),
    ("value", Kind::Object(INNER)),
    ("pointer", Kind::OptionalObject(INNER)),
    (
        "items",
        Kind::Array {
            element: &Kind::String,
            nil: true,
        },
    ),
    (
        "map",
        Kind::Map(&Kind::Array {
            element: &Kind::String,
            nil: true,
        }),
    ),
];
#[test]
fn projection_preserves_duplicate_null_pointer_array_and_map_semantics() {
    let v=p::project(br#"{"schema_version":-0,"value":{"count":4},"value":{"count":null,"label":"last"},"pointer":{"count":8},"pointer":null,"items":[null,"x"],"map":{"first":["x"]},"map":{"second":null}}"#,SHAPE,true).unwrap();
    assert_eq!(v["schema_version"], 0);
    assert_eq!(v["value"]["count"], 4);
    assert_eq!(v["value"]["label"], "last");
    assert!(v["pointer"].is_null());
    assert_eq!(v["items"], serde_json::json!(["", "x"]));
    assert_eq!(v["map"], serde_json::json!({"first":["x"],"second":null}));
    let v = p::project(br#"{"items":[],"items":null}"#, SHAPE, true).unwrap();
    assert!(v["items"].is_null());
}
#[test]
fn projection_integer_bounds_folded_names_unknown_fields_and_private_diagnostics() {
    for (raw, expected) in [
        (b"-0".as_slice(), 0),
        (b"9223372036854775807", i64::MAX),
        (b"-9223372036854775808", i64::MIN),
    ] {
        assert_eq!(p::signed_integer(raw).unwrap(), expected);
    }
    for raw in [
        "1.0",
        "0e0",
        "-0.0",
        "-0e0",
        "+0",
        "01",
        "9223372036854775808",
        "null",
        "true",
    ] {
        assert!(p::signed_integer(raw.as_bytes()).is_err(), "{raw}");
    }
    assert_eq!(
        p::project("{\"ſchema_version\":1}".as_bytes(), SHAPE, true).unwrap()["schema_version"],
        1
    );
    assert!(p::project(br#"{"unknown":true}"#, SHAPE, true).is_err());
    assert!(p::project(br#"{"unknown":true}"#, SHAPE, false).is_ok());
    let e = p::project(br#"{"schema_version":"private-number"}"#, SHAPE, true).unwrap_err();
    assert!(!format!("{e} {e:?}").contains("private-number"));
    assert!(p::project(
        br#"{"schema_version":false,"schema_version":1}"#,
        SHAPE,
        true
    )
    .is_err());
}
#[test]
fn duplicate_arrays_reuse_go_backing_values_and_empty_or_null_resets_them() {
    let v = p::project(
        br#"{"items":["old","keep"],"items":["new"],"items":[null,null]}"#,
        SHAPE,
        true,
    )
    .unwrap();
    assert_eq!(v["items"], serde_json::json!(["new", "keep"]));
    for reset in ["[]", "null"] {
        let raw = format!("{{\"items\":[\"old\"],\"items\":{reset},\"items\":[null]}}");
        let v = p::project(raw.as_bytes(), SHAPE, true).unwrap();
        assert_eq!(v["items"], serde_json::json!([""]));
    }
    const STRUCTS: p::Fields = &[(
        "objects",
        Kind::Array {
            element: &Kind::Object(INNER),
            nil: true,
        },
    )];
    let v = p::project(
        br#"{"objects":[{"count":4,"label":"old"}],"objects":[{"label":"new"}]}"#,
        STRUCTS,
        true,
    )
    .unwrap();
    assert_eq!(
        v["objects"][0],
        serde_json::json!({"count":4,"label":"new"})
    );
    // Map elements decode from zero even when their key already exists.
    let v = p::project(br#"{"map":{"x":["old"]},"map":{"x":[null]}}"#, SHAPE, true).unwrap();
    assert_eq!(v["map"]["x"], serde_json::json!([""]));
}
