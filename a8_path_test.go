package main

import (
	"bytes"
	"strings"
	"testing"
	"time"
)

func TestA8LaunchContractIsSharedByCLIAndWebPayload(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	w.Scope = "recette isolée"
	w.Tasks[0].PlanMaxAttempts = 2
	w.Tasks[0].PlanToolLimit = 12
	w.Tasks[0].ValidationPolicy = &ValidationPolicy{Mode: "human"}
	profile := LaunchProfile{Provider: "fixture", Workspace: s.root, Role: "worker"}
	if err := s.setBudget(w.ID, Budget{Limit: 5, Reserve: 1, Source: "barème de recette", PriceDate: "2026-09-18"}); err != nil {
		t.Fatal(err)
	}
	preview, err := organizedFixtureStore(t, s).missionLaunchPreview(w.ID, profile, 1)
	if err != nil {
		t.Fatal(err)
	}
	for label, value := range map[string]string{
		"portée": preview.Contract.Scope, "budget": preview.Contract.Budget,
		"reprises": preview.Contract.Recovery, "validations": preview.Contract.Validation,
	} {
		if strings.TrimSpace(value) == "" {
			t.Fatalf("%s absent du contrat: %+v", label, preview.Contract)
		}
	}
	if !strings.Contains(preview.Contract.Budget, "5.00 USD") || !strings.Contains(preview.Contract.Recovery, "2 tentatives") || !strings.Contains(preview.Contract.Validation, "revue humaine") {
		t.Fatalf("contrat incomplet: %+v", preview.Contract)
	}
	if err = s.setProfile(w.ID, "", profile, w.Revision); err != nil {
		t.Fatal(err)
	}
	var plain bytes.Buffer
	if err = missionCLI(s, []string{"mission", "preview", w.ID}, "", false, &plain); err != nil {
		t.Fatal(err)
	}
	for _, expected := range []string{"Portée :", "Budget :", "Reprises :", "Validations :", preview.Contract.Budget} {
		if !strings.Contains(plain.String(), expected) {
			t.Fatalf("aperçu CLI sans %q:\n%s", expected, plain.String())
		}
	}
}

func TestA8CoordinationNamesWaitingResolverExchangeAndController(t *testing.T) {
	w := Work{Tasks: []Task{
		{ID: "producer", Title: "Produire"},
		{ID: "consumer", Title: "Assembler"},
		{ID: "check", Title: "Vérifier"},
	}}
	d := MissionStatus{Running: 1, ActiveAgents: 1, Review: 1, Tasks: []MissionTask{
		{ID: "producer", Title: "Produire", State: "running"},
		{ID: "consumer", Title: "Assembler", State: "waiting", Reason: "Attend Produire", Understanding: understanding("Attend Produire", "Reprendre après la remise.", "Le superviseur", "supervisor", "attente_normale")},
		{ID: "check", Title: "Vérifier", State: "review", ValidationMode: "automatic"},
	}}
	now := time.Now().UTC()
	exchanges := []AgentExchange{{Kind: "handoff", SourceTask: "producer", RecipientTask: "consumer", State: "pending", CreatedAt: now.Add(-time.Minute).Format(time.RFC3339Nano)}}
	phases := missionCoordinationPhases(&w, d, exchanges, now)
	if len(phases) != 4 {
		t.Fatalf("quatre rôles attendus: %+v", phases)
	}
	if !strings.Contains(phases[1].Summary, "Attend Produire") || phases[1].Actor != "Le superviseur" {
		t.Fatalf("attente ou responsable absent: %+v", phases[1])
	}
	if !strings.Contains(phases[2].Summary, "Produire remet un résultat à Assembler") || phases[2].At == "" || phases[2].Relative == "" {
		t.Fatalf("échange illisible: %+v", phases[2])
	}
	if phases[3].Actor != "Le superviseur" || !strings.Contains(phases[3].Summary, "contrôles automatiques") {
		t.Fatalf("vérification sans acteur: %+v", phases[3])
	}
}

func TestA8CompletedExchangeDoesNotRequestAction(t *testing.T) {
	w := Work{Tasks: []Task{{ID: "p", Title: "Produire"}, {ID: "c", Title: "Assembler"}}}
	for _, state := range []string{"consumed", "answered"} {
		phases := missionCoordinationPhases(&w, MissionStatus{}, []AgentExchange{{SourceTask: "p", RecipientTask: "c", Kind: "handoff", State: state}}, time.Now())
		if phases[2].Actor != "Personne pour cet échange" || strings.Contains(phases[2].Summary, state) {
			t.Fatal(phases[2])
		}
	}
	phases := missionCoordinationPhases(&w, MissionStatus{}, []AgentExchange{{SourceTask: "p", RecipientTask: "c", Kind: "help_request", State: "pending"}, {SourceTask: "p", RecipientTask: "c", Kind: "handoff", State: "consumed"}}, time.Now())
	if phases[2].Actor != "Assembler" || !strings.Contains(phases[2].Summary, "à traiter") {
		t.Fatal("échange en attente masqué", phases[2])
	}
}

func TestA9EnvironmentBlockExplainsRequiredNewCondition(t *testing.T) {
	task := MissionTask{ID: "p1", Title: "Produire", State: "intervention", Reason: "Processus en échec", Label: "Examiner avant reprise"}
	agent := Agent{TaskID: "p1", Status: "failed", Diagnostic: AttemptDiagnostic{Items: []DiagnosticItem{{Category: "environment"}}}}
	explanation := taskUnderstanding(task, MissionStatus{}, []Agent{agent})
	if !strings.Contains(explanation.What, "ne sera pas relancée automatiquement") || !strings.Contains(explanation.NextStep, "nouvelle vérification") {
		t.Fatal(explanation)
	}
}
