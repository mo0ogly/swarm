//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"syscall"
	"time"
)

// ProviderCooldown is an observed provider rejection, not an inferred quota
// from a tool transcript. Zero ResetAt means the provider supplied no reset.
// It stays blocked until an explicit, reasoned operator clearance.
type ProviderCooldown struct {
	Provider    string `json:"provider"`
	ObservedAt  string `json:"observed_at"`
	ResetAt     int64  `json:"resets_at,omitempty"`
	Source      string `json:"source"`
	Signal      string `json:"signal"`
	ClearedAt   string `json:"cleared_at,omitempty"`
	ClearEvent  string `json:"clear_event,omitempty"`
	ClearReason string `json:"clear_reason,omitempty"`
	ClearActor  string `json:"clear_actor,omitempty"`
}

func observedProviderCooldown(event map[string]any, at time.Time) *ProviderCooldown {
	signal := ""
	reset := int64(0)
	switch event["type"] {
	case "rate_limit_event":
		info, ok := event["rate_limit_info"].(map[string]any)
		if !ok || info["status"] != "rejected" {
			return nil
		}
		signal = "rate_limit_event.rejected"
		if n, ok := info["resetsAt"].(float64); ok && n > 0 && n == float64(int64(n)) && n < 253402300800 {
			reset = int64(n)
		}
	case "result":
		if event["is_error"] != true || event["api_error_status"] != float64(429) {
			return nil
		}
		signal = "result.api_error_status.429"
	default:
		return nil
	}
	return &ProviderCooldown{ObservedAt: at.UTC().Format(time.RFC3339Nano), ResetAt: reset, Signal: signal}
}
func (c ProviderCooldown) active(at time.Time) bool {
	return c.ClearedAt == "" && (c.ResetAt == 0 || at.Unix() < c.ResetAt)
}
func (c ProviderCooldown) message() string {
	provider := c.Provider
	if provider == "" {
		provider = "IA"
	}
	prefix := "Fournisseur " + provider + " : quota refusé (429). "
	if c.ResetAt > 0 {
		return prefix + "Nouveaux appels suspendus jusqu’au " + time.Unix(c.ResetAt, 0).UTC().Format("2006-01-02 15:04:05 UTC") + ". Les tentatives et budgets déjà consommés restent conservés."
	}
	return prefix + "Aucune heure de reprise fournie. Vérifier le compte puis lever explicitement cette attente ; aucune relance automatique."
}
func (c ProviderCooldown) failure() error {
	return &CommandError{Code: "provider_cooldown", Message: c.message(), Retryable: false}
}
func (s *Store) providerCooldownPath(provider string) (string, error) {
	if !safeName(provider) {
		return "", fmt.Errorf("identifiant de fournisseur invalide")
	}
	return filepath.Join(s.root, ".swarm", "provider-cooldowns", provider+".json"), nil
}
func (s *Store) providerCooldown(provider string) (*ProviderCooldown, string, error) {
	path, e := s.providerCooldownPath(provider)
	if e != nil {
		return nil, "", e
	}
	data, e := os.ReadFile(path)
	if os.IsNotExist(e) {
		return nil, "", nil
	}
	if e != nil {
		return nil, "", e
	}
	var c ProviderCooldown
	if e = strict(data, &c); e != nil {
		return nil, "", fmt.Errorf("attente fournisseur illisible : %w", e)
	}
	if c.Provider != provider || c.ObservedAt == "" || c.Source == "" || c.Signal == "" || c.ResetAt < 0 {
		return nil, "", fmt.Errorf("attente fournisseur incohérente")
	}
	return &c, hash(data), nil
}
func (s *Store) providerCooldownGuardAt(provider string, at time.Time) error {
	c, _, e := s.providerCooldown(provider)
	if e != nil {
		return e
	}
	if c != nil && c.active(at) {
		return c.failure()
	}
	return nil
}
func (s *Store) providerCooldownGuard(provider string) error {
	return s.providerCooldownGuardAt(provider, time.Now())
}
func (s *Store) lockProviderCooldown(provider string) (func(), error) {
	path, e := s.providerCooldownPath(provider)
	if e != nil {
		return nil, e
	}
	if e = os.MkdirAll(filepath.Dir(path), 0700); e != nil {
		return nil, e
	}
	f, e := os.OpenFile(path+".lock", os.O_CREATE|os.O_RDWR, 0600)
	if e != nil {
		return nil, e
	}
	if e = syscall.Flock(int(f.Fd()), syscall.LOCK_EX); e != nil {
		f.Close()
		return nil, e
	}
	return func() { _ = syscall.Flock(int(f.Fd()), syscall.LOCK_UN); _ = f.Close() }, nil
}
func (s *Store) recordProviderCooldown(provider, source string, c *ProviderCooldown) error {
	unlock, e := s.lockProviderCooldown(provider)
	if e != nil {
		return e
	}
	defer unlock()
	old, digest, e := s.providerCooldown(provider)
	if e != nil {
		return e
	}
	c.Provider = provider
	c.Source = source
	observed, e := time.Parse(time.RFC3339Nano, c.ObservedAt)
	if e != nil {
		return fmt.Errorf("horodatage quota invalide")
	}
	if old != nil {
		previous, e := time.Parse(time.RFC3339Nano, old.ObservedAt)
		if e != nil {
			return e
		}
		cleared, _ := time.Parse(time.RFC3339Nano, old.ClearedAt)
		if observed.Before(previous) || (!cleared.IsZero() && !observed.After(cleared)) {
			return nil
		}
		if old.Source == source && old.ObservedAt == c.ObservedAt && old.Signal == c.Signal && old.ResetAt == c.ResetAt {
			return nil
		}
	}
	// The terminal 429 result belongs to the same rejected call and often omits
	// resetsAt. Preserve that call's authoritative event timestamp, never parse
	// a displayed clock time. A different unknown rejection remains unknown.
	if old != nil && old.ClearedAt == "" {
		if old.Source == source && c.ResetAt == 0 {
			c.ResetAt = old.ResetAt
		}
		if old.active(time.Now()) && old.ResetAt > c.ResetAt && c.ResetAt > 0 {
			c.ResetAt = old.ResetAt
		}
		if old.ResetAt == 0 && old.Source != source {
			c.ResetAt = 0
		}
	}
	path, _ := s.providerCooldownPath(provider)
	if old != nil {
		history := filepath.Join(filepath.Dir(path), "history", provider)
		if e = os.MkdirAll(history, 0700); e != nil {
			return e
		}
		raw, _ := json.MarshalIndent(old, "", "  ")
		if e = atomicWrite(filepath.Join(history, digest+".json"), raw); e != nil {
			return e
		}
	}
	raw, e := json.MarshalIndent(c, "", "  ")
	if e != nil {
		return e
	}
	return atomicWrite(path, raw)
}
func (s *Store) providerCooldownObserver(provider, source string) func(*ProviderCooldown) error {
	return func(c *ProviderCooldown) error { return s.recordProviderCooldown(provider, source, c) }
}

