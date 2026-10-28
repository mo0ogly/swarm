use swarm_enrollment_bootstrap::{
    decode_authenticated, encode_authenticated, AuthenticatedDocument, ControlDocument, Document,
    PersonalDeclaration,
};
use swarm_enrollment_config::Pins;
use swarm_enrollment_state::Head;
use swarm_identity_contract::{Principal, Scope};
/// Pure declared data, no filesystem, key custody, grant or actual human account.
/// Generation7 versus business1 is intentional; all paths are synthetic inputs.
fn document() -> AuthenticatedDocument {
    AuthenticatedDocument {
        schema_version: 3,
        control: ControlDocument {
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
                service_uid: 1,
                controller_uid: 2,
                configuration_directory: "/fixture/configuration".into(),
                witness_directory: "/fixture/witness".into(),
            },
            journal_path: "/fixture/business/journal.sqlite".into(),
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
        },
        personal: PersonalDeclaration {
            authority_public_key: "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc".into(),
            project_context_digest: "33".repeat(32),
            journal_path: "/fixture/personal/journal.sqlite".into(),
            checkpoint_directory: "/fixture/personal-witness".into(),
            generation: 7,
            bootstrap_head: Head {
                sequence: 1,
                digest: "ef".repeat(32),
            },
        },
    }
}
#[test]
fn authenticated_declaration_exact_roundtrip_keeps_versions_and_generations_distinct() {
    let d = document();
    let raw = encode_authenticated(&d).unwrap();
    assert_eq!(decode_authenticated(&raw).unwrap(), d);
    assert!(swarm_enrollment_bootstrap::decode_control(&raw).is_err());
    assert!(
        decode_authenticated(&swarm_enrollment_bootstrap::encode_control(&d.control).unwrap())
            .is_err()
    );
    assert_eq!(d.personal.generation, 7);
    assert_eq!(d.control.generation, 1);
}
#[test]
fn authenticated_declaration_invalid_domains_heads_keys_and_overlapping_roots_refuse() {
    let d = document();
    assert!(encode_authenticated(&d).is_ok());
    for variant in 0..15 {
        let mut bad = d.clone();
        match variant {
            0 => bad.schema_version = 2,
            1 => bad.personal.authority_public_key = "!".repeat(43),
            2 => bad.personal.project_context_digest = "X".repeat(64),
            3 => bad.personal.generation = 0,
            4 => bad.personal.bootstrap_head = Head::default(),
            5 => bad.personal.bootstrap_head.digest = "bad".into(),
            6 => bad.personal.journal_path = "relative/journal.sqlite".into(),
            7 => bad.personal.checkpoint_directory = "/fixture/../escape".into(),
            8 => {
                bad.personal.checkpoint_directory =
                    bad.control.installation.configuration_directory.clone()
            }
            9 => bad.personal.journal_path = bad.control.journal_path.clone(),
            10 => bad.personal.checkpoint_directory = "/fixture".into(),
            11 => bad.personal.journal_path = "/fixture/witness/account.sqlite".into(),
            12 => bad.personal.checkpoint_directory = "/fixture/personal/checkpoint".into(),
            13 => bad.personal.project_context_digest = bad.control.project_context_digest.clone(),
            _ => bad.personal.journal_path = "/fixture/personal-witness/account.sqlite".into(),
        }
        assert!(encode_authenticated(&bad).is_err(), "variant{variant}");
    }
    assert!(encode_authenticated(&d).is_ok());
}
#[test]
fn authenticated_declaration_wire_unknown_duplicate_null_and_noncanonical_refuse() {
    let raw = String::from_utf8(encode_authenticated(&document()).unwrap()).unwrap();
    for changed in [
        format!(" {raw}"),
        format!("{raw}\n"),
        raw.replacen("{", "{\"schema_version\":3,", 1),
        raw.replacen("{", "{\"actor\":\"model\",", 1),
        raw.replace("\"generation\":7", "\"generation\":null"),
        raw.replace("\"personal\":{", "\"personal\":{\"allow\":true,"),
    ] {
        assert_ne!(changed, raw);
        assert!(decode_authenticated(changed.as_bytes()).is_err());
    }
    assert!(decode_authenticated(raw.as_bytes()).is_ok());
}

