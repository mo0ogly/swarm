//! Read-only cursor pages over recorded agent output for live observation.
//! Nothing here starts, stops, leases or writes to an agent; rows are returned
//! exactly as recorded, so a page proves recorded output, not process liveness.
use crate::AgentStoreError;
use rusqlite::{Connection, Params, Rows};

/// Retained legacy telemetry page choice, still used by telemetry_store only.
/// Log/PTY readers require explicit saved rows and do not consume this value.
/// This is an open operator-policy migration, not an immutable protocol bound.
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
/// cursor when empty; `more` requires an actual extra retained row. Exact
/// decimal next_exact prevents browser precision loss, without a row-ID cap.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct CursorPage<T> {
    pub items: Vec<T>,
    pub next: i64,
    /// Exact decimal cursor for clients whose JSON numbers cannot retain i64.
    pub next_exact: String,
    pub more: bool,
}

/// Read log rows of `agent` with `seq > after`, under the supplied saved row
/// choice (positive finite or None unlimited), in
/// ascending order. A negative cursor is refused (`live-cursor-invalid`).
/// SQL, type and finalization failures stay explicit; an unknown agent yields
/// an empty page, so callers must check agent existence separately.
pub fn read_agent_logs_after(
    db: &Connection,
    agent: &str,
    after: i64,
    page_rows: Option<i64>,
) -> Result<CursorPage<AgentLogEntry>, AgentStoreError> {
    page(
        db,
        "SELECT seq,at,kind,message FROM main.agent_logs WHERE agent_id=? AND seq>? ORDER BY seq LIMIT ?",
        agent,
        after,
        page_rows,
        |row| {
            Ok(AgentLogEntry {
                seq: row.get(0)?,
                at: recorded_log_text(row,1)?,
                kind: recorded_log_text(row,2)?,
                message: recorded_log_text(row,3)?,
            })
        },
        |entry| entry.seq,
    )
}

