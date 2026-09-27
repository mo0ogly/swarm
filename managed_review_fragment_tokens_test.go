//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

func TestManagedFragmentTokenPlanPreservesLegacyAndEvidence(t *testing.T) {
	c := fragmentPlanFixture()
	old, err := planManagedReviewFragments(c, 100, 2)
	if err != nil {
		t.Fatal(err)
	}
	before, _ := json.Marshal(old)
	p, err := planTokenManagedFragments(c, old, reviewTokenBudgetFixture(), "", 7)
	if err != nil {
		t.Fatal(err)
	}
	if p.Version != 4 || fragmentPaidInspections(p) >= len(old.Packets) {
		t.Fatal("no useful reduction")
	}
	got, _ := json.Marshal(old)
	if string(got) != string(before) {
		t.Fatal("legacy plan mutated")
	}
	var restored managedReviewFragmentPlan
	raw, _ := json.Marshal(p)
	if err := json.Unmarshal(raw, &restored); err != nil {
		t.Fatal(err)
	}
	if err := preflightManagedFragmentCalls("", c, restored); err != nil {
		t.Fatal(err)
	}
	for _, packet := range p.Packets {
		prompt, err := managedFragmentInspectionPrompt("", packet)
		if err != nil {
			t.Fatal(err)
		}
		if _, err := p.InputBudget.measure(prompt, managedFragmentPacketSchema(packet)); err != nil {
			t.Fatal(err)
		}
	}
	// The current original evidence, questions and final decision remain checked.
	replies := fragmentHistoricalReplies(t, p)
	if _, err := managedFragmentRequestPrompt("", c, p, replies); err != nil {
		t.Fatal(err)
	}
	if _, _, err := managedFragmentDecisionPrompt("", c, p, replies, nil); err != nil {
		t.Fatal(err)
	}
	p.InputBudget.Model = "substituted"
	if validateManagedReviewFragments(c, p) == nil {
		t.Fatal("capacity corruption accepted")
	}
	old.InputBudget = p.InputBudget
	if validateManagedReviewFragments(c, old) == nil {
		t.Fatal("legacy capacity override accepted")
	}
}

func TestManagedFragmentTokenPlanKeepsHistoricalObligations(t *testing.T) {
	c, old, r, op, j, delta := fragmentHistoricalFixture(t)
	legacy, err := differentialFragmentPlan(c, old, r, op, j, delta, 100)
	if err != nil {
		t.Fatal(err)
	}
	p, err := planTokenManagedFragments(c, legacy, reviewTokenBudgetFixture(), "", 7)
	if err != nil {
		t.Fatal(err)
	}
	if len(p.Reused) != len(legacy.Reused) {
		t.Fatal("historical evidence dropped")
	}
	for i, ref := range p.Reused {
		if !reflect.DeepEqual(ref.Original, legacy.Reused[i].Original) || ref.Reply != legacy.Reused[i].Reply {
			t.Fatal("old identity rewritten")
		}
	}
	b, err := managedFragmentFinalBundle(c, p, fragmentHistoricalReplies(t, p))
	if err != nil || len(b.Questions) < len(p.Reused) || b.ChangeDiff != delta {
		t.Fatal("impact obligation lost", err)
	}
	if _, err := planTokenManagedFragments(c, legacy, reviewTokenBudgetFixture(), "", 2); err == nil {
		t.Fatal("call budget bypassed")
	}
}

func TestObservedReviewInputCapabilityIsLocalAndExact(t *testing.T) {
	dir := t.TempDir()
	t.Setenv("CODEX_HOME", dir)
	command := filepath.Join(dir, "codex")
	if err := os.WriteFile(command, []byte("#!/usr/bin/env python3\nimport json,os,sys\n"+tokenMetadataFixture), 0700); err != nil {
		t.Fatal(err)
	}
	p := Provider{Command: command}
	route := &ModelRoute{Model: "gpt-5.6-sol"}
	digest := strings.Repeat("a", 64)
	if _, err := observedReviewInputBudget(p, route, digest); err == nil {
		t.Fatal("missing catalog accepted")
	}
	path := filepath.Join(dir, "models_cache.json")
	valid := `{"client_version":"test-client","models":[{"slug":"gpt-5.6-sol","context_window":272000,"effective_context_window_percent":95}]}`
	if err := os.WriteFile(path, []byte(valid), 0600); err != nil {
		t.Fatal(err)
	}
	b, err := observedReviewInputBudget(p, route, digest)
	if err != nil || b.ProviderDigest != digest || b.ClientVersion != "codex-cli isolated-test-client" {
		t.Fatal("lost identity", err)
	}
	for _, invalid := range []string{strings.Replace(valid, "272000", "1050000", 1), strings.Replace(valid, "gpt-5.6-sol", "another-model", 1), `{}`} {
		os.WriteFile(path, []byte(invalid), 0600)
		if _, err := observedReviewInputBudget(p, route, digest); err == nil {
			t.Fatal("unknown or inflated capability admitted")
		}
	}
}
