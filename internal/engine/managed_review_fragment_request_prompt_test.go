//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestManagedFragmentAllCallsPreflight(t *testing.T) {
	c, p, replies := finalBundleFixture(t)
	if err := preflightManagedFragmentCalls("guidance", c, p); err != nil {
		t.Fatal(err)
	}
	prompt, err := managedFragmentRequestPrompt("guidance", c, p, replies)
	if err != nil {
		t.Fatal(err)
	}
	if len(prompt)+len(managedFragmentRequestSchema) > managedReviewPromptLimit || !strings.Contains(prompt, "selection_bytes_limit") || !strings.Contains(prompt, "AUCUN VERDICT") {
		t.Fatal("unbounded or misleading selector")
	}
	if err = preflightManagedFragmentCalls(strings.Repeat("x", managedReviewPromptLimit), c, p); err == nil {
		t.Fatal("oversized prefix accepted")
	}
	p.ReservedFinalCalls = 1
	if err = preflightManagedFragmentCalls("", c, p); err == nil {
		t.Fatal("missing final call accepted")
	}
}
func TestManagedFragmentSelectionCostsCoverActualEncoding(t *testing.T) {
	c, p, replies := finalBundleFixture(t)
	b, err := managedFragmentFinalBundle(c, p, replies)
	if err != nil {
		t.Fatal(err)
	}
	fixed, _ := json.Marshal(managedFragmentEvidenceSelection{Candidate: c.Candidate, ContextDigest: p.ContextDigest, PlanDigest: b.PlanDigest, References: []managedFragmentEvidenceRef{}, Artifacts: []managedReviewFragmentArtifact{}})
	refs := []managedFragmentEvidenceRef{}
	bound := len(fixed)
	for j, a := range p.Packets[0].Artifacts {
		if j == 2 {
			break
		}
		ref := managedFragmentEvidenceRef{0, j, a.Digest}
		refs = append(refs, ref)
		ar, _ := json.Marshal(a)
		rr, _ := json.Marshal(ref)
		bound += len(ar) + len(rr) + 2
	}
	selected, err := selectManagedFragmentEvidence(c, p, refs, managedReviewPromptLimit)
	if err != nil {
		t.Fatal(err)
	}
	raw, _ := json.Marshal(selected)
	if len(raw) > bound {
		t.Fatal("cost estimate too small", len(raw), bound)
	}
}

// Once immutable partial replies have arrived, a complete original that fits
// the actual final message must not be rejected using hypothetical future
// findings/defects. The speculative reserve remains a pre-call admission check.
func TestManagedFragmentSelectionUsesReceivedEvidenceCapacity(t *testing.T) {
	c, p, replies := finalBundleFixture(t)
	b, err := managedFragmentFinalBundle(c, p, replies)
	if err != nil {
		t.Fatal(err)
	}
	conservative, err := managedFragmentDecisionCapacity("", c, p)
	if err != nil {
		t.Fatal(err)
	}
	base, _, err := renderManagedFragmentDecision("", c, b, nil)
	if err != nil {
		t.Fatal(err)
	}
	actual, err := fragmentInputRoom(p.InputBudget, base, fragmentDecisionSchema(b))
	if err != nil {
		t.Fatal(err)
	}
	actual -= len(`,"requested_original_evidence":`)
	ref := managedFragmentEvidenceRef{}
	largest := 0
	for i, packet := range p.Packets {
		for j, artifact := range packet.Artifacts {
			if len(artifact.Content) > largest {
				ref = managedFragmentEvidenceRef{i, j, artifact.Digest}
				largest = len(artifact.Content)
			}
		}
	}
	selected, err := selectManagedFragmentEvidence(c, p, []managedFragmentEvidenceRef{ref}, fragmentInputByteLimit(p.InputBudget))
	if err != nil {
		t.Fatal(err)
	}
	original, _ := json.Marshal(selected)
	gap := actual - conservative
	if gap <= 0 || conservative <= len(original) {
		t.Fatal("fixture needs speculative reserve and tunable prefix", conservative, actual, len(original))
	}
	target := max(1, len(original)-gap/2)
	prefix := strings.Repeat("x", conservative-target)
	room, err := managedFragmentDecisionCapacity(prefix, c, p)
	if err != nil || room >= len(original) {
		t.Fatal("fixture must discriminate speculative rejection", room, len(original), err)
	}
	final, visible, err := managedFragmentDecisionPrompt(prefix, c, p, replies, []managedFragmentEvidenceRef{ref})
	if err != nil || len(final)+len(fragmentDecisionSchema(b)) > fragmentInputByteLimit(p.InputBudget) || len(visible.Sources) == 0 {
		t.Fatal("whole original must actually fit before selection", err)
	}
	prompt, err := managedFragmentRequestPrompt(prefix, c, p, replies)
	if err != nil {
		t.Fatal(err)
	}
	var received struct {
		Limit       int     `json:"selection_bytes_limit"`
		Tokens      int     `json:"selection_tokens_limit"`
		FixedTokens int     `json:"selection_fixed_tokens"`
		Rows        [][]int `json:"cost_rows"`
	}
	parts := strings.SplitN(prompt, "SWARM_FRAGMENT_EVIDENCE_REQUEST\n", 2)
	if len(parts) != 2 || json.Unmarshal([]byte(parts[1]), &received) != nil {
		t.Fatal("actual selector payload unavailable")
	}
	if received.Limit < len(original) {
		t.Fatal("selector falsely declares available complete proof too large", received.Limit, len(original), room)
	}
	if _, err := selectManagedFragmentEvidence(c, p, []managedFragmentEvidenceRef{ref}, received.Limit); err != nil {
		t.Fatal("advertised selection cannot carry original", err)
	}
	if _, _, err := managedFragmentDecisionPrompt(strings.Repeat("x", managedReviewPromptLimit), c, p, replies, []managedFragmentEvidenceRef{ref}); err == nil {
		t.Fatal("real final transport limit relaxed")
	}
}

