//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"testing"
	"time"
)

func contextPolicyFixture(t *testing.T) (*Store, Work) {
	t.Helper()
	s, w, _, _, _, _ := fragmentBeginFixture(t)
	command := filepath.Join(t.TempDir(), "codex")
	script := "#!/usr/bin/env python3\nimport json,sys\nif sys.argv[1:]==['--version']:\n print('codex-cli fixture-window')\nelif sys.argv[1:]==['debug','models','--bundled']:\n print(json.dumps({'models':[{'slug':'gpt-5.6-sol','context_window':272000,'max_context_window':872000,'effective_context_window_percent':95}]}))\nelse:\n raise SystemExit('provider invocation forbidden')\n"
	if err := os.WriteFile(command, []byte(script), 0700); err != nil {
		t.Fatal(err)
	}
	ps, err := s.providers()
	if err != nil {
		t.Fatal(err)
	}
	ps.Providers["managed-review-fixture"] = Provider{Command: command}
	raw, _ := json.Marshal(ps)
	if err = os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	cfg, err := s.reviewerConfig("managed-review-fixture", "standard", 10)
	if err != nil {
		t.Fatal(err)
	}
	w, err = s.mutate(w.ID, "test.context-policy", "context-policy-fixture", w.Revision, []byte(`{}`), func(current *Work) error { current.Planning.Reviewer = cfg; return nil })
	if err != nil {
		t.Fatal(err)
	}
	return s, w
}

func contextPolicyRequest(t *testing.T, w Work, event string, tokens int) QuotaChange {
	t.Helper()
	limits, _ := quotaValues(w.Planning)
	raw, _ := json.Marshal(QuotaChange{Schema: 1, EventID: event, Revision: w.Revision, Limits: limits, Reason: "Explicit context window preserves originals and consumption"})
	var doc map[string]any
	if err := json.Unmarshal(raw, &doc); err != nil {
		t.Fatal(err)
	}
	doc["limits"].(map[string]any)["review_context_tokens"] = tokens
	raw, _ = json.Marshal(doc)
	var r QuotaChange
	if err := strict(raw, &r); err != nil {
		t.Fatal("public policy rejects an explicit client-supported context window", err)
	}
	return r
}

