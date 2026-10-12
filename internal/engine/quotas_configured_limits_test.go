//go:build linux

package engine

import "testing"

func TestConfiguredQuotasBeyondFormerStaticCeilings(t *testing.T) {
	s, w := managedFixture(t)
	r := QuotaChange{Schema: 1, EventID: "explicit-larger-budgets", Revision: w.Revision, Reason: "Explicit operator budgets replace the former fixed ceilings", Limits: QuotaValues{Tasks: quotaInt(201), Planning: 501, Decisions: 301, Reviews: quotaInt(401), JudgeCycles: quotaInt(701)}}
	if _, err := s.previewQuotas(w.ID, r); err != nil {
		t.Fatal("configured quota still blocked by a hardcoded ceiling", err)
	}
	after, err := s.configureQuotas(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	if after.Planning.MaxTasks != 201 || after.Planning.MaxActivations != 501 || after.Planning.MaxDecisions != 301 || after.Planning.Reviewer.MaxCalls != 401 || after.Planning.Judge.MaxCycles != 701 || after.Planning.Activations != w.Planning.Activations || after.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls {
		t.Fatal("saved policy ignored or consumed calls refunded")
	}
	if err := validatePlanningState(&after); err != nil {
		t.Fatal("saved explicit policy cannot be read as a valid planning state", err)
	}
	if _, err := s.reviewerConfig("managed-review-fixture", "auto", 401); err != nil {
		t.Fatal("reviewer configuration still has a fixed budget ceiling", err)
	}
}
