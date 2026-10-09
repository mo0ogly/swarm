//go:build linux

package engine

import (
	"encoding/json"
	"golang.org/x/sys/unix"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestPreparationBundledMethodsInEmptyProject(t *testing.T) {
	s := storeTest(t)
	p := prepCreate(t, s)
	h := newWebHandler(s, "127.0.0.1:9876", "bundled-test")
	r := httptest.NewRequest("GET", "http://127.0.0.1:9876/api/v1/preparations/methods", nil)
	r.AddCookie(&http.Cookie{Name: "swarm_session", Value: "bundled-test"})
	w := httptest.NewRecorder()
	h.ServeHTTP(w, r)
	var methods []PreparationMethod
	if err := json.Unmarshal(w.Body.Bytes(), &methods); err != nil || w.Code != 200 || len(methods) != 5 {
		t.Fatalf("catalogue: %d %s %v", w.Code, w.Body.String(), err)
	}
	for _, m := range methods {
		if !m.Available || m.Hash == "" {
			t.Fatalf("fresh installation cannot use %s: %s", m.ID, m.Reason)
		}
		prompt, err := s.preparationPromptFor(p, m, nil, "Prepare a scoped project.", "brief")
		if err != nil {
			t.Fatal(err)
		}
		var payload struct {
			Sources map[string]string `json:"method_sources"`
		}
		if err = json.Unmarshal([]byte(prompt[strings.LastIndex(prompt, "\n{")+1:]), &payload); err != nil {
			t.Fatal(err)
		}
		for _, path := range m.Paths {
			want, err := os.ReadFile(filepath.Join(repositoryRoot(t), path))
			if err != nil || payload.Sources[path] != string(want) {
				t.Fatalf("missing or truncated bundled source %s: %v", path, err)
			}
			if _, err = os.Lstat(filepath.Join(s.root, path)); !os.IsNotExist(err) {
				t.Fatalf("reading a bundled method wrote project files: %s %v", path, err)
			}
		}
	}
}

func TestPreparationBundledMethodsKeepOverridesAndRejectStaleContext(t *testing.T) {
	s := storeTest(t)
	p := prepCreate(t, s)
	bundled, err := s.preparationMethod("apex")
	if err != nil {
		t.Fatal(err)
	}
	path := filepath.Join(s.root, ".claude/skills/apex/SKILL.md")
	if err = os.MkdirAll(filepath.Dir(path), 0700); err != nil {
		t.Fatal(err)
	}
	if err = os.WriteFile(path, []byte("Project-specific planning guidance."), 0600); err != nil {
		t.Fatal(err)
	}
	local, err := s.preparationMethod("apex")
	if err != nil || local.Hash == bundled.Hash {
		t.Fatal("override not fingerprinted", err)
	}
	if _, err = s.preparationPrompt(p, bundled, nil, "Continue."); err == nil {
		t.Fatal("stale bundled context accepted")
	}
	prompt, err := s.preparationPrompt(p, local, nil, "Continue.")
	if err != nil || !strings.Contains(prompt, "Project-specific planning guidance.") {
		t.Fatal("override omitted", err)
	}
	if err = os.Remove(path); err != nil {
		t.Fatal(err)
	}
	restored, err := s.preparationMethod("apex")
	if err != nil || restored.Hash != bundled.Hash {
		t.Fatal("bundle not restored after override removal", err)
	}
	if _, err = s.preparationPrompt(p, local, nil, "Continue."); err == nil {
		t.Fatal("removed override context accepted")
	}
}

func TestPreparationBundledMethodsRejectUnsafeOverrides(t *testing.T) {
	for _, kind := range []string{"leaf-link", "parent-link", "dangling-link", "directory", "fifo", "oversize", "binary", "empty"} {
		t.Run(kind, func(t *testing.T) {
			s := storeTest(t)
			path := filepath.Join(s.root, ".claude/skills/apex/SKILL.md")
			outside := filepath.Join(t.TempDir(), "method.md")
			if err := os.WriteFile(outside, []byte("Outside-project data"), 0600); err != nil {
				t.Fatal(err)
			}
			var err error
			if kind == "parent-link" {
				err = os.Symlink(filepath.Dir(outside), filepath.Join(s.root, ".claude"))
			} else {
				if err = os.MkdirAll(filepath.Dir(path), 0700); err != nil {
					t.Fatal(err)
				}
				switch kind {
				case "leaf-link":
					err = os.Symlink(outside, path)
				case "dangling-link":
					err = os.Symlink(outside+"-missing", path)
				case "directory":
					err = os.Mkdir(path, 0700)
				case "fifo":
					err = unix.Mkfifo(path, 0600)
				case "oversize":
					err = os.WriteFile(path, []byte(strings.Repeat("x", 131073)), 0600)
				case "binary":
					err = os.WriteFile(path, []byte("text\x00hidden"), 0600)
				case "empty":
					err = os.WriteFile(path, nil, 0600)
				}
			}
			if err != nil {
				t.Fatal(err)
			}
			m, err := s.preparationMethod("apex")
			if err == nil || m.Available || m.Hash != "" || m.Reason == "" {
				t.Fatalf("unsafe override silently replaced: %+v %v", m, err)
			}
			if _, err = s.preparationPrompt(prepCreate(t, s), m, nil, "Read."); err == nil {
				t.Fatal("unsafe source transmitted")
			}
		})
	}
}
