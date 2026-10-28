package engine

import "database/sql"

func migrateAgentIdentityRegistry(db *sql.DB) error {
	tx, err := db.Begin()
	if err != nil {
		return err
	}
	defer tx.Rollback()
	for _, statement := range []string{
		`CREATE TABLE IF NOT EXISTS agent_identity_snapshots(work_id TEXT NOT NULL REFERENCES works(id),revision INTEGER NOT NULL CHECK(revision>0),digest TEXT NOT NULL,body BLOB NOT NULL,PRIMARY KEY(work_id,revision))`,
		`CREATE TABLE IF NOT EXISTS agent_identity_publications(event_id TEXT PRIMARY KEY,work_id TEXT NOT NULL REFERENCES works(id),head_digest TEXT NOT NULL UNIQUE,body BLOB NOT NULL)`,
		`PRAGMA user_version=36`,
	} {
		if _, err = tx.Exec(statement); err != nil {
			return err
		}
	}
	return tx.Commit()
}
