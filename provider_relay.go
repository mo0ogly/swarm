//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"os"
	"sort"
	"strings"
	"time"
)

const (
	providerWaitAction  = "wait"
	providerRelayAction = "relay"
)

// ProviderRelayDecision records an operator choice. It never launches a
// process: the normal agent start operation still applies every launch guard.
type ProviderRelayDecision struct {
	AgentID  string `json:"agent_id"`
	Action   string `json:"action"`
	Provider string `json:"provider,omitempty"`
	EventID  string `json:"event_id"`
	Digest   string `json:"view_digest"`
	Actor    string `json:"actor"`
	At       string `json:"at"`
}

type ProviderRelayRequest struct {
	Schema   int    `json:"schema_version"`
	EventID  string `json:"event_id"`
	Revision int    `json:"expected_revision"`
	Digest   string `json:"expected_view_digest"`
	Action   string `json:"action"`
	Provider string `json:"provider,omitempty"`
}

type ProviderRelayOption struct {
	Action   string `json:"action"`
	Provider string `json:"provider,omitempty"`
	Label    string `json:"label"`
}

type ProviderRelayAccounting struct {
	Attempts    int      `json:"attempts_consumed"`
	ReportedUSD *float64 `json:"reported_usd,omitempty"`
	WithCost    int      `json:"attempts_with_cost"`
	WithoutCost int      `json:"attempts_without_cost"`
	Complete    bool     `json:"cost_complete"`
	CostText    string   `json:"cost_text"`
}

type ProviderRelayView struct {
	Schema          int                                `json:"schema_version"`
	WorkID          string                             `json:"work_id"`
	TaskID          string                             `json:"task_id"`
	AgentID         string                             `json:"agent_id"`
	Revision        int                                `json:"revision"`
	Provider        string                             `json:"provider"`
	Cause           string                             `json:"cause"`
	CooldownKnown   bool                               `json:"cooldown_known"`
	CooldownUntil   string                             `json:"cooldown_until,omitempty"`
	AttemptsUsed    int                                `json:"attempts_consumed"`
	AttemptsAllowed int                                `json:"attempts_allowed"`
	Accounting      map[string]ProviderRelayAccounting `json:"accounting_by_provider"`
	Options         []ProviderRelayOption              `json:"admissible_options"`
	Decision        *ProviderRelayDecision             `json:"decision,omitempty"`
	Digest          string                             `json:"view_digest"`
}

