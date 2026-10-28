#![cfg(target_os = "linux")]
use std::{fs, path::Path, process::{Command, Output}};
use swarm_engine::{RootStore, SystemEnvironment};

/// Actual public CLI, without providers or a real project; retain original errors.
fn version(root: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_swarm-rust")).args(["version","--json","--root"]).arg(root).output().unwrap()
}
/// A Root without any Git repository still has a known compiled binary identity.
/// The optional source candidate must be explicitly unavailable, without mutation.
#[test]
fn version_initialized_non_git_root_is_explicitly_unavailable() {
    let root=tempfile::tempdir().unwrap();
    RootStore::initialize(root.path(), &mut SystemEnvironment).unwrap().close().unwrap();
    let before=fs::read(root.path().join(".swarm/engine-settings.json")).unwrap();
    let output=version(root.path());
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let value:serde_json::Value=serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["diagnostic"]["candidate"]["available"],false);
    assert!(!value["binary"]["version"].as_str().unwrap().is_empty());
    assert_eq!(fs::read(root.path().join(".swarm/engine-settings.json")).unwrap(),before);
    assert!(!root.path().join(".git").exists());
}
/// A broken marker is not absence: preserve the existing typed Git failure.
#[test]
fn version_broken_git_marker_refuses_and_recovers() {
    let root=tempfile::tempdir().unwrap();
    RootStore::initialize(root.path(), &mut SystemEnvironment).unwrap().close().unwrap();
    fs::write(root.path().join(".git"),b"gitdir: missing-private-fixture\n").unwrap();
    let failed=version(root.path());assert!(!failed.status.success());
    let value:serde_json::Value=serde_json::from_slice(&failed.stderr).unwrap();
    assert_eq!(value["failure"]["code"],"git-metadata");
    assert!(value["failure"]["causes"].as_array().unwrap().iter().any(|v|v=="git-metadata"));
    fs::remove_file(root.path().join(".git")).unwrap();
    assert!(version(root.path()).status.success());
}
/// Cold-start identity remains available and does not initialize an absent Root.
#[test]
fn version_cold_root_does_not_initialize() {
    let parent=tempfile::tempdir().unwrap();let absent=parent.path().join("absent");
    assert!(version(&absent).status.success());assert!(!absent.exists());
}

/// An inherited real repository still reaches the checked Git observer. The
/// fixture lives in the ignored cache; no commit/index/configuration is changed.
#[test]
fn version_parent_repository_remains_observed() {
    let project=Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(3).unwrap();
    let cache=project.join(".cache");fs::create_dir_all(&cache).unwrap();
    let root=tempfile::tempdir_in(cache).unwrap();
    RootStore::initialize(root.path(), &mut SystemEnvironment).unwrap().close().unwrap();
    let output=version(root.path());assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));
    let value:serde_json::Value=serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["diagnostic"]["candidate"]["available"],true);
    assert_eq!(value["diagnostic"]["candidate"]["commit"].as_str().unwrap().len(),40);
}
