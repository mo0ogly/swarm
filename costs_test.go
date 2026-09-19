//go:build linux

package main

import (
	"strings"
	"testing"
)

func agentAvecCout(t *testing.T, s *Store, w Work, r Launch, cout *float64) Agent {
	t.Helper()
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	if cout != nil {
		a.Usage = &Usage{Input: 10, Output: 5, ReportedCost: cout, Source: "test"}
		if e = s.saveAgent(a); e != nil {
			t.Fatal(e)
		}
	}
	return a
}

func TestCostSummaryDistinguishesReportedFromSilent(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	cout := 1.25
	agentAvecCout(t, s, w, r, &cout)

	total, e := s.costSummary(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	if total.Reported != 1.25 {
		t.Fatalf("coût rapporté attendu 1.25, obtenu %v", total.Reported)
	}
	if total.WithCost != 1 || total.Silent != 0 {
		t.Fatalf("une tentative rapporte, aucune muette : %+v", total.CostTotal)
	}
	if total.ByTask["t1"].Reported != 1.25 {
		t.Fatalf("agrégation par tâche absente : %+v", total.ByTask)
	}
}

func TestCostSummaryCountsSilentAttemptsWithoutInventing(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	agentAvecCout(t, s, w, r, nil)

	total, e := s.costSummary(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	if total.Silent != 1 || total.WithCost != 0 {
		t.Fatalf("une tentative sans coût doit être comptée muette : %+v", total.CostTotal)
	}
	if total.Reported != 0 {
		t.Fatalf("aucun coût ne doit être inventé : %v", total.Reported)
	}
}

// La phrase affichée doit dire ce qu'elle sait et ce qu'elle ignore. Un total
// partiel présenté comme complet est un mensonge par omission.
func TestCostTextDeclaresWhatIsMissing(t *testing.T) {
	cas := []struct {
		nom      string
		total    CostTotal
		attendu  []string
		interdit []string
	}{
		{"rien de rapporté", CostTotal{Silent: 2}, []string{"non rapporté"}, []string{"0.00", "consommé"}},
		{"complet", CostTotal{Reported: 4.20, WithCost: 3}, []string{"4.20", "3 tentative"}, []string{"sans coût", "consommé"}},
		{"partiel", CostTotal{Reported: 4.20, WithCost: 3, Silent: 2}, []string{"4.20", "2 sans coût rapporté"}, []string{"consommé"}},
		{"aucune tentative", CostTotal{}, []string{"aucune tentative"}, []string{"0.00"}},
	}
	for _, c := range cas {
		t.Run(c.nom, func(t *testing.T) {
			texte := c.total.Text()
			for _, mot := range c.attendu {
				if !strings.Contains(texte, mot) {
					t.Fatalf("%q absent de %q", mot, texte)
				}
			}
			for _, mot := range c.interdit {
				if strings.Contains(texte, mot) {
					t.Fatalf("%q ne doit pas figurer dans %q", mot, texte)
				}
			}
		})
	}
}
