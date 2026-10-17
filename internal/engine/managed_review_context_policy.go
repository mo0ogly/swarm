//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"strconv"
)

func managedReviewCapacityDigest(b managedReviewInputBudget) string {
	raw, _ := json.Marshal([]any{b.ClientVersion, b.ClientStamp, b.Model, b.DefaultContextTokens, b.MaxContextTokens, b.ContextTokens, b.EffectivePct})
	return hash(raw)
}

func prepareReviewContextProvider(p Provider, b *managedReviewInputBudget) (Provider, error) {
	if b == nil || b.Version == 1 {
		return p, nil
	}
	if _, err := b.inputTokens(); err != nil {
		return p, err
	}
	raw, _ := json.Marshal(p)
	stamp, err := reviewClientStamp(p.Command)
	if err != nil || providerAdapter(p) != "codex" || stamp != b.ClientStamp || hash(raw) != b.ProviderDigest || b.CapabilityDigest != managedReviewCapacityDigest(*b) {
		return p, fmt.Errorf("capacité de contexte remplacée ; aucun appel fournisseur")
	}
	p.structuredContextTokens = b.ContextTokens
	return p, nil
}

func appendStructuredContextWindow(p Provider, tokens int) (Provider, error) {
	if tokens == 0 {
		return p, nil
	}
	if tokens < 0 || providerAdapter(p) != "codex" || len(p.Args) == 0 || p.Args[len(p.Args)-1] != "-" {
		return p, fmt.Errorf("fenêtre de contexte structurée non prise en charge")
	}
	args := append([]string{}, p.Args[:len(p.Args)-1]...)
	p.Args = append(args, "-c", "model_context_window="+strconv.Itoa(tokens), "-")
	return p, nil
}
