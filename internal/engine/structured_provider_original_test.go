//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
	"unicode/utf8"
)

func TestStructuredProviderPreservesCompleteOriginalJSON(t *testing.T) {
	for _, provider := range []string{"codex", "claude"} {
		for _, size := range []int{15999, 16000, 16001, 65536, 65537} {
			t.Run(fmt.Sprintf("%s/%d", provider, size), func(t *testing.T) {
				head, tail := `{"evidence":"`, `é"}`
				original := head + strings.Repeat("a", size-len(head)-len(tail)) + tail
				if len(original) != size || !json.Valid([]byte(original)) {
					t.Fatal("invalid boundary fixture")
				}
				event := map[string]any{"type": "result", "result": original}
				if provider == "codex" {
					event = map[string]any{"type": "item.completed", "item": map[string]any{"type": "agent_message", "text": original}}
				}
				raw, err := json.Marshal(event)
				if err != nil {
					t.Fatal(err)
				}
				command := filepath.Join(t.TempDir(), provider)
				if err = os.WriteFile(command, []byte("#!/bin/sh\ncat <<'SWARM_PUBLIC_REPLY'\n"+string(raw)+"\nSWARM_PUBLIC_REPLY\n"), 0700); err != nil {
					t.Fatal(err)
				}
				reply, err := runStructuredProvider(Provider{Command: command}, nil, "bounded fixture", "{}", 5*time.Second, func() bool { return true }, nil)
				if reply != original {
					t.Fatalf("structured original changed: want=%d received=%d suffix=%q", len(original), len(reply), reply[max(0, len(reply)-70):])
				}
				if size <= 65536 {
					if err != nil || !json.Valid([]byte(reply)) {
						t.Fatal("valid complete JSON rejected", err)
					}
				} else if err == nil || !strings.Contains(err.Error(), "64 Kio") {
					t.Fatal("technical limit lost or oversized original discarded", err)
				}
			})
		}
	}
}

func TestPublicReplyProjectionRemainsBoundedAndExcludesReasoning(t *testing.T) {
	original := strings.Repeat("é", 9000)
	event := map[string]any{"type": "item.completed", "item": map[string]any{"type": "agent_message", "text": original}}
	projected := providerReply(event)
	if !utf8.ValidString(projected) || !strings.HasSuffix(projected, "\n[Réponse tronquée à 16000 octets]") || len(projected) > 16050 {
		t.Fatal("human reply projection changed")
	}
	for _, kind := range []string{"reasoning", "command_execution", "tool_result"} {
		event["item"] = map[string]any{"type": kind, "text": "private reasoning or tool output"}
		if providerReply(event) != "" {
			t.Fatal("private event exposed", kind)
		}
	}
}
