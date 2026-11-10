-- Fresh v37 schema derived from the actual Go initializer; no row authority copied.
CREATE TABLE agent_dialogue_turns(id TEXT PRIMARY KEY, agent_id TEXT NOT NULL REFERENCES agents(id), question TEXT NOT NULL, status TEXT NOT NULL, session_id TEXT NOT NULL, checkpoint BLOB NOT NULL DEFAULT '');
CREATE TABLE agent_exchanges(
 id TEXT PRIMARY KEY,
 request_hash TEXT NOT NULL,
 work_id TEXT NOT NULL REFERENCES works(id),
 kind TEXT NOT NULL,
 source_agent_id TEXT NOT NULL REFERENCES agents(id),
 source_task_id TEXT NOT NULL,
 source_attempt_id TEXT NOT NULL,
 recipient_task_id TEXT NOT NULL,
 recipient_role TEXT NOT NULL,
 recipient_attempt_id TEXT NOT NULL DEFAULT '',
 reply_to TEXT NOT NULL DEFAULT '',
 state TEXT NOT NULL,
 result_state TEXT NOT NULL DEFAULT '',
 need TEXT NOT NULL DEFAULT '',
 artifacts BLOB NOT NULL,
 created_at TEXT NOT NULL,
 deadline_at TEXT NOT NULL DEFAULT '',
 acknowledged_at TEXT NOT NULL DEFAULT '',
 acknowledged_by TEXT NOT NULL DEFAULT '',
 acknowledgement TEXT NOT NULL DEFAULT ''
);
CREATE TABLE agent_identity_plan_heads(
 work_id TEXT PRIMARY KEY REFERENCES works(id) ON DELETE CASCADE,
 generation INTEGER NOT NULL CHECK(typeof(generation)='integer' AND generation>0),
 digest TEXT NOT NULL, projection_version INTEGER NOT NULL CHECK(projection_version=1));
CREATE TABLE agent_identity_publications(event_id TEXT PRIMARY KEY,work_id TEXT NOT NULL REFERENCES works(id),head_digest TEXT NOT NULL UNIQUE,body BLOB NOT NULL);
CREATE TABLE agent_identity_snapshots(work_id TEXT NOT NULL REFERENCES works(id),revision INTEGER NOT NULL CHECK(revision>0),digest TEXT NOT NULL,body BLOB NOT NULL,PRIMARY KEY(work_id,revision));
CREATE TABLE agent_logs(seq INTEGER PRIMARY KEY AUTOINCREMENT, agent_id TEXT NOT NULL REFERENCES agents(id), at TEXT NOT NULL, kind TEXT NOT NULL, message TEXT NOT NULL);
CREATE TABLE agents(id TEXT PRIMARY KEY, work_id TEXT NOT NULL REFERENCES works(id), task_id TEXT NOT NULL,
 cwd TEXT NOT NULL, status TEXT NOT NULL, desired TEXT NOT NULL DEFAULT '', body BLOB NOT NULL, request BLOB NOT NULL);
