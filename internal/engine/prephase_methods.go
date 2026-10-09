package engine

type PreparationMethod struct {
	ID        string   `json:"id"`
	Title     string   `json:"title"`
	Phases    []string `json:"phases"`
	Paths     []string `json:"paths"`
	Hash      string   `json:"sha256"`
	Available bool     `json:"available"`
	Reason    string   `json:"reason,omitempty"`
}

// An explicit catalogue; arbitrary file names never become commands or permissions.
func (s *Store) preparationMethods() []PreparationMethod {
	methods := []PreparationMethod{
		{ID: "apex", Title: "Analyse et planification", Phases: []string{"Analyze", "Plan"}, Paths: []string{".claude/skills/apex/SKILL.md"}},
		{ID: "ks-feature", Title: "Parcours guidé — préparer une évolution", Phases: []string{"Cadrage", "Plan"}, Paths: []string{".claude/commands/ks-feature.md", ".claude/commands/ks-plan.md", ".claude/skills/product-planning/SKILL.md"}},
		{ID: "ks-product", Title: "Parcours de création d’application", Phases: []string{"Cadrage produit", "Parcours et stories", "Architecture", "Conception", "Plan"}, Paths: []string{".claude/skills/product-planning/SKILL.md", ".claude/skills/product-delivery/SKILL.md", ".claude/skills/product-review/SKILL.md"}},
		{ID: "debug", Title: "Diagnostiquer et corriger un problème", Phases: []string{"Diagnostic", "Plan"}, Paths: []string{".claude/skills/debug/SKILL.md"}},
		{ID: "audit-pdca", Title: "Examiner et améliorer — préparer l’examen", Phases: []string{"Plan"}, Paths: []string{".claude/skills/audit-pdca/SKILL.md"}},
	}
	for i := range methods {
		m := &methods[i]
		m.Paths = append(m.Paths, "tools/agent-workflows/CONTRACT.md")
		joined := ""
		m.Available = true
		for _, path := range m.Paths {
			b, e := s.preparationMethodSource(path)
			if e != nil {
				m.Available = false
				m.Reason = e.Error()
				break
			}
			joined += path + "\x00" + hash(b) + "\n"
		}
		if m.Available {
			m.Hash = hash([]byte(joined))
		}
	}
	return methods
}

func (s *Store) preparationMethod(id string) (PreparationMethod, error) {
	if id == "audit_pdca" {
		id = "audit-pdca"
	}
	for _, m := range s.preparationMethods() {
		if m.ID == id {
			if !m.Available {
				return m, preparationError("method_unavailable", m.Reason)
			}
			return m, nil
		}
	}
	return PreparationMethod{}, preparationError("method_unavailable", "Méthode non prise en charge ; consulter prepare methods.")
}

// Only this fresh projection should drive the UI; a stored verdict is historical.
func (s *Store) preparationFreshness(p *Preparation) {
	p.PlanReady = false
	if p.Verdict == nil || p.Brief == nil || p.ReceiptHistorical || p.Brief.NeedHash != p.Documents["besoin"].Hash {
		return
	}
	v := p.Verdict
	m, e := s.preparationMethod(p.Method)
	p.PlanReady = e == nil && m.Hash == v.MethodHash && p.MethodHash == v.MethodHash && p.Brief.Hash == v.BriefHash && p.Documents["brief"].Hash == v.BriefHash && p.Documents["plan"].Hash == v.PlanHash
}
