//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"testing"
)

func TestCursorEngineT3ClaimUsesEffectiveScopeProviderCooldown(t *testing.T) {
	for _, tc := range []struct {
		name, scope, selection, cooling string
		blocked                         bool
	}{
		{"root_override_ignores_base_wait", "root", "alternate", "base", false},
		{"root_override_respects_own_wait", "root", "alternate", "alternate", true},
		{"root_inherits_base_wait", "root", "", "base", true},
		{"child_override_ignores_base_wait", "child", "alternate", "base", false},
		{"child_override_respects_own_wait", "child", "alternate", "alternate", true},
		{"child_without_override_inherits_base", "child", "", "base", true},
	} {
		t.Run(tc.name, func(t *testing.T) {
			s, _, provider := taskModelFixture(t)
			raw, _ := json.Marshal(Providers{Schema: 1, Providers: map[string]Provider{"base": provider, "alternate": provider}})
			if e := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); e != nil {
				t.Fatal(e)
			}
			w := createTest(t, s)
			w = planningDo(t, s, w, "enable", PlanningRequest{Provider: "base", MaxTasks: 10, MaxDecisions: 10, MaxActivations: 15})
			if tc.scope == "child" {
				var proposal PlanningRequest
				w, proposal = planningClaim(t, s, w, "root")
				proposal.Operations = []PlanningOperation{{Kind: "delegate", ID: "child", Title: "Child scope", Requirements: []string{"req-1"}, Next: "Inspect assigned sources"}}
				var err error
				w, err = s.planningChange(w.ID, "decide", proposal)
				if err != nil {
					t.Fatal(err)
				}
			}
			if tc.selection != "" {
				if e := s.pause(w.ID, true); e != nil {
					t.Fatal(e)
				}
				w, _ = s.get(w.ID)
				_, route, e := resolveModel(provider, "standard", "planning")
				if e != nil {
					t.Fatal(e)
				}
				req := RoleModelRequest{Schema: 1, EventID: "choose-alternate", Revision: w.Revision, Scope: tc.scope, Provider: tc.selection, Level: "standard", PolicyHash: route.PolicyHash}
				if _, e = s.previewRoleModel(w.ID, req); e != nil {
					t.Fatal(e)
				}
				w, e = s.configureRoleModel(w.ID, req)
				if e != nil {
					t.Fatal(e)
				}
				if e = s.pause(w.ID, false); e != nil {
					t.Fatal(e)
				}
				w, _ = s.get(w.ID)
			}
			originalWait := futureCooldown(t, s, tc.cooling)
			owner, _ := w.Planning.scope(tc.scope)
			req := PlanningRequest{Schema: 1, EventID: "effective-provider-claim", Revision: w.Revision, Scope: tc.scope, ScopeRevision: owner.Revision, Holder: "selected-provider-holder", LeaseSeconds: 60}
			got, e := s.planningChange(w.ID, "claim", req)
			if tc.blocked {
				if e == nil || commandFailure(e).Code != "provider_cooldown" {
					t.Fatalf("effective provider wait ignored: %v", e)
				}
				got, _ = s.get(w.ID)
				if got.Revision != w.Revision || got.Planning.Activations != w.Planning.Activations {
					t.Fatal("blocked claim consumed reservation")
				}
			} else {
				if e != nil {
					t.Fatalf("unrelated base provider blocks selected scope: %v", e)
				}
				owner, _ = got.Planning.scope(tc.scope)
				if got.Planning.Activations != w.Planning.Activations+1 || owner.LastModel.Provider != tc.selection || got.Planning.Provider != "base" {
					t.Fatal("wrong provider or reservation", owner)
				}
				// Completed operation replay is not a new call, including after a fresh wait.
				futureCooldown(t, s, tc.selection)
				replay, e := s.planningChange(w.ID, "claim", req)
				if e != nil || replay.Revision != got.Revision || replay.Planning.Activations != got.Planning.Activations {
					t.Fatal("claim replay spent or blocked", e)
				}
			}
			saved, _, e := s.providerCooldown(tc.cooling)
			if e != nil || !reflect.DeepEqual(saved, originalWait) {
				t.Fatal("other provider wait changed", e)
			}
		})
	}
}
