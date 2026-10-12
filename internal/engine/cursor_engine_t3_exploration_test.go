//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

func TestCursorEngineT3ExplorationReadAndPersistentDiscovery(t *testing.T) {
	s, w := managedFixture(t)
	candidate := w.Planning.Repository.Candidate
	// The managed source checkout is deliberately dirty; exploration must read
	// the immutable accepted Git candidate, never these unpublished bytes.
	if err := os.WriteFile(filepath.Join(s.root, "project", "value.txt"), []byte("unpublished content\n"), 0600); err != nil {
		t.Fatal(err)
	}
	w = planningDo(t, s, w, "resume", PlanningRequest{Scope: "root", Reason: "Explore the accepted source before adapting the task graph"})
	w, r := planningClaim(t, s, w, "root")
	request, _ := json.Marshal(map[string]any{"mode": "read", "candidate_commit": candidate, "files": []string{"value.txt"}})
	r.Operations = []PlanningOperation{{Kind: "explore", ID: "read-value", Deliverable: string(request)}}
	beforeActivations, beforeDecisions := w.Planning.Activations, w.Planning.Decisions
	mux := http.NewServeMux()
	s.registerPlanning(mux, func(out http.ResponseWriter, v any) { json.NewEncoder(out).Encode(v) }, func(out http.ResponseWriter, e error) { http.Error(out, e.Error(), 400) })
	raw, _ := json.Marshal(r)
	for i := 0; i < 2; i++ {
		out := httptest.NewRecorder()
		mux.ServeHTTP(out, httptest.NewRequest("POST", "/api/v1/planning?work="+w.ID+"&action=decide", bytes.NewReader(raw)))
		if out.Code != http.StatusOK {
			t.Fatal(out.Code, out.Body.String())
		}
	}
	after, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	events := 0
	var proof map[string]any
	for _, event := range after.Planning.Inbox {
		if event.Kind == "exploration" {
			events++
			raw, _ := json.Marshal(event)
			var value map[string]any
			json.Unmarshal(raw, &value)
			proof, _ = value["exploration"].(map[string]any)
		}
	}
	if events != 1 || proof == nil || proof["candidate_commit"] != candidate || proof["scope"] != "root" || proof["observed_revision"] != float64(w.Revision) {
		t.Fatal("exploration missing or misattributed", events, proof)
	}
	raw, _ = json.Marshal(proof)
	if !strings.Contains(string(raw), `initial\n`) || strings.Contains(string(raw), "unpublished content") {
		t.Fatal("exploration read the live checkout or changed original bytes")
	}
	if after.Planning.Activations != beforeActivations || after.Planning.Decisions != beforeDecisions+1 || after.Planning.Repository.Candidate != candidate {
		t.Fatal("exploration spent an extra call or changed the accepted candidate")
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	durable, err := reopened.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	context, err := planningContext(durable, "root")
	if err != nil || !strings.Contains(string(context), `initial\n`) {
		t.Fatal("original discovery not supplied to the next planner", err)
	}
	durable, decision := planningClaim(t, reopened, durable, "root")
	decision.Operations = []PlanningOperation{{Kind: "revise-dependencies", ID: "second", Depends: []string{"first"}}, planningTask("discovered-change")}
	updated, err := reopened.planningChange(w.ID, "decide", decision)
	if err != nil {
		t.Fatal(err)
	}
	if _, err = updated.task("discovered-change"); err != nil {
		t.Fatal("discovery did not adapt durable tasks", err)
	}
}

func TestCursorEngineT3ExplorationRejectsInvalidRequests(t *testing.T) {
	for _, name := range []string{"absolute", "traversal", "duplicate", "missing", "candidate", "generation", "scope", "unknown-field", "shell"} {
		t.Run(name, func(t *testing.T) {
			s, w := managedFixture(t)
			w = planningDo(t, s, w, "resume", PlanningRequest{Scope: "root", Reason: "Read immutable sources with attribution"})
			w, r := planningClaim(t, s, w, "root")
			query := map[string]any{"mode": "read", "candidate_commit": w.Planning.Repository.Candidate, "files": []string{"value.txt"}}
			switch name {
			case "absolute":
				query["files"] = []string{"/etc/passwd"}
			case "traversal":
				query["files"] = []string{"../value.txt"}
			case "duplicate":
				query["files"] = []string{"value.txt", "value.txt"}
			case "missing":
				query["files"] = []string{"absent.txt"}
			case "candidate":
				query["candidate_commit"] = "replaced-candidate"
			case "generation":
				r.Generation--
			case "scope":
				r.Scope = "unowned-scope"
			case "unknown-field":
				query["execute"] = "true"
			case "shell":
				query["mode"] = "shell"
				query["files"] = []string{"echo ignored"}
			}
			raw, _ := json.Marshal(query)
			r.Operations = []PlanningOperation{{Kind: "explore", ID: "invalid-read", Deliverable: string(raw)}}
			if _, err := s.planningChange(w.ID, "decide", r); err == nil {
				t.Fatal("invalid exploration accepted")
			}
			after, err := s.get(w.ID)
			if err != nil {
				t.Fatal(err)
			}
			if after.Planning.Activations != w.Planning.Activations || after.Planning.Decisions != w.Planning.Decisions || after.Planning.Repository.Candidate != w.Planning.Repository.Candidate || len(after.Tasks) != len(w.Tasks) {
				t.Fatal("refusal changed production or spent a call")
			}
			for _, event := range after.Planning.Inbox {
				if event.Kind == "exploration" {
					t.Fatal("refused input persisted as an exploration")
				}
			}
		})
	}
}

func TestCursorEngineT3ExplorationInventoryAndSchemaAvailability(t *testing.T) {
	s, w := managedFixture(t)
	w = planningDo(t, s, w, "resume", PlanningRequest{Scope: "root", Reason: "Inventory immutable accepted sources"})
	w, r := planningClaim(t, s, w, "root")
	raw, _ := json.Marshal(map[string]any{"mode": "list", "candidate_commit": w.Planning.Repository.Candidate})
	r.Operations = []PlanningOperation{{Kind: "explore", ID: "list-root", Deliverable: string(raw)}}
	after, err := s.planningChange(w.ID, "decide", r)
	if err != nil {
		t.Fatal(err)
	}
	var found bool
	for _, event := range after.Planning.Inbox {
		if event.Kind == "exploration" && event.Exploration != nil {
			found = len(event.Exploration.Entries) == 1 && event.Exploration.Entries[0] == "value.txt" && len(event.Exploration.Sources) == 0 && event.Exploration.Digest == explorationDigest(*event.Exploration)
		}
	}
	if !found {
		t.Fatal("complete inventory or integrity missing")
	}
	offered := planningOperationSchemaKinds(t, planningSchemaForDelivery(PlanningDelivery{ExplorationAvailable: true}))
	legacy := planningOperationSchemaKinds(t, planningSchemaForDelivery(PlanningDelivery{}))
	if !containsString(offered, "explore") || containsString(legacy, "explore") {
		t.Fatal("read capability offered without a managed repository")
	}
}

func TestCursorEngineT3FutureDependencyAdaptationPreservesBlockedResult(t *testing.T) {
	s, w := managedFixture(t)
	w, err := s.mutate(w.ID, "test.prerequisite", "blocked-fixture", w.Revision, []byte(`{}`), func(current *Work) error {
		first, _ := current.task("first")
		first.Status = "blocked"
		first.Blocker = "Completed production awaits independent review"
		second, _ := current.task("second")
		second.Depends = []string{"first"}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	first, _ := w.task("first")
	original := *first
	w = planningDo(t, s, w, "resume", PlanningRequest{Scope: "root", Reason: "Explicit priority: independently develop the future task before repairing the blocked review"})
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{{Kind: "revise-dependencies", ID: "second", Depends: []string{}}}
	mux := http.NewServeMux()
	s.registerPlanning(mux, func(out http.ResponseWriter, v any) { json.NewEncoder(out).Encode(v) }, func(out http.ResponseWriter, e error) { http.Error(out, e.Error(), 400) })
	body, _ := json.Marshal(r)
	if !strings.Contains(string(body), `"depends":[]`) {
		t.Fatal("explicit removal disappeared from public request")
	}
	out := httptest.NewRecorder()
	mux.ServeHTTP(out, httptest.NewRequest("POST", "/api/v1/planning?work="+w.ID+"&action=decide", bytes.NewReader(body)))
	if out.Code != 200 {
		t.Fatal(out.Code, out.Body.String())
	}
	after, _ := s.get(w.ID)
	first, _ = after.task("first")
	second, _ := after.task("second")
	if !reflect.DeepEqual(*first, original) || second.Status != "todo" || len(second.Depends) != 0 || len(second.Attempts) != 0 || second.AutoValidation != nil || after.Planning.Activations != w.Planning.Activations {
		t.Fatal("adaptation fabricated acceptance, spent a call or changed the blocked producer")
	}
	var tracked bool
	for _, event := range after.Planning.Inbox {
		if event.Kind == "task_dependencies_changed" && event.Task == "second" && strings.Contains(event.Message, `"previous_depends":["first"]`) {
			tracked = true
		}
	}
	if !tracked {
		t.Fatal("old/new dependency attribution missing")
	}
	out = httptest.NewRecorder()
	mux.ServeHTTP(out, httptest.NewRequest("POST", "/api/v1/planning?work="+w.ID+"&action=decide", bytes.NewReader(body)))
	if out.Code != 200 {
		t.Fatal("replay", out.Code, out.Body.String())
	}
	again, _ := s.get(w.ID)
	if !reflect.DeepEqual(after, again) {
		t.Fatal("replay duplicated adaptation")
	}
}

func TestCursorEngineT3DependencyAdaptationRejectsUnsafeChanges(t *testing.T) {
	for _, name := range []string{"cycle", "unknown", "duplicate", "blocked", "produced", "unowned", "missing-array", "extra-field", "prepared"} {
		t.Run(name, func(t *testing.T) {
			s, w := managedFixture(t)
			if name == "blocked" || name == "produced" || name == "unowned" {
				var err error
				w, err = s.mutate(w.ID, "test.unsafe-adaptation", "fixture-state", w.Revision, []byte(`{}`), func(current *Work) error {
					task, _ := current.task("second")
					switch name {
					case "blocked":
						task.Status = "blocked"
						task.Blocker = "original refusal"
					case "produced":
						task.Attempts = []Attempt{{ID: "historical", Status: "completed"}}
					case "unowned":
						current.Planning.Scopes = append(current.Planning.Scopes, PlanningScope{ID: "other", Parent: "root", Objective: "Other scope", Revision: 1, State: "ready"})
					}
					return nil
				})
				if err != nil {
					t.Fatal(err)
				}
			}
			if name == "prepared" {
				if _, err := s.ensureManagedAttempt(w, Launch{EventID: "prepared-second", TaskID: "second"}); err != nil {
					t.Fatal(err)
				}
			}
			owner := "root"
			if name == "unowned" {
				owner = "other"
			}
			w = planningDo(t, s, w, "resume", PlanningRequest{Scope: owner, Reason: "Reassess future task dependencies"})
			w, r := planningClaim(t, s, w, owner)
			op := PlanningOperation{Kind: "revise-dependencies", ID: "second", Depends: []string{"first"}}
			switch name {
			case "cycle":
				op.Depends = []string{"second"}
			case "unknown":
				op.Depends = []string{"absent"}
			case "duplicate":
				op.Depends = []string{"first", "first"}
			case "missing-array":
				op.Depends = nil
			case "extra-field":
				op.Criteria = []string{"weakened"}
			}
			r.Operations = []PlanningOperation{op}
			if _, err := s.planningChange(w.ID, "decide", r); err == nil {
				t.Fatal("unsafe adaptation accepted")
			}
			after, _ := s.get(w.ID)
			if !reflect.DeepEqual(after.Tasks, w.Tasks) || after.Planning.Activations != w.Planning.Activations || after.Planning.Decisions != w.Planning.Decisions {
				t.Fatal("refusal changed task contract or budgets")
			}
		})
	}
}

func TestCursorEngineT3ReplacedExplorationCannotAuthorizeNewTask(t *testing.T) {
	s, w := managedFixture(t)
	w = planningDo(t, s, w, "resume", PlanningRequest{Scope: "root", Reason: "Read sources before a later accepted integration"})
	w, r := planningClaim(t, s, w, "root")
	raw, _ := json.Marshal(map[string]any{"mode": "read", "candidate_commit": w.Planning.Repository.Candidate, "files": []string{"value.txt"}})
	r.Operations = []PlanningOperation{{Kind: "explore", ID: "earlier-source", Deliverable: string(raw)}}
	w, err := s.planningChange(w.ID, "decide", r)
	if err != nil {
		t.Fatal(err)
	}
	oldEvent := ""
	for _, event := range w.Planning.Inbox {
		if event.Kind == "exploration" {
			oldEvent = event.ID
		}
	}
	producer := managedCompleted(t, s, w, "first", "accepted replacement\n")
	if err = s.integrateManagedAttempt(producer); err != nil {
		t.Fatal(err)
	}
	w, err = s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	w, decision := planningClaim(t, s, w, "root")
	if containsString(decision.Inputs, oldEvent) {
		t.Fatal("old source admitted as a current input")
	}
	context, err := planningContext(w, "root")
	if err != nil || !strings.Contains(string(context), "historical_events") {
		t.Fatal("historical source original lost", err)
	}
	decision.Inputs = append(decision.Inputs, oldEvent)
	decision.Operations = []PlanningOperation{planningTask("stale-discovery-task")}
	if _, err = s.planningChange(w.ID, "decide", decision); err == nil {
		t.Fatal("replaced source authorized an action")
	}
	after, _ := s.get(w.ID)
	if len(after.Tasks) != len(w.Tasks) || after.Planning.Decisions != w.Planning.Decisions {
		t.Fatal("stale discovery changed tasks")
	}
}
