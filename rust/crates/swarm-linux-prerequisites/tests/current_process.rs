#![cfg(target_os = "linux")]
//! Real current process observation only, never a protection/launch assertion.
#[test]
fn native_linux_prerequisites_observe_current_process_without_claiming_protection() {
    let report = swarm_linux_prerequisites::observe();
    let value = serde_json::to_value(&report).unwrap();
    assert_eq!(value["schema_version"], 2);
    assert_eq!(value["kind"], "paea-linux-prerequisite-observation");
    assert_eq!(value["scope"], "current-process-namespace");
    assert_eq!(value["read_only"], true);
    for name in [
        "launch_verified",
        "separate_host_uid_verified",
        "confinement_verified",
        "human_verified",
    ] {
        assert_eq!(value[name], false, "{name}");
    }
    assert_eq!(value["identity"]["status"], "observed");
    assert_eq!(
        value["identity"]["value"]["euid"],
        nix::unistd::geteuid().as_raw()
    );
    assert_eq!(
        value["identity"]["value"]["egid"],
        nix::unistd::getegid().as_raw()
    );
    for name in ["uid_map", "gid_map"] {
        assert_eq!(value["maps"][name]["status"], "observed");
        assert!(!value["maps"][name]["value"].as_array().unwrap().is_empty());
    }
    assert!(value["missing_prerequisites"].is_array());
}