#[cfg(target_os = "linux")]
mod loading {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::{fs, os::unix::fs::PermissionsExt, path::Path};
    use swarm_enrollment_bootstrap::load_authenticated;
    /// Six actual private fixture directories, two opaque metadata-only files.
    /// No SQLite, grant, key custody or live daemon; pin is fixture-selected only.
    fn fixture() -> (tempfile::TempDir, AuthenticatedDocument, std::path::PathBuf) {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        for name in [
            "bootstrap",
            "configuration",
            "witness",
            "business",
            "personal",
            "personal-witness",
        ] {
            let p = root.path().join(name);
            fs::create_dir(&p).unwrap();
            fs::set_permissions(p, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let mut d = document();
        let prefix = root.path().to_str().unwrap();
        for path in [
            &mut d.control.installation.configuration_directory,
            &mut d.control.installation.witness_directory,
            &mut d.control.journal_path,
            &mut d.personal.journal_path,
            &mut d.personal.checkpoint_directory,
        ] {
            *path = path.replacen("/fixture", prefix, 1)
        }
        d.control.installation.service_uid = rustix::process::geteuid().as_raw();
        d.control.installation.controller_uid = if d.control.installation.service_uid == 0 {
            1
        } else {
            !d.control.installation.service_uid
        };
        for name in [&d.control.journal_path, &d.personal.journal_path] {
            fs::write(name, b"opaque metadata fixture").unwrap();
            fs::set_permissions(name, fs::Permissions::from_mode(0o600)).unwrap();
        }
        let path = root.path().join("bootstrap/authenticated.json");
        (root, d, path)
    }
    /// Save only fixture data with600permissions, returning its explicit hash pin.
    fn save(path: &Path, d: &AuthenticatedDocument) -> String {
        let raw = encode_authenticated(d).unwrap();
        fs::write(path, &raw).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
        format!("{:x}", Sha256::digest(raw))
    }
    #[test]
    fn authenticated_loader_observes_all_private_roots_without_sqlite_locks_or_mutation() {
        let (_root, d, path) = fixture();
        let pin = save(&path, &d);
        let before = [
            fs::read(&d.control.journal_path).unwrap(),
            fs::read(&d.personal.journal_path).unwrap(),
        ];
        assert_eq!(load_authenticated(path.to_str().unwrap(), &pin).unwrap(), d);
        for directory in [
            &d.control.installation.configuration_directory,
            &d.control.installation.witness_directory,
            &d.personal.checkpoint_directory,
        ] {
            assert_eq!(fs::read_dir(directory).unwrap().count(), 0)
        }
        assert_eq!(fs::read(&d.control.journal_path).unwrap(), before[0]);
        assert_eq!(fs::read(&d.personal.journal_path).unwrap(), before[1]);
        assert_eq!(
            load_authenticated(path.to_str().unwrap(), &"ef".repeat(32))
                .unwrap_err()
                .code(),
            "pin"
        );
        assert!(load_authenticated(path.to_str().unwrap(), &pin).is_ok());
    }
    #[test]
    fn authenticated_loader_both_journals_and_personal_root_links_modes_types_refuse_then_recover()
    {
        use std::os::unix::fs::symlink;
        let (root, d, path) = fixture();
        let pin = save(&path, &d);
        for name in [&d.control.journal_path, &d.personal.journal_path] {
            assert!(load_authenticated(path.to_str().unwrap(), &pin).is_ok());
            fs::set_permissions(name, fs::Permissions::from_mode(0o640)).unwrap();
            assert_eq!(
                load_authenticated(path.to_str().unwrap(), &pin)
                    .unwrap_err()
                    .code(),
                "path"
            );
            fs::set_permissions(name, fs::Permissions::from_mode(0o600)).unwrap();
            assert!(load_authenticated(path.to_str().unwrap(), &pin).is_ok());
            let alias = root.path().join("hard");
            fs::hard_link(name, &alias).unwrap();
            assert!(load_authenticated(path.to_str().unwrap(), &pin).is_err());
            fs::remove_file(alias).unwrap();
        }
        assert!(load_authenticated(path.to_str().unwrap(), &pin).is_ok());
        let original = root.path().join("original-journal");
        fs::rename(&d.personal.journal_path, &original).unwrap();
        symlink(&original, &d.personal.journal_path).unwrap();
        assert!(load_authenticated(path.to_str().unwrap(), &pin).is_err());
        fs::remove_file(&d.personal.journal_path).unwrap();
        fs::rename(&original, &d.personal.journal_path).unwrap();
        assert!(load_authenticated(path.to_str().unwrap(), &pin).is_ok());
        let personal_root = Path::new(&d.personal.journal_path).parent().unwrap();
        let retained = root.path().join("retained-personal");
        fs::rename(personal_root, &retained).unwrap();
        symlink(&retained, personal_root).unwrap();
        assert!(load_authenticated(path.to_str().unwrap(), &pin).is_err());
        fs::remove_file(personal_root).unwrap();
        fs::rename(retained, personal_root).unwrap();
        assert_eq!(load_authenticated(path.to_str().unwrap(), &pin).unwrap(), d);
    }
    #[cfg(feature = "test-fixtures")]
    #[test]
    fn authenticated_loader_read_stat_close_causes_remain_and_valid_recovery_creates_no_locks() {
        use swarm_enrollment_bootstrap::{load_authenticated_with_faults, FixtureFaults};
        let (_root, d, path) = fixture();
        let pin = save(&path, &d);
        for faults in [
            FixtureFaults {
                read: true,
                post_read_check: false,
                close: false,
            },
            FixtureFaults {
                read: false,
                post_read_check: true,
                close: false,
            },
            FixtureFaults {
                read: false,
                post_read_check: false,
                close: true,
            },
            FixtureFaults {
                read: true,
                post_read_check: false,
                close: true,
            },
        ] {
            assert!(load_authenticated(path.to_str().unwrap(), &pin).is_ok());
            let e =
                load_authenticated_with_faults(path.to_str().unwrap(), &pin, faults).unwrap_err();
            assert!(std::error::Error::source(&e).is_some());
            if faults.read && faults.close {
                assert!(e.cleanup_error().is_some())
            }
            assert_eq!(load_authenticated(path.to_str().unwrap(), &pin).unwrap(), d);
        }
        for directory in [
            &d.control.installation.configuration_directory,
            &d.control.installation.witness_directory,
            &d.personal.checkpoint_directory,
        ] {
            assert_eq!(fs::read_dir(directory).unwrap().count(), 0)
        }
    }
}
