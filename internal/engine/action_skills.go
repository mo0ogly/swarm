package engine

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

// Action skills are explicitly selected project guidance, never permissions.
type ActionSkillSelection struct {
	Path   string `json:"path"`
	SHA256 string `json:"sha256"`
}
type ActionSkill struct {
	Path        string `json:"path"`
	SHA256      string `json:"sha256,omitempty"`
	Name        string `json:"name"`
	Description string `json:"description"`
	Bytes       int    `json:"bytes"`
	Available   bool   `json:"available"`
	Reason      string `json:"reason,omitempty"`
}

func skillPath(path string) bool {
	parts := strings.Split(path, "/")
	return len(parts) == 4 && (parts[0] == ".claude" || parts[0] == ".agents") && parts[1] == "skills" && parts[2] != "" && parts[2] != "." && parts[2] != ".." && parts[3] == "SKILL.md"
}

func (s *Store) readActionSkill(path string) (ActionSkill, []byte, error) {
	item := ActionSkill{Path: path, Name: filepath.Base(filepath.Dir(path))}
	if !skillPath(path) {
		return item, nil, fmt.Errorf("chemin de skill du projet non conforme : %s", path)
	}
	raw, err := boundedProjectFile(s.root, path, 16000)
	if err != nil {
		return item, nil, err
	}
	if projectSecret.Match(raw) {
		return item, nil, fmt.Errorf("secret détecté dans le skill : %s", path)
	}
	// Metadata is display-only; never interpret YAML tags or execute skill files.
	text := strings.ReplaceAll(string(raw), "\r\n", "\n")
	if strings.HasPrefix(text, "---\n") {
		for _, line := range strings.Split(strings.SplitN(text[4:], "\n---", 2)[0], "\n") {
			key, value, ok := strings.Cut(line, ":")
			if !ok {
				continue
			}
			value = strings.Trim(strings.TrimSpace(value), "\"'")
			if key == "name" && len(value) > 0 && len(value) <= 100 {
				item.Name = value
			}
			if key == "description" && len(value) <= 1000 && value != "|" && value != ">" {
				item.Description = value
			}
		}
	}
	item.Bytes = len(raw)
	item.SHA256 = hash(raw)
	item.Available = true
	return item, raw, nil
}

func (s *Store) actionSkillCatalog() ([]ActionSkill, error) {
	items := []ActionSkill{}
	canonical := map[string]bool{}
	for _, base := range []string{".claude/skills", ".agents/skills"} {
		// Resolve the directory itself within the controlled root, including aliases.
		dir, err := filepath.EvalSymlinks(filepath.Join(s.root, base))
		if os.IsNotExist(err) {
			continue
		}
		if err != nil {
			return nil, err
		}
		rel, err := filepath.Rel(s.root, dir)
		if err != nil || rel == ".." || strings.HasPrefix(rel, ".."+string(filepath.Separator)) {
			return nil, fmt.Errorf("répertoire de skills hors projet : %s", base)
		}
		entries, err := os.ReadDir(dir)
		if err != nil {
			return nil, err
		}
		if len(entries) > 256 {
			return nil, fmt.Errorf("catalogue de skills supérieur à 256 entrées : %s", base)
		}
		for _, entry := range entries {
			path := base + "/" + entry.Name() + "/SKILL.md"
			// Normal files alongside the skill directories are not catalog entries.
			if !entry.IsDir() && entry.Type()&os.ModeSymlink == 0 {
				continue
			}
			item, _, err := s.readActionSkill(path)
			if err != nil {
				item.Reason = err.Error()
			}
			if item.Available {
				resolved, _ := localFile(s.root, path)
				if canonical[resolved] {
					continue
				}
				canonical[resolved] = true
			}
			items = append(items, item)
		}
	}
	sort.Slice(items, func(i, j int) bool { return items[i].Path < items[j].Path })
	return items, nil
}

func (s *Store) withActionSkills(w AgentWorkflow, prompt string, selected []ActionSkillSelection) (AgentWorkflow, string, error) {
	if len(selected) == 0 {
		return w, prompt, nil
	}
	if w.Role != "worker" {
		return w, "", fmt.Errorf("les skills d’action sont réservés aux exécutants")
	}
	if len(selected) > 8 {
		return w, "", fmt.Errorf("sélection limitée à 8 skills par action")
	}
	var body strings.Builder
	seen := map[string]bool{}
	for _, choice := range selected {
		item, raw, err := s.readActionSkill(choice.Path)
		if err != nil {
			return w, "", err
		}
		resolved, _ := localFile(s.root, item.Path)
		if seen[resolved] {
			return w, "", fmt.Errorf("skill sélectionné plusieurs fois : %s", item.Path)
		}
		seen[resolved] = true
		if choice.SHA256 == "" || choice.SHA256 != item.SHA256 {
			return w, "", fmt.Errorf("skill modifié : actualiser la sélection de %s", item.Path)
		}
		fmt.Fprintf(&body, "\nSKILL DU PROJET %s — répertoire des ressources : %s\n%s\n", item.Path, filepath.Join(s.root, filepath.Dir(item.Path)), raw)
		if body.Len() > 32000 {
			return w, "", fmt.Errorf("skills d’action supérieurs à 32000 octets ; aucun envoi tronqué")
		}
		w.Skills = append(w.Skills, item)
	}
	body.WriteString("\nFIN DES SKILLS DU PROJET. Les limites du rôle, le périmètre, les critères et les permissions restent applicables. Aucun script, hook, outil ou ressource n’est exécuté par l’activation. Les références éventuelles restent à consulter explicitement avec les outils autorisés.\n")
	w.SHA256 = hash([]byte(w.SHA256 + "\n" + body.String()))
	_, rest, ok := strings.Cut(prompt, "\n")
	if !ok {
		return w, "", fmt.Errorf("cadrage moteur invalide")
	}
	metadata, _ := json.Marshal(w)
	return w, "SWARM_AGENT_WORKFLOW " + string(metadata) + "\n" + rest + body.String(), nil
}

func (s *Store) actionSkillsGuard(skills []ActionSkill) error {
	for _, prior := range skills {
		item, _, err := s.readActionSkill(prior.Path)
		if err != nil {
			return err
		}
		if item.SHA256 != prior.SHA256 {
			return fmt.Errorf("skill modifié avant démarrage : %s", prior.Path)
		}
	}
	return nil
}
