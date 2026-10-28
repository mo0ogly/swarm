#![cfg(target_os = "linux")]
//! Owned inert Roots exercise discovery, never provider/mission admission.
use std::{fs,fs::OpenOptions,os::unix::fs::PermissionsExt,path::PathBuf};
use swarm_engine::{RootStore,SystemEnvironment,EngineSettingsChange,SettingCount};

/// Retain every owned Root on success/failure for attributed replay. No live
/// project selected; stdout records birth before engine initialization.
fn fixture() -> (PathBuf,RootStore) {
    let path=std::env::temp_dir().join(swarm_engine::generate_identifier("swarm-discovery-observation-").unwrap());
    fs::create_dir(&path).unwrap();fs::set_permissions(&path,fs::Permissions::from_mode(0o700)).unwrap();
    println!("OWNED_DISCOVERY_ROOT {}",path.display());
    let mut env=SystemEnvironment;let store=RootStore::initialize(&path,&mut env).unwrap();
    let doc=store.settings().unwrap();let mut values=doc.values.clone();values.storage_busy_retries=Some(SettingCount(0));
    let mut change=EngineSettingsChange{schema_version:doc.schema_version,event_id:"fixture-discovery-policy".into(),expected_revision:doc.revision,values,reason:"Owned zero-retry conflict witness".into(),preview_token:String::new()};
    change.preview_token=store.preview_settings(&change).unwrap().preview_token;store.apply_settings(&change,&mut env).unwrap();
    (path,store)
}

/// Existing reserved reads refuse a real Admin writer; discovery remains a
/// coherent readonly observation, with absence preserved and no permission grant.
#[test]
fn discovery_observes_saved_data_while_admin_reservation_is_exclusive() {
    let(path,mut store)=fixture();let work=store.execute_external("","work.create",br#"{"schema_version":1,"event_id":"discovery-work","expected_revision":0,"title":"Discovery fixture","objective":"Observe controls","scope":"Owned data only","criteria":["No authority granted"]}"#,&mut SystemEnvironment).unwrap().result.work;
    let expected=store.mission_control_view(&work.id).unwrap();let doc=store.settings().unwrap();
    let before=fs::read(path.join(".swarm/engine-settings.json")).unwrap();
    let lock=OpenOptions::new().read(true).write(true).open(path.join(".swarm/.engine-settings.lock")).unwrap();
    rustix::fs::flock(&lock,rustix::fs::FlockOperation::NonBlockingLockExclusive).unwrap();
    assert!(store.settings().is_err());assert!(store.mission_control_view(&work.id).is_err());
    let observed=store.mission_discovery_observation().unwrap();
    assert_eq!(observed.settings_revision,doc.revision);assert_eq!(observed.policy,doc.values.mission_supervision_policy().unwrap());
    assert_eq!(observed.missions.len(),1);assert_eq!(observed.missions[0].0,work.id);
    let actual=&observed.missions[0].1;assert_eq!(actual.revision,expected.revision);assert_eq!(actual.paused,expected.paused);assert_eq!(actual.autonomy,expected.autonomy);assert_eq!(actual.slots,expected.slots);assert_eq!(actual.consent_enabled,expected.consent_enabled);assert_eq!(actual.archived,expected.archived);
    assert_eq!(fs::read(path.join(".swarm/engine-settings.json")).unwrap(),before);
    rustix::fs::flock(&lock,rustix::fs::FlockOperation::Unlock).unwrap();drop(lock);
    assert!(store.settings().is_ok());store.close().unwrap();
}

/// Invalid atomic content must never become defaults or an empty discovery.
#[test]
fn discovery_preserves_malformed_settings_and_recovers_after_owned_restore() {
    let(path,store)=fixture();assert!(store.mission_discovery_observation().unwrap().missions.is_empty());
    let settings=path.join(".swarm/engine-settings.json");let original=fs::read(&settings).unwrap();
    fs::write(path.join("settings-original.json"),&original).unwrap();fs::write(&settings,b"{").unwrap();
    assert!(store.mission_discovery_observation().is_err());
    let pending=path.join(".swarm/fixture-settings-restore");fs::write(&pending,&original).unwrap();fs::set_permissions(&pending,fs::Permissions::from_mode(0o600)).unwrap();fs::rename(&pending,&settings).unwrap();
    assert!(store.mission_discovery_observation().unwrap().missions.is_empty());store.close().unwrap();
}

/// A valid document at a replaced pathname does not bind the opened old Root.
#[test]
fn discovery_refuses_a_replaced_physical_project() {
    let(path,store)=fixture();assert!(store.mission_discovery_observation().is_ok());
    let retained=path.with_extension("old");fs::rename(&path,&retained).unwrap();fs::create_dir(&path).unwrap();fs::set_permissions(&path,fs::Permissions::from_mode(0o700)).unwrap();
    println!("OWNED_DISCOVERY_RETAINED_ROOT {}",retained.display());
    let replacement=RootStore::initialize(&path,&mut SystemEnvironment).unwrap();replacement.close().unwrap();
    assert!(store.mission_discovery_observation().is_err());store.close().unwrap();
}
