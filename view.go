package main

import (
	"encoding/json"
	"fmt"
	"path/filepath"
	"strings"
	"unicode"
)

func clean(s string) string {
	return strings.Map(func(r rune) rune {
		if unicode.IsControl(r) && r != '\n' && r != '\t' {
			return -1
		}
		return r
	}, s)
}
func wrap(s string, width int) string {
	var out []string
	for _, line := range strings.Split(clean(s), "\n") {
		r := []rune(strings.ReplaceAll(line, "\t", "    "))
		for len(r) > width {
			cut := width
			for cut > width/2 && r[cut] != ' ' {
				cut--
			}
			if cut <= width/2 {
				cut = width
			}
			out = append(out, string(r[:cut]))
			r = []rune(strings.TrimLeft(string(r[cut:]), " "))
		}
		out = append(out, string(r))
	}
	return strings.Join(out, "\n")
}
func value(s string) string {
	if !nonempty(s) {
		return "non renseigné"
	}
	return clean(s)
}
func status(s string) string {
	m := map[string]string{"todo": "À FAIRE", "running": "EN COURS (activité non confirmée)", "blocked": "BLOQUÉE", "submitted": "SOUMISE — À VALIDER", "accepted": "ACCEPTÉE", "waived": "ACCEPTÉE PAR DÉROGATION", "abandoned": "ABANDONNÉE"}
	return m[s]
}
func (s *Store) workStatus(w Work) (string, int, int) {
	accepted, total, abandoned := 0, len(w.Tasks), 0
	blocked := false
	memo := map[string]bool{}
	for i := range w.Tasks {
		t := &w.Tasks[i]
		if t.Status == "abandoned" {
			abandoned++
		}
		if s.acceptedFreshMemo(&w, t, map[string]bool{}, memo) {
			accepted++
		}
		if t.Status == "blocked" || (t.Status == "accepted" || t.Status == "waived") && !s.acceptedFreshMemo(&w, t, map[string]bool{}, memo) {
			blocked = true
		}
	}
	if total > 0 && accepted == total {
		for _, t := range w.Tasks {
			if t.Status == "waived" {
				return "ACCEPTÉ AVEC DÉROGATION", accepted, total
			}
		}
		return "VALIDÉ", accepted, total
	}
	if blocked {
		return "BLOQUÉ", accepted, total
	}
	if total > 0 && abandoned == total {
		return "ABANDONNÉ", accepted, total
	}
	if abandoned > 0 && accepted+abandoned == total {
		return "PARTIEL", accepted, total
	}
	return "OUVERT", accepted, total
}
func (s *Store) listView(ws []Work) string {
	var b strings.Builder
	b.WriteString("TRAVAUX — choisir explicitement un identifiant\n\n")
	if len(ws) == 0 {
		b.WriteString("Aucun travail enregistré. Créer un travail avec swarm work create.\n")
	}
	for _, w := range ws {
		state, a, n := s.workStatus(w)
		fmt.Fprintf(&b, "%s | %s | %s\n  Tâches acceptées (preuves ou dérogation) : %d/%d | révision %d\n  Dernière activité : %s\n", w.ID, state, w.Title, a, n, w.Revision, w.Updated)
		for _, t := range w.Tasks {
			if t.Blocker != "" {
				fmt.Fprintf(&b, "  Blocage %s : %s\n", t.ID, t.Blocker)
			}
		}
	}
	b.WriteString("\nReprise : swarm resume <identifiant>\nAucune action de tâche n’est lancée par la reprise.\n")
	return wrap(b.String(), 80)
}
func (s *Store) view(w Work) (string, error) {
	var b strings.Builder
	state, a, n := s.workStatus(w)
	fmt.Fprintf(&b, "# %s\n\nTravail : %s | révision %d | %s\nDernière activité : %s\n\n## Objectif et résultat attendu\n%s\nPérimètre : %s\n", w.Title, w.ID, w.Revision, state, w.Updated, w.Objective, w.Scope)
	for _, c := range w.Criteria {
		fmt.Fprintf(&b, "- %s\n", c)
	}
	fmt.Fprintf(&b, "\n## Situation actuelle\n%s\nHEAD enregistré : %s\nBranche enregistrée : %s\n", value(w.Summary), value(w.Git.Head), value(w.Git.Branch))
	current := gitState(s.root)
	if current != w.Git {
		b.WriteString("ATTENTION : contexte Git modifié depuis le checkpoint.\n")
	}
	fmt.Fprintf(&b, "HEAD courant : %s\nBranche courante : %s\n", value(current.Head), value(current.Branch))
	if current.Changes != "" {
		b.WriteString("Modifications locales présentes ; consulter git status.\n")
	}
	b.WriteString("\n## Tâches actives et résultats attendus\n")
	active := 0
	for _, t := range w.Tasks {
		if t.Status == "running" || t.Status == "submitted" {
			active++
			fmt.Fprintf(&b, "- %s : %s — %s\n  Responsable : %s\n  Livrable : %s\n  Prochaine action : %s\n", t.ID, t.Title, status(t.Status), value(t.Owner), t.Deliverable, value(t.Next))
		}
	}
	if active == 0 {
		b.WriteString("Aucune tâche active déclarée.\n")
	}
	fmt.Fprintf(&b, "\n## Travail validé\nTâches acceptées (preuves ou dérogation) : %d/%d.\nCe ratio est distinct de la progression des critères d’une gate.\n", a, n)
	for i := range w.Tasks {
		t := &w.Tasks[i]
		if s.acceptedFresh(&w, t, map[string]bool{}) {
			fmt.Fprintf(&b, "- %s : %s\n", t.ID, t.Title)
			if t.Override != nil {
				fmt.Fprintf(&b, "  DÉROGATION : %s — %s — %s\n", t.Override.Reason, t.Override.Actor, t.Override.At)
			}
			if t.Gate != nil {
				for _, p := range sortedArtifacts(t.Gate.Evaluation.Artifacts) {
					fmt.Fprintf(&b, "  Preuve/artefact : %s\n", p)
				}
			}
		}
	}
	b.WriteString("\n## Travail restant et blocages\n")
	for i := range w.Tasks {
		t := &w.Tasks[i]
		if s.acceptedFresh(&w, t, map[string]bool{}) {
			continue
		}
		fmt.Fprintf(&b, "- %s : %s — %s\n  Livrable : %s\n  Dépendances : %s\n  Blocage : %s\n", t.ID, t.Title, status(t.Status), t.Deliverable, value(strings.Join(t.Depends, ", ")), value(t.Blocker))
		if (t.Status == "accepted" || t.Status == "waived") && !s.acceptedFresh(&w, t, map[string]bool{}) {
			b.WriteString("  Acceptation à revalider : tâche ou dépendance périmée/rouverte.\n")
		}
		for _, c := range t.Criteria {
			fmt.Fprintf(&b, "  Critère : %s\n", c)
		}
		for _, at := range t.Attempts {
			fmt.Fprintf(&b, "  Tentative %s : %s\n", at.ID, at.Status)
		}
	}
	b.WriteString("\n## Gates, qualité et progression vérifiée\n")
	for _, t := range w.Tasks {
		if t.Gate == nil {
			fmt.Fprintf(&b, "- %s : gate non renseignée.\n", t.ID)
			continue
		}
		ev, e := evaluate(t.Gate.Document, s.root, t.Gate.Evaluation.Phase)
		if e != nil {
			fmt.Fprintf(&b, "- %s : PREUVES PÉRIMÉES OU INVALIDES — %s\n", t.ID, e)
			continue
		}
		quality := "indisponible"
		if ev.Quality != nil {
			quality = fmt.Sprintf("%.2f/100", *ev.Quality)
		}
		pct := "indisponible"
		if ev.Progress.Percent != nil {
			pct = fmt.Sprintf("%.2f%%", *ev.Progress.Percent)
		}
		fmt.Fprintf(&b, "- %s [%s] : qualité %s ; provisoire=%t\n  Critères PASS : %d/%d (%s) ; livraison permise par gate=%t\n  Blocages : %s\n", t.ID, ev.Phase, quality, ev.Provisional, ev.Progress.Passed, ev.Progress.Applicable, pct, ev.Ship, value(strings.Join(ev.Blockers, ", ")))
	}
	next := w.Next
	if next == "" {
		next = "à définir"
	}
	fmt.Fprintf(&b, "\n## Prochaine action du travail\n%s\n\n## Décisions OODA et mémoire utile\n", next)
	events, e := s.events(w.ID)
	if e != nil {
		return "", e
	}
	decisions := []Event{}
	for _, event := range events {
		if event.Kind == "ooda" {
			decisions = append(decisions, event)
		}
	}
	start := len(decisions) - 10
	if start < 0 {
		start = 0
	}
	for _, event := range decisions[start:] {
		var r Request
		_ = json.Unmarshal(event.Payload, &r)
		if event.Kind == "ooda" {
			fmt.Fprintf(&b, "- %s : observation=%s ; orientation=%s ; décision=%s\n  Responsable=%s ; action=%s ; résultat=%s\n", event.At, r.Observation, r.Orientation, r.Decision, r.Owner, r.Next, value(r.Result))
		}
	}
	if len(w.Memory) == 0 {
		b.WriteString("Mémoire spécifique : non renseignée.\n")
	}
	for _, p := range w.Memory {
		_, e := localFile(s.root, p)
		fmt.Fprintf(&b, "- %s", p)
		if e != nil {
			b.WriteString(" (indisponible)")
		}
		b.WriteString("\n")
	}
	b.WriteString("\nLes tentatives enregistrées ne prouvent pas qu’un processus tourne encore.\nUne gate ne remplace pas une autorisation de publication.\n")
	return wrap(b.String(), 80), nil
}
func (s *Store) render(w Work) (string, error) {
	v, e := s.view(w)
	if e != nil {
		return "", e
	}
	p := filepath.Join(s.root, ".swarm", "views", w.ID+".md")
	if e = atomicWrite(p, []byte(v)); e != nil {
		return "", e
	}
	return v, nil
}
