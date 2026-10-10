//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func waitProvider(t *testing.T, body string) Provider {
	t.Helper()
	path := filepath.Join(t.TempDir(), "claude")
	if err := os.WriteFile(path, []byte("#!/bin/sh\n"+body+"\n"), 0700); err != nil {
		t.Fatal(err)
	}
	return Provider{Command: path}
}

func TestProviderWaitActiveThinkingSurvivesElapsedDuration(t *testing.T) {
	provider := waitProvider(t, `for i in 1 2 3 4 5 6; do printf '%s\n' '{"type":"system","subtype":"thinking_tokens"}'; sleep .12; done
printf '%s\n' '{"type":"result","result":"completed"}'`)
	start := time.Now()
	reply, err := runStructuredProviderPolicyClock(provider, nil, "prompt", "{}", nil, 300*time.Millisecond, 0, func() bool { return true }, nil, suspendAwareNow)
	if err != nil || reply != "completed" || time.Since(start) < 600*time.Millisecond {
		t.Fatal(reply, err)
	}
}

func TestProviderWaitSilenceAndUnproductiveOutput(t *testing.T) {
	for _, line := range []string{"", `printf '%s\n' '{"type":"system","subtype":"api_retry"}'`, `printf '%s\n' '{"type":"system","subtype":"init"}'`, `printf '%s\n' '{"type":"unknown","secret":"PRIVATE"}'`} {
		provider := waitProvider(t, "for i in 1 2 3 4 5 6 7; do "+line+"\nsleep .1; done")
		start := time.Now()
		_, err := runStructuredProviderPolicyClock(provider, nil, "prompt", "{}", nil, 250*time.Millisecond, 0, func() bool { return true }, nil, suspendAwareNow)
		if err == nil || !strings.Contains(err.Error(), "silence") || strings.Contains(err.Error(), "PRIVATE") || time.Since(start) > time.Second {
			t.Fatal(err)
		}
	}
}

func TestProviderWaitExplicitTotalAndRevocation(t *testing.T) {
	for _, tc := range []struct {
		name  string
		max   time.Duration
		valid func() bool
		want  string
	}{
		{"explicit maximum", 350 * time.Millisecond, func() bool { return true }, "délai"},
		{"manual stop without timers", 0, func() bool { return false }, "révoquée"},
	} {
		t.Run(tc.name, func(t *testing.T) {
			p := waitProvider(t, `while true; do printf '%s\n' '{"type":"system","subtype":"thinking_tokens"}'; sleep .1; done`)
			_, err := runStructuredProviderPolicyClock(p, nil, "prompt", "{}", nil, 0, tc.max, tc.valid, nil, suspendAwareNow)
			if err == nil || !strings.Contains(err.Error(), tc.want) {
				t.Fatal(err)
			}
		})
	}
	start := time.Now()
	p := waitProvider(t, "sleep 30")
	_, err := runStructuredProviderPolicyClock(p, nil, "prompt", "{}", nil, 0, 0, func() bool { return time.Since(start) < 350*time.Millisecond }, nil, suspendAwareNow)
	if err == nil || !strings.Contains(err.Error(), "révoquée") || time.Since(start) > time.Second {
		t.Fatal(err)
	}
}