CREATE TABLE assist_previews(id TEXT PRIMARY KEY, work_id TEXT NOT NULL REFERENCES works(id), created_at TEXT NOT NULL, body BLOB NOT NULL);
CREATE TABLE assist_reservations(turn_id TEXT PRIMARY KEY REFERENCES assist_turns(id), work_id TEXT NOT NULL REFERENCES works(id), amount REAL NOT NULL, state TEXT NOT NULL);
CREATE TABLE assist_turns(id TEXT PRIMARY KEY, work_id TEXT NOT NULL REFERENCES works(id), created_at TEXT NOT NULL, status TEXT NOT NULL, body BLOB NOT NULL);
CREATE TABLE automation_causal_recoveries(
 recovery_id TEXT PRIMARY KEY,
 request_id TEXT NOT NULL REFERENCES automation_requests(request_id),
 previous_reason TEXT NOT NULL,
 evidence_digest TEXT NOT NULL,
 evidence_kind TEXT NOT NULL,
 observed_at TEXT NOT NULL,
 UNIQUE(request_id,evidence_digest)
);
CREATE TABLE automation_effects(
 effect_id TEXT PRIMARY KEY,
 request_id TEXT NOT NULL UNIQUE REFERENCES automation_requests(request_id),
 target_work_id TEXT NOT NULL REFERENCES works(id),
 action TEXT NOT NULL CHECK(action='request_resume'),
 created_at TEXT NOT NULL
);
CREATE TABLE automation_external_audit(
 audit_id TEXT PRIMARY KEY,
 event_fingerprint TEXT NOT NULL,
 key_fingerprint TEXT NOT NULL,
 request_id TEXT NOT NULL,
 received_at TEXT NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('admitted','accepted','rejected')),
 reason TEXT NOT NULL
);
CREATE TABLE automation_external_events(
 event_id TEXT PRIMARY KEY,
 body_digest TEXT NOT NULL,
 request_id TEXT NOT NULL,
 key_fingerprint TEXT NOT NULL,
 received_at TEXT NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('received','accepted','rejected')),
 reason TEXT NOT NULL
);
CREATE TABLE automation_occurrences(
 occurrence_id TEXT PRIMARY KEY,
 request_id TEXT NOT NULL UNIQUE REFERENCES automation_requests(request_id),
 state TEXT NOT NULL CHECK(state IN ('received','waiting','claimed','executed','rejected','uncertain_effect')),
 revision INTEGER NOT NULL,
 claimed_by TEXT NOT NULL,
 lease_until TEXT NOT NULL,
 effect_id TEXT NOT NULL,
 reason TEXT NOT NULL,
 actor TEXT NOT NULL,
 next_action TEXT NOT NULL,
 created_at TEXT NOT NULL,
 updated_at TEXT NOT NULL
);
CREATE TABLE automation_request_origins(
 request_id TEXT NOT NULL REFERENCES automation_requests(request_id),
 schedule_id TEXT NOT NULL REFERENCES automation_schedules(schedule_id),
 intended_at TEXT NOT NULL,
 source TEXT NOT NULL,
 reason TEXT NOT NULL,
 PRIMARY KEY(request_id,schedule_id,intended_at)
);
CREATE TABLE automation_requests(
 request_id TEXT PRIMARY KEY,
 idempotency_key TEXT NOT NULL UNIQUE,
 content_digest TEXT NOT NULL,
 target_work_id TEXT NOT NULL REFERENCES works(id),
 source TEXT NOT NULL,
 action TEXT NOT NULL CHECK(action='request_resume'),
 occurrence_id TEXT NOT NULL UNIQUE,
 state TEXT NOT NULL CHECK(state IN ('received','waiting','claimed','executed','rejected','uncertain_effect')),
 revision INTEGER NOT NULL,
 reason TEXT NOT NULL,
 actor TEXT NOT NULL,
 next_action TEXT NOT NULL,
 created_at TEXT NOT NULL,
 updated_at TEXT NOT NULL
);
CREATE TABLE automation_schedule_journal(
 journal_id TEXT PRIMARY KEY,
 schedule_id TEXT NOT NULL REFERENCES automation_schedules(schedule_id),
 intended_at TEXT NOT NULL,
 observed_at TEXT NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('skipped','submitted','coalesced','held')),
 request_id TEXT NOT NULL,
 source TEXT NOT NULL,
 reason TEXT NOT NULL,
 UNIQUE(schedule_id,intended_at,state)
);
CREATE TABLE automation_schedules(
 schedule_id TEXT PRIMARY KEY,
 event_id TEXT NOT NULL UNIQUE,
 name TEXT NOT NULL,
 target_work_id TEXT NOT NULL REFERENCES works(id),
 action TEXT NOT NULL CHECK(action='request_resume'),
 kind TEXT NOT NULL CHECK(kind IN ('once','daily','weekly')),
 timezone TEXT NOT NULL,
 start_local TEXT NOT NULL,
 until_local TEXT NOT NULL,
 max_occurrences INTEGER NOT NULL,
 missed_policy TEXT NOT NULL CHECK(missed_policy='skip'),
 concurrency_policy TEXT NOT NULL CHECK(concurrency_policy='coalesce'),
 state TEXT NOT NULL CHECK(state IN ('disabled','enabled','paused','archived')),
 revision INTEGER NOT NULL,
 next_at TEXT NOT NULL,
 emitted_count INTEGER NOT NULL,
 last_observed_at TEXT NOT NULL,
 created_at TEXT NOT NULL,
 updated_at TEXT NOT NULL
);
CREATE TABLE budgets(work_id TEXT PRIMARY KEY REFERENCES works(id), body BLOB NOT NULL);
CREATE TABLE cockpit_controls(work_id TEXT PRIMARY KEY REFERENCES works(id), paused INTEGER NOT NULL DEFAULT 0, autonomy TEXT NOT NULL DEFAULT 'autonome', slots INTEGER NOT NULL DEFAULT 2);
CREATE TABLE cockpit_events(seq INTEGER PRIMARY KEY AUTOINCREMENT, work_id TEXT NOT NULL REFERENCES works(id), at TEXT NOT NULL, kind TEXT NOT NULL, message TEXT NOT NULL);
CREATE TABLE cockpit_tasks(work_id TEXT NOT NULL REFERENCES works(id), task_id TEXT NOT NULL, priority INTEGER NOT NULL DEFAULT 0, PRIMARY KEY(work_id,task_id));
CREATE TABLE decisions(id TEXT PRIMARY KEY, work_id TEXT NOT NULL REFERENCES works(id), body BLOB NOT NULL);
CREATE TABLE events(id TEXT PRIMARY KEY, work_id TEXT NOT NULL REFERENCES works(id), revision INTEGER NOT NULL, kind TEXT NOT NULL, at TEXT NOT NULL, payload BLOB NOT NULL, request BLOB NOT NULL, UNIQUE(work_id,revision));
CREATE TABLE exchange_acknowledgements(
 id TEXT PRIMARY KEY, work_id TEXT NOT NULL REFERENCES works(id),
 exchange_id TEXT NOT NULL REFERENCES agent_exchanges(id),
 request_hash TEXT NOT NULL, created_at TEXT NOT NULL);
