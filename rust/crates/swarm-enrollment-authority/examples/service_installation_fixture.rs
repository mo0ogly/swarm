//! Disposable installation material/process-exit fixture only. Required feature
//! test-fixtures; never use live roots. No business seed is written or returned.
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{stdout, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::Path,
    sync::Arc,
};
use swarm_enrollment_authority::provisioning::{
    self as p, service_artifact, service_installation as install,
};
use zeroize::Zeroizing;
/// Create a new private fixture file, flush actual bytes and close once before
/// returning. Existing paths refuse; any partial new file stays for diagnosis.
fn private_file(path: &Path, bytes: &[u8]) {
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .unwrap();
    f.write_all(bytes).unwrap();
    f.sync_all().unwrap();
    nix::unistd::close(f).unwrap();
}
/// Explicit fixture arguments root,inputdirectory,mode. New private directories
/// only, never adoption/deletion; metadata is printed before an intentional exit.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(args.len(), 4);
    let root = Path::new(&args[1]);
    let input = Path::new(&args[2]);
    let mode = &args[3];
    assert!(!root.exists());
    assert!(root.is_absolute() && input.is_absolute());
    assert!(matches!(
        mode.as_str(),
        "prepare"
            | "prepare-control"
            | "crash-intent"
            | "crash-business"
            | "crash-personal"
            | "crash-visible"
    ));
    let uid = rustix::process::geteuid().as_raw();
    let parent =
        swarm_enrollment_config::store::open_private_directory(root.parent().unwrap(), uid)
            .unwrap();
    nix::unistd::close(parent).unwrap();
    fs::DirBuilder::new().mode(0o700).create(input).unwrap();
    let operator = input.with_extension("operator-fixture");
    fs::DirBuilder::new().mode(0o700).create(&operator).unwrap();
    let mut request: p::Request = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/paea_provisioning_request.json"
    ))
    .unwrap();
    request.root_directory = root.to_str().unwrap().into();
    request.service_uid = uid;
    request.controller_uid = if uid == 0 { 1 } else { !uid };
    if mode == "prepare-control" {
        request.grant.permissions = ["read", "preview", "create", "apply"]
            .into_iter()
            .map(|operation| swarm_identity_contract::Permission {
                operation: "effect.execute".into(),
                resource: format!(
                    "enrollment-config/{}/{operation}",
                    request.installation_digest
                ),
            })
            .collect();
    }
    let material = p::prepare(&request).unwrap();
    let artifact = service_artifact::from_prepared(&material).unwrap();
    let raw = artifact.encode_private().unwrap();
    let pin = artifact.pin().unwrap();
    let path = input.join("service-v1.bin");
    private_file(&path, &raw);
    let proof_path = operator.join("synthetic-proof.bin");
    let mut proof = Zeroizing::new(Vec::with_capacity(32));
    material.write_installation_proof(&mut *proof).unwrap();
    private_file(&proof_path, &proof);
    drop(proof);
    drop(raw);
    let expected = swarm_enrollment_bootstrap::encode_authenticated(artifact.document()).unwrap();
    let metadata = serde_json::json!({"artifact_pin":pin,"artifact_path":path,"proof_path":proof_path,"root":root,"bootstrap_pin":format!("{:x}",Sha256::digest(expected)),"human_verified":false,"separate_os_uid_verified":false,"mode":mode});
    let mut out = stdout().lock();
    serde_json::to_writer(&mut out, &metadata).unwrap();
    out.write_all(b"\n").unwrap();
    out.flush().unwrap();
    drop(out);
    if mode != "prepare" && mode != "prepare-control" {
        let wanted = match mode.as_str() {
            "crash-intent" => install::FixturePhase::IntentDurable,
            "crash-business" => install::FixturePhase::BusinessApplied,
            "crash-personal" => install::FixturePhase::PersonalApplied,
            "crash-visible" => install::FixturePhase::RootRenamed,
            _ => unreachable!(),
        };
        let _result = install::install_with_hook(
            &install::Context::new(),
            &artifact,
            &pin,
            install::Mode::Create,
            Arc::new(move |phase| {
                if phase == wanted {
                    std::process::exit(73)
                }
                Ok(())
            }),
        );
        panic!("requested fixture exit was not reached");
    }
}
