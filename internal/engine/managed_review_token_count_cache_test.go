//go:build linux

package engine

import (
	"fmt"
	"strings"
	"sync"
	"testing"

	"github.com/tiktoken-go/tokenizer/codec"
)

func TestReviewTokenCacheExactCountsAndEviction(t *testing.T) {
	var c reviewTokenCountCache
	// More distinct contents than slots exercises replacement. Recounting an
	// evicted entry and hitting a stored entry must give the uncached result.
	for i := range 300 {
		text := fmt.Sprintf("Épreuve %d 東京 <|endoftext|>\n", i)
		want, err := codec.NewO200kBase().Count(text)
		if err != nil {
			t.Fatal(err)
		}
		for range 2 {
			got, err := c.count(text)
			if err != nil || got != want {
				t.Fatalf("content %d: %d != %d, %v", i, got, want, err)
			}
		}
	}
	text := "Épreuve 0 東京 <|endoftext|>\n"
	want, _ := codec.NewO200kBase().Count(text)
	got, err := c.count(text)
	if err != nil || got != want {
		t.Fatalf("replacement: %d != %d, %v", got, want, err)
	}
}

func TestReviewTokenCacheConcurrentCounts(t *testing.T) {
	var c reviewTokenCountCache
	var wg sync.WaitGroup
	for i := range 24 {
		text := fmt.Sprintf("shared proof %d", i%4)
		want, err := codec.NewO200kBase().Count(text)
		if err != nil {
			t.Fatal(err)
		}
		wg.Add(1)
		go func() {
			defer wg.Done()
			for range 20 {
				got, err := c.count(text)
				if err != nil || got != want {
					t.Errorf("concurrent count %q: %d != %d, %v", text, got, want, err)
				}
			}
		}()
	}
	wg.Wait()
}

func TestReviewTokenCacheDoesNotCacheAcceptance(t *testing.T) {
	b := reviewTokenBudgetFixture()
	text := strings.Repeat("hello world\n", 1000)
	first, err := b.measure(text, "{}")
	if err != nil {
		t.Fatal(err)
	}
	b.OutputReserve += first.TokenRoom + 1
	if _, err := b.measure(text, "{}"); err == nil {
		t.Fatal("cached text bypassed lower capacity")
	}
	b = reviewTokenBudgetFixture()
	b.MaxBytes = len(text)
	if _, err := b.measure(text, "{}"); err == nil {
		t.Fatal("cached text bypassed byte cap")
	}
	b = reviewTokenBudgetFixture()
	b.Tokenizer = "different"
	if _, err := b.measure(text, "{}"); err == nil {
		t.Fatal("cached text bypassed tokenizer contract")
	}
	b = reviewTokenBudgetFixture()
	changed := text + "new evidence"
	got, err := b.measure(changed, "{}")
	want, _ := codec.NewO200kBase().Count(changed)
	if err != nil || got.PromptTokens != want {
		t.Fatalf("changed evidence not counted: %+v %v", got, err)
	}
	schema := strings.Repeat("A ", 170000)
	if _, err := b.measure(text, schema); err == nil {
		t.Fatal("cached prompt hid schema overflow")
	}
}
