//! Owned projection fixture transport. Pure Work input is not a stored actor,
//! execution receipt or acceptance proof; root mode reads the actual stored row.
use serde::Deserialize;
use std::{
    error::Error,
    io::{self, Write},
    path::Path,
    process::ExitCode,
};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pure {
    work: Box<serde_json::value::RawValue>,
    language: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    work: String,
    language: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Text {
    text: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Decisions {
    events: Vec<String>,
}
fn language(s: &str) -> Result<swarm_engine::WorkViewLanguage, Box<dyn Error>> {
    match s {
        "fr" => Ok(swarm_engine::WorkViewLanguage::French),
        "en" => Ok(swarm_engine::WorkViewLanguage::English),
        _ => Err("fr or en required".into()),
    }
}
/// Read explicit marked fixtures only. All roots/DBs are checked closed before
/// emitting success; parsing/read/cleanup/output failures return code2. Text and
/// decision modes demonstrate saved-policy consumption, not full Work rendering.
fn run() -> Result<u8, Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 3 {
        return Err("mode and owned fixture root required".into());
    }
    let root = Path::new(&args[2]);
    if !root.is_absolute()
        || !std::fs::symlink_metadata(root.join(".rust-migration-fixture"))?
            .file_type()
            .is_file()
    {
        return Err("owned fixture marker required".into());
    }
    let raw = swarm_engine::read_command_document(&mut io::stdin().lock())?;
    if args[1] == "seed-work" {
        seed_work(root, &raw)?;
        let mut out = io::stdout().lock();
        out.write_all(b"{\"fixture_data_only\":true}\n")?;
        out.flush()?;
        return Ok(0);
    }
    if args[1] == "seed" {
        let frame: Stored = serde_json::from_slice(&raw)?;
        seed_history(root, &frame.work)?;
        let mut out = io::stdout().lock();
        out.write_all(b"{\"fixture_data_only\":true}\n")?;
        out.flush()?;
        return Ok(0);
    }

    let (code, value) = if args[1] == "pure" {
        let frame: Pure = serde_json::from_slice(&raw)?;
        let work = swarm_engine::decode_work(frame.work.get().as_bytes())?;
        let language = language(&frame.language)?;
        let settings = swarm_engine::EngineSettingsDocument::initial(
            swarm_engine::EngineSettingsValues::installation_defaults()?,
            "fixture".into(),
            "2026-10-18T00:00:00Z".into(),
        )?;
        let db = rusqlite::Connection::open_with_flags(
            root.join("history.db"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        // Owned, serialized fixture: expose unexpected contention without an implicit library wait.
        db.busy_timeout(std::time::Duration::ZERO)?;
        let result =
            swarm_engine::fixture_project_work_validation(&db, root, &work, &settings, language);
        let result = match (result, db.close()) {
            (result, Ok(())) => result,
            (Ok(_), Err((_, close))) => return Err(Box::new(close)),
            (Err(primary), Err((_, close))) => {
                return Err(Box::new(SqlCombined {
                    primary: Box::new(primary),
                    cleanup: close,
                }))
            }
        };
        match result {
            Ok(v) => (0, serde_json::json!({"validation":v})),
            Err(e) => (2, serde_json::json!({"code":e.code(),"reason":e.reason()})),
        }
    } else {
        let store = swarm_engine::RootStore::open(root, false)?;
        // Defer error propagation until the owned root is checked closed.
        let result = (|| -> Result<serde_json::Value, Box<dyn Error>> {
            if args[1] == "root" {
                let f: Stored = serde_json::from_slice(&raw)?;
                return Ok(
                    serde_json::json!({"observation":store.inspect_work_validation(&f.work,language(&f.language)?)?}),
                );
            }
            let settings = store.settings()?;
            let limits = settings.values.work_view_limits()?;
            if args[1] == "text" {
                let f: Text = serde_json::from_slice(&raw)?;
                return Ok(
                    serde_json::json!({"text":swarm_engine::wrap_work_text(&f.text,limits.wrap_characters)?}),
                );
            }
            if args[1] == "decisions" {
                let f: Decisions = serde_json::from_slice(&raw)?;
                let events: Vec<_> = f
                    .events
                    .into_iter()
                    .enumerate()
                    .map(|(i, kind)| swarm_engine::RecordedEvent {
                        id: i.to_string().into_bytes(),
                        work_id: vec![],
                        revision: i as i64,
                        kind: kind.into_bytes(),
                        at: vec![],
                        payload: None,
                    })
                    .collect();
                return Ok(
                    serde_json::json!({"revisions":swarm_engine::recent_work_decisions(&events,limits.recent_decisions).iter().map(|e|e.revision).collect::<Vec<_>>()}),
                );
            }
            Err("known mode required".into())
        })();
        let closed = store.close();
        match (result, closed) {
            (Ok(v), Ok(())) => (0, v),
            (Err(e), Ok(())) => return Err(e),
            (Ok(_), Err(e)) => return Err(Box::new(e)),
            (Err(primary), Err(cleanup)) => return Err(Box::new(Combined { primary, cleanup })),
        }
    };
    let mut out = io::stdout().lock();
    out.write_all(&swarm_engine::encode_model(&value)?)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(code)
}
/// Retain both failed observation and checked release without private rendering.
struct Combined {
    primary: Box<dyn Error>,
    cleanup: swarm_engine::RootStoreError,
}
impl std::fmt::Debug for Combined {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("projection-fixture-cleanup")
    }
}
impl std::fmt::Display for Combined {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "projection-fixture-cleanup: {}", self.cleanup.code())
    }
}
impl Error for Combined {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.primary.as_ref())
    }
}
fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}

