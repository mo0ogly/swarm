//go:build linux

package engine

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

const managedReviewSourceBytesExceeded = "sources de contexte supérieures à 128 Kio ; aucun contenu tronqué"
const managedReviewSourceCountExceeded = "contexte de revue : 24 fichiers maximum, aucun contenu tronqué"

const reviewManifestTransportBytes = 8192
const defaultReviewContextFiles = 24
const defaultReviewSourceBytes = 96 * 1024
const defaultReviewContextBytes = 128 * 1024

type reviewSourcePolicy struct{ Files, SourceBytes, TotalBytes int }

func effectiveReviewSourcePolicy(cfg *ReviewerConfig) reviewSourcePolicy {
	p := reviewSourcePolicy{defaultReviewContextFiles, defaultReviewSourceBytes, defaultReviewContextBytes}
	if cfg != nil {
		if cfg.MaxContextFiles != nil {
			p.Files = *cfg.MaxContextFiles
		}
		if cfg.MaxSourceBytes != nil {
			p.SourceBytes = *cfg.MaxSourceBytes
		}
		if cfg.MaxContextBytes != nil {
			p.TotalBytes = *cfg.MaxContextBytes
		}
	}
	return p
}

func reviewSourceCapacityReason(reason string) bool {
	return reason == managedReviewSourceCountExceeded || reason == managedReviewSourceBytesExceeded ||
		strings.HasPrefix(reason, "source de contexte ") && strings.Contains(reason, "source de revue trop grande : ") && strings.HasSuffix(reason, " ; aucun contenu tronqué") ||
		strings.HasPrefix(reason, "contexte de revue : ") && strings.HasSuffix(reason, " fichiers maximum, aucun contenu tronqué") ||
		strings.HasPrefix(reason, "sources de contexte supérieures à ") && strings.HasSuffix(reason, " octets ; aucun contenu tronqué")
}

type managedReviewSourceCapacityError struct{ reason string }

func (e *managedReviewSourceCapacityError) Error() string { return e.reason }

func managedReviewSourceCapacityRefusal(err error) bool {
	var capacity *managedReviewSourceCapacityError
	return errors.As(err, &capacity)
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
	if err != nil || size < 0 || limit < 0 {
		return source, fmt.Errorf("taille de source de revue invalide : %s", name)
	}
	if limit > 0 && size > limit {
		return source, &managedReviewSourceCapacityError{fmt.Sprintf("source de revue trop grande : %s ; aucun contenu tronqué", name)}
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
	policy := effectiveReviewSourcePolicy(w.Planning.Reviewer)
	if policy.Files < 0 || policy.SourceBytes < 0 || policy.TotalBytes < 0 {
		return nil, fmt.Errorf("limite de contexte de revue négative")
	}
	bare := filepath.Join(repo.Storage, "repository.git")
	seen := map[string]bool{}
	sources := []ReviewSource{}
	total := 0
	for _, task := range tasks {
		// A delivery evidence path is an explicit producer declaration. Resolve
		// it to candidate bytes, never to a quotation or a mutable checkout.
		files := []string{}
		manifestName := path.Join(repo.Subdir, "docs", task.Task+".review-context.json")
		manifest, err := candidateReviewSource(bare, candidate, manifestName, reviewManifestTransportBytes)
		if err != nil && !errors.Is(err, os.ErrNotExist) {
			return nil, err
		}
		if err == nil {
			var request struct {
				Version int      `json:"version"`
				Files   []string `json:"files"`
			}
			if err = strict([]byte(manifest.Content), &request); err != nil || request.Version != 1 || len(request.Files) == 0 {
				return nil, fmt.Errorf("manifeste de contexte de revue invalide : %s", manifestName)
			}
			files = append(files, request.Files...)
		}
		if task.Delivery != nil {
			for _, criterion := range task.Delivery.Criteria {
				for _, name := range criterion.Evidence {
					// The task report is already supplied in full in Tasks.Report.
					if name != task.Binding.Report {
						files = append(files, name)
					}
				}
			}
		}
		for _, name := range files {
			if seen[name] {
				continue
			}
			if policy.Files > 0 && len(sources) >= policy.Files {
				reason := managedReviewSourceCountExceeded
				if policy.Files != defaultReviewContextFiles {
					reason = fmt.Sprintf("contexte de revue : %d fichiers maximum, aucun contenu tronqué", policy.Files)
				}
				return nil, &managedReviewSourceCapacityError{reason}
			}
			source, err := candidateReviewSource(bare, candidate, name, policy.SourceBytes)
			if err != nil {
				return nil, fmt.Errorf("source de contexte %s : %w", name, err)
			}
			total += source.Bytes
			if policy.TotalBytes > 0 && total > policy.TotalBytes {
				reason := managedReviewSourceBytesExceeded
				if policy.TotalBytes != defaultReviewContextBytes {
					reason = fmt.Sprintf("sources de contexte supérieures à %d octets ; aucun contenu tronqué", policy.TotalBytes)
				}
				return nil, &managedReviewSourceCapacityError{reason}
			}
			seen[name] = true
			sources = append(sources, source)
		}
	}
	return sources, nil
}

// Each task uses the persisted source policy. Cumulative reviews are partitioned
// later into provider-capacity checked prompts; unrelated task packets are not
// summed against a per-task policy. No missing/invalid source is silently omitted.
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
			seen[source.Path] = true
			sources = append(sources, source)
		}
	}
	return sources, nil
}
