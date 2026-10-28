//! Synthetic operator package fixture only; explicit test-fixtures required.
//! Intentionally writes business/personal seeds and proof in its private /tmp
//! operator vault. Metadata expose no seeds/proof. Never use live operator data.
use std::{
    fs::DirBuilder,
    io::{self, Write},
    os::unix::fs::DirBuilderExt,
    path::Path,
};
use swarm_enrollment_authority::provisioning::{self as p, repository, service_artifact};
use swarm_enrollment_config::store::{
    documents::{Documents, Publication},
    Context,
};
/// Create absent fixture directories, prepare fresh synthetic material and pin it
/// before immutable private package publication. Modes exercise current/mismatched
/// controller data, never actual distinct OS accounts or a human/service grant.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(args.len(), 3);
    let parent = Path::new(&args[1]);
    assert!(parent.is_absolute());
    let mode = &args[2];
    assert!(matches!(mode.as_str(), "correct" | "wrong-controller"));
    let uid = rustix::process::geteuid().as_raw();
    let other = if uid == 0 { 1 } else { !uid };
    let held = swarm_enrollment_config::store::open_private_directory(parent, uid).unwrap();
    nix::unistd::close(held).unwrap();
    let operator = parent.join("operator");
    let handoff = parent.join("handoff");
    for d in [&operator, &handoff] {
        DirBuilder::new().mode(0o700).create(d).unwrap();
    }
    let target = parent.join("future-service");
    assert!(!target.exists());
    let mut request: p::Request = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/paea_provisioning_request.json"
    ))
    .unwrap();
    request.root_directory = target.to_str().unwrap().into();
    request.controller_uid = if mode == "correct" { uid } else { other };
    request.service_uid = if mode == "correct" { other } else { uid };
    let prepared = p::prepare(&request).unwrap();
    let artifact = service_artifact::from_prepared(&prepared).unwrap();
    let artifact_pin = artifact.pin().unwrap();
    drop(artifact);
    let seal = repository::seal(prepared).unwrap();
    let pin = seal.pin().clone();
    let store = repository::PackageStore::new(Documents::open(&operator, uid).unwrap());
    assert_eq!(
        store.create(&Context::new(), &seal).unwrap(),
        Publication::Durable
    );
    store.close().unwrap();
    assert!(!target.exists());
    let metadata = serde_json::json!({"fixture_kind":"operator-package-v1","operator_directory":operator.to_str().unwrap(),"operator_package_path":operator.join("initial-provisioning-v1.bin").to_str().unwrap(),"operator_package_pin":pin.as_str(),"service_artifact_pin":artifact_pin,"output_path":handoff.join("service-v1.bin").to_str().unwrap(),"service_target":target.to_str().unwrap(),"mode":mode,"human_verified":false,"separate_os_uid_verified":false,"synthetic_secrets_written":true});
    let mut out = io::stdout().lock();
    serde_json::to_writer(&mut out, &metadata).unwrap();
    out.write_all(b"\n").unwrap();
    out.flush().unwrap();
}
