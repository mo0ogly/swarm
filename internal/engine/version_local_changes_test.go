package engine

import (
	"strings"
	"testing"
)

func TestVersionHistoryLocalChangesAreSeparateFromPublishedReleases(t *testing.T) {
	valid := `{"schema_version":1,"releases":[],"local_changes":{"summary":{"fr":"Lecture des instructions","en":"Instruction reading"},"items":[{"fr":"Sommaire et source exacte","en":"Outline and exact source"}]}}`
	if _, err := parseVersionHistory([]byte(valid)); err != nil {
		t.Fatalf("valid local changes rejected: %v", err)
	}
	for _, invalid := range []string{
		strings.Replace(valid, `"en":"Instruction reading"`, `"en":""`, 1),
		strings.Replace(valid, `"items":[{`, `"unknown":true,"items":[{`, 1),
		strings.Replace(valid, `"fr":"Sommaire et source exacte"`, `"fr":"bad\u0000text"`, 1),
		strings.Replace(valid, `[{"fr":"Sommaire et source exacte","en":"Outline and exact source"}]`, `[]`, 1),
	} {
		if _, err := parseVersionHistory([]byte(valid)); err != nil {
			t.Fatalf("valid control failed before fault injection: %v", err)
		}
		if _, err := parseVersionHistory([]byte(invalid)); err == nil {
			t.Fatal("invalid local changes accepted")
		}
	}
	history, err := parseVersionHistory([]byte(valid))
	if err != nil || len(history.Releases) != 0 {
		t.Fatalf("local changes invented a published release: %+v, %v", history, err)
	}
}
