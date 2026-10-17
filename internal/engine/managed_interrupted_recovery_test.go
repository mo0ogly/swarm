//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func interruptedRecoveryFixture(t *testing.T) (*Store, Work, Agent) {
	t.Helper()
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "interrupted useful work\n")
	a.Status, a.Ended, a.Activity = "interrupted", now(), "Local transport permission denied"
	a.Diagnostic = AttemptDiagnostic{AgentID: a.ID, AttemptID: a.Attempt, ObservedErrors: 1, Items: []DiagnosticItem{{Category: "tool", Cause: "Loopback listener denied", Traces: []string{"listen tcp 127.0.0.1:0: socket: operation not permitted"}}}}
	if err := s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	if err := s.managedFailure(a, a.Activity); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	return s, w, a
}

func TestManagedInterruptedRecoveryRetainsSourcesAndDiagnostic(t *testing.T) {
	s, w, a := interruptedRecoveryFixture(t)
	before, _ := s.managedAttempt(a.ID)
	h, err := s.managedRecoveryHandoff(w, &w.Tasks[0])
	if err != nil {
		t.Fatal(err)
	}
	raw, err := managedRecoveryBytes(h)
	if err != nil {
		t.Fatal(err)
	}
	var packet struct {
		Checkpoint struct {
			Commit string `json:"commit"`
		} `json:"checkpoint"`
		Diagnostic AttemptDiagnostic `json:"diagnostic"`
	}
	if err = json.Unmarshal(raw, &packet); err != nil {
		t.Fatal(err)
	}
	if packet.Checkpoint.Commit == "" {
		t.Fatal("interrupted sources absent from recovery packet")
	}
	if packet.Diagnostic.AgentID != a.ID || !strings.Contains(string(raw), "socket: operation not permitted") {
		t.Fatal("original attributable failure lost from recovery packet")
	}
	bare := filepath.Join(w.Planning.Repository.Storage, "repository.git")
	if got := gitTest(t, bare, "show", packet.Checkpoint.Commit+":value.txt"); got != "interrupted useful work" {
		t.Fatal("checkpoint did not retain actual interrupted work", got)
	}
	if err = os.WriteFile(filepath.Join(a.CWD, "value.txt"), []byte("later mutable work\n"), 0600); err != nil {
		t.Fatal(err)
	}
	second, err := s.managedRecoveryHandoff(w, &w.Tasks[0])
	if err != nil {
		t.Fatal(err)
	}
	secondRaw, _ := managedRecoveryBytes(second)
	if string(raw) != string(secondRaw) {
		t.Fatal("recovery replaced its immutable interrupted checkpoint")
	}
	w, request := planningClaim(t, s, w, "root")
	request.Operations = []PlanningOperation{{Kind: "retry", ID: "first", Next: "Reuse preserved checkpoint; repair denied transport precondition before checking."}}
	w, err = s.planningChange(w.ID, "decide", request)
	if err != nil {
		t.Fatal(err)
	}
	next, created, err := s.prepare(w.ID, Launch{Schema: 1, EventID: "interrupted-recovery-next", Revision: w.Revision, TaskID: "first", Provider: "managed-review-fixture", Role: "worker", Instruction: "Reuse only the preserved work", Timeout: 60})
	if err != nil || !created {
		t.Fatal("public preparation failed", created, err)
	}
	if got := gitTest(t, next.CWD, "show", managedRecoveryRef+":value.txt"); got != "interrupted useful work" {
		t.Fatal("next worker cannot read preserved original", got)
	}
	if !strings.Contains(next.Prompt, "socket: operation not permitted") || !strings.Contains(next.Prompt, "checkpoint") {
		t.Fatal("next worker did not receive sources and original cause")
	}
	if got := gitTest(t, next.CWD, "rev-parse", "HEAD"); got != w.Planning.Repository.Candidate {
		t.Fatal("unverified checkpoint adopted as candidate")
	}
	after, _ := s.managedAttempt(a.ID)
	if after.Result != before.Result || before.Result != "" || managedReviewCalls(t, s) != 0 {
		t.Fatal("checkpoint fabricated delivery or consumed review")
	}
	current, _ := s.get(w.ID)
	if current.Tasks[0].PlanMaxAttempts != 2 || len(current.Tasks[0].Attempts) != 2 || current.Tasks[0].Status == "accepted" {
		t.Fatal("checkpoint changed budgets or acceptance")
	}
}

