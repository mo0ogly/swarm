//go:build linux

package main

import "fmt"

// Coût réellement rapporté par les fournisseurs, par travail et par tâche.
//
// Règle unique de tout affichage de coût : un total partiel se déclare partiel.
// Les tentatives qui ne rapportent rien sont comptées à part, jamais estimées :
// les compter pour zéro reviendrait à affirmer qu'elles n'ont rien coûté, et à
// présenter une somme incomplète comme une mesure.
type CostTotal struct {
	Reported float64 `json:"reported_usd"`
	WithCost int     `json:"attempts_with_cost"`
	Silent   int     `json:"attempts_without_cost"`
}

type CostSummary struct {
	CostTotal
	ByTask map[string]CostTotal `json:"by_task"`
}

func (s *Store) costSummary(work string) (CostSummary, error) {
	out := CostSummary{ByTask: map[string]CostTotal{}}
	agents, e := s.agents(work)
	if e != nil {
		return out, e
	}
	for _, a := range agents {
		t := out.ByTask[a.TaskID]
		if a.Usage != nil && a.Usage.ReportedCost != nil {
			out.Reported += *a.Usage.ReportedCost
			out.WithCost++
			t.Reported += *a.Usage.ReportedCost
			t.WithCost++
		} else {
			out.Silent++
			t.Silent++
		}
		out.ByTask[a.TaskID] = t
	}
	return out, nil
}

// Text rend la phrase affichée partout : jamais « consommé », qui ferait passer
// une somme partielle pour la dépense réelle.
func (c CostTotal) Text() string {
	if c.WithCost == 0 {
		if c.Silent == 0 {
			return "aucune tentative"
		}
		return "coût réel non rapporté"
	}
	texte := fmt.Sprintf("%.2f USD rapportés sur %d tentative(s)", c.Reported, c.WithCost)
	if c.Silent > 0 {
		texte += fmt.Sprintf(" · %d sans coût rapporté", c.Silent)
	}
	return texte
}
