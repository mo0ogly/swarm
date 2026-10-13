//go:build linux

package engine

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestStructuredProviderFailurePreservesSafeCause(t *testing.T) {
	for _, tc := range []struct{ name, line, expected string }{
		{"message", `{"type":"error","message":"Selected model is at capacity. PRIVATE_SECRET"}`, "provider_capacity"},
		{"nested", `{"type":"turn.failed","error":{"message":"Selected model is at capacity. PRIVATE_SECRET"}}`, "provider_capacity"},
		{"code", `{"type":"turn.failed","error":{"code":"model_capacity_exceeded","message":"PRIVATE_SECRET"}}`, "provider_capacity"},
		{"unknown", `{"type":"turn.failed","error":{"message":"PRIVATE_SECRET https://secret.invalid"}}`, "provider_error"},
	} {
		t.Run(tc.name, func(t *testing.T) {
			script := filepath.Join(t.TempDir(), "claude")
			if err := os.WriteFile(script, []byte("#!/bin/sh\nprintf '%s\\n' '"+tc.line+"'\nexit 1\n"), 0700); err != nil {
				t.Fatal(err)
			}
			_, err := runStructuredProvider(Provider{Command: script}, nil, "prompt", "{}", time.Second, func() bool { return true }, nil)
			if err == nil || !strings.Contains(err.Error(), tc.expected) || strings.Contains(err.Error(), "PRIVATE_SECRET") || strings.Contains(err.Error(), "secret.invalid") {
				t.Fatal("safe cause missing or private data exposed", err)
			}
		})
	}
	out := readAssistOutput(strings.NewReader(`{"type":"item.completed","item":{"text":"Selected model is at capacity PRIVATE_SECRET"}}`))
	if out.err != nil {
		t.Fatal("tool or assistant text classified as provider refusal", out.err)
	}
}