// Optional native-input oracle uses copied immutable files, never a live DB.
// It checks transport capacity only, not evidence sufficiency or task acceptance.
func TestManagedNativeSelectionCapacityOracle(t *testing.T) {
	dir := os.Getenv("SWARM_SELECTION_NATIVE_INPUTS")
	if dir == "" {
		t.Skip("native immutable input copies not supplied")
	}
	var c managedReviewContext
	var p managedReviewFragmentPlan
	var j managedFragmentJournal
	load := func(name string, out any) {
		t.Helper()
		raw, err := os.ReadFile(filepath.Join(dir, name))
		if err != nil {
			t.Fatal(err)
		}
		if err = json.Unmarshal(raw, out); err != nil {
			t.Fatal(err)
		}
	}
	load("context.json", &c)
	load("plan.json", &p)
	load("journal.json", &j)
	descriptor, workflow, err := agentWorkflow("reviewer")
	if err != nil {
		t.Fatal(err)
	}
	prefix := managedReviewPrefix(workflow)
	expected, err := os.ReadFile(filepath.Join(dir, "workflow-hash.txt"))
	methodRaw, _ := json.Marshal([]any{descriptor, workflow})
	if err != nil || strings.TrimSpace(string(expected)) != hash(methodRaw) {
		t.Fatal("actual workflow prefix differs", err)
	}
	reusable, err := validateManagedFragmentJournal(j, p, j.Attempt, j.ProviderDigest)
	if err != nil {
		t.Fatal(err)
	}
	replies := make([]string, len(p.Packets))
	for i := range replies {
		var ok bool
		replies[i], ok = reusable[i]
		if !ok {
			t.Fatal("missing original inspection", i)
		}
	}
	refs := []managedFragmentEvidenceRef{}
	for _, i := range []int{4, 5, 6, 7} {
		refs = append(refs, managedFragmentEvidenceRef{Packet: 0, Artifact: i, Digest: p.Packets[0].Artifacts[i].Digest})
	}
	prompt, _, err := managedFragmentDecisionPrompt(prefix, c, p, replies, refs)
	if err != nil {
		t.Fatal("four manifest parts do not actually fit", err)
	}
	bundle, err := managedFragmentFinalBundle(c, p, replies)
	if err != nil {
		t.Fatal(err)
	}
	measured, err := p.InputBudget.measure(prompt, fragmentDecisionSchema(bundle))
	if err != nil {
		t.Fatal(err)
	}
	selection, err := selectManagedFragmentEvidence(c, p, refs, fragmentInputByteLimit(p.InputBudget))
	if err != nil {
		t.Fatal(err)
	}
	encoded, _ := json.Marshal(selection)
	selector, err := managedFragmentRequestPromptVersion(prefix, c, p, replies, 3)
	if err != nil {
		t.Fatal(err)
	}
	parts := strings.Split(selector, "SWARM_FRAGMENT_EVIDENCE_REQUEST\n")
	if len(parts) != 2 {
		t.Fatal("missing selector payload")
	}
	var payload struct {
		Limit       int     `json:"selection_bytes_limit"`
		Tokens      int     `json:"selection_tokens_limit"`
		FixedTokens int     `json:"selection_fixed_tokens"`
		Rows        [][]int `json:"cost_rows"`
	}
	if err = json.Unmarshal([]byte(parts[1]), &payload); err != nil {
		t.Fatal(err)
	}
	t.Logf("actual four-part selection fits: encoded=%d advertised_bytes=%d final_bytes=%d final_tokens=%d remaining_tokens=%d", len(encoded), payload.Limit, measured.UTF8Bytes, measured.PromptTokens+measured.SchemaTokens, measured.TokenRoom)
	tokens := payload.FixedTokens
	for _, row := range payload.Rows {
		if row[0] == 0 && row[1] >= 4 && row[1] <= 7 {
			if len(row) != 4 {
				t.Fatal("missing separate token cost", row)
			}
			tokens += row[3]
		}
	}
	if tokens > payload.Tokens {
		t.Fatal("advertised token allowance rejects fitting manifest parts", tokens, payload.Tokens)
	}
	if payload.Limit < len(encoded) {
		t.Fatal("selector denies originals that fit exact declared byte/token budgets")
	}
}

