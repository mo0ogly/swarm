//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// Use the shipped pack, not the short method doubles used by other tests. A
// missing file or an oversized real method must fail before a user sends a turn.
func installRepositoryPreparationMethods(t *testing.T, s *Store) {
	t.Helper()
	for _, method := range s.preparationMethods() {
		for _, path := range method.Paths {
			data, err := os.ReadFile(path)
			if err != nil {
				t.Fatal(err)
			}
			target := filepath.Join(s.root, path)
			if err = os.MkdirAll(filepath.Dir(target), 0700); err != nil {
				t.Fatal(err)
			}
			if err = os.WriteFile(target, data, 0600); err != nil {
				t.Fatal(err)
			}
		}
	}
}

func TestRepositoryPreparationMethodsContext(t *testing.T) {
	s := storeTest(t)
	installRepositoryPreparationMethods(t, s)
	p := prepCreate(t, s)
	for _, method := range s.preparationMethods() {
		t.Run(method.ID, func(t *testing.T) {
			if !method.Available || method.Hash == "" {
				t.Fatalf("shipped preparation method unavailable: %+v", method)
			}
			for _, target := range []string{"brief", "plan"} {
				prompt, err := s.preparationPromptFor(p, method, nil, "Prepare the scoped change.", target)
				if err != nil {
					t.Fatalf("real method cannot produce a %s context: %v", target, err)
				}
				var payload struct {
					Method        string            `json:"method"`
					MethodSources map[string]string `json:"method_sources"`
				}
				start := strings.LastIndex(prompt, "\n{")
				if start < 0 {
					t.Fatal("no structured preparation payload")
				}
				if err = json.Unmarshal([]byte(prompt[start+1:]), &payload); err != nil {
					t.Fatal(err)
				}
				if payload.Method != method.ID || len(payload.MethodSources) != len(method.Paths) {
					t.Fatalf("wrong method or missing sources: %+v", payload)
				}
				for _, path := range method.Paths {
					want, err := os.ReadFile(path)
					if err != nil || payload.MethodSources[path] != string(want) {
						t.Fatalf("method/contract was omitted or truncated: %s (%v)", path, err)
					}
				}
				t.Logf("%s context: %d / %d bytes", target, len(prompt), preparationContextLimit)
			}
		})
	}
	alias, err := s.preparationMethod("audit_pdca")
	canonical, canonicalErr := s.preparationMethod("audit-pdca")
	if err != nil || canonicalErr != nil || alias.ID != canonical.ID || alias.Hash != canonical.Hash {
		t.Fatalf("audit_pdca compatibility alias diverged: %+v %+v %v %v", alias, canonical, err, canonicalErr)
	}
}

func TestRepositoryPreparationContractChangeRejectsOldContext(t *testing.T) {
	s := storeTest(t)
	installRepositoryPreparationMethods(t, s)
	p := prepCreate(t, s)
	before := s.preparationMethods()
	path := filepath.Join(s.root, "tools/agent-workflows/CONTRACT.md")
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	if err = os.WriteFile(path, append(data, []byte("\nAdditional scoped requirement.\n")...), 0600); err != nil {
		t.Fatal(err)
	}
	for _, old := range before {
		current, err := s.preparationMethod(old.ID)
		if err != nil || current.Hash == old.Hash {
			t.Fatalf("shared contract change did not invalidate %s: %v", old.ID, err)
		}
		if _, err = s.preparationPrompt(p, old, nil, "Continue."); err == nil {
			t.Fatalf("old %s contract accepted for a new turn", old.ID)
		}
	}
	for _, table := range []string{"agents", "reservations", "preparation_turns"} {
		var count int
		if err = s.db.QueryRow("SELECT count(*) FROM " + table).Scan(&count); err != nil || count != 0 {
			t.Fatalf("context preparation changed %s: %d (%v)", table, count, err)
		}
	}
}
