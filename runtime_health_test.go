package main

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"syscall"
	"testing"
	"time"
)

func TestRuntimeHealthThresholdsAndUnavailableVolumes(t *testing.T) {
	for _, tc := range []struct {
		name          string
		bytes, inodes uint64
		err           error
		state         string
		allowed       bool
	}{
		{"healthy", 2 * storageWarningBytes, 1000, nil, "ready", true},
		{"warning", storageStopBytes, 1000, nil, "warning", true},
		{"low", storageStopBytes - 1, 1000, nil, "blocked", false},
		{"full", 0, 1000, nil, "blocked", false},
		{"inodes", 2 * storageWarningBytes, 0, nil, "blocked", false},
		{"unknown", 0, 0, errors.New("permission denied"), "blocked", false},
	} {
		t.Run(tc.name, func(t *testing.T) {
			s := &Store{root: t.TempDir(), storageProbe: func(string) (uint64, uint64, error) { return tc.bytes, tc.inodes, tc.err }}
			h := s.runtimeHealth()
			if h.State != tc.state || h.LaunchAllowed != tc.allowed || len(h.Volumes) != 2 {
				t.Fatalf("%+v", h)
			}
			if (s.storageGuard() == nil) != tc.allowed {
				t.Fatal("guard and diagnosis disagree")
			}
		})
	}
	s := &Store{root: t.TempDir(), storageProbe: func(path string) (uint64, uint64, error) {
		if path == os.TempDir() {
			return 0, 1000, nil
		}
		return 2 * storageWarningBytes, 1000, nil
	}}
	if s.runtimeHealth().LaunchAllowed {
		t.Fatal("temporary filesystem not checked")
	}
}

func TestStorageFailureUses507AndStructuredDiagnosis(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	s.storageProbe = func(string) (uint64, uint64, error) { return 0, 1000, nil }
	body, _ := json.Marshal(webRequest{Kind: "start", Work: w.ID, Task: r.TaskID, Event: r.EventID, Revision: w.Revision, Provider: r.Provider})
	req := httptest.NewRequest("POST", "http://local.test/api/v1/action", bytes.NewReader(body))
	req.AddCookie(&http.Cookie{Name: "swarm_session", Value: "health-token"})
	req.Header.Set("Origin", "http://local.test")
	req.Header.Set("X-Swarm-CSRF", "health-token")
	rr := httptest.NewRecorder()
	newWebHandler(s, "local.test", "health-token").ServeHTTP(rr, req)
	if rr.Code != 507 || !strings.Contains(rr.Body.String(), "storage_unavailable") {
		t.Fatal(rr.Code, rr.Body.String())
	}
	if f := commandFailure(fmt.Errorf("save journal: %w", syscall.ENOSPC)); f.Code != "storage_unavailable" || f.Retryable {
		t.Fatal(f)
	}
	// Force an actual SQLite FULL error in the isolated fixture, not on the host.
	s.storageProbe = nil
	if _, err := s.db.Exec("PRAGMA max_page_count=1"); err != nil {
		t.Fatal(err)
	}
	_, err := s.db.Exec("CREATE TABLE disk_full AS SELECT zeroblob(1048576) AS value")
	if err == nil || commandFailure(err).Code != "storage_unavailable" {
		t.Fatal("SQLite FULL was not diagnosed", err)
	}
}

func TestRuntimeHealthEndpointSurvivesDatabaseFailure(t *testing.T) {
	s := storeTest(t)
	s.storageProbe = func(string) (uint64, uint64, error) { return 0, 1000, nil }
	s.db.Close()
	h := newWebHandler(s, "local.test", "health-token")
	for _, auth := range []bool{false, true} {
		req := httptest.NewRequest("GET", "http://local.test/api/v1/runtime-health", nil)
		if auth {
			req.AddCookie(&http.Cookie{Name: "swarm_session", Value: "health-token"})
		}
		rr := httptest.NewRecorder()
		h.ServeHTTP(rr, req)
		if !auth {
			if rr.Code != 403 {
				t.Fatal("health endpoint bypasses session")
			}
			continue
		}
		var health RuntimeHealth
		if rr.Code != 200 || json.Unmarshal(rr.Body.Bytes(), &health) != nil || health.State != "blocked" {
			t.Fatal(rr.Code, rr.Body.String())
		}
	}
}

