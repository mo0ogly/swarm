//go:build linux

package engine

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestIndependentDeliveryDocumentsBoundedAndFresh(t *testing.T) {
	s := storeTest(t)
	os.MkdirAll(filepath.Join(s.root, "docs"), 0700)
	path := filepath.Join(s.root, "docs/deliverable.md")
	os.WriteFile(path, []byte("Declared delivery evidence"), 0600)
	task := Task{Deliverable: "docs/deliverable.md"}
	docs, digests, err := s.independentDeliveryDocuments(&task, "docs/handoff.md")
	if err != nil || docs[task.Deliverable] != "Declared delivery evidence" || len(digests) != 1 {
		t.Fatal(docs, err)
	}
	if err = s.currentReportArtifacts(digests); err != nil {
		t.Fatal(err)
	}
	os.WriteFile(path, []byte("changed delivery"), 0600)
	if s.currentReportArtifacts(digests) == nil {
		t.Fatal("changed deliverable remained current")
	}
	os.WriteFile(path, []byte(strings.Repeat("x", 48001)), 0600)
	if _, _, err = s.independentDeliveryDocuments(&task, "docs/handoff.md"); err == nil {
		t.Fatal("oversized document accepted")
	}
	task.Deliverable = "docs/missing.md"
	docs, _, err = s.independentDeliveryDocuments(&task, "docs/handoff.md")
	if err != nil || len(docs) != 0 {
		t.Fatal("missing proof should stay absent", err)
	}
	task.Deliverable = "../external.md"
	docs, _, err = s.independentDeliveryDocuments(&task, "docs/handoff.md")
	if err != nil || len(docs) != 0 {
		t.Fatal("non-document contract became a file read", err)
	}
}
