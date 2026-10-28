//! Private automatic-reviewer attribution. Rows bind observations, never confer
//! a lease. Parent writers require the actual borrowed owning handle; the gate
//! separately checks live parent/epoch/consent/root before its actual exec.
use super::store::Job;
use crate::{agent_launch::query, AgentLaunchError as E, EngineSettingsDocument, MissionLease, Work};
use crate::root_store::{LocalDirectory, RootInstanceIdentity, RootStore};
use rusqlite::{params, Connection};
use serde::Serialize;
use crate::work_engine::MutationFence;
const NAME: &str = "native_mission_review_bindings";
const DDL: &str = "CREATE TABLE native_mission_review_bindings(job_id TEXT PRIMARY KEY REFERENCES native_review_jobs(id),generation INTEGER NOT NULL CHECK(generation>0),consent_sequence INTEGER NOT NULL CHECK(consent_sequence>0),directory_device TEXT NOT NULL,directory_inode TEXT NOT NULL,database_device TEXT NOT NULL,database_inode TEXT NOT NULL)";
/// Immutable claim observations. The root tuple comes from connected descriptors,
/// never request JSON. No counter here is an execution policy or a capability.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(super) struct Binding {
    job: String, generation: i64, consent: i64,
    directory_device: String, directory_inode: String,
    database_device: String, database_inode: String,
}
/// Preserve all typed primary/secondary errors under one safe static family.
fn caused(error: impl std::error::Error + Send + Sync + 'static) -> E {
    E::caused("mission-review-authority", error)
}
/// Check exact optional representation without creation. Added triggers or
/// malformed storage refuse; absent table is historical absence, not permission.
fn schema(db: &Connection) -> Result<bool, E> {
    crate::root_store::mission_supervision_store::auxiliary_schema(db, NAME, DDL,
        "mission-review-binding-schema-invalid").map_err(caused)
}
/// Read one original binding under the caller's transaction. Unknown SQL and
/// schema errors never become None; absence can only match the manual hash.
pub(super) fn read(db: &Connection, job: &str) -> Result<Option<Binding>, E> {
    if !schema(db)? { return Ok(None); }
    query(db,"SELECT generation,consent_sequence,directory_device,directory_inode,database_device,database_inode FROM native_mission_review_bindings WHERE job_id=?",[job],|rows| {
        rows.next()?.map(|r| Ok(Binding { job: job.into(), generation:r.get(0)?, consent:r.get(1)?, directory_device:r.get(2)?, directory_inode:r.get(3)?, database_device:r.get(4)?, database_inode:r.get(5)? })).transpose()
    })
}
/// Domain separate automatic source and exact binding; manual source bytes stay
/// unchanged. Removing/replacing a binding cannot reproduce its automatic hash.
/// Serialization errors retain their cause; hashes do not authenticate actors.
pub(super) fn digest(base: &str, binding: Option<&Binding>) -> Result<String, E> {
    let Some(binding) = binding else { return Ok(base.into()); };
    let encoded = serde_json::to_vec(&("swarm-native-mission-review-v1",base,binding)).map_err(caused)?;
    Ok(crate::provider_configuration::configuration_digest(&encoded))
}
/// Compare a current base source to the original automatic/manual domain. This
/// pure read binds history and is also used for terminal/review evidence; it does
/// not require a still-running mission merely to inspect a completed receipt.
pub(super) fn source(db: &Connection, job: &Job, base: &str) -> Result<String, E> {
    digest(base, read(db, &job.id)?.as_ref())
}
/// Current canonical consent/mode/archive; exact sequence prevents withdrawal
/// then restoration from reviving an older automatic control claim.
pub(super) fn enabled(db: &Connection, work: &str) -> Result<i64, E> {
    use crate::root_store::mission_controls as m;
    m::require_planning_mission_active(db, work).map_err(caused)?;
    let consent = m::consent(db,work).map_err(caused)?
        .filter(|v| v.enabled).ok_or(E::Refused("mission-review-consent-required"))?;
    let controls = m::controls(db,work).map_err(caused)?
        .filter(|v| !v.paused && v.autonomy == m::MissionAutonomy::Automatic)
        .ok_or(E::Refused("mission-review-automatic-required"))?;
    let _ = controls;
    Ok(consent.event_sequence)
}
/// Require shared current native producer evidence, including original physical
/// provenance for manually launched workers. This grants no control/review lease;
/// the caller still checks the actual owning handle/current consent separately.
fn producer(db:&Connection, directory:&LocalDirectory, work:&Work, agent:&str, settings:&EngineSettingsDocument)->Result<(),E> {
    crate::root_store::mission_producer::completed(db,directory,work,agent,settings,crate::root_store::mission_producer::OriginRequirement::NativeWorker).map_err(caused)?;
    Ok(())
}
/// Acquire no authority: validate the actual borrowed lease inside this writer,
/// then capture current consent and connected Root observations for a new job.
pub(super) fn claim(root:&RootStore, db:&Connection, lease:&MissionLease, job:&Job, settings:&EngineSettingsDocument)->Result<Binding,E> {
    let now=crate::system_timestamp().map_err(caused)?;
    lease.check(db,&job.work_id,&now).map_err(caused)?;
    let work=crate::read_recorded_work(db,&job.work_id).map_err(caused)?;
    producer(db,&root.directory,&work,&job.producer_id,settings)?;
    let instance=crate::root_store::observe_instance(db,&root.directory).map_err(caused)?;
    Ok(Binding { job:job.id.clone(),generation:lease.generation(),consent:enabled(db,&work.id)?,directory_device:instance.directory.0.to_string(),directory_inode:instance.directory.1.to_string(),database_device:instance.database.0.to_string(),database_inode:instance.database.1.to_string() })
}
/// Record the binding in the existing job claim writer. No nested transaction,
/// upsert or ignored conflict; any SQL/finalize failure aborts the entire claim.
pub(super) fn stage(db:&Connection,binding:&Binding)->Result<(),E> {
    if db.is_autocommit() { return Err(E::Refused("mission-review-writer-required")); }
    if !schema(db)? { db.execute_batch(DDL)?; }
    let n=crate::root_store::mission_supervision_store::execute(db,"INSERT INTO native_mission_review_bindings VALUES(?,?,?,?,?,?,?)",params![binding.job,binding.generation,binding.consent,binding.directory_device,binding.directory_inode,binding.database_device,binding.database_inode]).map_err(caused)?;
    if n!=1 { return Err(E::Refused("mission-review-binding-not-recorded")); } Ok(())
}
/// Observe the exact root tuple. Device/inode serialization is representation,
/// not policy; only comparison to connected owned descriptors makes it useful.
fn instance_matches(binding:&Binding, actual:&RootInstanceIdentity)->bool {
    binding.directory_device==actual.directory.0.to_string() && binding.directory_inode==actual.directory.1.to_string() && binding.database_device==actual.database.0.to_string() && binding.database_inode==actual.database.1.to_string()
}
/// Read-only gate/ongoing-process guard. The actual private gate additionally
/// verifies parent/child/peer identity. No owning lease is reconstructed here;
/// this only refuses revoked/stale/replaced current sources and preserves causes.
pub(super) fn observe(root:&RootStore,db:&Connection,job:&Job,settings:&EngineSettingsDocument)->Result<(),E> {
    let Some(binding)=read(db,&job.id)? else { return Ok(()); };
    let work=crate::read_recorded_work(db,&job.work_id).map_err(caused)?;
    if !instance_matches(&binding,&crate::root_store::observe_instance(db,&root.directory).map_err(caused)?) || enabled(db,&work.id)?!=binding.consent {
        return Err(E::Refused("mission-review-authority-changed"));
    }
    producer(db,&root.directory,&work,&job.producer_id,settings)?;
    crate::root_store::mission_supervision_store::observe_validation_owner(db,&work.id,binding.generation,job.owner_pid,&job.owner_stamp,&job.owner_host,&crate::system_timestamp().map_err(caused)?).map_err(caused)
}
/// Recheck actual owning handle and frozen binding in the permission writer.
/// Automatic rows without an actual lease refuse; manual rows cannot receive
/// one by accident. Replay/EOF/serialized rows never manufacture a capability.
pub(super) fn permission(root:&RootStore,db:&Connection,lease:Option<&MissionLease>,job:&Job,settings:&EngineSettingsDocument)->Result<(),E> {
    let binding=read(db,&job.id)?;
    match (binding.as_ref(),lease) {
        (None,None)=>Ok(()),
        (Some(binding),Some(lease)) if binding.generation==lease.generation()=>{
            lease.check(db,&job.work_id,&crate::system_timestamp().map_err(caused)?).map_err(caused)?;
            observe(root,db,job,settings)
        },
        _=>Err(E::Refused("mission-review-owning-lease-required")),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    /// Isolated normalized metadata double, not an acquired MissionLease or
    /// executable grant. Public native tests must prove actual process authority.
    fn binding()->Binding { Binding {job:"job".into(),generation:1,consent:1,directory_device:"1".into(),directory_inode:"2".into(),database_device:"3".into(),database_inode:"4".into()} }
    /// Minimal parent-key SQL fixture, preserving foreign-key enforcement. This
    /// is not the production Job schema and never fabricates execution authority.
    fn db()->Connection { let db=Connection::open_in_memory().unwrap();db.execute_batch("CREATE TABLE native_review_jobs(id TEXT PRIMARY KEY); INSERT INTO native_review_jobs VALUES('job');").unwrap();db }
    /// Manual compatibility and automatic downgrade guard are byte properties,
    /// not a claim of cryptographic actor authentication or OS confinement.
    #[test]
    fn mission_review_digest_preserves_manual_and_blocks_downgrade(){
        let base="a".repeat(64);let b=binding();
        assert_eq!(digest(&base,None).unwrap(),base);
        let auto=digest(&base,Some(&b)).unwrap();assert_ne!(auto,base);
        let mut variants=Vec::new();
        let mut changed=b.clone();changed.generation+=1;variants.push(changed);
        let mut changed=b.clone();changed.consent+=1;variants.push(changed);
        let mut changed=b.clone();changed.job.push('x');variants.push(changed);
        let mut changed=b.clone();changed.database_inode.push('5');variants.push(changed);
        for changed in variants {assert_ne!(digest(&base,Some(&changed)).unwrap(),auto);}
    }
    /// Positive SQL round trip precedes duplicate, deletion, malformed schema
    /// and restoration. A deleted binding falls to the manual domain, which can
    /// never match the retained automatic job source; no test touches live data.
    #[test]
    fn mission_review_binding_faults_and_recovery(){
        let db=db();let b=binding();let base="a".repeat(64);
        assert!(read(&db,"job").unwrap().is_none());assert!(stage(&db,&b).is_err());
        db.execute_batch("BEGIN").unwrap();stage(&db,&b).unwrap();assert_eq!(read(&db,"job").unwrap(),Some(b.clone()));
        assert!(stage(&db,&b).is_err());db.execute_batch("COMMIT").unwrap();
        let bound=digest(&base,read(&db,"job").unwrap().as_ref()).unwrap();
        db.execute("DELETE FROM native_mission_review_bindings",[]).unwrap();
        assert_ne!(digest(&base,read(&db,"job").unwrap().as_ref()).unwrap(),bound);
        db.execute_batch("BEGIN").unwrap();stage(&db,&b).unwrap();db.execute_batch("COMMIT").unwrap();
        assert_eq!(digest(&base,read(&db,"job").unwrap().as_ref()).unwrap(),bound);
        db.execute_batch("CREATE TRIGGER bad_binding AFTER INSERT ON native_mission_review_bindings BEGIN SELECT 1; END;").unwrap();
        assert!(read(&db,"job").is_err());db.execute_batch("DROP TRIGGER bad_binding").unwrap();assert_eq!(read(&db,"job").unwrap(),Some(b));db.close().unwrap();
    }
    /// Failed claim rollback removes representation and row together. No upsert,
    /// hidden transaction, auto-commit or permission is manufactured by staging.
    #[test]
    fn mission_review_binding_rolls_back_with_claim(){
        let db=db();db.execute_batch("BEGIN").unwrap();stage(&db,&binding()).unwrap();assert!(read(&db,"job").unwrap().is_some());db.execute_batch("ROLLBACK").unwrap();assert!(read(&db,"job").unwrap().is_none());db.close().unwrap();
    }
}
