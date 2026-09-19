//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// These tests use authenticated HTTP/CLI and deterministic subprocess fixtures.
// They prove engine boundaries, not autonomous planning quality.
func ownershipHTTP(t *testing.T, s *Store, path string, value any) (int, string) {
	t.Helper()
	raw, err := json.Marshal(value)
	if err != nil {
		t.Fatal(err)
	}
	r := httptest.NewRequest("POST", "http://localhost:18787"+path, bytes.NewReader(raw))
	r.AddCookie(&http.Cookie{Name: "swarm_session", Value: "ownership-test"})
	r.Header.Set("Origin", "http://localhost:18787")
	r.Header.Set("X-Swarm-CSRF", "ownership-test")
	rec := httptest.NewRecorder()
	newWebHandler(s, "localhost:18787", "ownership-test").ServeHTTP(rec, r)
	return rec.Code, rec.Body.String()
}

func ownershipPlanning(t *testing.T, s *Store, entry, work, action string, r PlanningRequest) (Work, error) {
	t.Helper()
	if entry == "http" {
		code, body := ownershipHTTP(t, s, "/api/v1/planning?work="+work+"&action="+action, r)
		if code != http.StatusOK {
			return Work{}, fmt.Errorf("HTTP %d: %s", code, body)
		}
	} else {
		path := filepath.Join(t.TempDir(), "request.json")
		raw, _ := json.Marshal(r)
		if err := os.WriteFile(path, raw, 0600); err != nil {
			t.Fatal(err)
		}
		var out, diagnostics bytes.Buffer
		if code := run([]string{"--root", s.root, "--json", "planning", action, work, "--input", path}, &out, &diagnostics); code != 0 {
			return Work{}, fmt.Errorf("CLI %d: %s %s", code, out.String(), diagnostics.String())
		}
	}
	return s.get(work)
}

func ownershipUnchanged(t *testing.T, s *Store, before Work) {
	t.Helper()
	next, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer next.db.Close()
	after, err := next.get(before.ID)
	if err != nil {
		t.Fatal(err)
	}
	want, _ := json.Marshal(before)
	got, _ := json.Marshal(after)
	if !bytes.Equal(want, got) {
		t.Fatal("refused operation changed durable ownership, leases, budgets or task state")
	}
}

func TestEngineContractOwnershipRejectsExecutablePlannerRoles(t *testing.T) {
	for _, role := range []string{"planner", "subplanner"} {
		for _, entry := range []string{"prepare", "http", "cli"} {
			t.Run(role+"/"+entry, func(t *testing.T) {
				s := storeTest(t)
				w, r := setupAgent(t, s)
				organizedFixtureStore(t, s)
				w, _ = s.get(w.ID)
				r.Revision, r.Role = w.Revision, role
				var diagnostic string
				switch entry {
				case "prepare":
					_, _, err := s.prepare(w.ID, r)
					if err == nil {
						t.Fatal("planner silently became a coding worker")
					}
					diagnostic = err.Error()
				case "http":
					code, body := ownershipHTTP(t, s, "/api/v1/action", webRequest{Kind: "start", Work: w.ID, Task: r.TaskID, Event: r.EventID, Revision: r.Revision, Provider: r.Provider, Role: role})
					if code == http.StatusOK {
						t.Fatal("HTTP launched a planner as worker", body)
					}
					diagnostic = body
				case "cli":
					path := filepath.Join(t.TempDir(), "launch.json")
					raw, _ := json.Marshal(r)
					if err := os.WriteFile(path, raw, 0600); err != nil {
						t.Fatal(err)
					}
					var out, diagnostics bytes.Buffer
					if code := run([]string{"--root", s.root, "--json", "agent", "start", w.ID, "--input", path}, &out, &diagnostics); code == 0 {
						t.Fatal("CLI launched a planner as worker")
					}
					diagnostic = out.String() + diagnostics.String()
				}
				if !strings.Contains(diagnostic, "responsable") {
					t.Fatal("wrong refusal:", diagnostic)
				}
				ownershipUnchanged(t, s, w)
				for _, table := range []string{"agents", "reservations", "managed_attempts"} {
					var count int
					if err := s.db.QueryRow("SELECT count(*) FROM " + table).Scan(&count); err != nil || count != 0 {
						t.Fatal("refused role reserved work", table, count, err)
					}
				}
			})
		}
	}
}

