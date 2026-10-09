//go:build linux

package engine

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"testing"
)

const graphDeliveryD03Manifest = "docs/graph-delivery-qualification-manifest.json"

type graphDeliveryD03Input struct {
	Path   string `json:"path"`
	Kind   string `json:"kind"`
	SHA256 string `json:"sha256"`
}

type graphDeliveryD03Requirement struct {
	ID       string   `json:"id"`
	Status   string   `json:"status"`
	Evidence []string `json:"evidence"`
	Limit    string   `json:"limit,omitempty"`
}

type graphDeliveryD03ManifestValue struct {
	SchemaVersion   int                           `json:"schema_version"`
	ManifestStatus  string                        `json:"manifest_status"`
	BaseCommit      string                        `json:"base_commit"`
	CandidateID     string                        `json:"candidate_id"`
	CandidateDigest string                        `json:"candidate_digest"`
	CoverageDigest  string                        `json:"coverage_digest"`
	Requirements    []graphDeliveryD03Requirement `json:"requirements"`
	Inputs          []graphDeliveryD03Input       `json:"inputs"`
}

func graphDeliveryD03Hash(raw []byte) string {
	digest := sha256.Sum256(raw)
	return hex.EncodeToString(digest[:])
}

func graphDeliveryD03CandidateDigest(inputs []graphDeliveryD03Input) string {
	lines := make([]string, 0, len(inputs))
	for _, input := range inputs {
		lines = append(lines, input.SHA256+"  "+input.Path+"\n")
	}
	sort.Strings(lines)
	return graphDeliveryD03Hash([]byte(strings.Join(lines, "")))
}

func graphDeliveryD03CoverageDigest(requirements []graphDeliveryD03Requirement) string {
	lines := make([]string, 0, len(requirements))
	for _, requirement := range requirements {
		evidence := append([]string(nil), requirement.Evidence...)
		sort.Strings(evidence)
		lines = append(lines, requirement.ID+"|"+requirement.Status+"|"+strings.Join(evidence, ",")+"|"+requirement.Limit+"\n")
	}
	sort.Strings(lines)
	return graphDeliveryD03Hash([]byte(strings.Join(lines, "")))
}

