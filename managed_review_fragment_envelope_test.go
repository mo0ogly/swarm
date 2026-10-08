//go:build linux

package main

import (
	"fmt"
	"strings"
	"testing"
)

func TestFragmentPlannerAccountsForFullTransportEnvelope(t *testing.T) {
	c, _ := hunkPacketFixture()
	c.Diff = ""
	for i := 0; i < 60; i++ {
		c.Diff += fmt.Sprintf("diff --git a/file%d b/file%d\n", i, i) + "+" + strings.Repeat("x", 5350) + "\n"
	}
	plan, e := planManagedReviewFragments(c, 30, 2)
	if e != nil {
		t.Fatal(e)
	}
	for _, packet := range plan.Packets {
		// The workflow reserve must remain available after schema and anchor growth.
		if _, e := managedFragmentInspectionPrompt(strings.Repeat("w", managedFragmentPromptReserve-2048), packet); e != nil {
			t.Fatalf("packet %d: %v", packet.Index, e)
		}
	}
	if e := validateManagedReviewFragments(c, plan); e != nil {
		t.Fatal(e)
	}
}
