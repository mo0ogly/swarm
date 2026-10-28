//! Console store (`.swarm/telemetry.db`) opened at every older schema the
//! console has shipped (v2, v4, v5, v6): a read-only open never migrates and
//! reads the newer tables as empty; a write open migrates to the current
//! schema in one step, after which annotations, evaluations and scenarios
//! work. Older stores are built from a fresh one by removing what each later
//! version added (fixture only, private temporary Root).
use rusqlite::Connection;
use std::fs;
use swarm_engine::{TelemetryStore, TELEMETRY_DB, TELEMETRY_SCHEMA_VERSION};

/// What each version added, newest first (v3 added `alerts.resolution`).
const ADDED: [(i64, &str); 5] = [
    (7, "DROP TABLE scenarios;"),
    (6, "DROP TABLE evaluations;"),
    (5, "DROP INDEX annotations_work; DROP TABLE annotations;"),
    (4, "DROP TABLE purges; DROP INDEX records_received;"),
    (3, "ALTER TABLE alerts DROP COLUMN resolution;"),
];

fn version(path: &std::path::Path) -> i64 {
    Connection::open(path).unwrap().query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap()
}

#[test]
fn every_older_store_reads_as_is_and_migrates_on_write() {
    assert_eq!(TELEMETRY_SCHEMA_VERSION, 7, "extend ADDED when the schema grows");
    for old in [2, 4, 5, 6] {
        let root = std::env::temp_dir().join(swarm_engine::generate_identifier("telemetry-migrate-").unwrap());
        fs::create_dir_all(root.join(".swarm")).unwrap();
        TelemetryStore::open(&root, true).unwrap().close().unwrap();
        let db = root.join(".swarm").join(TELEMETRY_DB);
        let conn = Connection::open(&db).unwrap();
        for (added, sql) in ADDED.iter().filter(|(v, _)| *v > old) {
            conn.execute_batch(sql).unwrap_or_else(|e| panic!("v{old}: undo v{added}: {e}"));
        }
        conn.execute_batch(&format!("PRAGMA user_version = {old};")).unwrap();
        drop(conn);
        // Read-only open: no migration, newer tables read as empty.
        let read = TelemetryStore::open(&root, false).unwrap();
        assert!(read.read_annotations("w").unwrap().is_empty(), "v{old}");
        assert!(read.evaluated_agents().unwrap().is_empty(), "v{old}");
        assert!(read.read_scenarios("w").unwrap().is_empty(), "v{old}");
        read.change_marker().unwrap();
        read.close().unwrap();
        assert_eq!(version(&db), old, "a read never migrates (v{old})");
        // Write open: migrated to the current schema, new tables usable.
        let mut store = TelemetryStore::open(&root, true).unwrap();
        assert_eq!(version(&db), TELEMETRY_SCHEMA_VERSION, "v{old} migrated");
        store.annotate("a", "w", "good", "", 0, 1).unwrap();
        let eval = swarm_engine::AttemptEvaluation { evaluator: "outcome", score: None, passed: Some(true), detail: String::new() };
        assert_eq!(store.record_evaluations(&[("a".into(), "w".into(), vec![eval])], 1).unwrap(), 1, "v{old}");
        let spec: swarm_engine::ScenarioSpec = serde_json::from_str(r#"{"schema_version":1,"expectations":[{"kind":"usage-observed"}]}"#).unwrap();
        assert_eq!(store.save_scenario("w", "t", &spec, 0, 1).unwrap(), 1, "v{old}");
        store.close().unwrap();
        fs::remove_dir_all(&root).unwrap();
    }
}