// Portable counterpart of the native four-manifest reproducer: UTF-8 bytes
// are not token counts. Historical selection prompts retain their old limits.
func TestManagedFragmentSelectionSeparatesTokensAndBytes(t *testing.T) {
	c := fragmentPlanFixture()
	old, err := planManagedReviewFragments(c, 100, 2)
	if err != nil {
		t.Fatal(err)
	}
	p, err := planTokenManagedFragments(c, old, reviewTokenBudgetFixture(), "", 12)
	if err != nil {
		t.Fatal(err)
	}
	replies := fragmentHistoricalReplies(t, p)
	refs := []managedFragmentEvidenceRef{}
	for i, packet := range p.Packets {
		for j, a := range packet.Artifacts {
			if a.Kind == "diff" && len(refs) < 8 {
				refs = append(refs, managedFragmentEvidenceRef{i, j, a.Digest})
			}
		}
	}
	if len(refs) != 8 {
		t.Fatal("missing discriminating originals", len(refs))
	}
	if _, _, err := managedFragmentDecisionPrompt("", c, p, replies, refs); err != nil {
		t.Fatal("originals must actually fit", err)
	}
	selected, err := selectManagedFragmentEvidence(c, p, refs, p.InputBudget.MaxBytes)
	if err != nil {
		t.Fatal(err)
	}
	raw, _ := json.Marshal(selected)
	before, err := managedSelectionCapacity("", c, p, replies, 2)
	if err != nil {
		t.Fatal(err)
	}
	after, err := managedSelectionCapacity("", c, p, replies, 3)
	if err != nil || before >= len(raw) || after < len(raw) {
		t.Fatal("fixture does not discriminate false byte ceiling", before, len(raw), after, err)
	}
	if _, _, err := managedFragmentDecisionPrompt(strings.Repeat("x", p.InputBudget.MaxBytes), c, p, replies, refs); err == nil {
		t.Fatal("actual byte limit bypassed")
	}
	constrained := p
	budget := *p.InputBudget
	budget.OutputReserve = 180000
	constrained.InputBudget = &budget
	if _, _, err := managedFragmentDecisionPrompt("", c, constrained, replies, refs); err == nil {
		t.Fatal("actual token limit bypassed")
	}
}

func TestManagedFragmentSelectionRejectsUnsafeHistoricalText(t *testing.T) {
	old := fragmentPlanFixture()
	text := strings.Repeat("a", 8193)
	old.Sources = append(old.Sources, ReviewSource{Path: "unsafe.txt", Blob: strings.Repeat("e", 40), Content: text, Bytes: len(text), Digest: hash([]byte(text))})
	original, err := planManagedReviewFragments(old, 100, 2)
	if err != nil {
		t.Fatal(err)
	}
	// Historical byte-only packets can carry a lexical run the tokenizer rejects.
	// Put each original in its own packet so unrelated changes cannot invalidate it.
	artifacts := []managedReviewFragmentArtifact{}
	for _, packet := range original.Packets {
		artifacts = append(artifacts, packet.Artifacts...)
	}
	original.Packets = nil
	for _, a := range artifacts {
		original.Packets = append(original.Packets, managedReviewFragmentPacket{Version: original.Version, Candidate: original.Candidate, ContextDigest: original.ContextDigest, Index: len(original.Packets), Artifacts: []managedReviewFragmentArtifact{a}})
	}
	replies := fragmentHistoricalReplies(t, original)
	pr, _ := json.Marshal(original)
	j := managedFragmentJournal{Version: 1, PlanDigest: hash(pr), Attempt: "old-attempt", ProviderDigest: "old-provider"}
	for i, reply := range replies {
		raw, _ := json.Marshal(original.Packets[i])
		j.Entries = append(j.Entries, managedFragmentJournalEntry{Packet: i, PacketDigest: hash(raw), CallID: fmt.Sprint("old-", i), State: "inspected", Reply: reply, ReplyDigest: hash([]byte(reply))})
	}
	origin := IndependentReview{ID: "old-review", CandidateSHA: old.Candidate, FragmentJournal: &ManagedFragmentJournalAnchor{}}
	raw, _ := json.Marshal(old)
	var current managedReviewContext
	json.Unmarshal(raw, &current)
	current.Candidate = "new-candidate"
	current.Diff = strings.Replace(current.Diff, "+évidence\n", "+correction verified\n", 1)
	delta := "diff --git a/f0 b/f0\n@@ -1 +1 @@\n-évidence\n+correction verified\n"
	legacy, err := differentialFragmentPlan(current, old, origin, original, j, delta, 100)
	if err != nil || len(legacy.Reused) == 0 {
		t.Fatal("historical input fixture", err)
	}
	before, _ := json.Marshal(legacy)
	_, err = planTokenManagedFragments(current, legacy, reviewTokenBudgetFixture(), "", 100)
	if err == nil || !strings.Contains(err.Error(), "séquence lexicale trop longue") {
		t.Fatal("unsafe historical original tokenized without lexical guard", err)
	}
	after, _ := json.Marshal(legacy)
	if string(before) != string(after) {
		t.Fatal("historical original changed")
	}
}
