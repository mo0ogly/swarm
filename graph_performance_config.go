//go:build linux

package main

import (
	_ "embed"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"syscall"
)

//go:embed config/graph-performance.json
var graphPerformanceDefaults []byte

type GraphPerformanceLoad struct {
	Cards     int `json:"cards"`
	RenderP95 int `json:"render_p95_ms"`
}
type GraphPerformanceValues struct {
	KeyboardP95 int                    `json:"keyboard_p95_ms"`
	Samples     int                    `json:"samples"`
	Loads       []GraphPerformanceLoad `json:"loads"`
}
type GraphPerformanceChange struct {
	Schema   int                    `json:"schema_version"`
	EventID  string                 `json:"event_id"`
	Revision int                    `json:"expected_revision"`
	Values   GraphPerformanceValues `json:"values"`
	Reason   string                 `json:"reason"`
}
type GraphPerformanceRevision struct {
	Revision int                    `json:"revision"`
	Values   GraphPerformanceValues `json:"values"`
	Actor    string                 `json:"actor"`
	At       string                 `json:"at"`
	Change   GraphPerformanceChange `json:"change"`
}
type GraphPerformanceConfig struct {
	Schema   int                        `json:"schema_version"`
	Revision int                        `json:"revision"`
	Values   GraphPerformanceValues     `json:"values"`
	History  []GraphPerformanceRevision `json:"history"`
}

type graphPerformanceBound struct {
	Min int `json:"min"`
	Max int `json:"max"`
}

var graphPerformanceBounds = map[string]graphPerformanceBound{
	"keyboard_p95_ms": {1, 60000}, "samples": {1, 100}, "cards": {2, 10000}, "render_p95_ms": {1, 60000}, "loads": {1, 10},
}