func TestEngineContractOwnershipManagedRoleRefusalCreatesNoCopy(t *testing.T) {
	s, w := managedFixture(t)
	storage := w.Planning.Repository.Storage
	for _, role := range []string{"planner", "subplanner"} {
		r := Launch{Schema: 1, EventID: newID("forbidden-owner-"), Revision: w.Revision, TaskID: "first", Provider: "managed-review-fixture", Role: role}
		if _, _, err := s.prepare(w.ID, r); err == nil || !strings.Contains(err.Error(), "responsable") {
			t.Fatal("managed planner launch was not refused", err)
		}
		for _, folder := range []string{"copies"} {
			items, err := os.ReadDir(filepath.Join(storage, folder))
			if err != nil && !os.IsNotExist(err) {
				t.Fatal(err)
			}
			if len(items) != 0 {
				t.Fatal("refusal left managed artifacts", folder, items)
			}
		}
		manifests, err := filepath.Glob(filepath.Join(storage, "copy-*.json"))
		if err != nil || len(manifests) != 0 {
			t.Fatal("refusal left a managed copy manifest", manifests, err)
		}
		ownershipUnchanged(t, s, w)
	}
}

func TestEngineContractOwnershipDefaultRoleStillWorker(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	organizedFixtureStore(t, s)
	w, _ = s.get(w.ID)
	r.Revision = w.Revision
	a, created, err := s.prepare(w.ID, r)
	if err != nil || !created || a.Role != "worker" {
		t.Fatal("default worker regressed", a, err)
	}
}

func ownershipTree(t *testing.T, entry string) (*Store, Work) {
	t.Helper()
	s := storeTest(t)
	w := createTest(t, s)
	w = applyTest(t, s, w, "work.update", Request{Criteria: []string{"A", "B", "Root"}})
	w = planningDo(t, s, w, "enable", PlanningRequest{MaxTasks: 10, MaxDecisions: 20, MaxActivations: 30})
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{{Kind: "delegate", ID: "child-a", Title: "Own A", Requirements: []string{"req-1"}}, {Kind: "delegate", ID: "child-b", Title: "Own B", Requirements: []string{"req-2"}}}
	w, err := ownershipPlanning(t, s, entry, w.ID, "decide", r)
	if err != nil {
		t.Fatal(err)
	}
	owners := map[string]string{}
	for _, scope := range w.Planning.Scopes {
		for _, requirement := range scope.Requirements {
			if owners[requirement] != "" {
				t.Fatal("shared ownership", requirement)
			}
			owners[requirement] = scope.ID
		}
	}
	if owners["req-1"] != "child-a" || owners["req-2"] != "child-b" || owners["req-3"] != "root" {
		t.Fatal(owners)
	}
	return s, w
}

func TestEngineContractOwnershipRejectsForeignOperationsAtomically(t *testing.T) {
	for _, entry := range []string{"http", "cli"} {
		for _, kind := range []string{"foreign-task", "foreign-delegate", "foreign-dependency", "foreign-retry", "ancestor-reclaims", "foreign-event"} {
			t.Run(entry+"/"+kind, func(t *testing.T) {
				s, w := ownershipTree(t, entry)
				w, r := planningClaim(t, s, w, "child-b")
				op := planningTask("sibling-task")
				op.Requirements = []string{"req-2"}
				r.Operations = []PlanningOperation{op}
				w, err := ownershipPlanning(t, s, entry, w.ID, "decide", r)
				if err != nil {
					t.Fatal(err)
				}
				w = applyTest(t, s, w, "task.update", Request{ID: "sibling-task", Status: "blocked", Blocker: "fixture refusal"})
				scope := "child-a"
				if kind == "ancestor-reclaims" {
					w = planningDo(t, s, w, "resume", PlanningRequest{Scope: "root", Reason: "Inspect delegated result ownership"})
					scope = "root"
				}
				w, r = planningClaim(t, s, w, scope)
				op = planningTask("illicit")
				want := "non possédée"
				switch kind {
				case "foreign-task":
					op.Requirements = []string{"req-2"}
				case "foreign-delegate":
					op.Kind, op.Requirements = "delegate", []string{"req-2"}
				case "foreign-dependency":
					op.Depends, want = []string{"sibling-task"}, "hors périmètre"
				case "foreign-retry":
					op = PlanningOperation{Kind: "retry", ID: "sibling-task", Next: "New correction from the wrong owner"}
					want = "reprise non disponible"
				case "foreign-event":
					for _, event := range w.Planning.Inbox {
						if event.Scope == "child-b" {
							r.Inputs = []string{event.ID}
							break
						}
					}
					want = "hors périmètre"
				}
				// A valid operation before the rejected one must also roll back.
				legitimate := planningTask("must-rollback")
				if scope == "root" {
					legitimate.Requirements = []string{"req-3"}
				}
				r.Operations = []PlanningOperation{legitimate, op}
				if _, err = ownershipPlanning(t, s, entry, w.ID, "decide", r); err == nil || !strings.Contains(err.Error(), want) {
					t.Fatal("ownership boundary not enforced", err)
				}
				ownershipUnchanged(t, s, w)
			})
		}
	}
}

