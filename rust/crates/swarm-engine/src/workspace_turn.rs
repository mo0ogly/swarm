//! Read-only persistent workspace queue projection. This module neither grants
//! a turn nor authenticates an Agent, and is not connected to public admission.
use rusqlite::{types::ValueRef, Connection, Row};
use std::{collections::BTreeSet, error::Error, fmt};

/// Observed protocol state, not a process-liveness or permission assertion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TurnState {
    Waiting,
    Acquired,
    Released,
}

/// One strictly decoded row from the existing schema37 table. Sequence is the
/// SQLite FIFO ordinal, not a clock or an operator-configured priority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorkspaceTurn {
    pub seq: i64,
    pub work_id: String,
    pub task_id: String,
    pub workspace: String,
    pub state: TurnState,
    pub agent_id: String,
    pub requested_at: String,
    pub acquired_at: String,
    pub released_at: String,
}

/// First older overlapping standing turn for one waiting candidate. Identifiers
/// and sequence are observations; no caller may use them as acquisition proof.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorkspaceBlocker {
    pub task_id: String,
    pub candidate_seq: i64,
    pub older_seq: i64,
    pub older_work_id: String,
    pub older_task_id: String,
}

/// Typed rejection or SQL/decoding failure. Finalization failure is retained
/// separately from its primary cause; Display exposes only static diagnostics.
#[derive(Debug)]
pub(crate) enum WorkspaceTurnError {
    Invalid(&'static str),
    Sql(rusqlite::Error),
    Timestamp(swarm_wire_json::go_rfc3339::GoTimestampError),
    Finalize {
        primary: Box<WorkspaceTurnError>,
        cleanup: rusqlite::Error,
    },
}

impl fmt::Display for WorkspaceTurnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(code) => f.write_str(code),
            Self::Sql(_) => f.write_str("workspace-turn-sql"),
            Self::Timestamp(_) => f.write_str("workspace-turn-timestamp-invalid"),
            Self::Finalize { .. } => f.write_str("workspace-turn-read-and-finalize-failed"),
        }
    }
}
impl Error for WorkspaceTurnError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Invalid(_) => None,
            Self::Sql(cause) => Some(cause),
            Self::Timestamp(cause) => Some(cause),
            Self::Finalize { primary, .. } => Some(primary.as_ref()),
        }
    }
}

/// Accept canonical absolute Unix text, including root. Reject ambiguous empty,
/// dot, parent, repeated/trailing separators and NUL; this does no filesystem IO
/// and proves neither containment, inode identity nor directory ownership.
fn canonical_path(path: &str) -> bool {
    path == "/"
        || path.starts_with('/')
            && !path.contains('\0')
            && path[1..]
                .split('/')
                .all(|part| !matches!(part, "" | "." | ".."))
}

/// Match equality and ancestor boundaries in both directions, including `/`.
/// Inputs must pass canonical_path; the separator is Unix representation, not
/// a configurable policy. A shared string prefix alone does not overlap.
fn overlaps(left: &str, right: &str) -> bool {
    left == right
        || left == "/"
        || right == "/"
        || left
            .strip_prefix(right)
            .is_some_and(|tail| tail.starts_with('/'))
        || right
            .strip_prefix(left)
            .is_some_and(|tail| tail.starts_with('/'))
}

/// Require actual SQLite TEXT and valid UTF-8; do not coerce numbers, blobs or
/// NULL into queue identities. SQL conversion errors retain the typed cause.
fn text(row: &Row<'_>, index: usize) -> Result<String, WorkspaceTurnError> {
    let raw = row.get_ref(index).map_err(WorkspaceTurnError::Sql)?;
    match raw {
        ValueRef::Text(bytes) => std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|cause| {
                WorkspaceTurnError::Sql(rusqlite::Error::FromSqlConversionFailure(
                    index,
                    raw.data_type(),
                    Box::new(cause),
                ))
            }),
        _ => Err(WorkspaceTurnError::Sql(rusqlite::Error::InvalidColumnType(
            index,
            "workspace turn text".into(),
            raw.data_type(),
        ))),
    }
}