CREATE TABLE graph_draft_authorizations(
 work_id TEXT NOT NULL REFERENCES works(id), actor TEXT NOT NULL, revision INTEGER NOT NULL,
 prepare INTEGER NOT NULL, apply INTEGER NOT NULL, scopes BLOB NOT NULL, updated TEXT NOT NULL,
 PRIMARY KEY(work_id, actor));
CREATE TABLE graph_drafts(
 id TEXT PRIMARY KEY, work_id TEXT NOT NULL REFERENCES works(id), base_revision INTEGER NOT NULL,
 revision INTEGER NOT NULL, status TEXT NOT NULL, actor TEXT NOT NULL, operations BLOB NOT NULL,
 content_digest TEXT NOT NULL, preview_token TEXT NOT NULL DEFAULT '', created TEXT NOT NULL, updated TEXT NOT NULL);
CREATE TABLE lifecycle_receipts(
 event_id TEXT PRIMARY KEY, work_id TEXT NOT NULL, action TEXT NOT NULL,
 request_digest TEXT NOT NULL, created_at TEXT NOT NULL, response BLOB NOT NULL
);
CREATE TABLE managed_attempts(agent_id TEXT PRIMARY KEY,work_id TEXT NOT NULL REFERENCES works(id) ON DELETE CASCADE,task_id TEXT NOT NULL,base_commit TEXT NOT NULL,path TEXT NOT NULL,state TEXT NOT NULL,result_commit TEXT NOT NULL DEFAULT '',detail TEXT NOT NULL DEFAULT '');
CREATE TABLE mission_coordination_events(
 event_key TEXT PRIMARY KEY,
 work_id TEXT NOT NULL REFERENCES works(id),
 agent_id TEXT NOT NULL,
 conductor_id TEXT NOT NULL,
 kind TEXT NOT NULL,
 at TEXT NOT NULL,
 detail TEXT NOT NULL
);
CREATE TABLE mission_lifecycle(
 work_id TEXT PRIMARY KEY REFERENCES works(id), archived_at TEXT NOT NULL,
 archive_path TEXT NOT NULL, archive_digest TEXT NOT NULL
);
CREATE TABLE mission_supervision(
 work_id TEXT PRIMARY KEY REFERENCES works(id),
 conductor_id TEXT NOT NULL,
 source TEXT NOT NULL,
 started_at TEXT NOT NULL,
 heartbeat_at TEXT NOT NULL,
 reconciled_at TEXT NOT NULL,
 last_check_at TEXT NOT NULL,
 last_error TEXT NOT NULL,
 next_check_at TEXT NOT NULL,
 stopped_at TEXT NOT NULL
);
CREATE TABLE mission_trash(
 work_id TEXT PRIMARY KEY, title TEXT NOT NULL, revision INTEGER NOT NULL,
 deleted_at TEXT NOT NULL, snapshot BLOB NOT NULL
);
CREATE TABLE planning_calls(id TEXT PRIMARY KEY,work_id TEXT NOT NULL REFERENCES works(id) ON DELETE CASCADE,scope_id TEXT NOT NULL,estimate REAL NOT NULL,usage BLOB,state TEXT NOT NULL,created_at TEXT NOT NULL);
CREATE TABLE preparation_commands(preparation_id TEXT NOT NULL REFERENCES preparations(id), event_id TEXT NOT NULL, request BLOB NOT NULL, response BLOB NOT NULL, PRIMARY KEY(preparation_id,event_id));
CREATE TABLE preparation_documents(preparation_id TEXT NOT NULL REFERENCES preparations(id), kind TEXT NOT NULL, revision INTEGER NOT NULL, body BLOB NOT NULL, PRIMARY KEY(preparation_id,kind,revision));
CREATE TABLE preparation_launch_locks(work_id TEXT NOT NULL REFERENCES works(id), task_id TEXT NOT NULL, preparation_id TEXT NOT NULL REFERENCES preparations(id), released INTEGER NOT NULL DEFAULT 0 CHECK(released IN (0,1)), PRIMARY KEY(work_id,task_id));
CREATE TABLE preparation_reservations(turn_id TEXT PRIMARY KEY REFERENCES preparation_turns(id), preparation_id TEXT NOT NULL REFERENCES preparations(id), work_id TEXT, amount REAL NOT NULL, work_amount REAL NOT NULL, state TEXT NOT NULL CHECK(state IN ('reserved','estimated','released')));
CREATE TABLE preparation_turns(id TEXT PRIMARY KEY, preparation_id TEXT NOT NULL REFERENCES preparations(id), status TEXT NOT NULL, body BLOB NOT NULL);
CREATE TABLE preparations(id TEXT PRIMARY KEY, work_id TEXT REFERENCES works(id), revision INTEGER NOT NULL, body BLOB NOT NULL);
CREATE TABLE reservations(agent_id TEXT PRIMARY KEY REFERENCES agents(id), work_id TEXT NOT NULL REFERENCES works(id), amount REAL NOT NULL, state TEXT NOT NULL);
CREATE TABLE run_limits_config(scope TEXT NOT NULL, mission_id TEXT NOT NULL DEFAULT '', scope_key TEXT NOT NULL DEFAULT '',
 revision INTEGER NOT NULL, body BLOB NOT NULL, actor TEXT NOT NULL, reason TEXT NOT NULL DEFAULT '', updated TEXT NOT NULL,
 PRIMARY KEY(scope,mission_id,scope_key));
