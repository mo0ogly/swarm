package engine

import (
	"encoding/json"
	"fmt"
	"strings"
)

// Rebuild execution context from current engine records, never from a previous
// report or the mutable status text in Task.Next. Do not copy sibling tasks.
func workerExecutionContext(w Work, t *Task, agent string) string {
	var b strings.Builder
	attempt := ""
	if len(t.Attempts) > 0 {
		attempt = t.Attempts[len(t.Attempts)-1].ID
	}
	count := fmt.Sprint(len(t.Attempts))
	if t.PlanMaxAttempts > 0 {
		count = fmt.Sprintf("%d/%d", len(t.Attempts), t.PlanMaxAttempts)
	}
	fmt.Fprintf(&b, "\nIDENTITÉ COURANTE FOURNIE PAR LE MOTEUR : mission %s ; tâche %s ; agent %s ; tentative %s ; départ %s ; révision du travail %d.\n", w.ID, t.ID, agent, attempt, count, w.Revision)
	b.WriteString("Les identités, limites et conclusions des rapports antérieurs sont historiques. Elles ne remplacent pas cette identité ni le contrat courant ci-dessous. Le contrat courant prévaut sur les consignes historiques recopiées dans un profil. Un ancien rapport ne prouve pas la réussite de cette tentative.\n")
	for i := len(w.Plans) - 1; i >= 0; i-- {
		plan := w.Plans[i]
		prefix := "plan-" + hash([]byte(plan.Source))[:10] + "-"
		found := false
		for _, task := range plan.Spec.Tasks {
			if prefix+task.ID != t.ID {
				continue
			}
			raw, _ := json.Marshal(task)
			fmt.Fprintf(&b, "CONTRAT COURANT DE LA TÂCHE (brief %s, plan %s) :\n%s\n", plan.BriefHash, plan.ResponseHash, raw)
			if w.Planning == nil || t.ScopeID == "root" {
				for _, q := range plan.Spec.Questions {
					fmt.Fprintf(&b, "Décision du plan : %s → %s\n", q.Question, q.Answer)
				}
			}
			found = true
			break
		}
		if found {
			break
		}
	}
	if w.Planning != nil {
		for _, req := range t.Requirements {
			for i, criterion := range w.Criteria {
				if req == fmt.Sprintf("req-%d", i+1) {
					fmt.Fprintf(&b, "Exigence assignée %s : %s\n", req, criterion)
				}
			}
		}
		// Only root workers receive the shared root brief. Delegated workers retain
		// their scope objective, assigned requirements and their own task contract.
		if scope, err := w.Planning.scope(t.ScopeID); err == nil && scope.Parent == "" {
			brief := w.Scope
			if w.PlanningBrief != nil {
				brief = w.PlanningBrief.Text
			}
			fmt.Fprintf(&b, "BRIEF COMMUN ADOPTÉ (contexte, pas preuve de réussite) :\n%s\n", brief)
		}
	}
	if recovery := t.CorrectiveRecovery; recovery != nil {
		fmt.Fprintf(&b, "AUTORISATION CORRECTIVE DU MOTEUR (événement %s) : un essai supplémentaire explicitement autorisé après la tentative %s. Le plafond courant est %d tentatives ; il prévaut sur le plafond historique du plan. Les critères et les limites d’appels restent inchangés.\nCorrection autorisée : %s\n", recovery.Event, recovery.Attempt, t.PlanMaxAttempts, recovery.Instruction)
	}
	if len(t.Attempts) > 1 {
		b.WriteString("REPRISE : comparer le contrat courant au rapport existant, conserver les preuves encore pertinentes et traiter les lacunes. Produire un rapport actualisé attribuable à cette tentative, même si aucun fichier applicatif ne change. Si une lacune ou un blocage persiste, le signaler explicitement ; ne pas déclarer terminé sur la seule présence d’un rapport ancien.\n")
	}
	return b.String()
}
