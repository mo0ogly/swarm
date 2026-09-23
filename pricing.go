//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"io"
	"math"
	"net/http"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"syscall"
	"time"
)

// Rates are explicit prices per million tokens, never bundled model defaults.
type ModelRate struct {
	Version       string   `json:"version"`
	Provider      string   `json:"provider"`
	Model         string   `json:"model"`
	Currency      string   `json:"currency"`
	Input         *float64 `json:"input_per_million"`
	Output        *float64 `json:"output_per_million"`
	CacheRead     *float64 `json:"cache_read_per_million"`
	CacheWrite    *float64 `json:"cache_write_per_million"`
	Source        string   `json:"source"`
	ReferenceDate string   `json:"reference_date"`
	Actor         string   `json:"actor"`
	Updated       string   `json:"updated"`
}
type RateCatalogue struct {
	Schema int         `json:"schema_version"`
	Digest string      `json:"digest"`
	Rates  []ModelRate `json:"rates"`
}
type RateChange struct {
	Schema   int       `json:"schema_version"`
	Expected string    `json:"expected_digest"`
	Rate     ModelRate `json:"rate"`
}
type RateQuoteRequest struct {
	Version string `json:"rate_version"`
	// Non-cached input is explicit: provider totals have incompatible cache semantics.
	Input      *int64 `json:"non_cached_input_tokens"`
	Output     *int64 `json:"output_tokens"`
	CacheRead  int64  `json:"cache_read_tokens"`
	CacheWrite int64  `json:"cache_write_tokens"`
}
type RateQuote struct {
	Status  string           `json:"status"`
	Amount  *float64         `json:"estimated_amount"`
	Rate    ModelRate        `json:"rate"`
	Tokens  RateQuoteRequest `json:"tokens"`
	Missing []string         `json:"missing_rates"`
}

