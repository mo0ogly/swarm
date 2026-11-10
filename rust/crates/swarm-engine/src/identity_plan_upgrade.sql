-- Component only: the root Store owns the global schema version and transaction.
CREATE TABLE agent_identity_plan_projection_history(
 work_id TEXT NOT NULL,
 from_generation INTEGER NOT NULL CHECK(typeof(from_generation)='integer' AND from_generation>0),
 from_digest TEXT NOT NULL, from_projection_version INTEGER NOT NULL CHECK(from_projection_version=1),
 to_generation INTEGER NOT NULL CHECK(typeof(to_generation)='integer' AND to_generation=from_generation+1),
 to_digest TEXT NOT NULL, to_projection_version INTEGER NOT NULL CHECK(to_projection_version=2),
 PRIMARY KEY(work_id,from_projection_version));
CREATE TRIGGER agent_identity_plan_history_update BEFORE UPDATE ON agent_identity_plan_projection_history BEGIN
 SELECT RAISE(ABORT,'identity plan projection history is immutable');
END;
CREATE TRIGGER agent_identity_plan_history_delete BEFORE DELETE ON agent_identity_plan_projection_history BEGIN
 SELECT RAISE(ABORT,'identity plan projection history is immutable');
END;
CREATE TABLE agent_identity_plan_heads_v2(
 work_id TEXT PRIMARY KEY REFERENCES works(id) ON DELETE CASCADE,
 generation INTEGER NOT NULL CHECK(typeof(generation)='integer' AND generation>0),
 digest TEXT NOT NULL, projection_version INTEGER NOT NULL CHECK(projection_version=2));