func graphPerformanceView(c GraphPerformanceConfig) any {
	return struct {
		GraphPerformanceConfig
		Bounds map[string]graphPerformanceBound `json:"bounds"`
	}{c, graphPerformanceBounds}
}
func withinGraphPerformance(name string, value int) bool {
	b := graphPerformanceBounds[name]
	return value >= b.Min && value <= b.Max
}
func validateGraphPerformance(v GraphPerformanceValues) error {
	// Operational safety bounds are separate from the editable acceptance targets.
	if !withinGraphPerformance("keyboard_p95_ms", v.KeyboardP95) || !withinGraphPerformance("samples", v.Samples) {
		return fmt.Errorf("keyboard_p95_ms: 1..60000 ms ; samples: 1..100")
	}
	if !withinGraphPerformance("loads", len(v.Loads)) {
		return fmt.Errorf("loads: 1..10 profils / profiles")
	}
	seen := map[int]bool{}
	for _, row := range v.Loads {
		if !withinGraphPerformance("cards", row.Cards) || !withinGraphPerformance("render_p95_ms", row.RenderP95) || seen[row.Cards] {
			return fmt.Errorf("loads: cards 2..10000 uniques / unique ; render_p95_ms 1..60000 ms")
		}
		seen[row.Cards] = true
	}
	return nil
}
func (s *Store) graphPerformanceConfig() (GraphPerformanceConfig, error) {
	var c GraphPerformanceConfig
	path := filepath.Join(s.root, ".swarm", "graph-performance.json")
	st, err := os.Lstat(path)
	if os.IsNotExist(err) {
		c.Schema = 1
		c.History = []GraphPerformanceRevision{}
		err = strict(graphPerformanceDefaults, &c.Values)
	} else if err == nil {
		if !st.Mode().IsRegular() || st.Size() > 1048576 {
			return c, fmt.Errorf("graph-performance.json: fichier local borné requis / bounded local file required")
		}
		var raw []byte
		raw, err = os.ReadFile(path)
		if err == nil {
			err = strict(raw, &c)
		}
	}
	if err != nil {
		return c, err
	}
	if c.Schema != 1 || c.Revision < 0 || c.Revision != len(c.History) {
		return c, fmt.Errorf("graph-performance.json: version ou historique invalide / invalid version or history")
	}
	if err = validateGraphPerformance(c.Values); err != nil {
		return c, err
	}
	for i, h := range c.History {
		if h.Revision != i+1 || validateGraphPerformance(h.Values) != nil {
			return c, fmt.Errorf("graph-performance.json: historique invalide / invalid history")
		}
	}
	if c.Revision > 0 {
		a, _ := json.Marshal(c.Values)
		b, _ := json.Marshal(c.History[len(c.History)-1].Values)
		if string(a) != string(b) {
			return c, fmt.Errorf("graph-performance.json: valeurs et historique divergents / values differ from history")
		}
	}
	return c, nil
}
func (s *Store) changeGraphPerformance(r GraphPerformanceChange, preview bool) (GraphPerformanceConfig, error) {
	var empty GraphPerformanceConfig
	if r.Schema != 1 || !safeName(r.EventID) || r.Revision < 0 || len(strings.TrimSpace(r.Reason)) < 8 || len(r.Reason) > 2000 {
		return empty, fmt.Errorf("schema_version=1, event_id, expected_revision et reason (8..2000 caractères) requis / required")
	}
	if err := validateGraphPerformance(r.Values); err != nil {
		return empty, err
	}
	path := filepath.Join(s.root, ".swarm", "graph-performance.lock")
	lock, err := os.OpenFile(path, os.O_CREATE|os.O_RDWR|syscall.O_NOFOLLOW, 0600)
	if err != nil {
		return empty, err
	}
	defer lock.Close()
	if err = syscall.Flock(int(lock.Fd()), syscall.LOCK_EX); err != nil {
		return empty, err
	}
	defer syscall.Flock(int(lock.Fd()), syscall.LOCK_UN)
	c, err := s.graphPerformanceConfig()
	if err != nil {
		return empty, err
	}
	for _, h := range c.History {
		if h.Change.EventID == r.EventID {
			a, _ := json.Marshal(h.Change)
			b, _ := json.Marshal(r)
			if string(a) != string(b) {
				return empty, &CommandError{Code: "event_conflict", Message: "event_id réutilisé avec d’autres valeurs / reused with different values"}
			}
			return c, nil
		}
	}
	if c.Revision != r.Revision {
		return empty, &CommandError{Code: "revision_conflict", Message: "Configuration modifiée ; rechargez avant de confirmer / Configuration changed; reload before confirming"}
	}
	c.Revision++
	c.Values = r.Values
	c.History = append(c.History, GraphPerformanceRevision{Revision: c.Revision, Values: r.Values, Actor: operatorIdentity(), At: now(), Change: r})
	if !preview {
		raw, err := json.MarshalIndent(c, "", "  ")
		if err != nil {
			return empty, err
		}
		if len(raw) > 1048576 {
			return empty, fmt.Errorf("Historique plein / History full")
		}
		if err = atomicWrite(filepath.Join(s.root, ".swarm", "graph-performance.json"), append(raw, '\n')); err != nil {
			return empty, err
		}
	}
	return c, nil
}
func (s *Store) graphPerformanceCLI(pos []string, input string, out io.Writer) error {
	usage := "swarm run-limits performance show|history|preview|apply [--input configuration.json]"
	if len(pos) != 3 {
		return fmt.Errorf("%s", usage)
	}
	if pos[2] == "show" || pos[2] == "history" {
		c, err := s.graphPerformanceConfig()
		if err != nil {
			return err
		}
		if pos[2] == "history" {
			return printJSON(out, c.History)
		}
		return printJSON(out, graphPerformanceView(c))
	}
	if pos[2] != "apply" && pos[2] != "preview" {
		return fmt.Errorf("%s", usage)
	}
	raw, err := readInput(input)
	if err != nil {
		return err
	}
	var r GraphPerformanceChange
	if err = strict(raw, &r); err != nil {
		return err
	}
	c, err := s.changeGraphPerformance(r, pos[2] == "preview")
	if err != nil {
		return err
	}
	return printJSON(out, map[string]any{"applied": pos[2] == "apply", "config": c})
}
func (s *Store) registerGraphPerformance(mux *http.ServeMux, send func(http.ResponseWriter, any), fail func(http.ResponseWriter, error)) {
	mux.HandleFunc("/api/v1/graph-performance", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" {
			http.Error(w, "GET required", 405)
			return
		}
		c, err := s.graphPerformanceConfig()
		if err != nil {
			fail(w, err)
			return
		}
		send(w, graphPerformanceView(c))
	})
	for _, action := range []string{"preview", "apply"} {
		action := action
		mux.HandleFunc("/api/v1/graph-performance/"+action, func(w http.ResponseWriter, r *http.Request) {
			if r.Method != "POST" {
				http.Error(w, "POST required", 405)
				return
			}
			raw, err := io.ReadAll(http.MaxBytesReader(w, r.Body, 16384))
			if err != nil {
				fail(w, err)
				return
			}
			var change GraphPerformanceChange
			if err = strict(raw, &change); err != nil {
				fail(w, err)
				return
			}
			c, err := s.changeGraphPerformance(change, action == "preview")
			if err != nil {
				fail(w, err)
				return
			}
			send(w, map[string]any{"applied": action == "apply", "config": c})
		})
	}
}
