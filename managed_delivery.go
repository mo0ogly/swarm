//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"path/filepath"
	"strings"
)

func (s *Store) managedRepository(w Work) (*ManagedRepository, error) {
	if w.Planning == nil || w.Planning.Repository == nil {
		return nil, fmt.Errorf("aucun dépôt géré pour cette mission")
	}
	r := w.Planning.Repository
	if filepath.IsAbs(r.Subdir) || strings.HasPrefix(filepath.Clean(r.Subdir), "..") {
		return nil, fmt.Errorf("périmètre de projet invalide")
	}
	expected := filepath.Join(s.root, ".swarm", "managed", w.ID)
	actual, e := filepath.EvalSymlinks(r.Storage)
	if e != nil || actual != expected || r.Storage != expected {
		return nil, fmt.Errorf("dépôt absent ou importé : préparer une nouvelle mission locale")
	}
	bare := filepath.Join(expected, "repository.git")
	resolved, e := filepath.EvalSymlinks(bare)
	if e != nil || resolved != bare {
		return nil, fmt.Errorf("dépôt Git absent ou redirigé")
	}
	raw, e := os.ReadFile(filepath.Join(expected, "repository.json"))
	if e != nil {
		return nil, e
	}
	var manifest ManagedRepository
	if e = json.Unmarshal(raw, &manifest); e != nil {
		return nil, e
	}
	if manifest.Base != r.Base || manifest.Subdir != r.Subdir || manifest.Source != r.Source || manifest.RequestDigest != r.RequestDigest {
		return nil, fmt.Errorf("manifeste local incompatible avec cette mission")
	}
	for _, rev := range []string{r.Base, r.Candidate} {
		if len(rev) != 40 && len(rev) != 64 {
			return nil, fmt.Errorf("révision Git invalide")
		}
		for _, c := range rev {
			if !(c >= '0' && c <= '9' || c >= 'a' && c <= 'f') {
				return nil, fmt.Errorf("révision Git invalide")
			}
		}
	}
	return r, nil
}
func (s *Store) managedBundle(work, dest string) error {
	w, err := s.get(work)
	if err != nil {
		return err
	}
	r, err := s.managedRepository(w)
	if err != nil {
		return err
	}
	unlock, err := managedLock(s.root, work)
	if err != nil {
		return err
	}
	defer unlock()
	// The database commit pointer is authoritative, including after a restart.
	bare := filepath.Join(r.Storage, "repository.git")
	if _, err = managedGit(bare, "update-ref", "refs/heads/swarm-result", r.Candidate); err != nil {
		return err
	}
	if _, err = os.Lstat(dest); err == nil {
		return fmt.Errorf("destination déjà présente")
	}
	_, err = managedGit(bare, "bundle", "create", dest, "refs/heads/swarm-result", "HEAD")
	return err
}
func (s *Store) registerPlanningBundle(mux *http.ServeMux, fail func(http.ResponseWriter, error)) {
	mux.HandleFunc("/api/v1/planning-bundle", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" {
			http.Error(w, "GET requis", 405)
			return
		}
		dir, err := os.MkdirTemp("", "swarm-delivery-")
		if err != nil {
			fail(w, err)
			return
		}
		defer os.RemoveAll(dir)
		file := filepath.Join(dir, "swarm-result.bundle")
		if err = s.managedBundle(r.URL.Query().Get("work"), file); err != nil {
			fail(w, err)
			return
		}
		w.Header().Set("Content-Type", "application/octet-stream")
		w.Header().Set("Content-Disposition", `attachment; filename="swarm-result.bundle"`)
		http.ServeFile(w, r, file)
	})
}
