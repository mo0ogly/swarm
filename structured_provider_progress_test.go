//go:build linux

package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestStructuredProviderProgressCounters(t *testing.T) {
	input := "not json\n{\"type\":\"assistant\",\"secret\":\"PRIVATE\"}\n{\"type\":\"PRIVATE\"}\n{\"type\":\"result\",\"result\":\"done\"}\n"
	out := readAssistOutput(strings.NewReader(input))
	if out.streamBytes != int64(len(input)) || out.events != 3 || !out.finalSeen || out.lastEvent != "result" {
		t.Fatalf("wrong counters: %+v", out)
	}
	if strings.Contains(out.progressDiagnostic(), "PRIVATE") {
		t.Fatal("diagnostic leaked content")
	}
	if assistEventKind("PRIVATE") != "other" {
		t.Fatal("unknown event exposed")
	}
}

func TestStructuredProviderTimeoutProgress(t *testing.T) {
	for _, tc := range []struct{ name, line, expected string }{
		{"silent", "", "0 octets, 0 événements JSON"},
		{"active", `printf '%s\n' '{"type":"assistant","secret":"PRIVATE"}'`, "1 événements JSON, dernier type=assistant"},
	} {
		t.Run(tc.name, func(t *testing.T) {
			script := filepath.Join(t.TempDir(), "claude")
			if err := os.WriteFile(script, []byte("#!/bin/sh\n"+tc.line+"\nsleep 10\n"), 0700); err != nil {
				t.Fatal(err)
			}
			_, err := runStructuredProvider(Provider{Command: script}, nil, "prompt", "{}", 150*time.Millisecond, func() bool { return true }, nil)
			if err == nil || !strings.Contains(err.Error(), tc.expected) || !strings.Contains(err.Error(), "dépassé") || strings.Contains(err.Error(), "PRIVATE") {
				t.Fatalf("unexpected diagnostic: %v", err)
			}
		})
	}
}
