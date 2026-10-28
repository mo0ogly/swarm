#![cfg(target_os = "linux")]
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};
use swarm_engine::{RootStore, SystemEnvironment};
/// One isolated Root with a configured custom model announcement; the private
/// unavailable command is staging data and is never executed by these tests.
fn fixture() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let store = RootStore::initialize(root.path(), &mut SystemEnvironment).unwrap();
    let configuration=swarm_engine::decode_provider_configuration(br#"{"schema_version":1,"providers":{"fixture":{"command":"/PRIVATE/program","args":["--model","shared"],"env_allow":["PRIVATE_ENV"]}}}"#).unwrap();
    let source = store
        .initialize_provider_configuration(&configuration)
        .unwrap();
    let policy = serde_json::json!({"version":1,"billing":"external","work_level":"standard","page_level":"simple","levels":{"simple":{"model":"shared","effort":""},"standard":{"model":"shared","effort":""},"exigeant":{"model":"shared","effort":""}}});
    let mut request = serde_json::json!({"version":1,"expected_digest":source.digest,"policies":{"fixture":policy},"preview":true});
    let change =
        swarm_engine::decode_provider_policy_change(&serde_json::to_vec(&request).unwrap())
            .unwrap();
    let before = std::fs::read(root.path().join(".swarm/providers.json")).unwrap();
    let view = store.preview_provider_policies(&change).unwrap();
    assert_eq!(
        std::fs::read(root.path().join(".swarm/providers.json")).unwrap(),
        before
    );
    assert!(!serde_json::to_string(&view).unwrap().contains("PRIVATE"));
    request["preview"] = false.into();
    request["preview_token"] = view.preview_token.into();
    let input = root.path().join("policies.json");
    std::fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
    store.close().unwrap();
    (root, input)
}
/// Actual CLI dispatcher inputs; no shell, fixture hook or live root is used.
fn args(root: &Path, input: &Path) -> Vec<String> {
    vec![
        "providers".into(),
        "policies".into(),
        "apply".into(),
        "--root".into(),
        root.to_str().unwrap().into(),
        "--input".into(),
        input.to_str().unwrap().into(),
        "--json".into(),
    ]
}
/// Controlled write/flush-only failure; private cause must not escape publicly.
struct Denied(bool);
impl Write for Denied {
    /// The flush witness accepts bytes; the other case refuses at write.
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        if self.0 {
            Ok(b.len())
        } else {
            Err(io::Error::other("PRIVATE sink"))
        }
    }
    /// Inject only the chosen output stage; never mutate provider/config data.
    fn flush(&mut self) -> io::Result<()> {
        if self.0 {
            Err(io::Error::other("PRIVATE sink"))
        } else {
            Ok(())
        }
    }
}
/// Actual publication precedes sink failure; export/readback retain policy,
/// old digest replay refuses without another snapshot, and causes stay safe.
#[test]
fn provider_policy_output_faults_preserve_applied_state_and_original() {
    for flush in [false, true] {
        let (control, input) = fixture();
        let mut output = vec![];
        let mut errors = vec![];
        assert_eq!(
            swarm_cli::run_detailed(
                &args(control.path(), &input),
                "en",
                &mut output,
                &mut errors
            )
            .code,
            0
        );
        let view: serde_json::Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(view["applied"], true);
        assert!(!String::from_utf8(output).unwrap().contains("PRIVATE"));
        let (root, input) = fixture();
        let file = root.path().join(".swarm/providers.json");
        let before = std::fs::read(&file).unwrap();
        let db = root.path().join(".swarm/state.db");
        let db_before = std::fs::read(&db).unwrap();
        let mut stderr = vec![];
        let result = swarm_cli::run_detailed(
            &args(root.path(), &input),
            "en",
            &mut Denied(flush),
            &mut stderr,
        );
        assert_eq!(result.code, 2);
        assert!(matches!(
            result.error,
            Some(swarm_cli::CliError::ConfigurationStatePreserved { .. })
        ));
        assert!(!String::from_utf8_lossy(&stderr).contains("PRIVATE"));
        let store = RootStore::open(root.path(), false).unwrap();
        let exported = store.export_provider_policies().unwrap();
        assert_eq!(exported.policies.len(), 1);
        assert_eq!(exported.policies["fixture"].work_level, "standard");
        let refs = store.provider_configuration_backups().unwrap().references;
        assert_eq!(refs.len(), 1);
        assert_eq!(
            std::fs::read(root.path().join(".swarm").join(&refs[0])).unwrap(),
            before
        );
        store.close().unwrap();
        assert_eq!(std::fs::read(&db).unwrap(), db_before);
        let mut output = vec![];
        let result =
            swarm_cli::run_detailed(&args(root.path(), &input), "en", &mut output, &mut stderr);
        assert_eq!(result.code, 2);
        let store = RootStore::open(root.path(), false).unwrap();
        assert_eq!(
            store.provider_configuration_backups().unwrap().references,
            refs
        );
        store.close().unwrap();
    }
}
/// Explicit preview:true and readonly Root refuse application, preserving all
/// bytes; a current writable explicit apply then succeeds without a provider run.
#[test]
fn provider_policy_preview_and_readonly_cannot_apply() {
    let (root, input) = fixture();
    let file = root.path().join(".swarm/providers.json");
    let before = std::fs::read(&file).unwrap();
    let raw = std::fs::read(&input).unwrap();
    let request = swarm_engine::decode_provider_policy_change(&raw).unwrap();
    let store = RootStore::open(root.path(), false).unwrap();
    assert!(store.preview_provider_policies(&request).is_ok());
    assert_eq!(
        store
            .apply_provider_policies(&request)
            .err()
            .unwrap()
            .code(),
        "root-read-only"
    );
    store.close().unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    value["preview"] = true.into();
    let r =
        swarm_engine::decode_provider_policy_change(&serde_json::to_vec(&value).unwrap()).unwrap();
    let store = RootStore::open(root.path(), true).unwrap();
    assert_eq!(
        store.apply_provider_policies(&r).err().unwrap().code(),
        "provider-policy-preview-not-application"
    );
    assert_eq!(std::fs::read(&file).unwrap(), before);
    assert!(store
        .provider_configuration_backups()
        .unwrap()
        .references
        .is_empty());
    assert!(store.apply_provider_policies(&request).unwrap().applied);
    store.close().unwrap();
}
