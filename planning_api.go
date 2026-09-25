package main

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
)

// The local authenticated API and CLI call the same transaction contract.
func (s *Store) planningCLI(pos []string, input string, out io.Writer) error {
	if len(pos) == 4 && pos[1] == "bundle" {
		return s.managedBundle(pos[2], pos[3])
	}
	if len(pos) != 3 {
		return fmt.Errorf("usage : planning show|enable|claim|decide|handoff|step|configure-reviewer|review-timeout|extend-attempt|authorize-recovery|requalify|recovery-preview|fragment-preview|submit-recovered-result|retry-review|retry-integration|review-step|pause|resume WORK [--input requête.json]")
	}
	if pos[1] == "review-step" {
		return s.independentReviewStep(pos[2])
	}
	if pos[1] == "step" {
		return s.planningStep(pos[2])
	}
	if pos[1] == "history" {
		v, e := s.planningHistory(pos[2])
		if e != nil {
			return e
		}
		return printJSON(out, v)
	}
	if pos[1] == "show" {
		w, err := s.get(pos[2])
		if err != nil {
			return err
		}
		return printJSON(out, w)
	}
	if input == "" {
		return fmt.Errorf("--input requis")
	}
	raw, err := readInput(input)
	if err != nil {
		return err
	}
	if pos[1] == "cleanup-preview" || pos[1] == "cleanup" {
		var request ManagedCleanupRequest
		if e := strict(raw, &request); e != nil {
			return e
		}
		v, e := s.managedCleanup(pos[2], request, pos[1] == "cleanup")
		if e != nil {
			return e
		}
		return printJSON(out, v)
	}
	var r PlanningRequest
	if err = strict(raw, &r); err != nil {
		return err
	}
	if pos[1] == "recovery-preview" {
		v, e := s.reviewRecoveryPreview(pos[2], r.Task)
		if e != nil {
			return e
		}
		return printJSON(out, v)
	}
	if pos[1] == "fragment-preview" {
		p, e := s.previewManagedFragments(pos[2], r.Task)
		if e != nil {
			return e
		}
		return printJSON(out, p)
	}
	w, err := s.planningChange(pos[2], pos[1], r)
	if err != nil {
		return err
	}
	return printJSON(out, w)
}
func (s *Store) registerPlanning(mux *http.ServeMux, send func(http.ResponseWriter, any), fail func(http.ResponseWriter, error)) {
	s.registerPlanningBundle(mux, fail)
	s.registerManagedCleanup(mux, send, fail)
	mux.HandleFunc("/api/v1/planning-history", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" {
			http.Error(w, "GET requis", 405)
			return
		}
		v, e := s.planningHistory(r.URL.Query().Get("work"))
		if e != nil {
			fail(w, e)
			return
		}
		send(w, v)
	})
	mux.HandleFunc("/api/v1/planning", func(w http.ResponseWriter, r *http.Request) {
		work := r.URL.Query().Get("work")
		if r.Method == "GET" && r.URL.Query().Get("action") == "recovery-preview" {
			v, e := s.reviewRecoveryPreview(work, r.URL.Query().Get("task"))
			if e != nil {
				fail(w, e)
				return
			}
			send(w, v)
			return
		}
		if r.Method == "GET" && r.URL.Query().Get("action") == "fragment-preview" {
			p, e := s.previewManagedFragments(work, r.URL.Query().Get("task"))
			if e != nil {
				fail(w, e)
				return
			}
			send(w, p)
			return
		}
		if r.Method == "GET" && r.URL.Query().Get("action") == "requalify-preview" {
			v, err := s.historicalRequalificationRequest(work, r.URL.Query().Get("task"))
			if err != nil {
				fail(w, err)
				return
			}
			send(w, v)
			return
		}
		if r.Method == "GET" {
			v, err := s.get(work)
			if err != nil {
				fail(w, err)
				return
			}
			send(w, v)
			return
		}
		if r.Method != "POST" {
			http.Error(w, "GET ou POST requis", 405)
			return
		}
		raw, err := io.ReadAll(http.MaxBytesReader(w, r.Body, 65536))
		if err != nil {
			fail(w, err)
			return
		}
		var request PlanningRequest
		if err = strict(raw, &request); err != nil {
			fail(w, err)
			return
		}
		result, err := s.planningChange(work, r.URL.Query().Get("action"), request)
		if err != nil {
			fail(w, err)
			return
		}
		send(w, result)
	})
}

type PlanningDecisionView struct {
	ID         string              `json:"id"`
	At         string              `json:"at"`
	Scope      string              `json:"scope"`
	Reason     string              `json:"reason"`
	Operations []PlanningOperation `json:"operations"`
}

func (s *Store) planningHistory(work string) ([]PlanningDecisionView, error) {
	events, e := s.events(work)
	if e != nil {
		return nil, e
	}
	out := []PlanningDecisionView{}
	for _, event := range events {
		if event.Kind != "planning.decide" {
			continue
		}
		var r PlanningRequest
		if e = json.Unmarshal(event.Payload, &r); e != nil {
			return nil, e
		}
		out = append(out, PlanningDecisionView{ID: event.ID, At: event.At, Scope: r.Scope, Reason: r.Reason, Operations: r.Operations})
	}
	return out, nil
}