CREATE TABLE run_limits_history(seq INTEGER PRIMARY KEY AUTOINCREMENT, scope TEXT NOT NULL, mission_id TEXT NOT NULL DEFAULT '',
 scope_key TEXT NOT NULL DEFAULT '', revision INTEGER NOT NULL, body BLOB NOT NULL, request BLOB NOT NULL, actor TEXT NOT NULL, reason TEXT NOT NULL DEFAULT '',
 at TEXT NOT NULL, event_id TEXT NOT NULL, rollback_of INTEGER, UNIQUE(scope,mission_id,scope_key,event_id));
CREATE TABLE session_visits(work_id TEXT NOT NULL REFERENCES works(id), operator TEXT NOT NULL, revision INTEGER NOT NULL, at TEXT NOT NULL, PRIMARY KEY(work_id,operator));
CREATE TABLE terminal_events(seq INTEGER PRIMARY KEY AUTOINCREMENT, agent_id TEXT NOT NULL REFERENCES agents(id), data BLOB NOT NULL, cols INTEGER NOT NULL DEFAULT 0, rows INTEGER NOT NULL DEFAULT 0);
CREATE TABLE works(id TEXT PRIMARY KEY, revision INTEGER NOT NULL, body BLOB NOT NULL);
CREATE TABLE workspace_integrations(
 id TEXT PRIMARY KEY,
 request_hash TEXT NOT NULL,
 work_id TEXT NOT NULL REFERENCES works(id),
 exchange_id TEXT NOT NULL REFERENCES agent_exchanges(id),
 canonical_workspace TEXT NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('running','integrated','conflict','failed')),
 requested_at TEXT NOT NULL,
 finished_at TEXT NOT NULL DEFAULT '',
 detail BLOB NOT NULL
);
CREATE TABLE workspace_turns(
 seq INTEGER PRIMARY KEY AUTOINCREMENT,
 work_id TEXT NOT NULL REFERENCES works(id),
 task_id TEXT NOT NULL,
 workspace TEXT NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('waiting','acquired','released')),
 agent_id TEXT NOT NULL DEFAULT '',
 requested_at TEXT NOT NULL,
 acquired_at TEXT NOT NULL DEFAULT '',
 released_at TEXT NOT NULL DEFAULT '',
 UNIQUE(work_id,task_id)
);
CREATE UNIQUE INDEX agent_cwd_active ON agents(cwd) WHERE status IN ('queued','starting','running','stopping');
CREATE INDEX agent_exchange_deadline ON agent_exchanges(work_id,state,deadline_at);
CREATE INDEX agent_exchange_work ON agent_exchanges(work_id,created_at);
CREATE INDEX agent_log_cursor ON agent_logs(agent_id,seq);
CREATE UNIQUE INDEX agent_task_active ON agents(work_id,task_id) WHERE status IN ('queued','starting','running','stopping');
CREATE UNIQUE INDEX assist_one_active ON assist_turns(work_id) WHERE status IN ('pending','running');
CREATE INDEX assist_turns_work ON assist_turns(work_id, created_at);
CREATE INDEX automation_external_audit_rate ON automation_external_audit(key_fingerprint,received_at);
CREATE INDEX automation_occurrences_claimable ON automation_occurrences(state,lease_until);
CREATE INDEX automation_schedule_journal_schedule ON automation_schedule_journal(schedule_id,observed_at);
CREATE INDEX automation_schedules_due ON automation_schedules(state,next_at);
CREATE INDEX decisions_work ON decisions(work_id);
CREATE UNIQUE INDEX dialogue_one_running ON agent_dialogue_turns(agent_id) WHERE status='running';
CREATE INDEX graph_drafts_work ON graph_drafts(work_id, updated);
CREATE INDEX mission_coordination_work
 ON mission_coordination_events(work_id,at);
