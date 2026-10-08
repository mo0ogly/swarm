package main

import (
	"bytes"
	"encoding/json"
	"errors"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

func setBuildMetadataForTest(t *testing.T, version, commit, modified, date, provenance string) {
	t.Helper()
	oldVersion, oldCommit, oldModified, oldDate, oldProvenance := buildVersion, buildCommit, buildModified, buildDate, buildProvenance
	buildVersion, buildCommit, buildModified, buildDate, buildProvenance = version, commit, modified, date, provenance
	t.Cleanup(func() {
		buildVersion, buildCommit, buildModified, buildDate, buildProvenance = oldVersion, oldCommit, oldModified, oldDate, oldProvenance
	})
}

func TestVersionCLIIsStableAndDoesNotOpenStorage(t *testing.T) {
	commit := strings.Repeat("a", 40)
	setBuildMetadataForTest(t, "v1.2.3", commit, "false", "2026-10-02T18:00:00+02:00", "injected")
	root := t.TempDir()
	if err := os.Mkdir(filepath.Join(root, ".swarm"), 0700); err != nil {
		t.Fatal(err)
	}
	database := filepath.Join(root, ".swarm", "state.db")
	if err := os.WriteFile(database, []byte("not sqlite"), 0600); err != nil {
		t.Fatal(err)
	}

	for _, args := range [][]string{{"--root", root, "--json", "version"}, {"--json", "--version", "--root", root}} {
		var out, stderr bytes.Buffer
		if code := run(args, &out, &stderr); code != 0 {
			t.Fatalf("run(%q)=%d stderr=%q", args, code, stderr.String())
		}
		var response versionResponse
		if err := json.Unmarshal(out.Bytes(), &response); err != nil {
			t.Fatalf("invalid JSON %q: %v", out.String(), err)
		}
		if response.Schema != 1 || response.Binary.Version != "v1.2.3" || response.Binary.Commit == nil || *response.Binary.Commit != commit || response.Binary.Modified == nil || *response.Binary.Modified || response.Binary.BuildDate == nil || *response.Binary.BuildDate != "2026-10-02T16:00:00Z" || response.Binary.Provenance != "injected" {
			t.Fatalf("unexpected response: %+v", response)
		}
	}
	raw, err := os.ReadFile(database)
	if err != nil || string(raw) != "not sqlite" {
		t.Fatalf("version touched storage: %q, %v", raw, err)
	}

	var out, stderr bytes.Buffer
	if code := run([]string{"--root", filepath.Join(root, "missing"), "--version"}, &out, &stderr); code != 0 {
		t.Fatalf("--version resolved an invalid root: %d %q", code, stderr.String())
	}
	want := "Swarm v1.2.3 (commit " + commit + "; modified=false; built=2026-10-02T16:00:00Z; provenance=injected)\n"
	if !strings.HasPrefix(out.String(), want) || !strings.Contains(out.String(), "CLI installée : v1.2.3 · "+commit) || !strings.Contains(out.String(), "Serveur actif : inconnue") || !strings.Contains(out.String(), "État : unknown") {
		t.Fatalf("text output=%q", out.String())
	}
}

func TestVersionDiagnosticDistinguishesThreeValuesAndMissingValue(t *testing.T) {
	commit := func(r byte) *string { value := strings.Repeat(string(r), 40); return &value }
	observed := func(r byte) VersionObservation {
		return VersionObservation{Available: true, Commit: commit(r), Provenance: "test"}
	}
	diagnostic := compareVersions(observed('a'), observed('b'), observed('c'))
	if diagnostic.State != versionStateDivergent || len(diagnostic.Differences) != 3 {
		t.Fatalf("three distinct values not diagnosed: %+v", diagnostic)
	}
	text := versionDiagnosticText(diagnostic)
	for _, expected := range []string{"CLI installée : " + strings.Repeat("a", 40), "Serveur actif : " + strings.Repeat("b", 40), "Candidat : " + strings.Repeat("c", 40), "État : divergent", "Divergence : installed_cli / active_server"} {
		if !strings.Contains(text, expected) {
			t.Fatalf("public CLI rendering misses %q in %q", expected, text)
		}
	}
	if diagnostic.Differences[0] != (VersionDifference{Left: "installed_cli", Right: "active_server"}) {
		t.Fatalf("divergent elements not named: %+v", diagnostic.Differences)
	}
	missing := compareVersions(observed('a'), VersionObservation{Available: false}, observed('a'))
	if missing.State != versionStateUnknown || len(missing.Differences) != 0 || !strings.Contains(missing.NextStep, "active_server") || !strings.Contains(missing.NextStep, "--server") {
		t.Fatalf("missing value was not actionable unknown: %+v", missing)
	}
	equal := compareVersions(observed('a'), observed('a'), observed('a'))
	if equal.State != versionStateIdentical || len(equal.Differences) != 0 {
		t.Fatalf("equal values not diagnosed: %+v", equal)
	}
}

func TestVersionRejectsExtraArgumentsBeforeStorage(t *testing.T) {
	var out, stderr bytes.Buffer
	if code := run([]string{"version", "extra"}, &out, &stderr); code != 2 || out.Len() != 0 || !strings.Contains(stderr.String(), "swarm version") {
		t.Fatalf("code=%d stdout=%q stderr=%q", code, out.String(), stderr.String())
	}
}

func TestBinaryVersionDegradesInvalidInjectedValues(t *testing.T) {
	setBuildMetadataForTest(t, "v1.not-semver", "not-a-sha", "maybe", "yesterday", "injected")
	v := binaryVersion()
	if v.Version != "devel" || v.Commit != nil || v.Modified != nil || v.BuildDate != nil || v.Provenance != "injected" {
		t.Fatalf("invalid build values were trusted: %+v", v)
	}
}

func TestReleaseBuildRequiresCompleteIdentity(t *testing.T) {
	baseEnv := append(os.Environ(),
		"SWARM_VERSION=v1.2.3",
		"SWARM_COMMIT="+strings.Repeat("a", 40),
		"SWARM_MODIFIED=false",
		"SWARM_BUILD_DATE=2026-10-02T18:00:00Z",
	)
	cases := []struct {
		name     string
		override string
		want     string
	}{
		{name: "semantic version", override: "SWARM_VERSION=v1.2.3-alpha..beta", want: "invalid SWARM_VERSION"},
		{name: "commit", override: "SWARM_COMMIT=", want: "SWARM_COMMIT is required"},
		{name: "modified", override: "SWARM_MODIFIED=unknown", want: "SWARM_MODIFIED is required"},
		{name: "build date", override: "SWARM_BUILD_DATE=", want: "SWARM_BUILD_DATE is required"},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			cmd := exec.Command("sh", "./build.sh", filepath.Join(t.TempDir(), "swarm"))
			cmd.Env = append(baseEnv, tc.override)
			output, err := cmd.CombinedOutput()
			if err == nil {
				t.Fatalf("release build with invalid %s succeeded", tc.name)
			}
			var exitErr *exec.ExitError
			if !errors.As(err, &exitErr) || exitErr.ExitCode() != 2 || !strings.Contains(string(output), tc.want) {
				t.Fatalf("missing %s: err=%v output=%q", tc.name, err, output)
			}
		})
	}
}