/// Decode exact storage types and a known state. Full row/collection consistency
/// is checked separately, so a partial malformed projection never escapes.
fn decode(row: &Row<'_>) -> Result<WorkspaceTurn, WorkspaceTurnError> {
    let seq = match row.get_ref(0).map_err(WorkspaceTurnError::Sql)? {
        ValueRef::Integer(value) => value,
        raw => {
            return Err(WorkspaceTurnError::Sql(rusqlite::Error::InvalidColumnType(
                0,
                "workspace turn sequence".into(),
                raw.data_type(),
            )))
        }
    };
    let state = match text(row, 4)?.as_str() {
        "waiting" => TurnState::Waiting,
        "acquired" => TurnState::Acquired,
        "released" => TurnState::Released,
        _ => return Err(WorkspaceTurnError::Invalid("workspace-turn-state-invalid")),
    };
    Ok(WorkspaceTurn {
        seq,
        work_id: text(row, 1)?,
        task_id: text(row, 2)?,
        workspace: text(row, 3)?,
        state,
        agent_id: text(row, 5)?,
        requested_at: text(row, 6)?,
        acquired_at: text(row, 7)?,
        released_at: text(row, 8)?,
    })
}

/// Validate all rows before returning any blockers. Ordinals must be strictly
/// increasing/positive and Work/task pairs unique. State controls required owner
/// and timestamps; a released waiting turn legitimately has no Agent. Timestamps
/// are syntax-checked, never assumed trusted or monotonically increasing clocks.
fn validate(turns: &[WorkspaceTurn]) -> Result<(), WorkspaceTurnError> {
    let mut previous = 0;
    let mut seen = BTreeSet::new();
    for turn in turns {
        if turn.seq <= previous || !seen.insert((&turn.work_id, &turn.task_id)) {
            return Err(WorkspaceTurnError::Invalid(
                "workspace-turn-order-or-duplicate-invalid",
            ));
        }
        previous = turn.seq;
        if !swarm_identity_contract::name(turn.work_id.as_bytes())
            || !swarm_identity_contract::name(turn.task_id.as_bytes())
            || !canonical_path(&turn.workspace)
        {
            return Err(WorkspaceTurnError::Invalid(
                "workspace-turn-identity-or-path-invalid",
            ));
        }
        swarm_wire_json::go_rfc3339::parse(&turn.requested_at)
            .map_err(WorkspaceTurnError::Timestamp)?;
        if !turn.acquired_at.is_empty() {
            swarm_wire_json::go_rfc3339::parse(&turn.acquired_at)
                .map_err(WorkspaceTurnError::Timestamp)?;
        }
        if !turn.released_at.is_empty() {
            swarm_wire_json::go_rfc3339::parse(&turn.released_at)
                .map_err(WorkspaceTurnError::Timestamp)?;
        }
        let unowned = turn.agent_id.is_empty() && turn.acquired_at.is_empty();
        let owned =
            swarm_identity_contract::name(turn.agent_id.as_bytes()) && !turn.acquired_at.is_empty();
        let consistent = match turn.state {
            TurnState::Waiting => unowned && turn.released_at.is_empty(),
            TurnState::Acquired => owned && turn.released_at.is_empty(),
            TurnState::Released => (unowned || owned) && !turn.released_at.is_empty(),
        };
        if !consistent {
            return Err(WorkspaceTurnError::Invalid(
                "workspace-turn-owner-state-invalid",
            ));
        }
    }
    Ok(())
}

/// Join mandatory statement finalization with the read result, preserving both
/// independent failures. No rollback/commit is attempted: the caller owns them.
fn finish<T>(
    result: Result<T, WorkspaceTurnError>,
    cleanup: rusqlite::Result<()>,
) -> Result<T, WorkspaceTurnError> {
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(primary), Ok(())) => Err(primary),
        (Ok(_), Err(cause)) => Err(WorkspaceTurnError::Sql(cause)),
        (Err(primary), Err(cleanup)) => Err(WorkspaceTurnError::Finalize {
            primary: Box::new(primary),
            cleanup,
        }),
    }
}

/// Read the complete existing queue in one caller-owned SQL transaction. An
/// autocommit connection refuses, preventing callers combining unrelated reads
/// as a snapshot. Missing/invalid schema, typed rows, state, timestamps, sequence
/// or finalization refuse the complete read. No table creation, mutation, pruning,
/// clock policy, retry or ownership is provided; caller owes transaction cleanup
/// and authoritative schema/settings/lease checks before any later acquisition.
pub(crate) fn read(db: &Connection) -> Result<Vec<WorkspaceTurn>, WorkspaceTurnError> {
    if db.is_autocommit() {
        return Err(WorkspaceTurnError::Invalid(
            "workspace-turn-read-transaction-required",
        ));
    }
    let mut statement = db.prepare("SELECT seq,work_id,task_id,workspace,state,agent_id,requested_at,acquired_at,released_at FROM main.workspace_turns ORDER BY seq").map_err(WorkspaceTurnError::Sql)?;
    let result = (|| {
        let mut rows = statement.query([]).map_err(WorkspaceTurnError::Sql)?;
        let mut turns = Vec::new();
        while let Some(row) = rows.next().map_err(WorkspaceTurnError::Sql)? {
            turns.push(decode(row)?);
        }
        validate(&turns)?;
        Ok(turns)
    })();
    finish(result, statement.finalize())
}

