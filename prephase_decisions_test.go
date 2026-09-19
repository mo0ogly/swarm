//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"strings"
	"testing"
)

func TestPreparationDecisionsBoundAndReplay(t *testing.T) {
	s, p, _ := prepDialogueFixture(t, prepReplyScript)
	p = prepAdopt(t, s, p)
	plan := fixturePlan()
	plan.Questions = []PlanQuestion{{Question: "Quel public ?"}, {Question: "Quelle langue ?"}}
	raw, _ := json.Marshal(plan)
	p = prepSave(t, s, p, "plan", string(raw))
	original := p.Documents["plan"]
	req := prepRequest(p, "answer-questions")
	req.Hash = original.Hash
	req.Decisions = []PlanQuestion{{Question: "Quel public ?", Answer: "Équipes métier"}, {Question: "Quelle langue ?"}}
	for _, bad := range []PreparationRequest{
		func() PreparationRequest { r := req; r.Hash = "old"; return r }(),
		func() PreparationRequest { r := req; r.Decisions = r.Decisions[:1]; return r }(),
		func() PreparationRequest {
			r := req
			r.Decisions = []PlanQuestion{{Question: "Autre question", Answer: "Oui"}, {Question: "Quelle langue ?"}}
			return r
		}(),
	} {
		if _, e := s.preparationCommand(bad); e == nil {
			t.Fatal("unbound answers accepted")
		}
	}
	next, e := s.preparationCommand(req)
	if e != nil {
		t.Fatal(e)
	}
	replay, e := s.preparationCommand(req)
	if e != nil || replay.Revision != next.Revision {
		t.Fatal("replay", e)
	}
	if next.PlanReady || next.Documents["plan"].BriefHash != original.BriefHash || next.Documents["plan"].MethodHash != original.MethodHash {
		t.Fatal("freshness changed")
	}
	verify := prepRequest(next, "validate-plan")
	verify.Hash = next.Documents["plan"].Hash
	if _, e = s.preparationCommand(verify); e == nil {
		t.Fatal("partial decisions validated")
	}
	var out bytes.Buffer
	term := preparationTerminal{s: s, p: next, out: &out, width: 80}
	if _, e = term.line("/decisions", false); e != nil {
		t.Fatal(e)
	}
	if !strings.Contains(out.String(), "À RÉSOUDRE") {
		t.Fatal(out.String())
	}
	if _, e = term.line("/repondre 2 Français", false); e != nil {
		t.Fatal(e)
	}
	if _, e = term.line("/verifier", false); e != nil || !term.p.PlanReady {
		t.Fatal(e)
	}
	var got ActionPlan
	json.Unmarshal([]byte(term.p.Documents["plan"].Text), &got)
	if got.Questions[1].Answer != "Français" || got.Tasks[1].Depends[0] != plan.Tasks[1].Depends[0] {
		t.Fatal(got)
	}
	// Resolving answers never rebinds an old plan to a changed brief.
	stale := prepSave(t, s, term.p, "brief", "Brief changé")
	req = prepRequest(stale, "answer-questions")
	req.Hash = stale.Documents["plan"].Hash
	req.Decisions = got.Questions
	stale, e = s.preparationCommand(req)
	if e != nil {
		t.Fatal(e)
	}
	if stale.PlanReady || stale.Verdict != nil || stale.Documents["plan"].BriefHash != original.BriefHash {
		t.Fatal("obsolete plan refreshed")
	}
	var count int
	if e = s.db.QueryRow("SELECT count(*) FROM agents").Scan(&count); e != nil || count != 0 {
		t.Fatal("agents created", e, count)
	}
}
