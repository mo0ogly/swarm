package main

import (
	"archive/zip"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

const archiveLimit = 128 << 20

type Bundle struct {
	Cockpit *CockpitHistory   `json:"cockpit_history,omitempty"`
	Schema  int               `json:"schema_version"`
	Work    Work              `json:"work"`
	Events  []Event           `json:"events"`
	Files   map[string]string `json:"files"`
	Missing []string          `json:"missing"`
}

func sortedArtifacts(m map[string]string) []string {
	a := []string{}
	for k := range m {
		a = append(a, k)
	}
	sort.Strings(a)
	return a
}

// State and event history are exported from the same SQLite read transaction.
func (s *Store) export(id, dest string) error {
	tx, e := s.db.Begin()
	if e != nil {
		return e
	}
	defer tx.Rollback()
	var raw []byte
	if e = tx.QueryRow("SELECT body FROM works WHERE id=?", id).Scan(&raw); e != nil {
		return e
	}
	var w Work
	if e = json.Unmarshal(raw, &w); e != nil {
		return e
	}
	rs, e := tx.Query("SELECT id,work_id,revision,kind,at,payload FROM events WHERE work_id=? ORDER BY revision", id)
	if e != nil {
		return e
	}
	events := []Event{}
	for rs.Next() {
		var v Event
		if e = rs.Scan(&v.ID, &v.WorkID, &v.Revision, &v.Kind, &v.At, &v.Payload); e != nil {
			rs.Close()
			return e
		}
		events = append(events, v)
	}
	e = rs.Err()
	rs.Close()
	if e != nil {
		return e
	}
	history, e := cockpitHistory(tx, id)
	if e != nil {
		return e
	}
	if e = tx.Commit(); e != nil {
		return e
	}
	bundle := Bundle{Cockpit: history, Schema: 1, Work: w, Events: events, Files: map[string]string{}, Missing: []string{}}
	data := map[string][]byte{}
	total := 0
	for _, t := range w.Tasks {
		if t.Gate == nil {
			continue
		}
		d, e := decodeAny(t.Gate.Document)
		if e != nil {
			return e
		}
		results, e := records(d["results"])
		if e != nil {
			return e
		}
		for _, r := range results {
			a, _ := r["evidence"].([]any)
			for _, v := range a {
				name := str(v)
				if prior, ok := data[name]; ok {
					if hash(prior) != t.Gate.Evaluation.Artifacts[name] {
						return fmt.Errorf("preuve partagée incompatible avec la gate de %s : %s", t.ID, name)
					}
					continue
				}
				p, e := localFile(s.root, name)
				if e != nil {
					bundle.Missing = append(bundle.Missing, name)
					continue
				}
				st, e := os.Stat(p)
				if e != nil {
					return e
				}
				if st.Size() > archiveLimit-int64(total) {
					return fmt.Errorf("export supérieur à 128 Mio")
				}
				b, e := os.ReadFile(p)
				if e != nil {
					return e
				}
				total += len(b)
				if hash(b) != t.Gate.Evaluation.Artifacts[name] {
					return fmt.Errorf("preuve modifiée pendant export : %s", name)
				}
				data[name] = b
				bundle.Files[name] = hash(b)
			}
		}
	}

	// Explicit memory references are portable archival pieces too.
	for _, name := range w.Memory {
		if _, ok := data[name]; ok {
			continue
		}
		path, err := localFile(s.root, name)
		if err != nil {
			bundle.Missing = append(bundle.Missing, name)
			continue
		}
		st, err := os.Stat(path)
		if err != nil {
			return err
		}
		if st.Size() > archiveLimit-int64(total) {
			return fmt.Errorf("export supérieur à 128 Mio")
		}
		b, err := os.ReadFile(path)
		if err != nil {
			return err
		}
		total += len(b)
		data[name] = b
		bundle.Files[name] = hash(b)
	}
	f, e := os.OpenFile(dest, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0600)
	if e != nil {
		return e
	}
	success := false
	defer func() {
		f.Close()
		if !success {
			os.Remove(dest)
		}
	}()
	z := zip.NewWriter(f)
	b, e := json.MarshalIndent(bundle, "", "  ")
	if e != nil {
		return e
	}
	if len(b)+total > archiveLimit {
		return fmt.Errorf("archive trop volumineuse")
	}
	entry, e := z.Create("manifest.json")
	if e != nil {
		return e
	}
	if _, e = entry.Write(b); e != nil {
		return e
	}
	written := map[string]bool{}
	for _, name := range sortedArtifacts(bundle.Files) {
		digest := bundle.Files[name]
		if written[digest] {
			continue
		}
		written[digest] = true
		entry, e = z.Create("blobs/" + digest)
		if e != nil {
			return e
		}
		if _, e = entry.Write(data[name]); e != nil {
			return e
		}
	}
	if e = z.Close(); e != nil {
		return e
	}
	if e = f.Sync(); e != nil {
		return e
	}
	success = true
	return nil
}
func (s *Store) importBundle(path string) (Work, error) {
	var zero Work
	z, e := zip.OpenReader(path)
	if e != nil {
		return zero, e
	}
	defer z.Close()
	files := map[string][]byte{}
	total := int64(0)
	for _, f := range z.File {
		if _, ok := files[f.Name]; ok {
			return zero, fmt.Errorf("entrée dupliquée")
		}
		blob := strings.TrimPrefix(f.Name, "blobs/")
		if f.Name != "manifest.json" && (!strings.HasPrefix(f.Name, "blobs/") || len(blob) != 64 || !safeName(blob)) {
			return zero, fmt.Errorf("entrée d’archive non autorisée")
		}
		if f.Mode()&os.ModeSymlink != 0 {
			return zero, fmt.Errorf("lien symbolique refusé")
		}
		r, e := f.Open()
		if e != nil {
			return zero, e
		}
		b, e := io.ReadAll(io.LimitReader(r, archiveLimit-total+1))
		r.Close()
		if e != nil {
			return zero, e
		}
		total += int64(len(b))
		if total > archiveLimit {
			return zero, fmt.Errorf("archive supérieure à 128 Mio")
		}
		files[f.Name] = b
	}
	var bundle Bundle
	if e = json.Unmarshal(files["manifest.json"], &bundle); e != nil {
		return zero, e
	}
	w := bundle.Work
	if bundle.Schema != 1 || w.Schema != 1 || !safeName(w.ID) || w.Revision < 1 || len(bundle.Events) != w.Revision {
		return zero, fmt.Errorf("manifeste/version/historique invalide")
	}
	seen := map[string]bool{}
	for i, v := range bundle.Events {
		if !safeName(v.ID) || seen[v.ID] || v.WorkID != w.ID || v.Revision != i+1 || !json.Valid(v.Payload) {
			return zero, fmt.Errorf("historique invalide")
		}
		seen[v.ID] = true
	}
	tasks := map[string]bool{}
	for _, t := range w.Tasks {
		if !safeName(t.ID) || tasks[t.ID] || status(t.Status) == "" {
			return zero, fmt.Errorf("tâche invalide")
		}
		tasks[t.ID] = true
		if t.Status == "running" && len(t.Attempts) == 0 {
			return zero, fmt.Errorf("tentative absente")
		}
	}
	if e = validateTaskGraph(w.Tasks); e != nil {
		return zero, e
	}
	for name, digest := range bundle.Files {
		if filepath.IsAbs(name) || name == "" || strings.HasPrefix(filepath.Clean(name), "..") {
			return zero, fmt.Errorf("chemin de preuve invalide")
		}
		b, ok := files["blobs/"+digest]
		if !ok || hash(b) != digest {
			return zero, fmt.Errorf("pièce manquante ou altérée")
		}
	}
	tx, e := s.db.Begin()
	if e != nil {
		return zero, e
	}
	defer tx.Rollback()
	var count int
	if e = tx.QueryRow("SELECT count(*) FROM works WHERE id=?", w.ID).Scan(&count); e != nil {
		return zero, e
	}
	if count != 0 {
		return zero, fmt.Errorf("travail déjà présent ; import sans écrasement")
	}
	// Archived pieces never overwrite or certify files in the destination project.
	dir := filepath.Join(s.root, ".swarm", "imports", w.ID)
	if e = os.MkdirAll(filepath.Dir(dir), 0700); e != nil {
		return zero, e
	}
	if e = os.Mkdir(dir, 0700); e != nil {
		return zero, e
	}
	success := false
	defer func() {
		if !success {
			os.RemoveAll(dir)
		}
	}()
	for _, digest := range bundle.Files {
		if e = atomicWrite(filepath.Join(dir, digest), files["blobs/"+digest]); e != nil {
			return zero, e
		}
	}
	if e = atomicWrite(filepath.Join(dir, "manifest.json"), files["manifest.json"]); e != nil {
		return zero, e
	}
	b, e := json.Marshal(w)
	if e != nil {
		return zero, e
	}
	if _, e = tx.Exec("INSERT INTO works VALUES(?,?,?)", w.ID, w.Revision, b); e != nil {
		return zero, e
	}
	for _, v := range bundle.Events {
		if _, e = tx.Exec("INSERT INTO events VALUES(?,?,?,?,?,?,?)", v.ID, w.ID, v.Revision, v.Kind, v.At, []byte(v.Payload), []byte(v.Payload)); e != nil {
			return zero, e
		}
	}
	if bundle.Cockpit != nil {
		for _, d := range bundle.Cockpit.Decisions {
			raw, err := json.Marshal(d)
			if err != nil {
				return zero, err
			}
			if _, e = tx.Exec("INSERT INTO decisions(id,work_id,body) VALUES(?,?,?)", d.ID, w.ID, raw); e != nil {
				return zero, e
			}
		}
		for _, v := range bundle.Cockpit.Visits {
			if _, e = tx.Exec("INSERT INTO session_visits(work_id,operator,revision,at) VALUES(?,?,?,?)", w.ID, v.Operator, v.Revision, v.At); e != nil {
				return zero, e
			}
		}
	}
	if e = tx.Commit(); e != nil {
		return zero, e
	}
	success = true
	return w, nil
}
