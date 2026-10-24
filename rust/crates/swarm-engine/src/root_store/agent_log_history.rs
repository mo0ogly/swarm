//! Historical CLI log pages, distinct from the 100-row live API cursor.
use super::{finished, material_path, RootStore, RootStoreError};
use rusqlite::{params, Connection};

/// Exact historical CLI wire row. Recorded text is not agent authentication,
/// liveness or permission; consumers must escape it for their rendering context.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct HistoricalAgentLog {
    pub seq: i64,
    pub agent_id: String,
    pub at: String,
    pub kind: String,
    pub message: String,
}

/// Go CLI response pagination, not retention or an operator execution limit.
/// Start with a negative cursor and continue from the last positive seq to read
/// the entire history. Zero deliberately requests the latest page instead.
const HISTORICAL_PAGE_ROWS: i64 = 200;

/// Decode an existing Agent's exact rows and explicitly finalize the statement.
/// Caller checks Agent existence and material identity. SQL/decoding/finalization
/// failures retain their causes; no malformed row becomes an empty success.
fn read_page(
    db: &Connection,
    agent: &str,
    after: i64,
) -> Result<Vec<HistoricalAgentLog>, RootStoreError> {
    let sql = if after == 0 {
        "SELECT seq,agent_id,at,kind,message FROM (SELECT seq,agent_id,at,kind,message FROM agent_logs WHERE agent_id=?1 AND seq>?2 ORDER BY seq DESC LIMIT ?3) ORDER BY seq"
    } else {
        "SELECT seq,agent_id,at,kind,message FROM agent_logs WHERE agent_id=?1 AND seq>?2 ORDER BY seq LIMIT ?3"
    };
    let mut statement = db.prepare(sql).map_err(RootStoreError::Sql)?;
    let result = (|| {
        let rows = statement
            .query_map(params![agent, after.max(0), HISTORICAL_PAGE_ROWS], |row| {
                Ok(HistoricalAgentLog {
                    seq: row.get(0)?,
                    agent_id: row.get(1)?,
                    at: row.get(2)?,
                    kind: row.get(3)?,
                    message: row.get(4)?,
                })
            })
            .map_err(RootStoreError::Sql)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(RootStoreError::Sql)
    })();
    finished(result, statement.finalize().map_err(RootStoreError::Sql))
}

impl RootStore {
    /// Read the historical CLI contract without mutating or launching anything.
    /// Requires an existing, valid stored Agent and unchanged opened project
    /// association under the saved settings reservation. Zero returns latest200
    /// ascending; negative starts the first200; positive continues strictly after
    /// that seq. Empty is an empty Vec. Rows are observations, never liveness.
    /// Existence, storage, decoding, identity and explicit-close errors propagate;
    /// this is not an atomic filesystem snapshot or later-action authority.
    pub fn historical_agent_logs(
        &self,
        agent: &str,
        after: i64,
    ) -> Result<Vec<HistoricalAgentLog>, RootStoreError> {
        material_path::with_material_read(self, || {
            self.read_agent(agent)?;
            read_page(&self.db, agent, after)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// An interleaved SQL fixture distinguishes latest, forward and negative
    /// boundaries; it is not runtime state seeding or an Agent-existence proof.
    #[test]
    fn historical_pagination_and_isolation() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE agent_logs(seq INTEGER PRIMARY KEY,agent_id TEXT,at TEXT,kind TEXT,message TEXT)").unwrap();
        for seq in 0..=250 {
            db.execute(
                "INSERT INTO agent_logs VALUES(?1,'a','now','output',?2)",
                params![seq, format!("line {seq}")],
            )
            .unwrap();
        }
        db.execute(
            "INSERT INTO agent_logs VALUES(251,'other','now','output','private')",
            [],
        )
        .unwrap();
        let latest = read_page(&db, "a", 0).unwrap();
        assert_eq!(latest.len(), 200);
        assert_eq!(latest[0].seq, 51);
        assert_eq!(latest[199].seq, 250);
        for cursor in [-1, i64::MIN] {
            let first = read_page(&db, "a", cursor).unwrap();
            assert_eq!(first.len(), 200);
            assert_eq!(first[0].seq, 1);
            assert_eq!(first[199].seq, 200);
        }
        let rest = read_page(&db, "a", 200).unwrap();
        assert_eq!(rest.len(), 50);
        assert_eq!(rest[0].seq, 201);
        assert!(read_page(&db, "a", i64::MAX).unwrap().is_empty());
        assert!(read_page(&db, "missing", 0).unwrap().is_empty());
        assert!(latest.iter().all(|r| r.agent_id == "a"));
    }
    /// A malformed stored cell must refuse rather than silently omit its row.
    #[test]
    fn historical_decode_error_is_not_empty() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE agent_logs(seq INTEGER PRIMARY KEY,agent_id TEXT,at TEXT,kind TEXT,message TEXT);INSERT INTO agent_logs VALUES(1,'a','now','output',NULL)").unwrap();
        assert!(matches!(
            read_page(&db, "a", 0),
            Err(RootStoreError::Sql(_))
        ));
    }
}
