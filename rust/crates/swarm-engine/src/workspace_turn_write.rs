//! Transaction-local FIFO transitions. These helpers grant no lease, consent,
//! Agent authenticity or execution permission. The caller owns authority checks,
//! schema validation, a write transaction, commit and rollback on EVERY error.
use crate::workspace_turn::{self, TurnState, WorkspaceTurn, WorkspaceTurnError as E};
use rusqlite::{params, Connection, Params};
use std::collections::BTreeSet;

/// Exact auxiliary representation. No FK to the replaceable active row: released
/// generations must survive re-enqueue. Ordinals and state words are protocol,
/// not operator policy. The Root validates this reserved schema before calling.
pub(crate) const JOURNAL_DDL: &str = "CREATE TABLE workspace_turn_events(event_seq INTEGER PRIMARY KEY AUTOINCREMENT,turn_seq INTEGER NOT NULL,work_id TEXT NOT NULL,task_id TEXT NOT NULL,workspace TEXT NOT NULL,state TEXT NOT NULL,agent_id TEXT NOT NULL,requested_at TEXT NOT NULL,acquired_at TEXT NOT NULL,released_at TEXT NOT NULL,transition TEXT NOT NULL)";

/// Authoritatively eligible candidate supplied in deterministic dispatch order.
/// Canonical path text is checked here; filesystem resolution/containment and
/// actual profile/permission checks belong to the caller, never this value.
pub(crate) struct Candidate<'a> {
    pub task_id: &'a str,
    pub workspace: &'a str,
}

/// Join statement execution/finalization without losing either typed cause.
/// This performs no transaction cleanup; callers must roll back on any error.
fn finish<T>(result: Result<T, E>, cleanup: rusqlite::Result<()>) -> Result<T, E> {
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(primary), Ok(())) => Err(primary),
        (Ok(_), Err(cause)) => Err(E::Sql(cause)),
        (Err(primary), Err(cleanup)) => Err(E::Finalize {
            primary: Box::new(primary),
            cleanup,
        }),
    }
}

/// Execute parameterized SQL and explicitly finalize. A failure after an earlier
/// write is NOT evidence of no mutation: the encompassing transaction must roll back.
fn execute(db: &Connection, sql: &str, values: impl Params) -> Result<usize, E> {
    let mut statement = db.prepare(sql).map_err(E::Sql)?;
    let result = statement.execute(values).map_err(E::Sql);
    finish(result, statement.finalize())
}

/// Validate canonical Unix text without touching the filesystem or asserting
/// ownership. Root `/` is represented correctly; containment is a caller check.
fn canonical(path: &str) -> bool {
    path == "/"
        || path.starts_with('/')
            && !path.contains('\0')
            && path[1..]
                .split('/')
                .all(|part| !matches!(part, "" | "." | ".."))
}

/// Validate non-deserializable-call prerequisites and protocol inputs. Merely
/// being in a transaction does not prove it is an authorized write transaction.
fn inputs(db: &Connection, work: &str, at: &str) -> Result<(), E> {
    if db.is_autocommit() {
        return Err(E::Invalid("workspace-turn-write-transaction-required"));
    }
    if !swarm_identity_contract::name(work.as_bytes()) {
        return Err(E::Invalid("workspace-turn-work-invalid"));
    }
    swarm_wire_json::go_rfc3339::parse(at).map_err(E::Timestamp)?;
    Ok(())
}

/// Append one complete historical snapshot. No UPDATE/DELETE of this journal is
/// performed. INSERT/finalize errors propagate and require caller rollback.
fn journal(db: &Connection, turn: &WorkspaceTurn, transition: &str) -> Result<(), E> {
    let state = match turn.state {
        TurnState::Waiting => "waiting",
        TurnState::Acquired => "acquired",
        TurnState::Released => "released",
    };
    let changed = execute(db, "INSERT INTO main.workspace_turn_events(turn_seq,work_id,task_id,workspace,state,agent_id,requested_at,acquired_at,released_at,transition) VALUES(?,?,?,?,?,?,?,?,?,?)",
        params![turn.seq,turn.work_id,turn.task_id,turn.workspace,state,turn.agent_id,turn.requested_at,turn.acquired_at,turn.released_at,transition])?;
    if changed != 1 {
        return Err(E::Invalid("workspace-turn-journal-write-missing"));
    }
    Ok(())
}

