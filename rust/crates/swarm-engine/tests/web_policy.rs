//! New native network defaults are explicit saved input, not library fallbacks.
use swarm_engine::{EngineSettingsCatalog, EngineSettingsValues};
/// Require the eleven explicit network defaults introduced in document v20.
/// The catalog integration owns the exact current global document version;
/// unrelated later policy groups must not break this network contract.
#[test]
fn web_network_defaults_are_declared_in_the_catalogue_and_installation_document() {
    let values = EngineSettingsValues::installation_defaults().unwrap();
    let json = serde_json::to_value(&values).unwrap();
    for (name, value) in [
        ("web_bind_address", "127.0.0.1:0"),
        ("web_http_header_max_bytes", "1048576"),
        ("web_http_max_headers", "100"),
        ("web_http_header_timeout_ms", "5000"),
        ("web_http_body_max_bytes", "16777216"),
        ("web_http_body_timeout_ms", "10000"),
        ("web_http_max_connections", "64"),
        ("web_root_worker_threads", "4"),
        ("web_admin_worker_threads", "1"),
        ("web_worker_idle_keep_alive_ms", "10000"),
        ("web_cookie_max_age_seconds", "2592000"),
    ] {
        assert_eq!(
            json.get(name).and_then(|x| x.as_str()),
            Some(value),
            "missing explicit default {name}"
        );
    }
    let catalog = EngineSettingsCatalog::installation().unwrap();
    assert_eq!(catalog.schema_version, 4);
    assert!(catalog.document_schema_version >= 20);
    assert_eq!(
        catalog
            .fields
            .iter()
            .filter(|f| f.introduced_schema_version == 20)
            .count(),
        11
    );
}

