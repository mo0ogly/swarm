//go:build linux

package engine

import (
	"errors"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

func TestManagedReviewGitPreservesOriginalBoundaryBytes(t *testing.T) {
	root := t.TempDir()
	gitTest(t, root, "init")
	originals := []string{"", "\n\tOriginal report with tabs\t \n\n", "é界\r\nLast line\t \r\n", "\t  \n"}
	for i, text := range originals {
		name := string(rune('a'+i)) + ".txt"
		if err := os.WriteFile(filepath.Join(root, name), []byte(text), 0600); err != nil {
			t.Fatal(err)
		}
	}
	gitTest(t, root, "add", ".")
	gitTest(t, root, "commit", "-m", "original byte fixtures")
	for i, original := range originals {
		name := string(rune('a'+i)) + ".txt"
		actual, err := managedReviewGit(2, root, "show", "HEAD:"+name)
		if err != nil || actual != original {
			t.Fatal("current review modified original boundary bytes", name, err)
		}
		for _, version := range []int{0, 1} {
			legacy, err := managedReviewGit(version, root, "show", "HEAD:"+name)
			if err != nil || legacy != strings.TrimSpace(original) {
				t.Fatal("historical normalization changed", version, name, err)
			}
		}
	}
	t.Setenv("GIT_TRACE", "1")
	actual, err := managedReviewGit(2, root, "show", "HEAD:b.txt")
	if err != nil || actual != originals[1] {
		t.Fatal("Git stderr polluted original stdout", err)
	}
	_, err = managedReviewGit(2, root, "show", "HEAD:missing.txt")
	var failure *exec.ExitError
	if err == nil || !errors.As(err, &failure) {
		t.Fatal("Git failure cause was discarded", err)
	}
}
