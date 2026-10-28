//! Owned native disk-pack fixtures, no project or business authority.
#![cfg(target_os = "linux")]
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    process::Command,
};
use swarm_engine::{
    run_git_pack, EngineSettingsDocument, EngineSettingsValues, GitPackError,
    GitPackOperation as Op,
};
struct Fixture {
    dir: tempfile::TempDir,
    source: PathBuf,
    target: PathBuf,
    oid: String,
    serial: usize,
}
/// Execute fixed fixture Git commands and assert original stderr on failures.
fn git(path: &Path, args: &[&str]) -> Vec<u8> {
    let r = Command::new("git")
        .arg("--no-pager")
        .arg("--git-dir")
        .arg(".")
        .args(args)
        .current_dir(path)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .unwrap();
    assert!(r.status.success(), "{:?}", r.stderr);
    r.stdout
}
/// Use real default settings as a fixture input, never change live policy.
fn settings() -> EngineSettingsDocument {
    EngineSettingsDocument::initial(
        EngineSettingsValues::installation_defaults().unwrap(),
        "fixture".into(),
        "2026-10-28T12:17:29Z".into(),
    )
    .unwrap()
}
impl Fixture {
    /// Make two fresh bare repositories and a real commit, without any Root DB.
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let target = dir.path().join("target");
        for p in [&source, &target] {
            fs::create_dir(p).unwrap();
            let r = Command::new("git")
                .args(["init", "--bare", "--object-format=sha1", "."])
                .current_dir(p)
                .output()
                .unwrap();
            assert!(r.status.success())
        }
        let tree = git(&source, &["mktree"]);
        let tree = std::str::from_utf8(&tree).unwrap().trim();
        let r = Command::new("git")
            .args([
                "--git-dir",
                ".",
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@invalid",
                "commit-tree",
                tree,
                "-m",
                "fixture",
            ])
            .current_dir(&source)
            .output()
            .unwrap();
        assert!(r.status.success(), "{:?}", r.stderr);
        let oid = String::from_utf8(r.stdout).unwrap().trim().to_string();
        git(&source, &["update-ref", "refs/heads/keep", &oid]);
        Self {
            dir,
            source,
            target,
            oid,
            serial: 0,
        }
    }
    /// Prepare fresh, independently owned files for one exact native operation.
    fn run(
        &mut self,
        op: Op,
        input: Vec<u8>,
        target: bool,
    ) -> (
        Result<swarm_engine::GitPackRun, GitPackError>,
        PathBuf,
        PathBuf,
    ) {
        self.serial += 1;
        let prefix = self.dir.path().join(format!("run{}", self.serial));
        let input_path = prefix.with_extension("input");
        let output_path = prefix.with_extension("output");
        let stderr_path = prefix.with_extension("stderr");
        fs::write(&input_path, input).unwrap();
        let mut output = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&output_path)
            .unwrap();
        let mut stderr = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&stderr_path)
            .unwrap();
        let r = run_git_pack(
            if target { &self.target } else { &self.source },
            op,
            File::open(input_path).unwrap(),
            &mut output,
            &mut stderr,
            &settings(),
        );
        (r, output_path, stderr_path)
    }
    /// Export an actual commit pack as the valid control before fault injection.
    fn pack(&mut self) -> Vec<u8> {
        let oid = format!("{}\n", self.oid);
        let (r, p, _) = self.run(Op::Export, oid.into_bytes(), false);
        assert!(r.unwrap().exit_status.success());
        fs::read(p).unwrap()
    }
}
/// Exact native pack/object content and refs are observed across export/import/repeat.
#[test]
fn git_pack_streams_commit_without_refs() {
    let mut f = Fixture::new();
    let before = git(&f.source, &["for-each-ref"]);
    let pack = f.pack();
    assert!(pack.starts_with(b"PACK"));
    for _ in 0..2 {
        let (r, _, _) = f.run(Op::Import, pack.clone(), true);
        assert!(r.unwrap().exit_status.success());
        assert_eq!(git(&f.target, &["cat-file", "-t", &f.oid]), b"commit\n");
        assert!(git(&f.target, &["for-each-ref"]).is_empty())
    }
    assert_eq!(git(&f.source, &["for-each-ref"]), before)
}
/// Real corruption refuses with retained channels/possible writes, then recovers.
#[test]
fn git_pack_corrupt_refusal_and_recovery() {
    let mut f = Fixture::new();
    let pack = f.pack();
    assert!(f.run(Op::Import, pack.clone(), true).0.is_ok());
    let mut damaged = pack.clone();
    let last = damaged.len() - 1;
    damaged[last] ^= 1;
    let (r, _, stderr) = f.run(Op::Import, damaged, true);
    let e = r.unwrap_err();
    assert_eq!(e.code(), "git-pack-exit");
    assert!(e.may_have_written);
    assert!(!e.exit_status.unwrap().success());
    assert!(!fs::read(stderr).unwrap().is_empty());
    let links = swarm_engine::native_error_links(&e);
    assert!(links.labels.iter().any(|v| v == "git-pack-writes-possible"));
    assert!(!e.branches().is_empty());
    assert!(f.run(Op::Import, pack, true).0.is_ok());
    assert!(git(&f.target, &["for-each-ref"]).is_empty())
}
/// Aliased descriptors refuse before spawn and preserve the original input bytes.
#[test]
fn git_pack_alias_refuses_before_spawn() {
    let mut f = Fixture::new();
    let _control = f.pack();
    let p = f.dir.path().join("alias");
    fs::write(&p, []).unwrap();
    let input = File::open(&p).unwrap();
    let mut output = OpenOptions::new().write(true).open(&p).unwrap();
    let mut stderr = File::create(f.dir.path().join("stderr")).unwrap();
    let e = run_git_pack(
        &f.source,
        Op::Export,
        input,
        &mut output,
        &mut stderr,
        &settings(),
    )
    .unwrap_err();
    assert_eq!(e.code(), "git-pack-files");
    assert!(!e.may_have_written);
    assert!(e.exit_status.is_none());
    assert!(fs::read(p).unwrap().is_empty())
}
/// Nonempty output refuses without overwriting it; fresh output remains usable.
#[test]
fn git_pack_nonempty_output_refuses_and_recovers() {
    let mut f = Fixture::new();
    let pack = f.pack();
    let input = f.dir.path().join("input");
    fs::write(&input, &pack).unwrap();
    let p = f.dir.path().join("existing");
    fs::write(&p, b"retained").unwrap();
    let mut output = OpenOptions::new().write(true).open(&p).unwrap();
    let mut stderr = File::create(f.dir.path().join("stderr")).unwrap();
    let e = run_git_pack(
        &f.target,
        Op::Import,
        File::open(input).unwrap(),
        &mut output,
        &mut stderr,
        &settings(),
    )
    .unwrap_err();
    assert_eq!(e.code(), "git-pack-files");
    assert!(!e.may_have_written);
    assert_eq!(fs::read(p).unwrap(), b"retained");
    assert!(f.run(Op::Import, pack, true).0.is_ok())
}
/// Missing object export retains nonzero status and disk diagnostic, not empty success.
#[test]
fn git_pack_missing_oid_retains_failure() {
    let mut f = Fixture::new();
    let _control = f.pack();
    let (r, p, stderr) = f.run(
        Op::Export,
        format!("{}\n", "3".repeat(40)).into_bytes(),
        false,
    );
    let e = r.unwrap_err();
    assert_eq!(e.code(), "git-pack-exit");
    assert!(e.may_have_written && e.exit_status.is_some());
    assert!(!fs::read(stderr).unwrap().is_empty());
    assert!(p.exists());
    let _recovery = f.pack();
}