/// Read PTY chunks with `seq > after`, under a supplied positive finite or
/// explicitly unlimited row choice. No row policy is inferred here.
/// Same cursor, refusal and error contract as [`read_agent_logs_after`].
/// Only recorded bytes are read; no terminal control channel is opened.
pub fn read_terminal_events_after(
    db: &Connection,
    agent: &str,
    after: i64,
    page_rows: Option<i64>,
) -> Result<CursorPage<TerminalChunk>, AgentStoreError> {
    page(
        db,
        "SELECT seq,data,cols,rows FROM main.terminal_events WHERE agent_id=? AND seq>? ORDER BY seq LIMIT ?",
        agent,
        after,
        page_rows,
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
    collect(
        db,
        "SELECT id,revision FROM main.works ORDER BY id",
        [],
        |rows| {
            while let Some(row) = rows.next()? {
                feed(row.get::<_, String>(0)?.as_bytes());
                feed(&row.get::<_, i64>(1)?.to_le_bytes());
            }
            Ok(())
        },
    )?;
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
    Ok(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Go bytewise SQL TEXT/BLOB log string; null/numeric types retain SQL cause.
/// No trim, escaping, normalization, identity, policy or data mutation.
pub(crate) fn recorded_log_text(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<String> {
    match row.get_ref(index)? {
        rusqlite::types::ValueRef::Text(raw) | rusqlite::types::ValueRef::Blob(raw) => {
            Ok(crate::text::go_utf8(raw))
        }
        value => Err(rusqlite::Error::InvalidColumnType(
            index,
            "recorded log text".into(),
            value.data_type(),
        )),
    }
}

/// Shared bounded cursor query; statement finalization errors are preserved
/// alongside a primary failure rather than dropped.
fn page<T>(
    db: &Connection,
    sql: &str,
    agent: &str,
    after: i64,
    page_rows: Option<i64>,
    decode: impl Fn(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
    seq: impl Fn(&T) -> i64,
) -> Result<CursorPage<T>, AgentStoreError> {
    if after < 0 {
        return Err(AgentStoreError::Refused("live-cursor-invalid"));
    }
    if page_rows.is_some_and(|n| n <= 0) {
        return Err(AgentStoreError::Refused("live-page-rows-invalid"));
    }
    let take = page_rows
        .map(|n| {
            n.checked_add(1)
                .ok_or(AgentStoreError::Refused("live-page-rows-representation"))
        })
        .transpose()?
        .unwrap_or(-1);
    let (items, more) = collect(db, sql, rusqlite::params![agent, after, take], |rows| {
        let mut items = Vec::new();
        let mut more = false;
        while let Some(row) = rows.next()? {
            let entry = decode(row)?;
            if page_rows.is_some_and(|n| i64::try_from(items.len()).ok() == Some(n)) {
                more = true;
                break;
            }
            items.push(entry);
        }
        Ok((items, more))
    })?;
    let next = items.last().map(&seq).unwrap_or(after);
    Ok(CursorPage {
        items,
        next,
        next_exact: next.to_string(),
        more,
    })
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
    const TEST_PAGE_ROWS: usize = 100;

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
        for i in 0..(TEST_PAGE_ROWS + 5) {
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
        let first = read_agent_logs_after(&db, "a", 0, Some(TEST_PAGE_ROWS as i64)).unwrap();
        assert_eq!(first.items.len(), TEST_PAGE_ROWS);
        assert!(first.more);
        assert_eq!(first.items[0].message, "m0");
        assert!(first.items.windows(2).all(|w| w[0].seq < w[1].seq));
        let second =
            read_agent_logs_after(&db, "a", first.next, Some(TEST_PAGE_ROWS as i64)).unwrap();
        assert_eq!(second.items.len(), 5);
        assert!(!second.more);
        assert_eq!(second.items[4].message, format!("m{}", TEST_PAGE_ROWS + 4));
        let empty =
            read_agent_logs_after(&db, "a", second.next, Some(TEST_PAGE_ROWS as i64)).unwrap();
        assert!(empty.items.is_empty());
        assert_eq!(empty.next, second.next);
    }

    #[test]
    fn negative_cursor_is_refused_after_a_valid_witness() {
        let db = db();
        assert!(read_agent_logs_after(&db, "a", 0, Some(TEST_PAGE_ROWS as i64)).is_ok());
        assert!(matches!(
            read_agent_logs_after(&db, "a", -1, Some(TEST_PAGE_ROWS as i64)),
            Err(AgentStoreError::Refused("live-cursor-invalid"))
        ));
        assert!(matches!(
            read_terminal_events_after(&db, "a", -1, Some(TEST_PAGE_ROWS as i64)),
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
        let page = read_terminal_events_after(&db, "a", 0, Some(TEST_PAGE_ROWS as i64)).unwrap();
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
        assert_eq!(
            read_live_fingerprint(&db).unwrap(),
            first,
            "unrelated table"
        );
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
            read_agent_logs_after(&db, "a", 0, Some(TEST_PAGE_ROWS as i64)),
            Err(AgentStoreError::Sql(_))
        ));
    }
}

#[cfg(test)]
mod explicit_row_policy_tests {
    use super::*;
    fn db() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE agent_logs(seq INTEGER PRIMARY KEY,agent_id TEXT,at TEXT,kind TEXT,message TEXT)").unwrap();
        db
    }
    /// Exact terminal pages and very large SQL cursors remain readable without
    /// a JavaScript-safe row-ID ceiling. Zero/overflow are invalid API inputs.
    #[test]
    fn supplied_rows_and_actual_more_preserve_large_decimal_cursor() {
        let db = db();
        let large = 9007199254740993i64;
        db.execute(
            "INSERT INTO agent_logs VALUES(?1,'a','t','output','x')",
            [large],
        )
        .unwrap();
        let single = read_agent_logs_after(&db, "a", 0, Some(1)).unwrap();
        assert_eq!(single.items.len(), 1);
        assert!(!single.more);
        assert_eq!(single.next, large);
        assert_eq!(single.next_exact, "9007199254740993");
        assert!(serde_json::to_string(&single)
            .unwrap()
            .contains("9007199254740993"));
        db.execute(
            "INSERT INTO agent_logs VALUES(1,'a','t','output','first')",
            [],
        )
        .unwrap();
        let first = read_agent_logs_after(&db, "a", 0, Some(1)).unwrap();
        assert_eq!(first.next, 1);
        assert!(first.more);
        let last = read_agent_logs_after(&db, "a", first.next, Some(1)).unwrap();
        assert_eq!(last.next_exact, "9007199254740993");
        assert!(!last.more);
        let all = read_agent_logs_after(&db, "a", 0, None).unwrap();
        assert_eq!(all.items.len(), 2);
        assert!(!all.more);
        for bound in [0, -1, i64::MAX] {
            assert!(read_agent_logs_after(&db, "a", 0, Some(bound)).is_err())
        }
        let empty = read_agent_logs_after(&db, "a", large, None).unwrap();
        assert_eq!(empty.next_exact, "9007199254740993");
        assert!(empty.items.is_empty());
        assert!(!empty.more);
    }
    /// TEXT/BLOB byte decoding follows Go; a nullable fixture then demonstrates
    /// that an unreadable cell fails after a valid positive control.
    #[test]
    fn recorded_sql_text_blob_and_null_refusal() {
        let db = db();
        db.execute(
            "INSERT INTO agent_logs VALUES(1,'a','t','output','valid')",
            [],
        )
        .unwrap();
        assert_eq!(
            read_agent_logs_after(&db, "a", 0, Some(1)).unwrap().items[0].message,
            "valid"
        );
        db.execute(
            "UPDATE agent_logs SET at=CAST(?1 AS TEXT),kind=?2,message=?3 WHERE seq=1",
            rusqlite::params![vec![0xe2u8, 0x82], b"output".to_vec(), vec![b'x', 0xff]],
        )
        .unwrap();
        let page = read_agent_logs_after(&db, "a", 0, None).unwrap();
        assert_eq!(page.items[0].at, "\u{fffd}\u{fffd}");
        assert_eq!(page.items[0].kind, "output");
        assert_eq!(page.items[0].message, "x\u{fffd}");
        db.execute("UPDATE agent_logs SET message=NULL WHERE seq=1", [])
            .unwrap();
        assert!(matches!(
            read_agent_logs_after(&db, "a", 0, None),
            Err(AgentStoreError::Sql(_))
        ));
    }
}
