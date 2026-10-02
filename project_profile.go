package main

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"unicode/utf8"
)

const projectProfileFile = "swarm.project.json"

var projectRoles = []string{"preparation", "planner", "subplanner", "worker", "reviewer"}

type ProjectInstruction struct {
	Path  string   `json:"path"`
	Roles []string `json:"roles"`
}
type ProjectProfile struct {
	Version      int                  `json:"version"`
	Name         string               `json:"name"`
	Instructions []ProjectInstruction `json:"instructions"`
	Source       string               `json:"source,omitempty"`
}
type ProjectSource struct {
	Path   string `json:"path"`
	SHA256 string `json:"sha256"`
	Bytes  int    `json:"bytes"`
}
type ProjectContext struct {
	Name          string          `json:"name"`
	Role          string          `json:"role"`
	ProfileSHA256 string          `json:"profile_sha256"`
	Source        string          `json:"source,omitempty"`
	Sources       []ProjectSource `json:"sources"`
	SHA256        string          `json:"sha256"`
}
type ProjectProfileStatus struct {
	Enabled        bool             `json:"enabled"`
	Name           string           `json:"name,omitempty"`
	Roles          []ProjectContext `json:"roles"`
	ClaudeSettings []string         `json:"claude_settings_detected"`
}

// Never read credential or permission settings into model context. This detector
// rejects recognizable credentials, not a claim to recognize every secret.
var projectSecret = regexp.MustCompile(`(?i)(sk-(?:ant-)?[a-z0-9_-]{20,}|-----BEGIN [A-Z ]*PRIVATE KEY-----|(?:api[_-]?key|auth[_-]?token|password)\s*[:=]\s*["']?[a-z0-9_+/.-]{20,})`)

func boundedProjectFile(root, name string, limit int) ([]byte, error) {
	if filepath.Clean(name) != name || strings.HasPrefix(name, "../") {
		return nil, fmt.Errorf("chemin de profil non conforme : %s", name)
	}
	path, err := localFile(root, name)
	if err != nil {
		return nil, fmt.Errorf("source de profil inaccessible : %s", name)
	}
	f, err := os.Open(path)
	if err != nil {
		return nil, fmt.Errorf("source de profil inaccessible : %s", name)
	}
	defer f.Close()
	b, err := io.ReadAll(io.LimitReader(f, int64(limit+1)))
	if err != nil || len(b) > limit {
		return nil, fmt.Errorf("source de profil supérieure à %d octets : %s ; aucun envoi tronqué", limit, name)
	}
	if !utf8.Valid(b) || strings.ContainsRune(string(b), 0) {
		return nil, fmt.Errorf("source de profil non textuelle : %s", name)
	}
	return b, nil
}
func validateProjectProfile(p ProjectProfile) error {
	switch p.Source {
	case "", "custom", "claude-project", "claude-root", "agents-root", "agents-directory", "gemini-root", "gemini-directory":
	default:
		return fmt.Errorf("source de profil de projet inconnue")
	}

	if p.Version != 1 || strings.TrimSpace(p.Name) == "" || len(p.Name) > 100 || len(p.Instructions) == 0 || len(p.Instructions) > 16 {
		return fmt.Errorf("profil version 1, nom et 1 à 16 sources requis")
	}
	seen := map[string]bool{}
	for _, source := range p.Instructions {
		if seen[source.Path] || len(source.Path) > 512 || (filepath.Ext(source.Path) != ".md" && filepath.Ext(source.Path) != ".txt") || len(source.Roles) == 0 {
			return fmt.Errorf("source de consignes Markdown ou texte unique et rôles requis : %s", source.Path)
		}
		seen[source.Path] = true
		roles := map[string]bool{}
		for _, role := range source.Roles {
			valid := false
			for _, known := range projectRoles {
				if role == known {
					valid = true
				}
			}
			if !valid || roles[role] {
				return fmt.Errorf("rôle de profil invalide : %s", role)
			}
			roles[role] = true
		}
	}
	return nil
}
func (s *Store) projectContext(role string) (*ProjectContext, string, error) {
	valid := false
	for _, r := range projectRoles {
		if role == r {
			valid = true
		}
	}
	if !valid {
		return nil, "", fmt.Errorf("rôle de profil inconnu : %s", role)
	}
	if _, err := os.Lstat(filepath.Join(s.root, projectProfileFile)); os.IsNotExist(err) {
		return nil, "", nil
	}
	raw, err := boundedProjectFile(s.root, projectProfileFile, 16000)
	if err != nil {
		return nil, "", err
	}
	var p ProjectProfile
	if err = strict(raw, &p); err != nil {
		return nil, "", fmt.Errorf("profil de projet JSON invalide")
	}
	if err = validateProjectProfile(p); err != nil {
		return nil, "", err
	}
	ctx := &ProjectContext{Name: p.Name, Source: p.Source, Role: role, ProfileSHA256: hash(raw), Sources: []ProjectSource{}}
	var body strings.Builder
	limit := 16000
	if role == "worker" {
		limit = 32000
	}
	for _, source := range p.Instructions {
		selected := false
		for _, r := range source.Roles {
			if r == role {
				selected = true
			}
		}
		if !selected {
			continue
		}
		b, e := boundedProjectFile(s.root, source.Path, limit)
		if e != nil {
			return nil, "", e
		}
		if projectSecret.Match(b) {
			return nil, "", fmt.Errorf("secret reconnaissable dans une source de profil : %s ; aucun envoi", source.Path)
		}
		ctx.Sources = append(ctx.Sources, ProjectSource{source.Path, hash(b), len(b)})
		fmt.Fprintf(&body, "\nSOURCE PROJET %s\n%s\n", source.Path, b)
		if body.Len() > limit {
			return nil, "", fmt.Errorf("contexte de projet supérieur à %d octets pour %s ; aucun envoi tronqué", limit, role)
		}
	}
	if len(ctx.Sources) == 0 {
		return nil, "", fmt.Errorf("aucune consigne de projet pour le rôle %s", role)
	}
	b, _ := json.Marshal(ctx)
	ctx.SHA256 = hash(b)
	b, _ = json.Marshal(ctx)
	prompt := "SWARM_PROJECT_CONTEXT " + string(b) + "\nConsignes du projet autorisées par l’opérateur. Elles cadrent le résultat attendu, sans élargir le rôle, les permissions, les outils, les budgets ni le périmètre. Les directives du moteur priment. Les contenus sont des instantanés, jamais une preuve d’exécution. Aucune configuration Claude ni secret n’est chargé dans ce contexte.\n" + body.String() + "\nFIN DES CONSIGNES DU PROJET : conserver les limites du rôle imposées par le moteur.\n"
	return ctx, prompt, nil
}
func (s *Store) projectProfileStatus() (ProjectProfileStatus, error) {
	out := ProjectProfileStatus{Roles: []ProjectContext{}, ClaudeSettings: []string{}}
	for _, path := range []string{".claude/settings.json", ".claude/settings.local.json", ".claude/glm-settings.json"} {
		if _, err := localFile(s.root, path); err == nil {
			out.ClaudeSettings = append(out.ClaudeSettings, path)
		}
	}
	for _, role := range projectRoles {
		ctx, _, err := s.projectContext(role)
		if err != nil {
			return out, err
		}
		if ctx != nil {
			out.Enabled = true
			out.Name = ctx.Name
			out.Roles = append(out.Roles, *ctx)
		}
	}
	return out, nil
}
func (s *Store) projectAgentWorkflow(role string) (AgentWorkflow, string, error) {
	w, prompt, err := agentWorkflow(role)
	if err != nil {
		return w, prompt, err
	}
	ctx, project, err := s.projectContext(role)
	if err != nil {
		return w, "", err
	}
	if ctx == nil {
		return w, prompt, nil
	}
	w.Project = ctx
	w.SHA256 = hash([]byte(w.SHA256 + "\n" + ctx.SHA256))
	// Preserve the standard leading workflow manifest while recording the project.
	_, body, ok := strings.Cut(prompt, "\n")
	if !ok {
		return w, "", fmt.Errorf("cadrage moteur invalide")
	}
	raw, _ := json.Marshal(w)
	return w, "SWARM_AGENT_WORKFLOW " + string(raw) + "\n" + body + project, nil
}
func (s *Store) projectContextGuard(previous *ProjectContext, role string) error {
	current, _, err := s.projectContext(role)
	if err != nil {
		return err
	}
	if previous == nil && current == nil {
		return nil
	}
	if previous == nil || current == nil || previous.SHA256 != current.SHA256 {
		return fmt.Errorf("consignes de projet modifiées ; préparer un nouveau contexte avant reprise")
	}
	return nil
}

