//! Isolated actual filesystem/key witnesses; constants are fixture input only.
use super::*;
use ed25519_dalek::{Signer, Verifier};
use std::{
    fs,
    os::unix::fs::{symlink, PermissionsExt},
    sync::Arc,
};
use tempfile::TempDir;
/// Existing private root/file, independently pinned fixture seed. No live secret.
fn fixture() -> (TempDir, std::path::PathBuf, [u8; 32]) {
    let d = tempfile::tempdir().unwrap();
    fs::set_permissions(d.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let p = d.path().join("signing.seed");
    fs::write(&p, [73; 32]).unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o600)).unwrap();
    let k = SigningKey::from_bytes(&[73; 32]).verifying_key().to_bytes();
    (d, p, k)
}
/// Assert loaded actual signer, signature and redaction, without exporting seed.
fn valid(p: &Path, k: [u8; 32]) {
    let s = load(p, k).unwrap();
    assert_eq!(s.public_key(), k);
    s.key
        .verifying_key()
        .verify(b"fixture signing", &s.key.sign(b"fixture signing"))
        .unwrap();
    assert_eq!(format!("{s:?}"), "ProtectedSigner { <private key> }");
}
#[test]
fn protected_signer_actual_signature_snapshot_and_zeroize_contract() {
    let (_d, p, k) = fixture();
    valid(&p, k);
    let s = load(&p, k).unwrap();
    fs::write(&p, [19; 32]).unwrap();
    assert_eq!(s.public_key(), k);
    assert_eq!(load(&p, k).unwrap_err().code(), "binding");
    fn zeroize_drop<T: zeroize::ZeroizeOnDrop>() {}
    zeroize_drop::<SigningKey>();
    zeroize_drop::<Zeroizing<[u8; 32]>>();
    fs::write(&p, [73; 32]).unwrap();
    valid(&p, k);
}
#[test]
fn protected_signer_wrong_pin_and_invalid_path_refuse_before_recovery() {
    let (_d, p, k) = fixture();
    valid(&p, k);
    let other = SigningKey::from_bytes(&[19; 32]).verifying_key().to_bytes();
    let e = load(&p, other).unwrap_err();
    assert_eq!(e.code(), "binding");
    let text = format!("{e:?} {e}");
    assert!(!text.contains(p.to_str().unwrap()));
    assert!(!text.contains("73"));
    assert_eq!(
        load(Path::new("signing.seed"), k).unwrap_err().code(),
        "path"
    );
    let alias = p.parent().unwrap().join(".").join("signing.seed");
    assert_eq!(load(&alias, k).unwrap_err().code(), "path");
    assert!(load(&p, [255; 32]).is_err());
    valid(&p, k);
}
#[test]
fn protected_signer_exact_seed_representation_and_missing_recovery() {
    let (_d, p, k) = fixture();
    valid(&p, k);
    for raw in [vec![], vec![73; 31], vec![73; 33], vec![b'A'; 64]] {
        fs::write(&p, raw).unwrap();
        assert_eq!(load(&p, k).unwrap_err().code(), "encoding");
        fs::write(&p, [73; 32]).unwrap();
        valid(&p, k);
    }
    fs::remove_file(&p).unwrap();
    let e = load(&p, k).unwrap_err();
    assert_eq!(e.code(), "open");
    assert!(std::error::Error::source(&e).is_some());
    fs::write(&p, [73; 32]).unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o600)).unwrap();
    valid(&p, k);
}
#[test]
fn protected_signer_modes_links_and_nonregular_types_refuse_with_valid_control() {
    let (d, p, k) = fixture();
    valid(&p, k);
    for mode in [0o400, 0o640, 0o1600] {
        fs::set_permissions(&p, fs::Permissions::from_mode(mode)).unwrap();
        assert_eq!(load(&p, k).unwrap_err().code(), "path");
        fs::set_permissions(&p, fs::Permissions::from_mode(0o600)).unwrap();
        valid(&p, k);
    }
    fs::set_permissions(d.path(), fs::Permissions::from_mode(0o750)).unwrap();
    assert!(load(&p, k).is_err());
    fs::set_permissions(d.path(), fs::Permissions::from_mode(0o700)).unwrap();
    valid(&p, k);
    let link = d.path().join("other");
    fs::hard_link(&p, &link).unwrap();
    assert_eq!(load(&p, k).unwrap_err().code(), "path");
    fs::remove_file(&link).unwrap();
    valid(&p, k);
    fs::rename(&p, &link).unwrap();
    symlink(&link, &p).unwrap();
    assert_eq!(load(&p, k).unwrap_err().code(), "open");
    fs::remove_file(&p).unwrap();
    fs::create_dir(&p).unwrap();
    assert_eq!(load(&p, k).unwrap_err().code(), "path");
    fs::remove_dir(&p).unwrap();
    unix::mknodat(
        rustix::fs::CWD,
        &p,
        FileType::Fifo,
        Mode::from_raw_mode(0o600),
        0,
    )
    .unwrap();
    assert_eq!(load(&p, k).unwrap_err().code(), "path");
    fs::remove_file(&p).unwrap();
    fs::rename(&link, &p).unwrap();
    valid(&p, k);
}
#[test]
fn protected_signer_name_substitution_is_refused_and_exact_restore_recovers() {
    let (d, p, k) = fixture();
    valid(&p, k);
    let old = d.path().join("held-original");
    let path = p.clone();
    let saved = old.clone();
    let h = Hooks {
        after_read: Some(Arc::new(move || {
            fs::rename(&path, &saved).unwrap();
            fs::write(&path, [73; 32]).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        })),
        ..Hooks::default()
    };
    assert_eq!(load_inner(&p, k, &h).unwrap_err().code(), "path");
    fs::remove_file(&p).unwrap();
    fs::rename(old, &p).unwrap();
    valid(&p, k);
}
#[test]
fn protected_signer_read_stat_both_close_causes_and_recovery() {
    let (_d, p, k) = fixture();
    for mask in [1, 2, 4, 8, 1 | 4 | 8, 2 | 4 | 8] {
        valid(&p, k);
        let e = load_inner(
            &p,
            k,
            &Hooks {
                fail: mask,
                ..Hooks::default()
            },
        )
        .unwrap_err();
        let mut stages = vec![];
        /// Visit every primary/cleanup branch; source() alone would omit closes.
        fn visit(e: &Error, out: &mut Vec<Stage>) {
            match e {
                Error::Io { stage, .. } => out.push(*stage),
                Error::Joined { cause, cleanup } => {
                    visit(cause, out);
                    visit(cleanup, out)
                }
                _ => (),
            }
        }
        visit(&e, &mut stages);
        assert!(std::error::Error::source(&e).is_some());
        assert_eq!(stages.len(), mask.count_ones() as usize);
        if mask & 4 != 0 {
            assert!(stages.contains(&Stage::FileClose));
        }
        if mask & 8 != 0 {
            assert!(stages.contains(&Stage::DirectoryClose));
        }
        assert!(!format!("{e:?} {e}").contains(p.to_str().unwrap()));
        valid(&p, k);
    }
}
#[test]
fn protected_signer_uid_and_directory_symlink_checks_are_actual() {
    let (d, p, k) = fixture();
    valid(&p, k);
    let dir = swarm_enrollment_config::store::open_private_directory(
        d.path(),
        rustix::process::geteuid().as_raw(),
    )
    .unwrap();
    let f = fs::File::open(&p).unwrap();
    let uid = rustix::process::geteuid().as_raw();
    assert_eq!(
        checked_file(&dir, &f, p.file_name().unwrap(), uid.wrapping_add(1))
            .unwrap_err()
            .code(),
        "path"
    );
    // Both owned descriptors above are fixtures. Production load closes explicitly.
    drop(f);
    drop(dir);
    let parent = tempfile::tempdir().unwrap();
    let alias = parent.path().join("alias");
    symlink(d.path(), &alias).unwrap();
    assert!(load(&alias.join("signing.seed"), k).is_err());
    valid(&p, k);
}

#[test]
fn protected_signer_actual_partial_read_preserves_eof_and_recovers() {
    let (_d, p, k) = fixture();
    valid(&p, k);
    let path = p.clone();
    let e = load_inner(
        &p,
        k,
        &Hooks {
            before_read: Some(Arc::new(move || {
                fs::OpenOptions::new()
                    .write(true)
                    .open(&path)
                    .unwrap()
                    .set_len(16)
                    .unwrap();
            })),
            ..Hooks::default()
        },
    )
    .unwrap_err();
    match e {
        Error::Io {
            stage: Stage::Read,
            cause,
        } => assert_eq!(cause.kind(), std::io::ErrorKind::UnexpectedEof),
        _ => panic!("actual partial read cause lost"),
    }
    fs::write(&p, [73; 32]).unwrap();
    valid(&p, k);
}
