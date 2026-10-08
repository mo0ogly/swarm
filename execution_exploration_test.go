//go:build linux

package main

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// Execute the exact shell examples injected by the engine, in an isolated
// workspace. A successful probe is not an acceptance check.
func TestExecutionExplorationDirectives(t *testing.T) {
	if _, err := exec.LookPath("rg"); err != nil {
		t.Skip("rg unavailable")
	}
	limits, _ := (RunLimits{}).normalized()
	prompt := executionDirectives("unused", t.TempDir(), "e4-reprises", limits)
	extract := func(start, end string) string {
		t.Helper()
		_, rest, ok := strings.Cut(prompt, start)
		if !ok {
			t.Fatal("missing directive", start)
		}
		text, _, ok := strings.Cut(rest, end)
		if !ok {
			t.Fatal("missing terminator", end)
		}
		return text
	}
	read := extract("Tester son existence avant lecture : ", ". Ne jamais")
	search := extract("« erreur » : ", ". Adapter")
	root := t.TempDir()
	run := func(command string) (string, error) {
		cmd := exec.Command("sh", "-c", command)
		cmd.Dir = root
		b, e := cmd.CombinedOutput()
		return string(b), e
	}
	if out, err := run(read); err != nil || !strings.Contains(out, "non encore créé") {
		t.Fatal(out, err)
	}
	if err := os.Mkdir(filepath.Join(root, "docs"), 0700); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(root, "docs/e4-reprises.md"), []byte("report proof"), 0600); err != nil {
		t.Fatal(err)
	}
	if out, err := run(read); err != nil || out != "report proof" {
		t.Fatal(out, err)
	}
	if out, err := run(search); err != nil || !strings.Contains(out, "Aucun fichier") {
		t.Fatal(out, err)
	}
	if err := os.WriteFile(filepath.Join(root, "one_review.go"), []byte("package sample"), 0600); err != nil {
		t.Fatal(err)
	}
	if out, err := run(search); err != nil || !strings.Contains(out, "one_review.go") {
		t.Fatal(out, err)
	}
	// Invalid rg flags must stay a genuine failure; no blanket exit-code masking.
	bad := strings.Replace(search, "rg --files", "rg --swarm-invalid-option", 1)
	if out, err := run(bad); err == nil {
		t.Fatal("search error masked", out)
	}
	if out, err := run("exit 7"); err == nil {
		t.Fatal("control failure masked", out)
	}
}

func TestExplorationDoesNotRelaxLoopGuard(t *testing.T) {
	limits, _ := (RunLimits{}).normalized()
	g := newLoopGuard(limits)
	for _, id := range []string{"a", "b", "c"} {
		g.call(id, "Bash", map[string]any{"command": "go test ./" + id}, time.Now())
		g.result(id, true, "test failed")
	}
	if g.errors != 3 || g.reason != "Limite d'erreurs d'outils consécutives atteinte" {
		t.Fatal("real failures no longer bounded", g.errors, g.reason)
	}
}