func TestEmbeddedVersionHistoryMatchesPublishedReleaseManifest(t *testing.T) {
	history := loadVersionHistory()
	if history.Schema != 1 || !history.Available || history.Releases == nil {
		t.Fatalf("unexpected embedded history: %+v", history)
	}
	seen := map[string]bool{}
	for _, release := range history.Releases {
		if err := validateVersionRelease(release, seen); err != nil {
			t.Fatalf("invalid embedded release: %v", err)
		}
		seen[release.Version] = true
	}
}

func TestVersionHistoryAcceptsOnlyBoundedCoherentGitHubData(t *testing.T) {
	sha := strings.Repeat("b", 40)
	valid := `{"schema_version":1,"releases":[{"version":"v1.2.3","date":"2026-10-02","summary":{"fr":"Résumé sûr","en":"Safe summary"},"commits":[{"sha":"` + sha + `","url":"https://github.com/acme/swarm/commit/` + sha + `"}],"release_url":"https://github.com/acme/swarm/releases/tag/v1.2.3"}]}`
	history, err := parseVersionHistory([]byte(valid))
	if err != nil || len(history.Releases) != 1 {
		t.Fatalf("valid history rejected: %+v %v", history, err)
	}

	unsafeCases := []string{
		strings.Replace(valid, "Safe summary", "unsafe\\u0000summary", 1),
		strings.Replace(valid, "https://github.com/acme/swarm/commit/", "javascript:alert(1)#", 1),
		strings.Replace(valid, sha, "short", 1),
		strings.Replace(valid, `"releases"`, `"unknown":true,"releases"`, 1),
		valid + `{}`,
	}
	for i, raw := range unsafeCases {
		if _, err := parseVersionHistory([]byte(raw)); err == nil {
			t.Fatalf("unsafe history case %d accepted", i)
		}
	}

	releases := make([]VersionRelease, maxVersionHistoryReleases+1)
	for i := range releases {
		releases[i] = VersionRelease{Version: "v1.2.3"}
	}
	raw, err := json.Marshal(versionHistoryManifest{Schema: 1, Releases: releases})
	if err != nil {
		t.Fatal(err)
	}
	if _, err := parseVersionHistory(raw); err == nil || !strings.Contains(err.Error(), "releases") {
		t.Fatalf("release bound not enforced: %v", err)
	}

	if _, err := parseVersionHistory(make([]byte, maxVersionHistoryBytes+1)); err == nil || !strings.Contains(err.Error(), "bytes") {
		t.Fatalf("document size bound not enforced: %v", err)
	}
	history.Releases[0].Summary.EN = strings.Repeat("x", maxReleaseSummaryBytes+1)
	raw, err = json.Marshal(history)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := parseVersionHistory(raw); err == nil || !strings.Contains(err.Error(), "summary") {
		t.Fatalf("summary size bound not enforced: %v", err)
	}
	history.Releases[0].Summary.EN = "Safe summary"
	history.Releases[0].Commits = make([]VersionCommit, maxReleaseCommits+1)
	for i := range history.Releases[0].Commits {
		history.Releases[0].Commits[i] = VersionCommit{SHA: sha, URL: "https://github.com/acme/swarm/commit/" + sha}
	}
	raw, err = json.Marshal(history)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := parseVersionHistory(raw); err == nil || !strings.Contains(err.Error(), "commits") {
		t.Fatalf("commit bound not enforced: %v", err)
	}
}

func TestServerVersionJSONSeparatesBinarySourceAndHistoryWithoutGit(t *testing.T) {
	setBuildMetadataForTest(t, "devel", "", "unknown", "", "injected")
	t.Setenv("PATH", "")
	v := (&Store{root: t.TempDir()}).serverVersion()
	if v.Sources.Available || v.Sources.Compared || v.Sources.MatchesBinary != nil {
		t.Fatalf("source availability was guessed without git: %+v", v.Sources)
	}
	raw, err := json.Marshal(v)
	if err != nil {
		t.Fatal(err)
	}
	text := string(raw)
	for _, key := range []string{`"binary"`, `"source"`, `"history"`, `"matches_binary":null`} {
		if !strings.Contains(text, key) {
			t.Fatalf("missing %s in %s", key, text)
		}
	}
	for _, legacy := range []string{`"revision"`, `"source_revision"`, `"current"`} {
		if strings.Contains(text, legacy) {
			t.Fatalf("legacy field %s leaked in %s", legacy, text)
		}
	}
}
