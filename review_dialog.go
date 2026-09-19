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
	text += "\n\nGate : " + gateLabel(t) + " · phase " + t.Gate.Evaluation.Phase
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
	return s.submitReportAt(work, id, report, -1, "")
}

// origin nomme le moteur quand le conducteur relaie ; vide pour un geste humain.
func (s *Store) submitReportAt(work, id, report string, expected int, origin string) error {
	return s.submitReportVerified(work, id, report, expected, origin, newID("operator-"), "")
}

func (s *Store) submitReportVerified(work, id, report string, expected int, origin, event, digest string) error {
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
	r := Request{Schema: 1, EventID: event, Revision: w.Revision, ID: id, Status: "submitted", Origin: origin, Next: "Évaluer les preuves et la gate delivery ; handoff : " + report}
	raw, _ := json.Marshal(r)
	_, e = s.mutate(work, "task.submit", r.EventID, r.Revision, raw, func(w *Work) error {
		t, e := w.task(id)
		if e != nil {
			return e
		}
		if t.Status != "blocked" && t.Status != "todo" {
			return fmt.Errorf(
				"Soumission refusée : la tâche est « %s ». Un rapport se soumet depuis une tâche « À faire » ou « Bloquée » qui vient de produire son livrable. Pour un nouveau rapport de contrôle, rouvrir explicitement la tâche (action « Rouvrir la tâche » ou [o] dans la console) ; cela retire sa validation courante.",
				uiStatus(t.Status),
			)
		}
		var reportBytes []byte
		if digest != "" || t.Revalidation != nil || origin == conductorAuthor {
			bytes, err := os.ReadFile(path)
			if err != nil {
				return err
			}
			reportBytes = bytes
			if digest != "" && hash(bytes) != digest {
				return fmt.Errorf("Rapport modifié : examiner une nouvelle proposition avant soumission")
			}
		}
		if t.Revalidation != nil {
			digest := hash(reportBytes)
			for _, old := range t.Revalidation.PreviousArtifacts {
				if digest == old {
					return fmt.Errorf("Revalidation : nouveau rapport de contrôles requis ; ce contenu appartient aux anciennes preuves")
				}
			}
			t.Revalidation.Report = report
			t.Revalidation.ReportHash = digest
		}
		// A conductor relays the completed production attempt; it does not
		// create a synthetic attempt merely to submit its report.
		if origin == conductorAuthor {
			if t.Status != "blocked" || len(t.Attempts) == 0 || t.Attempts[len(t.Attempts)-1].Status != "completed" {
				return fmt.Errorf("relais sans tentative terminée courante")
			}
			if w.Planning != nil && w.Planning.Repository == nil {
				attempt := t.Attempts[len(t.Attempts)-1].ID
				exists := false
				for _, event := range w.Planning.Inbox {
					if event.Kind == "handoff" && event.Attempt == attempt {
						exists = true
					}
				}
				if !exists {
					scope, err := w.Planning.scope(t.ScopeID)
					if err != nil {
						return err
					}
					scope.State = "ready"
					w.Planning.Inbox = append(w.Planning.Inbox, PlanningEvent{ID: planningEventID("report", id, attempt), Scope: scope.ID, Kind: "handoff", Task: id, Attempt: attempt, Message: "Rapport remis automatiquement après fin normale. Examiner les preuves et limites ; résultat non validé.", Artifacts: []ExchangeArtifact{{Path: report, SHA256: hash(reportBytes)}}, At: now()})
				}
			}
			t.Status, t.Blocker, t.Next = "submitted", "", r.Next
			return nil
		}
		if e = s.apply(w, "task.update", Request{ID: id, Status: "running", Origin: origin, Next: "Handoff examiné : " + report}); e != nil {
			return e
		}
		return s.apply(w, "task.update", Request{ID: id, Status: "submitted", Origin: origin, Outcome: "completed", Next: r.Next})
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
