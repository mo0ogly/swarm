//go:build linux

package engine

import (
	"strings"
	"testing"
)

func TestPreparationAnswerContractDiagnostics(t *testing.T) {
	cases := []struct{ name, reply, code string }{
		{"syntax", `{"message":`, "response_json_syntax"},
		{"type", `{"message":42}`, "response_json_type"},
		{"unknown", `{"message":"ok","private-secret":"secret"}`, "response_json_fields"},
		{"extra", `{"message":"ok"} {"message":"another"}`, "response_json"},
		{"empty", `{"message":" "}`, "message_empty"},
		{"size", `{"message":"` + strings.Repeat("x", 8001) + `"}`, "message_size"},
		{"total", strings.Repeat("x", 24577), "response_size"},
		{"utf8", string([]byte{255}), "response_encoding"},
		{"prose", "Here is your answer\n{\"message\":\"ok\"}", "response_json_syntax"},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			_, err := decodePreparationAnswer(c.reply)
			if err == nil || !strings.HasPrefix(err.Error(), c.code+":") {
				t.Fatalf("%v", err)
			}
			if strings.Contains(err.Error(), "secret") {
				t.Fatal("provider data disclosed")
			}
		})
	}
	for _, reply := range []string{`{"message":"ok","brief":"# Brief"}`, "```json\n{\"message\":\"ok\",\"brief\":\"# Brief\"}\n```", "```\n{\"message\":\"ok\",\"brief\":\"# Brief\"}\n```"} {
		a, err := decodePreparationAnswer(reply)
		if err != nil || a.Message != "ok" || a.Brief != "# Brief" {
			t.Fatalf("%+v %v", a, err)
		}
	}
}

func TestPreparationAnswerFailurePreservesDocumentsAndUsage(t *testing.T) {
	s, p, r := prepDialogueFixture(t, prepReplyScript)
	turn, err := s.sendPreparation(r)
	if err != nil {
		t.Fatal(err)
	}
	if err = s.finishPreparationTurn(turn, `{"message":"ok","secret":"hidden"}`, nil, ""); err != nil {
		t.Fatal(err)
	}
	got, _ := s.preparationTurn(turn.ID)
	after, _ := s.preparation(p.ID)
	if got.Status != "failed" || got.Answer != nil || !strings.Contains(got.Error, "response_json_fields:") || strings.Contains(got.Error, "secret") || after.Revision != p.Revision {
		t.Fatalf("%+v", got)
	}
	replay, err := s.sendPreparation(r)
	if err != nil || replay.ID != turn.ID {
		t.Fatal("event replay consumed new call", err)
	}
}
