//go:build linux

package main

import (
	"fmt"
	"os"
	"path/filepath"
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
	if w, e := s.get(a.WorkID); e == nil && w.Planning != nil && w.Planning.Repository != nil {
		if outcome == "completed" {
			if err := s.integrateManagedAttempt(a); err != nil {
				if commandFailure(err).Code == "storage_unavailable" {
					return
				}
				// Another bounded integration already owns the cross-process
				// lock. A polling pass must not emit a new failure every time.
				if strings.Contains(err.Error(), "déjà en cours") {
					return
				}
				_ = s.log(a.ID, "validation", err.Error())
				if allowed, _ := s.automaticValidationAuthorized(a.WorkID); allowed && commandFailure(err).Code != "revision_conflict" && commandFailure(err).Code != "provider_cooldown" && !strings.Contains(err.Error(), "déjà en cours") && !strings.Contains(err.Error(), "SQLITE_BUSY") {
					_ = s.managedFailure(a, "Intégration interrompue : "+err.Error())
				}
			}
		}
		return
	}

	relayed, reason := s.relayHandoff(a, outcome)
	for retry := 0; retry < 4 && (strings.Contains(reason, "SQLITE_BUSY") || strings.Contains(reason, "Le travail a changé") || strings.Contains(reason, "révision périmée")); retry++ {
		time.Sleep(time.Duration(retry+1) * 25 * time.Millisecond)
		relayed, reason = s.relayHandoff(a, outcome)
	}
	if reason == "" {
		return
	}
	_ = s.log(a.ID, "lifecycle", reason)
	a.Relay = reason
	_ = s.saveAgent(a)
	if relayed != "" {
		_ = s.controlEvent(a.WorkID, "conductor", a.TaskID+" : "+reason)
		accepted, validationReason := s.runAutomaticValidation(a, relayed)
		kind := "validation-retained"
		if accepted {
			kind = "validation-accepted"
		}
		_ = s.log(a.ID, "validation", validationReason)
		_ = s.controlEvent(a.WorkID, kind, a.TaskID+" · tentative "+a.Attempt+" : "+validationReason)
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
	if len(t.Attempts) == 0 || t.Attempts[len(t.Attempts)-1].ID != a.Attempt {
		return "", "Relais refusé : tentative remplacée"
	}
	report, reason := s.provenAttemptReport(a)
	if report == "" {
		return "", "Relais refusé par le " + conductorAuthor + " : " + reason + ". La tâche reste bloquée pour examen."
	}
	path, e := safeReport(s.root, report)
	if e != nil {
		return "", "Relais refusé : " + e.Error()
	}
	bytes, e := os.ReadFile(path)
	if e != nil {
		return "", "Relais refusé : " + e.Error()
	}
	if e := s.submitReportVerified(a.WorkID, t.ID, report, w.Revision, conductorAuthor, planningEventID("relay", a.ID, a.Attempt, hash(bytes)), hash(bytes)); e != nil {
		return "", "Relais refusé par le " + conductorAuthor + " : " + e.Error() + ". La tâche reste bloquée pour examen."
	}
	return report, "Relais automatique du handoff par le " + conductorAuthor + " : " + report +
		". Tâche soumise ; la politique enregistrée décide entre contrôles automatiques et revue humaine."
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

// Search only the attempt's workspace. Translate back to the canonical root for evidence.
func (s *Store) provenAttemptReport(a Agent) (string, string) {
	cwd := a.CWD
	if cwd == "" {
		cwd = s.root
	}
	resolved, e := resolveWorkspace(s.root, cwd)
	if e != nil {
		return "", "espace de tentative invalide : " + e.Error()
	}
	scoped := *s
	scoped.root = resolved
	report, reason := scoped.provenReport(a.TaskID, a.Started)
	if report == "" {
		return "", reason
	}
	relative, e := filepath.Rel(s.root, filepath.Join(resolved, report))
	if e != nil {
		return "", e.Error()
	}
	if _, e = safeReport(s.root, relative); e != nil {
		return "", e.Error()
	}
	return filepath.ToSlash(relative), ""
}