CREATE INDEX preparation_budget_prep ON preparation_reservations(preparation_id,state);
CREATE INDEX preparation_budget_work ON preparation_reservations(work_id,state);
CREATE UNIQUE INDEX preparation_one_active ON preparation_turns(preparation_id) WHERE status IN ('pending','running','stopping');
CREATE INDEX run_limits_history_scope ON run_limits_history(scope,mission_id,scope_key,revision);
CREATE INDEX terminal_cursor ON terminal_events(agent_id,seq);
CREATE INDEX workspace_integration_active ON workspace_integrations(state,canonical_workspace);
CREATE INDEX workspace_turn_waiting ON workspace_turns(state,seq);
CREATE TRIGGER agent_identity_plan_insert AFTER INSERT ON works BEGIN
 INSERT INTO agent_identity_plan_heads(work_id,generation,digest,projection_version)
 VALUES(NEW.id,1,swarm_identity_plan_digest_v1(NEW.body),1);
END;
CREATE TRIGGER agent_identity_plan_update AFTER UPDATE OF body ON works BEGIN
 UPDATE agent_identity_plan_heads SET
  generation=generation+CASE WHEN digest!=swarm_identity_plan_digest_v1(NEW.body) THEN 1 ELSE 0 END,
  digest=swarm_identity_plan_digest_v1(NEW.body) WHERE work_id=NEW.id;
END;
CREATE TRIGGER agent_identity_plan_update_guard BEFORE UPDATE OF body,id ON works BEGIN
 SELECT CASE WHEN NEW.id!=OLD.id OR NOT EXISTS(
  SELECT 1 FROM agent_identity_plan_heads WHERE work_id=OLD.id AND projection_version=1
  AND digest=swarm_identity_plan_digest_v1(OLD.body))
 THEN RAISE(ABORT,'identity plan stamp changed or missing') END;
END;
CREATE TRIGGER preparation_launch_insert BEFORE INSERT ON agents WHEN EXISTS(SELECT 1 FROM preparation_launch_locks WHERE work_id=NEW.work_id AND task_id=NEW.task_id AND released=0) BEGIN SELECT RAISE(ABORT,'Plan verrouille : autoriser le demarrage depuis la preparation'); END;
CREATE TRIGGER preparation_launch_update BEFORE UPDATE OF status ON agents WHEN NEW.status IN ('queued','starting','running') AND EXISTS(SELECT 1 FROM preparation_launch_locks WHERE work_id=NEW.work_id AND task_id=NEW.task_id AND released=0) BEGIN SELECT RAISE(ABORT,'Plan verrouille : autoriser le demarrage depuis la preparation'); END;
PRAGMA user_version=37;