func TestProviderWaitConfigPersistencePreviewReplayAndProduction(t *testing.T) {
	s := storeTest(t)
	c, err := s.providerWaitConfig()
	if err != nil {
		t.Fatal(err)
	}
	r := ProviderWaitChange{Schema: 1, EventID: "wait-settings", Revision: c.Revision, Values: ProviderWaitValues{Silence: 1, Maximum: 0, Lease: c.Values.Lease}, Reason: "Activity-based monitoring for provider calls"}
	if _, err = s.changeProviderWait(r, true); err != nil {
		t.Fatal(err)
	}
	before, _ := s.providerWaitConfig()
	if before.Revision != c.Revision {
		t.Fatal("preview wrote configuration")
	}
	after, err := s.changeProviderWait(r, false)
	if err != nil {
		t.Fatal(err)
	}
	replay, err := s.changeProviderWait(r, false)
	if err != nil || replay.Revision != after.Revision {
		t.Fatal("non-idempotent update", err)
	}
	conflict := r
	conflict.Values.Silence = 2
	if _, err = s.changeProviderWait(conflict, false); err == nil {
		t.Fatal("event conflict accepted")
	}
	stale := r
	stale.EventID = "stale-wait"
	if _, err = s.changeProviderWait(stale, false); err == nil {
		t.Fatal("stale revision accepted")
	}
	negative := stale
	negative.Revision = after.Revision
	negative.Values.Maximum = -1
	if _, err = s.changeProviderWait(negative, false); err == nil {
		t.Fatal("negative duration accepted")
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	read, err := reopened.providerWaitConfig()
	if err != nil || read.Values != r.Values {
		t.Fatal(read, err)
	}
	p := waitProvider(t, `for i in 1 2 3 4; do printf '%s\n' '{"type":"system","subtype":"thinking_tokens"}'; sleep .4; done
printf '%s\n' '{"type":"result","result":"completed"}'`)
	reply, err := reopened.runStructuredProvider(p, nil, "prompt", "{}", 0, func() bool { return true }, nil)
	if err != nil || reply != "completed" {
		t.Fatal("production ignored activity policy", reply, err)
	}
	data, _ := json.Marshal(ProviderWaitChange{Schema: 1, EventID: "restore-wait", Revision: read.Revision, Values: c.Values, Reason: "Restore initial monitoring values without erasing history"})
	var restore ProviderWaitChange
	json.Unmarshal(data, &restore)
	restored, err := s.changeProviderWait(restore, false)
	if err != nil || len(restored.History) != 2 || restored.Values != c.Values {
		t.Fatal("restore lost history", err)
	}
}

func TestProviderWaitPlannerRenewsOwnershipWithoutAnotherActivation(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	c, err := s.providerWaitConfig()
	if err != nil {
		t.Fatal(err)
	}
	_, err = s.changeProviderWait(ProviderWaitChange{Schema: 1, EventID: "short-lease", Revision: c.Revision, Values: ProviderWaitValues{Silence: 1, Maximum: 0, Lease: 5}, Reason: "Exercise renewal of a live planner ownership lease"}, false)
	if err != nil {
		t.Fatal(err)
	}
	response := `{"input_events":["enable-wait"],"reason":"Information examinée sans modification nécessaire","operations":[]}`
	envelope, _ := json.Marshal(map[string]any{"type": "result", "result": response})
	p := waitProvider(t, `cat >/dev/null
for i in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15; do printf '%s\n' '{"type":"system","subtype":"thinking_tokens"}'; sleep .4; done
printf '%s\n' '`+string(envelope)+`'`)
	providers, _ := json.Marshal(Providers{Schema: 1, Providers: map[string]Provider{"wait-planner": p}})
	if err = os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), providers, 0600); err != nil {
		t.Fatal(err)
	}
	w, err = s.planningChange(w.ID, "enable", PlanningRequest{Schema: 1, EventID: "enable-wait", Revision: w.Revision, MaxTasks: 10, MaxDecisions: 10, MaxActivations: 15, Provider: "wait-planner"})
	if err != nil {
		t.Fatal(err)
	}
	if err = s.planningStep(w.ID); err != nil {
		t.Fatal(err)
	}
	got, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	if got.Planning.Failure != "" || got.Planning.Activations != 1 || got.Planning.Decisions != 1 {
		t.Fatal("active planner lost its decision or spent twice", got.Planning)
	}
	events, err := s.events(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	found := false
	for _, e := range events {
		if e.Kind == "planning.lease-renew" {
			found = true
		}
	}
	if !found {
		t.Fatal("fixture did not exercise lease renewal")
	}
}

func TestProviderWaitLeaseCannotResurrectReplacedOrExpiredOwner(t *testing.T) {
	s, w := planningFixture(t)
	w, r := planningClaim(t, s, w, "root")
	if err := s.renewPlanningLease(w, "root", "another-owner", r.Generation, 60); err == nil {
		t.Fatal("replaced owner renewed")
	}
	if err := s.renewPlanningLease(w, "root", r.Holder, r.Generation+1, 60); err == nil {
		t.Fatal("wrong generation renewed")
	}
	var err error
	w, err = s.mutate(w.ID, "fixture-expiry", newID("expiry-"), w.Revision, []byte(`{}`), func(current *Work) error {
		current.Planning.Scopes[0].Until = time.Now().Add(-time.Second).Format(time.RFC3339Nano)
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	if err = s.renewPlanningLease(w, "root", r.Holder, r.Generation, 60); err == nil {
		t.Fatal("expired owner resurrected")
	}
	got, _ := s.get(w.ID)
	if got.Revision != w.Revision || got.Planning.Activations != w.Planning.Activations {
		t.Fatal("rejected renewal mutated ownership")
	}
}
