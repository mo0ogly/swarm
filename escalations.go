package main

import (
	"fmt"
	"time"
)

// Oracle unique de ce qui réclame un humain. Tout le reste du cockpit est
// consultable, pas bloquant : une information visible n'est pas une demande.
//
// Un sujet = une entrée. Si la tâche porte déjà la demande (rapport à examiner,
// gate à revalider), la tentative qui l'a produite n'en ajoute pas une seconde.
//
// Catégories :
//
//	handoff   — rapport soumis, en attente d'examen et d'acceptation
//	gate      — gate échouée, refusée ou preuve périmée
//	conduite  — tentative terminée dont le handoff n'a pas pu être relayé
//	execution — tentative en échec, interrompue, ou tentatives répétées
//	garde     — tentative arrêtée par un garde-fou d'exécution
//	silence   — tentative déclarée vivante mais sans signal
//	budget    — seuil de budget estimé atteint ; jamais un montant facturé
//	cout      — une tâche a dépassé son enveloppe de départ ; départs retenus
type escalation struct {
	TaskID  string
	AgentID string
	Kind    string
	Summary string
	Proof   string
	Version string
}

type escalationInputs struct {
	work       *Work
	agents     []Agent
	budget     BudgetView
	validation WorkValidation
	gateValid  map[string]bool
	taskCost   map[string]CostTotal
	reserve    float64
}

func buildEscalations(in escalationInputs) []escalation {
	out := []escalation{}
	claimed := map[string]bool{}
	for _, t := range in.work.Tasks {
		state := in.validation.Tasks[t.ID]
		if t.Status == "submitted" {
			claimed[t.ID] = true
			out = append(out, escalation{TaskID: t.ID, Kind: "handoff",
				Summary: "Rapport soumis : examiner les preuves puis accepter ou refuser",
				Proof:   t.Next, Version: fmt.Sprint(len(t.Attempts))})
		}
		if (t.Gate != nil && !in.gateValid[t.ID]) || state.State == "stale" {
			// Une acceptation dont les preuves ont dérivé après coup n'exige
			// aucun arbitrage tant que personne ne s'appuie dessus. L'état
			// reste visible sur la ligne de la tâche et dans le graphe ; il ne
			// prend une place dans la boîte à traiter que si une tâche non
			// terminée en dépend, car la faire avancer supposerait une preuve
			// qui n'est plus vérifiée.
			//
			// Sans cette distinction, travailler sur le produit périmait d'un
			// coup toutes les acceptations passées — les gates signent des
			// fichiers source partagés — et un travail terminé réclamait une
			// décision par tâche, indéfiniment.
			if settledTask(t.Status) && !blocksUnfinished(in.work, t.ID) {
				continue
			}
			claimed[t.ID] = true
			out = append(out, escalation{TaskID: t.ID, Kind: "gate",
				Summary: "Gate bloquée ou preuves périmées",
				Proof:   validationDetails(state), Version: gateDecisionVersion(t)})
		}
	}
	for _, e := range agentEscalations(in, claimed) {
		out = append(out, e)
	}
	if e, ok := budgetEscalation(in.budget); ok {
		out = append(out, e)
	}
	out = append(out, costEscalations(in)...)
	return out
}

func agentEscalations(in escalationInputs, claimed map[string]bool) []escalation {
	out := []escalation{}
	seen := map[string]bool{}
	failures := map[string]int{}
	for _, a := range in.agents {
		if a.Status == "failed" || a.Status == "interrupted" {
			failures[a.TaskID]++
		}
	}
	for _, a := range in.agents {
		t, e := in.work.task(a.TaskID)
		if e != nil || t.Status == "accepted" || t.Status == "waived" || claimed[a.TaskID] {
			continue
		}
		kind, summary, version := "", "", ""
		switch {
		case activeAgent(a) && lostSignal(a):
			kind, summary, version = "silence", "Aucun signal depuis le délai de surveillance ; processus non confirmé", a.Heartbeat
		case activeAgent(a):
			// Une tentative vivante n'est pas une décision.
			continue
		case a.Status == "completed":
			kind, summary = "conduite", relayReason(a)
		case a.Status == "interrupted" && a.StopKind == "garde":
			kind, summary = "garde", "Tentative arrêtée par un garde-fou : "+a.Activity
		default:
			kind = "execution"
			summary = fmt.Sprintf("%d tentative(s) infructueuse(s) sur cette tâche : %s", failures[a.TaskID], a.Activity)
		}
		// Un sujet par tâche : la tentative la plus récente porte la demande.
		if seen[a.TaskID+"|"+kind] {
			continue
		}
		seen[a.TaskID+"|"+kind] = true
		if version == "" {
			version = fmt.Sprint(failures[a.TaskID]) + "|" + a.ID
		}
		out = append(out, escalation{TaskID: a.TaskID, AgentID: a.ID, Kind: kind,
			Summary: summary, Proof: "Journaux de " + a.ID, Version: version})
	}
	return out
}

