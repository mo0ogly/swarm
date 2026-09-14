//go:build linux

package main

import (
	"strings"
	"testing"
)

func escalationKinds(list []escalation) map[string]escalation {
	out := map[string]escalation{}
	for _, e := range list {
		out[e.Kind] = e
	}
	return out
}

func escalationInput(tasks []Task, agents []Agent) escalationInputs {
	w := &Work{Schema: 1, ID: "w-test", Tasks: tasks}
	in := escalationInputs{work: w, agents: agents, gateValid: map[string]bool{}, validation: WorkValidation{Tasks: map[string]TaskValidation{}}}
	for _, t := range tasks {
		in.gateValid[t.ID] = t.Gate != nil
		in.validation.Tasks[t.ID] = TaskValidation{State: t.Status, Fresh: true, Owner: t.Owner}
	}
	return in
}

func TestEscalationOneEntryPerSubject(t *testing.T) {
	// Après relais : la tâche est soumise et la tentative est terminée.
	// Le sujet est unique — examiner ce rapport — donc une seule entrée.
	in := escalationInput(
		[]Task{{ID: "t1", Status: "submitted", Next: "Évaluer les preuves ; handoff : docs/t1.md"}},
		[]Agent{{ID: "a1", TaskID: "t1", Status: "completed", Activity: "Processus terminé", Relay: "docs/t1.md"}},
	)
	list := buildEscalations(in)
	if len(list) != 1 {
		t.Fatalf("un seul sujet attendu, %d entrées : %+v", len(list), list)
	}
	if list[0].Kind != "handoff" || list[0].TaskID != "t1" {
		t.Fatalf("entrée inattendue : %+v", list[0])
	}
}

func TestEscalationConductRefusalCarriesReason(t *testing.T) {
	in := escalationInput(
		[]Task{{ID: "t1", Status: "blocked", Blocker: "Processus terminé ; handoff et validation requis"}},
		[]Agent{{ID: "a1", TaskID: "t1", Status: "completed", Activity: "Processus terminé",
			Relay: "Relais refusé par le conducteur : aucun rapport lisible et non vide sous docs/ pour cette tâche."}},
	)
	kinds := escalationKinds(buildEscalations(in))
	got, ok := kinds["conduite"]
	if !ok {
		t.Fatalf("refus du conducteur non escaladé : %+v", kinds)
	}
	if !strings.Contains(got.Summary, "aucun rapport") {
		t.Fatalf("motif du refus absent : %q", got.Summary)
	}
}

func TestEscalationGuardStopDistinctFromFailure(t *testing.T) {
	cases := []struct {
		name, status, stop, want string
	}{
		{"garde-fou", "interrupted", "garde", "garde"},
		{"échec", "failed", "", "execution"},
		{"arrêt opérateur", "interrupted", "operateur", "execution"},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			in := escalationInput(
				[]Task{{ID: "t1", Status: "blocked", Blocker: "tentative interrompue"}},
				[]Agent{{ID: "a1", TaskID: "t1", Status: c.status, StopKind: c.stop, Activity: "Limite d'appels d'outils atteinte"}},
			)
			kinds := escalationKinds(buildEscalations(in))
			if _, ok := kinds[c.want]; !ok {
				t.Fatalf("catégorie %q attendue : %+v", c.want, kinds)
			}
		})
	}
}

func TestEscalationRepeatedFailuresAreCounted(t *testing.T) {
	in := escalationInput(
		[]Task{{ID: "t1", Status: "blocked", Blocker: "échec", Attempts: []Attempt{{ID: "at1"}, {ID: "at2"}}}},
		[]Agent{
			{ID: "a1", TaskID: "t1", Attempt: "at1", Status: "failed", Activity: "Processus en échec"},
			{ID: "a2", TaskID: "t1", Attempt: "at2", Status: "failed", Activity: "Processus en échec"},
		},
	)
	list := buildEscalations(in)
	if len(list) != 1 {
		t.Fatalf("une tâche, un sujet : %d entrées %+v", len(list), list)
	}
	if !strings.Contains(list[0].Summary, "2") {
		t.Fatalf("nombre de tentatives infructueuses absent : %q", list[0].Summary)
	}
}