/// Historical absence is distinct from explicit null or a partial future group;
/// exact original v19 bytes/history roundtrip without adoption or recomputation drift.
#[test]
fn web_policy_preserves_original_v19_and_refuses_missing_or_partial_adoption() {
    let raw = include_bytes!("fixtures/legacy-v19-document.json");
    let old = swarm_engine::EngineSettingsDocument::decode(raw).unwrap();
    assert_eq!(old.schema_version, 19);
    assert_eq!(swarm_engine::encode_model(&old).unwrap(), raw);
    assert_eq!(
        old.values.web_transport_policy().unwrap_err().reason(),
        "settings-web-policy-not-configured"
    );
    let mut value = serde_json::to_value(old.values).unwrap();
    value["web_bind_address"] = "127.0.0.1:0".into();
    assert_eq!(
        EngineSettingsValues::decode(&serde_json::to_vec(&value).unwrap())
            .unwrap_err()
            .reason(),
        "settings-web-policy-incomplete"
    );
}
/// Validate actual representation boundaries and operator meanings, never clamp
/// oversized choices or revive a disabled cap through an installation fallback.
#[test]
fn web_policy_limits_null_zero_addresses_and_combined_workers_are_explicit() {
    use swarm_engine::{SettingCap, SettingCount, WebTransportConstraints};
    let mut v = EngineSettingsValues::installation_defaults().unwrap();
    let limits = WebTransportConstraints::current();
    assert!(v.web_transport_policy().is_ok());
    v.web_http_max_headers = Some(SettingCount(limits.max_header_fields.0));
    assert!(v.web_transport_policy().is_ok());
    v.web_http_max_headers = Some(SettingCount(limits.max_header_fields.0 + 1));
    assert_eq!(
        v.web_transport_policy().unwrap_err().reason(),
        "settings-web-header-count-transport"
    );
    v.web_http_max_headers = Some(SettingCount(100));
    v.web_http_header_max_bytes = Some(SettingCount(1));
    assert_eq!(v.web_transport_policy().unwrap().header_max_bytes, 1);
    v.web_http_body_max_bytes = Some(SettingCap(Some(SettingCount(
        limits.native_body_max_bytes.0 + 1,
    ))));
    assert_eq!(
        v.web_transport_policy().unwrap_err().reason(),
        "settings-web-body-transport"
    );
    v.web_http_body_max_bytes = Some(SettingCap(Some(SettingCount(0))));
    v.web_http_max_connections = Some(SettingCap(Some(SettingCount(0))));
    assert_eq!(v.web_transport_policy().unwrap().max_connections, Some(0));
    assert_eq!(v.web_transport_policy().unwrap().body_max_bytes, Some(0));
    v.web_http_body_max_bytes = Some(SettingCap(None));
    v.web_http_max_connections = Some(SettingCap(None));
    v.web_http_header_timeout_ms = Some(SettingCap(None));
    v.web_http_body_timeout_ms = Some(SettingCap(None));
    v.web_cookie_max_age_seconds = Some(SettingCap(None));
    v.web_worker_idle_keep_alive_ms = Some(SettingCount(0));
    let p = v.web_transport_policy().unwrap();
    assert_eq!(p.body_max_bytes, None);
    assert_eq!(p.max_connections, None);
    assert_eq!(p.header_timeout, None);
    assert_eq!(p.body_timeout, None);
    assert_eq!(p.cookie_max_age_seconds, None);
    assert_eq!(p.worker_idle_keep_alive, std::time::Duration::ZERO);
    v.web_cookie_max_age_seconds = Some(SettingCap(Some(SettingCount(31_536_001))));
    assert!(v.web_transport_policy().is_ok());
    v.web_cookie_max_age_seconds = Some(SettingCap(Some(SettingCount(0))));
    assert_eq!(
        v.web_transport_policy().unwrap_err().reason(),
        "settings-web-cookie-age-positive-or-null"
    );
    v.web_cookie_max_age_seconds = Some(SettingCap(None));
    v.web_http_header_timeout_ms = Some(SettingCap(Some(SettingCount(0))));
    assert_eq!(
        v.web_transport_policy().unwrap_err().reason(),
        "settings-web-header-timeout-positive-or-null"
    );
    v.web_http_header_timeout_ms = Some(SettingCap(None));
    v.web_root_worker_threads = Some(SettingCount(limits.max_semaphore_permits.0));
    assert_eq!(
        v.web_transport_policy().unwrap_err().reason(),
        "settings-web-workers-transport"
    );
    v.web_root_worker_threads = Some(SettingCount(0));
    assert!(v.web_transport_policy().is_err());
    v.web_root_worker_threads = Some(SettingCount(4));
    for bad in [
        "localhost:0",
        "0.0.0.0:0",
        "127.0.0.2:0",
        "[::]:0",
        "[0:0:0:0:0:0:0:1]:0",
    ] {
        v.web_bind_address = Some(bad.into());
        assert!(v.web_transport_policy().is_err(), "{bad}");
    }
    v.web_bind_address = Some("[::1]:0".into());
    assert!(v.web_transport_policy().is_ok()); // representation only, no IPv6 socket claim
    v.web_bind_address = Some("127.0.0.1:0".into());
    assert!(v.web_transport_policy().is_ok());
}
/// Metadata distinguishes a session cookie from unlimited policy and exposes
/// technical maxima/effect boundaries without adopting or running the network.
#[test]
fn web_catalogue_null_meaning_limits_and_restart_are_authoritative_metadata() {
    let catalog = EngineSettingsCatalog::installation().unwrap();
    let json = serde_json::to_value(&catalog).unwrap();
    let fields = json["fields"].as_array().unwrap();
    let cookie = fields
        .iter()
        .find(|f| f["name"] == "web_cookie_max_age_seconds")
        .unwrap();
    assert_eq!(cookie["null_label"], "Cookie de session");
    assert_eq!(cookie["application"], "server_restart");
    assert_eq!(cookie["nullable"], true);
    let header = fields
        .iter()
        .find(|f| f["name"] == "web_http_max_headers")
        .unwrap();
    assert_eq!(
        header["technical_maximum"],
        json["network_transport"]["max_header_fields"]
    );
    assert_eq!(
        fields
            .iter()
            .filter(|f| f["application"] == "server_restart")
            .count(),
        11
    );
}
/// Shared preview/apply/reopen uses the saved group, preserves history/replay and
/// leaves Work storage unchanged. This proves policy construction, not TCP use.
#[cfg(target_os = "linux")]
#[test]
fn web_policy_saved_preview_replay_and_readback_keep_original_history() {
    use std::{fs, os::unix::fs::PermissionsExt};
    use swarm_engine::{
        EngineSettingsChange, RootStore, SettingCap, SettingCount, SystemEnvironment,
    };
    let root =
        std::env::temp_dir().join(swarm_engine::generate_identifier("web-policy94-").unwrap());
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let r = RootStore::initialize(&root, &mut SystemEnvironment).unwrap();
    let before = r.settings().unwrap();
    let db = fs::read(root.join(".swarm/state.db")).unwrap();
    let bytes = fs::read(root.join(".swarm/engine-settings.json")).unwrap();
    let mut values = before.values.clone();
    values.web_root_worker_threads = Some(SettingCount(2));
    values.web_admin_worker_threads = Some(SettingCount(3));
    values.web_http_max_connections = Some(SettingCap(None));
    values.web_http_header_timeout_ms = Some(SettingCap(None));
    values.web_cookie_max_age_seconds = Some(SettingCap(None));
    let mut request = EngineSettingsChange {
        schema_version: before.schema_version,
        event_id: "save-network".into(),
        expected_revision: before.revision,
        values,
        reason: "Explicit configured policy, not active runtime".into(),
        preview_token: String::new(),
    };
    request.preview_token = r.preview_settings(&request).unwrap().preview_token;
    assert_eq!(
        fs::read(root.join(".swarm/engine-settings.json")).unwrap(),
        bytes
    );
    let (after, replayed) = r.apply_settings(&request, &mut SystemEnvironment).unwrap();
    assert!(!replayed);
    assert_eq!(after.history[..before.history.len()], before.history);
    let saved = fs::read(root.join(".swarm/engine-settings.json")).unwrap();
    assert!(
        r.apply_settings(&request, &mut SystemEnvironment)
            .unwrap()
            .1
    );
    assert_eq!(
        fs::read(root.join(".swarm/engine-settings.json")).unwrap(),
        saved
    );
    r.close().unwrap();
    let r = RootStore::open(&root, false).unwrap();
    let policy = r.settings().unwrap().values.web_transport_policy().unwrap();
    assert_eq!(policy.root_worker_threads, 2);
    assert_eq!(policy.admin_worker_threads, 3);
    assert_eq!(policy.max_connections, None);
    assert_eq!(policy.cookie_max_age_seconds, None);
    assert!(r.list_works().unwrap().is_empty());
    r.close().unwrap();
    assert_eq!(fs::read(root.join(".swarm/state.db")).unwrap(), db);
    fs::remove_dir_all(root).unwrap();
}

/// Parser/conversion causes stay typed and safe diagnostics never echo supplied addresses.
#[test]
fn web_policy_address_and_count_causes_are_preserved_without_private_values() {
    use std::error::Error;
    use swarm_engine::SettingCount;
    let mut v = EngineSettingsValues::installation_defaults().unwrap();
    v.web_bind_address = Some("PRIVATE_INVALID_ADDRESS".into());
    let e = v.web_transport_policy().unwrap_err();
    assert!(e
        .source()
        .unwrap()
        .downcast_ref::<std::net::AddrParseError>()
        .is_some());
    assert!(!format!("{e:?}").contains("PRIVATE"));
    v.web_bind_address = Some("127.0.0.1:0".into());
    assert!(v.web_transport_policy().is_ok());
    v.web_http_header_max_bytes = Some(SettingCount(-1));
    let e = v.web_transport_policy().unwrap_err();
    assert!(e
        .source()
        .unwrap()
        .downcast_ref::<std::num::TryFromIntError>()
        .is_some());
    assert_eq!(e.reason(), "settings-web-count-representation");
}
