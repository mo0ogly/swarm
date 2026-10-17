//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func incrementalReviewFixture(t *testing.T) (*Store, Work, Agent, IndependentReview) {
	t.Helper()
	s, w := managedFixture(t)
	first := managedCompleted(t, s, w, "first", "initial\n")
	if e := os.WriteFile(filepath.Join(first.CWD, "baseline.txt"), []byte(strings.Repeat("baseline evidence line\n", 6000)), 0600); e != nil {
		t.Fatal(e)
	}
	if e := s.integrateManagedAttempt(first); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	if w.Tasks[0].Status != "accepted" {
		t.Fatal("baseline rejected", w.Tasks[0].Blocker)
	}
	old := *w.Tasks[0].IndependentReview
	second := managedCompleted(t, s, w, "second", "initial\n")
	if e := os.WriteFile(filepath.Join(second.CWD, "increment.txt"), []byte(strings.Repeat("new evidence line\n", 5000)), 0600); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	return s, w, second, old
}

func TestIncrementalReviewNewVerdictAndHistoricalEvidence(t *testing.T) {
	s, w, a, old := incrementalReviewFixture(t)
	calls := managedReviewCalls(t, s)
	if e := s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	after, _ := s.get(w.ID)
	task, _ := after.task(a.TaskID)
	if task.Status != "accepted" || managedReviewCalls(t, s) != calls+1 || task.IndependentReview.ID == old.ID || task.IndependentReview.CandidateSHA == old.CandidateSHA {
		t.Fatal("old verdict reused", task.Blocker)
	}
	data, e := os.ReadFile(filepath.Join(s.root, task.IndependentReview.Context))
	if e != nil {
		t.Fatal(e)
	}
	var c managedReviewContext
	if e = json.Unmarshal(data, &c); e != nil {
		t.Fatal(e)
	}
	if c.Baseline == nil || c.Baseline.Candidate != old.CandidateSHA || len(c.Baseline.Reviews) != 1 || c.Baseline.Reviews[0].ContextDigest != old.ContextDigest || len(c.Tasks) != 2 || strings.Contains(c.Diff, "+baseline evidence") || !strings.Contains(c.Diff, "+new evidence") {
		t.Fatal("incremental chain incomplete")
	}
	for _, target := range after.Tasks {
		if target.IndependentReview.CandidateSHA != c.Candidate || target.AutoValidation.CandidateSHA != c.Candidate {
			t.Fatal("mixed candidate acceptance")
		}
	}
	reopened, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer reopened.db.Close()
	if e = reopened.independentReviewGuard(&after, task); e != nil {
		t.Fatal("chain lost on reopen", e)
	}
	if e = os.WriteFile(filepath.Join(s.root, old.Context), []byte(`{}`), 0600); e != nil {
		t.Fatal(e)
	}
	if e = reopened.independentReviewGuard(&after, task); e == nil {
		t.Fatal("corrupted baseline still accepted")
	}
}

func TestIncrementalReviewRejectsUnverifiedBaseline(t *testing.T) {
	for _, mode := range []string{"missing-proof", "policy", "verdict"} {
		t.Run(mode, func(t *testing.T) {
			s, w, a, old := incrementalReviewFixture(t)
			if mode == "missing-proof" {
				os.Remove(filepath.Join(s.root, old.Context))
			} else {
				var e error
				w, e = s.mutate(w.ID, "test.baseline", mode, w.Revision, []byte(`{}`), func(w *Work) error {
					if mode == "policy" {
						w.Tasks[0].ValidationPolicy.Controls[0].Timeout++
					} else {
						w.Tasks[0].IndependentReview.State = "changes_requested"
					}
					return nil
				})
				if e != nil {
					t.Fatal(e)
				}
			}
			calls := managedReviewCalls(t, s)
			if e := s.integrateManagedAttempt(a); e != nil {
				t.Fatal(e)
			}
			after, _ := s.get(w.ID)
			if after.Tasks[1].Status != "blocked" || after.Planning.Repository.Candidate != old.CandidateSHA || managedReviewCalls(t, s) != calls {
				t.Fatal("unverified baseline used")
			}
		})
	}
}

func TestIncrementalReviewStillRequiresFreshPass(t *testing.T) {
	s, w, a, old := incrementalReviewFixture(t)
	if e := os.WriteFile(filepath.Join(s.root, "review-fixture/mode"), []byte("fail"), 0600); e != nil {
		t.Fatal(e)
	}
	if e := s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	after, _ := s.get(w.ID)
	if after.Tasks[1].Status != "blocked" || after.Planning.Repository.Candidate != old.CandidateSHA || after.Tasks[1].IndependentReview.State != "changes_requested" {
		t.Fatal("new refusal ignored")
	}
}

