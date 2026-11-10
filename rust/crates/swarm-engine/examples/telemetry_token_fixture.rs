//! Test fixture only (feature `test-fixtures`): issue one telemetry ingest token
//! for an agent of an isolated Root through the real engine API, standing in
//! for the provider-launch issuance that does not exist yet (LIVE-UX-03 D1).
//! Usage: `telemetry_token_fixture <root> <agent-id>`; prints the secret once on
//! stdout. Never point it at a live Root; the caller must not persist the secret.
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use swarm_engine::TelemetryStore;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [root, agent] = args.as_slice() else {
        eprintln!("usage: telemetry_token_fixture <root> <agent-id>");
        std::process::exit(2);
    };
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64);
    let result = TelemetryStore::open(Path::new(root), true).and_then(|store| {
        let secret = store.issue_token(agent, now)?;
        store.close().map(|()| secret)
    });
    match result {
        Ok(secret) => println!("{secret}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
