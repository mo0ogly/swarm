//go:build linux

package main

import (
	"crypto/sha256"
	"encoding/json"
	"fmt"
	"io"
	"io/fs"
	"net/http"
	"os"
	"path/filepath"
	"sort"
)

type ManagedCleanupRequest struct {
	Schema   int    `json:"schema_version"`
	Revision int    `json:"expected_revision"`
	Token    string `json:"preview_token,omitempty"`
}
type ManagedCleanupView struct {
	Revision int      `json:"revision"`
	Paths    []string `json:"paths"`
	Bytes    int64    `json:"bytes"`
	Token    string   `json:"preview_token"`
	Retained string   `json:"retained"`
}

func (s *Store) managedCleanupPreview(work string, r ManagedCleanupRequest) (ManagedCleanupView, error) {
	v := ManagedCleanupView{Paths: []string{}, Retained: "Dépôt de livraison, rapports, reçus, preuves et copies en échec conservés."}
	w, e := s.get(work)
	if e != nil {
		return v, e
	}
	if r.Schema != 1 || r.Revision != w.Revision {
		return v, fmt.Errorf("révision périmée ; actualiser l’aperçu")
	}
	repo, e := s.managedRepository(w)
	if e != nil {
		return v, e
	}
	v.Revision = w.Revision
	policy, e := s.missionPolicy(work)
	if e != nil {
		return v, e
	}
	if policy.Enabled && !s.paused(work) {
		return v, fmt.Errorf("mettre la mission en pause avant de nettoyer ses copies")
	}

	var active int
	if e = s.db.QueryRow("SELECT count(*) FROM agents WHERE work_id=? AND status IN ('queued','starting','running','stopping')", work).Scan(&active); e != nil {
		return v, e
	}
	if active > 0 {
		return v, fmt.Errorf("attendre la fin des agents avant nettoyage")
	}
	rows, e := s.db.Query("SELECT path FROM managed_attempts WHERE work_id=? AND state='integrated' ORDER BY path", work)
	if e != nil {
		return v, e
	}
	paths := []string{}
	for rows.Next() {
		var path string
		if e = rows.Scan(&path); e != nil {
			rows.Close()
			return v, e
		}
		paths = append(paths, path)
	}
	e = rows.Err()
	rows.Close()
	if e != nil {
		return v, e
	}
	fingerprints := []string{}
	for _, path := range paths {
		if filepath.Dir(path) != filepath.Join(repo.Storage, "copies") {
			return v, fmt.Errorf("copie hors périmètre")
		}
		if _, e = os.Lstat(path); os.IsNotExist(e) {
			continue
		}
		v.Paths = append(v.Paths, path)
		e = filepath.WalkDir(path, func(name string, d fs.DirEntry, walkErr error) error {
			if walkErr != nil {
				return walkErr
			}
			if d.Type()&os.ModeSymlink != 0 {
				return fmt.Errorf("copie contenant un lien ; conserver pour examen")
			}
			if d.IsDir() {
				return nil
			}
			st, e := d.Info()
			if e != nil {
				return e
			}
			if !st.Mode().IsRegular() {
				return fmt.Errorf("fichier spécial dans une copie")
			}
			f, e := os.Open(name)
			if e != nil {
				return e
			}
			digest := sha256.New()
			_, e = io.Copy(digest, f)
			f.Close()
			if e != nil {
				return e
			}
			v.Bytes += st.Size()
			fingerprints = append(fingerprints, fmt.Sprintf("%s:%x", name, digest.Sum(nil)))
			return nil
		})
		if e != nil {
			return v, e
		}
	}
	sort.Strings(fingerprints)
	raw, _ := json.Marshal([]any{w.ID, w.Revision, v.Paths, fingerprints})
	v.Token = hash(raw)
	return v, nil
}
func (s *Store) managedCleanup(work string, r ManagedCleanupRequest, apply bool) (ManagedCleanupView, error) {
	unlock, e := managedLock(s.root, work)
	if e != nil {
		return ManagedCleanupView{}, e
	}
	defer unlock()
	v, e := s.managedCleanupPreview(work, r)
	if e != nil || !apply {
		return v, e
	}
	if r.Token == "" || r.Token != v.Token {
		return v, fmt.Errorf("aperçu modifié ; examiner de nouveau les copies")
	}
	for _, path := range v.Paths {
		if e = os.RemoveAll(path); e != nil {
			return v, e
		}
	}
	return v, nil
}
func (s *Store) registerManagedCleanup(mux *http.ServeMux, send func(http.ResponseWriter, any), fail func(http.ResponseWriter, error)) {
	mux.HandleFunc("/api/v1/planning-cleanup", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" {
			http.Error(w, "POST requis", 405)
			return
		}
		var request ManagedCleanupRequest
		if e := json.NewDecoder(http.MaxBytesReader(w, r.Body, 4096)).Decode(&request); e != nil {
			fail(w, e)
			return
		}
		v, e := s.managedCleanup(r.URL.Query().Get("work"), request, r.URL.Query().Get("apply") == "true")
		if e != nil {
			fail(w, e)
			return
		}
		send(w, v)
	})
}
