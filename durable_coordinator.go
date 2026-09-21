//go:build linux

package main

import (
	"database/sql"
	"fmt"
	"time"
)

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

// recordCoordinationEvent is an idempotent receipt. The event key identifies
// the durable fact, not one polling pass, so a restart cannot duplicate it.
func (s *Store) recordCoordinationEvent(key, work, agent, conductor, kind, detail string) error {
	_, err := s.db.Exec(`INSERT OR IGNORE INTO mission_coordination_events
		(event_key,work_id,agent_id,conductor_id,kind,at,detail) VALUES(?,?,?,?,?,?,?)`,
		key, work, agent, conductor, kind, now(), terminalText(detail))
	return err
}

// reconcileMissionAttempts repairs only states whose identity is already
// durable. A queued intent is started with the same agent/attempt ID. A known
// terminal result is settled idempotently. An active row whose processes have
// disappeared is classified by reconcile; it is never promoted to success.
func (s *Store) reconcileMissionAttempts(work, conductor string, launch func(Agent) error) error {
	// registered in missionLoop is deliberately not trusted: fence the pass
	// against the persisted lease before any recovery side effect.
	if err := s.renewMissionSupervision(work, conductor, time.Now()); err != nil {
		return err
	}
	agents, err := s.agents(work)
	if err != nil {
		return err
	}
	w, err := s.get(work)
	if err != nil {
		return err
	}
	for _, agent := range agents {
		switch {
		case !activeAgent(agent):
			if w.Planning != nil && w.Planning.Repository != nil {
				// Managed settlement can run controls and an independent provider
				// review. Keep that bounded operation off the conductor loop so
				// its lease and other missions continue to be checked. The
				// existing cross-process managed lock and review claim deduplicate
				// polling/restarts; no new production attempt is created here.
				go func(a Agent) {
					if e := s.reconcileKnownMissionResult(a, conductor); e != nil {
						_ = s.recordCoordinationEvent("settle-error:"+a.ID, a.WorkID, a.ID, conductor, "settle-error", e.Error())
					}
				}(agent)
				continue
			}
			if err = s.reconcileKnownMissionResult(agent, conductor); err != nil {
				return err
			}
		case agent.Status == "queued" && agent.Supervisor == 0 && agent.Child == 0 && agent.Heartbeat == "":
			if err = s.recordCoordinationEvent("resume-intent:"+agent.ID, work, agent.ID, conductor,
				"resume-intent", "Intention persistée reprise avec la même identité de tentative"); err != nil {
				return err
			}
			if err = launch(agent); err != nil {
				return fmt.Errorf("reprendre l’intention %s : %w", agent.ID, err)
			}
		default:
			observed := observedAgent(agent)
			if observed != "unknown/supervisor-lost" && observed != "unknown/no-heartbeat" {
				continue
			}
			if err = s.reconcile(agent.ID); err != nil {
				// An unknown starting window or a still-live child is retained. This
				// is a persisted distinction from a terminal result, not a success.
				if eventErr := s.recordCoordinationEvent("process-retained:"+agent.ID, work, agent.ID, conductor,
					"process-retained", err.Error()); eventErr != nil {
					return eventErr
				}
				continue
			}
			if err = s.recordCoordinationEvent("process-gone:"+agent.ID, work, agent.ID, conductor,
				"process-gone", "Processus disparu confirmé ; résultat classé interrompu, jamais réussi"); err != nil {
				return err
			}
		}
	}
	return nil
}

func (s *Store) reconcileKnownMissionResult(agent Agent, conductor string) error {
	// Explicitly authorized external repairs keep their original stopped status.
	// A retry-review queues integration without changing that process history.
	if w, err := s.get(agent.WorkID); err == nil && w.Planning != nil && w.Planning.Repository != nil {
		if task, err := w.task(agent.TaskID); err == nil {
			if item, err := s.managedAttempt(agent.ID); err == nil && item.State == "integrating" && recoveredResultMatches(task, agent, item) {
				return s.integrateManagedAttempt(agent)
			}
		}
	}

	if err := s.settleAgentTask(agent); err != nil {
		return err
	}
	return s.recordCoordinationEvent("known-result:"+agent.ID, agent.WorkID, agent.ID, conductor,
		"known-result", "Résultat terminal connu et réconcilié sans nouvelle tentative")
}

func (s *Store) signalResourceReleased(agent Agent) {
	_ = s.recordCoordinationEvent("resource-released:"+agent.ID, agent.WorkID, agent.ID, conductorAuthor,
		"resource-released", "Réservation libérée ; les conducteurs autorisés réévaluent leur file")
}