/// Read complete snapshots for one exact historical generation, including a
/// generation replaced in the active table. Empty is not permission. Exact SQL
/// types, timestamps, identity, owner and state are checked; Root owes schema and
/// Binding provenance validation. Read/finalize errors require caller cleanup.
pub(crate) fn history(db: &Connection, seq: i64) -> Result<Vec<WorkspaceTurn>, E> {
    if db.is_autocommit() || seq <= 0 {
        return Err(E::Invalid("workspace-turn-history-input-invalid"));
    }
    let mut statement = db.prepare("SELECT event_seq,turn_seq,work_id,task_id,workspace,state,agent_id,requested_at,acquired_at,released_at,transition FROM main.workspace_turn_events WHERE turn_seq=? ORDER BY event_seq").map_err(E::Sql)?;
    let result = (|| {
        let mut rows = statement.query([seq]).map_err(E::Sql)?;
        let mut previous = 0;
        let mut out: Vec<WorkspaceTurn> = Vec::new();
        while let Some(row) = rows.next().map_err(E::Sql)? {
            let event: i64 = row.get(0).map_err(E::Sql)?;
            let actual: i64 = row.get(1).map_err(E::Sql)?;
            let state: String = row.get(5).map_err(E::Sql)?;
            let transition: String = row.get(10).map_err(E::Sql)?;
            if event <= previous || actual != seq {
                return Err(E::Invalid("workspace-turn-history-order-invalid"));
            }
            previous = event;
            let state = match state.as_str() {
                "waiting" => TurnState::Waiting,
                "acquired" => TurnState::Acquired,
                "released" => TurnState::Released,
                _ => return Err(E::Invalid("workspace-turn-history-state-invalid")),
            };
            let turn = WorkspaceTurn {
                seq: actual,
                work_id: row.get(2).map_err(E::Sql)?,
                task_id: row.get(3).map_err(E::Sql)?,
                workspace: row.get(4).map_err(E::Sql)?,
                state,
                agent_id: row.get(6).map_err(E::Sql)?,
                requested_at: row.get(7).map_err(E::Sql)?,
                acquired_at: row.get(8).map_err(E::Sql)?,
                released_at: row.get(9).map_err(E::Sql)?,
            };
            inputs(db, &turn.work_id, &turn.requested_at)?;
            if !swarm_identity_contract::name(turn.task_id.as_bytes())
                || !canonical(&turn.workspace)
            {
                return Err(E::Invalid("workspace-turn-history-binding-invalid"));
            }
            for at in [&turn.acquired_at, &turn.released_at] {
                if !at.is_empty() {
                    swarm_wire_json::go_rfc3339::parse(at).map_err(E::Timestamp)?;
                }
            }
            let unowned = turn.agent_id.is_empty() && turn.acquired_at.is_empty();
            let owned = swarm_identity_contract::name(turn.agent_id.as_bytes())
                && !turn.acquired_at.is_empty();
            let valid = match turn.state {
                TurnState::Waiting => unowned && turn.released_at.is_empty(),
                TurnState::Acquired => owned && turn.released_at.is_empty(),
                TurnState::Released => (unowned || owned) && !turn.released_at.is_empty(),
            };
            if !valid
                || !matches!(
                    (transition.as_str(), turn.state),
                    ("retained", _)
                        | ("enqueued", TurnState::Waiting)
                        | ("acquired", TurnState::Acquired)
                        | ("released", TurnState::Released)
                )
            {
                return Err(E::Invalid("workspace-turn-history-owner-invalid"));
            }
            if let Some(first) = out.first() {
                if first.work_id != turn.work_id
                    || first.task_id != turn.task_id
                    || first.workspace != turn.workspace
                    || first.requested_at != turn.requested_at
                {
                    return Err(E::Invalid("workspace-turn-history-generation-invalid"));
                }
            }
            if let Some(previous) = out.last() {
                let transition_valid = match transition.as_str() {
                    "acquired" => previous.state == TurnState::Waiting,
                    "released" => {
                        matches!(previous.state, TurnState::Waiting | TurnState::Acquired)
                            && previous.agent_id == turn.agent_id
                            && previous.acquired_at == turn.acquired_at
                    }
                    _ => false,
                };
                if !transition_valid {
                    return Err(E::Invalid("workspace-turn-history-transition-invalid"));
                }
            } else if !matches!(transition.as_str(), "retained" | "enqueued") {
                return Err(E::Invalid("workspace-turn-history-origin-invalid"));
            }
            out.push(turn);
        }
        Ok(out)
    })();
    finish(result, statement.finalize())
}

