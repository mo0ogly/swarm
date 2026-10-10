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
	"time"
)

var preparationTimeoutDefaults = resources.PreparationTimeout

type PreparationTimeoutValues struct {
	Timeout int `json:"timeout_seconds"`
	Maximum int `json:"max_timeout_seconds"`
}
type PreparationTimeoutChange struct {
	Schema   int                      `json:"schema_version"`
	EventID  string                   `json:"event_id"`
	Revision int                      `json:"expected_revision"`
	Values   PreparationTimeoutValues `json:"values"`
	Reason   string                   `json:"reason"`
}
type PreparationTimeoutRevision struct {
	Revision int                      `json:"revision"`
	Values   PreparationTimeoutValues `json:"values"`
	Actor    string                   `json:"actor"`
	At       string                   `json:"at"`
	Change   PreparationTimeoutChange `json:"change"`
}
type PreparationTimeoutConfig struct {
	Schema   int                          `json:"schema_version"`
	Revision int                          `json:"revision"`
	Values   PreparationTimeoutValues     `json:"values"`
	History  []PreparationTimeoutRevision `json:"history"`
}

func preparationTimeoutView(c PreparationTimeoutConfig) any { return c }
func validatePreparationTimeout(v PreparationTimeoutValues) error {
	// The editable maximum is policy; the arithmetic bound only prevents duration overflow.
	if v.Timeout < 1 || v.Maximum < v.Timeout || int64(v.Maximum) > int64(^uint64(0)>>1)/int64(time.Second) {
		return fmt.Errorf("Délai positif requis, inférieur ou égal au plafond configuré / positive timeout must not exceed configured maximum")
	}
	return nil
}
func (s *Store) preparationTimeoutConfig() (PreparationTimeoutConfig, error) {
	var c PreparationTimeoutConfig
	path := filepath.Join(s.root, ".swarm", "preparation-timeout.json")
	st, err := os.Lstat(path)
	if os.IsNotExist(err) {
		c.Schema = 1
		c.History = []PreparationTimeoutRevision{}
		err = strict(preparationTimeoutDefaults, &c.Values)
	} else if err == nil {
		if !st.Mode().IsRegular() || st.Size() > 1048576 {
			return c, fmt.Errorf("preparation-timeout.json: fichier local borné requis / bounded local file required")
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
		return c, fmt.Errorf("preparation-timeout.json: version ou historique invalide / invalid version or history")
	}
	if err = validatePreparationTimeout(c.Values); err != nil {
		return c, err
	}
	for i, h := range c.History {
		if h.Revision != i+1 || validatePreparationTimeout(h.Values) != nil {
			return c, fmt.Errorf("preparation-timeout.json: historique invalide / invalid history")
		}
	}
	if c.Revision > 0 {
		a, _ := json.Marshal(c.Values)
		b, _ := json.Marshal(c.History[len(c.History)-1].Values)
		if string(a) != string(b) {
			return c, fmt.Errorf("preparation-timeout.json: valeurs et historique divergents / values differ from history")
		}
	}
	return c, nil
}
func (s *Store) changePreparationTimeout(r PreparationTimeoutChange, preview bool) (PreparationTimeoutConfig, error) {
	var empty PreparationTimeoutConfig
	if r.Schema != 1 || !safeName(r.EventID) || r.Revision < 0 || len(strings.TrimSpace(r.Reason)) < 8 || len(r.Reason) > 2000 {
		return empty, fmt.Errorf("schema_version=1, event_id, expected_revision et reason (8..2000 caractères) requis / required")
	}
	if err := validatePreparationTimeout(r.Values); err != nil {
		return empty, err
	}
	path := filepath.Join(s.root, ".swarm", "preparation-timeout.lock")
	lock, err := os.OpenFile(path, os.O_CREATE|os.O_RDWR|syscall.O_NOFOLLOW, 0600)
	if err != nil {
		return empty, err
	}
	defer lock.Close()
	if err = syscall.Flock(int(lock.Fd()), syscall.LOCK_EX); err != nil {
		return empty, err
	}
	defer syscall.Flock(int(lock.Fd()), syscall.LOCK_UN)
	c, err := s.preparationTimeoutConfig()
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
	c.History = append(c.History, PreparationTimeoutRevision{Revision: c.Revision, Values: r.Values, Actor: operatorIdentity(), At: now(), Change: r})
	if !preview {
		raw, err := json.MarshalIndent(c, "", "  ")
		if err != nil {
			return empty, err
		}
		if len(raw) > 1048576 {
			return empty, fmt.Errorf("Historique plein / History full")
		}
		if err = atomicWrite(filepath.Join(s.root, ".swarm", "preparation-timeout.json"), append(raw, '\n')); err != nil {
			return empty, err
		}
	}
	return c, nil
}
func (s *Store) preparationTimeoutCLI(pos []string, input string, out io.Writer) error {
	usage := "swarm run-limits preparation show|history|preview|apply [--input configuration.json]"
	if len(pos) != 3 {
		return fmt.Errorf("%s", usage)
	}
	if pos[2] == "show" || pos[2] == "history" {
		c, err := s.preparationTimeoutConfig()
		if err != nil {
			return err
		}
		if pos[2] == "history" {
			return printJSON(out, c.History)
		}
		return printJSON(out, preparationTimeoutView(c))
	}
	if pos[2] != "apply" && pos[2] != "preview" {
		return fmt.Errorf("%s", usage)
	}
	raw, err := readInput(input)
	if err != nil {
		return err
	}
	var r PreparationTimeoutChange
	if err = strict(raw, &r); err != nil {
		return err
	}
	c, err := s.changePreparationTimeout(r, pos[2] == "preview")
	if err != nil {
		return err
	}
	return printJSON(out, map[string]any{"applied": pos[2] == "apply", "config": c})
}
func (s *Store) registerPreparationTimeout(mux *http.ServeMux, send func(http.ResponseWriter, any), fail func(http.ResponseWriter, error)) {
	mux.HandleFunc("/api/v1/preparation-timeout", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" {
			http.Error(w, "GET required", 405)
			return
		}
		c, err := s.preparationTimeoutConfig()
		if err != nil {
			fail(w, err)
			return
		}
		send(w, preparationTimeoutView(c))
	})
	for _, action := range []string{"preview", "apply"} {
		action := action
		mux.HandleFunc("/api/v1/preparation-timeout/"+action, func(w http.ResponseWriter, r *http.Request) {
			if r.Method != "POST" {
				http.Error(w, "POST required", 405)
				return
			}
			raw, err := io.ReadAll(http.MaxBytesReader(w, r.Body, 16384))
			if err != nil {
				fail(w, err)
				return
			}
			var change PreparationTimeoutChange
			if err = strict(raw, &change); err != nil {
				fail(w, err)
				return
			}
			c, err := s.changePreparationTimeout(change, action == "preview")
			if err != nil {
				fail(w, err)
				return
			}
			send(w, map[string]any{"applied": action == "apply", "config": c})
		})
	}
}
