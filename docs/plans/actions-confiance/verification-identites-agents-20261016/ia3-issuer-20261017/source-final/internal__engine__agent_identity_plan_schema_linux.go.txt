//go:build linux

package engine

import (
	"database/sql"
	"database/sql/driver"
	"fmt"

	"modernc.org/sqlite"
)

func init() {
	// Registered once before any Store connection. This pure function uses
	// only its input; no database access, clock, identity or business decision.
	sqlite.MustRegisterDeterministicScalarFunction("swarm_identity_plan_digest_v1", 1, func(_ *sqlite.FunctionContext, args []driver.Value) (driver.Value, error) {
		var raw []byte
		switch v := args[0].(type) {
		case []byte:
			raw = v
		case string:
			raw = []byte(v)
		default:
			return nil, fmt.Errorf("plan projection requires JSON text")
		}
		return agentIdentityPlanProjectionDigest(raw)
	})
}

func migrateAgentIdentityPlanHeads(db *sql.DB) error {
	tx, err := db.Begin()
	if err != nil {
		return err
	}
	defer tx.Rollback()
	_, err = tx.Exec(`
CREATE TABLE IF NOT EXISTS agent_identity_plan_heads(
 work_id TEXT PRIMARY KEY REFERENCES works(id) ON DELETE CASCADE,
 generation INTEGER NOT NULL CHECK(typeof(generation)='integer' AND generation>0),
 digest TEXT NOT NULL, projection_version INTEGER NOT NULL CHECK(projection_version=1));
INSERT OR IGNORE INTO agent_identity_plan_heads(work_id,generation,digest,projection_version)
 SELECT id,1,swarm_identity_plan_digest_v1(body),1 FROM works;
CREATE TRIGGER IF NOT EXISTS agent_identity_plan_insert AFTER INSERT ON works BEGIN
 INSERT INTO agent_identity_plan_heads(work_id,generation,digest,projection_version)
 VALUES(NEW.id,1,swarm_identity_plan_digest_v1(NEW.body),1);
END;
CREATE TRIGGER IF NOT EXISTS agent_identity_plan_update_guard BEFORE UPDATE OF body,id ON works BEGIN
 SELECT CASE WHEN NEW.id!=OLD.id OR NOT EXISTS(
  SELECT 1 FROM agent_identity_plan_heads WHERE work_id=OLD.id AND projection_version=1
  AND digest=swarm_identity_plan_digest_v1(OLD.body))
 THEN RAISE(ABORT,'identity plan stamp changed or missing') END;
END;
CREATE TRIGGER IF NOT EXISTS agent_identity_plan_update AFTER UPDATE OF body ON works BEGIN
 UPDATE agent_identity_plan_heads SET
  generation=generation+CASE WHEN digest!=swarm_identity_plan_digest_v1(NEW.body) THEN 1 ELSE 0 END,
  digest=swarm_identity_plan_digest_v1(NEW.body) WHERE work_id=NEW.id;
END;
PRAGMA user_version=37;`)
	if err != nil {
		return err
	}
	return tx.Commit()
}