// Le motif du conducteur prime sur l'activité brute du processus : il dit ce
// qui manque pour relayer, pas seulement que le processus est terminé.
func relayReason(a Agent) string {
	if nonempty(a.Relay) {
		return a.Relay
	}
	return "Tentative terminée sans relais possible : " + a.Activity
}

func lostSignal(a Agent) bool {
	last, e := time.Parse(time.RFC3339Nano, a.Heartbeat)
	if e != nil {
		return false
	}
	return time.Since(last) > time.Duration(max(30, a.Limits.SilenceSeconds))*time.Second
}

// Le budget est une estimation réservée, jamais une dépense facturée ; une
// alerte ne rembourse pas ce qui est déjà engagé.
func budgetEscalation(v BudgetView) (escalation, bool) {
	if v.Budget.Limit <= 0 {
		return escalation{}, false
	}
	version := ""
	summary := ""
	switch {
	case v.Remaining <= 0:
		version, summary = "epuise", fmt.Sprintf("Budget estimé épuisé : %.2f USD réservés ou imputés sur %.2f USD ; suspendre ou relever la limite", v.Reserved+v.Estimated, v.Budget.Limit)
	case v.Warning:
		version, summary = "seuil", fmt.Sprintf("Seuil de budget estimé atteint : %.2f USD sur %.2f USD ; décider avant le prochain départ", v.Reserved+v.Estimated, v.Budget.Limit)
	default:
		return escalation{}, false
	}
	return escalation{Kind: "budget", Summary: summary, Proof: v.Policy, Version: version}, true
}

// Libellé humain d'une catégorie. Le terminal et la web affichent le même mot.
func escalationLabel(kind string) string {
	switch kind {
	case "handoff":
		return "Rapport à examiner"
	case "gate":
		return "Gate à revalider"
	case "conduite":
		return "Handoff non relayé"
	case "execution":
		return "Tentative infructueuse"
	case "garde":
		return "Garde-fou déclenché"
	case "silence":
		return "Signal perdu"
	case "budget":
		return "Budget estimé"
	case "cout":
		return "Coût dépassé"
	}
	return kind
}

// Sujet affiché : une entrée de travail n'a pas de tâche.
func escalationSubject(task string) string {
	if nonempty(task) {
		return task
	}
	return "Travail"
}

// Une tâche qui a dépassé son enveloppe cesse de partir seule ; l'opérateur doit
// pouvoir en décider, donc le savoir. Le montant est celui rapporté par les
// fournisseurs, jamais une facture, et le retenir ne rend rien de ce qui est
// déjà engagé.
func costEscalations(in escalationInputs) []escalation {
	out := []escalation{}
	if in.reserve <= 0 {
		return out
	}
	for _, t := range in.work.Tasks {
		if t.Status == "accepted" || t.Status == "waived" || t.Status == "abandoned" {
			continue
		}
		c := in.taskCost[t.ID]
		if c.Reported <= costThresholdFactor*in.reserve {
			continue
		}
		out = append(out, escalation{TaskID: t.ID, Kind: "cout",
			Summary: fmt.Sprintf("%.2f USD rapportés sur %d tentative(s) pour une réserve de %.2f par départ ; départs automatiques retenus, la dépense engagée subsiste",
				c.Reported, c.WithCost, in.reserve),
			Proof: "Coût rapporté par le fournisseur ; jamais une facture vérifiée",
			// Version discrète, comme les autres sujets du fichier : le nombre de
			// tentatives ayant rapporté un coût. Versionner par le montant créait
			// une carte de plus à chaque centime, sur un écran qui ne doit porter
			// que ce qui attend une décision.
			Version: fmt.Sprint(c.WithCost)})
	}
	return out
}

// settledTask : la tâche a reçu une décision humaine qui la clôt.
func settledTask(status string) bool {
	return status == "accepted" || status == "waived"
}

// blocksUnfinished : une tâche encore à faire dépend-elle de celle-ci ? Si oui,
// la dérive de ses preuves redevient une demande, parce qu'on ne peut pas
// avancer sur la suite en s'appuyant sur une preuve qui n'est plus vérifiée.
func blocksUnfinished(w *Work, id string) bool {
	for _, t := range w.Tasks {
		if settledTask(t.Status) || t.Status == "abandoned" {
			continue
		}
		for _, d := range t.Depends {
			if d == id {
				return true
			}
		}
	}
	return false
}
