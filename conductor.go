//go:build linux

package main

import (
	"fmt"
	"os"
	"strings"
	"time"
)

// Le conducteur relaie un handoff prouvé ; il ne juge jamais le livrable.
// Une tentative réussie dont le rapport est lisible et attribuable à cette
// tentative passe en « soumise » par le chemin humain existant
// (submitReportAt). Tout le reste garde son blocage et son motif : échec,
// interruption, rapport absent, vide, périmé ou ambigu.
// Soumettre n'est ni valider ni accepter : gate et acceptation restent humaines.

const conductorAuthor = "conducteur"

// Marge de dérive entre horloge murale et horodatage de fichier.
const reportClockSkew = 2 * time.Second

// conduct relaie le handoff après règlement d'une fin de tentative. Un relais
// impossible n'est pas une erreur de fin d'agent : il est journalisé et la
// tâche reste à la main de l'opérateur.
func (s *Store) conduct(a Agent, outcome string) {
	relayed, reason := s.relayHandoff(a, outcome)
	if reason == "" {
		return
	}
	_ = s.log(a.ID, "lifecycle", reason)
	a.Relay = reason
	_ = s.saveAgent(a)
	if relayed != "" {
		_ = s.controlEvent(a.WorkID, "conductor", a.TaskID+" : "+reason)
	}
}

// relayHandoff retourne le rapport relayé et le motif journalisable.
// Un motif vide signifie qu'il n'y avait rien à dire : la tentative ne
// relevait pas du relais (échec, dialogue, tâche déjà réglée autrement).
func (s *Store) relayHandoff(a Agent, outcome string) (string, string) {
	if outcome != "completed" {
		return "", ""
	}
	w, e := s.get(a.WorkID)
	if e != nil {
		return "", "Relais impossible : " + e.Error()
	}
	t, e := w.task(a.TaskID)
	if e != nil {
		return "", "Relais impossible : " + e.Error()
	}
	// Un dialogue produit une réponse, pas un livrable soumis à gate.
	if t.Brainstorm || t.Status != "blocked" {
		return "", ""
	}
	report, reason := s.provenReport(t.ID, a.Started)
	if report == "" {
		return "", "Relais refusé par le " + conductorAuthor + " : " + reason + ". La tâche reste bloquée pour examen."
	}
	if e := s.submitReportAt(a.WorkID, t.ID, report, w.Revision, conductorAuthor); e != nil {
		return "", "Relais refusé par le " + conductorAuthor + " : " + e.Error() + ". La tâche reste bloquée pour examen."
	}
	return report, "Relais automatique du handoff par le " + conductorAuthor + " : " + report +
		". Tâche soumise pour évaluation ; ni gate ni acceptation automatique."
}

// provenReport retient le rapport produit par cette tentative : lisible, non
// vide et modifié après son départ. Aucun contenu n'est interprété — la
// lecture du fond reste humaine. Plusieurs candidats restent ambigus : choisir
// à la place de l'opérateur reviendrait à soumettre une preuve non désignée.
func (s *Store) provenReport(task, started string) (string, string) {
	since, e := time.Parse(time.RFC3339Nano, started)
	if e != nil {
		return "", "date de départ de la tentative illisible"
	}
	// Tolérance de dérive : l'horloge murale et la date de modification d'un
	// fichier ne viennent pas de la même source et peuvent se croiser de
	// quelques instants. Une tentative réelle dure bien plus que cette marge,
	// donc elle n'affaiblit pas le refus d'un rapport d'une tentative passée.
	since = since.Add(-reportClockSkew)
	fresh := []string{}
	stale := 0
	for _, rel := range s.taskReports(task) {
		p, e := safeReport(s.root, rel)
		if e != nil {
			continue
		}
		info, e := os.Stat(p)
		if e != nil || info.Size() == 0 {
			continue
		}
		if info.ModTime().Before(since) {
			stale++
			continue
		}
		fresh = append(fresh, rel)
	}
	switch {
	case len(fresh) == 1:
		return fresh[0], ""
	case len(fresh) > 1:
		return "", "plusieurs rapports candidats pour cette tentative (" + strings.Join(fresh, ", ") + ")"
	case stale > 0:
		return "", fmt.Sprintf("aucun rapport produit par cette tentative ; %d rapport(s) antérieur(s) ignoré(s)", stale)
	}
	return "", "aucun rapport lisible et non vide sous docs/ pour cette tâche"
}
