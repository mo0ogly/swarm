//go:build linux

package main

import (
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path"
	"path/filepath"
	"strconv"
	"strings"
	"unicode/utf8"
)

type ReviewSource struct {
	Path    string `json:"path"`
	Blob    string `json:"git_blob"`
	Digest  string `json:"sha256"`
	Bytes   int    `json:"bytes"`
	Content string `json:"content"`
}

// Read regular text blobs from the immutable tested candidate, never from a
// producer checkout or arbitrary report path. There is no silent truncation.
func candidateReviewSource(bare, candidate, name string, limit int) (ReviewSource, error) {
	var source ReviewSource
	if name == "" || len(name) > 240 || path.IsAbs(name) || path.Clean(name) != name || name == "." || strings.HasPrefix(name, "../") || strings.ContainsAny(name, "\x00\r\n\\:") || strings.HasPrefix(name, ".git/") || name == ".git" {
		return source, fmt.Errorf("chemin de source de revue invalide")
	}
	entry, err := managedGit(bare, "ls-tree", "-z", candidate, "--", ":(literal)"+name)
	if err != nil {
		return source, err
	}
	if entry == "" {
		return source, os.ErrNotExist
	}
	parts := strings.SplitN(strings.TrimSuffix(entry, "\x00"), "\t", 2)
	if len(parts) != 2 || parts[1] != name {
		return source, fmt.Errorf("entrée Git de revue ambiguë")
	}
	meta := strings.Fields(parts[0])
	if len(meta) != 3 || (meta[0] != "100644" && meta[0] != "100755") || meta[1] != "blob" {
		return source, fmt.Errorf("source de revue non régulière : %s", name)
	}
	sizeText, err := managedGit(bare, "cat-file", "-s", meta[2])
	if err != nil {
		return source, err
	}
	size, err := strconv.Atoi(sizeText)
	if err != nil || size < 0 || size > limit {
		return source, fmt.Errorf("source de revue trop grande : %s ; aucun contenu tronqué", name)
	}
	content, err := exec.Command("git", "--git-dir", bare, "cat-file", "blob", meta[2]).Output()
	if err != nil || len(content) != size {
		return source, fmt.Errorf("source de revue illisible : %s", name)
	}
	if !utf8.Valid(content) || strings.ContainsRune(string(content), '\x00') {
		return source, fmt.Errorf("source de revue non textuelle : %s", name)
	}
	return ReviewSource{Path: name, Blob: meta[2], Digest: hash(content), Bytes: len(content), Content: string(content)}, nil
}

func managedReviewSources(w Work, tasks []managedReviewTaskContext, candidate string) ([]ReviewSource, error) {
	repo := w.Planning.Repository
	bare := filepath.Join(repo.Storage, "repository.git")
	seen := map[string]bool{}
	sources := []ReviewSource{}
	total := 0
	for _, task := range tasks {
		manifestName := path.Join(repo.Subdir, "docs", task.Task+".review-context.json")
		manifest, err := candidateReviewSource(bare, candidate, manifestName, 8192)
		if errors.Is(err, os.ErrNotExist) {
			continue // Existing tasks still use their diff, report and receipt.
		}
		if err != nil {
			return nil, err
		}
		var request struct {
			Version int      `json:"version"`
			Files   []string `json:"files"`
		}
		if err = strict([]byte(manifest.Content), &request); err != nil || request.Version != 1 || len(request.Files) == 0 || len(request.Files) > 24 {
			return nil, fmt.Errorf("manifeste de contexte de revue invalide : %s", manifestName)
		}
		for _, name := range request.Files {
			if seen[name] {
				continue
			}
			if len(sources) >= 24 {
				return nil, fmt.Errorf("contexte de revue : 24 fichiers maximum, aucun contenu tronqué")
			}
			source, err := candidateReviewSource(bare, candidate, name, 96*1024)
			if err != nil {
				return nil, fmt.Errorf("source de contexte %s : %w", name, err)
			}
			total += source.Bytes
			if total > 128*1024 {
				return nil, fmt.Errorf("sources de contexte supérieures à 128 Kio ; aucun contenu tronqué")
			}
			seen[name] = true
			sources = append(sources, source)
		}
	}
	return sources, nil
}

// Each task keeps the original 128 KiB source bound. Cumulative reviews are
// partitioned later into bounded prompts; do not sum unrelated task packets
// against the single-task limit. The global 24-file bound still applies.
func managedReviewSourcesByTask(w Work, tasks []managedReviewTaskContext, candidate string) ([]ReviewSource, error) {
	sources := []ReviewSource{}
	seen := map[string]bool{}
	for _, task := range tasks {
		packet, e := managedReviewSources(w, []managedReviewTaskContext{task}, candidate)
		if e != nil {
			return nil, e
		}
		for _, source := range packet {
			if seen[source.Path] {
				continue
			}
			if len(sources) >= 24 {
				return nil, fmt.Errorf("contexte de revue : 24 fichiers maximum, aucun contenu tronqué")
			}
			seen[source.Path] = true
			sources = append(sources, source)
		}
	}
	return sources, nil
}
