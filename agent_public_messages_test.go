//go:build linux

package main

import (
	"strings"
	"testing"
)

func TestPublicMessagesWithoutDetailedCapture(t *testing.T) {
	sink := &outputSink{}
	for _, wire := range []string{
		`{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"HIDDEN"},{"type":"text","text":"Je vérifie les tests."},{"type":"tool_use","input":{"secret":"HIDDEN"}}]}}`,
		`{"type":"item.completed","item":{"type":"reasoning","text":"HIDDEN"}}`,
		`{"type":"item.completed","item":{"type":"agent_message","text":"Tests terminés."}}`,
		`{"type":"user","message":{"content":[{"type":"tool_result","content":"HIDDEN"}]}}`,
	} {
		sink.line([]byte(wire))
	}
	if len(sink.logs) != 2 {
		t.Fatalf("public messages: %+v", sink.logs)
	}
	for _, log := range sink.logs {
		if log.Kind != "message" || strings.Contains(log.Message, "HIDDEN") {
			t.Fatal(log)
		}
	}
}