func TestEngineContractOwnershipParentCannotCloseIncompleteDescendants(t *testing.T) {
	for _, entry := range []string{"http", "cli"} {
		for _, scope := range []string{"root", "child-a"} {
			t.Run(entry+"/"+scope, func(t *testing.T) {
				s, w := ownershipTree(t, entry)
				w, r := planningClaim(t, s, w, "child-a")
				r.Operations = []PlanningOperation{{Kind: "delegate", ID: "grandchild", Title: "Own leaf", Requirements: []string{"req-1"}}}
				w, err := ownershipPlanning(t, s, entry, w.ID, "decide", r)
				if err != nil {
					t.Fatal(err)
				}
				w = planningDo(t, s, w, "resume", PlanningRequest{Scope: scope, Reason: "Attempt premature parent closure"})
				w, r = planningClaim(t, s, w, scope)
				r.Operations = []PlanningOperation{{Kind: "close"}}
				if _, err = ownershipPlanning(t, s, entry, w.ID, "decide", r); err == nil || !strings.Contains(err.Error(), "enfant non terminé") {
					t.Fatal("parent closed prematurely", err)
				}
				ownershipUnchanged(t, s, w)
			})
		}
	}
}

func TestEngineContractOwnershipPlannerProcessDisablesCodingTools(t *testing.T) {
	for _, scope := range []string{"root", "child-a"} {
		t.Run(scope, func(t *testing.T) {
			s, w := ownershipTree(t, "http")
			if scope == "root" {
				w = planningDo(t, s, w, "resume", PlanningRequest{Scope: scope, Reason: "Inspect root planner process"})
			}
			// The fixture captures only argv, cwd and the actual scope context.
			folder := t.TempDir()
			command := filepath.Join(folder, "claude")
			observed := filepath.Join(folder, "observed.json")
			script := "#!/usr/bin/env python3\nimport json,os,sys\np=sys.stdin.read()\nwith open(" + fmt.Sprintf("%q", observed) + ",'w') as f: json.dump({'args':sys.argv[1:],'cwd':os.getcwd(),'prompt':p},f)\nprint(json.dumps({'type':'result','result':'{}'}))\n"
			if err := os.WriteFile(command, []byte(script), 0700); err != nil {
				t.Fatal(err)
			}
			context, err := planningContext(w, scope)
			if err != nil {
				t.Fatal(err)
			}
			provider := Provider{Command: command, Args: []string{"--dangerously-skip-permissions", "--resume", "producer-session", "--allowedTools", "Bash,Write"}}
			if _, err = runPlanningProvider(provider, string(context), time.Second*5, func() bool { return true }); err != nil {
				t.Fatal(err)
			}
			data, err := os.ReadFile(observed)
			if err != nil {
				t.Fatal(err)
			}
			var result struct {
				Args   []string `json:"args"`
				CWD    string   `json:"cwd"`
				Prompt string   `json:"prompt"`
			}
			if err = json.Unmarshal(data, &result); err != nil {
				t.Fatal(err)
			}
			toolsDisabled, ephemeral := false, false
			for i, arg := range result.Args {
				if arg == "--tools" && i+1 < len(result.Args) && result.Args[i+1] == "" {
					toolsDisabled = true
				}
				if arg == "--no-session-persistence" {
					ephemeral = true
				}
				if arg == "--dangerously-skip-permissions" || arg == "--resume" || arg == "--allowedTools" {
					t.Fatal("coding/session arguments leaked", result.Args)
				}
			}
			if !toolsDisabled || !ephemeral || result.CWD == s.root || result.Prompt != string(context) {
				t.Fatal("planner process is not isolated and tool-free", result)
			}
		})
	}
}
