package engine

import (
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
)

// Locate canonical fixtures without changing the working directory of provider helpers.
func repositoryRoot(t *testing.T) string {
	t.Helper()
	cwd, err := os.Getwd()
	if err != nil {
		t.Fatal(err)
	}
	_, source, _, ok := runtime.Caller(0)
	starts := []string{cwd}
	if ok && filepath.IsAbs(source) {
		starts = append(starts, filepath.Dir(source))
	}
	for _, start := range starts {
		for dir := start; ; dir = filepath.Dir(dir) {
			raw, err := os.ReadFile(filepath.Join(dir, "go.mod"))
			if err == nil && strings.Contains(string(raw), "module swarm.local/companion") {
				return dir
			}
			if filepath.Dir(dir) == dir {
				break
			}
		}
	}
	t.Fatal("Swarm repository root not found for source fixture")
	return ""
}

func repositoryCommand(t *testing.T, name string, args ...string) *exec.Cmd {
	t.Helper()
	cmd := exec.Command(name, args...)
	cmd.Dir = repositoryRoot(t)
	return cmd
}