func TestEscalationBudgetThreshold(t *testing.T) {
	in := escalationInput([]Task{{ID: "t1", Status: "todo"}}, nil)
	if kinds := escalationKinds(buildEscalations(in)); len(kinds) != 0 {
		t.Fatalf("aucun budget réglé : rien à escalader, %+v", kinds)
	}
	in.budget = BudgetView{Budget: Budget{Limit: 10}, Reserved: 9, Remaining: 1, Warning: true}
	kinds := escalationKinds(buildEscalations(in))
	got, ok := kinds["budget"]
	if !ok {
		t.Fatalf("seuil de budget non escaladé : %+v", kinds)
	}
	if !strings.Contains(got.Summary, "estim") {
		t.Fatalf("le budget doit se dire estimé, jamais facturé : %q", got.Summary)
	}
}

func TestEscalationStaysQuietOnSettledWork(t *testing.T) {
	in := escalationInput(
		[]Task{{ID: "t1", Status: "accepted", Gate: &GateRecord{}}},
		[]Agent{{ID: "a1", TaskID: "t1", Status: "completed", Activity: "Processus terminé"}},
	)
	if list := buildEscalations(in); len(list) != 0 {
		t.Fatalf("une tâche acceptée et fraîche n'interrompt personne : %+v", list)
	}
}

func TestEscalationRunningWorkIsQuiet(t *testing.T) {
	in := escalationInput(
		[]Task{{ID: "t1", Status: "running"}},
		[]Agent{{ID: "a1", TaskID: "t1", Status: "running", Heartbeat: now(), Activity: "12 appels · 11 résultats"}},
	)
	if list := buildEscalations(in); len(list) != 0 {
		t.Fatalf("une tentative vivante n'est pas une décision : %+v", list)
	}
}

func TestDecisionsFollowEscalationTaxonomy(t *testing.T) {
	s := storeTest(t)
	w, a := conductorAgent(t, s)
	writeReport(t, s, "t1.md", "handoff relayé")
	finishCompleted(t, s, a)

	list, e := s.decisions(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	open := []Decision{}
	for _, d := range list {
		if d.ResolvedAt == "" {
			open = append(open, d)
		}
	}
	if len(open) != 1 {
		t.Fatalf("après relais, une seule entrée à traiter attendue : %+v", open)
	}
	if open[0].Kind != "handoff" || open[0].TaskID != "t1" {
		t.Fatalf("entrée inattendue : %+v", open[0])
	}
}

func TestEscalationRaisesTaskCostOverrun(t *testing.T) {
	in := escalationInput([]Task{{ID: "t1", Status: "todo"}}, nil)
	in.reserve = 3.0
	in.taskCost = map[string]CostTotal{"t1": {Reported: 6.40, WithCost: 2}}
	kinds := escalationKinds(buildEscalations(in))
	got, ok := kinds["cout"]
	if !ok {
		t.Fatalf("dépassement de coût non escaladé : %+v", kinds)
	}
	if !strings.Contains(got.Summary, "rapportés") {
		t.Fatalf("le montant doit se dire rapporté, jamais facturé : %q", got.Summary)
	}
	if !strings.Contains(got.Summary, "subsiste") {
		t.Fatalf("l'entrée doit dire que la dépense engagée subsiste : %q", got.Summary)
	}

	// Sous le seuil, rien à décider.
	sous := escalationInput([]Task{{ID: "t1", Status: "todo"}}, nil)
	sous.reserve = 3.0
	sous.taskCost = map[string]CostTotal{"t1": {Reported: 4.00, WithCost: 2}}
	if _, ok := escalationKinds(buildEscalations(sous))["cout"]; ok {
		t.Fatal("sous le seuil, aucune demande ne doit être adressée à l'humain")
	}

	// Une tâche déjà acceptée n'a plus de départ à retenir.
	acceptee := escalationInput([]Task{{ID: "t1", Status: "accepted", Gate: &GateRecord{}}}, nil)
	acceptee.reserve = 3.0
	acceptee.taskCost = map[string]CostTotal{"t1": {Reported: 99.0, WithCost: 4}}
	if _, ok := escalationKinds(buildEscalations(acceptee))["cout"]; ok {
		t.Fatal("une tâche acceptée ne doit pas réclamer une décision de coût")
	}
}