func TestReviewContextWindowPolicyPersistsWithoutSpending(t *testing.T) {
	s, w := contextPolicyFixture(t)
	request := contextPolicyRequest(t, w, "context-window-save", 400000)
	if _, err := s.previewQuotas(w.ID, request); err != nil {
		t.Fatal(err)
	}
	unchanged, err := s.get(w.ID)
	if err != nil || unchanged.Revision != w.Revision {
		t.Fatal("preview mutated", err)
	}
	saved, err := s.configureQuotas(w.ID, request)
	if err != nil {
		t.Fatal(err)
	}
	raw, _ := json.Marshal(saved.Planning.Reviewer)
	var config map[string]any
	_ = json.Unmarshal(raw, &config)
	if config["input_context_tokens"] != float64(400000) {
		t.Fatal("context policy not persisted", config)
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	current, err := reopened.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	budget, err := reopened.reviewInputBudget(current.Planning.Reviewer)
	if err != nil || budget.ContextTokens != 400000 || budget.Version != 2 {
		t.Fatal("consumer ignored saved policy", budget, err)
	}
	_, before := quotaValues(w.Planning)
	_, after := quotaValues(current.Planning)
	a, _ := json.Marshal(before)
	b, _ := json.Marshal(after)
	if !bytes.Equal(a, b) {
		t.Fatal("configuration spent or refunded calls")
	}
	request.EventID = "stale-window"
	if _, err := s.configureQuotas(w.ID, request); err == nil {
		t.Fatal("stale context configuration admitted")
	}
	for _, value := range []int{-1, 872001} {
		bad := contextPolicyRequest(t, current, "invalid-context-window", value)
		if _, err := reopened.previewQuotas(w.ID, bad); err == nil {
			t.Fatal("invalid capacity preview admitted", value)
		}
		if _, err := reopened.configureQuotas(w.ID, bad); err == nil {
			t.Fatal("invalid capacity applied", value)
		}
		same, _ := reopened.get(w.ID)
		if same.Revision != current.Revision {
			t.Fatal("capacity rejection mutated state")
		}
	}
}

func TestReviewContextWindowReachesToolFreeClient(t *testing.T) {
	s, w := contextPolicyFixture(t)
	cfg := *w.Planning.Reviewer
	cfg.InputContextTokens = 400000
	ps, err := s.providers()
	if err != nil {
		t.Fatal(err)
	}
	provider := ps.Providers[cfg.Provider]
	capture := filepath.Join(t.TempDir(), "args.json")
	script := "#!/usr/bin/env python3\nimport json,sys\nif sys.argv[1:]==['--version']:\n print('codex-cli fixture-window')\nelif sys.argv[1:]==['debug','models','--bundled']:\n print(json.dumps({'models':[{'slug':'gpt-5.6-sol','context_window':272000,'max_context_window':872000,'effective_context_window_percent':95}]}))\nelse:\n with open(" + strconv.Quote(capture) + ",'w') as f: json.dump(sys.argv[1:],f)\n print(json.dumps({'type':'item.completed','item':{'type':'agent_message','text':'{\"received\":true}'}}))\n"
	if err = os.WriteFile(provider.Command, []byte(script), 0700); err != nil {
		t.Fatal(err)
	}
	budget, err := observedReviewInputBudget(provider, cfg.ModelRoute, cfg.ProviderDigest, 400000)
	if err != nil {
		t.Fatal(err)
	}
	prepared, err := prepareReviewContextProvider(provider, budget)
	if err != nil {
		t.Fatal(err)
	}
	reply, err := runStructuredProvider(prepared, cfg.ModelRoute, "read-only fixture", "{}", 5*time.Second, func() bool { return true }, nil)
	if err != nil || reply != `{"received":true}` {
		t.Fatal("configured client did not receive request", reply, err)
	}
	raw, err := os.ReadFile(capture)
	if err != nil {
		t.Fatal(err)
	}
	var args []string
	if err = json.Unmarshal(raw, &args); err != nil {
		t.Fatal(err)
	}
	found := false
	for i, arg := range args {
		if arg == "model_context_window=400000" && i > 0 && args[i-1] == "-c" {
			found = true
		}
	}
	if !found || !containsTrustlessArgs(args, "--ignore-user-config") || !containsTrustlessArgs(args, "--ephemeral") {
		t.Fatal("context override missing or tool-free boundary lost", args)
	}
	for _, mode := range []string{"digest", "stamp", "capacity", "provider"} {
		t.Run(mode, func(t *testing.T) {
			copy := *budget
			p := provider
			switch mode {
			case "digest":
				copy.CapabilityDigest = strings.Repeat("0", 64)
			case "stamp":
				copy.ClientStamp = strings.Repeat("0", 64)
			case "capacity":
				copy.ContextTokens = copy.MaxContextTokens + 1
			case "provider":
				p.Args = []string{"foreign"}
			}
			if _, err := prepareReviewContextProvider(p, &copy); err == nil {
				t.Fatal("modified capacity admitted")
			}
		})
	}
}

func containsTrustlessArgs(args []string, want string) bool {
	for _, arg := range args {
		if arg == want {
			return true
		}
	}
	return false
}

func TestReviewContextWindowFragmentRuntimeAndReopen(t *testing.T) {
	s, w, a, c, report, receipt := referencedFragmentRuntimeFixture(t)
	catalog := filepath.Join(t.TempDir(), "models_cache.json")
	t.Setenv("CODEX_HOME", filepath.Dir(catalog))
	cache := `{"models":[{"slug":"gpt-5.6-sol","visibility":"list","supported_reasoning_levels":[{"effort":"medium"}],"context_window":272000,"max_context_window":872000,"effective_context_window_percent":95}]}`
	if err := os.WriteFile(catalog, []byte(cache), 0600); err != nil {
		t.Fatal(err)
	}
	script, err := os.ReadFile(filepath.Join(s.root, "review-fixture/claude"))
	if err != nil {
		t.Fatal(err)
	}
	fixture := strings.ReplaceAll(string(script), "json.loads(sys.argv[sys.argv.index('--json-schema')+1])", "json.load(open(sys.argv[sys.argv.index('--output-schema')+1]))")
	capture := filepath.Join(t.TempDir(), "window-args.jsonl")
	fixture = strings.Replace(fixture, "text=sys.stdin.read()", tokenMetadataFixture+"\nwith open("+strconv.Quote(capture)+",'a') as output: output.write(json.dumps(sys.argv[1:])+'\\n')\ntext=sys.stdin.read()", 1)
	command := filepath.Join(s.root, "review-fixture/codex")
	if err = os.WriteFile(command, []byte(fixture), 0700); err != nil {
		t.Fatal(err)
	}
	ps, err := s.providers()
	if err != nil {
		t.Fatal(err)
	}
	ps.Providers["managed-review-fixture"] = Provider{Command: command}
	raw, _ := json.Marshal(ps)
	if err = os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	cfg, err := s.reviewerConfig("managed-review-fixture", "standard", 10)
	if err != nil {
		t.Fatal(err)
	}
	cfg.InputContextTokens = 400000
	w, err = s.mutate(w.ID, "test.context-runtime", "context-runtime", w.Revision, []byte(`{}`), func(current *Work) error { current.Planning.Reviewer = cfg; return nil })
	if err != nil {
		t.Fatal(err)
	}
	c, err = s.managedReviewContext(w, a, c.Candidate, receipt)
	if err != nil {
		t.Fatal(err)
	}
	managedReviewMode(t, s, "pass")
	r, err := s.beginManagedFragmentReview(w, a, c, report, receipt)
	if err != nil {
		t.Fatal(err)
	}
	state, _, runErr := s.runManagedFragmentReview(w, a, r)
	if runErr != nil {
		t.Fatal("configured fragment review did not run", state, runErr)
	}
	if err = s.finishManagedFragmentReview(w.ID, a, r.ID, runErr); err != nil || state != "passed" {
		t.Fatal("configured fragment review did not finish", state, err)
	}
	raw, err = os.ReadFile(capture)
	if err != nil {
		t.Fatal(err)
	}
	calls := strings.Split(strings.TrimSpace(string(raw)), "\n")
	if len(calls) < 3 {
		t.Fatal("inspection, selection or decision was not executed", len(calls))
	}
	for _, line := range calls {
		var args []string
		if err = json.Unmarshal([]byte(line), &args); err != nil {
			t.Fatal(err)
		}
		if !containsTrustlessArgs(args, "model_context_window=400000") {
			t.Fatal("fragment call ignored configured window", args)
		}
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	current, err := reopened.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, err := current.task(a.TaskID)
	if err != nil {
		t.Fatal(err)
	}
	p, _, err := reopened.readFragmentJournalAnchor(*task.IndependentReview)
	if err != nil || p.InputBudget == nil || p.InputBudget.Version != 2 || p.InputBudget.ContextTokens != 400000 {
		t.Fatal("configured capacity proof not durable", err)
	}
	if current.Planning.Reviewer.Calls != len(calls) {
		t.Fatal("provider calls and preserved consumption differ")
	}
}

func TestCorrectiveCandidateRetainsClosedFragmentMethodEvidence(t *testing.T) {
	s, w, a, c, path, receipt := referencedFragmentRuntimeFixture(t)
	managedReviewMode(t, s, "reference-invalid")
	r, err := s.beginManagedFragmentReview(w, a, c, path, receipt)
	if err != nil {
		t.Fatal(err)
	}
	_, _, runErr := s.runManagedFragmentReview(w, a, r)
	if runErr == nil {
		t.Fatal("protocol counterexample was not exercised")
	}
	if err = s.finishManagedFragmentReview(w.ID, a, r.ID, runErr); err == nil {
		t.Fatal("invalid reference was accepted")
	}
	current, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, err := current.task(a.TaskID)
	if err != nil {
		t.Fatal(err)
	}
	saved := *task.IndependentReview
	if !s.recoveredReviewAllowsCorrection(current, &saved, true) {
		t.Fatal("closed incident unavailable before method change")
	}
	profileFixture(t, s)
	if !s.recoveredReviewAllowsCorrection(current, &saved, true) {
		t.Fatal("closed method evidence blocks a changed corrective candidate")
	}
	if err = s.managedReviewProtocolPrecondition(saved); err == nil {
		t.Fatal("historical reading authorized an unchanged retry")
	}
	if managedReviewCalls(t, s) != current.Planning.Reviewer.Calls {
		t.Fatal("diagnosis spent a call")
	}
}

func TestCorrectiveFragmentPlanDoesNotReuseAChangedMethod(t *testing.T) {
	s, w, a, c, path, receipt := referencedFragmentRuntimeFixture(t)
	managedReviewMode(t, s, "reference-invalid")
	r, err := s.beginManagedFragmentReview(w, a, c, path, receipt)
	if err != nil {
		t.Fatal(err)
	}
	_, _, runErr := s.runManagedFragmentReview(w, a, r)
	if runErr == nil {
		t.Fatal("original protocol error was not exercised")
	}
	_ = s.finishManagedFragmentReview(w.ID, a, r.ID, runErr)
	current, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, err := current.task(a.TaskID)
	if err != nil {
		t.Fatal(err)
	}
	saved := *task.IndependentReview
	task.RecoveredResult = &RecoveredResult{PriorReview: saved.ID}
	profileFixture(t, s)
	c.Candidate = strings.Repeat("e", 40)
	c.Diff += "\n+Current corrective candidate evidence\n"
	before := managedReviewCalls(t, s)
	p, err := s.planLegacyCandidateFragments(current, a, c, 20)
	if err != nil {
		t.Fatal("changed corrective candidate must get a fresh plan", err)
	}
	if len(p.Reused) != 0 || p.Candidate != c.Candidate {
		t.Fatal("old inspection was reused or candidate changed")
	}
	if managedReviewCalls(t, s) != before {
		t.Fatal("planning spent a model call")
	}
	if err = s.managedReviewProtocolPrecondition(saved); err == nil {
		t.Fatal("fresh correction planning authorized an unchanged retry")
	}
}
