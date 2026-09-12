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

// Discovery is limited to project documentation; no host scan or symlink walk.
func (s *Store) taskReports(id string) []string {
	type candidate struct {
		path     string
		modified int64
	}
	found := []candidate{}
	visits := 0
	_ = filepath.WalkDir(filepath.Join(s.root, "docs"), func(path string, d fs.DirEntry, e error) error {
		visits++
		if visits > 5000 {
			return fs.SkipAll
		}
		if e != nil {
			return nil
		}
		if d.IsDir() {
			return nil
		}
		name := d.Name()
		if name != id+".md" && !(strings.HasPrefix(name, id+"-") && strings.Contains(name, "handoff") && strings.HasSuffix(name, ".md")) {
			return nil
		}
		rel, e := filepath.Rel(s.root, path)
		if e != nil {
			return nil
		}
		if _, e = safeReport(s.root, rel); e != nil {
			return nil
		}
		info, e := d.Info()
		if e == nil {
			found = append(found, candidate{rel, info.ModTime().UnixNano()})
		}
		return nil
	})
	sort.Slice(found, func(i, j int) bool { return found[i].modified > found[j].modified })
	out := []string{}
	for _, f := range found {
		out = append(out, f.path)
	}
	return out
}
func safeReport(root, path string) (string, error) {
	p, e := localFile(root, path)
	if e != nil {
		return "", e
	}
	info, e := os.Stat(p)
	if e != nil {
		return "", e
	}
	if !info.Mode().IsRegular() {
		return "", fmt.Errorf("Le rapport doit être un fichier ordinaire")
	}
	return p, nil
}
func (s *Store) reviewText(work string, d *taskDialog) string {
	w, e := s.get(work)
	if e != nil {
		return e.Error()
	}
	t, e := w.task(d.task.ID)
	if e != nil {
		return e.Error()
	}
	text := s.gateSummary(work, d.task.ID) + "\n\nTâche : " + t.Title + "\nÉtat : " + uiStatus(t.Status) + "\nLivrable : " + t.Deliverable + "\nCritères : " + strings.Join(t.Criteria, " ; ") + "\nProchaine action : " + t.Next
	if t.Override != nil {
		text += "\n\nDÉROGATION MANUELLE : " + t.Override.Reason + "\nOpérateur local : " + t.Override.Actor + " · " + t.Override.At + "\nLes contrôles ne sont pas transformés en PASS."
	}
	if t.Gate == nil {
		return text + "\n\nACCEPTATION INDISPONIBLE : aucune gate de validation enregistrée.\nLe conducteur doit examiner le rapport et enregistrer son évaluation."
	}
	text += "\n\nGate : " + t.Gate.Evaluation.Phase
	if current, err := evaluate(t.Gate.Document, s.root, "delivery"); err != nil {
		text += "\nContrôle des preuves : " + err.Error()
	} else {
		for _, reason := range current.Blockers {
			text += "\nBlocage : " + reason
		}
	}
	if s.validGate(t) {
		text += " · valide et preuves courantes"
	} else {
		text += " · échouée, périmée ou phase delivery absente"
	}
	b, e := jsonReview(t.Gate.Document)
	if e == nil {
		text += "\n" + b
	}
	for _, dep := range t.Depends {
		dt, _ := w.task(dep)
		if !s.acceptedFresh(&w, dt, map[string]bool{}) {
			text += "\nDépendance non validée ou périmée : " + dep
		}
	}
	return text
}
func (s *Store) acceptReviewedTask(work, id string) error {
	w, e := s.get(work)
	if e != nil {
		return e
	}
	t, e := w.task(id)
	if e != nil {
		return e
	}
	if t.Status == "accepted" {
		return fmt.Errorf("Tâche déjà acceptée. Voir les preuves ou utiliser une dérogation si elles sont périmées.")
	}
	if t.Status != "submitted" {
		return fmt.Errorf("Soumettre d’abord le rapport : état actuel %s", uiStatus(t.Status))
	}
	if !s.validGate(t) {
		return fmt.Errorf("Acceptation refusée : gate delivery absente, échouée ou preuves périmées. Ouvrir les gates et preuves.")
	}
	return s.operatorTask(work, Request{ID: id, Status: "accepted", Next: "Rapport accepté après revue et contrôle des preuves courantes"})
}

func jsonReview(raw json.RawMessage) (string, error) {
	var d struct {
		Results []struct {
			ID, Status string
			Evidence   []string
		}
	}
	if e := json.Unmarshal(raw, &d); e != nil {
		return "", e
	}
	lines := []string{}
	for _, r := range d.Results {
		lines = append(lines, r.ID+" : "+r.Status)
		for _, p := range r.Evidence {
			lines = append(lines, "Preuve : "+p)
		}
	}
	return strings.Join(lines, "\n"), nil
}

