package engine

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
)

type ProjectProfileChoice struct {
	ID        string         `json:"id"`
	Title     string         `json:"title"`
	Available bool           `json:"available"`
	Reason    string         `json:"reason,omitempty"`
	Profile   ProjectProfile `json:"profile"`
}
type ProjectProfileCatalog struct {
	ActiveSHA256 string                 `json:"active_sha256"`
	ActiveName   string                 `json:"active_name"`
	ActiveSource string                 `json:"active_source"`
	Choices      []ProjectProfileChoice `json:"choices"`
}

// Only allow-listed instruction documents are suggested. No settings, keys,
// external directory or native provider permissions are imported.
func (s *Store) projectProfileCatalog() (ProjectProfileCatalog, error) {
	out := ProjectProfileCatalog{Choices: []ProjectProfileChoice{}}
	name := filepath.Base(s.root)
	if raw, err := boundedProjectFile(s.root, projectProfileFile, 16000); err == nil {
		out.ActiveSHA256 = hash(raw)
		var p ProjectProfile
		if json.Unmarshal(raw, &p) == nil && p.Name != "" {
			name = p.Name
			out.ActiveName = name
			out.ActiveSource = p.Source
		}
	} else {
		if _, statErr := os.Lstat(filepath.Join(s.root, projectProfileFile)); !os.IsNotExist(statErr) {
			return out, err
		}
	}
	for _, candidate := range []struct{ id, title, path string }{
		{"claude-project", ".claude — consignes Claude du projet", ".claude/CLAUDE.md"},
		{"claude-root", "CLAUDE.md — consignes Claude", "CLAUDE.md"},
		{"agents-root", "AGENTS.md — consignes partagées", "AGENTS.md"},
		{"agents-directory", ".agents — consignes partagées", ".agents/AGENTS.md"},
		{"gemini-root", "GEMINI.md — consignes Gemini", "GEMINI.md"},
		{"gemini-directory", ".gemini — consignes Gemini", ".gemini/GEMINI.md"},
	} {
		choice := ProjectProfileChoice{ID: candidate.id, Title: candidate.title, Profile: ProjectProfile{Version: 1, Name: name, Source: candidate.id}}
		shared := false
		if _, err := localFile(s.root, "AGENTS.md"); err == nil {
			choice.Profile.Instructions = append(choice.Profile.Instructions, ProjectInstruction{"AGENTS.md", append([]string{}, projectRoles...)})
			shared = true
		}
		if _, err := localFile(s.root, candidate.path); err != nil {
			choice.Reason = "Fichier de consignes absent du projet."
			out.Choices = append(out.Choices, choice)
			continue
		}
		if candidate.path != "AGENTS.md" {
			roles := append([]string{}, projectRoles...)
			if shared {
				roles = []string{"worker"}
			}
			choice.Profile.Instructions = append(choice.Profile.Instructions, ProjectInstruction{candidate.path, roles})
		}
		if _, err := localFile(s.root, ".claude/field-guide/index.md"); err == nil {
			choice.Profile.Instructions = append(choice.Profile.Instructions, ProjectInstruction{".claude/field-guide/index.md", append([]string{}, projectRoles...)})
		}
		if err := s.validateProjectProfileSources(choice.Profile); err != nil {
			choice.Reason = err.Error()
		} else {
			choice.Available = true
		}
		out.Choices = append(out.Choices, choice)
	}
	return out, nil
}

func (s *Store) selectProjectProfile(id, expected string) (ProjectProfileStatus, error) {
	catalog, err := s.projectProfileCatalog()
	if err != nil {
		return ProjectProfileStatus{}, err
	}
	for _, choice := range catalog.Choices {
		if choice.ID != id {
			continue
		}
		if !choice.Available {
			return ProjectProfileStatus{}, fmt.Errorf("profil indisponible : %s", choice.Reason)
		}
		if err = s.applyProjectProfile(choice.Profile, expected); err != nil {
			return ProjectProfileStatus{}, err
		}
		return s.projectProfileStatus()
	}
	return ProjectProfileStatus{}, fmt.Errorf("profil de projet inconnu")
}
