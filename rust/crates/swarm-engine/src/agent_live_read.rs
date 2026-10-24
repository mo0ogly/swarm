//! Read-only cursor pages over recorded agent output for live observation.
//! Nothing here starts, stops, leases or writes to an agent; rows are returned
//! exactly as recorded, so a page proves recorded output, not process liveness.
use crate::AgentStoreError;
use rusqlite::{Connection, Params, Rows};

/// Maximum rows returned per cursor page. Protocol parity with the Go
/// `/api/v1/logs` page size (100); it bounds one response, not what is
/// readable: callers continue from `next` until `more` is false.
pub const LIVE_PAGE_ROWS: usize = 100;

/// One recorded `agent_logs` row; `seq` is the global monotonic cursor.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct AgentLogEntry {
    pub seq: i64,
    pub at: String,
    pub kind: String,
    pub message: String,
}

/// One recorded `terminal_events` PTY chunk. `text` is the lossy UTF-8 view
/// of the raw bytes, with `invalid_utf8` set whenever replacement occurred, so
/// a decoding loss is visible instead of silent. Escape sequences are kept;
/// rendering/sanitizing belongs to the consumer.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct TerminalChunk {
    pub seq: i64,
    pub text: String,
    pub invalid_utf8: bool,
    pub cols: i64,
    pub rows: i64,
}

/// A bounded page plus the cursor to resume from. `next` equals the input
/// cursor when the page is empty; `more` is true when the page was full and
/// further rows may follow.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct CursorPage<T> {
    pub items: Vec<T>,
    pub next: i64,
    pub more: bool,
}

/// Read up to [`LIVE_PAGE_ROWS`] log rows of `agent` with `seq > after`, in
/// ascending order. A negative cursor is refused (`live-cursor-invalid`).
/// SQL, type and finalization failures stay explicit; an unknown agent yields
/// an empty page, so callers must check agent existence separately.
pub fn read_agent_logs_after(
    db: &Connection,
    agent: &str,
    after: i64,
) -> Result<CursorPage<AgentLogEntry>, AgentStoreError> {
    page(
        db,
        "SELECT seq,at,kind,message FROM main.agent_logs WHERE agent_id=? AND seq>? ORDER BY seq LIMIT ?",
        agent,
        after,
        |row| {
            Ok(AgentLogEntry {
                seq: row.get(0)?,
                at: row.get(1)?,
                kind: row.get(2)?,
                message: row.get(3)?,
            })
        },
        |entry| entry.seq,
    )
}

/// Read up to [`LIVE_PAGE_ROWS`] PTY chunks of `agent` with `seq > after`.
/// Same cursor, refusal and error contract as [`read_agent_logs_after`].
/// Only recorded bytes are read; no terminal control channel is opened.
pub fn read_terminal_events_after(
    db: &Connection,
    agent: &str,
    after: i64,
) -> Result<CursorPage<TerminalChunk>, AgentStoreError> {
    page(
        db,
        "SELECT seq,data,cols,rows FROM main.terminal_events WHERE agent_id=? AND seq>? ORDER BY seq LIMIT ?",
        agent,
        after,
        |row| {
            let data: Vec<u8> = row.get(1)?;
            let text = String::from_utf8_lossy(&data);
            Ok(TerminalChunk {
                seq: row.get(0)?,
                invalid_utf8: matches!(text, std::borrow::Cow::Owned(_)),
                text: text.into_owned(),
                cols: row.get(2)?,
                rows: row.get(3)?,
            })
        },
        |chunk| chunk.seq,
    )
}

