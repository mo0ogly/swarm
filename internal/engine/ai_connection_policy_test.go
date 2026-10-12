//go:build linux

package engine

import (
	"context"
	"encoding/json"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

type deadlineProbeTransport func(*http.Request) (*http.Response, error)

func (f deadlineProbeTransport) RoundTrip(r *http.Request) (*http.Response, error) { return f(r) }

func TestAIConnectionTransportDoesNotInventDeadline(t *testing.T) {
	previous := http.DefaultTransport
	defer func() { http.DefaultTransport = previous }()
	var receivedDeadline time.Time
	var receivedBound bool
	http.DefaultTransport = deadlineProbeTransport(func(r *http.Request) (*http.Response, error) {
		receivedDeadline, receivedBound = r.Context().Deadline()
		return &http.Response{StatusCode: 200, Body: io.NopCloser(strings.NewReader(`{"choices":[{"message":{"content":"OK"}}]}`)), Header: http.Header{}}, nil
	})
	c := AIConnection{ID: "fixture", Label: "Fixture", Model: "fixture", BaseURL: "http://fixture.invalid/v1"}
	for _, maximum := range []time.Duration{0, 123 * time.Second} {
		ctx := context.Background()
		cancel := func() {}
		if maximum > 0 {
			ctx, cancel = context.WithTimeout(ctx, maximum)
		}
		expectedDeadline, expectedBound := ctx.Deadline()
		text, err := callAIConnection(ctx, c, "Fixture", "")
		cancel()
		if err != nil || text != "OK" || receivedBound != expectedBound || expectedBound && !receivedDeadline.Equal(expectedDeadline) {
			t.Fatal("HTTP client imposed another policy", maximum, receivedBound, receivedDeadline, expectedDeadline, text, err)
		}
	}
}

func TestAIConnectionPublicTestReceivesPersistedTotalPolicy(t *testing.T) {
	s := storeTest(t)
	previous := http.DefaultTransport
	defer func() { http.DefaultTransport = previous }()
	var bounded bool
	var remaining time.Duration
	http.DefaultTransport = deadlineProbeTransport(func(r *http.Request) (*http.Response, error) {
		deadline, ok := r.Context().Deadline()
		bounded = ok
		remaining = time.Until(deadline)
		return &http.Response{StatusCode: 200, Body: io.NopCloser(strings.NewReader(`{"choices":[{"message":{"content":"OK"}}]}`)), Header: http.Header{}}, nil
	})
	h := newWebHandler(s, "local.test", "policy-token")
	for _, maximum := range []int{0, 123} {
		current, err := s.providerWaitConfig()
		if err != nil {
			t.Fatal(err)
		}
		current.Values.Maximum = maximum
		_, err = s.changeProviderWait(ProviderWaitChange{Schema: 1, EventID: newID("policy-"), Revision: current.Revision, Values: current.Values, Reason: "Explicit common total duration in isolated fixture"}, false)
		if err != nil {
			t.Fatal(err)
		}
		raw, _ := json.Marshal(AIConnectionChange{Version: 1, Connection: AIConnection{ID: "fixture", Label: "Fixture", Model: "fixture", BaseURL: "http://fixture.invalid/v1"}})
		r := runLimitsWebRequest(h, "POST", "/api/v1/providers/connections/test", "policy-token", raw)
		if r.Code != 200 || !strings.Contains(r.Body.String(), `"ok":true`) || bounded != (maximum > 0) || maximum > 0 && (remaining < 122*time.Second || remaining > 123*time.Second) {
			t.Fatal("persisted policy not received", maximum, bounded, remaining, r.Code, r.Body.String())
		}
	}
}

func assistPolicyFixture(t *testing.T, s *Store, explicit int) (Work, AssistRequest) {
	t.Helper()
	w := assistWork(t, s)
	command := filepath.Join(t.TempDir(), "claude")
	if err := os.WriteFile(command, []byte("#!/bin/sh\nexit 0\n"), 0700); err != nil {
		t.Fatal(err)
	}
	raw, _ := json.Marshal(Providers{Schema: 1, Providers: map[string]Provider{"fixture": {Command: command, AssistantTimeout: explicit}}})
	if err := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	return w, AssistRequest{Revision: w.Revision, Coordinates: PageCoordinates{PageID: "tasks"}, TemplateID: "understand_page.v1", Provider: "fixture"}
}

func TestPageAssistanceInheritsTotalPolicyWithoutSecondCeiling(t *testing.T) {
	for _, maximum := range []int{0, 701} {
		s := storeTest(t)
		w, request := assistPolicyFixture(t, s, 0)
		current, err := s.providerWaitConfig()
		if err != nil {
			t.Fatal(err)
		}
		current.Values.Maximum = maximum
		if _, err = s.changeProviderWait(ProviderWaitChange{Schema: 1, EventID: newID("policy-"), Revision: current.Revision, Values: current.Values, Reason: "Explicit assistant waiting policy"}, false); err != nil {
			t.Fatal(err)
		}
		p, err := s.assistPreview(w.ID, request)
		if err != nil || p.Turn.TimeoutSeconds != maximum {
			t.Fatal("hidden assistance limit", maximum, p.Turn.TimeoutSeconds, err)
		}
	}
	s := storeTest(t)
	w, request := assistPolicyFixture(t, s, 701)
	p, err := s.assistPreview(w.ID, request)
	if err != nil || p.Turn.TimeoutSeconds != 701 {
		t.Fatal("explicit provider policy lost", p.Turn.TimeoutSeconds, err)
	}
}

func TestPageAssistanceWaitingPolicyFreshnessAndInvalidValues(t *testing.T) {
	s := storeTest(t)
	w, request := assistPolicyFixture(t, s, 0)
	request.EventID = newID("question-")
	p, err := s.assistPreview(w.ID, request)
	if err != nil {
		t.Fatal(err)
	}
	request.ContextHash = p.Turn.Context.Hash
	current, err := s.providerWaitConfig()
	if err != nil {
		t.Fatal(err)
	}
	current.Values.Maximum = 1
	if _, err = s.changeProviderWait(ProviderWaitChange{Schema: 1, EventID: newID("policy-"), Revision: current.Revision, Values: current.Values, Reason: "Policy changed after preview"}, false); err != nil {
		t.Fatal(err)
	}
	if _, err = s.assistAsk(w.ID, request); err == nil {
		t.Fatal("stale waiting policy accepted")
	}
	for _, seconds := range []int{-1, int(^uint(0) >> 1)} {
		if _, err = assistTimeoutDuration(seconds); err == nil {
			t.Fatal("invalid duration", seconds)
		}
	}
}

func TestPageAssistanceZeroDeadlineCompletesAndCancellationStillWorks(t *testing.T) {
	for _, cancelTurn := range []bool{false, true} {
		t.Run(map[bool]string{false: "complete", true: "cancel"}[cancelTurn], func(t *testing.T) {
			s := storeTest(t)
			w, request := assistPolicyFixture(t, s, 0)
			request.EventID = newID("ask-")
			preview, err := s.assistPreview(w.ID, request)
			if err != nil {
				t.Fatal(err)
			}
			providers, err := s.providers()
			if err != nil {
				t.Fatal(err)
			}
			command := providers.Providers["fixture"].Command
			answer := answerFor(preview.Turn.Context, request.TemplateID, nil)
			reply, _ := json.Marshal(map[string]string{"type": "result", "result": answer})
			body := "#!/bin/sh\ncat >/dev/null\nsleep .15\nprintf '%s\\n' '" + strings.ReplaceAll(string(reply), "'", "'\\''") + "'\n"
			if cancelTurn {
				body = "#!/bin/sh\ncat >/dev/null\nsleep 15\n"
			}
			if err = os.WriteFile(command, []byte(body), 0700); err != nil {
				t.Fatal(err)
			}
			request.ContextHash = preview.Turn.Context.Hash
			turn, err := s.assistAsk(w.ID, request)
			if err != nil {
				t.Fatal(err)
			}
			done := make(chan struct{})
			go func() { s.runAssistTurnWithin(turn, 0); close(done) }()
			if cancelTurn {
				deadline := time.Now().Add(3 * time.Second)
				for {
					current, e := s.assistTurn(turn.ID)
					if e != nil {
						t.Fatal(e)
					}
					if current.PID > 0 {
						break
					}
					if !current.active() || time.Now().After(deadline) {
						t.Fatal("no running cancellable turn", current)
					}
					time.Sleep(10 * time.Millisecond)
				}
				if err = s.cancelAssist(w.ID, turn.ID); err != nil {
					t.Fatal(err)
				}
			}
			select {
			case <-done:
			case <-time.After(4 * time.Second):
				t.Fatal("zero-duration turn leaked")
			}
			current, err := s.assistTurn(turn.ID)
			if err != nil {
				t.Fatal(err)
			}
			if cancelTurn {
				if current.Status != "interrupted" {
					t.Fatal(current)
				}
			} else if current.Refusal != nil || current.Status != "answered" {
				t.Fatal("zero became immediate timeout", current.Status, current.Refusal)
			}
		})
	}
}
