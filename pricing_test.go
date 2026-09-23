//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"math"
	"os"
	"path/filepath"
	"testing"
)

func priceNumber(v float64) *float64 { return &v }
func tokenNumber(v int64) *int64     { return &v }
func testRate() ModelRate {
	return ModelRate{Provider: "fixture", Model: "example-v1", Currency: "USD", Input: priceNumber(2), Output: priceNumber(5), Source: "Illustrative fixture, not a provider quote", ReferenceDate: "2026-09-23"}
}
func TestPricingVersionCacheAndPersistence(t *testing.T) {
	s := storeTest(t)
	c, err := s.rateCatalogue()
	if err != nil {
		t.Fatal(err)
	}
	if len(c.Rates) != 0 {
		t.Fatal("invented prices")
	}
	r := RateChange{Schema: 1, Expected: c.Digest, Rate: testRate()}
	first, err := s.saveRate(r)
	if err != nil {
		t.Fatal(err)
	}
	qreq := RateQuoteRequest{Version: first.Version, Input: tokenNumber(1000000), Output: tokenNumber(1000000), CacheRead: 1000000}
	q, err := s.quoteRate(qreq)
	if err != nil {
		t.Fatal(err)
	}
	if q.Amount != nil || q.Status != "unknown" || len(q.Missing) != 1 {
		t.Fatal(q)
	}
	if _, err := s.saveRate(r); err == nil || commandFailure(err).Code != "revision_conflict" {
		t.Fatal("stale change", err)
	}
	c, _ = s.rateCatalogue()
	r.Expected = c.Digest
	r.Rate.CacheRead = priceNumber(.5)
	second, err := s.saveRate(r)
	if err != nil {
		t.Fatal(err)
	}
	qreq.Version = second.Version
	q, err = s.quoteRate(qreq)
	if err != nil || q.Amount == nil || *q.Amount != 7.5 {
		t.Fatal(q, err)
	}
	// Explicit non-cached input avoids subtracting cache from provider-dependent totals.
	qreq.Version = first.Version
	q, err = s.quoteRate(qreq)
	if err != nil || q.Amount != nil {
		t.Fatal("old rate rewritten", q, err)
	}
	qreq.CacheRead = 0
	q, err = s.quoteRate(qreq)
	if err != nil || q.Amount == nil || *q.Amount != 7 {
		t.Fatal(q, err)
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	c, err = reopened.rateCatalogue()
	if err != nil || len(c.Rates) != 2 || c.Rates[0].Version != first.Version {
		t.Fatal(c, err)
	}
	qreq.Input = nil
	if _, err = s.quoteRate(qreq); err == nil {
		t.Fatal("missing count accepted")
	}
	qreq.Input = tokenNumber(-1)
	if _, err = s.quoteRate(qreq); err == nil {
		t.Fatal("negative count")
	}
}
func TestPricingInvalidAndConcurrentWrites(t *testing.T) {
	for _, n := range []float64{-1, math.NaN(), math.Inf(1), 1e10} {
		r := testRate()
		r.Input = priceNumber(n)
		if validateRate(r) == nil {
			t.Fatal("invalid accepted", n)
		}
	}
	s := storeTest(t)
	other, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer other.db.Close()
	c, _ := s.rateCatalogue()
	results := make(chan error, 2)
	for _, store := range []*Store{s, other} {
		go func(store *Store) {
			_, err := store.saveRate(RateChange{Schema: 1, Expected: c.Digest, Rate: testRate()})
			results <- err
		}(store)
	}
	success, conflicts := 0, 0
	for range 2 {
		err := <-results
		if err == nil {
			success++
		} else if commandFailure(err).Code == "revision_conflict" {
			conflicts++
		} else {
			t.Fatal(err)
		}
	}
	if success != 1 || conflicts != 1 {
		t.Fatal(success, conflicts)
	}
}
func TestPricingCLI(t *testing.T) {
	s := storeTest(t)
	c, _ := s.rateCatalogue()
	raw, _ := json.Marshal(RateChange{Schema: 1, Expected: c.Digest, Rate: testRate()})
	input := filepath.Join(t.TempDir(), "request.json")
	if err := os.WriteFile(input, raw, 0600); err != nil {
		t.Fatal(err)
	}
	var out bytes.Buffer
	if err := s.pricingCLI([]string{"pricing", "save"}, input, &out); err != nil {
		t.Fatal(err)
	}
	var saved ModelRate
	if err := json.Unmarshal(out.Bytes(), &saved); err != nil {
		t.Fatal(err)
	}
	if saved.Version == "" || saved.Actor == "" {
		t.Fatal(saved)
	}
	raw, _ = json.Marshal(RateQuoteRequest{Version: saved.Version, Input: tokenNumber(0), Output: tokenNumber(0)})
	if err := os.WriteFile(input, raw, 0600); err != nil {
		t.Fatal(err)
	}
	out.Reset()
	if err := s.pricingCLI([]string{"pricing", "estimate"}, input, &out); err != nil {
		t.Fatal(err)
	}
	var q RateQuote
	json.Unmarshal(out.Bytes(), &q)
	if q.Amount == nil || *q.Amount != 0 {
		t.Fatal(q)
	}
}
