package main

import (
	"context"
	"database/sql"
	"encoding/json"
	"fmt"
	"time"
)

// Consent is local cockpit metadata, never exported as authority with a Work.
// An absent or unreadable record is never inferred from autonomyDefault.
type MissionPolicy struct {
	Enabled bool   `json:"enabled"`
	Actor   string `json:"actor"`
	At      string `json:"at"`
}

func (s *Store) missionPolicy(work string) (MissionPolicy, error) {
	var p MissionPolicy
	var raw string
	e := s.db.QueryRow("SELECT message FROM cockpit_events WHERE work_id=? AND kind='mission-policy' ORDER BY rowid DESC LIMIT 1", work).Scan(&raw)
	if e == sql.ErrNoRows {
		return p, nil
	}
	if e != nil {
		return p, e
	}
	e = json.Unmarshal([]byte(raw), &p)
	return p, e
}
func (s *Store) setMission(work string, enabled bool) error {
	if _, e := s.get(work); e != nil {
		return e
	}
	raw, e := json.Marshal(MissionPolicy{enabled, operatorIdentity(), now()})
	if e != nil {
		return e
	}
	return s.controlEvent(work, "mission-policy", string(raw))
}
func (s *Store) configureMission(work string, p LaunchProfile, slots, revision int, events ...string) error {
	if archived, err := s.archived(work); err != nil {
		return err
	} else if archived {
		return &CommandError{Code: "mission_archived", Message: "mission archivée ; restaurer avant d’autoriser le conducteur"}
	}
	if slots < 1 || slots > slotsMax {
		return fmt.Errorf("Créneaux : 1 à %d", slotsMax)
	}
	ps, e := s.providers()
	if e != nil {
		return e
	}
	provider, known := ps.Providers[p.Provider]
	if !known {
		return fmt.Errorf("Fournisseur inconnu : %s", p.Provider)
	}
	if _, _, e = resolveModel(provider, p.Level, "work"); e != nil {
		return e
	}
	confirmedPreview := ""
	if len(events) > 1 {
		confirmedPreview = events[1]
	}
	rawRequest, _ := json.Marshal(map[string]any{"profile": p, "slots": slots, "revision": revision, "preview_token": confirmedPreview})
	p.Workspace, e = resolveWorkspace(s.root, p.Workspace)
	if e != nil {
		return e
	}
	if p.Role == "" {
		p.Role = "worker"
	}
	if p.Role != "worker" && p.Role != "planner" && p.Role != "subplanner" {
		return fmt.Errorf("Rôle inconnu")
	}
	if p.Limits != nil {
		limits, err := p.Limits.normalized()
		if err != nil {
			return err
		}
		p.Limits = &limits
	}
	p.Updated = now()
	p.Actor = originOperator
	event := newID("mission-")
	if len(events) > 0 && events[0] != "" {
		event = events[0]
	}
	_, e = s.mutateWithHook(work, "mission.start", event, revision, rawRequest, func(w *Work) error {
		if err := organizationGuard(*w); err != nil {
			return err
		}
		w.Profile = &p
		return nil
	}, func(tx *sql.Tx, w *Work) error {
		if _, err := tx.Exec("INSERT INTO cockpit_controls(work_id,autonomy,slots,paused) VALUES(?,?,?,0) ON CONFLICT(work_id) DO UPDATE SET autonomy=excluded.autonomy,slots=excluded.slots,paused=0", work, autonomyAuto, slots); err != nil {
			return err
		}
		raw, _ := json.Marshal(MissionPolicy{true, operatorIdentity(), now()})
		_, err := tx.Exec("INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,?,?)", work, now(), "mission-policy", string(raw))
		return err
	})
	return e
}

const missionPollInterval = 2 * time.Second
const missionRecheckInterval = 30 * time.Second

// This loop belongs to the server (or CLI watch), never to a browser tab.
// Fingerprints suppress idle redispatch/log spam. A global occupancy change also
// wakes work waiting on another work's workspace. Engine CAS deduplicates starts.
func (s *Store) missionLoop(ctx context.Context, scope ...string) {
	previous := map[string]string{}
	conductor := newID("conductor-")
	source := "serveur web"
	if len(scope) > 0 {
		source = "mission watch"
	}
	registered := map[string]bool{}
	tick := time.NewTicker(missionPollInterval)
	defer func() {
		tick.Stop()
		for work := range registered {
			s.stopMissionSupervision(work, conductor, time.Now())
		}
	}()
	for {
		s.missionOwnedCycle(previous, registered, conductor, source, scope...)
		select {
		case <-ctx.Done():
			return
		case <-tick.C:
		}
	}
}

