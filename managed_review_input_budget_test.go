//go:build linux

package main

import (
	"encoding/json"
	"os"
	"strings"
	"sync"
	"testing"
)

func reviewTokenBudgetFixture() managedReviewInputBudget {
	return managedReviewInputBudget{Version: 1, Model: "gpt-5.6-sol", Tokenizer: managedReviewTokenizer, ContextTokens: 272000, EffectivePct: 95, ClientReserve: 32768, OutputReserve: 65536, MaxBytes: 1024 * 1024}
}

func TestReviewInputTokensMatchIndependentPythonVectors(t *testing.T) {
	raw, err := os.ReadFile("testdata/review_tokens_o200k.json")
	if err != nil {
		t.Fatal(err)
	}
	var vectors struct {
		Cases []struct {
			Text   string
			Tokens int
		}
	}
	if err = json.Unmarshal(raw, &vectors); err != nil {
		t.Fatal(err)
	}
	if len(vectors.Cases) != 6 {
		t.Fatal("missing reference cases")
	}
	for i, c := range vectors.Cases {
		got, err := reviewTokenBudgetFixture().measure(c.Text, "{}")
		if err != nil || got.PromptTokens != c.Tokens || got.SchemaTokens != 1 || got.UTF8Bytes != len(c.Text)+2 {
			t.Fatalf("reference %d: %+v %v", i, got, err)
		}
	}
}

func TestReviewInputTokensKeepIndependentMemoryAndTokenLimits(t *testing.T) {
	b := reviewTokenBudgetFixture()
	text := strings.Repeat("hello world\n", 30000)
	if len(text) <= managedReviewPromptLimit {
		t.Fatal("fixture must exceed old byte limit")
	}
	got, err := b.measure(text, "{}")
	if err != nil || got.TokenRoom <= 0 {
		t.Fatalf("text fits token budget: %+v %v", got, err)
	}
	if _, err = b.measure(strings.Repeat("A ", 170000), "{}"); err == nil {
		t.Fatal("token overflow admitted")
	}
	if _, err = b.measure("hello", strings.Repeat("A ", 170000)); err == nil {
		t.Fatal("schema not counted")
	}
	if _, err = b.measure(strings.Repeat("a ", b.MaxBytes/2), "{}"); err == nil {
		t.Fatal("byte cap bypassed")
	}
	if _, err = b.measure("\xff", "{}"); err == nil {
		t.Fatal("invalid UTF8 replaced")
	}
	if _, err = b.measure(strings.Repeat("a", 8193), "{}"); err == nil {
		t.Fatal("unbounded BPE work")
	}
}

func TestReviewInputTokensRejectUnsupportedOrUnderreservedContract(t *testing.T) {
	changes := []func(*managedReviewInputBudget){
		func(b *managedReviewInputBudget) { b.Version++ },
		func(b *managedReviewInputBudget) { b.Model = "unknown" },
		func(b *managedReviewInputBudget) { b.Tokenizer = "guess" },
		func(b *managedReviewInputBudget) { b.ContextTokens = 1050000 },
		func(b *managedReviewInputBudget) { b.ContextTokens = -1 },
		func(b *managedReviewInputBudget) { b.EffectivePct = 100 },
		func(b *managedReviewInputBudget) { b.EffectivePct = 1 },
		func(b *managedReviewInputBudget) { b.ClientReserve = 32767 },
		func(b *managedReviewInputBudget) { b.OutputReserve = 65535 },
		func(b *managedReviewInputBudget) { b.MaxBytes = 0 },
		func(b *managedReviewInputBudget) { b.MaxBytes = 1024*1024 + 1 },
	}
	for i, change := range changes {
		b := reviewTokenBudgetFixture()
		change(&b)
		if _, err := b.measure("proof", "{}"); err == nil {
			t.Fatalf("invalid policy %d admitted", i)
		}
	}
}

func TestReviewInputTokensConcurrentCodecsAndImmutableContract(t *testing.T) {
	b := reviewTokenBudgetFixture()
	before, _ := json.Marshal(b)
	var wg sync.WaitGroup
	for range 12 {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for range 5 {
				got, err := b.measure("hello world", "{}")
				if err != nil || got.PromptTokens != 2 {
					t.Errorf("concurrent count: %+v %v", got, err)
				}
			}
		}()
	}
	wg.Wait()
	after, _ := json.Marshal(b)
	if string(before) != string(after) {
		t.Fatal("contract mutated")
	}
}
