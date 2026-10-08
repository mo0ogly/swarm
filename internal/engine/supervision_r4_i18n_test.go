package engine

import (
	"bytes"
	"strings"
	"testing"
)

func TestSupervisionR4EnglishHelpReflectsProviderRelay(t *testing.T) {
	for _, lang := range []string{"fr", "en"} {
		t.Run(lang, func(t *testing.T) {
			t.Setenv("SWARM_LANG", lang)
			var out, err bytes.Buffer
			if code := run([]string{"--help"}, &out, &err); code != 0 {
				t.Fatalf("%d %s", code, err.String())
			}
			text := out.String()
			if !strings.Contains(text, "providers relay show <agent> | decide <agent>") {
				t.Fatal("relay command absent from help")
			}
			want := "Options globales"
			if lang == "en" {
				want = "Global options"
				if strings.Contains(text, "Les mutations exigent") {
					t.Fatal("French help leaked")
				}
			}
			if !strings.Contains(text, want) {
				t.Fatal(text)
			}
		})
	}
}
