DROP TRIGGER agent_identity_plan_insert;
DROP TRIGGER agent_identity_plan_update_guard;
DROP TRIGGER agent_identity_plan_update;
DROP TABLE agent_identity_plan_heads;
ALTER TABLE agent_identity_plan_heads_v2 RENAME TO agent_identity_plan_heads;
CREATE TRIGGER agent_identity_plan_insert AFTER INSERT ON works BEGIN
 INSERT INTO agent_identity_plan_heads(work_id,generation,digest,projection_version)
 VALUES(NEW.id,1,swarm_identity_plan_digest_v2(NEW.body),2);
END;
CREATE TRIGGER agent_identity_plan_update_guard BEFORE UPDATE OF body,id ON works BEGIN
 SELECT CASE WHEN NEW.id!=OLD.id OR NOT EXISTS(
  SELECT 1 FROM agent_identity_plan_heads WHERE work_id=OLD.id AND projection_version=2
  AND digest=swarm_identity_plan_digest_v2(OLD.body))
 THEN RAISE(ABORT,'identity plan stamp changed or missing') END;
END;
CREATE TRIGGER agent_identity_plan_update AFTER UPDATE OF body ON works BEGIN
 UPDATE agent_identity_plan_heads SET
  generation=generation+CASE WHEN digest!=swarm_identity_plan_digest_v2(NEW.body) THEN 1 ELSE 0 END,
  digest=swarm_identity_plan_digest_v2(NEW.body) WHERE work_id=NEW.id;
END;
