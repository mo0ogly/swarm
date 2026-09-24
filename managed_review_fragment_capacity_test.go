//go:build linux

package main

import (
	"strings"
	"testing"
)

func TestManagedFragmentDecisionCapacityBoundsActual(t *testing.T) {
	c, p, replies := finalBundleFixture(t)
	room, err := managedFragmentDecisionCapacity("guidance", c, p)
	if err != nil || room <= 0 {
		t.Fatal(room, err)
	}
	prompt, _, err := managedFragmentDecisionPrompt("guidance", c, p, replies, nil)
	if err != nil {
		t.Fatal(err)
	}
	if len(prompt)+len(managedReviewSchema)+room > managedReviewPromptLimit {
		t.Fatal("capacity underestimates maximum input")
	}
	if _, err = managedFragmentDecisionCapacity(strings.Repeat("x", managedReviewPromptLimit), c, p); err == nil {
		t.Fatal("oversized fixed input accepted")
	}
}
func TestManagedFragmentFindingSerializedBudget(t *testing.T) {
	for _, s := range []string{strings.Repeat("x", 96), strings.Repeat("\n", 48), strings.Repeat("é", 48)} {
		if !boundedFragmentFindingText(s) {
			t.Fatal("valid boundary", s)
		}
	}
	for _, s := range []string{strings.Repeat("x", 97), strings.Repeat("\n", 49), strings.Repeat("é", 49), strings.Repeat("<", 17)} {
		if boundedFragmentFindingText(s) {
			t.Fatal("escaped or UTF8 budget bypass", s)
		}
	}
}