/// Digest of everything the live views display: every Work id/revision, every
/// agent id/status/desired/body, and the newest `agent_logs`/`terminal_events`
/// cursors. Equal digests mean no displayed state changed since the last read;
/// a different digest says *something* changed, not what. Cost is one read of
/// all agent rows (O(agents)); no row is written. Returned as lowercase hex.
pub fn read_live_fingerprint(db: &Connection) -> Result<String, AgentStoreError> {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    let mut feed = |part: &[u8]| {
        hash.update((part.len() as u64).to_le_bytes());
        hash.update(part);
    };
    collect(db, "SELECT id,revision FROM main.works ORDER BY id", [], |rows| {
        while let Some(row) = rows.next()? {
            feed(row.get::<_, String>(0)?.as_bytes());
            feed(&row.get::<_, i64>(1)?.to_le_bytes());
        }
        Ok(())
    })?;
    collect(
        db,
        "SELECT id,status,desired,CAST(body AS BLOB) FROM main.agents ORDER BY id",
        [],
        |rows| {
            while let Some(row) = rows.next()? {
                feed(row.get::<_, String>(0)?.as_bytes());
                feed(row.get::<_, String>(1)?.as_bytes());
                feed(row.get::<_, String>(2)?.as_bytes());
                feed(&row.get::<_, Vec<u8>>(3)?);
            }
            Ok(())
        },
    )?;
    for sql in [
        "SELECT coalesce(max(seq),0) FROM main.agent_logs",
        "SELECT coalesce(max(seq),0) FROM main.terminal_events",
    ] {
        let seq = collect(db, sql, [], |rows| match rows.next()? {
            Some(row) => row.get::<_, i64>(0),
            None => Ok(0),
        })?;
        feed(&seq.to_le_bytes());
    }
    Ok(hash
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// Shared bounded cursor query; statement finalization errors are preserved
/// alongside a primary failure rather than dropped.
fn page<T>(
    db: &Connection,
    sql: &str,
    agent: &str,
    after: i64,
    decode: impl Fn(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
    seq: impl Fn(&T) -> i64,
) -> Result<CursorPage<T>, AgentStoreError> {
    if after < 0 {
        return Err(AgentStoreError::Refused("live-cursor-invalid"));
    }
    let limit = LIVE_PAGE_ROWS as i64;
    let items = collect(db, sql, rusqlite::params![agent, after, limit], |rows| {
        let mut items = Vec::new();
        while let Some(row) = rows.next()? {
            items.push(decode(row)?);
        }
        Ok(items)
    })?;
    let next = items.last().map(&seq).unwrap_or(after);
    let more = items.len() == LIVE_PAGE_ROWS;
    Ok(CursorPage { items, next, more })
}

fn collect<T>(
    db: &Connection,
    sql: &str,
    parameters: impl Params,
    read: impl FnOnce(&mut Rows<'_>) -> rusqlite::Result<T>,
) -> Result<T, AgentStoreError> {
    let mut statement = db.prepare(sql)?;
    let result = match statement.query(parameters) {
        Ok(mut rows) => read(&mut rows).map_err(AgentStoreError::from),
        Err(e) => Err(e.into()),
    };
    match (result, statement.finalize()) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(error.into()),
        (Err(primary), Err(cleanup)) => Err(AgentStoreError::Finalize {
            primary: Box::new(primary),
            cleanup,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal schema subset; the real Root schema is exercised by swarm-web tests.
    fn db() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE agent_logs(seq INTEGER PRIMARY KEY AUTOINCREMENT, agent_id TEXT NOT NULL, at TEXT NOT NULL, kind TEXT NOT NULL, message TEXT NOT NULL);
             CREATE TABLE terminal_events(seq INTEGER PRIMARY KEY AUTOINCREMENT, agent_id TEXT NOT NULL, data BLOB NOT NULL, cols INTEGER NOT NULL DEFAULT 0, rows INTEGER NOT NULL DEFAULT 0);",
        )
        .unwrap();
        db
    }

    #[test]
    fn log_cursor_pages_only_the_requested_agent_in_order() {
        let db = db();
        for i in 0..(LIVE_PAGE_ROWS + 5) {
            db.execute(
                "INSERT INTO agent_logs(agent_id,at,kind,message) VALUES('a','t','message',?)",
                [format!("m{i}")],
            )
            .unwrap();
            db.execute(
                "INSERT INTO agent_logs(agent_id,at,kind,message) VALUES('b','t','message','other')",
                [],
            )
            .unwrap();
        }
        let first = read_agent_logs_after(&db, "a", 0).unwrap();
        assert_eq!(first.items.len(), LIVE_PAGE_ROWS);
        assert!(first.more);
        assert_eq!(first.items[0].message, "m0");
        assert!(first.items.windows(2).all(|w| w[0].seq < w[1].seq));
        let second = read_agent_logs_after(&db, "a", first.next).unwrap();
        assert_eq!(second.items.len(), 5);
        assert!(!second.more);
        assert_eq!(second.items[4].message, format!("m{}", LIVE_PAGE_ROWS + 4));
        let empty = read_agent_logs_after(&db, "a", second.next).unwrap();
        assert!(empty.items.is_empty());
        assert_eq!(empty.next, second.next);
    }

    #[test]
    fn negative_cursor_is_refused_after_a_valid_witness() {
        let db = db();
        assert!(read_agent_logs_after(&db, "a", 0).is_ok());
        assert!(matches!(
            read_agent_logs_after(&db, "a", -1),
            Err(AgentStoreError::Refused("live-cursor-invalid"))
        ));
        assert!(matches!(
            read_terminal_events_after(&db, "a", -1),
            Err(AgentStoreError::Refused("live-cursor-invalid"))
        ));
    }

    #[test]
    fn terminal_chunks_flag_invalid_utf8_instead_of_hiding_it() {
        let db = db();
        db.execute(
            "INSERT INTO terminal_events(agent_id,data,cols,rows) VALUES('a',?,80,24)",
            [b"\x1b[1mok\r\n".to_vec()],
        )
        .unwrap();
        db.execute(
            "INSERT INTO terminal_events(agent_id,data,cols,rows) VALUES('a',?,80,24)",
            [vec![0xff, b'x']],
        )
        .unwrap();
        let page = read_terminal_events_after(&db, "a", 0).unwrap();
        assert_eq!(page.items[0].text, "\x1b[1mok\r\n");
        assert!(!page.items[0].invalid_utf8);
        assert_eq!(page.items[0].cols, 80);
        assert!(page.items[1].invalid_utf8);
        assert_eq!(page.items[1].text, "\u{fffd}x");
    }

    #[test]
    fn fingerprint_changes_with_displayed_state_only() {
        let db = db();
        db.execute_batch(
            "CREATE TABLE works(id TEXT PRIMARY KEY, revision INTEGER NOT NULL, body BLOB NOT NULL);
             CREATE TABLE agents(id TEXT PRIMARY KEY, work_id TEXT, task_id TEXT, cwd TEXT, status TEXT NOT NULL, desired TEXT NOT NULL DEFAULT '', body BLOB NOT NULL, request BLOB);
             CREATE TABLE other(x);
             INSERT INTO works VALUES('w',1,'{}');
             INSERT INTO agents(id,status,body) VALUES('a','running','{\"heartbeat\":\"1\"}');",
        )
        .unwrap();
        let first = read_live_fingerprint(&db).unwrap();
        assert_eq!(first.len(), 64);
        assert_eq!(read_live_fingerprint(&db).unwrap(), first, "stable witness");
        db.execute("INSERT INTO other VALUES(1)", []).unwrap();
        assert_eq!(read_live_fingerprint(&db).unwrap(), first, "unrelated table");
        let mut seen = vec![first];
        for change in [
            "UPDATE agents SET body='{\"heartbeat\":\"2\"}'",
            "UPDATE agents SET status='completed'",
            "UPDATE agents SET desired='stop'",
            "UPDATE works SET revision=2",
            "INSERT INTO agent_logs(agent_id,at,kind,message) VALUES('a','t','k','m')",
            "INSERT INTO terminal_events(agent_id,data) VALUES('a',x'00')",
        ] {
            db.execute(change, []).unwrap();
            let next = read_live_fingerprint(&db).unwrap();
            assert!(!seen.contains(&next), "{change}");
            seen.push(next);
        }
    }

    #[test]
    fn missing_table_is_an_error_not_an_empty_page() {
        let db = Connection::open_in_memory().unwrap();
        assert!(matches!(
            read_agent_logs_after(&db, "a", 0),
            Err(AgentStoreError::Sql(_))
        ));
    }
}
