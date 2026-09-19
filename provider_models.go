package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strconv"
	"strings"
)

type ModelChoice struct {
	Model  string `json:"model"`
	Effort string `json:"effort"`
}
type ModelPolicy struct {
	Version   int                    `json:"version"`
	Billing   string                 `json:"billing"`
	WorkLevel string                 `json:"work_level"`
	PageLevel string                 `json:"page_level"`
	Levels    map[string]ModelChoice `json:"levels"`
}
type ModelRoute struct {
	Level      string `json:"level"`
	Model      string `json:"model"`
	Effort     string `json:"effort"`
	Billing    string `json:"billing"`
	PolicyHash string `json:"policy_hash"`
	Reason     string `json:"reason"`
}
type ModelOption struct {
	ID      string   `json:"id"`
	Efforts []string `json:"efforts"`
	Source  string   `json:"source"`
}

var modelLevels = []string{"simple", "standard", "exigeant"}
var modelName = regexp.MustCompile(`^[A-Za-z0-9][A-Za-z0-9._:/-]{0,159}$`)

func providerAdapter(p Provider) string {
	if p.APIConnectionID != "" {
		return "api"
	}
	switch filepath.Base(p.Command) {
	case "codex":
		return "codex"
	case "claude":
		return "claude"
	case "skynet_harness":
		return "skynet"
	}
	return "custom"
}
func configuredModel(p Provider) string {
	for i, a := range p.Args {
		if (a == "--model" || a == "-m") && i+1 < len(p.Args) {
			return p.Args[i+1]
		}
		if strings.HasPrefix(a, "--model=") {
			return strings.TrimPrefix(a, "--model=")
		}
	}
	return ""
}
func modelOptions(p Provider) []ModelOption {
	out := []ModelOption{}
	switch providerAdapter(p) {
	case "codex":
		home, _ := os.UserHomeDir()
		root := os.Getenv("CODEX_HOME")
		if root == "" {
			root = filepath.Join(home, ".codex")
		}
		b, e := os.ReadFile(filepath.Join(root, "models_cache.json"))
		if e == nil && len(b) < 2<<20 {
			var cache struct {
				Models []struct {
					Slug       string `json:"slug"`
					Visibility string `json:"visibility"`
					Levels     []struct {
						Effort string `json:"effort"`
					} `json:"supported_reasoning_levels"`
				} `json:"models"`
			}
			if json.Unmarshal(b, &cache) == nil {
				for _, m := range cache.Models {
					if m.Visibility != "list" || !modelName.MatchString(m.Slug) {
						continue
					}
					o := ModelOption{ID: m.Slug, Efforts: []string{}, Source: "catalogue local Codex"}
					for _, l := range m.Levels {
						o.Efforts = append(o.Efforts, l.Effort)
					}
					out = append(out, o)
				}
			}
		}
	case "claude":
		for _, id := range []string{"haiku", "sonnet", "opus"} {
			out = append(out, ModelOption{ID: id, Efforts: []string{}, Source: "alias de la CLI Claude ; accès à tester"})
		}
	}
	current := configuredModel(p)
	found := false
	for _, o := range out {
		if o.ID == current {
			found = true
		}
	}
	if current != "" && !found {
		out = append(out, ModelOption{ID: current, Efforts: []string{}, Source: "configuration de cet exécutable ; accès à tester"})
	}
	sort.Slice(out, func(i, j int) bool { return out[i].ID < out[j].ID })
	return out
}
func effectiveModelPolicy(p Provider) *ModelPolicy {
	if p.ModelPolicy != nil {
		return p.ModelPolicy
	}
	policy := &ModelPolicy{Version: 1, Billing: "external", WorkLevel: "standard", PageLevel: "simple", Levels: map[string]ModelChoice{}}
	switch providerAdapter(p) {
	case "codex":
		policy.Levels = map[string]ModelChoice{"simple": {Model: "gpt-5.6-luna", Effort: "low"}, "standard": {Model: "gpt-5.6-sol", Effort: "medium"}, "exigeant": {Model: "gpt-6-astra", Effort: "high"}}
	case "claude":
		policy.Levels = map[string]ModelChoice{"simple": {Model: "haiku"}, "standard": {Model: "sonnet"}, "exigeant": {Model: "opus"}}
	case "api":
		for _, level := range modelLevels {
			policy.Levels[level] = ModelChoice{Model: configuredModel(p)}
		}
	case "skynet":
		policy.Billing = "on_premise"
		for _, level := range modelLevels {
			policy.Levels[level] = ModelChoice{Model: configuredModel(p)}
		}
	default:
		return nil
	}
	return policy
}
func validatePolicyShape(policy *ModelPolicy) error {
	if policy == nil {
		return fmt.Errorf("Politique absente.")
	}
	if policy.Version != 1 || (policy.Billing != "external" && policy.Billing != "on_premise") {
		return fmt.Errorf("Version ou mode de facturation inconnu.")
	}
	if policy.WorkLevel == "exigeant" || policy.PageLevel == "exigeant" {
		return fmt.Errorf("Le niveau exigeant doit rester un choix explicite, jamais un défaut.")
	}
	if len(policy.Levels) != 3 {
		return fmt.Errorf("Les trois niveaux simple, standard et exigeant sont requis.")
	}
	if _, ok := policy.Levels[policy.WorkLevel]; !ok {
		return fmt.Errorf("Niveau par défaut des travaux invalide.")
	}
	if _, ok := policy.Levels[policy.PageLevel]; !ok {
		return fmt.Errorf("Niveau par défaut des questions invalide.")
	}
	for _, level := range modelLevels {
		if _, ok := policy.Levels[level]; !ok {
			return fmt.Errorf("Niveau requis : %s.", level)
		}
	}
	return nil
}
func validateModelPolicy(p Provider, policy *ModelPolicy) error {
	if err := validatePolicyShape(policy); err != nil {
		return err
	}
	options := modelOptions(p)
	for _, level := range modelLevels {
		choice, ok := policy.Levels[level]
		if !ok || !modelName.MatchString(choice.Model) {
			return fmt.Errorf("Modèle invalide pour %s.", level)
		}
		found := false
		effortOK := choice.Effort == ""
		for _, o := range options {
			if o.ID == choice.Model {
				found = true
				for _, effort := range o.Efforts {
					if choice.Effort == effort {
						effortOK = true
					}
				}
			}
		}
		if !found {
			return fmt.Errorf("Modèle %s absent du catalogue local ; vérifier sa disponibilité et la configuration CLI.", choice.Model)
		}
		if !effortOK {
			return fmt.Errorf("Effort %s non annoncé pour %s.", choice.Effort, choice.Model)
		}
		if level != "exigeant" && (strings.Contains(choice.Model, "astra") || choice.Model == "opus") {
			return fmt.Errorf("Le modèle frontière %s est réservé au niveau exigeant explicite.", choice.Model)
		}
	}
	return nil
}
func resolveModel(p Provider, level, purpose string) (Provider, *ModelRoute, error) {
	policy := effectiveModelPolicy(p)
	if policy == nil {
		if level != "" && level != "auto" {
			return p, nil, fmt.Errorf("Adaptateur de modèles indisponible pour cet exécutable.")
		}
		return p, nil, nil
	}
	if err := validatePolicyShape(policy); err != nil {
		return p, nil, err
	}
	// Validate the requested choice rather than requiring unrelated models to be
	// available: an account may have Sol but no frontier model.
	if level == "" || level == "auto" {
		if purpose == "page" {
			level = policy.PageLevel
		} else {
			level = policy.WorkLevel
		}
		if level == "exigeant" {
			return p, nil, fmt.Errorf("Niveau exigeant interdit comme défaut.")
		}
	}
	choice, ok := policy.Levels[level]
	if !ok {
		return p, nil, fmt.Errorf("Niveau inconnu : %s.", level)
	}
	if !modelName.MatchString(choice.Model) {
		return p, nil, fmt.Errorf("Modèle explicite requis ; aucun défaut coûteux hérité de la CLI.")
	}
	available := false
	effortOK := choice.Effort == ""
	for _, o := range modelOptions(p) {
		if o.ID == choice.Model {
			available = true
			for _, e := range o.Efforts {
				if e == choice.Effort {
					effortOK = true
				}
			}
		}
	}
	if !available || !effortOK {
		return p, nil, fmt.Errorf("Modèle ou effort indisponible dans le catalogue : %s / %s. Aucun repli automatique.", choice.Model, choice.Effort)
	}
	if level != "exigeant" && (strings.Contains(choice.Model, "astra") || choice.Model == "opus") {
		return p, nil, fmt.Errorf("Modèle frontière réservé au choix exigeant explicite.")
	}
	raw, _ := json.Marshal(policy)
	r := &ModelRoute{Level: level, Model: choice.Model, Effort: choice.Effort, Billing: policy.Billing, PolicyHash: hash(raw), Reason: "Politique du fournisseur · " + purpose + " · sans montée en gamme automatique"}
	return applyModelRoute(p, r), r, nil
}
func applyModelRoute(p Provider, r *ModelRoute) Provider {
	if r == nil {
		return p
	}
	args := []string{}
	for i := 0; i < len(p.Args); i++ {
		a := p.Args[i]
		if a == "--model" || a == "-m" || a == "--effort" {
			i++
			continue
		}
		if strings.HasPrefix(a, "--model=") || strings.HasPrefix(a, "--effort=") {
			continue
		}
		if (a == "-c" || a == "--config") && i+1 < len(p.Args) && isModelConfig(p.Args[i+1]) {
			i++
			continue
		}
		if strings.HasPrefix(a, "--config=") && isModelConfig(strings.TrimPrefix(a, "--config=")) {
			continue
		}
		if strings.HasPrefix(a, "-c") && len(a) > 2 && isModelConfig(a[2:]) {
			continue
		}
		args = append(args, a)
	}
	stdin := len(args) > 0 && args[len(args)-1] == "-"
	if stdin {
		args = args[:len(args)-1]
	}
	args = append(args, "--model", r.Model)
	if r.Effort != "" {
		if providerAdapter(p) == "codex" {
			args = append(args, "-c", "model_reasoning_effort="+strconv.Quote(r.Effort))
		} else {
			args = append(args, "--effort", r.Effort)
		}
	}
	if stdin {
		args = append(args, "-")
	}
	p.Args = args
	return p
}

func isModelConfig(value string) bool {
	key, _, ok := strings.Cut(value, "=")
	if !ok {
		return false
	}
	key = strings.TrimSpace(key)
	return key == "model" || key == "model_reasoning_effort"
}