func TestIncrementalReviewUsesVerifiedBaselineWhenContextFits(t *testing.T) {
	s, w := managedFixture(t)
	first := managedCompleted(t, s, w, "first", "unchanged accepted original\n")
	if err := os.WriteFile(filepath.Join(first.CWD, "baseline-only.txt"), []byte("baseline-only stable original\n"), 0600); err != nil {
		t.Fatal(err)
	}
	if err := s.integrateManagedAttempt(first); err != nil {
		t.Fatal(err)
	}
	w, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	if w.Tasks[0].Status != "accepted" {
		t.Fatal("fixture baseline not accepted", w.Tasks[0].Blocker)
	}
	old := *w.Tasks[0].IndependentReview
	second := managedCompleted(t, s, w, "second", "fresh changed original\n")
	if err = s.integrateManagedAttempt(second); err != nil {
		t.Fatal(err)
	}
	after, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, err := after.task(second.TaskID)
	if err != nil {
		t.Fatal(err)
	}
	if task.IndependentReview == nil || task.Status != "accepted" {
		t.Fatal("fixture candidate not independently accepted", task.Blocker)
	}
	raw, err := os.ReadFile(filepath.Join(s.root, task.IndependentReview.Context))
	if err != nil {
		t.Fatal(err)
	}
	var c managedReviewContext
	if err = json.Unmarshal(raw, &c); err != nil {
		t.Fatal(err)
	}
	if len(raw) > 160*1024 {
		t.Fatal("fixture should fit the ordinary review input", len(raw))
	}
	if c.Baseline == nil || c.Baseline.Version != managedReviewBaselineContractVersion || c.Baseline.Candidate != old.CandidateSHA || len(c.Baseline.Reviews) != 1 || len(c.Tasks) != 2 {
		t.Fatal("verified accepted baseline omitted from a fitting fresh context")
	}
	if strings.Contains(c.Diff, "+baseline-only stable original") || !strings.Contains(c.Diff, "fresh changed original") {
		t.Fatal("review does not isolate the complete current delta")
	}
	if task.IndependentReview.ID == old.ID || task.IndependentReview.CandidateSHA == old.CandidateSHA {
		t.Fatal("baseline substituted for a fresh verdict")
	}
}

func TestIncrementalBaselineScopeReachesFragmentInspection(t *testing.T) {
	for _, version := range []int{0, 1, 2, 3} {
		t.Run(fmt.Sprint(version), func(t *testing.T) {
			p, _ := inspectionReplyFixture()
			metadata := fmt.Sprintf(`{"accepted_baseline":{"version":%d,"candidate_commit":"baseline","accepted_reviews":[]}}`, version)
			p.Artifacts = append(p.Artifacts, managedReviewFragmentArtifact{Kind: "context", Name: "contracts-reports-controls-baseline", Content: metadata, Digest: hash([]byte(metadata))})
			prompt, err := managedFragmentInspectionPrompt("original-prefix", p)
			if version == 3 {
				if err == nil {
					t.Fatal("unknown baseline contract was accepted")
				}
				return
			}
			if err != nil {
				t.Fatal(err)
			}
			if version == 0 {
				if hash([]byte(prompt)) != "0842a14593d528c7a62cc26e0b12f8598eaa4f7237b41ae08f10fc7b3d2f7482" {
					t.Fatal("historical baseline prompt changed")
				}
			}
			if strings.Contains(prompt, "BASE ACCEPTÉE VÉRIFIÉE") != (version >= 1) {
				t.Fatal("fragment inspection did not receive its versioned baseline scope")
			}
		})
	}
}

func TestIncrementalFittingContextRefusesCorruptedAcceptedBaseline(t *testing.T) {
	s, w := managedFixture(t)
	first := managedCompleted(t, s, w, "first", "accepted source\n")
	if err := s.integrateManagedAttempt(first); err != nil {
		t.Fatal(err)
	}
	w, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	if w.Tasks[0].Status != "accepted" {
		t.Fatal("fixture baseline not accepted")
	}
	r := *w.Tasks[0].IndependentReview
	if err = os.WriteFile(filepath.Join(s.root, r.Context), []byte(`{}`), 0600); err != nil {
		t.Fatal(err)
	}
	second := managedCompleted(t, s, w, "second", "new source\n")
	calls := managedReviewCalls(t, s)
	if err = s.integrateManagedAttempt(second); err != nil {
		t.Fatal(err)
	}
	after, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, err := after.task(second.TaskID)
	if err != nil {
		t.Fatal(err)
	}
	if task.Status != "blocked" || managedReviewCalls(t, s) != calls || after.Planning.Repository.Candidate != r.CandidateSHA {
		t.Fatal("corrupt fitting baseline was trusted, spent or published", task.Status)
	}
}