func TestExecutionDirectivesAvoidKnownPathAndInterfaceGuesses(t *testing.T) {
	l, _ := (RunLimits{}).normalized()
	prompt := executionDirectives("/host/source", "/worker/copy", "first", l)
	for _, instruction := range []string{"omettre workdir", "réutiliser un symbole existant", "transport requis", "sans relancer"} {
		if !strings.Contains(prompt, instruction) {
			t.Errorf("missing recurrence prevention instruction: %s", instruction)
		}
	}
}

func TestManagedInterruptedRecoveryIsRetainedAtSettlementWithoutRetry(t *testing.T) {
	for _, mode := range []string{"retained", "redirected-copy"} {
		t.Run(mode, func(t *testing.T) {
			s, w := managedFixture(t)
			a := managedCompleted(t, s, w, "first", "work must survive without another attempt\n")
			w, err := s.get(w.ID)
			if err != nil {
				t.Fatal(err)
			}
			w, err = s.mutate(w.ID, "fixture.running", "restore-running", w.Revision, []byte(`{}`), func(w *Work) error {
				w.Tasks[0].Status = "running"
				w.Tasks[0].PlanMaxAttempts = 1
				return nil
			})
			if err != nil {
				t.Fatal(err)
			}
			if mode == "redirected-copy" {
				if err := os.Rename(a.CWD, a.CWD+"-retained"); err != nil {
					t.Fatal(err)
				}
				if err := os.Symlink(a.CWD+"-retained", a.CWD); err != nil {
					t.Fatal(err)
				}
			}
			if err = s.finishAgent(a, "interrupted", "Owner requested a diagnosis before recovery", nil); err != nil {
				t.Fatal(err)
			}
			after, _ := s.get(w.ID)
			manifest := filepath.Join(after.Planning.Repository.Storage, "interrupted-"+a.ID+".json")
			if mode == "retained" {
				raw, err := os.ReadFile(manifest)
				if err != nil {
					t.Fatal("sources not retained at settlement", err)
				}
				var c ManagedInterruptedCheckpoint
				if err := json.Unmarshal(raw, &c); err != nil {
					t.Fatal(err)
				}
				if got := gitTest(t, filepath.Join(after.Planning.Repository.Storage, "repository.git"), "show", c.Commit+":value.txt"); got != "work must survive without another attempt" {
					t.Fatal("wrong captured source", got)
				}
				found := false
				for _, event := range after.Planning.Inbox {
					for _, artifact := range event.Artifacts {
						if event.Attempt == a.Attempt && event.Scope == "root" && artifact.SHA256 == hash(raw) {
							found = true
						}
					}
				}
				if !found {
					t.Fatal("checkpoint not received by owner")
				}
			} else if !strings.Contains(after.Tasks[0].Blocker, "Conservation des sources interrompues impossible") {
				t.Fatal("preservation refusal was hidden", after.Tasks[0].Blocker)
			}
			item, _ := s.managedAttempt(a.ID)
			if len(after.Tasks[0].Attempts) != 1 || after.Tasks[0].Status != "blocked" || item.Result != "" || managedReviewCalls(t, s) != 0 {
				t.Fatal("preservation fabricated acceptance, attempt or review")
			}
		})
	}
}

