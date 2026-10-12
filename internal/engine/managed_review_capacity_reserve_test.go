//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"strings"
	"testing"
)

func TestMeasuredFinalCapacityCountsVariableFieldsOnce(t *testing.T) {
	c, _ := hunkPacketFixture()
	c.Diff = ""
	c.Sources = nil
	for i := 0; i < 280; i++ {
		c.Diff += fmt.Sprintf("diff --git a/f%d b/f%d\n+original evidence\n", i, i)
	}
	pieces, err := managedFragmentArtifacts(c)
	if err != nil {
		t.Fatal(err)
	}
	raw, _ := json.Marshal(c)
	budget := reviewTokenBudgetFixture()
	budget.ContextTokens = 247000
	budget.FinalReserveContractVersion = 1
	p := managedReviewFragmentPlan{Version: 4, InputBudget: &budget, Candidate: c.Candidate, ContextDigest: hash(raw), AvailableCalls: 20, ReservedFinalCalls: 2}
	for begin := 0; begin < len(pieces); begin += 42 {
		end := min(begin+42, len(pieces))
		p.Packets = append(p.Packets, managedReviewFragmentPacket{Version: 4, InputBudget: &budget, Candidate: c.Candidate, ContextDigest: p.ContextDigest, Index: len(p.Packets), DefectCitationContractVersion: 1, FindingContractVersion: 1, Artifacts: pieces[begin:end]})
	}
	legacy := budget
	legacy.FinalReserveContractVersion = 0
	old := p
	old.InputBudget = &legacy
	for i := range old.Packets {
		old.Packets[i].InputBudget = &legacy
	}
	if _, e := managedFragmentDecisionCapacity("", c, old); e == nil {
		t.Fatal("old double reserve no longer reproduced")
	}
	for i := range p.Packets {
		p.Packets[i].InputBudget = &budget
	}
	room, err := managedFragmentDecisionCapacity("", c, p)
	if err != nil {
		t.Fatal("complete bounded final inventory falsely refused", err)
	}
	if room <= 0 {
		t.Fatal("no original evidence room")
	}
	bundle := maximalManagedFragmentBundle(c, p)
	for i := range bundle.Evidence {
		bundle.Evidence[i].Opinion = strings.Repeat("\x00", 16)
		bundle.Evidence[i].Excerpt = "original"
	}
	for i := range bundle.Defects {
		bundle.Defects[i].Defect.Explanation = strings.Repeat("é", 512)
	}
	prompt, _, err := renderManagedFragmentDecision("", c, bundle, nil)
	if err != nil {
		t.Fatal(err)
	}
	m, err := budget.measure(prompt, fragmentDecisionSchema(bundle))
	if err != nil {
		t.Fatal(err)
	}
	if room > m.TokenRoom || room > budget.MaxBytes-m.UTF8Bytes {
		t.Fatal("advertised capacity exceeds actual byte/token room")
	}
}

func TestMeasuredFinalReserveContractRejectsUnknownVersion(t *testing.T) {
	for _, v := range []int{-1, 2} {
		b := reviewTokenBudgetFixture()
		b.FinalReserveContractVersion = v
		if _, err := b.measure("proof", "{}"); err == nil {
			t.Fatal("unknown reserve contract admitted")
		}
	}
}

func TestFreshReserveRejectsUnbudgetedHistoricalArray(t *testing.T) {
	c, p, replies := finalBundleFixture(t)
	_ = c
	packet := p.Packets[0]
	budget := reviewTokenBudgetFixture()
	budget.FinalReserveContractVersion = 1
	packet.Version = 4
	packet.InputBudget = &budget
	raw, _ := json.Marshal(packet)
	var reply map[string]json.RawMessage
	if err := json.Unmarshal([]byte(replies[0]), &reply); err != nil {
		t.Fatal(err)
	}
	identity, _ := json.Marshal(hash(raw))
	reply["packet_sha256"] = identity
	encoded, _ := json.Marshal(reply)
	if _, _, err := parseManagedFragmentInspection(string(encoded), packet); err == nil {
		t.Fatal("unbudgeted array admitted under fresh anchor contract")
	}
	if _, _, err := parseManagedFragmentInspection(replies[0], p.Packets[0]); err != nil {
		t.Fatal("historical array rejected", err)
	}
}
