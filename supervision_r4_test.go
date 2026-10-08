//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func supervisionR4Fixture(t *testing.T, maxAttempts int, withRelay bool, knownCooldown bool, knownCost bool) (*Store, Work, Launch, Agent) {
	t.Helper()
	s := storeTest(t)
	w, launch := setupAgent(t, s)
	w.Tasks[0].PlanMaxAttempts = maxAttempts
	w.Tasks[0].PlanRole = "worker"
	raw, _ := json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	if withRelay {
		providers, err := s.providersFile()
		if err != nil {
			t.Fatal(err)
		}
		providers.Providers["relay-fixture"] = providers.Providers[launch.Provider]
		raw, _ = json.Marshal(providers)
		if err = os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
			t.Fatal(err)
		}
	}
	first, _, err := s.prepare(w.ID, launch)
	if err != nil {
		t.Fatal(err)
	}
	cooldown := &ProviderCooldown{ObservedAt: now(), Signal: "rate_limit_event.rejected"}
	if knownCooldown {
		cooldown.ResetAt = time.Now().Add(time.Hour).Unix()
	}
	if err = s.recordProviderCooldown(launch.Provider, first.ID, cooldown); err != nil {
		t.Fatal(err)
	}
	first.ProviderCooldown = cooldown
	if knownCost {
		cost := 1.25
		first.Usage = &Usage{ReportedCost: &cost, Source: "double fournisseur R4"}
	}
	if err = s.saveAgent(first); err != nil {
		t.Fatal(err)
	}
	if err = s.finishAgent(first, "failed", cooldown.message(), nil); err != nil {
		t.Fatal(err)
	}
	first, err = s.agent(first.ID)
	if err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	return s, w, launch, first
}

func r4Decision(view ProviderRelayView, event, action, provider string) ProviderRelayRequest {
	return ProviderRelayRequest{Schema: 1, EventID: event, Revision: view.Revision, Digest: view.Digest, Action: action, Provider: provider}
}

func TestSupervisionR4DiagnosticShowsKnownAndUnknownQuotaAccounting(t *testing.T) {
	for _, tc := range []struct {
		name, wantCost           string
		knownCooldown, knownCost bool
	}{
		{"known", "1.25 USD", true, true},
		{"unknown", "coût réel non rapporté", false, false},
	} {
		t.Run(tc.name, func(t *testing.T) {
			s, _, _, first := supervisionR4Fixture(t, 2, true, tc.knownCooldown, tc.knownCost)
			var out bytes.Buffer
			if err := agentCLI(s, []string{"providers", "relay", "show", first.ID}, "", "", true, &out); err != nil {
				t.Fatal(err)
			}
			var view ProviderRelayView
			if err := json.Unmarshal(out.Bytes(), &view); err != nil {
				t.Fatal(err)
			}
			current := view.Accounting[first.Provider]
			if view.Provider != first.Provider || !strings.Contains(view.Cause, "quota refusé (429)") || !strings.Contains(view.Cause, "rate_limit_event.rejected") || view.CooldownKnown != tc.knownCooldown || view.AttemptsUsed != 1 || current.Attempts != 1 || !strings.Contains(current.CostText, tc.wantCost) || len(view.Options) != 2 {
				t.Fatalf("diagnostic incomplet: %+v / %+v", view, current)
			}
			if tc.knownCost && (current.ReportedUSD == nil || *current.ReportedUSD != 1.25 || !current.Complete) || !tc.knownCost && (current.ReportedUSD != nil || current.Complete) {
				t.Fatal("coût connu/inconnu ambigu", current)
			}
			if tc.knownCooldown && view.CooldownUntil == "" || !tc.knownCooldown && view.CooldownUntil != "" {
				t.Fatal("cooldown connu/inconnu mal représenté", view.CooldownUntil)
			}
		})
	}
}