/// Preserve the old row on its first writer mutation, including imported schema37
/// history with no native journal. Existing snapshots are not rewritten. This
/// does not authenticate pre-existing history; Root schema/input validation remains required.
fn retain(db: &Connection, turn: &WorkspaceTurn) -> Result<(), E> {
    let snapshots = history(db, turn.seq)?;
    match snapshots.last() {
        None => journal(db, turn, "retained")?,
        Some(last) if last == turn => {}
        Some(_) => return Err(E::Invalid("workspace-turn-history-current-conflict")),
    }
    Ok(())
}

/// Change one waiting/acquired row to released, retaining its old snapshot and
/// recording its new one. The caller established whether physical state allows
/// release; this primitive provides no liveness decision. Any failure needs rollback.
fn release_row(db: &Connection, turn: &WorkspaceTurn, at: &str) -> Result<WorkspaceTurn, E> {
    retain(db, turn)?;
    let changed = execute(db, "UPDATE main.workspace_turns SET state='released',released_at=? WHERE seq=? AND work_id=? AND task_id=? AND state=? AND agent_id=?",
        params![at,turn.seq,turn.work_id,turn.task_id,match turn.state { TurnState::Waiting => "waiting", TurnState::Acquired => "acquired", TurnState::Released => "released" },turn.agent_id])?;
    if changed != 1 {
        return Err(E::Invalid("workspace-turn-release-conflict"));
    }
    let mut released = turn.clone();
    released.state = TurnState::Released;
    released.released_at = at.into();
    journal(db, &released, "released")?;
    Ok(released)
}

