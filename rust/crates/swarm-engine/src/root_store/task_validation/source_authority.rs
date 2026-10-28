//! Native planning-control epoch, separate from ordinary inbox/decision churn.
//! Canonical stored events are provenance data, not authenticated human identity.
use crate::{agent_launch::query, AgentLaunchError as E, Work};
use serde::Serialize;
#[derive(PartialEq, Eq, Serialize)]
pub(super) struct Authority {
    pub event_id: String,
    pub revision: i64,
    pub kind: String,
}
/// Select the latest actual activation/pause/resume in the caller's Work SQL
/// snapshot. LIMIT1 means latest event, not a history-retention or attempt cap.
/// Require canonical request/event/revision coherence; malformed latest data
/// refuses rather than falling back to an older plausible authority. Ordinary
/// planning decisions do not revoke unchanged control authority. A new control
/// mutator requires review of this projection; this is not all-history validation.
/// Missing native provenance refuses, never fabricates consent or a grant.
pub(super) fn read(db: &rusqlite::Connection, work: &Work) -> Result<Authority, E> {
    query(db,"SELECT id,revision,kind,request FROM main.events WHERE work_id=? AND kind IN ('planning.enable','planning.pause','planning.resume') ORDER BY revision DESC LIMIT 1",[&work.id],|rows|{
        let r=rows.next()?.ok_or(E::Refused("task-validation-planning-control-event-required"))?;
        let id:String=r.get(0)?;let revision:i64=r.get(1)?;let kind:String=r.get(2)?;let raw:Vec<u8>=r.get(3)?;
        let request=crate::decode_planning_request(&raw).map_err(|e|E::caused("task-validation-planning-control-event-invalid",e))?;
        if request.event_id!=id||request.expected_revision.checked_add(1)!=Some(revision)||revision>work.revision{
            return Err(E::Refused("task-validation-planning-control-event-inconsistent"));
        }
        Ok(Authority{event_id:id,revision,kind})
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    /// Explicit isolated SQL provenance double; no actual role/human/child or
    /// Work mutation is proved. Public flows establish canonical event producers.
    fn insert(db: &rusqlite::Connection, id: &str, revision: i64, kind: &str, raw: &[u8]) {
        db.execute(
            "INSERT INTO events VALUES(?,'work',?,?,?)",
            rusqlite::params![id, revision, kind, raw],
        )
        .unwrap();
    }
    /// Valid activation precedes normal decision, pause/resume, malformed latest
    /// and missing-provenance faults. Restore changes only the injected fixture;
    /// latest malformed data cannot make the reader reuse an older authority.
    #[test]
    fn authority_tracks_revocation_and_restore_without_binding_normal_decisions() {
        let db = rusqlite::Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE events(id TEXT PRIMARY KEY,work_id TEXT,revision INTEGER,kind TEXT,request BLOB)").unwrap();
        let mut w = Work {
            id: "work".into(),
            revision: 2,
            ..Default::default()
        };
        insert(
            &db,
            "enable",
            2,
            "planning.enable",
            br#"{"schema_version":1,"event_id":"enable","expected_revision":1}"#,
        );
        let enabled = read(&db, &w).unwrap();
        assert_eq!(enabled.event_id, "enable");
        insert(
            &db,
            "ordinary",
            3,
            "planning.decide",
            b"Not authority input",
        );
        w.revision = 3;
        assert!(read(&db, &w).unwrap() == enabled);
        insert(
            &db,
            "pause",
            4,
            "planning.pause",
            br#"{"schema_version":1,"event_id":"pause","expected_revision":3}"#,
        );
        w.revision = 4;
        let paused = read(&db, &w).unwrap();
        assert!(paused != enabled);
        insert(
            &db,
            "resume",
            5,
            "planning.resume",
            br#"{"schema_version":1,"event_id":"resume","expected_revision":4}"#,
        );
        w.revision = 5;
        let resumed = read(&db, &w).unwrap();
        assert!(resumed != paused && resumed != enabled);
        insert(
            &db,
            "bad",
            6,
            "planning.resume",
            br#"{"schema_version":1,"event_id":"different","expected_revision":5}"#,
        );
        w.revision = 6;
        assert!(read(&db, &w).is_err());
        db.execute("DELETE FROM events WHERE id='bad'", []).unwrap();
        assert!(read(&db, &w).unwrap() == resumed);
        db.execute("DELETE FROM events WHERE kind!='planning.decide'", [])
            .unwrap();
        assert!(read(&db, &w).is_err());
        db.close().unwrap();
    }
}