func TestRuntimeHealthBlocksWithoutChargeAndResumesConductor(t *testing.T) {
	s := storeTest(t)
	w, launch := setupAgent(t, s)
	s.storageProbe = func(string) (uint64, uint64, error) { return 0, 1000, nil }
	before, _ := s.get(w.ID)
	if _, _, err := s.prepareLaunch(w.ID, launch, false); err == nil || commandFailure(err).Code != "storage_unavailable" {
		t.Fatal(err)
	}
	for _, err := range []error{s.planningStep(w.ID), s.independentReviewStep(w.ID), s.spawnAgent(Agent{}), s.reviewManagedCandidate(w, Agent{}, "", "", nil)} {
		if err == nil || commandFailure(err).Code != "storage_unavailable" {
			t.Fatal(err)
		}
	}
	previous, registered := map[string]string{}, map[string]bool{}
	s.missionOwnedCycle(previous, registered, "test-conductor", "mission watch", w.ID)
	if len(registered) != 0 {
		t.Fatal("conductor wrote while storage blocked")
	}
	after, _ := s.get(w.ID)
	agents, _ := s.agents(w.ID)
	if !reflect.DeepEqual(before, after) || len(agents) != 0 {
		t.Fatal("blocked preflight charged or changed mission")
	}
	status, err := s.missionStatus(w.ID)
	if err != nil || status.Enabled || status.Understanding.Situation != "incident_stockage" {
		t.Fatal(status, err)
	}
	s.storageProbe = func(string) (uint64, uint64, error) { return 2 * storageWarningBytes, 1000, nil }
	s.missionOwnedCycle(previous, registered, "test-conductor", "mission watch", w.ID)
	supervision, err := s.missionSupervision(w.ID, time.Now())
	if err != nil || supervision.State != "active" || supervision.LastCheckAt == "" {
		t.Fatal(supervision, err)
	}
	after, _ = s.get(w.ID)
	if !reflect.DeepEqual(before, after) {
		t.Fatal("recovery changed task results")
	}
	if _, created, err := s.prepareLaunch(w.ID, launch, false); err != nil || !created {
		t.Fatal("launch still blocked after recovery", err)
	}
}

func TestDoctorReadsWithoutOpeningDatabase(t *testing.T) {
	root := t.TempDir()
	os.Mkdir(filepath.Join(root, ".swarm"), 0700)
	os.WriteFile(filepath.Join(root, ".swarm/state.db"), []byte("invalid SQLite"), 0600)
	var out, errs bytes.Buffer
	if code := run([]string{"--root", root, "--json", "doctor"}, &out, &errs); code != 0 {
		t.Fatal(code, errs.String(), out.String())
	}
	var h RuntimeHealth
	if json.Unmarshal(out.Bytes(), &h) != nil || !h.LaunchAllowed {
		t.Fatal(out.String())
	}
	raw, _ := os.ReadFile(filepath.Join(root, ".swarm/state.db"))
	if string(raw) != "invalid SQLite" {
		t.Fatal("doctor modified database")
	}
}

func TestExhaustedMissionExposesDecisionInsteadOfGenericRetry(t *testing.T) {
	s, w := exhaustedTaskFixture(t)
	d, err := s.missionStatus(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task := d.Tasks[0]
	if !task.AttemptLimitReached || task.Action != "recovery" || task.Understanding.Situation != "limite_tentatives" || !strings.Contains(task.Understanding.What, "2/2") {
		t.Fatal(task)
	}
	var out bytes.Buffer
	if err := missionCLI(s, []string{"mission", "status", w.ID}, "", false, &out); err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(out.String(), "Aucun redémarrage automatique") || !strings.Contains(out.String(), "Missing evidence") {
		t.Fatal(out.String())
	}
	after, _ := s.get(w.ID)
	if !reflect.DeepEqual(w, after) {
		t.Fatal("reading diagnosis mutated mission")
	}
	// The final authorized attempt can still be under review. Exhausted launch
	// allowance must not hide that review or claim its result already failed.
	w.Tasks[0].IndependentReview.State = "running"
	raw, _ := json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	d, err = s.missionStatus(w.ID)
	if err != nil || d.Tasks[0].AttemptLimitReached {
		t.Fatal("active review presented as terminal exhaustion", d, err)
	}
}