func TestManagedInterruptedRecoveryRefusesTamperedCheckpoint(t *testing.T) {
	for _, mode := range []string{"manifest", "reference", "missing-manifest", "redirected-copy"} {
		t.Run(mode, func(t *testing.T) {
			s, w, a := interruptedRecoveryFixture(t)
			if _, err := s.managedRecoveryHandoff(w, &w.Tasks[0]); err != nil {
				t.Fatal(err)
			}
			manifest := filepath.Join(w.Planning.Repository.Storage, "interrupted-"+a.ID+".json")
			bare := filepath.Join(w.Planning.Repository.Storage, "repository.git")
			ref := "refs/swarm/interruptions/" + a.ID
			switch mode {
			case "manifest":
				raw, err := os.ReadFile(manifest)
				if err != nil {
					t.Fatal(err)
				}
				var c map[string]any
				json.Unmarshal(raw, &c)
				c["attempt"] = "unrelated-attempt"
				raw, _ = json.Marshal(c)
				os.WriteFile(manifest, raw, 0600)
			case "reference":
				gitTest(t, bare, "update-ref", ref, w.Planning.Repository.Candidate)
			case "missing-manifest":
				if err := os.Remove(manifest); err != nil {
					t.Fatal(err)
				}
			case "redirected-copy":
				if err := os.Rename(a.CWD, a.CWD+"-retained"); err != nil {
					t.Fatal(err)
				}
				if err := os.Symlink(a.CWD+"-retained", a.CWD); err != nil {
					t.Fatal(err)
				}
			}
			before, _ := s.get(w.ID)
			oldRef := gitTest(t, bare, "rev-parse", ref)
			if _, err := s.managedRecoveryHandoff(w, &w.Tasks[0]); err == nil {
				t.Fatal("unsafe checkpoint accepted")
			}
			after, _ := s.get(w.ID)
			if after.Revision != before.Revision || len(after.Tasks[0].Attempts) != len(before.Tasks[0].Attempts) || gitTest(t, bare, "rev-parse", ref) != oldRef || managedReviewCalls(t, s) != 0 {
				t.Fatal("refused checkpoint changed history, references or budget")
			}
		})
	}
}

func TestManagedInterruptedRecoverySerializesCheckpointAndChecksProcess(t *testing.T) {
	t.Run("concurrent", func(t *testing.T) {
		s, w, _ := interruptedRecoveryFixture(t)
		type result struct {
			handoff *ManagedRecoveryHandoff
			err     error
		}
		results := make(chan result, 2)
		for range 2 {
			go func() {
				h, err := s.managedRecoveryHandoff(w, &w.Tasks[0])
				results <- result{h, err}
			}()
		}
		first, second := <-results, <-results
		if first.err != nil && second.err != nil {
			t.Fatal("no concurrent caller retained a checkpoint", first.err, second.err)
		}
		var commit string
		for _, observed := range []result{first, second} {
			if observed.err != nil {
				if !strings.Contains(observed.err.Error(), "opération Git est déjà en cours") {
					t.Fatal("unexpected concurrent refusal", observed.err)
				}
				continue
			}
			if commit != "" && commit != observed.handoff.Checkpoint.Commit {
				t.Fatal("concurrent checkpoint diverged")
			}
			commit = observed.handoff.Checkpoint.Commit
		}
		afterRelease, err := s.managedRecoveryHandoff(w, &w.Tasks[0])
		if err != nil || afterRelease.Checkpoint.Commit != commit {
			t.Fatal("recovery after lock release replaced checkpoint", err)
		}
	})
	t.Run("live-child", func(t *testing.T) {
		s, w, a := interruptedRecoveryFixture(t)
		a.Child = os.Getpid()
		a.ChildStamp = processStamp(a.Child)
		if err := s.saveAgent(a); err != nil {
			t.Fatal(err)
		}
		if _, err := s.managedRecoveryHandoff(w, &w.Tasks[0]); err == nil {
			t.Fatal("live child was snapshotted")
		}
		if _, err := os.Stat(filepath.Join(w.Planning.Repository.Storage, "interrupted-"+a.ID+".json")); !os.IsNotExist(err) {
			t.Fatal("refused snapshot was persisted", err)
		}
	})
}