/// Retain a failed observation plus failed SQLite cleanup without private output.
struct SqlCombined {
    primary: Box<dyn Error>,
    cleanup: rusqlite::Error,
}
impl std::fmt::Debug for SqlCombined {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("projection-sql-cleanup")
            .field("sqlite_code", &self.cleanup.sqlite_error_code())
            .finish()
    }
}
impl std::fmt::Display for SqlCombined {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("projection-sql-cleanup")
    }
}
impl Error for SqlCombined {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.primary.as_ref())
    }
}

/// Seed declared historical data only in a marked fixture, using the real
/// supported task start and actual FK/identity projection. No execution or
/// acceptance is fabricated. Every write/rollback/close failure is checked.
fn seed_history(root: &Path, id: &str) -> Result<(), Box<dyn Error>> {
    let mut store = swarm_engine::RootStore::open(root, true)?;
    let result = (|| -> Result<swarm_engine::Work, Box<dyn Error>> {
        let work = store.read_work(id)?;
        let tasks = work.tasks.as_deref().unwrap_or_default();
        if tasks.len() != 1 {
            return Err("one fixture task required".into());
        }
        if tasks[0].attempts.as_ref().is_some_and(|v| !v.is_empty()) {
            return Ok(work);
        }
        let request = serde_json::to_vec(
            &serde_json::json!({"schema_version":1,"event_id":swarm_engine::generate_identifier("seed-start-")?,"expected_revision":work.revision,"id":tasks[0].id,"status":"running"}),
        )?;
        Ok(store
            .execute_external(
                id,
                "task.update",
                &request,
                &mut swarm_engine::SystemEnvironment,
            )?
            .result
            .work)
    })();
    let mut work = match (result, store.close()) {
        (Ok(w), Ok(())) => w,
        (Err(e), Ok(())) => return Err(e),
        (Ok(_), Err(e)) => return Err(Box::new(e)),
        (Err(primary), Err(cleanup)) => return Err(Box::new(Combined { primary, cleanup })),
    };
    let attempt = work
        .tasks
        .as_ref()
        .and_then(|v| v.first())
        .and_then(|t| t.attempts.as_ref())
        .and_then(|v| v.last())
        .ok_or("actual attempt required")?
        .id
        .clone();
    let db = rusqlite::Connection::open(root.join(".swarm/state.db"))?;
    // Owned, serialized fixture: expose unexpected contention without an implicit library wait.
    db.busy_timeout(std::time::Duration::ZERO)?;
    let result = (|| -> Result<(), Box<dyn Error>> {
        db.execute_batch("PRAGMA foreign_keys=ON")?;
        swarm_engine::register_identity_plan_projection(&db)?;
        db.execute_batch("BEGIN IMMEDIATE")?;
        let writes = (|| -> Result<(), Box<dyn Error>> {
            for n in 0..6 {
                work.revision = work
                    .revision
                    .checked_add(1)
                    .ok_or("fixture revision overflow")?;
                let event = swarm_engine::generate_identifier("declared-history-")?;
                let raw = serde_json::to_vec(
                    &serde_json::json!({"attempt_id":attempt,"receipt":event,"controls":[{"id":format!("fixture-{n}"),"wall_duration_ms":n,"executed":true,"passed":true}]}),
                )?;
                db.execute(
                    "UPDATE works SET revision=?1,body=?2 WHERE id=?3",
                    rusqlite::params![
                        work.revision,
                        String::from_utf8(swarm_engine::encode_model(&work)?)?,
                        work.id
                    ],
                )?;
                db.execute(
                    "INSERT INTO events VALUES(?1,?2,?3,'task.auto-validation','date',?4,?4)",
                    rusqlite::params![event, work.id, work.revision, raw],
                )?;
            }
            db.execute_batch("COMMIT")?;
            Ok(())
        })();
        if let Err(primary) = writes {
            return match db.execute_batch("ROLLBACK") {
                Ok(()) => Err(primary),
                Err(cleanup) => Err(Box::new(SqlCombined { primary, cleanup })),
            };
        }
        Ok(())
    })();
    match (result, db.close()) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(e), Ok(())) => Err(e),
        (Ok(()), Err((_, e))) => Err(Box::new(e)),
        (Err(primary), Err((_, cleanup))) => Err(Box::new(SqlCombined { primary, cleanup })),
    }
}

