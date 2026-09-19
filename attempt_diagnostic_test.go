//go:build linux

package main

import (
	"bytes"
	"strings"
	"testing"
	"time"
)

func TestAttemptDiagnosticSeparatesObservedErrorsFromConsecutiveLimit(t *testing.T) {
	failures := []toolFailure{
		{name: "Bash", action: "Exécution d’une commande", detail: "rg absent", technical: "command not found"},
		{name: "Read", action: "Lecture d’un fichier", detail: "/mnt/partage", technical: "permission denied by sandbox mount"},
		{name: "command_execution", action: "Exécution d’une commande", detail: "go test ./...", technical: "exit code 1"},
	}
	d := buildAttemptDiagnostic("agent-1", "attempt-1", failures, "Limite d'erreurs d'outils consécutives atteinte", 3)
	if d.ObservedErrors != 3 || d.ConsecutiveErrorLimit != 3 || !d.LimitReached || !d.ConsecutiveLimitReached {
		t.Fatalf("compteurs confondus : %+v", d)
	}
	for _, want := range []string{"3 erreurs d’outil observées", "plafond configuré est de 3 erreurs consécutives", "interrompu"} {
		if !strings.Contains(d.Summary, want) {
			t.Fatalf("résumé sans %q : %s", want, d.Summary)
		}
	}
	categories := map[string]bool{}
	for _, item := range d.Items {
		categories[item.Category] = true
		if item.Cause == "" || item.Consequence == "" || item.Action == "" || item.ActionKind == "" {
			t.Fatalf("diagnostic incomplet : %+v", item)
		}
	}
	for _, category := range []string{"configuration", "environment", "check", "limit"} {
		if !categories[category] {
			t.Fatalf("catégorie absente %s : %+v", category, d.Items)
		}
	}
}

func TestAttemptDiagnosticDoesNotAttributeAnotherGuardToErrorCeiling(t *testing.T) {
	d := buildAttemptDiagnostic("agent-1", "attempt-1", []toolFailure{{name: "MCP"}, {name: "MCP"}}, "Limite d'échecs identiques entrelacés atteinte", 3)
	if !d.LimitReached || d.ConsecutiveLimitReached || !strings.Contains(d.Summary, "ce plafond est distinct") || !strings.Contains(d.Summary, "Une autre limite d’exécution") {
		t.Fatalf("limite attribuée au mauvais plafond : %+v", d)
	}
}

func TestAttemptDiagnosticGroupsToolFailuresAndMarksUnknown(t *testing.T) {
	d := buildAttemptDiagnostic("agent-1", "attempt-1", []toolFailure{
		{name: "MCP", action: "Appel API", technical: "réponse invalide"},
		{name: "MCP", action: "Appel API", technical: "délai distant"},
	}, "", 3)
	if len(d.Items) != 1 || d.Items[0].Category != "tool" || d.Items[0].Count != 2 || len(d.Items[0].Traces) != 2 {
		t.Fatalf("échecs non regroupés : %+v", d)
	}
	unknown := fallbackAttemptDiagnostic(Agent{ID: "old", Attempt: "a-old", Status: "failed", Activity: "Processus en échec (code 2) ; consulter les journaux", Limits: RunLimits{MaxConsecutiveErrors: 3}})
	if len(unknown.Items) != 1 || !unknown.Items[0].Unknown || unknown.Items[0].Category != "unknown" {
		t.Fatalf("inconnu non signalé : %+v", unknown)
	}
}

func TestLoopGuardBuildsDiagnosticFromProviderEvents(t *testing.T) {
	limits, err := (RunLimits{MaxConsecutiveErrors: 3}).normalized()
	if err != nil {
		t.Fatal(err)
	}
	g := newLoopGuard(limits)
	for i, command := range []string{"go test ./...", "mount /mnt/partage", "outil --option"} {
		id := string(rune('a' + i))
		g.observe(map[string]any{"type": "item.started", "item": map[string]any{"type": "command_execution", "id": id, "command": command}}, time.Now())
		g.observe(map[string]any{"type": "item.completed", "item": map[string]any{"type": "command_execution", "id": id, "status": "failed", "exit_code": float64(1), "error": "operation not permitted"}}, time.Now())
	}
	d := g.diagnostic("agent-stream", "attempt-stream", "")
	if d.ObservedErrors != 3 || !d.LimitReached || !strings.Contains(d.Summary, "3 erreurs d’outil observées") {
		t.Fatalf("événements fournisseur non diagnostiqués : %+v", d)
	}
	for _, item := range d.Items {
		if strings.Contains(strings.ToLower(item.Action), "outil --option") {
			t.Fatalf("texte fournisseur transformé en action : %+v", item)
		}
	}
}

func TestMissionStatusAndCLIShareAttemptDiagnostic(t *testing.T) {
	s := storeTest(t)
	w, launch := setupAgent(t, s)
	a, _, err := s.prepare(w.ID, launch)
	if err != nil {
		t.Fatal(err)
	}
	a.Status = "failed"
	a.Activity = "Processus en échec (code 1) ; consulter les journaux"
	a.Diagnostic = buildAttemptDiagnostic(a.ID, a.Attempt, []toolFailure{{name: "command_execution", action: "Exécution d’une commande", detail: "go test ./...", technical: "exit code 1"}}, "", 3)
	if err = s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	status, err := s.missionStatus(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	if len(status.Tasks) != 1 || status.Tasks[0].Diagnostic == nil || status.Tasks[0].Diagnostic.AgentID != a.ID {
		t.Fatalf("diagnostic web/JSON absent : %+v", status.Tasks)
	}
	var out bytes.Buffer
	if err = missionCLI(s, []string{"mission", "status", w.ID}, "", false, &out); err != nil {
		t.Fatal(err)
	}
	for _, want := range []string{status.Tasks[0].Diagnostic.Summary, "Cause : ", "Conséquence : ", "Action disponible : ", "Traces techniques :"} {
		if !strings.Contains(out.String(), want) {
			t.Fatalf("sortie CLI sans %q :\n%s", want, out.String())
		}
	}
}

func TestAttemptDiagnosticDoesNotInferEnvironmentFromPath(t *testing.T) {
	for _, command := range []string{"go test ./sandbox/...", "pytest test_mount.py", "go test ./amount/..."} {
		if got := failureCategory(toolFailure{detail: command, technical: "assertion failed"}); got != "check" {
			t.Fatalf("%s classified as %s", command, got)
		}
	}
}

func TestAttemptDiagnosticBwrapMountFailureIsEnvironment(t *testing.T) {
	for _, command := range []string{"head report.md", "go test ./..."} {
		got := failureCategory(toolFailure{detail: command, technical: "bwrap: Can't bind mount /oldroot/ on /newroot/: Unable to apply mount flags: remount /newroot/cifs: No such device"})
		if got != "environment" {
			t.Fatalf("category %s", got)
		}
	}
}
