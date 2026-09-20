package main

import (
	"reflect"
	"strings"
	"testing"
)

func TestLanguageArgs(t *testing.T) {
	t.Setenv("SWARM_LANG", "fr")
	for _, tt := range []struct {
		args []string
		want []string
		lang string
	}{
		{[]string{"--lang", "en", "help"}, []string{"help"}, "en"},
		{[]string{"help", "--lang=fr"}, []string{"help"}, "fr"},
	} {
		got, lang, err := languageArgs(tt.args)
		if err != nil || lang != tt.lang || !reflect.DeepEqual(got, tt.want) {
			t.Fatalf("%v: %v %s %v", tt.args, got, lang, err)
		}
	}
	for _, args := range [][]string{{"--lang"}, {"--lang=de"}} {
		if _, _, err := languageArgs(args); err == nil {
			t.Fatalf("accepted %v", args)
		}
	}
	t.Setenv("SWARM_LANG", "en")
	if uiText("Langue") != "Language" {
		t.Fatal("English catalogue not embedded")
	}
	if uiText("My user content") != "My user content" {
		t.Fatal("unknown content changed")
	}
	_, lang, err := languageArgs([]string{"help"})
	if err != nil || lang != "en" {
		t.Fatal("environment language ignored")
	}
}

func TestEngineErrorTranslationPreservesDetails(t *testing.T) {
	t.Setenv("SWARM_LANG", "en")
	for _, tt := range []struct{ source, want string }{
		{"Le serveur a répondu HTTP 401. Vérifiez la clé, le modèle et l’adresse.", "The server returned HTTP 401. Check the key, model and URL."},
		{"Sélectionnez explicitement /ia mon-modèle avant de reprendre.", "Explicitly select /ia mon-modèle before resuming."},
		{"Configuration IA trop grande.", "AI configuration is too large."},
		{"provider raw output: $& /my/path", "provider raw output: $& /my/path"},
	} {
		if got := uiEngineText(tt.source); got != tt.want {
			t.Fatalf("%q: got %q, want %q", tt.source, got, tt.want)
		}
	}
	t.Setenv("SWARM_LANG", "fr")
	if got := uiEngineText("Configuration IA trop grande."); got != "Configuration IA trop grande." {
		t.Fatal(got)
	}
}

func TestIncompleteDeliveryTranslationKeepsEvidencePath(t *testing.T) {
	t.Setenv("SWARM_LANG", "en")
	source := "Livraison incomplète : bilan par critère absent (docs/task.delivery.json). Aucun appel de revue lancé ; le responsable doit examiner les éléments manquants avant une reprise autorisée."
	got := uiEngineText(source)
	if !strings.Contains(got, "Incomplete delivery: per-criterion delivery manifest missing (docs/task.delivery.json)") || strings.Contains(got, "Aucun") {
		t.Fatal(got)
	}
}
