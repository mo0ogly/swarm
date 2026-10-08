//go:build linux

package main

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"
)

// Query the configured executable’s bundled catalog, not a cache belonging to
// another Codex version. Only local metadata commands, no provider/model request.
// The executable stamp is rechecked without subprocesses during review polling.
func observedReviewInputBudget(provider Provider, route *ModelRoute, providerDigest string) (*managedReviewInputBudget, error) {
	if providerAdapter(provider) != "codex" || route == nil || route.Model != "gpt-5.6-sol" || len(providerDigest) != 64 {
		return nil, fmt.Errorf("capacité du fournisseur de revue non établie")
	}
	stamp, err := reviewClientStamp(provider.Command)
	if err != nil {
		return nil, err
	}
	version, err := reviewClientMetadata(provider, "--version")
	if err != nil {
		return nil, err
	}
	raw, err := reviewClientMetadata(provider, "debug", "models", "--bundled")
	if err != nil {
		return nil, err
	}
	var cache struct {
		Models []struct {
			Model   string `json:"slug"`
			Context int    `json:"context_window"`
			Percent int    `json:"effective_context_window_percent"`
		} `json:"models"`
	}
	clientVersion := strings.TrimSpace(string(version))
	if json.Unmarshal(raw, &cache) != nil || !strings.HasPrefix(clientVersion, "codex-cli ") {
		return nil, fmt.Errorf("catalogue embarqué de capacité invalide")
	}
	var found *managedReviewInputBudget
	for _, m := range cache.Models {
		if m.Model != route.Model {
			continue
		}
		if found != nil {
			return nil, fmt.Errorf("capacité modèle dupliquée")
		}
		b := &managedReviewInputBudget{Version: 1, Model: m.Model, Tokenizer: managedReviewTokenizer, ContextTokens: m.Context, EffectivePct: m.Percent, ClientReserve: 32768, OutputReserve: 65536, MaxBytes: 1024 * 1024, ProviderDigest: providerDigest, ClientVersion: clientVersion, ClientStamp: stamp}
		if _, err := b.inputTokens(); err != nil {
			return nil, err
		}
		capability, _ := json.Marshal([]any{clientVersion, stamp, m.Model, m.Context, m.Percent})
		b.CapabilityDigest = hash(capability)
		found = b
	}
	if found == nil {
		return nil, fmt.Errorf("modèle absent du catalogue de capacité")
	}
	return found, nil
}

func (s *Store) reviewInputBudget(cfg *ReviewerConfig) (*managedReviewInputBudget, error) {
	if cfg == nil {
		return nil, fmt.Errorf("configuration de revue absente")
	}
	ps, err := s.providers()
	if err != nil {
		return nil, err
	}
	p, ok := ps.Providers[cfg.Provider]
	if !ok {
		return nil, fmt.Errorf("fournisseur de revue absent")
	}
	level := "auto"
	if cfg.ModelRoute != nil {
		level = cfg.ModelRoute.Level
	}
	_, route, err := resolveModel(p, level, "planning")
	if err != nil {
		return nil, err
	}
	return observedReviewInputBudget(p, route, cfg.ProviderDigest)
}

func (s *Store) planCandidateFragments(w Work, a Agent, c managedReviewContext, available int) (managedReviewFragmentPlan, error) {
	if task, err := w.task(a.TaskID); err == nil && task.ReviewCoordination != nil {
		p, err := s.coordinatedFragments(w, a, c, 100000)
		if err != nil {
			return p, err
		}
		_, method, err := s.projectAgentWorkflow("reviewer")
		if err != nil {
			return managedReviewFragmentPlan{}, err
		}
		prefix := managedReviewPrefix(method)
		transportErr := preflightManagedFragmentCalls(prefix, c, p)
		remaining := available
		if w.Planning != nil && w.Planning.Reviewer != nil {
			remaining = min(remaining, max(0, w.Planning.Reviewer.MaxCalls-w.Planning.Reviewer.Calls))
		}
		if transportErr == nil && len(p.Packets)+2 <= remaining {
			p.AvailableCalls = available
			return p, nil
		}
		budget, capacityErr := s.reviewInputBudget(w.Planning.Reviewer)
		if capacityErr == nil {
			return planTokenManagedFragments(c, p, *budget, prefix, available)
		}
		if transportErr != nil {
			return managedReviewFragmentPlan{}, transportErr
		}
		return managedReviewFragmentPlan{}, fmt.Errorf("revues coordonnées : %d appels requis, %d disponibles", len(p.Packets)+2, available)
	}
	// Compute legacy inventory without relaxing the caller's execution budget.
	p, err := s.planLegacyCandidateFragments(w, a, c, 100000)
	if err != nil {
		return p, err
	}
	_, method, err := s.projectAgentWorkflow("reviewer")
	if err != nil {
		return managedReviewFragmentPlan{}, err
	}
	prefix := managedReviewPrefix(method)
	remaining := available
	if w.Planning != nil && w.Planning.Reviewer != nil {
		remaining = min(remaining, max(0, w.Planning.Reviewer.MaxCalls-w.Planning.Reviewer.Calls))
	}
	if fragmentPaidInspections(p)+2 <= remaining && preflightManagedFragmentCalls(prefix, c, p) == nil {
		p.AvailableCalls = available
		return p, nil
	}
	b, capacityErr := s.reviewInputBudget(w.Planning.Reviewer)
	if capacityErr == nil {
		return planTokenManagedFragments(c, p, *b, prefix, available)
	}
	// Preserve legacy failure behavior and estimates for unsupported providers.
	return s.planLegacyCandidateFragments(w, a, c, available)
}

func reviewClientStamp(command string) (string, error) {
	resolved, err := filepath.EvalSymlinks(command)
	if err != nil {
		return "", err
	}
	info, err := os.Stat(resolved)
	if err != nil {
		return "", err
	}
	if !info.Mode().IsRegular() || info.Mode()&0111 == 0 {
		return "", fmt.Errorf("client non exécutable")
	}
	raw, _ := json.Marshal([]any{resolved, info.Size(), info.ModTime().UnixNano(), uint32(info.Mode())})
	return hash(raw), nil
}

func reviewClientMetadata(p Provider, args ...string) ([]byte, error) {
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	cmd := exec.CommandContext(ctx, p.Command, args...)
	cmd.Env = providerEnvironment(p.Env)
	raw, err := cmd.Output()
	if err != nil || len(raw) > 2<<20 {
		return nil, fmt.Errorf("métadonnées du client indisponibles")
	}
	return raw, nil
}