func (s *Store) submitReport(work, id, report string) error {
	return s.submitReportAt(work, id, report, -1)
}
func (s *Store) submitReportAt(work, id, report string, expected int) error {
	path, e := safeReport(s.root, report)
	if e != nil {
		return e
	}
	info, e := os.Stat(path)
	if e != nil || info.Size() == 0 {
		return fmt.Errorf("handoff vide")
	}
	w, e := s.get(work)
	if e != nil {
		return e
	}
	if expected >= 0 && w.Revision != expected {
		return &CommandError{Code: "revision_conflict", Message: "Le travail a changé ; relire avant soumission.", Retryable: true}
	}
	r := Request{Schema: 1, EventID: newID("operator-"), Revision: w.Revision, ID: id, Status: "submitted", Next: "Évaluer les preuves et la gate delivery ; handoff : " + report}
	raw, _ := json.Marshal(r)
	_, e = s.mutate(work, "task.submit", r.EventID, r.Revision, raw, func(w *Work) error {
		t, e := w.task(id)
		if e != nil {
			return e
		}
		if t.Status != "blocked" && t.Status != "todo" {
			return fmt.Errorf("soumettre depuis une tâche bloquée ou à faire, après contrôle du handoff")
		}
		if e = s.apply(w, "task.update", Request{ID: id, Status: "running", Next: "Handoff examiné : " + report}); e != nil {
			return e
		}
		return s.apply(w, "task.update", Request{ID: id, Status: "submitted", Outcome: "completed", Next: r.Next})
	})
	return e
}

func (s *Store) loadReport(d *taskDialog) {
	path, e := safeReport(s.root, d.reportPath)
	if e != nil {
		d.review = e.Error()
		return
	}
	f, e := os.Open(path)
	if e != nil {
		d.review = e.Error()
		return
	}
	defer f.Close()
	b, e := io.ReadAll(io.LimitReader(f, 262145))
	if e != nil {
		d.review = e.Error()
		return
	}
	tail := ""
	if len(b) > 262144 {
		b = b[:262144]
		tail = "\n[Affichage limité à 256 Kio : consulter le fichier pour la suite.]"
	}
	d.review = "Rapport : " + d.reportPath + "\n\n" + string(b) + tail
}

// An override is an operator decision, never a fabricated successful gate.
func (s *Store) overrideReviewedTask(work, id, reason string) error {
	return s.overrideReviewedTaskAt(work, id, reason, -1)
}
func (s *Store) overrideReviewedTaskAt(work, id, reason string, expected int) error {
	reason = strings.TrimSpace(reason)
	if len([]rune(reason)) < 10 || len(reason) > 2000 {
		return fmt.Errorf("Motif requis : 10 caractères minimum, 2000 octets maximum.")
	}
	w, e := s.get(work)
	if e != nil {
		return e
	}
	if expected >= 0 && w.Revision != expected {
		return &CommandError{Code: "revision_conflict", Message: "Le travail a changé ; relire avant dérogation.", Retryable: true}
	}
	var active int
	if e = s.db.QueryRow("SELECT count(*) FROM agents WHERE work_id=? AND task_id=? AND status IN ('queued','starting','running','stopping')", work, id).Scan(&active); e != nil {
		return e
	}
	task, e := w.task(id)
	if e != nil {
		return e
	}
	actor := fmt.Sprintf("uid:%d", os.Getuid())
	decision := ManualOverride{Reason: reason, Actor: actor, At: now(), Revision: w.Revision, PreviousStatus: task.Status}
	b, _ := json.Marshal(struct {
		TaskID   string         `json:"task_id"`
		Decision ManualOverride `json:"decision"`
	}{id, decision})
	_, e = s.mutate(work, "task.override", newID("operator-"), w.Revision, b, func(w *Work) error {
		t, e := w.task(id)
		if e != nil {
			return e
		}
		if active > 0 || t.Status == "running" {
			return fmt.Errorf("Agent ou tâche en cours : arrêter et réconcilier avant dérogation.")
		}
		if t.Status == "waived" {
			return fmt.Errorf("Dérogation déjà enregistrée ; rouvrir la tâche pour la modifier.")
		}
		if t.Status == "abandoned" {
			return fmt.Errorf("Rouvrir la tâche abandonnée avant dérogation.")
		}
		for _, dep := range t.Depends {
			dt, _ := w.task(dep)
			if !s.acceptedFresh(w, dt, map[string]bool{}) {
				return fmt.Errorf("Accepter d’abord la dépendance %s (revue ou dérogation).", dep)
			}
		}
		t.Override = &decision
		t.Status = "waived"
		t.Blocker = ""
		t.Next = "Acceptée par dérogation : " + reason
		return nil
	})
	return e
}
