#![cfg(target_os = "linux")]
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt};
use swarm_enrollment_bootstrap::{decode, encode, load, Document};
use swarm_enrollment_config::Pins;
use swarm_identity_contract::Principal;
fn hash(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}
fn fixture() -> (tempfile::TempDir, Document, std::path::PathBuf) {
    let root = tempfile::TempDir::new().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    for name in ["bootstrap", "configuration", "witness"] {
        fs::create_dir(root.path().join(name)).unwrap();
        fs::set_permissions(root.path().join(name), fs::Permissions::from_mode(0o700)).unwrap();
    }
    let d = Document {
        schema_version: 1,
        pins: Pins {
            installation_digest: "a".repeat(64),
            public_key: "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc".into(),
            principal: Principal {
                id: "operator".into(),
                kind: "human".into(),
                issuer: "https://issuer.example".into(),
                subject: "fixture-human".into(),
            },
        },
        service_uid: rustix::process::geteuid().as_raw(),
        controller_uid: !rustix::process::geteuid().as_raw(),
        configuration_directory: root.path().join("configuration").to_str().unwrap().into(),
        witness_directory: root.path().join("witness").to_str().unwrap().into(),
    };
    let path = root.path().join("bootstrap/bootstrap.json");
    (root, d, path)
}
#[test]
fn bootstrap_readonly_roundtrip_binds_external_digest_without_creating_locks() {
    let (_root, d, path) = fixture();
    let raw = encode(&d).unwrap();
    assert_eq!(decode(&raw).unwrap(), d);
    fs::write(&path, &raw).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    for _ in 0..2 {
        assert_eq!(load(path.to_str().unwrap(), &hash(&raw)).unwrap(), d);
    }
    for directory in [&d.configuration_directory, &d.witness_directory] {
        assert_eq!(fs::read_dir(directory).unwrap().count(), 0);
    }
    assert_eq!(fs::read(&path).unwrap(), raw);
    for digest in [String::new(), "A".repeat(64), "b".repeat(64)] {
        assert_eq!(
            load(path.to_str().unwrap(), &digest).unwrap_err().code(),
            "pin"
        );
    }
    let mut replacement = d.clone();
    replacement.pins.principal.subject = "replacement".into();
    fs::write(&path, encode(&replacement).unwrap()).unwrap();
    assert_eq!(
        load(path.to_str().unwrap(), &hash(&raw))
            .unwrap_err()
            .code(),
        "pin"
    );
}
#[test]
fn bootstrap_canonical_document_refuses_invalid_paths_uid_pins_and_ambiguous_json() {
    let (_root, d, _) = fixture();
    let raw = encode(&d).unwrap();
    for variant in [
        "version",
        "pins",
        "principal",
        "same-uid",
        "relative",
        "root",
        "unclean",
        "nested",
        "same-path",
        "control",
    ] {
        let mut x = d.clone();
        match variant {
            "version" => x.schema_version = 2,
            "pins" => x.pins.public_key = "invalid".into(),
            "principal" => x.pins.principal.kind = "local".into(),
            "same-uid" => x.controller_uid = x.service_uid,
            "relative" => x.configuration_directory = "relative".into(),
            "root" => x.configuration_directory = "/".into(),
            "unclean" => x.configuration_directory += "/../configuration",
            "nested" => x.witness_directory = x.configuration_directory.clone() + "/child",
            "same-path" => x.witness_directory = x.configuration_directory.clone(),
            "control" => x.witness_directory.push('\n'),
            _ => unreachable!(),
        }
        assert_eq!(encode(&x).unwrap_err().code(), "document", "{variant}");
        assert_eq!(
            decode(&serde_json::to_vec(&x).unwrap()).unwrap_err().code(),
            "document"
        );
    }
    let s = String::from_utf8(raw).unwrap();
    for input in [
        "null".into(),
        format!(" {s}"),
        format!("{s}\n"),
        s.replacen("{", "{\"schema_version\":1,", 1),
        s.replacen("service_uid", "Service_uid", 1),
        s.replacen("{", "{\"approved\":true,", 1),
        s.replacen(
            "\"principal_kind\":\"human\"",
            "\"principal_kind\":\"human\",\"admin\":true",
            1,
        ),
        s.replacen(
            &format!("\"controller_uid\":{}", d.controller_uid),
            "\"controller_uid\":null",
            1,
        ),
    ] {
        assert_eq!(decode(input.as_bytes()).unwrap_err().code(), "document");
    }
    let e = decode(br#"{"schema_version":"private-secret-value"}"#).unwrap_err();
    assert!(!format!("{e:?} {e}").contains("private-secret-value"));
    assert!(std::error::Error::source(&e).is_some());
    let mut x = d.clone();
    x.configuration_directory = "/safe/prefix".into();
    x.witness_directory = "/safe/prefix-other".into();
    assert!(encode(&x).is_ok());
}
#[test]
fn bootstrap_load_rejects_unsafe_files_and_directories_without_writing() {
    use std::os::unix::fs::symlink;
    for variant in [
        "missing",
        "symlink",
        "hardlink",
        "mode",
        "directory-mode",
        "directory-symlink",
        "service-uid",
        "config-mode",
        "witness-mode",
        "config-symlink",
        "bootstrap-nested",
        "noncanonical",
        "directory-as-file",
        "relative",
        "fifo",
    ] {
        let (_root, mut d, mut path) = fixture();
        let mut raw = encode(&d).unwrap();
        fs::write(&path, &raw).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(load(path.to_str().unwrap(), &hash(&raw)).unwrap(), d); // same preconditions, valid control
        let expected = if variant == "noncanonical" {
            "document"
        } else {
            "path"
        };
        match variant {
            "missing" => fs::remove_file(&path).unwrap(),
            "symlink" => {
                let link = path.with_extension("link");
                symlink(&path, &link).unwrap();
                path = link;
            }
            "hardlink" => fs::hard_link(&path, path.with_extension("hard")).unwrap(),
            "mode" => fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap(),
            "directory-mode" => {
                fs::set_permissions(path.parent().unwrap(), fs::Permissions::from_mode(0o755))
                    .unwrap()
            }
            "directory-symlink" => {
                let link = path.parent().unwrap().with_extension("link");
                symlink(path.parent().unwrap(), &link).unwrap();
                path = link.join("bootstrap.json");
            }
            "service-uid" => {
                std::mem::swap(&mut d.service_uid, &mut d.controller_uid);
                raw = encode(&d).unwrap();
                fs::write(&path, &raw).unwrap();
            }
            "config-mode" => fs::set_permissions(
                &d.configuration_directory,
                fs::Permissions::from_mode(0o755),
            )
            .unwrap(),
            "witness-mode" => {
                fs::set_permissions(&d.witness_directory, fs::Permissions::from_mode(0o755))
                    .unwrap()
            }
            "config-symlink" => {
                let link = d.configuration_directory.clone() + "-link";
                symlink(&d.configuration_directory, &link).unwrap();
                d.configuration_directory = link;
                raw = encode(&d).unwrap();
                fs::write(&path, &raw).unwrap();
            }
            "bootstrap-nested" => {
                d.configuration_directory = path.parent().unwrap().to_str().unwrap().into();
                raw = encode(&d).unwrap();
                fs::write(&path, &raw).unwrap();
            }
            "noncanonical" => {
                raw.push(b'\n');
                fs::write(&path, &raw).unwrap();
            }
            "directory-as-file" => path = path.parent().unwrap().to_path_buf(),
            "relative" => path = "relative/bootstrap.json".into(),
            "fifo" => {
                fs::remove_file(&path).unwrap();
                rustix::fs::mknodat(
                    rustix::fs::CWD,
                    &path,
                    rustix::fs::FileType::Fifo,
                    rustix::fs::Mode::from_raw_mode(0o600),
                    0,
                )
                .unwrap();
            }
            _ => unreachable!(),
        }
        assert_eq!(
            load(path.to_str().unwrap(), &hash(&raw))
                .unwrap_err()
                .code(),
            expected,
            "{variant}"
        );
        assert_eq!(fs::read_dir(&d.witness_directory).unwrap().count(), 0);
    }
}
#[test]
fn bootstrap_recovery_restores_preconditions_and_pin_validation_has_priority() {
    let (_root, d, path) = fixture();
    let raw = encode(&d).unwrap();
    fs::write(&path, &raw).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let pin = hash(&raw);
    assert_eq!(load(path.to_str().unwrap(), &pin).unwrap(), d);
    assert_eq!(load("relative", "").unwrap_err().code(), "pin");
    fs::set_permissions(&d.witness_directory, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        load(path.to_str().unwrap(), &pin).unwrap_err().code(),
        "path"
    );
    fs::set_permissions(&d.witness_directory, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(load(path.to_str().unwrap(), &pin).unwrap(), d);
}
#[cfg(feature = "test-fixtures")]
#[test]
fn bootstrap_read_postcheck_and_all_close_failures_preserve_causes_and_refuse_data() {
    use swarm_enrollment_bootstrap::{load_with_faults, FixtureFaults};
    let (_root, d, path) = fixture();
    let raw = encode(&d).unwrap();
    fs::write(&path, &raw).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let pin = hash(&raw);
    for fault in [
        FixtureFaults {
            read: true,
            ..Default::default()
        },
        FixtureFaults {
            post_read_check: true,
            ..Default::default()
        },
        FixtureFaults {
            close: true,
            ..Default::default()
        },
        FixtureFaults {
            read: true,
            close: true,
            ..Default::default()
        },
    ] {
        assert_eq!(load(path.to_str().unwrap(), &pin).unwrap(), d);
        let e = load_with_faults(path.to_str().unwrap(), &pin, fault).unwrap_err();
        assert!(std::error::Error::source(&e).is_some());
        if fault.close {
            assert!(e.cleanup_error().is_some());
        }
        assert_eq!(fs::read(&path).unwrap(), raw);
        assert_eq!(load(path.to_str().unwrap(), &pin).unwrap(), d);
    }
    let e = load_with_faults(
        path.to_str().unwrap(),
        &"b".repeat(64),
        FixtureFaults {
            close: true,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(e.code(), "pin");
    assert!(e.contains_code("storage"));
    assert!(e.cleanup_error().is_some());
}
