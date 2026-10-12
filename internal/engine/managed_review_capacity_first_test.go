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

func TestCandidateFragmentsUseObservedCapacityBeforeLegacyIndivisibleRefusal(t *testing.T) {
	s, w, a, c, _, _ := fragmentBeginFixture(t)
	catalog := filepath.Join(t.TempDir(), "models_cache.json")
	t.Setenv("CODEX_HOME", filepath.Dir(catalog))
	cache := `{"models":[{"slug":"gpt-5.6-sol","visibility":"list","supported_reasoning_levels":[{"effort":"medium"}],"context_window":272000,"effective_context_window_percent":95}]}`
	if err := os.WriteFile(catalog, []byte(cache), 0600); err != nil {
		t.Fatal(err)
	}
	command := filepath.Join(s.root, "review-fixture/codex")
	if err := os.WriteFile(command, []byte("#!/usr/bin/env python3\nimport json,os,sys\n"+tokenMetadataFixture+"\nraise SystemExit('provider invocation forbidden')\n"), 0700); err != nil {
		t.Fatal(err)
	}
	ps, err := s.providers()
	if err != nil {
		t.Fatal(err)
	}
	ps.Providers["managed-review-fixture"] = Provider{Command: command}
	raw, _ := json.Marshal(ps)
	if err = os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	cfg, err := s.reviewerConfig("managed-review-fixture", "standard", 8)
	if err != nil {
		t.Fatal(err)
	}
	w.Planning.Reviewer = cfg
	// One complete text piece exceeds only the historical byte limit.
	c.Diff = "diff --git a/large.json b/large.json\n" + strings.Repeat("+evidence with spaces\n", 10000)
	if _, err = planManagedReviewFragments(c, 100, 2); err == nil {
		t.Fatal("regression fixture fits legacy packet")
	}
	before, _ := json.Marshal(c)
	p, err := s.planCandidateFragments(w, a, c, 8)
	if err != nil {
		t.Fatal("observed capacity not attempted after legacy refusal:", err)
	}
	if p.Version != 4 {
		t.Fatalf("expected anchored capacity protocol4, got %d", p.Version)
	}
	var restored strings.Builder
	for _, packet := range p.Packets {
		for _, piece := range packet.Artifacts {
			if piece.Kind == "diff" {
				restored.WriteString(piece.Content)
			}
		}
	}
	after, _ := json.Marshal(c)
	if restored.String() != c.Diff || string(before) != string(after) {
		t.Fatal("original evidence altered")
	}
	if err = validateManagedReviewFragments(c, p); err != nil {
		t.Fatal(err)
	}
	if managedReviewCalls(t, s) != 0 {
		t.Fatal("preview spent provider calls")
	}
}

func TestFreshCapacityFragmentsRetainRefusals(t *testing.T) {
	c := fragmentPlanFixture()
	pieces, err := managedFragmentArtifacts(c)
	if err != nil {
		t.Fatal(err)
	}
	for _, mode := range []string{"calls", "bytes", "tokens", "invalid-capacity", "missing", "duplicate", "binary"} {
		t.Run(mode, func(t *testing.T) {
			b := reviewTokenBudgetFixture()
			available := 10
			groups := [][]managedReviewFragmentArtifact{append([]managedReviewFragmentArtifact(nil), pieces...)}
			current := c
			switch mode {
			case "calls":
				available = 2
			case "bytes":
				b.MaxBytes = 100
			case "tokens":
				b.ContextTokens = 104000
			case "invalid-capacity":
				b.Model = "unknown"
			case "missing":
				groups[0] = groups[0][1:]
			case "duplicate":
				groups[0] = append(groups[0], pieces[0])
			case "binary":
				current.Diff += "diff --git a/b b/b\nGIT binary patch\n"
			}
			if _, err := planFreshTokenFragments(current, groups, nil, nil, nil, b, "", available); err == nil {
				t.Fatal("invalid plan admitted")
			}
		})
	}
}

func TestFreshCapacityFragmentsPreserveCoordination(t *testing.T) {
	c := fragmentPlanFixture()
	files := []string{}
	for i := 0; i < 20; i++ {
		files = append(files, fmt.Sprintf("f%d", i))
	}
	proposal := ReviewCoordinationProposal{Candidate: c.Candidate, Evidence: coordinationEvidence(c), FinalReview: "Inspect all interactions and every task criterion.", Lots: []ReviewCoordinationLot{{ID: "evidence", Kind: "component", Objective: "Inspect complete changed files", Files: files, Criteria: coordinationCriteria(c)}}}
	groups, order, err := coordinatedArtifactGroups(c, proposal, files)
	if err != nil {
		t.Fatal(err)
	}
	p, err := planFreshTokenFragments(c, groups, order, &proposal, files, reviewTokenBudgetFixture(), "", 12)
	if err != nil {
		t.Fatal(err)
	}
	if err = validateCoordinatedFragmentCoverage(c, p); err != nil {
		t.Fatal(err)
	}
	p.Packets[0].Lot = "other"
	if validateManagedReviewFragments(c, p) == nil {
		t.Fatal("lot substitution admitted")
	}
}

func TestFreshCapacityCurrentPublicDossier(t *testing.T) {
	path := os.Getenv("SWARM_CAPACITY_PUBLIC_CONTEXT")
	if path == "" {
		t.Skip("operator public dossier not provided")
	}
	raw, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	var c managedReviewContext
	if err = json.Unmarshal(raw, &c); err != nil {
		t.Fatal(err)
	}
	pieces, err := managedFragmentArtifacts(c)
	if err != nil {
		t.Fatal(err)
	}
	p, err := planFreshTokenFragments(c, [][]managedReviewFragmentArtifact{pieces}, nil, nil, nil, reviewTokenBudgetFixture(), "", 18)
	if err != nil {
		t.Fatal(err)
	}
	t.Logf("complete public candidate=%s pieces=%d packets=%d calls=%d", c.Candidate, len(pieces), len(p.Packets), len(p.Packets)+2)
}
