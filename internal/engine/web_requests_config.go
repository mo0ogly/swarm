//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	resources "swarm.local/companion"
	"syscall"
)

var webRequestsDefaults = resources.WebRequests

// Values are milliseconds, zero disables browser cancellation. The only upper
// bound is JSON/JavaScript exact integer representation, not a waiting policy.
const webRequestMaxSafeInteger int64 = 1<<53 - 1

type WebRequestsValues struct {
	General        int64 `json:"general_ms"`
	Snapshot       int64 `json:"snapshot_ms"`
	ConnectionTest int64 `json:"connection_test_ms"`
}

func (v *WebRequestsValues) UnmarshalJSON(raw []byte) error {
	var wire struct {
		General        *int64 `json:"general_ms"`
		Snapshot       *int64 `json:"snapshot_ms"`
		ConnectionTest *int64 `json:"connection_test_ms"`
	}
	if err := strict(raw, &wire); err != nil {
		return err
	}
	if wire.General == nil || wire.Snapshot == nil || wire.ConnectionTest == nil {
		return fmt.Errorf("general_ms, snapshot_ms, connection_test_ms required")
	}
	v.General, v.Snapshot, v.ConnectionTest = *wire.General, *wire.Snapshot, *wire.ConnectionTest
	return nil
}

type WebRequestsChange struct {
	Schema   int               `json:"schema_version"`
	EventID  string            `json:"event_id"`
	Revision int               `json:"expected_revision"`
	Values   WebRequestsValues `json:"values"`
	Reason   string            `json:"reason"`
}
type WebRequestsRevision struct {
	Revision int               `json:"revision"`
	Values   WebRequestsValues `json:"values"`
	Actor    string            `json:"actor"`
	At       string            `json:"at"`
	Change   WebRequestsChange `json:"change"`
}
type WebRequestsConfig struct {
	Schema   int                   `json:"schema_version"`
	Revision int                   `json:"revision"`
	Values   WebRequestsValues     `json:"values"`
	History  []WebRequestsRevision `json:"history"`
}