/// Populate only an already initialized, marked owned fixture using real schema
/// triggers/UDFs. Supplied data is not an authorized mutation or execution proof.
/// This single-statement replacement preserves the old row on SQL failure; all
/// primary/close causes are retained. Never exposed by the production CLI/API.
fn seed_work(root: &Path, raw: &[u8]) -> Result<(), Box<dyn Error>> {
    let work = swarm_engine::decode_work(raw)?;
    if work.schema_version != 1 || work.id.is_empty() || work.revision < 1 {
        return Err("valid fixture Work envelope required".into());
    }
    let path = root.join(".swarm/state.db");
    if !std::fs::symlink_metadata(&path)?.file_type().is_file() {
        return Err("initialized fixture database required".into());
    }
    let db = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    // Owned, serialized fixture: expose unexpected contention without an implicit library wait.
    db.busy_timeout(std::time::Duration::ZERO)?;
    let result = (|| -> Result<(), Box<dyn Error>> {
        db.execute_batch("PRAGMA foreign_keys=ON")?;
        swarm_engine::register_identity_plan_projection(&db)?;
        let changed = db.execute("INSERT INTO works(id,revision,body) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,body=excluded.body",rusqlite::params![work.id,work.revision,String::from_utf8(swarm_engine::encode_model(&work)?)?])?;
        if changed != 1 {
            return Err("fixture row was not replaced".into());
        }
        Ok(())
    })();
    match (result, db.close()) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(e), Ok(())) => Err(e),
        (Ok(()), Err((_, e))) => Err(Box::new(e)),
        (Err(primary), Err((_, cleanup))) => Err(Box::new(SqlCombined { primary, cleanup })),
    }
}
