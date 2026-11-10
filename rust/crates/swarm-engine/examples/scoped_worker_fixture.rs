//! Test-only sidecar for retained hierarchical fixture data. This example is
//! never linked into the application CLI or enabled by a production feature.
use std::{error::Error, path::PathBuf};
/// Seed only an explicitly marked private temporary root created by the Python
/// recipe. Register the real identity projection and preserve its SQL triggers;
/// decode/validate the candidate and CAS the original revision before commit.
/// No provider/agent/reservation is created. The caller must own the fresh root;
/// the marker is a fixture boundary, not authenticated proof against a same-UID writer.
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err("fixture requires root, work and candidate file".into());
    }
    let root = PathBuf::from(&args[0]).canonicalize()?;
    let work = args[1].to_str().ok_or("fixture work UTF8 required")?;
    let marker = std::fs::read(root.join(".swarm/scoped-fixture-owner"))?;
    let expected = format!(
        "swarm-scoped-worker-fixture-v1\n{}\n{work}\n",
        root.display()
    );
    if marker != expected.as_bytes() || root.parent() != Some(std::path::Path::new("/tmp")) {
        return Err("owned temporary fixture marker required".into());
    }
    let raw = std::fs::read(&args[2])?;
    let candidate = swarm_engine::decode_work(&raw)?;
    swarm_engine::validate_mission_organization(&candidate)?;
    if candidate.id != work {
        return Err("fixture work binding mismatch".into());
    }
    let mut db = rusqlite::Connection::open(root.join(".swarm/state.db"))?;
    swarm_engine::register_identity_plan_projection(&db)?;
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let original = swarm_engine::read_recorded_work(&tx, work)?;
    if original.planning.is_some() || original.revision != candidate.revision {
        return Err("fixture initial revision/planning changed".into());
    }
    let body = swarm_engine::encode_model(&candidate)?;
    if tx.execute(
        "UPDATE works SET body=? WHERE id=?",
        rusqlite::params![body, work],
    )? != 1
        || swarm_engine::read_recorded_work(&tx, work)? != candidate
    {
        return Err("fixture write not preserved".into());
    }
    tx.commit()?;
    db.close().map_err(|(_, e)| e)?;
    Ok(())
}
