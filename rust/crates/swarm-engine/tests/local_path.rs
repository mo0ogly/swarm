#![cfg(unix)]
use std::{
    fs,
    os::unix::fs::symlink,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use swarm_engine::{
    apply_checkpoint, canonical_owned_project_path, canonical_project_root, local_file, GitState,
    LocalPathError, PathRefusal, Request, Work,
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        loop {
            let path = std::env::temp_dir().join(format!(
                "swarm-path-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("fixture creation failed: {e}"),
            }
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).expect("owned fixture cleanup");
        }
    }
}
#[test]
fn local_files_resolve_internal_aliases_and_reject_escape_or_nonregular_targets() {
    let fixture = Fixture::new();
    let outside = Fixture::new();
    fs::write(fixture.0.join("file"), b"inside").unwrap();
    fs::write(outside.0.join("file"), b"outside").unwrap();
    fs::create_dir(fixture.0.join("dir")).unwrap();
    symlink("file", fixture.0.join("inside-link")).unwrap();
    symlink(outside.0.join("file"), fixture.0.join("escape")).unwrap();
    let expected = fs::canonicalize(fixture.0.join("file")).unwrap();
    for name in ["file", "./file", "dir/../file", "inside-link"] {
        assert_eq!(local_file(&fixture.0, name).unwrap(), expected);
    }
    assert!(matches!(
        local_file(&fixture.0, "escape"),
        Err(LocalPathError::Refused(PathRefusal::OutsideProject))
    ));
    assert!(matches!(
        local_file(&fixture.0, "dir"),
        Err(LocalPathError::Refused(PathRefusal::RegularFileRequired))
    ));
    for name in ["", " \u{0085}", "/dev/null"] {
        assert!(matches!(
            local_file(&fixture.0, name),
            Err(LocalPathError::Refused(PathRefusal::RelativeRequired))
        ));
    }
    let e = local_file(&fixture.0, "private-missing-name").unwrap_err();
    assert!(std::error::Error::source(&e).is_some());
    assert!(!format!("{e:?}").contains("private-missing-name"));
}
#[test]
fn lexical_parent_cleanup_precedes_physical_symlink_resolution_as_in_go() {
    let fixture = Fixture::new();
    let outside = Fixture::new();
    fs::create_dir(outside.0.join("dir")).unwrap();
    fs::write(fixture.0.join("file"), b"inside").unwrap();
    symlink(outside.0.join("dir"), fixture.0.join("redirect")).unwrap();
    assert_eq!(
        local_file(&fixture.0, "redirect/../file").unwrap(),
        fixture.0.join("file")
    );
    assert_eq!(
        canonical_project_root(&fixture.0.join("redirect/..")).unwrap(),
        fixture.0
    );
}
#[test]
fn complete_root_alias_is_owned_but_redirected_children_and_inner_root_links_are_not() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.0.join("root")).unwrap();
    let root = fixture.0.join("root");
    fs::create_dir(root.join("child")).unwrap();
    fs::write(root.join("file"), b"data").unwrap();
    symlink(&root, fixture.0.join("alias")).unwrap();
    symlink(&root, root.join("child/root-link")).unwrap();
    assert_eq!(
        canonical_owned_project_path(&root, &fixture.0.join("alias/file")).unwrap(),
        root.join("file")
    );
    assert!(matches!(
        canonical_owned_project_path(&root, &root.join("child/root-link/file")),
        Err(LocalPathError::Refused(PathRefusal::RedirectedOwnedPath))
    ));
    for path in [
        root.join("./file"),
        PathBuf::from(format!("{}/file/", root.display())),
        PathBuf::from(format!("{}//file", root.display())),
        PathBuf::from("relative"),
    ] {
        assert!(matches!(
            canonical_owned_project_path(&root, &path),
            Err(LocalPathError::Refused(
                PathRefusal::AbsoluteNormalizedRequired
            ))
        ));
    }
}
#[test]
fn missing_files_refuse_checkpoint_without_changes_then_recovery_preserves_nil_empty() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join("first"), b"data").unwrap();
    let mut w = Work {
        summary: "prior".into(),
        next: "prior next".into(),
        memory: Some(vec!["prior".into()]),
        ..Default::default()
    };
    let original = w.clone();
    let mut r = Request {
        schema_version: 1,
        summary: "new".into(),
        next: "next".into(),
        memory: Some(vec!["first".into(), "missing".into()]),
        ..Default::default()
    };
    assert!(apply_checkpoint(&mut w, &r, &fixture.0, GitState::default()).is_err());
    assert_eq!(w, original);
    fs::write(fixture.0.join("missing"), b"data").unwrap();
    let git = GitState {
        head: "head".into(),
        branch: "branch".into(),
        changes: "changes".into(),
    };
    apply_checkpoint(&mut w, &r, &fixture.0, git.clone()).unwrap();
    assert_eq!(w.summary, "new");
    assert_eq!(w.memory, r.memory);
    assert_eq!(w.git, git);
    r.memory = Some(vec![]);
    apply_checkpoint(&mut w, &r, &fixture.0, GitState::default()).unwrap();
    assert_eq!(w.memory, Some(vec![]));
    r.memory = None;
    apply_checkpoint(&mut w, &r, &fixture.0, GitState::default()).unwrap();
    assert_eq!(w.memory, None);
    let before = w.clone();
    r.summary = "\u{3000}".into();
    assert!(apply_checkpoint(&mut w, &r, &fixture.0, GitState::default()).is_err());
    assert_eq!(w, before);
}

#[test]
fn go_link_limit_is_preserved_and_link_targets_keep_directory_semantics() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join("file"), b"data").unwrap();
    fs::create_dir(fixture.0.join("dir")).unwrap();
    for count in [40usize, 41, 64, 255, 256] {
        for i in (0..count).rev() {
            symlink(
                if i + 1 == count {
                    "file".into()
                } else {
                    format!("link-{count}-{}", i + 1)
                },
                fixture.0.join(format!("link-{count}-{i}")),
            )
            .unwrap();
        }
        let result = local_file(&fixture.0, &format!("link-{count}-0"));
        if count <= 255 {
            assert_eq!(result.unwrap(), fixture.0.join("file"));
        } else {
            assert!(result.is_err());
        }
    }
    for (name, target) in [
        ("dot-file", "file/."),
        ("slash-file", "file/"),
        ("parent-file", "file/../file"),
        ("parent-dir", "dir/../file"),
    ] {
        symlink(target, fixture.0.join(name)).unwrap();
        let result = local_file(&fixture.0, name);
        if name == "parent-dir" {
            assert_eq!(result.unwrap(), fixture.0.join("file"));
        } else {
            assert!(result.is_err());
        }
    }
}
