package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestReportContextReadsSelectedReportAndTracksChanges(t *testing.T) {
	s := storeTest(t)
	w := assistWork(t, s)
	dir := filepath.Join(s.root, "docs")
	if err := os.MkdirAll(dir, 0755); err != nil {
		t.Fatal(err)
	}
	path := filepath.Join(dir, "SC-15.md")
	write := func(body string) {
		t.Helper()
		if err := os.WriteFile(path, []byte(body), 0600); err != nil {
			t.Fatal(err)
		}
	}
	write("Résultat annoncé : graphe réparé. Vérification encore attendue.")
	c := PageCoordinates{PageID: "tasks", Selected: []string{"SC-15"}, Report: "docs/SC-15.md"}
	ctx, err := s.pageContext(w.ID, c)
	if err != nil {
		t.Fatal(err)
	}
	last := ctx.Facts[len(ctx.Facts)-1]
	if !strings.Contains(last.Value, "graphe réparé") || last.Kind != "texte_non_fiable" || last.Source != "report/docs/SC-15.md" {
		t.Fatalf("rapport absent ou non sourcé : %+v", last)
	}
	write("Un autre résultat.")
	changed, err := s.pageContext(w.ID, c)
	if err != nil {
		t.Fatal(err)
	}
	if ctx.Hash == changed.Hash {
		t.Fatal("rapport modifié avec empreinte identique")
	}
	for _, bad := range []PageCoordinates{{PageID: "tasks", Selected: []string{"SC-16"}, Report: c.Report}, {PageID: "tasks", Selected: c.Selected, Report: "../secret.md"}, {PageID: "tasks", Report: c.Report}} {
		if _, err := s.pageContext(w.ID, bad); err == nil {
			t.Fatalf("coordonnées indues admises : %+v", bad)
		}
	}
	write(strings.Repeat("a", 13000))
	limited, err := s.pageContext(w.ID, c)
	if err != nil {
		t.Fatal(err)
	}
	if !limited.Truncated || !strings.Contains(strings.Join(limited.Omissions, " "), "12 000") {
		t.Fatal("troncature non annoncée")
	}
}

func TestReportSummaryRequiresTwoShortLines(t *testing.T) {
	s := storeTest(t)
	w := assistWork(t, s)
	ctx, err := s.pageContext(w.ID, PageCoordinates{PageID: "tasks"})
	if err != nil {
		t.Fatal(err)
	}
	tpl, _ := assistTemplate("report_summary.v1")
	prompt, err := buildAssistPrompt(ctx, tpl, tpl.Question)
	if err != nil || !strings.Contains(prompt, `"interpretation":"Le rapport annonce un résultat, à préciser ici.\nLa suite à donner, à préciser ici."`) {
		t.Fatal("exemple de sortie du prompt incompatible avec les deux lignes", err)
	}

	for _, text := range []string{"Constat.\nSuite.", "Une ligne.", "A.\nB.\nC.", strings.Repeat("a", 321) + "\nSuite."} {
		reply := answerFor(ctx, "report_summary.v1", func(a *AssistantAnswer) { a.Interpretation = text })
		_, refusal, _ := validateAssistantReply(reply, ctx, "report_summary.v1")
		if (refusal == nil) != (text == "Constat.\nSuite.") {
			t.Fatalf("contrat deux lignes : %q : %+v", text, refusal)
		}
	}
}
