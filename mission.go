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
	rawRequest, _ := json.Marshal(map[string]any{"profile": p, "slots": slots, "revision": revision})
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
	_, e = s.mutateWithHook(work, "mission.start", event, revision, rawRequest, func(w *Work) error { w.Profile = &p; return nil }, func(tx *sql.Tx, w *Work) error {
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
	tick := time.NewTicker(missionPollInterval)
	defer tick.Stop()
	for {
		s.missionCycle(previous, scope...)
		select {
		case <-ctx.Done():
			return
		case <-tick.C:
		}
	}
}
func (s *Store) missionCycle(previous map[string]string, scope ...string) {
	works, e := s.list()
	if e != nil {
		return
	}
	var occupancy string
	if e = s.db.QueryRow("SELECT coalesce(group_concat(id || ':' || status),'') FROM (SELECT id,status FROM agents WHERE status IN ('queued','starting','running','stopping') ORDER BY id)").Scan(&occupancy); e != nil {
		return
	}
	for _, w := range works {
		if len(scope) > 0 && scope[0] != w.ID {
			continue
		}
		p, e := s.missionPolicy(w.ID)
		if e != nil || !p.Enabled || s.paused(w.ID) || s.autonomy(w.ID) != autonomyAuto {
			delete(previous, w.ID)
			continue
		}
		b, e := s.budget(w.ID)
		if e != nil {
			continue
		}
		var control int64
		if e = s.db.QueryRow("SELECT coalesce(max(rowid),0) FROM cockpit_events WHERE work_id=? AND kind!='dispatch'", w.ID).Scan(&control); e != nil {
			continue
		}
		fingerprint, _ := json.Marshal([]any{w.Revision, occupancy, b, s.slots(w.ID), p, control, time.Now().Unix() / int64(missionRecheckInterval/time.Second)})
		key := string(fingerprint)
		if previous[w.ID] == key {
			continue
		}
		if _, e = s.dispatch(w.ID); e != nil {
			continue
		}
		previous[w.ID] = key
	}
}

// Recheck scheduling decisions in the same transaction as reservation/intent.
func automaticLaunchGuard(tx *sql.Tx, work string) error {
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
