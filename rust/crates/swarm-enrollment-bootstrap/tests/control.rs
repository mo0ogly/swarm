#![cfg(target_os = "linux")]
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use swarm_enrollment_bootstrap::{
    decode_control, encode_control, load_control, ControlDocument, Document,
};
use swarm_enrollment_config::Pins;
use swarm_enrollment_state::Head;
use swarm_identity_contract::{Principal, Scope};
/// Build a private filesystem declaration, not a signed journal or human proof.
/// All storage belongs to a disposable fixture; setup errors panic.
fn fixture() -> (tempfile::TempDir, ControlDocument, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    for name in ["bootstrap", "configuration", "witness", "authority"] {
        let p = root.path().join(name);
        fs::create_dir(&p).unwrap();
        fs::set_permissions(p, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let journal = root.path().join("authority/journal.sqlite");
    fs::write(&journal, b"metadata fixture only").unwrap();
    fs::set_permissions(&journal, fs::Permissions::from_mode(0o600)).unwrap();
    let d = ControlDocument {
        schema_version: 2,
        installation: Document {
            schema_version: 1,
            pins: Pins {
                installation_digest: "ab".repeat(32),
                public_key: "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc".into(),
                principal: Principal {
                    id: "operator".into(),
                    kind: "human".into(),
                    issuer: "https://fixture.invalid".into(),
                    subject: "employee[eu]".into(),
                },
            },
            service_uid: rustix::process::geteuid().as_raw(),
            controller_uid: match rustix::process::geteuid().as_raw() {
                0 => 1,
                uid => !uid,
            },
            configuration_directory: root.path().join("configuration").to_str().unwrap().into(),
            witness_directory: root.path().join("witness").to_str().unwrap().into(),
        },
        journal_path: journal.to_str().unwrap().into(),
        project_context_digest: "0b".repeat(32),
        authority_head: Head {
            sequence: 1,
            digest: "cd".repeat(32),
        },
        enrollment_id: "initial-human".into(),
        generation: 1,
        scope: Scope {
            work_id: "work".into(),
            plan_version: 3,
            plan_digest: "de".repeat(32),
        },
        trust_configuration_revision: 2,
    };
    let path = root.path().join("bootstrap/control.json");
    (root, d, path)
}
/// Publish this disposable declaration with its explicit test permissions.
fn save(path: &std::path::Path, d: &ControlDocument) -> String {
    let raw = encode_control(d).unwrap();
    fs::write(path, &raw).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    format!("{:x}", Sha256::digest(raw))
}
#[test]
fn control_roundtrip_load_and_legacy_schema_stay_distinct_without_locks() {
    let (_root, d, path) = fixture();
    let pin = save(&path, &d);
    let before = fs::read(&d.journal_path).unwrap();
    assert_eq!(decode_control(&encode_control(&d).unwrap()).unwrap(), d);
    assert_eq!(load_control(path.to_str().unwrap(), &pin).unwrap(), d);
    assert!(swarm_enrollment_bootstrap::decode(&encode_control(&d).unwrap()).is_err());
    assert!(decode_control(&swarm_enrollment_bootstrap::encode(&d.installation).unwrap()).is_err());
    for p in [
        &d.installation.configuration_directory,
        &d.installation.witness_directory,
    ] {
        assert_eq!(fs::read_dir(p).unwrap().count(), 0);
    }
    assert_eq!(fs::read(&d.journal_path).unwrap(), before);
    assert_eq!(
        load_control(path.to_str().unwrap(), &"ef".repeat(32))
            .unwrap_err()
            .code(),
        "pin"
    );
    assert_eq!(load_control(path.to_str().unwrap(), &pin).unwrap(), d);
}
#[test]
fn control_canonical_fields_context_checkpoint_and_paths_refuse_then_recover() {
    let (_root, d, path) = fixture();
    let pin = save(&path, &d);
    assert!(load_control(path.to_str().unwrap(), &pin).is_ok());
    for variant in 0..10 {
        let mut x = d.clone();
        match variant {
            0 => x.schema_version = 1,
            1 => x.project_context_digest = "X".repeat(64),
            2 => x.authority_head = Head::default(),
            3 => x.generation = 0,
            4 => x.generation = 2,
            5 => x.scope.plan_version = 0,
            6 => x.trust_configuration_revision = 0,
            7 => x.enrollment_id = " bad ".into(),
            8 => x.journal_path = "relative.sqlite".into(),
            _ => {
                x.journal_path = x.installation.configuration_directory.clone() + "/journal.sqlite"
            }
        };
        assert!(encode_control(&x).is_err(), "variant {variant}");
    }
    let raw = String::from_utf8(encode_control(&d).unwrap()).unwrap();
    for altered in [
        format!(" {raw}"),
        format!("{raw}\n"),
        raw.replacen("{", "{\"schema_version\":2,", 1),
        raw.replacen("{", "{\"authenticated\":true,", 1),
        raw.replacen("enrollment_id", "Enrollment_id", 1),
    ] {
        assert!(decode_control(altered.as_bytes()).is_err());
    }
    assert_eq!(load_control(path.to_str().unwrap(), &pin).unwrap(), d);
}
#[test]
fn control_journal_permissions_links_types_and_overlapping_roots_refuse_without_mutation() {
    for variant in [
        "missing",
        "mode",
        "hardlink",
        "symlink",
        "parent-mode",
        "parent-symlink",
        "directory",
        "fifo",
        "nested-bootstrap",
    ] {
        let (_root, mut d, path) = fixture();
        let pin = save(&path, &d);
        assert!(load_control(path.to_str().unwrap(), &pin).is_ok());
        let journal = PathBuf::from(&d.journal_path);
        match variant {
            "missing" => fs::remove_file(&journal).unwrap(),
            "mode" => fs::set_permissions(&journal, fs::Permissions::from_mode(0o640)).unwrap(),
            "hardlink" => fs::hard_link(&journal, journal.with_extension("other")).unwrap(),
            "symlink" => {
                let target = journal.with_extension("target");
                fs::rename(&journal, &target).unwrap();
                std::os::unix::fs::symlink(target, &journal).unwrap();
            }
            "parent-mode" => {
                fs::set_permissions(journal.parent().unwrap(), fs::Permissions::from_mode(0o770))
                    .unwrap()
            }
            "parent-symlink" => {
                let parent = journal.parent().unwrap();
                let target = parent.with_extension("target");
                fs::rename(parent, &target).unwrap();
                std::os::unix::fs::symlink(target, parent).unwrap();
            }
            "directory" => {
                fs::remove_file(&journal).unwrap();
                fs::create_dir(&journal).unwrap();
            }
            "fifo" => {
                fs::remove_file(&journal).unwrap();
                assert!(std::process::Command::new("mkfifo")
                    .arg(&journal)
                    .status()
                    .unwrap()
                    .success());
            }
            _ => {
                d.journal_path = path
                    .parent()
                    .unwrap()
                    .join("journal.sqlite")
                    .to_str()
                    .unwrap()
                    .into();
                fs::copy(&journal, &d.journal_path).unwrap();
            }
        }
        let altered = if variant == "nested-bootstrap" {
            save(&path, &d)
        } else {
            pin
        };
        assert!(
            load_control(path.to_str().unwrap(), &altered).is_err(),
            "{variant}"
        );
        assert_eq!(
            fs::read_dir(&d.installation.configuration_directory)
                .unwrap()
                .count(),
            0
        );
        assert_eq!(
            fs::read_dir(&d.installation.witness_directory)
                .unwrap()
                .count(),
            0
        );
    }
}

#[cfg(feature = "test-fixtures")]
#[test]
fn control_read_and_independent_close_failures_keep_both_causes_and_recover() {
    use swarm_enrollment_bootstrap::{load_control_with_faults, FixtureFaults};
    let (_root, d, path) = fixture();
    let pin = save(&path, &d);
    assert!(load_control(path.to_str().unwrap(), &pin).is_ok());
    let error = load_control_with_faults(
        path.to_str().unwrap(),
        &pin,
        FixtureFaults {
            read: true,
            post_read_check: false,
            close: true,
        },
    )
    .unwrap_err();
    assert_eq!(error.code(), "storage");
    assert!(error.cleanup_error().is_some());
    assert!(std::error::Error::source(&error).is_some());
    assert!(format!("{error}").contains("cleanup"));
    assert_eq!(load_control(path.to_str().unwrap(), &pin).unwrap(), d);
}
