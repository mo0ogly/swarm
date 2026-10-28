//! Held-Root profile authority on an attributed inert installation, no provider.
#![cfg(target_os = "linux")]
use std::{io::Write,os::unix::fs::{DirBuilderExt,OpenOptionsExt}};
use swarm_engine::{RootStore,SystemEnvironment};

/// A valid preview precedes the instance swap. Replacing the pathname's datastore
/// cannot transfer the held store's authority to a project write. Restoring the
/// exact owned original permits recovery; both roots and their birth survive.
#[test]
fn held_root_profile_refuses_replacement_and_recovers() {
    let path=std::env::temp_dir().join(format!("swarm-profile-config-root-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
    let mut birth=std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(path.join("fixture-birth.json")).unwrap();
    birth.write_all(serde_json::to_vec(&serde_json::json!({"root":path,"fixture":true,"live":false,"owner":"profile-configuration-root-test"})).unwrap().as_slice()).unwrap();birth.sync_all().unwrap();drop(birth);
    println!("PROJECT_PROFILE_CONFIGURATION_FIXTURE {}",path.display());
    let root=RootStore::initialize(&path,&mut SystemEnvironment).unwrap();
    std::fs::write(path.join("notes.md"),"Ordinary project instructions\n").unwrap();
    let request=br#"{"profile":{"version":1,"name":"Profile fixture","source":"custom","instructions":[{"path":"notes.md","roles":["preparation","planner","subplanner","worker","reviewer"]}]},"expected_sha256":""}"#;
    assert_eq!(root.project_profile_preview(request).unwrap().roles.len(),5);
    assert!(!path.join("swarm.project.json").exists());
    let original=path.join(".swarm-retained");std::fs::rename(path.join(".swarm"),&original).unwrap();
    let replacement=RootStore::initialize(&path,&mut SystemEnvironment).unwrap();
    let error=root.project_profile_apply(request).err().expect("replaced instance must refuse");
    assert!(format!("{error:?}").contains("project-profile-root-instance-changed"));
    assert!(!path.join("swarm.project.json").exists());
    replacement.close().unwrap();
    std::fs::rename(path.join(".swarm"),path.join(".swarm-replacement")).unwrap();std::fs::rename(original,path.join(".swarm")).unwrap();
    let status=root.project_profile_apply(request).unwrap();assert!(status.enabled);assert_eq!(status.roles.len(),5);
    assert!(path.join("swarm.project.json").is_file());root.close().unwrap();
}
