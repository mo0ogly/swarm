//go:build linux

package main

import (
	"encoding/json"
	"strings"
	"testing"
)

func TestPreparationExplicitRecentContextPreservesArchive(t *testing.T) {
	s, p, r := prepDialogueFixture(t, prepReplyScript)
	for i := 0; i < 6; i++ {
		turn := PreparationTurn{ID: newID("pt-"), PreparationID: p.ID, Status: "answered", Question: strings.Repeat("q", 7000), Answer: &PreparationAnswer{Message: "Dernière réponse complète", Brief: strings.Repeat("b", 10000)}, Revision: p.Revision, CreatedAt: now()}
		if i == 0 {
			turn.Question = "ANCIEN_ECHANGE_" + turn.Question
		}
		if i == 5 {
			turn.Question = "DERNIER_ECHANGE_" + turn.Question
		}
		raw, _ := json.Marshal(turn)
		if _, e := s.db.Exec("INSERT INTO preparation_turns(id,preparation_id,status,body) VALUES(?,?,?,?)", turn.ID, p.ID, turn.Status, raw); e != nil {
			t.Fatal(e)
		}
	}
	if _, e := s.sendPreparation(r); e == nil || !strings.Contains(e.Error(), "dernier échange") {
		t.Fatal(e)
	}
	r.ContextMode = "recent"
	turn, e := s.sendPreparation(r)
	if e != nil {
		t.Fatal(e)
	}
	if len(turn.Prompt) > preparationContextLimit || !strings.Contains(turn.Prompt, "DERNIER_ECHANGE_") || strings.Contains(turn.Prompt, "ANCIEN_ECHANGE_") || !strings.Contains(turn.Prompt, `"earlier_exchanges_not_sent":5`) {
		t.Fatal("wrong context scope")
	}
	turns, e := s.preparationTurns(p.ID)
	if e != nil || len(turns) != 7 || len(turns[0].Answer.Brief) != 10000 {
		t.Fatal("archive modified", e)
	}
	r.ContextMode = "full"
	if _, e = s.sendPreparation(r); e == nil {
		t.Fatal("same event changed context")
	}
}
func TestPreparationProposalBriefAdoptionAtomic(t *testing.T) {
	s, p, r := prepDialogueFixture(t, prepReplyScript)
	turn, e := s.sendPreparation(r)
	if e != nil {
		t.Fatal(e)
	}
	if e = s.finishPreparationTurn(turn, `{"message":"Brief relu","brief":"Nouveau besoin clarifié"}`, nil, ""); e != nil {
		t.Fatal(e)
	}
	req := prepRequest(p, "use-proposal")
	req.Turn = turn.ID
	req.AdoptBrief = true
	got, e := s.preparationCommand(req)
	if e != nil {
		t.Fatal(e)
	}
	if got.Revision != p.Revision+1 || !preparationBriefCurrent(got) || got.Brief.SourceTurn != turn.ID || got.PlanReady {
		t.Fatal(got)
	}
	replay, e := s.preparationCommand(req)
	if e != nil || replay.Revision != got.Revision {
		t.Fatal(e)
	}
	bad := prepRequest(got, "save")
	bad.Document = "brief"
	bad.Text = "autre"
	bad.AdoptBrief = true
	if _, e = s.preparationCommand(bad); e == nil {
		t.Fatal("implicit adoption")
	}
}