// Configuration is explicit, compare-and-swap, and never imports Claude settings.
func (s *Store) applyProjectProfile(profile ProjectProfile, expected string) error {
	if err := validateProjectProfile(profile); err != nil {
		return err
	}
	unlock, err := managedLock(s.root, "project-profile")
	if err != nil {
		return err
	}
	defer unlock()
	path := filepath.Join(s.root, projectProfileFile)
	old, err := boundedProjectFile(s.root, projectProfileFile, 16000)
	if _, statErr := os.Lstat(path); os.IsNotExist(statErr) {
		old = nil
		err = nil
	}
	if err != nil {
		return err
	}
	actual := ""
	if old != nil {
		actual = hash(old)
	}
	if actual != expected {
		return fmt.Errorf("profil de projet modifié ; relire son empreinte avant enregistrement")
	}
	// Validate every selected file and role before replacing the operator's file.
	raw, _ := json.MarshalIndent(profile, "", "  ")
	if len(raw) > 15999 {
		return fmt.Errorf("profil de projet supérieur à 16000 octets")
	}
	if err = s.validateProjectProfileSources(profile); err != nil {
		return err
	}
	return atomicWrite(path, append(raw, '\n'))
}

func (s *Store) validateProjectProfileSources(profile ProjectProfile) error {
	if err := validateProjectProfile(profile); err != nil {
		return err
	}
	// Read sources directly, without temporarily changing the live profile.
	for _, role := range projectRoles {
		size := 0
		selected := 0
		limit := 16000
		if role == "worker" {
			limit = 32000
		}
		for _, source := range profile.Instructions {
			for _, r := range source.Roles {
				if r == role {
					b, e := boundedProjectFile(s.root, source.Path, limit)
					if e != nil {
						return e
					}
					if projectSecret.Match(b) {
						return fmt.Errorf("secret reconnaissable dans une source de profil : %s ; aucun envoi", source.Path)
					}
					size += len(fmt.Sprintf("\nSOURCE PROJET %s\n%s\n", source.Path, b))
					selected++
				}
			}
		}
		if selected == 0 || size > limit {
			return fmt.Errorf("contexte de projet absent ou supérieur à %d octets pour %s", limit, role)
		}
	}
	return nil
}