func graphDeliveryD03Validate(root string, manifest graphDeliveryD03ManifestValue) error {
	if manifest.SchemaVersion != 1 || manifest.ManifestStatus == "" || len(manifest.BaseCommit) != 40 {
		return errors.New("invalid qualification identity")
	}
	wantedRequirements := make(map[string]bool, 18)
	for i := 1; i <= 18; i++ {
		wantedRequirements[fmt.Sprintf("R%02d", i)] = true
	}
	seenRequirements := make(map[string]bool, 18)
	for _, requirement := range manifest.Requirements {
		if !wantedRequirements[requirement.ID] || seenRequirements[requirement.ID] || requirement.Status == "" || len(requirement.Evidence) == 0 {
			return fmt.Errorf("invalid requirement coverage: %s", requirement.ID)
		}
		seenRequirements[requirement.ID] = true
	}
	if len(seenRequirements) != len(wantedRequirements) || graphDeliveryD03CoverageDigest(manifest.Requirements) != manifest.CoverageDigest {
		return errors.New("incomplete or stale requirement coverage")
	}

	allowedKinds := map[string]bool{"source": true, "test": true, "doc": true, "capture": true, "harness": true}
	seenKinds := make(map[string]bool)
	seenPaths := make(map[string]bool, len(manifest.Inputs))
	for _, input := range manifest.Inputs {
		clean := filepath.Clean(filepath.FromSlash(input.Path))
		if input.Path == "" || filepath.IsAbs(clean) || clean == "." || clean == ".." || strings.HasPrefix(clean, ".."+string(filepath.Separator)) {
			return fmt.Errorf("unsafe input path: %q", input.Path)
		}
		if !allowedKinds[input.Kind] || seenPaths[input.Path] {
			return fmt.Errorf("invalid or duplicate input: %s", input.Path)
		}
		raw, err := os.ReadFile(filepath.Join(root, clean))
		if err != nil {
			return fmt.Errorf("missing input %s: %w", input.Path, err)
		}
		if graphDeliveryD03Hash(raw) != input.SHA256 {
			return fmt.Errorf("stale input: %s", input.Path)
		}
		seenPaths[input.Path] = true
		seenKinds[input.Kind] = true
	}
	for _, kind := range []string{"source", "test", "doc", "capture", "harness"} {
		if !seenKinds[kind] {
			return fmt.Errorf("missing input kind: %s", kind)
		}
	}
	for _, required := range []string{
		"internal/engine/graph_delivery_e2e_test.go",
		"internal/engine/graph_delivery_migration_test.go",
		"internal/engine/graph_delivery_qualification_test.go",
		"tests/graph_automation_e2e.cjs",
		"tests/graph_delivery_docs.py",
		"docs/D01.md",
		"docs/D01-dossier.md",
		"docs/D02.md",
		"docs/D02-dossier.md",
		"docs/plans/graphe-automatisation-20261005/execution/d/verify.py",
		"docs/screenshots/graph-delivery-d01/fr-etat-graph.png",
		"docs/screenshots/graph-delivery-d02/fr-etat-graph.png",
	} {
		if !seenPaths[required] {
			return fmt.Errorf("required candidate input missing: %s", required)
		}
	}
	if graphDeliveryD03CandidateDigest(manifest.Inputs) != manifest.CandidateDigest {
		return errors.New("candidate digest mismatch")
	}
	if manifest.CandidateID != manifest.BaseCommit+"+sha256:"+manifest.CandidateDigest {
		return errors.New("candidate id mismatch")
	}
	return nil
}

