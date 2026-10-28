//! Synthetic byte streams and actual private file substitutions, never live keys.
use super::*;
use std::{cell::Cell, fs, io::Cursor, os::unix::fs::PermissionsExt};
/// Unchanged valid witness before every injected refusal.
fn valid() {
    let value = bytes(&mut Cursor::new(b"private"), 7, &|| Ok(())).unwrap();
    assert_eq!(value.as_bytes(), b"private");
    assert_eq!(format!("{value:?}"), "PrivateBytes { <private> }");
}
struct Partial {
    rest: Cursor<Vec<u8>>,
    interrupted: bool,
}
impl Read for Partial {
    /// Interrupt once then read at most two bytes, using the supplied buffer only.
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if !self.interrupted {
            self.interrupted = true;
            return Err(io::ErrorKind::Interrupted.into());
        }
        let n = out.len().min(2);
        self.rest.read(&mut out[..n])
    }
}
/// Partial reads/EINTR preserve exact bytes including NUL; empty is real EOF.
#[test]
fn guarded_read_partial_interrupted_exact_empty_and_redaction() {
    valid();
    let mut source = Partial {
        rest: Cursor::new(b"private\0\n".to_vec()),
        interrupted: false,
    };
    let got = bytes(&mut source, 9, &|| Ok(())).unwrap();
    assert_eq!(got.as_bytes(), b"private\0\n");
    assert!(got.0.capacity() >= 10);
    assert_eq!(
        bytes(&mut Cursor::new([]), 0, &|| Ok(()))
            .unwrap()
            .as_bytes(),
        b""
    );
}
/// Growth, truncation and actual representation/allocation overflow refuse.
#[test]
fn guarded_read_exact_eof_growth_shrink_and_representation_errors() {
    for (raw, size) in [(b"short".as_slice(), 6), (b"long".as_slice(), 3)] {
        valid();
        assert_eq!(
            bytes(&mut Cursor::new(raw), size, &|| Ok(()))
                .unwrap_err()
                .code(),
            "conflict"
        );
        valid();
    }
    valid();
    assert_eq!(
        bytes(&mut Cursor::new([]), usize::MAX, &|| Ok(()))
            .unwrap_err()
            .code(),
        "path"
    );
    let e = bytes(&mut Cursor::new([]), usize::MAX - 1, &|| Ok(())).unwrap_err();
    assert_eq!(e.code(), "storage");
    assert!(std::error::Error::source(&e).unwrap().is::<io::Error>());
    valid();
}
struct Failed {
    read: bool,
}
impl Read for Failed {
    /// Return two actual bytes before preserving the original private IO cause.
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.read {
            Err(io::Error::new(io::ErrorKind::Other, "PRIVATE_READ_CAUSE"))
        } else {
            self.read = true;
            out[..2].copy_from_slice(b"ab");
            Ok(2)
        }
    }
}
/// Partial failure retains typed IO but ordinary errors never print its text.
#[test]
fn guarded_read_partial_io_original_cause_redacted_and_recovery() {
    valid();
    let e = bytes(&mut Failed { read: false }, 7, &|| Ok(())).unwrap_err();
    assert_eq!(e.code(), "storage");
    assert!(!format!("{e:?} {e}").contains("PRIVATE_READ_CAUSE"));
    let cause = std::error::Error::source(&e)
        .unwrap()
        .downcast_ref::<io::Error>()
        .unwrap();
    assert_eq!(cause.kind(), io::ErrorKind::Other);
    valid();
}
/// Cancellation after an actual partial read refuses the guarded result.
#[test]
fn guarded_read_cancellation_after_read_refuses_then_fresh_check_recovers() {
    valid();
    let checks = Cell::new(0);
    let e = bytes(
        &mut Partial {
            rest: Cursor::new(b"private".to_vec()),
            interrupted: true,
        },
        7,
        &|| {
            checks.set(checks.get() + 1);
            if checks.get() == 3 {
                Err(Error::Canceled)
            } else {
                Ok(())
            }
        },
    )
    .unwrap_err();
    assert_eq!(e.code(), "canceled");
    assert_eq!(checks.get(), 3);
    valid();
}
/// Owned real private inode and its held directory; no production files involved.
fn physical() -> (tempfile::TempDir, File) {
    let d = tempfile::tempdir().unwrap();
    fs::set_permissions(d.path(), fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(d.path().join("secret"), b"private").unwrap();
    fs::set_permissions(d.path().join("secret"), fs::Permissions::from_mode(0o600)).unwrap();
    let dir = open_private_directory(d.path(), rustix::process::geteuid().as_raw()).unwrap();
    (d, dir)
}
/// Valid named real read before each physical mutation.
fn physical_valid(dir: &File) {
    assert_eq!(
        read(
            dir,
            rustix::process::geteuid().as_raw(),
            "secret",
            &|| Ok(()),
            &|_, _| Ok(())
        )
        .unwrap()
        .as_bytes(),
        b"private"
    );
}
/// Replacement before/after read cannot make an old descriptor represent a new name.
#[test]
fn guarded_read_actual_named_substitution_before_and_after_read_refuses() {
    for before in [true, false] {
        let (d, dir) = physical();
        physical_valid(&dir);
        let replace = || {
            fs::rename(d.path().join("secret"), d.path().join("old")).unwrap();
            fs::write(d.path().join("secret"), b"replace").unwrap();
            fs::set_permissions(d.path().join("secret"), fs::Permissions::from_mode(0o600))
                .unwrap();
            Ok(())
        };
        let noop = || Ok(());
        let e = if before {
            read_inner(
                &dir,
                rustix::process::geteuid().as_raw(),
                "secret",
                &noop,
                &|_, _| Ok(()),
                &replace,
                &noop,
            )
        } else {
            read_inner(
                &dir,
                rustix::process::geteuid().as_raw(),
                "secret",
                &noop,
                &|_, _| Ok(()),
                &noop,
                &replace,
            )
        }
        .unwrap_err();
        assert_eq!(e.code(), "path");
        fs::remove_file(d.path().join("secret")).unwrap();
        fs::rename(d.path().join("old"), d.path().join("secret")).unwrap();
        physical_valid(&dir);
        close(dir, IoClass::Path).unwrap();
    }
}
/// Real growth/shrink and deterministic mtime change are detected without sleeps.
#[test]
fn guarded_read_actual_growth_shrink_and_metadata_change_refuse() {
    use std::io::Write;
    for variant in 0..3 {
        let (d, dir) = physical();
        physical_valid(&dir);
        let before = || {
            match variant {
                0 => fs::OpenOptions::new()
                    .append(true)
                    .open(d.path().join("secret"))
                    .unwrap()
                    .write_all(b"x")
                    .unwrap(),
                1 => fs::write(d.path().join("secret"), b"").unwrap(),
                _ => (),
            };
            Ok(())
        };
        let after = || {
            if variant == 2 {
                fs::File::open(d.path().join("secret"))
                    .unwrap()
                    .set_modified(std::time::SystemTime::UNIX_EPOCH)
                    .unwrap()
            };
            Ok(())
        };
        let e = read_inner(
            &dir,
            rustix::process::geteuid().as_raw(),
            "secret",
            &|| Ok(()),
            &|_, _| Ok(()),
            &before,
            &after,
        )
        .unwrap_err();
        assert_eq!(e.code(), "conflict");
        fs::write(d.path().join("secret"), b"private").unwrap();
        physical_valid(&dir);
        close(dir, IoClass::Path).unwrap();
    }
}
/// A primary refusal and independent post-close error both remain structured.
#[test]
fn guarded_read_primary_and_actual_close_hook_causes_both_survive() {
    let (_d, dir) = physical();
    physical_valid(&dir);
    let e = read_inner(
        &dir,
        rustix::process::geteuid().as_raw(),
        "secret",
        &|| Ok(()),
        &|_, _| Err(Error::io(IoClass::Storage, io::Error::from_raw_os_error(5))),
        &|| Err(Error::Conflict),
        &|| Ok(()),
    )
    .unwrap_err();
    match e {
        Error::Joined { cause, cleanup } => {
            assert_eq!(cause.code(), "conflict");
            assert_eq!(cleanup.code(), "storage")
        }
        other => panic!("expected joined: {other}"),
    };
    physical_valid(&dir);
    close(dir, IoClass::Path).unwrap();
}

/// Cancel after the actual file fsync, retain that acknowledgement and an
/// independent post-close error; no check-count threshold or sleep is used.
#[test]
fn guarded_durable_read_cancel_after_file_ack_and_close_cause_recover() {
    let (_d, dir) = physical();
    physical_valid(&dir);
    let synced = Cell::new(false);
    let pin = format!("{:x}", Sha256::digest(b"private"));
    let e = read_durable(
        &dir,
        rustix::process::geteuid().as_raw(),
        "secret",
        &pin,
        &|| {
            if synced.get() {
                Err(Error::Canceled)
            } else {
                Ok(())
            }
        },
        &|stage, _| {
            match stage {
                Stage::FileSync => synced.set(true),
                Stage::ReadClose => {
                    return Err(Error::io(IoClass::Storage, io::Error::from_raw_os_error(5)))
                }
                _ => (),
            };
            Ok(())
        },
    )
    .unwrap_err();
    assert_eq!(e.validation, ReadValidation::PinMatched);
    assert_eq!(e.durability, ReadDurability::FileAcknowledged);
    assert_eq!(e.cause.code(), "canceled");
    assert!(e.cause.cleanup_error().is_some());
    assert!(
        read_durable(
            &dir,
            rustix::process::geteuid().as_raw(),
            "secret",
            &pin,
            &|| Ok(()),
            &|_, _| Ok(())
        )
        .unwrap()
        .as_bytes()
            == b"private"
    );
    close(dir, IoClass::Path).unwrap();
}
