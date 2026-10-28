//! Canonical request data for disposable native preflight tests only. Required
//! test-fixtures, no entropy/material/grant/account/service or production custody.
use sha2::{Digest, Sha256};
use std::{
    fs::OpenOptions,
    io::{self, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
};
use swarm_enrollment_authority::provisioning as p;
/// Write one absent synthetic0600 request, fsync file+held parent and close once.
/// Args are one already private owned /tmp fixture parent, never live state.
/// Setup failures panic; stdout contains fixture metadata only, not request data.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(args.len(), 2);
    let parent = Path::new(&args[1]);
    assert!(parent.is_absolute());
    let uid = rustix::process::geteuid().as_raw();
    let directory = swarm_enrollment_config::store::open_private_directory(parent, uid).unwrap();
    let mut request: p::Request = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/paea_provisioning_request.json"
    ))
    .unwrap();
    let target = parent.join("future-service");
    assert!(!target.exists());
    request.root_directory = target.to_str().unwrap().into();
    request.service_uid = uid;
    request.controller_uid = if uid == 0 { 1 } else { !uid };
    let raw = p::encode_request(&request).unwrap();
    let pin = format!("{:x}", Sha256::digest(&raw));
    let path = parent.join("request.json");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .unwrap();
    file.write_all(&raw).unwrap();
    file.sync_all().unwrap();
    nix::unistd::close(file).unwrap();
    directory.sync_all().unwrap();
    nix::unistd::close(directory).unwrap();
    let metadata = serde_json::json!({"request_path":path.to_str().unwrap(),"request_pin":pin,"target":target.to_str().unwrap(),"human_verified":false,"material_generated":false,"separate_os_uid_verified":false});
    let mut out = io::stdout().lock();
    serde_json::to_writer(&mut out, &metadata).unwrap();
    out.write_all(b"\n").unwrap();
    out.flush().unwrap();
}
