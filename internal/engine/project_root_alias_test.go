//go:build linux

package engine

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"net/url"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func aliasedFixtureRoot(t *testing.T, root string) string {
	t.Helper()
	alias := filepath.Join(t.TempDir(), "retained-root")
	if err := os.Symlink(root, alias); err != nil {
		t.Fatal(err)
	}
	return alias
}

func TestProjectRootAliasPreservesOwnedBoundaries(t *testing.T) {
	for _, kind := range []string{"file", "artifact", "workspace"} {
		t.Run(kind, func(t *testing.T) {
			s := storeTest(t)
			if err := os.MkdirAll(filepath.Join(s.root, "docs"), 0700); err != nil {
				t.Fatal(err)
			}
			data := []byte("preserved report")
			name := "docs/report.md"
			if err := os.WriteFile(filepath.Join(s.root, name), data, 0600); err != nil {
				t.Fatal(err)
			}
			alias := aliasedFixtureRoot(t, s.root)
			switch kind {
			case "file":
				p, err := localFile(alias, name)
				if err != nil || p != filepath.Join(s.root, name) {
					t.Fatal("owned file refused through retained root", p, err)
				}
			case "artifact":
				s.root = alias
				got, err := s.artifactDigest(ExchangeArtifact{Path: name, SHA256: hash(data)})
				if err != nil || got != hash(data) {
					t.Fatal("owned artifact refused through retained root", got, err)
				}
			case "workspace":
				p, err := resolveWorkspace(alias, "docs")
				if err != nil || p != filepath.Join(s.root, "docs") {
					t.Fatal("owned workspace refused through retained root", p, err)
				}
			}
		})
	}
}

func TestProjectRootAliasPublicReportAndEscapeRefusal(t *testing.T) {
	s := storeTest(t)
	original := s.root
	if err := os.MkdirAll(filepath.Join(original, "docs"), 0700); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(original, "docs/report.md"), []byte("preserved report"), 0600); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(original, "private.txt"), []byte("private bytes"), 0600); err != nil {
		t.Fatal(err)
	}
	if err := os.Symlink("../private.txt", filepath.Join(original, "docs/private-link.md")); err != nil {
		t.Fatal(err)
	}
	outside := t.TempDir()
	if err := os.WriteFile(filepath.Join(outside, "outside.md"), []byte("outside bytes"), 0600); err != nil {
		t.Fatal(err)
	}
	if err := os.Symlink(outside, filepath.Join(original, "docs/outside")); err != nil {
		t.Fatal(err)
	}
	s.root = aliasedFixtureRoot(t, original)
	h := newWebHandler(s, "local.test", "token")
	for _, entry := range []struct {
		path   string
		status int
	}{{"docs/report.md", 200}, {"docs/private-link.md", 403}, {"docs/outside/outside.md", 400}} {
		out := httptest.NewRecorder()
		req := httptest.NewRequest("GET", "http://local.test/api/v1/report?"+url.Values{"path": {entry.path}}.Encode(), nil)
		req.AddCookie(&http.Cookie{Name: "swarm_session", Value: "token"})
		h.ServeHTTP(out, req)
		if out.Code != entry.status {
			t.Fatal(entry.path, out.Code, out.Body.String())
		}
		if out.Code == 200 {
			var data map[string]string
			if json.Unmarshal(out.Body.Bytes(), &data) != nil || data["text"] != "preserved report" {
				t.Fatal("wrong report", out.Body.String())
			}
		} else if strings.Contains(out.Body.String(), "private bytes") || strings.Contains(out.Body.String(), "outside bytes") {
			t.Fatal("escaped report leaked", out.Body.String())
		}
	}
	if _, err := resolveWorkspace(s.root, "docs/outside"); err == nil {
		t.Fatal("external workspace admitted through alias")
	}
	if _, err := s.artifactDigest(ExchangeArtifact{Path: "docs/outside/outside.md", SHA256: hash([]byte("outside bytes"))}); err == nil {
		t.Fatal("external artifact admitted through alias")
	}
}