func TestSupervisionR4WaitRefusesRelayAndDecisionReplayIsIdempotent(t *testing.T) {
	s, w, launch, first := supervisionR4Fixture(t, 2, true, true, false)
	view, err := s.providerRelayView(first.ID)
	if err != nil {
		t.Fatal(err)
	}
	request := r4Decision(view, "r4-wait-choice", providerWaitAction, "")
	raw, _ := json.Marshal(request)
	input := filepath.Join(t.TempDir(), "wait.json")
	if err = os.WriteFile(input, raw, 0600); err != nil {
		t.Fatal(err)
	}
	var output bytes.Buffer
	if err = agentCLI(s, []string{"providers", "relay", "decide", first.ID}, input, "", true, &output); err != nil {
		t.Fatal("décision CLI refusée", err)
	}
	var decided ProviderRelayView
	if err = json.Unmarshal(output.Bytes(), &decided); err != nil || decided.Decision == nil || decided.Decision.Action != providerWaitAction {
		t.Fatal("refus non persisté", decided, err)
	}
	replayed, err := s.decideProviderRelay(first.ID, request)
	if err != nil || replayed.Revision != decided.Revision {
		t.Fatal("rejeu non idempotent", replayed, err)
	}
	launch.EventID = "r4-forbidden-relay"
	launch.Revision = decided.Revision
	launch.Previous = first.ID
	launch.Provider = "relay-fixture"
	if _, _, err = s.prepare(w.ID, launch); err == nil || !strings.Contains(err.Error(), "Relais fournisseur non autorisé") {
		t.Fatal("le refus a lancé un relais", err)
	}
	agents, _ := s.agents(w.ID)
	if len(agents) != 1 {
		t.Fatal("une tentative a été consommée malgré le refus", len(agents))
	}
}

func TestSupervisionR4ExplicitRelaySeparatesProviderHistoryAndUnknownCost(t *testing.T) {
	s, w, launch, first := supervisionR4Fixture(t, 2, true, true, true)
	view, err := s.providerRelayView(first.ID)
	if err != nil {
		t.Fatal(err)
	}
	decided, err := s.decideProviderRelay(first.ID, r4Decision(view, "r4-relay-choice", providerRelayAction, "relay-fixture"))
	if err != nil || decided.Decision == nil || decided.Decision.Provider != "relay-fixture" {
		t.Fatal("acceptation non persistée", decided, err)
	}
	launch.EventID = "r4-explicit-relay"
	launch.Revision = decided.Revision
	launch.Previous = first.ID
	launch.Provider = "relay-fixture"
	second, created, err := s.prepare(w.ID, launch)
	if err != nil || !created || second.Previous != first.ID {
		t.Fatal("relais explicite refusé", second, err)
	}
	summary, err := s.costSummary(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	if summary.ByProvider[first.Provider].Reported != 1.25 || summary.ByProvider[first.Provider].WithCost != 1 || summary.ByProvider["relay-fixture"].Silent != 1 || summary.ByProvider["relay-fixture"].Reported != 0 {
		t.Fatalf("historiques/coûts fusionnés ou coût inconnu inventé: %+v", summary.ByProvider)
	}
	agents, _ := s.agents(w.ID)
	if len(agents) != 2 || agents[0].Provider == agents[1].Provider {
		t.Fatal("historique fournisseur non séparé", agents)
	}
}

func TestSupervisionR4NoAdmissibleProviderRejectsRelay(t *testing.T) {
	s, _, _, first := supervisionR4Fixture(t, 2, false, false, false)
	view, err := s.providerRelayView(first.ID)
	if err != nil {
		t.Fatal(err)
	}
	if len(view.Options) != 1 || view.Options[0].Action != providerWaitAction {
		t.Fatal("option fournisseur inventée", view.Options)
	}
	_, err = s.decideProviderRelay(first.ID, r4Decision(view, "r4-missing-relay", providerRelayAction, "unconfigured"))
	if err == nil || !strings.Contains(err.Error(), "absent des options admissibles") {
		t.Fatal("fournisseur non configuré accepté", err)
	}
}

func TestSupervisionR4ExhaustedAttemptsStayExhaustedAfterRelayChoice(t *testing.T) {
	s, w, launch, first := supervisionR4Fixture(t, 1, true, true, false)
	view, err := s.providerRelayView(first.ID)
	if err != nil {
		t.Fatal(err)
	}
	decided, err := s.decideProviderRelay(first.ID, r4Decision(view, "r4-exhausted-choice", providerRelayAction, "relay-fixture"))
	if err != nil {
		t.Fatal(err)
	}
	launch.EventID = "r4-exhausted-launch"
	launch.Revision = decided.Revision
	launch.Previous = first.ID
	launch.Provider = "relay-fixture"
	if _, _, err = s.prepare(w.ID, launch); err == nil || !strings.Contains(err.Error(), "Plafond du plan atteint") {
		t.Fatal("tentative épuisée remboursée", err)
	}
	agents, _ := s.agents(w.ID)
	current, _ := s.get(w.ID)
	if len(agents) != 1 || len(current.Tasks[0].Attempts) != 1 || current.Tasks[0].ProviderRelayDecision == nil {
		t.Fatal("budget, historique ou décision perdus", len(agents), current.Tasks[0])
	}
}
