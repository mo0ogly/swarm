//go:build linux

package main

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestPreparationSourcesSafeSnapshotAndReplay(t *testing.T) {
	s := storeTest(t)
	prepMethods(t, s)
	p := prepCreate(t, s)
	os.WriteFile(filepath.Join(s.root, "main.go"), []byte("package main\n// donnée utile\n"), 0600)
	source, e := s.preparationReadSource("main.go", 1, 2)
	if e != nil {
		t.Fatal(e)
	}
	req := prepRequest(p, "source-add")
	req.Source = &PreparationSourceRef{Path: source.Path, Start: source.Start, End: source.End, Hash: source.Hash}
	p, e = s.preparationCommand(req)
	if e != nil || len(p.Sources) != 1 {
		t.Fatal(p, e)
	}
	os.WriteFile(filepath.Join(s.root, "main.go"), []byte("modifié"), 0600)
	replay, e := s.preparationCommand(req)
	if e != nil || len(replay.Sources) != 1 {
		t.Fatal("lost response", e)
	}
	m, _ := s.preparationMethod(p.Method)
	prompt, e := s.preparationPromptForMode(p, m, nil, "Analyse les sources.", "brief", "")
	if e != nil || !strings.Contains(prompt, "donnée utile") {
		t.Fatal(prompt, e)
	}
	req = prepRequest(p, "source-add")
	req.Source = &PreparationSourceRef{Path: source.Path, Start: 1, End: 2, Hash: source.Hash}
	if _, e = s.preparationCommand(req); e == nil {
		t.Fatal("stale selection accepted")
	}
	req = prepRequest(p, "source-remove")
	req.Source = &PreparationSourceRef{Path: source.Path}
	p, e = s.preparationCommand(req)
	if e != nil || len(p.Sources) != 0 {
		t.Fatal(p, e)
	}
}
func TestPreparationSourcesRejectSecretsSymlinksAndBinary(t *testing.T) {
	s := storeTest(t)
	for name, text := range map[string]string{".env": "secret=hidden", "creds.json": "{\"api_key\":\"hidden\"}", "binary.txt": "a\x00b", "README.md": "texte autorisé"} {
		os.WriteFile(filepath.Join(s.root, name), []byte(text), 0600)
	}
	os.Symlink(filepath.Join(s.root, "README.md"), filepath.Join(s.root, "link.md"))
	for _, name := range []string{"../outside.md", "/etc/passwd", ".env", "creds.json", "binary.txt", "link.md", ".swarm/providers.json"} {
		if _, e := s.preparationReadSource(name, 1, 10); e == nil {
			t.Error("unsafe source", name)
		}
	}
	files, e := s.preparationSourceFiles("", "")
	if e != nil {
		t.Fatal(e)
	}
	for _, file := range files.Paths {
		if file == ".env" || file == "link.md" || strings.HasPrefix(file, ".swarm/") {
			t.Fatal(files)
		}
	}
}

func TestPreparationResourcesREPLConfirmation(t *testing.T) {
	s := storeTest(t)
	prepMethods(t, s)
	p := prepCreate(t, s)
	os.WriteFile(filepath.Join(s.root, "README.md"), []byte("Texte du projet"), 0600)
	var out bytes.Buffer
	terminal := preparationTerminal{s: s, p: p, out: &out, width: 80}
	for _, command := range []string{"/fichiers README", "/joindre README.md 1 1"} {
		if _, e := terminal.line(command, false); e != nil {
			t.Fatal(e)
		}
	}
	current, _ := s.preparation(p.ID)
	if len(current.Sources) != 0 {
		t.Fatal("source added before confirmation")
	}
	if _, e := terminal.line("/confirmer", false); e != nil {
		t.Fatal(e)
	}
	if _, e := terminal.line("/budget 2 1 2026-09-16 Forfait de recette", false); e != nil {
		t.Fatal(e)
	}
	if _, e := terminal.line("/confirmer", false); e != nil {
		t.Fatal(e)
	}
	current, _ = s.preparation(p.ID)
	if len(current.Sources) != 1 || current.Budget == nil || current.Budget.Limit != 2 {
		t.Fatal(current)
	}
}
