//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// Exercise the actual structured adapter: missing review instructions must not
// disappear between prompt construction and provider reception. The double
// proves reception only, never a real model's understanding or acceptance.
func TestManagedReviewProducerStatusContractReachesProvider(t *testing.T) {
	p, inspection := inspectionReplyFixture()
	inspection.Findings[0].Verdict = "unknown"
	inspection.Findings[0].Needs = []string{"current-full-suite-receipt.out"}
	result, _ := json.Marshal(map[string]any{"type": "result", "structured_output": inspection})
	script := filepath.Join(t.TempDir(), "claude")
	body := "#!/bin/sh\ninput=$(cat)\nprintf '%s' \"$input\" >\"$0.prompt\"\n" +
		"case \"$input\" in *'status=pass signifie uniquement'*'outcome=complete signifie'*'absente de ce paquet'*) printf '%s\\n' '" + string(result) + "';; *) printf '%s\\n' '{\"type\":\"result\",\"structured_output\":{\"contract_missing\":true}}';; esac\n"
	if err := os.WriteFile(script, []byte(body), 0700); err != nil {
		t.Fatal(err)
	}
	prompt, err := managedFragmentInspectionPrompt(managedReviewPrefix("Independent method"), p)
	if err != nil {
		t.Fatal(err)
	}
	reply, err := runStructuredProvider(Provider{Command: script}, nil, prompt, managedFragmentPacketSchema(p), 5*time.Second, func() bool { return true }, nil)
	if err != nil {
		t.Fatal(err)
	}
	state, _, err := parseManagedFragmentInspection(reply, p)
	if err != nil || state != "unknown" {
		t.Fatalf("provider did not receive the producer/fragment boundaries: state=%s err=%v", state, err)
	}
	received, err := os.ReadFile(script + ".prompt")
	if err != nil || !strings.Contains(string(received), deliveryEvidenceBoundary) {
		t.Fatal("shared local-delivery contract missing from actual provider input", err)
	}
}

func TestManagedReviewProducerStatusContractSharedByFinalPrefix(t *testing.T) {
	prefix := managedReviewPrefix("Independent method")
	if !strings.Contains(prefix, deliveryEvidenceBoundary) {
		t.Fatal("final and unfragmented reviews lack the producer's exact status semantics")
	}
}
