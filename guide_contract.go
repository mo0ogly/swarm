package main

import (
	"encoding/json"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"unicode/utf8"
)

// Same budgets and local-link rules as tools/agent-workflows/check_guide.py.
// The digest covers every existing note, so a concurrent note update invalidates review.
func validateGuide(root, dir string, pending map[string]string) (string, error) {
	texts := map[string]string{}
	original := map[string]string{}
	e := filepath.WalkDir(dir, func(path string, d fs.DirEntry, e error) error {
		if e != nil {
			return e
		}
		if d.Type()&os.ModeSymlink != 0 {
			return fmt.Errorf("Guide : lien symbolique refusé")
		}
		if d.IsDir() {
			return nil
		}
		if filepath.Ext(path) != ".md" {
			return fmt.Errorf("Guide : seules les notes Markdown sont acceptées")
		}
		rel, _ := filepath.Rel(dir, path)
		b, e := os.ReadFile(path)
		if e != nil {
			return e
		}
		texts[rel] = string(b)
		original[rel] = hash(b)
		return nil
	})
	if e != nil {
		return "", e
	}
	for name, body := range pending {
		texts[name] = body
	}
	total := 0
	links := regexp.MustCompile(`\[[^\]]*\]\(([^)]+)\)`)
	for name, body := range texts {
		if !utf8.ValidString(body) {
			return "", fmt.Errorf("Guide : UTF-8 invalide")
		}
		lines := 0
		if body != "" {
			lines = len(strings.Split(strings.TrimSuffix(body, "\n"), "\n"))
		}
		limit := 200
		if name == "index.md" {
			limit = 120
		}
		if lines > limit {
			return "", fmt.Errorf("Guide : %s dépasse %d lignes", name, limit)
		}
		total += lines
		for _, match := range links.FindAllStringSubmatch(body, -1) {
			link := match[1]
			if strings.Contains(link, "://") || strings.HasPrefix(link, "#") {
				continue
			}
			target := filepath.Clean(filepath.Join(dir, filepath.Dir(name), strings.Split(link, "#")[0]))
			if rel, e := filepath.Rel(dir, target); e == nil {
				if _, ok := pending[rel]; ok {
					continue
				}
			}
			actual, e := filepath.EvalSymlinks(target)
			if e != nil {
				return "", fmt.Errorf("Guide : lien local absent : %s", link)
			}
			rel, e := filepath.Rel(root, actual)
			if e != nil || rel == ".." || strings.HasPrefix(rel, ".."+string(filepath.Separator)) {
				return "", fmt.Errorf("Guide : lien hors projet")
			}
			st, e := os.Stat(actual)
			if e != nil || !st.Mode().IsRegular() {
				return "", fmt.Errorf("Guide : fichier lié invalide")
			}
		}
	}
	if total > 1200 {
		return "", fmt.Errorf("Guide : budget total de 1200 lignes dépassé")
	}
	b, _ := json.Marshal(original)
	return hash(b), nil
}
