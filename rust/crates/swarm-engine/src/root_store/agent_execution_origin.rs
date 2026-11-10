//! Private original storage provenance at native permission, never a capability.
//! Missing legacy metadata stays unknown; a later reader cannot adopt a Root.
#![cfg(target_os = "linux")]
use crate::{agent_launch::query, local_storage::LocalDirectory, AgentLaunchError as E};
use crate::root_store::{observe_instance, RootInstanceIdentity};
use rusqlite::{params, Connection};
const NAME: &str = "native_agent_execution_origins";
const DDL: &str = "CREATE TABLE native_agent_execution_origins(agent_id TEXT PRIMARY KEY REFERENCES agents(id),directory_device TEXT NOT NULL,directory_inode TEXT NOT NULL,database_device TEXT NOT NULL,database_inode TEXT NOT NULL)";
/// Require the exact optional auxiliary representation. Schema/SQL/finalization
/// failures retain typed causes; absence does not create or upgrade history.
fn schema(db: &Connection) -> Result<bool,E> {
    crate::root_store::mission_supervision_store::auxiliary_schema(db,NAME,DDL,
        "agent-origin-schema-invalid").map_err(|e|E::caused("agent-origin-schema",e))
}
/// Decode a full canonical filesystem number, without signed narrowing or a
/// platform-independent policy ceiling. Invalid text is a refusal, not zero.
fn number(raw:String)->Result<u64,E>{
    let value:u64=raw.parse().map_err(|e|E::caused("agent-origin-identity-invalid",e))?;
    if value.to_string()!=raw{return Err(E::Refused("agent-origin-identity-invalid"));}Ok(value)
}
/// Read original metadata only. None is genuine historical absence; statement,
/// schema and codec failures never become absence or current-root adoption.
fn read(db:&Connection,agent:&str)->Result<Option<RootInstanceIdentity>,E>{
    if !schema(db)?{return Ok(None);}
    let row:Option<(String,String,String,String)>=query(db,"SELECT directory_device,directory_inode,database_device,database_inode FROM native_agent_execution_origins WHERE agent_id=?",[agent],|rows|{
        rows.next()?.map(|r|Ok::<_,rusqlite::Error>((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).transpose().map_err(E::from)
    })?;
    row.map(|(dd,di,bd,bi)|Ok(RootInstanceIdentity{directory:(number(dd)?,number(di)?),database:(number(bd)?,number(bi)?)})).transpose()
}
/// Insert once into the caller's actual writer. The instance must be freshly
/// observed by stage; this private helper is also tested as metadata, not grant.
/// Duplicate/FK/schema/finalize failures abort the owning permission transaction.
fn insert(db:&Connection,agent:&str,instance:&RootInstanceIdentity)->Result<(),E>{
    if db.is_autocommit(){return Err(E::Refused("agent-origin-writer-required"));}
    if !schema(db)?{db.execute_batch(DDL)?;}
    let n=crate::root_store::mission_supervision_store::execute(db,"INSERT INTO native_agent_execution_origins VALUES(?,?,?,?,?)",params![agent,instance.directory.0.to_string(),instance.directory.1.to_string(),instance.database.0.to_string(),instance.database.1.to_string()]).map_err(|e|E::caused("agent-origin-insert",e))?;
    if n!=1{return Err(E::Refused("agent-origin-not-recorded"));}Ok(())
}
/// Capture actual connected SQLite AND owned directory identity inside the
/// permission writer, after real child/owner/CAS/budget checks, before grant.
/// Never called during review/recovery to manufacture old provenance. This
/// stages metadata only; commit uncertainty cannot authorize a grant byte.
pub(crate) fn stage(db:&Connection,directory:&LocalDirectory,agent:&str)->Result<(),E>{
    let instance=observe_instance(db,directory).map_err(|e|E::caused("agent-origin-observation",e))?;
    insert(db,agent,&instance)
}
/// Compare observations only; no path/UID/hash supplies a permission or actor
/// identity. Both storage identities must match, including same-directory copies.
fn equal(expected:&RootInstanceIdentity,actual:&RootInstanceIdentity)->Result<(),E>{
    if expected!=actual{return Err(E::Refused("agent-origin-instance-changed"));}Ok(())
}
/// Require the permission-time provenance against actual connected descriptors.
/// Used by the native child gate and completed manual-producer reader, which
/// separately verify kernel parent/current attempt/original terminal receipt.
/// Missing legacy metadata refuses; no creation, execution, refund or acceptance.
pub(crate) fn require(db:&Connection,directory:&LocalDirectory,agent:&str)->Result<(),E>{
    let original=read(db,agent)?.ok_or(E::Refused("agent-origin-provenance-required"))?;
    let actual=observe_instance(db,directory).map_err(|e|E::caused("agent-origin-observation",e))?;
    equal(&original,&actual)
}
#[cfg(test)]
mod tests{
    use super::*;
    /// Pure identity witness before each mismatch, no owned Root or grant.
    #[test]
    fn native_agent_origin_requires_directory_and_database(){
        let good=RootInstanceIdentity{directory:(1,2),database:(3,4)};
        equal(&good,&good).unwrap();
        for bad in [RootInstanceIdentity{database:(3,5),..good.clone()},RootInstanceIdentity{directory:(1,9),..good.clone()}]{assert!(equal(&good,&bad).is_err());equal(&good,&good).unwrap();}
    }
    /// Explicit SQL metadata double, never execution provenance. Valid row first,
    /// then duplicate/corruption refusal and rollback; missing stays None.
    #[test]
    fn native_agent_origin_roundtrip_duplicate_and_corruption(){
        let mut db=Connection::open_in_memory().unwrap();db.execute_batch("PRAGMA foreign_keys=ON;CREATE TABLE agents(id TEXT PRIMARY KEY);INSERT INTO agents VALUES('agent')").unwrap();
        assert!(read(&db,"agent").unwrap().is_none());
        let good=RootInstanceIdentity{directory:(1,2),database:(3,4)};
        {let tx=db.transaction().unwrap();insert(&tx,"agent",&good).unwrap();tx.commit().unwrap();}
        assert_eq!(read(&db,"agent").unwrap(),Some(good.clone()));
        {let tx=db.transaction().unwrap();assert!(insert(&tx,"agent",&good).is_err());tx.rollback().unwrap();}
        assert_eq!(read(&db,"agent").unwrap(),Some(good.clone()));
        {let tx=db.transaction().unwrap();tx.execute("UPDATE native_agent_execution_origins SET database_inode='04' WHERE agent_id='agent'",[]).unwrap();assert!(read(&tx,"agent").is_err());tx.rollback().unwrap();}
        assert_eq!(read(&db,"agent").unwrap(),Some(good));assert!(read(&db,"absent").unwrap().is_none());db.close().unwrap();
    }
    /// Canonical wire numbers retain full u64; malformed input never normalizes.
    #[test]
    fn native_agent_origin_numbers_are_canonical(){
        assert_eq!(number(u64::MAX.to_string()).unwrap(),u64::MAX);
        for bad in ["","01","+1","-1","18446744073709551616"]{assert!(number(bad.into()).is_err());}
    }
}