/// A regular read-only sink reaches a real write error. Retain the typed runner
/// with its cleanup/child ownership, then recover using new writable outputs.
#[test]
fn git_pack_output_io_error_preserves_runner_and_recovers() {
    let mut f = Fixture::new();
    let _control = f.pack();
    let input = f.dir.path().join("io-input");
    fs::write(&input, format!("{}\n", f.oid)).unwrap();
    let output = f.dir.path().join("read-only-output");
    fs::write(&output, []).unwrap();
    let mut stdout = File::open(&output).unwrap();
    let mut stderr = File::create(f.dir.path().join("io-stderr")).unwrap();
    let mut e = run_git_pack(
        &f.source,
        Op::Export,
        File::open(input).unwrap(),
        &mut stdout,
        &mut stderr,
        &settings(),
    )
    .unwrap_err();
    assert_eq!(e.code(), "git-pack-process");
    assert!(e.may_have_written);
    assert!(e.process_error_mut().is_some());
    assert!(fs::read(output).unwrap().is_empty());
    let wrapped = std::io::Error::other(Box::new(e));
    let mut pending: Vec<&(dyn std::error::Error + 'static)> = vec![&wrapped];
    let mut labels = vec![];
    while let Some(e) = pending.pop() {
        let links = swarm_engine::native_error_links(e);
        labels.extend(links.labels);
        pending.extend(links.children.into_iter().map(|(_, e)| e));
    }
    assert!(labels.iter().any(|s| s == "git-pack-process"));
    let _recovery = f.pack();
}

/// Real kernel-refused proc output sinks preserve primary runner ownership and
/// both fsync causes with their distinct channels. No proc contents are read.
#[test]
fn git_pack_sync_failures_keep_channel_and_primary() {
    let mut f = Fixture::new();
    let _control = f.pack();
    let input = f.dir.path().join("sync-input");
    fs::write(&input, format!("{}\n", f.oid)).unwrap();
    let mut stdout = File::open("/proc/self/maps").unwrap();
    let mut stderr = File::open("/proc/self/status").unwrap();
    let mut e = run_git_pack(
        &f.source,
        Op::Export,
        File::open(input).unwrap(),
        &mut stdout,
        &mut stderr,
        &settings(),
    )
    .unwrap_err();
    assert_eq!(e.code(), "git-pack-process");
    assert!(e.may_have_written);
    assert!(e.process_error_mut().is_some());
    assert_eq!(e.branches().len(), 3);
    let wrapped = std::io::Error::other(Box::new(e));
    let mut pending: Vec<&(dyn std::error::Error + 'static)> = vec![&wrapped];
    let mut labels = vec![];
    let mut invalid = 0;
    while let Some(e) = pending.pop() {
        let links = swarm_engine::native_error_links(e);
        labels.extend(links.labels);
        if links.io_errno == Some(22) {
            invalid += 1;
        }
        pending.extend(links.children.into_iter().map(|(_, e)| e));
    }
    assert!(invalid >= 2, "both actual EINVAL causes retained");
    assert!(
        labels.iter().any(|s| s == "git-pack-stdout-sync"),
        "stdout sync channel missing"
    );
    assert!(
        labels.iter().any(|s| s == "git-pack-stderr-sync"),
        "stderr sync channel missing"
    );
    let _recovery = f.pack();
}
