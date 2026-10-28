#![cfg(target_os = "linux")]
//! Trusted native listener/bootstrap on isolated Roots; HTTP acceptance is tested separately.
use std::{
    fs,
    net::TcpListener,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::PathBuf,
};
use swarm_engine::{EngineSettingsChange, RootStore, SettingCap, SettingCount, SystemEnvironment};
use swarm_web::NativeHttpServer;
/// Real private Root, no account/provider/mission/database reused.
fn root() -> PathBuf {
    let p = std::env::temp_dir().join(swarm_engine::generate_identifier("server95-").unwrap());
    fs::create_dir(&p).unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
    RootStore::initialize(&p, &mut SystemEnvironment)
        .unwrap()
        .close()
        .unwrap();
    p
}
/// Configure only through public preview/apply; history remains intact.
fn configure(root: &std::path::Path, address: Option<String>, disabled: bool) {
    let s = RootStore::open(root, true).unwrap();
    let d = s.settings().unwrap();
    let mut values = d.values;
    if let Some(address) = address {
        values.web_bind_address = Some(address)
    }
    if disabled {
        values.web_http_max_connections = Some(SettingCap(Some(SettingCount(0))))
    }
    let mut r = EngineSettingsChange {
        schema_version: d.schema_version,
        event_id: "server-config".into(),
        expected_revision: d.revision,
        values,
        reason: "Owned server bootstrap witness".into(),
        preview_token: String::new(),
    };
    r.preview_token = s.preview_settings(&r).unwrap().preview_token;
    s.apply_settings(&r, &mut SystemEnvironment).unwrap();
    s.close().unwrap()
}
/// Actual bound port, source revision and private reused credential survive close;
/// binding uses the same opened Root that supplied the validated saved policy.
#[test]
fn native_server_bootstrap_binds_observed_address_and_closes_without_stopping_agents() {
    let p = root();
    let server = NativeHttpServer::bind(&p, None).unwrap();
    let address = server.local_addr().unwrap();
    assert!(address.ip().is_loopback());
    assert_ne!(address.port(), 0);
    assert_eq!(server.settings_revision(), SettingCount(1));
    assert_eq!(server.policy().bind_address.port(), 0);
    assert_eq!(server.policy().root_worker_threads, 4);
    let sessions: Vec<_> = fs::read_dir(p.join(".swarm"))
        .unwrap()
        .filter_map(|e| {
            let e = e.unwrap();
            e.file_name()
                .to_string_lossy()
                .starts_with("web-session-")
                .then_some(e.path())
        })
        .collect();
    assert_eq!(sessions.len(), 1);
    let key = fs::read(&sessions[0]).unwrap();
    assert_eq!(key.len(), 32);
    assert_eq!(fs::metadata(&sessions[0]).unwrap().mode() & 0o777, 0o600);
    server.close().unwrap();
    assert_eq!(fs::read(&sessions[0]).unwrap(), key);
    let r = RootStore::open(&p, false).unwrap();
    assert!(r.list_works().unwrap().is_empty());
    r.close().unwrap();
    fs::remove_dir_all(p).unwrap();
}
/// A valid bootstrap precedes disabled/bind faults. Disabled policy does not
/// create a session; occupied address preserves cause and leaves saved state.
#[test]
fn native_server_disabled_and_occupied_bind_refuse_without_new_session_or_policy_change() {
    let witness = root();
    NativeHttpServer::bind(&witness, None)
        .unwrap()
        .close()
        .unwrap();
    fs::remove_dir_all(witness).unwrap();
    let p = root();
    configure(&p, None, true);
    let before = fs::read(p.join(".swarm/engine-settings.json")).unwrap();
    let e = match NativeHttpServer::bind(&p, None) {
        Ok(s) => {
            s.close().unwrap();
            panic!("disabled server started")
        }
        Err(e) => e,
    };
    assert_eq!(e.code(), "web-network-disabled");
    assert!(!fs::read_dir(p.join(".swarm")).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with("web-session-")));
    assert_eq!(
        fs::read(p.join(".swarm/engine-settings.json")).unwrap(),
        before
    );
    fs::remove_dir_all(p).unwrap();
    let p = root();
    let occupied = TcpListener::bind("127.0.0.1:0").unwrap();
    configure(&p, Some(occupied.local_addr().unwrap().to_string()), false);
    let before = fs::read(p.join(".swarm/engine-settings.json")).unwrap();
    let e = match NativeHttpServer::bind(&p, None) {
        Ok(s) => {
            s.close().unwrap();
            panic!("occupied bind accepted")
        }
        Err(e) => e,
    };
    assert_eq!(e.code(), "web-listener-bind");
    assert!(std::error::Error::source(&e).is_some());
    assert!(!fs::read_dir(p.join(".swarm")).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with("web-session-")));
    assert_eq!(
        fs::read(p.join(".swarm/engine-settings.json")).unwrap(),
        before
    );
    drop(occupied);
    NativeHttpServer::bind(&p, None).unwrap().close().unwrap();
    fs::remove_dir_all(p).unwrap();
}