type ProviderCooldownClear struct {
	Schema int    `json:"schema_version"`
	Event  string `json:"event_id"`
	Digest string `json:"expected_digest"`
	Reason string `json:"reason"`
}

func (s *Store) clearProviderCooldown(provider string, r ProviderCooldownClear) (*ProviderCooldown, error) {
	if r.Schema != 1 || !safeName(r.Event) || len(strings.TrimSpace(r.Reason)) < 8 || len(r.Reason) > 2000 {
		return nil, fmt.Errorf("schema_version, event_id et justification de 8 à 2000 caractères requis")
	}
	unlock, e := s.lockProviderCooldown(provider)
	if e != nil {
		return nil, e
	}
	defer unlock()
	c, digest, e := s.providerCooldown(provider)
	if e != nil {
		return nil, e
	}
	if c == nil {
		return nil, fmt.Errorf("aucune attente enregistrée")
	}
	if c.ClearEvent == r.Event {
		if c.ClearReason != r.Reason {
			return nil, fmt.Errorf("event_id déjà utilisé avec un autre motif")
		}
		return c, nil
	}
	if r.Digest == "" || digest != r.Digest {
		return nil, fmt.Errorf("attente fournisseur modifiée ; relire son état")
	}
	if c.ResetAt > 0 && c.active(time.Now()) {
		return nil, c.failure()
	}
	path, _ := s.providerCooldownPath(provider)
	history := filepath.Join(filepath.Dir(path), "history", provider)
	if e = os.MkdirAll(history, 0700); e != nil {
		return nil, e
	}
	raw, _ := json.MarshalIndent(c, "", "  ")
	if e = atomicWrite(filepath.Join(history, digest+".json"), raw); e != nil {
		return nil, e
	}
	c.ClearedAt = now()
	c.ClearEvent = r.Event
	c.ClearReason = r.Reason
	c.ClearActor = fmt.Sprintf("local-uid-%d", os.Getuid())
	raw, _ = json.MarshalIndent(c, "", "  ")
	if e = atomicWrite(path, raw); e != nil {
		return nil, e
	}
	return c, nil
}

// Reconciliation reads only whole structured provider output events. A tool's
// text or nested result is never treated as a provider's control-plane signal.
// Historical observations cannot override a newer observation or operator clear.
func (s *Store) reconcileProviderCooldown(a *Agent) error {
	if activeAgent(*a) || a.Provider == "" || a.ProviderCooldown != nil {
		return nil
	}
	after := int64(-1)
	var found *ProviderCooldown
	for page := 0; page < 11; page++ {
		logs, e := s.logs(a.ID, after)
		if e != nil {
			return e
		}
		if len(logs) == 0 {
			break
		}
		for _, entry := range logs {
			after = entry.Seq
			if entry.Kind != "output" {
				continue
			}
			at, e := time.Parse(time.RFC3339Nano, entry.At)
			if e != nil {
				continue
			}
			var event map[string]any
			if json.Unmarshal([]byte(entry.Message), &event) != nil {
				continue
			}
			c := observedProviderCooldown(event, at)
			if c == nil {
				continue
			}
			if found != nil && c.ResetAt == 0 {
				c.ResetAt = found.ResetAt
			}
			found = c
		}
		if len(logs) < 200 {
			break
		}
	}
	if found == nil {
		return nil
	}
	if e := s.recordProviderCooldown(a.Provider, a.ID, found); e != nil {
		return e
	}
	a.ProviderCooldown = found
	if a.Status == "failed" || a.Status == "interrupted" {
		a.StopKind = "provider_quota"
		a.Activity = found.message()
		finalizeRecoveryState(a)
	}
	if e := s.saveAgent(*a); e != nil {
		return e
	}
	return s.log(a.ID, "provider-cooldown", "Refus fournisseur relu dans les événements structurés : "+found.message())
}
