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

const observedClaudeRejected = `{"type":"rate_limit_event","rate_limit_info":{"status":"rejected","resetsAt":1789851000,"rateLimitType":"five_hour","overageStatus":"rejected","overageDisabledReason":"out_of_credits","isUsingOverage":false}}`
const observedClaude429 = `{"type":"result","is_error":true,"subtype":"success","api_error_status":429,"terminal_reason":"api_error","result":"You've hit your session limit · resets 10:50pm (Europe/Paris)"}`

func decodeQuotaEvent(t *testing.T, raw string) map[string]any {
	t.Helper()
	var e map[string]any
	if err := json.Unmarshal([]byte(raw), &e); err != nil {
		t.Fatal(err)
	}
	return e
}
func futureCooldown(t *testing.T, s *Store, provider string) *ProviderCooldown {
	t.Helper()
	c := &ProviderCooldown{ObservedAt: now(), ResetAt: time.Now().Add(time.Hour).Unix(), Signal: "rate_limit_event.rejected"}
	if e := s.recordProviderCooldown(provider, "test-call", c); e != nil {
		t.Fatal(e)
	}
	return c
}

func TestProviderCooldownExactRejectedEventsAndNoTextFalsePositives(t *testing.T) {
	at := time.Date(2026, 9, 19, 20, 20, 0, 0, time.UTC)
	c := observedProviderCooldown(decodeQuotaEvent(t, observedClaudeRejected), at)
	if c == nil || c.ResetAt != 1789851000 || !c.active(at) || c.active(time.Unix(1789851000, 0)) {
		t.Fatal("reset timestamp ignored", c)
	}
	c = observedProviderCooldown(decodeQuotaEvent(t, observedClaude429), at)
	if c == nil || c.ResetAt != 0 || !c.active(at.Add(48*time.Hour)) {
		t.Fatal("displayed time was invented as reset", c)
	}
	for _, raw := range []string{
		`{"type":"rate_limit_event","rate_limit_info":{"status":"allowed_warning","resetsAt":1789851000,"overageStatus":"rejected"}}`,
		`{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","resetsAt":1789851000}}`,
		`{"type":"user","message":{"content":[{"type":"tool_result","is_error":true,"api_error_status":429,"content":"rate limit rejected"}]}}`,
		`{"type":"assistant","message":{"content":"429 You've hit your session limit"}}`,
		`{"type":"tool_result","is_error":true,"api_error_status":429}`,
		`{"type":"result","is_error":false,"api_error_status":429}`,
		`{"type":"result","is_error":true,"subtype":"success","result":"tests expected a rate limit 429 at 10:50pm"}`,
	} {
		if c := observedProviderCooldown(decodeQuotaEvent(t, raw), at); c != nil {
			t.Fatalf("false quota from non-rejection: %s", raw)
		}
	}
}
func TestProviderCooldownPersistsResetAcrossResultAndRestart(t *testing.T) {
	s := storeTest(t)
	sink := &outputSink{s: s, id: "quota-agent", provider: "claude"}
	if _, e := sink.Write([]byte(observedClaudeRejected + "\n" + observedClaude429 + "\n")); e != nil {
		t.Fatal(e)
	}
	c := sink.cooldownSnapshot()
	if c == nil || c.ResetAt != 1789851000 || !strings.Contains(sink.current(), "2026-09-19 20:50:00 UTC") {
		t.Fatalf("rejection lost in terminal result: %+v %s", c, sink.current())
	}
	next, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer next.db.Close()
	before := time.Unix(1789850999, 0)
	if e = next.providerCooldownGuardAt("claude", before); commandFailure(e).Code != "provider_cooldown" {
		t.Fatal("restart lost wait", e)
	}
	if e = next.providerCooldownGuardAt("claude", time.Unix(1789851000, 0)); e != nil {
		t.Fatal("reset boundary still blocked", e)
	}
	if e = next.providerCooldownGuardAt("other-provider", before); e != nil {
		t.Fatal("other provider blocked", e)
	}
}
func TestProviderCooldownStructuredProcessReportsQuotaInsteadOfExitOne(t *testing.T) {
	s := storeTest(t)
	cmd := filepath.Join(t.TempDir(), "claude")
	script := "#!/bin/sh\ncat >/dev/null\ncat <<'EVENTS'\n" + observedClaudeRejected + "\n" + observedClaude429 + "\nEVENTS\nexit 1\n"
	if e := os.WriteFile(cmd, []byte(script), 0700); e != nil {
		t.Fatal(e)
	}
	_, e := runStructuredProvider(Provider{Command: cmd}, nil, "review", "{}", time.Second, func() bool { return true }, nil, s.providerCooldownObserver("claude", "review-call"))
	if commandFailure(e).Code != "provider_cooldown" || !strings.Contains(e.Error(), "20:50:00 UTC") {
		t.Fatal("quota hidden by exit status", e)
	}
	c, _, e := s.providerCooldown("claude")
	if e != nil || c == nil || c.ResetAt != 1789851000 {
		t.Fatal(c, e)
	}
}
func TestProviderCooldownGuardsDoNotReserveAgentsPlannerOrReviewer(t *testing.T) {
	t.Run("worker", func(t *testing.T) {
		s := storeTest(t)
		w, launch := setupAgent(t, s)
		futureCooldown(t, s, launch.Provider)
		before, _ := s.get(w.ID)
		_, _, e := s.prepare(w.ID, launch)
		if commandFailure(e).Code != "provider_cooldown" {
			t.Fatal(e)
		}
		var count int
		s.db.QueryRow("SELECT count(*) FROM agents WHERE work_id=?", w.ID).Scan(&count)
		after, _ := s.get(w.ID)
		if count != 0 || len(after.Tasks[0].Attempts) != len(before.Tasks[0].Attempts) || after.Revision != before.Revision {
			t.Fatal("quota guard consumed attempt or reserved process")
		}
	})
	t.Run("planner", func(t *testing.T) {
		s, w := planningFixture(t)
		w.Planning.Provider = "claude"
		data, _ := json.Marshal(w)
		s.db.Exec("UPDATE works SET body=? WHERE id=?", data, w.ID)
		futureCooldown(t, s, "claude")
		before, _ := s.get(w.ID)
		if e := s.planningStep(w.ID); commandFailure(e).Code != "provider_cooldown" {
			t.Fatal(e)
		}
		scope, _ := before.Planning.scope("root")
		_, e := s.planningChange(w.ID, "claim", PlanningRequest{Schema: 1, EventID: "quota-claim", Revision: before.Revision, Scope: "root", ScopeRevision: scope.Revision, Holder: "quota-holder", LeaseSeconds: 60})
		if commandFailure(e).Code != "provider_cooldown" {
			t.Fatal(e)
		}
		after, _ := s.get(w.ID)
		var count int
		s.db.QueryRow("SELECT count(*) FROM planning_calls WHERE work_id=?", w.ID).Scan(&count)
		if count != 0 || after.Planning.Activations != before.Planning.Activations || after.Revision != before.Revision {
			t.Fatal("quota guard spent a planner claim")
		}
	})
	t.Run("reviewer", func(t *testing.T) {
		s, p := preparedTeam(t)
		w, _ := s.get(p.WorkID)
		futureCooldown(t, s, w.Planning.Reviewer.Provider)
		if e := s.independentReviewStep(w.ID); commandFailure(e).Code != "provider_cooldown" {
			t.Fatal(e)
		}
		after, _ := s.get(w.ID)
		if after.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls || after.Revision != w.Revision {
			t.Fatal("quota guard spent a reviewer claim")
		}
		if e := s.reviewerAvailable(w); commandFailure(e).Code != "provider_cooldown" {
			t.Fatal("worker may launch with reviewer in quota", e)
		}
	})
}
func TestProviderCooldownManagedCandidateWaitsWithoutRepeatingWorker(t *testing.T) {
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "reviewed after wait\n")
	before, _ := s.get(w.ID)
	futureCooldown(t, s, before.Planning.Reviewer.Provider)
	if e := s.integrateManagedAttempt(a); commandFailure(e).Code != "provider_cooldown" {
		t.Fatal(e)
	}
	item, e := s.managedAttempt(a.ID)
	if e != nil {
		t.Fatal(e)
	}
	if item.State != "integrating" || item.Result == "" || managedReviewCalls(t, s) != 0 {
		t.Fatalf("candidate lost or reviewer called: %+v", item)
	}
	// Exercise the controller path too: it must not turn the known wait into a conflict.
	s.conduct(a, "completed")
	item, _ = s.managedAttempt(a.ID)
	if item.State != "integrating" {
		t.Fatal("conductor destroyed pending candidate", item)
	}
	// Clock-independent reset boundary: persist a genuine elapsed timestamp.
	c := &ProviderCooldown{ObservedAt: now(), ResetAt: time.Now().Add(-time.Second).Unix(), Signal: "rate_limit_event.rejected"}
	// This is a test fixture advancing the recorded clock boundary, not a public bypass.
	c.Provider = before.Planning.Reviewer.Provider
	c.Source = "elapsed-fixture"
	path, _ := s.providerCooldownPath(c.Provider)
	data, _ := json.Marshal(c)
	if e = os.WriteFile(path, data, 0600); e != nil {
		t.Fatal(e)
	}
	if e = s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	after, _ := s.get(w.ID)
	if after.Tasks[0].Status != "accepted" || len(after.Tasks[0].Attempts) != len(before.Tasks[0].Attempts) || managedReviewCalls(t, s) != 1 || after.Planning.Reviewer.Calls != before.Planning.Reviewer.Calls+1 {
		t.Fatal("waiting candidate did not resume on same attempt", after.Tasks[0])
	}
}
func TestProviderCooldownRecoveryWaitsAndKeepsExhaustedBudget(t *testing.T) {
	at := time.Date(2026, 9, 19, 20, 20, 0, 0, time.UTC)
	a := failedRecoveryAgent("quota", "task", "", "quota", at)
	a.ProviderCooldown = observedProviderCooldown(decodeQuotaEvent(t, observedClaudeRejected), at)
	finalizeRecoveryState(&a)
	result := assessRecovery(a, Task{PlanMaxAttempts: 2}, at)
	if result.Disposition != recoveryDispositionWait || result.Category != recoveryProviderLimit || result.NextEligibleAt.Unix() != 1789851000 {
		t.Fatalf("quota retries immediately: %+v", result)
	}
	result = assessRecovery(a, Task{PlanMaxAttempts: 2}, time.Unix(1789851001, 0))
	if result.Disposition != recoveryDispositionRetry {
		t.Fatalf("elapsed reset did not release bounded recovery: %+v", result)
	}
	a.Recovery.AutomaticUsed = maxAutomaticAttempts
	result = assessRecovery(a, Task{PlanMaxAttempts: 2}, time.Unix(1789851001, 0))
	if result.Disposition == recoveryDispositionRetry || !strings.Contains(result.Reason, "budget") {
		t.Fatal("reset refunded attempts", result)
	}
	a.ProviderCooldown.ResetAt = 0
	a.Recovery.AutomaticUsed = 1
	if got := assessRecovery(a, Task{PlanMaxAttempts: 2}, at.Add(24*time.Hour)); got.Disposition != recoveryDispositionWait {
		t.Fatal("unknown reset retried", got)
	}
}
func TestProviderCooldownUnknownNeedsExplicitAuditedClear(t *testing.T) {
	s := storeTest(t)
	c := observedProviderCooldown(decodeQuotaEvent(t, observedClaude429), time.Now())
	if e := s.recordProviderCooldown("claude", "unknown-call", c); e != nil {
		t.Fatal(e)
	}
	c, digest, e := s.providerCooldown("claude")
	if e != nil {
		t.Fatal(e)
	}
	if e = s.providerCooldownGuardAt("claude", time.Now().AddDate(1, 0, 0)); commandFailure(e).Code != "provider_cooldown" {
		t.Fatal("unknown reset elapsed magically", e)
	}
	request := ProviderCooldownClear{Schema: 1, Event: "operator-quota-clear", Digest: digest, Reason: "Compte vérifié par opérateur ; autoriser un nouvel essai explicite"}
	input := filepath.Join(t.TempDir(), "clear.json")
	raw, _ := json.Marshal(request)
	os.WriteFile(input, raw, 0600)
	var output bytes.Buffer
	if e = agentCLI(s, []string{"providers", "cooldown", "clear", "claude"}, input, "", true, &output); e != nil {
		t.Fatal(e)
	}
	c, _, _ = s.providerCooldown("claude")
	if c.ClearedAt == "" || c.ClearActor == "" || c.ClearReason != request.Reason || c.ClearEvent != request.Event || c.ResetAt != 0 || !strings.Contains(output.String(), "non démontrée") {
		t.Fatal("clear is not audited or claims health", c, output.String())
	}
	if _, e = s.clearProviderCooldown("claude", request); e != nil {
		t.Fatal("clear replay", e)
	}
	if e = s.providerCooldownGuard("claude"); e != nil {
		t.Fatal(e)
	}
	futureCooldown(t, s, "claude")
	_, digest, _ = s.providerCooldown("claude")
	request.Event = "cannot-clear-future"
	request.Digest = digest
	if _, e = s.clearProviderCooldown("claude", request); commandFailure(e).Code != "provider_cooldown" {
		t.Fatal("future reset bypassed", e)
	}
}