/// Synchronize the complete eligible candidate set for ONE Work, in supplied
/// deterministic order. Existing waiting candidates keep their sequence; omitted
/// waiting candidates release, and changed waiting paths requeue at the tail.
/// Acquired rows never release here, including omitted candidates; a changed
/// acquired path refuses before writes. Released rows are retained in the journal
/// before replacement. New rows get SQLite's real AUTOINCREMENT ordinal.
///
/// Requires validated schema/current lease/eligibility and a caller-owned write
/// transaction. Candidate paths must already be filesystem-canonical. No provider,
/// commit, rollback, timeout, retry, priority cutoff or execution grant occurs.
/// Returns all current rows for the Work. EVERY error requires caller rollback,
/// including errors after writes and final snapshot validation failures.
pub(crate) fn sync_waiting(
    db: &Connection,
    work: &str,
    candidates: &[Candidate<'_>],
    at: &str,
) -> Result<Vec<WorkspaceTurn>, E> {
    inputs(db, work, at)?;
    let turns = workspace_turn::read(db)?;
    let mut seen = BTreeSet::new();
    for candidate in candidates {
        if !swarm_identity_contract::name(candidate.task_id.as_bytes())
            || !canonical(candidate.workspace)
            || !seen.insert(candidate.task_id)
        {
            return Err(E::Invalid("workspace-turn-candidate-invalid"));
        }
        if turns.iter().any(|t| {
            t.work_id == work
                && t.task_id == candidate.task_id
                && t.state == TurnState::Acquired
                && t.workspace != candidate.workspace
        }) {
            return Err(E::Invalid("workspace-turn-acquired-path-conflict"));
        }
    }
    for turn in turns
        .iter()
        .filter(|t| t.work_id == work && t.state == TurnState::Waiting)
    {
        if !candidates
            .iter()
            .any(|c| c.task_id == turn.task_id && c.workspace == turn.workspace)
        {
            release_row(db, turn, at)?;
        }
    }
    for candidate in candidates {
        let current = workspace_turn::read(db)?
            .into_iter()
            .find(|t| t.work_id == work && t.task_id == candidate.task_id);
        if let Some(turn) = current {
            if turn.state != TurnState::Released {
                continue;
            }
            retain(db, &turn)?;
            if execute(db, "DELETE FROM main.workspace_turns WHERE seq=? AND work_id=? AND task_id=? AND state='released'", params![turn.seq,work,candidate.task_id])? != 1 {
                return Err(E::Invalid("workspace-turn-requeue-conflict"));
            }
        }
        if execute(db, "INSERT INTO main.workspace_turns(work_id,task_id,workspace,state,requested_at) VALUES(?,?,?,'waiting',?)", params![work,candidate.task_id,candidate.workspace,at])? != 1 {
            return Err(E::Invalid("workspace-turn-enqueue-missing"));
        }
        let inserted = workspace_turn::read(db)?
            .into_iter()
            .find(|t| t.work_id == work && t.task_id == candidate.task_id)
            .ok_or(E::Invalid("workspace-turn-enqueue-missing"))?;
        journal(db, &inserted, "enqueued")?;
    }
    Ok(workspace_turn::read(db)?
        .into_iter()
        .filter(|t| t.work_id == work)
        .collect())
}

/// Find the exact generation and Work/task/workspace binding, refusing stale
/// references after requeue. This compares observations; caller proves authority.
fn exact(
    db: &Connection,
    seq: i64,
    work: &str,
    task: &str,
    workspace: &str,
) -> Result<WorkspaceTurn, E> {
    if seq <= 0 || !swarm_identity_contract::name(task.as_bytes()) || !canonical(workspace) {
        return Err(E::Invalid("workspace-turn-binding-invalid"));
    }
    let turn = workspace_turn::read(db)?
        .into_iter()
        .find(|t| {
            t.seq == seq && t.work_id == work && t.task_id == task && t.workspace == workspace
        })
        .ok_or(E::Invalid("workspace-turn-binding-stale"))?;
    if history(db, seq)?.last().is_some_and(|last| last != &turn) {
        return Err(E::Invalid("workspace-turn-history-current-conflict"));
    }
    Ok(turn)
}

/// Acquire the exact first-standing overlapping turn for an actual admitted
/// Agent. Same-generation/same-owner acquired replay is read-only; another owner,
/// released generation or earlier waiting/acquired overlap refuses. The caller
/// must bind this seq to its durable Agent intent in THIS transaction and perform
/// all lease/consent/settings/physical-resource checks. An Agent string is no proof.
/// Any error (including journal/finalize after UPDATE) requires caller rollback.
#[allow(clippy::too_many_arguments)]
pub(crate) fn acquire(
    db: &Connection,
    seq: i64,
    work: &str,
    task: &str,
    workspace: &str,
    agent: &str,
    at: &str,
) -> Result<WorkspaceTurn, E> {
    inputs(db, work, at)?;
    if !swarm_identity_contract::name(agent.as_bytes()) {
        return Err(E::Invalid("workspace-turn-agent-invalid"));
    }
    let turn = exact(db, seq, work, task, workspace)?;
    if turn.state == TurnState::Acquired && turn.agent_id == agent {
        return Ok(turn);
    }
    if turn.state != TurnState::Waiting {
        return Err(E::Invalid("workspace-turn-acquire-owner-conflict"));
    }
    let rows = workspace_turn::read(db)?;
    if workspace_turn::blockers(&rows, work)?
        .iter()
        .any(|b| b.candidate_seq == seq)
    {
        return Err(E::Invalid("workspace-turn-fifo-blocked"));
    }
    retain(db, &turn)?;
    if execute(db, "UPDATE main.workspace_turns SET state='acquired',agent_id=?,acquired_at=? WHERE seq=? AND work_id=? AND task_id=? AND state='waiting' AND agent_id=''", params![agent,at,seq,work,task])? != 1 {
        return Err(E::Invalid("workspace-turn-acquire-conflict"));
    }
    let mut acquired = turn;
    acquired.state = TurnState::Acquired;
    acquired.agent_id = agent.into();
    acquired.acquired_at = at.into();
    journal(db, &acquired, "acquired")?;
    Ok(acquired)
}

/// Release an exact acquired generation after the caller proves safe terminal or
/// cancelled state. Same released owner replay is read-only; waiting, stale seq,
/// wrong Work/task/path/Agent refuse. No expiry or process-liveness inference is
/// made, so uncertain execution must remain acquired. Caller rolls back ANY Err.
#[allow(clippy::too_many_arguments)]
pub(crate) fn release(
    db: &Connection,
    seq: i64,
    work: &str,
    task: &str,
    workspace: &str,
    agent: &str,
    at: &str,
) -> Result<WorkspaceTurn, E> {
    inputs(db, work, at)?;
    if !swarm_identity_contract::name(agent.as_bytes()) {
        return Err(E::Invalid("workspace-turn-agent-invalid"));
    }
    let turn = exact(db, seq, work, task, workspace)?;
    if turn.agent_id != agent || turn.state == TurnState::Waiting {
        return Err(E::Invalid("workspace-turn-release-owner-conflict"));
    }
    if turn.state == TurnState::Released {
        return Ok(turn);
    }
    release_row(db, &turn, at)
}

#[cfg(test)]
mod tests {
    use super::*;
    const AT: &str = "2026-10-22T10:00:00Z";
    const LATER: &str = "2026-10-22T10:01:00Z";

    /// Exact active/journal representation with real SQLite transactions. Work
    /// and Agent identities here are SQL fixture data, never native authority.
    fn fixture() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE works(id TEXT PRIMARY KEY); INSERT INTO works VALUES('first'),('second'); CREATE TABLE workspace_turns(seq INTEGER PRIMARY KEY AUTOINCREMENT,work_id TEXT NOT NULL REFERENCES works(id),task_id TEXT NOT NULL,workspace TEXT NOT NULL,state TEXT NOT NULL CHECK(state IN ('waiting','acquired','released')),agent_id TEXT NOT NULL DEFAULT '',requested_at TEXT NOT NULL,acquired_at TEXT NOT NULL DEFAULT '',released_at TEXT NOT NULL DEFAULT '',UNIQUE(work_id,task_id));").unwrap();
        db.execute_batch(JOURNAL_DDL).unwrap();
        db.execute_batch("BEGIN IMMEDIATE").unwrap();
        db
    }
    /// Construct fixture input whose order is deliberately supplied by its caller.
    fn candidate<'a>(task: &'a str, path: &'a str) -> Candidate<'a> {
        Candidate {
            task_id: task,
            workspace: path,
        }
    }
    /// Count persisted journal rows; an equal count proves no journal append only
    /// when combined with the corresponding complete active-row assertions.
    fn count(db: &Connection) -> i64 {
        db.query_row("SELECT count(*) FROM workspace_turn_events", [], |r| {
            r.get(0)
        })
        .unwrap()
    }

    /// Real ordering across missions blocks ancestors, admits component siblings,
    /// keeps waiting place on synchronization and makes acquired replay read-only.
    #[test]
    fn sql_fifo_cross_mission_root_siblings_and_idempotent_acquisition() {
        let db = fixture();
        let first =
            sync_waiting(&db, "first", &[candidate("a", "/project/a")], AT).unwrap()[0].clone();
        let second = sync_waiting(
            &db,
            "second",
            &[
                candidate("b", "/project/a/child"),
                candidate("c", "/project/ab"),
            ],
            AT,
        )
        .unwrap();
        assert!(second[0].seq > first.seq);
        let changes = db.total_changes();
        assert_eq!(
            sync_waiting(
                &db,
                "second",
                &[
                    candidate("c", "/project/ab"),
                    candidate("b", "/project/a/child")
                ],
                LATER
            )
            .unwrap(),
            second
        );
        assert_eq!(db.total_changes(), changes);
        assert!(matches!(
            acquire(
                &db,
                second[0].seq,
                "second",
                "b",
                "/project/a/child",
                "agent-b",
                AT
            ),
            Err(E::Invalid("workspace-turn-fifo-blocked"))
        ));
        let acquired = acquire(&db, first.seq, "first", "a", "/project/a", "agent-a", AT).unwrap();
        acquire(
            &db,
            second[1].seq,
            "second",
            "c",
            "/project/ab",
            "agent-c",
            AT,
        )
        .unwrap();
        let changes = db.total_changes();
        assert_eq!(
            acquire(&db, first.seq, "first", "a", "/project/a", "agent-a", LATER).unwrap(),
            acquired
        );
        assert_eq!(db.total_changes(), changes);
        assert!(acquire(&db, first.seq, "first", "a", "/project/a", "impostor", AT).is_err());
        release(&db, first.seq, "first", "a", "/project/a", "agent-a", LATER).unwrap();
        acquire(
            &db,
            second[0].seq,
            "second",
            "b",
            "/project/a/child",
            "agent-b",
            LATER,
        )
        .unwrap();
        db.execute_batch("ROLLBACK").unwrap();
        let root = sync_waiting(&db, "first", &[candidate("root", "/")], AT);
        assert!(
            root.is_err(),
            "autocommit must refuse, not create transactions"
        );
        db.execute_batch("BEGIN IMMEDIATE").unwrap();
        sync_waiting(&db, "first", &[candidate("root", "/")], AT).unwrap();
        let b = sync_waiting(&db, "second", &[candidate("b", "/project")], AT).unwrap()[0].clone();
        assert!(matches!(
            acquire(&db, b.seq, "second", "b", "/project", "agent", AT),
            Err(E::Invalid("workspace-turn-fifo-blocked"))
        ));
        db.execute_batch("ROLLBACK").unwrap();
    }

    /// Acquired rows survive omitted candidates and reject path changes. Released
    /// re-enqueue gets a new actual seq while old terminal history survives, and
    /// an old seq cannot release or acquire the replacement generation.
    #[test]
    fn sql_release_requeue_preserves_history_and_rejects_stale_owner() {
        let db = fixture();
        let a = sync_waiting(&db, "first", &[candidate("a", "/project")], AT).unwrap()[0].clone();
        acquire(&db, a.seq, "first", "a", "/project", "agent", AT).unwrap();
        assert_eq!(
            sync_waiting(&db, "first", &[], LATER).unwrap()[0].state,
            TurnState::Acquired
        );
        let before = workspace_turn::read(&db).unwrap();
        let n = count(&db);
        assert!(sync_waiting(&db, "first", &[candidate("a", "/elsewhere")], LATER).is_err());
        assert_eq!(workspace_turn::read(&db).unwrap(), before);
        assert_eq!(count(&db), n);
        assert!(release(&db, a.seq, "first", "a", "/project", "wrong", LATER).is_err());
        let released = release(&db, a.seq, "first", "a", "/project", "agent", LATER).unwrap();
        let changes = db.total_changes();
        assert_eq!(
            release(&db, a.seq, "first", "a", "/project", "agent", AT).unwrap(),
            released
        );
        assert_eq!(db.total_changes(), changes);
        let b =
            sync_waiting(&db, "first", &[candidate("a", "/elsewhere")], LATER).unwrap()[0].clone();
        assert!(b.seq > a.seq);
        assert_eq!(history(&db, a.seq).unwrap().last(), Some(&released));
        assert!(release(&db, a.seq, "first", "a", "/project", "agent", LATER).is_err());
        assert!(acquire(&db, a.seq, "first", "a", "/project", "agent", LATER).is_err());
        assert_eq!(history(&db, b.seq).unwrap(), vec![b]);
        db.execute_batch("ROLLBACK").unwrap();
    }

    /// Waiting changes release and requeue at the tail; obsolete waiting rows
    /// release, and malformed/duplicate candidate sets refuse before any write.
    #[test]
    fn sql_waiting_sync_obsolete_path_change_and_invalid_inputs() {
        let db = fixture();
        let old = sync_waiting(
            &db,
            "first",
            &[candidate("a", "/a"), candidate("b", "/b")],
            AT,
        )
        .unwrap();
        let now = sync_waiting(&db, "first", &[candidate("a", "/new")], LATER).unwrap();
        let a = now.iter().find(|t| t.task_id == "a").unwrap();
        let b = now.iter().find(|t| t.task_id == "b").unwrap();
        assert!(a.seq > old[1].seq);
        assert_eq!(b.state, TurnState::Released);
        assert_eq!(
            history(&db, old[0].seq).unwrap().last().unwrap().state,
            TurnState::Released
        );
        for invalid in [
            vec![candidate("a", "relative")],
            vec![candidate("a", "/x/..")],
            vec![candidate("a", "/x/")],
            vec![candidate("a", "/x"), candidate("a", "/x")],
            vec![candidate("", "/x")],
        ] {
            let changes = db.total_changes();
            assert!(sync_waiting(&db, "first", &invalid, LATER).is_err());
            assert_eq!(db.total_changes(), changes);
        }
        assert!(sync_waiting(&db, "first", &[], "invalid").is_err());
        db.execute_batch("ROLLBACK").unwrap();
    }

    /// The valid acquire witness precedes a real late journal INSERT failure on
    /// the SAME open SQL connection. The error can follow a row UPDATE: rollback
    /// restores both tables exactly, then removal of only the owned trigger lets
    /// the same operation recover. This does not claim a real finalize fault.
    #[test]
    fn sql_late_journal_failure_requires_rollback_and_recovers() {
        let db = fixture();
        let a = sync_waiting(&db, "first", &[candidate("a", "/a")], AT).unwrap()[0].clone();
        db.execute_batch("SAVEPOINT valid").unwrap();
        acquire(&db, a.seq, "first", "a", "/a", "agent", AT).unwrap();
        db.execute_batch("ROLLBACK TO valid; RELEASE valid")
            .unwrap();
        let before = workspace_turn::read(&db).unwrap();
        let hist = history(&db, a.seq).unwrap();
        db.execute_batch("CREATE TEMP TRIGGER owned_fault BEFORE INSERT ON main.workspace_turn_events WHEN NEW.transition='acquired' BEGIN SELECT RAISE(ABORT,'fixture journal refusal'); END; SAVEPOINT fault").unwrap();
        let error = acquire(&db, a.seq, "first", "a", "/a", "agent", AT).unwrap_err();
        assert!(matches!(error, E::Sql(_)));
        assert_eq!(
            workspace_turn::read(&db).unwrap()[0].state,
            TurnState::Acquired,
            "Err does not prove no mutation before caller rollback"
        );
        db.execute_batch("ROLLBACK TO fault; RELEASE fault; DROP TRIGGER owned_fault")
            .unwrap();
        assert_eq!(workspace_turn::read(&db).unwrap(), before);
        assert_eq!(history(&db, a.seq).unwrap(), hist);
        acquire(&db, a.seq, "first", "a", "/a", "agent", AT).unwrap();
        db.execute_batch("ROLLBACK").unwrap();
    }

    /// Imported schema37 state has no native proof retroactively. Its exact old
    /// snapshot is nevertheless retained before release/re-enqueue; corrupt
    /// journal ownership or typed data refuses rather than hiding the mismatch.
    #[test]
    fn sql_legacy_retention_and_corrupt_journal_refusal() {
        let db = fixture();
        db.execute("INSERT INTO workspace_turns(work_id,task_id,workspace,state,requested_at) VALUES('first','a','/a','waiting',?)",[AT]).unwrap();
        let a = workspace_turn::read(&db).unwrap()[0].clone();
        assert!(history(&db, a.seq).unwrap().is_empty());
        acquire(&db, a.seq, "first", "a", "/a", "agent", AT).unwrap();
        assert_eq!(history(&db, a.seq).unwrap()[0], a);
        db.execute_batch("SAVEPOINT invalid; UPDATE workspace_turn_events SET agent_id='wrong' WHERE transition='acquired'").unwrap();
        assert!(acquire(&db, a.seq, "first", "a", "/a", "agent", AT).is_err());
        db.execute_batch("ROLLBACK TO invalid; RELEASE invalid")
            .unwrap();
        release(&db, a.seq, "first", "a", "/a", "agent", LATER).unwrap();
        db.execute_batch("ROLLBACK").unwrap();
    }
}
