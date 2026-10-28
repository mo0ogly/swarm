#![cfg(target_os = "linux")]
//! Public session persistence checks on private, isolated Roots and real bound IPv4 listeners.
use std::{
    fs,
    io::Write,
    net::TcpListener,
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::PathBuf,
};
use swarm_engine::{RootStore, SystemEnvironment};
struct Fixture {
    path: PathBuf,
}
impl Fixture {
    /// Initialize real schema/settings; no fixture database or account is reused.
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(swarm_engine::generate_identifier("web-session91-").unwrap());
        fs::create_dir(&path).unwrap();
        RootStore::initialize(&path, &mut SystemEnvironment)
            .unwrap()
            .close()
            .unwrap();
        Self { path }
    }
    /// Observe the Go-compatible session namespace from the actual bound address.
    fn session_path(&self, listener: &TcpListener) -> PathBuf {
        use sha2::{Digest, Sha256};
        let h = format!(
            "{:x}",
            Sha256::digest(listener.local_addr().unwrap().to_string().as_bytes())
        );
        self.path
            .join(".swarm")
            .join(format!("web-session-{}", &h[..16]))
    }
    /// Remove only this owned successful fixture; retain failed roots as evidence.
    fn finish(self) {
        fs::remove_dir_all(self.path).unwrap();
    }
}
/// New bytes are private, durable-publication compatible and reused without a replacement.
#[test]
fn session_creates_reuses_and_scopes_to_observed_listener() {
    let f = Fixture::new();
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let r = RootStore::open(&f.path, true).unwrap();
    let a = r.web_session(&l).unwrap();
    assert!(a.was_created());
    assert_eq!(a.address(), l.local_addr().unwrap());
    let path = f.session_path(&l);
    let first = fs::metadata(&path).unwrap();
    assert_eq!(first.mode() & 0o777, 0o600);
    assert_eq!(first.nlink(), 1);
    assert_eq!(fs::read_to_string(&path).unwrap(), a.secret());
    assert_eq!(a.secret().len(), 32);
    assert!(!format!("{a:?}").contains(a.secret()));
    let b = r.web_session(&l).unwrap();
    assert!(!b.was_created());
    assert_eq!(a.secret(), b.secret());
    assert_eq!(fs::metadata(&path).unwrap().ino(), first.ino());
    let other = TcpListener::bind("127.0.0.1:0").unwrap();
    let c = r.web_session(&other).unwrap();
    assert!(c.was_created());
    assert_ne!(a.address(), c.address());
    assert_ne!(a.secret(), c.secret());
    let broad = TcpListener::bind("0.0.0.0:0").unwrap();
    let count = fs::read_dir(f.path.join(".swarm")).unwrap().count();
    assert_eq!(
        r.web_session(&broad).unwrap_err().code(),
        "web-listener-loopback-required"
    );
    assert_eq!(fs::read_dir(f.path.join(".swarm")).unwrap().count(), count);
    assert_eq!(r.list_works().unwrap().len(), 0);
    r.close().unwrap();
    f.finish();
}
/// Existing compatible Go token is adopted exactly; no generator overwrites it.
#[test]
fn session_reuses_go_alphanumeric_credential_without_rewriting() {
    let f = Fixture::new();
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let path = f.session_path(&l);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .unwrap();
    let token = "session-ABCDefgh_IJKLmnop-012345";
    assert_eq!(token.len(), 32);
    file.write_all(token.as_bytes()).unwrap();
    file.sync_all().unwrap();
    drop(file);
    let r = RootStore::open(&f.path, true).unwrap();
    let s = r.web_session(&l).unwrap();
    assert!(!s.was_created());
    assert_eq!(s.secret(), token);
    r.close().unwrap();
    f.finish();
}
/// Every invalid object has a valid witness first; refusal never repairs or replaces it.
#[test]
fn session_invalid_bytes_permissions_and_links_refuse_without_replacement_then_recover() {
    let f = Fixture::new();
    let r = RootStore::open(&f.path, true).unwrap();
    for fault in [
        "empty", "short", "oversize", "newline", "utf8", "public", "symlink", "hardlink", "fifo",
    ] {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let s = r.web_session(&l).unwrap();
        let path = f.session_path(&l);
        let original = fs::read(&path).unwrap();
        assert_eq!(original, s.secret().as_bytes());
        match fault {
            "empty" => fs::write(&path, b"").unwrap(),
            "short" => fs::write(&path, b"session-123").unwrap(),
            "oversize" => fs::write(&path, vec![b'x'; 33]).unwrap(),
            "newline" => fs::write(&path, b"session-0123456789abcdef0123456\n").unwrap(),
            "utf8" => fs::write(&path, vec![255; 32]).unwrap(),
            "public" => fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap(),
            "symlink" => {
                fs::remove_file(&path).unwrap();
                std::os::unix::fs::symlink("missing-target", &path).unwrap();
            }
            "hardlink" => fs::hard_link(&path, path.with_extension("link")).unwrap(),
            "fifo" => {
                fs::remove_file(&path).unwrap();
                rustix::fs::mknodat(
                    rustix::fs::CWD,
                    &path,
                    rustix::fs::FileType::Fifo,
                    rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
                    0,
                )
                .unwrap();
            }
            _ => unreachable!(),
        }
        let before = fs::symlink_metadata(&path).unwrap();
        let bytes = if before.is_file() {
            Some(fs::read(&path).unwrap())
        } else {
            None
        };
        let error = r.web_session(&l).unwrap_err();
        assert!(!format!("{error:?}").contains(s.secret()));
        assert_eq!(
            fs::symlink_metadata(&path).unwrap().ino(),
            before.ino(),
            "{fault}"
        );
        if let Some(bytes) = bytes {
            assert_eq!(fs::read(&path).unwrap(), bytes, "{fault}");
        }
        fs::remove_file(&path).unwrap();
        if fault == "hardlink" {
            fs::remove_file(path.with_extension("link")).unwrap();
        }
        let mut out = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .unwrap();
        out.write_all(&original).unwrap();
        out.sync_all().unwrap();
        drop(out);
        assert_eq!(r.web_session(&l).unwrap().secret(), s.secret(), "{fault}");
    }
    assert!(r.list_works().unwrap().is_empty());
    r.close().unwrap();
    f.finish();
}
/// A readonly Root refuses even creation; no session/Work side effect follows.
#[test]
fn session_readonly_root_refuses_without_creating_a_file() {
    let f = Fixture::new();
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let path = f.session_path(&l);
    let r = RootStore::open(&f.path, true).unwrap();
    assert!(r.web_session(&l).unwrap().was_created());
    r.close().unwrap();
    fs::remove_file(&path).unwrap();
    let r = RootStore::open(&f.path, false).unwrap();
    assert_eq!(
        r.web_session(&l).unwrap_err().code(),
        "web-session-readonly"
    );
    assert!(!path.exists());
    assert!(r.list_works().unwrap().is_empty());
    r.close().unwrap();
    f.finish();
}
/// Parallel independent opens converge on complete winning bytes; no attempt loop or sleep.
#[test]
fn session_concurrent_creators_share_one_private_complete_credential() {
    let f = Fixture::new();
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let barrier = std::sync::Barrier::new(4);
    let values = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let f = &f;
                let l = &l;
                let barrier = &barrier;
                scope.spawn(move || {
                    let r = RootStore::open(&f.path, true).unwrap();
                    barrier.wait();
                    let s = r.web_session(l).unwrap();
                    let result = (s.secret().to_owned(), s.was_created());
                    r.close().unwrap();
                    result
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(values.iter().filter(|(_, new)| *new).count(), 1);
    assert!(values.iter().all(|(s, _)| s == &values[0].0));
    assert_eq!(fs::read_to_string(f.session_path(&l)).unwrap(), values[0].0);
    f.finish();
}
