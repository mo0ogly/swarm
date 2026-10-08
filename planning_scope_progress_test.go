//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

// This is a disposable Store and provider double: it proves scheduling, not
// the quality of an AI decision or live mission autonomy.
func TestPlanningExhaustedScopeDoesNotSuspendHealthySibling(t *testing.T) {
	for _, allExhausted := range []bool{false, true} {
		name := "healthy-sibling"
		if allExhausted {
			name = "all-exhausted"
		}
		t.Run(name, func(t *testing.T) {
			s := storeTest(t)
			w := createTest(t, s)
			script := filepath.Join(t.TempDir(), "claude")
			reply := `{"input_events":["healthy-event"],"reason":"Information examinée, aucune opération nécessaire","operations":[]}`
			envelope, _ := json.Marshal(map[string]any{"type": "result", "result": reply})
			if err := os.WriteFile(script, []byte("#!/bin/sh\ncat >\"$0.prompt\"\nprintf '%s\\n' '"+string(envelope)+"'\n"), 0700); err != nil {
				t.Fatal(err)
			}
			ps, _ := json.Marshal(Providers{Schema: 1, Providers: map[string]Provider{"test": {Command: script}}})
			if err := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), ps, 0600); err != nil {
				t.Fatal(err)
			}
			w = planningDo(t, s, w, "enable", PlanningRequest{Provider: "test", MaxTasks: 10, MaxDecisions: 10, MaxActivations: 15})
			var err error
			w, err = s.mutate(w.ID, "test.scope-budgets", newID("fixture-"), w.Revision, []byte(`{}`), func(w *Work) error {
				p := w.Planning
				p.Activations = 1
				p.Scopes[0].Activations = 1
				p.Scopes = append(p.Scopes,
					PlanningScope{ID: "exhausted", Parent: "root", Revision: 1, State: "ready", ActivationLimit: 1, Activations: 1},
					PlanningScope{ID: "healthy", Parent: "root", Revision: 1, State: "ready", ActivationLimit: 2})
				if allExhausted {
					p.Scopes[2].Activations = 2
					p.Scopes[0].Activations = 3
					p.Activations = 3
				}
				p.Inbox = []PlanningEvent{{ID: "blocked-event", Scope: "exhausted", Kind: "brief", Message: "Retour en attente", At: now()}, {ID: "healthy-event", Scope: "healthy", Kind: "brief", Message: "Retour à examiner", At: now()}}
				return nil
			})
			if err != nil {
				t.Fatal(err)
			}
			before := w
			if err = s.planningStep(w.ID); err != nil {
				t.Fatal(err)
			}
			got, err := s.get(w.ID)
			if err != nil {
				t.Fatal(err)
			}
			if got.Planning.Failure != "" {
				t.Fatalf("local ceiling suspended mission: %s", got.Planning.Failure)
			}
			exhausted, _ := got.Planning.scope("exhausted")
			if exhausted.Activations != 1 || exhausted.ActivationLimit != 1 || got.Planning.Inbox[0].Decision != "" {
				t.Fatal("exhausted scope or its pending evidence changed")
			}
			if allExhausted {
				if got.Revision != before.Revision {
					t.Fatal("idle scheduler mutated exhausted mission")
				}
				if _, err := os.Stat(script + ".prompt"); !os.IsNotExist(err) {
					t.Fatal("provider called with no capacity")
				}
			} else {
				if got.Planning.Activations != before.Planning.Activations+1 || got.Planning.Inbox[1].Decision == "" {
					t.Fatal("healthy sibling did not process its event")
				}
			}
			revision := got.Revision
			for i := 0; i < 3; i++ {
				if err = s.planningStep(w.ID); err != nil {
					t.Fatal(err)
				}
			}
			got, err = s.get(w.ID)
			if err != nil || got.Revision != revision {
				t.Fatal("repeated polls write or fail", err)
			}
		})
	}
}

func TestPlanningOlderChildEventPrecedesRootResume(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	script := filepath.Join(t.TempDir(), "claude")
	reply := `{"input_events":["child-result"],"reason":"Résultat examiné","operations":[]}`
	envelope, _ := json.Marshal(map[string]any{"type": "result", "result": reply})
	if err := os.WriteFile(script, []byte("#!/bin/sh\ncat >\"$0.prompt\"\nprintf '%s\\n' '"+string(envelope)+"'\n"), 0700); err != nil {
		t.Fatal(err)
	}
	ps, _ := json.Marshal(Providers{Schema: 1, Providers: map[string]Provider{"test": {Command: script}}})
	if err := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), ps, 0600); err != nil {
		t.Fatal(err)
	}
	w = planningDo(t, s, w, "enable", PlanningRequest{Provider: "test", MaxTasks: 10, MaxDecisions: 10, MaxActivations: 15})
	var err error
	w, err = s.mutate(w.ID, "test.pending-order", newID("fixture-"), w.Revision, []byte(`{}`), func(w *Work) error {
		w.Planning.Scopes = append(w.Planning.Scopes, PlanningScope{ID: "child", Parent: "root", Revision: 1, State: "ready"})
		w.Planning.Inbox = []PlanningEvent{
			{ID: "child-result", Scope: "child", Kind: "brief", Message: "Résultat antérieur", At: now()},
			{ID: "root-resume", Scope: "root", Kind: "operator_resume", Message: "Reprise récente", At: now()},
		}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	if err = s.planningStep(w.ID); err != nil {
		t.Fatal(err)
	}
	got, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	if got.Planning.Failure != "" {
		t.Fatalf("wrong scope selected: %s", got.Planning.Failure)
	}
	if got.Planning.Inbox[0].Decision == "" || got.Planning.Inbox[1].Decision != "" || got.Planning.Activations != 1 {
		t.Fatal("older child event must consume the single activation; root resume must remain pending")
	}
}