func (s *Store) rateCatalogue() (RateCatalogue, error) {
	c := RateCatalogue{Schema: 1, Rates: []ModelRate{}}
	p, err := localFile(s.root, ".swarm/model-rates.json")
	if os.IsNotExist(err) {
		c.Digest = hash(nil)
		return c, nil
	}
	if err != nil {
		return c, err
	}
	b, err := os.ReadFile(p)
	if os.IsNotExist(err) {
		c.Digest = hash(nil)
		return c, nil
	}
	if err != nil {
		return c, err
	}
	if len(b) > 2*1024*1024 {
		return c, fmt.Errorf("Catalogue de tarifs trop volumineux.")
	}
	if err = strict(b, &c); err != nil {
		return c, err
	}
	if c.Schema != 1 {
		return c, fmt.Errorf("Version du catalogue de tarifs non prise en charge.")
	}
	c.Digest = hash(b)
	return c, nil
}
func validateRate(r ModelRate) error {
	if !safeName(r.Provider) || !modelName.MatchString(r.Model) || len(r.Model) > 200 || !regexp.MustCompile(`^[A-Z]{3}$`).MatchString(r.Currency) {
		return fmt.Errorf("Fournisseur, modèle et devise à trois lettres requis.")
	}
	if strings.TrimSpace(r.Source) == "" || len(r.Source) > 2000 {
		return fmt.Errorf("Source documentée du tarif requise.")
	}
	if _, err := time.Parse("2006-01-02", r.ReferenceDate); err != nil {
		return fmt.Errorf("Date de référence au format AAAA-MM-JJ requise.")
	}
	for _, n := range []*float64{r.Input, r.Output, r.CacheRead, r.CacheWrite} {
		if n != nil && (math.IsNaN(*n) || math.IsInf(*n, 0) || *n < 0 || *n > 1e9) {
			return fmt.Errorf("Les tarifs doivent être finis, positifs ou nuls et au plus égaux à un milliard par million de jetons.")
		}
	}
	if r.Input == nil && r.Output == nil && r.CacheRead == nil && r.CacheWrite == nil {
		return fmt.Errorf("Au moins un tarif explicite requis ; vide signifie inconnu, zéro signifie gratuit.")
	}
	return nil
}
func (s *Store) saveRate(r RateChange) (ModelRate, error) {
	if r.Schema != 1 {
		return ModelRate{}, fmt.Errorf("schema_version doit valoir 1.")
	}
	if err := validateRate(r.Rate); err != nil {
		return ModelRate{}, err
	}
	lock, err := os.OpenFile(filepath.Join(s.root, ".swarm/model-rates.lock"), os.O_CREATE|os.O_RDWR, 0600)
	if err != nil {
		return ModelRate{}, err
	}
	defer lock.Close()
	if err = syscall.Flock(int(lock.Fd()), syscall.LOCK_EX); err != nil {
		return ModelRate{}, err
	}
	defer syscall.Flock(int(lock.Fd()), syscall.LOCK_UN)
	c, err := s.rateCatalogue()
	if err != nil {
		return ModelRate{}, err
	}
	if r.Expected != c.Digest {
		return ModelRate{}, &CommandError{Code: "revision_conflict", Message: "Tarifs modifiés : actualisez avant d’enregistrer.", Retryable: true}
	}
	if len(c.Rates) >= 1000 {
		return ModelRate{}, fmt.Errorf("Limite de 1 000 versions atteinte ; les versions existantes sont conservées.")
	}
	// Append-only: editing a price never changes the meaning of earlier quotes.
	r.Rate.Version = newID("rate-")
	r.Rate.Actor = operatorIdentity()
	r.Rate.Updated = now()
	c.Rates = append(c.Rates, r.Rate)
	c.Digest = ""
	b, err := json.MarshalIndent(c, "", "  ")
	if err != nil {
		return ModelRate{}, err
	}
	if len(b) > 2*1024*1024 {
		return ModelRate{}, fmt.Errorf("Catalogue de tarifs trop volumineux.")
	}
	err = atomicWrite(filepath.Join(s.root, ".swarm/model-rates.json"), b)
	return r.Rate, err
}
func (s *Store) quoteRate(r RateQuoteRequest) (RateQuote, error) {
	q := RateQuote{Status: "unknown", Tokens: r, Missing: []string{}}
	if r.Input == nil || r.Output == nil {
		return q, fmt.Errorf("Renseignez explicitement les nombres de jetons d’entrée hors cache et de sortie.")
	}
	for _, n := range []int64{*r.Input, *r.Output, r.CacheRead, r.CacheWrite} {
		if n < 0 || n > 1e12 {
			return q, fmt.Errorf("Les nombres de jetons doivent être compris entre zéro et mille milliards.")
		}
	}
	c, err := s.rateCatalogue()
	if err != nil {
		return q, err
	}
	found := false
	for _, rate := range c.Rates {
		if rate.Version == r.Version {
			q.Rate = rate
			found = true
			break
		}
	}
	if !found {
		return q, fmt.Errorf("Version du tarif inconnue.")
	}
	sum := 0.0
	for _, part := range []struct {
		name   string
		tokens int64
		rate   *float64
	}{{"input", *r.Input, q.Rate.Input}, {"output", *r.Output, q.Rate.Output}, {"cache_read", r.CacheRead, q.Rate.CacheRead}, {"cache_write", r.CacheWrite, q.Rate.CacheWrite}} {
		if part.tokens == 0 {
			continue
		}
		if part.rate == nil {
			q.Missing = append(q.Missing, part.name)
		} else {
			sum += float64(part.tokens) * (*part.rate) / 1e6
		}
	}
	if len(q.Missing) == 0 {
		q.Status = "estimated"
		q.Amount = &sum
	}
	return q, nil
}
func (s *Store) pricingCLI(pos []string, input string, out io.Writer) error {
	if len(pos) != 2 {
		return fmt.Errorf("swarm pricing list|save|estimate [--input request.json]")
	}
	if pos[1] == "list" {
		c, err := s.rateCatalogue()
		if err != nil {
			return err
		}
		return printJSON(out, c)
	}
	if pos[1] != "save" && pos[1] != "estimate" {
		return fmt.Errorf("Action tarifaire inconnue.")
	}
	raw, err := readInput(input)
	if err != nil {
		return err
	}
	if pos[1] == "save" {
		var r RateChange
		if err = strict(raw, &r); err != nil {
			return err
		}
		v, err := s.saveRate(r)
		if err != nil {
			return err
		}
		return printJSON(out, v)
	}
	var r RateQuoteRequest
	if err = strict(raw, &r); err != nil {
		return err
	}
	v, err := s.quoteRate(r)
	if err != nil {
		return err
	}
	return printJSON(out, v)
}
func (s *Store) registerPricing(mux *http.ServeMux, send func(http.ResponseWriter, any), fail func(http.ResponseWriter, error)) {
	mux.HandleFunc("/api/v1/pricing", func(w http.ResponseWriter, r *http.Request) {
		if r.Method == "GET" {
			v, err := s.rateCatalogue()
			if err != nil {
				fail(w, err)
			} else {
				send(w, v)
			}
			return
		}
		if r.Method != "POST" {
			http.Error(w, "GET or POST required", 405)
			return
		}
		var v RateChange
		b, err := io.ReadAll(http.MaxBytesReader(w, r.Body, 12000))
		if err == nil {
			err = strict(b, &v)
		}
		if err != nil {
			fail(w, err)
			return
		}
		saved, err := s.saveRate(v)
		if err != nil {
			fail(w, err)
		} else {
			send(w, saved)
		}
	})
	mux.HandleFunc("/api/v1/pricing/estimate", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" {
			http.Error(w, "POST required", 405)
			return
		}
		var v RateQuoteRequest
		b, err := io.ReadAll(http.MaxBytesReader(w, r.Body, 12000))
		if err == nil {
			err = strict(b, &v)
		}
		if err != nil {
			fail(w, err)
			return
		}
		q, err := s.quoteRate(v)
		if err != nil {
			fail(w, err)
		} else {
			send(w, q)
		}
	})
}
