//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"strings"
	"testing"
	"time"
)

func TestMissionProviderCooldownExplainsQuotaWithoutPromisingRestart(t *testing.T) {
	for _, unknown := range []bool{false, true} {
		t.Run(map[bool]string{false: "known-reset", true: "unknown-reset"}[unknown], func(t *testing.T) {
			s := storeTest(t)
			w, _ := setupAgent(t, s)
			organizedFixtureStore(t, s)
			w, _ = s.get(w.ID)
			w.Planning.Failure = "exit status 1 :"
			raw, _ := json.Marshal(w)
			if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
				t.Fatal(err)
			}
			if _, err := s.db.Exec("INSERT OR REPLACE INTO cockpit_controls(work_id,paused) VALUES(?,1)", w.ID); err != nil {
				t.Fatal(err)
			}
			reset := time.Now().Add(time.Hour).Unix()
			if unknown {
				reset = 0
			}
			if err := s.recordProviderCooldown(w.Planning.Provider, "test-provider-process", &ProviderCooldown{ObservedAt: now(), ResetAt: reset, Signal: "result.api_error_status.429"}); err != nil {
				t.Fatal(err)
			}
			status, err := s.missionStatus(w.ID)
			if err != nil {
				t.Fatal(err)
			}
			if len(status.ProviderCooldowns) != 1 || !status.Paused || status.ActiveAgents != 0 || status.Validated != 0 {
				t.Fatalf("quota changed execution facts: %+v", status)
			}
			if !strings.Contains(status.Understanding.What, "quota refusé") || strings.Contains(status.Understanding.What, "exit status") || !strings.Contains(status.Next, "aucun redémarrage automatique") {
				t.Fatalf("misleading recovery guidance: %+v", status.Understanding)
			}
			if unknown && (status.Understanding.ActorKind != "user" || !strings.Contains(status.Understanding.What, "Aucune heure")) {
				t.Fatalf("invented recovery time: %+v", status.Understanding)
			}
			var out, errors bytes.Buffer
			if code := run([]string{"--root", s.root, "--json", "mission", "status", w.ID}, &out, &errors); code != 0 {
				t.Fatal(code, errors.String())
			}
			var cli MissionStatus
			if err := json.Unmarshal(out.Bytes(), &cli); err != nil || cli.Understanding != status.Understanding || cli.ProviderCooldowns[0].ResetAt != reset {
				t.Fatal("CLI differs from shared web status", out.String(), err)
			}
			after, _ := s.get(w.ID)
			if after.Revision != w.Revision || len(after.Tasks[0].Attempts) != len(w.Tasks[0].Attempts) {
				t.Fatal("reading status consumed an attempt or changed the work")
			}
		})
	}
}

func TestMissionProviderCooldownFiltersExpiredAndUnrelatedProviders(t *testing.T) {
	s := storeTest(t)
	w := Work{Profile: &LaunchProfile{Provider: "used"}}
	at := time.Now()
	for _, name := range []string{"used", "unrelated"} {
		if err := s.recordProviderCooldown(name, "fixture", &ProviderCooldown{ObservedAt: now(), ResetAt: at.Add(time.Minute).Unix(), Signal: "rate_limit_event.rejected"}); err != nil {
			t.Fatal(err)
		}
	}
	active, err := s.missionProviderCooldowns(w, at)
	if err != nil || len(active) != 1 || active[0].Provider != "used" {
		t.Fatal(active, err)
	}
	active, err = s.missionProviderCooldowns(w, at.Add(2*time.Minute))
	if err != nil || len(active) != 0 {
		t.Fatal("expired quota still presented as active", active, err)
	}
}

func TestMissionProviderCooldownDoesNotHideExhaustedAttempts(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	organizedFixtureStore(t, s)
	w, _ = s.get(w.ID)
	w.Tasks[0].Status = "blocked"
	w.Tasks[0].Blocker = "quota"
	w.Tasks[0].PlanMaxAttempts = 2
	w.Tasks[0].Attempts = []Attempt{{ID: "first", Status: "failed"}, {ID: "second", Status: "failed"}}
	raw, _ := json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	if err := s.recordProviderCooldown(w.Planning.Provider, "fixture", &ProviderCooldown{ObservedAt: now(), ResetAt: time.Now().Add(time.Hour).Unix(), Signal: "rate_limit_event.rejected"}); err != nil {
		t.Fatal(err)
	}
	status, err := s.missionStatus(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	if status.Understanding.ActorKind != "user" || !strings.Contains(status.Understanding.What, "La fin du quota ne débloquera pas ces tâches") || !strings.Contains(status.Next, "aucun nouveau départ n’est autorisé pour elles") {
		t.Fatal("deadline presented as sufficient despite exhausted attempts", status.Understanding)
	}
}