// missionOwnedCycle is the only production path that reconciles and starts
// automatic work. The persisted lease deduplicates web servers and CLI watches;
// launch transactions remain the final CAS for budget, dependency freshness,
// workspace reservation and task revision.
func (s *Store) missionOwnedCycle(previous map[string]string, registered map[string]bool, conductor, source string, scope ...string) {
	works, err := s.list()
	if err != nil {
		return
	}
	var occupancy string
	if err = s.db.QueryRow("SELECT coalesce(group_concat(id || ':' || status),'') FROM (SELECT id,status FROM agents WHERE status IN ('queued','starting','running','stopping') ORDER BY id)").Scan(&occupancy); err != nil {
		return
	}
	for _, work := range works {
		if len(scope) > 0 && scope[0] != work.ID {
			continue
		}
		if archived, archiveErr := s.archived(work.ID); archiveErr != nil || archived {
			if registered[work.ID] {
				s.stopMissionSupervision(work.ID, conductor, time.Now())
			}
			delete(registered, work.ID)
			delete(previous, work.ID)
			continue
		}
		// Une demande sans réponse est un état borné, pas une attente infinie.
		// Seul le conducteur possédant la mission effectue cette transition.
		at := time.Now()
		if !registered[work.ID] {
			owned, claimErr := s.claimMissionSupervision(work.ID, conductor, source, at)
			if claimErr != nil || !owned {
				continue
			}
			registered[work.ID] = true
			_ = s.recordCoordinationEvent("lease:"+work.ID+":"+conductor, work.ID, "", conductor,
				"lease-acquired", "Possession bornée du conducteur acquise")
		}
		cycleErr := s.reconcileMissionAttempts(work.ID, conductor, s.spawnAgent)
		if cycleErr == nil {
			_, cycleErr = s.expireAgentExchanges(work.ID, conductor, time.Now())
		}
		if cycleErr == nil {
			current, getErr := s.get(work.ID)
			if getErr != nil {
				cycleErr = getErr
			} else {
				cycleErr = s.missionCycleWork(current, occupancy, previous, conductor)
			}
		}
		if heartbeatErr := s.checkMissionSupervision(work.ID, conductor, time.Now(), cycleErr); heartbeatErr != nil {
			delete(registered, work.ID)
			delete(previous, work.ID)
		}
	}
}
func (s *Store) missionCycle(previous map[string]string, scope ...string) {
	s.missionCycleObserved(previous, nil, scope...)
}

func (s *Store) missionCycleObserved(previous map[string]string, observed func(string, bool, error), scope ...string) {
	works, e := s.list()
	if e != nil {
		return
	}
	var occupancy string
	if e = s.db.QueryRow("SELECT coalesce(group_concat(id || ':' || status),'') FROM (SELECT id,status FROM agents WHERE status IN ('queued','starting','running','stopping') ORDER BY id)").Scan(&occupancy); e != nil {
		if observed != nil {
			for _, w := range works {
				if len(scope) > 0 && scope[0] != w.ID {
					continue
				}
				observed(w.ID, true, nil)
				observed(w.ID, false, e)
			}
		}
		return
	}
	for _, w := range works {
		if len(scope) > 0 && scope[0] != w.ID {
			continue
		}
		if archived, archiveErr := s.archived(w.ID); archiveErr != nil || archived {
			if observed != nil && archiveErr != nil {
				observed(w.ID, true, nil)
				observed(w.ID, false, archiveErr)
			}
			delete(previous, w.ID)
			continue
		}
		if observed != nil {
			observed(w.ID, true, nil)
		}
		cycleErr := s.missionCycleWork(w, occupancy, previous)
		if observed != nil {
			observed(w.ID, false, cycleErr)
		}
	}
}

