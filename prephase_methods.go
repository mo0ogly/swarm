package main

import (
	"io"
	"os"
)

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
		{ID: "apex", Title: "APEX — analyser et planifier", Phases: []string{"Analyze", "Plan"}, Paths: []string{".claude/skills/apex/SKILL.md"}},
		{ID: "ks-feature", Title: "KS — cadrer une fonctionnalité", Phases: []string{"Cadrage", "Plan"}, Paths: []string{".claude/commands/ks-feature.md", ".claude/commands/ks-plan.md"}},
		{ID: "audit-pdca", Title: "Audit PDCA — préparer l’audit", Phases: []string{"Plan"}, Paths: []string{".claude/skills/audit-pdca/SKILL.md"}},
	}
	for i := range methods {
		m := &methods[i]
		m.Paths = append(m.Paths, "tools/agent-workflows/CONTRACT.md")
		joined := ""
		m.Available = true
		for _, path := range m.Paths {
			p, e := localFile(s.root, path)
			if e != nil {
				m.Available = false
				m.Reason = "Méthode hors périmètre : " + path
				break
			}
			f, e := os.Open(p)
			if e != nil {
				m.Available = false
				m.Reason = "Fichier de méthode indisponible : " + path
				break
			}
			info, statErr := f.Stat()
			if statErr != nil || !info.Mode().IsRegular() {
				f.Close()
				m.Available = false
				m.Reason = "Fichier de méthode non régulier : " + path
				break
			}
			b, e := io.ReadAll(io.LimitReader(f, 131073))
			f.Close()
			if e != nil || len(b) > 131072 {
				m.Available = false
				m.Reason = "Fichier de méthode illisible ou trop long : " + path
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