func (s *Store) providerRelayView(agentID string) (ProviderRelayView, error) {
	a, err := s.agent(agentID)
	if err != nil {
		return ProviderRelayView{}, err
	}
	w, err := s.get(a.WorkID)
	if err != nil {
		return ProviderRelayView{}, err
	}
	t, err := w.task(a.TaskID)
	if err != nil {
		return ProviderRelayView{}, err
	}
	cooldown := a.ProviderCooldown
	if cooldown == nil {
		cooldown, _, err = s.providerCooldown(a.Provider)
		if err != nil {
			return ProviderRelayView{}, err
		}
	}
	if cooldown == nil || !cooldown.active(time.Now()) {
		return ProviderRelayView{}, fmt.Errorf("Cette tentative n’est pas bloquée par un quota fournisseur actif.")
	}
	agents, err := s.agents(w.ID)
	if err != nil {
		return ProviderRelayView{}, err
	}
	totals := map[string]CostTotal{}
	for _, item := range agents {
		if item.TaskID == a.TaskID {
			total := totals[item.Provider]
			if item.Usage != nil && item.Usage.ReportedCost != nil {
				total.Reported += *item.Usage.ReportedCost
				total.WithCost++
			} else {
				total.Silent++
			}
			totals[item.Provider] = total
		}
	}
	accounting := map[string]ProviderRelayAccounting{}
	for provider, total := range totals {
		item := ProviderRelayAccounting{Attempts: total.WithCost + total.Silent, WithCost: total.WithCost, WithoutCost: total.Silent, Complete: total.Silent == 0, CostText: total.Text()}
		if total.WithCost > 0 {
			reported := total.Reported
			item.ReportedUSD = &reported
		}
		accounting[provider] = item
	}
	if _, ok := accounting[a.Provider]; !ok {
		accounting[a.Provider] = ProviderRelayAccounting{Complete: true, CostText: (CostTotal{}).Text()}
	}
	options := []ProviderRelayOption{{Action: providerWaitAction, Label: "Attendre sans lancer de nouvelle tentative"}}
	providers, err := s.providers()
	if err != nil {
		return ProviderRelayView{}, err
	}
	ids := make([]string, 0, len(providers.Providers))
	for id := range providers.Providers {
		ids = append(ids, id)
	}
	sort.Strings(ids)
	for _, id := range ids {
		p := providers.Providers[id]
		if id == a.Provider || p.APIConnectionID != "" {
			continue
		}
		if t.ModelSelection != nil && t.ModelSelection.Provider != id {
			continue
		}
		info, statErr := os.Stat(p.Command)
		if statErr != nil || !info.Mode().IsRegular() || info.Mode()&0111 == 0 || s.providerCooldownGuard(id) != nil {
			continue
		}
		options = append(options, ProviderRelayOption{Action: providerRelayAction, Provider: id, Label: "Relayer explicitement vers le fournisseur configuré " + id})
	}
	cause := "quota refusé (429)"
	if cooldown.Signal != "" {
		cause += " · " + cooldown.Signal
	}
	view := ProviderRelayView{Schema: 1, WorkID: w.ID, TaskID: t.ID, AgentID: a.ID, Revision: w.Revision, Provider: a.Provider, Cause: cause,
		CooldownKnown: cooldown.ResetAt > 0, AttemptsUsed: len(t.Attempts), AttemptsAllowed: t.PlanMaxAttempts,
		Accounting: accounting, Options: options, Decision: t.ProviderRelayDecision}
	if cooldown.ResetAt > 0 {
		view.CooldownUntil = time.Unix(cooldown.ResetAt, 0).UTC().Format(time.RFC3339Nano)
	}
	unsigned := view
	unsigned.Digest = ""
	raw, _ := json.Marshal(unsigned)
	view.Digest = hash(raw)
	return view, nil
}

func (s *Store) decideProviderRelay(agentID string, request ProviderRelayRequest) (ProviderRelayView, error) {
	if request.Schema != 1 || !safeName(request.EventID) || request.Revision < 1 || request.Digest == "" {
		return ProviderRelayView{}, fmt.Errorf("schema_version, event_id, expected_revision et expected_view_digest requis")
	}
	if request.Action != providerWaitAction && request.Action != providerRelayAction {
		return ProviderRelayView{}, fmt.Errorf("action attendue : wait ou relay")
	}
	if request.Action == providerWaitAction && strings.TrimSpace(request.Provider) != "" {
		return ProviderRelayView{}, fmt.Errorf("wait ne sélectionne aucun fournisseur")
	}
	before, err := s.providerRelayView(agentID)
	if err != nil {
		return ProviderRelayView{}, err
	}
	if before.Decision != nil && before.Decision.EventID == request.EventID {
		if before.Decision.AgentID != agentID || before.Decision.Action != request.Action || before.Decision.Provider != request.Provider {
			return ProviderRelayView{}, fmt.Errorf("event_id déjà utilisé avec une autre décision")
		}
		return before, nil
	}
	if before.Revision != request.Revision || before.Digest != request.Digest {
		return ProviderRelayView{}, &CommandError{Code: "revision_conflict", Message: "Le diagnostic fournisseur a changé ; relire les options.", Retryable: true}
	}
	admissible := request.Action == providerWaitAction
	for _, option := range before.Options {
		if option.Action == request.Action && option.Provider == request.Provider {
			admissible = true
			break
		}
	}
	if !admissible {
		return ProviderRelayView{}, fmt.Errorf("Fournisseur absent des options admissibles : %s", request.Provider)
	}
	raw, _ := json.Marshal(request)
	_, err = s.mutate(before.WorkID, "provider.relay.decide", request.EventID, request.Revision, raw, func(w *Work) error {
		t, e := w.task(before.TaskID)
		if e != nil {
			return e
		}
		t.ProviderRelayDecision = &ProviderRelayDecision{AgentID: agentID, Action: request.Action, Provider: request.Provider, EventID: request.EventID, Digest: request.Digest, Actor: operatorIdentity(), At: now()}
		return nil
	})
	if err != nil {
		return ProviderRelayView{}, err
	}
	return s.providerRelayView(agentID)
}