/// For each waiting turn of work, return its first older overlapping standing
/// turn across all missions. Released rows never block; disjoint trees advance
/// independently. The complete projection is revalidated, even for unknown Work
/// returning no candidates. No write, liveness check, acquisition or permission
/// results; the future writer must recheck in its own authoritative transaction.
pub(crate) fn blockers(
    turns: &[WorkspaceTurn],
    work: &str,
) -> Result<Vec<WorkspaceBlocker>, WorkspaceTurnError> {
    if !swarm_identity_contract::name(work.as_bytes()) {
        return Err(WorkspaceTurnError::Invalid("workspace-turn-work-invalid"));
    }
    validate(turns)?;
    let mut out = Vec::new();
    for candidate in turns
        .iter()
        .filter(|turn| turn.work_id == work && turn.state == TurnState::Waiting)
    {
        if let Some(older) = turns
            .iter()
            .take_while(|turn| turn.seq < candidate.seq)
            .find(|turn| {
                turn.state != TurnState::Released && overlaps(&turn.workspace, &candidate.workspace)
            })
        {
            out.push(WorkspaceBlocker {
                task_id: candidate.task_id.clone(),
                candidate_seq: candidate.seq,
                older_seq: older.seq,
                older_work_id: older.work_id.clone(),
                older_task_id: older.task_id.clone(),
            });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    const AT: &str = "2026-10-22T10:00:00Z";

    /// Deliberately constraint-free SQL projection admits negative types/states;
    /// this is neither a Root nor an authenticated mission/Agent fixture.
    fn fixture() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE workspace_turns(seq,work_id,task_id,workspace,state,agent_id,requested_at,acquired_at,released_at); BEGIN DEFERRED").unwrap();
        db
    }
    /// Insert fixture data only; no positive authority or production acquisition.
    fn insert(db: &Connection, seq: i64, work: &str, task: &str, path: &str, state: &str) {
        let acquired = state == "acquired";
        db.execute(
            "INSERT INTO workspace_turns VALUES(?,?,?,?,?,?,?,?,?)",
            rusqlite::params![
                seq,
                work,
                task,
                path,
                state,
                if acquired { "agent" } else { "" },
                AT,
                if acquired { AT } else { "" },
                if state == "released" { AT } else { "" }
            ],
        )
        .unwrap();
    }

    /// Real SQL read and pure blockers preserve order, released semantics and
    /// total_changes; no row writes occur in read/blockers, including replays.
    #[test]
    fn sql_cross_mission_fifo_disjoint_and_released_without_mutation() {
        let db = fixture();
        insert(&db, 1, "old", "released", "/project", "released");
        insert(&db, 2, "first", "a", "/project/a", "acquired");
        insert(&db, 3, "second", "b", "/project/a/child", "waiting");
        insert(&db, 4, "second", "c", "/project/ab", "waiting");
        insert(&db, 5, "second", "d", "/project", "waiting");
        let before = db.total_changes();
        let turns = read(&db).unwrap();
        let result = blockers(&turns, "second").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!((result[0].task_id.as_str(), result[0].older_seq), ("b", 2));
        assert_eq!((result[1].task_id.as_str(), result[1].older_seq), ("d", 2));
        assert_eq!(blockers(&turns, "unknown").unwrap(), vec![]);
        assert_eq!(read(&db).unwrap(), turns);
        assert_eq!(db.total_changes(), before);
        db.execute_batch("ROLLBACK").unwrap();
    }

    /// Root ancestry is a projection rule, not proof a filesystem-root project
    /// is permitted. Equality/ancestors are symmetric; siblings stay disjoint.
    #[test]
    fn root_and_component_boundaries_with_canonical_path_rejection() {
        for (a, b, expected) in [
            ("/", "/project", true),
            ("/", "/", true),
            ("/project/a", "/project/a", true),
            ("/project/a", "/project/a/b", true),
            ("/project/a", "/project/ab", false),
            ("/a", "/b", false),
        ] {
            assert_eq!(overlaps(a, b), expected);
            assert_eq!(overlaps(b, a), expected);
        }
        for path in [
            "", "relative", "//", "/a/", "/a//b", "/a/./b", "/a/../b", "/a\0b",
        ] {
            assert!(!canonical_path(path));
        }
        let db = fixture();
        insert(&db, 1, "first", "a", "/", "waiting");
        insert(&db, 2, "second", "b", "/project", "waiting");
        assert_eq!(
            blockers(&read(&db).unwrap(), "second").unwrap()[0].older_seq,
            1
        );
        db.execute_batch("ROLLBACK").unwrap();
    }

    /// Each malformed SQL case follows a valid witness with the same schema;
    /// refusal performs no write, and exact restore recovers the original read.
    #[test]
    fn malformed_sql_types_states_and_owners_refuse_and_restore() {
        let db = fixture();
        insert(&db, 1, "work", "task", "/project", "waiting");
        let original = read(&db).unwrap();
        for mutation in [
            "seq='1'",
            "seq=1.0",
            "seq=0",
            "work_id=x'776f726b'",
            "task_id=NULL",
            "workspace='/project/'",
            "workspace='/project/../escape'",
            "state='unknown'",
            "agent_id='agent'",
            "requested_at='bad'",
            "acquired_at='2026-10-22T10:00:00Z'",
            "released_at='2026-10-22T10:00:00Z'",
        ] {
            db.execute_batch("SAVEPOINT negative").unwrap();
            db.execute_batch(&format!("UPDATE workspace_turns SET {mutation}"))
                .unwrap();
            let before = db.total_changes();
            assert!(read(&db).is_err(), "negative fixture accepted: {mutation}");
            assert_eq!(db.total_changes(), before);
            db.execute_batch("ROLLBACK TO negative; RELEASE negative")
                .unwrap();
            assert_eq!(read(&db).unwrap(), original);
        }
        db.execute_batch("ROLLBACK").unwrap();
    }

    /// Duplicate identity/ordinal and malformed external projections refuse
    /// rather than producing partial blockers; released owned history is valid.
    #[test]
    fn duplicate_and_reordered_projections_refuse() {
        let db = fixture();
        insert(&db, 1, "first", "a", "/project", "waiting");
        insert(&db, 2, "second", "b", "/project", "waiting");
        let rows = read(&db).unwrap();
        let mut duplicate = rows.clone();
        duplicate[1].work_id = "first".into();
        duplicate[1].task_id = "a".into();
        assert!(blockers(&duplicate, "second").is_err());
        let mut reordered = rows.clone();
        reordered.reverse();
        assert!(blockers(&reordered, "second").is_err());
        insert(&db, 2, "third", "c", "/elsewhere", "waiting");
        assert!(read(&db).is_err());
        db.execute_batch("DELETE FROM workspace_turns WHERE work_id='third'; UPDATE workspace_turns SET state='released',agent_id='agent',acquired_at='2026-10-22T10:00:00Z',released_at='2026-10-22T09:00:00Z' WHERE seq=1").unwrap();
        assert!(blockers(&read(&db).unwrap(), "second").unwrap().is_empty());
        db.execute_batch("ROLLBACK").unwrap();
    }

    /// Actual missing-table SQL preserves its typed source; autocommit never
    /// silently creates a snapshot. Cleanup-join branch is a synthetic error
    /// propagation unit, not a claimed real SQLite finalize fault injection.
    #[test]
    fn transaction_and_sql_errors_preserve_sources_and_cleanup() {
        let db = Connection::open_in_memory().unwrap();
        assert!(matches!(read(&db), Err(WorkspaceTurnError::Invalid(_))));
        db.execute_batch("BEGIN DEFERRED").unwrap();
        assert!(matches!(read(&db), Err(WorkspaceTurnError::Sql(_))));
        db.execute_batch("ROLLBACK").unwrap();
        let joined = finish::<()>(
            Err(WorkspaceTurnError::Invalid("primary")),
            Err(rusqlite::Error::InvalidQuery),
        )
        .unwrap_err();
        assert!(joined.source().is_some());
        match joined {
            WorkspaceTurnError::Finalize { primary, cleanup } => {
                assert!(matches!(*primary, WorkspaceTurnError::Invalid("primary")));
                assert!(matches!(cleanup, rusqlite::Error::InvalidQuery));
            }
            other => panic!("missing joined causes: {other}"),
        }
    }
}
