//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"strings"
)

func (s *Store) hierarchyText(work string) string {
	w, e := s.get(work)
	if e != nil {
		return e.Error()
	}
	aa, e := s.agents(work)
	if e != nil {
		return e.Error()
	}
	lines := []string{uiText("OBJECTIF : ") + w.Title, w.Objective, ""}
	for _, t := range w.Tasks {
		lines = append(lines, "├─ "+t.ID+" · "+t.Title+" · "+uiStatus(t.Status), uiText("│  Responsable : ")+t.Owner, uiText("│  Dépendances : ")+value(strings.Join(t.Depends, ", ")))
		for _, a := range aa {
			if a.TaskID == t.ID {
				lines = append(lines, "│  └─ "+a.ID, uiText("│     Fournisseur : ")+a.Provider+uiText(" · rôle : ")+a.Role+uiText(" · processus : ")+uiStatus(observedAgent(a)), uiText("│     Tentative : ")+a.Attempt+" · parent : "+value(a.Parent))
			}
		}
	}
	lines = append(lines, "", uiText("Un état de tâche ne prouve pas qu’un processus est vivant."))
	return strings.Join(lines, "\n")
}
func (s *Store) resumeSinceText(work string, v Visit) string {
	w, e := s.get(work)
	if e != nil {
		return e.Error()
	}
	lines := []string{uiText("REPRISE · ") + v.Operator, uiText("Visite précédente : ") + value(v.At), fmt.Sprintf(uiText("Révision vue : %d · actuelle : %d"), v.Revision, w.Revision), uiText("Résumé historique (ne vaut pas validation actuelle) : ") + value(w.Summary), uiText("Prochaine action : ") + value(w.Next), ""}
	rows, e := s.db.Query("SELECT revision,kind,at,payload FROM events WHERE work_id=? AND revision>? ORDER BY revision LIMIT 200", work, v.Revision)
	if e != nil {
		return e.Error()
	}
	for rows.Next() {
		var rev int
		var kind, at string
		var raw []byte
		if e = rows.Scan(&rev, &kind, &at, &raw); e != nil {
			rows.Close()
			return e.Error()
		}
		lines = append(lines, fmt.Sprintf("r%d · %s · %s", rev, kind, at))
		if kind == "ooda" || kind == "checkpoint" {
			var r Request
			if json.Unmarshal(raw, &r) == nil {
				lines = append(lines, "  "+r.Summary+r.Observation, uiText("  Décision : ")+r.Decision, uiText("  Suite : ")+r.Next)
			}
		}
	}
	rows.Close()
	validation := s.validationState(&w)
	lines = append(lines, uiText("VALIDATION ACTUELLE : ")+validation.State)
	for _, t := range w.Tasks {
		if x := validation.Tasks[t.ID]; x.State == "stale" {
			lines = append(lines, t.ID+uiText(" : acceptation historique, revalidation obligatoire"), validationDetails(x))
		}
	}
	memo := map[string]bool{}
	lines = append(lines, "", uiText("BLOCAGES ET DÉCISIONS EN ATTENTE"))
	for i := range w.Tasks {
		t := &w.Tasks[i]
		if t.Status == "blocked" || t.Status == "submitted" {
			lines = append(lines, t.ID+" · "+uiStatus(t.Status)+" · "+value(t.Next))
		}
		for _, id := range t.Depends {
			d, _ := w.task(id)
			if !s.acceptedFreshMemo(&w, d, map[string]bool{}, memo) {
				lines = append(lines, t.ID+uiText(" : dépendance ")+id+uiText(" non acceptée ou preuves périmées"))
			}
		}
	}
	lines = append(lines, "", uiText("PROCHAINES TÂCHES ACCESSIBLES"))
	for i := range w.Tasks {
		t := &w.Tasks[i]
		if t.Status != "todo" && t.Status != "blocked" {
			continue
		}
		ready := true
		for _, id := range t.Depends {
			d, _ := w.task(id)
			if !s.acceptedFreshMemo(&w, d, map[string]bool{}, memo) {
				ready = false
			}
		}
		if ready {
			lines = append(lines, t.ID+" · "+t.Title+" · "+t.Next)
		}
	}
	lines = append(lines, "", uiText("200 événements au maximum ; l’historique complet reste dans la base."))
	return strings.Join(lines, "\n")
}
func (s *Store) gateSummary(work, id string) string {
	w, e := s.get(work)
	if e != nil {
		return e.Error()
	}
	t, e := w.task(id)
	if e != nil {
		return e.Error()
	}
	if t.Gate == nil {
		return uiText("Avancement vérifié : indisponible\nQualité : indisponible\nVerdict : aucune gate\nUne fin de processus n’est pas une validation.")
	}
	ev, e := evaluate(t.Gate.Document, s.root, "delivery")
	if e != nil {
		return uiText("Fraîcheur : preuves invalides\nVerdict : REFUSÉ\n") + e.Error()
	}
	quality := "indisponible"
	if ev.Quality != nil {
		quality = fmt.Sprintf("%.1f/100", *ev.Quality)
	}
	if ev.Provisional {
		quality += uiText(" (provisoire)")
	}
	verdict := uiText("REFUSÉ")
	if ev.Ship {
		verdict = "PASS"
	}
	for _, id := range t.Depends {
		dependency, err := w.task(id)
		if err != nil || !s.acceptedFresh(&w, dependency, map[string]bool{}) {
			verdict = uiText("REFUSÉ — dépendance non acceptée ou preuves périmées : ") + id
			break
		}
	}
	return fmt.Sprintf(uiText("Avancement vérifié : %d/%d contrôles\nQualité : %s\nFraîcheur : contrôlée sur les fichiers actuels\nVerdict delivery : %s"), ev.Progress.Passed, ev.Progress.Applicable, quality, verdict)
}
