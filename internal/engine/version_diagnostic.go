package engine

import (
	"encoding/json"
	"fmt"
	"net"
	"net/http"
	"net/http/cookiejar"
	"net/url"
	"strings"
	"time"
)

const (
	versionStateIdentical = "identical"
	versionStateDivergent = "divergent"
	versionStateUnknown   = "unknown"
)

type VersionObservation struct {
	Available  bool    `json:"available"`
	Version    *string `json:"version"`
	Commit     *string `json:"commit"`
	Modified   *bool   `json:"modified"`
	Provenance string  `json:"provenance"`
	Hint       string  `json:"hint,omitempty"`
}

type VersionDifference struct {
	Left  string `json:"left"`
	Right string `json:"right"`
}

type VersionDiagnostic struct {
	InstalledCLI VersionObservation  `json:"installed_cli"`
	ActiveServer VersionObservation  `json:"active_server"`
	Candidate    VersionObservation  `json:"candidate"`
	State        string              `json:"state"`
	Differences  []VersionDifference `json:"differences"`
	NextStep     string              `json:"next_step,omitempty"`
}

func binaryObservation(v BinaryVersion) VersionObservation {
	version := v.Version
	return VersionObservation{Available: true, Version: &version, Commit: v.Commit, Modified: v.Modified, Provenance: v.Provenance}
}

func candidateObservation(root string) VersionObservation {
	state := gitState(root)
	if state.Head == "" {
		return VersionObservation{Available: false, Provenance: "git", Hint: "Vérifiez --root et que le candidat est une copie Git lisible."}
	}
	commit, modified := state.Head, state.Changes != ""
	return VersionObservation{Available: true, Commit: &commit, Modified: &modified, Provenance: "git"}
}

func compareVersions(installed, server, candidate VersionObservation) VersionDiagnostic {
	d := VersionDiagnostic{InstalledCLI: installed, ActiveServer: server, Candidate: candidate, Differences: []VersionDifference{}}
	roles := []struct {
		name string
		v    VersionObservation
	}{{"installed_cli", installed}, {"active_server", server}, {"candidate", candidate}}
	missing := []string{}
	for _, role := range roles {
		if !role.v.Available || role.v.Commit == nil || !commitPattern.MatchString(*role.v.Commit) {
			missing = append(missing, role.name)
		}
	}
	if len(missing) > 0 {
		d.State = versionStateUnknown
		d.NextStep = "Rendez accessibles les identités manquantes (" + strings.Join(missing, ", ") + ") puis relancez le diagnostic. Pour le serveur actif, fournissez son URL de session avec --server."
		return d
	}
	for i := 0; i < len(roles); i++ {
		for j := i + 1; j < len(roles); j++ {
			if *roles[i].v.Commit != *roles[j].v.Commit {
				d.Differences = append(d.Differences, VersionDifference{Left: roles[i].name, Right: roles[j].name})
			}
		}
	}
	if len(d.Differences) == 0 {
		d.State = versionStateIdentical
		return d
	}
	d.State = versionStateDivergent
	d.NextStep = "Installez ou redémarrez le composant divergent avec le candidat voulu, puis relancez le diagnostic."
	return d
}

func serverObservation(rawURL string) (VersionObservation, error) {
	parsed, err := url.Parse(rawURL)
	if err != nil || parsed.Scheme != "http" || parsed.Hostname() == "" {
		return VersionObservation{}, fmt.Errorf("--server attend une URL HTTP locale de session")
	}
	ip := net.ParseIP(parsed.Hostname())
	if parsed.Hostname() != "localhost" && (ip == nil || !ip.IsLoopback()) {
		return VersionObservation{}, fmt.Errorf("--server doit désigner le serveur local")
	}
	jar, _ := cookiejar.New(nil)
	client := &http.Client{Jar: jar, Timeout: 5 * time.Second}
	if parsed.Path != "" && parsed.Path != "/" {
		response, err := client.Get(parsed.String())
		if err != nil {
			return VersionObservation{}, fmt.Errorf("serveur actif inaccessible : %w", err)
		}
		response.Body.Close()
		if response.StatusCode >= 400 {
			return VersionObservation{}, fmt.Errorf("URL de session refusée : HTTP %d", response.StatusCode)
		}
	}
	endpoint := *parsed
	endpoint.Path, endpoint.RawPath, endpoint.RawQuery, endpoint.Fragment = "/api/v1/runtime-health", "", "", ""
	response, err := client.Get(endpoint.String())
	if err != nil {
		return VersionObservation{}, fmt.Errorf("serveur actif inaccessible : %w", err)
	}
	defer response.Body.Close()
	if response.StatusCode != http.StatusOK {
		return VersionObservation{}, fmt.Errorf("diagnostic du serveur actif refusé : HTTP %d", response.StatusCode)
	}
	var health RuntimeHealth
	decoder := json.NewDecoder(response.Body)
	if err := decoder.Decode(&health); err != nil {
		return VersionObservation{}, fmt.Errorf("diagnostic du serveur actif invalide : %w", err)
	}
	return binaryObservation(health.Version.Binary), nil
}

func versionDiagnosticText(d VersionDiagnostic) string {
	show := func(v VersionObservation) string {
		if !v.Available {
			return "inconnue"
		}
		parts := []string{}
		if v.Version != nil {
			parts = append(parts, *v.Version)
		}
		if v.Commit != nil {
			parts = append(parts, *v.Commit)
		}
		if len(parts) == 0 {
			return "inconnue"
		}
		return strings.Join(parts, " · ")
	}
	var b strings.Builder
	fmt.Fprintf(&b, "CLI installée : %s\nServeur actif : %s\nCandidat : %s\nÉtat : %s\n", show(d.InstalledCLI), show(d.ActiveServer), show(d.Candidate), d.State)
	for _, difference := range d.Differences {
		fmt.Fprintf(&b, "Divergence : %s / %s\n", difference.Left, difference.Right)
	}
	if d.NextStep != "" {
		fmt.Fprintf(&b, "Action : %s\n", d.NextStep)
	}
	return b.String()
}
