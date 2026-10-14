//go:build linux

package engine

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"syscall"
	"testing"
	"time"
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
	InputParts      []graphDeliveryD03Input       `json:"input_parts,omitempty"`
}

func graphDeliveryD03ReadInputs(root string, m *graphDeliveryD03ManifestValue) error {
	if len(m.InputParts) == 0 {
		return nil
	}
	if len(m.Inputs) != 0 {
		return errors.New("ambiguous flat and partitioned source inputs")
	}
	seen := map[string]bool{}
	var inputs []graphDeliveryD03Input
	for _, part := range m.InputParts {
		clean := filepath.Clean(filepath.FromSlash(part.Path))
		if part.Path == "" || filepath.IsAbs(clean) || clean == "." || clean == ".." || strings.HasPrefix(clean, ".."+string(filepath.Separator)) || seen[clean] {
			return errors.New("unsafe or duplicate source input part")
		}
		seen[clean] = true
		raw, err := graphDeliveryD03ReadFile(root, clean)
		if err != nil {
			return err
		}
		if graphDeliveryD03Hash(raw) != part.SHA256 {
			return errors.New("source input part digest mismatch")
		}
		var rows []graphDeliveryD03Input
		if err = json.Unmarshal(raw, &rows); err != nil {
			return err
		}
		if len(rows) == 0 {
			return errors.New("empty source input part")
		}
		inputs = append(inputs, rows...)
	}
	m.Inputs = inputs
	return nil
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
		if !allowedKinds[input.Kind] || seenPaths[clean] {
			return fmt.Errorf("invalid or duplicate input: %s", input.Path)
		}
		raw, err := graphDeliveryD03ReadFile(root, clean)
		if err != nil {
			return fmt.Errorf("missing input %s: %w", input.Path, err)
		}
		if graphDeliveryD03Hash(raw) != input.SHA256 {
			return fmt.Errorf("stale input: %s", input.Path)
		}
		seenPaths[clean] = true
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

	// UI redesign records current source integrity without renewing historical acceptance.
	uiManifest := "docs/ui-redesign-source-manifest.json"
	if _, e := os.Stat(filepath.Join(root, uiManifest)); e == nil {
		manifestPath = uiManifest
	} else if !os.IsNotExist(e) {
		t.Fatal(e)
	}

	// Engine evolution records a distinct current source snapshot. Earlier
	// manifests remain immutable; this never renews their semantic acceptance.
	engineManifest := "docs/engine-cursor-source-manifest.json"
	if _, e := os.Stat(filepath.Join(root, engineManifest)); e == nil {
		manifestPath = engineManifest
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
	if err = graphDeliveryD03ReadInputs(root, &manifest); err != nil {
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

func TestGraphDeliverySourcePartsPreserveIntegrity(t *testing.T) {
	root := t.TempDir()
	rows := []graphDeliveryD03Input{{Path: "source.go", Kind: "source", SHA256: strings.Repeat("a", 64)}}
	raw, _ := json.Marshal(rows)
	if err := os.WriteFile(filepath.Join(root, "part.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	original := graphDeliveryD03ManifestValue{InputParts: []graphDeliveryD03Input{{Path: "part.json", SHA256: graphDeliveryD03Hash(raw)}}}
	m := original
	if err := graphDeliveryD03ReadInputs(root, &m); err != nil || len(m.Inputs) != 1 || graphDeliveryD03CandidateDigest(m.Inputs) != graphDeliveryD03CandidateDigest(rows) {
		t.Fatal("partition changed source identity", err)
	}
	for _, kind := range []string{"tampered", "escape", "duplicate", "alias-dot", "alias-parent", "ambiguous", "missing"} {
		t.Run(kind, func(t *testing.T) {
			m := original
			m.InputParts = append([]graphDeliveryD03Input(nil), original.InputParts...)
			switch kind {
			case "tampered":
				m.InputParts[0].SHA256 = strings.Repeat("0", 64)
			case "escape":
				m.InputParts[0].Path = "../part.json"
			case "duplicate":
				m.InputParts = append(m.InputParts, m.InputParts[0])
			case "alias-dot":
				m.InputParts = append(m.InputParts, graphDeliveryD03Input{Path: "./part.json", SHA256: m.InputParts[0].SHA256})
			case "alias-parent":
				m.InputParts = append(m.InputParts, graphDeliveryD03Input{Path: "sub/../part.json", SHA256: m.InputParts[0].SHA256})
			case "ambiguous":
				m.Inputs = rows
			case "missing":
				m.InputParts[0].Path = "missing.json"
			}
			if err := graphDeliveryD03ReadInputs(root, &m); err == nil {
				t.Fatal("invalid source part accepted", kind)
			}
		})
	}
}

func TestGraphDeliverySourcePartsRejectSymlinkEscape(t *testing.T) {
	for _, kind := range []string{"parent", "leaf"} {
		t.Run(kind, func(t *testing.T) {
			root, outside := t.TempDir(), t.TempDir()
			raw, _ := json.Marshal([]graphDeliveryD03Input{{Path: "source.go", Kind: "source", SHA256: strings.Repeat("a", 64)}})
			if err := os.WriteFile(filepath.Join(outside, "part.json"), raw, 0600); err != nil {
				t.Fatal(err)
			}
			name, target := "escape", outside
			if kind == "leaf" {
				name, target = "part.json", filepath.Join(outside, "part.json")
			}
			if err := os.Symlink(target, filepath.Join(root, name)); err != nil {
				t.Fatal(err)
			}
			path := "escape/part.json"
			if kind == "leaf" {
				path = "part.json"
			}
			m := graphDeliveryD03ManifestValue{InputParts: []graphDeliveryD03Input{{Path: path, SHA256: graphDeliveryD03Hash(raw)}}}
			if err := graphDeliveryD03ReadInputs(root, &m); err == nil || len(m.Inputs) != 0 {
				t.Fatal("outside source part accepted", err)
			}
		})
	}
}

// Go's rooted handle prevents symlink traversal outside the fixture even if
// paths change between validation and opening. The root itself may be canonicalized.
func graphDeliveryD03ReadFile(root, name string) ([]byte, error) {
	dir, err := os.OpenRoot(root)
	if err != nil {
		return nil, err
	}
	defer dir.Close()
	// Nonblocking open also rejects FIFOs without waiting for an external writer.
	file, err := dir.OpenFile(name, os.O_RDONLY|syscall.O_NONBLOCK, 0)
	if err != nil {
		return nil, err
	}
	defer file.Close()
	info, err := file.Stat()
	if err != nil {
		return nil, err
	}
	if !info.Mode().IsRegular() {
		return nil, errors.New("source input is not a regular file")
	}
	return io.ReadAll(file)
}

func TestGraphDeliveryFlatInputsRejectSymlinkEscape(t *testing.T) {
	root, outside := t.TempDir(), t.TempDir()
	raw := []byte("external original")
	if err := os.WriteFile(filepath.Join(outside, "source.go"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	if err := os.Symlink(outside, filepath.Join(root, "escape")); err != nil {
		t.Fatal(err)
	}
	m := graphDeliveryD03ManifestValue{SchemaVersion: 1, ManifestStatus: "fixture", BaseCommit: strings.Repeat("a", 40), Inputs: []graphDeliveryD03Input{{Path: "escape/source.go", Kind: "source", SHA256: graphDeliveryD03Hash(raw)}}}
	for i := 1; i <= 18; i++ {
		m.Requirements = append(m.Requirements, graphDeliveryD03Requirement{ID: fmt.Sprintf("R%02d", i), Status: "fixture", Evidence: []string{"fixture"}})
	}
	m.CoverageDigest = graphDeliveryD03CoverageDigest(m.Requirements)
	err := graphDeliveryD03Validate(root, m)
	var pathError *os.PathError
	if err == nil || !errors.As(err, &pathError) {
		t.Fatal("external source read before containment rejection", err)
	}
	if _, err = graphDeliveryD03ReadFile(root, "escape/source.go"); err == nil {
		t.Fatal("shared rooted reader escaped")
	}
	if err = os.WriteFile(filepath.Join(root, "local.go"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	local, err := graphDeliveryD03ReadFile(root, "local.go")
	if err != nil || string(local) != string(raw) {
		t.Fatal("local source refused", err)
	}
	if _, err = graphDeliveryD03ReadFile(root, "."); err == nil {
		t.Fatal("directory accepted as source")
	}
}

func TestGraphDeliverySourceRejectsFIFOWithoutBlocking(t *testing.T) {
	root := t.TempDir()
	name := filepath.Join(root, "fifo")
	if err := syscall.Mkfifo(name, 0600); err != nil {
		t.Fatal(err)
	}
	done := make(chan error, 1)
	go func() { _, err := graphDeliveryD03ReadFile(root, "fifo"); done <- err }()
	select {
	case err := <-done:
		if err == nil {
			t.Fatal("FIFO accepted")
		}
	case <-time.After(time.Second):
		// Release a defective blocking reader before failing, leaving no goroutine.
		writer, err := syscall.Open(name, syscall.O_WRONLY|syscall.O_NONBLOCK, 0)
		if err == nil {
			syscall.Close(writer)
		}
		<-done
		t.Fatal("non-regular source blocked before rejection")
	}
}

func TestGraphDeliveryFlatInputsRejectCanonicalAliases(t *testing.T) {
	root, original := graphDeliveryD03Load(t)
	// Build a current integrity fixture; never renew a historical acceptance.
	for i := range original.Inputs {
		raw, err := graphDeliveryD03ReadFile(root, original.Inputs[i].Path)
		if err != nil {
			t.Fatal(err)
		}
		original.Inputs[i].SHA256 = graphDeliveryD03Hash(raw)
	}
	for _, prefix := range []string{"./", "docs/../"} {
		t.Run(prefix, func(t *testing.T) {
			m := original
			m.Inputs = append([]graphDeliveryD03Input(nil), original.Inputs...)
			alias := m.Inputs[0]
			alias.Path = prefix + alias.Path
			m.Inputs = append(m.Inputs, alias)
			m.CandidateDigest = graphDeliveryD03CandidateDigest(m.Inputs)
			m.CandidateID = m.BaseCommit + "+sha256:" + m.CandidateDigest
			if err := graphDeliveryD03Validate(root, m); err == nil || !strings.Contains(err.Error(), "duplicate") {
				t.Fatal("canonical alias not rejected as duplicate", err)
			}
		})
	}
}
