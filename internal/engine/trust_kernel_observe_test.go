//go:build linux

package engine

import (
	"bytes"
	"context"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestKernelObservePolicyDefault(t *testing.T) {
	p := kernelObservePolicy(nil)
	if bool(p.Enabled) || p.WindowSeconds != 5 || p.TimeoutSeconds != 15 || p.RingBytes != 65536 || p.MaxEvents != 128 {
		t.Fatal("unexpected visible defaults")
	}
	if err := validateKernelObservePolicy(&p); err != nil {
		t.Fatal(err)
	}
}
func TestKernelObservePolicyInvalid(t *testing.T) {
	for _, raw := range []string{`null`, `{}`, `{"enabled":false,"window_seconds":5,"timeout_seconds":15,"ring_bytes":65536,"max_events":0}`, `{"enabled":null,"window_seconds":5,"timeout_seconds":15,"ring_bytes":65536,"max_events":128}`, `{"enabled":false,"window_seconds":15,"timeout_seconds":15,"ring_bytes":65536,"max_events":128}`, `{"enabled":false,"window_seconds":5,"timeout_seconds":15,"ring_bytes":12345,"max_events":128}`} {
		var p TrustKernelObservePolicy
		err := json.Unmarshal([]byte(raw), &p)
		if err == nil {
			err = validateKernelObservePolicy(&p)
		}
		if err == nil {
			t.Fatal("invalid policy accepted")
		}
	}
}

func kernelObserveChange(t *testing.T, s *Store, enabled bool) TrustConfigChange {
	t.Helper()
	v, err := s.trustView()
	if err != nil {
		t.Fatal(err)
	}
	c := lifecycleChange(t, "alice", false, v.(map[string]any)["revision"].(int))
	p := kernelObservePolicy(nil)
	p.Enabled = trustOIDCEnabled(enabled)
	c.KernelObservation = &p
	return c
}
func TestKernelObservePolicyPersists(t *testing.T) {
	s := storeTest(t)
	c := kernelObserveChange(t, s, true)
	c.KernelObservation.WindowSeconds = 2
	c.KernelObservation.TimeoutSeconds = 9
	c.KernelObservation.RingBytes = 131072
	c.KernelObservation.MaxEvents = 17
	before := kernelObserveStateBytes(t, s)
	v, err := s.trustConfig(c, true)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(before, kernelObserveStateBytes(t, s)) {
		t.Fatal("preview mutated state")
	}
	if v.(map[string]any)["kernel_observation"].(TrustKernelObservePolicy) != *c.KernelObservation {
		t.Fatal("preview dropped selected values")
	}
	if _, err = s.trustConfig(c, false); err != nil {
		t.Fatal(err)
	}
	view, err := s.trustView()
	if err != nil {
		t.Fatal(err)
	}
	if view.(map[string]any)["kernel_observation"].(TrustKernelObservePolicy) != *c.KernelObservation {
		t.Fatal("persisted policy did not reopen")
	}
}
func TestKernelObserveRefusesBeforeRunner(t *testing.T) {
	for _, variant := range []string{"inactive", "token", "body", "missing"} {
		t.Run(variant, func(t *testing.T) {
			s, _ := kernelAttemptFixture(t)
			c := kernelObserveChange(t, s, variant != "inactive")
			if _, err := s.trustConfig(c, false); err != nil {
				t.Fatal(err)
			}
			before := kernelObserveStateBytes(t, s)
			raw := []byte(`{"agent_id":"missing"}`)
			token := trustAdminToken
			if variant == "token" {
				token = trustRequesterToken
			}
			if variant == "body" {
				raw = []byte(`{"agent_id":"missing","pid":1}`)
			}
			called := false
			_, err := s.trustKernelObserveUsing(context.Background(), raw, token, func(context.Context, TrustKernelObservePolicy, kernelObservationLease, func() error) (TrustKernelObserveResult, error) {
				called = true
				return TrustKernelObserveResult{}, nil
			})
			if err == nil || called || !bytes.Equal(before, kernelObserveStateBytes(t, s)) {
				t.Fatal("invalid observer reached runner or changed business state")
			}
		})
	}
}

func kernelObserveStateBytes(t *testing.T, s *Store) []byte {
	t.Helper()
	b, err := os.ReadFile(filepath.Join(s.root, ".swarm", "trust-actions.json"))
	if err != nil && !os.IsNotExist(err) {
		t.Fatal(err)
	}
	return b
}
func TestKernelObserveFreshRevision(t *testing.T) {
	s, a := kernelAttemptFixture(t)
	c := kernelObserveChange(t, s, true)
	if _, err := s.trustConfig(c, false); err != nil {
		t.Fatal(err)
	}
	raw, _ := json.Marshal(kernelObserveCommand{AgentID: a.ID})
	called := false
	_, err := s.trustKernelObserveUsing(context.Background(), raw, trustAdminToken, func(ctx context.Context, p TrustKernelObservePolicy, l kernelObservationLease, fresh func() error) (TrustKernelObserveResult, error) {
		called = true
		if err := fresh(); err != nil {
			t.Fatal(err)
		}
		c2 := kernelObserveChange(t, s, true)
		c2.KernelObservation.MaxEvents = 17
		if _, err := s.trustConfig(c2, false); err != nil {
			t.Fatal(err)
		}
		result := kernelObserveFixed(p, l)
		result.StopReason = "unavailable"
		result.CleanupVerified = true
		return result, nil
	})
	if !called || err == nil || commandFailure(err).Code != "revision_conflict" {
		t.Fatalf("stale observer result accepted: %v", err)
	}
}

func TestKernelObserveCLIResultGuards(t *testing.T) {
	l := kernelObservationLease{AgentID: "agent", Mission: "mission", Task: "task", Attempt: "attempt", PID: 123, Start: "42", ResourceDigest: strings.Repeat("a", 64)}
	p := kernelObservePolicy(nil)
	p.Enabled = true
	r := kernelObserveFixed(p, l)
	r.StopReason = "unavailable"
	r.CleanupVerified = true
	var out bytes.Buffer
	if err := printKernelObserve(&out, r); err == nil || commandFailure(err).Code != "kernel_unavailable" || out.Len() == 0 {
		t.Fatalf("unavailable result lost: %v", err)
	}
	raw, _ := json.Marshal(r)
	var fields map[string]any
	_ = json.Unmarshal(raw, &fields)
	for _, key := range []string{"available", "events", "cleanup_verified", "effective_policy", "attribution", "attempts", "complete"} {
		t.Run(key, func(t *testing.T) {
			copy := map[string]any{}
			for k, v := range fields {
				copy[k] = v
			}
			delete(copy, key)
			var output bytes.Buffer
			if printKernelObserve(&output, copy) == nil || output.Len() != 0 {
				t.Fatal("incomplete result accepted")
			}
		})
	}
	r.Available = true
	r.StopReason = "window"
	out.Reset()
	if err := printKernelObserve(&out, r); err != nil {
		t.Fatalf("quiet valid window refused: %v", err)
	}
	r.Complete = true
	out.Reset()
	if printKernelObserve(&out, r) == nil || out.Len() != 0 {
		t.Fatal("unfounded full coverage accepted")
	}
}
