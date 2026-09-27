//go:build linux

package main

import (
	"fmt"
	"unicode"
	"unicode/utf8"

	"github.com/tiktoken-go/tokenizer/codec"
)

// A versioned capacity calculation, not provider authorization. This contract
// is deliberately not enabled for legacy fragment plans: their prompt bytes and
// digests must remain unchanged. A caller must additionally establish the actual
// provider/model/client configuration and anchor this object with the new plan.
type managedReviewInputBudget struct {
	Version       int    `json:"version"`
	Model         string `json:"model"`
	Tokenizer     string `json:"tokenizer"`
	ContextTokens int    `json:"context_tokens"`
	EffectivePct  int    `json:"effective_percent"`
	ClientReserve int    `json:"client_reserve_tokens"`
	OutputReserve int    `json:"output_reserve_tokens"`
	MaxBytes      int    `json:"max_utf8_bytes"`
}

const managedReviewTokenizer = "o200k_base/tiktoken-go-v0.6.2"

// These are conservative ceilings for the observed Codex client, not an API
// window upgrade. Reserves are explicit allowances, not measured hidden tokens.
func (b managedReviewInputBudget) inputTokens() (int, error) {
	if b.Version != 1 || b.Model != "gpt-5.6-sol" || b.Tokenizer != managedReviewTokenizer ||
		b.ContextTokens <= 0 || b.ContextTokens > 272000 || b.EffectivePct <= 0 || b.EffectivePct > 95 ||
		b.ClientReserve < 32768 || b.ClientReserve > b.ContextTokens ||
		b.OutputReserve < 65536 || b.OutputReserve > b.ContextTokens ||
		b.MaxBytes <= 0 || b.MaxBytes > 1024*1024 {
		return 0, fmt.Errorf("capacité de revue non reconnue ou réserves insuffisantes")
	}
	room := b.ContextTokens*b.EffectivePct/100 - b.ClientReserve - b.OutputReserve
	if room <= 0 {
		return 0, fmt.Errorf("aucune capacité d’entrée après réserves")
	}
	return room, nil
}

type managedReviewInputMeasure struct {
	PromptTokens int `json:"prompt_tokens"`
	SchemaTokens int `json:"schema_tokens"`
	UTF8Bytes    int `json:"utf8_bytes"`
	TokenRoom    int `json:"remaining_text_tokens"`
}

// Count prompt and schema independently; never let a merge across their message
// boundary undercount them. All source strings are ordinary text, including
// strings that resemble special tokens. No network, subprocess, Store or quota.
func (b managedReviewInputBudget) measure(prompt, schema string) (managedReviewInputMeasure, error) {
	var out managedReviewInputMeasure
	room, err := b.inputTokens()
	if err != nil {
		return out, err
	}
	if len(prompt) > b.MaxBytes || len(schema) > b.MaxBytes-len(prompt) {
		return out, fmt.Errorf("plafond mémoire de revue dépassé")
	}
	// The embedded BPE implementation merges a piece quadratically. Reject an
	// exceptionally long uninterrupted run before tokenizing untrusted evidence.
	// No truncation or substitution is permitted to make such evidence fit.
	for _, text := range []string{prompt, schema} {
		if !utf8.ValidString(text) {
			return out, fmt.Errorf("texte de revue UTF-8 invalide")
		}
		run := 0
		for _, r := range text {
			if unicode.IsSpace(r) {
				run = 0
			} else {
				run += utf8.RuneLen(r)
				if run > 8192 {
					return out, fmt.Errorf("séquence sans séparation trop longue pour le tokenizer de revue")
				}
			}
		}
	}
	// A fresh codec owns its regexp state. Vocabulary initialization is protected
	// by the dependency's sync.Once; no mutable codec is shared across callers.
	encoder := codec.NewO200kBase()
	if out.PromptTokens, err = encoder.Count(prompt); err != nil {
		return out, fmt.Errorf("comptage du message impossible : %w", err)
	}
	if out.SchemaTokens, err = encoder.Count(schema); err != nil {
		return out, fmt.Errorf("comptage du schéma impossible : %w", err)
	}
	out.UTF8Bytes = len(prompt) + len(schema)
	out.TokenRoom = room - out.PromptTokens - out.SchemaTokens
	if out.TokenRoom < 0 {
		return out, fmt.Errorf("capacité en tokens dépassée après réserves")
	}
	return out, nil
}