func (s *Store) missionCycleWork(w Work, occupancy string, previous map[string]string, conductors ...string) error {
	p, e := s.missionPolicy(w.ID)
	if e != nil {
		return e
	}
	if !p.Enabled || s.paused(w.ID) || s.autonomy(w.ID) != autonomyAuto {
		delete(previous, w.ID)
		return nil
	}
	if e := organizationGuard(w); e != nil {
		return e
	}
	if w.Planning != nil && w.Planning.Reviewer != nil {
		go func() { s.reviewFailure(w.ID, s.independentReviewStep(w.ID)) }()
	}
	// Inference must not block conductor heartbeats or other missions. A durable
	// lease arbitrates competing passes, including another server process.
	if w.Planning != nil && !w.Planning.Paused && w.Planning.Failure == "" && w.Planning.Provider != "" {
		go func() { _ = s.planningStep(w.ID) }()
	}
	changed, e := s.resumeAutomaticValidations(w.ID)
	if e != nil {
		return e
	}
	if changed {
		w, e = s.get(w.ID)
		if e != nil {
			return e
		}
	}
	b, e := s.budget(w.ID)
	if e != nil {
		return e
	}
	var control int64
	if e = s.db.QueryRow("SELECT coalesce(max(rowid),0) FROM cockpit_events WHERE work_id=? AND kind!='dispatch'", w.ID).Scan(&control); e != nil {
		return e
	}
	fingerprint, _ := json.Marshal([]any{w.Revision, occupancy, b, s.slots(w.ID), p, control, time.Now().Unix() / int64(missionRecheckInterval/time.Second)})
	key := string(fingerprint)
	if previous[w.ID] == key {
		return nil
	}
	if _, e = s.dispatch(w.ID, conductors...); e != nil {
		return e
	}
	previous[w.ID] = key
	return nil
}

// Recheck scheduling decisions in the same transaction as reservation/intent.
func automaticLaunchGuard(tx *sql.Tx, work string, conductors ...string) error {
	if e := organizationGuardTx(tx, work); e != nil {
		return e
	}
	var mode string
	var paused, slots int
	e := tx.QueryRow("SELECT autonomy,paused,slots FROM cockpit_controls WHERE work_id=?", work).Scan(&mode, &paused, &slots)
	if e == sql.ErrNoRows {
		mode = autonomyDefault
		slots = slotsDefault
	} else if e != nil {
		return e
	}
	if mode != autonomyAuto || paused != 0 {
		return fmt.Errorf("Départ automatique suspendu ou autonomie désactivée")
	}
	var policyRaw string
	e = tx.QueryRow("SELECT message FROM cockpit_events WHERE work_id=? AND kind='mission-policy' ORDER BY rowid DESC LIMIT 1", work).Scan(&policyRaw)
	if e != nil && e != sql.ErrNoRows {
		return e
	}
	if e == nil {
		var p MissionPolicy
		if e = json.Unmarshal([]byte(policyRaw), &p); e != nil {
			return e
		}
		if !p.Enabled {
			return fmt.Errorf("Autorisation de mission révoquée")
		}
	}
	if len(conductors) > 0 && conductors[0] != "" {
		var owner, heartbeat, stopped string
		if e = tx.QueryRow("SELECT conductor_id,heartbeat_at,stopped_at FROM mission_supervision WHERE work_id=?", work).Scan(&owner, &heartbeat, &stopped); e != nil {
			return fmt.Errorf("bail du conducteur indisponible : %w", e)
		}
		seen, parseErr := time.Parse(time.RFC3339Nano, heartbeat)
		age := time.Since(seen)
		if owner != conductors[0] || stopped != "" || parseErr != nil || age > missionConductorStaleAfter || age < -missionClockFutureTolerance {
			return fmt.Errorf("bail du conducteur perdu ; décision de départ annulée")
		}
	}
	var active int
	if e = tx.QueryRow("SELECT count(*) FROM agents WHERE work_id=? AND status IN ('queued','starting','running','stopping')", work).Scan(&active); e != nil {
		return e
	}
	if active >= slots {
		return fmt.Errorf("Créneaux occupés : %d sur %d ; reprise à leur libération", active, slots)
	}
	return nil
}

func (s *Store) stopMission(work string) error {
	if _, e := s.get(work); e != nil {
		return e
	}
	tx, e := s.db.Begin()
	if e != nil {
		return e
	}
	defer tx.Rollback()
	if _, e = tx.Exec("INSERT INTO cockpit_controls(work_id,paused) VALUES(?,1) ON CONFLICT(work_id) DO UPDATE SET paused=1", work); e != nil {
		return e
	}
	raw, _ := json.Marshal(MissionPolicy{false, operatorIdentity(), now()})
	if _, e = tx.Exec("INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,?,?)", work, now(), "mission-policy", string(raw)); e != nil {
		return e
	}
	return tx.Commit()
}
