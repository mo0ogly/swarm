//go:build !linux

package main

import "database/sql"

const durableCoordinatorMigration = `BEGIN IMMEDIATE;
CREATE TABLE IF NOT EXISTS mission_coordination_events(
 event_key TEXT PRIMARY KEY,
 work_id TEXT NOT NULL REFERENCES works(id),
 agent_id TEXT NOT NULL,
 conductor_id TEXT NOT NULL,
 kind TEXT NOT NULL,
 at TEXT NOT NULL,
 detail TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS mission_coordination_work
 ON mission_coordination_events(work_id,at);
PRAGMA user_version=14;
COMMIT;`

func migrateDurableCoordinator(db *sql.DB) error {
	_, err := db.Exec(durableCoordinatorMigration)
	return err
}

func (s *Store) recordCoordinationEvent(key, work, agent, conductor, kind, detail string) error {
	_, err := s.db.Exec(`INSERT OR IGNORE INTO mission_coordination_events
		(event_key,work_id,agent_id,conductor_id,kind,at,detail) VALUES(?,?,?,?,?,?,?)`,
		key, work, agent, conductor, kind, now(), detail)
	return err
}

func (s *Store) signalResourceReleased(agent Agent) {
	_ = s.recordCoordinationEvent("resource-released:"+agent.ID, agent.WorkID, agent.ID, conductorAuthor,
		"resource-released", "Réservation libérée")
}

func (s *Store) reconcileMissionAttempts(work, conductor string, launch func(Agent) error) error {
	return nil
}
