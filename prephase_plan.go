package main

// Counts describe the checked proposal, never completion or dispatch eligibility.
type PreparationPlanSummary struct {
	Missions      int `json:"missions"`
	Dependencies  int `json:"dependencies"`
	OpenQuestions int `json:"open_questions"`
}

func preparationPlanSummary(p ActionPlan) *PreparationPlanSummary {
	s := &PreparationPlanSummary{Missions: len(p.Tasks)}
	for _, t := range p.Tasks {
		s.Dependencies += len(t.Depends)
	}
	for _, q := range p.Questions {
		if !nonempty(q.Answer) {
			s.OpenQuestions++
		}
	}
	return s
}
func preparationBriefCurrent(p Preparation) bool {
	return p.Brief != nil && p.Brief.Hash == p.Documents["brief"].Hash && p.Brief.NeedHash == p.Documents["besoin"].Hash
}
func (t PreparationTurn) proposalDocument() (string, string) {
	if t.Answer == nil {
		return "brief", ""
	}
	if t.Target == "plan" {
		return "plan", t.Answer.Plan
	}
	return "brief", t.Answer.Brief
}