func graphDeliveryD03Load(t *testing.T) (string, graphDeliveryD03ManifestValue) {
	t.Helper()
	root := repositoryRoot(t)
	// A later authorized engine correction has its own manifest; retain the
	// accepted original manifest rather than rewriting historical evidence.
	manifestPath := graphDeliveryD03Manifest
	overlay := "docs/graph-delivery-contract-revision-manifest.json"
	if _, e := os.Stat(filepath.Join(root, overlay)); e == nil {
		manifestPath = overlay
	} else if !os.IsNotExist(e) {
		t.Fatal(e)
	}
	// Release packaging has a separate snapshot; accepted mission manifests stay historical.
	releaseManifest := "docs/graph-delivery-release-manifest.json"
	if _, e := os.Stat(filepath.Join(root, releaseManifest)); e == nil {
		manifestPath = releaseManifest
	} else if !os.IsNotExist(e) {
		t.Fatal(e)
	}
	// Billing recovery has a distinct candidate snapshot; none of the accepted
	// mission or release manifests are rewritten to bless subsequent changes.
	billingManifest := "docs/billing-engine-recovery-manifest.json"
	if _, e := os.Stat(filepath.Join(root, billingManifest)); e == nil {
		manifestPath = billingManifest
	} else if !os.IsNotExist(e) {
		t.Fatal(e)
	}
	// Local server delivery has its own snapshot. Historical acceptance inputs
	// remain intact; current inputs must still pass the same integrity checks.
	localManifest := "docs/local-web-recovery-manifest.json"
	if _, e := os.Stat(filepath.Join(root, localManifest)); e == nil {
		manifestPath = localManifest
	} else if !os.IsNotExist(e) {
		t.Fatal(e)
	}

	// Preparation recovery has a distinct snapshot; preserve prior delivery evidence.
	preparationManifest := "docs/preparation-contract-recovery-manifest.json"
	if _, e := os.Stat(filepath.Join(root, preparationManifest)); e == nil {
		manifestPath = preparationManifest
	} else if !os.IsNotExist(e) {
		t.Fatal(e)
	}

	// AI diagnostics use a new pending candidate snapshot, preserving historical proof.
	aiManifest := "docs/ai-connection-debug-candidate-manifest.json"
	if _, e := os.Stat(filepath.Join(root, aiManifest)); e == nil {
		manifestPath = aiManifest
	} else if !os.IsNotExist(e) {
		t.Fatal(e)
	}

	// Native distribution changes have their own pending snapshot; retain prior evidence.
	launcherManifest := "docs/native-launcher-candidate-manifest.json"
	if _, e := os.Stat(filepath.Join(root, launcherManifest)); e == nil {
		manifestPath = launcherManifest
	} else if !os.IsNotExist(e) {
		t.Fatal(e)
	}

	// Layout integrity covers the current combined source tree, after older snapshots.
	// It is never a renewal of historical acceptance.
	layoutManifest := "docs/repository-organization-manifest.json"
	if _, e := os.Stat(filepath.Join(root, layoutManifest)); e == nil {
		manifestPath = layoutManifest
	} else if !os.IsNotExist(e) {
		t.Fatal(e)
	}

	// Product workflow changes have a new source snapshot; historical acceptance stays unchanged.
	productManifest := "docs/ks-product-candidate-manifest.json"
	if _, e := os.Stat(filepath.Join(root, productManifest)); e == nil {
		manifestPath = productManifest
	} else if !os.IsNotExist(e) {
		t.Fatal(e)
	}

	// Terminology changes use a separate snapshot, preserving earlier evidence.
	namingManifest := "docs/product-workflow-naming-manifest.json"
	if _, e := os.Stat(filepath.Join(root, namingManifest)); e == nil {
		manifestPath = namingManifest
	} else if !os.IsNotExist(e) {
		t.Fatal(e)
	}

	// Training delivery has a separate source snapshot; earlier evidence stays intact.
	trainingManifest := "docs/training-delivery-source-manifest.json"
	if _, e := os.Stat(filepath.Join(root, trainingManifest)); e == nil {
		manifestPath = trainingManifest
	} else if !os.IsNotExist(e) {
		t.Fatal(e)
	}

	raw, err := os.ReadFile(filepath.Join(root, manifestPath))
	if err != nil {
		t.Fatal(err)
	}
	var manifest graphDeliveryD03ManifestValue
	if err = json.Unmarshal(raw, &manifest); err != nil {
		t.Fatal(err)
	}
	return root, manifest
}

func TestGraphDeliveryD03QualificationManifest(t *testing.T) {
	root, manifest := graphDeliveryD03Load(t)
	if err := graphDeliveryD03Validate(root, manifest); err != nil {
		t.Fatal(err)
	}

	missing := manifest
	missing.Inputs = append([]graphDeliveryD03Input(nil), manifest.Inputs...)
	missing.Inputs[0].Path = "missing-d03-candidate-input"
	if err := graphDeliveryD03Validate(root, missing); err == nil || !strings.Contains(err.Error(), "missing input") {
		t.Fatalf("missing candidate input was not rejected: %v", err)
	}

	stale := manifest
	stale.Inputs = append([]graphDeliveryD03Input(nil), manifest.Inputs...)
	stale.Inputs[0].SHA256 = strings.Repeat("0", 64)
	if err := graphDeliveryD03Validate(root, stale); err == nil || !strings.Contains(err.Error(), "stale input") {
		t.Fatalf("stale candidate input was not rejected: %v", err)
	}

	incomplete := manifest
	incomplete.Requirements = append([]graphDeliveryD03Requirement(nil), manifest.Requirements[:len(manifest.Requirements)-1]...)
	if err := graphDeliveryD03Validate(root, incomplete); err == nil || !strings.Contains(err.Error(), "incomplete") {
		t.Fatalf("incomplete coverage was not rejected: %v", err)
	}
}
