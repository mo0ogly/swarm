package engine

import (
	"fmt"
	"regexp"
	"runtime/debug"
	"strings"
	"time"
)

// These values are set by the supported build recipes. Keep safe degraded
// defaults so a plain `go build` remains truthful and useful outside a checkout.
var (
	buildVersion    = "devel"
	buildCommit     = ""
	buildModified   = "unknown"
	buildDate       = ""
	buildProvenance = ""
)

var (
	releaseVersionPattern = regexp.MustCompile(`^v(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$`)
	commitPattern         = regexp.MustCompile(`^[0-9a-f]{40}$`)
)

type BinaryVersion struct {
	Version    string  `json:"version"`
	Commit     *string `json:"commit"`
	Modified   *bool   `json:"modified"`
	BuildDate  *string `json:"build_date"`
	Provenance string  `json:"provenance"`
}

type versionResponse struct {
	Schema     int               `json:"schema_version"`
	Binary     BinaryVersion     `json:"binary"`
	Diagnostic VersionDiagnostic `json:"diagnostic"`
}

func validReleaseVersion(v string) bool {
	if !releaseVersionPattern.MatchString(v) {
		return false
	}
	withoutBuild := strings.SplitN(v, "+", 2)[0]
	parts := strings.SplitN(withoutBuild, "-", 2)
	if len(parts) == 1 {
		return true
	}
	for _, identifier := range strings.Split(parts[1], ".") {
		if len(identifier) > 1 && identifier[0] == '0' {
			numeric := true
			for _, r := range identifier {
				if r < '0' || r > '9' {
					numeric = false
					break
				}
			}
			if numeric {
				return false
			}
		}
	}
	return true
}

func binaryVersion() BinaryVersion {
	v := BinaryVersion{Version: "devel", Provenance: "unknown"}
	if buildVersion == "devel" || validReleaseVersion(buildVersion) {
		v.Version = buildVersion
	}
	if buildProvenance == "injected" {
		v.Provenance = "injected"
		if commitPattern.MatchString(buildCommit) {
			commit := buildCommit
			v.Commit = &commit
		}
		if modified, ok := parseModified(buildModified); ok {
			v.Modified = &modified
		}
		if parsed, err := time.Parse(time.RFC3339, buildDate); err == nil {
			date := parsed.UTC().Format(time.RFC3339)
			v.BuildDate = &date
		}
		return v
	}

	info, ok := debug.ReadBuildInfo()
	if !ok {
		return v
	}
	for _, setting := range info.Settings {
		switch setting.Key {
		case "vcs.revision":
			if commitPattern.MatchString(setting.Value) {
				commit := setting.Value
				v.Commit = &commit
			}
		case "vcs.modified":
			if modified, valid := parseModified(setting.Value); valid {
				v.Modified = &modified
			}
		}
	}
	if v.Commit != nil || v.Modified != nil {
		v.Provenance = "go_build_info"
	}
	return v
}

func parseModified(raw string) (bool, bool) {
	switch raw {
	case "true":
		return true, true
	case "false":
		return false, true
	default:
		return false, false
	}
}

func versionText(v BinaryVersion) string {
	commit := "unknown"
	if v.Commit != nil {
		commit = *v.Commit
	}
	modified := "unknown"
	if v.Modified != nil {
		modified = fmt.Sprintf("%t", *v.Modified)
	}
	built := "unknown"
	if v.BuildDate != nil {
		built = *v.BuildDate
	}
	return fmt.Sprintf("Swarm %s (commit %s; modified=%s; built=%s; provenance=%s)\n", strings.TrimSpace(v.Version), commit, modified, built, v.Provenance)
}