func TestProviderCooldownReconcileHistoricalLogsWithoutCallOrBudgetReset(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	if e = s.finishAgent(a, "failed", "Processus en échec (code 1)", nil); e != nil {
		t.Fatal(e)
	}
	a, _ = s.agent(a.ID)
	used := a.Recovery.AutomaticUsed
	before, _ := s.get(w.ID)
	observed := "2026-09-19T20:20:00Z"
	logs := []AgentLog{
		{AgentID: a.ID, At: observed, Kind: "message", Message: observedClaudeRejected}, // not provider output
		{AgentID: a.ID, At: observed, Kind: "output", Message: `{"type":"user","message":{"content":"429 rate limit"}}`},
		{AgentID: a.ID, At: observed, Kind: "output", Message: observedClaudeRejected},
		{AgentID: a.ID, At: observed, Kind: "output", Message: observedClaude429},
	}
	if e = s.logBatch(a.ID, logs); e != nil {
		t.Fatal(e)
	}
	if e = s.reconcile(a.ID); e != nil {
		t.Fatal(e)
	}
	a, _ = s.agent(a.ID)
	after, _ := s.get(w.ID)
	if a.ProviderCooldown == nil || a.ProviderCooldown.ResetAt != 1789851000 || a.ProviderCooldown.Source != a.ID || a.Recovery.AutomaticUsed != used || len(after.Tasks[0].Attempts) != len(before.Tasks[0].Attempts) {
		t.Fatal("history reconciliation changed budgets or lost signal", a)
	}
	events, _ := s.logs(a.ID, -1)
	count := len(events)
	if e = s.reconcile(a.ID); e != nil {
		t.Fatal(e)
	}
	events, _ = s.logs(a.ID, -1)
	if len(events) != count {
		t.Fatal("reconciliation replay duplicated logs")
	}
}
func TestProviderCooldownOldLogsCannotUndoClearOrNewObservation(t *testing.T) {
	s := storeTest(t)
	c := &ProviderCooldown{ObservedAt: time.Now().Add(-time.Minute).UTC().Format(time.RFC3339Nano), Signal: "result.api_error_status.429"}
	if e := s.recordProviderCooldown("claude", "past-call", c); e != nil {
		t.Fatal(e)
	}
	_, digest, _ := s.providerCooldown("claude")
	_, e := s.clearProviderCooldown("claude", ProviderCooldownClear{Schema: 1, Event: "clear-old-quota", Digest: digest, Reason: "Vérification opérateur explicite avant un nouvel essai"})
	if e != nil {
		t.Fatal(e)
	}
	if e = s.recordProviderCooldown("claude", "past-call", c); e != nil {
		t.Fatal(e)
	}
	if e = s.providerCooldownGuard("claude"); e != nil {
		t.Fatal("old log resurrected cleared quota", e)
	}
	future := futureCooldown(t, s, "claude")
	if e = s.recordProviderCooldown("claude", "past-call", c); e != nil {
		t.Fatal(e)
	}
	actual, _, _ := s.providerCooldown("claude")
	if actual.ResetAt != future.ResetAt || actual.Source != "test-call" {
		t.Fatal("old record replaced fresh wait", actual)
	}
}
func TestProviderCooldownArrivingAfterReservationDoesNotStartProcessOrRefund(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	before, _ := s.get(w.ID)
	futureCooldown(t, s, a.Provider)
	if e = s.supervise(a.ID); e != nil {
		t.Fatal(e)
	}
	a, _ = s.agent(a.ID)
	after, _ := s.get(w.ID)
	if a.Child != 0 || a.ProviderCooldown == nil || a.Status != "failed" || len(after.Tasks[0].Attempts) != len(before.Tasks[0].Attempts) {
		t.Fatal("queued process started or reservation refunded", a)
	}
	script := filepath.Join(t.TempDir(), "claude")
	marker := filepath.Join(t.TempDir(), "started")
	os.WriteFile(script, []byte("#!/bin/sh\nprintf 'started' > '"+marker+"'\n"), 0700)
	_, e = runStructuredProvider(Provider{Command: script}, nil, "prompt", "{}", time.Second, func() bool { return s.providerCooldownGuard(a.Provider) == nil }, nil)
	if e == nil {
		t.Fatal("structured process started despite late quota")
	}
	if _, e = os.Stat(marker); !os.IsNotExist(e) {
		t.Fatal("structured process ran", e)
	}
}
func TestProviderCooldownStorageVersionPreservesHistoryBeforeUpgrade(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	before, _ := s.get(w.ID)
	if _, e = s.db.Exec("PRAGMA user_version=21"); e != nil {
		t.Fatal(e)
	}
	next, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer next.db.Close()
	var version int
	next.db.QueryRow("PRAGMA user_version").Scan(&version)
	if version != 22 || schemaVersion != 22 {
		t.Fatal("old binaries are not fenced", version)
	}
	backup, e := filepath.Glob(filepath.Join(s.root, ".swarm", "state-pre-v22-*.db"))
	if e != nil || len(backup) != 1 {
		t.Fatal("upgrade backup absent", backup, e)
	}
	after, _ := next.get(w.ID)
	got, e := next.agent(a.ID)
	if e != nil || after.Revision != before.Revision || len(after.Tasks[0].Attempts) != len(before.Tasks[0].Attempts) || got.Status != a.Status {
		t.Fatal("migration changed runtime history", got, e)
	}
}

