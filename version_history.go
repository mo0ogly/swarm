package main

import (
	"bytes"
	_ "embed"
	"encoding/json"
	"fmt"
	"io"
	"net/url"
	"strings"
	"time"
	"unicode"
)

const (
	maxVersionHistoryBytes    = 1 << 20
	maxVersionHistoryReleases = 20
	maxReleaseCommits         = 50
	maxReleaseSummaryBytes    = 8 << 10
)

//go:embed version_history.json
var embeddedVersionHistory []byte

type LocalizedSummary struct {
	FR string `json:"fr"`
	EN string `json:"en"`
}

type VersionCommit struct {
	SHA string `json:"sha"`
	URL string `json:"url"`
}

type VersionRelease struct {
	Version    string           `json:"version"`
	Date       string           `json:"date"`
	Summary    LocalizedSummary `json:"summary"`
	Commits    []VersionCommit  `json:"commits"`
	ReleaseURL string           `json:"release_url"`
}

type versionHistoryManifest struct {
	Schema   int              `json:"schema_version"`
	Releases []VersionRelease `json:"releases"`
}

type VersionHistory struct {
	Schema    int              `json:"schema_version"`
	Available bool             `json:"available"`
	Releases  []VersionRelease `json:"releases"`
}

func loadVersionHistory() VersionHistory {
	manifest, err := parseVersionHistory(embeddedVersionHistory)
	if err != nil {
		return VersionHistory{Schema: 1, Available: false, Releases: []VersionRelease{}}
	}
	return VersionHistory{Schema: manifest.Schema, Available: true, Releases: manifest.Releases}
}

func parseVersionHistory(raw []byte) (versionHistoryManifest, error) {
	var history versionHistoryManifest
	if len(raw) > maxVersionHistoryBytes {
		return history, fmt.Errorf("version history exceeds %d bytes", maxVersionHistoryBytes)
	}
	decoder := json.NewDecoder(bytes.NewReader(raw))
	decoder.DisallowUnknownFields()
	if err := decoder.Decode(&history); err != nil {
		return history, fmt.Errorf("decode version history: %w", err)
	}
	var extra any
	if err := decoder.Decode(&extra); err != io.EOF {
		return history, fmt.Errorf("version history must contain one JSON document")
	}
	if history.Schema != 1 {
		return history, fmt.Errorf("unsupported version history schema %d", history.Schema)
	}
	if history.Releases == nil {
		return history, fmt.Errorf("version history releases must be an array")
	}
	if len(history.Releases) > maxVersionHistoryReleases {
		return history, fmt.Errorf("version history exceeds %d releases", maxVersionHistoryReleases)
	}
	seen := map[string]bool{}
	for i := range history.Releases {
		if err := validateVersionRelease(history.Releases[i], seen); err != nil {
			return history, fmt.Errorf("release %d: %w", i, err)
		}
		seen[history.Releases[i].Version] = true
	}
	return history, nil
}

func validateVersionRelease(release VersionRelease, seen map[string]bool) error {
	if !validReleaseVersion(release.Version) || seen[release.Version] {
		return fmt.Errorf("invalid or duplicate version")
	}
	if parsed, err := time.Parse("2006-01-02", release.Date); err != nil || parsed.Format("2006-01-02") != release.Date {
		return fmt.Errorf("invalid release date")
	}
	if err := validateSummary(release.Summary.FR); err != nil {
		return fmt.Errorf("invalid French summary: %w", err)
	}
	if err := validateSummary(release.Summary.EN); err != nil {
		return fmt.Errorf("invalid English summary: %w", err)
	}
	if len(release.Commits) > maxReleaseCommits {
		return fmt.Errorf("release exceeds %d commits", maxReleaseCommits)
	}
	repository, err := githubReleaseRepository(release.ReleaseURL, release.Version)
	if err != nil {
		return err
	}
	for _, commit := range release.Commits {
		if !commitPattern.MatchString(commit.SHA) {
			return fmt.Errorf("invalid commit SHA")
		}
		if !validGitHubPath(commit.URL, repository+"/commit/"+commit.SHA) {
			return fmt.Errorf("invalid commit URL")
		}
	}
	return nil
}

func validateSummary(summary string) error {
	if summary == "" || len([]byte(summary)) > maxReleaseSummaryBytes {
		return fmt.Errorf("summary is empty or too large")
	}
	if strings.IndexFunc(summary, unicode.IsControl) >= 0 {
		return fmt.Errorf("summary contains control characters")
	}
	return nil
}

func githubReleaseRepository(raw, version string) (string, error) {
	u, err := url.Parse(raw)
	if err != nil || u.Scheme != "https" || u.Host != "github.com" || u.RawQuery != "" || u.Fragment != "" || u.User != nil {
		return "", fmt.Errorf("invalid release URL")
	}
	suffix := "/releases/tag/" + version
	if !strings.HasSuffix(u.EscapedPath(), suffix) {
		return "", fmt.Errorf("release URL does not match version")
	}
	repository := strings.TrimSuffix(u.EscapedPath(), suffix)
	if len(strings.Split(strings.Trim(repository, "/"), "/")) != 2 {
		return "", fmt.Errorf("invalid GitHub repository path")
	}
	return repository, nil
}

func validGitHubPath(raw, expectedPath string) bool {
	u, err := url.Parse(raw)
	return err == nil && u.Scheme == "https" && u.Host == "github.com" && u.EscapedPath() == expectedPath && u.RawQuery == "" && u.Fragment == "" && u.User == nil
}
