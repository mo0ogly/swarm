//go:build linux

package engine

import (
	"fmt"
	"os"
	"strings"
)

// Only the declared Markdown deliverable is attached. Reports do not authorize
// following arbitrary links or scanning the workspace for additional context.
func (s *Store) independentDeliveryDocuments(t *Task, report string) (map[string]string, map[string]string, error) {
	documents, hashes := map[string]string{}, map[string]string{}
	name := t.Deliverable
	if name == report || !strings.HasPrefix(name, "docs/") || !strings.HasSuffix(name, ".md") {
		return documents, hashes, nil
	}
	path, err := safeReport(s.root, name)
	if os.IsNotExist(err) {
		return documents, hashes, nil
	}
	if err != nil {
		return nil, nil, err
	}
	info, err := os.Stat(path)
	if err != nil {
		return nil, nil, err
	}
	if info.Size() > 48000 {
		return nil, nil, fmt.Errorf("livrable documentaire supérieur à 48 Ko ; aucune troncature pour la vérification")
	}
	raw, err := os.ReadFile(path)
	if err != nil {
		return nil, nil, err
	}
	if len(raw) > 48000 {
		return nil, nil, fmt.Errorf("livrable documentaire supérieur à 48 Ko ; aucune troncature pour la vérification")
	}
	documents[name] = string(raw)
	hashes[name] = hash(raw)
	return documents, hashes, nil
}

func (s *Store) currentReportArtifacts(artifacts map[string]string) error {
	for name, digest := range artifacts {
		path, err := safeReport(s.root, name)
		if err != nil {
			return fmt.Errorf("preuve documentaire inaccessible : %s", name)
		}
		raw, err := os.ReadFile(path)
		if err != nil || hash(raw) != digest {
			return fmt.Errorf("vérification IA périmée : livrable modifié : %s", name)
		}
	}
	return nil
}
