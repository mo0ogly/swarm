//! Pure parser witnesses and disposable IO fixtures, never launch authority.
use super::*;
use std::os::unix::fs::{symlink, PermissionsExt};
/// Private disposable source tree; no live Root or host metadata is changed.
struct Fixture(PathBuf);
impl Fixture {
    /// Allocate one new private fixture without replacing prior data.
    fn new() -> Self {
        let p = tempfile::Builder::new()
            .prefix("paea-native-prereq-")
            .tempdir()
            .unwrap()
            .keep();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        Self(p)
    }
    /// Fixed valid metadata with explicit injected PATH, never process env mutation.
    fn sources(&self) -> Sources {
        for (name, body) in [
            ("passwd", "fixture:x:1001:1001::/:/bin/false\n"),
            ("subuid", "fixture:165536:65536\nother:265536:65536\n"),
            ("subgid", "1001:165536:65536\n"),
            ("uid_map", "0 0 4294967295\n"),
            ("gid_map", "0 0 4294967295\n"),
        ] {
            fs::write(self.0.join(name), body).unwrap()
        }
        Sources {
            passwd: self.0.join("passwd"),
            subuid: self.0.join("subuid"),
            subgid: self.0.join("subgid"),
            uid_map: self.0.join("uid_map"),
            gid_map: self.0.join("gid_map"),
            path: Some(self.0.clone().into_os_string()),
        }
    }
    /// Actual executable-looking fixture helper; temp ancestry cannot be trusted.
    fn executable(&self, name: &str) {
        fs::write(self.0.join(name), b"fixture-not-executed").unwrap();
        fs::set_permissions(self.0.join(name), fs::Permissions::from_mode(0o700)).unwrap()
    }
}
impl Drop for Fixture {
    /// Remove only this fixture; retain it on panic for diagnostic evidence.
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap()
        }
    }
}
/// Valid parser witnesses precede malformed, sentinel, overlap and extent faults.
#[test]
fn linux_prerequisites_parsers_refuse_invalid_without_empty_fallback_and_recover() {
    let valid = "fixture:165536:65536\n1001:400000:2\nother:bad:bad\n";
    assert_eq!(ranges(valid, Some("fixture"), 1001).unwrap().len(), 2);
    for bad in [
        "fixture:0:1",
        "fixture:4:0",
        "fixture:4294967295:1",
        "fixture:4294967294:2",
        "fixture:1:3\n1001:3:4",
        "fixture:1",
        "fixture:+1:2",
        "fixture:１:2",
        "fixture:1:4294967296",
    ] {
        assert!(ranges(valid, Some("fixture"), 1001).is_ok());
        assert!(ranges(bad, Some("fixture"), 1001).is_err(), "{bad}");
        assert!(ranges(valid, Some("fixture"), 1001).is_ok());
    }
    assert!(ranges("other:malformed", Some("fixture"), 1001)
        .unwrap()
        .is_empty());
    assert_eq!(ranges("1001:4:2", None, 1001).unwrap()[0].start, 4);
    for bad in [
        "",
        "0 0 0",
        "0 0 4294967296",
        "1 0 4294967295",
        "0 4294967295 1",
        "0 0 -1",
        "0 0 1 2",
        "+0 0 1",
    ] {
        assert_eq!(mappings("0 0 4294967295\n").unwrap()[0].count, u32::MAX);
        assert!(mappings(bad).is_err(), "{bad}");
        assert!(mappings("0 0 1\n").is_ok());
    }
}
/// Only one valid local current-UID name; NSS and unrelated records are excluded.
#[test]
fn linux_prerequisites_local_passwd_ambiguity_and_missing_are_explicit() {
    let valid = "other:x:1002:1002::/:/bin/false\nfixture:x:1001:1001::/:/bin/false\n";
    assert_eq!(username(valid, 1001).unwrap(), "fixture");
    for bad in [
        "other:x:1002:1002::/:/bin/false\n",
        "fixture:x:1001:1001",
        "fixture:x:1001:1001::/:/bin/false\nalias:x:1001:1001::/:/bin/false\n",
    ] {
        assert!(username(valid, 1001).is_ok());
        assert!(username(bad, 1001).is_err());
        assert_eq!(username(valid, 1001).unwrap(), "fixture");
    }
}
/// Independent read/parse errors survive in a report; valid fixture restores.
#[test]
fn linux_prerequisites_collect_preserves_unavailable_invalid_and_recovery() {
    let f = Fixture::new();
    let s = f.sources();
    let report = collect(s, 1001, 1001, Ok(vec![1001]));
    assert!(report.maps.uid_map.value().is_some());
    assert_eq!(report.assigned_ranges.subuid.value().unwrap().len(), 1);
    let s = f.sources();
    fs::remove_file(&s.uid_map).unwrap();
    fs::write(&s.subuid, "fixture:invalid:2").unwrap();
    let report = collect(s, 1001, 1001, Ok(vec![1001]));
    match &report.maps.uid_map {
        Observation::Unavailable { failure } => {
            assert_eq!(failure.errno, Some(nix::libc::ENOENT));
            assert_eq!(failure.cause.kind(), io::ErrorKind::NotFound)
        }
        _ => panic!("missing source became valid"),
    }
    assert!(matches!(
        report.assigned_ranges.subuid,
        Observation::Invalid { .. }
    ));
    assert!(report.missing_prerequisites.contains(&"map:uid_map"));
    assert!(report.missing_prerequisites.contains(&"range:subuid"));
    let report = collect(f.sources(), 1001, 1001, Ok(vec![1001]));
    assert!(report.maps.uid_map.value().is_some());
    assert!(!report.missing_prerequisites.contains(&"map:uid_map"));
    assert!(
        !report.launch_verified
            && !report.separate_host_uid_verified
            && !report.confinement_verified
            && !report.human_verified
    );
}
/// Racy helper metadata remains observation only; native search failure is not
/// skipped as absence, and replacing the owned fixture permits fresh observation.
#[test]
fn linux_prerequisites_helpers_unsafe_native_failure_no_fallback_and_recovery() {
    let f = Fixture::new();
    f.executable("newuidmap");
    let path = Some(f.0.clone().into_os_string());
    let valid = helper(path.as_ref(), "newuidmap");
    assert!(!valid.value().unwrap().metadata_safe);
    fs::remove_file(f.0.join("newuidmap")).unwrap();
    symlink("newuidmap", f.0.join("newuidmap")).unwrap();
    let failed = helper(path.as_ref(), "newuidmap");
    match failed {
        Observation::Unavailable { failure } => {
            assert_eq!(failure.errno, Some(nix::libc::ELOOP));
            assert_eq!(failure.code, "helper-search");
            assert!(failure.source().is_some())
        }
        _ => panic!("native refusal skipped"),
    }
    fs::remove_file(f.0.join("newuidmap")).unwrap();
    f.executable("newuidmap");
    assert!(helper(path.as_ref(), "newuidmap").value().is_some());
    assert!(matches!(
        helper(None, "newuidmap"),
        Observation::Invalid {
            code: "helper-path-unset"
        }
    ));
    assert!(matches!(
        helper(path.as_ref(), "newgidmap"),
        Observation::Unavailable { .. }
    ));
}
/// Safe formatting/JSON preserves actual optional errno without leaking source
/// text; original custom diagnostics remain available via the typed cause.
#[test]
fn linux_prerequisites_group_failure_preserves_cause_and_redacts_wire() {
    let f = Fixture::new();
    let report = collect(
        f.sources(),
        1001,
        1001,
        Err(io::Error::other("private-group-diagnostic")),
    );
    assert!(report.missing_prerequisites.contains(&"identity:groups"));
    if let Observation::Unavailable { failure } = &report.identity {
        assert!(failure.errno.is_none());
        assert!(failure
            .source()
            .unwrap()
            .to_string()
            .contains("private-group-diagnostic"));
        assert!(!format!("{failure:?}").contains("private-group"));
    } else {
        panic!("group refusal became observed")
    }
    let raw = serde_json::to_string(&report).unwrap();
    assert!(!raw.contains("private-group"));
    assert!(!raw.contains("\"cause\""));
    let recovered = collect(f.sources(), 1001, 1001, Ok(vec![1001]));
    assert!(recovered.identity.value().is_some());
}
/// Invalid UTF-8 is an original read error, distinct from malformed parsed text.
#[test]
fn linux_prerequisites_invalid_utf8_retains_io_and_valid_recovery() {
    let f = Fixture::new();
    let s = f.sources();
    assert!(parsed(&s.uid_map, mappings).value().is_some());
    fs::write(&s.uid_map, [0xff]).unwrap();
    match parsed(&s.uid_map, mappings) {
        Observation::Unavailable { failure } => {
            assert_eq!(failure.cause.kind(), io::ErrorKind::InvalidData);
            assert!(failure.errno.is_none())
        }
        _ => panic!("invalid UTF8 became empty data"),
    }
    fs::write(&s.uid_map, "0 0 1\n").unwrap();
    assert!(parsed(&s.uid_map, mappings).value().is_some());
}