func TestProviderCooldownClearAllowsExplicitRetryButDoesNotRestoreAttempts(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	w.Tasks[0].PlanMaxAttempts = 2
	w.Tasks[0].PlanRole = "worker"
	data, _ := json.Marshal(w)
	if _, e := s.db.Exec("UPDATE works SET body=? WHERE id=?", data, w.ID); e != nil {
		t.Fatal(e)
	}
	first, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	c := observedProviderCooldown(decodeQuotaEvent(t, observedClaude429), time.Now())
	if e = s.recordProviderCooldown(r.Provider, first.ID, c); e != nil {
		t.Fatal(e)
	}
	first.ProviderCooldown = c
	if e = s.finishAgent(first, "failed", c.message(), nil); e != nil {
		t.Fatal(e)
	}
	first, _ = s.agent(first.ID)
	if got := assessRecovery(first, w.Tasks[0], time.Now()); got.Disposition != recoveryDispositionWait {
		t.Fatal("unknown quota recovered automatically", got)
	}
	_, digest, _ := s.providerCooldown(r.Provider)
	request := ProviderCooldownClear{Schema: 1, Event: "clear-before-explicit-agent-retry", Digest: digest, Reason: "Compte vérifié, un essai explicite autorisé sans certitude de disponibilité"}
	input := filepath.Join(t.TempDir(), "clear.json")
	data, _ = json.Marshal(request)
	os.WriteFile(input, data, 0600)
	var out bytes.Buffer
	if e = agentCLI(s, []string{"providers", "cooldown", "clear", r.Provider}, input, "", true, &out); e != nil {
		t.Fatal(e)
	}
	var count int
	s.db.QueryRow("SELECT count(*) FROM agents WHERE work_id=?", w.ID).Scan(&count)
	if count != 1 {
		t.Fatal("clear started a worker")
	}
	current, _ := s.get(w.ID)
	r.EventID = "explicit-after-clear"
	r.Previous = first.ID
	r.Revision = current.Revision
	second, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal("explicit recovery blocked by historical snapshot", e)
	}
	if e = s.supervise(second.ID); e != nil {
		t.Fatal(e)
	}
	second, _ = s.agent(second.ID)
	if second.Status != "completed" || second.Previous != first.ID {
		t.Fatal("explicit process did not complete", second)
	}
	current, _ = s.get(w.ID)
	r.EventID = "cannot-exceed-attempt-budget"
	r.Previous = second.ID
	r.Revision = current.Revision
	if _, _, e = s.prepare(w.ID, r); e == nil || !strings.Contains(e.Error(), "Plafond du plan atteint") {
		t.Fatal("clear refunded attempt budget", e)
	}
	s.db.QueryRow("SELECT count(*) FROM agents WHERE work_id=?", w.ID).Scan(&count)
	if count != 2 || len(current.Tasks[0].Attempts) != 2 {
		t.Fatal("attempt history was lost", count)
	}
}