func webRequestsView(c WebRequestsConfig) any {
	return struct {
		WebRequestsConfig
		Bounds map[string]int64 `json:"bounds"`
	}{c, map[string]int64{"min_ms": 0, "max_safe_integer_ms": webRequestMaxSafeInteger}}
}
func validateWebRequests(v WebRequestsValues) error {
	for _, n := range []int64{v.General, v.Snapshot, v.ConnectionTest} {
		if n < 0 || n > webRequestMaxSafeInteger {
			return fmt.Errorf("Browser timeouts require non-negative exact integers in milliseconds; 0 disables cancellation")
		}
	}
	return nil
}
func (s *Store) webRequestsConfig() (WebRequestsConfig, error) {
	var c WebRequestsConfig
	path := filepath.Join(s.root, ".swarm", "web-requests.json")
	st, err := os.Lstat(path)
	if os.IsNotExist(err) {
		c.Schema = 1
		c.History = []WebRequestsRevision{}
		err = strict(webRequestsDefaults, &c.Values)
	} else if err == nil {
		if !st.Mode().IsRegular() || st.Size() > 1048576 {
			return c, fmt.Errorf("web-requests.json: fichier local borné requis / bounded local file required")
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
		return c, fmt.Errorf("web-requests.json: version ou historique invalide / invalid version or history")
	}
	if err = validateWebRequests(c.Values); err != nil {
		return c, err
	}
	for i, h := range c.History {
		if h.Revision != i+1 || validateWebRequests(h.Values) != nil {
			return c, fmt.Errorf("web-requests.json: historique invalide / invalid history")
		}
	}
	if c.Revision > 0 {
		a, _ := json.Marshal(c.Values)
		b, _ := json.Marshal(c.History[len(c.History)-1].Values)
		if string(a) != string(b) {
			return c, fmt.Errorf("web-requests.json: valeurs et historique divergents / values differ from history")
		}
	}
	return c, nil
}
func (s *Store) changeWebRequests(r WebRequestsChange, preview bool) (WebRequestsConfig, error) {
	var empty WebRequestsConfig
	if r.Schema != 1 || !safeName(r.EventID) || r.Revision < 0 || len(strings.TrimSpace(r.Reason)) < 8 || len(r.Reason) > 2000 {
		return empty, fmt.Errorf("schema_version=1, event_id, expected_revision et reason (8..2000 caractères) requis / required")
	}
	if err := validateWebRequests(r.Values); err != nil {
		return empty, err
	}
	path := filepath.Join(s.root, ".swarm", "web-requests.lock")
	lock, err := os.OpenFile(path, os.O_CREATE|os.O_RDWR|syscall.O_NOFOLLOW, 0600)
	if err != nil {
		return empty, err
	}
	defer lock.Close()
	if err = syscall.Flock(int(lock.Fd()), syscall.LOCK_EX); err != nil {
		return empty, err
	}
	defer syscall.Flock(int(lock.Fd()), syscall.LOCK_UN)
	c, err := s.webRequestsConfig()
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
	c.History = append(c.History, WebRequestsRevision{Revision: c.Revision, Values: r.Values, Actor: operatorIdentity(), At: now(), Change: r})
	raw, err := json.MarshalIndent(c, "", "  ")
	if err != nil {
		return empty, err
	}
	// Preview validates the exact representation that application writes.
	if len(raw)+1 > 1048576 {
		return empty, fmt.Errorf("Historique plein / History full")
	}
	if !preview {
		if err = atomicWrite(filepath.Join(s.root, ".swarm", "web-requests.json"), append(raw, '\n')); err != nil {
			return empty, err
		}
	}
	return c, nil
}
func (s *Store) webRequestsCLI(pos []string, input string, out io.Writer) error {
	usage := "swarm run-limits web-requests show|history|preview|apply [--input configuration.json]"
	if len(pos) != 3 {
		return fmt.Errorf("%s", usage)
	}
	if pos[2] == "show" || pos[2] == "history" {
		c, err := s.webRequestsConfig()
		if err != nil {
			return err
		}
		if pos[2] == "history" {
			return printJSON(out, c.History)
		}
		return printJSON(out, webRequestsView(c))
	}
	if pos[2] != "apply" && pos[2] != "preview" {
		return fmt.Errorf("%s", usage)
	}
	raw, err := readInput(input)
	if err != nil {
		return err
	}
	var r WebRequestsChange
	if err = strict(raw, &r); err != nil {
		return err
	}
	c, err := s.changeWebRequests(r, pos[2] == "preview")
	if err != nil {
		return err
	}
	return printJSON(out, map[string]any{"applied": pos[2] == "apply", "config": c})
}
func (s *Store) registerWebRequests(mux *http.ServeMux, send func(http.ResponseWriter, any), fail func(http.ResponseWriter, error)) {
	mux.HandleFunc("/api/v1/web-requests/script.js", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" && r.Method != "HEAD" {
			http.Error(w, "GET required", 405)
			return
		}
		c, err := s.webRequestsConfig()
		if err != nil {
			fail(w, err)
			return
		}
		raw, err := json.Marshal(struct {
			Revision int               `json:"revision"`
			Values   WebRequestsValues `json:"values"`
		}{c.Revision, c.Values})
		if err != nil {
			fail(w, err)
			return
		}
		w.Header().Set("Content-Type", "application/javascript; charset=utf-8")
		if r.Method == "GET" {
			_, _ = fmt.Fprintf(w, "globalThis.SwarmWebRequestConfig = %s;\n", raw)
		}
	})
	mux.HandleFunc("/api/v1/web-requests", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" {
			http.Error(w, "GET required", 405)
			return
		}
		c, err := s.webRequestsConfig()
		if err != nil {
			fail(w, err)
			return
		}
		send(w, webRequestsView(c))
	})
	for _, action := range []string{"preview", "apply"} {
		action := action
		mux.HandleFunc("/api/v1/web-requests/"+action, func(w http.ResponseWriter, r *http.Request) {
			if r.Method != "POST" {
				http.Error(w, "POST required", 405)
				return
			}
			raw, err := io.ReadAll(http.MaxBytesReader(w, r.Body, 16384))
			if err != nil {
				fail(w, err)
				return
			}
			var change WebRequestsChange
			if err = strict(raw, &change); err != nil {
				fail(w, err)
				return
			}
			c, err := s.changeWebRequests(change, action == "preview")
			if err != nil {
				fail(w, err)
				return
			}
			send(w, map[string]any{"applied": action == "apply", "config": c})
		})
	}
}
