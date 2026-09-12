//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"io"
	"io/fs"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

func (s *Store) gateFiles(id string) []string {
	out := []string{}
	visits := 0
	_ = filepath.WalkDir(filepath.Join(s.root, "docs"), func(path string, d fs.DirEntry, err error) error {
		visits++
		if visits > 5000 {
			return fs.SkipAll
		}
		if err != nil || d.IsDir() {
			return nil
		}
		if !strings.HasPrefix(d.Name(), id+".") && !strings.HasPrefix(d.Name(), id+"-") {
			return nil
		}
		if !strings.HasSuffix(d.Name(), ".evidence.json") {
			return nil
		}
		rel, e := filepath.Rel(s.root, path)
		if e == nil {
			if _, e = safeReport(s.root, rel); e == nil {
				out = append(out, rel)
			}
		}
		return nil
	})
	sort.Strings(out)
	return out
}
func (s *Store) previewGate(work string, d *taskDialog) error {
	path, e := safeReport(s.root, d.reportPath)
	if e != nil {
		return e
	}
	f, e := os.Open(path)
	if e != nil {
		return e
	}
	defer f.Close()
	raw, e := io.ReadAll(io.LimitReader(f, 1048577))
	if e != nil {
		return e
	}
	if len(raw) > 1048576 {
		return fmt.Errorf("Document de gate supérieur à 1 Mio")
	}
	w, e := s.get(work)
	if e != nil {
		return e
	}
	if e = s.applyGateDocument(&w, d.task.ID, "delivery", raw); e != nil {
		return e
	}
	t, _ := w.task(d.task.ID)
	ev := t.Gate.Evaluation
	verdict := "REFUSÉ"
	if ev.Ship {
		verdict = "PASS"
	}
	d.gateRaw = raw
	d.gateRevision = w.Revision
	d.review = fmt.Sprintf("Verdict : %s · critères réussis %d/%d\nFichier : %s\nLes preuves seront revérifiées à la confirmation.", verdict, ev.Progress.Passed, ev.Progress.Applicable, d.reportPath)
	for _, b := range ev.Blockers {
		d.review += "\nBlocage : " + b
	}
	return nil
}
func (s *Store) recordDialogGate(work string, d *taskDialog) error {
	request := struct {
		Schema   int             `json:"schema_version"`
		Event    string          `json:"event_id"`
		Revision int             `json:"expected_revision"`
		TaskID   string          `json:"task_id"`
		Phase    string          `json:"phase"`
		Document json.RawMessage `json:"document"`
	}{1, newID("operator-"), d.gateRevision, d.task.ID, "delivery", d.gateRaw}
	raw, _ := json.Marshal(request)
	_, e := s.mutate(work, "gate", request.Event, request.Revision, raw, func(w *Work) error {
		return s.applyGateDocument(w, request.TaskID, request.Phase, request.Document)
	})
	return e
}
